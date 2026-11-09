//! The ground below the terrain's samples (laws.md 0 and 8): one function of the seed and the map that every cell's
//! height, relief and level-volume curve and every pixel a shell draws read (`Terrain::heights`, `Terrain::inside`).
//!
//! Each leaf finer than `SAMPLE_DEPTH` departs from its parent's mean by two parts that keep that mean. The first is a
//! smooth continuation of the coarser heights: the parent's quarters lean as the quartic through the parent and two
//! neighbours each way on each axis does (`LEAN`; average interpolation, Donoho 1993), so the ground has no steps and
//! its slope no kinks at the leaves' edges, and a slope across many samples goes on across their quarters. The second
//! is the midpoint displacement of the noise finer than the samples (Fournier, Fussell and Carpenter 1982): draws keyed
//! by the leaf (`displace`), with no part along the lean (`across`), of variance by area G² ℓ² / 16, that of the
//! quarters of flat ground tilted at the slope G the noise or real ground holds at that size (`slope_at`); none on
//! leaves under `DETAIL_LEAST_M` a side, where the ground goes on as its continuation.
//!
//! The variance below a sample is what the lean and the displacement draw below it (`sample_variance`). Each leaf
//! below it holds what its parent held less the variance its quarters' means take about the parent's (`Patch::v`),
//! shared among them by what each will need (`share`), and no split takes more than its leaf holds (the departures
//! shrink to fit, where the ground bends more than its sample's neighbours do), so a leaf's spread of heights is exactly
//! its quarters' and the spread of their means about its own, at every depth. A leaf's height depends on its own place
//! only, whatever window it is found for.

use super::square::Square;
use super::terrain::{displace, slope_at, variance_below, Terrain, SAMPLE_DEPTH};
use super::tree::BLOCK;
use super::Leaf;

/// Leaves of each depth round a window whose heights are found too: a leaf's height reads its parent's variance and
/// the heights of two leaves each way of its parent, and a leaf's variance the heights of three each way of it
/// (`share`), each group of four whole, so a margin of M at a depth needs (M + 1) / 2 + 3 at the one above.
const MARGIN: i64 = 7;
/// The weights of a leaf's neighbours from two before it to two after it along an axis (its own: 0) in the lean of its
/// half after it: that half's mean departs from the leaf's by this sum, its half before by minus it. They are the
/// quartic through the five means (average interpolation, Donoho 1993), whose limit is smooth with its slope; the
/// quadratic through three (±(next − last) / 8) leaves kinks in the slope along the leaves' edges.
const LEAN: [f64; 5] = [3.0 / 128.0, -22.0 / 128.0, 0.0, 22.0 / 128.0, -3.0 / 128.0];
/// The least side of a leaf, m, whose quarters take the terrain's displacement: finer, the ground goes on as its smooth
/// continuation (the shape a metre of ground takes, not grit below it).
const DETAIL_LEAST_M: f64 = 1.0;
/// The variance a slope drawn at one depth holds over it and every depth below, over what it holds at that depth: the
/// sum of 1/4ⁿ, as each depth's leaves are half as long.
const CARRIED: f64 = 4.0 / 3.0;

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

/// The draws `z` (their mean by area none) of quarters of areas `area` with no part along the lean `lean` (by area),
/// scaled so their variance by area is `var`: so the lean's variance and theirs add, whatever they draw.
fn across(z: [f64; 4], lean: &[f64; 4], area: &[f64; 4], var: f64) -> [f64; 4] {
    let dot = |a: &[f64; 4], b: &[f64; 4]| (0..4).map(|q| area[q] * a[q] * b[q]).sum::<f64>();
    let (zl, ll) = (dot(&z, lean), dot(lean, lean));
    let z = if ll > 0.0 { std::array::from_fn(|q| z[q] - zl / ll * lean[q]) } else { z };
    let spread = dot(&z, &z) / area.iter().sum::<f64>();
    if spread > 0.0 { z.map(|x| x * (var / spread).sqrt()) } else { [0.0; 4] }
}

/// The variance by area of the heights below sample `k` of the drained heights `h` (`across` x `down`, row by row) whose
/// noise has the rms slope `g` there, m²: the lean's at the steepest of it and its eight
/// neighbours (the ground between samples bends from one's slope to the next), S² ℓ² / 16 at each depth below it (the
/// variance by area of a lean, ew² + sn² + corners², is that of flat ground tilted at S), and the displacement's whole
/// series at the noise's slope (`variance_below`), each taking a third more as the lean carries the slope each depth
/// draws on to the depths below it (the sum of 1/4ⁿ over the depths below). Neighbours off the map are the nearest on
/// it.
pub(super) fn sample_variance(map: &Square, h: &[f64], (across, down): (usize, usize), k: usize, g: f64) -> f64 {
    let (x, y) = ((k % across) as i64, (k / across) as i64);
    let at = |x: i64, y: i64| h[y.clamp(0, down as i64 - 1) as usize * across + x.clamp(0, across as i64 - 1) as usize];
    let steepest = (-1..=1i64)
        .flat_map(|j| (-1..=1i64).map(move |i| (i, j)))
        .map(|(i, j)| {
            let [ew, sn, corners] = leans(|dx, dy| at(x + i + dx, y + j + dy));
            ew * ew + sn * sn + corners * corners
        })
        .fold(0.0, f64::max);
    CARRIED * (CARRIED * steepest + variance_below(map, g, SAMPLE_DEPTH))
}

/// The leaves of one depth over a rectangle of them (from column `x` and row `y`, in leaves of that depth), row by
/// row: each one's mean height, m, the rms slope of the noise at its sample, and the variance by area of the heights
/// below it, m² (`sample_variance` at a sample).
struct Patch {
    depth: u8,
    x: i64,
    y: i64,
    w: usize,
    h: usize,
    z: Vec<f64>,
    g: Vec<f64>,
    v: Vec<f64>,
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
    /// mean by area of the drained samples it covers, finer the ground below them (the module's doc).
    pub fn heights(&self, seed: u64, map: &Square, lat: (f64, f64), win: [i64; 4], depth: u8) -> Vec<f64> {
        let (across, down) = ((win[2] - win[0] + 1) as usize, (win[3] - win[1] + 1) as usize);
        let pixel = |p: usize| (win[0] + (p % across) as i64, win[1] + (p / across) as i64);
        if depth <= SAMPLE_DEPTH {
            return (0..across * down).map(pixel).map(|(x, y)| self.inside(seed, map, leaf_at(depth, x, y), lat).0).collect();
        }
        let p = self.patch(seed, map, lat, win, depth);
        (0..across * down).map(pixel).map(|(x, y)| p.z[p.near(x, y)]).collect()
    }

    /// The terrain inside each quarter of leaf `l`, row by row (`Terrain::inside`), found together.
    pub fn quarters(&self, seed: u64, map: &Square, l: Leaf, lat: (f64, f64)) -> [(f64, f64); 4] {
        if l.depth >= SAMPLE_DEPTH {
            let (x, y) = (2 * (l.x / l.side()) as i64, 2 * (l.y / l.side()) as i64);
            let p = self.patch(seed, map, lat, [x, y, x + 1, y + 1], l.depth + 1);
            return std::array::from_fn(|q| {
                let k = p.near(x + (q % 2) as i64, y + (q / 2) as i64);
                (p.z[k], map.place_of(&l.quarters()[q], lat.0, lat.1).0 * p.v[k] / 2.0)
            });
        }
        l.quarters().map(|q| self.inside(seed, map, q, lat))
    }

    /// Leaf `l`, finer than `SAMPLE_DEPTH`: its mean height, m, and the variance by area of the heights below it, m².
    pub(super) fn below(&self, seed: u64, map: &Square, l: Leaf, lat: (f64, f64)) -> (f64, f64) {
        let (x, y) = ((l.x / l.side()) as i64, (l.y / l.side()) as i64);
        let p = self.patch(seed, map, lat, [x, y, x, y], l.depth);
        let k = p.near(x, y);
        (p.z[k], p.v[k])
    }

    /// The patch of leaves of `depth` (finer than `SAMPLE_DEPTH`) over the window `win` and its margin, from the drained
    /// samples down, a depth at a time (`finer`).
    fn patch(&self, seed: u64, map: &Square, lat: (f64, f64), win: [i64; 4], depth: u8) -> Patch {
        let s = self.samples(seed, map, lat);
        let shift = depth - map.square_depth();
        let extent = ((map.w as i64) << shift, (map.h as i64) << shift);
        let first = cover(win, depth, SAMPLE_DEPTH, extent);
        let (pw, ph) = ((first[2] - first[0] + 1) as usize, (first[3] - first[1] + 1) as usize);
        let mut patch = Patch { depth: SAMPLE_DEPTH, x: first[0], y: first[1], w: pw, h: ph, z: Vec::with_capacity(pw * ph), g: Vec::with_capacity(pw * ph), v: Vec::with_capacity(pw * ph) };
        for k in 0..pw * ph {
            let k = s.index(&leaf_at(SAMPLE_DEPTH, first[0] + (k % pw) as i64, first[1] + (k / pw) as i64));
            patch.z.push(s.h[k]);
            patch.g.push(s.g[k] as f64);
            patch.v.push(s.v[k]);
        }
        for d in SAMPLE_DEPTH + 1..=depth {
            patch = self.finer(seed, map, lat, &patch, cover(win, depth, d, extent));
        }
        patch
    }

    /// The leaves of the depth below `p`'s over the rectangle of them from `x0`, `y0` to `x1`, `y1` (inside `p`'s
    /// quarters), grown to whole groups of four: each quarter of a leaf of `p` takes the leaf's height, its lean
    /// (`LEAN`; across both ways, the product of the two leans; a neighbour off the map is the nearest leaf on it) less
    /// that lean's mean by area, and the displacement, so each leaf's quarters keep its mean (`split`); then the
    /// leaf's variance less what their means take, shared among them by what each will need (`share`).
    fn finer(&self, seed: u64, map: &Square, lat: (f64, f64), p: &Patch, [x0, y0, x1, y1]: [i64; 4]) -> Patch {
        let (x0, y0, x1, y1) = (x0 & !1, y0 & !1, x1 | 1, y1 | 1);
        let (cw, ch) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize);
        let mut out = Patch { depth: p.depth + 1, x: x0, y: y0, w: cw, h: ch, z: vec![0.0; cw * ch], g: vec![0.0; cw * ch], v: vec![0.0; cw * ch] };
        let mut area = vec![0.0; cw * ch];
        let at = |cx: i64, cy: i64| (cy - y0) as usize * cw + (cx - x0) as usize;
        for py in y0 >> 1..=y1 >> 1 {
            for px in x0 >> 1..=x1 >> 1 {
                let (a, d, left) = self.split(seed, map, lat, p, (px, py));
                let g = p.g[p.near(px, py)];
                for q in 0..4 {
                    let c = at(2 * px + (q % 2) as i64, 2 * py + (q / 2) as i64);
                    (out.z[c], out.g[c], out.v[c], area[c]) = (p.z[p.near(px, py)] + d[q], g, left, a[q]);
                }
            }
        }
        share(map, &mut out, &area);
        out
    }

    /// The departures of the quarters of the leaf (`px`, `py`) of `p` from its mean (row by row), with their areas, m²,
    /// and the variance left below them, m²: the lean, shrunk where it alone would take more than the leaf holds, and
    /// the draws (none along the lean, `across`) of the displacement's variance, no more than the leaf holds besides.
    fn split(&self, seed: u64, map: &Square, lat: (f64, f64), p: &Patch, (px, py): (i64, i64)) -> ([f64; 4], [f64; 4], f64) {
        let k = p.near(px, py);
        let (g, v, len) = (p.g[k], p.v[k], map.length(p.depth));
        let [ew, sn, corners] = leans(|dx, dy| p.z[p.near(px + dx, py + dy)]);
        let parent = leaf_at(p.depth, px, py);
        let area = parent.quarters().map(|q| map.place_of(&q, lat.0, lat.1).0);
        let whole: f64 = area.iter().sum();
        let lean: [f64; 4] = std::array::from_fn(|q| {
            let (sx, sy) = (if q % 2 == 1 { 1.0 } else { -1.0 }, if q >= 2 { 1.0 } else { -1.0 });
            sx * ew + sy * sn + sx * sy * corners
        });
        let mean = (0..4).map(|q| area[q] * lean[q]).sum::<f64>() / whole;
        let var = |d: &[f64; 4]| (0..4).map(|q| area[q] * d[q] * d[q]).sum::<f64>() / whole;
        let smooth = lean.map(|l| l - mean);
        let bent = var(&smooth);
        let smooth = if bent > v { smooth.map(|x| x * (v / bent).sqrt()) } else { smooth };
        let want = (slope_at(g, len) * len / 4.0).powi(2).min(v - var(&smooth));
        let noise = if map.length(p.depth + 1) >= DETAIL_LEAST_M && want > 0.0 { across(displace(seed, parent, area, 1.0), &smooth, &area, want) } else { [0.0; 4] };
        let d: [f64; 4] = std::array::from_fn(|q| smooth[q] + noise[q]);
        (area, d, (v - var(&d)).max(0.0))
    }
}

/// The variance each group of four leaves of `out` (areas `area`, m²) holds below their parent's departures (in
/// `out.v`, the same for the four) shared among them by what each will need: the variance its own lean takes (the
/// leans of its neighbours' heights in `out`) and the displacement's at its size. So a leaf on steep or
/// bending ground holds more than one on even ground, and the four hold together what their parent left.
fn share(map: &Square, out: &mut Patch, area: &[f64]) {
    let len = map.length(out.depth);
    let need: Vec<f64> = (0..out.z.len())
        .map(|c| {
            let (x, y) = (out.x + (c % out.w) as i64, out.y + (c / out.w) as i64);
            let [ew, sn, corners] = leans(|dx, dy| out.z[out.near(x + dx, y + dy)]);
            ew * ew + sn * sn + corners * corners + (slope_at(out.g[c], len) * len / 4.0).powi(2)
        })
        .collect();
    for gy in 0..out.h / 2 {
        for gx in 0..out.w / 2 {
            let four: [usize; 4] = std::array::from_fn(|q| (2 * gy + q / 2) * out.w + 2 * gx + q % 2);
            let whole: f64 = four.iter().map(|&c| area[c]).sum();
            let mean = four.iter().map(|&c| area[c] * need[c]).sum::<f64>() / whole;
            let left = out.v[four[0]];
            four.iter().for_each(|&c| out.v[c] = if mean > 0.0 { left * need[c] / mean } else { left });
        }
    }
}
