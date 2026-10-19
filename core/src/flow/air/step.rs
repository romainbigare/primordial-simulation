//! One step of the air: radiation, the exchange with the ground (evaporation and rain) and the wind, solved together
//! by backward Euler over the step, so a long step gives the world of short ones.
//!
//! The unknowns are each air node's temperature and gases at the end of the step. Each round reads them, finds what
//! every ground node exchanges with its air node over the step (`cell`: its surface balance, solved implicitly, the
//! rain it gets and what evaporates from it) and how that changes with its air node's temperature and gas, then solves
//! the air nodes' gas and heat with the wind in one implicit step each (`solve::band`), the ground's exchange put in by
//! its slopes (Newton's method on each node's own terms; the wind's pushes and the coupling of gas and heat from the
//! round before). The rounds stop once the air no longer moves. What the last round found crosses every link of the
//! step and is booked in one call (`book`), so the books are exact whether or not the rounds have converged.

use super::super::land::{melted, NONE};
use super::super::scratch::may_evaporate;
use super::super::{range, Scratch};
use super::book::record;
use super::exchange::{bucket_slope, cloud, dissolve};
use super::newton::{gather, solve, Wind};
use super::radiation::{insolation, Surface};
use super::wind::Links;
use super::{lerp, over_cell};
use crate::units::R_GAS;
use crate::{World, AIR};
use std::f64::consts::TAU;

/// The rounds stop when no air node's temperature moves by more than this from one round to the next, K,
const STILL_K: f64 = 5e-2;
/// and no gas of an air node by more than this share of the node's gas,
const STILL_SHARE: f64 = 1e-5;
/// or after this many rounds.
const ROUNDS: usize = 16;
/// A round moves the air at most so far toward its solve (a damped Newton step, Deuflhard 2004): no node's temperature
/// by more than `REACH_K`, K; and only half as far as the round before when the solve moved the air no less than then
/// (the rounds cross a kink of rain back and forth), twice as far again after two rounds that moved it less. Each gas
/// of a node then keeps at least this share of what it held (a bound on that one unknown, so a gas the step empties
/// from a node does not hold back the others).
const KEEP: f64 = 0.5;
const REACH_K: f64 = 10.0;
/// A ground node's surface balance is solved again about the temperature it gives, up to this many times, until that
/// moves by less than `SURFACE_STILL_K`, K: so what evaporates is the evaporation the balance holds.
const SURFACE_ROUNDS: usize = 3;
const SURFACE_STILL_K: f64 = 1e-3;
/// The saturation tables reach this far above the warmest ground or air node at the start of a step, K: no node warms
/// more in a step.
const SAT_REACH: f64 = 60.0;
/// A gas making up less than this share of the air is a trace (`Newton::traces`).
const TRACE_AIR: f64 = 1e-7;

/// A gas that may leave the ground or condense in the air: its index in `Scratch::gas` and its substance; whether it
/// may evaporate, and whether it may condense; and, if it is more than a trace, its place among the gases solved with
/// the heat (`Newton::coupled`).
#[derive(Clone, Copy)]
pub(super) struct Active {
    pub(super) gi: usize,
    pub(super) k: usize,
    pub(super) up: bool,
    pub(super) cond: bool,
    pub(super) field: Option<usize>,
}

/// The numbers of a ground node's row (`cell`): the heat it gives its air node, J, and its slope with the air's
/// temperature, J/K; the heat that leaves to space through the air, J; the sunlight the ground absorbs, J; its
/// temperature at the end of the step, K; the light each unit of capture area of its bodies gets, J/m²; the light set
/// aside for them, W/m²; then per active gas `PER_GAS` numbers.
pub(super) const FIXED: usize = 7;
/// Per active gas: what evaporates, mol, and its slopes with the gas in the column the node sees, mol/mol, and with the
/// temperature of the air touching it, mol/K; what rains, mol, and its two slopes; and the slope of the heat the node
/// gives its air node with the gas in the column, J/mol (what evaporates changes the node's temperature).
pub(super) const PER_GAS: usize = 7;

/// The air during the rounds: per node the temperature, K, and the reflectance of its clouds; per gas and node
/// (`[gas * nodes + node]`) the amount, mol.
#[derive(Clone)]
pub(super) struct Iterate {
    pub(super) t: Vec<f64>,
    pub(super) x: Vec<f64>,
    pub(super) refl: Vec<f64>,
}

/// What a round's cells read.
struct Ctx<'a> {
    w: &'a World,
    s: &'a Scratch,
    it: &'a Iterate,
    act: &'a [Active],
    /// Temperature each ground node ended the last round at, K (the point its exchange is linearised about).
    ts: &'a [f64],
    /// What does not change in a step, per ground node (`Fixed`).
    fixed: &'a [f64],
    dt: f64,
    decl: f64,
}

/// Per ground node, what its rows read that does not change in a step: the share of the air it sees that lies over it
/// (`air::over_cell`); the share of sunlight its bodies intercept (`cover`), and absorb, and the share of that their
/// light reactions store; then per active gas the height it fills near the ground, m.
const FIXED_CELL: usize = 4;

/// The numbers of `FIXED_CELL` for every ground node.
fn fixed(w: &World, s: &Scratch, act: &[Active]) -> Vec<f64> {
    let (n, na, per) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), FIXED_CELL + act.len());
    let mut out = vec![0.0; n * per];
    for (i, f) in out.chunks_exact_mut(per).enumerate() {
        let a = w.canopy[i] as f64;
        let cover = if a > 0.0 { crate::math::one_minus_exp(a) } else { 0.0 };
        f[..FIXED_CELL].copy_from_slice(&[over_cell(w, s, i), cover, cover * w.life.absorb.get(i).map_or(0.0, |&x| x as f64), w.life.keep.get(i).map_or(0.0, |&x| x as f64)]);
        for (x, ac) in act.iter().enumerate() {
            f[FIXED_CELL + x] = lerp(s, &s.hk[ac.gi * na..(ac.gi + 1) * na], i);
        }
    }
    out
}

/// The air's step of `dt` seconds: gases dissolve (`exchange::dissolve`, slow, from the air as it is), then radiation,
/// evaporation, rain and the wind together, in rounds (see the top of this file).
pub(crate) fn step(w: &mut World, s: &mut Scratch, dt: f64) {
    let (tmin, tmax) = range(&s.state.tg);
    let henry: Vec<(usize, usize)> = s.state.gas.iter().copied().enumerate().filter(|&(_, k)| s.sp[k].boil < tmin && w.chem.subs[k].solubility > 0.0).collect();
    dissolve(w, s, dt, &henry, (tmin, tmax));
    let act = active(w, s, &henry);
    let (na, n) = (w.mesh.level(AIR).n(), w.mesh.cells().n());
    // The sun at the end of the step, as backward Euler takes it (and as the state at the step's end then shows).
    let season = w.cfg.calendar().season(w.time + dt as u64);
    let decl = -w.cfg.tilt.to_radians() * (TAU * season).cos();
    let x0: Vec<f64> = s.state.gas.iter().flat_map(|&k| w.air[k * na..(k + 1) * na].iter().copied()).collect();
    let mut it = Iterate { t: s.ta.clone(), x: x0.clone(), refl: Vec::new() };
    let links = Links::of(w.mesh.level(AIR));
    let (mut ts, mut rows, mut wind) = (s.state.tg.clone(), std::mem::take(&mut s.rad), Wind::default());
    let width = FIXED + PER_GAS * act.len();
    rows.resize(n * width, 0.0);
    let (mut damp, mut last, mut shrunk) = (1.0_f64, f64::INFINITY, 0);
    let fixed = fixed(w, s, &act);
    for round in 0.. {
        pass(&Ctx { w, s, it: &it, act: &act, ts: &ts, fixed: &fixed, dt, decl }, &mut rows, width);
        for (i, t) in ts.iter_mut().enumerate() {
            *t = rows[i * width + 4];
        }
        let g = gather(w, s, &it, &act, &rows, width);
        it.refl = g.rained.iter().zip(w.mesh.level(AIR).area()).map(|(&r, &a)| cloud(w.cfg.cloud, (r[0], r[1]), a, dt)).collect();
        let next = solve(w, s, (&act, &links), (&x0, &g), &it, dt, &mut wind);
        let (moved_t, moved_x) = moved(&next, &it);
        if (moved_t < STILL_K && moved_x < STILL_SHARE) || round + 1 == ROUNDS {
            record(w, s, &act, (&rows, width), &wind, &it, dt);
            break;
        }
        let moved = moved_t / STILL_K + moved_x / STILL_SHARE;
        shrunk = if moved >= last { 0 } else { shrunk + 1 };
        damp = if shrunk == 0 { damp / 2.0 } else if shrunk % 2 == 0 { (2.0 * damp).min(1.0) } else { damp };
        last = moved;
        let step = damp.min(REACH_K / moved_t.max(f64::MIN_POSITIVE)).min(1.0);
        let to = |a: &[f64], b: &[f64], keep: f64| a.iter().zip(b).map(|(&u, &v)| if step < 1.0 { v + step * (u - v) } else { u }.max(keep * v)).collect();
        it = Iterate { t: to(&next.t, &it.t, 0.0), x: to(&next.x, &it.x, KEEP), refl: std::mem::take(&mut it.refl) };
    }
    s.rad = rows;
}

/// How far the solve moved the air from `it` to `next`: the most of any node's temperature, K, and the most of any
/// gas of a node, as a share of the node's gas.
fn moved(next: &Iterate, it: &Iterate) -> (f64, f64) {
    let na = it.t.len();
    let ng = it.x.len() / na.max(1);
    let t = next.t.iter().zip(&it.t).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
    let x = (0..na).map(|j| (0..ng).map(|q| (next.x[q * na + j] - it.x[q * na + j]).abs()).fold(0.0, f64::max) / (0..ng).map(|q| next.x[q * na + j]).sum::<f64>()).fold(0.0, f64::max);
    (t, x)
}

/// The gases that leave the ground or may condense (not those of `henry`, which dissolve), with their saturation
/// pressures up to the warmest ground or air node plus `SAT_REACH` (`Scratch::sat`).
fn active(w: &World, s: &mut Scratch, henry: &[(usize, usize)]) -> Vec<Active> {
    let tmax = range(&s.state.tg).1;
    let hi = tmax.max(range(&s.ta).1) + SAT_REACH;
    s.sat.resize(w.chem.ns(), Default::default());
    let (na, all) = (w.mesh.level(AIR).n(), w.air.iter().sum::<f64>());
    let (mut act, mut fields) = (Vec::new(), 0);
    for (gi, &k) in s.state.gas.iter().enumerate() {
        let up = s.state.loose.contains(&k) && may_evaporate(w, k, tmax) && !henry.iter().any(|h| h.1 == k);
        if up || s.cond[gi] {
            s.sat[k].fit(&w.chem, k, hi);
            let trace = w.air[k * na..(k + 1) * na].iter().sum::<f64>() < TRACE_AIR * all;
            act.push(Active { gi, k, up, cond: s.cond[gi], field: (!trace).then_some(fields) });
            fields += !trace as usize;
        }
    }
    act
}

/// Every ground node's row (`cell`) from the air of the round, in rows (in parallel, with `threads`).
fn pass(cx: &Ctx, rows: &mut [f64], width: usize) {
    let (cfg, lat) = (&cx.w.cfg, cx.w.mesh.cells().lat());
    // Each task keeps the last latitude and its sunlight: neighbouring cells often share a latitude (a row of the
    // square map), and the sunlight is the same function of it.
    crate::par::rows(rows, width, || (f64::NAN, 0.0), |last, i, row| {
        if lat[i] != last.0 {
            *last = (lat[i], insolation(cfg.sun, lat[i].to_radians(), cx.decl));
        }
        cell(cx, i, last.1, row);
    });
}

/// The row of ground node `i` under the sunlight `q` of its latitude, W/m² (`FIXED`, `PER_GAS`). Sunlight is reflected by
/// what covers the ground and by clouds; bodies with light-capture area `a` per ground area intercept a share 1 -
/// exp(-a) of what passes the clouds, and what they absorb warms the surface now but for the share their light reactions
/// store (as at their last update, `Life::keep`): that is set aside, and each takes its share when it updates
/// (`body::live`); taller members shade smaller ones (`survey::shade`). Then rain: where the air the cell sees (colder
/// on higher ground, by the drop of the air's temperature with height) holds more of a gas than it can at the
/// temperature of the air touching the ground, the rest rains out in `rain_time`. Then the surface balance
/// (`radiation::Surface`), with what evaporates (`evaporate`) and the heat the rain brings, linearised about the
/// temperature of the round before.
fn cell(cx: &Ctx, i: usize, q: f64, row: &mut [f64]) {
    let (w, s, it, dt) = (cx.w, cx.s, cx.it, cx.dt);
    let (cells, na, n) = (w.mesh.cells(), w.mesh.level(AIR).n(), w.mesh.cells().n());
    let (p, area) = (cells.parent()[i] as usize, cells.area()[i]);
    let adt = area * dt;
    let [gam, zair, _] = s.seen[i];
    let tl = (lerp(s, &it.t, i) - gam * (s.head[i] - zair)).max(1.0);
    let c = if it.refl.is_empty() { s.state.cloud[i] } else { lerp(s, &it.refl, i) };
    let fixed = &cx.fixed[i * (FIXED_CELL + cx.act.len())..(i + 1) * (FIXED_CELL + cx.act.len())];
    let [over, cover, caught, keep] = <[f64; FIXED_CELL]>::try_from(&fixed[..FIXED_CELL]).unwrap();
    // (Per unit of capture area the bodies intercept cover / a of the light, all of it where they are few.)
    let per_area = if cover > 0.0 { cover / w.canopy[i] as f64 } else { 1.0 };
    let (dl, kept) = (q * (1.0 - c) * per_area * dt, q * (1.0 - c) * caught * keep);
    let sw = q * (1.0 - c) * ((1.0 - cover) * (1.0 - s.alb[i]) + caught) - kept;
    // Rain, and its heat into the node's temperature: it brings its heat at the air's temperature and its fusion
    // enthalpy, of which the node holds what stays molten (`Scratch::fusion`). Until the balance is solved, the first
    // number of each gas holds the partial pressure of its vapour near the ground, Pa (it goes with the column and the
    // temperature).
    let (mut gain, mut g_rain) = (0.0, 0.0);
    for (x, ac) in cx.act.iter().enumerate() {
        let (o, sp) = (FIXED + PER_GAS * x, s.sp[ac.k]);
        let (col, hk) = (lerp(s, &it.x[ac.gi * na..(ac.gi + 1) * na], i) * over, fixed[FIXED_CELL + x]);
        row[o..o + PER_GAS].copy_from_slice(&[col * R_GAS * tl / (area * hk), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let full = s.sat[ac.k].at(tl) * area * hk / (R_GAS * tl);
        if ac.cond && col > full {
            let f = dt / w.cfg.rain_time;
            let r = f * (col - full);
            row[o + 3..o + 6].copy_from_slice(&[r, f, -f * full * (sp.lv / (R_GAS * tl * tl) - 1.0 / tl)]);
            g_rain += r * sp.cp / adt;
            gain += (r * sp.lf - s.fusion(ac.k, i, r)) / adt;
        }
    }
    // What evaporates at `t` (`evaporate`), and the heat it takes from the node's temperature.
    let evap = |t: f64, x: usize, row: &[f64]| {
        let o = FIXED + PER_GAS * x;
        evaporate(cx, i, x, t, row[o], row[o + 3])
    };
    let lat = |x: usize, m: f64| {
        let sp = s.sp[cx.act[x].k];
        m * (sp.lf + sp.lv) - s.fusion(cx.act[x].k, i, m)
    };
    let pk = s.state.pk[i];
    let fusion = if pk != NONE { w.ground[pk * n + i] * s.sp[pk].lf } else { 0.0 };
    // The balance with what evaporates linearised about `t0`; and whether anything evaporates there.
    let balance = |t0: f64| {
        let (mut gain, mut slope) = (gain, 0.0);
        for x in (0..cx.act.len()).filter(|&x| cx.act[x].up) {
            let (m, dm_dt, _) = evap(t0, x, row);
            gain -= lat(x, m) / adt;
            slope -= lat(x, dm_dt) / adt;
        }
        (Surface { sw, tl, t0, gain, slope: slope.min(0.0), t_rain: it.t[p], g_rain }.balance(s, (i, area, dt), w.cfg.sensible, fusion), slope != 0.0)
    };
    let ((mut bal, evaporates), mut from) = (balance(cx.ts[i]), cx.ts[i]);
    for _ in 1..SURFACE_ROUNDS {
        if !evaporates || (bal.ts - from).abs() < SURFACE_STILL_K {
            break;
        }
        from = bal.ts;
        bal = balance(from).0;
    }
    // What evaporates, and its slopes: the partial pressure goes with the column and with the temperature of the air
    // touching the ground, and what evaporates more cools the node, which then evaporates less and warms its air less.
    let mut row_slope = bal.slope;
    for x in 0..cx.act.len() {
        let o = FIXED + PER_GAS * x;
        let pa = row[o];
        let (m, dm_dt, dm_dpa) = if cx.act[x].up { evap(bal.ts, x, row) } else { (0.0, 0.0, 0.0) };
        let col = pa * area * fixed[FIXED_CELL + x] / (R_GAS * tl);
        let cools = bal.give * lat(x, dm_dpa) / adt;
        let (by_col, by_tl) = (pa / col.max(f64::MIN_POSITIVE), pa / tl);
        let (follow_col, follow_tl) = (-cools * by_col, -cools * by_tl);
        row[o..o + 3].copy_from_slice(&[m, dm_dpa * by_col + dm_dt * follow_col, dm_dpa * by_tl + dm_dt * follow_tl]);
        row[o + 6] = bal.warm * follow_col;
        row_slope += bal.warm * follow_tl;
    }
    row[..FIXED].copy_from_slice(&[bal.to_air, row_slope, bal.to_space, adt * sw, bal.ts, dl, kept]);
}

/// What evaporates of active gas `x` from ground node `i` over the step when the node ends it at `t`, the vapour's
/// partial pressure near the ground being `pa` and `rain` mol of it raining on the node, mol (`exchange::bucket_slope`), and
/// its slopes with `t`, mol/K, and with `pa`, mol/Pa: by the difference of the saturation pressure at `t` and `pa`
/// through the air's film (`evap`), the open rate while the ground is wet; a node over the boiling point gives all of it.
fn evaporate(cx: &Ctx, i: usize, x: usize, t: f64, pa: f64, rain: f64) -> (f64, f64, f64) {
    let (w, s, ac) = (cx.w, cx.s, cx.act[x]);
    let (n, sp, area) = (w.mesh.cells().n(), s.sp[ac.k], w.mesh.cells().area()[i]);
    let a = w.ground[ac.k * n + i];
    if a + rain <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    if s.state.tg[i] >= sp.boil {
        return (a + rain, 0.0, 0.0);
    }
    let (ps, c, v) = (s.sat[ac.k].at(t), w.cfg.evap / R_GAS * area, sp.v[melted(s, ac.k, i) as usize]);
    let (e, wet) = (c * (ps - pa) / t, w.cfg.phys.field * s.pd[i] * area / v);
    if e <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let (m, dm_de) = bucket_slope(a, rain, e, wet, cx.dt);
    // The open rate's slope with `t` (Clausius-Clapeyron), and with the gas through the partial pressure.
    let de_dt = c * (ps * sp.lv / (R_GAS * t * t) - (ps - pa) / t) / t;
    (m, dm_de * de_dt, -dm_de * c / t)
}
