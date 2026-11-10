//! The terrain's samples (laws.md 0 and 8), the leaves of `SAMPLE_DEPTH` of the map: the noise says how high the ground
//! would stand (`Terrain::sample`), and the ground is that worn by flowing liquid and creep at the world's start
//! (`space::wear`), so valleys branch between ridges and the closed basins the cutting left stay. A
//! flood over the worn heights (`space::drain`) finds where each sample's liquid runs, and the ponds: the samples
//! under a closed basin's level, each pond with its pass and the level of the pass. Every cell's height and relief, at
//! the start and at each split, are means over these samples (`Terrain::inside`), so a cell and its quarters stand on
//! the same ground at every size; so is the way it faces the sun (`Terrain::rise_south`); and the liquid runs down them
//! (`space::network`). They are a function of the seed and the map alone, found when first read and kept with the
//! terrain, not saved.

use super::drain::flood;
use super::grid::Grid;
use super::network::accumulate;
use super::ponds::{basins, grade};
use super::wear::{finest, wear};
use crate::math::central;
use super::square::Square;
use super::terrain::{Terrain, SAMPLE_DEPTH};
use super::tree::BLOCK;
use super::Leaf;
use std::sync::{Arc, Mutex};

/// The samples of a map for one seed: its width and height in samples, and of each sample row by row its height, m;
/// the terrain's channels on them (`space::network`): the sample its liquid runs to (`network::NONE` for a sink of the
/// open low ground), the area that drains through it, m², and its neighbour upstream that the most drains through
/// (`NONE` where none runs to it); the variance by area of the heights below each, m² (`below::sample_variances`);
/// the pond each lies in (`NONE` on ground that drains), and the ponds.
#[derive(Debug)]
pub struct Samples {
    pub seed: u64,
    pub across: usize,
    pub down: usize,
    pub h: Vec<f64>,
    pub to: Vec<u32>,
    pub up: Vec<f64>,
    pub main: Vec<u32>,
    pub v: Vec<f64>,
    pub pond: Vec<u32>,
    pub ponds: Vec<Pond>,
}

/// A pond: the samples of one closed basin that lie under its level, where liquid stands at one level before it runs
/// over the basin's lowest pass. Its pass is the first sample out of it on the way its liquid leaves by, and the level
/// is that of the pass, m.
#[derive(Clone, Copy, Debug)]
pub struct Pond {
    pub pass: u32,
    pub level: f64,
}

impl Samples {
    /// The index of the sample over leaf `l` of `SAMPLE_DEPTH` or finer.
    pub fn index(&self, l: &Leaf) -> usize {
        let side = BLOCK >> SAMPLE_DEPTH;
        (l.y / side) as usize * self.across + (l.x / side) as usize
    }
}

/// The samples a terrain found, kept with it (not saved: found again from the seed when first read).
#[derive(Debug, Default)]
pub(super) struct Kept(Mutex<Option<Arc<Samples>>>);

impl Clone for Kept {
    fn clone(&self) -> Kept {
        Kept(Mutex::new(self.0.lock().expect("the kept samples").clone()))
    }
}

impl Terrain {
    /// The worn samples of `map` for `seed` (its rows between latitudes `south` and `north`), found once and kept.
    /// They are found with no lock held (finding them works in parallel, and a worker that waited on the lock inside
    /// another parallel section would hold up the workers it needs); where two find them at once, the first kept is
    /// the one (they are the same).
    pub fn samples(&self, seed: u64, map: &Square, lat: (f64, f64)) -> Arc<Samples> {
        let known = |kept: &Option<Arc<Samples>>| kept.as_ref().filter(|s| s.seed == seed).cloned();
        if let Some(s) = known(&self.kept.0.lock().expect("the kept samples")) {
            return s;
        }
        let found = Arc::new(self.worn(seed, map, lat));
        let mut kept = self.kept.0.lock().expect("the kept samples");
        known(&kept).unwrap_or_else(|| {
            *kept = Some(found.clone());
            found
        })
    }

    /// The mean rise of the ground toward the south over leaf `l` of `map` for `seed` (its rows between latitudes `south`
    /// and `north`), m per m: by area over the samples it covers, each its central difference north to south
    /// (one-sided at the map's edge); a leaf finer than a sample takes its sample's. So a leaf's is its quarters' mean by
    /// area at every depth. It is what the sun on a cell reads of the way the cell faces (`flow::on_slope`).
    pub fn rise_south(&self, seed: u64, map: &Square, l: Leaf, (south, north): (f64, f64)) -> f64 {
        let s = self.samples(seed, map, (south, north));
        let dy = map.length(SAMPLE_DEPTH);
        let at = |f: &Leaf| {
            let k = s.index(f);
            central(k / s.across, s.down, |r| s.h[r * s.across + k % s.across], dy)
        };
        if l.depth <= SAMPLE_DEPTH {
            let (area, sum) = l.split(SAMPLE_DEPTH).map(|f| (map.place_of(&f, south, north).0, at(&f))).fold((0.0, 0.0), |(a, by), (da, r)| (a + da, by + da * r));
            return sum / area;
        }
        at(&l)
    }

    /// The samples of `map` for `seed` (`Samples`): the noise's heights (`Terrain::sample`) at the centres of the
    /// samples of the wear's finest grid (`wear::finest`), worn (`space::wear`) with the open low ground held; then the
    /// flood over them from all the open low ground, the closed basins too shallow to hold a pond graded (`ponds::grade`), the area that drains through each
    /// sample, the ponds and the variance below each.
    fn worn(&self, seed: u64, map: &Square, (south, north): (f64, f64)) -> Samples {
        let per = 1usize << (SAMPLE_DEPTH - map.square_depth());
        let (across, down) = (map.w * per, map.h * per);
        let side = BLOCK >> SAMPLE_DEPTH;
        let leaf = |k: usize| Leaf { x: (k % across) as u64 * side, y: (k / across) as u64 * side, depth: SAMPLE_DEPTH };
        let dy = map.length(SAMPLE_DEPTH);
        let grid = Grid::new(across, down, map.wrap, dy, (0..down).map(|r| map.place_of(&leaf(r * across), south, north).0 / dy).collect());
        // The noise at the centres of the samples of the wear's finest grid: leaves `by` samples a side.
        let by = finest(&grid);
        let (wide, depth) = (across / by, SAMPLE_DEPTH - by.trailing_zeros() as u8);
        let block = |k: usize| Leaf { x: (k % wide) as u64 * side * by as u64, y: (k / wide) as u64 * side * by as u64, depth };
        let rise = crate::par::map_n(wide * (down / by), crate::par::UNCOUNTED, |k| self.sample(seed, map, map.place_of(&block(k), south, north).1));
        let (mut h, open) = wear(seed, &grid, (by, &rise));
        let f = flood(&grid, &h, &open, false);
        let (up, main) = accumulate(&f.to, &f.order, |k| grid.area(k));
        let shallow = basins(&grid, &h, &f).1;
        grade(&mut h, &f, &shallow);
        let (pond, members) = basins(&grid, &h, &f);
        let ponds = members.iter().map(|m| Pond { pass: f.to[m[0] as usize], level: f.level[m[0] as usize] }).collect();
        let v = super::below::sample_variances(&h, (across, down));
        Samples { seed, across, down, h, to: f.to, up, main, v, pond, ponds }
    }
}
