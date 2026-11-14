//! The overturning cell of the low latitudes (laws.md 1.6): where its air rises and where its two halves end, month by
//! month, from the planet's sun as its low liquid feels it (`Cell::of`). The upper air flowing poleward from the rising
//! branch keeps its angular momentum, and the cell ends where the warmth's fall can no longer balance the wind that
//! gives it (Held and Hou 1980); with the heating's peak off the equator the branch rises poleward of it and the cell
//! across the equator, the winter one, is the wide one (Lindzen and Hou 1988).

use super::radiation::{declination, insolation};
use crate::chem::Phase;
use crate::units::{SIGMA, SPIN};
use crate::{Config, World};
use std::f64::consts::{FRAC_PI_2, TAU};

/// Points of the year at which the sun by latitude is taken (the year's mean for the cell's size, its harmonics for the
/// month's heating).
const YEAR_POINTS: usize = 360;
/// Points of the year at which the month's heating is summed (`peak`): enough for its harmonics (`HARMONICS`) many
/// times over.
const LAG_POINTS: usize = 48;
/// The harmonics of the year the heating keeps (`peak`): the low liquid damps the k-th by about k ω τ (7 k with the
/// default numbers), so the higher ones move its peak by far less than a tenth of a degree.
const HARMONICS: usize = 4;
/// Steps of the golden-section search for the heating's peak and of the bisections of the cells (`cells`), and of the
/// scans that bracket their roots: their errors far below a thousandth of a degree, each scan's step under a degree.
const SEARCH: usize = 36;
const SCAN: usize = 100;

/// The overturning cell of the low latitudes in a month (`Cell::of`): the latitude its air rises at, and where its
/// northern and southern halves end, degrees (the cell spans the planet where the planet does not spin).
#[derive(Clone, Copy, Debug)]
pub(super) struct Cell {
    pub(super) rise: f64,
    pub(super) north: f64,
    pub(super) south: f64,
}

impl Cell {
    /// The cell of the world `w` now (`World::season`): its heating's peak (`peak`), and Lindzen and Hou's two halves
    /// about it (`cells`), in the small-angle form of Held and Hou, the thermal Rossby number from the planet's own
    /// year (`rossby`). With the peak on the equator, the year's cell: from the equator to ±sqrt(5 R / 3).
    pub(super) fn of(w: &World) -> Cell {
        let Some(r) = rossby(&w.cfg) else { return Cell { rise: 0.0, north: f64::INFINITY, south: f64::NEG_INFINITY } };
        let (rise, north, south) = cells(peak(w, r), r);
        Cell { rise: rise.to_degrees(), north: north.to_degrees(), south: south.to_degrees() }
    }
    /// The year's cell of the planet of `cfg`: its heating's peak on the equator (Held and Hou 1980).
    #[cfg(test)]
    pub(super) fn year(cfg: &Config) -> Cell {
        let (rise, north, south) = cells(0.0, rossby(cfg).expect("a spinning planet"));
        Cell { rise: rise.to_degrees(), north: north.to_degrees(), south: south.to_degrees() }
    }
    /// The share of the overturning down the warmth gradient at latitude `lat`, degrees: whole where the air rises,
    /// falling as 1 - (distance from there / the half's width)² to none at the half's edge, where the cell's flow does
    /// in Held and Hou's theory, and none beyond it.
    pub(super) fn share(&self, lat: f64) -> f64 {
        let edge = if lat >= self.rise { self.north } else { self.south };
        (1.0 - ((lat - self.rise) / (edge - self.rise)).powi(2)).max(0.0)
    }
}

/// The thermal Rossby number of the planet, g H Δ / (Ω a)² (Held and Hou 1980): H the air's depth (`air_top_m`), a the
/// planet's radius, Ω its spin (`units::SPIN` × `rotation`), and Δ the fractional fall of the warmth of radiative
/// balance from the equator to the pole, from the planet's own sun: 1 - (pole's / equator's yearly mean
/// sunlight)^¼, as a body in balance with its sunlight warms as its fourth root. None where the planet does not spin.
fn rossby(cfg: &Config) -> Option<f64> {
    let spin = SPIN * cfg.rotation;
    let fall = 1.0 - (yearly(cfg, FRAC_PI_2) / yearly(cfg, 0.0)).powf(0.25);
    (spin != 0.0).then(|| cfg.gravity * cfg.air_top_m * fall / (spin * cfg.radius_m).powi(2))
}

/// The yearly mean sunlight at latitude `lat`, radians, W/m².
fn yearly(cfg: &Config, lat: f64) -> f64 {
    (0..YEAR_POINTS).map(|k| insolation(cfg.sun, lat, declination(cfg.tilt, (k as f64 + 0.5) / YEAR_POINTS as f64))).sum::<f64>() / YEAR_POINTS as f64
}

/// The latitude where the month's heating peaks, radians, within the year's cell: the sun by latitude as the low liquid
/// under it feels it, a layer of `h_mix` of the key liquid (`Chemistry::life_roles`) that answers the sun in τ =
/// its heat capacity / (4 σ T³), T the warmth of radiative balance of the equator's yearly sunlight (a first-order
/// lag: each harmonic of the year damped by 1 / (1 + i k ω τ)). On the equator where the chemistry names no key liquid.
fn peak(w: &World, r: f64) -> f64 {
    let cfg = &w.cfg;
    let Some((_, _, liquid, _)) = w.chem.life_roles() else { return 0.0 };
    let heat = cfg.phys.h_mix * w.chem.subs[liquid].cp / w.chem.molar_volume(liquid, Phase::Liquid);
    let t = (yearly(cfg, 0.0) / SIGMA).powf(0.25);
    let tau = heat / (4.0 * SIGMA * t.powi(3)) / cfg.calendar().year as f64;
    let now = w.season();
    // The lag is linear in the sun: the heating felt now is the year's sunlight weighed by each point's share in it,
    // (1 + 2 Σ_k Re[e^(i k 2π (now - at)) / (1 + i k 2π τ)]) / LAG_POINTS, its harmonics up to `HARMONICS`.
    let points: Vec<(f64, f64)> = (0..LAG_POINTS)
        .map(|n| {
            let at = (n as f64 + 0.5) / LAG_POINTS as f64;
            let weight = 1.0 + 2.0 * (1..=HARMONICS).map(|k| {
                let (b, th) = (TAU * k as f64 * tau, TAU * k as f64 * (now - at));
                (th.cos() + b * th.sin()) / (1.0 + b * b)
            }).sum::<f64>();
            (declination(cfg.tilt, at), weight / LAG_POINTS as f64)
        })
        .collect();
    let felt = |lat: f64| points.iter().map(|&(dec, weight)| insolation(cfg.sun, lat, dec) * weight).sum::<f64>();
    let edge = (5.0 * r / 3.0).sqrt();
    // Golden-section search for its peak over the year's cell.
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut a, mut b) = (-edge, edge);
    let (mut c, mut d) = (b - g * (b - a), a + g * (b - a));
    let (mut fc, mut fd) = (felt(c), felt(d));
    for _ in 0..SEARCH {
        if fc > fd {
            (b, d, fd) = (d, c, fc);
            c = b - g * (b - a);
            fc = felt(c);
        } else {
            (a, c, fc) = (c, d, fd);
            d = a + g * (b - a);
            fd = felt(d);
        }
    }
    (a + b) / 2.0
}

/// What a half of the cell from `y1` to `y`, radians, holds of warmth over radiative balance (Held and Hou's equal
/// area), with the cell's warmth at `y1` (`k`), both nondimensional: radiative balance -(y - y0)², the cell's own
/// k - (y² - y1²)² / (2 r), k such that the two meet at `y`.
fn area(y: f64, (y1, y0, r): (f64, f64, f64)) -> (f64, f64) {
    let k = -(y - y0).powi(2) + (y * y - y1 * y1).powi(2) / (2.0 * r);
    let p = |x: f64| x.powi(5) / 5.0 - 2.0 * y1 * y1 * x.powi(3) / 3.0 + y1.powi(4) * x;
    let q = |x: f64| (x - y0).powi(3) / 3.0;
    (k * (y - y1) - (p(y) - p(y1)) / (2.0 * r) + q(y) - q(y1), k)
}

/// The edge of the half of the cell on the side `north` of `y1` and the cell's warmth at `y1` it needs (`area`): the
/// farthest latitude where the half holds no more or less warmth than radiative balance and meets it there. None where
/// it has none.
fn half(y1: f64, (y0, r): (f64, f64), north: bool) -> Option<(f64, f64)> {
    let s = if north { 1.0 } else { -1.0 };
    let f = |d: f64| area(y1 + s * d, (y1, y0, r)).0;
    let step = FRAC_PI_2 / SCAN as f64;
    let mut far = f(FRAC_PI_2);
    for j in (1..SCAN).rev() {
        let d = step * j as f64;
        let v = f(d);
        if (v > 0.0) != (far > 0.0) {
            let (mut lo, mut hi) = (d, d + step);
            for _ in 0..SEARCH {
                let m = (lo + hi) / 2.0;
                if (f(m) > 0.0) == (v > 0.0) { lo = m } else { hi = m }
            }
            let y = y1 + s * (lo + hi) / 2.0;
            return Some((y, area(y, (y1, y0, r)).1));
        }
        far = v;
    }
    None
}

/// The root of `g` between `a` and `b`: the first change of sign of a scan, bisected. None where it has none.
fn root(g: impl Fn(f64) -> Option<f64>, (a, b): (f64, f64)) -> Option<f64> {
    let at = |j: usize| a + (b - a) * j as f64 / SCAN as f64;
    let (mut x0, mut v0) = (at(0), g(at(0)));
    for j in 1..=SCAN {
        let (x1, v1) = (at(j), g(at(j)));
        if let (Some(p), Some(q)) = (v0, v1) {
            if (p > 0.0) != (q > 0.0) {
                let (mut lo, mut hi) = (x0, x1);
                for _ in 0..SEARCH {
                    let m = (lo + hi) / 2.0;
                    match g(m) {
                        Some(v) if (v > 0.0) == (p > 0.0) => lo = m,
                        Some(_) => hi = m,
                        None => break,
                    }
                }
                return Some((lo + hi) / 2.0);
            }
        }
        (x0, v0) = (x1, v1);
    }
    None
}

/// Lindzen and Hou's cell about a heating peaked at `y0`, radians, for the thermal Rossby number `r`: the latitude its
/// air rises at and its northern and southern edges, radians. Both halves meet at the rising latitude with one warmth
/// and each holds the warmth of radiative balance (`half`); where the summer half has none, the winter half alone, its
/// edge on the summer side the rising latitude, where its warmth meets radiative balance. With the peak on the equator,
/// Held and Hou's: ±sqrt(5 r / 3).
fn cells(y0: f64, r: f64) -> (f64, f64, f64) {
    let edge = (5.0 * r / 3.0).sqrt();
    if y0 == 0.0 {
        return (0.0, edge, -edge);
    }
    let span = (y0 - edge, y0 + edge);
    let meet = |y1: f64| Some(half(y1, (y0, r), true)?.1 - half(y1, (y0, r), false)?.1);
    if let Some(y1) = root(meet, span) {
        if let (Some(n), Some(s)) = (half(y1, (y0, r), true), half(y1, (y0, r), false)) {
            return (y1, n.0, s.0);
        }
    }
    // The winter half alone, on the side away from the peak.
    let north = y0 < 0.0;
    let alone = |y1: f64| Some(half(y1, (y0, r), north)?.1 + (y1 - y0).powi(2));
    match root(alone, span).and_then(|y1| Some((y1, half(y1, (y0, r), north)?.0))) {
        Some((y1, other)) if north => (y1, other, y1),
        Some((y1, other)) => (y1, y1, other),
        None => (0.0, edge, -edge),
    }
}

#[cfg(test)]
mod tests {
    use super::{cells, rossby, Cell};
    use crate::Config;

    /// On a planet that spins as the one known, the year's cell ends in the subtropics, whole at the equator and none
    /// down the gradient at 45°; with no spin it spans the planet.
    #[test]
    fn the_years_cell_ends_where_the_spin_says() {
        let cfg = Config::default();
        let year = Cell::year(&cfg);
        assert!(year.north > 15.0 && year.north < 35.0 && year.south == -year.north, "the cell ends at {}°", year.north);
        assert_eq!((year.share(0.0), year.share(45.0), year.share(-45.0)), (1.0, 0.0, 0.0));
        assert!(rossby(&Config { rotation: 0.0, ..Config::default() }).is_none());
    }

    /// With the heating's peak off the equator the air rises poleward of it and the winter half, across the equator, is
    /// the wide one (Lindzen and Hou 1988): a peak at 6° of a cell that ends at 22° in the year rises near 20° and
    /// reaches past 30° on the winter side.
    #[test]
    fn the_cell_rises_poleward_of_the_heating_and_the_winter_half_is_wide() {
        let r = 3.0 * 22.2f64.to_radians().powi(2) / 5.0;
        let (rise, north, south) = cells(6f64.to_radians(), r);
        let [rise, north, south] = [rise, north, south].map(f64::to_degrees);
        assert!(rise > 15.0 && rise < 25.0 && north > rise && south < -30.0 && north - rise < rise - south, "rises at {rise}°, from {south}° to {north}°");
        let (a, n, s) = cells(-6f64.to_radians(), r);
        assert!((a.to_degrees() + rise).abs() < 1e-6 && (n.to_degrees() + south).abs() < 1e-6 && (s.to_degrees() + north).abs() < 1e-6, "not mirrored");
    }
}
