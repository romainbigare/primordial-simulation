//! Phases, melting and vapour pressure, solubility and heat capacity of the substances.

use super::{Chemistry, Phase, Substance};
use crate::units::{P_REF, R_GAS, T_SOL, T_SURF};

/// Molar heat capacity of an ideal gas of two atoms, J/(mol K): the air's when none is known.
pub(super) const DIATOMIC_CP: f64 = 3.5 * R_GAS;

/// The vapour pressure and solubility curves are read no colder than this, K (they divide by the temperature).
const T_MIN: f64 = 1.0;
/// Least share whose logarithm the saturation and melting curves take (a share of 0 is read as this).
const LEAST_SHARE: f64 = 1e-30;
/// Least amount a share is taken of, mol (no division by 0 where a node holds nothing).
const LEAST_MOL: f64 = 1e-300;

impl Substance {
    /// Energy a mole holds apart from its heat while a gas in the air: its fusion and vaporisation enthalpy, J/mol.
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
        if t < s.melt { Phase::Solid } else if t < s.boil { Phase::Liquid } else { Phase::Gas }
    }

    /// m³/mol of a condensed phase (gas has no fixed volume here).
    pub fn molar_volume(&self, k: usize, p: Phase) -> f64 {
        if p == Phase::Gas { 0.0 } else { self.subs[k].molar_mass / self.subs[k].density[p as usize] }
    }

    /// Equilibrium vapour pressure (Pa) from the boiling point (Clausius-Clapeyron).
    pub fn p_sat(&self, k: usize, t: f64) -> f64 {
        let (tb, l) = (self.subs[k].boil, self.subs[k].lv);
        P_REF * (l / R_GAS * (1.0 / tb - 1.0 / t.max(T_MIN))).exp()
    }


    /// Melting point of `k` as the share `x` of its liquid's moles, K: an ideal solution freezes lower (Schroeder-van
    /// Laar): 1 / T = 1 / Tm - R ln x / Lf; for the common liquid that is the measured 1.86 K per mol in a kg.
    pub(crate) fn t_melt(&self, k: usize, x: f64) -> f64 {
        let (tm, l) = (self.subs[k].melt, self.subs[k].lf);
        if l <= 0.0 || x >= 1.0 {
            return tm;
        }
        1.0 / (1.0 / tm - R_GAS * x.max(LEAST_SHARE).ln() / l)
    }

    /// Melting point of `k` among the amounts `a(j)` of one node or body, K: `t_melt` at its share among itself and what
    /// is dissolved in it, where `k` is the solvent (laws.md 2.4). A gas counts not: it dissolves only to Henry's share,
    /// a thousandth of a kelvin. Where `k` is absent it melts at its own point.
    pub fn melt_in(&self, k: usize, a: impl Fn(usize) -> f64) -> f64 {
        let Some(js) = self.mixes(k).filter(|_| self.lowers_melt(k)) else { return self.subs[k].melt };
        let (x, mut other, mut most) = (a(k).max(0.0), 0.0, 0.0f64);
        for &(j, dissolved, liquid) in js {
            let y = a(j).max(0.0);
            other += if dissolved { y } else { 0.0 };
            most = if liquid { most.max(y) } else { most };
        }
        if most > x || x <= 0.0 { self.subs[k].melt } else { self.t_melt(k, x / (x + other).max(LEAST_MOL)) }
    }

    /// The temperature at which the liquid `k` among the amounts `a(j)` of one node or body is densest, K: its own
    /// (`Substance::dense`), lowered in proportion to the mass dissolved in it per mass of it (Despretz's law,
    /// `Substance::dense_drop`; what is dissolved as `melt_in` counts it), and no lower than its melting point there
    /// (`melt_in`): a liquid salty enough is densest where it freezes.
    pub fn dense_in(&self, k: usize, a: impl Fn(usize) -> f64) -> f64 {
        let s = &self.subs[k];
        let solvent = a(k).max(0.0) * s.molar_mass;
        let dissolved: f64 = self.mixes(k).map_or(0.0, |js| js.iter().filter(|j| j.1).map(|&(j, ..)| a(j).max(0.0) * self.subs[j].molar_mass).sum());
        let t = if solvent > 0.0 { s.dense - s.dense_drop * dissolved / solvent } else { s.dense };
        t.max(self.melt_in(k, &a))
    }

    /// What may share the liquid `k` (`melt_in`), each with whether it dissolves in `k` and whether it is a liquid of the
    /// world; none: `k` is no liquid of the world. Found once per chemistry (`find_mixes`).
    pub fn mixes(&self, k: usize) -> Option<&[(usize, bool, bool)]> {
        self.derived().mixes[k].as_deref()
    }

    /// The substances others dissolve in (`mixes` is some), a bit per substance.
    pub(crate) fn solvents(&self) -> u64 {
        self.derived().solvents
    }

    /// Whether others dissolve in `k` (`mixes` is some): only such a substance melts below its own point (`melt_in`).
    #[inline(always)]
    pub(crate) fn lowers_melt(&self, k: usize) -> bool {
        self.solvents() >> k & 1 == 1
    }

    /// `mixes` of `k`, from the substances' points.
    pub(super) fn find_mixes(&self, k: usize) -> Option<Vec<(usize, bool, bool)>> {
        let (s, liquid) = (&self.subs[k], |d: &Substance| d.melt < T_SURF && d.boil > T_SURF);
        let mix = |(j, d): (usize, &Substance)| (j, d.melt < s.melt && d.boil > s.melt, liquid(d));
        liquid(s).then(|| self.subs.iter().enumerate().filter(|e| e.0 != k).map(mix).filter(|m| m.1 || m.2).collect())
    }


    /// Phase of `k` at `t` among the amounts `a(j)` of one node or body, at its melting point there (`melt_in`).
    #[inline(always)]
    pub(crate) fn phase_in(&self, k: usize, t: f64, a: impl Fn(usize) -> f64) -> Phase {
        match self.phase(k, t) {
            Phase::Solid if self.lowers_melt(k) && t >= self.melt_in(k, a) => Phase::Liquid,
            p => p,
        }
    }

    /// Fusion enthalpy a mole of `k` holds in the heat of a node or body at `t`, J/mol: none while solid there.
    #[inline]
    pub fn fused_heat(&self, k: usize, t: f64, a: impl Fn(usize) -> f64) -> f64 {
        if self.phase_in(k, t, a) != Phase::Solid { self.subs[k].lf } else { 0.0 }
    }

    /// Mole fraction of gas `k` a liquid at `t` holds under its partial pressure `p`, Pa (Henry's law), falling with
    /// warmth by van 't Hoff's law with the vaporisation enthalpy as heat of solution (2,100 K for the light reaction's
    /// gas, measured 2,400 K).
    pub fn henry(&self, k: usize, t: f64, p: f64) -> f64 {
        let s = &self.subs[k];
        s.solubility * p / P_REF * (s.lv / R_GAS * (1.0 / t.max(T_MIN) - 1.0 / T_SOL)).exp()
    }

    /// Molar heat capacity of the starting air, J/(mol K) (`DIATOMIC_CP` if none is known).
    pub fn air_cp(&self) -> f64 {
        self.derived().air_cp
    }

    /// Heat capacity of the amounts `m` (mol of each substance), J/K.
    #[inline]
    pub fn heat_cap_of(&self, m: &[f64]) -> f64 {
        m.iter().zip(&self.subs).map(|(a, s)| a * s.cp).sum()
    }

    /// Optical depth to outgoing heat radiation of a column of `c` mol/m² of substance `k`.
    pub fn depth(&self, k: usize, c: f64) -> f64 {
        let s = &self.subs[k];
        s.ir_abs * if s.ir_sat > 0.0 && c > s.ir_sat { (s.ir_sat * c).sqrt() } else { c }
    }
}
