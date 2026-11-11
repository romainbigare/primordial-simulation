//! The shelf the wear at the world's start lays on the open low ground (laws.md 0 and 8): what flowing liquid and
//! creep carry off the land into the open low ground (`space::wear`) settles where the liquid slows, at the mouths of
//! the channels, and the waves spread it along the shore. Each sample of the shore takes the load that reached the
//! shore within `ALONG_M` of it, as a share of that stretch of shore; from it the load builds out a platform a little
//! under 0 m (`TOPSET_M`, a delta's topset), its width in proportion to that load, and a front that falls from the
//! platform's edge at the slope of the load at rest under liquid (`FORESET`, a delta's foreset; Gilbert 1885). So a
//! channel that brings much load builds a fan at its mouth, a coast that gets little keeps its steep drop, and the
//! width that makes the shelf hold all the load is found by false position (`TOLERANCE`).

use super::grid::Grid;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The depth under 0 m the load fills the open low ground up to, m: a delta's platform, where the waves stir the bed.
const TOPSET_M: f64 = 5.0;
/// The slope of the front of the load under liquid, rise over run: its angle of rest there.
const FORESET: f64 = 0.3;
/// Half the length of shore the waves spread a mouth's load along, m.
const ALONG_M: f64 = 5e3;
/// The search for the shelf's width stops within this share of it, or where the shelf holds the load to this share; the
/// shelf is then thinned to hold it exactly.
const TOLERANCE: f64 = 1e-2;

/// A sample of the open low ground in the reach of the shelf (`shelf`): by how far it lies inside the platform's edge,
/// m (less than 0 past it), the farthest first, then the least index.
struct Inside(f64, usize);

impl Ord for Inside {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0).then(o.1.cmp(&self.1))
    }
}
impl PartialOrd for Inside {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Inside {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Inside {}

/// Lay the load `offshore` (m³ per m² of each sample of grid `g`, by the shore, where it was carried from coarser grids
/// smooth) on the open low ground `open` of heights `h` as a shelf, adding it to the loose ground `loose`, m; returns
/// the volume laid, m³: all the load, or on a map whose open low ground the platform fills, what it holds.
pub(super) fn lay(g: &Grid, h: &mut [f64], (open, offshore): (&[bool], &[f64]), loose: &mut [f64]) -> f64 {
    let n = g.n();
    let brought: Vec<f64> = (0..n).map(|k| offshore[k] * g.area(k)).collect();
    let volume: f64 = brought.iter().sum();
    if volume <= 0.0 {
        return 0.0;
    }
    let shore: Vec<f64> = (0..n).map(|k| if open[k] && g.around(k).any(|(j, _)| !open[j]) { 1.0 } else { 0.0 }).collect();
    let (along, shores) = (g.spread(&brought, ALONG_M), g.spread(&shore, ALONG_M));
    let load: Vec<f64> = (0..n).map(|k| if shore[k] > 0.0 { along[k] / shores[k] } else { 0.0 }).collect();
    let floor = (0..n).filter(|&k| open[k]).map(|k| h[k]).fold(0.0, f64::min);
    let laid = |wide: f64| shelf(g, h, open, (&load, wide), floor);
    let held = |wide: f64| laid(wide).iter().map(|&(k, t)| t * g.area(k)).sum::<f64>();
    // The width a m³ of load builds, m per m³: first as if the shelf stood `TOPSET_M` thick along a shore of the
    // samples' side, doubled until the shelf holds the load (where the open low ground is full to the platform before,
    // the rest leaves the map), then by false position between the two last (Illinois: Dowell and Jarratt 1971).
    let along_shore: f64 = load.iter().sum::<f64>() * g.dy * TOPSET_M;
    let (mut lo, mut hi) = ((0.0, f64::NAN), (volume / along_shore / g.dy, 0.0));
    hi.1 = held(hi.0) - volume;
    while hi.1 < 0.0 {
        let next = (2.0 * hi.0, held(2.0 * hi.0) - volume);
        if next.1 <= hi.1 {
            break;
        }
        (lo, hi) = (hi, next);
    }
    if lo.1.is_nan() && hi.1 > 0.0 {
        lo.1 = held(0.0) - volume;
    }
    // Where the fronts alone, with no platform, hold more than the load, they are thinned to it.
    if lo.1 >= 0.0 {
        hi = lo;
    }
    let mut side = 0;
    while hi.1 > TOLERANCE * volume && lo.1 < 0.0 && hi.0 - lo.0 > TOLERANCE * hi.0 {
        let w = (lo.0 * hi.1 - hi.0 * lo.1) / (hi.1 - lo.1);
        let f = held(w) - volume;
        if f < 0.0 {
            lo = (w, f);
            hi.1 /= if side == -1 { 2.0 } else { 1.0 };
            side = -1;
        } else {
            hi = (w, f);
            lo.1 /= if side == 1 { 2.0 } else { 1.0 };
            side = 1;
        }
    }
    let hi = hi.0;
    let shelf = laid(hi);
    let scale = (volume / shelf.iter().map(|&(k, t)| t * g.area(k)).sum::<f64>()).min(1.0);
    shelf.iter().map(|&(k, t)| {
        h[k] += t * scale;
        loose[k] += t * scale;
        t * scale * g.area(k)
    }).sum()
}

/// The thickness of the shelf on each sample of the open low ground `open` it reaches, m, where each sample of the
/// shore builds a platform `wide` m per m³ of its `load` wide at `TOPSET_M` under 0 m, fronted at `FORESET` down to
/// the lowest of the heights `h`, `floor`: a sample lies inside the edge by the most, over the samples of the shore,
/// of that one's width less the distance to it through the open low ground.
fn shelf(g: &Grid, h: &[f64], open: &[bool], (load, wide): (&[f64], f64), floor: f64) -> Vec<(usize, f64)> {
    let reach = (-TOPSET_M - floor).max(0.0) / FORESET;
    let mut inside = vec![f64::NEG_INFINITY; g.n()];
    let mut heap: BinaryHeap<Inside> = (0..g.n()).filter(|&k| load[k] > 0.0).map(|k| Inside(wide * load[k], k)).collect();
    heap.iter().for_each(|i| inside[i.1] = i.0);
    let mut out = Vec::new();
    while let Some(Inside(v, k)) = heap.pop() {
        if v < inside[k] {
            continue;
        }
        let top = -TOPSET_M - FORESET * (-v).max(0.0);
        if top > h[k] {
            out.push((k, top - h[k]));
        }
        for (j, d) in g.around(k) {
            if open[j] && v - d > inside[j] && v - d > -reach {
                inside[j] = v - d;
                heap.push(Inside(v - d, j));
            }
        }
    }
    out
}
