//! What the reactions and conversions do for life: their heat, which burn, which keep their energy, what life builds
//! of, and the roles the first life is placed with (`life_roles`, `matter_roles`, `fixing`).

use super::{Chemistry, Phase, Reaction};
use crate::units::T_SURF;

/// A reaction keeps energy (`keeps`) only where a burning gets more than this share more heat out of the same input:
/// equal heats with rounding are not a difference.
const KEEPS_MARGIN: f64 = 1e-9;

impl Chemistry {
    /// Enthalpy change of one unit of reaction `r`, J (positive takes energy).
    pub fn dh(&self, r: usize) -> f64 {
        self.dh_of(&self.reactions[r])
    }

    /// Enthalpy change of one unit of a reaction or a conversion (`build`, `builds`), J (positive takes energy).
    pub fn dh_of(&self, rx: &Reaction) -> f64 {
        let sum = |v: &[(u16, u16)]| v.iter().map(|&(k, n)| n as f64 * self.subs[k as usize].hf).sum::<f64>();
        sum(&rx.outputs) - sum(&rx.inputs)
    }

    /// Enthalpy change of one unit of reaction `r` with each substance in its phase at `t`, J: at `T_SURF` the reaction
    /// heat as measured.
    pub fn dh_at(&self, r: usize, t: f64) -> f64 {
        // A liquid holds its fusion enthalpy more, a gas also its vaporisation enthalpy.
        let h = |k: usize| self.subs[k].hf + [0.0, self.subs[k].lf, self.subs[k].latent_gas()][self.phase(k, t) as usize];
        let sum = |v: &[(u16, u16)]| v.iter().map(|&(k, n)| n as f64 * h(k as usize)).sum::<f64>();
        let rx = &self.reactions[r];
        sum(&rx.outputs) - sum(&rx.inputs)
    }

    /// Does reaction `r` burn: release heat taking a gas (at `T_SURF`)?
    pub fn burning(&self, r: usize) -> bool {
        self.dh(r) < 0.0 && self.reactions[r].inputs.iter().any(|p| self.gas_at_surface(p.0 as usize))
    }

    /// The first reaction that burns `k` (`burning`).
    pub(super) fn burner(&self, k: usize) -> Option<usize> {
        (0..self.reactions.len()).find(|&r| self.burning(r) && self.reactions[r].inputs.iter().any(|p| p.0 as usize == k))
    }

    /// Does reaction `r` get less heat out of a solid or liquid it takes than burning it does? Then it keeps the rest in
    /// what it makes: no burning, run at its catalyst's pace (laws.md 2.1).
    pub fn keeps(&self, r: usize) -> bool {
        let per = |q: usize, k: u16| self.reactions[q].inputs.iter().find(|p| p.0 == k).map_or(f64::NEG_INFINITY, |p| -self.dh(q) / p.1 as f64);
        (self.reactions[r].inputs.iter().filter(|p| !self.gas_at_surface(p.0 as usize))).any(|p| (0..self.reactions.len()).any(|q| self.burning(q) && per(q, p.0) > (1.0 + KEEPS_MARGIN) * per(r, p.0)))
    }

    /// The solids bodies build besides their matter (the first output of each of `builds`).
    pub fn materials(&self) -> impl Iterator<Item = usize> + '_ {
        self.builds.iter().filter_map(|b| b.outputs.first()).map(|p| p.0 as usize)
    }

    /// The body matter: the first output of growth (`build`), if bodies grow.
    pub fn matter(&self) -> Option<usize> {
        self.build.as_ref().and_then(|b| b.outputs.first()).map(|p| p.0 as usize)
    }

    /// What growth makes body matter of besides the fuel, with the mol per mol of matter: the inputs no reaction burns,
    /// which a body takes from the ground or what it eats. Empty where bodies do not grow or grow from one fuel.
    pub fn minerals(&self) -> &[(usize, f64)] {
        &self.derived().minerals
    }

    /// The roles a first body's chemistry is built on, found from the numbers: the light reaction (one that takes
    /// energy), the energy-rich solid it makes, the liquid it uses, and the reaction that burns that solid with a gas.
    pub fn life_roles(&self) -> Option<(usize, usize, usize, usize)> {
        self.derived().roles
    }

    /// `life_roles` worked out (`Chemistry::derived`).
    pub(super) fn find_life_roles(&self) -> Option<(usize, usize, usize, usize)> {
        let at = |k: u16| self.phase(k as usize, T_SURF);
        let r_light = (0..self.reactions.len()).find(|&r| self.reactions[r].light && self.dh(r) > 0.0)?;
        let rx = &self.reactions[r_light];
        let rich = rx.outputs.iter().find(|p| at(p.0) == Phase::Solid)?.0 as usize;
        let liquid = rx.inputs.iter().find(|p| at(p.0) == Phase::Liquid)?.0 as usize;
        Some((r_light, rich, liquid, self.burner(rich)?))
    }

    /// Body matter and the reaction that burns it with a gas (none where bodies do not grow).
    pub fn matter_roles(&self) -> Option<(usize, usize)> {
        let m = self.matter()?;
        Some((m, self.burner(m)?))
    }

    /// The fixing reaction: the one that makes a mineral of growth (`minerals`) from an element only a gas it takes brings.
    pub fn fixing(&self) -> Option<usize> {
        self.derived().fixing
    }

    /// `fixing` worked out from the `minerals` of growth (`Chemistry::derived`).
    pub(super) fn find_fixing(&self, minerals: &[(usize, f64)]) -> Option<usize> {
        let (gas, has) = (|k: u16| self.gas_at_surface(k as usize), |k: u16, e: usize| self.subs[k as usize].elems[e] > 0);
        (0..self.reactions.len()).find(|&r| {
            let i = &self.reactions[r].inputs;
            let from_air = |m: u16| (0..self.n_elements).any(|e| has(m, e) && i.iter().all(|p| !has(p.0, e) || gas(p.0)) && i.iter().any(|p| has(p.0, e)));
            self.reactions[r].outputs.iter().any(|p| minerals.iter().any(|m| m.0 == p.0 as usize) && from_air(p.0))
        })
    }
}
