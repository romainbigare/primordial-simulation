//! Phases, melting and vapour pressure, solubility and heat capacity of the substances.

use super::{Chemistry, Phase, Substance};
use crate::units::{P_REF, R_GAS, T_SOL, T_SURF};

/// The vapour pressure and solubility curves are read no colder than this, K (they divide by the temperature).
const T_MIN: f64 = 1.0;
/// Least share whose logarithm the saturation and melting curves take (a share of 0 is read as this).
const LEAST_SHARE: f64 = 1e-30;
/// Least amount a share is taken of, mol (no division by 0 where a node holds nothing).
const LEAST_MOL: f64 = 1e-300;

impl Substance {
    /// Energy a mole holds apart from its heat while it is a gas in the air: its fusion and vaporisation
    /// enthalpy, J/mol (`check::totals` counts it in the air).
    #[inline]
    pub fn latent_gas(&self) -> f64 {
        self.lf + self.lv
    }
}

impl Chemistry {
    /// Is `k` a gas at `T_SURF`?
    #[inline]
    pub fn gas_at_surface(&self, k: usize) -> bool {
        self.phase(k, T_SURF) == Phase::Gas
    }

    /// Phase of `k` at `t` by its own melting and boiling points.
    pub fn phase(&self, k: usize, t: f64) -> Phase {
        let s = &self.subs[k];
        if t < s.melt {
            Phase::Solid
        } else if t < s.boil {
            Phase::Liquid
        } else {
            Phase::Gas
        }
    }

    /// m³/mol of a condensed phase (gas has no fixed volume here).
    pub fn molar_volume(&self, k: usize, p: Phase) -> f64 {
        let s = &self.subs[k];
        match p {
            Phase::Solid => s.molar_mass / s.density[0],
            Phase::Liquid => s.molar_mass / s.density[1],
            Phase::Gas => 0.0,
        }
    }

    /// Equilibrium vapour pressure (Pa) from the boiling point (Clausius-Clapeyron).
    pub fn p_sat(&self, k: usize, t: f64) -> f64 {
        let (tb, l) = (self.subs[k].boil, self.subs[k].lv);
        P_REF * (l / R_GAS * (1.0 / tb - 1.0 / t.max(T_MIN))).exp()
    }

    /// Temperature at which the vapour pressure of `k` is `x` times the reference pressure, K.
    pub(crate) fn t_sat(&self, k: usize, x: f64) -> f64 {
        let (tb, l) = (self.subs[k].boil, self.subs[k].lv);
        1.0 / (1.0 / tb - R_GAS * x.max(LEAST_SHARE).ln() / l)
    }

    /// Melting point of `k` where it is the share `x` of the moles of its liquid, the rest dissolved in it, K: an ideal
    /// solution freezes lower (Schroeder-van Laar, the mirror of `t_sat`): 1 / T = 1 / Tm - R ln x / Lf. For the common
    /// liquid of chemistry 0 that is 1.86 K per mol dissolved in a kg (the measured cryoscopic constant), no new number.
    pub(crate) fn t_melt(&self, k: usize, x: f64) -> f64 {
        let (tm, l) = (self.subs[k].melt, self.subs[k].lf);
        if l <= 0.0 || x >= 1.0 {
            return tm;
        }
        1.0 / (1.0 / tm - R_GAS * x.max(LEAST_SHARE).ln() / l)
    }

    /// Melting point of `k` among the amounts `a(j)` (mol) of one node or body, K: `t_melt` at the share of `k` among
    /// itself and what is dissolved in it, where `k` is the solvent there: a liquid of the world (liquid at `T_SURF`) and
    /// the most of them. Dissolved in it is what melts below its melting point and is liquid there (the minerals that live
    /// in the liquid); a solid there (the energy-rich solid) is not, nor `k` itself, nor a gas: a gas dissolves only to
    /// Henry's share (a few parts in 100,000 of the air's, a thousandth of a kelvin lower), and what a node or body holds
    /// of it beyond that is not dissolved. A solute keeps its own melting point: the data give a dissolved mineral that of
    /// its solution. Bodies and ground nodes (their pores, the liquid that stands on them, its ice) freeze by it;
    /// `phase_in` reads phases through it. Where `k` is absent it melts at its own melting point.
    pub fn melt_in(&self, k: usize, a: impl Fn(usize) -> f64) -> f64 {
        match self.mixes(k) {
            Some(js) => self.melt_with(k, a, js),
            None => self.subs[k].melt,
        }
    }

    /// What may share the liquid `k` (`melt_in`): each with whether it is dissolved in `k` and whether it is a liquid of
    /// the world; none: `k` is not a liquid of the world.
    pub fn mixes(&self, k: usize) -> Option<impl Iterator<Item = (usize, bool, bool)> + '_> {
        let (s, liquid) = (&self.subs[k], |d: &Substance| d.melt < T_SURF && d.boil > T_SURF);
        let mix = move |j: usize, d: &Substance| (j, d.melt < s.melt && d.boil > s.melt, liquid(d));
        liquid(s).then(move || (0..self.ns()).filter(move |&j| j != k).map(move |j| mix(j, &self.subs[j])).filter(|m| m.1 || m.2))
    }

    /// `melt_in` over the list `mixes` gave.
    pub(crate) fn melt_with(&self, k: usize, a: impl Fn(usize) -> f64, js: impl Iterator<Item = (usize, bool, bool)>) -> f64 {
        let (x, mut other, mut most) = (a(k).max(0.0), 0.0, 0.0f64);
        for (j, dissolved, liquid) in js {
            let y = a(j).max(0.0);
            other += if dissolved { y } else { 0.0 };
            most = if liquid { most.max(y) } else { most };
        }
        if most > x || x <= 0.0 { self.subs[k].melt } else { self.t_melt(k, x / (x + other).max(LEAST_MOL)) }
    }

    /// Phase of `k` at `t` among the amounts `a(j)` (mol) of one node or body: as `phase`, at its melting point there
    /// (`melt_in`).
    pub(crate) fn phase_in(&self, k: usize, t: f64, a: impl Fn(usize) -> f64) -> Phase {
        match self.phase(k, t) {
            Phase::Solid if t >= self.melt_in(k, a) => Phase::Liquid,
            p => p,
        }
    }

    /// Fusion enthalpy a mole of `k` holds in the heat of a node or body at `t` among its amounts `a(j)`, J/mol: none
    /// while it is solid there (`phase_in`).
    #[inline]
    pub fn fused_heat(&self, k: usize, t: f64, a: impl Fn(usize) -> f64) -> f64 {
        if self.phase_in(k, t, a) != Phase::Solid { self.subs[k].lf } else { 0.0 }
    }

    /// Mole fraction of gas `k` that a liquid at `t` holds under its partial pressure `p`, Pa
    /// (Henry's law). It falls with warmth by van 't Hoff's law, with the heat of solution taken
    /// as the vaporisation enthalpy: dissolving is like condensing (for the gas the light-driven
    /// reaction takes in chemistry 0 that gives 2,100 K for the 2,400 K measured; for the others
    /// 800 to 1,200 K for 1,300 to 1,700 K).
    pub fn henry(&self, k: usize, t: f64, p: f64) -> f64 {
        let s = &self.subs[k];
        s.solubility * p / P_REF * (s.lv / R_GAS * (1.0 / t.max(T_MIN) - 1.0 / T_SOL)).exp()
    }

    /// Heat capacity of the amounts `m` (mol of each substance), J/K.
    #[inline]
    pub fn heat_cap_of(&self, m: &[f64]) -> f64 {
        m.iter().zip(&self.subs).map(|(a, s)| a * s.cp).sum()
    }

    /// Mass of amounts `m` (mol of each substance), kg.
    pub fn mass_of(&self, m: &[f64]) -> f64 {
        m.iter().zip(&self.subs).map(|(a, s)| a * s.molar_mass).sum()
    }

    /// The heat that goes with amounts `m` (mol of each substance) when they leave a holder of heat `e` (J) and heat capacity
    /// `cap` (J/K): their share of it by heat capacity, J.
    #[inline]
    pub fn heat_share(&self, e: f64, cap: f64, m: &[f64]) -> f64 {
        e * self.heat_cap_of(m) / cap
    }

    /// Optical depth to outgoing heat radiation of a column of `c` mol/m² of substance `k`.
    pub fn depth(&self, k: usize, c: f64) -> f64 {
        let s = &self.subs[k];
        s.ir_abs * if s.ir_sat > 0.0 && c > s.ir_sat { (s.ir_sat * c).sqrt() } else { c }
    }
}
