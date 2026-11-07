//! Values of the cells drawn as one smooth field, level by level of the tree (a Laplacian pyramid, Burt and Adelson
//! 1983). At the coarsest cells' depth each node's mean is spread as a quadratic B-spline whose mean over each node is
//! that node's value: its coefficients are the values through the inverse of the spline's mean filter (Unser 1999). At
//! each finer depth the same is done with each node's departure from what the coarser levels already draw over it,
//! found for the four children of a node together: their means less their parent's are their own departures, and the
//! coarser field slopes across the parent by its own; the children take as much of the coarser field's slope as their
//! own departures show (none for children that hold their parent's value, as a split leaves them; all for children that
//! slope as the field around them does), and the departure from that slope that is their own.
//!
//! So the field keeps the mean of each node of the coarsest depth, and the mean of each node departs from the field's
//! over its parent by its own departure from its parent's mean, but for the part of the field's slope across the
//! parent that its own value does not show. A cell that holds its parent's value shows the parent's smooth field and
//! no edge of its own, even where the cells beside it took very different values from their own parents; what a cell
//! holds of its own shows at its own scale. The field is continuous with its slope, so hill shading shows no creases,
//! and a pixel's value depends on its place only, whatever window it is found for. Nodes past the map's edge mirror
//! those inside it; a globe's rows wrap.

use pcore::World;
use std::collections::HashMap;

/// The nodes each way along an axis that the inverse of the mean filter reaches: that inverse is √3 rᵐ at m nodes
/// for r = √3 − 2, and |r|⁹ is below 1e-5.
const REACH: i64 = 9;
/// The mean over a node of the B-spline of itself (the middle) and of each neighbour along an axis.
const MEAN: [f64; 3] = [1.0 / 6.0, 2.0 / 3.0, 1.0 / 6.0];
/// No node.
const NONE: u32 = u32::MAX;

/// The inverse of the quadratic B-spline's mean filter (`MEAN`) along an axis, from −`REACH` to `REACH` nodes: √3 rᵐ, r
/// = √3 − 2 the root of r² + 4r + 1 = 0 inside the unit circle.
fn inverse() -> [f64; (2 * REACH + 1) as usize] {
    let (s3, r) = (3f64.sqrt(), 3f64.sqrt() - 2.0);
    std::array::from_fn(|m| s3 * r.powi((m as i64 - REACH).unsigned_abs() as i32))
}

/// The pixels of a window: from column `x` and row `y` of the map's pixels a leaf of `depth` each, `across` x `down`.
pub(super) struct Pixels {
    pub x: i64,
    pub y: i64,
    pub across: usize,
    pub down: usize,
    pub depth: u8,
}

/// A rectangle of nodes of one depth, by its first column and row and its width and height.
#[derive(Clone, Copy)]
struct Rect {
    x: i64,
    y: i64,
    w: usize,
    h: usize,
}

impl Rect {
    /// The nodes from column `x0` and row `y0` to `x1` and `y1`, inclusive.
    fn of(x0: i64, y0: i64, x1: i64, y1: i64) -> Rect {
        Rect { x: x0, y: y0, w: (x1 - x0 + 1) as usize, h: (y1 - y0 + 1) as usize }
    }
    fn end(&self) -> (i64, i64) {
        (self.x + self.w as i64 - 1, self.y + self.h as i64 - 1)
    }
    fn grown(self, by: i64) -> Rect {
        let (x1, y1) = self.end();
        Rect::of(self.x - by, self.y - by, x1 + by, y1 + by)
    }
    /// The nodes of the next depth up that hold it, and one more each way.
    fn parents(self) -> Rect {
        let (x1, y1) = self.end();
        Rect::of(self.x.div_euclid(2) - 1, self.y.div_euclid(2) - 1, x1.div_euclid(2) + 1, y1.div_euclid(2) + 1)
    }
    /// The part of it inside `o`, if any.
    fn within(self, o: Rect) -> Option<Rect> {
        let ((a1, b1), (c1, d1)) = (self.end(), o.end());
        let (x0, y0, x1, y1) = (self.x.max(o.x), self.y.max(o.y), a1.min(c1), b1.min(d1));
        (x1 >= x0 && y1 >= y0).then(|| Rect::of(x0, y0, x1, y1))
    }
    /// The index of node (`x`, `y`) in a grid over it, row by row.
    fn at(&self, x: i64, y: i64) -> usize {
        (y - self.y) as usize * self.w + (x - self.x) as usize
    }
}

/// A level of the pyramid for one window: the nodes whose coefficients it finds (those whose B-splines reach the
/// pixels, or the nodes a finer level reads), the nodes whose departures the inverse filter reads for them (`REACH`
/// more each way, in whole groups of four children) with the index of the node of the tree at each (`NONE`: none, its
/// cells are coarser), and the rectangle about those that are nodes of the tree.
struct Level {
    out: Rect,
    read: Rect,
    node: Vec<u32>,
    held: Option<Rect>,
}

/// The tree's nodes from the coarsest cells' depth down to the window's pixels or the finest cells, whichever is
/// coarser, each with its parent and area, and the node each cell lies in at each depth; and the levels of one window.
pub(super) struct Pyramid {
    parent: Vec<u32>,
    area: Vec<f64>,
    /// (cell, node, the cell's share of the node's area) for each cell and each depth from the coarsest cells' to its
    /// own or the finest drawn
    lies: Vec<(u32, u32, f64)>,
    levels: Vec<Level>,
    px: Pixels,
    /// The pixels a node of the finest level holds along a side, as a power of two
    shift: u8,
}

/// A child in a group of four (`Pyramid::departures`): its place in the grid read, its area, its mean, and the mean
/// over it of the field the coarser levels draw.
struct Kid {
    at: usize,
    area: f64,
    mean: f64,
    drawn: f64,
}

/// Column `j` of a level `n` nodes wide on the map: mirrored about the map's edges, round a globe's rows if `wrap`.
fn on_map(j: i64, n: i64, wrap: bool) -> i64 {
    if wrap {
        return j.rem_euclid(n);
    }
    let j = j.rem_euclid(2 * n);
    if j < n { j } else { 2 * n - 1 - j }
}

impl Pyramid {
    /// The pyramid of the cells of `w` for the window `px`.
    pub fn new(w: &World, px: Pixels) -> Pyramid {
        let (map, cells) = (&w.map, w.mesh.cells());
        let (sq, depths) = (map.square_depth(), map.leaves().iter().map(|l| l.depth));
        let (top, finest) = (depths.clone().min().unwrap_or(sq), depths.max().unwrap_or(sq).min(px.depth));
        let (mut index, mut parent, mut area, mut lies) = (vec![HashMap::new(); (finest - top + 1) as usize], Vec::new(), Vec::new(), Vec::new());
        for (i, l) in map.leaves().iter().enumerate() {
            let (cx, cy) = ((l.x / l.side()) as i64, (l.y / l.side()) as i64);
            let mut up = NONE;
            for k in top..=l.depth.min(finest) {
                let at = (cx >> (l.depth - k), cy >> (l.depth - k));
                let j = *index[(k - top) as usize].entry(at).or_insert_with(|| {
                    parent.push(up);
                    area.push(0.0);
                    parent.len() as u32 - 1
                });
                area[j as usize] += cells.area()[i];
                lies.push((i as u32, j, cells.area()[i]));
                up = j;
            }
        }
        lies.iter_mut().for_each(|(_, j, a)| *a /= area[*j as usize]);
        // The nodes each level needs, from the finest up: those whose B-splines reach the pixels, then those whose
        // coefficients the next finer level reads (`Level::read` and one more for the mean filter).
        let shift = px.depth - finest;
        let mut out = Rect::of(px.x >> shift, px.y >> shift, (px.x + px.across as i64 - 1) >> shift, (px.y + px.down as i64 - 1) >> shift).grown(1);
        let mut rects = Vec::new();
        for _ in top..=finest {
            let r = out.grown(REACH);
            let (x1, y1) = r.end();
            let read = Rect::of(r.x - r.x.rem_euclid(2), r.y - r.y.rem_euclid(2), x1 + 1 - x1.rem_euclid(2), y1 + 1 - y1.rem_euclid(2));
            rects.push((out, read));
            out = read.grown(1).parents();
        }
        rects.reverse();
        // The map's width or height `n` in squares, in nodes of depth `k` (a node past its edge where it ends in one).
        let nodes = |n: usize, k: u8| if k >= sq { (n as i64) << (k - sq) } else { (n as i64 + (1 << (sq - k)) - 1) >> (sq - k) };
        let levels = rects.into_iter().zip(&index).zip(top..).map(|(((out, read), at), k)| {
            let (n, m) = (nodes(map.w, k), nodes(map.h, k));
            let mut node = vec![NONE; read.w * read.h];
            let mut held: Option<[i64; 4]> = None;
            for y in read.y..=read.end().1 {
                for x in read.x..=read.end().0 {
                    if let Some(&j) = at.get(&(on_map(x, n, map.wrap), on_map(y, m, false))) {
                        node[read.at(x, y)] = j;
                        let b = held.get_or_insert([x, y, x, y]);
                        *b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
                    }
                }
            }
            Level { out, read, node, held: held.map(|[x0, y0, x1, y1]| Rect::of(x0, y0, x1, y1)) }
        });
        Pyramid { parent, area, lies, levels: levels.collect(), px, shift }
    }

    /// The field of the cells' values `v` at each pixel of the window, row by row (`Pyramid`).
    pub fn field(&self, v: &[f64]) -> Vec<f64> {
        let mut mean = vec![0.0; self.parent.len()];
        self.lies.iter().for_each(|&(i, j, a)| mean[j as usize] += a * v[i as usize]);
        let g = inverse();
        let mut coef: Option<(Rect, Vec<f64>)> = None;
        for lv in &self.levels {
            // The coarser levels' coefficients at this depth, over the nodes read and one more each way.
            let wide = lv.read.grown(1);
            let pre = coef.map(|(from, c)| finer(from, &c, wide));
            let detail = match &pre {
                Some(pre) => self.departures(lv, &mean, pre),
                None => lv.node.iter().map(|&j| if j == NONE { 0.0 } else { mean[j as usize] }).collect(),
            };
            let mut c = match &pre {
                Some(p) => (lv.out.y..=lv.out.end().1).flat_map(|y| (lv.out.x..=lv.out.end().0).map(move |x| (x, y))).map(|(x, y)| p[wide.at(x, y)]).collect(),
                None => vec![0.0; lv.out.w * lv.out.h],
            };
            if let Some(held) = lv.held {
                add_filtered(lv, &detail, held, &g, &mut c);
            }
            coef = Some((lv.out, c));
        }
        let (from, c) = coef.expect("a level");
        self.at_pixels(from, &c)
    }

    /// The departure each node read by level `lv` (finer than the coarsest) adds, over `lv.read` (0 where no node is),
    /// each group of four children together as `Pyramid` says, from the nodes' means `mean` and the coarser levels'
    /// coefficients `pre` (over `lv.read` and one more each way).
    fn departures(&self, lv: &Level, mean: &[f64], pre: &[f64]) -> Vec<f64> {
        let r = lv.read;
        let mut out = vec![0.0; r.w * r.h];
        let wide = r.grown(1);
        // The mean over node (`x`, `y`) of the coarser levels' field.
        let drawn = |x: i64, y: i64| (0..3).map(|b| (0..3).map(|a| MEAN[a] * MEAN[b] * pre[wide.at(x + a as i64 - 1, y + b as i64 - 1)]).sum::<f64>()).sum::<f64>();
        for y in (r.y..=r.end().1).step_by(2) {
            for x in (r.x..=r.end().0).step_by(2) {
                // The children of one parent in this group of four.
                let kids: Vec<Kid> = [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]
                    .into_iter()
                    .map(|(a, b)| (r.at(a, b), drawn(a, b)))
                    .filter(|&(at, _)| lv.node[at] != NONE)
                    .map(|(at, drawn)| Kid { at, area: self.area[lv.node[at] as usize], mean: mean[lv.node[at] as usize], drawn })
                    .collect();
                let Some(up) = kids.first().map(|k| self.parent[lv.node[k.at] as usize]) else { continue };
                let kids: Vec<Kid> = kids.into_iter().filter(|k| self.parent[lv.node[k.at] as usize] == up).collect();
                // The mean over the children by area of `of`.
                let over = |of: &dyn Fn(&Kid) -> f64| kids.iter().map(|k| k.area * of(k)).sum::<f64>() / kids.iter().map(|k| k.area).sum::<f64>();
                let (own, field) = (mean[up as usize], over(&|k| k.drawn));
                // The share of the coarser field's slope across the parent that the children's own departures show.
                let (sp, pp) = (over(&|k| (k.mean - own) * (k.drawn - field)), over(&|k| (k.drawn - field) * (k.drawn - field)));
                let share = if pp > 0.0 { (sp / pp).clamp(0.0, 1.0) } else { 0.0 };
                kids.iter().for_each(|k| out[k.at] = k.mean - own - share * (k.drawn - field));
            }
        }
        out
    }

    /// The B-spline of coefficients `c` over the nodes `from` of the finest level at the window's pixel centres.
    fn at_pixels(&self, from: Rect, c: &[f64]) -> Vec<f64> {
        let px = &self.px;
        let k = (1i64 << self.shift) as f64;
        // The first node and the weights of the three about pixel column `p`.
        let taps = |p: i64| {
            let u = (p as f64 + 0.5) / k;
            let (jc, t) = (u.floor() as i64, u - u.floor() - 0.5);
            (jc - 1, [0.5 * (0.5 - t) * (0.5 - t), 0.75 - t * t, 0.5 * (0.5 + t) * (0.5 + t)])
        };
        let across: Vec<(usize, [f64; 3])> = (0..px.across as i64).map(|p| taps(px.x + p)).map(|(j, w)| ((j - from.x) as usize, w)).collect();
        let rows: Vec<Vec<f64>> = (0..from.h).map(|r| across.iter().map(|&(j, w)| (0..3).map(|q| w[q] * c[r * from.w + j + q]).sum()).collect()).collect();
        let mut out = Vec::with_capacity(px.across * px.down);
        for p in 0..px.down as i64 {
            let (j, w) = taps(px.y + p);
            let j = (j - from.y) as usize;
            out.extend((0..px.across).map(|x| (0..3).map(|q| w[q] * rows[j + q][x]).sum::<f64>()));
        }
        out
    }
}

/// The coefficients over the nodes `to` of the next depth that give the same B-spline as `c` over the nodes `from`
/// (Chaikin's subdivision: a child takes 3/4 of its parent and 1/4 of the parent's neighbour on its side).
fn finer(from: Rect, c: &[f64], to: Rect) -> Vec<f64> {
    let half = |j: i64, at: i64| {
        let p = j.div_euclid(2) - at;
        (p as usize, if j % 2 == 0 { p as usize - 1 } else { p as usize + 1 })
    };
    let rows: Vec<Vec<f64>> = (0..from.h).map(|r| (to.x..=to.end().0).map(|j| half(j, from.x)).map(|(p, q)| 0.75 * c[r * from.w + p] + 0.25 * c[r * from.w + q]).collect()).collect();
    let mut out = Vec::with_capacity(to.w * to.h);
    for j in to.y..=to.end().1 {
        let (p, q) = half(j, from.y);
        out.extend((0..to.w).map(|x| 0.75 * rows[p][x] + 0.25 * rows[q][x]));
    }
    out
}

/// Add to `c` (over level `lv`'s nodes `out`) the coefficients of the departures `detail` (over `lv.read`): the inverse
/// of the mean filter `g` (`inverse`) along each axis of those of the nodes `held` (the others hold none).
fn add_filtered(lv: &Level, detail: &[f64], held: Rect, g: &[f64], c: &mut [f64]) {
    let reach = REACH;
    let Some(span) = held.grown(reach).within(lv.out) else { return };
    let (xs, ys) = (held.x..=held.end().0, held.y..=held.end().1);
    // Along each row of `held`, at the columns of `span`.
    let rows: Vec<Vec<f64>> = ys
        .clone()
        .map(|y| (span.x..=span.end().0).map(|x| (-reach..=reach).filter(|m| xs.contains(&(x + m))).map(|m| g[(m + reach) as usize] * detail[lv.read.at(x + m, y)]).sum()).collect())
        .collect();
    for y in span.y..=span.end().1 {
        for (k, x) in (span.x..=span.end().0).enumerate() {
            let s: f64 = (-reach..=reach).filter(|m| ys.contains(&(y + m))).map(|m| g[(m + reach) as usize] * rows[(y + m - held.y) as usize][k]).sum();
            c[lv.out.at(x, y)] += s;
        }
    }
}
