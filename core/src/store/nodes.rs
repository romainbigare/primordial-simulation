//! The node table: each node of the world has a stable id, a class (which arrays of `World` hold its amounts and its
//! heat), the node it lies inside and its cell. The nodes of the mesh are fixed; parts come and go, so their slots are
//! reused with a new generation, and the table is saved with the world.

use crate::space::Mesh;
use crate::AIR;
use crate::life::free_place;
use serde::{Deserialize, Serialize};

/// The stable name of a node: its slot in the table and the generation of the node in that slot (a slot given to a new
/// node gets a new generation, so an old id never names the new node, as for bodies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId {
    pub slot: u32,
    pub gen: u32,
}

/// How a node is stored: which arrays hold its amounts (`[k * len + index]`, mol) and its heat (`[index]`, J).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Class {
    /// The loose ground of a cell and the heat of its active layer: `World::ground`, `World::e_ground`
    Ground,
    /// An air node: `World::air`, `World::e_air`
    Air,
    /// The store of the bed under a cell: `World::deep`, `World::e_deep`
    Deep,
    /// A node beyond the world's boundary (`Boundary`): `World::boundary`, `World::e_boundary`. It may hold less than
    /// nothing: what it holds is what crossed, with the sign of a node (the sun's heat falls by the light it gives).
    Boundary,
    /// The potential energy the world's matter has gained in the field of gravity since the start: one node, holding no
    /// amounts, whose heat is `World::potential`. Lifting matter moves heat into it and a fall takes it back; like a
    /// boundary node it gives all it is asked (it holds less than nothing when matter ends lower than it started).
    Potential,
    /// A part (`bond::Part`): its walls, fill and heat in `World::parts`, by its index. The only class whose nodes come
    /// and go, so it is the last: a new slot is a part's.
    #[default]
    Part,
}

/// The number of classes.
pub(crate) const CLASSES: usize = 6;

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
    pub(crate) const ALL: [Class; CLASSES] = [Class::Ground, Class::Air, Class::Deep, Class::Boundary, Class::Potential, Class::Part];
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
struct Node {
    gen: u32,
    class: Class,
    /// The node it lies inside; none for the nodes of a cell
    parent: Option<NodeId>,
    /// Its cell (an air cell for an air node; for a boundary node its index among them, `Boundary`)
    cell: u32,
}

/// Every node of the world, by slot.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Nodes {
    nodes: Vec<Node>,
    /// The slot of the node at index 0 of each class: a node's index in its class's arrays is its slot past this
    first: [u32; CLASSES],
    /// The free slots of parts, the last freed first
    free: Vec<u32>,
}

impl Nodes {
    /// The table of a world on `mesh`: its ground nodes, then its air nodes, then its deep stores, each in the order of
    /// its cells, then the boundary nodes and the potential; the parts come after them.
    pub fn of_mesh(mesh: &Mesh) -> Self {
        let mut t = Nodes::default();
        for (class, len) in [(Class::Ground, mesh.cells().n()), (Class::Air, mesh.level(AIR).n()), (Class::Deep, mesh.cells().n()), (Class::Boundary, BOUNDARIES), (Class::Potential, 1)] {
            t.first[class as usize] = t.nodes.len() as u32;
            t.nodes.extend((0..len as u32).map(|cell| Node { gen: 0, class, parent: None, cell }));
        }
        t.first[Class::Part as usize] = t.nodes.len() as u32;
        t
    }
    /// A new part in `cell`: in the slot freed last (its generation moved on when it was freed, so an old id names
    /// nothing), or in a new slot at the end.
    pub(crate) fn add_part(&mut self, cell: usize) -> NodeId {
        let slot = free_place(&mut self.nodes, &mut self.free);
        let n = &mut self.nodes[slot];
        (n.class, n.cell) = (Class::Part, cell as u32);
        NodeId { slot: slot as u32, gen: n.gen }
    }
    /// Part `id` is gone: its slot is free for a new part, of the next generation.
    pub(crate) fn remove_part(&mut self, id: NodeId) {
        assert_eq!(self.node(id).class, Class::Part, "only parts come and go");
        self.nodes[id.slot as usize].gen += 1;
        self.free.push(id.slot);
    }
    /// Node `id` now lies in `cell`.
    pub(crate) fn relocate(&mut self, id: NodeId, cell: usize) {
        self.node(id);
        self.nodes[id.slot as usize].cell = cell as u32;
    }
    /// The node of `class` at `index` of its arrays.
    pub fn id(&self, class: Class, index: usize) -> NodeId {
        let slot = self.first[class as usize] + index as u32;
        let n = &self.nodes[slot as usize];
        assert_eq!(n.class, class, "index {index} beyond the nodes of {class:?}");
        NodeId { slot, gen: n.gen }
    }
    /// Every slot's id, in slot order (a free slot's names no node).
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
        (n.class, (id.slot - self.first[n.class as usize]) as usize)
    }
    /// `place` of node `id`, if it still exists.
    pub fn find(&self, id: NodeId) -> Option<(Class, usize)> {
        self.nodes.get(id.slot as usize).is_some_and(|n| n.gen == id.gen).then(|| self.place(id))
    }
    pub fn cell(&self, id: NodeId) -> usize {
        self.node(id).cell as usize
    }
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).parent
    }
}
