//! The low basin (laws.md 1.2): the cells joined to the lowest cell under the level its liquid stands at hold one body of
//! liquid with one level. `find` names them at each full build of the ground's state; `level` shares the basin's liquid
//! by that level once a month, after the runoff has reached it; `stand_level` stands the liquid of a split's quarters
//! at one level the same way (`share_out`).

use super::{air::blocks, drain::{liquid_heat, liquid_heat_at}, land::{cells_state, layers, stand, wet, NONE}, liquid::book, potential::apply, Scratch};
use crate::{store::{Class, LinkFlux}, World, AIR};
use std::collections::BTreeMap;

/// The basin: whether each cell is in it, and its cells in order.
#[derive(Clone, Debug, Default)]
pub(super) struct Basin {
    pub(super) is: Vec<bool>,
    pub(super) cells: Vec<u32>,
}

/// The basin now: the lowest cell and every cell joined to it through cells whose solid surface lies under the level its
/// liquid stands at (`land::stand`); none where the lowest cell's liquid does not stand.
pub(super) fn find(w: &World, s: &Scratch) -> Basin {
    let cells = w.mesh.cells();
    let n = cells.n();
    let mut basin = Basin { is: vec![false; n], cells: Vec::new() };
    let Some(lowest) = (0..n).min_by(|&a, &b| s.z[a].total_cmp(&s.z[b])) else { return basin };
    let top = stand(s, lowest);
    if top <= s.z[lowest] {
        return basin;
    }
    let (mut todo, is) = (vec![lowest], &mut basin.is);
    is[lowest] = true;
    while let Some(i) = todo.pop() {
        for j in cells.around(i).1.iter().map(|&j| j as usize) {
            if !is[j] && s.z[j] < top {
                is[j] = true;
                todo.push(j);
            }
        }
    }
    basin.cells = (0..n as u32).filter(|&i| basin.is[i as usize]).collect();
    basin
}

/// The level at which `vol` m³ fill floors of heights and areas `floors` (m, m²), sorted here: Σ area × (level -
/// floor) over the floors under it is `vol`, m.
fn level_of(floors: &mut [(f64, f64)], vol: f64) -> f64 {
    floors.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
    let (mut area, mut volume_under) = (0.0, 0.0);
    for (k, &(z, a)) in floors.iter().enumerate() {
        (area, volume_under) = (area + a, volume_under + a * z);
        let level = (vol + volume_under) / area;
        if floors.get(k + 1).is_none_or(|next| level <= next.0) {
            return level;
        }
    }
    f64::NAN
}

/// The basin's liquid of the liquids `liq` (what boils below the coldest node, `tmin`, is a dissolved gas) stands at one
/// level, found from all of it and the floors of its cells (each cell's solid surface less its pore space, which the
/// liquid fills first): that liquid body has one level (its slope under wind and rotation is not modelled). Each cell
/// that holds more than its depth under that level gives the share of its liquid over it, with that share of its heat
/// as it leaves (`drain::liquid_heat`); mixed, it goes to the cells that hold less, by what each lacks (`share_out`),
/// with the potential energy of what moves (`potential`).
pub(super) fn level(w: &mut World, s: &mut Scratch, liq: &[usize], tmin: f64) {
    let basin = std::mem::take(&mut s.basin);
    if basin.cells.is_empty() {
        s.basin = basin;
        return;
    }
    let area = w.mesh.cells().area();
    let (held, vol) = movable(w, s, liq, &basin.cells, tmin, (|k, i| wet(s, k, i), |i, mol| liquid_heat(w, s, liq, i, mol)));
    let floors: Vec<(f64, f64)> = basin.cells.iter().map(|&i| (s.z[i as usize] - s.pd[i as usize], area[i as usize])).collect();
    let top = level_of(&mut floors.clone(), vol.iter().sum());
    let want: Vec<f64> = basin.cells.iter().zip(&floors).map(|(&i, f)| area[i as usize] * (top - f.0).max(0.0)).collect();
    let (a, b, mol, heat) = share_out(&basin.cells, (&held, &vol), &want, liq.len());
    let ks: Vec<u16> = liq.iter().map(|&k| k as u16).collect();
    s.basin = basin;
    book(w, s, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &ks, mol: &mol, heat: &heat }], None);
    // The basin's cells, which the runoff and the level moved, find their state again before their heat mixes, and
    // after it.
    let cells = std::mem::take(&mut s.basin.cells);
    cells_state(w, s, &cells);
    s.basin.cells = cells;
    let mixed = mix_heat(w, s);
    cells_state(w, s, &mixed);
}

/// The liquid that stands over the pores of the quarters `cells` a split just made (`World::split`) stands at one level
/// over their solid surfaces, as the basin's does (`level`): each keeps what its pores hold, as its parent's were held,
/// and what stood over them fills the quarters from the lowest up (`level_of`), with its heat as it leaves at the
/// node's temperature (`drain::liquid_heat_at`), by `share_out`. The quarters' phases are those of their heat
/// (`World::thawed`) before and after. No part of Flow is under way between steps: the store books the moves
/// (`store::apply`), and the potential energy the liquid gives up is the cell's relief's (`World::split`). None before
/// Flow first built the state of the ground.
pub(crate) fn stand_level(w: &mut World, cells: &[usize]) {
    if !super::potential::stated(w) {
        return;
    }
    let (ks, (a, b, mol, heat)) = {
        let s = super::potential::scratch_of(w);
        let (n, area, por) = (w.mesh.cells().n(), w.mesh.cells().area(), w.cfg.phys.porosity);
        let (tmin, tmax) = super::range(&s.state.tg);
        let liq = s.liquids(&w.chem, (tmin, tmax), true);
        let at: Vec<u32> = cells.iter().map(|&i| i as u32).collect();
        let molten = |k: usize, i: usize| s.plateau(i).map_or(0.0, |(t, on)| w.molten(k, i, t, on));
        let wet = |k: usize, i: usize| if s.state.tg[i] < s.sp[k].boil { molten(k, i) } else { 0.0 };
        let (held, vol) = movable(w, &s, &liq, &at, tmin, (wet, |i, mol| liquid_heat_at(&s, &liq, s.state.tg[i], mol)));
        // Each quarter's solid surface, m, and the volume its pores hold, m³, as Flow's state finds them (`land::layers`).
        let ground: Vec<(f64, f64)> = (cells.iter().zip(&vol))
            .map(|(&i, &v)| {
                let amt = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
                let (l, z, _) = layers(w, &s, i, s.state.tg[i], amt, |k| molten(k, i), |k| w.melt(k, i));
                (z, v.min(por * l[4]))
            })
            .collect();
        let stands: f64 = vol.iter().zip(&ground).map(|(v, g)| v - g.1).sum();
        let mut floors: Vec<(f64, f64)> = cells.iter().zip(&ground).map(|(&i, g)| (g.0, area[i])).collect();
        let top = level_of(&mut floors, stands);
        let want: Vec<f64> = cells.iter().zip(&ground).map(|(&i, g)| g.1 + area[i] * (top - g.0).max(0.0)).collect();
        (liq.iter().map(|&k| k as u16).collect::<Vec<u16>>(), share_out(&at, (&held, &vol), &want, liq.len()))
    };
    crate::store::apply(w, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &ks, mol: &mol, heat: &heat }], &[]);
    w.thawed(cells);
}

/// What may move of the liquids `liq` in each cell of `cells` (`[c * (liq.len() + 1) + q]`): all of one that boils below
/// the coldest node, `tmin` (a gas dissolved in the liquid, which takes no room), else its liquid share `wet(k, i)`,
/// mol, then their heat as they leave, `heat(i, mol)`, J; and their volume, m³.
fn movable(w: &World, s: &Scratch, liq: &[usize], cells: &[u32], tmin: f64, (wet, heat): (impl Fn(usize, usize) -> f64, impl Fn(usize, &[f64]) -> f64)) -> (Vec<f64>, Vec<f64>) {
    let (n, nl, m) = (w.mesh.cells().n(), liq.len(), cells.len());
    let width = nl + 1;
    let (mut held, mut vol) = (vec![0.0; m * width], vec![0.0; m]);
    let gas: Vec<bool> = liq.iter().map(|&k| s.sp[k].boil <= tmin).collect();
    for (c, &i) in cells.iter().enumerate() {
        let i = i as usize;
        let row = &mut held[c * width..(c + 1) * width];
        for (q, &k) in liq.iter().enumerate() {
            row[q] = w.store.amounts(Class::Ground)[k * n + i] * if gas[q] { 1.0 } else { wet(k, i) };
            vol[c] += if gas[q] { 0.0 } else { row[q] * s.sp[k].v[1] };
        }
        row[nl] = heat(i, &row[..nl]);
    }
    (held, vol)
}

/// Cells `cells`, of the liquids that may move `held` (`movable`: `nl` liquids, then the heat) and their volumes `vol`,
/// m³, come to hold `want` m³: each cell that holds more gives the share of its liquid over it, with that share of its
/// heat; mixed, it goes to the cells that hold less, by what each lacks. As a link flux from the cell that wants the
/// most to each other one: its ends, the mol of each liquid (`[q * links + l]`) and the heat, J, of each link.
fn share_out(cells: &[u32], (held, vol): (&[f64], &[f64]), want: &[f64], nl: usize) -> (Vec<u32>, Vec<u32>, Vec<f64>, Vec<f64>) {
    let (m, width) = (cells.len(), nl + 1);
    // What the cells over the level give, mixed (as a row of `held`), and the volume the cells under it lack, m³.
    let (mut spare, mut lack) = (vec![0.0; width], 0.0);
    let mut change = vec![0.0; m * width];
    for c in 0..m {
        if vol[c] > want[c] {
            let gives = (vol[c] - want[c]) / vol[c];
            for (q, x) in spare.iter_mut().enumerate() {
                change[c * width + q] = -held[c * width + q] * gives;
                *x -= change[c * width + q];
            }
        } else {
            lack += want[c] - vol[c];
        }
    }
    for c in (0..m).filter(|&c| want[c] > vol[c]) {
        let gets = (want[c] - vol[c]) / lack;
        (0..width).for_each(|q| change[c * width + q] = spare[q] * gets);
    }
    // From the cell that holds the most to each other one.
    let hub = (0..m).max_by(|&a, &b| want[a].total_cmp(&want[b]).then(b.cmp(&a))).unwrap_or(0);
    let others: Vec<usize> = (0..m).filter(|&c| c != hub).collect();
    let links = others.len();
    let (mut mol, mut heat) = (vec![0.0; nl * links], vec![0.0; links]);
    for (l, &c) in others.iter().enumerate() {
        (0..nl).for_each(|q| mol[q * links + l] = change[c * width + q]);
        heat[l] = change[c * width + nl];
    }
    (vec![cells[hub]; links], others.iter().map(|&c| cells[c]).collect(), mol, heat)
}

/// Does cell `i` mix as one with the low basin under its air block (`mix_heat`) and face the air as one surface with
/// the basin under its air node (`air::surf`)? A cell of the low basin off its melting plateaus, with no soft cover and
/// no cover of bodies.
pub(super) fn one_surface(w: &World, s: &Scratch, i: usize) -> bool {
    s.basin.is[i] && s.state.pk[i] == NONE && s.cover[i][2] <= 0.0 && w.canopy[i] <= 0.0
}

/// The low basin under each air block mixes its heat in a month: its cells that mix as one (`one_surface`), with the same
/// phases, each take the block's temperature, the heat of their sum over their summed heat capacity (apart from the
/// fusion enthalpy each holds), as one heat flux from the cell of the most heat capacity to each other. Returns the cells
/// that mixed.
fn mix_heat(w: &mut World, s: &mut Scratch) -> Vec<u32> {
    let mut block = vec![0; w.mesh.level(AIR).n()];
    blocks(w).iter().enumerate().for_each(|(b, nodes)| nodes.iter().for_each(|&j| block[j as usize] = b));
    let parent = w.mesh.cells().parent();
    let mut by: BTreeMap<(usize, u64), Vec<usize>> = BTreeMap::new();
    for &i in s.basin.cells.iter().filter(|&&i| one_surface(w, s, i as usize)) {
        by.entry((block[parent[i as usize] as usize], s.latent[i as usize].molten)).or_default().push(i as usize);
    }
    let (mut a, mut b, mut heat, mut mixed) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for cells in by.values().filter(|c| c.len() > 1) {
        mixed.extend(cells.iter().map(|&i| i as u32));
        let free = |i: usize| w.store.heat(Class::Ground)[i] - s.latent[i].held;
        let t = cells.iter().map(|&i| free(i)).sum::<f64>() / cells.iter().map(|&i| s.cap[i]).sum::<f64>();
        let hub = cells.iter().copied().max_by(|&p, &q| s.cap[p].total_cmp(&s.cap[q]).then(q.cmp(&p))).unwrap_or(cells[0]);
        for &i in cells.iter().filter(|&&i| i != hub) {
            a.push(hub as u32);
            b.push(i as u32);
            heat.push(s.cap[i] * t - free(i));
        }
    }
    apply(w, s, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &[], mol: &[], heat: &heat }], &[]);
    mixed.sort_unstable();
    mixed
}
