//! Law 5: Choose. A body perceives a little of itself, what it holds, its memory, its cell, the cells around it in 8
//! directions (as far as its sight), up to a few other bodies in or next to its cell (their looks, signals and
//! well-being, and how past dealings with them went; walls and cover hide them) and up to 3 parts in its cell. How much
//! it remembers, sees and thinks is set by genes, and each costs energy per decision (`Lineage::cost`). The brain of its
//! group, plus a few numbers of its own, scores every action; one is drawn by those scores, and the body emits a signal
//! and writes its memory. Actions are physical: move (`go`), take from the ground or a body, put to the ground or give
//! to a body (`feed`), watch a body (`learn::watch`), rest, and the actions of Bond. Each costs energy paid by the body's
//! own reactions; if it cannot pay, it fails. At its next decision the brain learns from the change of the body's own
//! well-being (`learn`).

mod feed;
mod go;
mod learn;
mod perceive;

pub use feed::take_body;
pub use go::{go, wear};
pub(crate) use learn::regroup;
pub use perceive::{cells, Sector};

use perceive::{inputs, perceive, sector, DT_NEAR, DZ_SCALE, LN_LEAF, LN_MASS, WB_SCALE};

use crate::body::{Body, Lived};
use crate::brain::{self, ln, score, squash, Shape, Thought, ANCHOR, DIG, FIXED, FORM, FORM_OUT, GIVE, OVERBS, PUT, RELEASE, REST, SIG, TAKE, TAKE_FROM, VERBS, WATCH};
use crate::space::square::DIRS;
use crate::rng::{Purpose, Rng};
use crate::{bond, lanes, nodes, World};

/// Features of a neighbour direction and of a perceived body.
pub(crate) const NF: usize = 10;
pub(crate) const NT: usize = 11;
/// Most bodies perceived (the `SEEN` gene sets how many); parts perceived; moves (one per neighbour); options.
const MAX_SEEN: usize = 6;
const PARTS: usize = 3;
const MOVES: usize = DIRS.len();
const OPTS: usize = FIXED + MOVES + VERBS * MAX_SEEN + OVERBS * PARTS;
/// The first option toward a perceived body, and toward a perceived part.
const TO_BODY: usize = FIXED + MOVES;
const TO_PART: usize = TO_BODY + VERBS * MAX_SEEN;
/// Inputs of a brain besides two per substance (its stores, the cell's loose amounts) and its memory: the eight numbers
/// of its own state, its cell and the season, the signals heard, and what Bond senses (`bond::sense`).
pub(crate) const INPUTS: usize = 8 + SIG + bond::SENSE;
const NONE: u32 = u32::MAX;

/// Working memory of `act`, kept from body to body.
#[derive(Default)]
pub struct ActTmp {
    pub input: Vec<f32>,
    pub thought: Thought,
    /// Features of each direction (flat), for learning
    pub feat: Vec<f32>,
    /// By sight, the cells seen in each direction
    pub sector: Vec<Sector>,
}

/// One decision: what the body perceives (bodies, with whether each is in its cell; parts; the room it is in and the
/// part it holds), its options (the score of each, the cell each move goes to, the features of each body and part), the
/// chance of each and the action drawn.
struct Choice {
    seen: [(usize, bool); MAX_SEEN],
    nseen: usize,
    parts: [usize; PARTS],
    nparts: usize,
    room: Option<(usize, bond::Enclosure)>,
    held: Option<usize>,
    logit: [f32; OPTS],
    p: [f32; OPTS],
    to: [usize; MOVES],
    fb: [f32; MAX_SEEN * NT],
    fo: [f32; PARTS * bond::NO],
    a: usize,
}

/// A body's decision at its update: it perceives (`perceive`, `inputs`), its brain thinks and learns from the last
/// decision (`learn`), scores its options (`options`), draws one (`pick`), keeps the trace for learning and emits its
/// signal (`trace`), and acts (`dispatch`).
pub fn act(w: &mut World, slot: usize, b: &mut Body, lived: &Lived, t: &mut ActTmp) {
    let mut rng = Rng::new(w.seed, w.day(), b.id(slot), Purpose::Choose);
    // Mass and heat capacity are as `body::live` just left them.
    let temp = b.temp_at(&w.chem, lived.cap);
    // A brain perceives sizes per member.
    let pm = lived.mass / b.n();
    let wb = b.wellbeing_at(&w.chem, w.cfg.phys.think(w.cfg.phys.c_brain, b.dec(crate::body::COPY_MASS)), temp, lived.spare);
    w.life.settle(b.group);
    let Some(lin) = w.life.brains[b.lineage as usize].as_ref() else { return };
    let (s, nseen, sight) = (lin.shape, lin.seen.min(MAX_SEEN), lin.sight.max(1));
    b.mem.resize(s.mem, 0.0);
    while t.sector.len() <= sight {
        t.sector.push(sector(t.sector.len()));
    }
    let mut c = perceive(w, slot, b, nseen, &mut rng);
    inputs(w, b, lived, temp, &c, &mut t.input);
    brain::think(&lin.base, &w.life.groups[b.group as usize].rows, s, &t.input, &mut t.thought);
    learn::learn(w, b, s, wb, &t.thought.h);
    options(w, b, s, (pm, sight), &mut c, t);
    pick(w, &mut c, &mut rng);
    let form = trace(w, b, s, &c, t, &mut rng);
    b.wb = wb;
    dispatch(w, slot, b, &c, form, &mut rng);
}

/// The score of each option: the fixed actions, a move to each neighbour (each direction's features are the weighted
/// mean over the cells it sees that way), each verb toward each perceived body and part. Letting go needs something
/// held or walls around; joining to the ground, a held part. A node does the actions of Bond through one member split
/// off (`dispatch`): with less than two members it has none to spare. `pm` is the mass of a member, kg.
fn options(w: &World, b: &Body, s: Shape, (pm, sight): (f64, usize), c: &mut Choice, t: &mut ActTmp) {
    let (i, ns, out, env) = (b.cell as usize, w.chem.ns(), &t.thought.out, w.life.env[b.cell as usize]);
    for ((l, o), j) in c.logit[..FIXED].iter_mut().zip(out).zip(&b.adj) {
        *l = o + j;
    }
    // What the body can take (by its uptake selectivity) in a cell, as it perceives it.
    let smell = |j: usize| b.k.upt.iter().map(|&(k, u)| u * w.life.seen[j * ns + k]).sum::<f32>();
    let (here, f, sec) = (smell(i), &mut t.feat, &t.sector[sight]);
    f.clear();
    f.resize(MOVES * NF, 0.0);
    for (d, &off) in DIRS.iter().enumerate() {
        let Some(j) = w.map.step(i, off) else { continue };
        let (fd, mut sum) = (&mut f[d * NF..(d + 1) * NF], 0.0);
        for &(dx, dy, wt) in &sec[d] {
            let Some(k) = w.map.step(i, (dx, dy)) else { continue };
            let (ck, e) = (&w.life.cellf[k], w.life.env[k]);
            fd[..8].iter_mut().zip(ck).for_each(|(x, c)| *x += wt * c);
            fd[0] += wt * ((e[2] - env[2]) / DZ_SCALE) as f32;
            fd[6] += wt * ((e[0] - env[0]) / DT_NEAR) as f32;
            fd[8] += wt * smell(k);
            fd[9] += wt * ln(1.0 + w.parts.at[k].len() as f32);
            // The step into the next cell is felt along its edge, worn or not.
            if (dx, dy) == off {
                fd[7] += wt * (ln(lanes::resist(w, i, j) as f32) - ck[7]);
            }
            sum += wt;
        }
        fd.iter_mut().for_each(|x| *x /= sum);
        fd[8] -= here;
        c.logit[FIXED + d] = score(&out[s.dir()..s.dir() + NF + 1], fd) + b.adj[FIXED];
        c.to[d] = j;
    }
    for (q, &(o, same)) in c.seen[..c.nseen].iter().enumerate() {
        let ob = &w.life.bodies[o];
        let v = ob.vis;
        let known = b.recall(ob.id(o));
        let fq = &mut c.fb[q * NT..(q + 1) * NT];
        fq.copy_from_slice(&[ln(v[0] / pm as f32) / LN_MASS, ln(v[1] / v[0]) / LN_LEAF, v[2], v[3], v[4], v[5], ob.sig[0], ob.sig[1], known.unwrap_or(0.0), known.is_some() as u8 as f32, ob.wb / WB_SCALE]);
        for verb in 0..VERBS {
            if same || verb == WATCH {
                c.logit[TO_BODY + q * VERBS + verb] = score(&out[s.verb(verb)..s.verb(verb) + NT + 1], fq) + b.adj[FIXED + 1 + verb];
            }
        }
    }
    let vol = b.volume(&w.chem) / b.n();
    for (q, &p) in c.parts[..c.nparts].iter().enumerate() {
        let fq = &mut c.fo[q * bond::NO..(q + 1) * bond::NO];
        fq.copy_from_slice(&bond::features(w, b, p, pm));
        for v in (0..OVERBS).filter(|&v| bond::possible(w, b, v, p, vol)) {
            let r = s.verb(VERBS + v);
            c.logit[TO_PART + q * OVERBS + v] = score(&out[r..r + bond::NO + 1], fq) + b.adj[FIXED + 1 + VERBS + v];
        }
    }
    if b.n() > 1.0 && b.count < 2.0 {
        c.logit[FORM..FIXED].fill(f32::NEG_INFINITY);
        c.logit[TO_PART..].fill(f32::NEG_INFINITY);
    }
    if c.held.is_none() {
        c.logit[ANCHOR] = f32::NEG_INFINITY;
        if c.room.is_none() {
            c.logit[RELEASE] = f32::NEG_INFINITY;
        }
    }
}

/// Draw one action by its score (a softmax); with no option open at all, rest. Copying is not among them: a body makes
/// children only from its brood (`copy::copy`), so no brain bias sets how often anything is born.
fn pick(w: &mut World, c: &mut Choice, rng: &mut Rng) {
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
    (w.counts.decisions, w.counts.greedy) = (w.counts.decisions + 1.0, w.counts.greedy + (c.logit[c.a] >= top) as u8 as f64);
}

/// The trace of the decision for learning: the hidden layer, and per learned row how its output moves the log-chance of
/// what was done and of the signal. For the values drawn around a mean (signal, shape) that is the natural gradient
/// (times the spread squared), so their noise does not use up the step allowed to the actions. Then the signal and the
/// shape of a part made (elongation, hollowness), drawn around the brain's means, and the next memory. Returns the
/// shape.
fn trace(w: &World, b: &mut Body, s: Shape, c: &Choice, t: &ActTmp, rng: &mut Rng) -> [f32; FORM_OUT] {
    let (l, nh, learning, out) = (s.learned(), s.h, b.learn_rate() > 0.0 && !w.cfg.null, &t.thought.out);
    let (a, p) = (c.a, &c.p);
    b.trace.resize(if learning { nh + l } else { 0 }, 0.0);
    if learning {
        let (th, cr) = b.trace.split_at_mut(nh);
        th.copy_from_slice(&t.thought.h);
        cr.fill(0.0);
        let g = |o: usize| (o == a) as u8 as f32 - p[o];
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
    }
    for k in 0..SIG {
        let (mu, z) = (squash(out[s.sig() + k]), rng.normal() as f32);
        b.sig[k] = mu + w.cfg.phys.signal_noise * z;
        if learning {
            b.trace[nh + s.sig() + k] = z * w.cfg.phys.signal_noise * (1.0 - mu * mu);
        }
    }
    let mut form = [0f32; FORM_OUT];
    for k in 0..FORM_OUT {
        let (mu, z) = (squash(out[s.form() + k]), rng.normal() as f32);
        form[k] = mu + w.cfg.phys.shape_noise * z;
        if learning {
            b.trace[nh + s.form() + k] = if a == FORM { z * w.cfg.phys.shape_noise * (1.0 - mu * mu) } else { 0.0 };
        }
    }
    for (k, m) in b.mem.iter_mut().enumerate() {
        *m = squash(out[l + k]);
    }
    form
}

/// Do the action drawn. A node does an action of Bond through one member, split off as an individual that acts
/// (`nodes::one`). If it then holds or is inside something it lives on as one (`nodes::crowd` never merges it); else it
/// has no reason to stay apart and goes back into the node at once, with what it got.
fn dispatch(w: &mut World, slot: usize, node: &mut Body, c: &Choice, form: [f32; FORM_OUT], rng: &mut Rng) {
    let a = c.a;
    let (me, mut one) = if node.n() > 1.0 && ((FORM..FIXED).contains(&a) || a >= TO_PART) {
        let s = nodes::one(w, node, rng);
        (s, Some(std::mem::take(&mut w.life.bodies[s])))
    } else {
        (slot, None)
    };
    let (b, slot) = (one.as_mut().unwrap_or(&mut *node), me);
    match a {
        REST => {}
        PUT => feed::put(w, b),
        TAKE => feed::take_node(w, slot, b),
        FORM => bond::form(w, slot, b, [form[0], 0.5 * (form[1] + 1.0)]),
        RELEASE => bond::release(w, b),
        ANCHOR => bond::anchor(w, b),
        DIG => bond::dig(w, b),
        a if a < TO_BODY => go(w, slot, b, c.to[a - FIXED]),
        a if a >= TO_PART => bond::act(w, slot, b, (a - TO_PART) % OVERBS, c.parts[(a - TO_PART) / OVERBS]),
        a => {
            let (q, verb) = ((a - TO_BODY) / VERBS, (a - TO_BODY) % VERBS);
            match verb {
                TAKE_FROM => take_body(w, slot, b, c.seen[q].0, rng),
                GIVE => feed::give(w, slot, b, c.seen[q].0),
                _ => learn::watch(w, b, c.seen[q].0),
            }
        }
    }
    if let Some(mut o) = one {
        // A member that stays apart is seen as what its action left of it; one that goes back is seen in the node.
        let apart = o.hold.is_some() || o.inside.is_some();
        if apart {
            o.vis = o.visible(&w.chem, &w.cfg);
        }
        w.life.bodies[me] = o;
        if !apart {
            let o = w.life.bodies[me].clone();
            w.life.remove(me);
            nodes::take_in(w, node, o, None);
        }
    }
}
