//! The slopes of the ground worn at the world's start (laws.md 0 and 8, `space::wear`): they creep, and the ground
//! floats back up as the wear unloads it.

use super::grid::Grid;

/// The share of a load taken off the ground that the ground rises back by: the density of the ground over that of the
/// rock it floats on (Airy isostasy).
const FLOATS: f64 = 2700.0 / 3300.0;
/// The width over which the ground bends under a load, m.
const BENDS_M: f64 = 50e3;
/// The most a part of an explicit creep may diffuse, in squares of a sample's shorter side: a stable part on four
/// sides is under a quarter.
const STABLE: f64 = 0.2;

/// The ground rising back as a step's wear unloads it (Airy isostasy; Molnar and England 1990): each sample of land
/// rises by `FLOATS` of the ground the step took away (its height `was` and its rise less its height now: less what
/// settled), as the mean of that over a square `BENDS_M` wide about it (`Grid::spread`), as a plate that bends over
/// that width carries a load.
pub(super) fn rebound(g: &Grid, h: &mut [f64], was: &[f64], (up, open): (&[f64], &[bool]), dt: f64) {
    let taken: Vec<f64> = (0..g.n()).map(|k| if open[k] { 0.0 } else { was[k] + up[k] * dt - h[k] }).collect();
    let spread = g.spread(&taken, BENDS_M / 2.0);
    (0..g.n()).filter(|&k| !open[k]).for_each(|k| h[k] += FLOATS * spread[k]);
}

/// Creep of the slopes over a step: each sample of land changes by κ ∇²h across its four sides (`kappa` the
/// diffusivity times the step, m², Culling 1960), in explicit parts each under `STABLE` of the square of the shorter
/// side of a sample (Courant, Friedrichs and Lewy 1928); the open low ground stays, and a neighbour there stands at its
/// height or 0 m, the higher, and takes what creeps into it (`offshore`, m³).
pub(super) fn creep(g: &Grid, h: &mut [f64], open: &[bool], kappa: f64, offshore: &mut [f64]) {
    let side = g.dx.iter().fold(g.dy, |a, &b| a.min(b));
    let parts = (kappa / (STABLE * side * side)).ceil().max(1.0);
    for _ in 0..parts as usize {
        let was = h.to_vec();
        let at = |j: usize| if open[j] { was[j].max(0.0) } else { was[j] };
        for k in (0..g.n()).filter(|&k| !open[k]) {
            for (j, len) in g.sides(k) {
                let gain = kappa / parts * (at(j) - was[k]) / (len * len);
                h[k] += gain;
                if open[j] {
                    offshore[j] -= gain * g.area(k);
                }
            }
        }
    }
}
