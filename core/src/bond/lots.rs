//! Lots (laws.md 3.1): alike parts that lie alone in a cell are one record of `count` parts, as alike members are one
//! node (laws.md 4.4; handover 7.1), so the records of parts follow the designs in each cell, not the parts. A lot
//! holds the amounts of all its parts (their walls, the liquid in their hollows, their heat) and the shape of each one
//! (sizes, hollowness), with the depths of its parts as an even spread (`Part::buried`, `Part::spread`). Monthly the
//! alike lone parts of a cell join (`join_alike`); one is split out when a body holds or acts on one (`one_of`). Alike
//! parts lose their own history, as the members of a node do.

use super::{add, design, Part, NOBODY};
use crate::{chem::Chemistry, life::Chunk, World};
use std::collections::HashMap;

impl Part {
    /// `k` of its parts leave it as a lot of their own (not placed: no id, held by none): `k` / count of each amount and
    /// of its heat; it keeps the rest (the remainder, so the two add up to what it held to rounding). Shape and depths
    /// as its own: they are alike.
    pub(crate) fn split(&mut self, k: f64) -> Part {
        let f = k / self.count;
        let take = |v: &mut Vec<f64>| -> Vec<f64> {
            let out: Vec<f64> = v.iter().map(|a| f * a).collect();
            v.iter_mut().zip(&out).for_each(|(a, x)| *a -= x);
            out
        };
        let (wall, fill, e) = (take(&mut self.wall), take(&mut self.fill), f * self.e);
        (self.e, self.count) = (self.e - e, self.count - k);
        Part { count: k, wall, fill, e, id: 0, held: NOBODY, push: 0.0, ..self.clone() }
    }
    /// Lot `o`, alike and lying alone in its cell, joins it: counts, amounts and heat add up; each size, the hollowness
    /// and the mean depth become the means by count, the spread of the depths pools around it (the variance of an even
    /// spread is its half width squared over 3), and the sizes then fit the walls of one part.
    pub(super) fn join(&mut self, chem: &Chemistry, o: Part) {
        let (n, a) = (self.count + o.count, (o.count / (self.count + o.count)) as f32);
        let mix = |x: f32, y: f32| x + a * (y - x);
        let var = |p: &Part, m: f32| p.spread * p.spread / 3.0 + (p.buried - m).powi(2);
        let depth = mix(self.buried, o.buried);
        let pooled = (1.0 - a) * var(self, depth) + a * var(&o, depth);
        self.spread = (3.0 * pooled).sqrt().min(depth);
        (self.buried, self.hollow) = (depth, mix(self.hollow, o.hollow));
        self.size = std::array::from_fn(|x| mix(self.size[x], o.size[x]));
        self.wall.iter_mut().zip(&o.wall).for_each(|(s, x)| *s += x);
        self.fill.iter_mut().zip(&o.fill).for_each(|(s, x)| *s += x);
        (self.e, self.count) = (self.e + o.e, n);
        self.resize(chem);
    }
    /// The share of its parts buried past its smallest size (`month::bury`): of the even spread of their depths, the
    /// share deeper than it.
    pub(super) fn sunk(&self) -> f64 {
        let (lo, hi, s) = ((self.buried - self.spread) as f64, (self.buried + self.spread) as f64, self.size[2] as f64);
        if lo >= s {
            1.0
        } else if hi <= s {
            0.0
        } else {
            (hi - s) / (hi - lo)
        }
    }
}

/// The key by which lone parts of a cell join into a lot: the part's label (`design::label`: its material, shape and
/// mass bands) and how far it is buried, its depth over its smallest size in `design::BANDS` bands: within a lot the
/// depths of its parts are an even spread of about one band, so each part is buried within a band's share of its time
/// of its own (`month::bury`).
fn key(chem: &Chemistry, p: &Part) -> u64 {
    design::fnv(&[design::label(chem, p), design::band(p.buried as f64 / (p.size[2] as f64).max(1e-30))])
}

/// Monthly, before their turns: the alike parts of cell `i` of chunk `d` that lie alone (held by no body, no body inside
/// them, joined by no bond) join, each into the first of its `key` in the cell's list (`Part::join`), which keeps its
/// id; the list keeps its order.
pub(super) fn join_alike(w: &World, d: &mut Chunk, i: usize) {
    let c = d.at_mut(i);
    if c.parts.len() < 2 {
        return;
    }
    // The ids of the parts that do not lie alone, sorted.
    let mut tied: Vec<u64> = c.bonds.a.iter().chain(&c.bonds.b).copied().collect();
    tied.extend(c.bodies.iter().filter(|b| b.alive).flat_map(|b| [b.hold, b.inside]).flatten());
    tied.sort_unstable();
    let lone = |p: &Part| p.held == NOBODY && tied.binary_search(&p.id).is_err();
    // The place of the lot each part joins (its own if none).
    let mut first: HashMap<u64, usize> = HashMap::new();
    let into: Vec<usize> = (0..c.parts.len()).map(|x| if lone(&c.parts[x]) { *first.entry(key(&w.chem, &c.parts[x])).or_insert(x) } else { x }).collect();
    if into.iter().enumerate().all(|(x, &y)| x == y) {
        return;
    }
    let mut kept: Vec<Part> = Vec::with_capacity(first.len());
    let mut at = vec![0; into.len()];
    for (x, p) in std::mem::take(&mut c.parts).into_iter().enumerate() {
        if into[x] == x {
            at[x] = kept.len();
            kept.push(p);
        } else {
            kept[at[into[x]]].join(&w.chem, p);
        }
    }
    c.parts = kept;
}

/// The place of one part of the lot at `p` of cell `i` of chunk `d`, split out of it (`Part::split`) as a part of its
/// own, the last of the list, with the next id of the cell (`add`) where the lot is more than one: a body holds or acts
/// on one part. It lies at the lot's mean depth.
pub(crate) fn one_of(d: &mut Chunk, i: usize, p: usize) -> usize {
    let lot = &mut d.at_mut(i).parts[p];
    if lot.count <= 1.0 {
        return p;
    }
    let mut one = lot.split(1.0);
    one.spread = 0.0;
    add(d, i, one)
}

