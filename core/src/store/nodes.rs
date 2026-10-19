//! The names of the world's nodes. A node of the mesh (ground, air, deep), a boundary node and the potential are fixed:
//! each is named by its class and its index in that class's arrays (`Node`), so the names of one class do not move
//! when another changes. Only what comes and goes has a stable id of slot and generation (`NodeId`): a part (its slot
//! in `bond::Parts`); a body has its own (`Body::id`).

use serde::{Deserialize, Serialize};

/// The stable name of a part: its slot and the generation of the part in that slot (a slot given to a new part gets a
/// new generation, so an old id never names the new part, as for bodies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId {
    pub slot: u32,
    pub gen: u32,
}

/// How a node is stored: which arrays hold its amounts (`[k * len + index]`, mol) and its heat (`[index]`, J).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// The loose ground of a cell and the heat of its active layer
    Ground,
    /// An air node
    Air,
    /// The store of the bed under a cell: the liquid in it (with what is dissolved in it), and its heat with the fusion
    /// enthalpy of what is molten (`flow::drain`)
    Deep,
    /// The bonded ground under a cell: amounts only, as its skin's heat is its ground node's. It breaks into loose ground
    /// by a move from it (`Move::loosen`: wear by running liquid, digging).
    Bed,
    /// The bodies' node of a cell: the light and the heat set aside for its bodies between their updates, J (it holds no
    /// amounts). Flow brings the light their light reactions will store, the ground the heat they draw from it as time
    /// passes (`survey::survey`); each body takes its share at its update.
    Bodies,
    /// A node beyond the world's boundary (`Boundary`). It may hold less than nothing: what it holds is what crossed, with
    /// the sign of a node (the sun's heat falls by the light it gives).
    Boundary,
    /// The potential energy the world's matter has gained in the field of gravity since the start: one node, holding no
    /// amounts, whose heat is that energy. Lifting matter moves heat into it and a fall takes it back; like a boundary
    /// node it gives all it is asked (it holds less than nothing when matter ends lower than it started).
    Potential,
    /// A part (`bond::Part`): its walls, fill and heat in `World::parts`, at its slot. Its nodes come and go, named by a
    /// `NodeId`.
    Part,
    /// A body (`body::Body`): its stores and heat in the body, at its slot. Like a part, it holds its own amounts: the
    /// law that changes it books its side of a move, the store the other (`store::apply`).
    Body,
}

/// A fixed node: its class and its index in that class's arrays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node {
    pub class: Class,
    pub index: usize,
}

impl Node {
    pub fn new(class: Class, index: usize) -> Node {
        Node { class, index }
    }
    /// Ground node `i`.
    pub fn ground(i: usize) -> Node {
        Node::new(Class::Ground, i)
    }
    /// Air node `j`.
    pub fn air(j: usize) -> Node {
        Node::new(Class::Air, j)
    }
    /// The bodies' node of ground cell `i`.
    pub fn bodies(i: usize) -> Node {
        Node::new(Class::Bodies, i)
    }
}

/// The three nodes beyond the world's boundary, by their index among the `Class::Boundary` nodes: what the world
/// gains from them and gives them is all its energy and matter may change by (`check::drift`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// Gives the sunlight the world absorbs
    Sun,
    /// Takes the heat the world radiates away
    Space,
    /// Gives and takes what edits put in the world and take out
    Outside,
}

/// The number of boundary nodes.
pub const BOUNDARIES: usize = 3;

impl Class {
    /// Every class, by its index (`class as usize`).
    pub(crate) const ALL: [Class; 9] = [Class::Ground, Class::Air, Class::Deep, Class::Bed, Class::Bodies, Class::Boundary, Class::Potential, Class::Part, Class::Body];
}
