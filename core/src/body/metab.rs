//! The body's chemistry: the pace of its temperature, its catalysts, digestion and the fuel it burns.

use super::Body;
use crate::chem::{Chemistry, Phase};

impl Body {
    /// The pace of its life at `t`: the bare rate law of its main burning, k exp(-Ea/t), over its value at `t_cat` (1 for
    /// a body that burns nothing). Upkeep and what it takes and builds go by it, so seasons come from temperature
    /// (Gillooly et al. 2001).
    pub fn pace(&self, chem: &Chemistry, t: f64) -> f64 {
        self.k.burn.first().map_or(1.0, |&(r, _)| chem.reactions[r].arrhenius(t.max(1.0)) / self.k.pace_ref)
    }
    /// The share of its full force and speed its muscles give at `t`: its pace, at most 1. Muscle's power falls with
    /// its temperature by the rate law of what powers it (a Q10 of 2 to 3; Bennett 1990), so a cold body is weak and
    /// slow; at and above `t_cat`, where the force and the walk are anchored, it has all of them (a broad optimum).
    pub fn vigour(&self, chem: &Chemistry, t: f64) -> f64 {
        self.pace(chem, t).min(1.0)
    }
    /// Digestion: `taken` mol of body matter from outside are broken back into what growth made it of as they come in,
    /// so what it eats enters as fuel, never as body; the other inputs come from its stores. Returns the units reversed.
    pub fn digest(&mut self, chem: &Chemistry, taken: f64, rounded: &mut f64) -> f64 {
        let Some(bl) = chem.build.as_ref() else { return 0.0 };
        let bc = chem.body();
        let at = |k: u16| bc.place(k as usize).expect("growth's substances are stores");
        let m = at(bl.outputs[0].0);
        let most = (taken / bl.outputs[0].1 as f64).min(self.store[m] / bl.outputs[0].1 as f64);
        let x = bl.outputs[1..].iter().fold(most, |x, &(k, nu)| x.min(self.store[at(k)] / nu as f64));
        if x <= 0.0 {
            return 0.0;
        }
        for &(k, nu) in &bl.outputs {
            super::add_store(&mut self.store[at(k)], -(nu as f64) * x, rounded);
        }
        for &(k, nu) in &bl.inputs {
            self.store[at(k)] += nu as f64 * x;
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
        let (mut e, bc) = (0.0, chem.body());
        for &(r, q) in self.k.burn.iter() {
            let solid = chem.reactions[r].inputs.iter().filter(|p| chem.phase(p.0 as usize, t) != Phase::Gas);
            e += q * solid.map(|&(k, nu)| self.amount(bc, k as usize) / nu as f64).fold(f64::MAX, f64::min).min(1e30);
        }
        e
    }
    /// Does it burn substance `k` at `t`: is `k` an input, not a gas, of one of `Consts::burn`? Such a store is fuel.
    pub fn burns_at(&self, chem: &Chemistry, k: usize, t: f64) -> bool {
        self.k.fuel >> k & 1 == 1 && chem.phase(k, t) != Phase::Gas
    }
}
