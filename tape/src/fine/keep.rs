//! One field over a window's pixels from what its cells hold: continuous across the cells' borders, and holding each
//! cell's mean over its pixels (pycnophylactic, Tobler 1979). It starts from a field the caller makes, the general law
//! that shares a cell's mean over its ground evaluated at each pixel on the cells' smooth field (`pyramid`), so what
//! varies inside a cell comes from that law and what varies between cells is smooth. Round after round, what each cell
//! still lacks (its mean over what the field holds there, or the difference) is laid on the field as a smooth field of
//! its own (the same pyramid), so the field stays smooth while the cells' means close in; the last round lays each
//! cell's own remainder on it alone, which holds every mean and leaves at the borders only what the rounds did not
//! close. Each correction is bounded (`MOST`), so a cell whose law leaves it almost empty cannot drive its neighbours.

use super::pyramid::Pyramid;

/// Rounds of smooth corrections: each closes most of what the last left.
const ROUNDS: usize = 4;
/// A round's correction of a field that scales scales it by no more than this, and by no less than its inverse.
const MOST: f64 = 4.0;

/// How a field is corrected toward its cells' means: scaled (a field of none or more, as an amount per m²: what a
/// cell holds goes where its law puts it) or shifted (a field such as a temperature).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Mean {
    Scaled,
    Shifted,
}

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

    /// What cell `c` of field `v` lacks of its `target`: the ratio (bounded by `MOST`; 1 for a cell whose field holds
    /// none, which the last round fills evenly) or the difference.
    fn lacks(&self, c: usize, v: &[f64], target: f64, how: Mean) -> f64 {
        let now = self.mean(c, |p| v[p]);
        match how {
            Mean::Shifted => target - now,
            Mean::Scaled if now > 0.0 => (target / now).clamp(1.0 / MOST, MOST),
            Mean::Scaled => 1.0,
        }
    }

    /// Field `v` over the pixels, corrected so each cell's mean is its `target` (the module's doc).
    pub(super) fn keep(&self, pyramid: &Pyramid, mut v: Vec<f64>, target: &[f64], how: Mean) -> Vec<f64> {
        let neutral = if how == Mean::Scaled { 1.0 } else { 0.0 };
        let shown = || (0..self.pixels.len()).filter(|&c| !self.pixels[c].is_empty());
        for _ in 0..ROUNDS {
            let mut all = vec![neutral; self.n];
            shown().for_each(|c| all[self.cells[c]] = self.lacks(c, &v, target[c], how));
            let field = pyramid.field(&all);
            match how {
                Mean::Scaled => v.iter_mut().zip(field).for_each(|(v, f)| *v *= f.clamp(1.0 / MOST, MOST)),
                Mean::Shifted => v.iter_mut().zip(field).for_each(|(v, f)| *v += f),
            }
        }
        for c in shown() {
            let now = self.mean(c, |p| v[p]);
            match how {
                Mean::Shifted => self.pixels[c].iter().for_each(|&p| v[p] += target[c] - now),
                Mean::Scaled if now > 0.0 => self.pixels[c].iter().for_each(|&p| v[p] *= target[c] / now),
                Mean::Scaled => self.pixels[c].iter().for_each(|&p| v[p] = target[c]),
            }
        }
        v
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
