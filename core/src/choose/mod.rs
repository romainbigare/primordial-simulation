//! Law 5: Choose. A body perceives a little (`perceive`), its brain scores the actions and one is drawn (a node spreads
//! its members over them, `spread`), and it acts: move (`go`), take, put or give (`feed`), watch (`learn::watched`),
//! rest, or Bond's actions; at its next decision it learns from the change of its well-being (`learn`). Laws.md 5.

mod feed;
mod go;
mod learn;
mod perceive;
mod spread;

pub use feed::{take_body, COVER};
pub use go::{go, wear};
pub(crate) use go::land_move;
pub(crate) use learn::watched;
pub use perceive::cells;
pub(crate) use spread::land_act;

use perceive::{inputs, perceive, walk, DT_NEAR, DZ_SCALE, LN_LEAF, LN_MASS, WB_SCALE};

use crate::body::{Body, Lived};
use crate::brain::{self, ln, score, squash, Shape, Thought, ANCHOR, DIG, FIXED, FORM, FORM_OUT, OVERBS, PUT, RELEASE, REST, TAKE, TAKE_FROM, VERBS, WATCH};
use crate::life::Entry;
use crate::rng::{Purpose, Rng};
use crate::space::COMPASS;
use crate::survey::Face;
use crate::{bond, lanes, life::{Chunk, Handle}, World};

/// Features of a neighbour direction and of a perceived body.
pub(crate) const NF: usize = 10;
pub(crate) const NT: usize = 7;
/// The feature of a neighbour direction that is the depth of standing liquid there, ln(1 + depth / m) (`perceive::cells`).
pub const WET: usize = 1;
/// Most bodies perceived (the `SEEN` gene sets how many); parts perceived; moves (one per compass direction of the
/// mesh's links, `Level::way`); options.
const MAX_SEEN: usize = 6;
const PARTS: usize = 3;
const MOVES: usize = COMPASS;
const OPTS: usize = FIXED + MOVES + VERBS * MAX_SEEN + OVERBS * PARTS;
/// The first option toward a perceived body, and toward a perceived part.
const TO_BODY: usize = FIXED + MOVES;
const TO_PART: usize = TO_BODY + VERBS * MAX_SEEN;
/// Inputs of a brain besides two per substance and its memory: its own state, its cell and the season, and what Bond
/// senses.
pub(crate) const INPUTS: usize = 8 + bond::SENSE;

/// Working memory of `act`, kept from body to body.
#[derive(Default)]
pub struct ActTmp {
    pub input: Vec<f32>,
    pub thought: Thought,
    /// Features of each direction (flat), for learning
    pub feat: Vec<f32>,
    /// The cells seen with their weights (`walk`), and the marks of the walk over the cells
    seen: Vec<(u32, f32)>,
    mark: Vec<u32>,
    stamp: u32,
}

/// One decision: what the body perceives, its options with their scores and features, their chances and the action drawn.
struct Choice {
    /// The bodies perceived, as they were at the batch's start (their faces, with their ids), and whether each is in
    /// its own cell
    seen: [(Face, bool); MAX_SEEN],
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

/// A body's decision at its update: perceive, think and learn from the last decision, score the options; an individual draws one option, keeps its trace and acts, a node spreads its members over them (`spread`).
/// Every body decides at each update, once a month.
pub fn act(w: &World, d: &mut Chunk, b: &mut Body, lived: &Lived, t: &mut ActTmp) {
    let mut rng = Rng::new(w.seed, w.day(), b.id, Purpose::Choose);
    // Mass and heat capacity are as `body::live` just left them.
    let temp = b.temp(&w.chem);
    // A brain perceives sizes per member.
    let pm = lived.mass / b.n();
    let think = w.cfg.phys.think(w.cfg.phys.c_brain, b.k.copy_mass);
    // (Well-being before and after each act at the decision's temperature: an act moves amounts, not the season.)
    let now = |w: &World, b: &Body| b.wellbeing_at(&w.chem, think, temp, lived.spare);
    let wb = now(w, b);
    let Some(lin) = d.brain(w, b.lineage) else { return };
    let (s, nseen, sight) = (lin.shape, lin.seen.min(MAX_SEEN), lin.sight.max(1));
    // (Its brain reads and writes the first `s.mem` numbers of its memory; those past them are none.)
    b.mem[s.mem..].fill(0.0);
    let mut c = perceive(w, d, b, nseen, &mut rng);
    inputs(w, d.at(b.cell as usize), b, (lived, temp, s.mem), &mut t.input);
    brain::think(&lin.base, d.at(b.cell as usize).brains.rows(b.brain, s), s, &t.input, &mut t.thought);
    learn::learn(w, b, &mut d.brains(b.cell as usize).mind(b.brain, s), s, (wb, &t.thought.h));
    walk(w, b.cell as usize, sight, t);
    options(w, d, b, s, (pm, b.volume(&w.chem, temp) / b.n()), &mut c, t);
    pick(d, &mut c, &mut rng);
    let (form, z) = noise(w, b, s, t, &mut rng);
    b.wb = wb;
    let k = Kit { s, t, z, form, wb: &now };
    if b.n() > 1.0 {
        spread::spread(w, d, b, &c, &k, &mut rng);
    } else {
        b.traced = trace(w, b, &c, (&c.p, c.a, 0.0), &k, d.brains(b.cell as usize).mind(b.brain, s).trace);
        act_as(w, d, b, &c, &k, (c.a, None), &mut rng);
    }
}

/// The body of face `o` of body `b`'s cell as it is now, if it lives and is not leaving it.
fn here(d: &Chunk, b: &Body, o: &Face) -> Option<Handle> {
    let h = Handle { cell: b.cell, slot: o.slot };
    (d[h].alive && d[h].id == o.id && !d[h].leaving).then_some(h)
}

/// What the actions of one decision share besides its options: the brain's shape and what it thought, the noise drawn
/// for the shape of a part made, that shape (`noise`), and the well-being of a body now
/// (`Body::wellbeing_at`, at a decision's cost and the margin of the body's reactions over its upkeep).
struct Kit<'a> {
    s: Shape,
    t: &'a ActTmp,
    z: [f32; FORM_OUT],
    form: [f32; FORM_OUT],
    wb: &'a dyn Fn(&World, &Body) -> f32,
}

/// The score of each option: the fixed actions, a move to each neighbour (by the weighted mean of the cells it sees
/// that way), each verb toward each perceived body and part. `pm`: a member's mass, kg; `vol`: its volume, m³.
fn options(w: &World, ch: &Chunk, b: &Body, s: Shape, (pm, vol): (f64, f64), c: &mut Choice, t: &mut ActTmp) {
    let (i, ns, out, env) = (b.cell as usize, w.chem.ns(), &t.thought.out, w.life.env[b.cell as usize]);
    c.logit[..FIXED].copy_from_slice(&out[..FIXED]);
    // What the body can take (by its uptake selectivity) in a cell, as it perceives it.
    let smell = |j: usize| b.k.upt.iter().map(|&(k, u)| u * w.life.seen[j * ns + k]).sum::<f32>();
    let (here, f, cells) = (smell(i), &mut t.feat, w.mesh.cells());
    f.clear();
    f.resize(MOVES * NF, 0.0);
    // Each direction with a link: its neighbour and the link's heading.
    let (mut ways, mut nw) = ([(0, 0, [0.0; 2]); MOVES], 0);
    for (d, e) in (0..MOVES).filter_map(|d| cells.way(i, d).map(|e| (d, e))) {
        (ways[nw], nw) = ((d, cells.adj_cell()[e] as usize, cells.heading()[e]), nw + 1);
    }
    let mut sum = [0f32; MOVES];
    for &(k, wt) in &t.seen {
        // A cell seen counts toward the link whose heading lies nearest the way to it (the largest projection on it).
        let (k, o) = (k as usize, cells.offset(i, k as usize));
        let along = |h: [f64; 2]| o[0] * h[0] + o[1] * h[1];
        let &(d, j, _) = ways[..nw].iter().max_by(|p, q| along(p.2).total_cmp(&along(q.2))).expect("a cell seen lies past a link");
        let (fd, ck, e) = (&mut f[d * NF..(d + 1) * NF], &w.life.cellf[k], w.life.env[k]);
        fd.iter_mut().zip(ck).for_each(|(x, c)| *x += wt * c);
        fd[0] += wt * ((e[2] - env[2]) / DZ_SCALE) as f32;
        fd[6] += wt * ((e[0] - env[0]) / DT_NEAR) as f32;
        fd[8] += wt * smell(k);
        // The step into the next cell is felt along its link, worn or not.
        if k == j {
            fd[7] += wt * (ln(lanes::resist(w, i, j) as f32) - ck[7]);
        }
        sum[d] += wt;
    }
    for &(d, j, _) in &ways[..nw] {
        let fd = &mut f[d * NF..(d + 1) * NF];
        fd.iter_mut().for_each(|x| *x /= sum[d]);
        fd[8] -= here;
        c.logit[FIXED + d] = score(&out[s.dir()..s.dir() + NF + 1], fd);
        c.to[d] = j;
    }
    for (q, (o, same)) in c.seen[..c.nseen].iter().enumerate() {
        let v = o.vis;
        let fq = &mut c.fb[q * NT..(q + 1) * NT];
        fq.copy_from_slice(&[ln(v[0] / pm as f32) / LN_MASS, ln(v[1] / v[0]) / LN_LEAF, v[2], v[3], v[4], v[5], o.wb / WB_SCALE]);
        for verb in 0..VERBS {
            if *same || verb == WATCH {
                c.logit[TO_BODY + q * VERBS + verb] = score(&out[s.verb(verb)..s.verb(verb) + NT + 1], fq);
            }
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
    if c.held.is_none() {
        c.logit[ANCHOR] = f32::NEG_INFINITY;
        if c.room.is_none() {
            c.logit[RELEASE] = f32::NEG_INFINITY;
        }
    }
}

/// Draw one action by its score (a softmax); with no option open, rest.
fn pick(d: &mut Chunk, c: &mut Choice, rng: &mut Rng) {
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
    let mut add = |row: usize, gq: f32, fq: &[f32]| {
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

/// Body `b` (taken out of its slot), a part of node `part_of` if any (`spread`), does option `a`. Forming a part, acting
/// on one and watching are queued (`life::Entry`), applied in the serial pass; so is a move (`go`). A node's
/// part that queued one of these but a watch leaves the node until it is applied (`Body::leaving`).
fn act_as(w: &World, d: &mut Chunk, b: &mut Body, c: &Choice, k: &Kit, (a, part_of): (usize, Option<u64>), rng: &mut Rng) {
    b.leaving |= part_of.is_some() && (a == FORM || a >= TO_PART);
    let act = |on| Entry::Act { id: b.id, on, node: part_of };
    match a {
        REST => {}
        PUT => feed::put(w, d, b),
        TAKE => feed::take_node(w, d, b),
        FORM => d.out.push(act(Err([k.form[0], 0.5 * (k.form[1] + 1.0)]))),
        RELEASE => bond::release(d, b),
        ANCHOR => if let Some(p) = b.hold.filter(|_| bond::held(d.at(b.cell as usize), b).is_some()) { d.out.push(act(Ok((bond::ANCHORS, p)))) },
        DIG => bond::dig(w, d, b),
        a if a < TO_BODY => go(w, d, b, c.to[a - FIXED], part_of.is_some()),
        a if a >= TO_PART => {
            let part = d.at(b.cell as usize).parts[c.parts[(a - TO_PART) / OVERBS]].id;
            d.out.push(act(Ok(((a - TO_PART) % OVERBS, part))));
        }
        a => {
            let ((face, same), verb) = (c.seen[(a - TO_BODY) / VERBS], (a - TO_BODY) % VERBS);
            if verb == WATCH {
                return d.out.push(Entry::Watch { by: b.id, n: b.n(), lineage: b.lineage, of: face.id });
            }
            let Some(o) = here(d, b, &face).filter(|_| same) else { return };
            if verb == TAKE_FROM { take_body(w, d, b, o, rng) } else { feed::give(w, d, b, o) }
        }
    }
}
