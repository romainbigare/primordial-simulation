//! The build of the terrain's samples from what the running world's load laid (laws.md 1.3): every `build_years`
//! years, at a year's start, the samples stand at the seed's heights with the relief laid on them (`space::relief`),
//! each cell's at its ground; the ways, the ponds and the wetness index are found again over them
//! (`Terrain::built`), and with them each cell's ground as liquid stands on it (`curve`), the bodies, the key samples
//! of the load and their rooms (`bodies::Bodies::of`). First the load piled at the mouths since the last build is laid
//! out as lobes (`lay`): the only matter a build moves. What a cell's parts of the bodies held follows each part's pond
//! (`held`), and the state of the ground nodes is found again.

use super::{bodies::Bodies, curve, ground_now, land::solid, range, runoff, scratch::Holding, with, Scratch};
use crate::space::{delta::flood_depth, network::NONE, samples::Samples, terrain::SAMPLE_DEPTH};
use crate::World;

/// The particle classes of the soil's texture a move mixes in (`runoff::mix`).
const CLASSES: usize = crate::soil::CLASSES.len();

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
        lay(w, s, &old);
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

/// The load piled at the mouths since the last build (what was laid on the samples of the bodies since, `Relief::since`)
/// laid out as lobes on the samples as last built (`Terrain::lobes_of`, `delta::lobes`): each body's liquid at its
/// level now (`basin::Basin::level`), the open samples its pond's; each mouth's top its level and the flood's depth
/// over its entry (`delta::flood_depth`), no higher than its pass. The piles leave the relief, the lobes join it, and
/// what found no room is matter of the entry cell, which its offset spreads over it (as all matter the relief does
/// not place). What a lobe laid in another cell moves there from the entry cell's loose ground, by its mixture, with its
/// heat and its potential energy (`runoff::book_solids`): of each entry cell what leaves it, summed over its mouths, at
/// most what it holds loose (the rest stays, for the offsets); it mixes into the cell it reaches (`runoff::mix`).
fn lay(w: &mut World, s: &mut Scratch, samples: &Samples) {
    let (n, b) = (samples.h.len(), &s.bodies);
    let area = |k: usize| w.map.place_of(&samples.leaf(k), w.cfg.lat_south, w.cfg.lat_north).0;
    let (open, level): (Vec<bool>, Vec<f64>) = (0..n).map(|k| match b.at.get(k).copied() {
        Some(body) if body != NONE => (true, s.basin.level[body as usize]),
        _ => (false, f64::NEG_INFINITY),
    }).unzip();
    let mut mouths = Vec::new();
    let piled: Vec<usize> = (0..n).filter(|&k| open[k] && w.relief.since(k) > 0.0).collect();
    for k in piled {
        let pond = samples.ponds[samples.pond[k] as usize];
        let pass = if pond.pass == NONE { f64::INFINITY } else { pond.level };
        mouths.push((k, w.relief.since(k) * area(k), (level[k] + flood_depth(samples.up[k], samples.channel)).min(pass)));
        w.relief.laid[k] = w.relief.built.get(k).copied().unwrap_or(0.0);
    }
    if mouths.is_empty() {
        return;
    }
    let lat = (w.cfg.lat_south, w.cfg.lat_north);
    let (parts, _unlaid) = w.terrain.lobes_of(&w.map, lat, samples, (&open, &level), &mouths, w.cfg.build_years as f64);
    // What each lobe laid, on the relief and by cell: of each sample laid on, its cells and their shares of its area.
    let cells = cells_of(w, samples);
    let mut out: Vec<((u32, u32), f64)> = Vec::new();
    let mut index = std::collections::HashMap::new();
    for &(k, v, m) in &parts {
        w.relief.lay(k, v / area(k), n);
        let from = b.cell_of[mouths[m as usize].0];
        for &(to, share) in &cells[k] {
            if to != from {
                let at = *index.entry((from, to)).or_insert_with(|| {
                    out.push(((from, to), 0.0));
                    out.len() - 1
                });
                out[at].1 += v * share;
            }
        }
    }
    move_out(w, s, &out);
}

/// Of each sample the cells it lies in, each with its share of the sample's area (one, but under cells finer than it).
fn cells_of(w: &World, samples: &Samples) -> Vec<Vec<(u32, f64)>> {
    let lat = (w.cfg.lat_south, w.cfg.lat_north);
    let mut cells = vec![Vec::new(); samples.h.len()];
    for (i, l) in w.map.leaves().iter().enumerate() {
        if l.depth <= SAMPLE_DEPTH {
            l.split(SAMPLE_DEPTH).for_each(|f| cells[samples.index(&f)] = vec![(i as u32, 1.0)]);
        } else {
            let k = samples.index(l);
            cells[k].push((i as u32, w.map.place_of(l, lat.0, lat.1).0 / w.map.place_of(&samples.leaf(k), lat.0, lat.1).0));
        }
    }
    cells
}

/// The volumes `out` (m³ from a cell to another, in the order they came) moved as loose solids (`lay`): each entry
/// cell gives at most what it holds loose, every move from it scaled by the same share where it holds less.
fn move_out(w: &mut World, s: &mut Scratch, out: &[((u32, u32), f64)]) {
    let n = w.mesh.cells().n();
    let t = range(&s.state.tg);
    let sol = s.solids(t.0, &s.liquids(&w.chem, t, true));
    let loose = |i: usize| sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum::<f64>();
    let mut leaving = vec![0.0; n];
    out.iter().for_each(|&((from, _), v)| leaving[from as usize] += v);
    let mut rows = Vec::with_capacity(out.len());
    let mut mixed = vec![[0.0; CLASSES]; n];
    for &((from, to), v) in out {
        let (i, held) = (from as usize, loose(from as usize));
        let gives = if leaving[i] > held { v * held / leaving[i] } else { v };
        if gives <= 0.0 || held <= 0.0 {
            continue;
        }
        let share = gives / held;
        let mut row: Vec<f64> = sol.iter().map(|&k| solid(w, s, k, i) * share).collect();
        let heat: f64 = sol.iter().zip(&row).map(|(&k, m)| m * s.sp[k].cp * s.state.tg[i]).sum();
        row.push(heat);
        rows.push((from, to, row));
        let tx = &w.soil[i].texture;
        mixed[to as usize].iter_mut().zip([tx.fine, tx.middle(), tx.coarse]).for_each(|(m, c)| *m += gives * c);
    }
    runoff::book_solids(w, s, &sol, &rows);
    runoff::mix(w, s, &mixed, &sol);
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

/// What each cell's parts of the bodies hold after a build: a part of a new pond takes what the cell's part of the old
/// pond it came from held (`came_from`); a cell whose parts do not each take one part of its own, all of them, takes
/// `curve::first_held_at`. `samples` before and after; `was` the ponds of each cell's parts before, `held` what they
/// held.
fn held_by_pond(s: &Scratch, (old, new): (&Samples, &Samples), (was, held): (&[Vec<u32>], Vec<Holding>)) -> Vec<Holding> {
    let from = came_from(old, new);
    s.curves.iter().enumerate().map(|(i, c)| {
        let matched: Option<Vec<f64>> = was.get(i).zip(held.get(i)).and_then(|(before, h)| {
            let at = |q: u32| before.iter().position(|&p| from[q as usize] == p);
            let places: Option<Vec<usize>> = c.ponds().iter().map(|p| at(p.pond)).collect();
            places.filter(|ps| ps.len() == before.len()).map(|ps| ps.iter().map(|&q| h.m3[q]).collect())
        });
        matched.map_or_else(|| curve::first_held_at(s, i), |m3| curve::held_by(c, m3))
    }).collect()
}

/// Of each pond of the samples `new`, the pond of the samples `old` that held most of its samples, where that pond is
/// the one that most of no other new pond came from (`NONE` for none): one to one.
fn came_from(old: &Samples, new: &Samples) -> Vec<u32> {
    let mut counts: Vec<std::collections::BTreeMap<u32, u32>> = vec![Default::default(); new.ponds.len()];
    for (k, &q) in new.pond.iter().enumerate().filter(|p| *p.1 != NONE && old.pond[p.0] != NONE) {
        *counts[q as usize].entry(old.pond[k]).or_default() += 1;
    }
    let best: Vec<u32> = counts.iter().map(|c| c.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).map_or(NONE, |p| *p.0)).collect();
    let mut taken = vec![0u32; old.ponds.len()];
    best.iter().filter(|&&p| p != NONE).for_each(|&p| taken[p as usize] += 1);
    best.iter().map(|&p| if p != NONE && taken[p as usize] == 1 { p } else { NONE }).collect()
}
