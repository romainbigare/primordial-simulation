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
    /// Brain biases: rest, copy (its own chance, as a logit), put, take from the node, form a
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
const KINDS: [Kind; 2] = [
    Kind { scalars: [2.5, 0.0, 2.2, 0.3, 0.05, 0.3, 0.3, 1.3, -1.2, -4.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.1], cat: [15.0, 2.0], upt: [0.0, 0.0], mass: 1.0, cover: 0.1, per: 1, chance: 1.0, fixed: [2.0, -1.0, -3.0, -2.0, -6.0, -6.0, -6.0, -6.0], dir: -4.0, verbs: [-4.0, -4.0, -4.0, -6.0, -6.0, -6.0, -6.0, -6.0, -6.0] },
    Kind { scalars: [0.6, -0.5, 0.0, -3.0, 0.6, 0.3, 1.8, 1.0, -1.2, -2.0, -4.0, 12.0, 6.0, 3.0, 1.0, 0.1], cat: [-1.0, 2.6], upt: [0.3, 1.0], mass: 30.0, cover: 0.0, per: 2, chance: 0.3, fixed: [0.0, -1.6, -3.0, 0.5, -2.5, -2.0, -2.5, -3.0], dir: -1.0, verbs: [2.0, -2.0, -2.0, -1.0, 0.0, -2.0, -2.0, -2.5, -1.0] },
];

/// Place bodies of starting kind `which`: 0 light-eaters on land, 1 eaters where bodies already are.
pub fn seed(w: &mut World, which: usize) {
    let Some(kind) = KINDS.get(which) else { return };
    if roles(&w.chem).is_none() {
        return;
    }
    body::survey(w);
    let [h, mem, seen, sight] = choose::sizes(&kind.scalars);
    let shape = choose::shape(w.chem.ns(), h, mem);
    let mut r = Rng::new(w.seed, w.day, which as u64, Purpose::Body);
    let mut base = brain::random(shape, &mut r, 0.1, kind.fixed, kind.dir, kind.verbs);
    let rows = base.split_off(shape.base());
    let lineage = w.life.new_lineage(Lineage { shape, seen, sight, base });
    let genome = w.life.new_genome(rows);
    for i in 0..w.grid.n() {
        let [_, depth, z] = w.life.env[i];
        if z <= 0.0 || depth > 0.5 || r.f64() >= kind.chance || (which > 0 && w.life.at[i].is_empty()) {
            continue;
        }
        for _ in 0..kind.per {
            let Some(b) = make(w, which, i, lineage, genome) else { return };
            let delay = 1 + r.below(b.interval() as usize) as u64;
            w.life.add(b, w.day, delay);
        }
    }
}

/// A body of starting kind `which` in cell `i`, of `lineage`, born with brain rows `genome`, in
/// a new group of it, at the cell's temperature; its matter and energy are booked like an edit.
/// Not yet placed.
pub fn make(w: &mut World, which: usize, i: usize, lineage: u32, genome: u32) -> Option<Body> {
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
