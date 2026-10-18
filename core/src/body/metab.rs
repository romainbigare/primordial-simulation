//! The body's chemistry: the pace of its temperature, its catalysts, digestion and the fuel it burns.

use super::{Body, Layout};
use crate::chem::{Chemistry, Phase};

impl Body {
    /// How fast the body's chemistry runs at its temperature `t` against `Physics::t_cat`, where the costs are anchored:
    /// the rate law of Change, exp(-Ea (1/t - 1/t_cat)), Ea the barrier of its main burning reaction (the first of
    /// `Consts::burn`), 1 for a body that burns nothing.
    pub fn pace(&self, chem: &Chemistry, p: &crate::Physics, t: f64) -> f64 {
        match self.k.burn.first() {
            Some(&(r, _)) => (-chem.reactions[r].ea * (1.0 / t.max(1.0) - 1.0 / p.t_cat)).exp(),
            None => 1.0,
        }
    }
    /// End-product inhibition, 0..1: a catalyst is slowed by what the body holds of the reaction's products, its speed-up
    /// times 1 / (1 + Σ x / x*), x the store of each product and x* the level it keeps of it (`level`; a level of 0 stops
    /// it): non-competitive inhibition with the body's own level as the constant. Gases at `t` (they leave) and its key
    /// liquids do not count. The reactions it runs on demand to pay (`pay::plan`) are not slowed.
    pub fn inhibited(&self, chem: &Chemistry, r: usize, mass: f64, t: f64) -> f64 {
        let out = chem.reactions[r].outputs.iter().map(|p| p.0 as usize).filter(|&k| self.wet >> k & 1 == 0 && chem.phase(k, t) != Phase::Gas);
        1.0 / (1.0 + out.map(|k| (self.level(chem, k, mass), self.store[k].max(0.0))).map(|(l, x)| if l > 0.0 { x / l } else { f64::INFINITY }).sum::<f64>())
    }
    /// A catalyst's factor `f` (the speed-up of a reaction) at activity `a`: 1 + (f - 1) a. A dry body's catalysts do not
    /// work, but the bare reaction still runs.
    #[inline]
    pub fn catalyst(f: f64, a: f64) -> f64 {
        1.0 + (f - 1.0) * a
    }
    /// Digestion: `taken` mol of body matter that this body took in from outside are broken into its units as they come
    /// in, the reverse of growth (`Chemistry::build`), so it enters as fuel, never as body; the body builds its own matter
    /// from that, at the cost of work (`copy::grow`). The other inputs come from its own stores, as far as they go. Returns
    /// the units of growth reversed.
    pub fn digest(&mut self, chem: &Chemistry, taken: f64) -> f64 {
        let Some(bl) = chem.build.as_ref() else { return 0.0 };
        let m = bl.outputs[0].0 as usize;
        let mut x = (taken / bl.outputs[0].1 as f64).min(self.store[m] / bl.outputs[0].1 as f64);
        for &(k, nu) in &bl.outputs[1..] {
            x = x.min(self.store[k as usize] / nu as f64);
        }
        if x <= 0.0 {
            return 0.0;
        }
        for &(k, nu) in &bl.outputs {
            self.store[k as usize] = (self.store[k as usize] - nu as f64 * x).max(0.0);
        }
        for &(k, nu) in &bl.inputs {
            self.store[k as usize] += nu as f64 * x;
        }
        self.e -= -chem.build.as_ref().map_or(0.0, |b| chem.dh_of(b)) * x;
        x
    }
    /// Energy a mole of `k` gives its body's catalysed reactions at temperature `t`, J (0 for a gas or what none of its
    /// reactions burns): the best over `Consts::burn`.
    pub(crate) fn heat_of(&self, chem: &Chemistry, k: usize, t: f64) -> f64 {
        if chem.phase(k, t) == Phase::Gas {
            return 0.0;
        }
        self.k.burn.iter().filter_map(|&(r, e)| chem.reactions[r].inputs.iter().find(|x| x.0 as usize == k).map(|x| e / x.1 as f64)).fold(0.0, f64::max)
    }
    /// Does it break substance `k`: is its catalyst on for a reaction that takes `k`? A body takes in body matter and the
    /// solids bodies build only if it does (`Body::wanted`).
    pub fn breaks(&self, chem: &Chemistry, k: usize) -> bool {
        let cat = Layout::of(chem).cat;
        (0..chem.reactions.len()).any(|r| self.genes[cat + r] > 0.0 && chem.reactions[r].inputs.iter().any(|p| p.0 as usize == k))
    }
    /// Energy its catalysed reactions that release heat could release from its stores at temperature `t` (each as far as
    /// its inputs that are not gas go), J.
    pub fn fuel(&self, chem: &Chemistry, t: f64) -> f64 {
        let mut e = 0.0;
        for &(r, q) in &self.k.burn {
            let solid = chem.reactions[r].inputs.iter().filter(|p| chem.phase(p.0 as usize, t) != Phase::Gas);
            e += q * solid.map(|&(k, nu)| self.store[k as usize] / nu as f64).fold(f64::MAX, f64::min).min(1e30);
        }
        e
    }
    /// Does it burn substance `k` at its temperature `t`: is `k` an input, not a gas at `t`, of one of `Consts::burn`? A
    /// store it burns is fuel: a body short of fuel dies when it cannot pay its upkeep (`SPENT`), not when the store is
    /// below its safe minimum, which is then only the floor that growth and giving keep.
    pub fn burns_at(&self, chem: &Chemistry, k: usize, t: f64) -> bool {
        chem.phase(k, t) != Phase::Gas && self.k.burn.iter().any(|&(r, _)| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == k))
    }
}
