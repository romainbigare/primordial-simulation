//! The bodies seen from their cells: before each physics step what each cell holds and the shade of its bodies
//! (`survey`), and every batch of days the updates due (`batch`).

use crate::body::{die, live, Body, LiveTmp, LACK};
use crate::{brain::SIG, change::Reactor, choose, copy::Grown, life::Handle, math::mean_decay, units::DAY, World};
use crate::store::{apply, Batch, Class, Node};
use serde::{Deserialize, Serialize};

/// A brain perceives the loose amount of a substance in a cell, `a` mol/m², as `LOOSE_SENSE` ln(1 + a).
const LOOSE_SENSE: f32 = 0.2;

/// Temperature, depth of the standing liquid and surface height of a ground cell, as Flow's last step left them (once
/// Flow's working memory is built, `flow::ensure`).
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let (d, z) = w.scratch.view().standing(i);
    [w.t_ground_last(i), d, z]
}

/// Before each physics step: what movers broke since the last (`choose::wear`), what each cell holds (`sum_cells`);
/// once a month the merge of alike nodes, the traffic and each cell's surroundings.
pub fn survey(w: &mut World) {
    let n = w.mesh.cells().n();
    let monthly = w.time.is_multiple_of(w.cfg.calendar().month);
    choose::wear(w);
    if monthly {
        crate::nodes::crowd(w);
    }
    shade(w);
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
    let n = w.mesh.cells().n();
    let (mut abs, mut kept) = (vec![0.0; n], vec![0.0; n]);
    sums(w);
    for b in w.life.alive() {
        let (i, s) = (b.cell as usize, light_area(b) * b.shade);
        (abs[i], kept[i]) = (abs[i] + s, kept[i] + s * b.keep as f64);
    }
    let World { life, mesh, canopy, .. } = w;
    life.absorb.resize(n, 0.0);
    life.keep.resize(n, 0.0);
    let area = mesh.cells().area();
    for i in 0..n {
        let s = life.sum[i][2];
        canopy[i] = (s / area[i]) as f32;
        life.absorb[i] = if s > 0.0 { (abs[i] / s) as f32 } else { 0.0 };
        life.keep[i] = if abs[i] > 0.0 { (kept[i] / abs[i]) as f32 } else { 0.0 };
    }
}

/// Per cell: its members, then their visible traits summed, then the energy to break its bodies (`Life::sum`).
pub fn sums(w: &mut World) {
    let mut sum = vec![[0.0; 8]; w.mesh.cells().n()];
    for b in w.life.alive() {
        let (s, m) = (&mut sum[b.cell as usize], b.n());
        s[0] += m;
        s[1..7].iter_mut().zip(&b.vis).for_each(|(x, &v)| *x += v as f64 * m);
        s[7] += b.breaking(&w.chem, &w.cfg, 1.0);
    }
    w.life.sum = sum;
}

/// The capture area of body `b`'s members times the share of light they absorb, m².
fn light_area(b: &Body) -> f64 {
    b.vis[1] as f64 * b.n() * (1.0 - b.reflectance())
}

/// Per cell, what its bodies claim of its bodies' node, J: the light set aside for each since its mark, and the heat its
/// ground gave the node for each since its last update (`Draw::owed`).
pub(crate) fn claims(w: &World) -> Vec<f64> {
    let mut claim = vec![0.0; w.mesh.cells().n()];
    for b in w.life.alive() {
        let i = b.cell as usize;
        claim[i] += light_area(b) * b.keep as f64 * (w.light[i] - b.mark) + b.draw.owed;
    }
    claim
}

/// The heat a body draws from its ground between its updates (laws.md 1.4): what it drew at its last update, less in
/// proportion as the ground comes nearer the temperature at which it draws nothing: where what it evaporates stops (the
/// dew point of the air it breathes, the vapour's deficit going nearly with the temperature above it; or `COOL_T` where
/// it evaporated to cool), but no nearer than its conductance to the surface allows. No more than it drew then.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Draw {
    /// The heat it drew from the ground at its last update, W (0 if it gave heat)
    pub most: f64,
    /// What it draws per K of ground above `t`, W/K, and the ground temperature at which it draws nothing, K
    pub g: f64,
    pub t: f64,
    /// Heat its cell's ground gave its bodies' node for it since its last update, J
    pub owed: f64,
}

impl Draw {
    /// After an update that drew `drew` W through a conductance `g` W/K from ground at `tg` K, what it evaporated
    /// stopping at `stop` K (none if it evaporated nothing), owing nothing yet.
    pub(crate) fn after(drew: f64, g: f64, tg: f64, stop: Option<f64>) -> Draw {
        if drew <= 0.0 || g <= 0.0 {
            return Draw::default();
        }
        let t = stop.map_or(tg - drew / g, |s| s.min(tg - drew / g));
        Draw { most: drew, g: drew / (tg - t), t, owed: 0.0 }
    }
    /// What it draws from ground at `t` K, W, and its slope, W/K.
    #[inline]
    fn at(&self, t: f64) -> (f64, f64) {
        match self.g * (t - self.t) {
            r if r <= 0.0 => (0.0, 0.0),
            r if r >= self.most => (self.most, 0.0),
            r => (r, self.g),
        }
    }
}

/// The bodies that draw heat from each cell's ground (`Draw`), cell by cell, for an air step.
pub(crate) struct Draws {
    /// Per cell, its entries `at[i]..at[i + 1]`
    at: Vec<u32>,
    /// Per entry: the body and its draw
    body: Vec<Handle>,
    each: Vec<Draw>,
    /// Per cell, the heat its bodies' node holds that no body claims and its bodies draw first, over the step, W
    pool: Vec<f64>,
}

impl Draws {
    /// Before an air step of `dt` seconds: the bodies that draw from each cell's ground, and the heat each bodies' node
    /// holds that none of its bodies claim (`claims`). What its bodies could draw of that until their next updates
    /// stays for them to draw first, so a draw an update did not use spares the ground later rather than going back at
    /// once; the rest goes back to the ground now, so the step starts from it.
    pub(crate) fn settle(w: &mut World, dt: f64) -> Draws {
        let n = w.mesh.cells().n();
        let mut d = Draws { at: vec![0], body: Vec::new(), each: Vec::new(), pool: vec![0.0; n] };
        // What the bodies of each cell could draw until their next updates, J.
        let mut most = vec![0.0; n];
        for (i, most) in most.iter_mut().enumerate() {
            for b in w.life.cell(i).map(|s| (s, &w.life[s])).filter(|(_, b)| b.alive && b.draw.most > 0.0) {
                d.body.push(b.0);
                d.each.push(b.1.draw);
                *most += b.1.draw.most * b.1.interval(w) as f64 * DAY;
            }
            d.at.push(d.body.len() as u32);
        }
        let mut back = Batch::new([Class::Bodies, Class::Ground], &[]);
        for (i, c) in claims(w).into_iter().enumerate() {
            let free = w.store.heat(Class::Bodies)[i] - c;
            d.pool[i] = free.clamp(0.0, most[i]) / dt;
            if free > most[i] {
                back.link((i, i), free - most[i]);
            }
        }
        apply(w, &[back.flux()], &[], &[]);
        d
    }
    fn of(&self, i: usize) -> &[Draw] {
        &self.each[self.at[i] as usize..self.at[i + 1] as usize]
    }
    /// Do bodies draw from cell `i`'s ground?
    pub(crate) fn any(&self, i: usize) -> bool {
        self.at[i] < self.at[i + 1]
    }
    /// What the bodies of cell `i` draw at `t` K, W, and its slope, W/K.
    fn drawn(&self, i: usize, t: f64) -> (f64, f64) {
        self.of(i).iter().map(|d| d.at(t)).fold((0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1))
    }
    /// What the bodies of cell `i` draw from its ground at `t` K, past what they draw from their node's pool, W, and
    /// its slope, W/K.
    pub(crate) fn ground(&self, i: usize, t: f64) -> (f64, f64) {
        match self.drawn(i, t) {
            (r, s) if r > self.pool[i] => (r - self.pool[i], s),
            _ => (0.0, 0.0),
        }
    }
    /// Each body of cell `i` owes its share, by what it draws at `t(i)` K over `dt` seconds, of what its node got:
    /// `given[i]` J from the ground and what it drew of the pool.
    pub(crate) fn owe(&self, w: &mut World, t: impl Fn(usize) -> f64, given: &[f64], dt: f64) {
        for (i, &got) in given.iter().enumerate().filter(|e| self.any(e.0)) {
            let r = self.drawn(i, t(i)).0;
            if r > 0.0 {
                let per = (got + r.min(self.pool[i]) * dt) / r;
                for x in self.at[i] as usize..self.at[i + 1] as usize {
                    w.life[self.body[x]].draw.owed += self.each[x].at(t(i)).0 * per;
                }
            }
        }
    }
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

/// Light passes the bodies of a cell from the tallest member down, equal heights sharing a layer; a layer of capture
/// area `x` per ground area absorbs 1 - exp(-x) of what reaches it. The cell's light is only shared out (the shades,
/// weighted by capture area, have mean 1). Only cells whose bodies changed are worked out again (`Life::stir`).
fn shade(w: &mut World) {
    // What a layer of capture area `x` per ground area intercepts, per unit of `x`: (1 - e^-x) / x.
    let g = mean_decay;
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area());
    let life = &mut w.life;
    if life.stale.len() != n {
        life.stale = vec![true; n];
    }
    let mut l: Vec<(f32, f64, Handle)> = Vec::new();
    for (i, &a) in area.iter().enumerate() {
        if !std::mem::take(&mut life.stale[i]) {
            continue;
        }
        l.clear();
        l.extend(life.cell(i).map(|s| (s, &life[s])).map(|(s, b)| (b.vis[6], b.vis[1] as f64 * b.n() / a, s)));
        l.sort_by(|a, b| b.0.total_cmp(&a.0));
        let (all, mut above, mut k) = (g(l.iter().map(|x| x.1).sum()), 0f64, 0);
        while k < l.len() {
            let e = k + l[k..].iter().take_while(|x| x.0 == l[k].0).count();
            let x: f64 = l[k..e].iter().map(|x| x.1).sum();
            let f = (-above).exp() * g(x) / all;
            l[k..e].iter().for_each(|y| life[y.2].shade = f);
            (above, k) = (above + x, e);
        }
    }
}

/// What a body shows others in a batch, as it was at the batch's start (plan 3.5): what they perceive of it.
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
    /// The face of every living body of `w` as it is now, cell by cell.
    fn make(&mut self, w: &World) {
        self.clear();
        self.at.push(0);
        for i in 0..w.mesh.cells().n() {
            for b in w.life.cell(i).map(|h| &w.life[h]).filter(|b| b.alive) {
                let room = crate::bond::room(w, b).map(|(r, e)| (r as u32, e.closed));
                self.all.push(Face { id: b.id, n: b.n(), vis: b.vis, sig: b.sig, wb: b.wb, room });
            }
            self.at.push(self.all.len() as u32);
        }
    }
    /// No batch runs.
    fn clear(&mut self) {
        self.all.clear();
        self.at.clear();
    }
}

/// Update the bodies due in this batch, all at its first day, domain by domain (`life::Domain`), cell by cell, and in a
/// cell by id: an order that does not depend on where bodies are stored, and keeps memory reads close together. They
/// see each other as they were at the batch's start (`Faces`), and the store as this batch's moves of their domain
/// leave it (`store::Moves`), booked when the batch ends.
pub fn batch(w: &mut World) {
    let due = w.life.take_due(w.batch());
    if due.is_empty() {
        return;
    }
    let life = &w.life;
    let at = |id: u64| life.find(id).map(|s| life[s].cell as usize).map(|i| (life.place(i).0, i, id));
    let mut list: Vec<(usize, usize, u64)> = due.into_iter().filter_map(at).collect();
    list.sort_unstable();
    // The busiest domain's share of the batch's updates (`Counters::busiest`).
    let most = list.chunk_by(|a, b| a.0 == b.0).map(|d| d.len()).max().unwrap_or(0);
    (w.counts.busiest, w.counts.batches) = (w.counts.busiest + most as f64 / list.len().max(1) as f64, w.counts.batches + 1.0);
    let mut faces = std::mem::take(&mut w.life.faces);
    faces.make(w);
    w.life.faces = faces;
    let mut rc = Reactor::new(&w.chem, &w.cfg);
    let (mut tmp, mut act) = (LiveTmp::default(), choose::ActTmp::default());
    for (_, cell, id) in list {
        // (It may have died or joined another body since.)
        let Some(s) = w.life.find(id) else { continue };
        w.open_moves(cell, Node::new(Class::Body, s.slot as usize));
        update(w, s, &mut rc, (&mut tmp, &mut act));
    }
    w.book_moves();
    w.life.faces.clear();
}

/// The update of the body stored at `s`: its physics, decision, selection, lineage, maturing and split; or its death.
fn update(w: &mut World, s: Handle, rc: &mut Reactor, (tmp, act): (&mut LiveTmp, &mut choose::ActTmp)) {
    let mut b = std::mem::take(&mut w.life[s]);
    w.counts.updates += 1;
    // Its mean member's body before the update, for what its members grow (`copy::mature`).
    let before = b.member_body(&w.chem);
    // A node without a brain runs its update for a sample member too, which changes nothing (`copy::sample`).
    let sample = crate::copy::sample(w, &b, rc, tmp);
    let lived = match live(w, &mut b, rc, tmp) {
        Err(cause) => return die(w, s, b, cause),
        Ok(lived) => lived,
    };
    // A body with no brain takes no decision: its update is its physics only.
    if !w.life.brainless(&b) {
        choose::act(w, &mut b, &lived, act);
        // An action can leave nothing of a member: it dies at once, before another body deals with it.
        if b.empty_at(b.mass(&w.chem)) {
            return die(w, s, b, LACK);
        }
    }
    // Its mean moves by what the sample gained over its members (`copy::select`).
    if let Some(x) = sample {
        crate::copy::select(w, &mut b, x, lived.gain);
    }
    crate::lineage::resize(w, &mut b);
    // Young members who grew past their copy mass join their adult node (`copy::mature`).
    if let Grown::Joins(t) = crate::copy::mature(w, &mut b, before) {
        let s = w.life.put(s, b);
        return crate::nodes::merge(w, s, t);
    }
    crate::nodes::split(w, &mut b);
    w.see(&mut b);
    w.life.schedule(w.due_in(b.interval(w)), &mut b);
    w.life.put(s, b);
}
