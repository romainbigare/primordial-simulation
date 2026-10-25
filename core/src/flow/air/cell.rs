//! The row of one ground node in a round of the air's step (`step::pass`): sunlight, rain and the surface balance with
//! what evaporates, linearised about the round's air with its slopes (`step::FIXED`, `step::PER_GAS`).

use super::super::land::{layers, melted, reflectance, thaw_at, NONE};
use super::super::Scratch;
use super::exchange::bucket_slope;
use super::radiation::Surface;
use super::step::{Active, Iterate, FIXED, PER_GAS};
use super::{lerp, over_cell, shares};
use crate::{survey::Draws, store::Class, units::{MAX_SUBSTANCES, R_GAS}, World, AIR};

/// A ground node's surface balance is solved again about the temperature it gives, up to this many times, until that
/// moves by less than `SURFACE_STILL_K`, K.
const SURFACE_ROUNDS: usize = 8;
const SURFACE_STILL_K: f64 = 1e-3;
/// The reflectance a ground node's balance takes is guessed again up to this many times, until it moves by less than
/// `REFLECT_STILL` from the one its end state gives.
const REFLECT_ROUNDS: usize = 8;
const REFLECT_STILL: f64 = 1e-3;

/// What a round's cells read.
pub(super) struct Ctx<'a> {
    pub(super) w: &'a World,
    pub(super) s: &'a Scratch,
    pub(super) it: &'a Iterate,
    pub(super) act: &'a [Active],
    /// Temperature each ground node ended the last round at, K (the point its exchange is linearised about).
    pub(super) ts: &'a [f64],
    /// Reflectance of each ground node's surface at the end state the last round found (the step's start before the
    /// first).
    pub(super) alb: &'a [f64],
    /// What does not change in a step, per ground node (`Fixed`).
    pub(super) fixed: &'a [f64],
    /// The heat the bodies of each cell draw from its ground, and how much heat each ground node holds above `T_FLOOR`, J.
    pub(super) draws: &'a Draws,
    pub(super) room: &'a [f64],
    pub(super) dt: f64,
    pub(super) decl: f64,
}

/// Per ground node, what does not change in a step: the share of the air it sees over it; the share of sunlight its
/// bodies intercept and absorb, and the share their light reactions store; per active gas its height near the ground, m.
pub(super) const FIXED_CELL: usize = 4;

/// The numbers of `FIXED_CELL` for every ground node.
pub(super) fn fixed(w: &World, s: &Scratch, act: &[Active]) -> Vec<f64> {
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

/// The row of ground node `i` under the sunlight `q` of its latitude, W/m²: sunlight past the clouds, what bodies
/// intercept less the share their light reactions store (set aside for them, laws.md 1.4); rain (`rain`); then the
/// surface balance (`exchange`) under the reflectance its surface ends the step with, as backward Euler takes it: from
/// the round before's, each guess followed to the end it gives, and halved between two once a guess and its end cross,
/// until they agree.
pub(super) fn cell(cx: &Ctx, i: usize, q: f64, row: &mut [f64]) {
    let (w, s, it, dt) = (cx.w, cx.s, cx.it, cx.dt);
    let [gam, zair, _] = s.seen[i];
    let tl = (lerp(s, &it.t, i) - gam * (s.head[i] - zair)).max(1.0);
    let c = s.state.cloud[i];
    let [over, cover, caught, keep] = <[f64; FIXED_CELL]>::try_from(&cx.fixed[i * (FIXED_CELL + cx.act.len())..][..FIXED_CELL]).unwrap();
    // Per unit of capture area the bodies intercept cover / a of the light, all of it where they are few.
    let per_area = if cover > 0.0 { cover / w.canopy[i] as f64 } else { 1.0 };
    let (dl, kept) = (q * (1.0 - c) * per_area * dt, q * (1.0 - c) * caught * keep);
    let fall = rain(cx, i, tl, over, row);
    let sw = |a: f64| q * (1.0 - c) * ((1.0 - cover) * (1.0 - a) + caught) - kept;
    row[5..7].copy_from_slice(&[dl, kept]);
    let gas = row[FIXED..].to_vec();
    let (mut a, mut lo, mut hi) = (cx.alb[i], f64::NAN, f64::NAN);
    for _ in 0..REFLECT_ROUNDS {
        row[FIXED..].copy_from_slice(&gas);
        let end = exchange(cx, i, (tl, sw(a)), &fall, row);
        if (end - a).abs() < REFLECT_STILL {
            break;
        }
        (lo, hi) = if end > a { (a, hi) } else { (lo, a) };
        a = if lo.is_nan() || hi.is_nan() { end } else { 0.5 * (lo + hi) };
    }
}

/// What rain brings a ground node over a step (`rain`), as its surface balance reads it (`Surface`): heat, W/m²; its
/// heat capacity per second and area, W/(m² K), and temperature, K; and all the heat it brings, J.
struct Fall {
    gain: f64,
    g_rain: f64,
    t_rain: f64,
    rain_in: f64,
}

/// Rain on ground node `i` under air at `tl` at the ground, `over` the share of the air it sees over it: where that air
/// (colder on higher ground) holds more of a gas than it can, it rains out in `rain_time`, with its heat at the
/// temperature of the air it comes from (`shares`) and its fusion enthalpy, of which the node keeps what stays molten.
/// Where the air at the ground is below the gas's melting point, it falls as a solid and the air keeps the fusion
/// enthalpy its freezing frees.
/// Until the balance is solved, each gas's first number in `row` holds its vapour's partial pressure near the ground, Pa.
fn rain(cx: &Ctx, i: usize, tl: f64, over: f64, row: &mut [f64]) -> Fall {
    let (w, s, it, dt) = (cx.w, cx.s, cx.it, cx.dt);
    let (na, area) = (w.mesh.level(AIR).n(), w.mesh.cells().area()[i]);
    let (adt, fixed) = (area * dt, &cx.fixed[i * (FIXED_CELL + cx.act.len())..]);
    let (mut gain, mut g_rain, mut rain_heat, mut rain_in) = (0.0, 0.0, 0.0, 0.0);
    for (x, ac) in cx.act.iter().enumerate() {
        let (o, sp) = (FIXED + PER_GAS * x, s.sp[ac.k]);
        let (col, hk) = (lerp(s, &it.x[ac.gi * na..(ac.gi + 1) * na], i) * over, fixed[FIXED_CELL + x]);
        row[o..o + PER_GAS].copy_from_slice(&[col * R_GAS * tl / (area * hk), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let full = s.sat[ac.k].at(tl) * area * hk / (R_GAS * tl);
        if ac.cond && col > full {
            let f = dt / w.cfg.rain_time;
            let r = f * (col - full);
            row[o + 3..o + 6].copy_from_slice(&[r, f, -f * full * (sp.lv / (R_GAS * tl * tl) - 1.0 / tl)]);
            row[o + 7] = if tl >= sp.melt { sp.lf } else { 0.0 };
            let t_from: f64 = shares(s, &it.x[ac.gi * na..(ac.gi + 1) * na], i).iter().map(|&(q, share)| share * it.t[q]).sum();
            (g_rain, rain_heat, rain_in) = (g_rain + r * sp.cp / adt, rain_heat + r * sp.cp / adt * t_from, rain_in + r * (sp.cp * t_from + row[o + 7]));
            gain += (r * row[o + 7] - s.fusion(ac.k, i, r)) / adt;
        }
    }
    Fall { gain, g_rain, t_rain: if g_rain > 0.0 { rain_heat / g_rain } else { 0.0 }, rain_in }
}

/// The surface balance of ground node `i` under air at `tl` at the ground and sunlight `sw` absorbed, W/m², with the
/// rain `fall`, what evaporates and the heat its bodies draw (`survey::Draws`), linearised about the round before: its
/// row but the bodies' light; returns the reflectance of its surface at its end state (`end_reflectance`).
fn exchange(cx: &Ctx, i: usize, (tl, sw): (f64, f64), fall: &Fall, row: &mut [f64]) -> f64 {
    let (w, s, dt) = (cx.w, cx.s, cx.dt);
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[i]);
    let (adt, fixed) = (area * dt, &cx.fixed[i * (FIXED_CELL + cx.act.len())..]);
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
    // The balance with what evaporates and what the bodies draw linearised about `t0`; and whether either moves with it.
    let draws = cx.draws.any(i);
    let balance = |t0: f64, row: &[f64]| {
        let (draw, d_draw) = cx.draws.ground(i, t0);
        let (mut gain, mut slope) = (fall.gain - draw / area, -d_draw / area);
        for x in (0..cx.act.len()).filter(|&x| cx.act[x].up) {
            let (m, dm_dt, _) = evap(t0, x, row);
            gain -= lat(x, m) / adt;
            slope -= lat(x, dm_dt) / adt;
        }
        (Surface { sw, tl, t0, gain, slope: slope.min(0.0), t_rain: fall.t_rain, g_rain: fall.g_rain }.balance(s, (i, area, dt), w.cfg.sensible, fusion), draws || slope != 0.0)
    };
    // Newton's steps, kept between the temperatures the end has been shown to lie between: the net heat a node gets
    // falls as its end temperature rises, so a step down from `from` says the end lies below it.
    let ((mut bal, moves), mut from, (mut lo, mut hi)) = (balance(cx.ts[i], row), cx.ts[i], (f64::MIN, f64::MAX));
    for _ in 1..SURFACE_ROUNDS {
        if !moves || (bal.ts - from).abs() < SURFACE_STILL_K {
            break;
        }
        (lo, hi) = if bal.ts < from { (lo, from) } else { (from, hi) };
        from = if bal.ts > lo && bal.ts < hi { bal.ts } else { 0.5 * (lo + hi) };
        bal = balance(from, row).0;
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
    // The bodies draw at the end temperature, no more than the node holds above `T_FLOOR`.
    let given = (cx.draws.ground(i, bal.ts).0 * dt).min(cx.room[i]);
    row[..5].copy_from_slice(&[bal.to_air, row_slope, bal.to_space, adt * sw, bal.ts]);
    row[7] = given;
    row[FIXED - 1] = end_reflectance(cx, i, row, fall.rain_in);
    row[FIXED - 1]
}

/// The reflectance of ground node `i`'s surface at the end of the step its `row` books: its amounts with what rains
/// (bringing `rain_in` J) and evaporates, its energy with the heat the store books for it (`book`), and its phases there.
fn end_reflectance(cx: &Ctx, i: usize, row: &[f64], rain_in: f64) -> f64 {
    let (w, s) = (cx.w, cx.s);
    let n = w.mesh.cells().n();
    let a0 = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
    let mut gained = [0.0; MAX_SUBSTANCES];
    let (mut c, mut e) = (s.cap[i], w.store.heat(Class::Ground)[i] + row[3] - row[0] - row[2] - row[7] + rain_in);
    for (x, ac) in cx.act.iter().enumerate() {
        let (o, sp) = (FIXED + PER_GAS * x, s.sp[ac.k]);
        let (r, m) = (row[o + 3], row[o].min(a0(ac.k) + row[o + 3]));
        gained[ac.k] += r - m;
        c += (r - m) * sp.cp;
        e -= m * (sp.cp * s.state.tg[i] + sp.lf + sp.lv);
    }
    let amt = |k: usize| a0(k) + gained[k];
    let th = thaw_at(w, s, c.max(1.0), e, amt);
    reflectance(w, s, i, &layers(w, s, i, th.t, amt, |k| th.molten(k)).0)
}

/// What evaporates of active gas `x` from ground node `i` over the step, ending at `t` under a vapour pressure `pa` with
/// `rain` mol falling on it, mol, and its slopes with `t` and `pa`: the saturation deficit through the air's film
/// (`evap`), at the open rate while wet (`exchange::bucket_slope`); over the boiling point, all of it that the node's
/// heat over that point boils (`Scratch::boil_off`).
fn evaporate(cx: &Ctx, i: usize, x: usize, t: f64, pa: f64, rain: f64) -> (f64, f64, f64) {
    let (w, s, ac) = (cx.w, cx.s, cx.act[x]);
    let (n, sp, area) = (w.mesh.cells().n(), s.sp[ac.k], w.mesh.cells().area()[i]);
    let a = w.store.amounts(Class::Ground)[ac.k * n + i];
    if a + rain <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    if s.state.tg[i] >= sp.boil {
        return ((a + rain).min(s.boil_off(ac.k, i, s.state.tg[i])), 0.0, 0.0);
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
