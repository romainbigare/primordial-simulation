//! Flow in and through the air (laws.md 1.4 to 1.6): each air step each cell's exchange with the air it sees is found
//! as one law, linear in that air (`cells::respond`), the air level runs day by day under it (`daily`), and each cell's
//! exchange at the step's mean air is booked whole (`book`). Here: the state of the air nodes.

mod book;
mod cells;
mod corners;
mod daily;
mod edge;
mod exchange;
mod radiation;
mod planet;
mod outs;
mod rain;
mod seen;
mod soak;
mod sky;
mod step;
mod stretch;
mod surf;
mod together;
mod wind;

pub(super) use corners::Corners;
pub use planet::Planet;
pub(super) use daily::blocks;
pub(super) use exchange::{dissolve, henry, Sat};
pub use radiation::{band_depths, column, declination, insolation, on_slope, sun_by_latitude, Heat, DOWN, UP};
pub(super) use radiation::Skies;
pub(super) use step::step;

use super::Scratch;
use crate::{store::Class, units::R_GAS, World, AIR};
use std::f64::consts::LN_2;

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

/// The most air nodes a ground cell reads: the 3 x 3 round its own, for a cell as wide as an air block (`Scratch::near`).
pub(super) const SEES: usize = 9;

/// The air seen by ground cell `i`: a node field interpolated from the nodes it reads.
#[inline]
pub(super) fn lerp(s: &Scratch, v: &[f64], i: usize) -> f64 {
    s.near[i].iter().zip(&s.wt[i]).map(|(&j, &w)| w * v[j as usize]).sum()
}

/// The share of the air ground cell `i` sees that lies over it: its area over the area of the air it sees (`lerp`).
#[inline]
pub(super) fn over_cell(w: &World, s: &Scratch, i: usize) -> f64 {
    w.mesh.cells().area()[i] / s.seen[i]
}

impl World {
    /// How fast the air ground cell `i` sees cools with height, K/m (`lerp` of the nodes' drop with height), as the last
    /// air step found it: what a shell takes for the ground's temperature along the relief finer than the cell. None
    /// before the air has stepped on these cells.
    pub fn air_drop(&self, i: usize) -> Option<f64> {
        let s = &self.scratch;
        (s.near.len() == self.mesh.cells().n() && s.gam.len() == self.mesh.level(AIR).n()).then(|| lerp(s, &s.gam, i))
    }
}

/// Pa per mol of a gas over a ground of `area` m² that sees the share `over` of its air, at `tl` K near the ground, where
/// the gas's amount fills `hk` m near the ground (`Scratch::hk`): the ideal gas in that layer, as the ground's
/// evaporation reads its partial pressure (`surf::Vapour`).
#[inline]
pub(super) fn per_mol(over: f64, tl: f64, area: f64, hk: f64) -> f64 {
    over * R_GAS * tl / (area * hk)
}

impl World {
    /// Whether the map's edges are open on the planet (laws.md 1.6): it holds the planet's air (`World::planet`) and
    /// stands for a band of latitudes wider than itself (`Stretch::compressed`). Its air crosses them (`edge`), and so
    /// does its open low ground's liquid (`flow::runoff`).
    pub fn edges_open(&self) -> bool {
        self.planet.is_some() && stretch::Stretch::of(self).compressed()
    }
    /// The partial pressure of gas `k` in the air near the ground of cell `i`, Pa, as the ground's evaporation reads it
    /// (`surf::Vapour`, `per_mol`): the amount the air it sees holds (`lerp`) at that air's temperature at the ground's
    /// head (`at_height`). None before the air has stepped, or where `k` is not a gas of the air.
    pub fn near_ground(&self, i: usize, k: usize) -> Option<f64> {
        let (s, na) = (&self.scratch, self.mesh.level(AIR).n());
        let gi = s.state.gas.iter().position(|&g| g == k)?;
        if s.hk.len() < (gi + 1) * na || s.near.len() != self.mesh.cells().n() || s.head.len() != self.mesh.cells().n() {
            return None;
        }
        let x = lerp(s, &self.store.amounts(Class::Air)[k * na..(k + 1) * na], i);
        let tl = at_height(lerp(s, &s.ta, i), lerp(s, &s.gam, i), s.head[i], lerp(s, &s.zair, i));
        let j = self.mesh.cells().parent()[i] as usize;
        Some(x * per_mol(over_cell(self, s, i), tl, self.mesh.cells().area()[i], s.hk[gi * na + j]))
    }
}

/// Temperature of air at height `z`, m, that is at `t` K at its column's mean height `zair`, m, and cools by `gam` K/m.
#[inline]
pub(super) fn at_height(t: f64, gam: f64, z: f64, zair: f64) -> f64 {
    (t - gam * (z - zair)).max(1.0)
}

/// Temperature of the air touching ground cell `i` under air nodes at `t`, K: the air it sees (`lerp`) at its head, colder
/// by `Scratch::cool` (`at_height`).
#[inline]
pub(super) fn touching(s: &Scratch, t: &[f64], i: usize) -> f64 {
    (lerp(s, t, i) - s.cool[i]).max(1.0)
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

/// The state of every air node, how high each gas fills its column (a gas that would condense higher up is held near
/// the ground), and the layers that send its heat radiation.
pub(super) fn state(w: &World, s: &mut Scratch) {
    nodes(w, s);
    heights(w, s);
    layers(w, s);
    let cells = w.mesh.cells().n();
    s.cool = (0..cells).map(|i| lerp(s, &s.gam, i) * (s.head[i] - lerp(s, &s.zair, i))).collect();
}

/// Amount, mol, heat capacity, J/K, and mass, kg, of the gases of each air node, from its amounts now and the gas the
/// aside nodes under it hold: bodies give the air the warmth of their gas at once (laws.md 1.4), so its temperature
/// counts that gas too.
pub(super) fn sums(w: &World, s: &mut Scratch) {
    let (na, n, parent) = (w.mesh.level(AIR).n(), w.mesh.cells().n(), w.mesh.cells().parent());
    (s.nt, s.capa, s.mass) = (vec![0.0; na], vec![0.0; na], vec![0.0; na]);
    for &k in &s.state.gas {
        let sp = s.sp[k];
        // The aside nodes hold gas only between the air's steps (`book::up_from_bodies`): skip the array when it is empty.
        let held = &w.store.amounts(Class::Aside)[k * n..(k + 1) * n];
        let held = if held.iter().any(|&a| a != 0.0) { held } else { &[] };
        let bodies = held.iter().zip(parent).map(|(&a, &j)| (j as usize, a));
        for (j, a) in w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().copied().enumerate().chain(bodies) {
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
    for (i, &j) in cells.parent().iter().enumerate() {
        s.za[j as usize] += s.head[i] * cells.area()[i];
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

/// What each air node's air does to heat radiation (`Skies`): its gases that absorb it, each with the height its
/// amount fills (`Scratch::hk`, `radiation::band_depths`); and under the share of its sky the cloud of the step
/// before covers, that cloud too (`radiation::drops_heat`), spread like the air.
fn layers(w: &World, s: &mut Scratch) {
    let (air, chem, g, top) = (w.mesh.level(AIR), &w.chem, w.cfg.gravity, w.cfg.air_top_m);
    let na = air.n();
    s.heat = (0..na)
        .map(|j| {
            let gases: Vec<(usize, f64, f64)> = s.state.gas.iter().enumerate().filter(|e| chem.absorbs_heat(*e.1)).map(|(gi, &k)| (k, w.store.amounts(Class::Air)[k * na + j] / air.area()[j], s.hk[gi * na + j].max(COLUMN_MIN))).collect();
            let (big, [share, depth]) = (node_scale_height(s, j, g), s.state.cloud[j]);
            let bands = band_depths(chem, &gases, (s.ta[j], big));
            let sky = |cloud: f64| Heat::of(bands, (cloud, big), (s.gam[j], (top - s.za[j]).max(COLUMN_MIN)), w.cfg.line_spread);
            Skies::of(sky(0.0), (share > 0.0).then(|| (share, sky(radiation::drops_heat(depth)))))
        })
        .collect();
}
