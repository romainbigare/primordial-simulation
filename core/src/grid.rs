//! The space grid. Each cell is a ground node; its 8 neighbours are its edges.
//! Air nodes live on a coarser grid: one air node covers `air_scale`² ground cells.

use serde::{Deserialize, Serialize};

/// Forward edge offsets (dx, dy). Every undirected edge is visited once as (i, i + offset).
pub(crate) const FORWARD: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (-1, 1)];
/// All 8 neighbour offsets, clockwise from east.
pub const DIRS: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    /// Cell side, metres.
    pub cell_m: f64,
    pub air_scale: usize,
}

impl Grid {
    pub fn n(&self) -> usize {
        self.w * self.h
    }
    pub fn area(&self) -> f64 {
        self.cell_m * self.cell_m
    }
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }
    pub fn xy(&self, i: usize) -> (usize, usize) {
        (i % self.w, i / self.w)
    }
    /// Neighbour of cell `i` by offset, if inside the map.
    #[inline]
    pub fn step(&self, i: usize, d: (i32, i32)) -> Option<usize> {
        let (x, y) = self.xy(i);
        let (nx, ny) = (x as i32 + d.0, y as i32 + d.1);
        (nx >= 0 && ny >= 0 && (nx as usize) < self.w && (ny as usize) < self.h).then(|| self.idx(nx as usize, ny as usize))
    }
    /// Distance between centres for an offset, metres.
    pub(crate) fn dist(&self, d: (i32, i32)) -> f64 {
        if d.0 != 0 && d.1 != 0 { self.cell_m * std::f64::consts::SQRT_2 } else { self.cell_m }
    }
    pub fn aw(&self) -> usize {
        self.w.div_ceil(self.air_scale)
    }
    pub(crate) fn ah(&self) -> usize {
        self.h.div_ceil(self.air_scale)
    }
    pub fn na(&self) -> usize {
        self.aw() * self.ah()
    }
    /// The share of an air node over one ground cell (1 / `air_scale`²).
    pub fn air_share(&self) -> f64 {
        1.0 / (self.air_scale * self.air_scale) as f64
    }
    /// Air node above ground cell `i`.
    pub fn air_of(&self, i: usize) -> usize {
        let (x, y) = self.xy(i);
        (y / self.air_scale) * self.aw() + x / self.air_scale
    }
    /// Latitude (degrees) of a row, given the map's south and north edges.
    pub fn lat(&self, y: usize, south: f64, north: f64) -> f64 {
        north + (south - north) * (y as f64 + 0.5) / self.h as f64
    }
}
