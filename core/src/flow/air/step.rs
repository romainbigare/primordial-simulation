//! One step of the air (laws.md 1.4): the gas of the bodies' nodes comes up and gases dissolve; the cells' ground
//! gives the air level its forcing (`cells::forcing`); the air level runs day by day (`daily::run`); each cell then
//! exchanges with the step's mean air of its node (`cells::pass`); and all of it is booked whole (`book`).

use super::super::scratch::may_evaporate;
use super::super::{range, Scratch};
use super::book::{book, up_from_bodies};
use super::cells::{again, forcing, landed, pass, Ctx, ROW};
use super::daily::{run, Moved};
use super::exchange::dissolve;
use super::radiation::declination;
use crate::{store::Class, survey::Draws, units::DAY, World, AIR};

/// The saturation tables reach this far above the warmest node at a step's start, K: no node warms more in a step.
const SAT_REACH: f64 = 60.0;

/// A gas that may leave the ground or condense: its index in `Scratch::gas`, its substance, and whether it may
/// evaporate or condense.
#[derive(Clone, Copy)]
pub(super) struct Active {
    pub(super) gi: usize,
    pub(super) k: usize,
    pub(super) up: bool,
    pub(super) cond: bool,
}

/// The air's step of `dt` seconds, with the heat the bodies draw from the ground (`draws`).
pub(crate) fn step(w: &mut World, s: &mut Scratch, dt: f64, draws: &Draws) {
    up_from_bodies(w, s);
    let (tmin, tmax) = range(&s.state.tg);
    let henry: Vec<(usize, usize)> = s.state.gas.iter().copied().enumerate().filter(|&(_, k)| s.sp[k].boil < tmin && w.chem.subs[k].solubility > 0.0).collect();
    dissolve(w, s, dt, &henry, (tmin, tmax));
    let act = active(w, s, &henry);
    // The sun's declination at the middle of each day of the step.
    let days = (dt / DAY).ceil().max(1.0) as usize;
    let decl: Vec<f64> = (0..days).map(|d| declination(w.cfg.tilt, w.cfg.calendar().season(w.time + ((d as f64 + 0.5) * dt / days as f64) as u64))).collect();
    let f = forcing(w, s, &act, dt);
    let mut mean = run(w, s, &act, &f, dt);
    let mut rain = landed(w, &act, &f, &mean);
    let mut rows = pass(&Ctx { w, s, act: &act, mean: &mean, rain: &rain, draws, dt, decl: &decl });
    // Where an air node would end with less than nothing of a gas (its cells evaporated less than its days counted
    // on), what rains from it is cut by the share that leaves it nothing, and its cells' rows made again; where none
    // rains, what the wind takes of that gas from it. Each pass only lowers what is taken, so at most as many as there
    // are nodes, and twice that.
    let mut moved = std::mem::take(&mut mean.moved);
    for _ in 0..2 * w.mesh.level(AIR).n() + 1 {
        let short = short(w, &act, &rows, &moved);
        if short.is_empty() {
            break;
        }
        let (mut again_for, mut dry) = (Vec::new(), Vec::new());
        for &(x, j, end) in &short {
            let kids = w.mesh.level(AIR).children(j);
            let fell: f64 = kids.iter().map(|&i| rain[x * w.mesh.cells().n() + i as usize]).sum();
            if fell > 0.0 {
                kids.iter().for_each(|&i| (rain[x * w.mesh.cells().n() + i as usize] *= ((fell + end) / fell).max(0.0), again_for.push(i as usize)).1);
            } else {
                dry.push((x, j, end));
            }
        }
        cut_wind(&act, &dry, &mut moved);
        again(&Ctx { w, s, act: &act, mean: &mean, rain: &rain, draws, dt, decl: &decl }, &again_for, &mut rows);
    }
    mean.moved = moved;
    book(w, s, (&act, &rows), &mean, draws, dt);
}

/// A node is short of a gas where it would end below this share of what it holds and is given and taken, the rounding
/// of those sums (`store::apply` sets that to zero and counts it).
const ROUNDED: f64 = 1e-12;

/// The air nodes that would end the step with less than nothing of an active gas, by what they hold, what their cells
/// give them (`rows`) and the wind: the gas's place in `act`, the node and where it would end, mol.
fn short(w: &World, act: &[Active], rows: &[f64], moved: &Moved) -> Vec<(usize, usize, f64)> {
    let (n, na, width, pairs) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), ROW + act.len(), moved.heat.len());
    let mut out = Vec::new();
    for (x, ac) in act.iter().enumerate() {
        let mut end = w.store.amounts(Class::Air)[ac.k * na..(ac.k + 1) * na].to_vec();
        let mut size = end.clone();
        let mut add = |j: usize, d: f64| {
            (end[j], size[j]) = (end[j] + d, size[j] + d.abs());
        };
        (0..n).for_each(|i| add(w.mesh.cells().parent()[i] as usize, rows[i * width + ROW + x]));
        for p in 0..pairs {
            let m = moved.mol[ac.gi * pairs + p];
            add(moved.a[p] as usize, -m);
            add(moved.b[p] as usize, m);
        }
        out.extend((0..na).filter(|&j| end[j] < -ROUNDED * size[j]).map(|j| (x, j, end[j])));
    }
    out
}

/// What the wind takes of a gas from each node that would end with less than nothing of it (`short`) is cut by the
/// share that leaves it nothing (as `solve::sweep::within`).
fn cut_wind(act: &[Active], short: &[(usize, usize, f64)], moved: &mut Moved) {
    let pairs = moved.heat.len();
    for &(x, j, end) in short {
        let gi = act[x].gi;
        let leaves = |p: usize| (moved.a[p] as usize == j && moved.mol[gi * pairs + p] > 0.0) || (moved.b[p] as usize == j && moved.mol[gi * pairs + p] < 0.0);
        let from: Vec<usize> = (0..pairs).filter(|&p| leaves(p)).collect();
        let out: f64 = from.iter().map(|&p| moved.mol[gi * pairs + p].abs()).sum();
        let keep = if out > 0.0 { ((out + end) / out).max(0.0) } else { 1.0 };
        from.into_iter().for_each(|p| moved.mol[gi * pairs + p] *= keep);
    }
}

/// The gases that leave the ground or may condense (not those of `henry`, which dissolve), with their saturation
/// pressures up to the warmest ground or air node plus `SAT_REACH` (`Scratch::sat`).
fn active(w: &World, s: &mut Scratch, henry: &[(usize, usize)]) -> Vec<Active> {
    let tmax = range(&s.state.tg).1;
    let hi = tmax.max(range(&s.ta).1) + SAT_REACH;
    s.sat.resize(w.chem.ns(), Default::default());
    let mut act = Vec::new();
    for (gi, &k) in s.state.gas.iter().enumerate() {
        let up = s.state.loose.contains(&k) && may_evaporate(w, k, tmax) && !henry.iter().any(|h| h.1 == k);
        if up || s.cond[gi] {
            s.sat[k].fit(&w.chem, k, hi);
            act.push(Active { gi, k, up, cond: s.cond[gi] });
        }
    }
    act
}
