//! Where liquid stands over the ground (laws.md 1.2): one level over floors of heights and areas (`level_of`), and each
//! cell's level-volume curve over its own ground (`Curve`), the terrain's drained samples inside it (`World::ground_in`)
//! at the scale its ground stands the terrain (`World::relief_scale`) about its mean surface. Liquid fills a cell's ground from
//! its lowest sample up, so a cell is wet where part of its ground lies under its level, and it passes its liquid on
//! from its lowest ground, the way the terrain inside it drains.

use super::Scratch;
use crate::space::Leaf;
use crate::World;
use std::collections::BTreeMap;

/// The level at which `vol` m³ fill floors of heights and areas `floors` (m, m²), sorted here: Σ area × (level -
/// floor) over the floors under it is `vol`, m.
pub fn level_of(floors: &mut [(f64, f64)], vol: f64) -> f64 {
    floors.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
    level_sorted(floors, vol)
}

/// `level_of` on floors sorted lowest first.
fn level_sorted(floors: &[(f64, f64)], vol: f64) -> f64 {
    let (mut area, mut volume_under) = (0.0, 0.0);
    for (k, &(z, a)) in floors.iter().enumerate() {
        (area, volume_under) = (area + a, volume_under + a * z);
        let level = (vol + volume_under) / area;
        if floors.get(k + 1).is_none_or(|next| level <= next.0) {
            return level;
        }
    }
    f64::NAN
}

/// The level-volume curve of a cell's ground: each sample's departure from the cell's mean surface, lowest first, m,
/// with its area, m² (their mean departure by area is none); and the scale of the terrain it stands.
#[derive(Clone, Debug, Default)]
pub(super) struct Curve {
    floors: Vec<(f64, f64)>,
    scale: f64,
}

impl Curve {
    /// A flat ground of `area` m², standing no terrain (one floor, so its levels are the flat column's exactly).
    pub(super) fn flat(area: f64) -> Curve {
        Curve { floors: vec![(0.0, area)], scale: 0.0 }
    }
    /// The departure of the cell's lowest ground from its mean surface, m: where its liquid leaves it.
    pub(super) fn low(&self) -> f64 {
        self.floors[0].0
    }
    /// The cell's area, m².
    pub(super) fn area(&self) -> f64 {
        self.floors.iter().map(|f| f.1).sum()
    }
    /// The volume that stands under the level `x` over the cell's mean surface, m³.
    pub(super) fn volume(&self, x: f64) -> f64 {
        self.floors.iter().map(|&(d, a)| a * (x - d).max(0.0)).sum()
    }
    /// The level over the cell's mean surface at which `v` m³ stand, m; its lowest ground for none.
    pub(super) fn level(&self, v: f64) -> f64 {
        if v > 0.0 { level_sorted(&self.floors, v) } else { self.low() }
    }
}

/// The one level at which `v` m³ stand over the cells `parts` together, each its curve and its mean surface, m.
pub(super) fn level_over(parts: &[(&Curve, f64)], v: f64) -> f64 {
    let mut floors: Vec<(f64, f64)> = parts.iter().flat_map(|&(c, z)| c.floors.iter().map(move |&(d, a)| (z + d, a))).collect();
    level_of(&mut floors, v)
}

/// The curve of cell `i` of `w`, at the scale of the terrain its ground stands.
fn of(w: &World, i: usize) -> Curve {
    at_scale(w, i, w.relief_scale[i])
}

/// The curve of cell `i` of `w` standing the terrain inside it at `scale`.
pub(super) fn at_scale(w: &World, i: usize, scale: f64) -> Curve {
    let area = w.mesh.cells().area()[i];
    if scale == 0.0 {
        return Curve::flat(area);
    }
    let g = w.ground_in(w.map.leaves()[i]);
    let mean = g.mean(&g.h);
    let mut floors: Vec<(f64, f64)> = g.h.iter().zip(&g.area).map(|(h, a)| (scale * (h - mean), a * area)).collect();
    floors.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
    Curve { floors, scale }
}

/// The curve of cell `i`: Flow's where it has them, else found (`of`).
pub(super) fn curve_of(w: &World, i: usize) -> Curve {
    w.scratch.curves.get(i).filter(|c| w.scratch.curves.len() == w.mesh.cells().n() && c.scale == w.relief_scale[i]).cloned().unwrap_or_else(|| of(w, i))
}

/// The curves found so far, by leaf (kept through remaps).
pub(super) type Kept = BTreeMap<Leaf, Curve>;

/// Are Flow's curves missing, or found for a scale a cell no longer stands?
pub(super) fn stale(w: &World, s: &Scratch) -> bool {
    s.curves.len() != w.mesh.cells().n() || s.curves.iter().zip(&w.relief_scale).any(|(c, &r)| c.scale != r)
}

/// The curve of each cell (`Scratch::curves`), from those kept where the leaf and its scale are the same; the others
/// found from the ground inside the cell (`at_scale`).
pub(super) fn build(w: &World, s: &mut Scratch) {
    let leaves = w.map.leaves();
    let mut kept = std::mem::take(&mut s.kept);
    s.curves = crate::par::map_n(leaves.len(), crate::par::UNCOUNTED, |i| match kept.get(&leaves[i]).filter(|c| c.scale == w.relief_scale[i]) {
        Some(c) => c.clone(),
        None => of(w, i),
    });
    kept.clear();
    leaves.iter().zip(&s.curves).for_each(|(&l, c)| _ = kept.insert(l, c.clone()));
    s.kept = kept;
}
