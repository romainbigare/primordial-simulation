//! The dispersal kernel: where what disperses lands (children, `copy`; seeds, `cover`): a lot starts at a point drawn evenly over its source
//! cell and moves by an offset drawn from a kernel of two scales, a near one and a far one: most lands within tens of
//! metres, a long tail reaches km (Clark et al. 1999). Each scale is an isotropic Gaussian. The lot lands in the cell
//! that holds the point, so in expectation what lands in a cell is the kernel integrated over it from the source's
//! area, at any size of source and target, and the cost of a lot does not depend on the cells.

use crate::rng::Rng;
use crate::World;

/// The cell that one lot of what disperses from cell `i` lands in (maybe `i` itself); none where its point is off
/// the map, so it stays in `i`. The draws are from `r`, whose key the caller fixes by stable data.
pub fn lot(w: &World, i: usize, r: &mut Rng) -> Option<usize> {
    let (near, far, mix) = (w.cfg.phys.kernel_near, w.cfg.phys.kernel_far, w.cfg.phys.kernel_far_share);
    let leaf = w.map.leaves()[i];
    // The cell's side in leaves of the most depth, and its length east and south, m (on a globe its rows narrow).
    let (side, south) = (leaf.side() as f64, w.map.length(leaf.depth));
    let size = [w.mesh.cells().area()[i] / south, south];
    let (corner, scale) = (leaf.corner(), if r.f64() < mix { far } else { near });
    let start = [r.f64(), r.f64()];
    let at: [f64; 2] = std::array::from_fn(|a| corner[a] as f64 + side * (start[a] + scale * r.normal() / size[a]));
    w.mesh.tree().holding(at[0], at[1])
}

