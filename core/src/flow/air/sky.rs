//! Gas crossing between the ground (or what lives on it) and the air: the energy of lifting it.

use crate::{World, AIR};

impl World {
    /// Energy to lift a kg from ground cell `i` into its air node, J/kg: from the ground's head to the air's mean height,
    /// paid by the air's heat (its expansion does the work), and given back to the air by a fall (laws.md 1.5).
    pub(crate) fn lift(&self, i: usize) -> f64 {
        self.scratch.view().lift(i, self.mesh.up(i, 0, AIR), self.cfg.gravity)
    }
}
