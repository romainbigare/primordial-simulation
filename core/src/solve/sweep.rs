//! Sweep: one explicit step of anything that moves from high to low along links and fills nodes. A link passes
//! `g (x_a - x_b)` per second, a node holds `cap x`: over a step each link carries `dt g (x_a - x_b)` of the values at
//! the step's start (explicit Euler), at most `HALF_WAY` of what would level its two nodes (so two nodes linked alone
//! never cross), and no node gives more than it holds (`within`). It is safe where `dt Σ g / cap` of each node is within
//! `LIMIT`; `steps` says how many steps a node needs to stay so (laws.md 1, the step by the size).

use crate::ground::share;

/// A link carries at most this share of what would level its two nodes in a step.
pub const HALF_WAY: f64 = 0.5;

/// The network of one sweep: each link's two nodes and conductance g (at least 0), and each node's capacity (above 0).
pub struct Net<'a> {
    pub a: &'a [u32],
    pub b: &'a [u32],
    pub g: &'a [f64],
    pub cap: &'a [f64],
}

/// What each link of `net` carries over `dt` seconds from the values `x` (+ from `a` to `b`), no node giving more than
/// it holds, `have`.
pub fn sweep(net: &Net, x: &[f64], dt: f64, have: &[f64]) -> Vec<f64> {
    let mut flux: Vec<f64> = (0..net.a.len())
        .map(|l| {
            let (i, j) = (net.a[l] as usize, net.b[l] as usize);
            let (d, level) = (x[i] - x[j], net.cap[i] * net.cap[j] / (net.cap[i] + net.cap[j]));
            (dt * net.g[l] * d).clamp(-HALF_WAY * level * d.abs(), HALF_WAY * level * d.abs())
        })
        .collect();
    within(net.a, net.b, &mut flux, have);
    flux
}

/// The most `dt Σ g / cap` an explicit step of a node may take: half of where its own weight in its next value,
/// 1 - dt Σ g / cap, reaches 0 (Patankar 1980, 4.3), so no pattern of the values changes sign in a step and a hump
/// spreads without turning into a pit.
pub const LIMIT: f64 = 0.5;

/// The steps each node of `net` needs to cover `dt` seconds within `LIMIT`: dt Σ g / cap over its links, rounded up (at
/// least 1).
pub fn steps(net: &Net, dt: f64) -> Vec<u32> {
    let mut g = vec![0.0; net.cap.len()];
    for ((&i, &j), &gl) in net.a.iter().zip(net.b).zip(net.g) {
        g[i as usize] += gl;
        g[j as usize] += gl;
    }
    g.iter().zip(net.cap).map(|(&g, &cap)| ((dt * g / cap / LIMIT).ceil() as u32).max(1)).collect()
}

/// Where a node would give more than it holds (`held`), cuts each flux that leaves it by the share it holds of all that
/// leaves it; only those nodes' outgoing fluxes change (laws.md 0, "No overdraw").
fn within(a: &[u32], b: &[u32], flux: &mut [f64], held: &[f64]) {
    let mut out = vec![0.0; held.len()];
    for ((&i, &j), &f) in a.iter().zip(b).zip(flux.iter()) {
        if f > 0.0 {
            out[i as usize] += f;
        } else {
            out[j as usize] -= f;
        }
    }
    for ((&i, &j), f) in a.iter().zip(b).zip(flux.iter_mut()) {
        let from = if *f > 0.0 { i } else { j } as usize;
        *f *= share(held[from].max(0.0), out[from]);
    }
}

/// What a sweep of a step moved (`by_size`): through each link, of the quantity swept (+ from `a` to `b`); and the links
/// that took sub-steps (indices into the sweep's, in order), with what crossed each of the rows that go with the
/// quantity, `rows[r * fast.len() + u]`.
pub struct Moved {
    pub flux: Vec<f64>,
    pub fast: Vec<usize>,
    pub rows: Vec<f64>,
}

/// The potential of every node of a sweep and the conductance of some links of it (by index) when the nodes hold what it
/// is given (`by_size`).
pub type Again<'a> = dyn Fn(&[f64], &[usize]) -> (Vec<f64>, Vec<f64>) + 'a;

/// A sweep (`sweep`) of a step of `dt` seconds, sub-stepped where a node is past `LIMIT` (laws.md 1.8): a node that needs
/// `steps` over one, and its links, take the most any node needs, each from what the sub-steps so far leave the nodes
/// that need them, the other nodes as the step started but for their potential, which follows what the sub-stepped
/// links brought them; each link's sum is booked once by both its nodes (a flux register, Berger and Colella 1989). The other links then step once from the step's start, no node giving more than
/// its sub-stepped links left it. `net` at the step's start, with the potential `x`; each node holds `held` of the
/// quantity swept and the rows `rows[r * n + i]` that go with it, of which a link carries the share its sender gives;
/// `can(i, v)` is what node `i` may give holding `v`; `again(level, links)` the potential of every node and the
/// conductance of those links when the nodes hold `level`. With no node past the limit, one sweep, and no rows.
pub fn by_size(net: &Net, x: &[f64], dt: f64, (held, rows): (&[f64], &[f64]), can: &dyn Fn(usize, f64) -> f64, again: &Again) -> Moved {
    let n = net.cap.len();
    let rate = steps(net, dt);
    let most = rate.iter().copied().max().unwrap_or(1);
    let start: Vec<f64> = (0..n).map(|i| can(i, held[i])).collect();
    if most <= 1 {
        return Moved { flux: sweep(net, x, dt, &start), fast: Vec::new(), rows: Vec::new() };
    }
    let fast: Vec<bool> = rate.iter().map(|&r| r > 1).collect();
    let (quick, slow): (Vec<usize>, Vec<usize>) = (0..net.a.len()).partition(|&l| fast[net.a[l] as usize] || fast[net.b[l] as usize]);
    let pick = |v: &[u32], ls: &[usize]| ls.iter().map(|&l| v[l]).collect::<Vec<u32>>();
    let (qa, qb, nr, links) = (pick(net.a, &quick), pick(net.b, &quick), rows.len() / n.max(1), quick.len());
    // What the fast nodes hold as the sub-steps go, and what the others gave; and what each node holds for its potential,
    // the others too with what they got and gave through the links sub-stepped.
    let (mut now, mut row, mut gave, mut level) = (held.to_vec(), rows.to_vec(), vec![0.0; n], held.to_vec());
    let (mut swept, mut moved, mut dv, mut dr) = (vec![0.0; links], vec![0.0; nr * links], vec![0.0; n], vec![0.0; nr * n]);
    for _ in 0..most {
        let (xs, g) = again(&level, &quick);
        let left: Vec<f64> = (0..n).map(|i| if fast[i] { can(i, now[i]) } else { start[i] - gave[i] }).collect();
        let f = sweep(&Net { a: &qa, b: &qb, g: &g, cap: net.cap }, &xs, dt / most as f64, &left);
        for (u, &fu) in f.iter().enumerate().filter(|p| *p.1 != 0.0) {
            let (from, to) = if fu > 0.0 { (qa[u] as usize, qb[u] as usize) } else { (qb[u] as usize, qa[u] as usize) };
            let share = fu.abs() / now[from];
            for r in 0..nr {
                let m = row[r * n + from] * share;
                (moved[r * links + u], dr[r * n + from], dr[r * n + to]) = (moved[r * links + u] + fu.signum() * m, dr[r * n + from] - m, dr[r * n + to] + m);
            }
            (swept[u], dv[from], dv[to]) = (swept[u] + fu, dv[from] - fu.abs(), dv[to] + fu.abs());
            if !fast[from] {
                gave[from] += fu.abs();
            }
        }
        // The fast nodes take what moved; what a node beside them gets lands with the step's end, but its potential
        // follows it.
        for i in 0..n {
            level[i] += dv[i];
            if fast[i] {
                now[i] += dv[i];
                (0..nr).for_each(|r| row[r * n + i] += dr[r * n + i]);
            }
        }
        dv.fill(0.0);
        dr.fill(0.0);
    }
    let left: Vec<f64> = (0..n).map(|i| start[i] - gave[i]).collect();
    let once = sweep(&Net { a: &pick(net.a, &slow), b: &pick(net.b, &slow), g: &slow.iter().map(|&l| net.g[l]).collect::<Vec<f64>>(), cap: net.cap }, x, dt, &left);
    let mut flux = vec![0.0; net.a.len()];
    slow.iter().zip(once).for_each(|(&l, f)| flux[l] = f);
    quick.iter().zip(swept).for_each(|(&l, f)| flux[l] = f);
    Moved { flux, fast: quick, rows: moved }
}
