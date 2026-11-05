//! The terrain (laws.md 0 and 8): the height of the ground at any point of the map, one function of the world's seed,
//! its map and its level (`Terrain`), at any size. Value noise summed over octaves (fractal noise), with ridges, its
//! domain warped so coasts and ridges bend (Perlin 1985; Musgrave 1989), raised to heights; an island falls away from
//! the middle of its map. It gives the cells their starting heights (`worldgen`), and a split the ground its cell held
//! finer than the cell (`World::split`).

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
/// An island's rise falls by (`FALL_BY` x its distance from the middle, in sides)^`FALL_POWER`.
const FALL_BY: f64 = 2.0;
const FALL_POWER: f64 = 1.6;
/// The rise an island's level 0 m lies at.
pub const ISLAND_LEVEL: f64 = 0.3;

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

impl Terrain {
    /// The height of the ground at `place` of `map` for `seed`, m (0 m is the reference level): its rise over the level;
    /// on an island (a flat map) less the fall from the middle of the map.
    pub fn height(&self, seed: u64, map: &Square, place: Place) -> f64 {
        let r = rise(seed, map, place);
        if map.wrap {
            return height_of(r - self.level);
        }
        // How far the place lies from the middle of the map along each axis, in sides.
        let middle = |c: f64, len: usize| c / map.side / len as f64 - 0.5;
        let (dx, dy) = (middle(place.centre[0], map.w), middle(place.centre[1], map.h));
        height_of(r - ((dx * dx + dy * dy).sqrt() * FALL_BY).powf(FALL_POWER) - self.level)
    }
    /// The terrain inside leaf `l` of `map` (its rows between latitudes `south` and `north`), sampled at the centres of
    /// its leaves of the most depth (`DEPTH_MOST`): their mean height by area, m, and the spread of their heights about
    /// it, Σ a (h − mean)² / 2, m⁴ (a bed of density ρ standing at those heights holds ρ g times that more potential
    /// energy than a flat one of its volume).
    pub fn inside(&self, seed: u64, map: &Square, l: Leaf, (south, north): (f64, f64)) -> (f64, f64) {
        let fine: Vec<(f64, f64)> = l.split(super::tree::DEPTH_MOST).map(|f| map.place_of(&f, south, north)).map(|(a, p)| (a, self.height(seed, map, p))).collect();
        let (area, sum) = fine.iter().fold((0.0, 0.0), |(a, s), &(da, h)| (a + da, s + da * h));
        let mean = sum / area;
        (mean, fine.iter().map(|&(a, h)| a * (h - mean) * (h - mean)).sum::<f64>() / 2.0)
    }
}
