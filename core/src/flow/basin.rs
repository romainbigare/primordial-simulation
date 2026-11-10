//! The low basin (laws.md 1.2): the cells joined to the lowest cell through cells whose own low ground lies under the
//! level its liquid stands at hold one body of liquid with one level, over each one's own ground. `find` names them at each full build of the ground's state; `level` shares the basin's liquid
//! by that level once a month, after the runoff has reached it (`share_out`).

use super::{air::blocks, curve::{level_over, Curve}, drain::liquid_heat, land::{cells_state, stand, wet, NONE}, liquid::book, parts::TOP, potential::apply, Scratch};
use crate::{store::{Class, LinkFlux}, World, AIR};
use std::collections::BTreeMap;

/// The basin: whether each cell holds part of its liquid body, and those cells in order; and whether each lies under its
/// level (its mean surface under it: a cell under the body, not one of its rim).
#[derive(Clone, Debug, Default)]
pub(super) struct Basin {
    pub(super) body: Vec<bool>,
    pub(super) cells: Vec<u32>,
    pub(super) is: Vec<bool>,
}

/// The basin now: the lowest cell and every cell joined to it through cells whose own low ground (`curve::Curve`) lies
/// under the level its liquid stands at (`land::stand`), each of them under it where its mean surface is; none where
/// the lowest cell's liquid does not stand.
pub(super) fn find(w: &World, s: &Scratch) -> Basin {
    let cells = w.mesh.cells();
    let n = cells.n();
    let mut basin = Basin { body: vec![false; n], cells: Vec::new(), is: vec![false; n] };
    let Some(lowest) = (0..n).min_by(|&a, &b| s.z[a].total_cmp(&s.z[b])) else { return basin };
    let top = stand(s, lowest);
    if top <= s.z[lowest] {
        return basin;
    }
    let (mut todo, body) = (vec![lowest], &mut basin.body);
    body[lowest] = true;
    while let Some(i) = todo.pop() {
        for j in cells.around(i).1.iter().map(|&j| j as usize) {
            if !body[j] && s.z[j] + s.curves[j].low() < top {
                body[j] = true;
                todo.push(j);
            }
        }
    }
    basin.cells = (0..n as u32).filter(|&i| basin.body[i as usize]).collect();
    basin.is = (0..n).map(|i| basin.body[i] && s.z[i] < top).collect();
    basin
}

/// The basin's liquid of the liquids `liq` (what boils below the coldest node, `tmin`, is a dissolved gas) stands at one
/// level: each cell's pores keep what they hold, and all the rest stands over the cells' own ground (`curve::level_over`):
/// that liquid body has one level (its slope under wind and rotation is not modelled). Each cell that holds more than
/// its pores and what stands on its ground under that level gives the share of its liquid over it, with that share of
/// its heat as it leaves (`drain::liquid_heat`); mixed, it goes to the cells that hold less, by what each lacks
/// (`share_out`), with the potential energy of what moves (`potential`).
pub(super) fn level(w: &mut World, s: &mut Scratch, liq: &[usize], tmin: f64) {
    let basin = std::mem::take(&mut s.basin);
    if basin.cells.is_empty() {
        s.basin = basin;
        return;
    }
    let area = w.mesh.cells().area();
    let (held, vol) = movable(w, s, liq, &basin.cells, tmin, (|k, i| wet(s, k, i), |i, mol| liquid_heat(w, s, liq, i, mol)));
    let pore: Vec<f64> = basin.cells.iter().zip(&vol).map(|(&i, &v)| v.min(s.pd[i as usize] * area[i as usize])).collect();
    let parts: Vec<(&Curve, f64)> = basin.cells.iter().map(|&i| (&s.curves[i as usize], s.z[i as usize])).collect();
    let top = level_over(&parts, vol.iter().zip(&pore).map(|(v, p)| v - p).sum());
    let want: Vec<f64> = parts.iter().zip(&pore).map(|((c, z), p)| p + c.volume(top - z)).collect();
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

/// What may move of the liquids `liq` in each cell of `cells` (`[c * (liq.len() + 1) + q]`): all of one that boils below
/// the coldest node, `tmin` (a gas dissolved in the liquid, which takes no room), else its liquid share `wet(k, i)`,
/// mol, then their heat as they leave, `heat(i, mol)`, J; and their volume, m³.
pub(super) fn movable(w: &World, s: &Scratch, liq: &[usize], cells: &[u32], tmin: f64, (wet, heat): (impl Fn(usize, usize) -> f64, impl Fn(usize, &[f64]) -> f64)) -> (Vec<f64>, Vec<f64>) {
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
pub(super) fn share_out(cells: &[u32], (held, vol): (&[f64], &[f64]), want: &[f64], nl: usize) -> (Vec<u32>, Vec<u32>, Vec<f64>, Vec<f64>) {
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
/// the basin under its air node (`air::surf`)? A cell of the low basin all under its liquid (a node that is one, or of
/// parts with no land, its liquid's top, `parts`), off its melting plateaus, with no soft cover and no cover of bodies.
pub(super) fn one_surface(w: &World, s: &Scratch, i: usize) -> bool {
    let (pk, cover) = match s.mixed(i) {
        Some(m) if m.two_faces() => return false,
        Some(m) => (m.piece[TOP].pk, m.piece[TOP].cover[2]),
        None => (s.state.pk[i], s.cover[i][2]),
    };
    s.basin.is[i] && pk == NONE && cover <= 0.0 && w.canopy[i] <= 0.0
}

/// Does the liquid of cell `i` of the low basin mix as one with the basin's (`mix_heat`, `air::dissolve`)? A cell that
/// faces the air as one surface with it (`one_surface`); of a cell of parts, its liquid's top, off its plateaus, with no
/// soft cover and no cover of bodies.
pub(super) fn mixes(w: &World, s: &Scratch, i: usize) -> bool {
    s.mixed(i).map(|m| &m.piece[TOP]).map_or(one_surface(w, s, i), |p| s.basin.body[i] && p.pk == NONE && p.cover[2] <= 0.0 && w.canopy[i] <= 0.0)
}

/// What is molten in the liquid of cell `i` that mixes (`mixes`), one bit per substance: of a cell of parts its
/// liquid's top's.
pub(super) fn phases(s: &Scratch, i: usize) -> u64 {
    s.mixed(i).map_or(s.latent[i].molten, |m| m.piece[TOP].latent.molten)
}

/// The low basin under each air block mixes its heat in a month: the liquid of its cells that mixes as one (`mixes`),
/// with the same phases, takes the block's temperature, the heat of their sum over their summed heat capacity (apart from the
/// fusion enthalpy each holds), as one heat flux from the cell of the most heat capacity to each other, a cell of parts'
/// its liquid's top's (`parts::Parts`). Returns the cells that mixed.
fn mix_heat(w: &mut World, s: &mut Scratch) -> Vec<u32> {
    let mut block = vec![0; w.mesh.level(AIR).n()];
    blocks(w).iter().enumerate().for_each(|(b, nodes)| nodes.iter().for_each(|&j| block[j as usize] = b));
    let parent = w.mesh.cells().parent();
    let liquid = |i: usize| s.mixed(i).map(|m| &m.piece[TOP]);
    let mut by: BTreeMap<(usize, u64), Vec<usize>> = BTreeMap::new();
    for &i in s.basin.cells.iter().filter(|&&i| mixes(w, s, i as usize)) {
        by.entry((block[parent[i as usize] as usize], phases(s, i as usize))).or_default().push(i as usize);
    }
    let free = |i: usize| liquid(i).map_or(w.store.heat(Class::Ground)[i] - s.latent[i].held, |p| p.heat - p.latent.held);
    let cap = |i: usize| liquid(i).map_or(s.cap[i], |p| p.cap);
    let (mut a, mut b, mut heat, mut mixed, mut parts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for cells in by.values().filter(|c| c.len() > 1) {
        mixed.extend(cells.iter().map(|&i| i as u32));
        let t = cells.iter().map(|&i| free(i)).sum::<f64>() / cells.iter().map(|&i| cap(i)).sum::<f64>();
        let hub = cells.iter().copied().max_by(|&p, &q| cap(p).total_cmp(&cap(q)).then(q.cmp(&p))).unwrap_or(cells[0]);
        for &i in cells.iter().filter(|&&i| i != hub) {
            a.push(hub as u32);
            b.push(i as u32);
            heat.push(cap(i) * t - free(i));
        }
        parts.extend(cells.iter().filter(|&&i| liquid(i).is_some()).map(|&i| (i, cap(i) * t - free(i))));
    }
    apply(w, s, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &[], mol: &[], heat: &heat }], &[]);
    for (i, q) in parts {
        if let Some(Some(p)) = s.state.parts.get_mut(i) {
            p.heat[TOP] += q;
        }
    }
    mixed.sort_unstable();
    mixed
}
