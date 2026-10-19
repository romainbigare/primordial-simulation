//! The bodies seen from their cells: before each physics step what each cell holds and the shade of its bodies
//! (`survey`), and every day the updates due (`day`).

use crate::body::{die, live, LiveTmp, LACK};
use crate::chem::Phase;
use crate::change::Reactor;
use crate::life::Life;
use crate::math::mean_decay;
use crate::units::DAY;
use crate::copy::Grown;
use crate::ground::heat_above_floor;
use crate::store::{apply, Batch, Class, Node};
use crate::{bond, choose, World};

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

/// Before each physics step: what each cell holds and its bodies' node (`sum_cells`); once a month the merge of alike
/// nodes, the traffic and each cell's surroundings.
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
/// share set aside for their light reactions. Then its bodies' node settles with its ground to hold what its bodies
/// will claim: the light set aside since, and the heat each draws at the rate of its last update (laws.md 1.4). What no
/// body can claim goes back to the ground.
fn sum_cells(w: &mut World) {
    let (n, day) = (w.mesh.cells().n(), w.day() as f64);
    let (mut abs, mut kept, mut claim) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let mut sum = vec![[0.0; 7]; n];
    for (slot, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
        let (i, m, k) = (b.cell as usize, b.n(), b.keep as f64);
        sum[i][0] += m;
        for (x, &v) in sum[i][1..].iter_mut().zip(&b.vis) {
            *x += v as f64 * m;
        }
        let a = b.vis[1] as f64 * m * (1.0 - b.reflectance());
        let s = a * w.life.shade[slot];
        (abs[i], kept[i]) = (abs[i] + s, kept[i] + s * k);
        claim[i] += a * k * (w.light[i] - b.mark) + b.drew as f64 * (day - b.last).max(0.0) * DAY;
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
    let mut settle = Batch::new([Class::Ground, Class::Bodies], &[]);
    for (i, &c) in claim.iter().enumerate() {
        let d = c - w.store.heat(Class::Bodies)[i];
        let d = if d > 0.0 { d.min(heat_above_floor(w.store.heat(Class::Ground)[i], w.heat_cap_ground(i))) } else { d };
        if d != 0.0 {
            settle.link((i, i), d);
        }
    }
    apply(w, &[settle.flux()], &[], &[]);
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
