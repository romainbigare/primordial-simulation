//! The life of each ground cell (laws.md 0, 3): its cover (`Cell`), and from the month's start what stands on it and
//! its place (`contract`). The cover's month runs on chunks of cells (`chunk`), one task per chunk on the pool.

mod books;
mod chunk;
mod disperse;
pub mod kin;
mod parts;
mod saved;
mod shared;

pub use books::{book, Books};
pub use chunk::Chunk;
pub(crate) use chunk::run;
pub use disperse::lot;
pub use kin::NO_ROOT;
pub use parts::{Parts, FUEL, MATTER, NP, WALL};
pub use shared::{breaking, AGE, BROKEN, DEATHS, DROWNED, LACK, PHASE, SPENT, TAKEN, WET_TISSUE};

use crate::cover::Cover;
use crate::fauna::Fauna;
use crate::space::Mesh;
use crate::World;

/// The life of one ground cell: its cover and its takers.
#[derive(Clone, Debug, Default)]
pub struct Cell {
    pub cover: Cover,
    pub fauna: Fauna,
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
    /// Per cell, from the month's start (`contract::surroundings`): the area of its land, m² (`contract::land`). Not
    /// saved: built with the places
    pub(crate) land: Vec<f64>,
    /// Per cell, from the month's start (`contract::surroundings`): its ground under the standing liquid by depth
    /// (`contract::wet`). Not saved: built with the places
    pub(crate) wet: Vec<[[f64; 2]; crate::cover::BANDS]>,
    /// Per cell, from the start of life's month (`contract::ways_to_liquid`): the way over land to the nearest cell with open
    /// liquid of at least each decade over the month, m, and that cell (`contract::to_liquid`). Not saved: life's month finds it before any
    /// law reads it
    pub(crate) to_liquid: Vec<[(f64, u32); crate::contract::LEVELS]>,
    /// While a parallel pass runs (`life::run`): what the cells see in each air node as the store held it at the
    /// pass's start (`World::a_seen`, of each substance), the same for every cell under it, worked out where first read;
    /// else empty
    pub(crate) air_seen: Vec<std::sync::OnceLock<Vec<f64>>>,
}

/// Once a month, after Flow's state is built in full: the month of each cell's life, one task per chunk on the pool:
/// cover's seeds land (`cover::landed`), and what the takers sent (`fauna::land`).
/// cover's seeds land (`cover::landed`).
pub fn month(w: &mut World) {
    crate::contract::ways_to_liquid(w);
    let load = w.life.cells.iter().filter(|c| c.cover.any() || c.fauna.any()).count();
    run(w, load, |w, d| {
        crate::cover::step(w, d);
        crate::fauna::step(w, d);
    });
    crate::cover::landed(w);
    crate::fauna::land(w);
}

impl Cell {
    /// What its life holds of each substance, mol, for its books: its cover's slots and seeds lying (by `p`'s parts),
    /// then its takers'.
    pub fn amounts<'a>(&'a self, p: &'a Parts) -> impl Iterator<Item = (usize, f64)> + 'a {
        let cover = self.cover.all_mol().flat_map(move |m| m.iter().zip(p.sub).map(|(&a, k)| (k, a)));
        let s = crate::fauna::sub(p);
        cover.chain(self.fauna.all_mol().flat_map(move |m| m.iter().zip(s).map(|(&a, k)| (k, a))))
    }
    /// The mass of its takers, kg (the substances of `p`'s parts at molar masses `mm`).
    pub fn takers_kg(&self, p: &Parts, mm: impl Fn(usize) -> f64) -> f64 {
        let s = crate::fauna::sub(p);
        self.fauna.all_mol().flat_map(|m| m.iter().zip(s)).map(|(&a, k)| a * mm(k)).sum()
    }
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
