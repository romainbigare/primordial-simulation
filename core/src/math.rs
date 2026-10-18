//! Small functions of decay the laws share, the tail of a normal, and an exact sum.

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

/// The Chebyshev fit of erfc of Press et al. (1992, Numerical Recipes, `erfcc`): erfc(x) = t exp(-x² + P(t)) for x at
/// least 0, t = 1 / (1 + x / 2), P with these coefficients from t⁰; fractional error under 1.2e-7 everywhere.
const ERFC: [f64; 10] = [-1.265_512_23, 1.000_023_68, 0.374_091_96, 0.096_784_18, -0.186_288_06, 0.278_868_07, -1.135_203_98, 1.488_515_87, -0.822_152_23, 0.170_872_77];

/// The share of a standard normal above `z`: erfc(z / sqrt 2) / 2 (`ERFC`).
pub(crate) fn upper_tail(z: f64) -> f64 {
    let x = (z / std::f64::consts::SQRT_2).abs();
    let t = 1.0 / (1.0 + 0.5 * x);
    let erfc = t * (-x * x + ERFC.iter().rev().fold(0.0, |p, &c| p * t + c)).exp();
    if z >= 0.0 { 0.5 * erfc } else { 1.0 - 0.5 * erfc }
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
