//! Heat of ground and air nodes: heat capacities, temperatures, the melting plateaus of the
//! ground, and drawing heat from a node without taking it below `T_FLOOR`.

use crate::chem::{self, Chemistry};
use crate::params::Config;
use crate::units::T_FLOOR;
use crate::world::{Counters, World};

/// Take up to `q` J from a node with energy `e` and heat capacity `cap` (at most what it holds above `T_FLOOR`), count
/// the shortfall in `Counters::clamped_heat`, return what was taken: whoever wanted `q` passes on only that. A negative
/// `q` gives heat to the node and is returned whole. For the moves not booked by the store (bodies, parts and ground
/// reactions), whose guard this is; the store's is `store::apply`.
#[inline]
pub(crate) fn draw(e: &mut f64, counts: &mut Counters, cap: f64, q: f64) -> f64 {
    let took = if q > 0.0 { q.min(heat_above_floor(*e, cap)) } else { q };
    *e -= took;
    counts.clamped_heat += q - took;
    took
}

/// The share of a wanted amount that was really given (exactly 1 when all of it was).
#[inline]
pub(crate) fn share(took: f64, want: f64) -> f64 {
    if took >= want { 1.0 } else { took / want }
}

/// Take `d` mol (of `cp` J/(mol K) each) from a ground node of energy `e` and heat capacity `cap`, which pays `want` J with
/// them (the heat and latent enthalpy they take): it pays as far as it holds heat above `T_FLOOR` without their heat
/// capacity (`draw`), and as much less is taken. Returns the mol taken and the heat paid (nothing when `want` <= 0).
#[inline]
pub(crate) fn take_paid(e: &mut f64, counts: &mut Counters, cap: f64, d: f64, cp: f64, want: f64) -> (f64, f64) {
    if want <= 0.0 {
        return (0.0, 0.0);
    }
    let took = draw(e, counts, cap - d * cp, want);
    (d * share(took, want), took)
}

/// Heat capacity of the skin of a ground node's bed, J/K: `skin_m` of its bonded amounts `b(k)` (mol) of the substances
/// `ks` (any that may be there, in order), at their mean heat capacity per volume.
pub(crate) fn skin_cap(chem: &Chemistry, cfg: &Config, area: f64, ks: impl IntoIterator<Item = usize>, b: impl Fn(usize) -> f64) -> f64 {
    let (mut c, mut v) = (0.0, 0.0);
    for k in ks {
        c += b(k) * chem.subs[k].cp;
        v += b(k) * chem.molar_volume(k, chem::Phase::Solid);
    }
    if v > 0.0 { c / v * cfg.skin_m * area } else { 0.0 }
}

/// Where a ground node or a body of heat capacity `c` (J/K) and energy `e` (J) stands among its melting plateaus.
pub(crate) struct Melting {
    /// Temperature, K
    pub t: f64,
    /// The substance held at its melting point, with its molten share
    pub on: Option<(usize, f64)>,
    /// Fusion enthalpy held by what is molten (not counting `on`), J
    pub held: f64,
    /// The energies between which the node stays off a plateau with this `held`, J (`f64::MIN` for `hi` on a plateau)
    pub lo: f64,
    pub hi: f64,
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

/// Temperature of a node (or body) of heat capacity `c` and energy `e`, where molten amounts hold their fusion enthalpy: `melts` gives,
/// in the order they melt, each substance's melting point there, the substance and the fusion enthalpy of its amount (J). A
/// node stays at a melting point (a plateau) until all of that substance has frozen or melted. `World::energy_ground` is the
/// inverse.
pub(crate) fn melting(c: f64, e: f64, melts: impl IntoIterator<Item = (f64, usize, f64)>) -> Melting {
    let (mut held, mut lo, mut hi, mut bits) = (0.0, f64::MIN, f64::MAX, 0u64);
    for (tm, k, lat) in melts {
        let at = c * tm + held;
        if e < at {
            hi = at;
            break;
        }
        bits |= 1 << k;
        if e < at + lat {
            return Melting { t: tm, on: Some((k, (e - at) / lat)), held, lo, hi: f64::MIN, bits };
        }
        held += lat;
        lo = at + lat;
    }
    Melting { t: (e - held) / c, on: None, held, lo, hi, bits }
}

/// Heat a node with energy `e` and heat capacity `cap` can give before it falls to `T_FLOOR`, J.
#[inline]
pub(crate) fn heat_above_floor(e: f64, cap: f64) -> f64 {
    (e - cap.max(1.0) * T_FLOOR).max(0.0)
}

impl World {
    /// Heat capacity of a ground node's active layer, J/K: the skin of its bed (`skin_cap`), then its loose amounts in the
    /// order of the substances (the sum `flow::land::ground_state` keeps for every node).
    pub fn heat_cap_ground(&self, i: usize) -> f64 {
        let (n, ns) = (self.grid.n(), self.chem.ns());
        let skin = skin_cap(&self.chem, &self.cfg, self.grid.area(), 0..ns, |k| self.bed[k * n + i]);
        (0..ns).fold(skin, |c, k| c + self.ground[k * n + i] * self.chem.subs[k].cp)
    }
    /// Melting point of substance `k` in ground node `i`, lowered by what is dissolved in its liquid (`Chemistry::melt_in`).
    pub fn melt(&self, k: usize, i: usize) -> f64 {
        self.chem.melt_in(k, |j| self.g(j, i))
    }
    /// Temperature of a ground node, and the substance held at its melting point with its
    /// liquid share, if any. Molten amounts hold their fusion enthalpy in `e_ground`, so a node
    /// freezes or melts at a plateau.
    pub fn thaw(&self, i: usize) -> (f64, Option<(usize, f64)>) {
        let c = self.heat_cap_ground(i).max(1.0);
        let mut ks: Vec<(f64, usize)> = (0..self.chem.ns()).filter(|&k| self.g(k, i) > 0.0).map(|k| (self.melt(k, i), k)).collect();
        ks.sort_by(|a, b| a.0.total_cmp(&b.0));
        let th = melting(c, self.e_ground[i], ks.into_iter().map(|(tm, k)| (tm, k, self.g(k, i) * self.chem.subs[k].lf)));
        (th.t, th.on)
    }
    /// `take_paid` from ground node `i`.
    pub(crate) fn take_paid(&mut self, i: usize, d: f64, cp: f64, want: f64) -> (f64, f64) {
        if want <= 0.0 {
            return (0.0, 0.0);
        }
        let cap = self.heat_cap_ground(i);
        take_paid(&mut self.e_ground[i], &mut self.counts, cap, d, cp, want)
    }
    /// Temperature of ground node `i` as every law and the shell read it, K, and the substance on its melting plateau with
    /// its molten share: Flow's solve of the plateaus at its last step (`flow::land::ground_state`), which keeps it for its
    /// own moves; before Flow has run, the same solve from the amounts now (`thaw`).
    pub fn thaw_last(&self, i: usize) -> (f64, Option<(usize, f64)>) {
        self.scratch.plateau(i).unwrap_or_else(|| self.thaw(i))
    }
    /// Temperature of ground node `i` as every law and the shell read it, K (`thaw_last`).
    pub fn t_ground_last(&self, i: usize) -> f64 {
        self.thaw_last(i).0
    }
    /// Temperature of ground node `i` from its amounts and energy now, K (`thaw`): for building a world and for tests
    /// that set a node's state by hand.
    pub fn t_ground(&self, i: usize) -> f64 {
        self.thaw(i).0
    }
    /// Thermal energy of a ground node at temperature `t` with the fusion enthalpy of what is molten there
    /// (`Chemistry::fused_heat`: all of it at its melting point): the inverse of `thaw` (`melting`).
    pub fn energy_ground(&self, i: usize, t: f64) -> f64 {
        let held: f64 = (0..self.chem.ns()).map(|k| self.g(k, i) * self.chem.fused_heat(k, t, |j| self.g(j, i))).sum();
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
        (0..self.chem.ns()).map(|k| self.a(k, j) * self.chem.subs[k].cp).sum()
    }
    /// Temperature of air node `j`, K.
    pub fn t_air(&self, j: usize) -> f64 {
        crate::flow::air_temp(self.e_air[j], self.heat_cap_air(j))
    }
    /// Height of the solid surface (bonded + loose solids), m, at the node's temperature now and with the solid share of a
    /// substance on its plateau (`survey::env_of` reads the last flow step's temperature and whole phases; `flow::land::surface`
    /// adds the soft cover and the reflectance).
    pub fn surface_z(&self, i: usize) -> f64 {
        let (t, pl) = self.thaw(i);
        let mut v = 0.0;
        for k in 0..self.chem.ns() {
            let mv = self.chem.molar_volume(k, chem::Phase::Solid);
            v += (self.b(k, i) + self.g(k, i) * (1.0 - self.molten(k, i, t, pl))) * mv;
        }
        self.base_z + v / self.grid.area()
    }
}
