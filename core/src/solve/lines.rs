//! The implicit step along a line of nodes, open or closed on itself (a ring, as a row round the globe).

/// Solves x' + (out − in) = x along a line of nodes, in place: edge p (between nodes p and p + 1)
/// moves `to_next[p]`·x'[p] forward and `to_prev[p]`·x'[p + 1] back, so x holds the amounts before
/// and then the amounts after the step. The sum of x is kept, and the step is stable for any
/// coefficients. It is a tridiagonal system, solved by the Thomas algorithm (Thomas 1949):
/// elimination forward, then substitution back. `work` has the length of `x`.
pub fn along_line(to_next: &[f64], to_prev: &[f64], x: &mut [f64], work: &mut [f64]) {
    let len = x.len();
    let di = |p: usize| 1.0 + if p > 0 { to_prev[p - 1] } else { 0.0 } + if p + 1 < len { to_next[p] } else { 0.0 };
    thomas(|p| -to_next[p - 1], di, |p| -to_prev[p], x, work);
}

/// `along_line` on a ring of at least 3 nodes: edge p joins node p to node p + 1, and the last edge the last node to
/// node 0, so `to_next` and `to_prev` have one entry per node. The two corners of the cyclic system are taken out as
/// a matrix of rank one and put back by the Sherman–Morrison formula (Press et al. 2007, 2.7.2): two open solves.
/// `work` and `corner` have the length of `x`.
pub fn along_ring(to_next: &[f64], to_prev: &[f64], x: &mut [f64], work: &mut [f64], corner: &mut [f64]) {
    let len = x.len();
    assert!(len >= 3 && to_next.len() == len && to_prev.len() == len, "a ring has at least 3 nodes and one edge per node");
    let di = |p: usize| 1.0 + to_prev[(p + len - 1) % len] + to_next[p];
    // The corners: node 0 gains from the last node, the last node from node 0.
    let (first, last, gamma) = (-to_next[len - 1], -to_prev[len - 1], -di(0));
    let open = |p: usize| di(p) - if p == 0 { gamma } else if p + 1 == len { first * last / gamma } else { 0.0 };
    thomas(|p| -to_next[p - 1], open, |p| -to_prev[p], x, work);
    corner.fill(0.0);
    (corner[0], corner[len - 1]) = (gamma, last);
    thomas(|p| -to_next[p - 1], open, |p| -to_prev[p], corner, work);
    let f = (x[0] + first / gamma * x[len - 1]) / (1.0 + corner[0] + first / gamma * corner[len - 1]);
    for (v, &z) in x.iter_mut().zip(corner.iter()) {
        *v -= f * z;
    }
}

/// The Thomas algorithm on the tridiagonal system with `lo(p)` left of the diagonal (p > 0), `di(p)` on it and `up(p)`
/// right of it (p + 1 < len), in place on `x`.
fn thomas(lo: impl Fn(usize) -> f64, di: impl Fn(usize) -> f64, up: impl Fn(usize) -> f64, x: &mut [f64], work: &mut [f64]) {
    let len = x.len();
    for p in 0..len {
        let lo = if p > 0 { lo(p) } else { 0.0 };
        let m = di(p) - if p > 0 { lo * work[p - 1] } else { 0.0 };
        work[p] = if p + 1 < len { up(p) / m } else { 0.0 };
        x[p] = (x[p] - if p > 0 { lo * x[p - 1] } else { 0.0 }) / m;
    }
    for p in (0..len - 1).rev() {
        x[p] -= work[p] * x[p + 1];
    }
}
