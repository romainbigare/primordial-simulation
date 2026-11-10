//! The way liquid leaves the terrain's samples (laws.md 0 and 8): a priority flood from the open low ground that does
//! not raise the ground (Barnes, Lehman and Mulla 2014). It reaches the samples by the level their liquid stands to,
//! lowest first: a sample's own height where it drains, and under a closed basin's level that of the basin's lowest
//! pass (the highest ground on the lowest way out). Each sample's liquid runs to the neighbour reached before it that
//! its flood's surface falls to most steeply (D8, O'Callaghan and Mark 1984): the ground's own height where it drains,
//! under a basin's level that level raised by `FLOOD_RISE` along the way the flood came, so the way runs to the pass
//! (Planchon and Darboux 2001; Cordonnier, Bovy and Braun 2019).
//! The samples' ways (`space::samples`) and the law that wears the ground (`space::wear`) both run on it.

use super::grid::Grid;
use super::network::NONE;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The rise of the flood's surface per m along its way across a closed basin, m per m: it only orders the samples
/// under one level, so the way of each runs to the pass and no way runs in a circle; far below the ground's slopes.
const FLOOD_RISE: f64 = 1e-6;

/// A sample waiting in the flood, by its level and then its height on the flood's surface: the lowest first, then the
/// lowest index.
struct Low(f64, f64, usize);

impl Ord for Low {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then(o.1.total_cmp(&self.1)).then(o.2.cmp(&self.2))
    }
}
impl PartialOrd for Low {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Low {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Low {}

/// What the flood finds: of each sample its height on the flood's surface and the level its liquid stands to, m (its
/// own height on ground that drains), the neighbour its liquid runs to (`NONE` where none lies lower: a sink of the
/// open low ground) and how far it lies, m, and its place in the order the flood reached them (`NONE` if it never
/// did); and the samples in that order, so each comes after the one its liquid runs to.
pub(super) struct Flood {
    pub(super) surface: Vec<f64>,
    pub(super) level: Vec<f64>,
    pub(super) to: Vec<u32>,
    pub(super) apart: Vec<f64>,
    pub(super) rank: Vec<u32>,
    pub(super) order: Vec<u32>,
}

impl Flood {
    /// Does sample `k` of heights `z` lie under its basin's level (under standing liquid, with no way down)?
    pub(super) fn under(&self, z: &[f64], k: usize) -> bool {
        self.level[k] > z[k]
    }

    /// The neighbours of sample `k` on grid `g` the flood reached before it (`Grid::ways`), the only ones liquid and
    /// load spread down to from it, so they spread in the flood's order.
    pub(super) fn earlier<'a>(&'a self, g: &'a Grid, k: usize) -> impl Iterator<Item = (usize, bool, f64)> + 'a {
        g.ways(k).filter(move |w| self.rank[w.0] < self.rank[k])
    }
}

/// The open low ground of heights `z` on grid `g`, where the flood starts: on a globe every sample under 0 m; on an
/// island those under 0 m joined to its lowest sample, so ground under 0 m inland is a closed basin too; and the lowest
/// sample where none lies under 0 m.
pub(super) fn open_low(g: &Grid, z: &[f64]) -> Vec<bool> {
    let mut open: Vec<bool> = z.iter().map(|&h| h < 0.0 && g.wrap).collect();
    let low = (0..z.len()).min_by(|&a, &b| z[a].total_cmp(&z[b]).then(a.cmp(&b))).expect("a sample");
    if !g.wrap || !open.contains(&true) {
        let mut todo = vec![low];
        open[low] = true;
        while let Some(k) = todo.pop() {
            for (j, _) in g.around(k) {
                if z[j] < 0.0 && !open[j] {
                    open[j] = true;
                    todo.push(j);
                }
            }
        }
    }
    open
}

/// The flood of heights `z` on grid `g` from the open low ground `open` (`Flood`): from all of it, or with `shore`
/// from the samples of it beside land only (the rest it never reaches); the sample of the lowest level first, then the
/// lowest on the flood's surface, then the least index. A neighbour it reaches stands at its own height or the level
/// the flood came at, the higher, and on the surface at its own height or `FLOOD_RISE` times their distance higher than
/// the sample it was reached from, the higher, so each sample of land has a way down.
pub(super) fn flood(g: &Grid, z: &[f64], open: &[bool], shore: bool) -> Flood {
    let n = z.len();
    let (mut surface, mut level, mut seen) = (z.to_vec(), z.to_vec(), open.to_vec());
    let (mut to, mut apart, mut rank, mut order) = (vec![NONE; n], vec![0.0; n], vec![NONE; n], Vec::with_capacity(n));
    let start = |i: usize| open[i] && (!shore || g.around(i).any(|(j, _)| !open[j]));
    let mut heap: BinaryHeap<Low> = (0..n).filter(|&i| start(i)).map(|i| Low(z[i], z[i], i)).collect();
    while let Some(Low(l, s, i)) = heap.pop() {
        rank[i] = order.len() as u32;
        order.push(i as u32);
        let mut steepest = (NONE, 0.0, 0.0);
        for (j, d) in g.around(i) {
            if rank[j] != NONE {
                // Reached before: lower on the surface, the way down.
                let fall = (s - surface[j]) / d;
                if fall > steepest.1 {
                    steepest = (j as u32, fall, d);
                }
            }
            if seen[j] {
                continue;
            }
            seen[j] = true;
            (surface[j], level[j]) = (z[j].max(s + FLOOD_RISE * d), z[j].max(l));
            heap.push(Low(level[j], surface[j], j));
        }
        (to[i], apart[i]) = (steepest.0, steepest.2);
    }
    Flood { surface, level, to, apart, rank, order }
}
