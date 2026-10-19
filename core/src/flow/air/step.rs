//! One step of the air: radiation, the exchange with the ground (evaporation and rain) and the wind, solved together
//! by backward Euler, so a long step gives the world of short ones (laws.md 1.4). Each round finds every ground node's
//! exchange and its slopes (`cell`), then solves the air level (`newton`); what the last round found is booked whole
//! (`book`), so the books are exact whether or not the rounds converged.

use super::super::land::{melted, NONE};
use super::super::scratch::may_evaporate;
use super::super::{range, Scratch};
use super::book::book;
use super::exchange::{bucket_slope, dissolve};
use super::newton::{far, gather, solve, Drive, Wind};
use super::radiation::{declination, insolation, Surface};
use super::{lerp, over_cell, shares};
use crate::ground::heat_above_floor;
use crate::store::{apply, Batch};
use crate::units::R_GAS;
use crate::{World, AIR};
use crate::store::Class;

/// The rounds stop when no air node's temperature moves by more than this from one round to the next, K,
const STILL_K: f64 = 5e-2;
/// and no gas of an air node by more than this share of the node's gas,
const STILL_SHARE: f64 = 1e-5;
/// or after this many rounds.
const ROUNDS: usize = 16;
/// A round leaves each gas of a node at least this share of what it held, a bound on that one unknown alone.
const KEEP: f64 = 0.5;
/// A ground node's surface balance is solved again about the temperature it gives, up to this many times, until that
/// moves by less than `SURFACE_STILL_K`, K.
const SURFACE_ROUNDS: usize = 3;
const SURFACE_STILL_K: f64 = 1e-3;
/// The saturation tables reach this far above the warmest node at a step's start, K: no node warms more in a step.
const SAT_REACH: f64 = 60.0;
/// A gas making up less than this share of the air is a trace (`Newton::traces`).
const TRACE_AIR: f64 = 1e-7;

/// A gas that may leave the ground or condense: its index in `Scratch::gas`, its substance, whether it may evaporate or
/// condense, and (more than a trace) its place among the gases solved with the heat.
#[derive(Clone, Copy)]
pub(super) struct Active {
    pub(super) gi: usize,
    pub(super) k: usize,
    pub(super) up: bool,
    pub(super) cond: bool,
    pub(super) field: Option<usize>,
}

/// A ground node's row (`cell`): the heat it gives its air node, J, and its slope with the air's temperature, J/K; the
/// heat to space through the air, J; the sunlight it absorbs, J; its end temperature, K; the light per unit of its
/// bodies' capture area, J/m²; the light set aside for them, W/m²; then `PER_GAS` numbers per active gas.
pub(super) const FIXED: usize = 7;
/// Per active gas: what evaporates, mol, and its slopes with the column's gas and the air's temperature; what rains and
/// its two slopes; the slope of the heat the node gives its air with the column's gas, J/mol.
pub(super) const PER_GAS: usize = 7;

/// The air during the rounds: per node the temperature, K; per gas and node (`[gas * nodes + node]`) the amount, mol.
#[derive(Clone)]
pub(super) struct Iterate {
    pub(super) t: Vec<f64>,
    pub(super) x: Vec<f64>,
}

/// The air at the step's start with the gas of the bodies' nodes, which comes up into each cell's air node over the
/// step (laws.md 1.5): per gas and node (`[gas * nodes + node]`) the amount, mol; per node its heat less the lift of
/// that gas, J, and the lift, J.
pub(super) struct Start {
    pub(super) x: Vec<f64>,
    pub(super) e: Vec<f64>,
    pub(super) lift: Vec<f64>,
}

impl Start {
    fn of(w: &World, s: &Scratch) -> Start {
        let (n, na, g, parent) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.cfg.gravity, w.mesh.cells().parent());
        let (mut all, mut lift) = (w.store.amounts(Class::Air).to_vec(), vec![0.0; na]);
        for (at, &m) in w.store.amounts(Class::Bodies).iter().enumerate().filter(|e| *e.1 != 0.0) {
            let (k, i, j) = (at / n, at % n, parent[at % n] as usize);
            all[k * na + j] += m;
            lift[j] += m * s.sp[k].mm * s.view().lift(i, j, g);
        }
        let x = s.state.gas.iter().flat_map(|&k| all[k * na..(k + 1) * na].iter().copied()).collect();
        Start { x, e: w.store.heat(Class::Air).iter().zip(&lift).map(|(e, l)| e - l).collect(), lift }
    }
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
    /// Heat each ground node gives its bodies' node over the step, J (`settle`).
    settle: &'a [f64],
    dt: f64,
    decl: f64,
}

/// Per ground node, what does not change in a step: the share of the air it sees over it; the share of sunlight its
/// bodies intercept and absorb, and the share their light reactions store; per active gas its height near the ground, m.
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

/// Each bodies' node settles with its ground toward what its bodies claim at the end of a step to `t_end`
/// (`survey::claims`; the light set aside for them in the step comes from the sun, laws.md 1.4). What no body claims
/// goes back to the ground now, so the step starts from it (a large return, as when many bodies die, would take the
/// ground's balance far from the temperature it is linearised about); returns the heat each ground node gives its node
/// over the step, J, no more than it holds above `T_FLOOR`.
pub(crate) fn settle(w: &mut World, t_end: f64) -> Vec<f64> {
    let claims = crate::survey::claims(w, t_end);
    let (mut back, mut draw) = (Batch::new([Class::Bodies, Class::Ground], &[]), vec![0.0; claims.len()]);
    for (i, &c) in claims.iter().enumerate() {
        match c - w.store.heat(Class::Bodies)[i] {
            d if d < 0.0 => _ = back.link((i, i), -d),
            d => draw[i] = d.min(heat_above_floor(w.store.heat(Class::Ground)[i], w.heat_cap_ground(i))),
        }
    }
    apply(w, &[back.flux()], &[], &[]);
    draw
}

/// The air's step of `dt` seconds, with the heat each ground node gives its bodies' node (`settle`): gases dissolve
/// (from the air as it is), then radiation, evaporation, rain and the wind together, in rounds.
pub(crate) fn step(w: &mut World, s: &mut Scratch, dt: f64, settle: &[f64]) {
    let (tmin, tmax) = range(&s.state.tg);
    let henry: Vec<(usize, usize)> = s.state.gas.iter().copied().enumerate().filter(|&(_, k)| s.sp[k].boil < tmin && w.chem.subs[k].solubility > 0.0).collect();
    dissolve(w, s, dt, &henry, (tmin, tmax));
    let act = active(w, s, &henry);
    let n = w.mesh.cells().n();
    // The sun at the end of the step, as backward Euler takes it (and as the state at the step's end then shows).
    let decl = declination(w.cfg.tilt, w.cfg.calendar().season(w.time + dt as u64));
    let start = Start::of(w, s);
    let mut it = Iterate { t: s.ta.clone(), x: start.x.clone() };
    let drive = Drive::of(w, s, &start.x, dt);
    let (mut ts, mut rows, mut wind) = (s.state.tg.clone(), std::mem::take(&mut s.rad), Wind::default());
    let width = FIXED + PER_GAS * act.len();
    rows.resize(n * width, 0.0);
    let (fixed, far) = (fixed(w, s, &act), far(s));
    for round in 0.. {
        pass(&Ctx { w, s, it: &it, act: &act, ts: &ts, fixed: &fixed, settle, dt, decl }, &mut rows, width);
        ts.iter_mut().zip(rows.chunks_exact(width)).for_each(|(t, row)| *t = row[4]);
        let g = gather(w, s, &it, &act, (&rows, width), far);
        let next = solve(w, s, (&act, &drive), (&start, &g), &it, dt, &mut wind);
        let (moved_t, moved_x) = moved(&next, &it);
        if (moved_t < STILL_K && moved_x < STILL_SHARE) || round + 1 == ROUNDS {
            book(w, s, (&act, &g.rained, settle), (&rows, width), &wind, (&it, &start), dt);
            break;
        }
        let x = next.x.iter().zip(&it.x).map(|(&u, &v)| u.max(KEEP * v)).collect();
        it = Iterate { t: next.t, x };
    }
    s.rad = rows;
}

/// How far the solve moved the air from `it` to `next`: the most of any node's temperature, K, and the most of any
/// gas of a node, as a share of the node's gas.
fn moved(next: &Iterate, it: &Iterate) -> (f64, f64) {
    let (na, ng) = (it.t.len(), it.x.len() / it.t.len().max(1));
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
    let (na, all) = (w.mesh.level(AIR).n(), w.store.amounts(Class::Air).iter().sum::<f64>());
    let (mut act, mut fields) = (Vec::new(), 0);
    for (gi, &k) in s.state.gas.iter().enumerate() {
        let up = s.state.loose.contains(&k) && may_evaporate(w, k, tmax) && !henry.iter().any(|h| h.1 == k);
        if up || s.cond[gi] {
            s.sat[k].fit(&w.chem, k, hi);
            let trace = w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().sum::<f64>() < TRACE_AIR * all;
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

/// The row of ground node `i` under the sunlight `q` of its latitude, W/m²: sunlight past the clouds, what bodies
/// intercept less the share their light reactions store (set aside for them, laws.md 1.4); rain, where the air the cell
/// sees (colder on higher ground) holds more of a gas than it can, in `rain_time`; then the surface balance with what
/// evaporates, the rain's heat and the heat its bodies' node takes (`settle`), linearised about the round before.
fn cell(cx: &Ctx, i: usize, q: f64, row: &mut [f64]) {
    let (w, s, it, dt) = (cx.w, cx.s, cx.it, cx.dt);
    let (cells, na, n) = (w.mesh.cells(), w.mesh.level(AIR).n(), w.mesh.cells().n());
    let area = cells.area()[i];
    let adt = area * dt;
    let [gam, zair, _] = s.seen[i];
    let tl = (lerp(s, &it.t, i) - gam * (s.head[i] - zair)).max(1.0);
    let c = s.state.cloud[i];
    let fixed = &cx.fixed[i * (FIXED_CELL + cx.act.len())..(i + 1) * (FIXED_CELL + cx.act.len())];
    let [over, cover, caught, keep] = <[f64; FIXED_CELL]>::try_from(&fixed[..FIXED_CELL]).unwrap();
    // Per unit of capture area the bodies intercept cover / a of the light, all of it where they are few.
    let per_area = if cover > 0.0 { cover / w.canopy[i] as f64 } else { 1.0 };
    let (dl, kept) = (q * (1.0 - c) * per_area * dt, q * (1.0 - c) * caught * keep);
    let sw = q * (1.0 - c) * ((1.0 - cover) * (1.0 - s.alb[i]) + caught) - kept;
    // Rain brings its heat at the temperature of the air it comes from (`shares`) and its fusion enthalpy, of which the
    // node keeps what stays molten. Until the balance is solved, each gas's first number holds its vapour's partial
    // pressure near the ground, Pa.
    let (mut gain, mut g_rain, mut rain_heat) = (-cx.settle[i] / adt, 0.0, 0.0);
    for (x, ac) in cx.act.iter().enumerate() {
        let (o, sp) = (FIXED + PER_GAS * x, s.sp[ac.k]);
        let (col, hk) = (lerp(s, &it.x[ac.gi * na..(ac.gi + 1) * na], i) * over, fixed[FIXED_CELL + x]);
        row[o..o + PER_GAS].copy_from_slice(&[col * R_GAS * tl / (area * hk), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let full = s.sat[ac.k].at(tl) * area * hk / (R_GAS * tl);
        if ac.cond && col > full {
            let f = dt / w.cfg.rain_time;
            let r = f * (col - full);
            row[o + 3..o + 6].copy_from_slice(&[r, f, -f * full * (sp.lv / (R_GAS * tl * tl) - 1.0 / tl)]);
            let t_from: f64 = shares(s, &it.x[ac.gi * na..(ac.gi + 1) * na], i).iter().map(|&(q, share)| share * it.t[q]).sum();
            (g_rain, rain_heat) = (g_rain + r * sp.cp / adt, rain_heat + r * sp.cp / adt * t_from);
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
    let fusion = if pk != NONE { w.store.amounts(Class::Ground)[pk * n + i] * s.sp[pk].lf } else { 0.0 };
    // The balance with what evaporates linearised about `t0`; and whether anything evaporates there.
    let balance = |t0: f64| {
        let (mut gain, mut slope) = (gain, 0.0);
        for x in (0..cx.act.len()).filter(|&x| cx.act[x].up) {
            let (m, dm_dt, _) = evap(t0, x, row);
            gain -= lat(x, m) / adt;
            slope -= lat(x, dm_dt) / adt;
        }
        (Surface { sw, tl, t0, gain, slope: slope.min(0.0), t_rain: if g_rain > 0.0 { rain_heat / g_rain } else { 0.0 }, g_rain }.balance(s, (i, area, dt), w.cfg.sensible, fusion), slope != 0.0)
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
    // touching the ground, the node's temperature follows that air (`Balance::follow`), and what evaporates more cools
    // the node, which then evaporates less and warms its air less.
    let mut row_slope = bal.slope;
    for x in 0..cx.act.len() {
        let o = FIXED + PER_GAS * x;
        let pa = row[o];
        let (m, dm_dt, dm_dpa) = if cx.act[x].up { evap(bal.ts, x, row) } else { (0.0, 0.0, 0.0) };
        let col = pa * area * fixed[FIXED_CELL + x] / (R_GAS * tl);
        let cools = bal.give * lat(x, dm_dpa) / adt;
        let (by_col, by_tl) = (pa / col.max(f64::MIN_POSITIVE), pa / tl);
        let (follow_col, follow_tl) = (-cools * by_col, -cools * by_tl);
        row[o..o + 3].copy_from_slice(&[m, dm_dpa * by_col + dm_dt * follow_col, dm_dpa * by_tl + dm_dt * (bal.follow + follow_tl)]);
        row[o + 6] = bal.warm * follow_col;
        row_slope += bal.warm * follow_tl;
    }
    row[..FIXED].copy_from_slice(&[bal.to_air, row_slope, bal.to_space, adt * sw, bal.ts, dl, kept]);
}

/// What evaporates of active gas `x` from ground node `i` over the step, ending at `t` under a vapour pressure `pa` with
/// `rain` mol falling on it, mol, and its slopes with `t` and `pa`: the saturation deficit through the air's film
/// (`evap`), at the open rate while wet (`exchange::bucket_slope`); over the boiling point, all of it.
fn evaporate(cx: &Ctx, i: usize, x: usize, t: f64, pa: f64, rain: f64) -> (f64, f64, f64) {
    let (w, s, ac) = (cx.w, cx.s, cx.act[x]);
    let (n, sp, area) = (w.mesh.cells().n(), s.sp[ac.k], w.mesh.cells().area()[i]);
    let a = w.store.amounts(Class::Ground)[ac.k * n + i];
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
