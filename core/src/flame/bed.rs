//! The first stage of the flames' month (laws.md 3c): what the remains lying on each land cell are to a flame (`Bed`),
//! read by each cell of its own numbers alone.

use super::{CARRY_SHARE, SORB_HIGHEST, SORB_T};
use crate::cover::{dec, per_input, Parts, LEAF, MATTER, WALL};
use crate::life::WET_TISSUE;
use crate::World;

/// What the remains lying on a land cell are to a flame this month: their load, kg a m² of land; the share of the
/// ground they cover; the liquid they hold per kg dry (their moisture) and the most they could hold and still carry a
/// flame (their moisture of extinction); what a kg of them frees burning, J, and what lighting a kg of them takes, J.
#[derive(Clone, Copy, Debug)]
pub struct Bed {
    pub load: f64,
    pub cover: f64,
    pub moist: f64,
    pub extinct: f64,
    pub heat: f64,
    pub light: f64,
}

impl Bed {
    /// Whether a flame runs through it: its remains cover the ground past the site-percolation threshold, so they join
    /// across it (`CARRY_SHARE`), and the heat a burning kg sends ahead dries and lights the next kg (`extinct`).
    pub fn carries(&self) -> bool {
        self.cover >= CARRY_SHARE && self.moist < self.extinct
    }
}

/// The remains lying on land cell `i` as a flame meets them this month (`Bed`); none on a cell with no land, no
/// remains or no air read yet.
pub fn bed(w: &World, p: &Parts, i: usize) -> Option<Bed> {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let load = crate::contract::remains_in(w, i, |k| w.g(k, i));
    if load <= 0.0 {
        return None;
    }
    let cover = 1.0 - (-leaf_per_kg(w, p, i) * load).exp();
    if cover < CARRY_SHARE {
        return None;
    }
    let t = w.thaw_last(i).0;
    let moist = moisture(w, p, i, t)?;
    // Per kg of the remains, by the mass of their parts: the heat their burning frees and their heat capacity.
    let mm = p.mm(chem);
    let kg = [MATTER, WALL].map(|q| w.g(p.sub[q], i).max(0.0) * mm[q]);
    let all = (kg[0] + kg[1]).max(f64::MIN_POSITIVE);
    let per_kg = |q: usize, j: usize, of: &dyn Fn(usize) -> f64| kg[j] / all * of(q);
    let burnt = |q: usize| chem.burner(p.sub[q]).map_or(0.0, |r| -chem.dh(r) / (per_input(&chem.reactions[r], p.sub[q]).unwrap_or(1.0) * mm[q]));
    let cp = |q: usize| chem.subs[p.sub[q]].cp / mm[q];
    let heat = per_kg(MATTER, 0, &burnt) + per_kg(WALL, 1, &burnt);
    let c_dry = per_kg(MATTER, 0, &cp) + per_kg(WALL, 1, &cp);
    // Lighting a kg: warming it to `t_light`, and its liquid to its boiling point and boiling it off (Rothermel 1972).
    let liquid = &chem.subs[p.liquid];
    let dry = c_dry * (ph.t_light - t).max(0.0);
    let per_liquid = (liquid.cp * (liquid.boil - t).max(0.0) + liquid.lv) / liquid.molar_mass;
    let light = dry + moist * per_liquid;
    // The most liquid a kg may hold where the share `flux_ahead` of its heat still lights the next kg.
    let extinct = (ph.flux_ahead * heat - dry) / per_liquid;
    Some(Bed { load, cover, moist, extinct, heat, light })
}

/// The capture area a kg of the remains of cell `i` held as leaves, m² per kg dry: its cover's `LEAF` gene over the
/// dry share of living tissue, its slots' mean by their built cover (their leaves fall as its remains); none with no
/// cover.
fn leaf_per_kg(w: &World, p: &Parts, i: usize) -> f64 {
    let (c, mm) = (w.life.cover(i), p.mm(&w.chem));
    let (sum, all) = c.held().fold((0.0, 0.0), |(s, a), x| {
        let b = c.built(&mm, x);
        (s + b * dec(c.genes[x][LEAF]) / (1.0 - WET_TISSUE), a + b)
    });
    if all > 0.0 { sum / all } else { 0.0 }
}

/// The liquid the remains on cell `i` at `t` K hold per kg dry: their sorption curve (Nelson 2000) at the relative
/// vapour of the air in their bed, which lies between the air near the ground's (`World::near_ground`) and saturation
/// by the share of what its ground holds for roots that it holds (`cover::soak`, `Pores::available`: the share by which
/// the ground's evaporation draws its liquid; Manabe 1969). None before the air near the ground is read.
fn moisture(w: &World, p: &Parts, i: usize, t: f64) -> Option<f64> {
    let ph = &w.cfg.phys;
    let near = (w.near_ground(i, p.liquid)? / w.chem.p_sat(p.liquid, t)).clamp(0.0, 1.0);
    let wet = crate::cover::soak(w, p, i, w.g(p.liquid, i).max(0.0)).1.moist;
    let h = (near + wet * (1.0 - near)).min(SORB_HIGHEST);
    let at = |c: [f64; 2]| c[0] + c[1] * (t - SORB_T);
    Some(at(ph.sorb_scale) * (-(1.0 - h).ln()).powf(at(ph.sorb_power)))
}
