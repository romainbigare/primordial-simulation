//! Placing the bodies of a starting kind: their lineage and brain, a spray of variants, the fixers among the
//! light-eaters, and the line that reports it.

use super::kinds::{light, Kind, KINDS};
use super::make::{make_with, split, KEEP_LIQUID};
use pcore::body::{self, Body, Layout, Lineage};
use pcore::brain::{self, Shape};
use pcore::rng::{Purpose, Rng};
use pcore::{choose, World};
use serde::{Deserialize, Serialize};

/// The share of the starting light-eaters that carry the gene of the fixing reaction (`Chemistry::fixing`), chosen
/// before any run: one in ten. Plants that fix with a partner are near a tenth of the flowering species, and a floor
/// of grass or forest has 5 to 20% of its cover in them. The gene then evolves as any other: it costs the enzyme's
/// upkeep.
pub const FIXERS: f64 = 0.1;
/// log10 of the speed-up of the fixing reaction in those founders: the speed a growing body needs. A body turns its
/// matter over in about half a year (3 x 10^-9 mol of the mineral a kg and second), and the gene is set as the light
/// reaction's was, the speed a body needs over the speed of the bare reaction.
pub const FIX_GENE: f32 = 9.8;
/// The stream of the draw that makes a founder a fixer (`Rng::new(seed, FIX_STREAM, id, Purpose::Body)`).
const FIX_STREAM: u64 = 0xf1;
/// Cells under more liquid than this, m, get no founder (a basin of standing liquid).
pub(super) const MAX_STANDING_M: f64 = 0.5;
/// Size of the random weights of a founder lineage's brain.
const BRAIN_NOISE: f32 = 0.1;
/// A founder's id (the key of its random streams): 1 + its kind above `KIND_BIT`, its cell above `CELL_BIT`, its
/// number in the cell below.
const KIND_BIT: u32 = 48;
const CELL_BIT: u32 = 8;

fn founder_id(which: usize, i: usize, k: usize) -> u64 {
    (1 + which as u64) << KIND_BIT | (i as u64) << CELL_BIT | k as u64
}

/// How the first bodies are placed. What is placed, and when, is initial condition and must be reported with every
/// result (`seed_with` returns the line to print).
///
/// A neutral brain has every action bias at 0 (resting, putting, taking, moving, the verbs on bodies and parts) and
/// small random weights, so nothing says "eat" or "move when hungry"; selection has to find that. The one bias kept
/// from the hand-set brains is the copy logit (the body's own chance to copy): copying is already gated by physics (a
/// member must hold its copy mass and pay for it), and a neutral 0 would make every body that holds the mass copy on
/// half of its decisions, a rate nobody chose. Then every body gets `rounds` rounds of mutation (genes by
/// `copy::mutate`, born brain rows by `copy::shake`), each body its own random walk and genome, so the first
/// generation is already a spray of variants. A neutral start gets more than one generation's step, which is too
/// small to give selection anything to pick from: the bias of every row gets rounds of `size` (after 20 rounds of 0.2
/// they span about 0.9), the other row entries keep the step of one generation so the features do not swamp the
/// brain, and the genes get `genes` times the step of one generation. Hand-set starts keep the step of one generation
/// for everything.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Seeding {
    /// Hand-set brain biases, per kind (light-eaters, animals). The default, what `ew run` uses, sets the animals'
    /// (neutral animal founders nearly all die in their first year, a lottery) and not the light-eaters'.
    pub hand: [bool; 2],
    /// Rounds of mutation given to every body placed (0: all bodies of a kind are the same)
    pub rounds: u32,
    /// Step of one round on the bias of every row (neutral starts)
    pub size: f32,
    /// How many times the step of one generation the genes get in all (neutral starts)
    pub genes: f32,
    /// The share of the light-eaters placed that carry the gene of the fixing reaction (`FIXERS`)
    pub fixers: f64,
}

impl Seeding {
    /// Hand-set brain biases and no mutation (for tests of the laws that need the same body every time).
    pub fn old() -> Self {
        Seeding { hand: [true; 2], rounds: 0, fixers: 0.0, ..Default::default() }
    }

    /// Neutral brains for the animals too (for tests that start every kind from no behaviour).
    pub fn neutral() -> Self {
        Seeding { hand: [false; 2], ..Default::default() }
    }
}

impl Default for Seeding {
    fn default() -> Self {
        Seeding { hand: [false, true], rounds: 20, size: 0.2, genes: 3.0, fixers: FIXERS }
    }
}

/// Place bodies of starting kind `which` (a row of `KINDS`) as `ew run` does (`Seeding::default`).
pub fn seed(w: &mut World, which: usize) -> String {
    seed_with(w, which, &Seeding::default())
}

/// What was placed, for the seeding line: bodies, members, kg and fixers; the ranges of the light and burning genes
/// and of the copy-mass gene; and the range, sum and sum of squares of the bias of taking from a body.
struct Placed {
    bodies: usize,
    members: f64,
    kg: f64,
    fixers: usize,
    cat: [(f32, f32); 2],
    copy_mass: (f32, f32),
    take: (f32, f32, f64, f64),
}

impl Placed {
    fn add(&mut self, w: &World, b: &Body, cat: [usize; 2], take: usize) {
        for (c, g) in self.cat.iter_mut().zip(cat) {
            *c = (c.0.min(b.genes[g]), c.1.max(b.genes[g]));
        }
        self.copy_mass = (self.copy_mass.0.min(b.genes[body::COPY_MASS]), self.copy_mass.1.max(b.genes[body::COPY_MASS]));
        let x = w.life.born(b)[take];
        self.take = (self.take.0.min(x), self.take.1.max(x), self.take.2 + x as f64, self.take.3 + (x * x) as f64);
        (self.bodies, self.members, self.kg) = (self.bodies + 1, self.members + b.count, self.kg + b.mass(&w.chem));
    }
}

/// `seed` with the choices of `how`. Returns what was placed, in one line.
pub fn seed_with(w: &mut World, which: usize, how: &Seeding) -> String {
    let Some(kind) = KINDS.get(which) else { return String::new() };
    let Some((r_light, _, _, burn)) = w.chem.life_roles() else { return String::new() };
    body::survey(w);
    let mut r = Rng::new(w.seed, w.day, which as u64, Purpose::Body);
    let hand = how.hand[(!kind.anywhere) as usize];
    let (shape, lineage, rows) = founder_lineage(w, which, kind, hand, &mut r);
    // One genome for all when nothing is sprayed; else one per body.
    let shared = (how.rounds == 0).then(|| w.life.new_genome(rows.clone()));
    let fix = w.chem.fixing().filter(|_| light(kind));
    let lay = Layout::of(&w.chem);
    // One action bias, "take from a body": its range and spread over the bodies placed.
    let take = shape.verb(0) * shape.r() + shape.h;
    let mut p = Placed { bodies: 0, members: 0.0, kg: 0.0, fixers: 0, cat: [(f32::MAX, f32::MIN); 2], copy_mass: (f32::MAX, f32::MIN), take: (f32::MAX, f32::MIN, 0.0, 0.0) };
    for i in 0..w.grid.n() {
        let [_, depth, z] = w.life.env[i];
        if z <= 0.0 || depth > MAX_STANDING_M || r.f64() >= kind.chance || (!kind.anywhere && w.life.at[i].is_empty()) {
            continue;
        }
        for k in 0..kind.per {
            let id = founder_id(which, i, k);
            let genome = shared.unwrap_or_else(|| sprayed(w, kind, shape, &rows, how, hand, id));
            let Some(mut b) = make_with(w, which, i, lineage, genome, how.rounds, if hand { 1.0 } else { how.genes }, id) else { return String::new() };
            // A share `fixers` of the light-eaters carry the gene of the fixing reaction; the draw is by the body's own id.
            if let Some(fx) = fix.filter(|_| Rng::new(w.seed, FIX_STREAM, id, Purpose::Body).f64() < how.fixers) {
                b.genes[lay.cat + fx] = FIX_GENE;
                p.fixers += 1;
            }
            p.add(w, &b, [lay.cat + r_light, lay.cat + burn], take);
            let delay = 1 + r.below(b.interval() as usize) as u64;
            w.life.add(b, w.day, delay);
        }
    }
    line(w, kind, how, hand, fix, &p)
}

/// The founders' lineage (labelled with the kind's row) and its brain: hand-set biases, or neutral ones that keep the
/// copy logit. Returns its shape, the lineage and the born rows of its brain.
fn founder_lineage(w: &mut World, which: usize, kind: &Kind, hand: bool, r: &mut Rng) -> (Shape, u32, Vec<f32>) {
    let [h, mem, seen, sight] = choose::sizes(&kind.scalars);
    let shape = choose::shape(w.chem.ns(), h, mem);
    let (fixed, dir, verbs) = if hand {
        kind.hand
    } else {
        let mut fixed = [0.0; brain::FIXED];
        fixed[1] = kind.hand.0[1];
        (fixed, 0.0, [0.0; brain::VERBS + brain::OVERBS])
    };
    let mut base = brain::random(shape, r, BRAIN_NOISE, fixed, dir, verbs);
    let rows = base.split_off(shape.base());
    (shape, w.life.new_lineage(Lineage { shape, seen, sight, base, root: which as u32 }), rows)
}

/// A genome of its own for founder `id`: the born rows after `rounds` rounds of mutation, and for a neutral start the
/// bias of each row (its last entry) as far as `rounds` rounds of `size` go.
#[allow(clippy::too_many_arguments)]
fn sprayed(w: &mut World, kind: &Kind, shape: Shape, rows: &[f32], how: &Seeding, hand: bool, id: u64) -> u32 {
    let mut v = rows.to_vec();
    let mut rr = Rng::new(w.seed, w.day, id, Purpose::Mutate);
    let size = w.cfg.phys.row_noise * 10f32.powf(kind.scalars[body::MUT]) / (w.cfg.phys.row_chance as f32).sqrt();
    for _ in 0..how.rounds {
        pcore::copy::shake(&mut v, &mut rr, size);
    }
    if !hand {
        let (r, extra) = (shape.r(), how.size * (how.rounds as f32).sqrt());
        v.chunks_exact_mut(r).for_each(|row| pcore::copy::shake(&mut row[shape.h..], &mut rr, extra));
    }
    w.life.new_genome(v)
}

/// The seeding line: what was placed, how, and the spread of the genes and of one bias.
fn line(w: &World, kind: &Kind, how: &Seeding, hand: bool, fix: Option<usize>, p: &Placed) -> String {
    let sc = &kind.scalars;
    let (fuel, liq, mat) = split(kind.store, KEEP_LIQUID as f64, w.chem.matter_roles().is_some());
    let name = format!("{} (adults: members of {:.4} kg, their body at their copy mass of {:.4} kg, every {:.0} days, strength {}, repair {}, starting mass {:.1}% energy-rich solid, {:.1}% liquid, {:.1}% body matter, a child's body {:.2} of its parent's copy mass (CHILD gene {}), {:.0}% of what a mature body builds to offspring (ALLOC gene {}))", kind.name, 10f64.powf(sc[body::COPY_MASS] as f64) / (1.0 - if mat > 0.0 { kind.store } else { 0.0 }), 10f32.powf(sc[body::COPY_MASS]), 10f32.powf(sc[body::INTERVAL]), sc[body::STRENGTH], sc[body::REPAIR], 100.0 * fuel, 100.0 * liq, 100.0 * mat, 10f32.powf(sc[body::CHILD]), sc[body::CHILD], 100.0 * sc[body::ALLOC], sc[body::ALLOC]);
    let brain = if hand { "hand-set brain biases (initial condition)" } else { "neutral brain (action biases 0, copy logit kept)" };
    let (m, n) = (p.take.2 / p.bodies.max(1) as f64, p.bodies.max(1) as f64);
    let spray = if how.rounds == 0 {
        String::new()
    } else if hand {
        ", row and gene steps of one generation".to_string()
    } else {
        format!(", row biases {} x {} (spread {:.2}), other row entries and genes at one generation's step (genes x{})", how.rounds, how.size, how.size * (how.rounds as f32).sqrt(), how.genes)
    };
    let fixing = match fix {
        Some(r) => format!("; {} of them ({:.1}%) carry the gene of reaction {r}, the fixing reaction (log10 speed-up {FIX_GENE}, share {} chosen before any run), the others have it off", p.fixers, 100.0 * p.fixers as f64 / p.bodies.max(1) as f64, how.fixers),
        None => String::new(),
    };
    format!(
        "year {:.1}: placed {} {name} ({:.3e} members, {:.3e} kg) with {brain}, {} rounds of mutation each{spray}{fixing}; light gene {:.2}..{:.2}, burn gene {:.2}..{:.2}, copy-mass gene {:.2}..{:.2}; bias of taking from a body {:.2}..{:.2} (mean {m:.2}, sd {:.2})",
        w.year(),
        p.bodies,
        p.members,
        p.kg,
        how.rounds,
        p.cat[0].0,
        p.cat[0].1,
        p.cat[1].0,
        p.cat[1].1,
        p.copy_mass.0,
        p.copy_mass.1,
        p.take.0,
        p.take.1,
        (p.take.3 / n - m * m).max(0.0).sqrt()
    )
}
