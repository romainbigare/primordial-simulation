//! The node table: each node of the world has a stable id, a class (which arrays of `World` hold its amounts and its
//! heat), the node it lies inside and its cell.

use crate::grid::Grid;
use serde::{Deserialize, Serialize};

/// The stable name of a node: its slot in the table and the generation of the node in that slot (a slot given to a new
/// node gets a new generation, so an old id never names the new node, as for bodies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId {
    pub slot: u32,
    pub gen: u32,
}

/// How a node is stored: which arrays hold its amounts (`[k * len + index]`, mol) and its heat (`[index]`, J).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// The loose ground of a cell and the heat of its active layer: `World::ground`, `World::e_ground`
    Ground,
    /// An air node: `World::air`, `World::e_air`
    Air,
    /// The store of the bed under a cell: `World::deep`, `World::e_deep`
    Deep,
}

/// The number of classes.
pub(crate) const CLASSES: usize = 3;

#[derive(Clone, Copy, Debug)]
struct Node {
    gen: u32,
    class: Class,
    /// The node it lies inside; none for the nodes of a cell
    parent: Option<NodeId>,
    /// Its cell (an air cell for an air node); for the nodes of a cell also the index of its amounts and heat
    cell: u32,
}

/// Every node of the world, by slot.
#[derive(Clone, Debug, Default)]
pub struct Nodes {
    nodes: Vec<Node>,
    /// The slot of the node at index 0 of each class
    first: [u32; CLASSES],
}

impl Nodes {
    /// The table of a world on `grid`: its ground nodes, then its air nodes, then its deep stores, each in the order of
    /// its cells. It follows from the grid's sizes alone, so a loaded world rebuilds it with the same ids.
    pub fn of_grid(grid: &Grid) -> Self {
        let mut t = Nodes::default();
        for (class, len) in [(Class::Ground, grid.n()), (Class::Air, grid.na()), (Class::Deep, grid.n())] {
            t.first[class as usize] = t.nodes.len() as u32;
            t.nodes.extend((0..len as u32).map(|cell| Node { gen: 0, class, parent: None, cell }));
        }
        t
    }
    /// The node of `class` at `index` of its arrays.
    pub fn id(&self, class: Class, index: usize) -> NodeId {
        let slot = self.first[class as usize] + index as u32;
        let n = &self.nodes[slot as usize];
        assert_eq!(n.class, class, "index {index} beyond the nodes of {class:?}");
        NodeId { slot, gen: n.gen }
    }
    /// Every node, in slot order.
    pub fn ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().enumerate().map(|(s, n)| NodeId { slot: s as u32, gen: n.gen })
    }
    fn node(&self, id: NodeId) -> &Node {
        let n = &self.nodes[id.slot as usize];
        assert_eq!(n.gen, id.gen, "node {id:?} is gone");
        n
    }
    /// Where the amounts and heat of node `id` lie: its class, and its index in that class's arrays.
    pub fn place(&self, id: NodeId) -> (Class, usize) {
        let n = self.node(id);
        (n.class, n.cell as usize)
    }
    pub fn cell(&self, id: NodeId) -> usize {
        self.node(id).cell as usize
    }
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).parent
    }
}
