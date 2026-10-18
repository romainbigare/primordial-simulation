//! STORE (`docs/foundation/plan.md`, Ideas 1 and 2): the table of the world's nodes, and the one way the laws change
//! their amounts and heat (moves, transforms and link fluxes, booked by `apply`), with the three nodes beyond the
//! world's boundary that book what crosses it. The store decides nothing physical.

mod apply;
pub mod links;
pub mod moves;
pub mod nodes;

pub use links::Links;
pub use apply::{apply, Booking};
pub use moves::{held, LinkFlux, Move, Transform};
pub use nodes::{Boundary, Class, NodeId, Nodes, BOUNDARIES};

use crate::World;

impl World {
    /// Size the deep stores (empty until first used).
    pub(crate) fn fit_deep(&mut self) {
        let (n, ns) = (self.grid.n(), self.chem.ns());
        if self.deep.len() != ns * n {
            (self.deep, self.e_deep) = (vec![0.0; ns * n], vec![0.0; n]);
        }
    }
    /// `m` mol of substance `k` of the bonded ground under ground node `i` break into its loose ground (the heat of the
    /// bed's skin is the node's already). The bed is not a node of its own, so this is the one place its amounts change,
    /// until the ground of a cell becomes nodes of its own (patches, `docs/foundation/plan.md` F6).
    pub(crate) fn loosen(&mut self, k: usize, i: usize, m: f64) {
        let at = k * self.grid.n() + i;
        (self.bed[at], self.ground[at]) = (self.bed[at] - m, self.ground[at] + m);
    }
    /// `(k, mol)` of substances and `heat` J cross into the world from boundary node `b` (out to it where negative):
    /// the books of a law or an edit that changes the world without a `Move`. The caller puts them in the world; the
    /// node gives them as the ground holds them (`held`), so the world gains `heat` and the mol's formation enthalpy.
    pub fn cross(&mut self, b: Boundary, mol: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
        for (k, m) in mol {
            self.boundary[k * BOUNDARIES + b as usize] -= m;
        }
        self.e_boundary[b as usize] -= heat;
    }
}
