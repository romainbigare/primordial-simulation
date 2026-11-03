//! Pairs of reactions that undo each other, and the amounts at which they balance in a node.

use super::kernel::{scarce, Reactor};
use crate::chem::{Chemistry, Phase};

/// `Reactor::balance`: halvings of the interval that holds the balance, and the most mol it looks for.
const BISECT: usize = 200;
const MOST_MOL: f64 = 1e30;

/// Pairs of reactions that undo each other, as (first, back, m, substance): the back takes `m` times what the first
/// makes and gives `m` times what it took.
pub fn reversible(chem: &Chemistry) -> Vec<(usize, usize, u16, usize)> {
    let mut out = Vec::new();
    for (f, rf) in chem.reactions.iter().enumerate().filter(|(_, r)| !r.light && r.outputs.len() == 1) {
        let (u, made) = rf.outputs[0];
        for (b, rb) in chem.reactions.iter().enumerate().filter(|(b, r)| *b != f && !r.light && r.inputs.len() == 1 && r.inputs[0].0 == u) {
            let m = rb.inputs[0].1 / made;
            let scaled = |v: &[(u16, u16)]| {
                let mut v: Vec<_> = v.iter().map(|&(k, c)| (k, c * m)).collect();
                v.sort_unstable();
                v
            };
            let mut back = rb.outputs.clone();
            back.sort_unstable();
            if m >= 1 && m * made == rb.inputs[0].1 && back == scaled(&rf.inputs) {
                out.push((f, b, m, u as usize));
            }
        }
    }
    out
}

impl Reactor {
    /// The amounts at which pairs of reactions that undo each other balance in one node at `t`, from the chemistry's own
    /// rates, with the gas around (`gas`) held: of each pair the substance the first makes, if the node holds none, is set
    /// where the two go at equal rates (laws.md 2.6). Returns the substances set and their amounts.
    pub fn balance(&mut self, chem: &Chemistry, n: &mut [f64], gas: &[f64], t: f64) -> Vec<(usize, f64)> {
        let mut out = Vec::new();
        for (f, b, m, u) in reversible(chem) {
            if n[u] > 0.0 || gas[u] > 0.0 {
                continue;
            }
            let (made, took) = (chem.reactions[f].outputs[0].1 as f64, m as f64 * chem.reactions[f].outputs[0].1 as f64);
            let used: Vec<(usize, f64, f64)> = (chem.reactions[f].inputs.iter()).filter(|p| chem.phase(p.0 as usize, t) != Phase::Gas).map(|&(k, c)| (k as usize, n[k as usize], c as f64 / made)).collect();
            // The rate of change of `u`, mol/s, with `x` of it in the node: positive below the balance.
            let net = |rc: &mut Self, n: &mut [f64], x: f64| {
                n[u] = x;
                used.iter().for_each(|&(k, a, c)| n[k] = (a - c * x).max(0.0));
                let tot = rc.phases(chem, n, Some(gas), t);
                let eps = |rc: &Self, r: usize| {
                    let rx = &chem.reactions[r];
                    let lim = scarce(&rx.inputs, |k| rc.av[k]).0;
                    if lim > 0.0 { lim * rc.rate(rx, lim, &tot, t) } else { 0.0 }
                };
                made * eps(rc, f) - took * eps(rc, b)
            };
            if net(self, n, 0.0) <= 0.0 {
                continue;
            }
            let (mut lo, mut hi) = (0.0, 1.0);
            while net(self, n, hi) > 0.0 && hi < MOST_MOL {
                (lo, hi) = (hi, hi * 2.0);
            }
            for _ in 0..BISECT {
                let mid = 0.5 * (lo + hi);
                if net(self, n, mid) > 0.0 { lo = mid } else { hi = mid }
            }
            let x = 0.5 * (lo + hi);
            net(self, n, x);
            out.push((u, x));
        }
        out
    }
}
