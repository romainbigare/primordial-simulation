//! The low basin (laws.md 1.2): the cells joined to the lowest cell under the level its liquid stands at hold one body of
//! liquid with one level. `find` names them at each full build of the ground's state; `level` shares the basin's liquid
//! by that level once a month, after the runoff has reached it.

use super::{air::blocks, drain::liquid_heat, land::{stand, wet, NONE}, liquid::book, potential::apply, Scratch};
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
/// as it leaves (`drain::liquid_heat`); mixed, it goes to the cells that hold less, by what each lacks. Booked as a link
/// from the cell that holds the most to each other cell of the basin, with the potential energy of what moves
/// (`potential`).
pub(super) fn level(w: &mut World, s: &mut Scratch, liq: &[usize], tmin: f64) {
    let basin = std::mem::take(&mut s.basin);
    if basin.cells.is_empty() {
        s.basin = basin;
        return;
    }
    let (cells, nl, m) = (w.mesh.cells(), liq.len(), basin.cells.len());
    let (n, area) = (cells.n(), cells.area());
    // Per basin cell `c`: its liquid, mol (`[c * (nl + 1) + q]`, then its heat as it leaves, J), its volume, m³, and its
    // floor, m.
    let width = nl + 1;
    let (mut held, mut vol, mut floors) = (vec![0.0; m * width], vec![0.0; m], Vec::with_capacity(m));
    let gas: Vec<bool> = liq.iter().map(|&k| s.sp[k].boil <= tmin).collect();
    for (c, &i) in basin.cells.iter().enumerate() {
        let i = i as usize;
        let row = &mut held[c * width..(c + 1) * width];
        for (q, &k) in liq.iter().enumerate() {
            row[q] = w.store.amounts(Class::Ground)[k * n + i] * if gas[q] { 1.0 } else { wet(s, k, i) };
            vol[c] += if gas[q] { 0.0 } else { row[q] * s.sp[k].v[1] };
        }
        row[nl] = liquid_heat(w, s, liq, i, &row[..nl]);
        floors.push((s.z[i] - s.pd[i], area[i]));
    }
    let top = level_of(&mut floors.clone(), vol.iter().sum());
    let want: Vec<f64> = basin.cells.iter().zip(&floors).map(|(&i, f)| area[i as usize] * (top - f.0).max(0.0)).collect();
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
    let (a, b): (Vec<u32>, Vec<u32>) = (vec![basin.cells[hub]; links], others.iter().map(|&c| basin.cells[c]).collect());
    let ks: Vec<u16> = liq.iter().map(|&k| k as u16).collect();
    s.basin = basin;
    book(w, s, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &ks, mol: &mol, heat: &heat }]);
    mix_heat(w, s);
}

/// Does cell `i` mix as one with the low basin under its air block (`mix_heat`) and face the air as one surface with
/// the basin under its air node (`air::surf`)? A cell of the low basin off its melting plateaus (and whose state is
/// not stale), with no soft cover and no cover of bodies.
pub(super) fn one_surface(w: &World, s: &Scratch, i: usize) -> bool {
    s.basin.is[i] && s.state.pk[i] == NONE && s.latent[i].hi != f64::MIN && s.cover[i][2] <= 0.0 && w.canopy[i] <= 0.0
}

/// The low basin under each air block mixes its heat in a month: its cells that mix as one (`one_surface`), with the same
/// phases, each take the block's temperature, the heat of their sum over their summed heat capacity (apart from the
/// fusion enthalpy each holds), as one heat flux from the cell of the most heat capacity to each other.
fn mix_heat(w: &mut World, s: &mut Scratch) {
    let mut block = vec![0; w.mesh.level(AIR).n()];
    blocks(w).iter().enumerate().for_each(|(b, nodes)| nodes.iter().for_each(|&j| block[j as usize] = b));
    let parent = w.mesh.cells().parent();
    let mut by: BTreeMap<(usize, u64), Vec<usize>> = BTreeMap::new();
    for &i in s.basin.cells.iter().filter(|&&i| one_surface(w, s, i as usize)) {
        by.entry((block[parent[i as usize] as usize], s.latent[i as usize].molten)).or_default().push(i as usize);
    }
    let (mut a, mut b, mut heat) = (Vec::new(), Vec::new(), Vec::new());
    for cells in by.values().filter(|c| c.len() > 1) {
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
}
