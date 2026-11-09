//! The terrain's heights below the cells as `fine` reads them: the core's own ground (`Terrain::heights`,
//! `space::below`), the one every cell's height, relief and level-volume curve read, so the ground drawn is the ground
//! the world stands on, at any zoom, whatever window it is found for.

use pcore::space::Leaf;
use pcore::World;
use std::collections::BTreeMap;

pub(super) use pcore::space::below::leaf_at as leaf;

/// What a shell keeps of the terrain while it draws: the terrain's mean height over each cell's leaf found so far, and
/// the ground and the wetness index of the last window drawn (`Kept`). The terrain does not change, so neither does
/// what is kept; a new one holds none.
#[derive(Default)]
pub struct Samples {
    means: BTreeMap<Leaf, f64>,
    kept: Option<Kept>,
}

/// A window's pixels (from leaf column `x` and row `y`, `across` x `down` leaves of `depth`), their ground, m, and its
/// wetness index (`inside::wetness_index`).
struct Kept {
    at: ([i64; 2], [usize; 2], u8),
    ground: Vec<f64>,
    twi: Vec<f64>,
}

impl Samples {
    /// The terrain's mean height over leaf `l`, m (kept once found).
    pub(super) fn mean_of(&mut self, w: &World, l: Leaf) -> f64 {
        *self.means.entry(l).or_insert_with(|| w.terrain.inside(w.terrain.seed, &w.map, l, (w.cfg.lat_south, w.cfg.lat_north)).0)
    }
    /// The ground of window `win` (`window`) and its wetness index (`inside::wetness_index`), found again only for a
    /// window other than the last.
    pub(super) fn ground(&mut self, w: &World, win: &super::inside::Window) -> (&[f64], &[f64]) {
        let at = (win.at, win.size, win.depth);
        if self.kept.as_ref().is_none_or(|k| k.at != at) {
            let ([gx, gy], [across, down]) = (win.at, win.size);
            let ground = window(w, [gx, gy, gx + across as i64 - 1, gy + down as i64 - 1], win.depth);
            let twi = super::inside::wetness_index(w, win, &ground);
            self.kept = Some(Kept { at, ground, twi });
        }
        let k = self.kept.as_ref().expect("the window kept");
        (&k.ground, &k.twi)
    }
}

/// The terrain's height at each leaf of `depth` (no shallower than a square) of the window `win` (from column `x0` and
/// row `y0` to `x1` and `y1`, inclusive, in those leaves), m, row by row (`Terrain::heights`).
pub(super) fn window(w: &World, win: [i64; 4], depth: u8) -> Vec<f64> {
    w.terrain.heights(w.terrain.seed, &w.map, (w.cfg.lat_south, w.cfg.lat_north), win, depth)
}
