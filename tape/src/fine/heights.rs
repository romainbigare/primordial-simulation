//! The terrain's heights below the cells as `fine` reads them: the core's own ground (`Terrain::heights`,
//! `space::below`), the one every cell's height, relief and level-volume curve read, so the ground drawn is the ground
//! the world stands on, at any zoom, whatever window it is found for.

use pcore::inside::Ground;
use pcore::space::Leaf;
use pcore::World;
use std::collections::BTreeMap;

pub(super) use pcore::space::below::leaf_at as leaf;

/// What a shell keeps of the terrain while it draws: the terrain's mean height over each cell's leaf found so far, and
/// the ground inside each cell's leaf as the core shares its means over it (`World::ground_in`). The terrain does not
/// change, so neither does what is kept; a new one holds none.
#[derive(Default)]
pub struct Samples {
    means: BTreeMap<Leaf, f64>,
    grounds: BTreeMap<Leaf, Ground>,
}

impl Samples {
    /// The terrain's mean height over leaf `l`, m (kept once found).
    pub(super) fn mean_of(&mut self, w: &World, l: Leaf) -> f64 {
        *self.means.entry(l).or_insert_with(|| w.terrain.inside(w.terrain.seed, &w.map, l, (w.cfg.lat_south, w.cfg.lat_north)).0)
    }
    /// The ground inside leaf `l` (`World::ground_in`, kept once found).
    pub(super) fn ground(&mut self, w: &World, l: Leaf) -> &Ground {
        self.grounds.entry(l).or_insert_with(|| w.ground_in(l))
    }
}

/// The terrain's height at each leaf of `depth` (no shallower than a square) of the window `win` (from column `x0` and
/// row `y0` to `x1` and `y1`, inclusive, in those leaves), m, row by row (`Terrain::heights`).
pub(super) fn window(w: &World, win: [i64; 4], depth: u8) -> Vec<f64> {
    w.terrain.heights(w.terrain.seed, &w.map, (w.cfg.lat_south, w.cfg.lat_north), win, depth)
}
