//! The terrain's samples (laws.md 0 and 5), the leaves of `SAMPLE_DEPTH` of the map: the noise says how high the ground
//! would stand (`Terrain::sample`), and the ground is that worn by flowing liquid and creep at the world's start
//! (`space::wear`), so valleys branch between ridges and the closed basins the cutting left stay. A
//! flood over the worn heights (`space::drain`) finds where each sample's liquid runs, and the ponds: the samples
//! under a closed basin's level, each pond with its pass and the level of the pass. Every cell's height is a mean over
//! these samples (`Terrain::inside`); so is the way it faces the sun (`Samples::rise_south`); and the liquid runs down
//! them (`space::network`). They are a function of the seed and the map alone, found when first read and kept with the
//! terrain, not saved.

use super::delta::{exposures, lobes, Liquid, Mouth};
use super::drain::{draws, flood, open_from, starts};
use super::fill::heading;
use super::relief::{Lift, Relief};
use super::grid::Grid;
use super::mantle::edited;
use super::network::{accumulate, NONE};
use super::ponds::{basins, grade, open_parts, renumber, shallow};
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
/// ln(a / tan β) (`sample_index`); the channel law of the terrain (`Terrain::channel`); and what edits had shaped of
/// each sample when these were built from the relief, m (`Relief::shaped`; none for the seed's).
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
    /// The area of a sample of each row, m² (`Square::place_of`, found with the samples): a sample's is its row's.
    pub rows: Vec<f64>,
    pub shaped: Vec<f64>,
}

/// A pond: the samples of one closed basin that lie under its level, where liquid stands at one level before it runs
/// over the basin's lowest pass. Its pass is the first sample out of it on the way its liquid leaves by, and the level
/// is that of the pass, m. The open low ground is ponds too, one for each part of it joined through its samples (an
/// island's one): its liquid leaves by no pass (`network::NONE`) and stands at any level (infinite). A place of the
/// list no pond holds (`ponds::renumber`) has no pass and no level (NaN).
#[derive(Clone, Copy, Debug)]
pub struct Pond {
    pub pass: u32,
    pub level: f64,
}

impl Samples {
    /// The key of each place of the ponds, numbered after the keys `was` (`ponds::kept_keys`): the first of its pond's
    /// samples, and for an empty place the key it had.
    pub fn pond_keys(&self, was: &[u32]) -> Vec<u32> {
        super::ponds::kept_keys(&self.pond, self.ponds.len(), was)
    }
    /// The keys `was` of the places of its ponds (`Relief::ponds`) for a build of the years to number its ponds after:
    /// the empty places dropped where they are too many (`ponds::compact`).
    pub fn compact_keys(&self, was: &[u32]) -> Vec<u32> {
        super::ponds::compact(was, &self.ponds)
    }
    /// The samples of the open low ground (the ponds of no pass).
    pub fn open_samples(&self) -> Vec<usize> {
        (0..self.pond.len()).filter(|&k| self.pond[k] != NONE && self.ponds[self.pond[k] as usize].pass == NONE).collect()
    }
    /// The area of sample `k`, m² (its row's, `rows`).
    #[inline]
    pub fn area(&self, k: usize) -> f64 {
        self.rows[k / self.across]
    }

    /// The mean by area of `x(samples, k)` over the samples `k` of leaf `l` (of `SAMPLE_DEPTH` or coarser).
    pub fn over(&self, l: Leaf, x: impl Fn(&Samples, usize) -> f64) -> f64 {
        let (area, sum) = l.split(SAMPLE_DEPTH).map(|f| self.index(&f)).map(|k| (self.area(k), x(self, k))).fold((0.0, 0.0), |(a, sum), (da, v)| (a + da, sum + da * v));
        sum / area
    }

    /// The mean rise of the ground toward the south over leaf `l` of `map`, m per m: by area over the samples it covers,
    /// each its central difference north to south (one-sided at the map's edge); a leaf finer than a sample takes its
    /// sample's. So a leaf's is its quarters' mean by area at every depth. It is what the sun on a cell reads of the way
    /// the cell faces (`flow::on_slope`).
    pub fn rise_south(&self, map: &Square, l: Leaf) -> f64 {
        let dy = map.length(SAMPLE_DEPTH);
        let at = |f: &Leaf| {
            let k = self.index(f);
            central(k / self.across, self.down, |r| self.h[r * self.across + k % self.across], dy)
        };
        if l.depth <= SAMPLE_DEPTH {
            let (area, sum) = l.split(SAMPLE_DEPTH).map(|f| (self.area(self.index(&f)), at(&f))).fold((0.0, 0.0), |(a, by), (da, r)| (a + da, by + da * r));
            return sum / area;
        }
        at(&l)
    }

    /// What edits of the relief `r` shaped of sample `k` since these samples were built, m (`Relief::shaped`).
    pub fn shaped_since(&self, r: &Relief, k: usize) -> f64 {
        r.shaped_at(k) - self.shaped.get(k).copied().unwrap_or(0.0)
    }

    /// The height of the ground of sample `k` as it stands, m: as built, with what edits of the relief `r` shaped since
    /// (`shaped_since`).
    pub fn height_now(&self, r: &Relief, k: usize) -> f64 {
        self.h[k] + self.shaped_since(r, k)
    }

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
/// from what the running world laid and edits shaped (`Terrain::built`), with the version of the relief they are of;
/// how the cells' offsets last raised them (`relief::Lift`), which serves every version of the same offsets (edits
/// that shape the ground move no offset, `flow::build::reshape`); and the draws of the ways down for a seed and a
/// number of samples (`drain::draws`, which every build reads).
#[derive(Debug, Default)]
pub(super) struct Kept(Mutex<Option<Arc<Samples>>>, Mutex<Option<(u32, Arc<Samples>)>>, Mutex<Option<Arc<Lift>>>, Mutex<Option<(u64, Arc<Vec<f64>>)>>);

impl Clone for Kept {
    fn clone(&self) -> Kept {
        let (seed, built, lift) = (self.0.lock().expect("the kept samples"), self.1.lock().expect("the built samples"), self.2.lock().expect("the lift"));
        let draws = self.3.lock().expect("the draws").clone();
        Kept(Mutex::new(seed.clone()), Mutex::new(built.clone()), Mutex::new(lift.clone()), Mutex::new(draws))
    }
}

/// The loose ground of the seed's samples `s0` with what the relief `r` (`Relief::built`) laid on them over `years`
/// since the start: the relief is loose ground that settled (`Samples::loose`, `settled`), and the age of a sample's
/// loose ground the mean by volume of the seed's and the relief's, the relief laid evenly over the years since the
/// start, so half as old as they are on its mean.
fn laid_ground(s0: &Samples, r: &Relief, years: f64) -> Ground {
    let laid = |k: usize| laid_on(r, k);
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

/// What the relief `r` laid on sample `k`, m (`Relief::built`).
fn laid_on(r: &Relief, k: usize) -> f64 {
    r.built.get(k).copied().unwrap_or(0.0)
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
    /// grading: it would move ground the books do not know); a sample of the seed's open low ground is land where it
    /// stands over the liquid's level (`wear::STAND_M`), and the open low ground is what lies under that level joined
    /// to the lowest sample of the seed's (`drain::open_from`), so a pit dug inland, however deep, is a closed basin. What the relief laid on
    /// a sample is loose ground that settled there (`laid_ground`), laid over `years` since the start; where edits
    /// shaped a sample its loose ground follows on its new slope (`mantle::edited`). The ponds are numbered after those
    /// the relief keeps (`Relief::ponds`, `ponds::renumber`). The seed's samples where there was no build (no version, or a relief of no
    /// cells' offsets: a world put back to before its first build, `rename_built`); found once for each version of the
    /// relief (`Relief::version`) and kept.
    pub fn built(&self, seed: u64, map: &Square, lat: (f64, f64), r: &Relief, years: f64) -> Arc<Samples> {
        let s0 = self.samples(seed, map, lat);
        if r.version == 0 || r.offset.is_empty() {
            return s0;
        }
        let known = |kept: &Option<(u32, Arc<Samples>)>| kept.as_ref().filter(|b| b.0 == r.version && b.1.seed == seed).map(|b| b.1.clone());
        if let Some(s) = known(&self.kept.1.lock().expect("the built samples")) {
            return s;
        }
        let (grid, rows, _) = grid_of(map, lat);
        let known_lift = self.kept.2.lock().expect("the lift").as_ref().filter(|l| l.offset == r.offset).cloned();
        let lift = known_lift.unwrap_or_else(|| {
            let l = Arc::new(super::relief::lift(&s0, map, lat, r));
            *self.kept.2.lock().expect("the lift") = Some(l.clone());
            l
        });
        let h = super::relief::heights(&s0, r, &lift);
        let open = open_from(&grid, &h, STAND_M, &s0.open_samples());
        let ground = laid_ground(&s0, r, years);
        // (The wetness index is found again where it can differ from that of the samples last built, or the seed's.)
        let last = self.kept.1.lock().expect("the built samples").as_ref().filter(|b| b.1.seed == seed).map(|b| b.1.clone());
        let mut found = self.found(seed, (&grid, rows), (h, &open, false), (ground, Some(last.as_deref().unwrap_or(&s0))));
        for k in (0..r.shaped.len()).filter(|&k| r.shaped[k] != 0.0) {
            [found.loose[k], found.settled[k]] = edited(&grid, k, (&|j| found.h[j], found.up[k]), (found.loose[k], found.settled[k]), &|j| r.shaped[j]);
        }
        renumber(&mut found.pond, &mut found.ponds, &r.ponds);
        found.shaped = r.shaped.clone();
        let found = Arc::new(found);
        let mut kept = self.kept.1.lock().expect("the built samples");
        known(&kept).unwrap_or_else(|| {
            *kept = Some((r.version, found.clone()));
            found
        })
    }

    /// The samples kept as built for the version `from` of the relief kept as those of version `to` (`built`): a world
    /// put back as it was (an undo) takes a new version, so no version names two grounds, and keeps its samples.
    pub fn rename_built(&self, from: u32, to: u32) {
        if let Some(b) = self.kept.1.lock().expect("the built samples").as_mut().filter(|b| b.0 == from) {
            b.0 = to;
        }
    }

    /// The lobes the running world's mouths lay at a build of the samples `s` of `map` (`delta::lobes`, one call on the
    /// whole grid): the liquid stands on the samples of a body `body` (`NONE` for none) at each one's `level`, m; each mouth an entry sample, the
    /// load it brings, m³, and its top, m, its way in along its main channel (`Samples::main`, `fill::heading`); over `years`, nothing behind them. Returns each sample each lobe laid on, with the
    /// volume, m³, and the mouth's place in `mouths`; and the load they found no room for, m³. The ages of the layers are
    /// not read (the running world keeps no age of its relief).
    pub fn lobes_of(&self, map: &Square, lat: (f64, f64), s: &Samples, (body, level): (&[u32], &[f64]), mouths: &[(usize, f64, f64)], years: f64) -> (Vec<(usize, f64, u32)>, f64) {
        let (grid, _, _) = grid_of(map, lat);
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

    /// The samples of `map` for `seed` (`Samples`): the noise's heights (`Terrain::sample`) at the centres of the
    /// samples of the grid the wear reads it on (`wear::noise_grid`), worn (`space::wear`) with the open low ground
    /// held; then the flood over them from all the open low ground, the closed basins too shallow to hold a pond graded
    /// (`ponds::grade`), the area that drains through each sample, and the ponds; the folds of the bed's layers and its
    /// bodies are read on the noise's grid too, their rims at each sample (`Strata::at`).
    fn worn(&self, seed: u64, map: &Square, (south, north): (f64, f64)) -> Samples {
        let (grid, rows, leaf) = grid_of(map, (south, north));
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
        self.found(seed, (&grid, rows), (worn.h, &worn.open, true), (ground, None))
    }

    /// The draws of the ways down of `n` samples for `seed` (`drain::draws`), found once and kept.
    pub(super) fn draws(&self, seed: u64, n: usize) -> Arc<Vec<f64>> {
        let mut kept = self.kept.3.lock().expect("the draws");
        match kept.as_ref().filter(|d| d.0 == seed && d.1.len() == n) {
            Some(d) => d.1.clone(),
            None => {
                let d = Arc::new(draws(seed, 1, n));
                *kept = Some((seed, d.clone()));
                d
            }
        }
    }

    /// The samples of heights `h` on grid `g` for `seed`, the open low ground `open` (`drain::open_low`), and their
    /// ground `ground`: the flood over them from all the open low ground, the closed basins too shallow to hold a pond
    /// (`ponds::shallow`) with `graded` filled (`ponds::grade`), else left as they lie but no ponds (the floods flow
    /// through them), the area that drains through each sample, the ponds, and the wetness index (`sample_index`; found
    /// again only where it can differ from that of the samples `was` where given, `index_from`).
    fn found(&self, seed: u64, (grid, rows): (&Grid, Vec<f64>), (mut h, open, graded): (Vec<f64>, &[bool], bool), (ground, was): (Ground, Option<&Samples>)) -> Samples {
        let f = flood(grid, &h, open, (&starts(grid, open, false), &self.draws(seed, grid.n())));
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
        let twi = match was.filter(|p| p.h.len() == h.len()) {
            Some(p) => index_from(p, (grid.across, grid.down, grid.dy), (&up, &h)),
            None => sample_index(grid.across, grid.down, grid.dy, &up, &h),
        };
        let Ground { make, loose, settled, age, books } = ground;
        Samples { seed, across: grid.across, down: grid.down, h, to: f.to, order: f.order, up, main, run: f.apart, pond, ponds, make, loose, settled, age, books, twi, channel: self.channel, rows, shaped: Vec::new() }
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
/// they lie there; the area of a sample of each row, m² (`Square::place_of`); and the leaf of each sample.
pub(super) fn grid_of(map: &Square, (south, north): (f64, f64)) -> (Grid, Vec<f64>, impl Fn(usize) -> Leaf) {
    let per = 1usize << (SAMPLE_DEPTH - map.square_depth());
    let (across, down) = (map.w * per, map.h * per);
    let leaf = move |k: usize| leaf_at(across, k);
    let dy = map.length(SAMPLE_DEPTH);
    let rows: Vec<f64> = (0..down).map(|r| map.place_of(&leaf(r * across), south, north).0).collect();
    let grid = Grid::new(across, down, map.wrap, dy, rows.iter().map(|a| a / dy).collect());
    (grid, rows, leaf)
}

/// The leaf of sample `k` of samples `across` a row.
fn leaf_at(across: usize, k: usize) -> Leaf {
    let side = BLOCK >> SAMPLE_DEPTH;
    Leaf { x: (k % across) as u64 * side, y: (k / across) as u64 * side, depth: SAMPLE_DEPTH }
}

/// How much liquid gathers at each of the samples `across` x `down`, `dy` m a side, of heights `h`, m, through which
/// `up` m² drain (`network::accumulate`): the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979,
/// `inside::Grid::wetness_at`), a sample's width its side and its slope that of the samples' ground (`Grid::slope_at`).
/// A function of the place alone: the law (`inside::wet_ground`), the page's tiles and the tape read this one.
pub fn sample_index(across: usize, down: usize, dy: f64, up: &[f64], h: &[f64]) -> Vec<f64> {
    let grid = index_grid(across, down, dy);
    (0..h.len()).map(|k| index_at(&grid, dy, (up, h), k)).collect()
}

/// The grid the wetness index is read on (`sample_index`): samples `dy` m a side.
fn index_grid(across: usize, down: usize, dy: f64) -> crate::inside::Grid {
    crate::inside::Grid::new(across, down, dy, vec![dy; down])
}

/// The wetness index of sample `k` of `grid`, its samples `dy` m a side (`sample_index`).
fn index_at(grid: &crate::inside::Grid, dy: f64, (up, h): (&[f64], &[f64]), k: usize) -> f64 {
    crate::inside::Grid::wetness_at(up[k], dy * dy, grid.slope_at(h, k))
}

/// The wetness index of samples `across` x `down` as `sample_index` finds it, found again only where it can differ from
/// that of the samples `was` of the same grid: where what drains through a sample moved, or the height of the sample or
/// of one beside it across a side (`Grid::slope_at`), to the last bit; elsewhere it is `was`'s.
fn index_from(was: &Samples, (across, down, dy): (usize, usize, f64), (up, h): (&[f64], &[f64])) -> Vec<f64> {
    let grid = index_grid(across, down, dy);
    let moved: Vec<bool> = h.iter().zip(&was.h).map(|(a, b)| a.to_bits() != b.to_bits()).collect();
    let near = |k: usize| {
        let (r, c) = (k / across, k % across);
        moved[k] || (c > 0 && moved[k - 1]) || (c + 1 < across && moved[k + 1]) || (r > 0 && moved[k - across]) || (r + 1 < down && moved[k + across])
    };
    (0..h.len()).map(|k| if near(k) || up[k].to_bits() != was.up[k].to_bits() { index_at(&grid, dy, (up, h), k) } else { was.twi[k] }).collect()
}

impl crate::World {
    /// Of each of the terrain's samples `ks`, the loose ground the samples built from the world's relief as it stands
    /// will hold (`Terrain::built`, `mantle::edited`) where they stand at heights `h`, m: what was laid on it and what
    /// edits shaped of it, on the slope of `h` there, draining what it drained at the last build. A shell reads it to
    /// draw the ground an edit moves before the samples are built again.
    pub fn loose_to_build(&self, ks: &[usize], h: &dyn Fn(usize) -> f64) -> Vec<f64> {
        let lat = (self.cfg.lat_south, self.cfg.lat_north);
        let (grid, _, _) = grid_of(&self.map, lat);
        let (s0, built) = (self.terrain.samples(self.terrain.seed, &self.map, lat), self.terrain_samples());
        let r = &self.relief;
        ks.iter().map(|&k| edited(&grid, k, (h, built.up[k]), (s0.loose[k] + laid_on(r, k), s0.settled[k] + laid_on(r, k)), &|j| r.shaped_at(j))[0]).collect()
    }
}
