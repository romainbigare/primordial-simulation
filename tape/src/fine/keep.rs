//! One field over a window's pixels from what its cells hold: continuous across the cells' borders, and holding each
//! cell's mean over its pixels exactly (pycnophylactic interpolation, Tobler 1979). Each pixel's value is a law of a
//! smooth level `u` at that pixel (`keep`'s `law`, rising, or level, as `u` rises): the general law that shares a
//! cell's mean over its ground there (the soil's liquid by the wetness index, a kind of cover by its activity), so what
//! varies inside a cell comes from that law, and what varies between cells from `u`, which has no edge where cells
//! meet. `u` starts from the cells' smooth field (`pyramid`); round after round, the lift each cell needs so its mean
//! holds (Newton's steps) is added to `u` as a smooth field of those lifts (the same pyramid), so `u` stays smooth while
//! the cells' means close in; the last lift, one amount over each cell (halving where Newton's steps do not reach it),
//! holds every mean, and leaves at the borders only what the rounds did not close.

/// Rounds of smooth lifts: each closes most of what the last left.
const ROUNDS: usize = 4;
/// Newton's steps of a cell's lift.
const NEWTON: usize = 2;
/// A cell's mean holds when it is off by no more than this share of the largest of its mean and the law's span.
const HOLDS: f64 = 1e-9;
/// Halvings that find a lift Newton's steps did not, at the end: enough for a lift to the last bit.
const HALVINGS: usize = 64;

use super::pyramid::Pyramid;

/// The pixels of a window and their cells: each its cell's number (`of`) and its area (`area`, a weight: the pixels of
/// a row are alike), with each cell's pixels; and the cell of the world each number is, of the world's `n`.
pub(super) struct Cells {
    of: Vec<u32>,
    cells: Vec<usize>,
    n: usize,
    area: Vec<f64>,
    pixels: Vec<Vec<usize>>,
}

impl Cells {
    /// Pixels `across` x `down` of cells `of` (numbers of the world's cells `cells`, of `n`), the area of a pixel of
    /// each row `row`.
    pub(super) fn new(across: usize, down: usize, (of, cells, n): (&[u32], &[usize], usize), row: &[f64]) -> Cells {
        let mut pixels = vec![Vec::new(); cells.len()];
        of.iter().enumerate().for_each(|(p, &c)| pixels[c as usize].push(p));
        Cells { of: of.to_vec(), cells: cells.to_vec(), n, area: (0..across * down).map(|p| row[p / across]).collect(), pixels }
    }

    /// The mean by area of `f` over the pixels of cell `c`.
    pub(super) fn mean(&self, c: usize, f: impl Fn(usize) -> f64) -> f64 {
        let (sum, of) = self.pixels[c].iter().fold((0.0, 0.0), |(s, a), &p| (s + self.area[p] * f(p), a + self.area[p]));
        sum / of
    }

    /// The lift `u` needs over cell `c` so the mean of `law` over its pixels is `target`: Newton's steps (a slope found
    /// by a small lift), and where they leave it off and `exact`, halving between lifts that fall short and go over.
    fn lift(&self, c: usize, u: &[f64], law: &impl Fn(usize, f64) -> f64, target: f64, exact: bool) -> f64 {
        let span = self.pixels[c].iter().map(|&p| u[p].abs()).fold(0.0, f64::max).max(target.abs()).max(1e-300);
        let off = |by: f64| self.mean(c, |p| law(p, u[p] + by)) - target;
        let mut by = 0.0;
        for _ in 0..NEWTON {
            let now = off(by);
            if now.abs() <= HOLDS * span {
                break;
            }
            let d = span * 1e-6;
            let slope = (off(by + d) - now) / d;
            if slope <= 0.0 {
                break;
            }
            by -= now / slope;
        }
        if exact && off(by).abs() > HOLDS * span { halve(off, span) } else { by }
    }

    /// The values of `law` over the pixels, from the smooth field of the world's cells' `start` (`pyramid`), each cell's
    /// mean its `target` (the module's doc).
    pub(super) fn keep(&self, pyramid: &Pyramid, start: &[f64], law: &impl Fn(usize, f64) -> f64, target: &[f64]) -> Vec<f64> {
        let mut u = pyramid.field(start);
        let smooth = |by: &[f64]| {
            let mut all = vec![0.0; self.n];
            self.cells.iter().zip(by).for_each(|(&i, &b)| all[i] = b);
            pyramid.field(&all)
        };
        let cells = 0..self.pixels.len();
        for _ in 0..ROUNDS {
            let by: Vec<f64> = cells.clone().map(|c| if self.pixels[c].is_empty() { 0.0 } else { self.lift(c, &u, law, target[c], false) }).collect();
            u.iter_mut().zip(smooth(&by)).for_each(|(u, b)| *u += b);
        }
        for c in cells.filter(|&c| !self.pixels[c].is_empty()) {
            let by = self.lift(c, &u, law, target[c], true);
            self.pixels[c].iter().for_each(|&p| u[p] += by);
        }
        (0..u.len()).map(|p| law(p, u[p])).collect()
    }

    /// The number of each pixel's cell.
    pub(super) fn of(&self) -> &[u32] {
        &self.of
    }

    /// The pixels of cell `c`.
    pub(super) fn pixels(&self, c: usize) -> &[usize] {
        &self.pixels[c]
    }
}

/// The lift at which `off` (rising with the lift) is none, by halving between a lift that falls short and one that goes
/// over, found from `span` out; the nearer end where it never crosses.
fn halve(off: impl Fn(f64) -> f64, span: f64) -> f64 {
    let (mut lo, mut hi) = (-span, span);
    for _ in 0..HALVINGS {
        if off(lo) <= 0.0 {
            break;
        }
        lo *= 2.0;
    }
    for _ in 0..HALVINGS {
        if off(hi) >= 0.0 {
            break;
        }
        hi *= 2.0;
    }
    for _ in 0..HALVINGS {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if off(mid) < 0.0 { lo = mid } else { hi = mid }
    }
    0.5 * (lo + hi)
}
