//! The names of the world's nodes: a fixed node (ground, air, deep, bed, what life sets aside, boundary) by its class and
//! index, and the cover of a cell (`Node`).

/// How a node is stored: which arrays hold its amounts (`[k * len + index]`, mol) and its heat (`[index]`, J).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// The loose ground of a cell and the heat of its active layer
    Ground,
    /// An air node
    Air,
    /// The store of the bed under a cell: its liquid and what is dissolved in it, and its heat (`flow::drain`)
    Deep,
    /// The bonded ground under a cell: amounts only (its skin's heat is its ground node's); wear breaks it
    /// into its ground node
    Bed,
    /// What a cell's life sets aside: the light its cover's capture area absorbs that is set aside for its light reaction
    /// until the cover's month takes it, J (`cover::release` gives the rest back to the ground before each air step),
    /// and the gas the cover gives off until the air's step takes it up (holding its latent heat as the air does)
    Aside,
    /// A node beyond the world's boundary (`Boundary`): it holds what crossed, with a node's sign, so it may go below zero
    Boundary,
    /// The life of a cell (`life::Cell`: its cover): it holds its own amounts and heat, and its laws book its side. Its
    /// index is its cell.
    Life,
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
    /// What the life of ground cell `i` sets aside (`Class::Aside`).
    pub fn aside(i: usize) -> Node {
        Node::new(Class::Aside, i)
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
    pub(crate) const ALL: [Class; 7] = [Class::Ground, Class::Air, Class::Deep, Class::Bed, Class::Aside, Class::Boundary, Class::Life];
}
