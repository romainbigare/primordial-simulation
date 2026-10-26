//! The bodies seen from their cells: before each physics step what each cell holds and the shade of its bodies
//! (`survey`), and every batch of days the updates due, in a parallel and a serial phase (`batch`).

use crate::body::{die, live, Body, Layout, LiveTmp, LACK};
use crate::{change::Reactor, choose, copy::Grown, life::{Domain, Handle}, math::mean_decay, World};
use crate::store::{Class, Node};

mod draws;
mod faces;
pub use draws::Draw;
pub(crate) use draws::Draws;
pub use faces::{Face, Faces, UNCOVERED};

/// A brain perceives the loose amount of a substance in a cell, `a` mol/m², as `LOOSE_SENSE` ln(1 + a).
const LOOSE_SENSE: f32 = 0.2;

/// Temperature, depth of the standing liquid and surface height of a ground cell, as Flow's last step left them (once
/// Flow's working memory is built, `flow::ensure`).
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let (d, z) = w.scratch.view().standing(i);
    [w.t_ground_last(i), d, z]
}

/// Before each physics step, one task per domain on the pool (`run`), in each of its cells: what movers broke since the
/// last (`choose::wear`), once a month the merge of alike nodes (`nodes::crowd`), the light shared out (`shade`) and what
/// the cell holds (`rows`). A death or a merge that lets go of a part queues that in the domain's outbox, applied after
/// the tasks (`outbox::land`, laws.md 0). Then the caches of the cells (`caches`), once a month the traffic and each
/// cell's surroundings. With `reversed`, the domains start in reversed order: only for the order test.
pub fn survey(w: &mut World, reversed: bool) {
    let n = w.mesh.cells().n();
    let monthly = monthly(w);
    let lay = Layout::of(&w.chem);
    // (Movers who pushed through cells where no body is break nothing.)
    w.life.domains.iter_mut().filter(|d| d.count() == 0).for_each(|d| d.pushed.fill(0.0));
    let mut sums = std::mem::take(&mut w.life.sums);
    let work: Vec<_> = sums.iter_mut().enumerate().map(|(k, s)| (k, w.life.domains[k].count(), s)).filter(|x| x.1 > 0 || x.2.live).collect();
    let ran: Vec<usize> = work.iter().map(|x| x.0).collect();
    run(w, work, reversed, |_| Vec::new(), |w, d, s, l| {
        (0..d.cells.len()).for_each(|k| worn(w, d, &lay, k, monthly));
        (0..d.cells.len()).for_each(|c| shade(w, d, c, l));
        rows(w, d, &mut s.rows);
        s.live = d.count() > 0;
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

/// The numbers of the cells of domain `d` (`Life::sum`), from its bodies in the order they came: their members, then
/// their visible traits summed, then the energy to break them; then their capture area times their shade, and that
/// times the share they set aside for their light reactions.
fn rows(w: &World, d: &Domain, rows: &mut [[f64; ROW]]) {
    for (k, row) in rows.iter_mut().enumerate() {
        *row = [0.0; ROW];
        for b in d.at[k].iter().map(|&h| &d[h]).filter(|b| b.alive) {
            let (m, a) = (b.n(), light_area(b) * b.shade);
            row[0] += m;
            row[1..7].iter_mut().zip(&b.vis).for_each(|(x, &v)| *x += v as f64 * m);
            (row[7], row[8], row[9]) = (row[7] + b.breaking(&w.chem, &w.cfg, 1.0), row[8] + a, row[9] + a * b.keep as f64);
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

/// The capture area of `n` members who each show `vis` of it (`Body::vis`), m².
pub(super) fn capture(vis: f32, n: f64) -> f64 {
    vis as f64 * n
}

/// The capture area of body `b`'s members times the share of light they absorb, m².
fn light_area(b: &Body) -> f64 {
    capture(b.vis[1], b.n()) * (1.0 - b.reflectance())
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

/// Light passes the bodies of cell `c` of domain `d` from the tallest member down, equal heights sharing a layer; a layer
/// of capture area `x` per ground area absorbs 1 - exp(-x) of what reaches it. The cell's light is only shared out (the
/// shades, weighted by capture area, have mean 1). Only if its bodies changed (`Domain::stir`), with `l` for the work.
fn shade(w: &World, d: &mut Domain, c: usize, l: &mut Vec<(f32, f64, Handle)>) {
    if !std::mem::take(&mut d.stale[c]) {
        return;
    }
    // What a layer of capture area `x` per ground area intercepts, per unit of `x`: (1 - e^-x) / x.
    let g = mean_decay;
    let a = w.mesh.cells().area()[d.cells[c] as usize];
    l.clear();
    l.extend(d.at[c].iter().map(|&s| (s, &d[s])).map(|(s, b)| (b.vis[6], b.vis[1] as f64 * b.n() / a, s)));
    l.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (all, mut above, mut k) = (g(l.iter().map(|x| x.1).sum()), 0f64, 0);
    while k < l.len() {
        let e = k + l[k..].iter().take_while(|x| x.0 == l[k].0).count();
        let x: f64 = l[k..e].iter().map(|x| x.1).sum();
        let f = (-above).exp() * g(x) / all;
        l[k..e].iter().for_each(|y| d[y.2].shade = f);
        (above, k) = (above + x, e);
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
    let due = w.life.take_due(w.batch());
    if due.is_empty() {
        return;
    }
    let life = &w.life;
    let at = |id: u64| life.find(id).map(|s| &life[s]).map(|b| (life.domain_of(b.cell as usize), b.cell as usize, id, b.hold.is_some() || b.inside.is_some()));
    let mut list: Vec<(usize, usize, u64, bool)> = due.into_iter().filter_map(at).collect();
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
type Scratch = (Reactor, LiveTmp, choose::ActTmp);

/// A new scratch of a body's update.
fn scratch(w: &World) -> Scratch {
    (Reactor::new(&w.chem, &w.cfg), LiveTmp::default(), choose::ActTmp::default())
}

/// The update of body `id` of cell `cell` in domain `d`, if it is still stored there (it may have died or joined
/// another body since): its physics, decision, selection, lineage, maturing and split; or its death.
fn update(w: &World, d: &mut Domain, (cell, id): (usize, u64), (rc, tmp, act): &mut Scratch) {
    let Some(s) = d.find(id) else { return };
    d.moves.open(w, cell, Node::new(Class::Body, s.slot as usize));
    let mut b = std::mem::take(&mut d[s]);
    (d.counts.updates, d.began) = (d.counts.updates + 1, d.out.len());
    // Its mean member's body before the update, for what its members grow (`copy::mature`).
    let before = b.member_body(&w.chem);
    // A node without a brain runs its update for a sample member too, which changes nothing (`copy::sample`).
    let sample = crate::copy::sample(w, d, &b, rc, tmp);
    let lived = match live(w, d, &mut b, rc, tmp) {
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
    // Its mean moves by what the sample gained over its members (`copy::select`).
    if let Some(x) = sample {
        crate::copy::select(w, &mut b, x, lived.gain);
    }
    crate::lineage::resize(w, d, &mut b);
    // Young members who grew past their copy mass join their adult node (`copy::mature`).
    if let Grown::Joins(t) = crate::copy::mature(w, d, &mut b, before) {
        d[s] = b;
        return crate::nodes::merge(w, d, s, t);
    }
    crate::nodes::split(w, d, &mut b);
    d.see(w, &mut b);
    d.schedule(w.due_in(b.interval(w)), &mut b);
    d[s] = b;
}
