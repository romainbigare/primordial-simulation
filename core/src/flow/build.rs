//! The build of the terrain's samples from what the running world's load laid (laws.md 1.3): every `build_years`
//! years, at a year's start, the samples stand at the seed's heights with the relief laid on them (`space::relief`),
//! each cell's at its ground; the ways, the ponds and the wetness index are found again over them
//! (`Terrain::built`), and with them each cell's ground as liquid stands on it (`curve`), the bodies, the key samples
//! of the load and their rooms (`bodies::Bodies::of`). First the load piled at the mouths since the last build is laid
//! out as lobes (`lay`): the only matter a build moves. What a cell's parts of the bodies held follows each part's pond
//! (`held`), and the state of the ground nodes is found again. After edits shaped the ground the same is done at once,
//! without the lobes (`reshape`).

use super::{bodies::Bodies, curve, ground_now, land::solid, range, runoff, scratch::Holding, with, Scratch};
use crate::space::{delta::flood_depth, network::NONE, samples::Samples, terrain::SAMPLE_DEPTH};
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
        lay(w, s, &old);
        w.relief.built = w.relief.laid.clone();
        w.relief.builds += 1;
        w.relief.offset = offsets(w);
        anew(w, s, &old);
    });
    after(w);
}

/// The samples built anew after edits shaped the relief (`Relief::due`, `World::settle`): as a build, but no lobes are
/// laid, no build of the years is counted (`Relief::builds`; only the samples' version moves) and the cells' offsets stay
/// those of the last build (found here only where there was none); the caches of the slow-changing ground are found
/// again first (`scratch::refresh`: the bed an edit moved). An edit moves a cell's ground and its samples' mean
/// by the same volume, so its offset stays, to within the rounding of the areas, and the samples move only where edits
/// shaped them.
pub fn reshape(w: &mut World) {
    if w.relief.offset.len() != w.mesh.cells().n() {
        w.relief.offset = offsets(w);
    }
    with(w, true, |w, s| {
        // The edits moved the ground faster than the slow ground is checked (`scratch::refresh`): its caches first.
        super::scratch::refresh(w, s);
        let old = w.terrain_samples();
        anew(w, s, &old);
        reroute(w, (&old, &w.terrain_samples()));
    });
    after(w);
}

/// What runs through each sample (`World::discharge`, a mean over about a year) carried onto the samples' ways as
/// built again after edits (`reshape`), so the channels run where the ground now sends them: what each sample adds of
/// its own (what runs through it less what runs into it down the ways of the samples `old`) runs down the ways of the
/// samples `new`, each before the one it runs to, as the runoff's pass sums it (`runoff::discharge`). A pond's samples
/// keep what reaches them; a pond whose pass is new passes on what comes into it from that pass, as a mean over the years
/// does (a body fills first, then spills, and then all that comes in runs through); what a pond of `old` let over its
/// pass stays in what its pass adds of its own.
fn reroute(w: &mut World, (old, new): (&Samples, &Samples)) {
    let n = old.h.len();
    if w.discharge.len() != n || new.h.len() != n {
        return;
    }
    let passes = |s: &Samples, k: usize| s.pond[k] == NONE && s.to[k] != NONE;
    let mut q: Vec<f64> = w.discharge.iter().map(|&d| d as f64).collect();
    for k in (0..n).filter(|&k| passes(old, k)) {
        q[old.to[k] as usize] -= w.discharge[k] as f64;
    }
    q.iter_mut().for_each(|v| *v = v.max(0.0));
    // What comes into each pond whose pass is new, which runs on from that pass (the open low ground has none; what a
    // pond of `old` let over its pass is in what that pass adds of its own already). A pond's samples come before its
    // pass up the flood's order, as do those that run into them.
    let was: std::collections::BTreeSet<u32> = old.ponds.iter().map(|p| p.pass).collect();
    let mut at_pass: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for (p, pond) in new.ponds.iter().enumerate().filter(|(_, pond)| pond.pass != NONE && !was.contains(&pond.pass)) {
        at_pass.entry(pond.pass as usize).or_default().push(p);
    }
    let mut into = vec![0.0; new.ponds.len()];
    for &k in new.order.iter().rev() {
        let k = k as usize;
        if let Some(ps) = at_pass.get(&k) {
            ps.iter().for_each(|&p| q[k] += into[p]);
        }
        if new.pond[k] != NONE {
            into[new.pond[k] as usize] += q[k];
        } else if new.to[k] != NONE {
            q[new.to[k] as usize] += q[k];
        }
    }
    w.discharge = q.into_iter().map(|v| v as f32).collect();
}

/// The samples of a new version of the relief (`Relief::version`) with the cells' offsets it holds, and with them each
/// cell's ground as liquid stands on it (`curve::update`: found again where its samples moved), the bodies, what each cell's parts of them hold (`held_by_pond`; the
/// samples `old` before) and the state of the ground nodes.
fn anew(w: &mut World, s: &mut Scratch, old: &Samples) {
    w.relief.version += 1;
    w.relief.due = false;
    let held = std::mem::take(&mut s.state.held);
    let was: Vec<Vec<u32>> = s.curves.iter().map(|c| c.ponds().iter().map(|p| p.pond).collect()).collect();
    curve::update(w, s, old);
    s.bodies = Bodies::of(w, &s.curves);
    let new = w.terrain_samples();
    w.relief.ponds = new.pond_keys(&w.relief.ponds);
    s.state.held = held_by_pond(s, (old, &new), (&was, held));
    ground_now(w, s, true);
}

/// The ground the cells' plants root in and the way they face, from the samples as built.
fn after(w: &mut World) {
    w.loose_bins = crate::cover::loose_bins(w);
    w.rise_south = w.rise_at_build();
}

/// The load piled at the mouths since the last build (what was laid on the samples of the bodies since, `Relief::since`)
/// laid out as lobes on the samples as last built (`Terrain::lobes_of`, `delta::lobes`): each body's liquid at its
/// level now (from what its parts hold), the samples of its liquid its pond's (a lobe stays in its
/// mouth's body); each mouth's top its level and the flood's depth
/// over its entry (`delta::flood_depth`), no higher than its pass. The piles leave the relief, the lobes join it, and
/// what found no room is matter of the entry cell, which its offset spreads over it (as all matter the relief does
/// not place). What a lobe laid in another cell moves there from the entry cell's loose ground, by its mixture, with its
/// heat and its potential energy (`runoff::book_solids`): of each entry cell what leaves it, summed over its mouths, at
/// most what it holds loose (the rest stays, for the offsets); it mixes into the cell it reaches (`runoff::mix`).
fn lay(w: &mut World, s: &mut Scratch, samples: &Samples) {
    let (n, b) = (samples.h.len(), &s.bodies);
    // Each body's level: what its parts hold (the saved state) at one level over their samples (`level::level_over`),
    // each cell's at its ground found from its amounts (`World::ground_z`), no higher than its pass; so a loaded world
    // finds the same, as it reads no cache of the run.
    let levels: Vec<f64> = b.list.iter().map(|body| {
        let parts: Vec<(&super::curve::Part, f64)> = body.cells.iter().zip(&body.part).map(|(&i, &q)| (&s.curves[i as usize].parts[q], w.ground_z(i as usize))).collect();
        let held: f64 = body.cells.iter().zip(&body.part).map(|(&i, &q)| s.state.held[i as usize].m3[q]).sum();
        super::level::level_over(&parts, held).min(body.level)
    }).collect();
    let body: Vec<u32> = (0..n).map(|k| b.at.get(k).copied().unwrap_or(NONE)).collect();
    let level: Vec<f64> = body.iter().map(|&x| if x == NONE { f64::NEG_INFINITY } else { levels[x as usize] }).collect();
    let mut mouths = Vec::new();
    let piled: Vec<usize> = (0..n).filter(|&k| body[k] != NONE && w.relief.since(k) > 0.0).collect();
    for k in piled {
        let pond = samples.ponds[samples.pond[k] as usize];
        let pass = if pond.pass == NONE { f64::INFINITY } else { pond.level };
        mouths.push((k, w.relief.since(k) * samples.area(k), (level[k] + flood_depth(samples.up[k], samples.channel)).min(pass)));
        w.relief.laid[k] = w.relief.built.get(k).copied().unwrap_or(0.0);
    }
    if mouths.is_empty() {
        return;
    }
    let lat = (w.cfg.lat_south, w.cfg.lat_north);
    let (parts, unlaid) = w.terrain.lobes_of(&w.map, lat, samples, (&body, &level), &mouths, w.cfg.build_years as f64);
    // What each lobe laid, on the relief and by cell: of each sample laid on, its cells and their shares of its area.
    let cells = cells_of(w, samples);
    let mut out: Vec<((u32, u32), f64)> = Vec::new();
    let mut index = std::collections::HashMap::new();
    for &(k, v, m) in &parts {
        w.relief.lay(k, v / samples.area(k), n);
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
    let short = move_out(w, s, &out);
    (w.counts.load[5], w.counts.load[6]) = (w.counts.load[5] + unlaid.max(0.0), w.counts.load[6] + short);
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
            cells[k].push((i as u32, w.map.place_of(l, lat.0, lat.1).0 / samples.area(k)));
        }
    }
    cells
}

/// The volumes `out` (m³ from a cell to another, in the order they came) moved as loose solids (`lay`): each entry
/// cell gives at most what it holds loose, every move from it scaled by the same share where it holds less. Returns
/// what the moves lacked, m³.
fn move_out(w: &mut World, s: &mut Scratch, out: &[((u32, u32), f64)]) -> f64 {
    let n = w.mesh.cells().n();
    let t = range(&s.state.tg);
    let sol = s.solids(t.0, &s.liquids(&w.chem, t, true));
    let mut leaving = vec![0.0; n];
    out.iter().for_each(|&((from, _), v)| leaving[from as usize] += v);
    let mut rows = Vec::with_capacity(out.len());
    let (mut mixed, mut short) = (vec![[0.0; runoff::CLASS_COUNT]; n], 0.0);
    {
        let s: &Scratch = s;
        // (The loose ground: its solids that are not soft cover lying on it at its land's temperature, `Sp::soft_at`.)
        let ground = |i: usize| {
            let t = runoff::land_t(s, i);
            move |k: usize| !s.sp[k].soft_at(t)
        };
        let loose = |i: usize| sol.iter().filter(|&&k| ground(i)(k)).map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum::<f64>();
        for &((from, to), v) in out {
            let (i, held) = (from as usize, loose(from as usize));
            let gives = if leaving[i] > held { v * held / leaving[i] } else { v };
            short += v - gives.max(0.0);
            if gives <= 0.0 || held <= 0.0 {
                continue;
            }
            rows.push((from, to, runoff::loose_share(w, s, (&sol, ground(i)), i, gives / held)));
            let tx = &w.soil[i].texture;
            mixed[to as usize].iter_mut().zip([tx.fine, tx.middle(), tx.coarse]).for_each(|(m, c)| *m += gives * c);
        }
    }
    runoff::book_solids(w, s, &sol, &rows);
    runoff::mix(w, s, &mixed, &sol);
    short
}

/// Of each cell, how far its ground (`World::ground_z`) stands over the mean by area of its samples as the seed made
/// them with what was laid on each by the build and what edits shaped (`Relief::ground`), m; a cell finer than a sample
/// reads its sample.
fn offsets(w: &World) -> Vec<f64> {
    let (seed, lat) = (w.terrain.seed, (w.cfg.lat_south, w.cfg.lat_north));
    let s0 = w.terrain.samples(seed, &w.map, lat);
    w.map.leaves().iter().enumerate().map(|(i, &l)| {
        let mean = if l.depth <= SAMPLE_DEPTH {
            w.terrain.over(seed, &w.map, l, lat, |s, k| s.h[k] + w.relief.ground(k))
        } else {
            let k = s0.index(&l);
            s0.h[k] + w.relief.ground(k)
        };
        w.ground_z(i) - mean
    }).collect()
}

/// What each cell's parts of the bodies hold after a build: a part of a new pond takes what the cell's part of the old
/// pond it came from held (`came_from`), and a part that came from none of the cell's parts holds none yet (a pond new
/// to the cell fills from what comes into it, not from the liquid of the parts beside it); a cell some of whose parts
/// before no part takes takes `curve::first_held_at`. `samples` before and after; `was` the ponds of each cell's parts
/// before, `held` what they held.
fn held_by_pond(s: &Scratch, (old, new): (&Samples, &Samples), (was, held): (&[Vec<u32>], Vec<Holding>)) -> Vec<Holding> {
    let from = came_from(old, new);
    s.curves.iter().enumerate().map(|(i, c)| {
        let matched: Option<Vec<f64>> = was.get(i).zip(held.get(i)).and_then(|(before, h)| {
            let at = |q: u32| before.iter().position(|&p| from[q as usize] == p);
            let places: Vec<Option<usize>> = c.ponds().iter().map(|p| at(p.pond)).collect();
            (0..before.len()).all(|q| places.contains(&Some(q))).then(|| places.iter().map(|q| q.map_or(0.0, |q| h.m3[q])).collect())
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
