//! The bodies seen from their cells: before each physics step what each cell holds and the shade of its bodies
//! (`survey`), and every day the updates due (`day`).

use crate::body::{die, live, Body, LiveTmp, LACK};
use crate::chem::Phase;
use crate::change::Reactor;
use crate::life::Life;
use crate::math::mean_decay;
use crate::copy::Grown;
use crate::units::DAY;
use crate::store::{apply, Batch, Class, Node};
use crate::{bond, choose, World};
use serde::{Deserialize, Serialize};

/// A brain perceives the loose amount of a substance in a cell, `a` mol/m², as `LOOSE_SENSE` ln(1 + a).
const LOOSE_SENSE: f32 = 0.2;

/// Temperature (of the last flow step), depth of the standing liquid and surface height of a ground cell (whole phases).
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let t = w.t_ground_last(i);
    let (mut z, mut d, mut loose) = (0.0, 0.0, 0.0);
    for k in 0..w.chem.ns() {
        z += w.b(k, i) * w.chem.molar_volume(k, Phase::Solid);
        match w.chem.phase_in(k, t, |j| w.g(j, i)) {
            Phase::Solid => {
                loose += w.g(k, i) * w.chem.molar_volume(k, Phase::Solid);
                z += w.g(k, i) * w.chem.molar_volume(k, Phase::Solid);
            }
            Phase::Liquid => d += w.g(k, i) * w.chem.molar_volume(k, Phase::Liquid),
            Phase::Gas => {}
        }
    }
    let area = w.mesh.cells().area()[i];
    [t, (d - w.cfg.phys.porosity * loose).max(0.0) / area, w.base_z + z / area]
}

/// Before each physics step: what each cell holds (`sum_cells`); once a month the merge of alike nodes, the traffic
/// and each cell's surroundings.
pub fn survey(w: &mut World) {
    let n = w.mesh.cells().n();
    w.life.fit_map(n);
    if w.canopy.len() != n {
        (w.canopy, w.light, w.rained) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    }
    bond::ready(w);
    let monthly = w.time.is_multiple_of(w.cfg.calendar().month);
    if monthly {
        crate::nodes::crowd(w);
    }
    shade(w);
    sum_cells(w);
    if monthly && w.time > 0 {
        traffic(w);
    }
    if monthly || w.life.env.len() != n || w.life.seen.len() != n * w.chem.ns() {
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
    let mut sum = vec![[0.0; 7]; n];
    for (slot, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
        let (i, m, k) = (b.cell as usize, b.n(), b.keep as f64);
        sum[i][0] += m;
        for (x, &v) in sum[i][1..].iter_mut().zip(&b.vis) {
            *x += v as f64 * m;
        }
        let s = light_area(b) * w.life.shade[slot];
        (abs[i], kept[i]) = (abs[i] + s, kept[i] + s * k);
    }
    let World { life, mesh, canopy, .. } = w;
    life.sum = sum;
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

/// The capture area of body `b`'s members times the share of light they absorb, m².
fn light_area(b: &Body) -> f64 {
    b.vis[1] as f64 * b.n() * (1.0 - b.reflectance())
}

/// Per cell, what its bodies claim of its bodies' node, J: the light set aside for each since its mark, and the heat its
/// ground gave the node for each since its last update (`Draw::owed`).
pub(crate) fn claims(w: &World) -> Vec<f64> {
    let mut claim = vec![0.0; w.mesh.cells().n()];
    for b in w.life.bodies.iter().filter(|b| b.alive) {
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
    /// Per entry: the body's slot and its draw
    slot: Vec<u32>,
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
        let (n, none) = (w.mesh.cells().n(), Vec::new());
        let mut d = Draws { at: vec![0], slot: Vec::new(), each: Vec::new(), pool: vec![0.0; n] };
        // What the bodies of each cell could draw until their next updates, J.
        let mut most = vec![0.0; n];
        for (i, cell) in (0..n).map(|i| (i, w.life.at.get(i).unwrap_or(&none))) {
            for b in cell.iter().map(|&s| (s, &w.life.bodies[s as usize])).filter(|(_, b)| b.alive && b.draw.most > 0.0) {
                d.slot.push(b.0);
                d.each.push(b.1.draw);
                most[i] += b.1.draw.most * b.1.interval(w) as f64 * DAY;
            }
            d.at.push(d.slot.len() as u32);
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
                    w.life.bodies[self.slot[x] as usize].draw.owed += self.each[x].at(t(i)).0 * per;
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

/// The share of what falls that a layer of capture area `x` per ground area intercepts, per unit of `x`: (1 - e^-x) / x.
pub(crate) fn intercept(x: f64) -> f64 {
    mean_decay(x)
}

/// Light passes the bodies of a cell from the tallest member down, equal heights sharing a layer; a layer of capture
/// area `x` per ground area absorbs 1 - exp(-x) of what reaches it. The cell's light is only shared out (the shades,
/// weighted by capture area, have mean 1). Only cells whose bodies changed are worked out again (`Life::stir`).
fn shade(w: &mut World) {
    let g = intercept;
    let (Life { bodies, at, shade, stale, .. }, area) = (&mut w.life, w.mesh.cells().area());
    shade.resize(bodies.len(), 1.0);
    if stale.len() != at.len() {
        *stale = vec![true; at.len()];
    }
    let mut l: Vec<(f32, f64, usize)> = Vec::new();
    for (i, s) in at.iter().enumerate().filter(|&(i, _)| std::mem::take(&mut stale[i])) {
        l.clear();
        l.extend(s.iter().map(|&s| (s as usize, &bodies[s as usize])).map(|(s, b)| (b.vis[6], b.vis[1] as f64 * b.n() / area[i], s)));
        l.sort_by(|a, b| b.0.total_cmp(&a.0));
        let (all, mut above, mut k) = (g(l.iter().map(|x| x.1).sum()), 0f64, 0);
        while k < l.len() {
            let e = k + l[k..].iter().take_while(|x| x.0 == l[k].0).count();
            let x: f64 = l[k..e].iter().map(|x| x.1).sum();
            let f = (-above).exp() * g(x) / all;
            l[k..e].iter().for_each(|y| shade[y.2] = f);
            (above, k) = (above + x, e);
        }
    }
}

/// Update the bodies due today.
pub fn day(w: &mut World) {
    let due = w.life.take_due(w.day());
    if due.is_empty() {
        return;
    }
    // Cell by cell (a fixed order that keeps memory reads close together).
    let bodies = &w.life.bodies;
    let mut list: Vec<(u32, u32, u32)> = due.iter().filter(|&&(s, g)| bodies[s as usize].alive && bodies[s as usize].gen == g).map(|&(s, g)| (bodies[s as usize].cell, s, g)).collect();
    list.sort_unstable();
    let mut rc = Reactor::new(&w.chem, &w.cfg);
    let (mut tmp, mut act) = (LiveTmp::default(), choose::ActTmp::default());
    for (cell, slot, gen) in list {
        let s = slot as usize;
        if !w.life.bodies[s].alive || w.life.bodies[s].gen != gen {
            continue;
        }
        // What it takes from and gives to the store is gathered and booked once, at the end of its update.
        w.open_moves(cell as usize, Node::new(Class::Body, s));
        update(w, s, &mut rc, (&mut tmp, &mut act));
        w.book_moves();
    }
}

/// The update of the body in `slot`: its physics, decision, selection, lineage, maturing and split; or its death.
fn update(w: &mut World, s: usize, rc: &mut Reactor, (tmp, act): (&mut LiveTmp, &mut choose::ActTmp)) {
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    w.counts.updates += 1;
    // Its mean member's body before the update, for what its members grow (`copy::mature`).
    let before = b.member_body(&w.chem);
    // A node without a brain runs its update for a sample member too, which changes nothing (`copy::sample`).
    let sample = crate::copy::sample(w, s, &b, rc, tmp);
    let lived = match live(w, s, &mut b, rc, tmp) {
        Err(cause) => return die(w, s, b, cause),
        Ok(lived) => lived,
    };
    // A body with no brain takes no decision: its update is its physics only.
    if !w.life.brainless(&b) {
        choose::act(w, s, &mut b, &lived, act);
        // An action can leave nothing of a member: it dies at once, before another body deals with it.
        if b.empty_at(b.mass(&w.chem)) {
            return die(w, s, b, LACK);
        }
    }
    // Its mean moves by what the sample gained over its members (`copy::select`).
    if let Some(x) = sample {
        crate::copy::select(w, &mut b, x, lived.gain);
    }
    crate::lineage::resize(w, s, &mut b);
    // Young members who grew past their copy mass join their adult node (`copy::mature`).
    if let Grown::Joins(t) = crate::copy::mature(w, s, &mut b, before) {
        w.life.bodies[s] = b;
        return crate::nodes::merge(w, s, t);
    }
    crate::nodes::split(w, s, &mut b);
    w.see(&mut b);
    w.life.schedule(w.day() + b.interval(w), s as u32, b.gen);
    w.life.bodies[s] = b;
}
