//! The ground of a cell as the shares inside it read it (laws.md 0, `inside`): the liquid in its pores and what stands
//! over them, by Flow's state of the ground (`means`, `levels`).

use super::{basin::movable, curve::of_cell, drain::liquid_heat_at, land::layers, potential::{scratch_of, stated}, range, Scratch};
use crate::inside::Means;
use crate::store::Class;
use crate::World;

/// The volume of the liquid that may move in each of `cells`, m³, and each one's
/// solid surface, m, and pore space, m³ (`movable`, `land::layers`), as `means` and `levels` read them.
struct Wet {
    vol: Vec<f64>,
    z: Vec<f64>,
    pores: Vec<f64>,
}

fn wet(w: &World, s: &Scratch, cells: &[usize]) -> Wet {
    let (n, por) = (w.mesh.cells().n(), w.cfg.phys.porosity);
    let (tmin, tmax) = range(&s.state.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let at: Vec<u32> = cells.iter().map(|&i| i as u32).collect();
    let molten = |k: usize, i: usize| s.plateau(i).map_or(0.0, |(t, on)| w.molten(k, i, t, on));
    let liquid = |k: usize, i: usize| if s.state.tg[i] < s.sp[k].boil { molten(k, i) } else { 0.0 };
    let (_, vol) = movable(w, s, &liq, &at, tmin, (liquid, |i, mol| liquid_heat_at(s, &liq, s.state.tg[i], mol)));
    let (mut z, mut pores) = (Vec::new(), Vec::new());
    for &i in cells {
        let amt = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
        let (l, top, _) = layers(w, s, (i, 1.0, 1.0), s.state.tg[i], amt, |k| molten(k, i), |k| w.melt(k, i));
        z.push(top);
        pores.push((por * l[4] - s.dry(i)).max(0.0));
    }
    Wet { vol, z, pores }
}

/// What each of `cells` holds as the shares inside it read it (`inside::Means`), by Flow's state of the ground: the
/// liquid in its pores (`movable`, as far as its pores hold), what its pores hold when full and what stands over them
/// (`land::layers`), as depths, m. None before Flow first built the state.
pub fn means(w: &World, cells: &[usize]) -> Option<Vec<Means>> {
    if !stated(w) {
        return None;
    }
    let (s, area) = (scratch_of(w), w.mesh.cells().area());
    let x = wet(w, &s, cells);
    let one = |c: usize| {
        let (a, pore) = (area[cells[c]], x.vol[c].min(x.pores[c]));
        Means { pore: pore / a, pores: x.pores[c] / a, stands: (x.vol[c] - pore) / a }
    };
    Some((0..cells.len()).map(one).collect())
}

/// The height of the lowest ground of cell `i`, m: its solid surface and its own ground's lowest departure from it
/// (`curve::Curve`), where its liquid leaves it.
pub fn lowest(w: &World, i: usize) -> f64 {
    w.surface_z(i) + of_cell(w, &w.scratch, i).low()
}

/// The volume over the own ground of cell `i` (`curve::Curve`, on its solid surface) under the level `level`, m³: what
/// liquid that stands over its pores at that level holds there.
pub fn volume_under(w: &World, i: usize, level: f64) -> f64 {
    of_cell(w, &w.scratch, i).volume(level - w.surface_z(i))
}

/// Of each of `cells`: its lowest ground and the level the liquid standing over its pores reaches over its own ground
/// (`curve::Curve`; its lowest ground where none stands), m, by Flow's state of the ground. None before Flow first built
/// the state.
pub fn levels(w: &World, cells: &[usize]) -> Option<Vec<[f64; 2]>> {
    if !stated(w) {
        return None;
    }
    let s = scratch_of(w);
    let x = wet(w, &s, cells);
    let one = |c: usize| {
        let (curve, stands) = (of_cell(w, &w.scratch, cells[c]), (x.vol[c] - x.vol[c].min(x.pores[c])).max(0.0));
        [x.z[c] + curve.low(), x.z[c] + curve.level(stands)]
    };
    Some((0..cells.len()).map(one).collect())
}
