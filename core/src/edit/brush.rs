//! A round brush over the map (handover.md 11): the terrain's samples it covers, the finest the ground is held at
//! (`SAMPLE_DEPTH`), and the cells under it.

use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;
use std::collections::{BTreeMap, BTreeSet};

/// What a brush adds at a distance `d` from its centre, in radii, as a share of what it adds there: (1 − d²)², a smooth
/// bump, flat at its edge.
pub fn falloff(d: f64) -> f64 {
    (1.0 - d * d).max(0.0).powi(2)
}

/// A round brush: its centre, in squares from the map's north-west corner (as `Edit::place`), and its radius, m.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brush {
    pub centre: [f64; 2],
    pub radius_m: f64,
}

/// A cell under a brush (`World::under`): the cell, the share of it the brush covers, and the place in it nearest the
/// brush's centre, in squares (the centre of its sample the brush covers nearest that centre).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Under {
    pub cell: usize,
    pub share: f64,
    pub place: [f64; 2],
}

/// A sample a brush covers (`World::covered`): its index among the terrain's samples, the cell it lies in, its centre
/// in squares, and how far that lies from the brush's centre, in radii.
pub(super) struct Covered {
    pub k: usize,
    pub cell: usize,
    pub place: [f64; 2],
    pub d: f64,
}

impl World {
    /// The terrain's samples the brush `b` covers, row by row (`Covered`; the samples `across` a row): those whose
    /// centres lie within its radius, measured in samples from north to south (`Square::length`), and the sample under
    /// its centre always, at none from it (a brush finer than a sample covers that sample).
    pub(super) fn covered(&self, b: Brush, across: usize) -> Vec<Covered> {
        let map = &self.map;
        let per = (1u64 << SAMPLE_DEPTH.saturating_sub(map.square_depth())) as f64;
        let r = b.radius_m / map.length(SAMPLE_DEPTH);
        let [cx, cy] = b.centre.map(|v| v * per);
        let span = |c: f64, n: usize| {
            let most = n * per as usize;
            (((c - r).floor().max(0.0) as usize).min(most - 1), ((c + r).ceil().max(0.0) as usize).min(most))
        };
        let ((x0, x1), (y0, y1)) = (span(cx, map.w), span(cy, map.h));
        let mid = (cx.floor(), cy.floor());
        let mut out = Vec::new();
        for sy in y0..y1.max(y0 + 1) {
            for sx in x0..x1.max(x0 + 1) {
                let (x, y) = (sx as f64 + 0.5, sy as f64 + 0.5);
                let d = (x - cx).hypot(y - cy);
                let centre = (sx as f64, sy as f64) == mid;
                if d > r && !centre {
                    continue;
                }
                let place = self.sample_place(sy * across + sx, across);
                out.push(Covered { k: sy * across + sx, cell: self.cell_under(place), place, d: if d > r { 0.0 } else { d / r.max(f64::MIN_POSITIVE) } });
            }
        }
        out
    }

    /// The centre of the terrain's sample `k` (of samples `across` a row), in squares from the map's north-west corner.
    pub(super) fn sample_place(&self, k: usize, across: usize) -> [f64; 2] {
        let per = (1u64 << SAMPLE_DEPTH.saturating_sub(self.map.square_depth())) as f64;
        [((k % across) as f64 + 0.5) / per, ((k / across) as f64 + 0.5) / per]
    }

    /// The cells under the brushes `brushes` (one, or the places of a stroke), in the order of the cells (`Under`): each
    /// cell's share is the share of its samples one of them covers (`covered`), its place the sample nearest the centre
    /// of one.
    pub fn under(&self, brushes: &[Brush]) -> Vec<Under> {
        let map = &self.map;
        let per = (1u64 << SAMPLE_DEPTH.saturating_sub(map.square_depth())) as f64;
        let across = self.terrain_samples().across;
        // Of each cell: the samples covered, and the place of the one nearest a centre with its distance.
        let mut cells: BTreeMap<usize, (BTreeSet<usize>, f64, [f64; 2])> = BTreeMap::new();
        for c in brushes.iter().flat_map(|&b| self.covered(b, across)) {
            let at = cells.entry(c.cell).or_insert((BTreeSet::new(), f64::INFINITY, c.place));
            at.0.insert(c.k);
            if c.d < at.1 {
                (at.1, at.2) = (c.d, c.place);
            }
        }
        cells.into_iter().map(|(cell, (ks, _, place))| Under { cell, share: (ks.len() as f64 / (map.squares(cell) * per * per)).min(1.0), place }).collect()
    }
}
