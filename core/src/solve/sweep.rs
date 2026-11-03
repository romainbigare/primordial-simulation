//! Sweep: one explicit step of anything that moves from high to low along links and fills nodes. A link passes
//! `g (x_a - x_b)` per second, a node holds `cap x`: over a step each link carries `dt g (x_a - x_b)` of the values at
//! the step's start (explicit Euler), at most `HALF_WAY` of what would level its two nodes (so two nodes linked alone
//! never cross), and
//! no node gives more than it holds (`within`). It is stable where `dt g / cap` is small, as for the liquid under the
//! ground and the heat through it at their steps (laws.md 1.1, 1.2).

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
