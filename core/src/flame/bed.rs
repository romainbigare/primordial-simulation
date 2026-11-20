//! The first stage of the flames' month (laws.md 3c): what the remains lying on each land cell are to a flame (`Bed`),
//! read by each cell of its own numbers alone.

use super::{CARRY_SHARE, SORB_HIGHEST, SORB_T};
use crate::chem::Phase;
use crate::cover::{dec, per_input, Parts, LEAF, MATTER, WALL};
use crate::life::WET_TISSUE;
use crate::units::SIGMA;
use crate::World;

/// What the remains lying on a land cell and the cured leaves standing over them (`Cover::cured`) are to a flame this
/// month: the load of the remains, kg a m² of land; the load of the cured leaves dry enough to flame, kg a m²; the share
/// of the ground they cover; the mean liquid per kg dry of what flames (its moisture) and the most a kg could hold and
/// still carry a flame (their moisture of extinction); what a kg of them frees burning, J, and what lighting a kg of
/// them takes, J; the least load whose flame holds its heat (`least`), kg a m²; and the shares of the remains' load dry
/// enough to flame and to smoulder (`layer`); the density of its particles, kg/m³ (the frame's); and the share of the
/// low wind that reaches the height of its flame (`sheltered`).
#[derive(Clone, Copy, Debug)]
pub struct Bed {
    pub load: f64,
    pub standing: f64,
    pub cover: f64,
    pub moist: f64,
    pub extinct: f64,
    pub heat: f64,
    pub light: f64,
    pub least: f64,
    pub flaming: f64,
    pub smoulder: f64,
    pub particle_rho: f64,
    pub waf: f64,
}

impl Bed {
    /// Whether a flame runs through it: its remains cover the ground past the site-percolation threshold, so they join
    /// across it (`CARRY_SHARE`), and the part dry enough that the heat a burning kg sends ahead dries and lights the
    /// next kg is as much as a flame needs to hold its heat (`least`).
    pub fn carries(&self) -> bool {
        self.cover >= CARRY_SHARE && self.flaming > 0.0 && self.flames() >= self.least
    }
    /// The load of its part that flames, kg a m² of land: the cured leaves and the top of the remains.
    pub fn flames(&self) -> f64 {
        self.standing + self.flaming * self.load
    }
    /// The depth of its part that flames, m: the cured leaves standing at `cured_rho`, the remains lying at
    /// `litter_rho`.
    pub fn depth(&self, ph: &crate::Physics) -> f64 {
        self.standing / ph.cured_rho + self.flaming * self.load / ph.litter_rho
    }
}

/// The remains lying on land cell `i` and the cured leaves standing over them as a flame meets them this month (`Bed`);
/// none on a cell with no land, nothing that burns, less than the least load even dry, or no air read yet.
pub fn bed(w: &World, p: &Parts, i: usize) -> Option<Bed> {
    let (chem, ph, mm) = (&w.chem, &w.cfg.phys, p.mm(&w.chem));
    let load = crate::contract::remains_in(w, i, |k| w.g(k, i));
    let land = crate::contract::land(w, i);
    let standing_dead = w.life.cover(i).cured.map(|m| m.max(0.0));
    let cured = if land > 0.0 { (standing_dead[MATTER] * mm[MATTER] + standing_dead[WALL] * mm[WALL]) / land } else { 0.0 };
    if load + cured <= 0.0 {
        return None;
    }
    let cover = 1.0 - (-leaf_per_kg(w, p, i) * (load + cured)).exp();
    if cover < CARRY_SHARE {
        return None;
    }
    let t = w.thaw_last(i).0;
    // Per kg of what burns, by the mass of its parts: the heat their burning frees and their heat capacity.
    let kg = [MATTER, WALL].map(|q| (w.g(p.sub[q], i).max(0.0) + standing_dead[q]) * mm[q]);
    let all = (kg[0] + kg[1]).max(f64::MIN_POSITIVE);
    let per_kg = |q: usize, j: usize, of: &dyn Fn(usize) -> f64| kg[j] / all * of(q);
    let burnt = |q: usize| chem.burner(p.sub[q]).map_or(0.0, |r| -chem.dh(r) / (per_input(&chem.reactions[r], p.sub[q]).unwrap_or(1.0) * mm[q]));
    let cp = |q: usize| chem.subs[p.sub[q]].cp / mm[q];
    let heat = per_kg(MATTER, 0, &burnt) + per_kg(WALL, 1, &burnt);
    let c_dry = per_kg(MATTER, 0, &cp) + per_kg(WALL, 1, &cp);
    // Lighting a kg: warming it to `t_light`, and its liquid to its boiling point and boiling it off (Rothermel 1972).
    let liquid = &chem.subs[p.liquid];
    let dry = c_dry * (ph.t_light - t).max(0.0);
    // (Frozen, it must melt first.)
    let fusion = if t < liquid.melt { liquid.lf } else { 0.0 };
    let per_liquid = (liquid.cp * (liquid.boil - t).max(0.0) + liquid.lv + fusion) / liquid.molar_mass;
    // The most liquid a kg may hold where the share `flux_ahead` of its heat still lights the next kg.
    let extinct = (ph.flux_ahead * heat - dry) / per_liquid;
    // (A bed under the least load of its dry remains is under it at any moisture: its layer need not be read.)
    if load + cured < least(ph, t, heat - dry) {
        return None;
    }
    let l = layer(w, p, i, t)?;
    // The remains on saturated ground are too wet to carry a flame (`inside::wet_ground`).
    let cover = cover * (1.0 - l.saturated);
    // The cured leaves stand in the air near the ground and hold what the curve holds at its vapour.
    let (flaming, top) = (l.share(extinct, 0.0), l.sorbed(l.top));
    let standing = if top < extinct { cured } else { 0.0 };
    let lying = flaming * load;
    let moist = (standing * top + lying * l.sorbed(l.h(flaming / 2.0))) / (standing + lying).max(f64::MIN_POSITIVE);
    let light = dry + moist * per_liquid;
    let smoulder = l.share(ph.smoulder_moist, ph.duff_hold).max(flaming);
    let mut bed = Bed { load, standing, cover, moist, extinct, heat, light, least: least(ph, t, heat - light), flaming, smoulder, particle_rho: chem.subs[p.sub[WALL]].density[0], waf: 0.0 };
    bed.waf = sheltered(w, p, i, bed.depth(ph));
    Some(bed)
}

/// The layer of remains on the drier ground of a cell: the relative vapour of the air in it at its top and at its
/// bottom, the share of what its ground holds for roots that its ground holds, and its sorption curve's scale and power
/// at its temperature; and the share of the cell's land saturated, where no flame runs (`inside::wet_ground`).
struct Layer {
    top: f64,
    bottom: f64,
    wet: f64,
    curve: [f64; 2],
    saturated: f64,
}

/// Steps of halving that find where the liquid of a layer reaches a bound (`Layer::share`): its depth to 2^-40.
const HALVINGS: usize = 40;

impl Layer {
    /// The relative vapour at the share `z` of its depth from its top: a straight line from its top to its bottom.
    fn h(&self, z: f64) -> f64 {
        self.top + (self.bottom - self.top) * z
    }
    /// What its sorption curve holds at relative vapour `h`, per kg dry (Nelson 2000; `h` at most `SORB_HIGHEST`).
    fn sorbed(&self, h: f64) -> f64 {
        self.curve[0] * (-(1.0 - h.min(SORB_HIGHEST)).ln()).powf(self.curve[1])
    }
    /// The share of its load from its top down to where the liquid it holds per kg dry reaches `most`: at the share z of its depth it holds what its curve holds there and, as duff drawing liquid up
    /// from the ground, `hold` × its ground's share × z.
    fn share(&self, most: f64, hold: f64) -> f64 {
        let at = |z: f64| self.sorbed(self.h(z)) + hold * self.wet * z;
        if at(1.0) <= most {
            1.0
        } else if at(0.0) >= most {
            0.0
        } else {
            let (mut lo, mut hi) = (0.0, 1.0);
            for _ in 0..HALVINGS {
                let mid = (lo + hi) / 2.0;
                if at(mid) < most { lo = mid } else { hi = mid }
            }
            lo
        }
    }
}

/// The least load of remains whose flame holds its heat on ground at `t` K, kg a m², where a kg of them burning leaves
/// `net` J once it is lit: a flaming bed at `t_burn` sends σ (T_b⁴ − T⁴) a m² of its top to the open sky for as long
/// as its particles flame (`flame::residence`), and a thinner bed frees less heat than that, so its flame cools and goes
/// out (a flame in fine dead leaves stops spreading under 0.1 to 0.2 kg/m²; Cheney and Sullivan 2008). None burns where
/// a kg frees no more than lighting it takes.
fn least(ph: &crate::Physics, t: f64, net: f64) -> f64 {
    let lost = SIGMA * (ph.t_burn.powi(4) - t.powi(4)).max(0.0) * super::residence(ph);
    if net > 0.0 { lost / net } else { f64::INFINITY }
}

/// Albini and Baughman's (1979) share of the wind at 20 feet that blows at mid-flame (Andrews 2012): over a bed `depth`
/// ft deep in the open `OPEN_K` / ln((`WAF_A` + `WAF_B` H) / (`WAF_C` H)), H its depth; under a canopy H ft tall whose
/// crowns fill the share f of the space under it, `SHELTER_K` / (√(f H) ln((`WAF_A` + `WAF_B` H) / (`WAF_C` H))).
const OPEN_K: f64 = 1.83;
const SHELTER_K: f64 = 0.555;
const WAF_A: f64 = 20.0;
const WAF_B: f64 = 0.36;
const WAF_C: f64 = 0.13;
/// A crown fills a third of the volume of the cylinder it would fill, so the crowns' fill is their cover times their
/// depth's share of the canopy's height over this (Andrews 2012).
const CROWN_FILL: f64 = 3.0;
/// Metres in a foot, for the fits in feet.
const M_PER_FT: f64 = 0.3048;

/// The share of the low wind that reaches the flame of a bed `depth` m deep on cell `i` (`Bed::waf`): Albini and
/// Baughman's (1979) for open ground and for ground under a canopy, by the share of the ground the canopy covers,
/// 1 − e^(−its capture area a m²), of the cover's slots taller than `surface_m`; the canopy's height and its crowns'
/// depth are those of its tallest slot (`Cover::crown`).
fn sheltered(w: &World, p: &Parts, i: usize, depth: f64) -> f64 {
    let (ph, mm, c) = (&w.cfg.phys, p.mm(&w.chem), w.life.cover(i));
    let (ground, rho) = (crate::cover::ground(w, i).max(f64::MIN_POSITIVE), w.chem.subs[p.sub[WALL]].density[0]);
    let log = |h: f64| ((WAF_A + WAF_B * h) / (WAF_C * h)).ln();
    let bed = (depth / M_PER_FT).max(f64::MIN_POSITIVE);
    let open = (OPEN_K / log(bed)).min(1.0);
    let tall = c.held().map(|x| (x, c.height(ph, &mm, rho, x))).filter(|&(_, h)| h > ph.surface_m);
    let (leaf, top) = tall.fold((0.0, None), |(l, t): (f64, Option<(usize, f64)>), (x, h)| (l + c.leaf(&mm, x), if t.is_none_or(|t| h > t.1) { Some((x, h)) } else { t }));
    let Some((x, h)) = top else { return open };
    let cover = 1.0 - (-leaf / ground).exp();
    let (base, h) = c.crown(ph, &mm, ground, x, h);
    let fill = cover * (h - base) / h / CROWN_FILL;
    let feet = h / M_PER_FT;
    let under = (SHELTER_K / ((fill * feet).sqrt().max(f64::MIN_POSITIVE) * log(feet))).min(open);
    (1.0 - cover) * open + cover * under
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

/// The layer of the remains on cell `i` at `t` K (`Layer`). The air at its top is the air near the ground
/// (`World::near_ground`); at its bottom it lies between that and saturation by the share of what its ground holds for
/// roots that its ground holds, its frozen liquid with it (`cover::soak`, `Pores::available`: the share by which the
/// ground's evaporation draws its liquid; Manabe 1969), on the drier ground: its liquid less what its saturated share
/// holds (`inside::wet_ground`). Vapour diffuses through it, so in a steady state its relative
/// vapour runs in a straight line from its top to its bottom. None where snow lies over the remains
/// (`contract::Place::snow`), or before the air near the ground is read.
fn layer(w: &World, p: &Parts, i: usize, t: f64) -> Option<Layer> {
    let ph = &w.cfg.phys;
    let near = (w.near_ground(i, p.liquid)? / w.chem.p_sat(p.liquid, t)).clamp(0.0, 1.0);
    let (held, plateau) = (w.g(p.liquid, i).max(0.0), w.thaw_last(i).1);
    let (soak, ground) = crate::inside::wet_land(w, p, i);
    // Snow lying hides the remains from a flame (`contract::place`); the key liquid frozen in the ground's pores fills
    // them as the liquid does.
    if crate::contract::place(w, i).snow > 0.0 {
        return None;
    }
    let pores = w.scratch.land_pores(i).max(1e-9);
    let frozen = held * (1.0 - w.molten(p.liquid, i, t, plateau)) * w.chem.molar_volume(p.liquid, Phase::Solid) / w.mesh.cells().area()[i];
    let wet = soak.pores.available(ground.filled + frozen / pores);
    let at = |c: [f64; 2]| c[0] + c[1] * (t - SORB_T);
    let bottom = near + wet * (1.0 - near);
    Some(Layer { top: near, bottom, wet, curve: [at(ph.sorb_scale), at(ph.sorb_power)], saturated: ground.share })
}
