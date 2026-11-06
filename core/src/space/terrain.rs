//! The terrain (laws.md 0 and 8): the height of the ground at any point of the map, one function of the world's seed,
//! its map and its level (`Terrain`), at any size. Value noise summed over octaves (fractal noise), with ridges, its
//! domain warped so coasts and ridges bend (Perlin 1985; Musgrave 1989), raised to heights; an island falls away from
//! the middle of its map. It gives the cells their starting heights (`worldgen`), and a split the ground its cell held
//! finer than the cell (`World::split`). Below `SAMPLE_DEPTH` the ground goes on as a midpoint displacement that
//! continues the noise's octaves (`Terrain::inside`), so a cell of any depth has a mean height and a relief.

use super::square::Square;
use super::tree::BLOCK;
use super::{Leaf, Level};
use crate::rng::{mix, Purpose, Rng};
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
/// An island's rise falls by (`FALL_BY` x its distance from the middle, in sides)^`FALL_POWER`.
const FALL_BY: f64 = 2.0;
const FALL_POWER: f64 = 1.6;
/// The rise an island's level 0 m lies at.
pub const ISLAND_LEVEL: f64 = 0.3;
/// The depth below its air block whose leaves' centres sample the terrain inside a coarser cell (`Terrain::inside`):
/// 1/64 of a block's side, 125 m on the island, finer than the noise's finest octave there.
pub const SAMPLE_DEPTH: u8 = 6;
/// The rms slope of real ground (rise over run) measured over `REAL_LENGTH_M`, and the Hurst exponent of its roughness:
/// the slope over a length ℓ goes as ℓ^(`HURST` − 1) (`real_slope`).
const REAL_SLOPE: f64 = 0.1;
const REAL_LENGTH_M: f64 = 30.0;
const HURST: f64 = 0.8;

/// What the terrain of a world needs beside its seed and its map: the rise its level 0 m lies at (an island's
/// `ISLAND_LEVEL`; a globe's the one that leaves a share of its area above it, `worldgen`).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Terrain {
    pub level: f64,
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
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += amp * value_noise(seed, p.map(|v| v * freq), o as u64);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// The rise of the ground at point `p` of the noise: fractal noise with ridges, its domain warped.
fn rise_at<const D: usize>(seed: u64, p: [f64; D]) -> f64 {
    let half = p.map(|v| v * 0.5);
    let warp: [f64; D] = std::array::from_fn(|d| p[d] + WARP_BY * (fbm(seed ^ WARP[d], half, OCTAVES[0]) - 0.5));
    let base = fbm(seed, warp, OCTAVES[1]);
    let ridge = 1.0 - (2.0 * fbm(seed ^ RIDGE_KEY, warp.map(|v| v * RIDGE_SCALE), OCTAVES[2]) - 1.0).abs();
    base * BASE_WEIGHT + ridge.powi(3) * RIDGE_WEIGHT
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

/// The rise of the ground at `place` of `map` (`rise_at`), `ACROSS` units of the noise across the map's longer side.
pub fn rise(seed: u64, map: &Square, place: Place) -> f64 {
    match spot(map, place, ACROSS / map.w.max(map.h) as f64) {
        Spot::Flat(p) => rise_at(seed, p),
        Spot::Round(p) => rise_at(seed, p),
    }
}

/// Height of ground whose rise lies `above` the level, m: land climbs steeply, low ground falls gently.
fn height_of(above: f64) -> f64 {
    if above > 0.0 { above.powf(LAND_POWER) * LAND_M } else { above * LOW_M }.max(FLOOR_M)
}

/// How fast `height_of` climbs with the rise at `above`, m per unit of rise (none on the floor).
fn height_slope(above: f64) -> f64 {
    if height_of(above) <= FLOOR_M {
        0.0
    } else if above > 0.0 {
        LAND_M * LAND_POWER * above.powf(LAND_POWER - 1.0)
    } else {
        LOW_M
    }
}

/// The mean square slope of smooth value noise of `dims` dimensions along one axis, per unit of its lattice:
/// (1/5) (26/35)^(dims − 1). Along an axis the noise is (1 − s) a + s b, s = 3t² − 2t³, between corners of variance
/// 1/12, so its slope s'(t) (b − a) has E[s'²] = 6/5 and E[(b − a)²] = 1/6; each other axis weighs its two corners by
/// (1 − s)² + s², 26/35 on the mean.
fn slope_variance(dims: i32) -> f64 {
    0.2 * (26.0f64 / 35.0).powi(dims - 1)
}

/// The rms slope of the rise, per m, that each octave of the base noise (`fbm`) adds on `map`: octave o has the weight
/// 2^−o over the octaves' sum and 2^o lattice units per unit of the noise, so each adds the same slope (a fractal of
/// Hurst exponent 1), `BASE_WEIGHT` / sum × √(2 · `slope_variance`) per unit along the ground (two ways across it); the noise has
/// `ACROSS` / (the longer side, in squares) units a square, flat or round.
fn octave_slope(map: &Square) -> f64 {
    let (dims, sum) = (if map.wrap { 3 } else { 2 }, 2.0 - 0.5f64.powi(OCTAVES[1] as i32 - 1));
    BASE_WEIGHT / sum * (2.0 * slope_variance(dims)).sqrt() * ACROSS / map.w.max(map.h) as f64 / map.side
}

/// The rms slope of real ground over a length `len`, m: self-affine, so it falls slowly as the length grows.
fn real_slope(len: f64) -> f64 {
    REAL_SLOPE * (len / REAL_LENGTH_M).powf(HURST - 1.0)
}

/// The slope G of the ground at a scale `len`, m, where the noise holds slope `noise` there: the larger of the two, so
/// ground the noise leaves smooth is as rough as real ground at that scale.
fn slope_at(noise: f64, len: f64) -> f64 {
    noise.max(real_slope(len))
}

/// The variance by area, m², of the heights below a leaf of `depth` whose noise slope is `noise`: each depth k down adds
/// (G_k ℓ_k / 4)² (the departures of the quarters of a leaf of side ℓ_k, `displace`). Where the noise's slope is the
/// larger the terms are those of a constant slope; from the depth where real ground's is, the terms fall as 4^−`HURST`
/// each depth, a geometric series summed whole.
fn variance_below(map: &Square, noise: f64, depth: u8) -> f64 {
    let mut sum = 0.0;
    for k in depth.. {
        let len = map.length(k);
        if real_slope(len) >= noise {
            return sum + (real_slope(len) * len).powi(2) / 16.0 / (1.0 - 0.25f64.powf(HURST));
        }
        sum += (noise * len).powi(2) / 16.0;
    }
    unreachable!("real ground's slope grows as the length falls")
}

/// The departures of the means of the four quarters (of areas `area`, m², row by row) of leaf `l` from its mean, m:
/// four normal draws keyed by the leaf, less their mean by area, scaled so their variance by area is exactly `var`, m².
fn displace(seed: u64, l: Leaf, area: [f64; 4], var: f64) -> [f64; 4] {
    let mut r = Rng::new(seed, l.depth as u64, mix(l.x ^ mix(l.y)), Purpose::Detail);
    let z: [f64; 4] = std::array::from_fn(|_| r.normal());
    let whole: f64 = area.iter().sum();
    let mean = (0..4).map(|q| area[q] * z[q]).sum::<f64>() / whole;
    let d = z.map(|x| x - mean);
    let spread = (0..4).map(|q| area[q] * d[q] * d[q]).sum::<f64>() / whole;
    if spread > 0.0 { d.map(|x| x * (var / spread).sqrt()) } else { [0.0; 4] }
}

impl Terrain {
    /// The rise of the ground at `place` of `map` for `seed` over the level; on an island (a flat map) less the fall
    /// from the middle of the map.
    fn above(&self, seed: u64, map: &Square, place: Place) -> f64 {
        let r = rise(seed, map, place);
        if map.wrap {
            return r - self.level;
        }
        // How far the place lies from the middle of the map along each axis, in sides.
        let middle = |c: f64, len: usize| c / map.side / len as f64 - 0.5;
        let (dx, dy) = (middle(place.centre[0], map.w), middle(place.centre[1], map.h));
        r - ((dx * dx + dy * dy).sqrt() * FALL_BY).powf(FALL_POWER) - self.level
    }
    /// The height of the ground at `place` of `map` for `seed`, m (0 m is the reference level).
    pub fn height(&self, seed: u64, map: &Square, place: Place) -> f64 {
        height_of(self.above(seed, map, place))
    }
    /// The height at `place`, m, and the rms slope the noise holds there finer than `SAMPLE_DEPTH`: the slope an octave
    /// of the base noise adds (`octave_slope`) times how fast the height climbs with the rise there.
    fn sample(&self, seed: u64, map: &Square, place: Place) -> (f64, f64) {
        let above = self.above(seed, map, place);
        (height_of(above), height_slope(above) * octave_slope(map))
    }
    /// The terrain inside leaf `l` of `map` (its rows between latitudes `south` and `north`): its mean height by area,
    /// m, and the spread of its heights about it, ∫ (h − mean)² / 2 over its area, m⁴ (a bed of density ρ standing at
    /// those heights holds ρ g times that more potential energy than a flat one of its volume).
    ///
    /// The noise is sampled at the centres of the leaves of `SAMPLE_DEPTH`. Below them the ground goes on as a midpoint
    /// displacement (Fournier, Fussell and Carpenter 1982) that continues the base noise's octaves: the means of the
    /// quarters of a leaf of side ℓ (north to south) depart from its mean by draws keyed by the leaf (`displace`), of
    /// variance by area exactly G² ℓ² / 16 (that of the quarters of flat ground tilted at slope G), G at least the slope of
    /// real ground at ℓ (`slope_at`) and at least that at its sample (`sample`). So the variance below a leaf is the sum
    /// of those over its depths (`variance_below`), and its spread A times that over 2. A leaf of `SAMPLE_DEPTH` or coarser: the mean of its samples, and their spread about it
    /// with that below each; a finer one: its sample's height with its ancestors' departures down to it, and the spread
    /// below it. Each sum is exact, so a leaf's spread is its quarters' and that of their means about its own.
    pub fn inside(&self, seed: u64, map: &Square, l: Leaf, (south, north): (f64, f64)) -> (f64, f64) {
        let below = |area: f64, g: f64, depth: u8| area * variance_below(map, g, depth) / 2.0;
        if l.depth <= SAMPLE_DEPTH {
            let fine: Vec<(f64, (f64, f64))> = l.split(SAMPLE_DEPTH).map(|f| map.place_of(&f, south, north)).map(|(a, p)| (a, self.sample(seed, map, p))).collect();
            let (area, sum) = fine.iter().fold((0.0, 0.0), |(a, s), &(da, (h, _))| (a + da, s + da * h));
            let mean = sum / area;
            let spread = fine.iter().map(|&(a, (h, _))| a * (h - mean) * (h - mean)).sum::<f64>() / 2.0;
            return (mean, spread + fine.iter().map(|&(a, (_, g))| below(a, g, SAMPLE_DEPTH)).sum::<f64>());
        }
        let side = BLOCK >> SAMPLE_DEPTH;
        let mut at = Leaf { x: l.x / side * side, y: l.y / side * side, depth: SAMPLE_DEPTH };
        let (mut h, g) = self.sample(seed, map, map.place_of(&at, south, north).1);
        while at.depth < l.depth {
            let quarters = at.quarters();
            let d = displace(seed, at, quarters.map(|q| map.place_of(&q, south, north).0), (slope_at(g, map.length(at.depth)) * map.length(at.depth) / 4.0).powi(2));
            let k = quarters.iter().position(|q| q.x <= l.x && l.x < q.x + q.side() && q.y <= l.y && l.y < q.y + q.side()).expect("a quarter holds the leaf");
            (h, at) = (h + d[k], quarters[k]);
        }
        (h, below(map.place_of(&l, south, north).0, g, l.depth))
    }
}
