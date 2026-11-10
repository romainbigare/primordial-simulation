//! Fields over a window's pixels from what its cells hold, continuous across the cells' borders. A cell's value is
//! evened over its neighbours' within half its own side (`Cells::even`: a box about each pixel), so
//! the field goes from one cell's value to the next over the width of the cells, never past the values it evens, and
//! a cell no wider than a pixel keeps its own. A field that holds each cell's mean over its pixels (pycnophylactic,
//! Tobler 1979; `Cells::keep`) starts from the general law that shares a cell's mean over its ground evaluated at each
//! pixel on such an evened field, so what varies inside a cell comes from that law and what varies between cells is
//! smooth. Round after round, what each cell still lacks (its mean over what the field holds there, or the
//! difference) is laid on the field evened the same way, so the field stays smooth while the cells' means close in. What
//! the rounds do not close is left: a cell's own remainder laid on it alone would step at its borders. Each correction
//! is bounded (`MOST`).

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

/// The pixels of a window and their cells: `across` pixels a row, each its cell's number (`of`) and its area (`area`, a
/// weight: the pixels of a row are alike), with each cell's pixels and half its side in pixels (`reach`, at least one).
pub(super) struct Cells {
    across: usize,
    of: Vec<u32>,
    area: Vec<f64>,
    pixels: Vec<Vec<usize>>,
    reach: Vec<usize>,
}

impl Cells {
    /// Pixels `across` x `down` of cells `of` (numbers from 0 to `count`), each cell `side` pixels a side, the area of a
    /// pixel of each row `row`.
    pub(super) fn new(across: usize, down: usize, (of, side): (&[u32], &[f64]), row: &[f64]) -> Cells {
        let mut pixels = vec![Vec::new(); side.len()];
        of.iter().enumerate().for_each(|(p, &c)| pixels[c as usize].push(p));
        let reach = side.iter().map(|s| ((s / 2.0) as usize).max(1)).collect();
        Cells { across, of: of.to_vec(), area: (0..across * down).map(|p| row[p / across]).collect(), pixels, reach }
    }

    /// The mean by area of `f` over the pixels of cell `c`.
    pub(super) fn mean(&self, c: usize, f: impl Fn(usize) -> f64) -> f64 {
        let (sum, of) = self.pixels[c].iter().fold((0.0, 0.0), |(s, a), &p| (s + self.area[p] * f(p), a + self.area[p]));
        sum / of
    }

    /// Of each pixel, the mean of `value` of the cells over the pixels within half its own cell's side along each way (a
    /// box), each pixel of the box counted where `counts` (the pixel's own cell, the cell of the pixel in the box) says;
    /// NaN where none is counted within it. Each cell's box sums come from a table of sums over its pixels and its reach
    /// (a summed-area table, Crow 1984).
    pub(super) fn even(&self, value: &[f64], counts: impl Fn(usize, usize) -> bool) -> Vec<f64> {
        let (across, down) = (self.across, self.of.len() / self.across);
        let mut out = vec![f64::NAN; self.of.len()];
        let (mut sum, mut of) = (Vec::new(), Vec::new());
        for (c, px) in self.pixels.iter().enumerate() {
            let (Some(&first), Some(&last)) = (px.first(), px.last()) else { continue };
            let r = self.reach[c];
            let (x0, x1) = px.iter().map(|p| p % across).fold((across, 0), |(a, b), x| (a.min(x), b.max(x)));
            let (bx, by) = (x0.saturating_sub(r), (first / across).saturating_sub(r));
            let (ex, ey) = ((x1 + r).min(across - 1), (last / across + r).min(down - 1));
            // The table has a row and a column of none before the box.
            let wide = ex - bx + 2;
            sum.clear();
            sum.resize(wide * (ey - by + 2), 0.0);
            of.clear();
            of.resize(sum.len(), 0.0);
            for y in by..=ey {
                let (mut row, mut row_of) = (0.0, 0.0);
                for x in bx..=ex {
                    let k = self.of[y * across + x] as usize;
                    if counts(c, k) {
                        (row, row_of) = (row + value[k], row_of + 1.0);
                    }
                    let t = (y - by + 1) * wide + x - bx + 1;
                    (sum[t], of[t]) = (sum[t - wide] + row, of[t - wide] + row_of);
                }
            }
            // The sum over the box from column `a` and row `b` to `e` and `f`, inclusive (the table's own columns and rows).
            let boxed = |t: &[f64], (a, b, e, f): (usize, usize, usize, usize)| t[(f + 1) * wide + e + 1] - t[b * wide + e + 1] - t[(f + 1) * wide + a] + t[b * wide + a];
            for &p in px {
                let (x, y) = (p % across - bx, p / across - by);
                let at = (x.saturating_sub(r), y.saturating_sub(r), (x + r).min(ex - bx), (y + r).min(ey - by));
                let n = boxed(&of, at);
                if n > 0.0 {
                    out[p] = boxed(&sum, at) / n;
                }
            }
        }
        out
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

    /// Field `v` over the pixels, corrected toward each cell's mean being its `target` (the module's doc).
    pub(super) fn keep(&self, mut v: Vec<f64>, target: &[f64], how: Mean) -> Vec<f64> {
        let shown = |c: usize| !self.pixels[c].is_empty();
        for _ in 0..ROUNDS {
            let lacks: Vec<f64> = (0..self.pixels.len()).map(|c| if shown(c) { self.lacks(c, &v, target[c], how) } else { 0.0 }).collect();
            let field = self.even(&lacks, |_, k| shown(k));
            match how {
                Mean::Scaled => v.iter_mut().zip(field).for_each(|(v, f)| *v *= f),
                Mean::Shifted => v.iter_mut().zip(field).for_each(|(v, f)| *v += f),
            }
        }
        // A shifted field (a temperature) whose rounds leave little takes each cell's own remainder: a step under what
        // it is drawn in.
        if how == Mean::Shifted {
            for c in (0..self.pixels.len()).filter(|&c| shown(c)) {
                let off = target[c] - self.mean(c, |p| v[p]);
                self.pixels[c].iter().for_each(|&p| v[p] += off);
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
