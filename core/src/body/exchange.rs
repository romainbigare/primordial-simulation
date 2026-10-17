//! Exchange of a body with its surroundings: heat to the ground, the boundary layer, gas through its pores.

use super::Body;
use crate::chem::Chemistry;
use crate::math::cube_area;
use crate::units::{DAY, R_GAS, SIGMA};
use crate::{bond, World};

impl Body {
    /// Heat conductance to the surface, W/K: through each member's surface, which grows as
    /// mass^(2/3), so a small body cools faster per kg than a big one. Needs `prepare`.
    pub fn conductance(&self, m_ref: f64, mass: f64) -> f64 {
        let n = self.n();
        self.k.heat * n * m_ref.cbrt() * (mass / n).max(0.0).powf(2.0 / 3.0)
    }
    /// Surface the body exchanges gas with the air through, m²: per member its light-capture area
    /// (`LEAF` per kg-equivalent of its body, as `visible` and the light code count it)
    /// plus its skin, 6 (m1 / `rho_body`)^(2/3), as the surface of walls and the cover; times the
    /// count. A 1 kg plant of 2 m² per kg: 2.06 m²; a 30 kg eater with 0.03 m² of capture area per
    /// kg-equivalent (12.8): 0.38 + 0.58 m². Needs `prepare`. The `GAS` gene is the air exchanged
    /// per m² of it per day.
    pub fn open_area(&self, chem: &Chemistry, p: &crate::Physics, mass: f64) -> f64 {
        self.leaf_area(chem, p) + self.skin_area(p, mass)
    }
    /// Its light-capture area, all members', m²: `LEAF` per kg-equivalent of its body (`equiv` of `built`). Needs `prepare`.
    pub(crate) fn leaf_area(&self, chem: &Chemistry, p: &crate::Physics) -> f64 {
        self.equiv(p, self.built(chem)) * self.k.leaf
    }
    /// Its skin at a whole mass of `mass` kg, all members', m²: each member a cube of its volume at `rho_body`.
    pub(crate) fn skin_area(&self, p: &crate::Physics, mass: f64) -> f64 {
        let m1 = mass / self.n();
        cube_area(self.n(), m1 / p.rho_body)
    }
}

/// Molar heat capacity of an ideal gas of two atoms, J/(mol K): the air's when none is known.
const DIATOMIC_CP: f64 = 3.5 * R_GAS;
/// Halvings of the interval in `through` (2^-40 of the level).
const BISECT_STEPS: usize = 40;

/// The cover of other bodies over a body's cell, kg/m²: the mass of the others per ground area.
pub(super) fn cover_of(w: &World, b: &Body) -> f64 {
    (w.life.sum[b.cell as usize][1] - b.vis[0] as f64 * b.n()).max(0.0) / w.grid.area()
}

/// A body's heat conductance to the ground it stands on at its temperature `t`, the ground at `tg`, W/K: one network for
/// every body (16 Oct 2026). Heat crosses its coat (`Body::conductance`: the `HEAT` gene, mass^(2/3)) to its outer surface,
/// at Ts; from there it leaves in parallel to the still air (`sensible` per m² and K, the ground's number) and by radiation
/// from Ts, σ (Ts² + tg²)(Ts + tg) per m² (emissivity 1). The outer surface is its skin, 6 (m1 / `rho_body`)^(2/3) per member
/// (as the space of walls), and both faces of its light-capture area. A layer of `cover` kg/m² passes `phys.shelter` /
/// `cover` W per K and m² of skin in series with the skin's still air (the leaf is above it); it does not block radiation
/// (it is matter about as warm as the ground). Ts is where the coat and the outer surface meet (radiation taken at the
/// body's temperature first, then once at Ts). Walls around it are in series with all of it (their share of their
/// conductance). A bare 30 kg member (skin 0.58 m²) gets at most the still air and radiation of its skin, 9 W/K; fur
/// (gene -1.0) gives 0.8 W/K (Herreid and Kessel 1967: 0.94). A 2 kg plant member of 3.4 m² of capture area gets at most
/// about 100 W/K. A burning node of a high gene radiates from about its own temperature, 1700 W/K per m² at 3000 K.
#[allow(clippy::too_many_arguments)]
pub(super) fn surface(chem: &Chemistry, cfg: &crate::Config, b: &Body, mass: f64, cover: f64, walls: Option<(f64, f64)>, t: f64, tg: f64) -> f64 {
    let ((air, area), coat) = (film(chem, cfg, b, mass, cover), b.conductance(cfg.phys.m_ref, mass));
    let out = air + radiated(area, t, tg);
    let out = air + radiated(area, (coat * t + out * tg) / (coat + out), tg);
    let g = coat * out / (coat + out);
    walls.map_or(g, |x| 1.0 / (1.0 / g + 1.0 / x.1))
}

/// The boundary layer of a body's outer surface, W/K, and that surface, m² (`surface`): its skin (through the cover) and
/// both faces of its light-capture area. Forced convection in the wind near the ground (16 Oct 2026, Campbell and Norman
/// 1998): `forced` x sqrt(`wind_ground` / d) mol/(m² s) x the air's molar heat capacity, d the body's length, (m1 /
/// `rho_body`)^(1/3), or a leaf's (`leaf_size`): 12.8 W/(m² K) for a 30 kg member, 33 for 0.1 kg, 32 for a leaf. (Free
/// convection, a few W/(m² K) at a few K, is under it at this wind.)
pub(super) fn film(chem: &Chemistry, cfg: &crate::Config, b: &Body, mass: f64, cover: f64) -> (f64, f64) {
    let p = &cfg.phys;
    let (leaf, m1, skin) = (b.leaf_area(chem, p), mass / b.n(), b.skin_area(p, mass));
    let h = |d: f64| p.forced * air_cp(chem) * (p.wind_ground / d).sqrt();
    let still = h((m1 / p.rho_body).cbrt()) * skin;
    (if cover > 0.0 { 1.0 / (1.0 / still + cover / (p.shelter * skin)) } else { still } + h(p.leaf_size) * 2.0 * leaf, skin + 2.0 * leaf)
}

/// Molar heat capacity of the starting air, J/(mol K) (of an ideal gas of two atoms, 3.5 R, if none is known).
fn air_cp(chem: &Chemistry) -> f64 {
    let c: f64 = chem.air.iter().zip(&chem.subs).map(|(x, s)| x * s.cp).sum();
    if c > 0.0 { c } else { DIATOMIC_CP }
}

/// The air a body exchanges gas with in `days`, mol (`Body::open_area` times its conductance; 16 Oct 2026): its pores (the
/// `GAS` gene, mol/(m² day), a stomatal conductance) in series with the boundary layer of its surface (`film`, over the air's
/// molar heat capacity: the Lewis relation); liquid over it slows it. Its gases and the vapour of its liquids go through it
/// alike; for the vapour the pores open only as far as it is wet (`live`).
#[allow(clippy::too_many_arguments)]
pub(super) fn breath(chem: &Chemistry, cfg: &crate::Config, b: &Body, mass: f64, cover: f64, days: f64, depth: f64) -> f64 {
    let ((air, area), gs) = (film(chem, cfg, b, mass, cover), b.k.gas);
    let gb = air / air_cp(chem) / area.max(1e-12) * DAY;
    b.open_area(chem, &cfg.phys, mass) * days * (-depth / cfg.phys.gas_depth).exp() * if gs > 0.0 { gs * gb / (gs + gb) } else { 0.0 }
}

/// What a key liquid of `l` mol loses over an update in which it would lose `e` mol with its pores open, when they open as far
/// as it is wet (`Body::activity`: min(1, l / `ls`)^n, `ls` the level it keeps): at the full rate down to its level, then
/// du/dt = -r u^n (u = l / `ls`), solved exactly: u^(1 - n) grows by (n - 1) times what is left of `e` over `ls`.
fn closing(l: f64, ls: f64, e: f64, n: f64) -> f64 {
    if ls <= 0.0 || e <= (l - ls).max(0.0) || l <= 0.0 {
        return e.min(l.max(0.0));
    }
    let (full, u0) = ((l - ls).max(0.0), (l / ls).min(1.0));
    let u = (u0.powf(1.0 - n) + (n - 1.0) * (e - full) / ls).powf(-1.0 / (n - 1.0));
    l - ls * u
}

/// What a key liquid loses through the pores over an update when it also soaks (16 Oct 2026): the pores pass `e` mol in
/// the update open, and open as its liquid goes (`closing`), while it soaks at `rd` (its rate per day times the days) of
/// how far it is below its level `ls`. Its time constant is days, so it stands where the two meet, u = l / `ls`:
/// `rd` (1 - u) `ls` = `e` u^n, and what it loses is `e` u^n, what flows through it; or, if it soaks little, what it
/// loses drying out (`closing`), whichever is more.
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

/// The heat conductance by which a body of `mass` kg at the temperature `t` loses heat to the ground it stands on,
/// W/K (`surface`: its `HEAT` gene, cover and walls, and radiation from its outer surface). The ground's
/// temperature is that of the last flow step, as in its update.
pub fn conductance_to_ground(w: &World, b: &Body, mass: f64, t: f64) -> f64 {
    let i = b.cell as usize;
    let tg = ground_t(w, i);
    surface(&w.chem, &w.cfg, b, mass, cover_of(w, b), bond::walls_around(w, b), t, tg)
}

/// Heat conductance of radiation between a surface of `area` m² at `t` and the ground at `tg`, W/K: σ (T² + tg²)
/// (T + tg) per m², as every surface radiates (σT⁴, emissivity 1, `flow::air::radiation`).
fn radiated(area: f64, t: f64, tg: f64) -> f64 {
    SIGMA * area * (t * t + tg * tg) * (t + tg)
}

/// The temperature of ground cell `i` as of the last flow step, K (the monthly survey's before the first).
pub(super) fn ground_t(w: &World, i: usize) -> f64 {
    w.t_ground_last(i).unwrap_or(w.life.env[i][0])
}
