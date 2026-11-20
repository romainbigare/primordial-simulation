//! The high stand at the world's start (laws.md 0): after the wear (`space::wear`) the liquid of the open low ground
//! rises to 0 m and stands there for `HIGH_STAND_YEARS`, and the channels fill the heads of the valleys it drowned. Each
//! mouth brings the load its catchment yields, by the law fitted to the load of the channels of the one planet known
//! (Syvitski and Milliman 2007): Qs = ω Q^0.31 A^0.5 R T, its discharge Q, its area A, its relief R and its mean
//! temperature T. The ground its catchment loses is taken where the wear's last stage lowered it fastest, and the load
//! is laid as a shelf at the mouth (`shelf::lay`), up to the top a deposit under liquid builds to (`shelf::top`): the
//! channel's flood at the mouth, less the waves' base there, so a sheltered valley head fills to new land and an open
//! coast keeps a platform under the liquid.

use super::drain::{draws, flood, open_low, starts};
use super::grid::Grid;
use super::network::NONE;
use super::shelf::{self, lay};
use super::slopes::ROCK_KG_M3;
use super::wear::{Books, STAND_M};
use crate::units::T_SURF;

/// The years the liquid stands at its level before the world starts, the channels filling what it drowned.
const HIGH_STAND_YEARS: f64 = 6.0e3;
/// The yield law (Syvitski and Milliman 2007), in its own units: the load is `YIELD_OMEGA` Mt a year times the
/// discharge in km³ a year to `YIELD_Q_EXP`, the area in km² to `YIELD_A_EXP`, the relief in km and the temperature in
/// degrees over `YIELD_T0_K`.
const YIELD_OMEGA: f64 = 6.0e-4;
const YIELD_Q_EXP: f64 = 0.31;
const YIELD_A_EXP: f64 = 0.5;
const YIELD_T0_K: f64 = 273.15;
/// The liquid that runs off a catchment in a year, m (its discharge over its area).
const RUNOFF_M: f64 = 0.3;

/// The load a catchment of `area` m² whose highest ground stands `relief` m over the mouth yields in a year, m³ of bed
/// (Syvitski and Milliman 2007, at the typical surface temperature `T_SURF`).
fn yields(area: f64, relief: f64) -> f64 {
    let (km2, km3) = (area / 1e6, RUNOFF_M * area / 1e9);
    let mt = YIELD_OMEGA * km3.powf(YIELD_Q_EXP) * km2.powf(YIELD_A_EXP) * (relief / 1e3) * (T_SURF - YIELD_T0_K);
    mt * 1e9 / ROCK_KG_M3
}

/// The high stand on grid `g` of heights `h` (the module's doc), the wear's last stage having lowered each sample at
/// `lowers`, m a year: lowers each catchment by its load where it lowered fastest, lays the load at its mouth as a
/// shelf (adding it to the loose ground `loose`, m), and books both in `books`. Returns what it laid on each sample, m, and how long ago its top was laid, years: a lobe fed at an even rate
/// grows out in proportion to what it was fed, so its front is laid last and its middle first (`shelf::lay`).
pub(super) fn high_stand(seed: u64, g: &Grid, h: &mut [f64], loose: &mut [f64], lowers: &[f64], books: &mut Books) -> (Vec<f64>, Vec<f64>) {
    let n = g.n();
    let open = open_low(g, h, STAND_M);
    let f = flood(g, h, &open, (&starts(g, &open, false), &draws(seed, 1, n)));
    // The ground that drains to the open low ground with no closed basin on its way, from the open low ground up.
    let mut free = vec![false; n];
    for &k in &f.order {
        let (k, j) = (k as usize, f.to[k as usize]);
        free[k] = !open[k] && !f.under(h, k) && j != NONE && (open[j as usize] || free[j as usize]);
    }
    // Of each sample, what drains through it: its area, m², and its highest ground, m.
    let (mut area, mut high) = (vec![0.0; n], h.to_vec());
    for &k in f.order.iter().rev() {
        let k = k as usize;
        if !free[k] {
            continue;
        }
        area[k] += g.area(k);
        let j = f.to[k] as usize;
        if !open[j] {
            (area[j], high[j]) = (area[j] + area[k], high[j].max(high[k]));
        }
    }
    // How far each sample stands over the one its liquid runs to (over the liquid's level where it runs into it), m: the
    // most flowing liquid can cut it; how fast it lowered, m³ a year; and that summed over each catchment.
    let over: Vec<f64> = (0..n).map(|k| if free[k] { (h[k] - h[f.to[k] as usize].max(STAND_M)).max(0.0) } else { 0.0 }).collect();
    let cuts: Vec<f64> = (0..n).map(|k| if free[k] { lowers[k].max(0.0) * g.area(k) } else { 0.0 }).collect();
    let mut cut = cuts.clone();
    for &k in f.order.iter().rev() {
        let (k, j) = (k as usize, f.to[k as usize]);
        if free[k] && !open[j as usize] {
            cut[j as usize] += cut[k];
        }
    }
    // Each mouth's load over the stand, m³; each sample's mouth, from the open low ground up.
    let mut mouth = vec![NONE; n];
    let mut load = vec![0.0; n];
    for &k in &f.order {
        let k = k as usize;
        if !free[k] {
            continue;
        }
        let j = f.to[k] as usize;
        mouth[k] = if open[j] { k as u32 } else { mouth[j] };
        if open[j] {
            load[k] = yields(area[k], (high[k] - STAND_M).max(0.0)) * HIGH_STAND_YEARS;
        }
    }
    // The catchment gives it where it lowered fastest (by its area where none lowered), each sample no lower than the
    // one its liquid runs to: the mouth brings what its catchment gives.
    let mut given = vec![0.0; n];
    for k in (0..n).filter(|&k| free[k]) {
        let m = mouth[k] as usize;
        let share = if cut[m] > 0.0 { cuts[k] / cut[m] } else { g.area(k) / area[m] };
        let down = (load[m] * share / g.area(k)).min(over[k]);
        h[k] -= down;
        given[m] += down * g.area(k);
    }
    let load = given;
    let taken: f64 = load.iter().sum();
    // The load at the sample of the open low ground each mouth runs to, m³ per m², with the mouth's flood there.
    let (mut offshore, mut flooded) = (vec![0.0; n], vec![0.0f64; n]);
    for k in (0..n).filter(|&k| load[k] > 0.0) {
        let j = f.to[k] as usize;
        offshore[j] += load[k] / g.area(j);
        flooded[j] = flooded[j].max(area[k]);
    }
    let was = h.to_vec();
    let (laid, since) = lay(g, h, (&open, &offshore), (&|k| shelf::exposure(g, &open, k), &|k, exposed| shelf::top(STAND_M, flooded[k], exposed)), loose);
    (books.taken, books.high, books.beyond) = (books.taken + taken, laid, books.beyond + taken - laid);
    ((0..n).map(|k| (h[k] - was[k]).max(0.0)).collect(), since.iter().map(|s| s * HIGH_STAND_YEARS).collect())
}
