//! Flow in and through the air, solved together over a step (`step`; laws.md 1.4). Here: the state of the air nodes.

mod book;
mod exchange;
mod newton;
mod radiation;
mod sky;
mod step;
mod wind;

pub(super) use exchange::Sat;
pub(crate) use radiation::LAYERS;
pub use radiation::{column, declination, glow, insolation};
pub(super) use step::{settle, step};

use super::Scratch;
use crate::AIR;
use crate::math::one_minus_exp;
use crate::units::R_GAS;
use crate::World;
use std::f64::consts::LN_2;
use crate::store::Class;

/// Least depth of the air over the ground, m (the air's top is `Config::air_top_m`).
const COLUMN_MIN: f64 = 100.0;
/// The fall of a saturation pressure per K is taken from this far below to this far above, K.
const HALF_K: f64 = 0.5;

/// Least heat capacity an air node's temperature divides by, J/K (a node emptied of gas reads 0 K).
const CAPA_MIN: f64 = 1.0;

/// Temperature of an air node of energy `e`, J, and heat capacity `capa`, J/K.
#[inline]
pub(crate) fn air_temp(e: f64, capa: f64) -> f64 {
    e / capa.max(CAPA_MIN)
}

/// The air seen by ground cell `i`: a node field interpolated from the four nearest nodes.
#[inline]
pub(super) fn lerp(s: &Scratch, v: &[f64], i: usize) -> f64 {
    let (j, w) = (&s.near[i], &s.wt[i]);
    w[0] * v[j[0] as usize] + w[1] * v[j[1] as usize] + w[2] * v[j[2] as usize] + w[3] * v[j[3] as usize]
}

/// The four air nodes ground cell `i` sees, each with the share it gives of the node field `v` the cell sees (`lerp`):
/// what condenses there comes out of each node in that share, so no node gives more than it holds.
#[inline]
pub(super) fn shares(s: &Scratch, v: &[f64], i: usize) -> [(usize, f64); 4] {
    let (j, w, seen) = (&s.near[i], &s.wt[i], lerp(s, v, i));
    std::array::from_fn(|c| (j[c] as usize, if seen > 0.0 { w[c] * v[j[c] as usize] / seen } else { 0.0 }))
}

/// The share of the air ground cell `i` sees that lies over it: its area over the area of the air it sees (`lerp`).
#[inline]
pub(super) fn over_cell(w: &World, s: &Scratch, i: usize) -> f64 {
    let mesh = &w.mesh;
    mesh.cells().area()[i] / lerp(s, mesh.level(AIR).area(), i)
}

/// Scale height of air at `t` K whose `mol` mol weigh `kg` kg under gravity `g`, RT/(Mg), m.
#[inline]
pub fn scale_height(t: f64, mol: f64, kg: f64, g: f64) -> f64 {
    R_GAS * t * mol / (g * kg)
}

/// Scale height of the air of node `j`, m.
#[inline]
fn node_scale_height(s: &Scratch, j: usize, g: f64) -> f64 {
    scale_height(s.ta[j], s.nt[j], s.mass[j], g)
}

/// The state of every air node, the air each ground cell sees, and how high each gas fills its column (a gas that would
/// condense higher up is held near the ground).
pub(super) fn state(w: &World, s: &mut Scratch) {
    nodes(w, s);
    heights(w, s);
    seen(w, s);
}

/// Amount, mol, heat capacity, J/K, and mass, kg, of the gases of each air node, from its amounts now.
pub(super) fn sums(w: &World, s: &mut Scratch) {
    let na = w.mesh.level(AIR).n();
    (s.nt, s.capa, s.mass) = (vec![0.0; na], vec![0.0; na], vec![0.0; na]);
    for &k in &s.state.gas {
        let sp = s.sp[k];
        for (j, &a) in w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().enumerate() {
            s.nt[j] += a;
            s.capa[j] += a * sp.cp;
            s.mass[j] += a * sp.mm;
        }
    }
}

/// Amounts, heat capacity, mass, optical depth, volume, temperature and mean height of each air node.
fn nodes(w: &World, s: &mut Scratch) {
    let (na, g, top) = (w.mesh.level(AIR).n(), w.cfg.gravity, w.cfg.air_top_m);
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    sums(w, s);
    s.za.fill(0.0);
    s.fabs.fill(0.0);
    for (i, &j) in cells.parent().iter().enumerate() {
        s.za[j as usize] += s.head[i] * cells.area()[i];
    }
    for &k in s.state.gas.iter().filter(|&&k| w.chem.subs[k].ir_abs > 0.0) {
        for (j, &a) in w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().enumerate() {
            s.fabs[j] += w.chem.depth(k, a / air.area()[j]);
        }
    }
    for j in 0..na {
        s.za[j] /= air.area()[j];
        let (a, l) = (air.area()[j], (top - s.za[j]).max(COLUMN_MIN));
        s.vol[j] = a * l;
        s.ta[j] = air_temp(w.store.heat(Class::Air)[j], s.capa[j]);
        s.gam[j] = g * s.mass[j] / s.capa[j].max(CAPA_MIN);
        let h = node_scale_height(s, j, g);
        s.zair[j] = s.za[j] + h - l / (l / h).exp_m1();
    }
}

/// Which gases may condense, the air's temperature drop with height (slower where one condenses), and per gas and node
/// the height it fills and its share in the upper half of the column.
fn heights(w: &World, s: &mut Scratch) {
    let (na, g, top) = (w.mesh.level(AIR).n(), w.cfg.gravity, w.cfg.air_top_m);
    let ng = s.state.gas.len();
    (s.cond, s.hk, s.split) = (vec![false; ng], vec![0.0; ng * na], vec![0.0; ng * na]);
    // The heat capacity each node gains from the vaporisation enthalpy of what condenses as it cools, J/K.
    let mut held = vec![0.0; na];
    for (gi, &k) in s.state.gas.iter().enumerate() {
        s.cond[gi] = (0..na).any(|j| {
            let t = (s.ta[j] - s.gam[j] * (top - s.zair[j])).max(1.0);
            w.chem.p_sat(k, t) < w.store.amounts(Class::Air)[k * na + j] * R_GAS * s.ta[j] / s.vol[j]
        });
    }
    // Rising air in which a gas condenses cools slower: its vaporisation enthalpy is freed (Clausius-Clapeyron).
    for (gi, &k) in s.state.gas.iter().enumerate() {
        if s.cond[gi] {
            for (j, h) in held.iter_mut().enumerate() {
                let (t, lv) = (s.ta[j], s.sp[k].lv);
                let x = w.chem.p_sat(k, t) * s.vol[j] / (s.nt[j] * R_GAS * t);
                *h += s.nt[j] * lv * lv * x.min(1.0) / (R_GAS * t * t);
            }
        }
    }
    for (j, h) in held.iter().enumerate() {
        s.gam[j] = g * s.mass[j] / (s.capa[j] + h).max(CAPA_MIN);
    }
    // The vapour pressure falls by exp(lv/R (1/(t-½) - 1/(t+½))) per K; a gas that does not condense spreads like the air.
    for (gi, &k) in s.state.gas.iter().enumerate() {
        for j in 0..na {
            let h = node_scale_height(s, j, g).min((top - s.za[j]).max(COLUMN_MIN));
            let (mut hk, mut split) = (h, 0.5);
            if s.cond[gi] {
                let t = s.ta[j] + s.gam[j] * (s.zair[j] - s.za[j]);
                let fall = s.sp[k].lv / R_GAS * (1.0 / (t - HALF_K).max(1.0) - 1.0 / (t + HALF_K).max(1.0));
                hk = (1.0 / (s.gam[j] * fall)).min(h);
                split = (-(h / hk) * LN_2).exp();
            }
            s.hk[gi * na + j] = hk;
            s.split[gi * na + j] = split;
        }
    }
}

/// The air each ground cell sees (from the four nearest nodes, `lerp`) and the layers of its column.
fn seen(w: &World, s: &mut Scratch) {
    let (n, g, top) = (w.mesh.cells().n(), w.cfg.gravity, w.cfg.air_top_m);
    s.lay.resize(n, [[0.0; LAYERS]; 2]);
    for (i, &j) in w.mesh.cells().parent().iter().enumerate() {
        let (gam, zair, fabs) = (lerp(s, &s.gam, i), lerp(s, &s.zair, i), lerp(s, &s.fabs, i));
        s.lay[i] = column(fabs, gam, (top - s.head[i]).max(COLUMN_MIN), node_scale_height(s, j as usize, g));
        s.seen[i] = [gam, zair, one_minus_exp(fabs)];
    }
}
