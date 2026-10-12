//! First life: bodies placed on land at the start. These are initial conditions only;
//! afterwards everything follows the laws. The reactions they speed up are found from the
//! chemistry's numbers: the light reaction, the energy-rich solid it makes, the liquid it
//! uses, and the reaction that burns that solid with a gas.

use pcore::body::{self, Body, Layout, Lineage};
use pcore::chem::{Chemistry, Phase};
use pcore::rng::{Purpose, Rng};
use pcore::{brain, choose, World};

/// Typical surface temperature, K.
const T_SURF: f64 = 290.0;

struct Kind {
    scalars: [f32; body::NSCALAR],
    /// log10 speed-up of the light reaction and of the burning of the energy-rich solid
    cat: [f32; 2],
    /// Uptake selectivity of the liquid and the energy-rich solid
    upt: [f32; 2],
    /// Mass of a member, kg; cover of a node, kg/m² (0: individuals); bodies per cell placed
    mass: f64,
    cover: f64,
    per: usize,
    /// Chance per land cell
    chance: f64,
    /// Hand-set brain biases (the old start, used only with `Seeding::hand`; by default only the
    /// copy logit is used): rest, copy (its own chance, as a logit), put, take from the node, form a
    /// part, let go, join to the ground, break the ground; move; take from, put to, watch a body;
    /// hold, take from, put to, join to, break, enter a part
    fixed: [f32; brain::FIXED],
    dir: f32,
    verbs: [f32; brain::VERBS + brain::OVERBS],
}

/// Light reaction, energy-rich solid, liquid, burning reaction.
pub fn roles(chem: &Chemistry) -> Option<(usize, usize, usize, usize)> {
    let at = |k: u16| chem.phase(k as usize, T_SURF);
    let r_light = (0..chem.reactions.len()).find(|&r| chem.reactions[r].light && chem.dh(r) > 0.0)?;
    let rx = &chem.reactions[r_light];
    let rich = rx.outputs.iter().find(|p| at(p.0) == Phase::Solid)?.0 as usize;
    let liquid = rx.inputs.iter().find(|p| at(p.0) == Phase::Liquid)?.0 as usize;
    let burn = (0..chem.reactions.len()).find(|&r| {
        let i = &chem.reactions[r].inputs;
        chem.dh(r) < 0.0 && i.iter().any(|p| p.0 as usize == rich) && i.iter().any(|p| at(p.0) == Phase::Gas)
    })?;
    Some((r_light, rich, liquid, burn))
}

/// Starting kinds. Light-eaters: strong light reaction, slow burning, wide gas exchange and
/// capture area, long intervals, resting; one node per land cell, members of 1 kg (copying at
/// 2 kg), 0.1 kg/m². Eaters: fast burning, take the energy-rich solid from bodies and the
/// ground, short intervals; individuals of 30 kg (copying at 63 kg), two in some cells. Both
/// start with a mutation size of 10^-1.2 per generation (generations are short at these sizes).
/// The third kind is the clever lineage of the experiment `--clever-year`: an eater with a big
/// brain (48 hidden units, 12 memory numbers, 6 bodies perceived, sight of 3 cells) and learning
/// on (rate 10^-1.5), one per cell in 30% of the cells with bodies. It always starts with the
/// hand-set eater biases: with neutral ones even the small brain dies out (see `Seeding`), so
/// the question "does a big brain pay for its cost" could not be asked.
const KINDS: [Kind; 3] = [
    Kind { scalars: [2.5, 0.0, 2.2, 0.3, 0.05, 0.3, 0.3, 1.3, -1.2, -4.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.1], cat: [15.0, 2.0], upt: [0.0, 0.0], mass: 1.0, cover: 0.1, per: 1, chance: 1.0, fixed: [2.0, -1.0, -3.0, -2.0, -6.0, -6.0, -6.0, -6.0], dir: -4.0, verbs: [-4.0, -4.0, -4.0, -6.0, -6.0, -6.0, -6.0, -6.0, -6.0] },
    Kind { scalars: [0.6, -0.5, 0.0, -3.0, 0.6, 0.3, 1.8, 1.0, -1.2, -2.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.1], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 30.0, cover: 0.0, per: 2, chance: 0.3, fixed: [0.0, -1.6, -3.0, 0.5, -2.5, -2.0, -2.5, -3.0], dir: -1.0, verbs: [2.0, -2.0, -2.0, -1.0, 0.0, -2.0, -2.0, -2.5, -1.0] },
    Kind { scalars: [0.6, -0.5, 0.0, -3.0, 0.6, 0.3, 1.8, 1.0, -1.2, -1.5, -4.0, 48.0, 12.0, 6.0, 3.0, 0.1], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 30.0, cover: 0.0, per: 1, chance: 0.3, fixed: [0.0, -1.6, -3.0, 0.5, -2.5, -2.0, -2.5, -3.0], dir: -1.0, verbs: [2.0, -2.0, -2.0, -1.0, 0.0, -2.0, -2.0, -2.5, -1.0] },
];

/// How the first bodies are placed. What is placed, and when, is initial condition and must be
/// reported with every result (`seed` returns the line to print).
///
/// By default each kind starts as a simple ancestor with a NEUTRAL brain: every action bias is 0
/// (resting, putting, taking, moving, the verbs on bodies and parts) and the weights are small
/// random numbers, so nothing says "eat" or "move when hungry"; selection has to find that. The
/// one bias kept from the old hand-set brains is the copy logit (`Kind::fixed[1]`, the body's own
/// chance to copy): copying is already gated by physics (a member must hold its copy mass and pay
/// for it), and a neutral 0 would make every body that holds the mass copy on half of its
/// decisions, a rate nobody chose; the old value keeps the pace of generations, not a behaviour.
/// Then every body gets `rounds` rounds of mutation (genes by `copy::mutate`, born brain rows by
/// `copy::shake` as large as a copy that changes rows), each body its own random walk and, for
/// the rows, its own genome, so the first generation is already a spray of variants.
#[derive(Clone, Copy, Debug)]
pub struct Seeding {
    /// Restore the old hand-set brain biases, per kind (light-eaters, eaters), for comparison.
    pub hand: [bool; 2],
    /// Rounds of mutation given to every body placed (0: all bodies of a kind are the same).
    pub rounds: u32,
}

impl Seeding {
    /// The old start: hand-set brain biases and no mutation (for tests of the laws that need the
    /// same body every time).
    pub fn old() -> Self {
        Seeding { hand: [true; 2], rounds: 0 }
    }
}

impl Default for Seeding {
    fn default() -> Self {
        Seeding { hand: [false; 2], rounds: 20 }
    }
}

/// Place bodies of starting kind `which` (0 light-eaters on land, 1 eaters where bodies already
/// are, 2 the clever lineage where bodies already are) as a neutral ancestor sprayed with 20 rounds of mutation (`Seeding::default`).
pub fn seed(w: &mut World, which: usize) -> String {
    seed_with(w, which, &Seeding::default())
}

/// `seed` with the choices of `how`. Returns what was placed, in one line.
pub fn seed_with(w: &mut World, which: usize, how: &Seeding) -> String {
    let Some(kind) = KINDS.get(which) else { return String::new() };
    if roles(&w.chem).is_none() {
        return String::new();
    }
    body::survey(w);
    let [h, mem, seen, sight] = choose::sizes(&kind.scalars);
    let shape = choose::shape(w.chem.ns(), h, mem);
    let mut r = Rng::new(w.seed, w.day, which as u64, Purpose::Body);
    let hand = how.hand[which.min(1)] || which == 2;
    let (fixed, dir, verbs) = if hand {
        (kind.fixed, kind.dir, kind.verbs)
    } else {
        let mut fixed = [0.0; brain::FIXED];
        fixed[1] = kind.fixed[1];
        (fixed, 0.0, [0.0; brain::VERBS + brain::OVERBS])
    };
    let mut base = brain::random(shape, &mut r, 0.1, fixed, dir, verbs);
    let rows = base.split_off(shape.base());
    let lineage = w.life.new_lineage(Lineage { shape, seen, sight, base });
    // One genome for all when nothing is sprayed; else one per body.
    let shared = (how.rounds == 0).then(|| w.life.new_genome(rows.clone()));
    let (mut placed, mut members, mut kg) = (0usize, 0.0, 0.0);
    let (mut cat, mut cmass) = ([(f32::MAX, f32::MIN); 2], (f32::MAX, f32::MIN));
    let (light, burn) = roles(&w.chem).map_or((0, 0), |(l, _, _, b)| (l, b));
    for i in 0..w.grid.n() {
        let [_, depth, z] = w.life.env[i];
        if z <= 0.0 || depth > 0.5 || r.f64() >= kind.chance || (which > 0 && w.life.at[i].is_empty()) {
            continue;
        }
        for k in 0..kind.per {
            let id = (1 + which as u64) << 48 | (i as u64) << 8 | k as u64;
            let genome = if let Some(g) = shared {
                g
            } else {
                let mut v = rows.clone();
                let mut rr = Rng::new(w.seed, w.day, id, Purpose::Mutate);
                let size = w.cfg.phys.row_noise * 10f32.powf(kind.scalars[body::MUT]) / (w.cfg.phys.row_chance as f32).sqrt();
                for _ in 0..how.rounds {
                    pcore::copy::shake(&mut v, &mut rr, size);
                }
                w.life.new_genome(v)
            };
            let Some(b) = make_with(w, which, i, lineage, genome, how.rounds, id) else { return String::new() };
            let lay = Layout::of(&w.chem);
            for (c, g) in cat.iter_mut().zip([lay.cat + light, lay.cat + burn]) {
                *c = (c.0.min(b.genes[g]), c.1.max(b.genes[g]));
            }
            cmass = (cmass.0.min(b.genes[body::COPY_MASS]), cmass.1.max(b.genes[body::COPY_MASS]));
            (placed, members, kg) = (placed + 1, members + b.count, kg + b.mass(&w.chem));
            let delay = 1 + r.below(b.interval() as usize) as u64;
            w.life.add(b, w.day, delay);
        }
    }
    let name = ["light-eaters", "eaters", "clever eaters (48 hidden, 12 memory, 6 seen, sight 3, learning on)"][which];
    let brain = if hand { "hand-set brain biases (old start)" } else { "neutral brain (action biases 0, copy logit kept)" };
    format!(
        "year {:.1}: placed {placed} {name} ({members:.3e} members, {kg:.3e} kg) with {brain}, {} rounds of mutation each; light gene {:.2}..{:.2}, burn gene {:.2}..{:.2}, copy-mass gene {:.2}..{:.2}",
        w.year(),
        how.rounds,
        cat[0].0,
        cat[0].1,
        cat[1].0,
        cat[1].1,
        cmass.0,
        cmass.1
    )
}

/// A body of starting kind `which` in cell `i`, of `lineage`, born with brain rows `genome`, in
/// a new group of it, at the cell's temperature; its matter and energy are booked like an edit.
/// Not yet placed.
pub fn make(w: &mut World, which: usize, i: usize, lineage: u32, genome: u32) -> Option<Body> {
    make_with(w, which, i, lineage, genome, 0, 0)
}

/// `make` after `rounds` rounds of mutation of its genes (a random walk keyed by `id`).
fn make_with(w: &mut World, which: usize, i: usize, lineage: u32, genome: u32, rounds: u32, id: u64) -> Option<Body> {
    let ((r_light, rich, liquid, burn), kind) = (roles(&w.chem)?, KINDS.get(which)?);
    let group = w.life.new_group(lineage, w.life.genomes[genome as usize].rows.clone());
    let (chem, ledger, t) = (&w.chem, &mut w.ledger, w.life.env[i][0]);
    let (lay, ns) = (Layout::of(chem), chem.ns());
    // Genes that are off start below zero, so small changes keep them off.
    let mut genes = vec![-0.1f32; lay.len];
    genes[lay.cat..lay.min].fill(-1.0);
    genes[..body::NSCALAR].copy_from_slice(&kind.scalars);
    (genes[lay.cat + r_light], genes[lay.cat + burn]) = (kind.cat[0], kind.cat[1]);
    (genes[lay.min + liquid], genes[lay.min + rich]) = (0.3, 0.02);
    (genes[lay.upt + liquid], genes[lay.upt + rich]) = (kind.upt[0], kind.upt[1]);
    genes[lay.keep..].fill(0.9);
    genes[lay.keep + liquid] = 0.75;
    let mut rg = Rng::new(w.seed, w.day, id, Purpose::Mutate);
    for _ in 0..rounds {
        genes = pcore::copy::mutate(&lay, &genes, &mut rg);
    }
    let count = if kind.cover > 0.0 { kind.cover * w.grid.area() / kind.mass } else { 1.0 };
    let mut store = vec![0.0; ns];
    store[liquid] = 0.7 * count * kind.mass / chem.subs[liquid].molar_mass;
    store[rich] = 0.3 * count * kind.mass / chem.subs[rich].molar_mass;
    let mut b = Body { cell: i as u32, count, lineage, group, near: u32::MAX, mark: w.light[i], store, genes, genome, ..Default::default() };
    b.e = b.heat_cap(chem) * t;
    ledger.imposed_energy += b.e;
    for (k, &a) in b.store.iter().enumerate() {
        chem.add_elems(k, a, &mut ledger.imposed_elems);
        ledger.imposed_energy += a * chem.subs[k].hf;
    }
    (b.wet, b.vis) = (b.key_liquids(chem), b.visible(chem));
    Some(b)
}
