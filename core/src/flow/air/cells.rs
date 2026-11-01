//! The cells and their air (laws.md 1.4, 1.5): what the ground under each air node gives the air over a step, from its
//! cells at the step's start (`forcing`), and each cell's exchange over the step with the mean air of its node
//! (`pass`): sunlight, heat radiation, contact heat, what evaporates and rains, and the light and heat of its bodies.

use super::super::land::{layers, melted, melts, reflectance, thaw_at, NONE};
use super::super::Scratch;
use super::daily::{Forcing, Mean, Wet};
use super::exchange::bucket;
use super::radiation::{glow, insolation, Air, Surface};
use super::step::Active;
use super::{lerp, over_cell, touching};
use crate::{math::one_minus_exp, store::Class, survey::Draws, units::{MAX_SUBSTANCES, R_GAS, SIGMA}, World, AIR};

/// A ground node's surface balance is solved again about the temperature it gives, up to this many times, until that
/// moves by less than `SURFACE_STILL_K`, K.
const SURFACE_ROUNDS: usize = 8;
const SURFACE_STILL_K: f64 = 1e-3;
/// The reflectance a ground node's balance takes is guessed again up to this many times, until it moves by less than
/// `REFLECT_STILL` from the one its end state gives.
const REFLECT_ROUNDS: usize = 8;
const REFLECT_STILL: f64 = 1e-3;

/// What each cell's ground gives its air node over a step of `dt` seconds, summed by node (`Forcing`), from the cells
/// as they are at its start: the heat radiation of each the air absorbs, its sensible heat with the air touching it,
/// what evaporates by the saturation deficit through the air's film (`evap`), as wet as its liquid over field capacity
/// allows (the bucket of Manabe 1969) and no more than it holds over the step, all it can boil over a boiling point;
/// and where each cell rains (`Wet`).
pub(super) fn forcing(w: &World, s: &Scratch, act: &[Active], dt: f64) -> Forcing {
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    let (n, na, nx, g, sensible) = (cells.n(), air.n(), act.len(), w.cfg.gravity, w.cfg.sensible);
    let at = std::iter::once(0).chain((0..na).scan(0, |sum, j| Some(*sum + air.children(j).len()).inspect(|&x| *sum = x))).collect();
    let wet = Wet { ratio: vec![0.0; nx * n], at, steps: vec![[0.0; 3]; nx * n] };
    let mut f = Forcing { up: vec![0.0; na], warm: vec![0.0; na], touch: vec![0.0; na], open: vec![0.0; nx * na], close: vec![0.0; nx * na], brings: vec![0.0; nx * na], wet };
    for i in 0..n {
        let (j, area, t0) = (cells.parent()[i] as usize, cells.area()[i], s.state.tg[i]);
        // How much colder the air touching the cell is than that at its node's mean ground.
        let (rise, tn) = (s.gam[j] * (s.head[i] - s.za[j]), (s.ta[j] - s.gam[j] * (s.za[j] - s.zair[j])).max(1.0));
        f.up[j] += s.absorbs[j] * SIGMA * t0.powi(4) * area;
        (f.warm[j], f.touch[j]) = (f.warm[j] + sensible * area * (t0 + rise), f.touch[j] + sensible * area);
        for (x, ac) in act.iter().enumerate() {
            let (y, sp, held) = (x * na + j, s.sp[ac.k], w.store.amounts(Class::Ground)[ac.k * n + i]);
            if ac.cond {
                let tl = (tn - rise).max(1.0);
                f.wet.ratio[x * n + i] = s.sat[ac.k].at(tl) / tl / (s.sat[ac.k].at(tn) / tn).max(f64::MIN_POSITIVE);
            }
            if !ac.up || held <= 0.0 {
                continue;
            }
            let lift = sp.mm * g * (s.zair[j] - s.head[i]);
            if t0 >= sp.boil {
                let m = held.min(s.boil_off(ac.k, i, t0));
                (f.open[y], f.brings[y]) = (f.open[y] + m / dt, f.brings[y] + m / dt * (sp.cp * t0 - lift));
                continue;
            }
            let (film, ps) = (w.cfg.evap / R_GAS * area / t0, s.sat[ac.k].at(t0));
            let wet = w.cfg.phys.field * s.pd[i] * area / sp.v[melted(s, ac.k, i) as usize];
            let open = film * ps * if wet > 0.0 { (held / wet).min(1.0) } else { 1.0 };
            let k = open.min(held / dt) / ps;
            (f.open[y], f.close[y], f.brings[y]) = (f.open[y] + k * ps, f.close[y] + k * R_GAS / (air.area()[j] * s.hk[ac.gi * na + j]), f.brings[y] + k * ps * (sp.cp * t0 - lift));
        }
    }
    for (b, &o) in f.brings.iter_mut().zip(&f.open) {
        *b = if o > 0.0 { *b / o } else { 0.0 };
    }
    for (x, _) in act.iter().enumerate().filter(|a| a.1.cond) {
        for j in 0..na {
            let (from, kids) = (x * n + f.wet.at[j], air.children(j));
            let mut by: Vec<(f64, f64)> = kids.iter().map(|&i| (f.wet.ratio[x * n + i as usize], cells.area()[i as usize] / air.area()[j])).collect();
            by.sort_by(|p, q| p.0.total_cmp(&q.0));
            let mut sum = [0.0; 2];
            for (c, (r, share)) in by.into_iter().enumerate() {
                sum = [sum[0] + share, sum[1] + share * r];
                f.wet.steps[from + c] = [r, sum[0], sum[1]];
            }
        }
    }
    f
}

/// What rains on each cell over the step, mol (`[x * n + i]`): what rained from its node, shared among the node's
/// cells by what each would hold over its own saturation at the ratio the node rained at (`Wet`).
pub(super) fn landed(w: &World, act: &[Active], f: &Forcing, mean: &Mean) -> Vec<f64> {
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    let (n, na) = (cells.n(), air.n());
    let mut out = vec![0.0; act.len() * n];
    for (x, j) in (0..act.len()).flat_map(|x| (0..na).map(move |j| (x, j))).filter(|&(x, j)| mean.rain[x * na + j] > 0.0) {
        let over = |i: u32| cells.area()[i as usize] * (mean.ratio[x * na + j] - f.wet.ratio[x * n + i as usize]).max(0.0);
        let all: f64 = air.children(j).iter().map(|&i| over(i)).sum();
        air.children(j).iter().for_each(|&i| out[x * n + i as usize] = mean.rain[x * na + j] * over(i) / all);
    }
    out
}

/// The numbers of a cell's row (`pass`): its surface's end temperature, K; the heat it gives its air node over its link,
/// J (sensible, the heat radiation the air absorbs less what it sends down, and the heat of the gas that crosses); the
/// heat it sends to space, J; the sunlight it absorbs, J; the light per unit of its bodies' capture area, J/m²; the light
/// set aside for them, J; the heat they draw from it, J; what rains on it and what evaporates, mol; the lift its air
/// pays for the gas that goes up, J; then per active gas what goes up from it (less what rains), mol.
pub(super) const ROW: usize = 10;
pub(super) const TS: usize = 0;
pub(super) const LINK: usize = 1;
pub(super) const SPACE: usize = 2;
pub(super) const SUN: usize = 3;
pub(super) const LIGHT: usize = 4;
pub(super) const KEPT: usize = 5;
pub(super) const GIVEN: usize = 6;
pub(super) const RAINED: usize = 7;
pub(super) const EVAPORATED: usize = 8;
pub(super) const LIFT: usize = 9;

/// What a pass reads: the world, the active gases, the step's mean air and the rain on each cell (`landed`), the
/// bodies' draws, the step's length and the sun's declination on each of its days.
pub(super) struct Ctx<'a> {
    pub(super) w: &'a World,
    pub(super) s: &'a Scratch,
    pub(super) act: &'a [Active],
    pub(super) mean: &'a Mean,
    pub(super) rain: &'a [f64],
    pub(super) draws: &'a Draws,
    pub(super) dt: f64,
    pub(super) decl: &'a [f64],
}

/// Each cell's row (`ROW`), in parallel with `threads`.
pub(super) fn pass(cx: &Ctx) -> Vec<f64> {
    let (n, width) = (cx.w.mesh.cells().n(), ROW + cx.act.len());
    let mut rows = vec![0.0; n * width];
    // Each task keeps the last latitude and its mean sunlight over the step: neighbouring cells often share a latitude.
    crate::par::rows(&mut rows, width, || (f64::NAN, 0.0), |(lat, q), i, row| {
        let at = cx.w.mesh.cells().lat()[i];
        if at != *lat {
            (*lat, *q) = (at, sunlight(cx, at));
        }
        cell(cx, i, *q, row);
    });
    rows
}

/// The rows of `cells` made again (`pass`), one by one.
pub(super) fn again(cx: &Ctx, cells: &[usize], rows: &mut [f64]) {
    let width = ROW + cx.act.len();
    for &i in cells {
        cell(cx, i, sunlight(cx, cx.w.mesh.cells().lat()[i]), &mut rows[i * width..(i + 1) * width]);
    }
}

/// The mean sunlight at the top of the air at latitude `lat`, degrees, over the days of the step, W/m².
fn sunlight(cx: &Ctx, lat: f64) -> f64 {
    cx.decl.iter().map(|&d| insolation(cx.w.cfg.sun, lat.to_radians(), d)).sum::<f64>() / cx.decl.len() as f64
}

/// An active gas of a ground node, as its evaporation reads it (`Vapour::evaporate`): its partial pressure near the
/// ground, Pa; what rains, mol; and what does not move with the node's end temperature.
struct Vapour {
    k: usize,
    /// It may evaporate, and the node holds some or gets some as rain.
    up: bool,
    pa: f64,
    rain: f64,
    /// What the node holds, mol; what it holds wet (`exchange::bucket`), mol; the air's film, mol K/(Pa s).
    held: f64,
    wet: f64,
    film: f64,
    /// Over its boiling point, what the node's heat boils, mol (`Scratch::boil_off`).
    boil: Option<f64>,
    /// Its vaporisation enthalpy, and that plus its fusion enthalpy, J/mol; the fusion enthalpy a mol of it holds in the
    /// node (`Scratch::fusion`), J/mol.
    lv: f64,
    lf_lv: f64,
    fusion: f64,
}

impl Vapour {
    /// Ground node `i`'s active gas `ac` under a partial pressure `pa` with `rain` mol falling.
    fn of(cx: &Ctx, i: usize, ac: Active, (pa, rain): (f64, f64)) -> Vapour {
        let (w, s) = (cx.w, cx.s);
        let (n, sp, area) = (w.mesh.cells().n(), s.sp[ac.k], w.mesh.cells().area()[i]);
        let held = w.store.amounts(Class::Ground)[ac.k * n + i];
        let up = ac.up && held + rain > 0.0;
        let boil = (up && s.state.tg[i] >= sp.boil).then(|| (held + rain).min(s.boil_off(ac.k, i, s.state.tg[i])));
        let (film, wet) = (w.cfg.evap / R_GAS * area, w.cfg.phys.field * s.pd[i] * area / sp.v[melted(s, ac.k, i) as usize]);
        Vapour { k: ac.k, up, pa, rain, held, wet, film, boil, lv: sp.lv, lf_lv: sp.lf + sp.lv, fusion: s.fusion(ac.k, i, 1.0) }
    }
    /// The heat `m` mol take from the node as they leave it, J: their fusion and vaporisation enthalpies, less the fusion
    /// enthalpy the node holds for them.
    fn lat(&self, m: f64) -> f64 {
        m * self.lf_lv - m * self.fusion
    }
    /// What evaporates over the step from the node, ending at `t`, mol, and its slope with `t`: the saturation deficit
    /// through the air's film (`evap`), at the open rate while wet (`exchange::bucket`); over the boiling point, all of
    /// it that the node's heat over that point boils.
    fn evaporate(&self, s: &Scratch, t: f64, dt: f64) -> (f64, f64) {
        if !self.up {
            return (0.0, 0.0);
        }
        if let Some(m) = self.boil {
            return (m, 0.0);
        }
        let ps = s.sat[self.k].at(t);
        let e = self.film * (ps - self.pa) / t;
        if e <= 0.0 {
            return (0.0, 0.0);
        }
        let (m, dm_de) = bucket(self.held, self.rain, e, self.wet, dt);
        // The open rate's slope with `t` (Clausius-Clapeyron).
        (m, dm_de * self.film * (ps * self.lv / (R_GAS * t * t) - (ps - self.pa) / t) / t)
    }
}

/// The row of cell `i` under the mean sunlight `q` of its latitude over the step, W/m², and the mean air of its node:
/// sunlight past the clouds, what bodies intercept less the share their light reactions store (set aside for them,
/// laws.md 1.4); the rain it gets (`landed`), with its heat at the temperature of the air it comes from and its fusion
/// enthalpy (none for snow, below the gas's melting point in the air touching it), of which the node keeps what stays
/// molten; then the surface balance with what evaporates and what its bodies draw (`survey::Draws`), by Newton's steps
/// about its end temperature (`solve`), under the reflectance its surface has at the step's start. Where a soft cover
/// lies or snow falls, under the reflectance it ends the step with, as backward Euler takes it (`end_reflectance`):
/// from the start's, each guess followed to the end it gives, and halved between two once a guess and its end cross,
/// until they agree; so new snow reflects at once, and a cover that melts away darkens the ground in its step.
fn cell(cx: &Ctx, i: usize, q: f64, row: &mut [f64]) {
    let (w, s, dt, act) = (cx.w, cx.s, cx.dt, cx.act);
    let (n, na, j, area) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.mesh.cells().parent()[i] as usize, w.mesh.cells().area()[i]);
    let (adt, tl, sun) = (area * dt, touching(w, s, &cx.mean.t, i), q * (1.0 - s.state.cloud[j][0]));
    let a = w.canopy[i] as f64;
    let cover = if a > 0.0 { one_minus_exp(a) } else { 0.0 };
    let caught = cover * w.life.absorb.get(i).map_or(0.0, |&x| x as f64);
    // Per unit of capture area the bodies intercept cover / a of the light, all of it where they are few.
    let kept = sun * caught * w.life.keep.get(i).map_or(0.0, |&x| x as f64);
    let sw = |alb: f64| sun * ((1.0 - cover) * (1.0 - alb) + caught) - kept;
    let air = Air { tl, f: s.absorbs[j], down: glow(s.absorbs[j], tl, &s.lay[j][0]) };
    // The rain: heat, W/m²; its heat capacity per second and area, W/(m² K), at the air's temperature; all it brings, J.
    let (mut gain, mut g_rain, mut rain_in, mut gas, mut snow) = (0.0, 0.0, 0.0, Vec::with_capacity(act.len()), false);
    for (x, &ac) in act.iter().enumerate() {
        let (sp, r) = (s.sp[ac.k], cx.rain[x * n + i]);
        let lf = if tl >= sp.melt { sp.lf } else { 0.0 };
        snow |= r > 0.0 && tl < sp.melt;
        (gain, g_rain, rain_in) = (gain + (r * lf - s.fusion(ac.k, i, r)) / adt, g_rain + r * sp.cp / adt, rain_in + r * (sp.cp * cx.mean.t[j] + lf));
        let col = lerp(s, &cx.mean.x[ac.gi * na..(ac.gi + 1) * na], i) * over_cell(w, s, i);
        gas.push(Vapour::of(cx, i, ac, (col * R_GAS * tl / (area * s.hk[ac.gi * na + j]), r)));
    }
    let pk = s.state.pk[i];
    let fusion = if pk != NONE { w.store.amounts(Class::Ground)[pk * n + i] * s.sp[pk].lf } else { 0.0 };
    let draws = cx.draws.any(i);
    // The balance with what evaporates and what the bodies draw linearised about `t0`.
    let balance = |t0: f64, sw: f64| {
        let (draw, d_draw) = if draws { cx.draws.ground(i, t0) } else { (0.0, 0.0) };
        let (mut gain, mut slope) = (gain - draw / area, -d_draw / area);
        for g in gas.iter().filter(|g| g.up) {
            let (m, dm_dt) = g.evaporate(s, t0, dt);
            (gain, slope) = (gain - g.lat(m) / adt, slope - g.lat(dm_dt) / adt);
        }
        let surface = Surface { sw, air: &air, t0, gain, slope: slope.min(0.0), t_rain: cx.mean.t[j], g_rain };
        surface.balance(s, (i, area, dt), w.cfg.sensible, fusion)
    };
    // Newton's steps, kept between the temperatures the end has been shown to lie between: the net heat a node gets
    // falls as its end temperature rises, so a step down from `from` says the end lies below it. Either may move with
    // the end temperature where the bodies draw or a gas may evaporate (none may at the start and much at the end).
    let moves = draws || gas.iter().any(|g| g.up);
    let solve = |sw: f64| {
        let (mut bal, mut from, (mut lo, mut hi)) = (balance(s.state.tg[i], sw), s.state.tg[i], (f64::MIN, f64::MAX));
        for _ in 1..SURFACE_ROUNDS {
            if !moves || (bal.ts - from).abs() < SURFACE_STILL_K {
                break;
            }
            (lo, hi) = if bal.ts < from { (lo, from) } else { (from, hi) };
            from = if bal.ts > lo && bal.ts < hi { bal.ts } else { 0.5 * (lo + hi) };
            bal = balance(from, sw);
        }
        bal
    };
    // The row of the reflectance `alb`.
    let fill = |alb: f64, row: &mut [f64]| {
        let bal = solve(sw(alb));
        // What evaporates at the end temperature leaves with its heat at the node's temperature and its latent heat.
        let (mut link, mut evaporated, mut lift) = (bal.to_air - rain_in, 0.0, 0.0);
        for (x, (g, ac)) in gas.iter().zip(act).enumerate() {
            let (m, sp) = (g.evaporate(s, bal.ts, dt).0, s.sp[ac.k]);
            (link, evaporated) = (link + m * (sp.cp * s.state.tg[i] + sp.lf + sp.lv), evaporated + m);
            row[ROW + x] = m - g.rain;
            lift += row[ROW + x] * sp.mm * w.cfg.gravity * (s.zair[j] - s.head[i]);
        }
        // The bodies draw at the end temperature, no more than the node holds above `T_FLOOR`.
        let given = if draws { (cx.draws.ground(i, bal.ts).0 * dt).min(crate::ground::heat_above_floor(w.store.heat(Class::Ground)[i], w.heat_cap_ground(i))) } else { 0.0 };
        let per_area = if cover > 0.0 { cover / a } else { 1.0 };
        let rained = cx.rain.iter().skip(i).step_by(n).sum();
        row[..ROW].copy_from_slice(&[bal.ts, link, bal.to_space, adt * sw(alb), sun * per_area * dt, kept * adt, given, rained, evaporated, lift]);
    };
    let (mut alb, mut lo, mut hi) = (s.alb[i], f64::NAN, f64::NAN);
    fill(alb, row);
    let rounds = if snow || s.cover[i][2] > 0.0 { REFLECT_ROUNDS } else { 1 };
    for _ in 1..rounds {
        let end = end_reflectance(cx, i, row);
        if (end - alb).abs() < REFLECT_STILL {
            break;
        }
        (lo, hi) = if end > alb { (alb, hi) } else { (lo, alb) };
        alb = if lo.is_nan() || hi.is_nan() { end } else { 0.5 * (lo + hi) };
        fill(alb, row);
    }
}

/// The reflectance of ground node `i`'s surface at the end of the step its `row` books: its amounts with what rains and
/// evaporates, its energy with the heat the store books for it (`book`), and its phases there.
fn end_reflectance(cx: &Ctx, i: usize, row: &[f64]) -> f64 {
    let (w, s) = (cx.w, cx.s);
    let n = w.mesh.cells().n();
    // The node's amounts, read once: the melting points read them many times.
    let mut amt = [0.0; MAX_SUBSTANCES];
    amt.iter_mut().zip(w.store.amounts(Class::Ground)[i..].iter().step_by(n)).for_each(|(a, &a0)| *a = a0);
    let (mut c, e) = (s.cap[i], w.store.heat(Class::Ground)[i] + row[SUN] - row[LINK] - row[SPACE] - row[GIVEN]);
    for (x, ac) in cx.act.iter().enumerate() {
        (amt[ac.k], c) = (amt[ac.k] - row[ROW + x], c - row[ROW + x] * s.sp[ac.k].cp);
    }
    // The melting points, found once: the plateaus and the cover read them.
    let melt = melts(w, s, |k| amt[k]);
    let th = thaw_at(s, c.max(1.0), e, |k| amt[k], &melt);
    reflectance(w, s, i, &layers(w, s, i, th.t, |k| amt[k], |k| th.molten(k), |k| melt[k]).0)
}
