//! Flow in and through the air: radiation, contact heat, evaporation and condensation,
//! and gas between air nodes.

use super::{range, settle, wet, Scratch, NONE};
use crate::chem::R_GAS;
use crate::World;
use std::f64::consts::{LN_2, PI, TAU};

/// Stefan-Boltzmann constant, W/(m² K⁴): every surface emits σT⁴ (emissivity 1).
pub(crate) const SIGMA: f64 = 5.670e-8;
/// Share of the pressure difference between two air nodes evened out per call.
const EVEN: f64 = 0.25;
/// Most of a node's amount the overturning may exchange over one edge per call.
const OVER: f64 = 3.0;
/// A gas making up less than this share of the air is a trace: it evaporates and condenses
/// only every `TRACE_TURN`-th call, for all the time since.
const TRACE_AIR: f64 = 1e-7;
const TRACE_TURN: u32 = 4;

/// The air seen by ground cell `i`: a node field interpolated from the four nearest nodes.
#[inline]
pub(super) fn lerp(s: &Scratch, v: &[f64], i: usize) -> f64 {
    let (j, w) = (&s.near[i], &s.wt[i]);
    w[0] * v[j[0] as usize] + w[1] * v[j[1] as usize] + w[2] * v[j[2] as usize] + w[3] * v[j[3] as usize]
}

/// Amounts, heat capacity, mass, heights, volume and optical depth of every air node, the air
/// each ground cell sees, and how high each gas fills its column: a gas that would condense
/// higher up is held near the ground, by how fast its vapour pressure falls as the air cools upwards.
pub(super) fn state(w: &World, s: &mut Scratch, full: bool) {
    let (n, na, g, top) = (w.grid.n(), w.grid.na(), w.cfg.gravity, w.cfg.air_top_m);
    if !full {
        for j in 0..na {
            s.ta[j] = w.e_air[j] / s.capa[j];
        }
        return;
    }
    for j in 0..na {
        (s.za[j], s.nt[j], s.capa[j], s.mass[j], s.fabs[j], s.node_tmp[j]) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    }
    for i in 0..n {
        s.za[s.air_of[i] as usize] += s.head[i];
    }
    for &k in &s.gas {
        let (sp, ir) = (s.sp[k], w.chem.subs[k].ir_abs > 0.0);
        for (j, &a) in w.air[k * na..(k + 1) * na].iter().enumerate() {
            s.nt[j] += a;
            s.capa[j] += a * sp.cp;
            s.mass[j] += a * sp.mm;
            s.fabs[j] += if ir { w.chem.depth(k, a / (s.area * s.cells[j])) } else { 0.0 };
        }
    }
    for j in 0..na {
        s.za[j] /= s.cells[j];
        let (a, l) = (s.area * s.cells[j], (top - s.za[j]).max(100.0));
        s.vol[j] = a * l;
        s.ta[j] = w.e_air[j] / s.capa[j];
        s.gam[j] = g * s.mass[j] / s.capa[j];
        let h = R_GAS * s.ta[j] * s.nt[j] / (g * s.mass[j]);
        s.zair[j] = s.za[j] + h - l / (l / h).exp_m1();
    }
    let ng = s.gas.len();
    (s.cond, s.hk, s.split) = (vec![false; ng], vec![0.0; ng * na], vec![0.0; ng * na]);
    for (gi, &k) in s.gas.iter().enumerate() {
        s.cond[gi] = (0..na).any(|j| {
            let t = (s.ta[j] - s.gam[j] * (top - s.zair[j])).max(1.0);
            w.chem.p_sat(k, t) < w.air[k * na + j] * R_GAS * s.ta[j] / s.vol[j]
        });
    }
    // Rising air in which a gas condenses cools slower: its vaporisation enthalpy is freed as
    // the share it can hold falls with the temperature (Clausius-Clapeyron).
    for (gi, &k) in s.gas.iter().enumerate() {
        if s.cond[gi] {
            for j in 0..na {
                let (t, lv) = (s.ta[j], s.sp[k].lv);
                let x = w.chem.p_sat(k, t) * s.vol[j] / (s.nt[j] * R_GAS * t);
                s.node_tmp[j] += s.nt[j] * lv * lv * x.min(1.0) / (R_GAS * t * t);
            }
        }
    }
    for j in 0..na {
        s.gam[j] = g * s.mass[j] / (s.capa[j] + s.node_tmp[j]);
        s.node_tmp[j] = 0.0;
    }
    // (The vapour pressure falls by a factor exp(lv/R (1/(t-½) - 1/(t+½))) per K.)
    let half = (-LN_2).exp();
    for (gi, &k) in s.gas.iter().enumerate() {
        for j in 0..na {
            let h = (R_GAS * s.ta[j] * s.nt[j] / (g * s.mass[j])).min((top - s.za[j]).max(100.0));
            let (mut hk, mut split) = (h, half);
            if s.cond[gi] {
                let t = s.ta[j] + s.gam[j] * (s.zair[j] - s.za[j]);
                let fall = s.sp[k].lv / R_GAS * (1.0 / (t - 0.5).max(1.0) - 1.0 / (t + 0.5).max(1.0));
                hk = (1.0 / (s.gam[j] * fall)).min(h);
                split = (-(h / hk) * LN_2).exp();
            }
            s.hk[gi * na + j] = hk;
            s.split[gi * na + j] = split;
        }
    }
    let node: Vec<[f64; 4]> = (0..na).map(|j| [s.gam[j], s.zair[j], s.fabs[j], s.ta[j]]).collect();
    s.lay.resize(n, [[0.0; LAYERS]; 2]);
    for i in 0..n {
        let (j, wt, mut v) = (s.near[i], s.wt[i], [0.0; 4]);
        for c in 0..4 {
            let x = node[j[c] as usize];
            for f in 0..4 {
                v[f] += wt[c] * x[f];
            }
        }
        let o = s.air_of[i] as usize;
        s.lay[i] = column(v[2], v[0], (top - s.head[i]).max(100.0), R_GAS * s.ta[o] * s.nt[o] / (g * s.mass[o]));
        s.seen[i] = [v[0], v[1], -(-v[2]).exp_m1(), s.seen[i][3], v[3]];
    }
    s.ta0.copy_from_slice(&s.ta);
}

/// Daily-mean sunlight at the top of the air, W/m².
fn insolation(s0: f64, lat: f64, decl: f64) -> f64 {
    let h0 = (-lat.tan() * decl.tan()).clamp(-1.0, 1.0).acos();
    (s0 / PI * (h0 * lat.sin() * decl.sin() + lat.cos() * decl.cos() * h0.sin())).max(0.0)
}

/// Layers of the air over a cell in `column`.
pub(crate) const LAYERS: usize = 4;

/// The layers of the air over a cell that send heat radiation [down to the ground, up to space],
/// as how much colder than the air touching the ground each is, K: the grey absorber of optical
/// depth `x` lies like the air's mass (scale height `h`, in a column `l` deep), and the air's
/// temperature falls by `gam` per metre. Each layer sends that side the same share of what the
/// column sends it, from its height (Schwarzschild's integral, summed at `LAYERS` points: see
/// `glow`). More gas lifts the layers that reach space into colder air, so less leaves and the
/// ground warms, with no ceiling (one slab at the mean temperature has one: 2^(1/4) times the bare ground's).
pub fn column(x: f64, gam: f64, l: f64, h: f64) -> [[f64; LAYERS]; 2] {
    let (f, e, mut out) = (-(-x).exp_m1(), (-l / h).exp(), [[0.0; LAYERS]; 2]);
    // Drop to the height with a share `above` of the absorber above it.
    let drop = |above: f64| gam * h * (e + (1.0 - e) * above).ln();
    for m in (0..LAYERS).filter(|_| x > 0.0) {
        // Share of the absorber between the layer and the side it sends to.
        let near = -(-f * (m as f64 + 0.5) / LAYERS as f64).ln_1p() / x;
        (out[0][m], out[1][m]) = (drop(1.0 - near), drop(near));
    }
    out
}

/// What layers (`column`) over ground with air at `tl` that absorbs a share `f` send, W/m².
pub fn glow(f: f64, tl: f64, lay: &[f64; LAYERS]) -> f64 {
    lay.iter().map(|&d| (tl + d).max(1.0).powi(4)).sum::<f64>() * SIGMA * f / LAYERS as f64
}

/// Sunlight by latitude and season, reflected by what covers the ground; the ground emits
/// σT⁴, of which the air absorbs its share and emits by its layers (`column`); and
/// contact heat with the air touching the ground (warmer than the air's mean by the drop
/// from the air's mean height). The ground temperature is solved implicitly. All the air
/// absorbs and emits is booked to its node.
pub(super) fn radiation(w: &mut World, s: &mut Scratch, dt: f64) {
    let season = w.season();
    let World { cfg, grid, ground, e_ground, e_air, ledger, canopy, light, light_pool, life, .. } = w;
    light_pool.resize(grid.n(), 0.0);
    let (n, na, adt) = (grid.n(), grid.na(), s.area * dt);
    let decl = -cfg.tilt.to_radians() * (TAU * season).cos();
    let (mut sun, mut out) = (0.0, 0.0);
    s.node_tmp.fill(0.0);
    let suns: Vec<f64> = (0..grid.h).map(|y| insolation(cfg.sun, grid.lat(y, cfg.lat_south, cfg.lat_north).to_radians(), decl)).collect();
    // Each cell's numbers (they depend on no other cell) in a row: [air temperature at the ground,
    // light it gets, light set aside, shortwave in, heat gained, heat given to its air node, heat
    // sent to space, heat it could not pay], applied in the order of the cells.
    let mut rows = std::mem::take(&mut s.rad);
    rows.resize(n * 8, 0.0);
    let (sc, e_ground_now, canopy_now) = (&*s, &*e_ground, &*canopy);
    crate::par::rows(&mut rows, 8, || (), |_, i, row| {
        let q = suns[i / grid.w];
        // The air this cell sees, and its temperature where it touches the ground.
        let ([gam, zair, f, c, ta], t, j) = (sc.seen[i], sc.tg[i], sc.air_of[i] as usize);
        let ta = ta + sc.ta[j] - sc.ta0[j];
        let tl = (ta - gam * (sc.head[i] - zair)).max(1.0);
        let (down, emit) = (glow(f, tl, &sc.lay[i][0]), glow(f, tl, &sc.lay[i][1]));
        // Bodies with light-capture area `a` per ground area intercept a share 1 - exp(-a) of what
        // passes the clouds. What they absorb warms the surface now but for the share their light
        // reactions store (as at their last update, `Life::keep`): that is set aside, and each takes
        // its share when it updates (`body::live`). Taller members shade smaller ones (`body::shade`).
        let (a, mut cover, mut caught, mut kept, mut dl) = (canopy_now[i] as f64, 0.0, 0.0, 0.0, 0.0);
        if a > 0.0 {
            cover = -(-a).exp_m1();
            dl = q * (1.0 - c) * cover * dt / a;
            caught = cover * life.absorb.get(i).map_or(0.0, |&x| x as f64);
            kept = q * (1.0 - c) * caught * life.keep.get(i).map_or(0.0, |&x| x as f64);
        }
        let sw = q * (1.0 - c) * ((1.0 - cover) * (1.0 - sc.alb[i]) + caught) - kept;
        // The surface temperature, implicit: heat reaches the surface from the node through its heat
        // capacity over the step (none on a melting plateau) in series with any soft solid cover.
        let t3 = SIGMA * t * t * t;
        let surface = |r: f64| if r > 0.0 { (t / r + sw + down + 3.0 * t3 * t + cfg.sensible * tl) / (1.0 / r + 4.0 * t3 + cfg.sensible) } else { t };
        let flux = |ts: f64| (t3 * (4.0 * ts - 3.0 * t), cfg.sensible * (ts - tl));
        let mut ts = surface(if sc.pk[i] != NONE { 0.0 } else { adt / sc.cap[i] } + sc.rc[i]);
        // A plateau holds the node only while its fusion enthalpy lasts: a step that would freeze
        // (or melt) more than the node holds takes it off, and its heat capacity holds it back.
        if sc.pk[i] != NONE {
            let ((up, hs), lat) = (flux(ts), ground[sc.pk[i] * n + i] * sc.sp[sc.pk[i]].lf);
            let net = adt * (sw + down - up - hs);
            if net < -sc.phi[i] * lat || net > (1.0 - sc.phi[i]) * lat {
                ts = surface(adt / sc.cap[i] + sc.rc[i]);
            }
        }
        let (mut up, mut hs) = flux(ts);
        // A node pays only what it holds above `T_FLOOR`; past that it emits and conducts less (as `take_heat`).
        let (room, lose, mut clamped) = (crate::heat_above_floor(e_ground_now[i], sc.cap[i]), adt * (up + hs), 0.0);
        if lose > room + adt * (sw + down) {
            let lam = (room + adt * (sw + down)) / lose;
            clamped = lose * (1.0 - lam);
            (up, hs) = (up * lam, hs * lam);
        }
        *<&mut [f64; 8]>::try_from(row).unwrap() = [tl, dl, kept, sw, adt * (sw + down - up - hs), adt * (f * up + hs - down - emit), adt * ((1.0 - f) * up + emit), clamped];
    });
    for (i, row) in rows.chunks_exact(8).enumerate() {
        let &[tl, dl, kept, sw, gain, to_air, to_space, clamped] = <&[f64; 8]>::try_from(row).unwrap();
        s.tl[i] = tl;
        if canopy[i] as f64 > 0.0 {
            light[i] += dl;
            for &b in &life.at[i] {
                life.bodies[b as usize].mark -= (life.shade.get(b as usize).map_or(1.0, |&f| f) - 1.0) * dl;
            }
        }
        light_pool[i] += adt * kept;
        sun += adt * kept;
        if clamped != 0.0 {
            ledger.clamped_heat += clamped;
        }
        e_ground[i] += gain;
        s.node_tmp[s.air_of[i] as usize] += to_air;
        sun += adt * sw;
        out += to_space;
    }
    s.rad = rows;
    for j in 0..na {
        e_air[j] += s.node_tmp[j];
        s.ta[j] = e_air[j] / s.capa[j];
    }
    ledger.sun_in += sun;
    ledger.space_out += out;
}

/// Saturation pressure of one substance, tabulated every 0.05 K over a range of temperatures.
struct Sat(f64, Vec<f64>);

impl Sat {
    /// Exact every 64 entries; between, each from the last by their ratio, exp(b (1/t' - 1/t)),
    /// summed to its fifth term (ln p is linear in 1/t).
    fn new(w: &World, k: usize, lo: f64, hi: f64) -> Self {
        let (t0, b) = (lo.max(2.0) - 1.0, w.chem.subs[k].lv / R_GAS);
        let mut p = 0.0;
        let v = (0..((hi + 1.0 - t0) * 20.0) as usize + 2).map(|i| {
            let t = t0 + i as f64 / 20.0;
            if i % 64 == 0 {
                p = w.chem.p_sat(k, t);
            } else {
                let x = b * 0.05 / (t * (t - 0.05));
                p *= 1.0 + x * (1.0 + x / 2.0 * (1.0 + x / 3.0 * (1.0 + x / 4.0)));
            }
            p
        });
        Sat(t0, v.collect())
    }
    fn at(&self, t: f64) -> f64 {
        let x = ((t - self.0) * 20.0).clamp(0.0, (self.1.len() - 2) as f64);
        let i = x as usize;
        let f = x - i as f64;
        self.1[i] + (self.1[i + 1] - self.1[i]) * f
    }
}

/// Evaporation by vapour pressure (the ground's saturation pressure against the partial pressure
/// near the ground); then condensation where the air a cell sees holds more than it can at its
/// temperature (colder on higher ground): what it cannot hold rains out within the call; then gases
/// that cannot condense dissolve in the ground's liquid (`dissolve`, slow: with the trace gases).
pub(super) fn exchange(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, na, area, g) = (w.grid.n(), w.grid.na(), s.area, w.cfg.gravity);
    let ((tmin, tmax), tl) = (range(&s.tg), range(&s.tl));
    let (lo, hi) = (tmin.min(tl.0), tmax.max(tl.1));
    let henry: Vec<(usize, usize)> = s.gas.iter().copied().enumerate().filter(|&(_, k)| s.sp[k].boil < tmin && w.chem.subs[k].solubility > 0.0).collect();
    // The gases that leave the ground or may condense, their saturation pressures and evaporation
    // per unit of deficit; each pass sees the air as it was before. A trace gas moves only every
    // `TRACE_TURN`-th call, for all the time since.
    let all: f64 = s.nt.iter().sum();
    s.trace = [s.trace[0] + dt, (s.trace[1] + 1.0) % TRACE_TURN as f64];
    let (turn, since) = (s.trace[1] == 0.0, s.trace[0]);
    if turn {
        s.trace[0] = 0.0;
    }
    let act: Vec<(usize, usize, bool, Sat, f64)> = (0..s.gas.len())
        .filter_map(|gi| {
            let k = s.gas[gi];
            let up = s.loose.contains(&k) && w.chem.p_sat(k, tmax) > 1.0 && !henry.iter().any(|h| h.1 == k);
            let trace = w.air[k * na..(k + 1) * na].iter().sum::<f64>() < TRACE_AIR * all;
            ((up || s.cond[gi]) && (turn || !trace)).then(|| (gi, k, up, Sat::new(w, k, lo, hi), w.cfg.evap * area * if trace { since } else { dt } / R_GAS))
        })
        .collect();
    let snap = |w: &World, s: &mut Scratch| {
        s.snap.clear();
        for &(_, k, ..) in &act {
            s.snap.extend_from_slice(&w.air[k * na..(k + 1) * na]);
        }
    };
    let (c_cond, mut lifted) = (area / R_GAS, 0.0);
    snap(w, s);
    for i in 0..n {
        let (t, j, tl) = (s.tg[i], s.air_of[i] as usize, s.tl[i]);
        for (x, &(gi, k, up, ref sat, c_evap)) in act.iter().enumerate() {
            let a = w.ground[k * n + i];
            if !up || a <= 0.0 {
                continue;
            }
            // The column above the cell, and the height it fills near the ground.
            let (share, hk) = (lerp(s, &s.snap[x * na..(x + 1) * na], i) * s.per[i], lerp(s, &s.hk[gi * na..(gi + 1) * na], i));
            let (sp, pa) = (s.sp[k], share * tl / (c_cond * hk));
            let ps = if t >= sp.boil { f64::INFINITY } else { sat.at(t) };
            if ps <= pa {
                continue;
            }
            let m = if t >= sp.boil {
                a
            } else {
                let d = a * sp.v[super::melted(s, k, i) as usize] / area;
                // A wet surface evaporates at the open rate: liquid stands on it, or its loose ground holds its field
                // capacity or more (stage 1); a drier one goes with its liquid over that (stage 2: the bucket of Manabe 1969).
                // No more than fills the column above the cell to saturation (the rest would rain back at once).
                (c_evap * (ps - pa) / t * (d / (w.cfg.phys.field * s.pd[i])).min(1.0)).min(a).min((ps - pa) * c_cond * hk / tl)
            };
            // Evaporating cools the node, no further than where the vapour pressure falls to the
            // partial pressure (the boiling point, when it boils): only heat above that pays.
            let over = t - if t >= sp.boil { sp.boil } else { w.chem.t_sat(k, pa / crate::chem::P_REF).min(t) };
            let m = m.min(s.cap[i].max(1.0) * over / (sp.cp * over + sp.lv));
            // The ground also pays the fusion and vaporisation enthalpy and the gas's heat; if it cannot, less evaporates.
            let want = m * (sp.cp * t + sp.lf + sp.lv);
            let m = if want > 0.0 { m * crate::share(w.take_heat_with(i, want, m * sp.cp), want) } else { 0.0 };
            if m <= 0.0 {
                continue;
            }
            let (heat, lift) = (m * sp.cp * t, m * g * sp.mm * (s.zair[j] - s.head[i]));
            settle(s, k, i, -m);
            w.ground[k * n + i] -= m;
            w.air[k * na + j] += m;
            (s.cap[i], s.capa[j], s.nt[j], s.mass[j]) = (s.cap[i] - m * sp.cp, s.capa[j] + m * sp.cp, s.nt[j] + m, s.mass[j] + m * sp.mm);
            w.e_air[j] += heat - lift;
            lifted += lift;
            if s.z[i] > 0.0 && t < sp.boil {
                w.diag.liquid_land[1] += m * sp.v[1];
            }
        }
    }
    snap(w, s);
    let mut rained = vec![false; act.len()];
    for i in 0..n {
        let (j, tl) = (s.air_of[i] as usize, s.tl[i]);
        for (x, &(gi, k, _, ref sat, _)) in act.iter().enumerate() {
            if !s.cond[gi] {
                continue;
            }
            let (share, hk) = (lerp(s, &s.snap[x * na..(x + 1) * na], i) * s.per[i], lerp(s, &s.hk[gi * na..(gi + 1) * na], i));
            let m = (share - sat.at(tl) * c_cond * hk / tl).min(w.air[k * na + j]);
            if m <= 0.0 {
                continue;
            }
            let sp = s.sp[k];
            let (tair, lift) = (w.e_air[j] / s.capa[j], m * g * sp.mm * (s.head[i] - s.zair[j]));
            settle(s, k, i, m);
            w.ground[k * n + i] += m;
            w.air[k * na + j] -= m;
            (s.cap[i], s.capa[j], s.nt[j], s.mass[j]) = (s.cap[i] + m * sp.cp, s.capa[j] - m * sp.cp, s.nt[j] - m, s.mass[j] - m * sp.mm);
            // Condensing frees the vaporisation enthalpy into the air; the liquid keeps its fusion.
            w.e_ground[i] += m * (sp.cp * tair + sp.lf) - lift;
            w.e_air[j] += m * (sp.lv - sp.cp * tair);
            lifted += lift;
            s.rain[i] += m;
            if s.z[i] > 0.0 {
                w.diag.liquid_land[0] += m * sp.v[1];
            }
            s.cloud[j] = [s.cloud[j][0] + m, s.cloud[j][1] + m * sp.refl[0]];
            rained[x] = true;
        }
    }
    if turn {
        lifted += dissolve(w, s, since, &henry, (tmin, tmax));
    }
    w.ledger.gravity -= lifted;
    for (&(_, k, ..), _) in act.iter().zip(&rained).filter(|(_, &r)| r) {
        if !s.loose.contains(&k) {
            s.loose.push(k);
            s.vary.push(k);
        }
    }
    // Condensate in the air scatters sunlight like the powder of its solid; the faster a
    // node condenses, the more of its sky it covers.
    for j in 0..na {
        let [m, mr] = std::mem::take(&mut s.cloud[j]);
        let rate = m / (area * s.cells[j] * dt);
        s.node_tmp[j] = if m > 0.0 { rate / (rate + w.cfg.cloud) * mr / m } else { 0.0 };
    }
    let ema = (dt / (w.days_per_year() as f64 * 86_400.0)) as f32;
    for i in 0..n {
        s.seen[i][3] = lerp(s, &s.node_tmp, i);
        let r = (s.rain[i] / (area * dt)) as f32;
        w.diag.rain[i] += (r - w.diag.rain[i]) * ema;
        s.rain[i] = 0.0;
    }
}

/// Gases that cannot condense on the ground (`henry`: index in `gas`, substance) dissolve in the
/// liquid standing on each ground node toward the share Henry's law gives (`Chemistry::henry`), and
/// come out as the liquid warms, thins or freezes or the gas thins. They cross the surface through
/// the air's film (`evap`, as evaporation) and the liquid's (`dissolve`, over its whole depth: one
/// mixed layer, a known limit) in series, solved exactly over the step toward the balance of node
/// and column. Dissolving frees the vaporisation enthalpy into the node (the heat of solution) and
/// the gas brings its heat; coming out takes them back. Dissolved gas moves with the liquid
/// (`liquid`), may feed ground reactions (Law 2), and goes back to the air only here.
/// Returns the energy spent lifting gas, J.
fn dissolve(w: &mut World, s: &mut Scratch, dt: f64, henry: &[(usize, usize)], (tmin, tmax): (f64, f64)) -> f64 {
    let (n, na, area, g) = (w.grid.n(), w.grid.na(), s.area, w.cfg.gravity);
    let solv: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt <= tmax && s.sp[k].boil > tmin).collect();
    let mut lifted = 0.0;
    for i in 0..n {
        let (t, j, tl) = (s.tg[i], s.air_of[i] as usize, s.tl[i]);
        let (l, v) = solv.iter().fold((0.0, 0.0), |(l, v), &k| (l + w.ground[k * n + i] * wet(s, k, i), v + w.ground[k * n + i] * wet(s, k, i) * s.sp[k].v[1]));
        for &(gi, k) in henry {
            let (a, sp) = (w.ground[k * n + i], s.sp[k]);
            if l <= 0.0 && a <= 0.0 {
                continue;
            }
            // The column over the cell, the height it fills, mol dissolved per mol at balance, and the
            // time to balance through the two films.
            let (col, hk) = (lerp(s, &w.air[k * na..(k + 1) * na], i) * s.per[i], lerp(s, &s.hk[gi * na..(gi + 1) * na], i));
            let b = l * w.chem.henry(k, t, R_GAS * tl / (area * hk));
            let tau = b * hk / w.cfg.evap + v / area / w.cfg.dissolve;
            let m = (b * (a + col) / (1.0 + b) - a) * -(-dt * (1.0 + b) / tau).exp_m1();
            let m = if m >= 0.0 {
                m.min(w.air[k * na + j])
            } else {
                // Coming out cools the node, no further than the boiling point; it pays what it can.
                let q = (-m).min(s.cap[i].max(1.0) * (t - sp.boil) / (sp.cp * (t - sp.boil) + sp.lv));
                let want = q * (sp.cp * t + sp.lf + sp.lv);
                -q * if want > 0.0 { crate::share(crate::draw(&mut w.e_ground[i], &mut w.ledger, s.cap[i] - q * sp.cp, want), want) } else { 0.0 }
            };
            if m == 0.0 {
                continue;
            }
            let (heat, lift) = (m * sp.cp * if m > 0.0 { w.e_air[j] / s.capa[j] } else { t }, m * g * sp.mm * (s.head[i] - s.zair[j]));
            w.e_air[j] -= heat + if m < 0.0 { lift } else { 0.0 };
            if m > 0.0 {
                w.e_ground[i] += heat + m * (sp.lf + sp.lv) - lift;
            }
            w.ground[k * n + i] += m;
            w.air[k * na + j] -= m;
            (s.cap[i], s.capa[j], s.nt[j], s.mass[j]) = (s.cap[i] + m * sp.cp, s.capa[j] - m * sp.cp, s.nt[j] - m, s.mass[j] - m * sp.mm);
            lifted += lift;
        }
    }
    for &(_, k) in henry {
        if !s.loose.contains(&k) && (0..n).any(|i| w.ground[k * n + i] > 0.0) {
            s.loose.push(k);
            let at = s.vary.partition_point(|&x| s.sp[x].melt < s.sp[k].melt);
            s.vary.insert(at, k);
        }
    }
    lifted
}

/// Solve x' + (out − in) = x along a line of nodes, in place, where edge p (between nodes
/// p and p + 1) carries al[p]·x'[p] − be[p]·x'[p + 1]. `c` is work space.
fn thomas(al: &[f64], be: &[f64], x: &mut [f64], c: &mut [f64]) {
    let len = x.len();
    for p in 0..len {
        let lo = if p > 0 { -al[p - 1] } else { 0.0 };
        let di = 1.0 + if p > 0 { be[p - 1] } else { 0.0 } + if p + 1 < len { al[p] } else { 0.0 };
        let m = di - if p > 0 { lo * c[p - 1] } else { 0.0 };
        c[p] = if p + 1 < len { -be[p] / m } else { 0.0 };
        x[p] = (x[p] - if p > 0 { lo * x[p - 1] } else { 0.0 }) / m;
    }
    for p in (0..len - 1).rev() {
        x[p] -= c[p] * x[p + 1];
    }
}

/// Work space of the solver for one line of air nodes.
#[derive(Default)]
struct Lane {
    push: Vec<[f64; 3]>,
    halves: Vec<[f64; 2]>,
    to_next: Vec<f64>,
    to_prev: Vec<f64>,
    x: Vec<f64>,
    c: Vec<f64>,
    gas: Vec<f64>,
}

/// Gas between neighbouring air nodes, by three pushes on each edge: the pressure of the whole
/// column (p = nRT/V) drives gas from high to low; the warmth difference at equal height turns the
/// columns over (the lower half flows to the warmer node and the upper half back, both turned
/// sideways by rotation; condensable gases crowd in the lower half); eddies mix the two columns.
/// Solved implicitly along lines (east-west, then north-south), so any speed is stable. The lines
/// of one direction share no node, so each is solved from the amounts before the step (in
/// parallel, with `threads`) into a row [flux of each gas, of heat, mass lifted, kg moved] per
/// edge; the rows are applied in the order of the lines.
pub(super) fn wind(w: &mut World, s: &mut Scratch, dt: f64) {
    let World { cfg, grid, air, e_air, ledger, .. } = w;
    let (na, aw, ah, ng, g) = (grid.na(), grid.aw(), grid.ah(), s.gas.len(), cfg.gravity);
    let side = grid.cell_m * grid.air_scale as f64;
    let mut rows = std::mem::take(&mut s.lanes);
    let mut lift = 0.0;
    for dir in 0..2 {
        s.nt_new.clone_from(&s.nt);
        for j in 0..na {
            s.ta[j] = e_air[j] / s.capa[j];
            s.node_tmp[j] = s.nt[j] * R_GAS * s.ta[j] / s.vol[j];
            s.warmth[j] = s.ta[j] + s.gam[j] * s.zair[j];
        }
        for j in 0..na {
            let (x, y) = (j % aw, j / aw);
            let (e, wst) = (if x + 1 < aw { j + 1 } else { j }, if x > 0 { j - 1 } else { j });
            let (no, so) = (if y > 0 { j - aw } else { j }, if y + 1 < ah { j + aw } else { j });
            s.grad[j] = [(s.warmth[e] - s.warmth[wst]) / (e - wst).max(1) as f64, (s.warmth[no] - s.warmth[so]) / ((so - no) / aw).max(1) as f64];
        }
        let (lines, len) = if dir == 0 { (ah, aw) } else { (aw, ah) };
        let node = |line: usize, p: usize| if dir == 0 { line * aw + p } else { p * aw + line };
        let (flux, heat, lifted, moved, width) = (0, ng * len, (ng + 1) * len, (ng + 2) * len, (ng + 3) * len);
        rows.resize(lines * width, 0.0);
        let (sc, air_now, e_now) = (&*s, &*air, &*e_air);
        crate::par::rows(&mut rows, width, Lane::default, |ln, line, row| {
            ln.push.resize(len, [0.0; 3]);
            ln.halves.resize(len, [0.0; 2]);
            for v in [&mut ln.to_next, &mut ln.to_prev, &mut ln.x, &mut ln.c] {
                v.resize(len, 0.0);
            }
            ln.gas.resize(ng * len, 0.0);
            for p in 0..len - 1 {
                let (a, b) = (node(line, p), node(line, p + 1));
                let rho = (sc.nt[a] / sc.vol[a] + sc.nt[b] / sc.vol[b]) / 2.0;
                let depth = cfg.air_top_m - (sc.za[a] + sc.za[b]) / 2.0;
                let lat_row = if dir == 0 { line * grid.air_scale + grid.air_scale / 2 } else { (p + 1) * grid.air_scale };
                let turn = cfg.rotation * grid.lat(lat_row.min(grid.h - 1), cfg.lat_south, cfg.lat_north).to_radians().sin();
                let across = (sc.grad[a][1 - dir] + sc.grad[b][1 - dir]) / 2.0;
                let cap = OVER * sc.nt[a].min(sc.nt[b]) / dt;
                let over = (cfg.wind * (sc.warmth[b] - sc.warmth[a] + turn * across) * rho * side * depth / 2.0).clamp(-cap, cap);
                let even = EVEN * (sc.node_tmp[a] - sc.node_tmp[b]) / (sc.node_tmp[a] / sc.nt[a] + sc.node_tmp[b] / sc.nt[b]) / dt;
                ln.push[p] = [cfg.mix * rho * depth, over, even];
            }
            // Gases, split into lower and upper halves by the amounts before the step. Overturning
            // moves as much up as down: any net amount beyond the pressure push is handed back.
            for p in 0..len {
                ln.halves[p] = [0.0; 2];
                for t in 0..ng {
                    let j = node(line, p);
                    let a = air_now[sc.gas[t] * na + j];
                    let u = a * sc.split[t * na + j];
                    ln.halves[p][0] += a - u;
                    ln.halves[p][1] += u;
                }
            }
            for t in 0..ng {
                for p in 0..len - 1 {
                    let (a, b) = (node(line, p), node(line, p + 1));
                    let [mix, over, even] = ln.push[p];
                    let (fp, fm, ep, em) = (over.max(0.0), (-over).max(0.0), even.max(0.0), (-even).max(0.0));
                    let ([la, ua], [lb, ub]) = (ln.halves[p], ln.halves[p + 1]);
                    let (sa, sb) = (sc.split[t * na + a], sc.split[t * na + b]);
                    ln.to_next[p] = dt * ((mix + ep) / sc.nt[a] + fp * (1.0 - sa) / la + fm * sa / ua);
                    ln.to_prev[p] = dt * ((mix + em) / sc.nt[b] + fp * sb / ub + fm * (1.0 - sb) / lb);
                }
                let k = sc.gas[t];
                for p in 0..len {
                    ln.x[p] = air_now[k * na + node(line, p)];
                }
                thomas(&ln.to_next[..len - 1], &ln.to_prev[..len - 1], &mut ln.x[..len], &mut ln.c[..len]);
                ln.gas[t * len..(t + 1) * len].copy_from_slice(&ln.x[..len]);
                for p in 0..len - 1 {
                    row[flux + t * len + p] = ln.to_next[p] * ln.x[p] - ln.to_prev[p] * ln.x[p + 1];
                }
            }
            for p in 0..len {
                ln.x[p] = (0..ng).map(|t| ln.gas[t * len + p]).sum();
            }
            for p in 0..len - 1 {
                let (a, b, even) = (node(line, p), node(line, p + 1), ln.push[p][2]);
                let want = dt * (even.max(0.0) * ln.x[p] / sc.nt[a] - (-even).max(0.0) * ln.x[p + 1] / sc.nt[b]);
                let extra = (0..ng).map(|t| row[flux + t * len + p]).sum::<f64>() - want;
                let src = if extra > 0.0 { p + 1 } else { p };
                for t in 0..ng {
                    row[flux + t * len + p] -= extra * ln.gas[t * len + src] / ln.x[src];
                }
            }
            for p in 0..len - 1 {
                row[moved + p] = 0.0;
                for t in 0..ng {
                    row[moved + p] += row[flux + t * len + p] * sc.sp[sc.gas[t]].mm;
                }
            }
            // Heat moves with the gas, per mol its node's mean; mass moved up or down is lifted by the heat of the node it enters.
            for p in 0..len - 1 {
                let (a, b, [mix, over, even]) = (node(line, p), node(line, p + 1), ln.push[p]);
                ln.to_next[p] = dt * (mix + over.abs() + even.max(0.0)) / sc.nt[a];
                ln.to_prev[p] = dt * (mix + over.abs() + (-even).max(0.0)) / sc.nt[b];
            }
            for p in 0..len {
                ln.x[p] = e_now[node(line, p)];
            }
            thomas(&ln.to_next[..len - 1], &ln.to_prev[..len - 1], &mut ln.x[..len], &mut ln.c[..len]);
            for p in 0..len - 1 {
                let (a, b) = (node(line, p), node(line, p + 1));
                row[heat + p] = ln.to_next[p] * ln.x[p] - ln.to_prev[p] * ln.x[p + 1];
                row[lifted + p] = g * row[moved + p] * (sc.zair[b] - sc.zair[a]);
            }
        });
        for (line, row) in rows.chunks(width).enumerate() {
            for t in 0..ng {
                let (k, sp) = (s.gas[t], s.sp[s.gas[t]]);
                for p in 0..len - 1 {
                    let (a, b, f) = (node(line, p), node(line, p + 1), row[flux + t * len + p]);
                    air[k * na + a] -= f;
                    air[k * na + b] += f;
                    (s.nt_new[a], s.nt_new[b]) = (s.nt_new[a] - f, s.nt_new[b] + f);
                    (s.capa[a], s.capa[b]) = (s.capa[a] - f * sp.cp, s.capa[b] + f * sp.cp);
                    (s.mass[a], s.mass[b]) = (s.mass[a] - f * sp.mm, s.mass[b] + f * sp.mm);
                }
            }
            for p in 0..len - 1 {
                let (a, b, f, l) = (node(line, p), node(line, p + 1), row[heat + p], row[lifted + p]);
                e_air[a] -= f;
                e_air[b] += f;
                e_air[if row[moved + p] > 0.0 { b } else { a }] -= l;
                lift += l;
            }
        }
        s.nt.clone_from(&s.nt_new);
    }
    s.lanes = rows;
    ledger.gravity -= lift;
    for &k in &s.gas {
        for x in &mut air[k * na..(k + 1) * na] {
            if *x < 0.0 {
                ledger.clamped -= *x;
                *x = 0.0;
            }
        }
    }
    for j in 0..na {
        s.ta[j] = e_air[j] / s.capa[j];
    }
}
