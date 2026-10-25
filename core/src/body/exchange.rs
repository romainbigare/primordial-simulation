//! Exchange of a body with its surroundings: its surfaces, the heat network to the ground it stands on, the boundary
//! layer of its surface, and the gas and vapour through its pores.

use super::Body;
use crate::bond::{self, Enclosure};
use crate::chem::Chemistry;
use crate::math::cube_area;
use crate::units::{DAY, SIGMA};
use crate::{life::Domain, World};

impl Body {
    /// Heat conductance of its coat, W/K (the `HEAT` gene), through each member's surface, as mass^(2/3).
    pub fn conductance(&self, m_ref: f64, mass: f64) -> f64 {
        let n = self.n();
        self.k.heat * n * (m_ref * (mass / n).max(0.0).powi(2)).cbrt()
    }
    /// Its light-capture area, all members', m²: `LEAF` per kg-equivalent of its body (`equiv` of `built`).
    pub fn leaf_area(&self, chem: &Chemistry, p: &crate::Physics) -> f64 {
        self.equiv(p, self.built(chem)) * self.k.leaf
    }
    /// Its skin at a whole mass of `mass` kg, all members', m²: each member a cube of its volume at `rho_body`.
    pub fn skin_area(&self, p: &crate::Physics, mass: f64) -> f64 {
        let m1 = mass / self.n();
        cube_area(self.n(), m1 / p.rho_body)
    }
}

/// Most values of Newton's method in `through` (it stops as soon as a step no longer falls, in a few).
const NEWTON_STEPS: usize = 40;

/// The cover of other bodies over a body's cell, kg/m²: the mass of the others there now per ground area.
pub fn cover_of(w: &World, d: &Domain, b: &Body) -> f64 {
    let i = b.cell as usize;
    let others = d.cell(i).map(|s| &d[s]).filter(|o| o.alive && !std::ptr::eq(*o, b));
    others.map(|o| o.vis[0] as f64 * o.n()).sum::<f64>() / w.mesh.cells().area()[i]
}

/// A body's heat conductance to the ground, W/K, one network for every body (laws.md 6.3): its coat to its outer
/// surface at Ts, then in parallel the still air (`film`) and radiation (from Ts, found once). A cover passes `shelter` /
/// `cover` per m² of skin in series with the skin's still air; walls are in series with it all. A 30 kg member with a
/// coat gene of -1.0 gets 0.8 W/K (Herreid and Kessel 1967 measured 0.94 for fur).
pub(super) fn surface(cfg: &crate::Config, b: &Body, mass: f64, f: &Film, walls: Option<Enclosure>, t: f64, tg: f64) -> f64 {
    let coat = b.conductance(cfg.phys.m_ref, mass);
    let out = f.g + radiated(f.area, t, tg);
    let out = f.g + radiated(f.area, (coat * t + out * tg) / (coat + out), tg);
    let g = coat * out / (coat + out);
    walls.map_or(g, |x| 1.0 / (1.0 / g + 1.0 / x.g))
}

/// The boundary layer of a body's outer surface (skin and both faces of its leaf): forced convection, `forced` ×
/// sqrt(`wind_ground` / d) × the air's molar heat capacity, d a member's length or `leaf_size` (Campbell and Norman
/// 1998); free convection is smaller at this wind.
pub(super) fn film(chem: &Chemistry, cfg: &crate::Config, b: &Body, mass: f64, cover: f64) -> Film {
    let p = &cfg.phys;
    let (leaf, m1, skin) = (b.leaf_area(chem, p), mass / b.n(), b.skin_area(p, mass));
    let h = |d: f64| p.forced * chem.air_cp() * (p.wind_ground / d).sqrt();
    let still = h((m1 / p.rho_body).cbrt()) * skin;
    let g = if cover > 0.0 { 1.0 / (1.0 / still + cover / (p.shelter * skin)) } else { still } + h(p.leaf_size) * 2.0 * leaf;
    Film { g, area: skin + 2.0 * leaf, leaf, skin }
}

/// The boundary layer of a body's outer surface (`film`).
#[derive(Clone, Copy)]
pub(super) struct Film {
    /// Its heat conductance, W/K, and the surface, m²
    pub g: f64,
    pub area: f64,
    /// Of the body: its light-capture area and its skin, m² (their sum is its open surface)
    pub leaf: f64,
    pub skin: f64,
}

/// The air a body exchanges gas with in `days`, mol: its open surface (light-capture area and skin) times its pores (`GAS`, a stomatal conductance) in
/// series with its boundary layer (the Lewis relation), slowed by liquid over it (`depth`).
pub(super) fn breath(chem: &Chemistry, cfg: &crate::Config, b: &Body, f: &Film, days: f64, depth: f64) -> f64 {
    let gs = b.k.gas;
    let gb = f.g / chem.air_cp() / f.area.max(1e-12) * DAY;
    (f.leaf + f.skin) * days * (-depth / cfg.phys.gas_depth).exp() * if gs > 0.0 { gs * gb / (gs + gb) } else { 0.0 }
}

/// What a key liquid of `l` mol loses in an update where its open pores would pass `e` mol, as they open only as far as
/// it is wet (min(1, l / `ls`)^n): the full rate down to `ls`, then du/dt = -r u^n solved exactly (n above 1).
fn closing(l: f64, ls: f64, e: f64, n: f64) -> f64 {
    if ls <= 0.0 || e <= (l - ls).max(0.0) || l <= 0.0 {
        return e.min(l.max(0.0));
    }
    let (full, u0) = ((l - ls).max(0.0), (l / ls).min(1.0));
    let u = (u0.powf(1.0 - n) + (n - 1.0) * (e - full) / ls).powf(-1.0 / (n - 1.0));
    l - ls * u
}

/// What a key liquid loses through the pores in an update while it soaks at `rd` of its gap below `ls`: it settles in
/// days where the two meet, `rd` (1 - u) `ls` = `e` u^n, and loses `e` u^n, or `closing` if more. The meeting is found
/// by Newton's method on e u^n + a u - a (a = `rd` `ls`), which rises and curves up on 0..1: from the smaller of 1 and
/// where its first term alone reaches a, above the meeting, each step falls toward it.
pub(super) fn through(l: f64, ls: f64, e: f64, rd: f64, n: f64) -> f64 {
    let dry = closing(l, ls, e, n);
    if rd <= 0.0 || ls <= 0.0 || e <= 0.0 {
        return dry;
    }
    let (a, u0) = (rd * ls, (rd * ls / e).powf(1.0 / n).min(1.0));
    let step = |&u: &f64| Some(u - (e * u.powf(n) + a * u - a) / (n * e * u.powf(n) / u + a)).filter(|&v| v < u);
    let u = std::iter::successors(Some(u0), step).take(NEWTON_STEPS).last().unwrap_or(u0);
    dry.max(e * u.powf(n))
}

/// The heat conductance of a body of `mass` kg at `t` to the ground (`surface`, with its cover and walls), W/K.
pub fn conductance_to_ground(w: &World, d: &Domain, b: &Body, mass: f64, t: f64) -> f64 {
    let i = b.cell as usize;
    let tg = w.t_ground_last(i);
    surface(&w.cfg, b, mass, &film(&w.chem, &w.cfg, b, mass, cover_of(w, d, b)), bond::enclosure(w, d, b), t, tg)
}

/// Heat conductance of radiation between a surface of `area` m² at `t` and the ground at `tg`, W/K: σ (T² + tg²)
/// (T + tg) per m², the linearised σT⁴ (emissivity 1, as `flow::air::radiation`).
fn radiated(area: f64, t: f64, tg: f64) -> f64 {
    SIGMA * area * (t * t + tg * tg) * (t + tg)
}
