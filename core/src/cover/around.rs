//! What the cover of a cell reads of its world in a month (laws.md 4.5): one view, `Around`, the full world's
//! (`around`) or a year a nursery keeps; and what its leaves lose of the key liquid for the gas they take in (`thirst`).

use super::Parts;
use crate::chem::{Chemistry, Phase};
use crate::store::{Moves, Node};
use crate::units::MAX_SUBSTANCES;
use crate::{World, AIR};

/// What the cover of a cell reads of its world in a month: the full world's (`around`) or a year a nursery keeps
/// (`worldgen::life::nursery`). Its ground's temperature, K; of the key liquid in its ground, what is liquid, mol, and
/// how wet that leaves its ground (`wet`); the share of the light that reaches it under standing liquid; the cell's light
/// (`World::light`), J/m²; the air's factor for the light reaction (`Physics::gas_factor`, the least over its gas
/// inputs); the key liquid its leaves lose as vapour per mol of gas they take in (`thirst`); per substance what the
/// cell's share of its air node offers as a gas, mol (infinite for what is not a gas), and what its ground holds, mol;
/// and the tick that keys its seeds' draws (`World::month`).
#[derive(Clone, Debug)]
pub struct Around {
    pub t: f64,
    pub liquid: f64,
    pub moist: f64,
    pub reach: f64,
    pub light: f64,
    pub gas: f64,
    pub thirst: f64,
    pub air: [f64; MAX_SUBSTANCES],
    pub ground: [f64; MAX_SUBSTANCES],
    pub tick: u64,
}

/// What the cover of cell `i` reads of the world now, through the moves `moves` opened on it.
pub(crate) fn around(w: &World, moves: &Moves, p: &Parts, i: usize) -> Around {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let (j, t) = (w.mesh.up(i, 0, AIR), w.thaw_last(i).0);
    let (liquid, moist) = super::wet(w, p, i, moves.amount(w, Node::ground(i), p.liquid));
    let reach = (-w.life.env.get(i).map_or(0.0, |e| e[1]) / ph.light_depth).exp();
    let seen = moves.seen(w, j);
    let all: f64 = (0..chem.ns()).map(&seen).sum();
    let rx = &chem.reactions[p.light];
    let gas = rx.inputs.iter().filter(|q| chem.phase(q.0 as usize, t) == Phase::Gas).map(|q| ph.gas_factor(seen(q.0 as usize) / all.max(1e-300))).fold(1.0, f64::min);
    // Its leaves read the air near its ground as the ground's own evaporation does (`World::near_ground`).
    let thirst = rx.inputs.iter().find(|q| chem.phase(q.0 as usize, t) == Phase::Gas).and_then(|q| {
        let gas = q.0 as usize;
        Some(thirst(chem, ph, (p.liquid, gas), t, [w.near_ground(i, p.liquid)?, w.near_ground(i, gas)?]))
    });
    let air = std::array::from_fn(|kk| if kk < chem.ns() && chem.phase(kk, t) == Phase::Gas { seen(kk) } else { f64::INFINITY });
    let ground = std::array::from_fn(|x| if x < chem.ns() { moves.amount(w, Node::ground(i), x) } else { 0.0 });
    Around { t, liquid, moist, reach, light: w.light[i], gas, thirst: thirst.unwrap_or(0.0), air, ground, tick: w.month() }
}

/// The key liquid `liquid` a leaf at `t` K loses as vapour per mol of the gas `gas` it takes in, mol, where the air near
/// it holds the partial pressures `[p_liquid, p_gas]` of them, Pa: its pores pass both, the vapour out down its deficit
/// (saturation at the leaf, `Chemistry::p_sat`, less the air's), the gas in down its drawdown inside the leaf
/// (`Physics::drawdown` of the air's), each at its diffusivity, whose ratio is the square root of the ratio of their
/// molar masses (Graham's law): the transpiration ratio of Cowan and Farquhar 1977 and Farquhar and Sharkey 1982.
pub fn thirst(chem: &Chemistry, ph: &crate::Physics, (liquid, gas): (usize, usize), t: f64, [p_liquid, p_gas]: [f64; 2]) -> f64 {
    let deficit = (chem.p_sat(liquid, t) - p_liquid).max(0.0);
    let diffusivity = (chem.subs[gas].molar_mass / chem.subs[liquid].molar_mass).sqrt();
    diffusivity * deficit / (ph.drawdown * p_gas).max(1e-300)
}

