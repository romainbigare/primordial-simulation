//! Inside a cell (laws.md 0, `pcore::inside`): one rule shares a cell's means over its own ground. Each share keeps its
//! mean over the cell exactly, and the cover of a kind that likes wet ground gathers where liquid gathers.

use pcore::inside::{Ground, Inside, Means};
use pcore::World;
use worldgen::testkit;

/// The liquid in its pores, its pores' room, what stands and its snow, m: a cell half wet, some liquid standing.
const MEANS: Means = Means { pore: 0.08, pores: 0.2, stands: 0.05, snow: 0.3 };

/// The land cell of `w` whose ground inside it is the roughest (the widest span of its samples' heights): its ground, its
/// shares with what `upstream` of its areas drain to it from upstream, and the area that drains to it so, m².
fn rough(w: &World, upstream: f64) -> (Ground, Inside, f64) {
    let span = |g: &Ground| g.h.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)) - g.h.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let i = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).max_by(|&a, &b| span(&w.ground_in(w.map.leaves()[a])).total_cmp(&span(&w.ground_in(w.map.leaves()[b])))).expect("land");
    let g = w.ground_in(w.map.leaves()[i]);
    assert!(span(&g) > 10.0, "rough ground: its samples span {} m", span(&g));
    let inflow = upstream * w.mesh.cells().area()[i];
    let shares = Inside::of(&g, &w.cfg.phys, inflow, MEANS);
    (g, shares, inflow)
}

/// Each share of a cell's means over its ground (the liquid in its soil, its snow, each kind of cover by its moisture
/// gene) is none below 0 and has a mean of 1 by area over the cell, to the rounding of a sum, on rough ground with liquid
/// coming from upstream; its quarters' parts sum to 1; and no point's soil holds more than its pores.
#[test]
fn the_shares_keep_each_mean_over_a_cell() {
    let w = testkit::small_world();
    let (g, s, _) = rough(&w, 30.0);
    let kinds: Vec<Vec<f64>> = [-2.0, -1.0, -0.3, 0.0].iter().map(|&m| s.kind(&g, &w.cfg.phys, m)).collect();
    for (name, share) in [("soil", &s.soil), ("snow", &s.snow)].into_iter().chain(kinds.iter().map(|k| ("a kind of cover", k))) {
        assert!(share.iter().all(|&x| x >= 0.0), "{name}: no share below 0");
        assert!((g.mean(share) - 1.0).abs() < 1e-12, "{name}: the mean of its shares is {}", g.mean(share));
        let parts = g.quarters(share);
        assert!((parts.iter().sum::<f64>() - 1.0).abs() < 1e-12, "{name}: its quarters' parts sum to {}", parts.iter().sum::<f64>());
    }
    let most = s.soil.iter().fold(0.0f64, |a, &b| a.max(b)) * MEANS.pore;
    assert!(most <= MEANS.pores * (1.0 + 1e-12), "a point's soil holds {most} m, its pores {} m", MEANS.pores);
    assert!(s.snow.iter().any(|&x| (x - 1.0).abs() > 0.01) && s.soil.iter().any(|&x| (x - 1.0).abs() > 0.01), "rough ground shares unevenly");
}

/// Where liquid gathers most in a cell (the highest wetness index: a hollow, the foot of a slope) a kind of cover that
/// likes wet ground has a greater share than where it gathers least, and a kind that likes dry ground a smaller one.
#[test]
fn wet_places_hold_more_of_a_wet_kind_and_less_of_a_dry_one() {
    let w = testkit::small_world();
    let (g, s, inflow) = rough(&w, 3.0);
    let index = g.wetness_index(inflow);
    let at = |better: fn(f64, f64) -> bool| (0..index.len()).fold(0, |p, q| if better(index[q], index[p]) { q } else { p });
    let (wet, dry) = (at(|a, b| a > b), at(|a, b| a < b));
    assert!(s.wet[wet] > s.wet[dry], "the hollow is wetter than the ridge: {} against {}", s.wet[wet], s.wet[dry]);
    let (likes_wet, likes_dry) = (s.kind(&g, &w.cfg.phys, -0.1), s.kind(&g, &w.cfg.phys, -1.8));
    assert!(likes_wet[wet] > likes_wet[dry], "a wet kind: {} in the hollow against {} on the ridge", likes_wet[wet], likes_wet[dry]);
    assert!(likes_dry[wet] < likes_dry[dry], "a dry kind: {} in the hollow against {} on the ridge", likes_dry[wet], likes_dry[dry]);
}
