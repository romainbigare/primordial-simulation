//! Values of the cells blended between their centres, so no cell's edge shows: each cell weighs a point by the quadratic
//! B-spline (`spline`) of its distance from its centre along each axis, in its own sides, and a point takes the weighted
//! mean of the cells that reach it (Shepard 1968). Where the cells are of one size the weights sum to one by themselves;
//! where they differ (by one depth at most, 2:1) the blend stays smooth, changing over the finer cells' width. The blend is continuous with its slope, so hill
//! shading shows no creases and a field's edges are curves, not the cells' edges.

use pcore::World;

/// The most cells a point blends between.
const MOST: usize = 128;
/// How many of its sides a cell's spline reaches from its centre.
const REACH: f64 = 1.5;
/// The points along each axis of a cell that its mean of a blended value is taken at, in halves of its side from its
/// centre, and their weights: the three-point Gauss-Legendre rule, exact for the quadratic a blend is along an axis
/// inside a cell among cells of one size.
const GAUSS: [(f64, f64); 3] = [(-0.774_596_669_241_483_4, 5.0 / 18.0), (0.0, 8.0 / 18.0), (0.774_596_669_241_483_4, 5.0 / 18.0)];

/// The quadratic B-spline at `t` from its centre: 3/4 − t² within 1/2, (3/2 − |t|)² / 2 out to `REACH`.
fn spline(t: f64) -> f64 {
    let t = t.abs();
    if t < 0.5 { 0.75 - t * t } else { 0.5 * (REACH - t).max(0.0).powi(2) }
}

/// The cells a point blends between, and their weights (summing to 1).
pub(super) struct Weights {
    pub cell: [u32; MOST],
    pub wt: [f64; MOST],
    pub n: usize,
}

impl Weights {
    /// The blend of the cells' values `v`.
    pub fn mix(&self, v: &[f64]) -> f64 {
        (0..self.n).map(|k| self.wt[k] * v[self.cell[k] as usize]).sum()
    }
    /// The share of the blend's weight of the cells for which `is` holds.
    pub fn share(&self, is: impl Fn(usize) -> bool) -> f64 {
        (0..self.n).filter(|&k| is(self.cell[k] as usize)).map(|k| self.wt[k]).sum()
    }
}

/// Each cell's centre and side, in squares, the cells that touch it (itself first), and the cells that reach it (itself
/// first).
pub(super) struct Blend {
    pub centre: Vec<[f64; 2]>,
    side: Vec<f64>,
    start: Vec<u32>,
    touch: Vec<u32>,
    reach: Vec<Vec<u32>>,
    /// The map's width in squares where its rows wrap east to west
    wrap: Option<f64>,
}

impl Blend {
    pub fn new(w: &World) -> Blend {
        let (n, tree) = (w.mesh.cells().n(), w.mesh.tree());
        let bounds: Vec<[f64; 4]> = (0..n).map(|i| w.map.bounds(i)).collect();
        let (mut start, mut touch) = (vec![0u32], Vec::new());
        for (i, &[x0, y0, x1, y1]) in bounds.iter().enumerate() {
            // Points just outside each side at a quarter and three quarters along it, and just outside each corner: each
            // lies in a cell that touches this one, and every such cell holds one (cells across a side are at most half as
            // wide).
            let (s, e) = (x1 - x0, (x1 - x0) / 8.0);
            let (q1, q3) = (s / 4.0, 3.0 * s / 4.0);
            let at = [
                (x1 + e, y0 + q1), (x1 + e, y0 + q3), (x0 - e, y0 + q1), (x0 - e, y0 + q3),
                (x0 + q1, y0 - e), (x0 + q3, y0 - e), (x0 + q1, y1 + e), (x0 + q3, y1 + e),
                (x0 - e, y0 - e), (x1 + e, y0 - e), (x0 - e, y1 + e), (x1 + e, y1 + e),
            ];
            touch.push(i as u32);
            let from = touch.len();
            for (x, y) in at {
                let off = y < 0.0 || y >= w.map.h as f64 || !w.map.wrap && (x < 0.0 || x >= w.map.w as f64);
                let c = if off { None } else { Some(tree.cell_at(w.map.point_at(x, y)) as u32) };
                if let Some(c) = c.filter(|&c| c as usize != i && !touch[from..].contains(&c)) {
                    touch.push(c);
                }
            }
            start.push(touch.len() as u32);
        }
        let centre = bounds.iter().map(|&[a, b, c, d]| [(a + c) / 2.0, (b + d) / 2.0]).collect();
        let side = bounds.iter().map(|b| b[2] - b[0]).collect();
        let mut b = Blend { centre, side, start, touch, reach: Vec::new(), wrap: w.map.wrap.then_some(w.map.w as f64) };
        let (widest, mut seen) = (b.side.iter().fold(0.0, |a: f64, &s| a.max(s)), vec![u32::MAX; n]);
        b.reach = (0..n).map(|i| b.reaching(i, widest, &mut seen)).collect();
        b
    }
    /// Whether the spline of cell `c`, widened by `more` squares, reaches cell `i`.
    fn reaches(&self, c: usize, i: usize, more: f64) -> bool {
        let [x, y] = self.centre[i];
        let (dx, dy) = self.way(c, x, y);
        let span = REACH * self.side[c] + self.side[i] / 2.0 + more;
        dx.abs() < span && dy.abs() < span
    }
    /// The cells whose splines reach cell `i`, itself first: found among the cells that touch, from cell `i` on, while the
    /// widest spline (of a cell `widest` squares a side) could reach them; `seen` marks each cell found by the cell it was
    /// found for.
    fn reaching(&self, i: usize, widest: f64, seen: &mut [u32]) -> Vec<u32> {
        let (mut todo, mut out) = (vec![i as u32], vec![i as u32]);
        seen[i] = i as u32;
        while let Some(c) = todo.pop() {
            for &d in &self.touch[self.start[c as usize] as usize..self.start[c as usize + 1] as usize] {
                if seen[d as usize] != i as u32 && self.reaches(i, d as usize, (REACH + 0.5) * widest) {
                    seen[d as usize] = i as u32;
                    todo.push(d);
                    if self.reaches(d as usize, i, 0.0) {
                        out.push(d);
                    }
                }
            }
        }
        out
    }
    /// The cells that reach a point of cell `i`, itself first.
    pub fn near(&self, i: usize) -> &[u32] {
        &self.reach[i]
    }
    /// The way from cell `i`'s centre to the point (`x`, `y`) east and south, in squares (on a globe the near way round).
    fn way(&self, i: usize, x: f64, y: f64) -> (f64, f64) {
        let dx = x - self.centre[i][0];
        (self.wrap.map_or(dx, |wide| dx - wide * (dx / wide).round()), y - self.centre[i][1])
    }
    /// The blend at the point (`x`, `y`) of cell `i`.
    pub fn at(&self, i: usize, x: f64, y: f64) -> Weights {
        let mut out = Weights { cell: [0; MOST], wt: [0.0; MOST], n: 0 };
        let mut sum = 0.0;
        for &c in self.near(i) {
            let ((dx, dy), s) = (self.way(c as usize, x, y), self.side[c as usize]);
            let v = spline(dx / s) * spline(dy / s);
            if v > 0.0 {
                assert!(out.n < MOST, "a point blends between at most {MOST} cells");
                (out.cell[out.n], out.wt[out.n], out.n, sum) = (c, v, out.n + 1, sum + v);
            }
        }
        out.wt[..out.n].iter_mut().for_each(|v| *v /= sum);
        out
    }
    /// The mean over cell `i` of the blend, as weights of the cells that reach it (`GAUSS` along each axis).
    pub fn mean(&self, i: usize) -> Weights {
        let (near, [cx, cy], half) = (self.near(i), self.centre[i], self.side[i] / 2.0);
        let mut sum = vec![0.0; near.len()];
        for (u, wu) in GAUSS {
            for (v, wv) in GAUSS {
                let b = self.at(i, cx + u * half, cy + v * half);
                for q in 0..b.n {
                    sum[near.iter().position(|&c| c == b.cell[q]).expect("a cell that reaches it")] += wu * wv * b.wt[q];
                }
            }
        }
        let mut out = Weights { cell: [0; MOST], wt: [0.0; MOST], n: 0 };
        for (&c, &v) in near.iter().zip(&sum).filter(|(_, &v)| v > 0.0) {
            (out.cell[out.n], out.wt[out.n], out.n) = (c, v, out.n + 1);
        }
        out
    }
}
