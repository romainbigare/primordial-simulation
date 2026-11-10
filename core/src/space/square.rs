//! A map of squares, numbered row by row from the north-west corner (x east, y south): flat, or a globe of squares in
//! latitude and longitude whose rows wrap east to west. The air level is built on the squares (`Square::mesh`), and
//! the tape's frames show one each. The cells of the map are the leaves of a tree under each air
//! block of `scale` squares a side (`tree`), of any depth down to `DEPTH_MOST`, numbered by their north-west corners row
//! by row; two cells that share a border differ by one depth at most. Cells link across a border, by its width, as far
//! apart as their centres across it; and across a corner (width 0) where four cells meet, whatever their sizes, the two
//! across it one depth apart at most, so heat and pore liquid cross only real borders while the downhill path of surface
//! liquid may use all 8 ways. No cell has more than 8 links: a
//! corner links its two cells unless one of them has 8 already (the cells in order). On a globe the cells of a row narrow toward the poles, and no link crosses a pole.

use super::tree::{self, Leaf, Tree, BLOCK, EXTENT_MOST};
use super::terrain::Place;
use super::{Level, LinkSpec, Mesh, COMPASS};
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
        assert!(scale.is_power_of_two() && scale as u64 <= BLOCK, "an air block is a power of 2 squares a side, at most {BLOCK}");
        let map = Square { w, h, side, wrap, scale, leaves: Vec::new() };
        assert!(map.extent().0.max(map.extent().1) < EXTENT_MOST, "a map is less than {EXTENT_MOST} leaves of the most depth a side");
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
    /// The cell that is leaf `l`, if one is.
    pub fn cell(&self, l: Leaf) -> Option<usize> {
        self.leaves.binary_search(&l).ok()
    }
    /// The depth of a square below its air block.
    pub fn square_depth(&self) -> u8 {
        self.scale.trailing_zeros() as u8
    }
    /// Leaves of the most depth along a side of a square, and the map's width and height in them.
    fn unit(&self) -> u64 {
        BLOCK / self.scale as u64
    }
    fn extent(&self) -> (u64, u64) {
        (self.w as u64 * self.unit(), self.h as u64 * self.unit())
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
    pub fn point(&self, s: usize) -> [u64; 2] {
        let (x, y) = self.xy(s);
        self.point_at(x as f64 + 0.5, y as f64 + 0.5)
    }
    /// The point at column `x` and row `y` of the map, in squares from its north-west corner (a fraction is inside a
    /// square), in leaves of the most depth; a place off the map is the nearest edge's, east and west round a globe.
    pub fn point_at(&self, x: f64, y: f64) -> [u64; 2] {
        let (across, down) = self.extent();
        let u = self.unit() as f64;
        let x = if self.wrap { x.rem_euclid(self.w as f64) } else { x };
        [(x * u).floor().clamp(0.0, (across - 1) as f64) as u64, (y * u).floor().clamp(0.0, (down - 1) as f64) as u64]
    }
    /// The side of a leaf of `depth` from north to south, m.
    pub fn length(&self, depth: u8) -> f64 {
        self.side * (BLOCK >> depth) as f64 / self.unit() as f64
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
    /// The west, north, east and south edges of cell `i`, in squares from the map's north-west corner.
    pub fn bounds(&self, i: usize) -> [f64; 4] {
        let (l, unit) = (&self.leaves[i], self.unit() as f64);
        let [x, y, s] = [l.x, l.y, l.side()].map(|v| v as f64 / unit);
        [x, y, x + s, y + s]
    }
    /// The tree of the cells.
    fn tree(&self) -> Tree {
        let (across, down) = self.extent();
        Tree::new(&self.leaves, across, down, self.wrap)
    }
    /// The mesh of the map: its cells, rows spread from latitude `north` (row 0) to `south`, and a coarser level of the
    /// air blocks, numbered row by row, built from the map's squares so it does not depend on the sizes of the cells (four
    /// blocks link across their corner where four squares meet there, whatever cells lie at it).
    pub fn mesh(&self, south: f64, north: f64) -> Mesh {
        assert!(!self.wrap || (self.w >= 3 && self.w.is_multiple_of(self.scale)), "a globe's row has 3 cells or more, in whole blocks");
        let (tree, squares) = (self.tree(), Square::new(self.w, self.h, self.side, self.wrap, self.scale));
        let block = |l: &Leaf| tree.block(l.x, l.y) as u32;
        let (mut blocks, links) = squares.level(&squares.tree(), south, north).coarser(&squares.leaves.iter().map(block).collect::<Vec<u32>>());
        blocks.join(links);
        let tree = self.tree();
        let parent = self.leaves.iter().map(|l| tree.block(l.x, l.y) as u32).collect();
        let mesh = Mesh::of(self.level(&tree, south, north), blocks, parent, tree);
        // The wind runs along the lines of the blocks east and west, and north and south (`flow::air::wind::lines`).
        let air = mesh.level(1);
        let along = |i: usize, d: usize| air.way(i, d).is_none_or(|e| air.width()[air.adj_link()[e] as usize] == 0.0);
        assert!((0..air.n()).all(|i| [1, 3, 5, 7].iter().all(|&d| along(i, d))), "a block shares a border only east, south, west and north");
        mesh
    }
    /// The area of leaf `t`, m², and its centre (`Place`), its rows between latitudes `south` (row `h`) and `north` (row
    /// 0): longitudes east from 0 by the degrees of latitude per row (a whole turn on a globe).
    pub fn place_of(&self, t: &Leaf, south: f64, north: f64) -> (f64, Place) {
        let (unit, row) = (self.unit() as f64, (north - south) / self.h as f64);
        let sq = |u: u64| u as f64 / unit;
        let mid = |from: u64, k: u64| sq(from) + sq(k) / 2.0;
        // On a globe: the sphere's radius, m, and the latitude of the north edge of row `y`, radians.
        let (radius, edge) = (self.side / row.to_radians(), |y: u64| (north - row * sq(y)).to_radians());
        let k = sq(t.side());
        let area = if self.wrap { radius * radius * TAU / self.w as f64 * k * (edge(t.y).sin() - edge(t.y + t.side()).sin()) } else { (k * self.side) * (k * self.side) };
        let centre = [mid(t.x, t.side()) * self.side, mid(t.y, t.side()) * self.side];
        let lon = mid(t.x, t.side()) * if self.wrap { 360.0 / self.w as f64 } else { row };
        (area, Place { centre, lat: north + (south - north) * mid(t.y, t.side()) / self.h as f64, lon })
    }
    /// The level of the cells (`tree`: their tree), longitudes east from 0 by the degrees of latitude per row (a whole
    /// turn on a globe). Each cell links to the cells across its east side and its south side (north to south, west to
    /// east), then to those across its south-east and south-west corners.
    fn level(&self, tree: &Tree, south: f64, north: f64) -> Level {
        let (leaves, unit, row, wide) = (&self.leaves, self.unit() as f64, (north - south) / self.h as f64, self.extent().0);
        // A length in leaves of the most depth, in squares.
        let sq = |u: u64| u as f64 / unit;
        // On a globe: the sphere's radius, m, and the latitude of the north edge of row `y`, radians.
        let (radius, edge) = (self.side / row.to_radians(), |y: u64| (north - row * sq(y)).to_radians());
        // The latitude `rows` rows south of the north edge of the map, degrees.
        let lat = |rows: f64| north + (south - north) * rows / self.h as f64;
        // The length of the circle of latitude `phi` (radians) across one column, m: a side on the flat map.
        let across = |phi: f64| if self.wrap { radius * phi.cos() * TAU / self.w as f64 } else { self.side };
        let mid = |from: u64, k: u64| sq(from) + sq(k) / 2.0;
        // The cells north-west, north-east, south-west and south-east of the point at column `px` (round the globe) and
        // row `py`, where it is a corner of all four: four cells meet there.
        let round = |px: i64, py: u64| -> Option<[usize; 4]> {
            let q = [(-1, -1), (0, -1), (-1, 0), (0, 0)].map(|(dx, dy)| tree.at(px + dx, py as i64 + dy));
            let q = [q[0]?, q[1]?, q[2]?, q[3]?];
            let corner = |c: usize, east: bool, south: bool| {
                let l = &leaves[c];
                ((if east { l.x } else { l.x + l.side() }) % wide, if south { l.y } else { l.y + l.side() }) == (px.rem_euclid(wide as i64) as u64, py)
            };
            (corner(q[0], false, false) && corner(q[1], true, false) && corner(q[2], false, true) && corner(q[3], true, true)).then_some(q)
        };
        // How far apart the centres of two cells across a corner on row `py` are, their sides summing to `k`.
        let apart = |k: u64, py: u64| {
            let d = sq(k) / 2.0;
            if self.wrap { (d * self.side).hypot(across(edge(py)) * d) } else { (d * self.side) * SQRT_2 }
        };
        let placed: Vec<(f64, Place)> = leaves.iter().map(|t| self.place_of(t, south, north)).collect();
        let place = (placed.iter().map(|p| p.1.lat).collect(), placed.iter().map(|p| p.1.lon).collect());
        let mut cells = Level::of_cells(placed.iter().map(|p| p.0).collect(), placed.iter().map(|p| p.1.centre).collect(), place, if self.wrap { radius } else { 0.0 });
        // Each cell's links across its sides, then with those across its corners as they come.
        let mut links_of: Vec<usize> = leaves.iter().map(|t| tree::SIDES.iter().map(|&way| tree.beside(t, way).count()).sum()).collect();
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
            // Across its south-east corner to the cell south-east of it, then across its south-west corner to the cell
            // south-west of it, where four cells meet, the two differ by one depth at most and neither has `COMPASS`
            // links yet.
            for (px, other) in [(x as i64 + k as i64, 3), (x as i64, 2)] {
                let j = round(px, y + k).map(|q| q[other]).filter(|&j| t.depth.abs_diff(leaves[j].depth) <= 1 && links_of[i] < COMPASS && links_of[j] < COMPASS);
                if let Some(j) = j {
                    (links_of[i], links_of[j]) = (links_of[i] + 1, links_of[j] + 1);
                    push(j, apart(k + leaves[j].side(), y + k), 0.0);
                }
            }
        }
        cells.join(links);
        cells
    }
}
