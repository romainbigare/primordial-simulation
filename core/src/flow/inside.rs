//! The ground of a cell as the shares inside it read it and as a split shares it (laws.md 0, `inside`): the liquid in
//! its pores and what stands over them, by Flow's state of the ground (`means`); a split's quarters take the liquid of
//! their pores by the shares of their own ground, and what stands at one level (`share_ground`).

use super::{basin::{movable, share_out}, curve::{at_scale, of_cell, level_over, Curve}, drain::liquid_heat_at, land::layers, potential::{scratch_of, stated}, range, Scratch};
use crate::inside::Means;
use crate::store::{Class, LinkFlux};
use crate::World;

/// The liquids that may move in the ground nodes, the volume of what may move of them in each of `cells` and each one's
/// solid surface, m, and pore space, m³ (`movable`, `land::layers`), as `share_ground` and `means` read them, with the
/// rows `movable` gives (`nl` liquids, then the heat).
struct Wet {
    liq: Vec<usize>,
    held: Vec<f64>,
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
    let (held, vol) = movable(w, s, &liq, &at, tmin, (liquid, |i, mol| liquid_heat_at(s, &liq, s.state.tg[i], mol)));
    let (mut z, mut pores) = (Vec::new(), Vec::new());
    for &i in cells {
        let amt = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
        let (l, top, _) = layers(w, s, (i, 1.0, 1.0), s.state.tg[i], amt, |k| molten(k, i), |k| w.melt(k, i));
        z.push(top);
        pores.push((por * l[4] - s.dry(i)).max(0.0));
    }
    Wet { liq, held, vol, z, pores }
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

/// The quarters `cells` a split just made (`World::split`), flat copies of their parent by area, take the shares of
/// their own ground (`inside::Inside`, each quarter's part `Ground::quarters`): the liquid their pores hold by `soil`,
/// as far as each one's pores hold it, and the liquid that stood over them and what their pores cannot hold at one
/// level over their own ground, as the basin's does (`level`; each quarter's curve at the terrain's `scale` the split
/// stands, `curve::at_scale`, laid on its solid surface less the room its pores have left, which the liquid fills
/// first). Each move takes its share of every liquid with what is dissolved and its
/// heat at the node's temperature (`drain::liquid_heat_at`), by `share_out`, from the quarter that holds the most to
/// each other. The quarters' phases are those of their heat (`World::thawed`) after. No part of Flow is under way
/// between steps: the store books the moves (`store::apply`), and the potential energy they give up or take is the
/// cell's relief's (`World::split`). None before Flow first built the state of the ground.
pub(crate) fn share_ground(w: &mut World, cells: &[usize; 4], soil: [f64; 4], scale: f64) {
    if !stated(w) {
        return;
    }
    let (ks, (a, b, mol, heat)) = {
        let (s, area) = (scratch_of(w), w.mesh.cells().area());
        let x = wet(w, &s, cells.as_slice());
        let pore: f64 = x.vol.iter().zip(&x.pores).map(|(v, p)| v.min(*p)).sum();
        let kept: Vec<f64> = (0..4).map(|c| (pore * soil[c]).min(x.pores[c])).collect();
        let stands = x.vol.iter().sum::<f64>() - kept.iter().sum::<f64>();
        let curves: Vec<Curve> = cells.iter().map(|&i| at_scale(w, i, scale)).collect();
        let floor: Vec<f64> = cells.iter().enumerate().map(|(c, &i)| x.z[c] - (x.pores[c] - kept[c]) / area[i]).collect();
        let top = level_over(&curves.iter().zip(&floor).map(|(c, &z)| (c, z)).collect::<Vec<_>>(), stands);
        let want: Vec<f64> = (0..4).map(|c| kept[c] + curves[c].volume(top - floor[c])).collect();
        let at: Vec<u32> = cells.iter().map(|&i| i as u32).collect();
        (x.liq.iter().map(|&k| k as u16).collect::<Vec<u16>>(), share_out(&at, (&x.held, &x.vol), &want, x.liq.len()))
    };
    crate::store::apply(w, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &ks, mol: &mol, heat: &heat }], &[]);
    w.thawed(cells);
}
