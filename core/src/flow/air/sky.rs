//! Gas crossing between the ground (or what lives on it) and the air: the energy of lifting it.

use crate::{World, AIR};

impl World {
    /// Energy to lift a kg from ground cell `i` into its air node, J/kg: from the ground's head to the air node's mean
    /// ground, paid by the air's heat, and given back by a fall (laws.md 0, "The books").
    pub(crate) fn lift(&self, i: usize) -> f64 {
        self.scratch.view().lift(i, self.mesh.up(i, 0, AIR), self.cfg.gravity)
    }
}
