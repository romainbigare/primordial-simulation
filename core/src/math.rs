//! Small functions of decay the laws share, and an exact sum.

/// 1 - exp(-x), exact for small `x`: the share a first-order decay at rate times time `x` takes, or a layer of depth `x`
/// absorbs.
#[inline]
pub(crate) fn one_minus_exp(x: f64) -> f64 {
    -(-x).exp_m1()
}

/// `n` times the surface of a cube of volume `v`: n 6 v^(2/3) (`n` first, for the order of the products).
#[inline]
pub(crate) fn cube_area(n: f64, v: f64) -> f64 {
    n * 6.0 * v.powf(2.0 / 3.0)
}

/// (1 - exp(-x)) / x: the mean over a time of a pulse that decays `x` time constants in it (1 below 1e-9).
#[inline]
pub(crate) fn mean_decay(x: f64) -> f64 {
    if x < 1e-9 { 1.0 } else { one_minus_exp(x) / x }
}

/// Veltkamp's constant, 2^27 + 1: it splits a number into two halves of 26 bits (`add_times`).
const SPLIT: f64 = 134_217_729.0;

/// Adds `x` to `parts`, a sum held exactly as numbers that do not overlap, the smallest first (Shewchuk 1997, Grow-
/// Expansion with zero elimination): terms that nearly cancel keep what is left of them.
#[inline]
fn grow(parts: &mut Vec<f64>, x: f64) {
    let (mut x, mut kept) = (x, 0);
    for j in 0..parts.len() {
        // `x + y` and its rounding error (Knuth 1969).
        let (y, s) = (parts[j], x + parts[j]);
        let bv = s - x;
        let err = (x - (s - bv)) + (y - bv);
        if err != 0.0 {
            parts[kept] = err;
            kept += 1;
        }
        x = s;
    }
    parts.truncate(kept);
    if x != 0.0 {
        parts.push(x);
    }
}

/// Adds `n x` to `parts` (`grow`) exactly, for a whole `n` of at most 26 bits: `x` split into two halves of 26 bits
/// (Veltkamp), each of whose products with `n` is exact. The parts added from the smallest give the sum, rounded.
#[inline]
pub(crate) fn add_times(parts: &mut Vec<f64>, n: f64, x: f64) {
    let t = SPLIT * x;
    let hi = t - (t - x);
    grow(parts, n * hi);
    grow(parts, n * (x - hi));
}
