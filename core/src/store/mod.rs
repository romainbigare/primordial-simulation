//! The store: the names of the world's nodes, what the fixed ones hold (`Store`, private arrays), and the one way the
//! laws change them (moves, transforms and link fluxes, booked by `apply`, their only writer), with the boundary nodes
//! that book what crosses the world's edge. It decides nothing physical: it adds what the laws give it and counts the
//! rounding it sets to zero (laws.md 0, "The books").

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

/// What the fixed nodes hold, by class (`Class::Ground` to `Class::Potential`): amounts `[k * len + index]`, mol, and
/// heat, J (the bed holds no heat, the potential no amounts). Only `apply` writes them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Store {
    amounts: Vec<Vec<f64>>,
    heat: Vec<Vec<f64>>,
    /// What rounding took from the amounts of the ground and of the air (`CARRIED`) as `apply` added to them, by class,
    /// `[k * len + index]`, mol: under half the last digit of each
    carry: Vec<Vec<f64>>,
}

/// The classes whose nodes may hold much more than a body moves (deep standing liquid, the air's main gases), whose
/// amounts `apply` sums with what rounding takes from them (`Store::carry`).
pub(crate) const CARRIED: usize = Class::Air as usize + 1;

impl Store {
    /// A store of `n` ground nodes (`ground`, `bed`, `e_ground`) and of air nodes (`air`, `e_air`); the other classes empty.
    pub fn new(n: usize, (ground, bed, e_ground): (Vec<f64>, Vec<f64>, Vec<f64>), (air, e_air): (Vec<f64>, Vec<f64>)) -> Store {
        let ns = ground.len() / n;
        let carry = vec![vec![0.0; ground.len()], vec![0.0; air.len()]];
        let amounts = vec![ground, air, vec![0.0; ns * n], bed, vec![0.0; ns * n], vec![0.0; ns * BOUNDARIES], Vec::new()];
        let heat = vec![e_ground, e_air, vec![0.0; n], Vec::new(), vec![0.0; n], vec![0.0; BOUNDARIES], vec![0.0]];
        Store { amounts, heat, carry }
    }
    /// The amounts of the nodes of `class`, `[k * len + index]`, mol (none for a class whose nodes hold none here).
    pub fn amounts(&self, class: Class) -> &[f64] {
        self.amounts.get(class as usize).map_or(&[], |v| v)
    }
    /// What rounding took from the amounts of the nodes of `class` (`Store::carry`), mol (none for a class not carried).
    pub fn carry(&self, class: Class) -> &[f64] {
        self.carry.get(class as usize).map_or(&[], |v| v)
    }
    /// The heat of each node of `class`, J (none for a class whose nodes hold none here).
    pub fn heat(&self, class: Class) -> &[f64] {
        self.heat.get(class as usize).map_or(&[], |v| v)
    }
}

impl World {
    /// `(k, mol)` of substances and `heat` J cross into node `to` from boundary node `b` (out where negative): an edit.
    pub fn cross(&mut self, (b, to): (Boundary, Node), mol: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
        let from = Node::new(Class::Boundary, b as usize);
        let mut moves: Vec<Move> = mol.into_iter().map(|(k, m)| Move { from, to, k: k as u16, mol: m, heat: 0.0 }).collect();
        moves.push(Move { from, to, k: 0, mol: 0.0, heat });
        apply(self, &[], &moves, &[]);
    }
}
