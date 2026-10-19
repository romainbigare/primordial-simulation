//! Law 5: Choose. A body perceives a little (`perceive`), its brain scores the actions and one is drawn (a node spreads
//! its members over them, `spread`), and it acts: move (`go`), take, put or give (`feed`), watch (`learn::watch`),
//! rest, or Bond's actions; at its next decision it learns from the change of its well-being (`learn`). Laws.md 5.

mod feed;
mod go;
mod learn;
mod perceive;
mod spread;

pub use feed::take_body;
pub use go::{go, wear};
pub use perceive::{cells, Sector};

use perceive::{inputs, perceive, sector, DT_NEAR, DZ_SCALE, LN_LEAF, LN_MASS, WB_SCALE};

use crate::body::{Body, Lived};
use crate::brain::{self, ln, score, squash, Shape, Thought, ANCHOR, DIG, FIXED, FORM, FORM_OUT, GIVE, OVERBS, PUT, RELEASE, REST, SIG, TAKE, TAKE_FROM, VERBS, WATCH};
use crate::space::square::DIRS;
use crate::rng::{Purpose, Rng};
use crate::{bond, lanes, World};

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
/// Inputs of a brain besides two per substance and its memory: its own state, its cell and the season, the signals
/// heard, and what Bond senses.
pub(crate) const INPUTS: usize = 8 + SIG + bond::SENSE;

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

/// One decision: what the body perceives, its options with their scores and features, their chances and the action drawn.
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

/// A body's decision at its update: perceive, think and learn from the last decision, score the options, emit the
/// signal; an individual draws one option, keeps its trace and acts, a node spreads its members over them (`spread`).
pub fn act(w: &mut World, slot: usize, b: &mut Body, lived: &Lived, t: &mut ActTmp) {
    let mut rng = Rng::new(w.seed, w.day(), b.id(slot), Purpose::Choose);
    // Mass and heat capacity are as `body::live` just left them.
    let temp = b.temp(&w.chem);
    // A brain perceives sizes per member.
    let pm = lived.mass / b.n();
    let think = w.cfg.phys.think(w.cfg.phys.c_brain, b.dec(crate::body::COPY_MASS));
    let wb = b.wellbeing_at(&w.chem, think, temp, lived.spare);
    let Some(lin) = w.life.brains[b.lineage as usize].as_ref() else { return };
    let (s, nseen, sight) = (lin.shape, lin.seen.min(MAX_SEEN), lin.sight.max(1));
    b.mem.resize(s.mem, 0.0);
    while t.sector.len() <= sight {
        t.sector.push(sector(t.sector.len()));
    }
    let mut c = perceive(w, slot, b, nseen, &mut rng);
    inputs(w, b, lived, temp, &c, &mut t.input);
    brain::think(&lin.base, &b.rows, s, &t.input, &mut t.thought);
    learn::learn(w, b, s, wb, &t.thought.h);
    options(w, b, s, (pm, b.volume(&w.chem, temp) / b.n(), sight), &mut c, t);
    pick(w, &mut c, &mut rng);
    let (form, z) = noise(w, b, s, t, &mut rng);
    b.wb = wb;
    let k = Kit { s, t, z, form, think, spare: lived.spare };
    if b.n() > 1.0 {
        spread::spread(w, slot, b, &c, &k, &mut rng);
    } else {
        b.trace = trace(w, b, &c, &k, c.a, 0.0);
        act_as(w, slot, b, &c, &k, c.a, &mut rng);
    }
}

/// What the actions of one decision share besides its options: the brain's shape and what it thought, the noise drawn
/// for the signal and the shape of a part made, that shape (`noise`), a decision's cost, J, and the margin of the body's
/// reactions over its upkeep (`Lived::spare`).
struct Kit<'a> {
    s: Shape,
    t: &'a ActTmp,
    z: [f32; SIG + FORM_OUT],
    form: [f32; FORM_OUT],
    think: f64,
    spare: f64,
}

impl Kit<'_> {
    /// The well-being of body `b` now (`Body::wellbeing_at`).
    fn wb(&self, w: &World, b: &Body) -> f32 {
        b.wellbeing_at(&w.chem, self.think, b.temp(&w.chem), self.spare)
    }
}

/// The score of each option: the fixed actions, a move to each neighbour (by the weighted mean of the cells it sees
/// that way), each verb toward each perceived body and part. `pm`: a member's mass, kg; `vol`: its volume, m³.
fn options(w: &World, b: &Body, s: Shape, (pm, vol, sight): (f64, f64, usize), c: &mut Choice, t: &mut ActTmp) {
    let (i, ns, out, env) = (b.cell as usize, w.chem.ns(), &t.thought.out, w.life.env[b.cell as usize]);
    c.logit[..FIXED].copy_from_slice(&out[..FIXED]);
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
        c.logit[FIXED + d] = score(&out[s.dir()..s.dir() + NF + 1], fd);
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
                c.logit[TO_BODY + q * VERBS + verb] = score(&out[s.verb(verb)..s.verb(verb) + NT + 1], fq);
            }
        }
    }
    for (q, &p) in c.parts[..c.nparts].iter().enumerate() {
        let fq = &mut c.fo[q * bond::NO..(q + 1) * bond::NO];
        fq.copy_from_slice(&bond::features(w, b, p, pm));
        for v in (0..OVERBS).filter(|&v| bond::possible(w, b, v, p, vol)) {
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

/// The signal it emits, the shape of a part it would make and its next memory, with the noise drawn for the signal and
/// the shape (`trace` learns from it).
fn noise(w: &World, b: &mut Body, s: Shape, t: &ActTmp, rng: &mut Rng) -> ([f32; FORM_OUT], [f32; SIG + FORM_OUT]) {
    let (out, p) = (&t.thought.out, &w.cfg.phys);
    let (mut z, mut form) = ([0f32; SIG + FORM_OUT], [0f32; FORM_OUT]);
    for k in 0..SIG {
        z[k] = rng.normal() as f32;
        b.sig[k] = squash(out[s.sig() + k]) + p.signal_noise * z[k];
    }
    for k in 0..FORM_OUT {
        z[SIG + k] = rng.normal() as f32;
        form[k] = squash(out[s.form() + k]) + p.shape_noise * z[SIG + k];
    }
    for (k, m) in b.mem.iter_mut().enumerate() {
        *m = squash(out[s.learned() + k]);
    }
    (form, z)
}

/// The trace of the decision for learning (none if the body does not learn): the hidden layer; per learned row how its
/// output moves the log-chance of option `a`, what was done (for the signal and shape the natural gradient, so their
/// noise does not use up the step); and `adv`, how much more that raised the well-being of the members who did it than
/// the mean of a node's members (`spread`; 0 for an individual).
fn trace(w: &World, b: &Body, c: &Choice, k: &Kit, a: usize, adv: f32) -> Vec<f32> {
    if b.learn_rate() <= 0.0 || w.cfg.null {
        return Vec::new();
    }
    let (s, t, p) = (k.s, k.t, &w.cfg.phys);
    let (nh, l, out) = (s.h, s.learned(), &t.thought.out);
    let mut v = vec![0.0; nh + l + 1];
    v[..nh].copy_from_slice(&t.thought.h);
    v[nh + l] = adv;
    let cr = &mut v[nh..nh + l];
    let g = |o: usize| (o == a) as u8 as f32 - c.p[o];
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
    for j in 0..SIG {
        let mu = squash(out[s.sig() + j]);
        cr[s.sig() + j] = k.z[j] * p.signal_noise * (1.0 - mu * mu);
    }
    for j in 0..FORM_OUT {
        let mu = squash(out[s.form() + j]);
        cr[s.form() + j] = if a == FORM { k.z[SIG + j] * p.shape_noise * (1.0 - mu * mu) } else { 0.0 };
    }
    v
}

/// Body `b` (in slot `slot`) does option `a`.
fn act_as(w: &mut World, slot: usize, b: &mut Body, c: &Choice, k: &Kit, a: usize, rng: &mut Rng) {
    match a {
        REST => {}
        PUT => feed::put(w, b),
        TAKE => feed::take_node(w, slot, b),
        FORM => bond::form(w, slot, b, [k.form[0], 0.5 * (k.form[1] + 1.0)]),
        RELEASE => bond::release(w, b),
        ANCHOR => bond::anchor(w, b),
        DIG => bond::dig(w, b),
        a if a < TO_BODY => go(w, slot, b, c.to[a - FIXED]),
        // (Members of a node acting before may have used up the part or killed the body.)
        a if a >= TO_PART => {
            let p = c.parts[(a - TO_PART) / OVERBS];
            if w.parts.parts[p].alive {
                bond::act(w, slot, b, (a - TO_PART) % OVERBS, p);
            }
        }
        a => {
            let (o, verb) = (c.seen[(a - TO_BODY) / VERBS].0, (a - TO_BODY) % VERBS);
            if !w.life.bodies[o].alive {
                return;
            }
            match verb {
                TAKE_FROM => take_body(w, slot, b, o, rng),
                GIVE => feed::give(w, slot, b, o),
                _ => learn::watch(w, slot, b, o),
            }
        }
    }
}
