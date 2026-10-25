//! Stacks: the nodes above and below each cell and the vertical links between them. Every cell has the same layers;
//! a layer's nodes live on one level of the mesh. Only geometry: which node is above which, through what area.

use super::Mesh;

/// The stacks of a mesh, and their vertical links.
#[derive(Clone, Debug)]
pub struct Stack {
    /// The first node of each layer (then the number of nodes)
    first: Vec<u32>,
    /// Each vertical link: the node below and above, and its area, m²
    lo: Vec<u32>,
    hi: Vec<u32>,
    area: Vec<f64>,
}

impl Stack {
    /// The stacks of layers whose nodes live on the levels `layers`, bottom to top: neighbouring layers link once per
    /// cell of the finer level, across its area.
    pub fn new(mesh: &Mesh, layers: &[usize]) -> Stack {
        assert!(layers.iter().all(|&l| l < mesh.levels()), "a layer lies on a level of the mesh");
        let mut first = vec![0u32];
        for &l in layers {
            first.push(first[first.len() - 1] + mesh.level(l).n() as u32);
        }
        let mut s = Stack { first, lo: vec![], hi: vec![], area: vec![] };
        for (k, pair) in layers.windows(2).enumerate() {
            let at = pair[0].min(pair[1]);
            for c in 0..mesh.level(at).n() {
                s.lo.push(s.node(k, mesh.up(c, at, pair[0])) as u32);
                s.hi.push(s.node(k + 1, mesh.up(c, at, pair[1])) as u32);
                s.area.push(mesh.level(at).area()[c]);
            }
        }
        s
    }
    /// Number of nodes in all the stacks.
    pub fn nodes(&self) -> usize {
        self.first[self.first.len() - 1] as usize
    }
    /// The node of layer `k` over cell `c` of that layer's level.
    pub fn node(&self, k: usize, c: usize) -> usize {
        self.first[k] as usize + c
    }
    super::views! { lo: u32, hi: u32, area: f64 }
}
