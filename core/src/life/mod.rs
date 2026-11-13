//! The life of each ground cell (laws.md 0, 3): its cover (`Cell`), and from the month's start what stands on it and
//! its place (`contract`). The cover's month runs on chunks of cells (`chunk`), one task per chunk on the pool.

mod books;
mod chunk;
mod disperse;
pub mod kin;
mod saved;
mod shared;

pub use books::{book, Books};
pub use chunk::Chunk;
pub(crate) use chunk::run;
pub use disperse::lot;
pub use kin::NO_ROOT;
pub use shared::{AGE, BROKEN, LACK, PHASE, SPENT, TAKEN, WET_TISSUE};

use crate::cover::Cover;
use crate::space::Mesh;

/// The life of one ground cell: its cover.
#[derive(Clone, Debug, Default)]
pub struct Cell {
    pub cover: Cover,
}

#[derive(Clone, Debug, Default)]
pub struct Life {
    /// The life of each ground cell (taken out of the world while the cover's month runs on it, `chunk::run`)
    pub(crate) cells: Vec<Cell>,
    /// Per cell, from the month's start: what stands on it (`contract::Stand`). Not saved: a snapshot is taken at a
    /// month's start, where it is built before any law reads it; the tape reads it first, so `tape::load_snapshot`
    /// builds it again (`contract::stands`)
    pub(crate) stands: Vec<crate::contract::Stand>,
    /// Per cell, from the month's start (`contract::surroundings`): ground temperature, liquid depth, surface height
    /// (`contract::place`)
    pub env: Vec<[f64; 4]>,
    /// While a parallel pass runs (`life::run`): what the cells see in each air node as the store held it at the
    /// pass's start (`World::a_seen`, of each substance), the same for every cell under it, worked out where first read;
    /// else empty
    pub(crate) air_seen: Vec<std::sync::OnceLock<Vec<f64>>>,
}

impl Life {
    /// No life yet on `mesh`.
    pub fn new(mesh: &Mesh) -> Life {
        let n = mesh.cells().n();
        Life { cells: vec![Cell::default(); n], stands: vec![Default::default(); n], ..Default::default() }
    }
    /// The cover of cell `i`.
    pub fn cover(&self, i: usize) -> &Cover {
        &self.cells[i].cover
    }
    /// The life of cell `i`.
    pub fn cell_at(&self, i: usize) -> &Cell {
        &self.cells[i]
    }
}
