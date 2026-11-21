//! Where each patch of a cell's cover lies on its ground (laws.md 3, `cover::Patch`): the laws hold a patch as a share
//! of its cell, and a patch past the first keeps one point, the sample where it was made (`Patch::at`). Its ground is
//! the share of the cell's samples nearest that point, by distance over the ground, those of the patches before it
//! taken out first, so no two overlap; the rest of the cell is the first patch. Nothing of it is stored: every reader
//! (the tape, a shell's pictures) finds it here.

use crate::cover::PATCHES;
use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;

/// The ground of each patch of a cell's cover (`patch_ground`).
#[derive(Clone, Debug, PartialEq)]
pub struct PatchGround {
    /// The cell's edges in squares from the map's north-west corner: west, north, east, south (`Square::bounds`).
    pub bounds: [f64; 4],
    /// Its samples along a side: they are `side` x `side`, row by row from its north-west corner (`Leaf::split`).
    pub side: usize,
    /// The patch of each of its samples.
    pub of: Vec<u8>,
    /// Of each patch past the first in use, its point (the centre of its sample, m east and south of the cell's
    /// north-west corner) and its reach, m: the distance from its point halfway between its farthest sample and the
    /// nearest it leaves, so a circle of that radius about the point, inside the cell and out of the patches before it,
    /// holds its samples' centres and no other's (a sample's diagonal past its farthest where it took all that were
    /// left). The first patch and those not in use: none (a reach of 0).
    pub reach: [([f64; 2], f64); PATCHES],
}

impl PatchGround {
    /// The patch at the place `[x, y]` in squares from the map's north-west corner: that of the cell's sample under it
    /// (the nearest inside the cell).
    pub fn patch_at(&self, place: [f64; 2]) -> usize {
        self.of[sample_under(self.bounds, self.side, place)] as usize
    }
}

/// The samples along a side of cell `i` of `w` (`PatchGround::side`), and their depth: the terrain's samples, or the
/// cell itself where it is finer than they are.
fn sides(w: &World, i: usize) -> (usize, u8) {
    let l = w.map.leaves()[i];
    let depth = SAMPLE_DEPTH.max(l.depth);
    (1 << (depth - l.depth), depth)
}

/// The sample, row by row in a cell of edges `bounds` (squares) and `side` samples a side, under the place `[x, y]` in
/// squares: the nearest inside the cell.
fn sample_under(bounds: [f64; 4], side: usize, [x, y]: [f64; 2]) -> usize {
    let at = |v: f64, from: f64, to: f64| (((v - from) / (to - from) * side as f64).floor().max(0.0) as usize).min(side - 1);
    at(y, bounds[1], bounds[3]) * side + at(x, bounds[0], bounds[2])
}

impl World {
    /// The sample of cell `i` under the place `[x, y]` in squares from the map's north-west corner, row by row from the
    /// cell's north-west corner (`PatchGround`): the nearest inside the cell.
    pub fn sample_in(&self, i: usize, place: [f64; 2]) -> usize {
        sample_under(self.map.bounds(i), sides(self, i).0, place)
    }
    /// The sample at the centre of cell `i` (`sample_in`).
    pub fn centre_sample(&self, i: usize) -> usize {
        let [w0, n0, e0, s0] = self.map.bounds(i);
        self.sample_in(i, [(w0 + e0) / 2.0, (n0 + s0) / 2.0])
    }
}

/// The ground of each patch of the cover of cell `i` of `w` (`PatchGround`): each patch past the first in turn takes
/// the samples not yet taken nearest its point (`Patch::at`), by the distance between their centres in metres, ties by
/// their order, as many as its share of the cell's samples (rounded, at least one while any is left); what no patch
/// took is the first patch's.
pub fn patch_ground(w: &World, i: usize) -> PatchGround {
    let (side, depth) = sides(w, i);
    let (c, l) = (w.life.cover(i), w.map.leaves()[i]);
    let n = side * side;
    let mut out = PatchGround { bounds: w.map.bounds(i), side, of: vec![0; n], reach: [([0.0; 2], 0.0); PATCHES] };
    if c.patch_count() < 2 {
        return out;
    }
    // Each sample's centre, m east and south of the cell's north-west corner: its row's width from its area.
    let dy = w.map.length(depth);
    let samples: Vec<_> = l.split(depth).collect();
    let dx: Vec<f64> = (0..side).map(|r| w.map.place_of(&samples[r * side], w.cfg.lat_south, w.cfg.lat_north).0 / dy).collect();
    let centre = |p: usize| [(p % side) as f64 * dx[p / side] + dx[p / side] / 2.0, (p / side) as f64 * dy + dy / 2.0];
    for k in 1..c.patch_count() {
        let point = centre((c.patches[k].at as usize).min(n - 1));
        let far = |p: usize| {
            let q = centre(p);
            (q[0] - point[0]).hypot(q[1] - point[1])
        };
        let mut open: Vec<(f64, usize)> = (0..n).filter(|&p| out.of[p] == 0).map(|p| (far(p), p)).collect();
        open.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let take = ((c.patches[k].share * n as f64).round() as usize).max(1).min(open.len());
        open[..take].iter().for_each(|&(_, p)| out.of[p] = k as u8);
        let reach = match (take.checked_sub(1).map(|t| open[t].0), open.get(take)) {
            (Some(last), Some(next)) => (last + next.0) / 2.0,
            (Some(last), None) => last + dy.hypot(dx[0]),
            (None, _) => 0.0,
        };
        out.reach[k] = (point, reach);
    }
    out
}
