//! A map of squares, numbered row by row from the north-west corner (x east, y south): flat, or a globe of squares in
//! latitude and longitude whose rows wrap east to west. The air level is built on the squares (`Square::samples`), and
//! the tape's frames show one each. The cells of the map are the leaves of a tree under each air
//! block of `scale` squares a side (`tree`), of any depth down to `DEPTH_MOST`, numbered by their north-west corners row
//! by row; two cells that share a border differ by one depth at most. Cells link across a border, by its width, as far
//! apart as their centres across it; and across a corner (width 0) only where four cells of one size meet, so heat and
//! pore liquid cross only real borders while the downhill path of surface liquid may use all 8 ways, and no cell has
//! more than 8 links. On a globe the cells of a row narrow toward the poles, and no link crosses a pole.

use super::tree::{self, Leaf, Tree, BLOCK};
use super::{Level, LinkSpec, Mesh};
use serde::{Deserialize, Serialize};
use std::f64::consts::{SQRT_2, TAU};

/// A map of `w × h` squares of side `side`, m, in air blocks of `scale` squares a side (a power of 2, at most `BLOCK`;
/// fewer at the south and east edges); with `wrap`, a band of a sphere whose rows are whole circles of latitude and
/// whose squares are `side` from north to south. Its cells are the leaves of the tree under the blocks, in order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Square {
    pub w: usize,
    pub h: usize,
    pub side: f64,
    pub wrap: bool,
    pub scale: usize,
    leaves: Vec<Leaf>,
}

impl Square {
    /// A map whose cells are its squares.
    pub fn new(w: usize, h: usize, side: f64, wrap: bool, scale: usize) -> Square {
        assert!(scale.is_power_of_two() && scale as u32 <= BLOCK, "an air block is a power of 2 squares a side, at most {BLOCK}");
        let map = Square { w, h, side, wrap, scale, leaves: Vec::new() };
        let depth = map.square_depth();
        map.by_blocks(|_, _| depth)
    }
    /// The map with the cells of each air block of one depth, `depth(column, row)` of the block; each must lie inside
    /// the map (a block at the south or east edge of fewer squares is no coarser than it).
    pub fn by_blocks(self, depth: impl Fn(usize, usize) -> u8) -> Square {
        let (across, down) = self.extent();
        let blocks = (0..down.div_ceil(BLOCK)).flat_map(|by| (0..across.div_ceil(BLOCK)).map(move |bx| (bx, by)));
        let leaves = blocks.flat_map(|(bx, by)| Leaf { y: by * BLOCK, x: bx * BLOCK, depth: 0 }.split(depth(bx as usize, by as usize)));
        self.with(leaves.filter(|l| l.x < across && l.y < down).collect())
    }
    /// The map with its cells split where one borders a cell more than one depth finer, until none does (2:1 balance).
    pub fn balanced(mut self) -> Square {
        let (across, down) = self.extent();
        let leaves = tree::balance(std::mem::take(&mut self.leaves), across, down, self.wrap);
        self.with(leaves)
    }
    /// The map's samples: each cell coarser than a square cut into its squares, the finer ones as they are.
    pub fn samples(&self) -> Square {
        let depth = self.square_depth();
        Square { leaves: Vec::new(), ..*self }.with(self.leaves.iter().flat_map(|l| l.split(l.depth.max(depth))).collect())
    }
    /// The map with the cells `leaves`, put in order.
    pub fn with(self, mut leaves: Vec<Leaf>) -> Square {
        leaves.sort_unstable();
        Square { leaves, ..self }
    }
    /// The cells, in order.
    pub fn leaves(&self) -> &[Leaf] {
        &self.leaves
    }
    /// The depth of a square below its air block.
    pub fn square_depth(&self) -> u8 {
        self.scale.trailing_zeros() as u8
    }
    /// Leaves of the most depth along a side of a square, and the map's width and height in them.
    fn unit(&self) -> u32 {
        BLOCK / self.scale as u32
    }
    fn extent(&self) -> (u32, u32) {
        (self.w as u32 * self.unit(), self.h as u32 * self.unit())
    }
    pub fn n(&self) -> usize {
        self.w * self.h
    }
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }
    pub fn xy(&self, i: usize) -> (usize, usize) {
        (i % self.w, i / self.w)
    }
    /// Neighbour of square `i` by offset, if inside the map (round the globe east and west).
    #[inline]
    pub fn step(&self, i: usize, d: (i32, i32)) -> Option<usize> {
        let (x, y) = self.xy(i);
        let (nx, ny) = (if self.wrap { (x as i32 + d.0).rem_euclid(self.w as i32) } else { x as i32 + d.0 }, y as i32 + d.1);
        (nx >= 0 && ny >= 0 && (nx as usize) < self.w && (ny as usize) < self.h).then(|| self.idx(nx as usize, ny as usize))
    }
    /// The point at the centre of square `s` (`Tree::cell_at`), in leaves of the most depth: the one whose north-west
    /// corner it is.
    pub fn point(&self, s: usize) -> [u32; 2] {
        let ((x, y), u) = (self.xy(s), self.unit());
        [x as u32 * u + u / 2, y as u32 * u + u / 2]
    }
    /// The squares in cell `i` (a share of one in a cell finer than a square).
    pub fn squares(&self, i: usize) -> f64 {
        let k = self.leaves[i].side() as f64 / self.unit() as f64;
        k * k
    }
    /// The row of the map at the north-west corner of cell `i`, in squares.
    pub fn row(&self, i: usize) -> f64 {
        self.leaves[i].y as f64 / self.unit() as f64
    }
    /// The tree of the cells.
    fn tree(&self) -> Tree {
        let (across, down) = self.extent();
        Tree::new(&self.leaves, across, down, self.wrap)
    }
    /// The mesh of the map: its cells, rows spread from latitude `north` (row 0) to `south`, and a coarser level of the
    /// air blocks, numbered row by row, built from the map's samples (`samples`) so it does not depend on the sizes of the
    /// cells.
    pub fn mesh(&self, south: f64, north: f64) -> Mesh {
        assert!(!self.wrap || (self.w >= 3 && self.w.is_multiple_of(self.scale)), "a globe's row has 3 cells or more, in whole blocks");
        let (tree, samples) = (self.tree(), self.samples());
        let block = |l: &Leaf| tree.block(l.x, l.y) as u32;
        let blocks = samples.level(&samples.tree(), south, north).coarser(&samples.leaves.iter().map(block).collect::<Vec<u32>>());
        let parent = self.leaves.iter().map(block).collect();
        let mesh = Mesh::of(self.level(&tree, south, north), blocks, parent, tree);
        // The wind runs along the lines of the blocks east and west, and north and south (`flow::air::wind::lines`).
        let air = mesh.level(1);
        let along = |i: usize, d: usize| air.way(i, d).is_none_or(|e| air.width()[air.adj_link()[e] as usize] == 0.0);
        assert!((0..air.n()).all(|i| [1, 3, 5, 7].iter().all(|&d| along(i, d))), "a block shares a border only east, south, west and north");
        mesh
    }
    /// The level of the cells (`tree`: their tree), longitudes east from 0 by the degrees of latitude per row (a whole
    /// turn on a globe). Each cell links to the cells across its east side and its south side (north to south, west to
    /// east), then to those across its south-east and south-west corners.
    fn level(&self, tree: &Tree, south: f64, north: f64) -> Level {
        let (leaves, unit, row, wide) = (&self.leaves, self.unit() as f64, (north - south) / self.h as f64, self.extent().0);
        // A length in leaves of the most depth, in squares.
        let sq = |u: u32| u as f64 / unit;
        // On a globe: the sphere's radius, m, and the latitude of the north edge of row `y`, radians.
        let (radius, edge) = (self.side / row.to_radians(), |y: u32| (north - row * sq(y)).to_radians());
        // The latitude `rows` rows south of the north edge of the map, degrees.
        let lat = |rows: f64| north + (south - north) * rows / self.h as f64;
        // The length of the circle of latitude `phi` (radians) across one column, m: a side on the flat map.
        let across = |phi: f64| if self.wrap { radius * phi.cos() * TAU / self.w as f64 } else { self.side };
        let mid = |from: u32, k: u32| sq(from) + sq(k) / 2.0;
        let area = |t: &Leaf| {
            let k = sq(t.side());
            if self.wrap { radius * radius * TAU / self.w as f64 * k * (edge(t.y).sin() - edge(t.y + t.side()).sin()) } else { (k * self.side) * (k * self.side) }
        };
        let centre = |t: &Leaf| [mid(t.x, t.side()) * self.side, mid(t.y, t.side()) * self.side];
        let lon = |t: &Leaf| mid(t.x, t.side()) * if self.wrap { 360.0 / self.w as f64 } else { row };
        let at = |x: u32, y: u32| tree.at(x as i64, y as i64);
        // Whether cell `c` is the leaf of side `k` at column `x` (round the globe) and row `y`.
        let is = |c: Option<usize>, x: u32, y: u32, k: u32| c.is_some_and(|c| (leaves[c].x, leaves[c].y, leaves[c].side()) == (x % wide, y, k));
        let mut links: Vec<LinkSpec> = Vec::new();
        for (i, t) in leaves.iter().enumerate() {
            let (k, x, y) = (t.side(), t.x, t.y);
            let mut push = |j: usize, dist: f64, width: f64| {
                assert_ne!(i, j, "a cell does not border itself");
                assert!(t.depth.abs_diff(leaves[j].depth) <= 1, "cells across a border differ by one depth at most");
                links.push((i.min(j) as u32, i.max(j) as u32, dist, width));
            };
            for j in tree.beside(t, (1, 0)) {
                let o = &leaves[j];
                let (from, to) = (y.max(o.y), (y + k).min(o.y + o.side()));
                push(j, across(lat(mid(from, to - from)).to_radians()) * (sq(k + o.side()) / 2.0), self.side * sq(to - from));
            }
            for j in tree.beside(t, (0, 1)) {
                let o = &leaves[j];
                let (from, to) = (x.max(o.x), (x + k).min(o.x + o.side()));
                push(j, self.side * (sq(k + o.side()) / 2.0), across(edge(y + k)) * sq(to - from));
            }
            let diagonal = if self.wrap { (sq(k) * self.side).hypot(across(edge(y + k)) * sq(k)) } else { (sq(k) * self.side) * SQRT_2 };
            let below = is(at(x, y + k), x, y + k, k);
            if below && is(at(x + k, y), x + k, y, k) && is(at(x + k, y + k), x + k, y + k, k) {
                push(at(x + k, y + k).expect("a cell"), diagonal, 0.0);
            }
            let west = (x >= k || self.wrap).then(|| (x + wide - k) % wide);
            if let Some(wx) = west.filter(|&wx| below && is(at(wx, y), wx, y, k) && is(at(wx, y + k), wx, y + k, k)) {
                push(at(wx, y + k).expect("a cell"), diagonal, 0.0);
            }
        }
        let place = (leaves.iter().map(|t| lat(mid(t.y, t.side()))).collect(), leaves.iter().map(lon).collect());
        let mut cells = Level::of_cells(leaves.iter().map(area).collect(), leaves.iter().map(centre).collect(), place, if self.wrap { radius } else { 0.0 });
        cells.join(links);
        cells
    }
}
