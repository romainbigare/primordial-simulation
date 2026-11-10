//! Lots (laws.md 3.1): alike parts that lie alone in a cell are one record of `count` parts, as alike members are one
//! node (laws.md 4.4; handover 7.1), so the records of parts follow the designs in each cell, not the parts. A lot
//! holds the amounts of all its parts (their walls, the liquid in their hollows, their heat) and the shape of each one
//! (sizes, hollowness), with the depths of its parts as an even spread (`Part::buried`, `Part::spread`). Monthly the
//! alike lone parts of a cell join (`join_alike`); a body acts on one split out of a lot (`some_of`). A node holds a
//! lot of up to one part per member (`hold_by_members`), and the lots of alike holders join as they do
//! (`join_held`). Alike parts lose their own history, as the members of a node do.

use super::{add, at, design, held, holder, Part, NOBODY};
use crate::{body::Body, chem::Chemistry, life::{Cell, Chunk, Handle}, World};
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

/// Lots a cell keeps at most after its monthly join; past them its lots join the nearest designs (`key` at a coarser
/// level) until they are no more (a speed setting).
pub const MAX_LOTS: usize = 16;

/// Levels of `key`: the full key; with no band of burial; with the mass band 2 to 64 times wider; with no band of
/// elongation, of hollowness, of bond strength, of hardness; and with no mass band, one lot.
const LEVELS: usize = 13;

/// The key by which lone parts of a cell join into a lot at `level` (0: alike; `LEVELS`): the bands of the part's label
/// (`design::bands`: its material, shape and mass bands) and how far it is buried, its depth over its smallest size in
/// `design::BANDS` bands: within a lot the depths of its parts are an even spread of about one band, so each part is
/// buried within a band's share of its time of its own (`month::bury`). A coarser level drops the least of them first.
fn key(chem: &Chemistry, p: &Part, level: usize) -> u64 {
    let mut b = design::bands(chem, p);
    let burial = if level == 0 { design::band(p.buried as f64 / (p.size[2] as f64).max(1e-30)) } else { 0 };
    b[4] >>= (level.saturating_sub(1)).min(6);
    // (Then the bands of elongation, hollowness, bond strength and hardness, and last the mass band.)
    for (x, l) in [(3, 8), (2, 9), (1, 10), (0, 11), (4, 12)] {
        if level >= l {
            b[x] = 0;
        }
    }
    design::fnv(&[design::label_of(b), burial as u64])
}

/// Monthly, before their turns: the alike parts of cell `i` of chunk `d` that lie alone (held by no body, no body inside
/// them, joined by no bond) join, each into the first of its `key` in the cell's list (`Part::join`), which keeps its
/// id; the list keeps its order. While the cell holds more than `MAX_LOTS` lots, they join again by a coarser key.
pub(super) fn join_alike(w: &World, d: &mut Chunk, i: usize) {
    let c = d.at_mut(i);
    // The ids of the parts that do not lie alone, sorted.
    let mut tied: Vec<u64> = c.bonds.a.iter().chain(&c.bonds.b).copied().collect();
    tied.extend(c.bodies.iter().filter(|b| b.alive).flat_map(|b| [b.hold, b.inside]).flatten());
    tied.sort_unstable();
    let lone = |p: &Part| p.held == NOBODY && tied.binary_search(&p.id).is_err();
    for level in 0..LEVELS {
        if level > 0 && c.parts.iter().filter(|p| lone(p)).count() <= MAX_LOTS {
            break;
        }
        join_by(&w.chem, &mut c.parts, |p| lone(p).then(|| key(&w.chem, p, level)));
    }
}

/// The parts of `parts` of a key (`key`; none: it joins nothing) join into the first of their key in the list's order.
fn join_by(chem: &Chemistry, parts: &mut Vec<Part>, key: impl Fn(&Part) -> Option<u64>) {
    if parts.len() < 2 {
        return;
    }
    // The place of the lot each part joins (its own if none).
    let mut first: HashMap<u64, usize> = HashMap::new();
    let into: Vec<usize> = (0..parts.len()).map(|x| key(&parts[x]).map_or(x, |k| *first.entry(k).or_insert(x))).collect();
    if into.iter().enumerate().all(|(x, &y)| x == y) {
        return;
    }
    let mut kept: Vec<Part> = Vec::with_capacity(first.len());
    let mut at = vec![0; into.len()];
    for (x, p) in std::mem::take(parts).into_iter().enumerate() {
        if into[x] == x {
            at[x] = kept.len();
            kept.push(p);
        } else {
            kept[at[into[x]]].join(chem, p);
        }
    }
    *parts = kept;
}

/// The place of `k` parts of the lot at `p` of cell `i` of chunk `d`, split out of it (`Part::split`) as a lot of their
/// own, held by none, the last of the list, with the next id of the cell (`add`) where the lot holds more: a body holds
/// or acts on so many. They lie at the lot's mean depth.
pub(crate) fn some_of(d: &mut Chunk, i: usize, p: usize, k: f64) -> usize {
    let lot = &mut d.at_mut(i).parts[p];
    if lot.count <= k {
        return p;
    }
    let mut some = lot.split(k);
    some.spread = 0.0;
    add(d, i, some)
}

/// Monthly, before the lone parts join: a body of cell `i` of chunk `d` that holds more parts than it has members (some
/// of them left it or died) lets go of the rest, which lie alone.
pub(super) fn hold_by_members(d: &mut Chunk, i: usize) {
    for p in 0..d.at(i).parts.len() {
        let c = d.at(i);
        let Some(o) = holder(c, p) else { continue };
        let extra = c.parts[p].count - c.bodies[o].n();
        if extra > 0.0 {
            let rest = d.at_mut(i).parts[p].split(extra);
            add(d, i, rest);
        }
    }
}

/// What the part body `b` holds in its cell `c` is to the monthly merge (`nodes::Kin`): of a lot no bond joins, its
/// design and its band of mass (`design::label`), so bodies that hold alike parts merge and their lots join
/// (`join_held`); of a part of a structure, the structure's root.
pub fn held_kind(chem: &Chemistry, c: &Cell, b: &Body) -> Option<(bool, u64)> {
    let p = held(c, b)?;
    let pt = &c.parts[p];
    Some(if c.bonds.of(pt.id).next().is_none() { (true, design::label(chem, pt)) } else { (false, pt.root) })
}

/// Body `j` (taken out of its slot) joins the body at `t` of its cell (`nodes::merge`): if both hold a lot of one kind
/// (`held_kind`), `j`'s lot joins `t`'s (`Part::join`) and `j` holds nothing; returns whether it did.
pub(crate) fn join_held(w: &World, d: &mut Chunk, j: &mut Body, t: Handle) -> bool {
    let (i, c) = (j.cell as usize, d.at(j.cell as usize));
    let kind = held_kind(&w.chem, c, j);
    if kind.is_none_or(|k| !k.0) || kind != held_kind(&w.chem, c, &d[t]) || j.hold == d[t].hold {
        return false;
    }
    let (from, to) = (j.hold.take().expect("it holds"), d[t].hold.expect("it holds"));
    let c = d.at_mut(i);
    let lot = c.parts.remove(at(c, from).expect("its lot"));
    let x = at(c, to).expect("the lot it joins");
    c.parts[x].join(&w.chem, lot);
    true
}

