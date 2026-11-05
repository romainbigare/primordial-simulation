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
pub(crate) use apply::{life_moved, life_went};
pub(crate) use apply::ROUNDING;
pub use links::Links;
pub use batch::Batch;
pub(crate) use book::{book_by_array, Booker};
pub use gather::Moves;
pub use moves::{held, held_more, LinkFlux, Move};
pub(crate) use moves::heat_received;
pub use nodes::{Boundary, Class, Node, BOUNDARIES};

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
    /// The nodes of each cell (`Class::CELLS`) follow their cells (`World::remap`): a new cell's hold nothing, those of
    /// a cell a split or a join made their share or their sum, by the records of the joins (`joined`, `Made::held`).
    pub(crate) fn remap(&mut self, r: &crate::remap::Renumber, joined: &[Option<crate::remap::Joined>]) {
        use crate::remap::Of;
        for c in Class::CELLS {
            let a = &mut self.amounts[c as usize];
            *a = r.apply_made(std::mem::take(a), || 0.0, |k, m, at| m.held(joined, Some(Of::Amount(c, k)), at));
            let h = &mut self.heat[c as usize];
            *h = r.apply_made(std::mem::take(h), || 0.0, |_, m, at| m.held(joined, Some(Of::Heat(c)), at));
        }
    }
}

impl World {
    /// `(k, mol)` of substances and `heat` J cross into node `to` from boundary node `b` (out where negative): an edit.
    /// A node of life (a body, a part, a cover) is named by its cell. The boundary node gives the potential energy the
    /// matter has where it lies (`placed`), against `Store::lifted`.
    pub fn cross(&mut self, (b, to): (Boundary, Node), mol: impl IntoIterator<Item = (usize, f64)>, heat: f64) {
        let from = Node::new(Class::Boundary, b as usize);
        let mut moves: Vec<Move> = mol.into_iter().map(|(k, m)| Move { from, to, k: k as u16, mol: m, heat: 0.0 }).collect();
        let lift = self.placed(to, &moves);
        moves.push(Move { from, to, k: 0, mol: 0.0, heat });
        apply(self, &[], &moves);
        fall_heat(self, Class::Boundary, [(b as usize, -lift)]);
    }
    /// What the potential energy of the world's matter gains when `moves` put their amounts into node `to`, J
    /// (`flow::potential`): a ground cell's nodes and its life change its stack and the air over it (`flow::gain`), an
    /// air node's matter lies at the mean head of its cells (`flow::air_head`).
    fn placed(&self, to: Node, moves: &[Move]) -> f64 {
        let kg: f64 = moves.iter().map(|m| m.mol * self.chem.subs[m.k as usize].molar_mass).sum();
        let d = |c: Class, k: usize| if c == to.class { moves.iter().filter(|m| m.k as usize == k).map(|m| m.mol).sum() } else { 0.0 };
        match to.class {
            Class::Ground | Class::Bed | Class::Deep | Class::Bodies => crate::flow::gain(self, to.index, d, 0.0),
            Class::Body | Class::Part | Class::Cover => crate::flow::gain(self, to.index, |_, _| 0.0, kg),
            Class::Air => kg * self.cfg.gravity * crate::flow::air_head(self, to.index),
            Class::Boundary => 0.0,
        }
    }
}
