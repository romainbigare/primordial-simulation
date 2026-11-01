//! The store: the names of the world's nodes, what the fixed ones hold (`Store`, private arrays), and the one way the
//! laws change them (moves and link fluxes, booked by `apply`; the bodies' moves and Change's rows array
//! by array by `book_by_array`, which adds as `apply` adds), with the boundary nodes that book what crosses the world's
//! edge. It decides nothing physical: it adds what the laws give it and counts the rounding it sets to zero (laws.md 0,
//! "The books").

mod apply;
mod batch;
mod book;
mod gather;
pub mod links;
pub mod moves;
pub mod nodes;

pub use apply::{apply, fall_heat};
pub(crate) use apply::ROUNDING;
pub use links::Links;
pub use batch::Batch;
pub(crate) use book::{book_by_array, Booker};
pub use gather::Moves;
pub use moves::{held, held_more, LinkFlux, Move};
pub(crate) use moves::heat_received;
pub use nodes::{Boundary, Class, Node, NodeId, BOUNDARIES};

use crate::World;
use serde::{Deserialize, Serialize};

/// What the fixed nodes hold, by class (`Class::Ground` to `Class::Boundary`): amounts `[k * len + index]`, mol, and
/// heat, J (the bed holds no heat); and the potential energy the world's matter gained since the start, J: what its
/// lifts took from the heat of the nodes that paid them less what its falls gave back (`fall_heat`). Only `apply`,
/// `fall_heat` and `book_by_array` write them, and the bookings of Change's rows and of the bodies' moves add their
/// lifts to `lifted`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Store {
    amounts: Vec<Vec<f64>>,
    heat: Vec<Vec<f64>>,
    lifted: f64,
}

impl Store {
    /// A store of `n` ground nodes (`ground`, `bed`, `e_ground`) and of air nodes (`air`, `e_air`); the other classes empty.
    pub fn new(n: usize, (ground, bed, e_ground): (Vec<f64>, Vec<f64>, Vec<f64>), (air, e_air): (Vec<f64>, Vec<f64>)) -> Store {
        let ns = ground.len() / n;
        let amounts = vec![ground, air, vec![0.0; ns * n], bed, vec![0.0; ns * n], vec![0.0; ns * BOUNDARIES]];
        let heat = vec![e_ground, e_air, vec![0.0; n], Vec::new(), vec![0.0; n], vec![0.0; BOUNDARIES]];
        Store { amounts, heat, lifted: 0.0 }
    }
    /// The potential energy the world's matter gained since the start, J (`Store::lifted`).
    pub fn lifted(&self) -> f64 {
        self.lifted
    }
    /// A booking's lifts, `e` J, summed apart (less its falls), join the potential energy (`Store::lifted`).
    pub(crate) fn lift(&mut self, e: f64) {
        self.lifted += e;
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
    /// `(k, mol)` of substances and `heat` J cross into node `to` from boundary node `b` (out where negative): an edit.
    pub fn cross(&mut self, (b, to): (Boundary, Node), mol: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
        let from = Node::new(Class::Boundary, b as usize);
        let mut moves: Vec<Move> = mol.into_iter().map(|(k, m)| Move { from, to, k: k as u16, mol: m, heat: 0.0 }).collect();
        moves.push(Move { from, to, k: 0, mol: 0.0, heat });
        apply(self, &[], &moves);
    }
}
