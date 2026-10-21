//! Numbers computed from the world's raw facts: how far learning has moved the brains, and means over areas. The core
//! knows none of this.

use pcore::World;

/// Mean absolute learned offset of the brains of the living bodies (their rows less the rows they were born with),
/// averaged over bodies by their members.
pub(crate) fn learned(w: &World) -> f64 {
    let (mut sum, mut members) = (0.0, 0.0);
    for b in w.life.alive().filter(|b| !b.rows.is_empty()) {
        let d: f64 = b.rows.iter().zip(w.life.born(b)).map(|(r, x)| (r - x).abs() as f64).sum();
        (sum, members) = (sum + d / b.rows.len() as f64 * b.n(), members + b.n());
    }
    sum / members.max(1.0)
}

/// The mean of `(value, area)` pairs weighted by area, 0 of none: the cells of a globe differ in area (a mean over cells
/// would count a polar cell as much as an equatorial one).
pub(crate) fn area_mean(items: impl Iterator<Item = (f64, f64)>) -> f64 {
    let (sum, area) = items.fold((0.0, 0.0), |(s, a), (v, w)| (s + v * w, a + w));
    sum / area.max(f64::MIN_POSITIVE)
}
