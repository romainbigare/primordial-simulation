//! The terrain (laws.md 0 and 8): the height of the ground at any point of the map, one function of the world's seed,
//! its map and its level (`Terrain`), at any size. Value noise summed over octaves (fractal noise), with ridges, its
//! domain warped so coasts and ridges bend (Perlin 1985; Musgrave 1989), raised to heights, says how high the ground
//! would stand; an island's falls away from its rises (`space::rises`). The ground at the samples (`SAMPLE_DEPTH`) is
//! that worn at the world's start (`space::samples`, `space::wear`). It gives the cells their starting heights
//! (`worldgen`). Below `SAMPLE_DEPTH` the ground goes on smooth (`space::below`), so a leaf of any depth has a mean
//! height, and a shell draws that ground.

pub use super::rises::{Lift, Shape};
use super::samples::Samples;
use super::square::Square;
use super::{Leaf, Level};
use crate::rng::{Purpose, Rng};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

/// Units of the coarsest noise across a map's longer side (a globe's equator).
const ACROSS: f64 = 6.0;
/// The keys mixed into the seed of the noise that warps the terrain along each axis, and how far it warps, in units of
/// the noise.
const WARP: [u64; 3] = [11, 13, 17];
const WARP_BY: f64 = 1.5;
/// The key of the ridges' noise, its scale against the base's, and the weights of the base and of the ridges in the rise.
const RIDGE_KEY: u64 = 7;
const RIDGE_SCALE: f64 = 0.7;
const BASE_WEIGHT: f64 = 0.9;
const RIDGE_WEIGHT: f64 = 0.6;
/// The octaves of the warp, of the base and of the ridges.
const OCTAVES: [u32; 3] = [3, 6, 5];
/// Heights of a rise `above` the level, m: land climbs as `LAND_M` x above^`LAND_POWER`, low ground falls `LOW_M` per
/// unit, no lower than `FLOOR_M`.
const LAND_M: f64 = 3600.0;
const LAND_POWER: f64 = 1.3;
const LOW_M: f64 = 1500.0;
const FLOOR_M: f64 = -800.0;
/// The first octave of the base noise that keeps its own size where the land rises slowly (`rise`): a quarter of the
/// coarsest's wavelength and finer, the ground's own swells and hollows.
const FINE_FROM: u32 = 2;
/// The rise an island's level 0 m lies at.
pub const ISLAND_LEVEL: f64 = 0.27;
/// The least and the most scale of a drawn island's rises against the one rise's reach, and the halvings between them
/// that find the scale that gives its share of land (`Terrain::lift`): the share to about a hundredth.
const SCALES: [f64; 2] = [0.2, 3.0];
const SCALE_HALVINGS: usize = 10;
/// The depth below its air block whose leaves' centres sample the terrain inside a coarser cell (`Terrain::inside`):
/// 1/64 of a block's side, 125 m on the island, finer than the noise's finest octave there.
pub const SAMPLE_DEPTH: u8 = 6;

/// What the terrain of a world needs beside its map: the seed of its noise (the world's as built: a run that draws
/// chance anew from another seed, `World::seed`, stands on the same ground); the rise its level 0 m lies at (an
/// island's `ISLAND_LEVEL`; a globe's the one that leaves a share of its area above it, `worldgen`); how much faster
/// flowing liquid wears each make of its bed than the first (`space::strata`; the chemistry's, `worldgen`); an
/// island's large shape (`Shape`); and its drained samples, found from those when first read (`Terrain::samples`), not
/// saved.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Terrain {
    pub seed: u64,
    pub level: f64,
    pub cut: Vec<f64>,
    pub shape: Shape,
    #[serde(skip)]
    pub(super) kept: super::samples::Kept,
}

/// A point of the map: its place on a flat map (east, south), m, and its latitude and longitude, degrees.
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub centre: [f64; 2],
    pub lat: f64,
    pub lon: f64,
}

impl Place {
    /// The centre of cell `i` of `level`.
    pub fn of(level: &Level, i: usize) -> Place {
        Place { centre: level.centre()[i], lat: level.lat()[i], lon: level.lon()[i] }
    }
}

/// A draw in [0, 1) for the corner `at` of the noise's lattice.
fn hash01<const D: usize>(seed: u64, at: [i64; D], octave: u64) -> f64 {
    let bits = 64 / D as u32;
    let key = at.iter().fold(0u64, |k, &c| (k << bits) ^ (c as u64 & (u64::MAX >> (64 - bits))));
    Rng::new(seed, octave, key, Purpose::Terrain).f64()
}

/// Smooth value noise in [0, 1) at point `p` (2 or 3 dimensions): the draws of the corners of its lattice cell, eased in
/// along each axis.
fn value_noise<const D: usize>(seed: u64, p: [f64; D], octave: u64) -> f64 {
    let lo = p.map(f64::floor);
    let ease = |t: f64| t * t * (3.0 - 2.0 * t);
    // Corner `c` lies at `lo` plus bit `d` of `c` along axis `d`; each axis in turn folds the corners in pairs.
    let mut v = [0.0; 8];
    for (c, x) in v.iter_mut().enumerate().take(1 << D) {
        *x = hash01::<D>(seed, std::array::from_fn(|d| lo[d] as i64 + (c >> d & 1) as i64), octave);
    }
    let mut len = 1 << D;
    for (&x, &l) in p.iter().zip(&lo) {
        let t = ease(x - l);
        len /= 2;
        for c in 0..len {
            v[c] = v[2 * c] + (v[2 * c + 1] - v[2 * c]) * t;
        }
    }
    v[0]
}

/// Fractal noise in roughly [0, 1): `octaves` of value noise, each of twice the frequency and half the weight.
fn fbm<const D: usize>(seed: u64, p: [f64; D], octaves: u32) -> f64 {
    fbm_parts(seed, p, octaves, octaves).0
}

/// `fbm`, and the part of it the octaves from `fine` on add about their mean.
fn fbm_parts<const D: usize>(seed: u64, p: [f64; D], octaves: u32, fine: u32) -> (f64, f64) {
    let (mut sum, mut finer, mut amp, mut freq, mut norm) = (0.0, 0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        let v = value_noise(seed, p.map(|v| v * freq), o as u64);
        sum += amp * v;
        if o >= fine {
            finer += amp * (v - 0.5);
        }
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    (sum / norm, finer / norm)
}

/// Point `p` of the noise moved by the noise that warps the terrain, so coasts, ridges and the range bend.
fn warped<const D: usize>(seed: u64, p: [f64; D]) -> [f64; D] {
    let half = p.map(|v| v * 0.5);
    std::array::from_fn(|d| p[d] + WARP_BY * (fbm(seed ^ WARP[d], half, OCTAVES[0]) - 0.5))
}

/// The rise of the ground at the warped point `warp` of the noise: fractal noise with ridges; and the part of it its
/// base's octaves from `FINE_FROM` on add about their mean, the ground's own swells and hollows.
fn rise_at<const D: usize>(seed: u64, warp: [f64; D]) -> (f64, f64) {
    let (base, fine) = fbm_parts(seed, warp, OCTAVES[1], FINE_FROM);
    let ridge = 1.0 - (2.0 * fbm(seed ^ RIDGE_KEY, warp.map(|v| v * RIDGE_SCALE), OCTAVES[2]) - 1.0).abs();
    (base * BASE_WEIGHT + ridge.powi(3) * RIDGE_WEIGHT, fine * BASE_WEIGHT)
}

/// Fractal noise of `octaves` (in roughly [0, 1)) at `place` of `map` with key `seed`, `per_square` units of the noise a
/// square: on a flat map at its place, in squares, less half a square; on a globe at the point over it on a sphere
/// `per_square` units a square round its equator (no seam where the rows wrap).
pub fn noise(seed: u64, map: &Square, place: Place, per_square: f64, octaves: u32) -> f64 {
    match spot(map, place, per_square) {
        Spot::Flat(p) => fbm(seed, p, octaves),
        Spot::Round(p) => fbm(seed, p, octaves),
    }
}

/// Where a place lies in the noise (`noise`).
enum Spot {
    Flat([f64; 2]),
    Round([f64; 3]),
}

fn spot(map: &Square, place: Place, per_square: f64) -> Spot {
    if !map.wrap {
        return Spot::Flat(place.centre.map(|c| (c / map.side - 0.5) * per_square));
    }
    let (r, lat, lon) = (per_square * map.w as f64 / TAU, place.lat.to_radians(), place.lon.to_radians());
    Spot::Round([r * lat.cos() * lon.cos(), r * lat.cos() * lon.sin(), r * lat.sin()])
}

/// The rise of the ground at `place` of `map` (`rise_at`), `ACROSS` units of the noise across the map's longer side;
/// and how much of it stands over `level`, its land's as fast as the land rises there: an island's large shape (all
/// but its fine octaves) lowered by how far it lies from the island's rises and scaled by their ranges (`Lift`), its
/// fine octaves as much larger where a range raises the land faster than the mean and at their own size elsewhere, so a
/// plain keeps its own swells and hollows while its large heights stay low; a globe's alike everywhere (its noise is of
/// the scale of its continents, its cells wider than a range). The second is the first less `level` where the land
/// rises at the mean.
pub fn rise(seed: u64, map: &Square, lift: &Lift, place: Place, level: f64) -> (f64, f64) {
    match spot(map, place, per_square(map)) {
        Spot::Flat(p) => {
            let warp = warped(seed, p);
            let (r, fine) = rise_at(seed, warp);
            let share = |c: f64, len: usize| c / map.side / len as f64;
            let fast = lift.fast(warp);
            let large = r - fine - level - lift.fall([share(place.centre[0], map.w), share(place.centre[1], map.h)]);
            (r, if large > 0.0 { large * fast } else { large } + fine * fast.max(1.0))
        }
        Spot::Round(p) => {
            let r = rise_at(seed, warped(seed, p)).0;
            (r, r - level)
        }
    }
}

/// Units of the noise a square of `map`: `ACROSS` across its longer side.
pub(super) fn per_square(map: &Square) -> f64 {
    ACROSS / map.w.max(map.h) as f64
}

/// Height of ground whose rise lies `above` the level, m: land climbs steeply, low ground falls gently.
fn height_of(above: f64) -> f64 {
    if above > 0.0 { above.powf(LAND_POWER) * LAND_M } else { above * LOW_M }.max(FLOOR_M)
}

impl Terrain {
    /// The terrain of `seed` whose level 0 m lies at the rise `level`, flowing liquid wearing each make of its bed
    /// `cut` times as fast as the first.
    pub fn new(seed: u64, level: f64, cut: Vec<f64>) -> Terrain {
        Terrain { seed, level, cut, ..Terrain::default() }
    }
    /// The terrain of an island of `shape` for `seed` (`Lift`), its level 0 m at `ISLAND_LEVEL`.
    pub fn island(seed: u64, shape: Shape, cut: Vec<f64>) -> Terrain {
        Terrain { seed, level: ISLAND_LEVEL, cut, shape, ..Terrain::default() }
    }
    /// The rises of this terrain's island on the flat map `map` for `seed` (`Lift`): one in the middle, or those drawn
    /// from the seed, scaled so the share of the centres of its squares above the level (`land`) is their drawn share
    /// (`rises::drawn_share`), found by halving `SCALE_HALVINGS` times between `SCALES`.
    pub fn lift(&self, seed: u64, map: &Square) -> Lift {
        if self.shape == Shape::One {
            return Lift::one(seed, map);
        }
        let (share, mut range) = (super::rises::drawn_share(seed), SCALES);
        for _ in 0..SCALE_HALVINGS {
            let mid = (range[0] + range[1]) / 2.0;
            if self.land(seed, map, &Lift::drawn(seed, map, mid)) < share {
                range[0] = mid;
            } else {
                range[1] = mid;
            }
        }
        Lift::drawn(seed, map, (range[0] + range[1]) / 2.0)
    }
    /// The share of the centres of the squares of the flat map `map` whose noise stands above the level for `seed`, its
    /// island raised by `lift` (`rise`).
    fn land(&self, seed: u64, map: &Square, lift: &Lift) -> f64 {
        let above = |s: usize| {
            let (x, y) = map.xy(s);
            let place = Place { centre: [(x as f64 + 0.5) * map.side, (y as f64 + 0.5) * map.side], lat: 0.0, lon: 0.0 };
            rise(seed, map, lift, place, self.level).1 > 0.0
        };
        crate::par::map_n(map.n(), crate::par::UNCOUNTED, above).iter().filter(|&&a| a).count() as f64 / map.n() as f64
    }
    /// The height of the noise at `place` of `map` for `seed`, its island raised by `lift`, m: how high the ground
    /// would stand there unworn.
    pub(super) fn sample(&self, seed: u64, map: &Square, lift: &Lift, place: Place) -> f64 {
        height_of(rise(seed, map, lift, place, self.level).1)
    }
    /// The terrain's mean height by area over leaf `l` of `map` (its rows between latitudes `south` and `north`), m.
    /// The ground is sampled at the centres of the leaves of `SAMPLE_DEPTH`, worn (`Terrain::samples`); below them it
    /// goes on smooth (`space::below`). A leaf of `SAMPLE_DEPTH` or coarser: the mean of its samples; a finer one: its
    /// height there (`Terrain::below`).
    pub fn inside(&self, seed: u64, map: &Square, l: Leaf, lat: (f64, f64)) -> f64 {
        if l.depth <= SAMPLE_DEPTH {
            return self.over(seed, map, l, lat, |s, k| s.h[k]);
        }
        self.below(seed, map, l, lat)
    }
    /// The mean by area of `x(samples, k)` over the samples `k` of leaf `l` of `map` (of `SAMPLE_DEPTH` or coarser;
    /// for `seed`, its rows between latitudes `south` and `north`).
    pub fn over(&self, seed: u64, map: &Square, l: Leaf, (south, north): (f64, f64), x: impl Fn(&Samples, usize) -> f64) -> f64 {
        let s = self.samples(seed, map, (south, north));
        let (area, sum) = l.split(SAMPLE_DEPTH).map(|f| (map.place_of(&f, south, north).0, x(&s, s.index(&f)))).fold((0.0, 0.0), |(a, sum), (da, v)| (a + da, sum + da * v));
        sum / area
    }
}
