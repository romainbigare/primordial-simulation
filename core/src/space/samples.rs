//! The terrain's samples (laws.md 0 and 8), the leaves of `SAMPLE_DEPTH` of the map: the noise says how high the ground
//! would stand (`Terrain::sample`), and the ground is that worn by flowing liquid and creep at the world's start
//! (`space::wear`), so valleys branch between ridges and the closed basins the cutting left stay. A
//! flood over the worn heights (`space::drain`) finds where each sample's liquid runs, and the ponds: the samples
//! under a closed basin's level, each pond with its pass and the level of the pass. Every cell's height is a mean over
//! these samples (`Terrain::inside`); so is the way it faces the sun (`Terrain::rise_south_on`); and the liquid runs down
//! them (`space::network`). They are a function of the seed and the map alone, found when first read and kept with the
//! terrain, not saved.

use super::delta::{exposures, lobes, Liquid, Mouth};
use super::drain::{draws, flood, open_low, starts};
use super::fill::heading;
use super::relief::Relief;
use super::grid::Grid;
use super::network::{accumulate, NONE};
use super::ponds::{basins, grade, open_parts, shallow};
use super::wear::{noise_grid, wear, Books, STAND_M};
use crate::math::central;
use super::square::Square;
use super::strata::Strata;
use super::terrain::{Terrain, SAMPLE_DEPTH};
use super::tree::BLOCK;
use super::Leaf;
use std::sync::{Arc, Mutex};

/// The samples of a map for one seed: its width and height in samples, and of each sample row by row its height, m;
/// the terrain's channels on them (`space::network`): the sample its liquid runs to (`network::NONE` for a sink of the
/// open low ground), the area that drains through it, m², and its neighbour upstream that the most drains through
/// (`NONE` where none runs to it), and the samples in the order the flood reached them (each after the one its liquid
/// runs to), and how far it lies from the sample its liquid runs to, m (none where it runs to none); the
/// pond each lies in (`NONE` on ground that drains), and the ponds; the make of each sample's bed at its surface
/// (`space::strata`, as the wear left it); the depth of its loose ground the wear left, m, how much of it settled in
/// the wear's last stage, m (all of it on the open low ground), and how long it has lain, years (`mantle::age`); and
/// the volumes the wear moved (`wear::Books`); and how much liquid gathers at each, its topographic wetness index
/// ln(a / tan β) (`sample_index`); and the channel law of the terrain (`Terrain::channel`).
#[derive(Clone, Debug)]
pub struct Samples {
    pub seed: u64,
    pub across: usize,
    pub down: usize,
    pub h: Vec<f64>,
    pub to: Vec<u32>,
    pub order: Vec<u32>,
    pub up: Vec<f64>,
    pub main: Vec<u32>,
    pub run: Vec<f64>,
    pub pond: Vec<u32>,
    pub ponds: Vec<Pond>,
    pub make: Vec<u8>,
    pub loose: Vec<f64>,
    pub settled: Vec<f64>,
    pub age: Vec<f64>,
    pub books: Books,
    pub twi: Vec<f64>,
    pub channel: [f64; 2],
}

/// A pond: the samples of one closed basin that lie under its level, where liquid stands at one level before it runs
/// over the basin's lowest pass. Its pass is the first sample out of it on the way its liquid leaves by, and the level
/// is that of the pass, m. The open low ground is ponds too, one for each part of it joined through its samples (an
/// island's one): its liquid leaves by no pass (`network::NONE`) and stands at any level (infinite).
#[derive(Clone, Copy, Debug)]
pub struct Pond {
    pub pass: u32,
    pub level: f64,
}

impl Samples {
    /// The slope of sample `k` to the sample its liquid runs to, rise over run (none where it runs to none or uphill).
    pub fn slope(&self, k: usize) -> f64 {
        if self.to[k] == NONE { 0.0 } else { (self.h[k] - self.h[self.to[k] as usize]).max(0.0) / self.run[k] }
    }

    /// The leaf of sample `k` (`SAMPLE_DEPTH`).
    pub fn leaf(&self, k: usize) -> Leaf {
        leaf_at(self.across, k)
    }

    /// The index of the sample over leaf `l` of `SAMPLE_DEPTH` or finer.
    pub fn index(&self, l: &Leaf) -> usize {
        let side = BLOCK >> SAMPLE_DEPTH;
        (l.y / side) as usize * self.across + (l.x / side) as usize
    }
}

/// The samples a terrain found, kept with it (not saved: found again when first read): the seed's, and those last built
/// from what the running world laid (`Terrain::built`), with the build they are of.
#[derive(Debug, Default)]
pub(super) struct Kept(Mutex<Option<Arc<Samples>>>, Mutex<Option<(u32, Arc<Samples>)>>);

impl Clone for Kept {
    fn clone(&self) -> Kept {
        Kept(Mutex::new(self.0.lock().expect("the kept samples").clone()), Mutex::new(self.1.lock().expect("the built samples").clone()))
    }
}

/// The loose ground of the seed's samples `s0` with what the relief `r` (`Relief::built`) laid on them over `years`
/// since the start: the relief is loose ground that settled (`Samples::loose`, `settled`), and the age of a sample's
/// loose ground the mean by volume of the seed's and the relief's, the relief laid evenly over the years since the
/// start, so half as old as they are on its mean.
fn laid_ground(s0: &Samples, r: &Relief, years: f64) -> Ground {
    let laid = |k: usize| r.built.get(k).copied().unwrap_or(0.0);
    let n = s0.h.len();
    let age = (0..n).map(|k| {
        let (old, new) = (s0.loose[k], laid(k));
        if old + new > 0.0 { (old * s0.age[k] + new * 0.5 * years) / (old + new) } else { s0.age[k] }
    });
    Ground {
        make: s0.make.clone(),
        loose: (0..n).map(|k| s0.loose[k] + laid(k)).collect(),
        settled: (0..n).map(|k| s0.settled[k] + laid(k)).collect(),
        age: age.collect(),
        books: s0.books,
    }
}

impl Terrain {
    /// The samples under leaf `l` of `map` (of `SAMPLE_DEPTH` or coarser; for `seed`, its rows between latitudes `lat`)
    /// set to their mean height by area: the ground inside it level, the ground round it as it was.
    pub(crate) fn level_under(&self, seed: u64, map: &Square, l: Leaf, lat: (f64, f64)) {
        let mut s = (*self.samples(seed, map, lat)).clone();
        let mean = self.inside(seed, map, l, lat);
        for f in l.split(SAMPLE_DEPTH) {
            let k = s.index(&f);
            s.h[k] = mean;
        }
        s.twi = sample_index(s.across, s.down, map.length(SAMPLE_DEPTH), &s.up, &s.h);
        *self.kept.0.lock().expect("the kept samples") = Some(Arc::new(s));
    }

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

    /// The samples of `map` for `seed` (its rows between latitudes `lat`) as built from the relief `r` (`Relief`): the
    /// seed's (`samples`) with the heights `relief::heights` gives, and the flood over them found again (`found`; no
    /// grading: it would move ground the books do not know), each sample keeping the seed's way where that still runs
    /// down (the offsets move a flat's samples by millimetres, which would turn its ways each build); a sample of the seed's open low ground is land where it
    /// stands over the liquid's level (`wear::STAND_M`). What the relief laid on a sample is loose ground that settled
    /// there (`laid_ground`), laid over `years` since the start. The seed's samples where there was no build; found
    /// once for each build and kept.
    pub fn built(&self, seed: u64, map: &Square, lat: (f64, f64), r: &Relief, years: f64) -> Arc<Samples> {
        let s0 = self.samples(seed, map, lat);
        if r.builds == 0 {
            return s0;
        }
        let known = |kept: &Option<(u32, Arc<Samples>)>| kept.as_ref().filter(|b| b.0 == r.builds && b.1.seed == seed).map(|b| b.1.clone());
        if let Some(s) = known(&self.kept.1.lock().expect("the built samples")) {
            return s;
        }
        let (grid, _) = grid_of(map, lat);
        let h = super::relief::heights(&s0, map, lat, r);
        let open = open_low(&grid, &h, STAND_M);
        let ground = laid_ground(&s0, r, years);
        let found = Arc::new(self.found(seed, &grid, (h, &open, false), ground, &s0.to));
        let mut kept = self.kept.1.lock().expect("the built samples");
        known(&kept).unwrap_or_else(|| {
            *kept = Some((r.builds, found.clone()));
            found
        })
    }

    /// The lobes the running world's mouths lay at a build of the samples `s` of `map` (`delta::lobes`, one call on the
    /// whole grid): the liquid stands on the samples of a body `body` (`NONE` for none) at each one's `level`, m; each mouth an entry sample, the
    /// load it brings, m³, and its top, m, its way in along its main channel (`Samples::main`, `fill::heading`); over `years`, nothing behind them. Returns each sample each lobe laid on, with the
    /// volume, m³, and the mouth's place in `mouths`; and the load they found no room for, m³. The ages of the layers are
    /// not read (the running world keeps no age of its relief).
    pub fn lobes_of(&self, map: &Square, lat: (f64, f64), s: &Samples, (body, level): (&[u32], &[f64]), mouths: &[(usize, f64, f64)], years: f64) -> (Vec<(usize, f64, u32)>, f64) {
        let (grid, _) = grid_of(map, lat);
        let open: Vec<bool> = body.iter().map(|&b| b != NONE).collect();
        let exposed = exposures(&grid, &open);
        let liquid = Liquid::new(&grid, (&open, &exposed), (level, body));
        let heading = |k: usize| if s.main[k] == NONE { [0.0; 2] } else { heading(&grid, |j| s.main[j], (s.main[k] as usize, k)) };
        let mut order: Vec<usize> = (0..mouths.len()).collect();
        order.sort_by(|&a, &b| mouths[b].1.total_cmp(&mouths[a].1).then(mouths[a].0.cmp(&mouths[b].0)));
        let laid_out: Vec<Mouth> = order.iter().map(|&m| Mouth { entry: mouths[m].0, load: mouths[m].1, top: mouths[m].2, heading: heading(mouths[m].0) }).collect();
        let (mut h, mut loose) = (s.h.clone(), s.loose.clone());
        let laid = lobes(&grid, (&mut h, &mut loose), (&liquid, &vec![false; grid.n()]), &laid_out, years);
        (laid.parts.into_iter().map(|(k, v, m)| (k, v, order[m as usize] as u32)).collect(), laid.unlaid)
    }

    /// The mean rise of the ground toward the south over leaf `l` of `map` (its rows between latitudes `south` and
    /// `north`) on the samples `s` (the seed's, or as built: `Terrain::built`), m per m: by area over the samples it covers, each its central difference north to south
    /// (one-sided at the map's edge); a leaf finer than a sample takes its sample's. So a leaf's is its quarters' mean by
    /// area at every depth. It is what the sun on a cell reads of the way the cell faces (`flow::on_slope`).
    pub fn rise_south_on(&self, s: &Samples, map: &Square, l: Leaf, (south, north): (f64, f64)) -> f64 {
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
    /// samples of the grid the wear reads it on (`wear::noise_grid`), worn (`space::wear`) with the open low ground
    /// held; then the flood over them from all the open low ground, the closed basins too shallow to hold a pond graded
    /// (`ponds::grade`), the area that drains through each sample, and the ponds; the folds of the bed's layers and its
    /// bodies are read on the noise's grid too, their rims at each sample (`Strata::at`).
    fn worn(&self, seed: u64, map: &Square, (south, north): (f64, f64)) -> Samples {
        let (grid, leaf) = grid_of(map, (south, north));
        let (across, down) = (grid.across, grid.down);
        let side = BLOCK >> SAMPLE_DEPTH;
        // The noise at the centres of the samples of the grid the wear reads it on: leaves `by` samples a side.
        let by = noise_grid(&grid);
        let (wide, depth) = (across / by, SAMPLE_DEPTH - by.trailing_zeros() as u8);
        let block = |k: usize| Leaf { x: (k % wide) as u64 * side * by as u64, y: (k / wide) as u64 * side * by as u64, depth };
        let (place, lift) = (|k: usize| map.place_of(&block(k), south, north).1, self.lift(seed, map));
        let at: Vec<(f64, (f64, f64))> = crate::par::map_n(wide * (down / by), crate::par::UNCOUNTED, |k| (self.sample(seed, map, &lift, place(k)), Strata::at(seed, map, place(k))));
        let (rise, (fold, body)): (Vec<f64>, (Vec<f64>, Vec<f64>)) = at.into_iter().unzip();
        let rim = crate::par::map_n(grid.n(), crate::par::UNCOUNTED, |k| Strata::rim(seed, map, map.place_of(&leaf(k), south, north).1));
        let worn = wear(seed, &grid, (by, &rise), (&fold, [&body, &rim], &Strata::of(seed, &self.makes)), self.channel);
        let ground = Ground { make: worn.make, loose: worn.loose, settled: worn.settled, age: worn.age, books: worn.books };
        self.found(seed, &grid, (worn.h, &worn.open, true), ground, &[])
    }

    /// The samples of heights `h` on grid `g` for `seed`, the open low ground `open` (`drain::open_low`), and their
    /// ground `ground`: the flood over them from all the open low ground, the closed basins too shallow to hold a pond
    /// (`ponds::shallow`) with `graded` filled (`ponds::grade`), else left as they lie but no ponds (the floods flow
    /// through them), the area that drains through each sample, the ponds, and the wetness index (`sample_index`); each
    /// sample's way the one `keep` gives it where that is still a way down (`drain::flood`; empty: none).
    fn found(&self, seed: u64, grid: &Grid, (mut h, open, graded): (Vec<f64>, &[bool], bool), ground: Ground, keep: &[u32]) -> Samples {
        let f = flood(grid, &h, open, (&starts(grid, open, false), &draws(seed, 1, grid.n())), keep);
        let (up, main) = accumulate(&f.to, &f.order, |k| grid.area(k));
        if graded {
            let shallow = basins(grid, &h, &f).1;
            grade(&mut h, &f, &shallow);
        }
        let (mut pond, mut members) = basins(grid, &h, &f);
        if !graded {
            let (deep, flat): (Vec<Vec<u32>>, Vec<Vec<u32>>) = members.into_iter().partition(|m| !shallow(&h, &f, m));
            flat.iter().flatten().for_each(|&k| pond[k as usize] = NONE);
            deep.iter().enumerate().for_each(|(p, m)| m.iter().for_each(|&k| pond[k as usize] = p as u32));
            members = deep;
        }
        let mut ponds: Vec<Pond> = members.iter().map(|m| Pond { pass: f.to[m[0] as usize], level: f.level[m[0] as usize] }).collect();
        open_parts(grid, open, (&mut pond, &mut ponds));
        let twi = sample_index(grid.across, grid.down, grid.dy, &up, &h);
        let Ground { make, loose, settled, age, books } = ground;
        Samples { seed, across: grid.across, down: grid.down, h, to: f.to, order: f.order, up, main, run: f.apart, pond, ponds, make, loose, settled, age, books, twi, channel: self.channel }
    }
}

/// The ground of the samples that the flood over them does not find: of each its bed's make, its loose ground, what of
/// it settled and how long it has lain (`Samples`), and the volumes the wear moved.
struct Ground {
    make: Vec<u8>,
    loose: Vec<f64>,
    settled: Vec<f64>,
    age: Vec<f64>,
    books: Books,
}

/// The grid of the samples of `map` (its rows between latitudes `south` and `north`): each row's samples as wide as
/// they lie there; and the leaf of each sample.
fn grid_of(map: &Square, (south, north): (f64, f64)) -> (Grid, impl Fn(usize) -> Leaf) {
    let per = 1usize << (SAMPLE_DEPTH - map.square_depth());
    let (across, down) = (map.w * per, map.h * per);
    let leaf = move |k: usize| leaf_at(across, k);
    let dy = map.length(SAMPLE_DEPTH);
    let grid = Grid::new(across, down, map.wrap, dy, (0..down).map(|r| map.place_of(&leaf(r * across), south, north).0 / dy).collect());
    (grid, leaf)
}

/// The leaf of sample `k` of samples `across` a row.
fn leaf_at(across: usize, k: usize) -> Leaf {
    let side = BLOCK >> SAMPLE_DEPTH;
    Leaf { x: (k % across) as u64 * side, y: (k / across) as u64 * side, depth: SAMPLE_DEPTH }
}

/// How much liquid gathers at each of the samples `across` x `down`, `dy` m a side, of heights `h`, m, through which
/// `up` m² drain (`network::accumulate`): the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979,
/// `inside::Grid::wetness_index`), a sample's width its side and its slope that of the samples' ground (`Grid::slope`).
/// A function of the place alone: the law (`inside::wet_ground`), the page's tiles and the tape read this one.
pub fn sample_index(across: usize, down: usize, dy: f64, up: &[f64], h: &[f64]) -> Vec<f64> {
    let grid = crate::inside::Grid::new(across, down, dy, vec![dy; down]);
    let own = vec![dy * dy; h.len()];
    crate::inside::Grid::wetness_index(up, &own, &grid.slope(h))
}
