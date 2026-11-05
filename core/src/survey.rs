//! The bodies seen from their cells: at each month's start what each cell holds and the shade of its bodies
//! (`survey`), and every body's update, in a parallel and a serial pass (`batch`), at the month's start and again inside
//! it in the cells finer than a square (`steps`).

use crate::body::{die, live, Layout, LiveTmp, LACK};
use crate::life::{self, Chunk, Handle};
use crate::store::{Class, Node};
use crate::{choose, World};

mod draws;
mod steps;
pub(crate) use draws::release;
pub use steps::substeps;


/// Temperature, depth of the standing liquid and surface height of a ground cell, as Flow's last step left them (once
/// Flow's working memory is built, `flow::ensure`).
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let (d, z) = w.scratch.view().standing(i);
    [w.t_ground_last(i), d, z]
}

/// At each month's start: what movers broke and what each cell holds (`break_and_sum`). Then the caches of the cells
/// (`caches`), once a month the traffic and each cell's surroundings, and the cells' features (`choose::cells`); both
/// read the members that moved in since the last month's survey, and that count starts again.
pub fn survey(w: &mut World) {
    let n = w.mesh.cells().n();
    let monthly = monthly(w);
    break_and_sum(w, None);
    caches(w);
    if monthly && w.time > 0 {
        traffic(w);
    }
    if monthly || w.life.env.len() != n {
        // From Flow's state rebuilt in full, as at a month's start, so a resumed run reads what an unbroken one does.
        crate::flow::state(w, true);
        w.life.env = (0..n).map(|i| env_of(w, i)).collect();
    }
    choose::cells(w);
    // (The members that moved in since the last survey are read by both; the count starts again.)
    w.life.cells.iter_mut().for_each(|c| c.crossings = 0.0);
}

/// One task per chunk on the pool (`life::run`): in each cell `steps` marks (every cell with none), what movers broke
/// since its last survey (`choose::wear`); then in every cell the visible traits that changed and what it holds
/// (`rows`). A death that lets go of a part queues that, applied after the tasks (`life::land`, laws.md 0).
fn break_and_sum(w: &mut World, steps: Option<&[bool]>) {
    w.life.compact();
    let load = w.life.cells.iter().filter(|c| !c.bodies.is_empty()).count();
    let sums = life::run(w, rows_of(w), load, |_| (), |w, d, sums, _| {
        for i in d.cells().filter(|&i| steps.is_none_or(|s| s[i])) {
            // (A cell with no bodies and no cover has nothing to break or sum.)
            if d.at(i).bodies.is_empty() && !d.at(i).cover.any() {
                d.at_mut(i).pushed = 0.0;
                continue;
            }
            let e = std::mem::take(&mut d.at_mut(i).pushed);
            if e > 0.0 {
                choose::wear(w, d, i, e);
            }
        }
        d.see_changed(w);
        rows(w, d, sums);
    });
    w.life.sums = sums.concat();
    life::land(w);
}

/// Once a month, after Flow's state is built in full: the cover's month (`cover::step`) in each cell that holds any,
/// one task per chunk on the pool; then its seeds land (`cover::land`).
pub fn cover(w: &mut World) {
    let load = w.life.cells.iter().filter(|c| c.cover.any()).count();
    life::run(w, vec![(); life::chunks(w)], load, |_| (), |w, d, _, _| crate::cover::step(w, d));
    life::land(w);
    crate::cover::land(w);
}

/// Whether it is a month's start: the survey adds up the traffic and rebuilds Flow's state in full.
fn monthly(w: &World) -> bool {
    w.time.is_multiple_of(w.cfg.calendar().month)
}

/// Per chunk, a row of zeros for each of its cells (`rows`).
fn rows_of(w: &World) -> Vec<Vec<[f64; ROW]>> {
    life::bounds(w).windows(2).map(|b| vec![[0.0; ROW]; b[1] - b[0]]).collect()
}

/// The numbers of the cells of chunk `d` (`Life::sum`), in `sums`.
fn rows(w: &World, d: &Chunk, sums: &mut [[f64; ROW]]) {
    for (i, row) in d.cells().zip(sums) {
        *row = cell_row(w, d, i);
    }
}

/// The numbers of cell `i` of chunk `d` (`Life::sum`), from its cover and its bodies in the order they came: their
/// members, then their visible traits summed, then the energy to break them; then the cover's capture area times the
/// share of light it absorbs, and that times the share set aside for its light reaction.
fn cell_row(w: &World, d: &Chunk, i: usize) -> [f64; ROW] {
    let mut row = [0.0; ROW];
    let cell = d.at(i);
    // Its cover, as one more body of no members: its mass, capture area and energy to break, and the light it absorbs
    // at its mean shade (its kinds share it out among them, `cover::step`).
    if let Some(p) = crate::cover::Parts::of(&w.chem) {
        let (c, mm) = (&cell.cover, p.mm(&w.chem));
        for x in c.held() {
            let (leaf, abs) = (c.leaf(&mm, x), c.leaf(&mm, x) * (1.0 - c.reflectance(&w.chem, &p, x)));
            (row[1], row[2], row[8], row[9]) = (row[1] + c.kg(&mm, x), row[2] + leaf, row[8] + abs, row[9] + abs * c.keep as f64);
        }
        row[7] += c.breaking(&w.chem, &w.cfg, &p, 1.0);
    }
    for b in cell.bodies.iter().filter(|b| b.alive) {
        let m = b.n();
        row[0] += m;
        // (Its mass, then its strength and colour past the capture area, `row[2]`: bodies catch no light.)
        row[1] += b.vis[0] as f64 * m;
        row[3..7].iter_mut().zip(&b.vis[1..]).for_each(|(x, &v)| *x += v as f64 * m);
        row[7] += b.breaking(&w.chem, &w.cfg, 1.0);
    }
    row
}

/// Per cell: the capture area of its bodies per ground area, the light they absorb and the share set aside for their
/// light reactions.
fn caches(w: &mut World) {
    let (n, World { life, mesh, canopy, .. }) = (w.mesh.cells().n(), w);
    life.absorb.resize(n, 0.0);
    life.keep.resize(n, 0.0);
    let area = mesh.cells().area();
    for (i, r) in life.sums.iter().enumerate() {
        canopy[i] = (r[2] / area[i]) as f32;
        life.absorb[i] = if r[2] > 0.0 { (r[8] / r[2]) as f32 } else { 0.0 };
        life.keep[i] = if r[8] > 0.0 { (r[9] / r[8]) as f32 } else { 0.0 };
    }
}

/// The numbers of `Life::sum` per cell.
pub(crate) const ROW: usize = 10;

/// Per cell (`Life::sum`), on the pool.
pub fn sums(w: &mut World) {
    let load = w.life.cells.len();
    w.life.sums = life::run(w, rows_of(w), load, |_| (), |w, d, sums, _| rows(w, d, sums)).concat();
}

/// The yearly moving average of the crossings of each cell (`Cell::crossings`), once a month.
fn traffic(w: &mut World) {
    let World { life, counts, cfg, mesh, .. } = w;
    counts.traffic.resize(mesh.cells().n(), 0.0);
    let a = 1.0 / cfg.months_per_year as f32;
    for (t, c) in counts.traffic.iter_mut().zip(&life.cells) {
        *t += a * (c.crossings * cfg.months_per_year as f32 - *t);
    }
}

/// Update every living body at the month's start (laws.md 0; the bodies of finer cells step again inside the month,
/// `substeps`): `batch_of` every cell.
pub fn batch(w: &mut World) {
    batch_of(w, None);
}

/// Update every living body of the cells `steps` marks (every cell with none). First the parallel pass: the cells are
/// taken out of the world in chunks (`life::run`), and each chunk runs, on a thread of the pool with the feature
/// `threads`, the bodies of those of its cells cell by cell, and in a cell the mature ones first, each in the order
/// they came (`mature_first`), with the world read only. They see the others of their cell as they are when they
/// decide, the parts as they were at the pass's start, and the store as their cell's moves leave it (`store::Moves`);
/// what they do outside their cell or to a part is queued (`life::Entry`), and the lineages and born rows they make are
/// provisional until the chunk ends. Then the serial pass (`life::land`): the moves and litters land and the other
/// entries are applied. Then the merge of alike nodes in each of those cells (`nodes::crowd`), after the moves and
/// births, so a crowded cell ends its step with its nodes of each kind and stage. Last the holders of each lineage and
/// born rows are counted: those left with none die out or are freed.
fn batch_of(w: &mut World, steps: Option<&[bool]>) {
    let stepping = |i: usize| steps.is_none_or(|s| s[i]);
    w.life.compact();
    w.life.recount();
    let bounds = life::bounds(w);
    let mut per = vec![0usize; bounds.len() - 1];
    w.life.living().filter(|(h, _)| stepping(h.cell as usize)).for_each(|(h, _)| per[bounds.partition_point(|&b| b <= h.cell as usize) - 1] += 1);
    let all: usize = per.iter().sum();
    if all == 0 {
        return;
    }
    // The busiest chunk's share of the batch's updates (`Counters::busiest_share`).
    let c = &mut w.counts;
    (c.busiest_share, c.body_batches) = (c.busiest_share + *per.iter().max().unwrap_or(&0) as f64 / all.max(1) as f64, c.body_batches + 1.0);
    life::run(w, per, all, scratch, |w, d, _, s| updates(w, d, &stepping, s));
    life::land(w);
    let lay = Layout::of(&w.chem);
    let load = w.life.cells.iter().enumerate().filter(|(i, c)| stepping(*i) && c.bodies.len() > 1).count();
    life::run(w, vec![(); life::chunks(w)], load, |_| (), |w, d, _, _| {
        // (Fewer than two bodies have nothing to merge.)
        for i in d.cells().filter(|&i| stepping(i)) {
            if d.cell(i).nth(1).is_some() {
                crate::nodes::crowd(w, d, &lay, i);
            }
        }
    });
    life::land(w);
    w.life.recount();
    // The bodies' visible traits from what they hold now (`Chunk::see_changed`), so a world saved after the batch,
    // whose bodies load as seen, reads at the next survey as an unbroken one does.
    let load = w.life.cells.iter().filter(|c| !c.bodies.is_empty()).count();
    life::run(w, vec![(); life::chunks(w)], load, |_| (), |w, d, _, _| d.see_changed(w));
}

/// The scratch of a chunk's updates, made once per thread and used for each chunk it runs.
type Scratch = (LiveTmp, choose::Minds, choose::ActTmp);

/// A new scratch of a chunk's updates.
fn scratch(_: &World) -> Scratch {
    (LiveTmp::default(), choose::Minds::default(), choose::ActTmp::default())
}

/// The updates of the living bodies of the cells of chunk `d` that step (`stepping`), cell by cell: the physics of each
/// of its bodies, the mature ones first (`mature_first`, `live_one`); then the inputs of those with a brain, the sums
/// of the hidden layers of each lineage's bodies together (`choose::Minds::sense`); then each one's decision in that
/// order (`decide`). A cell's stages end before the next cell's begin, so what the chunk queues and the ids it makes
/// come cell by cell, as for any count of chunks.
fn updates(w: &World, d: &mut Chunk, stepping: &(dyn Fn(usize) -> bool + Sync), (tmp, minds, act): &mut Scratch) {
    act.new_chunk();
    for i in d.cells().filter(|&i| stepping(i)) {
        minds.clear();
        let before = d.at(i).bodies.len();
        for slot in mature_first(w, d, i, before) {
            let h = Handle { cell: i as u32, slot: slot as u32 };
            if d[h].alive {
                if let Some(lived) = live_one(w, d, h, tmp) {
                    minds.bodies.push((h, d[h].id, lived));
                }
            }
        }
        // Children who started a body of their own in the cell during these updates decide at once, with the others.
        for slot in before..d.at(i).bodies.len() {
            let h = Handle { cell: i as u32, slot: slot as u32 };
            if d[h].alive && !w.life.brainless(&d[h]) {
                minds.bodies.push((h, d[h].id, crate::body::newborn(w, &d[h])));
            }
        }
        minds.sense(w, d);
        for k in 0..minds.bodies.len() {
            let (h, id, lived) = minds.bodies[k];
            // (A body taken from to death by another since does not decide.)
            if d[h].alive && d[h].id == id {
                let shape = d.shape_of(w, d[h].lineage).unwrap_or_default();
                decide(w, d, h, &lived, minds.of(k, shape.h), act);
            }
        }
    }
}

/// The first `before` slots of cell `i` of chunk `d` in the order their bodies update: the mature ones first, then the
/// young, each in the order they came. So the children a mature body makes join their young node before its update of
/// the same time, which runs them from their birth (`copy::copy`).
fn mature_first(w: &World, d: &Chunk, i: usize, before: usize) -> Vec<usize> {
    let mature = |s: &usize| d[Handle { cell: i as u32, slot: *s as u32 }].adult(&w.chem);
    (0..before).filter(mature).chain((0..before).filter(|s| !mature(s))).collect()
}

/// The physics of the living body at `h` in chunk `d` (`body::live`), or its death. A body with no brain takes no
/// decision: its update ends with its lineage (`lineage::resize`). Returns what its update left for its decision, if
/// it has a brain.
fn live_one(w: &World, d: &mut Chunk, h: Handle, tmp: &mut LiveTmp) -> Option<crate::body::Lived> {
    d.moves.open(w, h.cell as usize, Node::new(Class::Body, h.slot as usize));
    let mut b = std::mem::take(&mut d[h]);
    d.counts.updates += 1;
    let lived = match live(w, d, &mut b, tmp) {
        Err(cause) => {
            die(w, d, h, b, cause);
            return None;
        }
        Ok(lived) => lived,
    };
    let brain = !w.life.brainless(&b);
    if !brain {
        crate::lineage::resize(w, d, &mut b);
    }
    b.stale = true;
    d[h] = b;
    brain.then_some(lived)
}

/// The decision of the body at `h` in chunk `d` from the sums of its hidden layer `z` (`choose::act`), its lineage
/// (`lineage::resize`); or its death, if an action left nothing of a member.
fn decide(w: &World, d: &mut Chunk, h: Handle, lived: &crate::body::Lived, z: &[f32], act: &mut choose::ActTmp) {
    d.moves.open(w, h.cell as usize, Node::new(Class::Body, h.slot as usize));
    let mut b = std::mem::take(&mut d[h]);
    let stands = b.temp(&w.chem);
    choose::act(w, d, &mut b, lived, z, act);
    // An action can leave nothing of a member: it dies at once, before another body deals with it.
    if b.empty_at(b.mass(&w.chem)) {
        return die(w, d, h, b, LACK);
    }
    // What it took and digesting it moved its temperature by fades before its next update (laws.md 6.3).
    crate::body::fade_meal_heat(w, d, &mut b, stands, lived.released);
    crate::lineage::resize(w, d, &mut b);
    b.stale = true;
    d[h] = b;
}
