//! The ground below the terrain's samples (laws.md 0 and 8): one function of the seed and the map that every cell's
//! height and level-volume curve and every pixel a shell draws read (`Terrain::heights`, `Terrain::inside`).
//!
//! Each leaf finer than `SAMPLE_DEPTH` departs from its parent's mean by a smooth continuation of the coarser heights
//! that keeps that mean: the parent's quarters lean as the quartic through the parent and two neighbours each way on
//! each axis does (`LEAN`; average interpolation, Donoho 1993), so the ground has no steps and its slope no kinks at
//! the leaves' edges, and a slope across many samples goes on across their quarters. Nothing random is laid on it: the
//! worn samples (`space::wear`) hold the ground's shape. A leaf's height depends on its own place only, whatever window
//! it is found for.

use super::square::Square;
use super::terrain::{Terrain, SAMPLE_DEPTH};
use super::tree::BLOCK;
use super::Leaf;

/// Leaves of each depth round a window whose heights are found too: a leaf's height reads the heights of two leaves
/// each way of its parent, each group of four whole, so a margin of M at a depth needs at least (M + 1) / 2 + 2 at the
/// one above.
const MARGIN: i64 = 7;
/// The weights of a leaf's neighbours from two before it to two after it along an axis (its own: 0) in the lean of its
/// half after it: that half's mean departs from the leaf's by this sum, its half before by minus it. They are the
/// quartic through the five means (average interpolation, Donoho 1993), whose limit is smooth with its slope; the
/// quadratic through three (±(next − last) / 8) leaves kinks in the slope along the leaves' edges.
const LEAN: [f64; 5] = [3.0 / 128.0, -22.0 / 128.0, 0.0, 22.0 / 128.0, -3.0 / 128.0];

/// The leaf of `depth` at column `x` and row `y` of them.
pub fn leaf_at(depth: u8, x: i64, y: i64) -> Leaf {
    let side = BLOCK >> depth;
    Leaf { x: x as u64 * side, y: y as u64 * side, depth }
}

/// The lean along each axis and across both of the quarters of the leaf at (`x`, `y`), from the heights `at(dx, dy)`
/// of it and its neighbours (`LEAN`).
fn leans(at: impl Fn(i64, i64) -> f64) -> [f64; 3] {
    let along = |f: &dyn Fn(i64) -> f64| (-2..=2).map(|i| LEAN[(i + 2) as usize] * f(i)).sum::<f64>();
    [along(&|i| at(i, 0)), along(&|j| at(0, j)), along(&|j| along(&|i| at(i, j)))]
}

/// The departures of the means of the quarters (of areas `area`, m², row by row) of a leaf from its mean, m, from the
/// heights `at(dx, dy)` of it and its neighbours: their lean (`leans`; across both ways, the product of the two leans)
/// less the lean's mean by area, so the quarters keep the leaf's mean; the whole lean scaled down where a quarter would
/// lie past the lowest or the highest of the leaf and its eight neighbours (Barth and Jespersen 1989), so the finer
/// ground makes no new pit and no new peak, and a valley's floor falls along it as its leaves' means do.
pub(super) fn lean_quarters(at: impl Fn(i64, i64) -> f64, area: [f64; 4]) -> [f64; 4] {
    let around = (-1..=1).flat_map(|j| (-1..=1).map(move |i| (i, j))).map(|(i, j)| at(i, j));
    let (lo, hi) = around.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| (lo.min(z), hi.max(z)));
    let own = at(0, 0);
    let [ew, sn, corners] = leans(at);
    let lean: [f64; 4] = std::array::from_fn(|q| {
        let (sx, sy) = (if q % 2 == 1 { 1.0 } else { -1.0 }, if q >= 2 { 1.0 } else { -1.0 });
        sx * ew + sy * sn + sx * sy * corners
    });
    let mean = (0..4).map(|q| area[q] * lean[q]).sum::<f64>() / area.iter().sum::<f64>();
    let lean = lean.map(|l| l - mean);
    let room = |d: f64| if d > 0.0 { (hi - own) / d } else if d < 0.0 { (lo - own) / d } else { 1.0 };
    let scale = lean.iter().map(|&d| room(d)).fold(1.0, f64::min);
    lean.map(|l| l * scale)
}

/// The leaves of one depth over a rectangle of them (from column `x` and row `y`, in leaves of that depth), row by
/// row: each one's mean height, m.
struct Patch {
    depth: u8,
    x: i64,
    y: i64,
    w: usize,
    h: usize,
    z: Vec<f64>,
}

impl Patch {
    /// The index of leaf (`x`, `y`), the nearest the patch holds where it holds none.
    fn near(&self, x: i64, y: i64) -> usize {
        (y - self.y).clamp(0, self.h as i64 - 1) as usize * self.w + (x - self.x).clamp(0, self.w as i64 - 1) as usize
    }
}

/// The rectangle of leaves of `depth` that hold the window `win` (from column `x0` and row `y0` to `x1` and `y1`,
/// inclusive, in leaves of `of`, no shallower) with `MARGIN` leaves round it, on a map `across` x `down` leaves of `of`.
fn cover([x0, y0, x1, y1]: [i64; 4], of: u8, depth: u8, (across, down): (i64, i64)) -> [i64; 4] {
    let k = of - depth;
    let (aw, ad) = ((across >> k).max(1), (down >> k).max(1));
    [((x0 >> k) - MARGIN).max(0), ((y0 >> k) - MARGIN).max(0), ((x1 >> k) + MARGIN).min(aw - 1), ((y1 >> k) + MARGIN).min(ad - 1)]
}

impl Terrain {
    /// The terrain's mean height over each leaf of `depth` (no shallower than a square) of the window `win` (from column
    /// `x0` and row `y0` to `x1` and `y1`, inclusive, in those leaves), m, row by row: at or above `SAMPLE_DEPTH` the
    /// mean by area of the samples it covers, finer the ground below them (the module's doc).
    pub fn heights(&self, seed: u64, map: &Square, lat: (f64, f64), win: [i64; 4], depth: u8) -> Vec<f64> {
        let (across, down) = ((win[2] - win[0] + 1) as usize, (win[3] - win[1] + 1) as usize);
        let pixel = |p: usize| (win[0] + (p % across) as i64, win[1] + (p / across) as i64);
        if depth <= SAMPLE_DEPTH {
            return (0..across * down).map(pixel).map(|(x, y)| self.inside(seed, map, leaf_at(depth, x, y), lat)).collect();
        }
        let p = self.patch(seed, map, lat, win, depth);
        (0..across * down).map(pixel).map(|(x, y)| p.z[p.near(x, y)]).collect()
    }

    /// The mean height of leaf `l`, finer than `SAMPLE_DEPTH`, m.
    pub(super) fn below(&self, seed: u64, map: &Square, l: Leaf, lat: (f64, f64)) -> f64 {
        let (x, y) = ((l.x / l.side()) as i64, (l.y / l.side()) as i64);
        let p = self.patch(seed, map, lat, [x, y, x, y], l.depth);
        p.z[p.near(x, y)]
    }

    /// The patch of leaves of `depth` (finer than `SAMPLE_DEPTH`) over the window `win` and its margin, from the drained
    /// samples down, a depth at a time (`finer`).
    fn patch(&self, seed: u64, map: &Square, lat: (f64, f64), win: [i64; 4], depth: u8) -> Patch {
        let s = self.samples(seed, map, lat);
        let shift = depth - map.square_depth();
        let extent = ((map.w as i64) << shift, (map.h as i64) << shift);
        let first = cover(win, depth, SAMPLE_DEPTH, extent);
        let (pw, ph) = ((first[2] - first[0] + 1) as usize, (first[3] - first[1] + 1) as usize);
        let mut patch = Patch { depth: SAMPLE_DEPTH, x: first[0], y: first[1], w: pw, h: ph, z: Vec::with_capacity(pw * ph) };
        for k in 0..pw * ph {
            let k = s.index(&leaf_at(SAMPLE_DEPTH, first[0] + (k % pw) as i64, first[1] + (k / pw) as i64));
            patch.z.push(s.h[k]);
        }
        for d in SAMPLE_DEPTH + 1..=depth {
            patch = finer(map, lat, &patch, cover(win, depth, d, extent));
        }
        patch
    }
}

/// The leaves of the depth below `p`'s over the rectangle of them from `x0`, `y0` to `x1`, `y1` (inside `p`'s quarters),
/// grown to whole groups of four: each quarter of a leaf of `p` takes the leaf's height and its lean
/// (`lean_quarters`; a neighbour off the map is the nearest leaf on it), so each leaf's quarters keep its mean.
fn finer(map: &Square, lat: (f64, f64), p: &Patch, [x0, y0, x1, y1]: [i64; 4]) -> Patch {
    let (x0, y0, x1, y1) = (x0 & !1, y0 & !1, x1 | 1, y1 | 1);
    let (cw, ch) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize);
    let mut out = Patch { depth: p.depth + 1, x: x0, y: y0, w: cw, h: ch, z: vec![0.0; cw * ch] };
    let at = |cx: i64, cy: i64| (cy - y0) as usize * cw + (cx - x0) as usize;
    for py in y0 >> 1..=y1 >> 1 {
        for px in x0 >> 1..=x1 >> 1 {
            let k = p.near(px, py);
            let a = leaf_at(p.depth, px, py).quarters().map(|q| map.place_of(&q, lat.0, lat.1).0);
            let d = lean_quarters(|dx, dy| p.z[p.near(px + dx, py + dy)], a);
            for (q, dq) in d.iter().enumerate() {
                out.z[at(2 * px + (q % 2) as i64, 2 * py + (q / 2) as i64)] = p.z[k] + dq;
            }
        }
    }
    out
}
