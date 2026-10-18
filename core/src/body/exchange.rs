//! Exchange of a body with its surroundings: its surfaces, the heat network to the ground it stands on, the boundary
//! layer of its surface, and the gas and vapour through its pores.

use super::Body;
use crate::bond::{self, Enclosure};
use crate::chem::Chemistry;
use crate::math::cube_area;
use crate::units::{DAY, SIGMA};
use crate::World;

impl Body {
    /// Heat conductance of its coat, W/K (the `HEAT` gene): through each member's surface, which grows as mass^(2/3), so a
    /// small body cools faster per kg than a big one. Needs `prepare`.
    pub fn conductance(&self, m_ref: f64, mass: f64) -> f64 {
        let n = self.n();
        self.k.heat * n * m_ref.cbrt() * (mass / n).max(0.0).powf(2.0 / 3.0)
    }
    /// Surface the body exchanges gas with the air through, m²: its light-capture area and its skin. Needs `prepare`.
    pub fn open_area(&self, chem: &Chemistry, p: &crate::Physics, mass: f64) -> f64 {
        self.leaf_area(chem, p) + self.skin_area(p, mass)
    }
    /// Its light-capture area, all members', m²: `LEAF` per kg-equivalent of its body (`equiv` of `built`). Needs `prepare`.
    pub(crate) fn leaf_area(&self, chem: &Chemistry, p: &crate::Physics) -> f64 {
        self.leaf_area_at(chem, p, self.k.leaf)
    }
    /// `leaf_area` at `leaf` m² per kg-equivalent (the `LEAF` gene's value, `dec`).
    pub(super) fn leaf_area_at(&self, chem: &Chemistry, p: &crate::Physics, leaf: f64) -> f64 {
        self.equiv(p, self.built(chem)) * leaf
    }
    /// Its skin at a whole mass of `mass` kg, all members', m²: each member a cube of its volume at `rho_body`.
    pub(crate) fn skin_area(&self, p: &crate::Physics, mass: f64) -> f64 {
        let m1 = mass / self.n();
        cube_area(self.n(), m1 / p.rho_body)
    }
}

/// Halvings of the interval in `through`.
const BISECT_STEPS: usize = 40;

/// The cover of other bodies over a body's cell, kg/m²: the mass of the others there now per ground area. (A body in
/// its update is out of its slot; one in its slot is left out by its place.)
pub(super) fn cover_of(w: &World, b: &Body) -> f64 {
    let i = b.cell as usize;
    let others = w.life.at[i].iter().map(|&s| &w.life.bodies[s as usize]).filter(|o| o.alive && !std::ptr::eq(*o, b));
    others.map(|o| o.vis[0] as f64 * o.n()).sum::<f64>() / w.mesh.cells().area()[i]
}

/// A body's heat conductance to the ground it stands on, at its temperature `t` and the ground's `tg`, W/K: one network
/// for every body. Heat crosses its coat (`Body::conductance`) to its outer surface, at Ts; from there it leaves in
/// parallel to the still air (`film`) and by radiation (`radiated`, from Ts: taken at the body's temperature first, then
/// once at Ts). The outer surface is its skin and both faces of its light-capture area. A cover of `cover` kg/m² passes
/// `phys.shelter` / `cover` W per K and m² of skin in series with the skin's still air, and does not block radiation
/// (it is about as warm as the ground). The link through walls around it (`walls`, `bond::enclosure`) is in series with
/// all of it. A 30 kg member with a coat gene of -1.0 gets 0.8 W/K (Herreid and Kessel 1967 measured 0.94 for fur).
#[allow(clippy::too_many_arguments)]
pub(super) fn surface(cfg: &crate::Config, b: &Body, mass: f64, f: &Film, walls: Option<Enclosure>, t: f64, tg: f64) -> f64 {
    let coat = b.conductance(cfg.phys.m_ref, mass);
    let out = f.g + radiated(f.area, t, tg);
    let out = f.g + radiated(f.area, (coat * t + out * tg) / (coat + out), tg);
    let g = coat * out / (coat + out);
    walls.map_or(g, |x| 1.0 / (1.0 / g + 1.0 / x.g))
}

/// The boundary layer of a body's outer surface (`Film`, `surface`): its skin (through the cover) and both faces of its
/// light-capture area. Forced convection in the wind near the ground (Campbell and Norman 1998):
/// `forced` x sqrt(`wind_ground` / d) mol/(m² s) x the air's molar heat capacity, d the length of a member, (m1 /
/// `rho_body`)^(1/3), or of a leaf (`leaf_size`). Free convection is smaller at this wind.
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
    /// Of the body: its light-capture area and its skin, m² (`Body::open_area` is their sum)
    pub leaf: f64,
    pub skin: f64,
}

/// The air a body of boundary layer `f` (`film`) exchanges gas with in `days`, mol: `Body::open_area` times its pores (the
/// `GAS` gene, mol/(m² day), a stomatal conductance) in series with that boundary layer (over the air's molar heat
/// capacity: the Lewis relation); liquid over it (`depth`) slows it. Its gases and the vapour of its liquids go through
/// it alike; for the vapour the pores open only as far as it is wet (`closing`).
pub(super) fn breath(chem: &Chemistry, cfg: &crate::Config, b: &Body, f: &Film, days: f64, depth: f64) -> f64 {
    let gs = b.k.gas;
    let gb = f.g / chem.air_cp() / f.area.max(1e-12) * DAY;
    (f.leaf + f.skin) * days * (-depth / cfg.phys.gas_depth).exp() * if gs > 0.0 { gs * gb / (gs + gb) } else { 0.0 }
}

/// What a key liquid of `l` mol loses over an update in which it would lose `e` mol with its pores open, when they open
/// as far as it is wet (`Body::activity`: min(1, l / `ls`)^n, `ls` the level it keeps): at the full rate down to its
/// level, then du/dt = -r u^n (u = l / `ls`), solved exactly: u^(1 - n) grows by (n - 1) times what is left of `e` over
/// `ls` (`n` above 1, `Physics::validate`).
fn closing(l: f64, ls: f64, e: f64, n: f64) -> f64 {
    if ls <= 0.0 || e <= (l - ls).max(0.0) || l <= 0.0 {
        return e.min(l.max(0.0));
    }
    let (full, u0) = ((l - ls).max(0.0), (l / ls).min(1.0));
    let u = (u0.powf(1.0 - n) + (n - 1.0) * (e - full) / ls).powf(-1.0 / (n - 1.0));
    l - ls * u
}

/// What a key liquid loses through the pores over an update when it also soaks: the pores pass `e` mol in the update
/// open, and open as its liquid goes (`closing`), while it soaks at `rd` (its rate per day times the days) of how far it
/// is below its level `ls`. Its time constant is days, so it stands where the two meet, u = l / `ls`, found by halving:
/// `rd` (1 - u) `ls` = `e` u^n; it loses `e` u^n, what flows through it, or what it loses drying out (`closing`) if more.
pub(super) fn through(l: f64, ls: f64, e: f64, rd: f64, n: f64) -> f64 {
    let dry = closing(l, ls, e, n);
    if rd <= 0.0 || ls <= 0.0 || e <= 0.0 {
        return dry;
    }
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..BISECT_STEPS {
        let u = 0.5 * (lo + hi);
        if rd * (1.0 - u) * ls > e * u.powf(n) { lo = u } else { hi = u }
    }
    dry.max(e * (0.5 * (lo + hi)).powf(n))
}

/// The heat conductance by which a body of `mass` kg at the temperature `t` loses heat to the ground it stands on, W/K
/// (`surface`, with its cover and walls), the ground at its temperature of the last flow step (`World::t_ground_last`).
pub fn conductance_to_ground(w: &World, b: &Body, mass: f64, t: f64) -> f64 {
    let i = b.cell as usize;
    let tg = w.t_ground_last(i);
    surface(&w.cfg, b, mass, &film(&w.chem, &w.cfg, b, mass, cover_of(w, b)), bond::enclosure(w, b), t, tg)
}

/// Heat conductance of radiation between a surface of `area` m² at `t` and the ground at `tg`, W/K: σ (T² + tg²)
/// (T + tg) per m², the linearised σT⁴ (emissivity 1, as `flow::air::radiation`).
fn radiated(area: f64, t: f64, tg: f64) -> f64 {
    SIGMA * area * (t * t + tg * tg) * (t + tg)
}
