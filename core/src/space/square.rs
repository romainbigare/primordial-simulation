//! A map of squares, numbered row by row from the north-west corner (x east, y south): flat, or a globe of squares in
//! latitude and longitude whose rows wrap east to west. The cells of the map are squares of 1, 2, 4, ... squares on a
//! side, each aligned to its own size (a tree of cells under each block of the air), numbered by their north-west
//! squares row by row; two cells that share a border differ by one size at most. Cells link across a border, by its
//! width, as far apart as their centres across it; and across a corner (width 0) only where four cells of one size
//! meet, so heat and pore liquid cross only real borders while the downhill path of surface liquid may use all 8 ways,
//! and no cell has more than 8 links. On a globe the cells of a row narrow toward the poles, and no link crosses a pole.

use super::{Level, LinkSpec, Mesh};
use serde::{Deserialize, Serialize};
use std::f64::consts::{SQRT_2, TAU};

/// A map of `w × h` squares of side `side`, m; with `wrap`, a band of a sphere whose rows are whole circles of latitude
/// and whose squares are `side` from north to south; `size`, the side of the cell that holds each square as a power of
/// 2 squares (0: the square is a cell).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Square {
    pub w: usize,
    pub h: usize,
    pub side: f64,
    pub wrap: bool,
    pub size: Vec<u8>,
}

/// A cell of the map: its north-west square (column, row) and its side, squares.
#[derive(Clone, Copy)]
struct Tile {
    x: usize,
    y: usize,
    k: usize,
}

impl Square {
    /// A map whose cells are its squares.
    pub fn new(w: usize, h: usize, side: f64, wrap: bool) -> Square {
        Square { w, h, side, wrap, size: vec![0; w * h] }
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
    /// The cells of the map by their north-west squares row by row, and the cell of each square. Each cell lies inside
    /// the map, all its squares of its size, and differs by one size at most from a cell across its border.
    fn tiles(&self) -> (Vec<Tile>, Vec<u32>) {
        assert_eq!(self.size.len(), self.n(), "a size for each square");
        let (mut tiles, mut cell) = (Vec::new(), vec![u32::MAX; self.n()]);
        for s in 0..self.n() {
            let ((x, y), k) = (self.xy(s), 1usize << self.size[s]);
            if cell[s] != u32::MAX {
                continue;
            }
            assert!(x % k == 0 && y % k == 0 && x + k <= self.w && y + k <= self.h, "square {s} starts a cell of its size inside the map");
            for t in (y..y + k).flat_map(|ty| (x..x + k).map(move |tx| (tx, ty))).map(|(tx, ty)| self.idx(tx, ty)) {
                assert!(self.size[t] == self.size[s] && cell[t] == u32::MAX, "the squares of a cell are of its size");
                cell[t] = tiles.len() as u32;
            }
            tiles.push(Tile { x, y, k });
        }
        for s in 0..self.n() {
            for t in [self.step(s, (1, 0)), self.step(s, (0, 1))].into_iter().flatten() {
                assert!(self.size[s].abs_diff(self.size[t]) <= 1, "cells across a border differ by one size at most");
            }
        }
        (tiles, cell)
    }
    /// The mesh of the map: its cells, rows spread from latitude `north` (row 0) to `south`, and a coarser level of the
    /// blocks of `scale`² squares (fewer at the south and east edges), each cell inside one block, built from the
    /// squares so it does not depend on the sizes of the cells.
    pub fn mesh(&self, scale: usize, south: f64, north: f64) -> Mesh {
        assert!(!self.wrap || (self.w >= 3 && self.w.is_multiple_of(scale)), "a globe's row has 3 cells or more, in whole blocks");
        let (tiles, cell) = self.tiles();
        assert!(tiles.iter().all(|t| scale.is_multiple_of(t.k)), "each cell lies inside one block");
        // The block of `scale`² squares that holds square `i`, numbered row by row.
        let block = |i: usize| {
            let ((x, y), to) = ((self.xy(i).0 / scale, self.xy(i).1 / scale), self.w.div_ceil(scale));
            (y * to + x) as u32
        };
        let squares: Vec<Tile> = (0..self.n()).map(|s| Tile { x: self.xy(s).0, y: self.xy(s).1, k: 1 }).collect();
        let blocks = self.level(&squares, &(0..self.n() as u32).collect::<Vec<u32>>(), south, north).coarser(&(0..self.n()).map(block).collect::<Vec<u32>>());
        let corner: Vec<u32> = tiles.iter().map(|t| self.idx(t.x, t.y) as u32).collect();
        let parent = corner.iter().map(|&s| block(s as usize)).collect();
        Mesh::of(self.level(&tiles, &cell, south, north), blocks, parent, cell, corner)
    }
    /// The level of the cells `tiles` (`cell`: the cell of each square), longitudes east from 0 by the degrees of
    /// latitude per row (a whole turn on a globe). Each cell links to the cells across its east side and its south side
    /// (north to south, west to east), then to those across its south-east and south-west corners.
    fn level(&self, tiles: &[Tile], cell: &[u32], south: f64, north: f64) -> Level {
        let row = (north - south) / self.h as f64;
        // On a globe: the sphere's radius, m, and the latitude of the north edge of row `y`, radians.
        let (radius, edge) = (self.side / row.to_radians(), |y: usize| (north - row * y as f64).to_radians());
        // The latitude `rows` rows south of the north edge of the map, degrees.
        let lat = |rows: f64| north + (south - north) * rows / self.h as f64;
        // The length of the circle of latitude `phi` (radians) across one column, m: a side on the flat map.
        let across = |phi: f64| if self.wrap { radius * phi.cos() * TAU / self.w as f64 } else { self.side };
        let mid = |from: usize, k: usize| from as f64 + k as f64 / 2.0;
        let area = |t: &Tile| {
            let k = t.k as f64;
            if self.wrap { radius * radius * TAU / self.w as f64 * k * (edge(t.y).sin() - edge(t.y + t.k).sin()) } else { (k * self.side) * (k * self.side) }
        };
        let centre = |t: &Tile| [mid(t.x, t.k) * self.side, mid(t.y, t.k) * self.side];
        let lon = |t: &Tile| mid(t.x, t.k) * if self.wrap { 360.0 / self.w as f64 } else { row };
        // The cell at column `x` (round the globe) and row `y`, if inside the map.
        let at = |x: usize, y: usize| (y < self.h && (self.wrap || x < self.w)).then(|| cell[self.idx(x % self.w, y)] as usize);
        // Whether cell `c` starts at column `x` (round the globe) and row `y` with side `k`.
        let is = |c: Option<usize>, x: usize, y: usize, k: usize| c.is_some_and(|c| (tiles[c].x, tiles[c].y, tiles[c].k) == (x % self.w, y, k));
        let mut links: Vec<LinkSpec> = Vec::new();
        for (i, t) in tiles.iter().enumerate() {
            let (k, x, y) = (t.k, t.x, t.y);
            let mut push = |j: usize, dist: f64, width: f64| {
                assert_ne!(i, j, "a cell does not border itself");
                links.push((i.min(j) as u32, i.max(j) as u32, dist, width));
            };
            // The distinct cells along a side, in order.
            let along = |of: &mut dyn Iterator<Item = Option<usize>>| {
                let mut out: Vec<usize> = Vec::new();
                of.flatten().for_each(|c| if out.last() != Some(&c) { out.push(c) });
                out
            };
            for j in along(&mut (y..y + k).map(|r| at(x + k, r))) {
                let o = &tiles[j];
                let (from, to) = (y.max(o.y), (y + k).min(o.y + o.k));
                push(j, across(lat(mid(from, to - from)).to_radians()) * ((k + o.k) as f64 / 2.0), self.side * (to - from) as f64);
            }
            for j in along(&mut (x..x + k).map(|c| at(c, y + k))) {
                let o = &tiles[j];
                let (from, to) = (x.max(o.x), (x + k).min(o.x + o.k));
                push(j, self.side * ((k + o.k) as f64 / 2.0), across(edge(y + k)) * (to - from) as f64);
            }
            let diagonal = if self.wrap { (k as f64 * self.side).hypot(across(edge(y + k)) * k as f64) } else { (k as f64 * self.side) * SQRT_2 };
            let below = is(at(x, y + k), x, y + k, k);
            if below && is(at(x + k, y), x + k, y, k) && is(at(x + k, y + k), x + k, y + k, k) {
                push(at(x + k, y + k).expect("a cell"), diagonal, 0.0);
            }
            let west = (x >= k || self.wrap).then(|| (x + self.w - k) % self.w);
            if let Some(wx) = west.filter(|&wx| below && is(at(wx, y), wx, y, k) && is(at(wx, y + k), wx, y + k, k)) {
                push(at(wx, y + k).expect("a cell"), diagonal, 0.0);
            }
        }
        let place = (tiles.iter().map(|t| lat(mid(t.y, t.k))).collect(), tiles.iter().map(lon).collect());
        let mut cells = Level::of_cells(tiles.iter().map(area).collect(), tiles.iter().map(centre).collect(), place, if self.wrap { radius } else { 0.0 });
        cells.join(links);
        cells
    }
}
