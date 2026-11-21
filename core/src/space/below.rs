//! The ground below the terrain's samples (laws.md 0 and 8): one function of the seed and the map that every cell's
//! height and level-volume curve and every pixel a shell draws read (`Terrain::heights_on`, `Terrain::inside`).
//!
//! Each leaf finer than `SAMPLE_DEPTH` departs from its parent's mean by a smooth continuation of the coarser heights
//! that keeps that mean: the parent's quarters lean as the quartic through the parent and two neighbours each way on
//! each axis does (`LEAN`; average interpolation, Donoho 1993), so the ground has no steps and its slope no kinks at
//! the leaves' edges, and a slope across many samples goes on across their quarters. Nothing random is laid on it: the
//! worn samples (`space::wear`) hold the ground's shape. A leaf's height depends on its own place only, whatever window
//! it is found for.

use super::samples::Samples;
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

/// The heights of a leaf and of its neighbours two each way, row by row from the north-west: the leaf's at [2][2].
pub(super) type Near = [[f64; 5]; 5];

/// The lean along each axis and across both of the quarters of a leaf, from the heights `n` of it and its neighbours
/// (`LEAN`).
fn leans(n: &Near) -> [f64; 3] {
    let row = |j: usize| (0..5).map(|i| LEAN[i] * n[j][i]).sum::<f64>();
    [row(2), (0..5).map(|j| LEAN[j] * n[j][2]).sum::<f64>(), (0..5).map(|j| LEAN[j] * row(j)).sum::<f64>()]
}

/// The departures of the means of the quarters (of areas `area`, m², row by row) of a leaf from its mean, m, from the
/// heights `n` of it and its neighbours: their lean (`leans`; across both ways, the product of the two leans) less the
/// lean's mean by area, so the quarters keep the leaf's mean.
pub(super) fn lean_quarters(n: &Near, area: [f64; 4]) -> [f64; 4] {
    let [ew, sn, corners] = leans(n);
    let lean: [f64; 4] = std::array::from_fn(|q| {
        let (sx, sy) = (if q % 2 == 1 { 1.0 } else { -1.0 }, if q >= 2 { 1.0 } else { -1.0 });
        sx * ew + sy * sn + sx * sy * corners
    });
    let mean = (0..4).map(|q| area[q] * lean[q]).sum::<f64>() / area.iter().sum::<f64>();
    lean.map(|l| l - mean)
}

/// The lean `lean` of a leaf's quarters (`lean_quarters`) scaled down where a quarter would lie past the lowest or the
/// highest of the leaf and its eight neighbours, of heights `n` (Barth and Jespersen 1989), so the finer heights make no
/// new pit and no new peak: the wear carries its grids so (`Grid::finer`), as a pit made there would hold liquid.
pub(super) fn limited(n: &Near, lean: [f64; 4]) -> [f64; 4] {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for z in n[1..4].iter().flat_map(|row| &row[1..4]) {
        (lo, hi) = (lo.min(*z), hi.max(*z));
    }
    let own = n[2][2];
    if lean.iter().all(|&d| (lo..=hi).contains(&(own + d))) {
        return lean;
    }
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
    /// `x0` and row `y0` to `x1` and `y1`, inclusive, in those leaves), m, row by row, over the samples `s` (the seed's,
    /// or as built: `Terrain::built`): at or above `SAMPLE_DEPTH` the mean by area of the samples it covers, finer the
    /// ground below them (the module's doc).
    pub fn heights_on(&self, s: &Samples, map: &Square, lat: (f64, f64), win: [i64; 4], depth: u8) -> Vec<f64> {
        let (across, down) = ((win[2] - win[0] + 1) as usize, (win[3] - win[1] + 1) as usize);
        let pixel = |p: usize| (win[0] + (p % across) as i64, win[1] + (p / across) as i64);
        if depth <= SAMPLE_DEPTH {
            return (0..across * down).map(pixel).map(|(x, y)| self.over_on(s, map, leaf_at(depth, x, y), lat, |s, k| s.h[k])).collect();
        }
        let p = patch(s, map, lat, win, depth);
        (0..across * down).map(pixel).map(|(x, y)| p.z[p.near(x, y)]).collect()
    }

    /// The mean height of leaf `l`, finer than `SAMPLE_DEPTH`, m.
    pub(super) fn below(&self, seed: u64, map: &Square, l: Leaf, lat: (f64, f64)) -> f64 {
        let (x, y) = ((l.x / l.side()) as i64, (l.y / l.side()) as i64);
        let p = patch(&self.samples(seed, map, lat), map, lat, [x, y, x, y], l.depth);
        p.z[p.near(x, y)]
    }
}

/// The patch of leaves of `depth` (finer than `SAMPLE_DEPTH`) over the window `win` and its margin, from the samples `s`
/// down, a depth at a time (`finer`).
fn patch(s: &Samples, map: &Square, lat: (f64, f64), win: [i64; 4], depth: u8) -> Patch {
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

/// The leaves of the depth below `p`'s over the rectangle of them from `x0`, `y0` to `x1`, `y1` (inside `p`'s quarters),
/// grown to whole groups of four: each quarter of a leaf of `p` takes the leaf's height and its lean
/// (`lean_quarters`; a neighbour off the map is the nearest leaf on it), so each leaf's quarters keep its mean.
fn finer(map: &Square, lat: (f64, f64), p: &Patch, [x0, y0, x1, y1]: [i64; 4]) -> Patch {
    let (x0, y0, x1, y1) = (x0 & !1, y0 & !1, x1 | 1, y1 | 1);
    let (cw, ch) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize);
    let mut out = Patch { depth: p.depth + 1, x: x0, y: y0, w: cw, h: ch, z: vec![0.0; cw * ch] };
    let at = |cx: i64, cy: i64| (cy - y0) as usize * cw + (cx - x0) as usize;
    for py in y0 >> 1..=y1 >> 1 {
        // The areas of the quarters: a leaf's area depends on its row alone.
        let a = leaf_at(p.depth, x0 >> 1, py).quarters().map(|q| map.place_of(&q, lat.0, lat.1).0);
        for px in x0 >> 1..=x1 >> 1 {
            let near: Near = std::array::from_fn(|j| std::array::from_fn(|i| p.z[p.near(px + i as i64 - 2, py + j as i64 - 2)]));
            let d = lean_quarters(&near, a);
            for (q, dq) in d.iter().enumerate() {
                out.z[at(2 * px + (q % 2) as i64, 2 * py + (q / 2) as i64)] = near[2][2] + dq;
            }
        }
    }
    out
}
