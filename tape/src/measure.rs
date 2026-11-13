//! Numbers computed from the world's raw facts: means over areas. The core knows none of this.

/// The mean of `(value, area)` pairs weighted by area, 0 of none: the cells of a globe differ in area (a mean over cells
/// would count a polar cell as much as an equatorial one).
pub(crate) fn area_mean(items: impl Iterator<Item = (f64, f64)>) -> f64 {
    let (sum, area) = items.fold((0.0, 0.0), |(s, a), (v, w)| (s + v * w, a + w));
    sum / area.max(f64::MIN_POSITIVE)
}
