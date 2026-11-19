//! What a cell's ground holds of dead matter and of minerals, and how fast its dead frame rots there, read for the
//! shells' maps (laws.md 4): no law reads it.

use crate::World;

/// What the ground of a cell holds, kg/m²: its fresh dead matter (of the substances life's parts are made of), its slow
/// organic matter (what particles hold, `Substance::bound`), its minerals dissolved in its liquid and held on its
/// exchange sites (`dissolved`); and the pace its dead frame rots at, a year: the open world's catalyst as moist as it
/// is, times the rate constant at its temperature, in the share of it whose pores hold gas (laws.md 2.1, 2.3), with
/// all the gas it needs.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ground {
    pub litter: f64,
    pub slow: f64,
    pub minerals: [f64; 2],
    pub rot: f64,
}

/// What the ground of cell `i` of `w` holds (`Ground`), by Flow's last state of it.
pub fn ground(w: &World, i: usize) -> Ground {
    let (area, view) = (w.mesh.cells().area()[i], w.scratch.view());
    let kg = |k: usize| w.g(k, i) * w.chem.subs[k].molar_mass / area;
    let parts = crate::life::Parts::of(&w.chem);
    let litter = parts.map_or(0.0, |p| p.sub.iter().map(|&k| kg(k)).sum());
    let slow = (0..w.chem.ns()).filter(|&k| w.chem.subs[k].bound).map(kg).sum();
    let mineral: f64 = w.chem.minerals().iter().map(|m| kg(m.0)).sum();
    let dissolved = super::dissolved(w.soil[i].holds, view.liquid_depth(i) * area);
    let (t, moist) = (w.t_ground_last(i), view.moist(i, &w.cfg.phys));
    let aired = if view.standing(i).0 > 0.0 { 1.0 } else { 1.0 - w.pores(i).full(view.filled(i)) };
    let year = w.cfg.calendar().year as f64;
    let rot = parts.and_then(|p| w.chem.reactions.iter().find(|rx| rx.open > 1.0 && rx.inputs.iter().any(|q| q.0 as usize == p.sub[crate::life::WALL])));
    let rot = rot.map_or(0.0, |rx| crate::change::open_catalyst(rx.open, moist) * rx.arrhenius(t) * aired * year);
    Ground { litter, slow, minerals: [mineral * dissolved, mineral * (1.0 - dissolved)], rot }
}
