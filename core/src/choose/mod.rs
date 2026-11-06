//! Law 5: Choose. A body perceives a little (`perceive`), its brain scores the actions and one is drawn (a node spreads
//! its members over them, `spread`), and it acts: move (`go`), take, put or give (`feed`), watch (`learn::watched`),
//! rest, or Bond's actions; at its next decision it learns from the change of its well-being (`learn`). Laws.md 5.

mod feed;
mod go;
mod learn;
mod minds;
mod perceive;
mod spread;

pub use feed::{take_body, COVER};
pub use go::{go, wear};
pub(crate) use feed::{eat_for, meal};
pub(crate) use go::land_move;
pub(crate) use learn::watched;
pub use minds::Minds;
pub use perceive::cells;
pub(crate) use spread::land_act;

use perceive::{perceive, Face, HEIGHT, LN_MASS, T, WB_SCALE};
pub use perceive::{DEAD, EDIBLE, NCELL, OWN, SIGHTS, WET};

use crate::body::{Body, Lived};
use crate::brain::{self, ln, score, squash, Shape, Thought, FIXED, FORM, FORM_OUT, OVERBS, PUT, RELEASE, REST, TAKE, TAKE_FROM, VERBS, WATCH};
use crate::life::Entry;
use crate::rng::{Purpose, Rng};
use crate::space::COMPASS;
use crate::{bond, lanes, life::{Chunk, Handle}, World};

/// Features of a neighbour direction (those of the cell it leads to, `perceive::cells`, and how hard its link is to
/// pass) and of a perceived body.
pub(crate) const NF: usize = NCELL + 1;
pub(crate) const NT: usize = 6;
/// Most bodies perceived (the `SEEN` gene sets how many); parts perceived; moves (one per compass direction of the
/// mesh's links, `Level::way`); options.
const MAX_SEEN: usize = 6;
const PARTS: usize = 3;
const MOVES: usize = COMPASS;
const OPTS: usize = FIXED + MOVES + VERBS * MAX_SEEN + OVERBS * PARTS;
/// The first option toward a perceived body, and toward a perceived part.
const TO_BODY: usize = FIXED + MOVES;
const TO_PART: usize = TO_BODY + VERBS * MAX_SEEN;
/// Inputs of a brain besides one per substance and its memory: its warmth, size and the season, its cell's features,
/// and what Bond senses.
pub(crate) const INPUTS: usize = OWN + NCELL + bond::SENSE;

/// Working memory of `act`, kept from body to body.
#[derive(Default)]
pub struct ActTmp {
    pub thought: Thought,
    /// Features of each direction (flat), for learning
    pub feat: Vec<f32>,
    /// The sums of the hidden layer without the body's own cell's features (`plan`)
    plan: Vec<f32>,
    /// The base's weights on a cell's features, per hidden unit, worked out once per lineage and cell in a chunk
    /// (`plan`): where each lies in `on_sums`
    on_at: std::collections::HashMap<(u32, u32), usize>,
    on_sums: Vec<f32>,
}

impl ActTmp {
    /// A new chunk: the sums of `plan` of the last are not its (its lineages' ids are its own).
    pub fn new_chunk(&mut self) {
        self.on_at.clear();
        self.on_sums.clear();
    }
}

/// One decision: what the body perceives, its options with their scores and features, their chances and the action drawn.
struct Choice {
    /// The bodies of its cell perceived (their faces, with their ids)
    seen: [Face; MAX_SEEN],
    nseen: usize,
    parts: [usize; PARTS],
    nparts: usize,
    room: Option<(u64, bond::Enclosure)>,
    held: Option<usize>,
    logit: [f32; OPTS],
    p: [f32; OPTS],
    to: [usize; MOVES],
    fb: [f32; MAX_SEEN * NT],
    fo: [f32; PARTS * bond::NO],
    a: usize,
}

/// A body's decision at its update, from the sums of its hidden layer `z` (`Minds::sense`): perceive the bodies and
/// parts of its cell, think and learn from the last decision, score the options; an individual draws one option, keeps
/// its trace and acts, a node spreads its members over them (`spread`). Every body decides at each update, at its cell's
/// step; `lived` is what its last sub-step left, `days` the time since its last decision, d, which it learns over.
pub fn act(w: &World, d: &mut Chunk, b: &mut Body, (lived, days): (&Lived, f64), z: &[f32], t: &mut ActTmp) {
    let mut rng = Rng::new(w.seed, w.key_at(b.cell as usize, w.time), b.id, Purpose::Choose);
    let temp = b.temp(&w.chem);
    // A brain perceives sizes per member, as the body is when it decides.
    let pm = b.mass(&w.chem) / b.n();
    // (Well-being counts its energy in months of the starting brain's thinking, a unit whatever the pace of decisions.)
    let think = w.cfg.phys.think(w.cfg.phys.c_brain, b.k.copy_mass);
    // (Well-being before and after each act at the decision's temperature: an act moves amounts, not the season.)
    let now = |w: &World, b: &Body| b.wellbeing_at(&w.chem, think, temp, lived.spare);
    let wb = now(w, b);
    let Some(lin) = d.brain(w, b.lineage) else { return };
    let (s, nseen, sight) = (lin.shape, lin.seen.min(MAX_SEEN), lin.sight.max(1));
    // (Its brain reads and writes the first `s.mem` numbers of its memory; those past them are none.)
    b.mem[s.mem..].fill(0.0);
    let mut c = perceive(w, d, b, nseen, &mut rng);
    brain::think(d.at(b.cell as usize).brains.rows(b.brain, s), s, z, &mut t.thought, read(s, &c));
    learn::learn(w, b, &mut d.brains(b.cell as usize).mind(b.brain, s), s, (wb, &t.thought.h), w.cfg.span(days));
    options(w, d, b, (s, sight), (pm, b.volume(&w.chem, temp) / b.n()), &mut c, t);
    if let Some(lin) = d.brain(w, b.lineage) {
        plan(w, (b.lineage, &lin.base, d.at(b.cell as usize).brains.rows(b.brain, s)), s, z, b.cell as usize, &mut c, t);
    }
    pick(d, &mut c, &mut rng);
    let (form, zn) = noise(w, b, s, t, &mut rng);
    b.wb = wb;
    let i = b.cell as usize;
    let k = Kit { s, t, z: zn, form, wb: &now, days, ahead: w.step_days(i), meal: w.phys_days(i) };
    if b.n() > 1.0 {
        spread::spread(w, d, b, &c, &k, &mut rng);
    } else {
        b.traced = trace(w, b, &c, (&c.p, c.a, 0.0), &k, d.brains(b.cell as usize).mind(b.brain, s).trace);
        act_as(w, d, b, &c, &k, (c.a, None), &mut rng);
    }
}

/// Which rows of a brain of shape `s` decision `c` reads: all but the verbs toward bodies when it sees none and the
/// verbs toward parts when it sees none (`options`).
fn read(s: Shape, c: &Choice) -> impl Fn(usize) -> bool {
    let (bodies, parts) = (s.verb(0)..s.verb(VERBS), s.verb(VERBS)..s.form());
    let (any_body, any_part) = (c.nseen > 0, c.nparts > 0);
    move |q| (any_body || !bodies.contains(&q)) && (any_part || !parts.contains(&q))
}

/// The body of face `o` of body `b`'s cell as it is now, if it lives and is not leaving it.
fn here(d: &Chunk, b: &Body, o: &Face) -> Option<Handle> {
    let h = Handle { cell: b.cell, slot: o.slot };
    (d[h].alive && d[h].id == o.id && !d[h].leaving).then_some(h)
}

/// What the actions of one decision share besides its options: the brain's shape and what it thought, the noise drawn
/// for the shape of a part made, that shape (`noise`), the well-being of a body now (`Body::wellbeing_at`, at a
/// decision's cost and the margin of the body's reactions over its upkeep), the time since its last decision, d, which
/// a watch goes by, the time to its next update, d (its cell's step, `World::step_days`), which
/// what it gives, bites and does to parts goes by, and its physiological step, d (`World::phys_days`), which a meal by
/// the take law feeds it for: the next sub-step's physics of its update follows it, and each later one has its own
/// meal (`feed::meal`; laws.md 5.3).
struct Kit<'a> {
    s: Shape,
    t: &'a ActTmp,
    z: [f32; FORM_OUT],
    form: [f32; FORM_OUT],
    wb: &'a dyn Fn(&World, &Body) -> f32,
    days: f64,
    ahead: f64,
    meal: f64,
}

/// The score of each option: the fixed actions, a move to each neighbour (by the features of the cell it leads to as a
/// body of its sight sees it, `Life::feat`, with the differences of temperature and height from its own, and how hard
/// its link is to pass), each verb toward each perceived body and part. `pm`: a member's mass, kg; `vol`: its volume, m³.
fn options(w: &World, ch: &Chunk, b: &Body, (s, sight): (Shape, usize), (pm, vol): (f64, f64), c: &mut Choice, t: &mut ActTmp) {
    let (i, out) = (b.cell as usize, &t.thought.out);
    c.logit[..FIXED].copy_from_slice(&out[..FIXED]);
    let (f, cells, own) = (&mut t.feat, w.mesh.cells(), w.life.feat(i, 1));
    f.clear();
    f.resize(MOVES * NF, 0.0);
    for (d, e) in (0..MOVES).filter_map(|d| cells.way(i, d).map(|e| (d, e))) {
        let j = cells.adj_cell()[e] as usize;
        let fd = &mut f[d * NF..(d + 1) * NF];
        fd[..NCELL].copy_from_slice(w.life.feat(j, sight));
        (fd[T], fd[HEIGHT]) = (fd[T] - own[T], fd[HEIGHT] - own[HEIGHT]);
        fd[NCELL] = ln(lanes::resist(w, i, j) as f32);
        c.logit[FIXED + d] = score(&out[s.dir()..s.dir() + NF + 1], fd);
        c.to[d] = j;
    }
    for (q, o) in c.seen[..c.nseen].iter().enumerate() {
        let v = o.vis;
        let fq = &mut c.fb[q * NT..(q + 1) * NT];
        fq.copy_from_slice(&[ln(v[0] / pm as f32) / LN_MASS, v[1], v[2], v[3], v[4], o.wb / WB_SCALE]);
        for verb in 0..VERBS {
            c.logit[TO_BODY + q * VERBS + verb] = score(&out[s.verb(verb)..s.verb(verb) + NT + 1], fq);
        }
    }
    for (q, &p) in c.parts[..c.nparts].iter().enumerate() {
        let fq = &mut c.fo[q * bond::NO..(q + 1) * bond::NO];
        let cell = ch.at(i);
        fq.copy_from_slice(&bond::features(cell, w, b, p, pm));
        for v in (0..OVERBS).filter(|&v| bond::possible(cell, b, v, p, vol)) {
            let r = s.verb(VERBS + v);
            c.logit[TO_PART + q * OVERBS + v] = score(&out[r..r + bond::NO + 1], fq);
        }
    }
    if c.held.is_none() && c.room.is_none() {
        c.logit[RELEASE] = f32::NEG_INFINITY;
    }
}

/// One step of look-ahead (laws.md 5.3): the critic's value of the state the body would be in in each cell it can step
/// into (its own state with that cell's features in place of its own cell's: the sums of its hidden layer `z` moved by
/// the base's weights on the difference) less its value here, where staying keeps it, both at its next decision, a step
/// of its cell's step ahead (`World::step_days`; `Physics::discount` over it, as the critic learns it): γ(V(s') − V(s)),
/// times `Physics::plan_weight`, added to the score of the move there. `brain`: the lineage, its base and the body's
/// rows. The base's weights on a cell's features are worked out once per lineage and cell in a chunk
/// (`ActTmp::on_at`). Off in a null run.
fn plan(w: &World, (lineage, base, rows): (u32, &[f32], &[f32]), s: Shape, z: &[f32], i: usize, c: &mut Choice, t: &mut ActTmp) {
    let (weight, discount) = (w.cfg.phys.plan_weight, w.cfg.span(w.step_days(i)).weight(w.cfg.phys.discount));
    if w.cfg.null || weight == 0.0 || s.h == 0 {
        return;
    }
    let (at, critic) = (w.chem.ns() + OWN, &rows[s.critic() * s.r()..(s.critic() + 1) * s.r()]);
    let ActTmp { plan: at_here, on_at, on_sums, .. } = t;
    // The base's weights on the features of cell `j`, of each hidden unit: where they lie in `on_sums`.
    let mut on = |j: usize| *on_at.entry((lineage, j as u32)).or_insert_with(|| {
        let x = on_sums.len();
        on_sums.extend((0..s.h).map(|m| brain::dot(&base[m * s.ni + at..m * s.ni + at + NCELL], w.life.feat(j, 1))));
        x
    });
    // The sums without its own cell's.
    let own = on(i);
    let to: [Option<usize>; MOVES] = std::array::from_fn(|d| (c.to[d] != usize::MAX).then(|| on(c.to[d])));
    at_here.clear();
    at_here.extend((0..s.h).map(|m| z[m] - on_sums[own + m]));
    let value = |x: usize| critic[s.h] + (0..s.h).map(|m| critic[m] * squash(at_here[m] + on_sums[x + m])).sum::<f32>();
    let here = critic[s.h] + (0..s.h).map(|m| critic[m] * squash(z[m])).sum::<f32>();
    for (d, x) in to.iter().enumerate().filter_map(|(d, x)| Some((d, (*x)?))) {
        c.logit[FIXED + d] += weight * discount * (value(x) - here);
    }
}

/// Draw one action by its score (a softmax); with no option open, rest.
fn pick(d: &mut Chunk, c: &mut Choice, rng: &mut Rng) {
    debug_assert!(!c.logit.iter().any(|l| l.is_nan()), "a score from a row not worked out (`read`)");
    let top = c.logit.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    if top == f32::NEG_INFINITY {
        (c.p, c.a) = ([0.0; OPTS], REST);
        c.p[REST] = 1.0;
    } else {
        c.p = c.logit.map(|l| if l == f32::NEG_INFINITY { 0.0 } else { (l - top).exp() });
        let sum = c.p.iter().sum::<f32>();
        c.p.iter_mut().for_each(|v| *v /= sum);
        let mut u = rng.f64() as f32;
        let a = c.p.iter().position(|&v| {
            u -= v;
            u < 0.0
        });
        c.a = a.unwrap_or(REST);
    }
    (d.counts.decisions, d.counts.greedy) = (d.counts.decisions + 1.0, d.counts.greedy + (c.logit[c.a] >= top) as u8 as f64);
}

/// The shape of a part it would make and its next memory, with the noise drawn for the shape (`trace` learns from it).
fn noise(w: &World, b: &mut Body, s: Shape, t: &ActTmp, rng: &mut Rng) -> ([f32; FORM_OUT], [f32; FORM_OUT]) {
    let (out, p) = (&t.thought.out, &w.cfg.phys);
    let z: [f32; FORM_OUT] = std::array::from_fn(|_| rng.normal() as f32);
    let form = std::array::from_fn(|k| squash(out[s.form() + k]) + p.shape_noise * z[k]);
    b.mem[..s.mem].iter_mut().enumerate().for_each(|(k, m)| *m = squash(out[s.learned() + k]));
    (form, z)
}

/// The trace of the decision for learning, into `v` (the trace of the body's brain, `life::Brains`; none if the body
/// does not learn, false): the hidden layer; per learned row how its output moves the log-chance of option `a`, what
/// was done (for the shape the natural gradient, so its noise does not use up the step); and `adv`, how
/// much more that raised the well-being of the members who did it than the mean of a node's members (`spread`; 0 for
/// an individual). `chances`: the chances the action was drawn by (a node's over its likeliest options,
/// `spread::top_chances`), so the gradient's mean is 0.
fn trace(w: &World, b: &Body, c: &Choice, (chances, a, adv): (&[f32; OPTS], usize, f32), k: &Kit, v: &mut [f32]) -> bool {
    if b.learn_rate() <= 0.0 || w.cfg.null {
        return false;
    }
    let (s, t, p, out) = (k.s, k.t, &w.cfg.phys, &k.t.thought.out);
    let (nh, l) = (s.h, s.learned());
    debug_assert_eq!(v.len(), nh + l + 1, "a trace of the brain's shape");
    v.fill(0.0);
    v[..nh].copy_from_slice(&t.thought.h);
    v[nh + l] = adv;
    let cr = &mut v[nh..nh + l];
    let g = |o: usize| (o == a) as u8 as f32 - chances[o];
    for (q, cq) in cr[..FIXED].iter_mut().enumerate() {
        *cq = g(q);
    }
    // (An option of no chance that was not done, as a node's outside its likeliest, adds nothing: it is skipped.)
    let mut add = |row: usize, gq: f32, fq: &[f32]| {
        if gq == 0.0 {
            return;
        }
        cr[row] += gq;
        for (x, f) in cr[row + 1..].iter_mut().zip(fq) {
            *x += gq * f;
        }
    };
    for d in (0..MOVES).filter(|&d| c.to[d] != usize::MAX) {
        add(s.dir(), g(FIXED + d), &t.feat[d * NF..(d + 1) * NF]);
    }
    for q in 0..c.nseen {
        for verb in 0..VERBS {
            add(s.verb(verb), g(TO_BODY + q * VERBS + verb), &c.fb[q * NT..(q + 1) * NT]);
        }
    }
    for q in 0..c.nparts {
        for v in 0..OVERBS {
            add(s.verb(VERBS + v), g(TO_PART + q * OVERBS + v), &c.fo[q * bond::NO..(q + 1) * bond::NO]);
        }
    }
    let natural = |r: usize, z: f32, noise: f32| (z * noise) * (1.0 - squash(out[r]) * squash(out[r]));
    (0..FORM_OUT).for_each(|j| cr[s.form() + j] = if a == FORM { natural(s.form() + j, k.z[j], p.shape_noise) } else { 0.0 });
    true
}

/// Whether option `a` fills the step (rest, a bite, a gift, a watch, Bond's acts on parts): its members take nothing by
/// the take law until their next decision. Putting down, letting go, taking and a move leave the step to eating.
fn busy(a: usize) -> bool {
    !(matches!(a, PUT | TAKE | RELEASE) || (FIXED..TO_BODY).contains(&a))
}

/// Body `b` (taken out of its slot), a part of node `part_of` if any (`spread`), does option `a`. Forming a part, acting
/// on one and watching are queued (`life::Entry`), applied in the serial pass; so is a move (`go`). A node's
/// part that queued one of these but a watch leaves the node until it is applied (`Body::leaving`).
///
/// An act takes the time it needs of the step (laws.md 5.3): a move its walk, putting down and letting go none, every
/// other act the whole step (`busy`), which what it gives, bites and does to parts goes by. The rest of the step it
/// takes what lies at its place by the take law (`feed::take_node`), where it ends: a mover where it lands
/// (`land_move`), one whose move fails where it is; now for its physiological step, and at each later sub-step of its
/// update for that (`feed::meal`, by `Body::busy`).
fn act_as(w: &World, d: &mut Chunk, b: &mut Body, c: &Choice, k: &Kit, (a, part_of): (usize, Option<u64>), rng: &mut Rng) {
    b.leaving |= part_of.is_some() && (a == FORM || a >= TO_PART);
    b.busy = busy(a) as u8 as f32;
    let act = |on| Entry::Act { id: b.id, on, node: part_of, days: k.ahead };
    match a {
        REST => {}
        PUT => {
            feed::put(w, d, b);
            feed::take_node(w, d, b, k.meal);
        }
        TAKE => feed::take_node(w, d, b, k.meal),
        FORM => d.out.push(act(Err([k.form[0], 0.5 * (k.form[1] + 1.0), 0.5 * (k.form[2] + 1.0)]))),
        RELEASE => {
            bond::release(d, b);
            feed::take_node(w, d, b, k.meal);
        }
        a if a < TO_BODY => {
            if !go(w, d, b, c.to[a - FIXED], part_of.is_some()) {
                feed::take_node(w, d, b, k.meal);
            }
        }
        a if a >= TO_PART => {
            let part = d.at(b.cell as usize).parts[c.parts[(a - TO_PART) / OVERBS]].id;
            d.out.push(act(Ok(((a - TO_PART) % OVERBS, part))));
        }
        a => {
            let (face, verb) = (c.seen[(a - TO_BODY) / VERBS], (a - TO_BODY) % VERBS);
            if verb == WATCH {
                return d.out.push(Entry::Watch { by: b.id, n: b.n(), lineage: b.lineage, of: face.id, days: k.days });
            }
            let Some(o) = here(d, b, &face) else { return };
            if verb == TAKE_FROM { take_body(w, d, b, (o, k.ahead), rng) } else { feed::give(w, d, b, (o, k.ahead)) }
        }
    }
}
