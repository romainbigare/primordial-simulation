//! The bodies seen from their cells: at each month's start what each cell holds and the shade of its bodies
//! (`survey`), and every body's update, in a parallel and a serial pass (`batch`), at the month's start and again inside
//! it in the cells finer than a square (`steps`).

use crate::body::{die, live, perish, Layout, Lived, LiveTmp, LACK};
use crate::life::{self, Chunk, Handle, Of};
use crate::store::{Class, Node};
use crate::{choose, World};

mod draws;
mod steps;
pub(crate) use draws::release;
pub use steps::{substeps, Grid};


/// Temperature, depth of the standing liquid and surface height of a ground cell as life stands on it (a cell of
/// parts' land, `flow::parts`), as Flow's last step left them (once Flow's working memory is built, `flow::ensure`).
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let (d, z) = w.scratch.standing_land(i);
    [w.t_ground_last(i), d, z]
}

/// At each month's start: what movers broke and what each cell holds (`break_and_sum`). Then the caches of the cells
/// (`caches`), once a month the traffic and each cell's surroundings, and the cells' features (`choose::cells`); both
/// read the members that moved in since the last month's survey, and that count starts again.
pub fn survey(w: &mut World) {
    let n = w.mesh.cells().n();
    let monthly = monthly(w);
    break_and_sum(w, Of::All, &|_| true);
    life::land(w);
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

/// One task per chunk of the cells `of` on the pool (`life::run`): in each that `steps`, what movers broke since its last
/// survey (`choose::wear`); then in every cell the visible traits that changed and what it holds (`rows`, `Life::sum`).
/// A death that lets go of a part queues that, for the serial pass after (`life::land`, laws.md 0).
fn break_and_sum(w: &mut World, of: Of, steps: &(dyn Fn(usize) -> bool + Sync)) {
    w.life.compact();
    let load = cells_of(w, of).filter(|&i| !w.life.cells[i].bodies.is_empty()).count();
    let sums = life::run(w, of, rows_of(w, of), load, |_| (), |w, d, sums, _| {
        for i in d.cells().filter(|&i| steps(i)) {
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
    match of {
        Of::All => w.life.sums = sums.concat(),
        Of::Only(list) => list.iter().zip(sums.concat()).for_each(|(&i, r)| w.life.sums[i as usize] = r),
    }
}

/// Once a month, after Flow's state is built in full: the cover's month (`cover::step`) in each cell that holds any,
/// one task per chunk on the pool; then its seeds land (`cover::land`).
pub fn cover(w: &mut World) {
    let load = w.life.cells.iter().filter(|c| c.cover.any()).count();
    life::run(w, Of::All, vec![(); life::chunks(w)], load, |_| (), |w, d, _, _| crate::cover::step(w, d));
    life::land(w);
    crate::cover::land(w);
}

/// Whether it is a month's start: the survey adds up the traffic and rebuilds Flow's state in full.
fn monthly(w: &World) -> bool {
    w.time.is_multiple_of(w.cfg.calendar().month)
}

/// Per chunk, a row of zeros for each of its cells of `of` (`rows`).
fn rows_of(w: &World, of: Of) -> Vec<Vec<[f64; ROW]>> {
    let b = life::bounds(w);
    let mut per = vec![0; b.len() - 1];
    cells_of(w, of).for_each(|i| per[b.partition_point(|&x| x <= i) - 1] += 1);
    per.into_iter().map(|k| vec![[0.0; ROW]; k]).collect()
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
    w.life.sums = life::run(w, Of::All, rows_of(w, Of::All), load, |_| (), |w, d, sums, _| rows(w, d, sums)).concat();
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
fn batch_of(w: &mut World, of: Of) -> Vec<u32> {
    w.life.compact();
    w.life.recount();
    let bounds = life::bounds(w);
    let mut per = vec![0usize; bounds.len() - 1];
    cells_of(w, of).for_each(|i| per[bounds.partition_point(|&b| b <= i) - 1] += w.life.in_cell(i));
    let all: usize = per.iter().sum();
    if all == 0 {
        return Vec::new();
    }
    // The busiest chunk's share of the batch's updates (`Counters::busiest_share`).
    let c = &mut w.counts;
    (c.busiest_share, c.body_batches) = (c.busiest_share + *per.iter().max().unwrap_or(&0) as f64 / all.max(1) as f64, c.body_batches + 1.0);
    life::run(w, of, per, all, scratch, |w, d, _, s| updates(w, d, s));
    life::land(w);
    // The bodies that landed in a cell in the serial pass (movers, litters) run their physics on to its next step, as
    // the bodies there did (`physics`); what that queues lands after. (A body whose physics reached it does nothing.)
    let every = matches!(of, Of::All);
    let landed = w.life.touched.take_landed(every);
    let on = if every { Of::All } else { Of::Only(&landed) };
    let load = cells_of(w, on).filter(|&i| !w.life.cells[i].bodies.is_empty()).count();
    life::run(w, on, vec![(); life::chunks(w)], load, scratch, |w, d, _, (tmp, _, _, subs)| {
        for i in d.cells() {
            physics(w, d, i, (tmp, subs), w.step_grid(i).after(w.time));
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
    // On some cells, the numbers of those cells too (`Life::sum`), which the survey of the next sub-step reads as they
    // are now where no pass held the cell since (`substeps`).
    let Some(held) = w.life.touched.since_compact() else {
        let stale = cells_where(w, Of::All, |c| c.bodies.iter().any(|b| b.alive && b.stale));
        life::run(w, Of::Only(&stale), vec![(); life::chunks(w)], stale.len(), |_| (), |w, d, _, _| d.see_changed(w));
        return Vec::new();
    };
    let sums = life::run(w, Of::Only(&held), rows_of(w, Of::Only(&held)), held.len(), |_| (), |w, d, sums, _| {
        d.see_changed(w);
        rows(w, d, sums);
    });
    held.iter().zip(sums.concat()).for_each(|(&i, r)| w.life.sums[i as usize] = r);
    held
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
        physics(w, d, i, (tmp, subs), w.step_grid(i).after(w.time));
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
    let (grid, day) = (w.phys_grid(i), w.cfg.calendar().day as f64);
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
    if d.cell(i).any(|h| !d[h].leaving && d[h].last < end) {
        d.behind(i);
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
