//! First life: bodies placed on land at the start. These are initial conditions only;
//! afterwards everything follows the laws. The reactions they speed up are found from the
//! chemistry's numbers: the light reaction, the energy-rich solid it makes, the liquid it
//! uses, and the reaction that burns that solid with a gas.

use pcore::body::{self, Body, Layout};
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
    mass: f64,
    /// Chance per land cell
    chance: f64,
    /// Brain biases: rest, copy, put, take from the node; move; take from a body
    fixed: [f32; 4],
    dir: f32,
    other: f32,
}

/// Light reaction, energy-rich solid, liquid, burning reaction.
fn roles(chem: &Chemistry) -> Option<(usize, usize, usize, usize)> {
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

/// Place bodies of starting kind `which`: 0 light-eaters on land, 1 eaters where bodies already are.
pub fn seed(w: &mut World, which: usize) {
    let Some((r_light, rich, liquid, burn)) = roles(&w.chem) else { return };
    body::survey(w);
    let chem = w.chem.clone();
    let (lay, shape, ns) = (Layout::of(&chem), choose::shape(chem.ns()), chem.ns());
    let mut r = Rng::new(w.seed, w.day, which as u64, Purpose::Body);
    let kinds = [
        // Light-eaters: strong light reaction, slow burning, wide gas exchange and capture
        // area, long intervals, resting.
        Kind { scalars: [2.5, 0.0, 2.2, 0.3, 0.05, 0.3, 5.7, 1.3, -0.7], cat: [15.0, 1.2], upt: [0.0, 0.0], mass: 2e5, chance: 0.3, fixed: [2.0, 1.0, -3.0, -2.0], dir: -4.0, other: -4.0 },
        // Eaters: fast burning, take the energy-rich solid from bodies and the ground, short
        // intervals.
        Kind { scalars: [0.3, -0.5, 0.0, -3.0, 0.6, 0.3, 4.7, 1.0, -0.7], cat: [-1.0, 1.5], upt: [0.3, 1.0], mass: 2e4, chance: 0.3, fixed: [0.0, 0.5, -3.0, 0.5], dir: -1.0, other: 2.0 },
    ];
    let Some(kind) = kinds.get(which) else { return };
    let lineage = w.life.new_lineage(brain::random(shape, &mut r, 0.1, kind.fixed, kind.dir, kind.other));
    for i in 0..w.grid.n() {
        let [t, depth, z] = w.life.env[i];
        if z <= 0.0 || depth > 0.5 || r.f64() >= kind.chance || (which > 0 && w.life.at[i].is_empty()) {
            continue;
        }
        // Genes that are off start below zero, so small changes keep them off.
        let mut genes = vec![-0.1f32; lay.len];
        genes[lay.cat..lay.min].fill(-1.0);
        genes[..body::NSCALAR].copy_from_slice(&kind.scalars);
        (genes[lay.cat + r_light], genes[lay.cat + burn]) = (kind.cat[0], kind.cat[1]);
        (genes[lay.min + liquid], genes[lay.min + rich]) = (0.3, 0.02);
        (genes[lay.upt + liquid], genes[lay.upt + rich]) = (kind.upt[0], kind.upt[1]);
        genes[lay.keep..].fill(0.9);
        genes[lay.keep + liquid] = 0.75;
        let mut store = vec![0.0; ns];
        store[liquid] = 0.7 * kind.mass / chem.subs[liquid].molar_mass;
        store[rich] = 0.3 * kind.mass / chem.subs[rich].molar_mass;
        let mut b = Body { cell: i as u32, lineage, mark: w.light[i], store, genes, ..Default::default() };
        b.e = b.heat_cap(&chem) * t;
        // Matter and energy brought in, booked like an edit.
        w.ledger.imposed_energy += b.e;
        for (k, &a) in b.store.iter().enumerate() {
            chem.add_elems(k, a, &mut w.ledger.imposed_elems);
            w.ledger.imposed_energy += a * chem.subs[k].hf;
        }
        (b.wet, b.vis) = (b.key_liquids(&chem), b.visible(&chem));
        let delay = 1 + r.below(b.interval() as usize) as u64;
        w.life.add(b, w.day, delay);
    }
}
