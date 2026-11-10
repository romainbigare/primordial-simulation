//! The ground worn at the world's start (laws.md 0 and 8): the terrain's noise says how high the ground would stand;
//! the samples' heights come from the land rising there from 0 m for `WEAR_YEARS` while flowing liquid cuts it, its
//! slopes creep and it floats back up as it is unloaded. The law is the stream power law (Howard 1994; Whipple and
//! Tucker 1999): the ground lowers by K √A S, A the area that drains through a sample (spread down every lower way,
//! Quinn et al. 1991) and S its slope to the sample its liquid runs to (m = 1/2, n = 1), solved each step exactly from
//! the open low ground up the ways (Braun and Willett 2013), so a step of any length is stable. Under a closed basin's
//! level (`space::drain`) the liquid does not cut, and what the ways above it cut settles there (`space::ponds`). The
//! time runs on grids coarser than the samples (`STAGES`), each stage carried smooth to the next and the last to the
//! samples (`Grid::finer`). All in a fixed serial order in 64-bit numbers, so a seed gives one ground on every machine.

use super::drain::{flood, open_low, Flood};
use super::grid::Grid;
use super::network::NONE;
use super::ponds::settle;
use crate::math::spread_down;
use crate::rng::{Purpose, Rng};

/// The years the land rises and is worn.
const WEAR_YEARS: f64 = 2.0e6;
/// The stream power law's K for m = 1/2 and n = 1, per year: ground under liquid that drains 1 m² lowers by K a year
/// on a unit slope.
const CUT: f64 = 2.0e-7;
/// The diffusivity of the slopes' creep, m² a year.
const CREEP: f64 = 3.0e-3;
/// The grids the time runs on, coarsest first, each half as coarse as the one before: the samples a side to one of
/// the grid's (on a map no larger than `MOST_SAMPLES` allows), the share of the time on it, and its steps.
const STAGES: [(usize, f64, usize); 2] = [(4, 0.8, 40), (2, 0.2, 10)];
/// The most samples the first stage's grid may hold: a larger map runs every stage on grids twice as coarse, until it
/// fits (the cost of a large map's start).
const MOST_SAMPLES: usize = 1 << 16;
/// The share of a load taken off the ground that the ground rises back by: the density of the ground over that of the
/// rock it floats on (Airy isostasy).
const FLOATS: f64 = 2700.0 / 3300.0;
/// The width over which the ground bends under a load, m.
const BENDS_M: f64 = 50e3;
/// The spread of the land's heights where it starts to rise, m: the roughness the first ways find.
const ROUGH_M: f64 = 1.0;
/// The most a part of an explicit creep may diffuse, in squares of a sample's shorter side: a stable part on four
/// sides is under a quarter.
const STABLE: f64 = 0.2;

/// The samples a side to one of the last stage's grid on grid `g` (`STAGES`, `MOST_SAMPLES`): the terrain's noise is
/// read on that grid.
pub(super) fn finest(g: &Grid) -> usize {
    let first = STAGES[0].0;
    let mut scale = 1;
    while g.n() / (first * scale).pow(2) > MOST_SAMPLES && g.across.is_multiple_of(2 * first * scale) && g.down.is_multiple_of(2 * first * scale) {
        scale *= 2;
    }
    STAGES[STAGES.len() - 1].0 * scale
}

/// The worn heights of the samples of grid `g`, m, and the open low ground among them (`drain::open_low`), where the
/// terrain's noise stands at `rise`, m, on the grid `by` times coarser (`finest`). Each stage's grid takes the
/// noise's mean over each of its samples; the first starts from 0 m on land (the noise's heights under 0 m) and a
/// roughness of `ROUGH_M` drawn from the seed, each later one from the last carried to it; the samples take the last
/// carried to them, and the noise carried to them too. On every grid the open low ground stands at the noise's heights
/// and land no lower than 0 m.
pub(super) fn wear(seed: u64, g: &Grid, (by, rise): (usize, &[f64])) -> (Vec<f64>, Vec<bool>) {
    let rough = |k: usize| ROUGH_M * (Rng::new(seed, 0, k as u64, Purpose::Terrain).f64() - 0.5);
    let (fine, last_by) = (g.coarser(by), STAGES[STAGES.len() - 1].0);
    let mut last: Option<(usize, Vec<f64>)> = None;
    for &(stage_by, share, steps) in &STAGES {
        let to = stage_by / last_by * by;
        let (stage, rise_s) = (g.coarser(to), fine.mean_over(rise, to / by));
        let open_s = open_low(&stage, &rise_s);
        let mut h = match &last {
            None => (0..stage.n()).map(|k| if open_s[k] { rise_s[k] } else { rise_s[k].min(0.0) + rough(k) }).collect(),
            Some((from, z)) => held(&carried(g, (*from, z), to), &rise_s, &open_s),
        };
        run(&stage, &mut h, (&rate(&rise_s, &open_s), &open_s), (WEAR_YEARS * share, steps));
        last = Some((to, h));
    }
    let (from, z) = last.expect("a stage");
    let rise_g = carried(g, (by, rise), 1);
    let open = open_low(g, &rise_g);
    (held(&carried(g, (from, &z), 1), &rise_g, &open), open)
}

/// Heights `z` on the grid `from` times coarser than `g` carried smooth to the grid `to` times coarser, a halving at a
/// time (`Grid::finer`).
fn carried(g: &Grid, (mut from, z): (usize, &[f64]), to: usize) -> Vec<f64> {
    let mut z = z.to_vec();
    while from > to {
        z = g.coarser(from).finer(&z, &g.coarser(from / 2).dx);
        from /= 2;
    }
    z
}

/// Heights `h` with the open low ground `open` at the noise's heights `rise` and land (where the noise lies at 0 m or
/// higher) no lower than 0 m.
fn held(h: &[f64], rise: &[f64], open: &[bool]) -> Vec<f64> {
    (0..h.len()).map(|k| if open[k] { rise[k] } else if rise[k] >= 0.0 { h[k].max(0.0) } else { h[k] }).collect()
}

/// The rate each sample rises at, m a year: to its noise's height in `WEAR_YEARS` on land, none on ground under 0 m or
/// on the open low ground.
fn rate(rise: &[f64], open: &[bool]) -> Vec<f64> {
    rise.iter().zip(open).map(|(&z, &o)| if o { 0.0 } else { z.max(0.0) / WEAR_YEARS }).collect()
}

/// `steps` equal steps over `years` of heights `h` on grid `g`, rising at `up` (m a year), the samples `open` held:
/// each the flood from the shore (`drain::flood`), the cut, the load settling, the creep and the rebound.
fn run(g: &Grid, h: &mut [f64], (up, open): (&[f64], &[bool]), (years, steps): (f64, usize)) {
    let dt = years / steps as f64;
    for _ in 0..steps {
        let f = flood(g, h, open, true);
        let was = h.to_vec();
        cut(g, h, &f, (up, open), dt);
        settle(g, h, &f, &was, (up, open), dt);
        creep(g, h, open, CREEP * dt);
        rebound(g, h, &was, (up, open), dt);
    }
}

/// One step of `dt` years of the stream power law (n = 1, solved exactly, Braun and Willett 2013): each sample, in the
/// flood's order (the one its liquid runs to first), rises by its rate and then falls toward the height of that one
/// (0 m on the open low ground, the level of the liquid there) as h = (h₀ + f h_down) / (1 + f), f = K dt √A / d. A
/// sample under its basin's level, or whose liquid runs to higher ground, only rises.
fn cut(g: &Grid, h: &mut [f64], f: &Flood, (up, open): (&[f64], &[bool]), dt: f64) {
    let own = (0..g.n()).map(|k| g.area(k)).collect();
    let area = spread_down(&f.surface, f.order.iter().rev().map(|&k| k as usize), |k| f.earlier(g, k), own);
    for &k in &f.order {
        let k = k as usize;
        if open[k] {
            continue;
        }
        let (j, mut z) = (f.to[k], h[k] + up[k] * dt);
        if j != NONE && !f.under(h, k) {
            let j = j as usize;
            let down = if open[j] { h[j].max(0.0) } else { h[j] };
            if down < z {
                let c = CUT * dt * area[k].sqrt() / f.apart[k];
                z = (z + c * down) / (1.0 + c);
            }
        }
        h[k] = z;
    }
}

/// The ground rising back as a step's wear unloads it (Airy isostasy; Molnar and England 1990): each sample of land
/// rises by `FLOATS` of the ground the step took away (its height `was` and its rise less its height now: less what
/// settled), as the mean of that over a square `BENDS_M` wide about it (`Grid::spread`), as a plate that bends over
/// that width carries a load.
fn rebound(g: &Grid, h: &mut [f64], was: &[f64], (up, open): (&[f64], &[bool]), dt: f64) {
    let taken: Vec<f64> = (0..g.n()).map(|k| if open[k] { 0.0 } else { was[k] + up[k] * dt - h[k] }).collect();
    let spread = g.spread(&taken, BENDS_M / 2.0);
    (0..g.n()).filter(|&k| !open[k]).for_each(|k| h[k] += FLOATS * spread[k]);
}

/// Creep of the slopes over a step: each sample of land changes by κ ∇²h across its four sides (`kappa` the
/// diffusivity times the step, m², Culling 1960), in explicit parts each under `STABLE` of the square of the shorter
/// side of a sample (Courant, Friedrichs and Lewy 1928); the open low ground stays, and a neighbour there stands at its
/// height or 0 m, the higher.
fn creep(g: &Grid, h: &mut [f64], open: &[bool], kappa: f64) {
    let side = g.dx.iter().fold(g.dy, |a, &b| a.min(b));
    let parts = (kappa / (STABLE * side * side)).ceil().max(1.0);
    for _ in 0..parts as usize {
        let was = h.to_vec();
        let at = |j: usize| if open[j] { was[j].max(0.0) } else { was[j] };
        for k in (0..g.n()).filter(|&k| !open[k]) {
            h[k] += kappa / parts * g.sides(k).iter().map(|&(j, len)| (at(j) - was[k]) / (len * len)).sum::<f64>();
        }
    }
}
