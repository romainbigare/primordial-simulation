//! Gas crossing between the ground (or what lives on it) and the air: the energy of lifting it.

use crate::{World, AIR};

impl World {
    /// Energy to lift a kg from ground cell `i` into its air node, J/kg (`FlowView::lift`). Gas going up is lifted from
    /// the head of the ground node it leaves to the mean height of its air node; the air pays the lift from its heat
    /// (its expansion does the work). Gas coming down falls the same height, and the fall heats the air it falls
    /// through. The world holds the difference as potential energy (the store's potential node), so a gas that goes up and
    /// comes down again leaves the energy as it was. Flow's exchange and Change book it as link fluxes with the
    /// potential's node (`store::Batch::falls`), bodies and parts as moves (`store::Moves::gas`).
    pub(crate) fn lift(&self, i: usize) -> f64 {
        self.scratch.view().lift(i, self.mesh.up(i, 0, AIR), self.cfg.gravity)
    }
}
