//! Gas crossing between the ground (or what lives on it) and the air: the one way in and out.

use crate::chem::Chemistry;
use crate::World;

/// What gas crossing into or out of the air changes: the air's amounts and heat, and the world's potential energy.
///
/// Gas going up is lifted from the head of the ground node it leaves to the mean height of its air node; the air pays
/// the lift from its heat (its expansion does the work). Gas coming down falls the same height, and the fall heats the
/// air it falls through. The world holds the difference as potential energy (`World::potential`), so a gas that goes
/// up and comes down again leaves the energy as it was. The gas of bodies (`body`), of parts (`bond`) and of growth
/// (`copy`) crosses so, until they move through the store; Flow's exchange and Change book the same as link fluxes
/// with the potential's node (`store::Batch::falls`). The caller moves the gas's own heat and latent enthalpy.
pub(crate) struct Sky<'a> {
    pub(crate) chem: &'a Chemistry,
    pub(crate) air: &'a mut [f64],
    pub(crate) e_air: &'a mut [f64],
    pub(crate) potential: &'a mut f64,
}

impl Sky<'_> {
    /// `m` mol of gas `k` go up into air node `j` (negative `m`: come down out of it) from a place `lift` J/kg below its
    /// mean height (`FlowView::lift`).
    #[inline]
    pub(crate) fn cross(&mut self, k: usize, j: usize, m: f64, lift: f64) {
        let na = self.e_air.len();
        let e = m * self.chem.subs[k].molar_mass * lift;
        self.air[k * na + j] += m;
        self.e_air[j] -= e;
        *self.potential += e;
    }
}

impl World {
    /// The air's side of the world, for gas crossing into or out of it.
    pub(crate) fn sky(&mut self) -> Sky<'_> {
        Sky { chem: &self.chem, air: &mut self.air, e_air: &mut self.e_air, potential: &mut self.potential }
    }
    /// Energy to lift a kg from ground cell `i` into its air node, J/kg (`FlowView::lift`).
    pub(crate) fn lift(&self, i: usize) -> f64 {
        self.scratch.view().lift(i, self.cfg.gravity)
    }
}
