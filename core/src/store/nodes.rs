//! The names of the world's nodes: a fixed node (ground, air, deep, bed, bodies, boundary) by its class and index
//! (`Node`); what comes and goes by slot and generation (`NodeId` for a part; a body has `Body::id`).

use serde::{Deserialize, Serialize};

/// The stable name of a part: its slot and generation (a reused slot gets a new generation, so an old id names nothing).
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
    /// The store of the bed under a cell: its liquid and what is dissolved in it, and its heat (`flow::drain`)
    Deep,
    /// The bonded ground under a cell: amounts only (its skin's heat is its ground node's); wear and digging break it
    /// into its ground node
    Bed,
    /// The bodies' node of a cell: the light and heat set aside for its bodies between their updates, J, and the gas its
    /// bodies and parts give off until the air's step takes it up (holding its latent heat as the air does)
    Bodies,
    /// A node beyond the world's boundary (`Boundary`): it holds what crossed, with a node's sign, so it may go below zero
    Boundary,
    /// A part (`bond::Part`), at its slot in `World::parts`, named by a `NodeId`
    Part,
    /// A body (`body::Body`): like a part it holds its own amounts, and its law books its side of a move. Its index is
    /// its place in its cell (`life::Handle`), a name for the length of its update only: `apply` never reads a body's
    /// side, and a body may be stored elsewhere after its update.
    Body,
    /// The cover of a cell (`cover::Cover`): like a body it holds its own amounts and heat, and its law books its side.
    /// Its index is its cell.
    Cover,
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

/// The three nodes beyond the world's boundary: what the world gains from and gives them is all its energy and matter
/// may change by (`check::drift`).
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
    pub(crate) const ALL: [Class; 9] = [Class::Ground, Class::Air, Class::Deep, Class::Bed, Class::Bodies, Class::Boundary, Class::Part, Class::Body, Class::Cover];
}
