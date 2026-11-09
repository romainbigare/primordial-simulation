//! Fields over a window's pixels from what its cells hold, continuous across the cells' borders. A cell's value is
//! evened over its neighbours' within half its own side (`Cells::even`: a box, along one way and then the other), so
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

    /// Of each pixel, the mean of `value` of the cells that `counted` names over the pixels within half its own cell's
    /// side along each way (a box, along one way and then the other), NaN where none is counted within it.
    pub(super) fn even(&self, value: &[f64], counted: impl Fn(usize) -> bool) -> Vec<f64> {
        let n = self.of.len();
        let cell = |p: usize| self.of[p] as usize;
        let mut sum: Vec<f64> = (0..n).map(|p| if counted(cell(p)) { value[cell(p)] } else { 0.0 }).collect();
        let mut of: Vec<f64> = (0..n).map(|p| if counted(cell(p)) { 1.0 } else { 0.0 }).collect();
        for (step, len) in [(1, self.across), (self.across, n / self.across)] {
            (sum, of) = (self.boxed(&sum, step, len), self.boxed(&of, step, len));
        }
        (0..n).map(|p| if of[p] > 0.0 { sum[p] / of[p] } else { f64::NAN }).collect()
    }

    /// `v` summed over a box along one way: each pixel the sum of the pixels within its own cell's `reach` before and
    /// after it along lines of `len` pixels, `step` apart in `v` (1 along a row, the width down a column).
    fn boxed(&self, v: &[f64], step: usize, len: usize) -> Vec<f64> {
        let mut out = vec![0.0; v.len()];
        let at = |l: usize, k: usize| if step == 1 { l * len + k } else { k * step + l };
        let mut prefix = vec![0.0; len + 1];
        for l in 0..v.len() / len {
            for k in 0..len {
                prefix[k + 1] = prefix[k] + v[at(l, k)];
            }
            for k in 0..len {
                let p = at(l, k);
                let r = self.reach[self.of[p] as usize];
                out[p] = prefix[(k + r + 1).min(len)] - prefix[k.saturating_sub(r)];
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
            let field = self.even(&lacks, shown);
            match how {
                Mean::Scaled => v.iter_mut().zip(field).for_each(|(v, f)| *v *= f),
                Mean::Shifted => v.iter_mut().zip(field).for_each(|(v, f)| *v += f),
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
