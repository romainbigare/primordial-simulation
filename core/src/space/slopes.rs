//! The slopes of the ground worn at the world's start (laws.md 0 and 8, `space::wear`): they creep, and the ground
//! floats back up as the wear unloads it.

use super::grid::Grid;

/// The share of a load taken off the ground that the ground rises back by: the density of the ground over that of the
/// rock it floats on (Airy isostasy).
const FLOATS: f64 = 2700.0 / 3300.0;
/// The width over which the ground bends under a load, m.
const BENDS_M: f64 = 50e3;
/// The samples of the grid the rebound is spread on across that width: a plate's bending is smooth at any finer scale.
const BEND_SAMPLES: f64 = 16.0;
/// The most a part of an explicit creep may diffuse, in squares of a sample's shorter side: a stable part on four
/// sides is under a quarter.
const STABLE: f64 = 0.2;

/// The ground rising back as a step's wear unloads it (Airy isostasy; Molnar and England 1990): each sample of land
/// rises by `FLOATS` of the ground the step took away (its height `was` and its rise less its height now: less what
/// settled), as the mean of that over a square `BENDS_M` wide about it (`Grid::spread`), as a plate that bends over
/// that width carries a load (on a grid coarse enough that the width spans `BEND_SAMPLES` of it).
pub(super) fn rebound(g: &Grid, h: &mut [f64], was: &[f64], (up, open): (&[f64], &[bool]), dt: f64) {
    let taken: Vec<f64> = (0..g.n()).map(|k| if open[k] { 0.0 } else { was[k] + up[k] * dt - h[k] }).collect();
    // On a grid coarse enough that the plate's width spans `BEND_SAMPLES` of its samples.
    let mut by = 1;
    while (by * 2) as f64 * g.dy * BEND_SAMPLES <= BENDS_M && g.across.is_multiple_of(by * 2) && g.down.is_multiple_of(by * 2) {
        by *= 2;
    }
    let coarse = g.coarser(by);
    let spread = coarse.spread(&g.mean_over(&taken, by), BENDS_M / 2.0);
    // Blended between the coarse samples' centres (bilinear), nearest at the map's edge.
    let at = |r: i64, c: i64| spread[r.clamp(0, coarse.down as i64 - 1) as usize * coarse.across + c.clamp(0, coarse.across as i64 - 1) as usize];
    let blend = |k: usize| {
        let (y, x) = (((k / g.across) as f64 + 0.5) / by as f64 - 0.5, ((k % g.across) as f64 + 0.5) / by as f64 - 0.5);
        let (r, c, fy, fx) = (y.floor() as i64, x.floor() as i64, y - y.floor(), x - x.floor());
        (at(r, c) * (1.0 - fx) + at(r, c + 1) * fx) * (1.0 - fy) + (at(r + 1, c) * (1.0 - fx) + at(r + 1, c + 1) * fx) * fy
    };
    (0..g.n()).filter(|&k| !open[k]).for_each(|k| h[k] += FLOATS * blend(k));
}

/// Creep of the slopes over a step: each sample of land changes by κ ∇²h across its four sides (`kappa` the
/// diffusivity times the step, m², Culling 1960), in explicit parts each under `STABLE` of the square of the shorter
/// side of a sample (Courant, Friedrichs and Lewy 1928); the open low ground stays, and a neighbour there stands at its
/// height or the level of its liquid `stand`, m, the higher, and takes what creeps into it (`offshore`, m³).
pub(super) fn creep(g: &Grid, h: &mut [f64], (open, stand): (&[bool], f64), kappa: f64, offshore: &mut [f64]) {
    let side = g.dx.iter().fold(g.dy, |a, &b| a.min(b));
    let parts = (kappa / (STABLE * side * side)).ceil().max(1.0);
    for _ in 0..parts as usize {
        let was = h.to_vec();
        let at = |j: usize| if open[j] { was[j].max(stand) } else { was[j] };
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
