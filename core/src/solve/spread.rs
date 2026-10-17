//! Spread: one implicit step of anything that moves from high to low along links and fills nodes (heat, liquid in the
//! pores and what it carries, gases, the evening of pressure). A link passes `g (x_a - x_b)` per second and a node
//! holds `cap x`, where `x` is the potential (a temperature, a head, a concentration). The step is backward Euler: it
//! solves (C + dt L) x' = C x, with C the capacities and L the links' Laplacian, so it is stable for any step length
//! (Patankar 1980, ch. 4). The solver is conjugate gradient with a Jacobi preconditioner (Hestenes & Stiefel 1952),
//! at most a given number of iterations, always in the same order. The books are exact even when it has not
//! converged: the link fluxes come from the solved potentials, and each node's new value comes only from its fluxes.

/// The share of its start value below which the preconditioned residual's square (`rz`) ends the solve: about 1e-12
/// of the residual's size, near the rounding of a step. The same inputs stop at the same iteration.
const CONVERGED: f64 = 1e-24;

/// The network of one spread, as plain slices: the two nodes of each link, its conductance g (amount per second per
/// unit of potential, at least 0) and the capacity of each node (amount per unit of potential, more than 0).
pub struct Net<'a> {
    pub a: &'a [u32],
    pub b: &'a [u32],
    pub g: &'a [f64],
    pub cap: &'a [f64],
}

/// Work space of the solver, kept between calls so that a step allocates nothing once it has grown. Per node: the
/// change of the potential solved so far, the residual, the preconditioned residual, the search direction, the system
/// times the direction, the diagonal of the system, and the amount the node gains over the step.
#[derive(Default)]
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
    /// One step of `dt` seconds on `net`: writes into `flux` the amount each link carries over the step (+ from `a` to
    /// `b`) and moves the potentials `x` by those fluxes. It iterates until converged (`CONVERGED`) or `iters`, and
    /// returns the iterations made. A caller sets `iters` at least about the number of nodes on a stiff network: a
    /// stop before convergence keeps the books exact, but each node then moves by its fluxes from a partial solve,
    /// which is an explicit step on the solve's error and grows it on links with dt g / cap much above 1. Converged,
    /// the rounding of the result grows like the machine epsilon times dt g / cap.
    ///
    /// Every capacity must be positive: a node that holds nothing could gain nothing, so its fluxes would have to
    /// balance exactly, which only a converged solve gives. Leave such a node out of the network.
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
