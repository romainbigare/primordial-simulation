//! Flow in and through the air: radiation, contact heat, evaporation and condensation,
//! and gas between air nodes.

use super::{range, settle, Scratch, HIDE_M};
use crate::chem::R_GAS;
use crate::World;
use std::f64::consts::{LN_2, PI, TAU};

const SIGMA: f64 = 5.670e-8;
/// Share of the pressure difference between two air nodes evened out per call.
const EVEN: f64 = 0.25;
/// Most of a node's amount the overturning may exchange over one edge per call.
const OVER: f64 = 3.0;

/// The air seen by ground cell `i`: a node field interpolated from the four nearest nodes.
#[inline]
pub(super) fn lerp(s: &Scratch, v: &[f64], i: usize) -> f64 {
    let (j, w) = (&s.near[i], &s.wt[i]);
    w[0] * v[j[0] as usize] + w[1] * v[j[1] as usize] + w[2] * v[j[2] as usize] + w[3] * v[j[3] as usize]
}

/// Amounts, heat capacity, mass, heights, volume and heat-absorbing share of every air node,
/// the air each ground cell sees, and for each gas how high it fills its column: a gas that
/// would condense higher up is held near the ground, by how fast its vapour pressure falls
/// with the temperature falling upwards.
pub(super) fn state(w: &World, s: &mut Scratch) {
    let (n, na, g, top) = (w.grid.n(), w.grid.na(), w.cfg.gravity, w.cfg.air_top_m);
    for j in 0..na {
        (s.za[j], s.nt[j], s.capa[j], s.mass[j], s.fabs[j]) = (0.0, 0.0, 0.0, 0.0, 0.0);
    }
    for i in 0..n {
        s.za[s.air_of[i] as usize] += s.head[i];
    }
    for &k in &s.gas {
        let (sp, ir) = (s.sp[k], w.chem.subs[k].ir_abs);
        for (j, &a) in w.air[k * na..(k + 1) * na].iter().enumerate() {
            s.nt[j] += a;
            s.capa[j] += a * sp.cp;
            s.mass[j] += a * sp.mm;
            s.fabs[j] += a * ir;
        }
    }
    for j in 0..na {
        s.za[j] /= s.cells[j];
        let (a, l) = (s.area * s.cells[j], (top - s.za[j]).max(100.0));
        s.vol[j] = a * l;
        s.fabs[j] = 1.0 - (-s.fabs[j] / a).exp();
        s.ta[j] = w.e_air[j] / s.capa[j];
        s.gam[j] = g * s.mass[j] / s.capa[j];
        let h = R_GAS * s.ta[j] * s.nt[j] / (g * s.mass[j]);
        s.zair[j] = s.za[j] + h - l / (l / h).exp_m1();
    }
    for i in 0..n {
        let (ta, f) = (lerp(s, &s.ta, i), lerp(s, &s.fabs, i));
        s.view[i] = [(ta - lerp(s, &s.gam, i) * (s.head[i] - lerp(s, &s.zair, i))).max(1.0), f, f * SIGMA * ta.powi(4)];
    }
    let ng = s.gas.len();
    (s.cond, s.hk, s.split) = (vec![false; ng], vec![0.0; ng * na], vec![0.0; ng * na]);
    for (gi, &k) in s.gas.iter().enumerate() {
        s.cond[gi] = (0..na).any(|j| {
            let t = (s.ta[j] - s.gam[j] * (top - s.zair[j])).max(1.0);
            w.chem.p_sat(k, t) < w.air[k * na + j] * R_GAS * s.ta[j] / s.vol[j]
        });
        for j in 0..na {
            let h = (R_GAS * s.ta[j] * s.nt[j] / (g * s.mass[j])).min((top - s.za[j]).max(100.0));
            let mut hk = h;
            if s.cond[gi] {
                let t = s.ta[j] + s.gam[j] * (s.zair[j] - s.za[j]);
                hk = (1.0 / (s.gam[j] * (w.chem.p_sat(k, t + 0.5) / w.chem.p_sat(k, t - 0.5)).ln())).min(h);
            }
            s.hk[gi * na + j] = hk;
            s.split[gi * na + j] = (-(h / hk) * LN_2).exp();
        }
    }
}

/// Daily-mean sunlight at the top of the air, W/m².
fn insolation(s0: f64, lat: f64, decl: f64) -> f64 {
    let h0 = (-lat.tan() * decl.tan()).clamp(-1.0, 1.0).acos();
    (s0 / PI * (h0 * lat.sin() * decl.sin() + lat.cos() * decl.cos() * h0.sin())).max(0.0)
}

/// Sunlight by latitude and season, reflected by what covers the ground; the ground emits
/// σT⁴, of which the air absorbs its share and emits as much up as down (two layers); and
/// contact heat with the air touching the ground (warmer than the air's mean by the drop
/// from the air's mean height). The ground temperature is solved implicitly.
pub(super) fn radiation(w: &mut World, s: &mut Scratch, dt: f64) {
    let season = w.season();
    let World { cfg, grid, e_ground, e_air, ledger, canopy, light, .. } = w;
    let (n, na, area) = (grid.n(), grid.na(), s.area);
    let decl = -cfg.tilt.to_radians() * (TAU * season).cos();
    let (mut sun, mut out, mut row, mut q) = (0.0, 0.0, usize::MAX, 0.0);
    s.p.fill(0.0);
    for i in 0..n {
        if i / grid.w != row {
            row = i / grid.w;
            q = insolation(cfg.sun, grid.lat(row, cfg.lat_south, cfg.lat_north).to_radians(), decl);
        }
        let ([tl, f, down], t) = (s.view[i], s.tg[i]);
        // Bodies with light-capture area `a` per ground area intercept a share 1 - exp(-a).
        let a = canopy[i] as f64;
        let cover = -(-a).exp_m1();
        if a > 0.0 {
            light[i] += q * cover * dt / a;
        }
        let sw = q * (1.0 - cover) * (1.0 - s.alb[i]);
        let (k1, t3) = (s.cap[i] / (area * dt), SIGMA * t * t * t);
        let t1 = (k1 * t + sw + down + 3.0 * t3 * t + cfg.sensible * tl) / (k1 + 4.0 * t3 + cfg.sensible);
        let up = t3 * (4.0 * t1 - 3.0 * t);
        let hs = cfg.sensible * (t1 - tl);
        e_ground[i] += area * dt * (sw + down - up - hs);
        s.p[s.air_of[i] as usize] += area * dt * (f * up + hs - 2.0 * down);
        sun += area * dt * sw;
        out += area * dt * ((1.0 - f) * up + down);
    }
    for j in 0..na {
        e_air[j] += s.p[j];
        s.ta[j] = e_air[j] / s.capa[j];
    }
    ledger.sun_in += sun;
    ledger.space_out += out;
}

/// Evaporation by vapour pressure (the ground's saturation pressure against the partial
/// pressure near the ground), gases leaving the ground, and condensation where the air a
/// ground cell sees holds more than it can at its temperature (colder on higher ground).
pub(super) fn exchange(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, na, area, g) = (w.grid.n(), w.grid.na(), s.area, w.cfg.gravity);
    let (_, tmax) = range(&s.tg);
    for gi in 0..s.gas.len() {
        let (k, sp) = (s.gas[gi], s.sp[s.gas[gi]]);
        if !s.loose.contains(&k) || w.chem.p_sat(k, tmax) <= 1.0 {
            continue;
        }
        s.snap.clear();
        s.snap.extend_from_slice(&w.air[k * na..(k + 1) * na]);
        for i in 0..n {
            let (a, t, j) = (w.ground[k * n + i], s.tg[i], s.air_of[i] as usize);
            if a <= 0.0 {
                continue;
            }
            let m = if t >= sp.boil {
                a
            } else {
                let (ps, hk, tl) = (w.chem.p_sat(k, t), lerp(s, &s.hk[gi * na..(gi + 1) * na], i), s.view[i][0]);
                let pa = lerp(s, &s.snap, i) * s.per[i] * R_GAS * tl / (area * hk);
                if ps <= pa {
                    continue;
                }
                let d = a * sp.v[(t >= sp.melt) as usize] / area;
                let rate = w.cfg.evap * (ps - pa) / (R_GAS * t) * d / (d + HIDE_M[1]) * area * dt;
                rate.min((ps - pa) * area * hk / (R_GAS * tl)).min(a)
            };
            let heat = m * sp.cp * w.e_ground[i] / s.cap[i];
            let lift = m * g * sp.mm * (s.zair[j] - s.head[i]);
            settle(s, k, i, -m);
            w.ground[k * n + i] -= m;
            w.air[k * na + j] += m;
            (s.cap[i], s.capa[j], s.nt[j], s.mass[j]) = (s.cap[i] - m * sp.cp, s.capa[j] + m * sp.cp, s.nt[j] + m, s.mass[j] + m * sp.mm);
            w.e_ground[i] -= heat;
            w.e_air[j] += heat - lift;
            w.ledger.gravity -= lift;
        }
    }
    for gi in 0..s.gas.len() {
        let (k, sp) = (s.gas[gi], s.sp[s.gas[gi]]);
        if !s.cond[gi] {
            continue;
        }
        s.snap.clear();
        s.snap.extend_from_slice(&w.air[k * na..(k + 1) * na]);
        for i in 0..n {
            let (j, tl) = (s.air_of[i] as usize, s.view[i][0]);
            let hold = w.chem.p_sat(k, tl) * area * lerp(s, &s.hk[gi * na..(gi + 1) * na], i) / (R_GAS * tl);
            let m = (lerp(s, &s.snap, i) * s.per[i] - hold).min(w.air[k * na + j]);
            if m <= 0.0 {
                continue;
            }
            let tair = w.e_air[j] / s.capa[j];
            let lift = m * g * sp.mm * (s.head[i] - s.zair[j]);
            settle(s, k, i, m);
            w.ground[k * n + i] += m;
            w.air[k * na + j] -= m;
            (s.cap[i], s.capa[j], s.nt[j], s.mass[j]) = (s.cap[i] + m * sp.cp, s.capa[j] - m * sp.cp, s.nt[j] - m, s.mass[j] - m * sp.mm);
            w.e_ground[i] += m * sp.cp * tair - lift;
            w.e_air[j] -= m * sp.cp * tair;
            w.ledger.gravity -= lift;
            s.rain[i] += m;
        }
        if !s.loose.contains(&k) && (0..n).any(|i| s.rain[i] > 0.0) {
            s.loose.push(k);
            s.vary.push(k);
        }
    }
    let ema = (dt / (w.days_per_year() as f64 * 86_400.0)) as f32;
    for i in 0..n {
        let r = (s.rain[i] / (area * dt)) as f32;
        w.diag.rain[i] += (r - w.diag.rain[i]) * ema;
        s.rain[i] = 0.0;
    }
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

/// Gas between neighbouring air nodes, by three pushes on each edge: the pressure of the
/// whole column (p = nRT/V) drives gas from high to low; the warmth difference at equal
/// height turns the columns over (the lower half flows to the warmer node and the upper half
/// back, both turned sideways by rotation; condensable gases crowd in the lower half); and
/// eddies mix the two columns. Solved implicitly along lines (east-west, then north-south),
/// so any speed is stable; the edge fluxes of the solution are applied to both nodes.
pub(super) fn wind(w: &mut World, s: &mut Scratch, dt: f64) {
    let World { cfg, grid, air, e_air, ledger, .. } = w;
    let (na, aw, ah, ng, g) = (grid.na(), grid.aw(), grid.ah(), s.gas.len(), cfg.gravity);
    let (side, len_max) = (grid.cell_m * grid.air_scale as f64, aw.max(ah));
    for v in [&mut s.al, &mut s.be, &mut s.x, &mut s.cw, &mut s.mflux] {
        v.resize(len_max, 0.0);
    }
    (s.push, s.lh) = (vec![[0.0; 3]; len_max], vec![[0.0; 2]; len_max]);
    s.xs.resize(ng * len_max, 0.0);
    s.fl.resize(ng * len_max, 0.0);
    let mut lift = 0.0;
    for dir in 0..2 {
        s.nt_new.clone_from(&s.nt);
        for j in 0..na {
            s.ta[j] = e_air[j] / s.capa[j];
            s.p[j] = s.nt[j] * R_GAS * s.ta[j] / s.vol[j];
            s.th[j] = s.ta[j] + s.gam[j] * s.zair[j];
        }
        for j in 0..na {
            let (x, y) = (j % aw, j / aw);
            let (e, wst) = (if x + 1 < aw { j + 1 } else { j }, if x > 0 { j - 1 } else { j });
            let (no, so) = (if y > 0 { j - aw } else { j }, if y + 1 < ah { j + aw } else { j });
            s.grad[j] = [(s.th[e] - s.th[wst]) / (e - wst).max(1) as f64, (s.th[no] - s.th[so]) / ((so - no) / aw).max(1) as f64];
        }
        let (lines, len) = if dir == 0 { (ah, aw) } else { (aw, ah) };
        let node = |line: usize, p: usize| if dir == 0 { line * aw + p } else { p * aw + line };
        for line in 0..lines {
            for p in 0..len - 1 {
                let (a, b) = (node(line, p), node(line, p + 1));
                let rho = (s.nt[a] / s.vol[a] + s.nt[b] / s.vol[b]) / 2.0;
                let depth = cfg.air_top_m - (s.za[a] + s.za[b]) / 2.0;
                let row = if dir == 0 { line * grid.air_scale + grid.air_scale / 2 } else { (p + 1) * grid.air_scale };
                let turn = cfg.rotation * grid.lat(row.min(grid.h - 1), cfg.lat_south, cfg.lat_north).to_radians().sin();
                let across = (s.grad[a][1 - dir] + s.grad[b][1 - dir]) / 2.0;
                let cap = OVER * s.nt[a].min(s.nt[b]) / dt;
                let over = (cfg.wind * (s.th[b] - s.th[a] + turn * across) * rho * side * depth / 2.0).clamp(-cap, cap);
                let even = EVEN * (s.p[a] - s.p[b]) / (s.p[a] / s.nt[a] + s.p[b] / s.nt[b]) / dt;
                s.push[p] = [cfg.mix * rho * depth, over, even];
                s.mflux[p] = 0.0;
            }
            // Gases. The halves are refreshed from the previous solution (Picard); the
            // overturning moves as much up as down, so any net amount left over on an edge
            // beyond the pressure push is handed back by the node that gained it.
            for it in 0..3 {
                for p in 0..len {
                    s.lh[p] = [0.0; 2];
                    for t in 0..ng {
                        let (j, k) = (node(line, p), s.gas[t]);
                        let a = if it == 0 { air[k * na + j] } else { s.xs[t * len + p] };
                        let u = a * s.split[t * na + j];
                        s.lh[p][0] += a - u;
                        s.lh[p][1] += u;
                    }
                }
                for t in 0..ng {
                    for p in 0..len - 1 {
                        let (a, b) = (node(line, p), node(line, p + 1));
                        let [mix, over, even] = s.push[p];
                        let (fp, fm, ep, em) = (over.max(0.0), (-over).max(0.0), even.max(0.0), (-even).max(0.0));
                        let ([la, ua], [lb, ub]) = (s.lh[p], s.lh[p + 1]);
                        let (sa, sb) = (s.split[t * na + a], s.split[t * na + b]);
                        s.al[p] = dt * ((mix + ep) / s.nt[a] + fp * (1.0 - sa) / la + fm * sa / ua);
                        s.be[p] = dt * ((mix + em) / s.nt[b] + fp * sb / ub + fm * (1.0 - sb) / lb);
                    }
                    let k = s.gas[t];
                    for p in 0..len {
                        s.x[p] = air[k * na + node(line, p)];
                    }
                    thomas(&s.al[..len - 1], &s.be[..len - 1], &mut s.x[..len], &mut s.cw[..len]);
                    s.xs[t * len..(t + 1) * len].copy_from_slice(&s.x[..len]);
                    for p in 0..len - 1 {
                        s.fl[t * len + p] = s.al[p] * s.x[p] - s.be[p] * s.x[p + 1];
                    }
                }
            }
            for p in 0..len {
                s.x[p] = (0..ng).map(|t| s.xs[t * len + p]).sum();
            }
            for p in 0..len - 1 {
                let (a, b, even) = (node(line, p), node(line, p + 1), s.push[p][2]);
                let want = dt * (even.max(0.0) * s.x[p] / s.nt[a] - (-even).max(0.0) * s.x[p + 1] / s.nt[b]);
                let extra = (0..ng).map(|t| s.fl[t * len + p]).sum::<f64>() - want;
                let src = if extra > 0.0 { p + 1 } else { p };
                for t in 0..ng {
                    s.fl[t * len + p] -= extra * s.xs[t * len + src] / s.x[src];
                }
            }
            for t in 0..ng {
                let (k, sp) = (s.gas[t], s.sp[s.gas[t]]);
                for p in 0..len - 1 {
                    let (a, b, f) = (node(line, p), node(line, p + 1), s.fl[t * len + p]);
                    air[k * na + a] -= f;
                    air[k * na + b] += f;
                    (s.nt_new[a], s.nt_new[b]) = (s.nt_new[a] - f, s.nt_new[b] + f);
                    (s.capa[a], s.capa[b]) = (s.capa[a] - f * sp.cp, s.capa[b] + f * sp.cp);
                    (s.mass[a], s.mass[b]) = (s.mass[a] - f * sp.mm, s.mass[b] + f * sp.mm);
                    s.mflux[p] += f * sp.mm;
                }
            }
            // Heat moves with the gas, per mol its node's mean; mass moved up or down is
            // lifted by the heat of the node it enters.
            for p in 0..len - 1 {
                let (a, b, [mix, over, even]) = (node(line, p), node(line, p + 1), s.push[p]);
                s.al[p] = dt * (mix + over.abs() + even.max(0.0)) / s.nt[a];
                s.be[p] = dt * (mix + over.abs() + (-even).max(0.0)) / s.nt[b];
            }
            for p in 0..len {
                s.x[p] = e_air[node(line, p)];
            }
            thomas(&s.al[..len - 1], &s.be[..len - 1], &mut s.x[..len], &mut s.cw[..len]);
            for p in 0..len - 1 {
                let (a, b) = (node(line, p), node(line, p + 1));
                let f = s.al[p] * s.x[p] - s.be[p] * s.x[p + 1];
                e_air[a] -= f;
                e_air[b] += f;
                let l = g * s.mflux[p] * (s.zair[b] - s.zair[a]);
                e_air[if s.mflux[p] > 0.0 { b } else { a }] -= l;
                lift += l;
            }
        }
        s.nt.clone_from(&s.nt_new);
    }
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
