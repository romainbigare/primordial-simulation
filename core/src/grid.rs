//! The space grid: a thin view on the square mesh (`space::square`) for the laws that still index cells by row and
//! column. Each cell is a ground node; its 8 neighbours are its edges. Air nodes live on the mesh's coarser level
//! `AIR`: one air node covers `air_scale`² ground cells.

use crate::params::Config;
use crate::space::square::Square;
use crate::space::Mesh;
use serde::{Deserialize, Serialize};

/// The level of the mesh the air nodes live on.
pub const AIR: usize = 1;

/// The map's numbers are saved; the mesh is built from them and the latitudes of the map's edges (`Config`), so it is
/// not saved (a loaded world rebuilds it with `Grid::new`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    /// Cell side, metres.
    pub cell_m: f64,
    pub air_scale: usize,
    #[serde(skip)]
    mesh: Mesh,
}

impl Grid {
    /// A map of `w × h` squares of side `cell_m`, with air nodes over blocks of `air_scale`², its rows spread between
    /// the latitudes of `cfg`.
    pub fn new(w: usize, h: usize, cell_m: f64, air_scale: usize, cfg: &Config) -> Grid {
        let mesh = Square { w, h, side: cell_m }.mesh(&[air_scale], cfg.lat_south, cfg.lat_north);
        Grid { w, h, cell_m, air_scale, mesh }
    }
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
    fn square(&self) -> Square {
        Square { w: self.w, h: self.h, side: self.cell_m }
    }
    pub fn n(&self) -> usize {
        self.mesh.cells().n()
    }
    /// Area of a cell, m² (every square has the same).
    pub fn area(&self) -> f64 {
        self.mesh.cells().area()[0]
    }
    pub fn idx(&self, x: usize, y: usize) -> usize {
        self.square().idx(x, y)
    }
    pub fn xy(&self, i: usize) -> (usize, usize) {
        self.square().xy(i)
    }
    /// Neighbour of cell `i` by offset, if inside the map.
    #[inline]
    pub fn step(&self, i: usize, d: (i32, i32)) -> Option<usize> {
        self.square().step(i, d)
    }
    /// Distance between centres for an offset, metres.
    pub(crate) fn dist(&self, d: (i32, i32)) -> f64 {
        self.square().dist(d)
    }
    pub fn aw(&self) -> usize {
        self.w.div_ceil(self.air_scale)
    }
    pub(crate) fn ah(&self) -> usize {
        self.h.div_ceil(self.air_scale)
    }
    pub fn na(&self) -> usize {
        self.mesh.level(AIR).n()
    }
    /// The share of its air node over ground cell `i`: the cell's area over the node's.
    pub fn air_share(&self, i: usize) -> f64 {
        self.mesh.cells().area()[i] / self.mesh.level(AIR).area()[self.air_of(i)]
    }
    /// Air node above ground cell `i`: its parent at the air level.
    pub fn air_of(&self, i: usize) -> usize {
        self.mesh.up(i, 0, AIR)
    }
    /// Latitude of row `y`, degrees.
    pub fn lat(&self, y: usize) -> f64 {
        self.mesh.cells().lat()[self.idx(0, y)]
    }
}
