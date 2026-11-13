//! The bodies seen from their cells, the contract's first taker (`contract`): at each month's start what movers broke
//! (`press`), what the bodies add to what stands on each cell and their own numbers (`sums`), and what they see
//! (`features`); and every body's update at the month's start, in a parallel and a serial pass (`batch`; the steps,
//! `steps`).

use crate::body::{die, live, perish, Layout, Lived, LiveTmp};
use crate::life::{self, Chunk, Handle, Of, LACK};
use crate::store::{Class, Node};
use crate::{choose, World};

mod steps;
pub use steps::Grid;

/// At each month's start, first: what movers broke since the last month's start breaks (`choose::wear`), in each cell
/// one task per chunk on the pool (`life::run`), and the bodies' visible traits that changed are worked out again
/// (`Chunk::see_changed`). A death that lets go of a part queues that, for the serial pass after (`life::land`, laws.md
/// 0). With no body and nothing pushed there is nothing to do.
pub fn press(w: &mut World) {
    w.life.compact();
    if !w.life.cells.iter().any(|c| c.pushed > 0.0 || !c.bodies.is_empty()) {
        return;
    }
    let load = w.life.cells.iter().filter(|c| !c.bodies.is_empty()).count();
    life::run(w, Of::All, vec![(); life::chunks(w)], load, |_| (), |w, d, _, _| {
        for i in d.cells() {
            let e = std::mem::take(&mut d.at_mut(i).pushed);
            if e > 0.0 && (!d.at(i).bodies.is_empty() || d.at(i).cover.any()) {
                choose::wear(w, d, i, e);
            }
        }
        d.see_changed(w);
    });
}

/// After the cover's part of what stands on each cell (`contract::stands`): the bodies' part, each in the order they
/// came (their mass, `Body::vis`, and the energy to break them, `Stand::add`), and their own numbers per cell (`ROW`:
/// their members, and their strength summed by members).
pub fn sums(w: &mut World) {
    let World { life, chem, cfg, .. } = w;
    for (i, c) in life.cells.iter().enumerate() {
        let mut row = [0.0; ROW];
        for b in c.bodies.iter().filter(|b| b.alive) {
            let m = b.n();
            row[0] += m;
            row[1] += b.vis[1] as f64 * m;
            life.stands[i].add(b.vis[0] as f64 * m, b.breaking(chem, cfg, 1.0));
        }
        life.sums[i] = row;
    }
}

/// After the surroundings (`contract::surroundings`): once a month the traffic, and the cells' features
/// (`choose::cells`), which read the members that moved in since the last month's start; that count starts again.
/// Only the decisions of bodies read the features: with no body living they are not made. A body comes only from one
/// living or from a placement between months (`worldgen::schedule`), so the start of its first month makes them before
/// it decides.
pub fn features(w: &mut World) {
    if w.time.is_multiple_of(w.cfg.calendar().month) && w.time > 0 {
        traffic(w);
    }
    if w.life.alive().next().is_some() {
        choose::cells(w);
    } else {
        w.life.feat.clear();
    }
    w.life.cells.iter_mut().for_each(|c| c.crossings = 0.0);
}

/// The bodies' part of what stands on each cell and their own numbers (`sums`), then what the press queued lands
/// (`life::land`), before Flow's state is built from the store.
pub fn stand(w: &mut World) {
    sums(w);
    life::land(w);
}

/// The cells `of` whose life `holds` it, in order.
fn cells_where(w: &World, of: Of, holds: impl Fn(&life::Cell) -> bool) -> Vec<u32> {
    cells_of(w, of).filter(|&i| holds(&w.life.cells[i])).map(|i| i as u32).collect()
}

/// The cells `of`, by index, in order.
fn cells_of<'a>(w: &World, of: Of<'a>) -> Box<dyn Iterator<Item = usize> + 'a> {
    match of {
        Of::All => Box::new(0..w.mesh.cells().n()),
        Of::Only(list) => Box::new(list.iter().map(|&i| i as usize)),
    }
}

/// The numbers of the bodies of a cell (`Life::sum`).
pub(crate) const ROW: usize = 2;

/// The yearly moving average of the crossings of each cell (`Cell::crossings`), once a month.
fn traffic(w: &mut World) {
    let World { life, counts, cfg, mesh, .. } = w;
    counts.traffic.resize(mesh.cells().n(), 0.0);
    let a = 1.0 / cfg.months_per_year as f32;
    for (t, c) in counts.traffic.iter_mut().zip(&life.cells) {
        *t += a * (c.crossings * cfg.months_per_year as f32 - *t);
    }
}

/// Update every living body at the month's start (laws.md 0): `batch_of` every cell.
pub fn batch(w: &mut World) {
    batch_of(w, Of::All);
}

/// Update every living body of the cells `of`. First the parallel pass: those cells are taken out of the world in
/// chunks (`life::run`), and each chunk runs, on a thread of the pool with the feature `threads`, the bodies of its
/// cells cell by cell, and in a cell the mature ones first, each in the order they came (`mature_first`), with the
/// world read only. They see the others of their cell as they are when they decide, the parts as they were at the
/// pass's start, and the store as their cell's moves leave it (`store::Moves`); what they do outside their cell or to a
/// part is queued (`life::Entry`), and the lineages and born rows they make are provisional until the chunk ends. Then
/// the serial pass (`life::land`): the moves and litters land and the other entries are applied; then the bodies that
/// landed since such a pass last ran (`Touched::take_landed`) run their physics on to their cell's next step, a parallel
/// pass, and what that queues lands. Then the merge of alike nodes in each of the cells `of` (`nodes::crowd`), after the
/// moves and births, so a crowded cell ends its step with its nodes of each kind and stage. Last the holders of each
/// lineage and born rows are counted: those left with none die out or are freed; and the visible traits of the bodies
/// of the cells the batch held are worked out again, and on some cells (`Of::Only`) those cells' numbers, which it
/// returns.
fn batch_of(w: &mut World, of: Of) {
    w.life.compact();
    w.life.recount();
    let bounds = life::bounds(w);
    let mut per = vec![0usize; bounds.len() - 1];
    cells_of(w, of).for_each(|i| per[bounds.partition_point(|&b| b <= i) - 1] += w.life.in_cell(i));
    let all: usize = per.iter().sum();
    if all == 0 {
        return;
    }
    // The busiest chunk's share of the batch's updates (`Counters::busiest_share`).
    let c = &mut w.counts;
    (c.busiest_share, c.body_batches) = (c.busiest_share + *per.iter().max().unwrap_or(&0) as f64 / all.max(1) as f64, c.body_batches + 1.0);
    life::run(w, of, per, all, scratch, |w, d, _, s| updates(w, d, s));
    life::land(w);
    // The bodies that landed in a cell in the serial pass (movers, litters) run their physics on to its next step, as
    // the bodies there did (`physics`); what that queues lands after. (A body whose physics reached it does nothing.)
    let on = of;
    let load = cells_of(w, on).filter(|&i| !w.life.cells[i].bodies.is_empty()).count();
    life::run(w, on, vec![(); life::chunks(w)], load, scratch, |w, d, _, (tmp, _, _, subs)| {
        for i in d.cells() {
            physics(w, d, i, (tmp, subs), w.step_grid().after(w.time));
        }
    });
    life::land(w);
    // (Fewer than two bodies have nothing to merge.)
    let (lay, crowded) = (Layout::of(&w.chem), cells_where(w, of, |c| c.bodies.iter().filter(|b| b.alive).nth(1).is_some()));
    life::run(w, Of::Only(&crowded), vec![(); life::chunks(w)], crowded.len(), |_| (), |w, d, _, _| {
        for i in d.cells() {
            crate::nodes::crowd(w, d, &lay, i);
        }
    });
    life::land(w);
    w.life.recount();
    // The bodies' visible traits from what they hold now (`Chunk::see_changed`), so a world saved after the batch,
    // whose bodies load as seen, reads at the next survey as an unbroken one does: in the cells the batch held, where
    // alone they changed.
    let held = w.life.touched.since_compact().unwrap_or_else(|| cells_where(w, Of::All, |c| c.bodies.iter().any(|b| b.alive && b.stale)));
    life::run(w, Of::Only(&held), vec![(); life::chunks(w)], held.len(), |_| (), |w, d, _, _| d.see_changed(w));
}

/// The scratch of a chunk's updates, made once per thread and used for each chunk it runs.
type Scratch = (LiveTmp, choose::Minds, choose::ActTmp, Subs);

/// A new scratch of a chunk's updates.
fn scratch(_: &World) -> Scratch {
    (LiveTmp::default(), choose::Minds::default(), choose::ActTmp::default(), Subs::default())
}

/// Per slot of the cell whose bodies run sub-steps (`physics`): whether it ate up to the start of the sub-step; and the
/// slots in the order they run (`mature_first`).
#[derive(Default)]
struct Subs {
    fed: Vec<bool>,
    order: Vec<usize>,
}

/// The updates of the living bodies of the cells of chunk `d`, cell by cell (laws.md 0): first
/// the physics to now of those whose physics is behind (they came into the cell since its last step, `physics`); then
/// the inputs of those with a brain, the sums of the hidden layers of each lineage's bodies together
/// (`choose::Minds::sense`); then each one's decision, the mature ones first, each in the order they came (`decide`);
/// then the physics of every body over its step ahead, to the cell's next step (`physics`), and a body with no brain,
/// which takes no decision, ends its update with its lineage (`lineage::resize`). A cell's stages end before the next
/// cell's begin, so what the chunk queues and the ids it makes come cell by cell, as for any count of chunks.
fn updates(w: &World, d: &mut Chunk, (tmp, minds, act, subs): &mut Scratch) {
    act.new_chunk();
    for i in d.cells() {
        physics(w, d, i, (tmp, subs), w.time);
        minds.clear();
        mature_first(w, d, i, d.at(i).bodies.len(), &mut subs.order);
        for &slot in &subs.order {
            let h = Handle { cell: i as u32, slot: slot as u32 };
            d.counts.updates += d[h].alive as u64;
            if d[h].alive && !w.life.brainless(&d[h]) {
                // (A body with no physics behind it since it was placed, a founder, has its spare now.)
                let lived = if d[h].last > d[h].decided { d[h].lived } else { crate::body::newborn(w, &d[h]) };
                minds.bodies.push((h, d[h].id, lived));
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
        physics(w, d, i, (tmp, subs), w.step_grid().after(w.time));
        for h in d.cell(i).filter(|&h| w.life.brainless(&d[h])).collect::<Vec<_>>() {
            let mut b = std::mem::take(&mut d[h]);
            crate::lineage::resize(w, d, &mut b);
            d[h] = b;
        }
    }
}

/// The physics of the bodies of cell `i` of chunk `d` to `to` s (laws.md 0, 6), in sub-steps of its physiological step
/// (`World::phys_grid`) on the times that step back from `to`, from the earliest a body's physics reached
/// (`Body::last`), so a body's physics runs alike in every cell: at each, every living body of the cell not leaving it
/// whose physics reached less than the sub-step's end, the mature ones first, each in the order they came
/// (`mature_first`: the children a mature body makes join their young node before its sub-step of the same time, which
/// runs them from their birth, `copy::copy`), runs it (`live_one`). Before any runs it, each that ran the sub-step
/// before, or was placed during it (a child that starts a body of its own, fed to its end at birth, `copy::feed`),
/// takes its meal for the sub-step (`meal_one`), in the same order: so at every sub-step the bodies of a cell eat before
/// any lives it, as at a decision, where all decide and eat before the physics runs, and the children born in the
/// sub-step, fed from their birth, join a young node that has eaten for it. One whose physics starts here ate for its
/// first sub-step when it decided or landed.
fn physics(w: &World, d: &mut Chunk, i: usize, (tmp, subs): (&mut LiveTmp, &mut Subs), to: u64) {
    let (grid, day) = (w.phys_grid(), w.cfg.calendar().day as f64);
    let Subs { fed, order } = subs;
    let end = to as f64 / day;
    let first = d.cell(i).filter(|&h| !d[h].leaving).map(|h| d[h].last).fold(end, f64::min);
    let last = grid.by(to);
    let parts = last - grid.by(to.saturating_sub(((end - first) * day).round() as u64));
    fed.clear();
    fed.resize(d.at(i).bodies.len(), false);
    for k in 1..=parts {
        let (from, at) = (grid.at(last - (parts - k) - 1), grid.at(last - (parts - k)));
        let held = d.at(i).bodies.len();
        fed.resize(held, true);
        mature_first(w, d, i, held, order);
        let runs = |d: &Chunk, h: Handle| d[h].alive && !d[h].leaving && d[h].last < at as f64 / day;
        for &slot in order.iter() {
            let h = Handle { cell: i as u32, slot: slot as u32 };
            if runs(d, h) && fed[slot] {
                meal_one(w, d, h, (at - from) as f64 / day);
            }
        }
        for &slot in order.iter() {
            let h = Handle { cell: i as u32, slot: slot as u32 };
            if runs(d, h) {
                fed[slot] = true;
                live_one(w, d, h, tmp, at);
            }
        }
    }
}

/// The first `before` slots of cell `i` of chunk `d`, into `order`, in the order their bodies run: the mature ones
/// first, then the young, each in the order they came.
fn mature_first(w: &World, d: &Chunk, i: usize, before: usize, order: &mut Vec<usize>) {
    let mature = |s: &usize| d[Handle { cell: i as u32, slot: *s as u32 }].adult(&w.chem);
    order.clear();
    order.extend((0..before).filter(mature).chain((0..before).filter(|s| !mature(s))));
}

/// The meal of the living body at `h` in chunk `d` for the `days` d of a sub-step (`choose::meal`).
fn meal_one(w: &World, d: &mut Chunk, h: Handle, days: f64) {
    d.moves.open(w, h.cell as usize, Node::new(Class::Body, h.slot as usize));
    let mut b = std::mem::take(&mut d[h]);
    choose::meal(w, d, &mut b, days);
    d[h] = b;
}

/// A sub-step of the living body at `h` in chunk `d` to `at` s, the same in every cell: its physics (`body::live`), or
/// its death, dated by its sub-step. It leaves what the sub-step left for the body's next decision (`Body::lived`).
fn live_one(w: &World, d: &mut Chunk, h: Handle, tmp: &mut LiveTmp, at: u64) {
    d.moves.open(w, h.cell as usize, Node::new(Class::Body, h.slot as usize));
    let mut b = std::mem::take(&mut d[h]);
    match live(w, d, &mut b, tmp, at) {
        Err(cause) => {
            perish(w, d, &mut b, (cause, tmp.day));
            d.remove(h, &b);
        }
        Ok(lived) => {
            (b.lived, b.stale) = (lived, true);
            d[h] = b;
        }
    }
}

/// The decision of the body at `h` in chunk `d` from the sums of its hidden layer `z` and what its physics since its
/// last decision left (`lived`; `choose::act`), its lineage (`lineage::resize`); or its death, if an action left nothing
/// of a member.
fn decide(w: &World, d: &mut Chunk, h: Handle, lived: &Lived, z: &[f32], act: &mut choose::ActTmp) {
    d.moves.open(w, h.cell as usize, Node::new(Class::Body, h.slot as usize));
    let mut b = std::mem::take(&mut d[h]);
    let (stands, days) = (b.temp(&w.chem), w.days_now() - b.decided);
    // (Its parts that act apart count their time from this decision too.)
    b.decided = w.days_now();
    choose::act(w, d, &mut b, (lived, days), z, act);
    // An action can leave nothing of a member: it dies at once, before another body deals with it.
    if b.empty_at(b.mass(&w.chem)) {
        return die(w, d, h, b, LACK);
    }
    // What it took and digesting it moved its temperature by fades before its physics (laws.md 6.3).
    crate::body::fade_meal_heat(w, d, &mut b, stands, lived.released);
    crate::lineage::resize(w, d, &mut b);
    b.stale = true;
    d[h] = b;
}
