//! The body's chemistry: the pace of its temperature, its catalysts, digestion and the fuel it burns.

use super::{Body, DENATURE};
use crate::chem::{Chemistry, Phase};

impl Body {
    /// How fast the body's chemistry runs at `t`: its main burning's rate constant (`rate_at`, active) over that at
    /// `t_cat`, so it rises by Arrhenius up to the catalysts' unfolding and falls past it (1 for a body that burns
    /// nothing). What its catalysts do for its supply, taking and building, goes as the energy they release.
    pub fn pace(&self, chem: &Chemistry, p: &crate::Physics, t: f64) -> f64 {
        self.k.burn.first().map_or(1.0, |&(r, _)| self.rate_at(chem, p, r, 1.0, t.max(1.0)) / self.k.ref_rate[0])
    }
    /// The pace of its upkeep at `t`: the bare rate law of its main burning, k exp(-Ea/t), over its value at `t_cat` (1
    /// for a body that burns nothing). It never falls with heat: unfolded proteins are tagged and replaced, so a body past
    /// its catalysts' unfolding pays more, not less (Hofmann and Somero 1995).
    pub fn upkeep_pace(&self, chem: &Chemistry, t: f64) -> f64 {
        self.k.burn.first().map_or(1.0, |&(r, _)| chem.reactions[r].arrhenius(t.max(1.0)) / self.k.ref_rate[1])
    }
    /// The rate constant of reaction `r` as the body catalyses it at activity `a` and `t`, 1/s: its catalyst's factor
    /// (`catalyst`), unfolded past its `DENATURE` gene (`change::denature`), times k exp(-Ea/t).
    pub(crate) fn rate_at(&self, chem: &Chemistry, p: &crate::Physics, r: usize, a: f64, t: f64) -> f64 {
        crate::change::denature(Self::catalyst(self.k.cat[r], a), t, self.gene(DENATURE), p.unfold) * chem.reactions[r].arrhenius(t)
    }
    /// End-product inhibition, 0..1: 1 - Σ x / x*, at least 0, x the store of each product and x* its level: a full
    /// store stops the enzyme that fills it, as roots take up to the level (laws.md 2.4). Gases and key liquids do not
    /// count.
    pub fn inhibited(&self, chem: &Chemistry, r: usize, mass: f64, t: f64) -> f64 {
        let out = chem.reactions[r].outputs.iter().map(|p| p.0 as usize).filter(|&k| self.wet >> k & 1 == 0 && chem.phase(k, t) != Phase::Gas);
        (1.0 - out.map(|k| (self.level(chem, k, mass), self.store[k].max(0.0))).map(|(l, x)| if l > 0.0 { x / l } else { f64::INFINITY }).sum::<f64>()).max(0.0)
    }
    /// A catalyst's factor `f` at activity `a`: 1 + (f - 1) a (a dry body's bare reactions still run).
    #[inline]
    pub fn catalyst(f: f64, a: f64) -> f64 {
        1.0 + (f - 1.0) * a
    }
    /// Digestion: `taken` mol of body matter from outside are broken back into what growth made it of as they come in,
    /// so what it eats enters as fuel, never as body; the other inputs come from its stores. Returns the units reversed.
    pub fn digest(&mut self, chem: &Chemistry, taken: f64, rounded: &mut f64) -> f64 {
        let Some(bl) = chem.build.as_ref() else { return 0.0 };
        let m = bl.outputs[0].0 as usize;
        let most = (taken / bl.outputs[0].1 as f64).min(self.store[m] / bl.outputs[0].1 as f64);
        let x = bl.outputs[1..].iter().fold(most, |x, &(k, nu)| x.min(self.store[k as usize] / nu as f64));
        if x <= 0.0 {
            return 0.0;
        }
        for &(k, nu) in &bl.outputs {
            super::add_store(&mut self.store[k as usize], -(nu as f64) * x, rounded);
        }
        for &(k, nu) in &bl.inputs {
            self.store[k as usize] += nu as f64 * x;
        }
        self.e -= -chem.dh_of(bl) * x;
        x
    }
    /// Energy a mole of `k` gives its body's burnings at `t`, J: the best over `Consts::burn` (0 for a gas).
    pub(crate) fn heat_of(&self, chem: &Chemistry, k: usize, t: f64) -> f64 {
        if chem.phase(k, t) == Phase::Gas {
            return 0.0;
        }
        self.k.burn.iter().filter_map(|&(r, e)| chem.reactions[r].inputs.iter().find(|x| x.0 as usize == k).map(|x| e / x.1 as f64)).fold(0.0, f64::max)
    }
    /// Energy its burnings could release from its stores at `t`, J.
    pub fn fuel(&self, chem: &Chemistry, t: f64) -> f64 {
        let mut e = 0.0;
        for &(r, q) in &self.k.burn {
            let solid = chem.reactions[r].inputs.iter().filter(|p| chem.phase(p.0 as usize, t) != Phase::Gas);
            e += q * solid.map(|&(k, nu)| self.store[k as usize] / nu as f64).fold(f64::MAX, f64::min).min(1e30);
        }
        e
    }
    /// Does it burn substance `k` at `t`: is `k` an input, not a gas, of one of `Consts::burn`? Such a store is fuel.
    pub fn burns_at(&self, chem: &Chemistry, k: usize, t: f64) -> bool {
        self.k.fuel >> k & 1 == 1 && chem.phase(k, t) != Phase::Gas
    }
}
