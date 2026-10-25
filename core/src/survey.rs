//! The bodies seen from their cells: before each physics step what each cell holds and the shade of its bodies
//! (`survey`), and every batch of days the updates due, in a parallel and a serial phase (`batch`).

use crate::body::{die, live, Body, Layout, LiveTmp, LACK};
use crate::{brain::SIG, change::Reactor, choose, copy::Grown, life::{Domain, Handle}, math::mean_decay, World};
use crate::store::{Class, Node};

mod draws;
pub use draws::Draw;
pub(crate) use draws::Draws;

/// A brain perceives the loose amount of a substance in a cell, `a` mol/m², as `LOOSE_SENSE` ln(1 + a).
const LOOSE_SENSE: f32 = 0.2;

/// Temperature, depth of the standing liquid and surface height of a ground cell, as Flow's last step left them (once
/// Flow's working memory is built, `flow::ensure`).
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let (d, z) = w.scratch.view().standing(i);
    [w.t_ground_last(i), d, z]
}

/// Before each physics step, in each cell: what movers broke since the last (`choose::wear`) and once a month the merge
/// of alike nodes (`nodes::crowd`); then each cell's light shared out (`shade`) and what it holds (`sum_cells`); once a
/// month the traffic and each cell's surroundings. The wear and the merge run per domain on the pool (`on_domains`) in
/// the cells where no part is and no body holds or is inside one; then in those cells, one by one, each a serial step
/// with its outbox applied after it, as a death or a merge there lets go of a part (laws.md 0). With `reversed`, the
/// domains start in reversed order: only for the order test.
pub fn survey(w: &mut World, reversed: bool) {
    let n = w.mesh.cells().n();
    let monthly = w.time.is_multiple_of(w.cfg.calendar().month);
    let (pushed, lay) = (std::mem::take(&mut w.life.pushed), Layout::of(&w.chem));
    let worn = |w: &World, d: &mut Domain, i: usize| {
        if let Some(&e) = pushed.get(i).filter(|&&e| e > 0.0) {
            choose::wear(w, d, i, e);
        }
        if monthly {
            crate::nodes::crowd(w, d, &lay, i);
        }
    };
    let parts = |w: &World, d: &Domain, i: usize| !w.parts.at[i].is_empty() || d.cell(i).any(|h| d[h].hold.is_some() || d[h].inside.is_some());
    // The domains that store bodies, each with their count.
    let peopled = |w: &World| w.life.domains.iter().enumerate().map(|(k, d)| (k, d.at.iter().map(Vec::len).sum(), ())).filter(|x| x.1 > 0).collect();
    on_domains(w, peopled(w), reversed, |w, d, _| {
        for k in 0..d.cells.len() {
            let i = d.cells[k] as usize;
            if !parts(w, d, i) {
                worn(w, d, i);
            }
        }
    });
    let serial: Vec<usize> = (0..n).filter(|&i| parts(w, w.life.domain(i), i)).collect();
    for i in serial {
        w.in_domain(i, |w, d| worn(w, d, i));
        crate::outbox::deliver(w);
    }
    on_domains(w, peopled(w), reversed, |w, d, _| shade(w, d));
    sum_cells(w);
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

/// Per cell: its members and visible traits summed, their capture area per ground area, the light they absorb and the
/// share set aside for their light reactions.
fn sum_cells(w: &mut World) {
    sums(w);
    let (n, World { life, mesh, canopy, .. }) = (w.mesh.cells().n(), w);
    life.absorb.resize(n, 0.0);
    life.keep.resize(n, 0.0);
    let area = mesh.cells().area();
    for (i, r) in life.sum.iter().enumerate() {
        canopy[i] = (r[2] / area[i]) as f32;
        life.absorb[i] = if r[2] > 0.0 { (r[8] / r[2]) as f32 } else { 0.0 };
        life.keep[i] = if r[8] > 0.0 { (r[9] / r[8]) as f32 } else { 0.0 };
    }
}

/// The numbers of `Life::sum` per cell.
pub(crate) const ROW: usize = 10;

/// Per cell (`Life::sum`), cell by cell on the pool (`par::rows`), each cell's bodies in the order they came: its
/// members, then their visible traits summed, then the energy to break its bodies; then their capture area times their
/// shade, and that times the share they set aside for their light reactions.
pub fn sums(w: &mut World) {
    let mut rows = vec![[0.0; ROW]; w.mesh.cells().n()];
    let v = &*w;
    crate::par::rows(rows.as_flattened_mut(), ROW, || (), |_, i, s| {
        for b in v.life.cell(i).map(|h| &v.life[h]).filter(|b| b.alive) {
            let (m, a) = (b.n(), light_area(b) * b.shade);
            s[0] += m;
            s[1..7].iter_mut().zip(&b.vis).for_each(|(x, &v)| *x += v as f64 * m);
            (s[7], s[8], s[9]) = (s[7] + b.breaking(&v.chem, &v.cfg, 1.0), s[8] + a, s[9] + a * b.keep as f64);
        }
    });
    w.life.sum = rows;
}

/// The capture area of body `b`'s members times the share of light they absorb, m².
fn light_area(b: &Body) -> f64 {
    b.vis[1] as f64 * b.n() * (1.0 - b.reflectance())
}

/// The yearly moving average of the crossings of each cell (`Counters::traffic`), once a month.
fn traffic(w: &mut World) {
    let (n, d) = (w.mesh.cells().n(), &mut w.counts);
    d.traffic.resize(n, 0.0);
    d.crossings.resize(n, 0.0);
    let a = 1.0 / w.cfg.months_per_year as f32;
    for (t, c) in d.traffic.iter_mut().zip(d.crossings.iter_mut()) {
        *t += a * (*c * w.cfg.months_per_year as f32 - *t);
        *c = 0.0;
    }
}

/// Light passes the bodies of a cell of domain `d` from the tallest member down, equal heights sharing a layer; a layer of
/// capture area `x` per ground area absorbs 1 - exp(-x) of what reaches it. The cell's light is only shared out (the
/// shades, weighted by capture area, have mean 1). Only cells whose bodies changed are worked out again
/// (`Domain::stir`).
fn shade(w: &World, d: &mut Domain) {
    // What a layer of capture area `x` per ground area intercepts, per unit of `x`: (1 - e^-x) / x.
    let g = mean_decay;
    let area = w.mesh.cells().area();
    let mut l: Vec<(f32, f64, Handle)> = Vec::new();
    for c in 0..d.cells.len() {
        if !std::mem::take(&mut d.stale[c]) {
            continue;
        }
        let a = area[d.cells[c] as usize];
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
}

/// What a body shows others in a batch, as it was at the batch's start (laws.md 0): what they perceive of it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Face {
    pub id: u64,
    /// Its members (`Body::n`), visible traits (`Body::vis`), signal and well-being at its last decision
    pub n: f64,
    pub vis: [f32; 7],
    pub sig: [f32; SIG],
    pub wb: f32,
    /// The root of the structure it is inside and how closed its walls are (`bond::room`)
    pub room: Option<(u32, f32)>,
}

/// The faces of the living bodies at the start of the batch running: of cell `i`, `all[at[i]..at[i + 1]]`, in the
/// order they came. Perception reads only these (`choose::perceive`), whatever happened in the batch since.
#[derive(Clone, Debug, Default)]
pub struct Faces {
    all: Vec<Face>,
    at: Vec<u32>,
}

impl Faces {
    /// The faces of the bodies of cell `i` (none outside a batch).
    pub fn of(&self, i: usize) -> &[Face] {
        self.at.get(i + 1).map_or(&[], |&e| &self.all[self.at[i] as usize..e as usize])
    }
    /// The face of every living body of `w` as it is now, cell by cell, in the room these faces held.
    fn make(mut self, w: &World) -> Faces {
        self.clear();
        self.at.push(0);
        for i in 0..w.mesh.cells().n() {
            for b in w.life.cell(i).map(|h| &w.life[h]).filter(|b| b.alive) {
                let room = crate::bond::room(w, b).map(|(r, e)| (r as u32, e.closed));
                self.all.push(Face { id: b.id, n: b.n(), vis: b.vis, sig: b.sig, wb: b.wb, room });
            }
            self.at.push(self.all.len() as u32);
        }
        self
    }
    /// No batch runs.
    fn clear(&mut self) {
        self.all.clear();
        self.at.clear();
    }
}

/// Update the bodies due in this batch, all at its first day (laws.md 0). First the parallel phase: the domains are taken
/// out of the world (`life::Domain`), and each runs, on a thread of the pool with the feature `threads` (`on_domains`),
/// its bodies that hold no part and are inside none, cell by cell, and in a cell by id (an order that does not depend on
/// where bodies are stored, and keeps memory reads close together), with the world read only. They see each other as
/// they were at the batch's start (`Faces`) and the store as their domain's moves leave it (`store::Moves`); what they
/// do outside their domain goes to its outbox (`outbox`), and the lineages and born rows they make are provisional. Then
/// the serial phase: the domains back, the moves booked, the world settles (`World::settle`: events, counters, those ids
/// made global), the outboxes applied, then the bodies that hold or are inside a part, each a serial step on its domain
/// (`World::in_domain`) with its outbox applied after it, and last the changes of holders are added: the lineages and
/// born rows left with no holder die out or are freed.
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
    w.life.faces = std::mem::take(&mut w.life.faces).make(w);
    let (serial, list): (Vec<_>, Vec<_>) = list.into_iter().partition(|x| x.3);
    w.life.batch = true;
    let runs = list.chunk_by(|a, b| a.0 == b.0).map(|run| (run[0].0, run.len(), run)).collect();
    on_domains(w, runs, reversed, |w, d, run| {
        let mut scratch = (Reactor::new(&w.chem, &w.cfg), LiveTmp::default(), choose::ActTmp::default());
        run.iter().for_each(|&(_, cell, id, _)| update(w, d, (cell, id), &mut scratch));
    });
    let mut scratch = (Reactor::new(&w.chem, &w.cfg), LiveTmp::default(), choose::ActTmp::default());
    for (_, cell, id, _) in serial {
        w.in_domain(cell, |w, d| update(w, d, (cell, id), &mut scratch));
        crate::outbox::deliver(w);
    }
    w.life.batch = false;
    w.settle(0..w.life.domains.len());
    w.life.settle_moved();
    w.life.faces.clear();
}

/// Run `f` on the domains of `work` (each with its count of work and what it is given), taken out of the world
/// (`life::Domain`), on the pool with the feature `threads` (`par::each`), the most work first so the last to end is a
/// small one (longest processing time first, Graham 1969; ties by domain), backwards with `reversed`; then the domains
/// back, their moves booked, the world settled (`World::settle`) and the outboxes applied (`outbox::deliver`).
fn on_domains<T: Send + Sync>(w: &mut World, mut work: Vec<(usize, usize, T)>, reversed: bool, f: impl Fn(&World, &mut Domain, &T) + Sync + Send) {
    work.sort_by_key(|x| (std::cmp::Reverse(x.1), x.0));
    if reversed {
        work.reverse();
    }
    let mut domains = std::mem::take(&mut w.life.domains);
    let mut free: Vec<Option<&mut Domain>> = domains.iter_mut().map(Some).collect();
    let mut runs: Vec<_> = work.into_iter().map(|(k, _, t)| (free[k].take().expect("once a domain"), t)).collect();
    crate::par::each(&mut runs, |(d, t)| f(w, d, t));
    w.life.domains = domains;
    w.book_moves();
    w.settle(0..w.life.domains.len());
    crate::outbox::deliver(w);
}

/// The update of body `id` of cell `cell` in domain `d`, if it is still stored there (it may have died or joined
/// another body since): its physics, decision, selection, lineage, maturing and split; or its death.
fn update(w: &World, d: &mut Domain, (cell, id): (usize, u64), (rc, tmp, act): &mut (Reactor, LiveTmp, choose::ActTmp)) {
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
