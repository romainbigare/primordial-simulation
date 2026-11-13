//! Placing the bodies of a starting kind: their lineage and brain, a spray of variants, the fixers among the
//! light-eaters, and the line that reports it.

use super::kinds::{Bodies, Field, Hand, Kind, Place, KINDS};
use super::open_land;
use super::make::{make_with, split, KEEP_LIQUID};
use pcore::body::{self, Body, Layout};
use pcore::lineage::Lineage;
use pcore::brain::{self, Shape};
use pcore::rng::{Purpose, Rng};
use pcore::{cover, lineage, World};
use serde::{Deserialize, Serialize};

/// The share of the cells of the starting cover that carry the gene of the fixing reaction: one in ten, as fixing plants
/// are near a tenth of flowering species (docs/numbers.md). The gene then evolves as any other.
pub const FIXERS: f64 = 0.1;
/// The fixing gene of that cover (`cover::FIX`), log10 mol per kg of living cover per s: 10^-8.3, about 10 g of the
/// fixed element per m² a year for a kg of the starting cover per m² (2 kg living), what real fixing stands fix
/// (Cleveland et al. 1999).
pub const FIX_GENE: f32 = -8.3;
/// Size of the random weights of a founder lineage's brain.
const BRAIN_NOISE: f32 = 0.1;
/// Step of one round of the spray on the bias of every row of a neutral start.
const ROW_SPRAY: f32 = 0.2;
/// How many decades the mutation size of the genes of a neutral start is raised for the spray: log10 3, so the genes get
/// 3 times the step of one generation (a constant, as a library `log10` of a constant may round either way).
const GENE_SPRAY_DECADES: f32 = 0.477_121_26;
/// A founder's id (the key of its random streams): 1 + its kind above `KIND_BIT`, its cell above `CELL_BIT`, its
/// number in the cell below.
const KIND_BIT: u32 = 48;
const CELL_BIT: u32 = 8;

fn founder_id(which: usize, i: usize, k: usize) -> u64 {
    (1 + which as u64) << KIND_BIT | (i as u64) << CELL_BIT | k as u64
}

/// How the first bodies are placed: an initial condition, reported with every result (`seed_with` returns the line). A
/// neutral brain has every action bias at 0, so selection must find what to do. Every body then gets `rounds` rounds of
/// mutation, so the first generation is a spray of variants; a neutral start gets more than one generation's step (row
/// biases by `ROW_SPRAY`, genes `GENE_SPRAY` times the step), as one step gives selection too little to pick from.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Seeding {
    /// Hand-set brain biases for the kinds that have them (the animals). The default, what `ew run` uses, sets them:
    /// neutral animal founders nearly all die in their first year, a lottery
    pub hand: bool,
    /// Rounds of mutation given to every body placed (0: all bodies of a kind are the same)
    pub rounds: u32,
    /// The share of the cells of cover placed that carry the gene of the fixing reaction (`FIXERS`)
    pub fixers: f64,
}

impl Seeding {
    /// Every body of a kind alike: hand-set brain biases, no mutation and no fixers (for tests of the laws that need
    /// the same body every time).
    pub fn alike() -> Self {
        Seeding { hand: true, rounds: 0, fixers: 0.0 }
    }

    /// Neutral brains for the animals too (for tests that start every kind from no behaviour).
    pub fn neutral() -> Self {
        Seeding { hand: false, ..Default::default() }
    }
}

impl Default for Seeding {
    fn default() -> Self {
        Seeding { hand: true, rounds: 20, fixers: FIXERS }
    }
}

/// Place bodies of starting kind `which` (a row of `KINDS`) as `ew run` does (`Seeding::default`).
pub fn seed(w: &mut World, which: usize) -> String {
    seed_with(w, which, &Seeding::default())
}

/// What was placed, for the seeding line: bodies, members and kg; the ranges of the burning gene and of the copy-mass
/// gene; and the range, sum and sum of squares of the bias of taking from a body (none without a
/// brain).
struct Placed {
    bodies: usize,
    members: f64,
    kg: f64,
    burn: (f32, f32),
    copy_mass: (f32, f32),
    take: (f32, f32, f64, f64),
}

impl Placed {
    fn add(&mut self, w: &World, b: &Body, burn: usize, take: Option<usize>) {
        self.burn = (self.burn.0.min(b.genes[burn]), self.burn.1.max(b.genes[burn]));
        self.copy_mass = (self.copy_mass.0.min(b.genes[body::COPY_MASS]), self.copy_mass.1.max(b.genes[body::COPY_MASS]));
        if let Some(x) = take.map(|k| w.life.born(b)[k]) {
            self.take = (self.take.0.min(x), self.take.1.max(x), self.take.2 + x as f64, self.take.3 + (x * x) as f64);
        }
        (self.bodies, self.members, self.kg) = (self.bodies + 1, self.members + b.count, self.kg + b.mass(&w.chem));
    }
}

/// `seed` with the choices of `how`. Returns what was placed, in one line.
pub fn seed_with(w: &mut World, which: usize, how: &Seeding) -> String {
    let Some(kind) = KINDS.get(which) else { return String::new() };
    match &kind.place {
        Place::Field(f) => field(w, which, kind, f, how),
        Place::Bodies(b) => bodies(w, which, kind, b, how),
    }
}

/// Place the cover of field row `f` of kind `which` on the open land, each cell by its chance: its genes after `rounds`
/// rounds of mutation (a random walk keyed by the cell), the fixing gene on in the share `fixers` of the cells.
fn field(w: &mut World, which: usize, kind: &Kind, f: &Field, how: &Seeding) -> String {
    pcore::survey::survey(w);
    let mut r = Rng::new(w.seed, w.day(), which as u64, Purpose::Brain);
    let (mut cells, mut fixers, mut kg) = (0, 0, 0.0);
    for i in 0..w.mesh.cells().n() {
        if !open_land(w, i) || r.f64() >= f.chance {
            continue;
        }
        let id = founder_id(which, i, 0);
        let mut genes = f.genes;
        let mut rg = Rng::new(w.seed, w.day(), id, Purpose::Make);
        for _ in 0..how.rounds {
            genes = cover::mutate(&genes, w.cfg.phys.cover_mut, &mut rg);
        }
        let fixes = Rng::new(w.seed, w.day(), id, Purpose::Fixer).f64() < how.fixers;
        if fixes {
            genes[cover::FIX] = FIX_GENE;
        }
        if cover::place(w, i, genes, which as u32, f.kg) {
            (cells, fixers, kg) = (cells + 1, fixers + fixes as usize, kg + f.kg * w.mesh.cells().area()[i]);
        }
    }
    let g = &f.genes;
    format!(
        "year {:.1}: placed the cover of {} on {cells} cells ({kg:.3e} kg, {:.2} kg per m2; light capacity 10^{} mol per kg living per s, capture area 10^{} m2 per kg living, walls 10^{} of its matter, fuel kept 10^{}, frost at {} K, active at moisture 10^{}, {:.0}% to seeds, repair {}), {} rounds of mutation each; {fixers} cells ({:.1}%) carry the fixing gene 10^{FIX_GENE} (share {} chosen before any run)",
        w.year(), kind.name, f.kg, g[cover::LIGHT], g[cover::LEAF], g[cover::WALLS], g[cover::STORE], g[cover::FROST], g[cover::MOIST], 100.0 * g[cover::SEED], g[cover::REPAIR], how.rounds, 100.0 * fixers as f64 / cells.max(1) as f64, how.fixers
    )
}

/// Place the bodies of row `b` of kind `which`.
fn bodies(w: &mut World, which: usize, kind: &Kind, b: &Bodies, how: &Seeding) -> String {
    let Some((_, _, _, burn)) = w.chem.life_roles() else { return String::new() };
    pcore::survey::survey(w);
    let mut r = Rng::new(w.seed, w.day(), which as u64, Purpose::Brain);
    let hand = b.hand.filter(|_| how.hand);
    let (shape, lineage, rows) = founder_lineage(w, which, b, hand, &mut r);
    let hand = hand.is_some();
    // One genome for all when nothing is sprayed; else one per body.
    let shared = (how.rounds == 0).then(|| w.life.new_genome(rows.clone()));
    let lay = Layout::of(&w.chem);
    // One action bias, "take from a body": its range and spread over the bodies placed (a brain's only).
    let take = (shape.h > 0).then(|| shape.verb(brain::TAKE_FROM) * shape.r() + shape.h);
    let mut p = Placed { bodies: 0, members: 0.0, kg: 0.0, burn: (f32::MAX, f32::MIN), copy_mass: (f32::MAX, f32::MIN), take: (f32::MAX, f32::MIN, 0.0, 0.0) };
    for i in 0..w.mesh.cells().n() {
        let lives = w.life.in_cell(i) > 0 || pcore::contract::stand(w, i).mass > 0.0;
        if !open_land(w, i) || r.f64() >= b.chance || (!b.anywhere && !lives) {
            continue;
        }
        for k in 0..b.per {
            let id = founder_id(which, i, k);
            let genome = shared.unwrap_or_else(|| sprayed(w, b, shape, &rows, how, hand, id));
            let Some(body) = make_with(w, which, i, lineage, genome, how.rounds, if hand { 0.0 } else { GENE_SPRAY_DECADES }, id) else { return String::new() };
            p.add(w, &body, lay.diet(&w.chem, burn).expect("bodies may run life's burning"), take);
            w.add(body);
        }
    }
    line(w, kind, b, how, hand, &p)
}

/// The founders' lineage (labelled with the kind's row) and its brain: the hand-set biases `hand`, or neutral ones.
/// Returns its shape, the lineage and the born rows of its brain.
fn founder_lineage(w: &mut World, which: usize, kind: &Bodies, hand: Option<Hand>, r: &mut Rng) -> (Shape, u32, Vec<f32>) {
    let [h, mem, seen, sight] = lineage::sizes(&kind.scalars);
    let shape = lineage::shape(w.chem.ns(), h, mem);
    let mut base = match hand {
        Some((fixed, dir, wet, verbs)) => {
            let mut base = brain::random(shape, r, BRAIN_NOISE, fixed, dir, verbs);
            // The bias of the row that weighs the depth of standing liquid in a move's score.
            base[shape.base() + (shape.dir() + 1 + pcore::choose::WET) * shape.r() + shape.h] = wet;
            base
        }
        None => brain::random(shape, r, BRAIN_NOISE, [0.0; brain::FIXED], 0.0, [0.0; brain::VERBS + brain::OVERBS]),
    };
    let mut rows = base.split_off(shape.base());
    // (A brain of no hidden units has no born rows: nothing reads them, `brain::rerow`.)
    if shape.h == 0 {
        rows.clear();
    }
    (shape, w.life.new_lineage(Lineage { shape, seen, sight, base, root: which as u32 }), rows)
}

/// A genome of its own for founder `id`: the born rows after `rounds` rounds of mutation, and for a neutral start the
/// bias of each row (its last entry) as far as `rounds` rounds of `ROW_SPRAY` go.
#[allow(clippy::too_many_arguments)]
fn sprayed(w: &mut World, kind: &Bodies, shape: Shape, rows: &[f32], how: &Seeding, hand: bool, id: u64) -> u32 {
    let mut v = rows.to_vec();
    let mut rr = Rng::new(w.seed, w.day(), id, Purpose::Spray);
    let size = pcore::brain::row_step(&w.cfg.phys, body::mutation_size(&kind.scalars));
    for _ in 0..how.rounds {
        pcore::brain::shake(&mut v, &mut rr, size);
    }
    if !hand {
        let (r, extra) = (shape.r(), ROW_SPRAY * (how.rounds as f32).sqrt());
        v.chunks_exact_mut(r).for_each(|row| pcore::brain::shake(&mut row[shape.h..], &mut rr, extra));
    }
    w.life.new_genome(v)
}

/// The seeding line: what was placed, how, and the spread of the genes and of one bias.
fn line(w: &World, kind: &Kind, b: &Bodies, how: &Seeding, hand: bool, p: &Placed) -> String {
    let sc = &b.scalars;
    let (fuel, liq, mat) = split(b.store, KEEP_LIQUID as f64, w.chem.matter_roles().is_some());
    let name = format!("{} (adults: members of {:.4} kg, their body at their copy mass of {:.4} kg, strength {}, repair {}, starting mass {:.1}% energy-rich solid, {:.1}% liquid, {:.1}% body matter, a child's body {:.2} of its parent's copy mass (CHILD gene {}), {:.0}% of what a mature body builds to offspring (ALLOC gene {}))", kind.name, 10f64.powf(sc[body::COPY_MASS] as f64) / (1.0 - if mat > 0.0 { b.store } else { 0.0 }), 10f32.powf(sc[body::COPY_MASS]), sc[body::STRENGTH], sc[body::REPAIR], 100.0 * fuel, 100.0 * liq, 100.0 * mat, 10f32.powf(sc[body::CHILD]), sc[body::CHILD], 100.0 * sc[body::ALLOC], sc[body::ALLOC]);
    let brain = match (hand, lineage::sizes(sc)[0] == 0) {
        (_, true) => "no brain (HIDDEN gene 0: no decisions)",
        (true, _) => "hand-set brain biases (initial condition)",
        (false, _) => "neutral brain (action biases 0)",
    };
    let (m, n) = (p.take.2 / p.bodies.max(1) as f64, p.bodies.max(1) as f64);
    let spray = if how.rounds == 0 {
        String::new()
    } else if hand {
        ", row and gene steps of one generation".to_string()
    } else {
        format!(", row biases {} x {ROW_SPRAY} (spread {:.2}), other row entries and genes at one generation's step (genes x{:.0})", how.rounds, ROW_SPRAY * (how.rounds as f32).sqrt(), 10f32.powf(GENE_SPRAY_DECADES))
    };
    let bias = if p.take.0 > p.take.1 { String::new() } else { format!("; bias of taking from a body {:.2}..{:.2} (mean {m:.2}, sd {:.2})", p.take.0, p.take.1, (p.take.3 / n - m * m).max(0.0).sqrt()) };
    format!(
        "year {:.1}: placed {} {name} ({:.3e} members, {:.3e} kg) with {brain}, {} rounds of mutation each{spray}; burn gene {:.2}..{:.2}, copy-mass gene {:.2}..{:.2}{bias}",
        w.year(),
        p.bodies,
        p.members,
        p.kg,
        how.rounds,
        p.burn.0,
        p.burn.1,
        p.copy_mass.0,
        p.copy_mass.1,
    )
}
