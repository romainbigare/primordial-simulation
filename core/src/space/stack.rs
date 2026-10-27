//! Stacks: the nodes above and below each cell. Every cell has the same layers; a layer's nodes live on one level of
//! the mesh. Only geometry: which node of a layer is over which cell.

use super::Mesh;

/// The stacks of a mesh: its layers' nodes numbered one after another.
#[derive(Clone, Debug)]
pub struct Stack {
    /// The first node of each layer (then the number of nodes)
    first: Vec<u32>,
}

impl Stack {
    /// The stacks of layers whose nodes live on the levels `layers`, bottom to top.
    pub fn new(mesh: &Mesh, layers: &[usize]) -> Stack {
        assert!(layers.iter().all(|&l| l < mesh.levels()), "a layer lies on a level of the mesh");
        let mut first = vec![0u32];
        for &l in layers {
            first.push(first[first.len() - 1] + mesh.level(l).n() as u32);
        }
        Stack { first }
    }
    /// Number of nodes in all the stacks.
    pub fn nodes(&self) -> usize {
        self.first[self.first.len() - 1] as usize
    }
    /// The node of layer `k` over cell `c` of that layer's level.
    pub fn node(&self, k: usize, c: usize) -> usize {
        self.first[k] as usize + c
    }
}
