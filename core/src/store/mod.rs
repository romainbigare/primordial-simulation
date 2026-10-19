//! STORE (`docs/foundation/plan.md`, Ideas 1 and 2): the names of the world's nodes, what the fixed ones hold
//! (`Store`, whose arrays are private), and the one way the laws change their amounts and heat (moves, transforms and
//! link fluxes, booked by `apply`, the only writer), with the three nodes beyond the world's boundary that book what
//! crosses it. So the compiler keeps the contract between the layers (plan 5.2). The store decides nothing physical: it adds what the laws give it (a
//! law never asks a node for more than it holds) and counts the rounding it sets to zero.

mod apply;
mod batch;
mod gather;
pub mod links;
pub mod moves;
pub mod nodes;

pub use apply::apply;
pub(crate) use apply::ROUNDING;
pub use links::Links;
pub use batch::Batch;
pub use gather::Moves;
pub use moves::{held, held_more, LinkFlux, Move, Transform};
pub use nodes::{Boundary, Class, Node, NodeId, BOUNDARIES};

use crate::World;
use serde::{Deserialize, Serialize};

/// What the store's fixed nodes hold (each named by its class and index, `Class`), by class, for the classes `apply`
/// books (`Class::Ground` to `Class::Potential`): the amounts of the nodes of a class, `[k * len + index]`, mol, and the
/// heat of each, J (none for a class whose nodes hold none: the bed holds no heat, the bodies' node and the potential no
/// amounts). Only `apply` writes them; the laws read them here.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Store {
    amounts: Vec<Vec<f64>>,
    heat: Vec<Vec<f64>>,
}

impl Store {
    /// A store over `n` ground nodes of their loose amounts `ground`, bonded amounts `bed` and heat `e_ground`, and of air
    /// nodes of their amounts `air` and heat `e_air`, each of the same substances; its deep stores, bodies' nodes,
    /// boundary nodes and potential empty.
    pub fn new(n: usize, (ground, bed, e_ground): (Vec<f64>, Vec<f64>, Vec<f64>), (air, e_air): (Vec<f64>, Vec<f64>)) -> Store {
        let ns = ground.len() / n;
        let amounts = vec![ground, air, vec![0.0; ns * n], bed, Vec::new(), vec![0.0; ns * BOUNDARIES], Vec::new()];
        let heat = vec![e_ground, e_air, vec![0.0; n], Vec::new(), vec![0.0; n], vec![0.0; BOUNDARIES], vec![0.0]];
        Store { amounts, heat }
    }
    /// The amounts of the nodes of `class`, `[k * len + index]`, mol (none for a class whose nodes hold none here).
    pub fn amounts(&self, class: Class) -> &[f64] {
        self.amounts.get(class as usize).map_or(&[], |v| v)
    }
    /// The heat of each node of `class`, J (none for a class whose nodes hold none here).
    pub fn heat(&self, class: Class) -> &[f64] {
        self.heat.get(class as usize).map_or(&[], |v| v)
    }
}

impl World {
    /// `(k, mol)` of substances and `heat` J cross into node `to` from boundary node `b` (out to it where negative): an
    /// edit, booked as moves from the boundary node (`apply`; the caller books a body's or a part's own end).
    pub fn cross(&mut self, (b, to): (Boundary, Node), mol: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
        let from = Node::new(Class::Boundary, b as usize);
        let mut moves: Vec<Move> = mol.into_iter().map(|(k, m)| Move { from, to, k: k as u16, mol: m, heat: 0.0 }).collect();
        moves.push(Move { from, to, k: 0, mol: 0.0, heat });
        apply(self, &[], &moves, &[]);
    }
}
