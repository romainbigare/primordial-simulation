//! The bodies seen from their cells: at each month's start what each cell holds and the shade of its bodies
//! (`survey`), and each month the updates due, in a parallel and a serial phase (`batch`).

use crate::body::{die, live, Layout, LiveTmp, LACK};
use crate::{choose, life::Domain, World};
use crate::store::{Class, Node};

mod draws;
mod faces;
pub(crate) use draws::release;
pub use faces::{Face, Faces, UNCOVERED};

/// A brain perceives the loose amount of a substance in a cell, `a` mol/m², as `LOOSE_SENSE` ln(1 + a).
const LOOSE_SENSE: f32 = 0.2;

/// Temperature, depth of the standing liquid and surface height of a ground cell, as Flow's last step left them (once
/// Flow's working memory is built, `flow::ensure`).
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let (d, z) = w.scratch.view().standing(i);
    [w.t_ground_last(i), d, z]
}

/// At each month's start, one task per domain on the pool (`run`), in each of its cells: what movers broke since the
/// last (`choose::wear`), once a month the merge of alike nodes (`nodes::crowd`), and what the cell holds (`rows`). A death or a merge that lets go of a part queues that in the domain's outbox, applied after
/// the tasks (`outbox::land`, laws.md 0). Then the caches of the cells (`caches`), once a month the traffic and each
/// cell's surroundings. With `reversed`, the domains start in reversed order: only for the order test.
pub fn survey(w: &mut World, reversed: bool) {
    let n = w.mesh.cells().n();
    let monthly = monthly(w);
    let lay = Layout::of(&w.chem);
    // (Movers who pushed through cells where no body is break nothing.)
    w.life.domains.iter_mut().filter(|d| d.count() == 0).for_each(|d| d.pushed.fill(0.0));
    let mut sums = std::mem::take(&mut w.life.sums);
    // (A domain with no bodies and no cover that had none at the last survey has nothing to do.)
    let work: Vec<_> = sums.iter_mut().enumerate().map(|(k, s)| (k, w.life.domains[k].count() + w.life.domains[k].cells.len(), s)).filter(|x| x.2.live || w.life.domains[x.0].lives()).collect();
    let ran: Vec<usize> = work.iter().map(|x| x.0).collect();
    run(w, work, reversed, |_| (), |w, d, s, _| {
        (0..d.cells.len()).for_each(|k| worn(w, d, &lay, k, monthly));
        d.see_changed(w);
        rows(w, d, &mut s.rows);
        s.live = d.lives();
    });
    w.life.sums = sums;
    land_all(w, reversed);
    caches(w, &ran);
    if monthly && w.time > 0 {
        traffic(w);
    }
    if monthly || w.life.env.len() != n || w.life.seen.len() != n * w.chem.ns() {
        // From Flow's state rebuilt in full, as at a month's start, so a resumed run reads what an unbroken one does.
        crate::flow::state(w, true);
        w.life.env = (0..n).map(|i| env_of(w, i)).collect();
        let (ns, area) = (w.chem.ns(), w.mesh.cells().area());
        w.life.seen = (0..n * ns).map(|x| LOOSE_SENSE * crate::brain::ln(1.0 + (w.store.amounts(Class::Ground)[(x % ns) * n + x / ns] / area[x / ns]) as f32)).collect();
    }
    choose::cells(w);
}

/// Once a month, after Flow's state is built in full: the cover's month (`cover::step`) in each domain that holds any,
/// one task per domain on the pool, its moves booked as a batch's are; then its seeds land (`cover::land`).
pub fn cover(w: &mut World) {
    let work: Vec<_> = (0..w.life.domains.len()).filter(|&k| w.life.domains[k].cover.any()).map(|k| (k, w.life.domains[k].cells.len(), ())).collect();
    run(w, work, false, |_| (), |w, d, _, _| crate::cover::step(w, d));
    land_all(w, false);
    crate::cover::land(w);
}

/// Whether it is a month's start: the survey merges nodes, adds up the traffic and rebuilds Flow's state in full.
fn monthly(w: &World) -> bool {
    w.time.is_multiple_of(w.cfg.calendar().month)
}

/// Cell `k` of domain `d`: what movers broke since the last survey (`choose::wear`), and once a month the merge of alike
/// nodes (`nodes::crowd`).
fn worn(w: &World, d: &mut Domain, lay: &Layout, k: usize, monthly: bool) {
    let (i, e) = (d.cells[k] as usize, std::mem::take(&mut d.pushed[k]));
    if e > 0.0 {
        choose::wear(w, d, i, e);
    }
    // (Fewer than two bodies have nothing to merge.)
    if monthly && d.at[k].len() > 1 {
        crate::nodes::crowd(w, d, lay, i);
    }
}

/// The numbers of the cells of domain `d` (`Life::sum`), from its cover and its bodies in the order they came: their
/// members, then their visible traits summed, then the energy to break them; then the cover's capture area times the
/// share of light it absorbs, and that times the share set aside for its light reaction.
fn rows(w: &World, d: &Domain, rows: &mut [[f64; ROW]]) {
    let parts = crate::cover::Parts::of(&w.chem);
    for (k, row) in rows.iter_mut().enumerate() {
        *row = [0.0; ROW];
        // Its cover, as one more body of no members: its mass, capture area and energy to break, and the light it absorbs
        // at its mean shade (its kinds share it out among them, `cover::step`).
        if let Some(p) = parts.as_ref() {
            let (c, mm) = (&d.cover, p.mm(&w.chem));
            for x in crate::cover::Cover::slots(k).filter(|&x| c.root[x] != crate::cover::NO_ROOT) {
                let (leaf, abs) = (c.leaf(&mm, x), c.leaf(&mm, x) * (1.0 - c.reflectance(&w.chem, p, x)));
                (row[1], row[2], row[8], row[9]) = (row[1] + c.kg(&mm, x), row[2] + leaf, row[8] + abs, row[9] + abs * c.keep[k] as f64);
            }
            row[7] += c.breaking(&w.chem, &w.cfg, p, k, 1.0);
        }
        for b in d.at[k].iter().map(|&h| &d[h]).filter(|b| b.alive) {
            let m = b.n();
            row[0] += m;
            row[1..7].iter_mut().zip(&b.vis).for_each(|(x, &v)| *x += v as f64 * m);
            row[7] += b.breaking(&w.chem, &w.cfg, 1.0);
        }
    }
}

/// Per cell of the domains `ran`: their capture area per ground area, the light they absorb and the share set aside for
/// their light reactions.
fn caches(w: &mut World, ran: &[usize]) {
    let (n, World { life, mesh, canopy, .. }) = (w.mesh.cells().n(), w);
    life.absorb.resize(n, 0.0);
    life.keep.resize(n, 0.0);
    let area = mesh.cells().area();
    for &k in ran {
        for (&i, r) in life.domains[k].cells.iter().zip(&life.sums[k].rows) {
            let i = i as usize;
            canopy[i] = (r[2] / area[i]) as f32;
            life.absorb[i] = if r[2] > 0.0 { (r[8] / r[2]) as f32 } else { 0.0 };
            life.keep[i] = if r[8] > 0.0 { (r[9] / r[8]) as f32 } else { 0.0 };
        }
    }
}

/// The numbers of `Life::sum` per cell.
pub(crate) const ROW: usize = 10;

/// Per cell (`Life::sum`), for every domain, on the pool (`run`).
pub fn sums(w: &mut World) {
    let mut sums = std::mem::take(&mut w.life.sums);
    let work = sums.iter_mut().enumerate().map(|(k, s)| (k, w.life.domains[k].count(), s)).collect();
    run(w, work, false, |_| (), |w, d, s, _| rows(w, d, &mut s.rows));
    w.life.sums = sums;
}

/// The yearly moving average of the crossings of each cell (`Domain::crossings`, kept by its domain), once a month.
fn traffic(w: &mut World) {
    let World { life, counts, cfg, mesh, .. } = w;
    counts.traffic.resize(mesh.cells().n(), 0.0);
    let a = 1.0 / cfg.months_per_year as f32;
    for d in &mut life.domains {
        for (&i, c) in d.cells.iter().zip(&mut d.crossings) {
            let t = &mut counts.traffic[i as usize];
            *t += a * (*c * cfg.months_per_year as f32 - *t);
            *c = 0.0;
        }
    }
}

/// Update the bodies due in this batch, all at its first day (laws.md 0). First the parallel phase: the domains are taken
/// out of the world (`life::Domain`), and each runs, on a thread of the pool with the feature `threads` (`on_domains`),
/// its bodies that hold no part and are inside none, cell by cell, and in a cell by id (an order that does not depend on
/// where bodies are stored, and keeps memory reads close together), with the world read only. They see each other as
/// they were at the batch's start (`Faces`) and the store as their domain's moves leave it (`store::Moves`); what they
/// do outside their domain goes to its outbox (`outbox`), and the lineages and born rows they make are provisional. Then
/// the domains back, the world settles (`World::settle`: events, counters, those ids made global), the moves and
/// litters land per domain on the pool while the moves gathered are booked beside them, array by array, and the other
/// entries are applied one by one (`outbox::land`); then the bodies that hold or are inside a part, each a serial step
/// on its domain (`World::in_domain`) with its outbox applied after it, and last the changes of holders are added: the
/// lineages and born rows left with no holder die out or are freed.
/// With `reversed`, the domains of the parallel phase start in reversed order: only for the test that this order (and
/// which thread runs a domain) changes nothing.
pub fn batch(w: &mut World, reversed: bool) {
    let life = &w.life;
    let mut list: Vec<(usize, usize, u64, bool)> = life.alive().map(|b| (life.domain_of(b.cell as usize), b.cell as usize, b.id, b.hold.is_some() || b.inside.is_some())).collect();
    if list.is_empty() {
        return;
    }
    list.sort_unstable();
    // The busiest domain's share of the batch's updates (`Counters::busiest_share`).
    let most = list.chunk_by(|a, b| a.0 == b.0).map(|d| d.len()).max().unwrap_or(0);
    let c = &mut w.counts;
    (c.busiest_share, c.body_batches) = (c.busiest_share + most as f64 / list.len().max(1) as f64, c.body_batches + 1.0);
    faces::make(w);
    let (serial, list): (Vec<_>, Vec<_>) = list.into_iter().partition(|x| x.3);
    w.life.batch = true;
    let runs = list.chunk_by(|a, b| a.0 == b.0).map(|run| (run[0].0, run.len(), run)).collect();
    on_domains(w, runs, reversed, scratch, |w, d, run, s| {
        run.iter().for_each(|&(_, cell, id, _)| update(w, d, (cell, id), s));
    });
    let mut scratch = scratch(w);
    for (_, cell, id, _) in serial {
        w.in_domain(cell, |w, d| update(w, d, (cell, id), &mut scratch));
        crate::outbox::deliver(w);
    }
    w.life.batch = false;
    w.settle(0..w.life.domains.len());
    w.life.settle_moved();
    // The bodies' visible traits from what they hold now (`Domain::see_changed`), so a world saved after the batch,
    // whose bodies load as seen, reads at the next survey as an unbroken one does.
    let work = (0..w.life.domains.len()).map(|k| (k, w.life.domains[k].count(), ())).filter(|x| x.1 > 0).collect();
    run(w, work, reversed, |_| (), |w, d, _, _| d.see_changed(w));
}

/// Run `f` on the domains of `work` (`run`), each ending by taking its bodies that move out of its storage
/// (`Domain::post`); then the domains back, the world settled (`World::settle`) and the outboxes applied
/// (`outbox::land`), where the moves gathered are booked beside the landing.
fn on_domains<T: Send + Sync, S>(w: &mut World, work: Vec<(usize, usize, T)>, reversed: bool, init: impl Fn(&World) -> S + Sync + Send, f: impl Fn(&World, &mut Domain, &T, &mut S) + Sync + Send) {
    run(w, work, reversed, init, |w, d, t, s| {
        f(w, d, t, s);
        d.post();
    });
    land_all(w, reversed);
}

/// The domains back from a run: the world settled (`World::settle`) and the outboxes applied (`outbox::land`), where
/// the moves gathered are booked beside the landing.
fn land_all(w: &mut World, reversed: bool) {
    w.settle(0..w.life.domains.len());
    crate::outbox::land(w, reversed);
}

/// Run `f` on the domains of `work` (each with its count of work and what it is given, which `f` may change), marked as run (`Life::mark`)
/// and taken out of the world (`life::Domain`), with a scratch made by `init` once per thread, by the one runner of
/// per-domain work (`par::longest_first`); then the domains back. Each domain writes only its own numbers (CLAUDE.md
/// 3.6).
pub(crate) fn run<T: Send, S>(w: &mut World, work: Vec<(usize, usize, T)>, reversed: bool, init: impl Fn(&World) -> S + Sync + Send, f: impl Fn(&World, &mut Domain, &mut T, &mut S) + Sync + Send) {
    work.iter().for_each(|x| w.life.mark(x.0));
    let mut domains = std::mem::take(&mut w.life.domains);
    let mut free: Vec<Option<&mut Domain>> = domains.iter_mut().map(Some).collect();
    let runs = work.into_iter().map(|(k, n, t)| (k, n, (free[k].take().expect("once a domain"), t))).collect();
    crate::par::longest_first(runs, reversed, || init(w), |s, (d, t)| f(w, d, t, s));
    w.life.domains = domains;
}

/// The scratch of a body's update, made once per thread and used for each update it runs.
type Scratch = (LiveTmp, choose::ActTmp);

/// A new scratch of a body's update.
fn scratch(_: &World) -> Scratch {
    (LiveTmp::default(), choose::ActTmp::default())
}

/// The update of body `id` of cell `cell` in domain `d`, if it is still stored there (it may have died or joined
/// another body since): its physics, decision, lineage and split; or its death.
fn update(w: &World, d: &mut Domain, (cell, id): (usize, u64), (tmp, act): &mut Scratch) {
    let Some(s) = d.find(id) else { return };
    d.moves.open(w, cell, Node::new(Class::Body, s.slot as usize));
    let mut b = std::mem::take(&mut d[s]);
    d.counts.updates += 1;
    let lived = match live(w, d, &mut b, tmp) {
        Err(cause) => return die(w, d, s, b, cause),
        Ok(lived) => lived,
    };
    // A body with no brain takes no decision: its update is its physics only.
    if !w.life.brainless(&b) {
        choose::act(w, d, &mut b, &lived, act);
        // An action can leave nothing of a member: it dies at once, before another body deals with it.
        if b.empty_at(b.mass(&w.chem)) {
            return die(w, d, s, b, LACK);
        }
    }
    crate::lineage::resize(w, d, &mut b);
    b.stale = true;
    d[s] = b;
}
