//! The grid the terrain's samples lie on (laws.md 0): `across` x `down` samples row by row from the north-west, those of
//! each row `dx` m wide and all `dy` m long, east and west round a globe. The samples (`space::samples`), their flood
//! (`space::drain`) and the law that wears the ground at the world's start (`space::wear`) read it, on the samples'
//! own grid and on one a few times coarser (`Grid::coarser`).

use super::below::{lean_quarters, limited, Near};

/// The steps to the 8 neighbours of a sample, rows and columns.
const STEPS: [(i64, i64); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];

/// A grid of samples, and of each row the distance to its neighbour by each of `STEPS`, m: between the centres, the
/// width the mean of the two rows'.
#[derive(Clone, Debug)]
pub(super) struct Grid {
    pub(super) across: usize,
    pub(super) down: usize,
    pub(super) wrap: bool,
    pub(super) dy: f64,
    pub(super) dx: Vec<f64>,
    far: Vec<[f64; 8]>,
}

/// The distance between the centres of two samples `a` rows and `b` columns apart, m, in rows `dx` wide and `dy` long:
/// along the rows by the mean of the two rows' widths.
fn length(dx: [f64; 2], dy: f64, a: i64, b: i64) -> f64 {
    ((dx[0] + dx[1]) / 2.0 * b as f64).hypot(dy * a as f64)
}

impl Grid {
    /// A grid `across` x `down` of samples `dy` m long, those of row r `dx[r]` m wide, round a globe if `wrap`.
    pub(super) fn new(across: usize, down: usize, wrap: bool, dy: f64, dx: Vec<f64>) -> Grid {
        let far = (0..down)
            .map(|r| {
                STEPS.map(|(a, b)| {
                    let r2 = (r as i64 + a).clamp(0, down as i64 - 1) as usize;
                    length([dx[r], dx[r2]], dy, a, b)
                })
            })
            .collect();
        Grid { across, down, wrap, dy, dx, far }
    }

    /// The distance between the centres of samples `k` and `j`, m (`length`; the shorter way round a globe).
    pub(super) fn apart(&self, k: usize, j: usize) -> f64 {
        let (r, rj) = (k / self.across, j / self.across);
        let mut b = (j % self.across) as i64 - (k % self.across) as i64;
        if self.wrap && 2 * b.abs() > self.across as i64 {
            b -= b.signum() * self.across as i64;
        }
        length([self.dx[r], self.dx[rj]], self.dy, rj as i64 - r as i64, b)
    }

    /// The number of samples.
    pub(super) fn n(&self) -> usize {
        self.across * self.down
    }

    /// The area of sample `k`, m².
    pub(super) fn area(&self, k: usize) -> f64 {
        self.dx[k / self.across] * self.dy
    }

    /// The sample at row `r` and column `c` (round a globe), if on the grid.
    fn at(&self, r: i64, c: i64) -> Option<usize> {
        let c = if self.wrap { c.rem_euclid(self.across as i64) } else { c };
        ((0..self.down as i64).contains(&r) && (0..self.across as i64).contains(&c)).then(|| r as usize * self.across + c as usize)
    }

    /// The neighbours of sample `k` across its sides and corners, each with whether it lies across a corner and its
    /// distance, m.
    pub(super) fn ways(&self, k: usize) -> impl Iterator<Item = (usize, bool, f64)> + '_ {
        let (r, c, wide) = ((k / self.across) as i64, (k % self.across) as i64, self.across as i64);
        let inside = r > 0 && r + 1 < self.down as i64 && c > 0 && c + 1 < wide;
        let far = &self.far[r as usize];
        STEPS.iter().zip(far).filter_map(move |(&(a, b), &d)| {
            let j = if inside { Some((k as i64 + a * wide + b) as usize) } else { self.at(r + a, c + b) };
            j.map(|j| (j, a != 0 && b != 0, d))
        })
    }

    /// The two samples beside both sample `k` and its neighbour `j` across a corner, between which the way from one to
    /// the other passes.
    pub(super) fn corner_sides(&self, k: usize, j: usize) -> [usize; 2] {
        let w = self.across as i64;
        let (r, c, rj, cj) = ((k / self.across) as i64, (k % self.across) as i64, (j / self.across) as i64, (j % self.across) as i64);
        let wrapped = |c: i64| c.rem_euclid(w) as usize;
        [r as usize * self.across + wrapped(cj), rj as usize * self.across + wrapped(c)]
    }

    /// The neighbours of sample `k` across its sides and corners, each with its distance, m.
    pub(super) fn around(&self, k: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.ways(k).map(|(j, _, d)| (j, d))
    }

    /// The neighbours of sample `k` east, west, north and south, each with the length it lies away, m; one off the
    /// grid is the sample itself.
    pub(super) fn sides(&self, k: usize) -> [(usize, f64); 4] {
        let (r, c) = ((k / self.across) as i64, (k % self.across) as i64);
        let or_own = |j: Option<usize>| j.unwrap_or(k);
        let dx = self.dx[r as usize];
        [(or_own(self.at(r, c + 1)), dx), (or_own(self.at(r, c - 1)), dx), (or_own(self.at(r - 1, c)), self.dy), (or_own(self.at(r + 1, c)), self.dy)]
    }

    /// The mean of `x` over the square of samples of side 2r + 1 about each sample, r the samples in `half` m (by its
    /// length north to south), the part of the square off the map left out (round a globe it wraps): two passes of
    /// sums run along the rows and then down the columns.
    pub(super) fn spread(&self, x: &[f64], half: f64) -> Vec<f64> {
        let r = (half / self.dy).round() as i64;
        let (wide, high) = (self.across as i64, self.down as i64);
        let along = |n: i64, wrap: bool, get: &dyn Fn(i64) -> f64| -> Vec<f64> {
            let wraps = wrap && 2 * r + 1 < n;
            let from = if wraps { -r } else { 0 };
            let mut sums = vec![0.0];
            for j in from..if wraps { n + r } else { n } {
                sums.push(sums[sums.len() - 1] + get(j.rem_euclid(n)));
            }
            let span = |i: i64| if wraps { (i - r, i + r) } else { ((i - r).max(0), (i + r).min(n - 1)) };
            (0..n).map(span).map(|(lo, hi)| (sums[(hi + 1 - from) as usize] - sums[(lo - from) as usize]) / (hi - lo + 1) as f64).collect()
        };
        let rows: Vec<f64> = (0..high).flat_map(|y| along(wide, self.wrap, &|c| x[(y * wide + c) as usize])).collect();
        let mut out = vec![0.0; self.n()];
        for c in 0..wide {
            for (y, v) in along(high, false, &|y| rows[(y * wide + c) as usize]).into_iter().enumerate() {
                out[y * self.across + c as usize] = v;
            }
        }
        out
    }

    /// The grid of `by` x `by` samples to one (`by` divides both sides): a row of it as wide as `by` samples of the
    /// rows it spans are on their mean.
    pub(super) fn coarser(&self, by: usize) -> Grid {
        let dx = self.dx.chunks(by).map(|rows| rows.iter().sum::<f64>()).collect();
        Grid::new(self.across / by, self.down / by, self.wrap, self.dy * by as f64, dx)
    }

    /// The mean by area of `x` over each sample of the grid `by` times coarser (`coarser`).
    pub(super) fn mean_over(&self, x: &[f64], by: usize) -> Vec<f64> {
        let (across, down) = (self.across / by, self.down / by);
        let mut sum = vec![0.0; across * down];
        let mut area = vec![0.0; across * down];
        for (k, xk) in x.iter().enumerate() {
            let c = (k / self.across / by) * across + (k % self.across) / by;
            sum[c] += self.area(k) * xk;
            area[c] += self.area(k);
        }
        sum.iter().zip(&area).map(|(s, a)| s / a).collect()
    }

    /// Heights `z` on the grid `from` times coarser than this one carried smooth to the grid `to` times coarser, a halving
    /// at a time (`finer`).
    pub(super) fn carried(&self, (mut from, z): (usize, &[f64]), to: usize) -> Vec<f64> {
        let mut z = z.to_vec();
        while from > to {
            z = self.coarser(from).finer(&z, &self.coarser(from / 2).dx);
            from /= 2;
        }
        z
    }

    /// The heights `z` of this grid's samples carried to the grid twice as fine along each side (twice the `across`
    /// and the `down`, its rows `dx`): each sample's quarters take its mean and the smooth lean through it and its
    /// neighbours (`below::lean_quarters`, `below::limited`), so the finer heights have no steps, no new pit and no new peak.
    pub(super) fn finer(&self, z: &[f64], dx: &[f64]) -> Vec<f64> {
        let (r_max, wide) = (self.down as i64 - 1, self.across as i64);
        let at = |r: i64, c: i64| {
            let c = if self.wrap { c.rem_euclid(wide) } else { c.clamp(0, wide - 1) };
            z[r.clamp(0, r_max) as usize * self.across + c as usize]
        };
        let mut out = vec![0.0; 4 * self.n()];
        for (k, zk) in z.iter().enumerate() {
            let (r, c) = ((k / self.across) as i64, (k % self.across) as i64);
            let rows = [dx[2 * r as usize], dx[2 * r as usize + 1]];
            let near: Near = std::array::from_fn(|j| std::array::from_fn(|i| at(r + j as i64 - 2, c + i as i64 - 2)));
            let d = limited(&near, lean_quarters(&near, [rows[0], rows[0], rows[1], rows[1]]));
            for (q, dq) in d.iter().enumerate() {
                let (y, x) = (2 * r as usize + q / 2, 2 * c as usize + q % 2);
                out[y * 2 * self.across + x] = zk + dq;
            }
        }
        out
    }
}
