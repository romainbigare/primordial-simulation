//! The large shape of an island's land (laws.md 0 and 5): where its crust is raised and how strongly (`Lift`). The
//! terrain's noise is lowered by how far a place lies from the rises (`Lift::fall`), and each rise has its range, where
//! the land rises fastest (`Range`). An island of one rise (`Shape::One`) has one rise in the middle of its map; an
//! island of drawn rises (`Shape::Drawn`) has one to several, their count, places, sizes, shapes and strengths drawn
//! from its seed, with low ground between them that the liquid fills: one land, two or three, a main land with
//! outliers, a long land, now and then a breached ring round an inner basin.

use super::square::Square;
use super::terrain::{across, per_square, ACROSS};
use crate::rng::{Purpose, Rng};
use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};

/// The land of an island of one rise falls by (its distance in reaches)^`FALL_POWER`; a drawn rise's by a power drawn
/// in `POWER`, a broad dome to a block with a flat top and steep flanks.
const FALL_POWER: f64 = 1.6;
const POWER: [f64; 2] = [1.8, 3.0];
/// The reach of the rise of an island of one rise, in sides of its map: its land falls by (2 x the distance from the
/// middle, in sides)^`FALL_POWER`.
const ONE_REACH: f64 = 0.5;
/// An island's range (`Range`): the key of the draws of an island of one rise; how far its axis lies from its rise's
/// middle at most, and the least and the most half-width of its band, in units of the noise for a rise of `ONE_REACH`
/// on a map `ACROSS` units wide (a range is as wide as its raised block is); and how fast land rises far from it and on
/// its axis, times the mean.
const RANGE_KEY: u64 = 19;
const RANGE_OFF: f64 = 1.0;
const RANGE_WIDE: [f64; 2] = [0.7, 1.4];
const RISES: [f64; 2] = [0.2, 1.3];
/// The key of the draws of the drawn rises (`Lift::drawn`).
const SHAPE_KEY: u64 = 23;
/// The share of a drawn island's map its unworn land holds, the least and the most.
const SHARE: [f64; 2] = [0.3, 0.45];
/// The least area of land one drawn rise raises, km²: the most rises a map holds is the count of lands down to that
/// size that the number-area rule gives for all its land (`most_rises`).
const LEAST_KM2: f64 = 600.0;
/// The exponent B of the number-area rule of a drawn island's rises, the least and the most: the k-th largest raises
/// an area as k^(-1/B) (Korčák 1938; Mandelbrot 1975: B about 0.5 to 0.9 for the archipelagos of the one planet
/// known, from memory).
const NUMBER_AREA: [f64; 2] = [0.5, 0.9];
/// A drawn rise's most length over its width (a long land, as a raised block is).
const LONG_MOST: f64 = 4.0;
/// The share of drawn rises that are rings, the rim of a sunken block (a caldera's), and their ring's radius in their
/// band's half-widths, the least and the most (a ring's band is as narrow as leaves it the area of a dome of its reach).
const RING_SHARE: f64 = 0.03;
const RING: [f64; 2] = [1.5, 2.5];
/// How much less a drawn rise rises on its lowered side at most, in units of the noise: a tilted block, a ring
/// breached on one side.
const TILT_MOST: f64 = 0.8;
/// The share of rings whose rim is breached, as a caldera at the level of the open low ground mostly is: tilted past
/// `MEAN_TOP`, so its low side lies under the level where the large shape rises as it does on the mean.
const BREACHED: f64 = 0.85;
/// How much more or less a drawn rise rises than the mean at most, in units of the noise.
const LIFT_MOST: f64 = 0.1;
/// The large shape's rise over an island's level, in units of the noise, in nine places of ten and on the mean (as the
/// noise has it): a rise's land reaches at most and mostly as far as its fall is that (`Rise::extent`).
const TOP: f64 = 0.75;
const MEAN_TOP: f64 = 0.5;
/// How much lower the land lies where two rises raise it alike, in units of the noise, and over how much more fall of
/// one rise than the other that falls off by e: the crust sags between two raised blocks.
const DIVIDE: f64 = 0.5;
const DIVIDE_WIDE: f64 = 0.2;
/// How far apart drawn rises' middles lie at least, in their mean extents summed, and the tries to place each.
const APART: f64 = 1.0;
const TRIES: u64 = 24;
/// How far the land of a drawn rise stays from the map's edge, in squares: so the open low ground runs along every edge
/// and joins the straits between the lands.
const EDGE_SQUARES: f64 = 2.0;
/// The step a drawn rise that does not fit inside its map is made shorter, then smaller, by (`place`).
const SHRINK: f64 = 0.95;

/// The large shape of an island: one rise in the middle of its map, or rises drawn from its seed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    #[default]
    One,
    Drawn,
}

/// A band along an axis where the land of a rise rises fast, falling off to the low ground where it rises slowly, as
/// the crust is raised in blocks (a range, its foothills, a low plain), not alike everywhere: on the warped noise, so
/// the axis bends as the ridges do. Its point on the axis, the unit vector across the axis, its half-width and its
/// half-length along the axis, in units of the noise.
struct Range {
    at: [f64; 2],
    across: [f64; 2],
    wide: f64,
    long: f64,
}

impl Range {
    /// How much a place at the warped point `warp` lies in the range: 1 on its axis, by a bell across it and along it.
    fn bell(&self, warp: [f64; 2]) -> f64 {
        let off = [warp[0] - self.at[0], warp[1] - self.at[1]];
        let d = off[0] * self.across[0] + off[1] * self.across[1];
        let along = off[1] * self.across[0] - off[0] * self.across[1];
        (-(d / self.wide).powi(2)).exp() * (-(along / self.long).powi(2)).exp()
    }
}

/// A rise of an island's crust: its middle, in shares of its map's width and height; the unit vector of its length;
/// its reach along and across, in shares of its map's side; its ring's radius, in reaches (0: a dome); how much less it
/// rises on its lowered side, in units of the noise, and the unit vector toward that side; how much more than the mean
/// it rises, in units of the noise; the power its land falls by; and its range.
struct Rise {
    at: [f64; 2],
    along: [f64; 2],
    reach: [f64; 2],
    ring: f64,
    tilt: f64,
    low_side: [f64; 2],
    lift: f64,
    power: f64,
    range: Range,
}

impl Rise {
    /// How far its land falls at the point `p` of the map, in shares of its width and height: by (the distance from
    /// its middle, or from its ring, in reaches) to its power, plus its tilt toward its lowered side, less its lift.
    fn fall(&self, p: [f64; 2]) -> f64 {
        let off = [p[0] - self.at[0], p[1] - self.at[1]];
        let q = [(off[0] * self.along[0] + off[1] * self.along[1]) / self.reach[0], (off[0] * -self.along[1] + off[1] * self.along[0]) / self.reach[1]];
        let r = (q[0] * q[0] + q[1] * q[1]).sqrt();
        let mut fall = (r - self.ring).abs().powf(self.power);
        if self.tilt > 0.0 && r > 0.0 {
            fall += self.tilt * (1.0 + (q[0] * self.low_side[0] + q[1] * self.low_side[1]) / r) / 2.0;
        }
        fall - self.lift
    }

    /// How far its land reaches from its middle, in its reaches, where the large shape rises `top` over the level:
    /// where its fall is that and its lift.
    fn extent(&self, top: f64) -> f64 {
        self.ring + reach_of(top, self.lift, self.power)
    }
}

/// How far the land of a dome lifted `lift` whose land falls by the power `power` reaches, in its reaches: where its
/// fall is `top` and `lift`.
fn reach_of(top: f64, lift: f64, power: f64) -> f64 {
    (top + lift).powf(power.recip())
}

/// Where an island's crust is raised: its rises (`Rise`).
pub struct Lift {
    rises: Vec<Rise>,
}

/// A point of a flat map, in shares of its width and height, in units of the noise, `per_square` units a square.
fn noise_of(map: &Square, p: [f64; 2], per_square: f64) -> [f64; 2] {
    [(p[0] * map.w as f64 - 0.5) * per_square, (p[1] * map.h as f64 - 0.5) * per_square]
}

/// The draw of key `key` of the drawn rises of `seed`.
fn shape_draw(seed: u64, key: u64) -> f64 {
    Rng::new(seed, SHAPE_KEY, key, Purpose::Terrain).f64()
}

/// The share of the map the unworn land of the drawn rises of `seed` holds, in `SHARE`.
pub(super) fn drawn_share(seed: u64) -> f64 {
    SHARE[0] + (SHARE[1] - SHARE[0]) * shape_draw(seed, 0)
}

impl Lift {
    /// One rise in the middle of the flat map `map` for `seed`, its range along an axis at any heading up to
    /// `RANGE_OFF` from the middle, its band `RANGE_WIDE` wide, all drawn from the seed, with no end.
    pub fn one(seed: u64, map: &Square) -> Lift {
        let (per_square, by) = (per_square(map), across(map) / ACROSS);
        let draw = |k: u64| Rng::new(seed, RANGE_KEY, k, Purpose::Terrain).f64();
        let (heading, off) = (draw(0) * TAU / 2.0, (2.0 * draw(1) - 1.0) * RANGE_OFF * by);
        let side_way = [heading.sin(), -heading.cos()];
        let middle = [map.w, map.h].map(|n| (n as f64 / 2.0 - 0.5) * per_square);
        let wide = (RANGE_WIDE[0] + (RANGE_WIDE[1] - RANGE_WIDE[0]) * draw(2)) * by;
        let range = Range { at: [middle[0] + off * side_way[0], middle[1] + off * side_way[1]], across: side_way, wide, long: f64::INFINITY };
        let rise = Rise { at: [0.5, 0.5], along: [1.0, 0.0], reach: [ONE_REACH; 2], ring: 0.0, tilt: 0.0, low_side: [1.0, 0.0], lift: 0.0, power: FALL_POWER, range };
        Lift { rises: vec![rise] }
    }

    /// Rises drawn from the seed `seed` on the flat map `map`: their count the most the map holds (`most_rises`) or one
    /// fewer; the area each raises by the number-area rule with its exponent drawn in `NUMBER_AREA`, its reach `scale`
    /// times the one rise's by the square root of its share of their areas; each a dome or a ring (`RING_SHARE`, most
    /// breached: `BREACHED`), up to `LONG_MOST` times as long as wide at any heading, falling by a power in `POWER`,
    /// tilted up to `TILT_MOST`, lifted up to `LIFT_MOST`, its range along its length. The largest are placed first,
    /// each where its land stays `EDGE_SQUARES` from the map's edge (`place`). The draws do not depend on `scale`.
    pub(super) fn drawn(seed: u64, map: &Square, scale: f64) -> Lift {
        let (per_square, across) = (per_square(map), across(map));
        let mut key = 0;
        let mut draw = || {
            key += 1;
            shape_draw(seed, key)
        };
        let between = |r: [f64; 2], u: f64| r[0] + (r[1] - r[0]) * u;
        let most = most_rises(map, drawn_share(seed));
        let count = most - (draw() * most.min(2) as f64) as usize;
        let b = between(NUMBER_AREA, draw());
        let areas: Vec<f64> = (1..=count).map(|k| (k as f64).powf(-1.0 / b)).collect();
        let total: f64 = areas.iter().sum();
        let mut rises: Vec<Rise> = Vec::with_capacity(count);
        for area in areas {
            let reach = ONE_REACH * scale * (area / total).sqrt();
            let (long, heading) = (between([1.0, LONG_MOST], draw()).sqrt(), draw() * PI);
            let along = [heading.cos(), heading.sin()];
            let (side, lift, power) = (draw() * TAU, LIFT_MOST * (2.0 * draw() - 1.0), between(POWER, draw()));
            let ring = if draw() < RING_SHARE { between(RING, draw()) } else { 0.0 };
            // A ring's rim is breached where its low side lies lower than the large shape rises over the level there:
            // most rings are tilted past its mean rise (`BREACHED`).
            let tilt = match ring > 0.0 {
                true if draw() < BREACHED => between([MEAN_TOP, TILT_MOST], draw()),
                true => MEAN_TOP * draw(),
                false => TILT_MOST * draw() * draw(),
            };
            // A ring's band is as narrow as leaves it a dome's area: 4 pi ring reach_of band² = pi reach_of².
            let band = if ring > 0.0 { (reach_of(MEAN_TOP, lift, power) / (4.0 * ring)).sqrt() } else { 1.0 };
            let none = Range { at: [0.0; 2], across: [0.0; 2], wide: 0.0, long: 0.0 };
            let low_side = [side.cos(), side.sin()];
            let mut rise = Rise { at: [0.5, 0.5], along, reach: [reach * long * band, reach / long * band], ring, tilt, low_side, lift, power, range: none };
            place(&mut rise, map, &rises, &mut draw);
            // The range along the rise's length, as far off its axis and as wide as the one rise's for its reach.
            let (off, wide) = ((2.0 * draw() - 1.0) * RANGE_OFF, between(RANGE_WIDE, draw()));
            let (side_way, at) = ([-along[1], along[0]], noise_of(map, rise.at, per_square));
            let by = (rise.reach[0] * rise.reach[1]).sqrt() / band / ONE_REACH * (across / ACROSS);
            let long = rise.reach[0] * rise.extent(TOP) * across;
            rise.range = Range { at: [at[0] + off * by * side_way[0], at[1] + off * by * side_way[1]], across: side_way, wide: wide * by, long };
            rises.push(rise);
        }
        Lift { rises }
    }

    /// How far an island's land falls at the point `p` of the map, in shares of its width and height: that of the
    /// rise that raises it most, and where a second raises it nearly as much (between two rises) lower by `DIVIDE`
    /// falling off by e every `DIVIDE_WIDE` that the second falls more: the crust sags between two raised blocks.
    pub(super) fn fall(&self, p: [f64; 2]) -> f64 {
        let (first, second) = self.rises.iter().map(|r| r.fall(p)).fold((f64::INFINITY, f64::INFINITY), |(a, b), f| if f < a { (f, a) } else { (a, b.min(f)) });
        if second.is_finite() { first + DIVIDE * (-(second - first) / DIVIDE_WIDE).exp() } else { first }
    }

    /// How fast the land rises at the warped point `warp` of the noise, against the mean (`RISES`): fastest on a
    /// range's axis, by its bell, down to the low ground's far from every range.
    pub(super) fn fast(&self, warp: [f64; 2]) -> f64 {
        let bell = self.rises.iter().map(|r| r.range.bell(warp)).fold(0.0, f64::max);
        RISES[0] + (RISES[1] - RISES[0]) * bell
    }
}

/// The most rises a map of `share` land holds: the count of lands of at least `LEAST_KM2` the number-area rule gives
/// when all its land would lie in one, with B = 1/2, √(land / `LEAST_KM2`): it grows with the map's side, not its area.
fn most_rises(map: &Square, share: f64) -> usize {
    let km2 = map.w as f64 * map.h as f64 * (map.side / 1000.0).powi(2);
    ((share * km2 / LEAST_KM2).sqrt().round() as usize).max(1)
}

/// Place `rise` on `map` beside the rises `placed`: its middle drawn by `draw` where its land (`Rise::extent`) stays
/// `EDGE_SQUARES` from the map's edge, the rise made shorter and then smaller where it would not fit (`SHRINK`); of
/// `TRIES` draws the first whose middle lies `APART` of their extents summed from every placed rise's, or the one that
/// lies farthest by that measure.
fn place(rise: &mut Rise, map: &Square, placed: &[Rise], draw: &mut impl FnMut() -> f64) {
    let edge = [EDGE_SQUARES / map.w as f64, EDGE_SQUARES / map.h as f64];
    let half = |r: &Rise| {
        let (a, b) = (r.reach[0] * r.extent(TOP), r.reach[1] * r.extent(TOP));
        [(a * r.along[0]).hypot(b * r.along[1]), (a * r.along[1]).hypot(b * r.along[0])]
    };
    let fits = |r: &Rise| (0..2).all(|d| half(r)[d] <= 0.5 - edge[d]);
    while !fits(rise) {
        // Shorter first, down to as long as wide; then smaller.
        let long = (rise.reach[0] * SHRINK).max(rise.reach[1]);
        rise.reach = if long < rise.reach[0] { [long, rise.reach[1]] } else { rise.reach.map(|x| x * SHRINK) };
    }
    let h = half(rise);
    let size = |r: &Rise| r.reach[0].max(r.reach[1]) * r.extent(MEAN_TOP);
    let mut best = ([0.5, 0.5], f64::NEG_INFINITY);
    for _ in 0..TRIES {
        let u = [draw(), draw()];
        let at: [f64; 2] = std::array::from_fn(|d| edge[d] + h[d] + (1.0 - 2.0 * (edge[d] + h[d])) * u[d]);
        let apart = placed.iter().map(|o| (at[0] - o.at[0]).hypot(at[1] - o.at[1]) / (size(rise) + size(o))).fold(f64::INFINITY, f64::min);
        if best.1 < APART && apart > best.1 {
            best = (at, apart);
        }
    }
    rise.at = best.0;
}
