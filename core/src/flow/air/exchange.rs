use super::super::land::{melted, settle, wet};
use super::super::{follow, range, year_weight, Scratch};
use super::lerp;
use crate::math::one_minus_exp;
use crate::units::R_GAS;
use crate::World;

/// A gas making up less than this share of the air is a trace: it evaporates and condenses
/// only every `TRACE_TURN`-th call, for all the time since.
const TRACE_AIR: f64 = 1e-7;
const TRACE_TURN: u32 = 4;

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
pub(crate) fn exchange(w: &mut World, s: &mut Scratch, dt: f64) {
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
                let d = a * sp.v[melted(s, k, i) as usize] / area;
                // A wet surface evaporates at the open rate: liquid stands on it, or its loose ground holds its field
                // capacity or more (stage 1); a drier one goes with its liquid over that (stage 2: the bucket of Manabe 1969).
                // No more than fills the column above the cell to saturation (the rest would rain back at once).
                (c_evap * (ps - pa) / t * (d / (w.cfg.phys.field * s.pd[i])).min(1.0)).min(a).min((ps - pa) * c_cond * hk / tl)
            };
            // Evaporating cools the node, no further than where the vapour pressure falls to the
            // partial pressure (the boiling point, when it boils): only heat above that pays.
            let over = t - if t >= sp.boil { sp.boil } else { w.chem.t_sat(k, pa / crate::units::P_REF).min(t) };
            let m = m.min(s.cap[i].max(1.0) * over / (sp.cp * over + sp.lv));
            // The ground also pays the fusion and vaporisation enthalpy and the gas's heat; if it cannot, less evaporates.
            let m = w.take_paid(i, m, sp.cp, m * (sp.cp * t + sp.lf + sp.lv)).0;
            if m <= 0.0 {
                continue;
            }
            let (heat, lift) = (m * sp.cp * t, m * g * sp.mm * (s.zair[j] - s.head[i]));
            settle(s, k, i, -m);
            to_air(w, s, k, (i, j), m);
            w.e_air[j] += heat - lift;
            lifted += lift;
            if s.z[i] > 0.0 && t < sp.boil {
                w.diag.liquid_land[crate::Diag::LAND_GROUND] += m * sp.v[1];
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
            to_air(w, s, k, (i, j), -m);
            // Condensing frees the vaporisation enthalpy into the air; the liquid keeps its fusion.
            w.e_ground[i] += m * (sp.cp * tair + sp.lf) - lift;
            w.e_air[j] += m * (sp.lv - sp.cp * tair);
            lifted += lift;
            s.rain[i] += m;
            if s.z[i] > 0.0 {
                w.diag.liquid_land[crate::Diag::LAND_RAIN] += m * sp.v[1];
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
        s.add_loose(k, false);
    }
    // Condensate in the air scatters sunlight like the powder of its solid; the faster a
    // node condenses, the more of its sky it covers.
    for j in 0..na {
        let [m, mr] = std::mem::take(&mut s.cloud[j]);
        let rate = m / (area * s.cells[j] * dt);
        s.node_tmp[j] = if m > 0.0 { rate / (rate + w.cfg.cloud) * mr / m } else { 0.0 };
    }
    let ema = year_weight(w, dt);
    for i in 0..n {
        s.seen[i][3] = lerp(s, &s.node_tmp, i);
        follow(&mut w.rain[i], (s.rain[i] / (area * dt)) as f32, ema);
        s.rain[i] = 0.0;
    }
}

/// Move `m` mol of gas `k` from ground node `i` to air node `j` (negative `m`: from the air to the
/// ground), with its heat capacity and mass. The caller moves the heat and the surface.
fn to_air(w: &mut World, s: &mut Scratch, k: usize, (i, j): (usize, usize), m: f64) {
    let (n, na, p) = (w.grid.n(), w.grid.na(), s.sp[k]);
    w.ground[k * n + i] -= m;
    w.air[k * na + j] += m;
    (s.cap[i], s.capa[j], s.nt[j], s.mass[j]) = (s.cap[i] - m * p.cp, s.capa[j] + m * p.cp, s.nt[j] + m, s.mass[j] + m * p.mm);
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
    let solv = s.liquids(&w.chem, (tmin, tmax), false);
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
            let m = (b * (a + col) / (1.0 + b) - a) * one_minus_exp(dt * (1.0 + b) / tau);
            let m = if m >= 0.0 {
                m.min(w.air[k * na + j])
            } else {
                // Coming out cools the node, no further than the boiling point; it pays what it can.
                let q = (-m).min(s.cap[i].max(1.0) * (t - sp.boil) / (sp.cp * (t - sp.boil) + sp.lv));
                -crate::ground::take_paid(&mut w.e_ground[i], &mut w.ledger, s.cap[i], q, sp.cp, q * (sp.cp * t + sp.lf + sp.lv)).0
            };
            if m == 0.0 {
                continue;
            }
            let (heat, lift) = (m * sp.cp * if m > 0.0 { w.e_air[j] / s.capa[j] } else { t }, m * g * sp.mm * (s.head[i] - s.zair[j]));
            w.e_air[j] -= heat + if m < 0.0 { lift } else { 0.0 };
            if m > 0.0 {
                w.e_ground[i] += heat + m * (sp.lf + sp.lv) - lift;
            }
            to_air(w, s, k, (i, j), -m);
            lifted += lift;
        }
    }
    for &(_, k) in henry {
        if (0..n).any(|i| w.ground[k * n + i] > 0.0) {
            s.add_loose(k, true);
        }
    }
    lifted
}
