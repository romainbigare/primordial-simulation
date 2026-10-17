//! Stacks: the nodes above and below each cell (soil layers, a deep store, air layers), and the vertical links between
//! them. Every cell has the same layers, bottom to top; a layer's nodes live on one level of the mesh, so a coarse air
//! layer has one node over many cells. The stack numbers its nodes layer by layer (`node`), so its vertical links are
//! plain arrays like the mesh's links. Only geometry: which node is above which, through what area, how far apart.

use super::Mesh;

/// One layer of the stacks: the level of the mesh its nodes live on, and its thickness, m.
#[derive(Clone, Copy, Debug)]
pub struct Layer {
    pub level: usize,
    pub depth: f64,
}

/// The stacks of a mesh, and their vertical links.
#[derive(Clone, Debug)]
pub struct Stack {
    /// The level of each layer, and the first node of each layer (then the number of nodes)
    level: Vec<usize>,
    first: Vec<u32>,
    /// Each vertical link: the node below, the node above, the distance between the layers' middles, m, and the area
    /// it crosses, m²
    lo: Vec<u32>,
    hi: Vec<u32>,
    dist: Vec<f64>,
    area: Vec<f64>,
}

impl Stack {
    /// The stacks of `layers`, bottom to top. Two layers next to each other are linked once per cell of the finer of
    /// their two levels, across that cell's area: a cell under a coarse layer has its own link up to the node over it.
    pub fn new(mesh: &Mesh, layers: &[Layer]) -> Stack {
        assert!(layers.iter().all(|l| l.level < mesh.levels() && l.depth > 0.0), "a layer lies on a level of the mesh and has a thickness");
        let mut first = vec![0u32];
        for l in layers {
            first.push(first[first.len() - 1] + mesh.level(l.level).n() as u32);
        }
        let mut s = Stack { level: layers.iter().map(|l| l.level).collect(), first, lo: vec![], hi: vec![], dist: vec![], area: vec![] };
        for (k, pair) in layers.windows(2).enumerate() {
            let at = pair[0].level.min(pair[1].level);
            for c in 0..mesh.level(at).n() {
                s.lo.push(s.node(k, mesh.up(c, at, pair[0].level)) as u32);
                s.hi.push(s.node(k + 1, mesh.up(c, at, pair[1].level)) as u32);
                s.dist.push((pair[0].depth + pair[1].depth) / 2.0);
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
    /// The node of layer `k` over cell `i` of the finest level.
    pub fn over(&self, mesh: &Mesh, k: usize, i: usize) -> usize {
        self.node(k, mesh.up(i, 0, self.level[k]))
    }
    pub fn lo(&self) -> &[u32] {
        &self.lo
    }
    pub fn hi(&self) -> &[u32] {
        &self.hi
    }
    pub fn dist(&self) -> &[f64] {
        &self.dist
    }
    pub fn area(&self) -> &[f64] {
        &self.area
    }
}
