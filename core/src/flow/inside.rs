//! The ground of a cell as the shares inside it read it and as a split shares it (laws.md 0, `inside`): the liquid in
//! its pores, what stands over them and its soft cover, by Flow's state of the ground (`means`); a split's quarters take
//! the liquid of their pores and their soft cover by the shares of their own ground, and what stands at one level
//! (`share_ground`).

use super::{basin::{level_of, movable, share_out}, drain::liquid_heat_at, land::{layers, soft_at}, potential::{scratch_of, stated}, range, Scratch};
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
    soft: Vec<f64>,
}

fn wet(w: &World, s: &Scratch, cells: &[usize]) -> Wet {
    let (n, por) = (w.mesh.cells().n(), w.cfg.phys.porosity);
    let (tmin, tmax) = range(&s.state.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let at: Vec<u32> = cells.iter().map(|&i| i as u32).collect();
    let molten = |k: usize, i: usize| s.plateau(i).map_or(0.0, |(t, on)| w.molten(k, i, t, on));
    let liquid = |k: usize, i: usize| if s.state.tg[i] < s.sp[k].boil { molten(k, i) } else { 0.0 };
    let (held, vol) = movable(w, s, &liq, &at, tmin, (liquid, |i, mol| liquid_heat_at(s, &liq, s.state.tg[i], mol)));
    let (mut z, mut pores, mut soft) = (Vec::new(), Vec::new(), Vec::new());
    for &i in cells {
        let amt = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
        let (l, top, _) = layers(w, s, i, s.state.tg[i], amt, |k| molten(k, i), |k| w.melt(k, i));
        z.push(top);
        pores.push(por * l[4]);
        soft.push(l[0]);
    }
    Wet { liq, held, vol, z, pores, soft }
}

/// What each of `cells` holds as the shares inside it read it (`inside::Means`), by Flow's state of the ground: the
/// liquid in its pores (`movable`, as far as its pores hold), what its pores hold when full, what stands over them and
/// its soft cover (`land::layers`), as depths, m. None before Flow first built the state.
pub fn means(w: &World, cells: &[usize]) -> Option<Vec<Means>> {
    if !stated(w) {
        return None;
    }
    let (s, area) = (scratch_of(w), w.mesh.cells().area());
    let x = wet(w, &s, cells);
    let one = |c: usize| {
        let (a, pore) = (area[cells[c]], x.vol[c].min(x.pores[c]));
        Means { pore: pore / a, pores: x.pores[c] / a, stands: (x.vol[c] - pore) / a, snow: x.soft[c] / a }
    };
    Some((0..cells.len()).map(one).collect())
}

/// The quarters `cells` a split just made (`World::split`), flat copies of their parent by area, take the shares of
/// their own ground (`inside::Inside`, each quarter's part `Ground::quarters`): the liquid their pores hold by `soil`,
/// as far as each one's pores hold it, and the liquid that stood over them and what their pores cannot hold at one
/// level over their floors, as the basin's does (`level`; a floor is a quarter's solid surface less the room its pores
/// have left, which the liquid fills first); then their soft cover by `snow`. Each move takes its share of every liquid
/// with what is dissolved and its heat at the node's temperature (`drain::liquid_heat_at`), or of every soft solid with
/// its heat, by `share_out`, from the quarter that holds the most to each other. The quarters' phases are those of
/// their heat (`World::thawed`) before and after. No part of Flow is under way between steps: the store books the moves
/// (`store::apply`), and the potential energy they give up or take is the cell's relief's (`World::split`). None before
/// Flow first built the state of the ground.
pub(crate) fn share_ground(w: &mut World, cells: &[usize; 4], (soil, snow): ([f64; 4], [f64; 4])) {
    if !stated(w) {
        return;
    }
    let (ks, (a, b, mol, heat)) = {
        let (s, area) = (scratch_of(w), w.mesh.cells().area());
        let x = wet(w, &s, cells.as_slice());
        let pore: f64 = x.vol.iter().zip(&x.pores).map(|(v, p)| v.min(*p)).sum();
        let kept: Vec<f64> = (0..4).map(|c| (pore * soil[c]).min(x.pores[c])).collect();
        let stands = x.vol.iter().sum::<f64>() - kept.iter().sum::<f64>();
        let floors: Vec<(f64, f64)> = cells.iter().enumerate().map(|(c, &i)| (x.z[c] - (x.pores[c] - kept[c]) / area[i], area[i])).collect();
        let top = level_of(&mut floors.clone(), stands);
        let want: Vec<f64> = (0..4).map(|c| kept[c] + floors[c].1 * (top - floors[c].0).max(0.0)).collect();
        let at: Vec<u32> = cells.iter().map(|&i| i as u32).collect();
        (x.liq.iter().map(|&k| k as u16).collect::<Vec<u16>>(), share_out(&at, (&x.held, &x.vol), &want, x.liq.len()))
    };
    crate::store::apply(w, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &ks, mol: &mol, heat: &heat }], &[]);
    w.thawed(cells);
    let (ks, (a, b, mol, heat)) = {
        let s = scratch_of(w);
        let x = soft_rows(w, &s, cells.as_slice());
        let all: f64 = x.1.iter().sum();
        let want: Vec<f64> = snow.iter().map(|f| all * f).collect();
        let at: Vec<u32> = cells.iter().map(|&i| i as u32).collect();
        (x.2.iter().map(|&k| k as u16).collect::<Vec<u16>>(), share_out(&at, (&x.0, &x.1), &want, x.2.len()))
    };
    crate::store::apply(w, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &ks, mol: &mol, heat: &heat }], &[]);
    w.thawed(cells);
}

/// The soft cover of each of `cells` (`land::layers`): of each substance that may lie as one, the mol of it that is
/// solid where it is soft at its node's temperature (`soft_at`), then their heat, J, at that temperature (rows as
/// `movable` gives them); their volume, m³; and those substances.
fn soft_rows(w: &World, s: &Scratch, cells: &[usize]) -> (Vec<f64>, Vec<f64>, Vec<usize>) {
    let n = w.mesh.cells().n();
    let ks: Vec<usize> = s.state.vary.clone();
    let width = ks.len() + 1;
    let (mut held, mut vol) = (vec![0.0; cells.len() * width], vec![0.0; cells.len()]);
    for (c, &i) in cells.iter().enumerate() {
        let (t, on) = s.plateau(i).unwrap_or_else(|| w.thaw(i));
        let row = &mut held[c * width..(c + 1) * width];
        for (q, &k) in ks.iter().enumerate() {
            let p = &s.sp[k];
            if t < p.boil && soft_at(w, p, t) {
                row[q] = w.store.amounts(Class::Ground)[k * n + i] * (1.0 - w.molten(k, i, t, on));
                (row[ks.len()], vol[c]) = (row[ks.len()] + row[q] * p.cp * t, vol[c] + row[q] * p.v[0]);
            }
        }
    }
    (held, vol, ks)
}
