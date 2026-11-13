//! Growth of a slot of cover in its month (laws.md 3): it builds body matter and walls from its fuel and the minerals
//! of its ground, fixing what it lacks if it can, and sends seeds out (`build`); and it builds walls alone, which pay no
//! upkeep, up to those its plants hold (`frame`).

use super::grow::{Books, Slot};
use super::{dec, Cover, Parts, FIX, FIX_OFF, FUEL, MATTER, NP, SEED, SLOTS, STORE, WALL, WALLS};
use crate::life::WET_TISSUE;
use crate::chem::Reaction;
use crate::rng::{Purpose, Rng};
use crate::units::MAX_SUBSTANCES;
use crate::World;

/// The units of `rx` per unit of substance `k` among its inputs (none if it takes none).
pub(super) fn per_input(rx: &Reaction, k: usize) -> Option<f64> {
    rx.inputs.iter().find(|q| q.0 as usize == k).map(|q| q.1 as f64)
}

/// The energy one kg of living cover costs to build, J: per kg of its body matter (`Cover::living`) the fuel its
/// conversion (`Chemistry::build`) takes, as burning it would free it, and the work of building, `c_copy`.
pub(super) fn living_cost(chem: &crate::chem::Chemistry, ph: &crate::Physics, p: &Parts) -> f64 {
    let Some(bl) = chem.build.as_ref() else { return ph.c_copy * (1.0 - WET_TISSUE) };
    let burn = &chem.reactions[p.burn];
    let fuel = per_input(bl, p.sub[FUEL]).unwrap_or(0.0) / p.mm(chem)[MATTER] / bl.outputs[0].1 as f64;
    (fuel * -chem.dh(p.burn) / per_input(burn, p.sub[FUEL]).unwrap_or(1.0) + ph.c_copy) * (1.0 - WET_TISSUE)
}

/// Slot `x` builds what its fuel over its reserve (`STORE`) and the minerals of its ground (`ground`, mol, shared out
/// as slots build) allow, by the conversions of growth (`Chemistry::build`, `builds`) at its share of walls (`WALLS`),
/// paying `c_copy` per kg by burning fuel; a fixer (`FIX`) makes the mineral it lacks, at its
/// capacity, paid by the fixing reaction's fuel. The share `SEED` of what it builds leaves as seeds, in `seed::LOTS` lots
/// that land where the dispersal kernel draws (`life::lot`, `seed::send`), with a generation's mutation; all of it where
/// one more kg would not pay its upkeep (`pays` false: its light at the bottom of its layer is under its compensation
/// point), so a cover stops thickening where its lowest leaves would not pay for themselves (Monsi and Saeki 1953;
/// Hirose 2005).
#[allow(clippy::too_many_arguments)]
pub(super) fn build(w: &World, c: &mut Cover, p: &Parts, x: usize, (s, pays): (&Slot, bool), (i, tick, t, dt): (usize, u64, f64, f64), ground: &mut [f64; MAX_SUBSTANCES], b: &mut Books) {
    let (chem, ph, g) = (&w.chem, &w.cfg.phys, c.genes[x]);
    let (Some(bl), mm) = (chem.build.as_ref(), p.mm(chem)) else { return };
    let Some(wall) = chem.builds.iter().find(|r| r.outputs[0].0 as usize == p.sub[WALL]) else { return };
    // Per kg built: the units of each conversion, and what they take; a small plant builds few walls (`stand::walled`).
    let walls = dec(g[WALLS]) * super::stand::walled(ph, c.plant(&mm, x));
    let fm = 1.0 / (1.0 + walls);
    let units = [(bl, fm / mm[MATTER] / bl.outputs[0].1 as f64), (wall, walls * fm / mm[WALL] / wall.outputs[0].1 as f64)];
    let mut need = [0.0; MAX_SUBSTANCES];
    units.iter().for_each(|(rx, u)| rx.inputs.iter().for_each(|&(k, n)| need[k as usize] += n as f64 * u));
    let burn = &chem.reactions[p.burn];
    let (fuel_burn, burn_e) = (per_input(burn, p.sub[FUEL]).unwrap_or(1.0), -chem.dh(p.burn));
    let fpk = need[p.sub[FUEL]] + ph.c_copy / burn_e * fuel_burn;
    let surplus = (c.mol[x][FUEL] - dec(g[STORE]) * s.built / mm[FUEL]).max(0.0);
    // The most each limit allows, kg: the fuel, and each mineral from the ground; a fixer then makes what the mineral it
    // fixes lacks, its fuel counted.
    let minerals = || (0..chem.ns()).filter(|&k| need[k] > 0.0 && k != p.sub[FUEL]);
    let by = |k: usize| ground[k] / need[k];
    // (Fixing takes its gas from the cell's share of the air node, as far as that still offers it.)
    let fixed = p.fix.filter(|_| g[FIX] > FIX_OFF).and_then(|r| {
        let rx = &chem.reactions[r];
        let &(m, out) = rx.outputs.iter().find(|o| need[o.0 as usize] > 0.0)?;
        let most = (dec(g[FIX]) * s.living * rx.arrhenius(t.max(1.0)) / rx.arrhenius(ph.t_cat) * s.act * dt).min(b.room(p, rx));
        Some((rx, m as usize, out as f64, per_input(rx, p.sub[FUEL])?, most))
    });
    let others = minerals().filter(|&k| fixed.is_none_or(|f| f.1 != k)).map(by).fold(surplus / fpk, f64::min);
    let mut kg = minerals().map(by).fold(others, f64::min);
    let mut y = 0.0;
    if let Some((rx, m, out, fuel, most)) = fixed {
        let r = fuel / out;
        let with = others.min((surplus + r * ground[m]) / (fpk + r * need[m])).min((ground[m] + out * most) / need[m]);
        if with > kg {
            (kg, y) = (with, (with * need[m] - ground[m]).max(0.0) / out);
        }
        b.run(chem, p, &mut c.mol[x], rx, y);
    }
    // Building's work burns fuel with the gas the cell's share of its air node still offers.
    kg = kg.min(b.room(p, burn) * burn_e / ph.c_copy.max(1e-300));
    if kg <= 0.0 {
        return;
    }
    minerals().for_each(|k| ground[k] = (ground[k] - kg * need[k]).max(0.0));
    if let Some((_, m, out, ..)) = fixed {
        ground[m] += y * out - (y * out).min(kg * need[m]);
    }
    b.run(chem, p, &mut c.mol[x], burn, kg * ph.c_copy / burn_e);
    // What it builds goes into the slot; the share `SEED` of it leaves as seeds.
    let before = c.mol[x];
    units.iter().for_each(|(rx, u)| b.run(chem, p, &mut c.mol[x], rx, kg * u));
    // Each seed carries its store of fuel at its `STORE` gene, from the slot's fuel: a slot short of it makes fewer seeds.
    let grew: [f64; NP] = std::array::from_fn(|q| if q == FUEL { 0.0 } else { (c.mol[x][q] - before[q]).max(0.0) });
    let new = super::built_kg(&mm, &grew);
    let store = dec(g[STORE]) / mm[FUEL];
    let seed = (if pays { g[SEED].clamp(0.0, 1.0) as f64 } else { 1.0 }).min(c.mol[x][FUEL].max(0.0) / (store * new).max(1e-300));
    if seed > 0.0 {
        // Seeds are body matter first (an embryo and its store, little wall), then walls.
        let (mut mol, mut left) = ([0.0; NP], seed * new);
        for q in [MATTER, WALL] {
            mol[q] = (left / mm[q]).min(grew[q]).max(0.0);
            left -= mol[q] * mm[q];
            c.mol[x][q] -= mol[q];
        }
        mol[FUEL] = (store * super::built_kg(&mm, &mol)).min(c.mol[x][FUEL].max(0.0));
        c.mol[x][FUEL] -= mol[FUEL];
        let heat = (0..NP).map(|q| mol[q] * chem.subs[p.sub[q]].cp * t).sum::<f64>();
        b.heat -= heat;
        let mut r = Rng::new(w.seed, tick, (i * SLOTS + x) as u64, Purpose::Cover);
        let mut lands: Vec<(usize, f64)> = Vec::new();
        for _ in 0..super::seed::LOTS {
            let Some(j) = crate::life::lot(w, i, &mut r).filter(|&j| j != i) else { continue };
            match lands.iter_mut().find(|l| l.0 == j) {
                Some(l) => l.1 += 1.0 / super::seed::LOTS as f64,
                None => lands.push((j, 1.0 / super::seed::LOTS as f64)),
            }
        }
        let genes = super::mutate(&g, ph.cover_mut, &mut r);
        b.budget[super::budget::SEEDS] -= super::budget::held(&p.content(chem), &mol);
        super::seed::send(c, x, (mol, heat), genes, (i, lands));
    }
}


/// The walls slot `x` lacks of those its plants hold (`WALLS` × `stand::walled` of their mass, per kg of matter), kg.
pub(super) fn frame_short(w: &World, c: &Cover, p: &Parts, x: usize) -> f64 {
    let mm = p.mm(&w.chem);
    let walls = dec(c.genes[x][WALLS]) * super::stand::walled(&w.cfg.phys, c.plant(&mm, x));
    (walls * c.mol[x][MATTER] * mm[MATTER] - c.walls(&mm, x)).max(0.0)
}

/// Slot `x` builds walls alone of its fuel over its reserve (`STORE`), up to those its plants hold (`frame_short`),
/// paying `c_copy` per kg by burning fuel with the gas the cell's share of its air node still offers. Walls are built of
/// the fuel alone and pay no upkeep, so a plant short of minerals, or whose leaves are all its light pays for, still
/// grows its frame, and with it its height, from the fuel it makes; it then thins out (`stand::most_plants`).
pub(super) fn frame(w: &World, c: &mut Cover, p: &Parts, x: usize, b: &mut Books) {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let Some(wall) = chem.builds.iter().find(|r| r.outputs[0].0 as usize == p.sub[WALL]) else { return };
    let (mm, short) = (p.mm(chem), frame_short(w, c, p, x));
    let burn = &chem.reactions[p.burn];
    let (fuel_burn, burn_e) = (per_input(burn, p.sub[FUEL]).unwrap_or(1.0), -chem.dh(p.burn));
    let units = 1.0 / mm[WALL] / wall.outputs[0].1 as f64;
    let fpk = per_input(wall, p.sub[FUEL]).unwrap_or(0.0) * units + ph.c_copy / burn_e * fuel_burn;
    let surplus = (c.mol[x][FUEL] - dec(c.genes[x][STORE]) * c.built(&mm, x) / mm[FUEL]).max(0.0);
    let kg = short.min(surplus / fpk).min(b.room(p, burn) * burn_e / ph.c_copy.max(1e-300));
    if kg > 0.0 {
        b.run(chem, p, &mut c.mol[x], burn, kg * ph.c_copy / burn_e);
        b.run(chem, p, &mut c.mol[x], wall, kg * units);
    }
}

