//! The cells and their air (laws.md 1.4, 1.5): each cell's exchange over a step with the air it sees, solved once in its
//! own implicit surface balance at the air of the step's start and taken as linear in that air, its slopes from that
//! balance on the branch it ended on (`exchange`). The air level's days sum that one law over each node's cells
//! (`Response::ground`), and the book takes each cell's at the step's mean air (`Response::at`), so the air and its
//! cells agree.

use super::super::land::{layers, melted, melts, reflectance, thaw_at, NONE};
use super::super::Scratch;
use super::daily::Ground;
use super::rain::Wet;
use super::exchange::bucket;
use super::radiation::{glow, glow_slope, insolation, Air, Surface};
use super::step::Active;
use super::{at_height, lerp, over_cell};
use crate::{math::one_minus_exp, store::Class, units::{MAX_SUBSTANCES, R_GAS}, World, AIR};

/// A ground node's surface balance is solved again about the temperature it gives, up to this many times, until that
/// moves by less than `SURFACE_STILL_K`, K.
const SURFACE_ROUNDS: usize = 8;
const SURFACE_STILL_K: f64 = 1e-3;
/// The reflectance a ground node's balance takes is guessed again up to this many times, until it moves by less than
/// `REFLECT_STILL` from the one its end state gives.
const REFLECT_ROUNDS: usize = 8;
const REFLECT_STILL: f64 = 1e-3;
/// The air a cell sees moves in a step by up to this, K, as its slopes are held to what it holds (`hold`).
const AIR_SWING_K: f64 = 60.0;

/// The outputs of a cell's exchange that change with its air: what its air node gains over the step, J (sensible heat,
/// the heat radiation the air absorbs less what it sends down, and the heat of the gas that goes up less its lift); the
/// heat over its link to the air as the store books it, J (with the gas's latent heat); the heat it sends to space, J;
/// the lift its air pays for its gas, J; then per active gas what evaporates, mol.
pub(super) const GAINS: usize = 0;
pub(super) const LINK: usize = 1;
pub(super) const SPACE: usize = 2;
pub(super) const LIFT: usize = 3;
pub(super) const UP: usize = 4;
/// The outputs that do not: the sunlight it absorbs, J; the light per unit of its cover's capture area, J/m²; the light
/// set aside for its cover, J.
pub(super) const SUN: usize = 0;
pub(super) const LIGHT: usize = 1;
pub(super) const KEPT: usize = 2;
const FIXED: usize = 3;
/// The parts the changing outputs are made of (`put`), each laid out as an output is: the heat the cell gives its air
/// node and the heat it sends to space, J (`Balance`), then what evaporates of each active gas, mol.
const TO_AIR: usize = 0;
const TO_SPACE: usize = 1;
const GAS: usize = 2;

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
        let fixed = exchange(cx, i, *q, &seen[i * (1 + nx)..(i + 1) * (1 + nx)], &mut row[FIXED..]);
        row[..FIXED].copy_from_slice(&fixed);
    });
    Response { nx, rows, seen }
}

/// The mean sunlight at the top of the air at latitude `lat`, degrees, over the days of the step, their sun's
/// declinations `decl`, W/m².
fn sunlight(w: &World, decl: &[f64], lat: f64) -> f64 {
    decl.iter().map(|&d| insolation(w.cfg.sun, lat.to_radians(), d)).sum::<f64>() / decl.len() as f64
}

/// Holds the slopes of cell `i`'s parts (`p`, `exchange`) so that it gives no more than it holds however the air it
/// sees moves in the step: its temperature by up to `AIR_SWING_K`, each gas between none and twice what it held at the
/// start (`seen`). What evaporates of each gas stays within what the cell holds of it; then the heat over its link and
/// to space, with what each mol carries (`carry`), within its heat and the sunlight it absorbs, `sun`, J.
fn hold(cx: &Ctx, i: usize, (seen, sun): (&[f64], f64), carry: &[[f64; 3]], p: &mut [f64]) {
    let (w, nx) = (cx.w, cx.act.len());
    let (n, stride) = (w.mesh.cells().n(), 2 + nx);
    // The share of a row's slopes that keeps its value within `most` however far the air moves.
    let keep = |r: &[f64], most: f64| {
        let far = r[1].abs() * AIR_SWING_K + (0..nx).map(|v| (r[2 + v] * seen[1 + v]).abs()).sum::<f64>();
        if r[0] + far > most && far > 0.0 { ((most - r[0]) / far).clamp(0.0, 1.0) } else { 1.0 }
    };
    for (y, ac) in cx.act.iter().enumerate() {
        let r = &mut p[(GAS + y) * stride..(GAS + y + 1) * stride];
        let k = keep(r, w.store.amounts(Class::Ground)[ac.k * n + i]);
        r[1..].iter_mut().for_each(|d| *d *= k);
    }
    let loss: Vec<f64> = (0..stride).map(|v| p[TO_AIR * stride + v] + p[TO_SPACE * stride + v] + carry.iter().enumerate().map(|(y, c)| p[(GAS + y) * stride + v] * c[0]).sum::<f64>()).collect();
    let k = keep(&loss, w.store.heat(Class::Ground)[i] + sun);
    if k < 1.0 {
        p.chunks_mut(stride).for_each(|r| r[1..].iter_mut().for_each(|d| *d *= k));
    }
}

/// What a mol of active gas `k` that evaporates from cell `i` carries, J: the heat over its link as the store books it
/// (its heat at the cell's temperature and its latent heat), what its air node gains (that heat less its latent heat and
/// its lift), and the lift its air pays.
fn carried(cx: &Ctx, i: usize, k: usize) -> [f64; 3] {
    let (s, sp, j) = (cx.s, cx.s.sp[k], cx.w.mesh.cells().parent()[i] as usize);
    let (heat, lift) = (sp.cp * s.state.tg[i], sp.mm * s.view().lift(i, j, cx.w.cfg.gravity));
    [heat + sp.lf + sp.lv, heat - lift, lift]
}

/// A cell's changing outputs (`out`) from its parts (`p`), each a row of its value and its slopes, with what each mol of
/// each gas carries (`carry`).
fn put(p: &[f64], carry: &[[f64; 3]], out: &mut [f64]) {
    let stride = 2 + carry.len();
    for v in 0..stride {
        let part = |r: usize| p[r * stride + v];
        let gas = |c: usize| carry.iter().enumerate().map(|(y, h)| part(GAS + y) * h[c]).sum::<f64>();
        out[GAINS * stride + v] = part(TO_AIR) + gas(1);
        out[LINK * stride + v] = part(TO_AIR) + gas(0);
        out[SPACE * stride + v] = part(TO_SPACE);
        out[LIFT * stride + v] = gas(2);
        (0..carry.len()).for_each(|y| out[(UP + y) * stride + v] = part(GAS + y));
    }
}

/// An active gas of a ground node, as its evaporation reads it (`Vapour::evaporate`): its partial pressure near the
/// ground, Pa, and that per mol of the gas in the air, Pa/mol; and what does not move with the node's end temperature.
struct Vapour {
    k: usize,
    /// It may evaporate, and the node holds some.
    up: bool,
    pa: f64,
    per: f64,
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
    /// Ground node `i`'s active gas `ac` with `x` mol of it in the air, at `per` Pa per mol.
    fn of(cx: &Ctx, i: usize, ac: Active, x: f64, per: f64) -> Vapour {
        let (w, s) = (cx.w, cx.s);
        let (n, sp, area) = (w.mesh.cells().n(), s.sp[ac.k], w.mesh.cells().area()[i]);
        let held = w.store.amounts(Class::Ground)[ac.k * n + i];
        let up = ac.up && held > 0.0;
        let boil = (up && s.state.tg[i] >= sp.boil).then(|| held.min(s.boil_off(ac.k, i, s.state.tg[i])));
        let (film, wet) = (w.cfg.evap / R_GAS * area, w.cfg.phys.field * s.pd[i] * area / sp.v[melted(s, ac.k, i) as usize]);
        Vapour { k: ac.k, up, pa: x * per, per, held, wet, film, boil, lv: sp.lv, lf_lv: sp.lf + sp.lv, fusion: s.fusion(ac.k, i, 1.0) }
    }
    /// The heat `m` mol take from the node as they leave it, J: their fusion and vaporisation enthalpies, less the fusion
    /// enthalpy the node holds for them.
    fn lat(&self, m: f64) -> f64 {
        m * self.lf_lv - m * self.fusion
    }
    /// What evaporates over the step from the node, ending at `t`, mol, and its slopes with `t` and with the partial
    /// pressure: the saturation deficit through the air's film (`evap`), at the open rate while wet
    /// (`exchange::bucket`); over the boiling point, all of it that the node's heat over that point boils.
    fn evaporate(&self, s: &Scratch, t: f64, dt: f64) -> [f64; 3] {
        if !self.up {
            return [0.0; 3];
        }
        if let Some(m) = self.boil {
            return [m, 0.0, 0.0];
        }
        let ps = s.sat[self.k].at(t);
        let e = self.film * (ps - self.pa) / t;
        if e <= 0.0 {
            return [0.0; 3];
        }
        let (m, dm_de) = bucket(self.held, 0.0, e, self.wet, dt);
        // The open rate's slopes with `t` (Clausius-Clapeyron) and with the partial pressure.
        [m, dm_de * self.film * (ps * self.lv / (R_GAS * t * t) - (ps - self.pa) / t) / t, -dm_de * self.film / t]
    }
}

/// The exchange of cell `i` over the step under the mean sunlight `q` of its latitude, W/m², and the air it sees, `air`
/// (its temperature, K, then each active gas, mol): its changing outputs with their slopes into `out` (`Response`), and
/// its fixed ones returned. Sunlight past the clouds, what the cover intercepts less the share its light reaction
/// stores (set aside for it, laws.md 1.4); the surface balance with what evaporates, by Newton's steps about its end
/// temperature, under the reflectance its surface has at the step's start; where a soft cover lies, under the
/// reflectance it ends the step with, as backward Euler takes it (`end_reflectance`): from the start's, each guess
/// followed to the end it gives, and halved between two once a guess and its end cross, until they agree, so a cover
/// that melts away darkens the ground in its step. The slopes come from the last balance on the branch it ended on
/// (`Air::moves`), held to what the cell holds (`hold`).
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
    let (f, lay) = (s.absorbs[j], &s.lay[j][0]);
    let rad = Air { tl, f, down: glow(f, tl, lay), slope: glow_slope(f, tl, lay) };
    let gas: Vec<Vapour> = act.iter().enumerate().map(|(y, &ac)| Vapour::of(cx, i, ac, air[1 + y], over_cell(w, s, i) * R_GAS * tl / (area * s.hk[ac.gi * na + j]))).collect();
    let pk = s.state.pk[i];
    let fusion = if pk != NONE { w.store.amounts(Class::Ground)[pk * n + i] * s.sp[pk].lf } else { 0.0 };
    // The balance with what evaporates linearised about `t0`.
    let balance = |t0: f64, sw: f64| {
        let (mut gain, mut slope) = (0.0, 0.0);
        for g in gas.iter().filter(|g| g.up) {
            let [m, dm_dt, _] = g.evaporate(s, t0, dt);
            (gain, slope) = (gain - g.lat(m) / adt, slope - g.lat(dm_dt) / adt);
        }
        let surface = Surface { sw, air: &rad, t0, gain, slope: slope.min(0.0) };
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
    let (nx, stride) = (act.len(), 2 + act.len());
    // The parts (`TO_AIR`) under the reflectance `alb`, what evaporates at the end temperature, with their slopes: the
    // air's temperature moves the air touching the cell and each gas's partial pressure, a gas's amount its own.
    let parts = |alb: f64| {
        let bal = solve(sw(alb));
        let ev: Vec<[f64; 3]> = gas.iter().map(|g| g.evaporate(s, bal.ts, dt)).collect();
        let mut p = vec![0.0; (GAS + nx) * stride];
        (p[TO_AIR * stride], p[TO_SPACE * stride]) = (bal.to_air, bal.to_space);
        ev.iter().enumerate().for_each(|(y, e)| p[(GAS + y) * stride] = e[0]);
        for v in 0..1 + nx {
            // How the air's change `v` moves each gas's partial pressure, Pa per K or per mol.
            let dpa = |y: usize| if v == 0 { gas[y].pa / tl } else if v == 1 + y { gas[y].per } else { 0.0 };
            let dgain = -gas.iter().zip(&ev).enumerate().map(|(y, (g, e))| g.lat(e[2] * dpa(y))).sum::<f64>() / adt;
            let [dts, d_air, d_space] = rad.moves(s, (i, area, dt), w.cfg.sensible, &bal, (if v == 0 { 1.0 } else { 0.0 }, dgain));
            (p[TO_AIR * stride + 1 + v], p[TO_SPACE * stride + 1 + v]) = (d_air, d_space);
            ev.iter().enumerate().for_each(|(y, e)| p[(GAS + y) * stride + 1 + v] = e[1] * dts + e[2] * dpa(y));
        }
        p
    };
    let carry: Vec<[f64; 3]> = act.iter().map(|ac| carried(cx, i, ac.k)).collect();
    let (mut alb, mut lo, mut hi) = (s.alb[i], f64::NAN, f64::NAN);
    let mut p = parts(alb);
    put(&p, &carry, out);
    for _ in 1..if s.cover[i][2] > 0.0 { REFLECT_ROUNDS } else { 1 } {
        let end = end_reflectance(cx, i, (adt * sw(alb), out));
        if (end - alb).abs() < REFLECT_STILL {
            break;
        }
        (lo, hi) = if end > alb { (alb, hi) } else { (lo, alb) };
        alb = if lo.is_nan() || hi.is_nan() { end } else { 0.5 * (lo + hi) };
        p = parts(alb);
        put(&p, &carry, out);
    }
    hold(cx, i, (air, adt * sw(alb)), &carry, &mut p);
    put(&p, &carry, out);
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
