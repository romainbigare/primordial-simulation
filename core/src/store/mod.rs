//! STORE (`docs/foundation/plan.md`, Ideas 1 and 2): the table of the world's nodes, and the one way the laws change
//! their amounts and heat (moves, transforms and link fluxes, booked by `apply`). The store decides nothing physical.

pub mod moves;
pub mod nodes;

pub use moves::{apply, held, LinkFlux, Move, Transform};
pub use nodes::{Class, NodeId, Nodes};

use crate::World;

impl World {
    /// Size the deep stores (empty until first used).
    pub(crate) fn fit_deep(&mut self) {
        let (n, ns) = (self.grid.n(), self.chem.ns());
        if self.deep.len() != ns * n {
            (self.deep, self.e_deep) = (vec![0.0; ns * n], vec![0.0; n]);
        }
    }
}
