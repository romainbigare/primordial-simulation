//! Small functions of decay the laws share.

/// 1 - exp(-x), exact for small `x`: the share a first-order decay at rate times time `x` takes, or a layer of depth `x`
/// absorbs.
#[inline]
pub(crate) fn one_minus_exp(x: f64) -> f64 {
    -(-x).exp_m1()
}

/// (1 - exp(-x)) / x: the mean over a time of a pulse that decays `x` time constants in it (1 below 1e-9).
#[inline]
pub(crate) fn mean_decay(x: f64) -> f64 {
    if x < 1e-9 { 1.0 } else { one_minus_exp(x) / x }
}
