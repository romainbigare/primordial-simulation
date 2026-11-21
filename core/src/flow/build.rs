//! The build of the terrain's samples from what the running world's load laid (laws.md 1.3): every `build_years`
//! years, at a year's start, the samples stand at the seed's heights with the relief laid on them (`space::relief`),
//! each cell's at its ground; the ways, the ponds and the wetness index are found again over them
//! (`Terrain::built`), and with them each cell's ground as liquid stands on it (`curve`), the bodies, the key samples
//! of the load and their rooms (`bodies::Bodies::of`). No matter moves: what a cell's parts of the bodies held follows
//! each part's pond (`held`), and the state of the ground nodes is found again.

use super::{bodies::Bodies, curve, ground_now, scratch::Holding, with, Scratch};
use crate::space::{network::NONE, samples::Samples, terrain::SAMPLE_DEPTH};
use crate::World;

/// Is a build of the samples due at the world's time now: a year's start, every `Config::build_years` years (none at the
/// start)?
pub fn due(w: &World) -> bool {
    let cal = w.cfg.calendar();
    let year = w.time / cal.year;
    w.time.is_multiple_of(cal.year) && year > 0 && year.is_multiple_of(w.cfg.build_years)
}

/// Build the samples anew (the module's doc).
pub fn build(w: &mut World) {
    with(w, true, |w, s| {
        let old = w.terrain_samples();
        w.relief.offset = offsets(w);
        w.relief.built = w.relief.laid.clone();
        w.relief.builds += 1;
        let held = std::mem::take(&mut s.state.held);
        let was: Vec<Vec<u32>> = s.curves.iter().map(|c| c.ponds().iter().map(|p| p.pond).collect()).collect();
        curve::build(w, s);
        s.bodies = Bodies::of(w, &s.curves);
        let new = w.terrain_samples();
        s.state.held = held_by_pond(s, (&old, &new), (&was, held));
        ground_now(w, s, true);
    });
}

/// Of each cell, how far its ground (`World::ground_z`) stands over the mean by area of its samples as the seed made
/// them with what was laid on each (`Relief::laid`), m; a cell finer than a sample reads its sample.
fn offsets(w: &World) -> Vec<f64> {
    let (seed, lat) = (w.terrain.seed, (w.cfg.lat_south, w.cfg.lat_north));
    let s0 = w.terrain.samples(seed, &w.map, lat);
    w.map.leaves().iter().enumerate().map(|(i, &l)| {
        let mean = if l.depth <= SAMPLE_DEPTH {
            w.terrain.over(seed, &w.map, l, lat, |s, k| s.h[k] + w.relief.at(k))
        } else {
            let k = s0.index(&l);
            s0.h[k] + w.relief.at(k)
        };
        w.ground_z(i) - mean
    }).collect()
}

/// What each cell's parts of the bodies hold after a build: a part of a new pond takes what the cell's part of the
/// same pond held before, ponds matched by their pass (`samples` before and after), the open low ground's one part by
/// itself; a cell whose parts do not all match takes `curve::first_held`. `was` the ponds of each cell's parts before,
/// `held` what they held.
fn held_by_pond(s: &Scratch, (old, new): (&Samples, &Samples), (was, held): (&[Vec<u32>], Vec<Holding>)) -> Vec<Holding> {
    let first = curve::first_held(s);
    let key = |samples: &Samples, p: u32| samples.ponds[p as usize].pass;
    s.curves.iter().enumerate().map(|(i, c)| {
        let (before, h) = (was.get(i), held.get(i));
        let matched: Option<Vec<f64>> = before.zip(h).and_then(|(before, h)| {
            let open_before = before.iter().filter(|&&p| key(old, p) == NONE).count();
            let take = |q: u32| before.iter().position(|&p| key(old, p) == key(new, q) && (key(new, q) != NONE || open_before == 1)).map(|at| h.m3[at]);
            let m3: Option<Vec<f64>> = c.ponds().iter().map(|p| take(p.pond)).collect();
            m3.filter(|m| m.len() == before.len())
        });
        matched.map_or_else(|| first[i].clone(), |m3| curve::held_by(c, m3))
    }).collect()
}
