//! Numbers computed from the world's raw facts: how far learning has moved the brains, and means over areas. The core
//! knows none of this.

use pcore::par;
use pcore::body::Body;
use pcore::World;

/// Mean absolute learned offset of the brains of the living bodies (their rows less the rows they were born with),
/// averaged over bodies by their members.
pub(crate) fn learned(w: &World) -> f64 {
    let bodies: Vec<&Body> = w.life.alive().filter(|b| !w.life.rows_of(b).is_empty()).collect();
    let offsets = par::map(&bodies, |b| w.life.rows_of(b).iter().zip(w.life.born(b)).map(|(r, x)| (r - x).abs() as f64).sum::<f64>());
    let (mut sum, mut members) = (0.0f64, 0.0f64);
    for (b, d) in bodies.iter().zip(offsets) {
        (sum, members) = (sum + d / w.life.rows_of(b).len() as f64 * b.n(), members + b.n());
    }
    sum / members.max(1.0)
}

/// The mean of `(value, area)` pairs weighted by area, 0 of none: the cells of a globe differ in area (a mean over cells
/// would count a polar cell as much as an equatorial one).
pub(crate) fn area_mean(items: impl Iterator<Item = (f64, f64)>) -> f64 {
    let (sum, area) = items.fold((0.0, 0.0), |(s, a), (v, w)| (s + v * w, a + w));
    sum / area.max(f64::MIN_POSITIVE)
}
