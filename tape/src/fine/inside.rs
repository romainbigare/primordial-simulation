//! What lies inside the cells of a window, pixel by pixel: each cell's means shared over the window's own ground by the
//! general laws the core shares them by over a cell's samples (`pcore::inside`), evaluated at the pixels, and kept
//! continuous across the cells' borders with each cell's mean held as far as that allows (`keep`). The soil's liquid gathers by the
//! wetness index of the window's ground (TOPMODEL, Beven and Kirkby 1979), found across the cells' borders; what stands
//! is the core's level over the ground drawn; each kind of cover goes by its activity on ground that wet, a field of
//! its own across the cells that hold it.

use super::heights::leaf;
use super::keep::{Cells, Mean};
use super::Shown;
use pcore::cover::{Parts, SLOTS};
use pcore::inside::Grid;
use pcore::World;
use std::collections::BTreeMap;

/// The window's pixels: from leaf column `gx` and row `gy`, `across` x `down` leaves of `depth`.
pub(super) struct Window {
    pub at: [i64; 2],
    pub size: [usize; 2],
    pub depth: u8,
}

impl Window {
    /// The area of a pixel of each row, m².
    pub(super) fn rows(&self, w: &World) -> Vec<f64> {
        (0..self.size[1]).map(|r| w.map.place_of(&leaf(self.depth, self.at[0], self.at[1] + r as i64), w.cfg.lat_south, w.cfg.lat_north).0).collect()
    }
}

/// The wetness index of each pixel of the window `win` of ground `ground` (`Grid::wetness_index`): what drains through
/// it of the window's own area, across the cells' borders, over its slope.
pub(super) fn wetness_index(w: &World, win: &Window, ground: &[f64]) -> Vec<f64> {
    let rows = win.rows(w);
    let (across, dy) = (win.size[0], w.map.length(win.depth));
    let grid = Grid::new(across, win.size[1], dy, rows.iter().map(|a| a / dy).collect());
    let own: Vec<f64> = (0..ground.len()).map(|p| rows[p / across]).collect();
    Grid::wetness_index(&grid.drain(ground, own.clone()), &own, &grid.slope(ground))
}

/// The liquid in the soil of each pixel, m (as a depth), held to each cell's (`Means::pore`): from the cells' liquid
/// evened across their borders (`Cells::even`), it departs by `soil_m` per unit of the window's wetness index `twi`
/// (`wetness_index`) over the index's mean, as in TOPMODEL, taken as a share of the depth the pores hold there and
/// compounded, so it never falls below none; the core shares a cell's liquid over its samples the same way
/// (`inside::Inside`), here over the window's pixels and across the cells' borders.
fn soil(w: &World, cells: &Cells, shown: &Shown, twi: &[f64]) -> Vec<f64> {
    let middle = twi.iter().sum::<f64>() / twi.len() as f64;
    let all = |_| true;
    let pore: Vec<f64> = shown.means.iter().map(|m| m.pore).collect();
    let (level, pores) = (cells.even(&pore, all), cells.even(&shown.means.iter().map(|m| m.pores).collect::<Vec<f64>>(), all));
    let m = w.cfg.phys.soil_m;
    let start = (0..twi.len()).map(|p| level[p] * (m * (twi[p] - middle) / pores[p].max(1e-9)).exp()).collect();
    cells.keep(start, &pore, Mean::Scaled)
}

/// Of each pixel, how wet its ground `ground` is, 0..1 (`Physics::wetness` of the liquid in its soil, `soil`, by the
/// window's wetness index `twi`, and of what stands over it at the level `level`).
pub(super) fn wetness(w: &World, (cells, shown): (&Cells, &Shown), (ground, twi): (&[f64], &[f64]), level: &[f64]) -> Vec<f64> {
    let pore = soil(w, cells, shown, twi);
    let deep = |p: usize| if level[p].is_nan() { 0.0 } else { (level[p] - ground[p]).max(0.0) };
    (0..pore.len()).map(|p| w.cfg.phys.wetness(pore[p] + deep(p))).collect()
}

/// The built cover of each kind (the starting kind its slots descend from, `Cover::root`) in each cell of the world,
/// kg/m², and in each cell shown the genes of its heaviest slot of the kind (none where it holds none).
struct Kinds {
    built: BTreeMap<u32, Vec<f64>>,
    genes: BTreeMap<u32, Vec<Option<[f32; pcore::cover::NG]>>>,
}

fn kinds(w: &World, shown: &Shown, p: &Parts) -> Kinds {
    let (mm, n, area) = (p.mm(&w.chem), w.mesh.cells().n(), w.mesh.cells().area());
    let mut k = Kinds { built: BTreeMap::new(), genes: BTreeMap::new() };
    for (i, a) in area.iter().enumerate() {
        let c = w.life.cover(i);
        for x in c.held() {
            k.built.entry(c.root[x]).or_insert_with(|| vec![0.0; n])[i] += c.built(&mm, x) / a;
        }
    }
    for (s, &i) in shown.cells.iter().enumerate() {
        let c = w.life.cover(i);
        for x in c.held() {
            let g = k.genes.entry(c.root[x]).or_insert_with(|| vec![None; shown.cells.len()]);
            let heaviest = c.held().filter(|&y| c.root[y] == c.root[x]).max_by(|&a, &b| c.built(&mm, a).total_cmp(&c.built(&mm, b)));
            g[s] = heaviest.map(|y| c.genes[y]);
        }
    }
    k
}

/// Of each pixel, the built cover per m² of each of the `SLOTS` slots its cell holds (plane `k` slot `k`, 0 where it
/// holds fewer), kg/m²: each kind a field across the cells (`keep`), each cell's mean its built cover of the kind, by
/// its activity on the pixel's ground as wet as `wet` (`cover::activity`, with the genes of the cell's heaviest slot of
/// the kind; even over a cell where none is active, as the core shares it, `Inside::kind`); a cell's slots of a kind
/// take the kind's field by their shares of its built cover there.
pub(super) fn cover(w: &World, cells: &Cells, shown: &Shown, wet: &[f64]) -> Vec<Vec<f64>> {
    let pixels = wet.len();
    let mut planes = vec![vec![0.0; pixels]; SLOTS];
    let Some(parts) = Parts::of(&w.chem) else { return planes };
    let (mm, k, ph) = (parts.mm(&w.chem), kinds(w, shown, &parts), &w.cfg.phys);
    for (root, built) in &k.built {
        let Some(genes) = k.genes.get(root) else { continue };
        let any = genes.iter().flatten().next().copied().expect("a kind shown has genes");
        let act: Vec<f64> = (0..pixels).map(|p| pcore::cover::activity(ph, &genes[cells.of()[p] as usize].unwrap_or(any), wet[p])).collect();
        let idle: Vec<bool> = (0..shown.cells.len()).map(|s| cells.mean(s, |p| act[p]) <= 0.0).collect();
        let target: Vec<f64> = shown.cells.iter().map(|&i| built[i]).collect();
        let smooth = cells.even(&target, |_| true);
        let start = (0..pixels).map(|p| smooth[p] * if idle[cells.of()[p] as usize] { 1.0 } else { act[p] }).collect();
        let field = cells.keep(start, &target, Mean::Scaled);
        for (s, &i) in shown.cells.iter().enumerate() {
            let c = w.life.cover(i);
            let of: f64 = c.held().filter(|&x| c.root[x] == *root).map(|x| c.built(&mm, x)).sum();
            for (slot, x) in c.held().enumerate().filter(|&(_, x)| c.root[x] == *root) {
                let share = c.built(&mm, x) / of.max(1e-300);
                cells.pixels(s).iter().for_each(|&p| planes[slot][p] = field[p] * share);
            }
        }
    }
    planes
}

/// Of each pixel of the window `win`, whose cells are `shown` and whose wetness is `wet` (`wetness`), appended to `out`
/// after zero bytes up to a multiple of 4 of its length: how wet its ground is, 0..1 (f32); then `SLOTS` planes, plane
/// `k` the built cover per m² of the `k`-th slot its cell holds (in the order `tape::things` lists them), kg/m² (f32;
/// 0 where its cell holds fewer; `cover`). Then of each pixel the number of its cell among the window's cells (u32),
/// and the count of those cells (u32) and of each of them, for each of its `SLOTS` slots, its walls and its capture area
/// per kg of its built cover (kg/kg, m²/kg; f32, 0 for a slot it does not hold), so a shell sees what each slot builds
/// where it stands.
pub(super) fn extend(out: &mut Vec<u8>, w: &World, (cells, shown): (&Cells, &Shown), wet: &[f64]) {
    let cover = cover(w, cells, shown, wet);
    out.resize(out.len().next_multiple_of(4), 0);
    out.extend(wet.iter().chain(cover.iter().flatten()).flat_map(|&v| (v as f32).to_le_bytes()));
    out.extend(shown.number.iter().chain([&(shown.cells.len() as u32)]).flat_map(|v| v.to_le_bytes()));
    let parts = Parts::of(&w.chem);
    for &i in &shown.cells {
        let c = w.life.cover(i);
        let make: Vec<[f64; 2]> = parts.map_or(Vec::new(), |p| {
            let mm = p.mm(&w.chem);
            let per_kg = |x: usize, v: f64| v / c.built(&mm, x).max(1e-300);
            c.held().map(|x| [per_kg(x, c.walls(&mm, x)), per_kg(x, c.leaf(&mm, x))]).collect()
        });
        let make = (0..SLOTS).map(|k| make.get(k).copied().unwrap_or([0.0; 2]));
        out.extend(make.flatten().flat_map(|v| (v as f32).to_le_bytes()));
    }
}
