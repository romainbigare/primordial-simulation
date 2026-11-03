//! The brains of a cell's bodies, evaluated together (laws.md 5.1): every body with a brain perceives its inputs
//! after the physics of all the cell's bodies and before any of them acts (`Minds::sense`), and the bodies of one
//! lineage, whose base is one, find the sums of their hidden layers as one matrix product (`brain::sums`); each then
//! decides in the cell's order (`act`).

use super::perceive::inputs;
use crate::body::Lived;
use crate::brain;
use crate::life::{Chunk, Handle};
use crate::World;

/// A cell's bodies with a brain in the order they decide: each one's place, id and what its update left, and where
/// the sums of its hidden layer lie in `z`; and the working memory of `sense`.
#[derive(Default)]
pub struct Minds {
    pub bodies: Vec<(Handle, u64, Lived)>,
    at: Vec<usize>,
    z: Vec<f32>,
    /// Per body its lineage and its place in `bodies`, sorted by lineage; the inputs of one lineage's bodies one after
    /// another, and the sums of their hidden layers
    order: Vec<(u32, usize)>,
    x: Vec<f32>,
    one: Vec<f32>,
    sums: Vec<f32>,
}

impl Minds {
    /// No bodies yet.
    pub fn clear(&mut self) {
        self.bodies.clear();
    }
    /// Each body's inputs (`perceive::inputs`), gathered lineage by lineage, and the sums of the hidden layers of each
    /// lineage's bodies found together (`brain::sums`); each body's at its place in `z`.
    pub fn sense(&mut self, w: &World, d: &Chunk) {
        let Minds { bodies, at, z, order, x, one, sums } = self;
        order.clear();
        order.extend(bodies.iter().enumerate().map(|(k, &(h, _, _))| (d[h].lineage, k)));
        order.sort_by_key(|o| o.0);
        at.clear();
        at.resize(bodies.len(), 0);
        z.clear();
        for group in order.chunk_by(|a, b| a.0 == b.0) {
            let Some(lin) = d.brain(w, group[0].0) else { continue };
            let s = lin.shape;
            x.clear();
            for &(_, k) in group {
                let (h, _, lived) = &bodies[k];
                let b = &d[*h];
                inputs(w, d.at(h.cell as usize), b, (lived, b.temp(&w.chem), s.mem), one);
                x.extend_from_slice(one);
            }
            brain::sums(&lin.base, s, x, sums);
            for (q, &(_, k)) in group.iter().enumerate() {
                at[k] = z.len();
                z.extend_from_slice(&sums[q * s.h..(q + 1) * s.h]);
            }
        }
    }
    /// The sums of the hidden layer of the `k`-th body (`sense`), of `h` hidden units.
    pub fn of(&self, k: usize, h: usize) -> &[f32] {
        &self.z[self.at[k]..self.at[k] + h]
    }
}
