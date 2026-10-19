//! Spread: one implicit step of anything that moves from high to low along links and fills nodes. A link passes
//! `g (x_a - x_b)` per second, a node holds `cap x`. Backward Euler, (C + dt L) x' = C x, is stable for any step
//! (Patankar 1980, ch. 4), solved by conjugate gradient with a Jacobi preconditioner (Hestenes and Stiefel 1952). The
//! books are exact even unconverged: each node moves only by the link fluxes.

use crate::ground::share;

/// The share of its start value below which the preconditioned residual's square ends the solve: about 1e-12 of the
/// residual's size, near the rounding of a step.
const CONVERGED: f64 = 1e-24;

/// The network of one spread: each link's two nodes and conductance g (at least 0), and each node's capacity (above 0).
pub struct Net<'a> {
    pub a: &'a [u32],
    pub b: &'a [u32],
    pub g: &'a [f64],
    pub cap: &'a [f64],
}

/// Work space of the solver, kept between calls: per node the change solved so far, the residual, the preconditioned
/// residual, the search direction, the system times it, the diagonal, and the node's gain over the step.
#[derive(Clone, Debug, Default)]
pub struct Spread {
    change: Vec<f64>,
    res: Vec<f64>,
    pre: Vec<f64>,
    dir: Vec<f64>,
    sys_dir: Vec<f64>,
    diag: Vec<f64>,
    gain: Vec<f64>,
}

impl Spread {
    /// One step of `dt` seconds on `net`: writes into `flux` what each link carries (+ from `a` to `b`), moves `x` by it,
    /// and returns the iterations made (at most `iters`). On a stiff network `iters` should be about the number of nodes:
    /// an unconverged stop keeps the books but steps the solve's error explicitly. Every capacity must be positive (a node
    /// of none could only balance under a converged solve).
    pub fn step(&mut self, net: &Net, x: &mut [f64], dt: f64, iters: usize, flux: &mut [f64]) -> usize {
        let n = x.len();
        debug_assert!(net.cap.len() == n, "one capacity per node");
        debug_assert!(net.b.len() == net.a.len() && net.g.len() == net.a.len() && flux.len() == net.a.len());
        debug_assert!(net.cap.iter().all(|&c| c > 0.0), "every node of a spread holds something");
        debug_assert!(net.g.iter().all(|&g| g >= 0.0) && dt >= 0.0, "conductances and the step are not negative");
        let Spread { change, res, pre, dir, sys_dir, diag, gain } = self;
        for v in [&mut *change, &mut *res, &mut *pre, &mut *dir, &mut *sys_dir, &mut *gain] {
            v.clear();
            v.resize(n, 0.0);
        }
        // The system for the change d = x' - x: (C + dt L) d = -dt L x. Its diagonal is C + dt (sum of g at the node).
        diag.clear();
        diag.extend_from_slice(net.cap);
        for (k, &g) in net.g.iter().enumerate() {
            let (i, j) = (net.a[k] as usize, net.b[k] as usize);
            let gdt = dt * g;
            diag[i] += gdt;
            diag[j] += gdt;
            let f = gdt * (x[i] - x[j]);
            res[i] -= f;
            res[j] += f;
        }
        precondition(res, diag, pre);
        dir.copy_from_slice(pre);
        let mut rz = dot(res, pre);
        let (stop, mut made) = (CONVERGED * rz, 0);
        while made < iters && rz > stop {
            made += 1;
            times(net, dt, dir, sys_dir);
            let alpha = rz / dot(dir, sys_dir);
            for (((d, r), &p), &q) in change.iter_mut().zip(res.iter_mut()).zip(dir.iter()).zip(sys_dir.iter()) {
                *d += alpha * p;
                *r -= alpha * q;
            }
            precondition(res, diag, pre);
            let rz_next = dot(res, pre);
            let beta = rz_next / rz;
            for (p, &z) in dir.iter_mut().zip(pre.iter()) {
                *p = z + beta * *p;
            }
            rz = rz_next;
        }
        // The fluxes from the solved potentials; each node's change only from its fluxes, so the books are exact.
        for (k, (fl, &g)) in flux.iter_mut().zip(net.g).enumerate() {
            let (i, j) = (net.a[k] as usize, net.b[k] as usize);
            *fl = dt * g * ((x[i] + change[i]) - (x[j] + change[j]));
            gain[i] -= *fl;
            gain[j] += *fl;
        }
        for ((xi, &gi), &c) in x.iter_mut().zip(gain.iter()).zip(net.cap) {
            *xi += gi / c;
        }
        made
    }
}

/// Where a node would give more than it holds (`held`), cuts each flux that leaves it by the share it holds of all that
/// leaves it; only those nodes' outgoing fluxes change (laws.md 0, "No overdraw").
pub fn within(a: &[u32], b: &[u32], flux: &mut [f64], held: &[f64]) {
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

/// `out = (C + dt L) v`: each node's capacity times `v`, plus what the links would pass out of it over `dt` at `v`.
fn times(net: &Net, dt: f64, v: &[f64], out: &mut [f64]) {
    for ((o, &c), &vi) in out.iter_mut().zip(net.cap).zip(v) {
        *o = c * vi;
    }
    for (k, &g) in net.g.iter().enumerate() {
        let (i, j) = (net.a[k] as usize, net.b[k] as usize);
        let f = dt * g * (v[i] - v[j]);
        out[i] += f;
        out[j] -= f;
    }
}

/// The Jacobi preconditioner: each residual over the diagonal of the system.
fn precondition(res: &[f64], diag: &[f64], out: &mut [f64]) {
    for ((o, &r), &d) in out.iter_mut().zip(res).zip(diag) {
        *o = r / d;
    }
}

/// The dot product, summed in index order.
fn dot(u: &[f64], v: &[f64]) -> f64 {
    u.iter().zip(v).map(|(p, q)| p * q).sum()
}
