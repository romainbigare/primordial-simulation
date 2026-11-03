//! Heat of ground and air nodes: heat capacities, temperatures, melting plateaus, and the heat above `T_FLOOR`.

use crate::{chem::{self, Chemistry}, params::Config, store::{Class, Node}, units::T_FLOOR, world::World};

/// The share of a wanted amount that was really given (exactly 1 when all of it was).
#[inline]
pub(crate) fn share(took: f64, want: f64) -> f64 {
    if took >= want { 1.0 } else { took / want }
}

/// Heat capacity of the skin of a ground node's bed, J/K: `skin_m` of its bonded amounts at their mean heat capacity
/// per volume.
pub(crate) fn skin_cap(chem: &Chemistry, cfg: &Config, area: f64, ks: impl IntoIterator<Item = usize>, b: impl Fn(usize) -> f64) -> f64 {
    let (mut c, mut v) = (0.0, 0.0);
    for k in ks {
        c += b(k) * chem.subs[k].cp;
        v += b(k) * chem.molar_volume(k, chem::Phase::Solid);
    }
    skin_of(cfg, area, c, v)
}

/// Heat capacity of the skin of a bed of `area` m² whose bonded amounts hold `c` J/K in `v` m³ (`skin_cap`), J/K.
#[inline]
pub(crate) fn skin_of(cfg: &Config, area: f64, c: f64, v: f64) -> f64 {
    if v > 0.0 { c / v * cfg.skin_m * area } else { 0.0 }
}

/// Where a ground node or a body of heat capacity `c` (J/K) and energy `e` (J) stands among its melting plateaus.
pub struct Melting {
    /// Temperature, K
    pub t: f64,
    /// The substance held at its melting point, with its molten share
    pub on: Option<(usize, f64)>,
    /// Fusion enthalpy held by what is molten (not counting `on`), J
    pub held: f64,
    /// The energies between which the node stays off a plateau with this `held`, J (`f64::MIN` for `hi` on a plateau),
    /// and the fusion enthalpy of the plateaus there, J
    pub lo: f64,
    pub hi: f64,
    pub lat: [f64; 2],
    /// What is at or over its melting point, a bit per substance
    pub bits: u64,
}

impl Melting {
    /// The molten share of substance `k` (of those given to `melting`).
    pub fn molten(&self, k: usize) -> f64 {
        match self.on {
            Some((j, f)) if j == k => f,
            _ => (self.bits >> k & 1) as f64,
        }
    }
}

/// Temperature of a node or body of heat capacity `c` and energy `e` whose molten amounts hold their fusion enthalpy:
/// `melts` gives, in melting order, each substance's point there and its fusion enthalpy, J. It stays at a melting point
/// until that substance has all frozen or melted (laws.md 1.1).
pub(crate) fn melting(c: f64, e: f64, melts: impl IntoIterator<Item = (f64, usize, f64)>) -> Melting {
    let (mut held, mut lo, mut hi, mut bits, mut lats) = (0.0, f64::MIN, f64::MAX, 0u64, [0.0; 2]);
    for (tm, k, lat) in melts {
        let at = c * tm + held;
        if e < at {
            (hi, lats[1]) = (at, lat);
            break;
        }
        bits |= 1 << k;
        if e < at + lat {
            return Melting { t: tm, on: Some((k, (e - at) / lat)), held, lo, hi: f64::MIN, lat: lats, bits };
        }
        held += lat;
        (lo, lats[0]) = (at + lat, lat);
    }
    Melting { t: (e - held) / c, on: None, held, lo, hi, lat: lats, bits }
}

/// Heat a node with energy `e` and heat capacity `cap` can give before it falls to `T_FLOOR`, J.
#[inline]
pub(crate) fn heat_above_floor(e: f64, cap: f64) -> f64 {
    (e - cap.max(1.0) * T_FLOOR).max(0.0)
}

impl World {
    /// Heat capacity of a ground node's active layer, J/K: its bed's skin, then its loose amounts in order.
    pub fn heat_cap_ground(&self, i: usize) -> f64 {
        self.heat_cap_of(i, self.stock(Node::ground(i)), self.stock(Node::new(Class::Bed, i)))
    }
    /// Heat capacity of ground node `i`'s active layer with loose amounts `g` and bonded amounts `bed`, mol of each, J/K.
    pub(crate) fn heat_cap_of(&self, i: usize, g: impl Fn(usize) -> f64, bed: impl Fn(usize) -> f64) -> f64 {
        let ns = self.chem.ns();
        let skin = skin_cap(&self.chem, &self.cfg, self.mesh.cells().area()[i], 0..ns, bed);
        (0..ns).fold(skin, |c, k| c + g(k) * self.chem.subs[k].cp)
    }
    /// Melting point of substance `k` in ground node `i`, lowered by what is dissolved in its liquid (`Chemistry::melt_in`).
    pub fn melt(&self, k: usize, i: usize) -> f64 {
        self.chem.melt_in(k, self.stock(Node::ground(i)))
    }
    /// Temperature of a ground node from its amounts and heat in the store, and the substance on its plateau with its
    /// liquid share.
    pub fn thaw(&self, i: usize) -> (f64, Option<(usize, f64)>) {
        let (c, g) = (self.heat_cap_ground(i).max(1.0), self.stock(Node::ground(i)));
        let mut ks: Vec<(f64, usize)> = (0..self.chem.ns()).filter(|&k| g(k) > 0.0).map(|k| (self.chem.melt_in(k, &g), k)).collect();
        ks.sort_by(|a, b| a.0.total_cmp(&b.0));
        let th = melting(c, self.store.heat(Class::Ground)[i], ks.into_iter().map(|(tm, k)| (tm, k, g(k) * self.chem.subs[k].lf)));
        (th.t, th.on)
    }
    /// Temperature of ground node `i` as every law and the shell read it, K, with its plateau: Flow's solve at its last
    /// step; before Flow has run, `thaw`.
    pub fn thaw_last(&self, i: usize) -> (f64, Option<(usize, f64)>) {
        self.scratch.plateau(i).unwrap_or_else(|| self.thaw(i))
    }
    /// Temperature of ground node `i` as every law and the shell read it, K (`thaw_last`).
    pub fn t_ground_last(&self, i: usize) -> f64 {
        self.thaw_last(i).0
    }
    /// Temperature of ground node `i` from its amounts and energy now, K (`thaw`), for building a world and for tests.
    pub fn t_ground(&self, i: usize) -> f64 {
        self.thaw(i).0
    }
    /// Thermal energy of a ground node at temperature `t` with the fusion enthalpy molten there: the inverse of `thaw`.
    pub fn energy_ground(&self, i: usize, t: f64) -> f64 {
        let g = self.stock(Node::ground(i));
        let held: f64 = (0..self.chem.ns()).map(|k| g(k) * self.chem.fused_heat(k, t, &g)).sum();
        self.heat_cap_ground(i) * t + held
    }
    /// Molten share of substance `k` in ground node `i` at temperature `t` (from `thaw`).
    pub fn molten(&self, k: usize, i: usize, t: f64, plateau: Option<(usize, f64)>) -> f64 {
        match plateau {
            Some((pk, f)) if pk == k => f,
            _ => (t >= self.melt(k, i)) as u8 as f64,
        }
    }
    /// Heat capacity of air node `j`, J/K.
    pub fn heat_cap_air(&self, j: usize) -> f64 {
        self.cap_air(|k| self.a_seen(k, j))
    }
    /// Heat capacity of air that bodies see `seen` mol of each substance in, J/K.
    pub(crate) fn cap_air(&self, seen: impl Fn(usize) -> f64) -> f64 {
        (0..self.chem.ns()).map(|k| seen(k) * self.chem.subs[k].cp).sum()
    }
    /// Temperature of air node `j`, K.
    pub fn t_air(&self, j: usize) -> f64 {
        crate::flow::air_temp(self.store.heat(Class::Air)[j], self.heat_cap_air(j))
    }
    /// Height of the solid surface (bonded and loose solids), m, at the node's temperature now, with the solid share of a
    /// substance on its plateau.
    pub fn surface_z(&self, i: usize) -> f64 {
        let (t, pl) = self.thaw(i);
        let solid = |k: usize| (self.b(k, i) + self.g(k, i) * (1.0 - self.molten(k, i, t, pl))) * self.chem.molar_volume(k, chem::Phase::Solid);
        let v: f64 = (0..self.chem.ns()).map(solid).sum();
        self.base_z + v / self.mesh.cells().area()[i]
    }
}
