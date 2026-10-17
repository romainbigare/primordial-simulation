//! Flow in and through the air: radiation, contact heat, evaporation and condensation,
//! and gas between air nodes.

mod exchange;
mod radiation;
mod wind;
pub(super) use exchange::exchange;
pub(crate) use radiation::LAYERS;
pub(super) use radiation::radiation;
pub use radiation::{column, glow};
pub(super) use wind::wind;

use super::Scratch;
use crate::math::one_minus_exp;
use crate::units::R_GAS;
use crate::World;
use std::f64::consts::LN_2;

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
        s.seen[i] = [v[0], v[1], one_minus_exp(v[2]), s.seen[i][3], v[3]];
    }
    s.ta0.copy_from_slice(&s.ta);
}
