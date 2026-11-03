//! Line: one implicit (backward Euler) step of linear flows along paths and rings of nodes, solved exactly. A link
//! from `a` to `b` passes `ca y_a - cb y_b + k` per second (`ca`, `cb` at least 0), `y` the values at the step's end,
//! so a step of any length is stable and keeps what is not negative so (Patankar 1980, 4.3); a path is solved by the
//! Thomas algorithm, a ring with the correction of Sherman and Morrison (Press et al. 2007, 2.7.2).

/// Paths and rings of nodes: the nodes of each in order, then the link between each node and the next (a ring's last
/// link closes it); each line's first node and first link (`at`, one more at the end).
pub struct Lines {
    pub nodes: Vec<u32>,
    pub links: Vec<u32>,
    at: Vec<[usize; 2]>,
}

/// The lines that the links `chosen` (of `a`, `b`) make on `n` nodes, each node on at most two of them: the paths, then
/// the rings, each from its lowest node.
pub fn lines(a: &[u32], b: &[u32], chosen: &[usize], n: usize) -> Lines {
    let mut on: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &l in chosen {
        [a[l], b[l]].iter().for_each(|&j| on[j as usize].push(l));
    }
    assert!(on.iter().all(|e| e.len() <= 2), "a node lies on at most two links of a line");
    let (mut seen, mut out) = (vec![false; n], Lines { nodes: Vec::new(), links: Vec::new(), at: vec![[0, 0]] });
    for ends in [1, 2] {
        for j in (0..n).filter(|&j| on[j].len() == ends) {
            if seen[j] {
                continue;
            }
            let (mut here, mut came) = (j, usize::MAX);
            seen[j] = true;
            out.nodes.push(j as u32);
            while let Some(&l) = on[here].iter().find(|&&l| l != came) {
                out.links.push(l as u32);
                (here, came) = ((a[l] + b[l]) as usize - here, l);
                if seen[here] {
                    break;
                }
                seen[here] = true;
                out.nodes.push(here as u32);
            }
            out.at.push([out.nodes.len(), out.links.len()]);
        }
    }
    out
}


/// One implicit step of `dt` seconds along `lines` under the flows of their links (`ca`, `cb`, indexed by link),
/// factored once (`factor`) for any values and constant flows (`solve`). Row i of all lines in a row reads
/// lo[i] y[i-1] + mid[i] y[i] + up[i] y[i+1] = r[i], a line's first row with no entry before it and its last none
/// after; per row the inverse pivot and the factor of the Thomas algorithm (`inv`, `c`); per ring the correction of its
/// corners (`Ring`, `z`).
pub struct Step {
    dt: f64,
    lo: Vec<f64>,
    inv: Vec<f64>,
    c: Vec<f64>,
    z: Vec<f64>,
    rings: Vec<Ring>,
}

/// A ring's place in `Lines`, its corner entries (first row's last column, `top`), the shift of its first pivot (`g`)
/// and the denominator of its correction (`den`).
struct Ring {
    line: usize,
    top: f64,
    g: f64,
    den: f64,
}

/// The step of `dt` seconds along `lines` under the flows `ca`, `cb` of their links: what each node holds changes by
/// what flows in less what flows out.
pub fn factor(lines: &Lines, a: &[u32], (ca, cb): (&[f64], &[f64]), dt: f64) -> Step {
    let all = lines.nodes.len();
    let (mut lo, mut up, mut mid) = (vec![0.0; all], vec![0.0; all], vec![1.0; all]);
    let mut corners = Vec::new();
    for (line, w) in lines.at.windows(2).enumerate() {
        let ([n0, l0], [n1, l1]) = (w[0], w[1]);
        let m = n1 - n0;
        for (i, &l) in lines.links[l0..l1].iter().enumerate() {
            let (l, j) = (l as usize, (i + 1) % m);
            // The flow from node i to node j: own y[i] - other y[j].
            let (own, other) = if a[l] == lines.nodes[n0 + i] { (ca[l], cb[l]) } else { (cb[l], ca[l]) };
            (mid[n0 + i], mid[n0 + j]) = (mid[n0 + i] + dt * own, mid[n0 + j] + dt * other);
            // The entry of row p at the column of q next to it: after it, before it, or a ring's corner.
            let mut off = |p: usize, q: usize, v: f64| *if q == p + 1 || (p == m - 1 && q == 0 && m > 2) { &mut up[n0 + p] } else { &mut lo[n0 + p] } -= v;
            off(i, j, dt * other);
            off(j, i, dt * own);
        }
        if l1 - l0 == m && m > 2 {
            // Sherman and Morrison: the path without the ring's corners, corrected for them (`solve`).
            let (top, end, g) = (lo[n0], up[n1 - 1], -mid[n0]);
            (lo[n0], up[n1 - 1], mid[n0], mid[n1 - 1]) = (0.0, 0.0, mid[n0] - g, mid[n1 - 1] - end * top / g);
            corners.push((line, top, end, g));
        }
    }
    let (mut inv, mut c) = (vec![0.0; all], vec![0.0; all]);
    for i in 0..all {
        inv[i] = 1.0 / (mid[i] - if i > 0 { lo[i] * c[i - 1] } else { 0.0 });
        c[i] = up[i] * inv[i];
    }
    let mut st = Step { dt, lo, inv, c, z: vec![0.0; all], rings: Vec::new() };
    for (line, top, end, g) in corners {
        let (n0, n1) = (lines.at[line][0], lines.at[line + 1][0]);
        (st.z[n0], st.z[n1 - 1]) = (g, end);
        substitute((&st.lo[n0..n1], &st.inv[n0..n1], &st.c[n0..n1]), &mut st.z[n0..n1]);
        st.rings.push(Ring { line, top, g, den: 1.0 + st.z[n0] + top * st.z[n1 - 1] / g });
    }
    st
}

/// The values `y` (indexed by node; those on no line are left) at the end of the step `st` along `lines` from `x`,
/// with the constant flows `k` of their links (indexed by link; none where `None`).
pub fn solve(lines: &Lines, st: &Step, a: &[u32], k: Option<&[f64]>, x: &[f64], y: &mut [f64]) {
    let mut r: Vec<f64> = lines.nodes.iter().map(|&j| x[j as usize]).collect();
    if let Some(k) = k {
        for w in lines.at.windows(2) {
            let ([n0, l0], [n1, l1]) = (w[0], w[1]);
            for (i, &l) in lines.links[l0..l1].iter().enumerate() {
                let (l, j) = (l as usize, (i + 1) % (n1 - n0));
                let toward = if a[l] == lines.nodes[n0 + i] { k[l] } else { -k[l] };
                (r[n0 + i], r[n0 + j]) = (r[n0 + i] - st.dt * toward, r[n0 + j] + st.dt * toward);
            }
        }
    }
    substitute((&st.lo, &st.inv, &st.c), &mut r);
    for ring in &st.rings {
        let (n0, n1) = (lines.at[ring.line][0], lines.at[ring.line + 1][0]);
        let f = (r[n0] + ring.top * r[n1 - 1] / ring.g) / ring.den;
        (n0..n1).for_each(|i| r[i] -= f * st.z[i]);
    }
    lines.nodes.iter().zip(r).for_each(|(&j, v)| y[j as usize] = v);
}

/// The solution of the rows factored by `factor` (`Step`), in place of their right side `r`.
fn substitute((lo, inv, c): (&[f64], &[f64], &[f64]), r: &mut [f64]) {
    let mut prev = 0.0;
    for i in 0..r.len() {
        prev = (r[i] - lo[i] * prev) * inv[i];
        r[i] = prev;
    }
    let mut next = 0.0;
    for i in (0..r.len()).rev() {
        next = r[i] - c[i] * next;
        r[i] = next;
    }
}
