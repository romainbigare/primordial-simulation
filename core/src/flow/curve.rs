//! Where liquid stands over the ground (laws.md 1.2): each cell's ground at the terrain's worn samples inside it
//! (`Curve`), about its mean surface. A cell's samples that lie in
//! a pond of the terrain (`Samples::ponds`) are its parts of the bodies of standing liquid, one part for each pond, and
//! liquid stands over each part at its body's level; a cell that holds no pond's samples holds liquid only until the
//! runoff's pass, from its lowest sample up.

use super::{level::level_of, Scratch};
use crate::space::{network::NONE, terrain::SAMPLE_DEPTH};
use crate::World;
use std::borrow::Cow;

/// No part: a sample of a cell that lies in no pond (`Curve::sides`).
const NO_PART: u8 = u8::MAX;

/// One part of a cell's ground where liquid stands at one level: its samples of one pond (`pond`), or all its samples
/// where it holds no pond's (`NONE`): each sample's departure from the cell's mean surface, lowest first, m, with its
/// area, m²; and of the floors under each, their area, m², and the sums of area × departure, m³, and of area ×
/// departure², m⁴ (one more than the floors: all of them last).
#[derive(Clone, Debug, Default)]
pub(super) struct Part {
    pub(super) pond: u32,
    floors: Vec<(f64, f64)>,
    sums: Vec<[f64; 3]>,
}

impl Part {
    /// The part of `floors`, sorted here, of pond `pond`.
    fn new(pond: u32, mut floors: Vec<(f64, f64)>) -> Part {
        floors.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        let mut sums = vec![[0.0; 3]];
        for &(d, a) in &floors {
            let s = sums[sums.len() - 1];
            sums.push([s[0] + a, s[1] + a * d, s[2] + a * d * d]);
        }
        Part { pond, floors, sums }
    }
    /// Its floors, lowest first: each sample's departure from the cell's mean surface, m, with its area, m².
    pub(super) fn floors(&self) -> &[(f64, f64)] {
        &self.floors
    }
    /// The sums of the floors under the level `x` over the cell's mean surface (`Part::sums`).
    fn under_x(&self, x: f64) -> [f64; 3] {
        self.sums[self.floors.partition_point(|f| f.0 < x)]
    }
    /// The volume that stands on it under the level `x` over the cell's mean surface, m³.
    pub(super) fn volume(&self, x: f64) -> f64 {
        let s = self.under_x(x);
        x * s[0] - s[1]
    }
    /// The level over the cell's mean surface at which `v` m³ stand on it, m; its lowest sample for none: over the
    /// floors under which less than `v` stands.
    pub(super) fn level(&self, v: f64) -> f64 {
        if v <= 0.0 {
            return self.floors[0].0;
        }
        let (mut k, mut over) = (1, self.floors.len());
        while k < over {
            let mid = (k + over) / 2;
            if self.floors[mid].0 * self.sums[mid][0] - self.sums[mid][1] < v {
                k = mid + 1;
            } else {
                over = mid;
            }
        }
        (v + self.sums[k][1]) / self.sums[k][0]
    }
    /// Its area under the level `x`, m².
    pub(super) fn under(&self, x: f64) -> f64 {
        self.under_x(x)[0]
    }
    /// The first moment about the cell's mean surface of what stands on it under the level `x`, m⁴: Σ area × (x² -
    /// departure²) / 2 over the floors under it.
    fn moment(&self, x: f64) -> f64 {
        let s = self.under_x(x);
        (x * x * s[0] - s[2]) / 2.0
    }
    /// Its area, m².
    fn area(&self) -> f64 {
        self.sums[self.sums.len() - 1][0]
    }
    /// Its lowest floor over a cell whose mean surface lies at `z`, m.
    pub(super) fn lowest(&self, z: f64) -> f64 {
        z + self.floors[0].0
    }
    /// Its lowest and highest floors over a cell whose mean surface lies at `z`, m.
    pub(super) fn span(&self, z: f64) -> [f64; 2] {
        [z + self.floors[0].0, z + self.floors[self.floors.len() - 1].0]
    }
}

/// The ground of a cell as liquid stands on it: its parts (`Part`; the ponds' in the order of the ponds), its area, m²,
/// the area of its samples in no part, m², and its lowest sample's departure from its mean surface, m.
/// And where its samples meet (`Sides`), and a sample's mean side, m, so the edge of the ground under the liquid is
/// found at the samples (`Fill::shore`).
#[derive(Clone, Debug, Default)]
pub(super) struct Curve {
    pub(super) parts: Vec<Part>,
    area: f64,
    apart: f64,
    low: f64,
    sides: Sides,
    side: f64,
}

/// Where a cell's samples meet: each two beside each other across a side, by the parts they lie in, with their
/// departures from the cell's mean surface, m, each sorted, so a count under a level is a search: of two in one part,
/// the lower and the higher of the two (`low[p]`, `high[p]`); of two of which one lies in no part, the other's
/// (`lone[p]`); two in two parts as they are, each with its part (`two`). Two in no part never meet the liquid's edge.
#[derive(Clone, Debug, Default)]
struct Sides {
    low: Vec<Vec<f32>>,
    high: Vec<Vec<f32>>,
    lone: Vec<Vec<f32>>,
    two: Vec<[(f32, u8); 2]>,
}

impl Sides {
    /// The sides of the samples `pairs`, each with its departure and its part (`NO_PART`: none), of a cell of `parts`
    /// parts.
    fn new(pairs: impl Iterator<Item = [(f32, u8); 2]>, parts: usize) -> Sides {
        let mut s = Sides { low: vec![Vec::new(); parts], high: vec![Vec::new(); parts], lone: vec![Vec::new(); parts], two: Vec::new() };
        for [(d, p), (e, q)] in pairs {
            match (p, q) {
                (NO_PART, NO_PART) => (),
                (NO_PART, r) | (r, NO_PART) => s.lone[r as usize].push(if p == NO_PART { e } else { d }),
                _ if p == q => {
                    s.low[p as usize].push(d.min(e));
                    s.high[p as usize].push(d.max(e));
                }
                _ => s.two.push([(d, p), (e, q)]),
            }
        }
        s.low.iter_mut().chain(&mut s.high).chain(&mut s.lone).for_each(|v| v.sort_unstable_by(f32::total_cmp));
        s
    }
    /// How many sides have one of their two samples under its part's level `x[p]` and the other not.
    fn across(&self, x: &[f64]) -> usize {
        let under = |v: &[f32], x: f64| v.partition_point(|&d| (d as f64) < x);
        let one: usize = x.iter().enumerate().map(|(p, &x)| under(&self.low[p], x) - under(&self.high[p], x) + under(&self.lone[p], x)).sum();
        let at = |(d, p): (f32, u8)| (d as f64) < x[p as usize];
        one + self.two.iter().filter(|s| at(s[0]) != at(s[1])).count()
    }
}

impl Curve {
    /// The cell's area, m².
    pub(super) fn area(&self) -> f64 {
        self.area
    }
    /// The parts of the bodies of standing liquid (a pond's samples in the cell), none where it holds none.
    pub(super) fn ponds(&self) -> &[Part] {
        if self.parts[0].pond == NONE { &[] } else { &self.parts }
    }
}

/// A cell's ground (`Curve`) and what its parts of the bodies held as the last runoff's pass left them (`State::held`,
/// m³; empty before the first): `v` m³ that stand over its pores lie on its parts by those shares, or, where they held
/// none, at one level over them all.
#[derive(Clone, Copy)]
pub(super) struct Fill<'a> {
    c: &'a Curve,
    held: &'a [f64],
}

impl Fill<'_> {
    /// What its parts of the bodies held at the last pass, m³.
    pub(super) fn held(&self) -> f64 {
        self.held.iter().sum()
    }
    /// The volume under the level `x` over the cell's mean surface on all its parts, m³.
    pub(super) fn volume(&self, x: f64) -> f64 {
        self.c.parts.iter().map(|p| p.volume(x)).sum()
    }
    /// The level of each part over the cell's mean surface where `v` m³ stand (`shares`), m.
    fn levels(&self, v: f64) -> Vec<f64> {
        self.c.parts.iter().zip(self.shares(v)).map(|(p, u)| p.level(u)).collect()
    }
    /// What each part holds where `v` m³ stand, m³: of one part all; of more, each its share by what it held, or, where
    /// they held none, what one level over them all gives each.
    pub(super) fn shares(&self, v: f64) -> Vec<f64> {
        let parts = &self.c.parts;
        let all: f64 = self.held.iter().sum();
        if parts.len() == 1 {
            return vec![v];
        }
        if all > 0.0 {
            return self.held.iter().map(|h| v * h / all).collect();
        }
        let mut floors: Vec<(f64, f64)> = parts.iter().flat_map(|p| p.floors.iter().copied()).collect();
        let x = if v > 0.0 { level_of(&mut floors, v) } else { self.c.low };
        parts.iter().map(|p| p.volume(x)).collect()
    }
    /// `f` of each part at its level where `v` m³ stand (`levels`), summed.
    fn sum(&self, v: f64, f: impl Fn(&Part, f64) -> f64) -> f64 {
        match &self.c.parts[..] {
            [p] => f(p, p.level(v)),
            parts => parts.iter().zip(self.levels(v)).map(|(p, x)| f(p, x)).sum(),
        }
    }
    /// The level over the cell's mean surface at which `v` m³ stand, m: its lowest sample for none, and of a cell of two
    /// parts or more their levels' mean by the volume each holds.
    pub(super) fn level(&self, v: f64) -> f64 {
        match &self.c.parts[..] {
            _ if v <= 0.0 => self.c.low,
            [p] => p.level(v),
            _ => self.sum(v, |p, x| p.volume(x) * x) / self.sum(v, Part::volume),
        }
    }
    /// The share of the cell's area under the liquid where `v` m³ stand (the ground they cover), 0..1: none for none.
    /// What is not under it is summed, so ground all under it is all of it (no share left by rounding).
    pub(super) fn wet(&self, v: f64) -> f64 {
        if v <= 0.0 {
            return 0.0;
        }
        (1.0 - (self.c.apart + self.sum(v, |p, x| p.area() - p.under(x))) / self.c.area).clamp(0.0, 1.0)
    }
    /// What stands deeper than `h` under the liquid's level where `v` m³ stand, m³.
    pub(super) fn deep(&self, v: f64, h: f64) -> f64 {
        self.sum(v, |p, x| p.volume(x - h))
    }
    /// The first moment of what stands about the cell's mean surface where `v` m³ stand, m⁴: Σ area × (level - floor)
    /// × (level + floor) / 2 over the floors under each part's level, so its centroid lies `moment / v` over the surface.
    pub(super) fn moment(&self, v: f64) -> f64 {
        if v <= 0.0 {
            return 0.0;
        }
        self.sum(v, Part::moment)
    }
    /// The length of the edge of the ground under the liquid where `v` m³ stand, m: a sample's side for each two samples
    /// beside each other of which one lies under its part's level.
    pub(super) fn shore(&self, v: f64) -> f64 {
        self.c.side * self.c.sides.across(&self.levels(v)) as f64
    }
}

impl Scratch {
    /// The ground of cell `i` as liquid stands on it (`Fill`): its curve and what its parts held at the last pass.
    pub(super) fn fill(&self, i: usize) -> Fill<'_> {
        fill_of(&self.curves[i], &self.state.held, i)
    }
}

/// The `Fill` of `c`, the curve of cell `i`, with what its parts held, `held[i]` (`State::held`; none where that is
/// not one number for each of its ponds' parts).
fn fill_of<'a>(c: &'a Curve, held: &'a [Vec<f64>], i: usize) -> Fill<'a> {
    Fill { c, held: held.get(i).filter(|h| h.len() == c.ponds().len()).map_or(&[], |h| &h[..]) }
}

/// Does `held` (`State::held`) hold one number for each part of a body of each of the cells' `curves`?
pub(super) fn holds_all(curves: &[Curve], held: &[Vec<f64>]) -> bool {
    held.len() == curves.len() && curves.iter().zip(held).all(|(c, h)| h.len() == c.ponds().len())
}

/// What each part of a body holds of what stands over its cell's pores now, m³, as if they held none before: at one
/// level over the cell's parts (`Fill::shares`); none for a cell that holds no pond's samples.
pub(super) fn first_held(s: &Scratch) -> Vec<Vec<f64>> {
    let one = |(i, c): (usize, &Curve)| if c.ponds().is_empty() { Vec::new() } else { Fill { c, held: &[] }.shares(super::land::standing(s, i)) };
    s.curves.iter().enumerate().map(one).collect()
}

/// The curve of cell `i` of `w` (`Curve`): the terrain's samples inside it, their departures from their mean by area,
/// grouped by the pond each lies in.
fn of(w: &World, i: usize) -> Curve {
    let (s, l, lat) = (w.terrain_samples(), w.map.leaves()[i], (w.cfg.lat_south, w.cfg.lat_north));
    let depth = SAMPLE_DEPTH.max(l.depth);
    let ks: Vec<(usize, f64)> = l.split(depth).map(|f| (s.index(&f), w.map.place_of(&f, lat.0, lat.1).0)).collect();
    let (all, area) = (ks.iter().map(|k| k.1).sum::<f64>(), w.mesh.cells().area()[i]);
    let mean = ks.iter().map(|&(k, a)| s.h[k] * a).sum::<f64>() / all;
    let mut ponds: Vec<u32> = ks.iter().map(|k| s.pond[k.0]).filter(|&p| p != NONE).collect();
    ponds.sort_unstable();
    ponds.dedup();
    let part_of = |k: usize| if ponds.is_empty() { 0 } else { ponds.iter().position(|&p| p == s.pond[k]).map_or(NO_PART, |p| p as u8) };
    let mut floors: Vec<Vec<(f64, f64)>> = vec![Vec::new(); ponds.len().max(1)];
    let mut apart = 0.0;
    for &(k, a) in &ks {
        match floors.get_mut(part_of(k) as usize) {
            Some(f) => f.push((s.h[k] - mean, a * area / all)),
            None => apart += a * area / all,
        }
    }
    let pond = |q: usize| ponds.get(q).copied().unwrap_or(NONE);
    let parts: Vec<Part> = floors.into_iter().enumerate().map(|(q, f)| Part::new(pond(q), f)).collect();
    let side = 1usize << (depth - l.depth);
    let at_grid = |p: usize| ((s.h[ks[p].0] - mean) as f32, part_of(ks[p].0));
    let pairs = (0..ks.len()).flat_map(|p| [(p % side + 1 < side).then(|| (p, p + 1)), (p + side < ks.len()).then(|| (p, p + side))]).flatten();
    let sides = Sides::new(pairs.map(|(p, q)| [at_grid(p), at_grid(q)]), parts.len());
    let low = ks.iter().map(|&(k, _)| s.h[k] - mean).fold(f64::INFINITY, f64::min);
    Curve { parts, area, apart, low, sides, side: (area / ks.len() as f64).sqrt() }
}

impl World {
    /// The ground inside cell `i` (of a square or coarser) made level at its mean (`Terrain::level_under`): its liquid
    /// stands at one depth over it. Flow finds every cell's curve and the bodies again at its next part.
    pub fn level_inside(&mut self, i: usize) {
        let lat = (self.cfg.lat_south, self.cfg.lat_north);
        self.terrain.level_under(self.terrain.seed, &self.map, self.map.leaves()[i], lat);
        self.scratch.curves.clear();
        self.scratch.bodies = Default::default();
    }
}

/// The curve of cell `i`: that of `s` where it has them, else found (`of`).
fn curve_of<'a>(w: &World, s: &'a Scratch, i: usize) -> Cow<'a, Curve> {
    match s.curves.get(i).filter(|_| !stale(w, s)) {
        Some(c) => Cow::Borrowed(c),
        None => Cow::Owned(of(w, i)),
    }
}

/// `f` of the `Fill` of cell `i` (`curve_of`, and what its parts held, `State::held`).
pub(super) fn with_fill<T>(w: &World, s: &Scratch, i: usize, f: impl FnOnce(Fill) -> T) -> T {
    let c = curve_of(w, s, i);
    f(fill_of(&c, &s.state.held, i))
}

/// Are Flow's curves missing?
pub(super) fn stale(w: &World, s: &Scratch) -> bool {
    s.curves.len() != w.mesh.cells().n()
}

/// The curve of each cell (`Scratch::curves`), found from the terrain's samples inside it (`of`).
pub(super) fn build(w: &World, s: &mut Scratch) {
    // (The terrain's samples found first, before the parallel section that reads them, `Terrain::samples`.)
    w.terrain_samples();
    s.curves = crate::par::map_n(w.map.leaves().len(), crate::par::UNCOUNTED, |i| of(w, i));
}
