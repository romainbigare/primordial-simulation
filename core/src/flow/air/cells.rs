//! The cells and their air (laws.md 1.4, 1.5): each cell's exchange over a step with the air it sees, solved once in its
//! own implicit surface balance at the air of the step's start and taken as linear in that air, its slopes found by a
//! small step of the air (`respond`). The air level's days sum that one law over each node's cells (`Response::ground`),
//! and the book takes each cell's at the step's mean air (`Response::at`), so the air and its cells agree. And where a
//! node's rain falls (`wet`, `landed`).

use super::super::land::{layers, melted, melts, reflectance, thaw_at, NONE};
use super::super::Scratch;
use super::daily::{Ground, Mean, Wet};
use super::exchange::bucket;
use super::radiation::{glow, insolation, Air, Surface};
use super::step::{Active, SAT_REACH};
use super::{at_height, lerp, over_cell, touching};
use crate::{math::one_minus_exp, store::Class, units::{MAX_SUBSTANCES, R_GAS}, World, AIR};

/// A ground node's surface balance is solved again about the temperature it gives, up to this many times, until that
/// moves by less than `SURFACE_STILL_K`, K.
const SURFACE_ROUNDS: usize = 8;
const SURFACE_STILL_K: f64 = 1e-3;
/// The reflectance a ground node's balance takes is guessed again up to this many times, until it moves by less than
/// `REFLECT_STILL` from the one its end state gives.
const REFLECT_ROUNDS: usize = 8;
const REFLECT_STILL: f64 = 1e-3;
/// The step of the air's temperature, K, and of a gas's amount, as a share of it, by which a cell's response is found
/// to change with its air.
const STEP_K: f64 = 1.0;
const STEP_SHARE: f64 = 1e-3;

/// The outputs of a cell's exchange that change with its air: its surface's end temperature, K; what its air node gains
/// over the step, J (sensible heat, the heat radiation the air absorbs less what it sends down, and the heat of the gas
/// that goes up less its lift); the heat over its link to the air as the store books it, J (with the gas's latent
/// heat); the heat it sends to space, J; the lift its air pays for its gas, J; then per active gas what evaporates, mol.
pub(super) const TS: usize = 0;
pub(super) const GAINS: usize = 1;
pub(super) const LINK: usize = 2;
pub(super) const SPACE: usize = 3;
pub(super) const LIFT: usize = 4;
pub(super) const UP: usize = 5;
/// The outputs that do not: the sunlight it absorbs, J; the light per unit of its cover's capture area, J/m²; the light
/// set aside for its cover, J.
pub(super) const SUN: usize = 0;
pub(super) const LIGHT: usize = 1;
pub(super) const KEPT: usize = 2;
const FIXED: usize = 3;

/// What a pass reads: the world, the active gases, the air of the step's start (per node its temperature, K; per active
/// gas and node, `[x * na + j]`, mol), the step's length and the sun's declination on each of its days.
pub(super) struct Ctx<'a> {
    pub(super) w: &'a World,
    pub(super) s: &'a Scratch,
    pub(super) act: &'a [Active],
    pub(super) t0: &'a [f64],
    pub(super) x0: &'a [f64],
    pub(super) dt: f64,
    pub(super) decl: &'a [f64],
}

/// Each cell's response to the air it sees: per cell the fixed outputs, then each changing output's value at the air it
/// sees at the step's start, its slope per K of that air, and per mol of each active gas there; and that air.
pub(super) struct Response {
    nx: usize,
    rows: Vec<f64>,
    seen: Vec<f64>,
}

impl Response {
    fn width(nx: usize) -> usize {
        FIXED + (UP + nx) * (2 + nx)
    }
    /// Fixed output `o` of cell `i`.
    pub(super) fn fixed(&self, i: usize, o: usize) -> f64 {
        self.rows[i * Self::width(self.nx) + o]
    }
    /// Output `o` of cell `i` under the air it sees: at temperature `t`, K, with `x(y)` mol of each active gas `y`.
    pub(super) fn at(&self, i: usize, o: usize, t: f64, x: impl Fn(usize) -> f64) -> f64 {
        let (nx, seen) = (self.nx, &self.seen[i * (1 + self.nx)..]);
        let r = &self.rows[i * Self::width(nx) + FIXED + o * (2 + nx)..];
        r[0] + r[1] * (t - seen[0]) + (0..nx).map(|y| r[2 + y] * (x(y) - seen[1 + y])).sum::<f64>()
    }
    /// Each node's ground (`Ground`): what its air gains and what evaporates into it, summed over its cells, with its
    /// slopes with the air of each node they see, by the cells' shares of it (`lerp`).
    pub(super) fn ground(&self, cx: &Ctx, wet: Wet) -> Ground {
        let (w, s, nx) = (cx.w, cx.s, self.nx);
        let (n, na, outs, per) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), 1 + nx, (1 + nx) * (1 + nx));
        let mut g = Ground::new(na, nx, (cx.t0.to_vec(), cx.x0.to_vec()), wet);
        // The nodes the cells of each node see, and their slopes there.
        let mut near: Vec<Vec<(u32, Vec<f64>)>> = vec![Vec::new(); na];
        for i in 0..n {
            let (j, width) = (w.mesh.cells().parent()[i] as usize, Self::width(nx));
            let row = &self.rows[i * width + FIXED..];
            let out = |o: usize| if o == 0 { GAINS } else { UP + o - 1 };
            (0..outs).for_each(|o| g.base[j * outs + o] += row[out(o) * (2 + nx)]);
            for c in 0..4 {
                let (q, wt) = (s.near[i][c], s.wt[i][c]);
                if wt == 0.0 {
                    continue;
                }
                let at = match near[j].iter().position(|e| e.0 == q) {
                    Some(at) => at,
                    None => {
                        near[j].push((q, vec![0.0; per]));
                        near[j].len() - 1
                    }
                };
                for o in 0..outs {
                    (0..outs).for_each(|v| near[j][at].1[o * outs + v] += wt * row[out(o) * (2 + nx) + 1 + v]);
                }
            }
        }
        for list in near {
            for (q, slope) in list {
                g.near.push(q);
                g.slope.extend(slope);
            }
            g.at.push(g.near.len());
        }
        g
    }
}

/// Each cell's response over the step (`Response`), in parallel with `threads`.
pub(super) fn respond(cx: &Ctx) -> Response {
    let (n, nx, na) = (cx.w.mesh.cells().n(), cx.act.len(), cx.w.mesh.level(AIR).n());
    let seen: Vec<f64> = (0..n)
        .flat_map(|i| std::iter::once(lerp(cx.s, cx.t0, i)).chain((0..nx).map(move |y| lerp(cx.s, &cx.x0[y * na..(y + 1) * na], i))))
        .collect();
    let width = Response::width(nx);
    let mut rows = vec![0.0; n * width];
    // Each task keeps the last latitude and its mean sunlight over the step: neighbouring cells often share a latitude.
    crate::par::rows(&mut rows, width, || (f64::NAN, 0.0), |(lat, q), i, row| {
        let at = cx.w.mesh.cells().lat()[i];
        if at != *lat {
            (*lat, *q) = (at, sunlight(cx.w, cx.decl, at));
        }
        respond_cell(cx, i, *q, &seen[i * (1 + nx)..(i + 1) * (1 + nx)], row);
    });
    Response { nx, rows, seen }
}

/// The mean sunlight at the top of the air at latitude `lat`, degrees, over the days of the step, their sun's
/// declinations `decl`, W/m².
fn sunlight(w: &World, decl: &[f64], lat: f64) -> f64 {
    decl.iter().map(|&d| insolation(w.cfg.sun, lat.to_radians(), d)).sum::<f64>() / decl.len() as f64
}

/// The row of cell `i` (`Response`) under the mean sunlight `q` of its latitude, W/m², and the air it sees at the
/// step's start, `seen` (its temperature, K, then each active gas, mol): its exchange there (`exchange`), and again a
/// step of the air's temperature and of each gas away, for the slopes.
fn respond_cell(cx: &Ctx, i: usize, q: f64, seen: &[f64], row: &mut [f64]) {
    let (nx, outs) = (cx.act.len(), UP + cx.act.len());
    let mut air = seen.to_vec();
    let fixed = exchange(cx, i, q, &air, &mut row[FIXED..]);
    row[..FIXED].copy_from_slice(&fixed);
    let base: Vec<f64> = (0..outs).map(|o| row[FIXED + o * (2 + nx)]).collect();
    for v in 0..1 + nx {
        let step = if v == 0 { STEP_K } else { (STEP_SHARE * seen[v]).max(f64::MIN_POSITIVE.sqrt()) };
        air[v] += step;
        let mut moved = vec![0.0; outs * (2 + nx)];
        exchange(cx, i, q, &air, &mut moved);
        (0..outs).for_each(|o| row[FIXED + o * (2 + nx) + 1 + v] = (moved[o * (2 + nx)] - base[o]) / step);
        air[v] = seen[v];
    }
    hold(cx, i, seen, row);
}

/// The slopes of what cell `i` evaporates of each gas, cut so that it takes no more than it holds however the air it
/// sees moves in the step: its temperature by up to `SAT_REACH`, each gas between none and twice what it held at the
/// start (`seen`); the heat of what it no longer takes comes off its link and its air's gain (`exchange`).
fn hold(cx: &Ctx, i: usize, seen: &[f64], row: &mut [f64]) {
    let (w, s, nx) = (cx.w, cx.s, cx.act.len());
    let (n, j, stride) = (w.mesh.cells().n(), w.mesh.cells().parent()[i] as usize, 2 + nx);
    for (y, ac) in cx.act.iter().enumerate() {
        let (m, sp) = (FIXED + (UP + y) * stride, s.sp[ac.k]);
        let held = w.store.amounts(Class::Ground)[ac.k * n + i];
        let most = row[m] + row[m + 1].abs() * SAT_REACH + (0..nx).map(|v| (row[m + 2 + v] * seen[1 + v]).abs()).sum::<f64>();
        if most <= held || most <= row[m] {
            continue;
        }
        let keep = ((held - row[m]) / (most - row[m])).max(0.0);
        let lift = sp.mm * s.view().lift(i, j, w.cfg.gravity);
        for v in 1..stride {
            let cut = row[m + v] * (1.0 - keep);
            row[m + v] -= cut;
            row[FIXED + LINK * stride + v] -= cut * (sp.cp * s.state.tg[i] + sp.lf + sp.lv);
            row[FIXED + GAINS * stride + v] -= cut * (sp.cp * s.state.tg[i] - lift);
            row[FIXED + LIFT * stride + v] -= cut * lift;
        }
    }
}

/// An active gas of a ground node, as its evaporation reads it (`Vapour::evaporate`): its partial pressure near the
/// ground, Pa; and what does not move with the node's end temperature.
struct Vapour {
    k: usize,
    /// It may evaporate, and the node holds some.
    up: bool,
    pa: f64,
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
    /// Ground node `i`'s active gas `ac` under a partial pressure `pa`.
    fn of(cx: &Ctx, i: usize, ac: Active, pa: f64) -> Vapour {
        let (w, s) = (cx.w, cx.s);
        let (n, sp, area) = (w.mesh.cells().n(), s.sp[ac.k], w.mesh.cells().area()[i]);
        let held = w.store.amounts(Class::Ground)[ac.k * n + i];
        let up = ac.up && held > 0.0;
        let boil = (up && s.state.tg[i] >= sp.boil).then(|| held.min(s.boil_off(ac.k, i, s.state.tg[i])));
        let (film, wet) = (w.cfg.evap / R_GAS * area, w.cfg.phys.field * s.pd[i] * area / sp.v[melted(s, ac.k, i) as usize]);
        Vapour { k: ac.k, up, pa, held, wet, film, boil, lv: sp.lv, lf_lv: sp.lf + sp.lv, fusion: s.fusion(ac.k, i, 1.0) }
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
        let (m, dm_de) = bucket(self.held, 0.0, e, self.wet, dt);
        // The open rate's slope with `t` (Clausius-Clapeyron).
        (m, dm_de * self.film * (ps * self.lv / (R_GAS * t * t) - (ps - self.pa) / t) / t)
    }
}

/// The exchange of cell `i` over the step under the mean sunlight `q` of its latitude, W/m², and the air it sees, `air`
/// (its temperature, K, then each active gas, mol): its changing outputs into `out` (`TS` on, each the first of its
/// columns), and its fixed ones returned. Sunlight past the clouds, what the cover intercepts less the share its light
/// reaction stores (set aside for it, laws.md 1.4); the surface balance with what evaporates, by Newton's steps about
/// its end temperature, under the reflectance its surface has at the step's start; where a soft cover lies, under the
/// reflectance it ends the step with, as backward Euler takes it (`end_reflectance`): from the start's, each guess
/// followed to the end it gives, and halved between two once a guess and its end cross, until they agree, so a cover
/// that melts away darkens the ground in its step.
fn exchange(cx: &Ctx, i: usize, q: f64, air: &[f64], out: &mut [f64]) -> [f64; FIXED] {
    let (w, s, dt, act) = (cx.w, cx.s, cx.dt, cx.act);
    let (n, na, j, area) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.mesh.cells().parent()[i] as usize, w.mesh.cells().area()[i]);
    let (adt, tl, sun) = (area * dt, at_height(air[0], lerp(s, &s.gam, i), s.head[i], lerp(s, &s.zair, i)), q * (1.0 - s.state.cloud[j][0]));
    let a = w.canopy[i] as f64;
    let cover = if a > 0.0 { one_minus_exp(a) } else { 0.0 };
    let caught = cover * w.life.absorb.get(i).map_or(0.0, |&x| x as f64);
    // Per unit of capture area the cover intercepts cover / a of the light, all of it where it is thin.
    let kept = sun * caught * w.life.keep.get(i).map_or(0.0, |&x| x as f64);
    let sw = |alb: f64| sun * ((1.0 - cover) * (1.0 - alb) + caught) - kept;
    let rad = Air { tl, f: s.absorbs[j], down: glow(s.absorbs[j], tl, &s.lay[j][0]) };
    let gas: Vec<Vapour> = act.iter().enumerate().map(|(y, &ac)| Vapour::of(cx, i, ac, air[1 + y] * over_cell(w, s, i) * R_GAS * tl / (area * s.hk[ac.gi * na + j]))).collect();
    let pk = s.state.pk[i];
    let fusion = if pk != NONE { w.store.amounts(Class::Ground)[pk * n + i] * s.sp[pk].lf } else { 0.0 };
    // The balance with what evaporates linearised about `t0`.
    let balance = |t0: f64, sw: f64| {
        let (mut gain, mut slope) = (0.0, 0.0);
        for g in gas.iter().filter(|g| g.up) {
            let (m, dm_dt) = g.evaporate(s, t0, dt);
            (gain, slope) = (gain - g.lat(m) / adt, slope - g.lat(dm_dt) / adt);
        }
        let surface = Surface { sw, air: &rad, t0, gain, slope: slope.min(0.0), t_rain: 0.0, g_rain: 0.0 };
        surface.balance(s, (i, area, dt), w.cfg.sensible, fusion)
    };
    // Newton's steps, kept between the temperatures the end has been shown to lie between: the net heat a node gets
    // falls as its end temperature rises, so a step down from `from` says the end lies below it. It may move with the
    // end temperature where a gas may evaporate (none may at the start and much at the end).
    let moves = gas.iter().any(|g| g.up);
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
    let stride = 2 + act.len();
    // The outputs under the reflectance `alb`; what evaporates at the end temperature leaves with its heat at the
    // node's temperature and its latent heat, and its air pays its lift.
    let fill = |alb: f64, out: &mut [f64]| {
        let bal = solve(sw(alb));
        let (mut link, mut gains, mut lift) = (bal.to_air, bal.to_air, 0.0);
        for (y, (g, ac)) in gas.iter().zip(act).enumerate() {
            let (m, sp) = (g.evaporate(s, bal.ts, dt).0, s.sp[ac.k]);
            (link, gains) = (link + m * (sp.cp * s.state.tg[i] + sp.lf + sp.lv), gains + m * sp.cp * s.state.tg[i]);
            lift += m * sp.mm * s.view().lift(i, j, w.cfg.gravity);
            out[(UP + y) * stride] = m;
        }
        [(TS, bal.ts), (GAINS, gains - lift), (LINK, link), (SPACE, bal.to_space), (LIFT, lift)].into_iter().for_each(|(o, v)| out[o * stride] = v);
    };
    let (mut alb, mut lo, mut hi) = (s.alb[i], f64::NAN, f64::NAN);
    fill(alb, out);
    for _ in 1..if s.cover[i][2] > 0.0 { REFLECT_ROUNDS } else { 1 } {
        let end = end_reflectance(cx, i, (adt * sw(alb), out));
        if (end - alb).abs() < REFLECT_STILL {
            break;
        }
        (lo, hi) = if end > alb { (alb, hi) } else { (lo, alb) };
        alb = if lo.is_nan() || hi.is_nan() { end } else { 0.5 * (lo + hi) };
        fill(alb, out);
    }
    let per_area = if cover > 0.0 { cover / a } else { 1.0 };
    [adt * sw(alb), sun * per_area * dt, kept * adt]
}

/// The reflectance of ground node `i`'s surface at the end of the step whose sunlight and outputs (`exchange`) are
/// `(sun, out)`: its amounts with what evaporates, its energy with the heat the store books for it, and its phases there.
fn end_reflectance(cx: &Ctx, i: usize, (sun, out): (f64, &[f64])) -> f64 {
    let (w, s, stride) = (cx.w, cx.s, 2 + cx.act.len());
    let n = w.mesh.cells().n();
    // The node's amounts, read once: the melting points read them many times.
    let mut amt = [0.0; MAX_SUBSTANCES];
    amt.iter_mut().zip(w.store.amounts(Class::Ground)[i..].iter().step_by(n)).for_each(|(a, &a0)| *a = a0);
    let (mut c, e) = (s.cap[i], w.store.heat(Class::Ground)[i] + sun - out[LINK * stride] - out[SPACE * stride]);
    for (y, ac) in cx.act.iter().enumerate() {
        let m = out[(UP + y) * stride];
        (amt[ac.k], c) = (amt[ac.k] - m, c - m * s.sp[ac.k].cp);
    }
    // The melting points, found once: the plateaus and the cover read them.
    let melt = melts(w, s, |k| amt[k]);
    let th = thaw_at(s, c.max(1.0), e, |k| amt[k], &melt);
    reflectance(w, s, i, &layers(w, s, i, th.t, |k| amt[k], |k| th.molten(k), |k| melt[k]).0)
}

/// Where each node's rain falls, as of the step's start (`Wet`): on each cell that sees the node (`lerp`), by the share
/// `a` of the node's air over it (its weight there times its area over the area it sees), past the node's saturation
/// ratio `e` at which the air touching the cell saturates (colder on higher ground). The node rains what its cells hold
/// over their saturation, summed (laws.md 1.5).
pub(super) fn wet(w: &World, s: &Scratch, act: &[Active], t0: &[f64]) -> Wet {
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    let (n, na) = (cells.n(), air.n());
    let mut by: Vec<Vec<(u32, f64, Vec<f64>)>> = vec![Vec::new(); na];
    for i in 0..n {
        let tl = touching(s, t0, i);
        for c in (0..4).filter(|&c| s.wt[i][c] > 0.0) {
            let q = s.near[i][c] as usize;
            let ts = at_height(t0[q], s.gam[q], s.za[q], s.zair[q]);
            let (share, seen) = (s.wt[i][c] * over_cell(w, s, i), lerp(s, air.area(), i) / air.area()[q]);
            let ratio = act.iter().map(|ac| s.sat[ac.k].at(tl) / tl / (s.sat[ac.k].at(ts) / ts).max(f64::MIN_POSITIVE) * seen).collect();
            by[q].push((i as u32, share, ratio));
        }
    }
    Wet::new(act.len(), by)
}

/// What rains of one gas on each cell from each node it sees over the step: (cell, node, mol), in the order of the nodes.
pub(super) type Landed = Vec<(u32, u32, f64)>;

/// What rains on each cell from each node it sees over the step (`Landed`), per active gas: each node's rain shared
/// among the cells that see it by what each would hold over its own saturation at the ratio the node rained at (`Wet`).
pub(super) fn landed(act: &[Active], wet: &Wet, mean: &Mean) -> Vec<Landed> {
    let na = wet.at.len() - 1;
    (0..act.len())
        .map(|x| {
            let mut out = Vec::new();
            for j in (0..na).filter(|&j| mean.rain[x * na + j] > 0.0) {
                let entries = wet.at[j]..wet.at[j + 1];
                let over = |e: usize| wet.share[e] * (mean.ratio[x * na + j] - wet.ratio[x][e]).max(0.0);
                let all: f64 = entries.clone().map(over).sum();
                out.extend(entries.filter(|&e| over(e) > 0.0).map(|e| (wet.cell[e], j as u32, mean.rain[x * na + j] * over(e) / all)));
            }
            out
        })
        .collect()
}
