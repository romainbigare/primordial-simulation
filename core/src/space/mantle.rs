//! The loose ground the wear at the world's start leaves on the land (laws.md 0 and 5, `space::wear`): the depth where
//! the bed's breaking into loose ground balances what takes it away. The bed breaks at `BREAKS` a year under no loose
//! ground, e times slower under each `BREAKS_UNDER_M` of it (Heimsath, Dietrich, Nishiizumi and Finkel 1997). Two things
//! take it away, and the depth is the smaller of the two balances (all of the loose ground creeps, what settled too):
//! - the lowering of the ground E (cut and creep): a depth of `BREAKS_UNDER_M` ln(`BREAKS` / E), none where E outruns
//!   `BREAKS`;
//! - the creep of the loose ground down its slope S: all the loose ground made on the area a that drains through a
//!   sample, per m of its width, passes it, with what settled on it (as fast as it came over the last stage, across the
//!   sample's width: b), and creeps at `CREEP` S / (1 − (S / `REST`)²) a m of its depth over
//!   `CREEP_DEPTH_M` (a flux that grows without bound as the slope nears the slope loose ground rests at: Roering,
//!   Kirchner and Dietrich 1999, each m of depth creeping alike), so a depth H holds where H `CREEP` / `CREEP_DEPTH_M`
//!   S / (1 − (S / `REST`)²) = a `BREAKS` exp(−H / `BREAKS_UNDER_M`) + b; none at `REST` or steeper.
//!
//! What settled on a sample in the wear's last stage lies over the first. So a slope near `REST` holds almost no loose
//! ground (its bed bare), and the gentle ground at its foot, where a grows and S falls, holds the more.
//!
//! Where edits shaped a sample (`Relief::shaped`), its loose ground follows (`edited`): a lowering takes it first, and
//! it thins as the creep holds less on its new slope than on its old, so the flanks of a hill raised steep keep little.

use super::grid::Grid;

/// The rate the bed breaks into loose ground under none, m a year, and the depth of loose ground under which it breaks
/// e times slower, m.
const BREAKS: f64 = 7.7e-5;
const BREAKS_UNDER_M: f64 = 0.4;
/// The slowest lowering the mantle's law reads, a share of `BREAKS`: a sample that does not lower keeps the mantle of
/// this lowering, not an infinite one.
const SLOWEST: f64 = 1e-6;
/// The slope loose ground rests at, rise over run: its creep grows without bound as a slope nears it, and ground edits
/// shape slides till it stands no steeper (`space::shaped`).
pub const REST: f64 = 0.8;
/// The depth of loose ground the creep's diffusivity (`wear::CREEP`) is anchored at, m.
const CREEP_DEPTH_M: f64 = 1.0;
/// The search for the depth the creep holds stops after this many Newton's steps, or a step shorter than this, m.
const NEWTON_STEPS: usize = 30;
const CLOSE_M: f64 = 1e-6;

/// The depth of the loose ground of each sample of grid `g`, m: what `settled` on it over the last `years`, m, and the
/// mantle (the module's doc) where the ground lowers at `lowers`, m a year, drains `drained`, m², and stands at heights
/// `h`, m, its loose ground creeping at `creep`, m² a year at `CREEP_DEPTH_M` of depth.
pub(super) fn mantle(g: &Grid, (h, drained): (&[f64], &[f64]), (settled, lowers): (&[f64], &[f64]), (creep, years): (f64, f64)) -> Vec<f64> {
    (0..g.n()).map(|k| depth((settled[k], lowers[k]), (slope(g, &|j| h[j], k), drained[k] / g.dy), (settled[k] / years * g.dy, creep))).collect()
}

/// The loose ground of sample `k` of grid `g` once edits shaped each sample j by `shaped(j)` m (raised; lowered where
/// under none), where it held `loose` m with `settled` m of it laid by the liquid, and stands at heights `h` draining
/// `drained` m²: a lowering takes the loose ground first, from its top, where what settled lies (the module's doc), and
/// of what is left the share stays that the creep holds on its new slope over what it held on its slope before the
/// edits, where less (`depth`, the bed breaking under it as under ground that does not lower, `SLOWEST`; nothing coming
/// on it from upslope, as an edit is over in no time), the creep taking what settled with the rest. So ground the
/// edits barely tilted keeps nearly all its own. Returns its depth and what of it settled, m (never more than its
/// depth). Unshaped, it keeps its own.
pub(super) fn edited(g: &Grid, k: usize, (h, drained): (&dyn Fn(usize) -> f64, f64), (loose, settled): (f64, f64), shaped: &dyn Fn(usize) -> f64) -> [f64; 2] {
    if shaped(k) == 0.0 {
        return [loose, settled];
    }
    let (left, laid) = if shaped(k) < 0.0 { ((loose + shaped(k)).max(0.0), (settled + shaped(k)).max(0.0)) } else { (loose, settled) };
    let holds = |s: f64| depth((settled, BREAKS * SLOWEST), (s, drained / g.dy), (0.0, super::wear::CREEP));
    let (now, before) = (holds(slope(g, h, k)), holds(slope(g, &|j| h(j) - shaped(j), k)));
    let kept = if now >= before { left } else { left * now / before.max(f64::MIN_POSITIVE) };
    [kept, laid.min(kept)]
}

/// How long the loose ground of `loose` m has lain, years, where `settled` m settled on it over the last `years` and
/// its ground lowers at `lowers`, m a year: its depth over how fast it is renewed, from below as fast as the ground
/// lowers (the bed breaks into it as fast as it is taken away, the module's doc) or from above as fast as it settled,
/// at least a lowering of `SLOWEST` of `BREAKS` (the mean age of a mixed store, its content over its flux).
pub(super) fn age(loose: f64, (settled, years): (f64, f64), lowers: f64) -> f64 {
    loose / lowers.max(settled / years).max(BREAKS * SLOWEST)
}

/// The slope of heights `h` at sample `k` of grid `g`, rise over run: from the differences across it east to west and
/// north to south (one-sided at the map's edge).
fn slope(g: &Grid, h: &dyn Fn(usize) -> f64, k: usize) -> f64 {
    let [(e, dx), (w, _), (n, dy), (s, _)] = g.sides(k);
    let across = |a: usize, b: usize, len: f64| (h(a) - h(b)) / (len * ((a != k) as u8 + (b != k) as u8).max(1) as f64);
    across(e, w, dx).hypot(across(s, n, dy))
}

/// The loose ground's depth, m, where `settled` m settled and the ground lowers at `e`, m a year, on slope `s` with `a`
/// m² drained per m of width, what settled coming at `brought` m² a year per m of width, its loose ground creeping at
/// `creep` (the module's doc): what settled and the mantle the lowering leaves, or less where the creep holds less.
fn depth((settled, e): (f64, f64), (s, a): (f64, f64), (brought, creep): (f64, f64)) -> f64 {
    let lowered = settled + BREAKS_UNDER_M * (BREAKS / e.max(BREAKS * SLOWEST)).ln().max(0.0);
    if s >= REST {
        return 0.0;
    }
    // What the creep carries against what the bed breaks into loose ground upslope and what settled: the first grows
    // with the depth in proportion, the second falls (concave in the depth), so one depth balances them, found from no
    // depth up by Newton's steps, each short of it.
    let carries = creep / CREEP_DEPTH_M * s / (1.0 - (s / REST) * (s / REST));
    let off = |d: f64| d * carries - a * BREAKS * (-d / BREAKS_UNDER_M).exp() - brought;
    if off(lowered) <= 0.0 {
        return lowered;
    }
    let mut d = 0.0;
    for _ in 0..NEWTON_STEPS {
        let step = -off(d) / (carries + a * BREAKS / BREAKS_UNDER_M * (-d / BREAKS_UNDER_M).exp());
        d += step;
        if step < CLOSE_M {
            break;
        }
    }
    d.min(lowered)
}
