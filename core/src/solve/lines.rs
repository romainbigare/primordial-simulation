//! The implicit step along a line of nodes.

/// Solves x' + (out − in) = x along a line of nodes, in place: edge p (between nodes p and p + 1)
/// moves `to_next[p]`·x'[p] forward and `to_prev[p]`·x'[p + 1] back, so x holds the amounts before
/// and then the amounts after the step. The sum of x is kept, and the step is stable for any
/// coefficients. It is a tridiagonal system, solved by the Thomas algorithm (Thomas 1949):
/// elimination forward, then substitution back. `work` has the length of `x`.
pub fn along_line(to_next: &[f64], to_prev: &[f64], x: &mut [f64], work: &mut [f64]) {
    let len = x.len();
    for p in 0..len {
        let lo = if p > 0 { -to_next[p - 1] } else { 0.0 };
        let di = 1.0 + if p > 0 { to_prev[p - 1] } else { 0.0 } + if p + 1 < len { to_next[p] } else { 0.0 };
        let m = di - if p > 0 { lo * work[p - 1] } else { 0.0 };
        work[p] = if p + 1 < len { -to_prev[p] / m } else { 0.0 };
        x[p] = (x[p] - if p > 0 { lo * x[p - 1] } else { 0.0 }) / m;
    }
    for p in (0..len - 1).rev() {
        x[p] -= work[p] * x[p + 1];
    }
}
