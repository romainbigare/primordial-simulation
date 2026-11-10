//! Where liquid stands over the ground (laws.md 1.2): one level over floors of heights and areas (`level_of`), and each
//! cell's level-volume curve over its own ground (`Curve`), the terrain's worn samples inside it (`World::ground_in`)
//! about its mean surface. Liquid fills a cell's ground from
//! its lowest sample up, so a cell is wet where part of its ground lies under its level, and it passes its liquid on
//! from its lowest ground, the way the terrain inside it drains.

use super::Scratch;
use crate::World;
use std::borrow::Cow;

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
/// with its area, m² (their mean departure by area is none). And where its
/// samples meet: the departures of each two beside each other across a side, m, and a sample's mean side, m, so the
/// edge of the ground under a level is found at the samples (`shore`).
#[derive(Clone, Debug, Default)]
pub(super) struct Curve {
    floors: Vec<(f64, f64)>,
    sides: Vec<(f32, f32)>,
    side: f64,
}

impl Curve {
    /// A flat ground of `area` m², standing no terrain (one floor, so its levels are the flat column's exactly).
    pub(super) fn flat(area: f64) -> Curve {
        Curve { floors: vec![(0.0, area)], sides: Vec::new(), side: area.sqrt() }
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
    /// The share of the cell's area under the level at which `v` m³ stand (the ground they cover), 0..1: none for none.
    pub(super) fn wet(&self, v: f64) -> f64 {
        if v <= 0.0 {
            return 0.0;
        }
        let x = self.level(v);
        self.floors.iter().take_while(|f| f.0 < x).map(|f| f.1).sum::<f64>() / self.area()
    }
    /// The length of the edge of the ground under the level at which `v` m³ stand, m: a sample's side for each two
    /// samples beside each other of which one lies under it.
    pub(super) fn shore(&self, v: f64) -> f64 {
        let x = self.level(v);
        self.side * self.sides.iter().filter(|&&(a, b)| ((a as f64) < x) != ((b as f64) < x)).count() as f64
    }
    /// The first moment of the volume under the level `x` over the cell's mean surface about that surface, m⁴: Σ area ×
    /// (x - floor) × (x + floor) / 2 over the floors under it, so its centroid lies `moment / volume` over the surface.
    pub(super) fn moment(&self, x: f64) -> f64 {
        self.floors.iter().take_while(|f| f.0 < x).map(|&(d, a)| a * (x - d) * (x + d) / 2.0).sum()
    }
}

/// The one level at which `v` m³ stand over the cells `parts` together, each its curve and its mean surface, m.
pub(super) fn level_over(parts: &[(&Curve, f64)], v: f64) -> f64 {
    let mut floors: Vec<(f64, f64)> = parts.iter().flat_map(|&(c, z)| c.floors.iter().map(move |&(d, a)| (z + d, a))).collect();
    level_of(&mut floors, v)
}

/// The curve of cell `i` of `w`: the departures of the terrain's samples inside it from their mean.
fn of(w: &World, i: usize) -> Curve {
    let area = w.mesh.cells().area()[i];
    let g = w.ground_in(w.map.leaves()[i]);
    let mean = g.mean(&g.h);
    let mut floors: Vec<(f64, f64)> = g.h.iter().zip(&g.area).map(|(h, a)| (h - mean, a * area)).collect();
    let sides = g.sides().map(|(p, q)| ((g.h[p] - mean) as f32, (g.h[q] - mean) as f32)).collect();
    floors.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
    Curve { side: (area / floors.len() as f64).sqrt(), floors, sides }
}

impl World {
    /// The ground inside cell `i` (of a square or coarser) made level at its mean (`Terrain::level_under`): its liquid
    /// stands at one depth over it. Flow finds every cell's curve again at its next part.
    pub fn level_inside(&mut self, i: usize) {
        let lat = (self.cfg.lat_south, self.cfg.lat_north);
        self.terrain.level_under(self.terrain.seed, &self.map, self.map.leaves()[i], lat);
        self.scratch.curves.clear();
    }
}

/// The curve of cell `i`: that of `s` where it has them, else found (`of`).
pub(super) fn of_cell<'a>(w: &World, s: &'a Scratch, i: usize) -> Cow<'a, Curve> {
    match s.curves.get(i).filter(|_| s.curves.len() == w.mesh.cells().n()) {
        Some(c) => Cow::Borrowed(c),
        None => Cow::Owned(of(w, i)),
    }
}

/// Are Flow's curves missing?
pub(super) fn stale(w: &World, s: &Scratch) -> bool {
    s.curves.len() != w.mesh.cells().n()
}

/// The curve of each cell (`Scratch::curves`), found from the ground inside it (`of`).
pub(super) fn build(w: &World, s: &mut Scratch) {
    // (The terrain's samples found first, before the parallel section that reads them, `Terrain::samples`.)
    w.terrain_samples();
    s.curves = crate::par::map_n(w.map.leaves().len(), crate::par::UNCOUNTED, |i| of(w, i));
}
