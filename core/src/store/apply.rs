//! `apply`: books the link fluxes and moves of a call as they are given, then sets what rounding left below zero to
//! zero and counts it. A law never asks a node for more than it holds (laws.md 0, "No overdraw"), so the store only
//! adds, checks and counts.

use super::{heat_received, held_more};
use super::moves::{LinkFlux, Move};
use super::nodes::{Class, Node, BOUNDARIES};
use crate::chem::Chemistry;
use crate::{World, AIR};

/// What a number may end below zero by to rounding, relative to what its node holds of that unit, what it held and the
/// call's changes to it: more is a law that overdrew (checked in debug builds).
pub(crate) const ROUNDING: f64 = 1e-9;
/// The classes `apply` books, the first ones: those whose amounts and heat lie in the `Store` (a part or a body books
/// its own).
const BOOKED: usize = Class::Part as usize;
/// The first array of the nodes that give all they are asked and may hold less than nothing: the boundary nodes
/// (`Class::Boundary`).
const BOUNDARY_ARRAYS: usize = 2 * Class::Boundary as usize;

/// One number of the world: entry `at` of array `arr` (`2 * class` the amounts of a class, `2 * class + 1` its heat);
/// an amount is at `k * len + index`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Slot {
    arr: usize,
    at: usize,
}

/// One column of a link flux (a substance, or the heat): `out[l]` leaves array `arr[0]` at link `l`'s `a` end and
/// `into[l]` enters `arr[1]` at its `b` end, node `i`'s number at `at[end] + i`.
struct Column<'x> {
    arr: [usize; 2],
    at: [usize; 2],
    a: &'x [u32],
    b: &'x [u32],
    out: &'x [f64],
    into: &'x [f64],
}

impl Column<'_> {
    /// Each change it makes, link by link.
    #[inline(always)]
    fn each(&self, mut f: impl FnMut(Slot, f64)) {
        for (((&a, &b), &out), &into) in self.a.iter().zip(self.b).zip(self.out).zip(self.into) {
            f(Slot { arr: self.arr[0], at: self.at[0] + a as usize }, -out);
            f(Slot { arr: self.arr[1], at: self.at[1] + b as usize }, into);
        }
    }
}

/// A change of a call: a whole column of a link flux, or one number's change by a move.
enum Change<'x> {
    Column(Column<'x>),
    One(Slot, f64),
}

/// What `apply` reads besides the changes: the chemistry and the count of the nodes of each class.
struct Ctx<'w> {
    chem: &'w Chemistry,
    len: [usize; BOOKED],
}

/// The class of node `x` and its index, in a class `apply` books; none for a part or a body, whose law books its side.
fn booked(cx: &Ctx, x: Node) -> Option<(usize, usize)> {
    let c = x.class as usize;
    if c >= BOOKED {
        return None;
    }
    assert!(x.index < cx.len[c], "apply books no node {x:?}");
    Some((c, x.index))
}

/// Each change of the call in order: each link flux's columns (substances, then heat less what the mol hold more at
/// the `b` end), then each move's changes.
fn each_change<'x>(cx: &Ctx, (fluxes, lifted): (&'x [LinkFlux], &'x [Vec<f64>]), moves: &[Move], mut f: impl FnMut(Change<'x>)) {
    let (chem, len) = (cx.chem, &cx.len);
    for (x, lifted) in fluxes.iter().zip(lifted) {
        let (c, links, (a, b)) = (x.class.map(|c| c as usize), x.a.len(), (x.a, x.b));
        for (q, &k) in x.ks.iter().enumerate() {
            let m = &x.mol[q * links..(q + 1) * links];
            f(Change::Column(Column { arr: c.map(|c| 2 * c), at: c.map(|c| k as usize * len[c]), a, b, out: m, into: m }));
        }
        let into = if lifted.is_empty() { x.heat } else { lifted };
        f(Change::Column(Column { arr: c.map(|c| 2 * c + 1), at: [0; 2], a, b, out: x.heat, into }));
    }
    let mut one = |arr, at, d| f(Change::One(Slot { arr, at }, d));
    // A move of heat alone changes no amount, and one of amounts alone no heat.
    for m in moves {
        let k = m.k as usize;
        for (x, mol, heat) in [(m.from, -m.mol, -m.heat), (m.to, m.mol, heat_received(m.heat, m.mol, held_more(chem, m.from.class, m.to.class, k)))] {
            if let Some((c, i)) = booked(cx, x) {
                if mol != 0.0 {
                    one(2 * c, k * len[c] + i, mol);
                }
                if heat != 0.0 {
                    one(2 * c + 1, i, heat);
                }
            }
        }
    }
}

/// Adds `d` to number `s`, `v`, noting it in `low` when it falls below zero (but of a node that gives all it is asked).
#[inline(always)]
fn add(v: &mut f64, s: Slot, d: f64, low: &mut Vec<Slot>) {
    let was = *v;
    *v += d;
    if *v < 0.0 && was >= 0.0 && s.arr < BOUNDARY_ARRAYS {
        low.push(s);
    }
}

/// Panics when number `now`, below zero, ends there by more than rounding (`ROUNDING`): relative to `node`, what its
/// node holds of that unit, what it held before (`now - sum`) and the sizes of its changes, `sum` their sum and `size`
/// the sum of their sizes. More is a law that overdrew a node (laws.md 0, "No overdraw"); `at` names the number.
pub(super) fn overdraw_check(now: f64, node: f64, [sum, size]: [f64; 2], at: impl std::fmt::Debug) {
    assert!(-now <= ROUNDING * (node + (now - sum).abs() + size), "a law overdrew a node: number {at:?} ends at {now}");
}

/// What rounding left of number `v` below zero is set to zero and counted in `rounded` (mol or J).
#[inline(always)]
pub(super) fn zero_rounding(v: &mut f64, rounded: &mut f64) {
    *rounded -= if *v < 0.0 { std::mem::take(v) } else { 0.0 };
}

/// Book the link fluxes, then the moves, as given: what leaves one node enters another, so the books are kept by their
/// form. Only what each number holds at the end counts, so what passes through a node in one call needs nothing held.
/// Then each number below zero, the rounding of the sums, is set to zero and counted in `Counters::rounded` (handover
/// 12.1; checked in debug builds). Boundary nodes give all they are asked. Panics on a flux of the
/// wrong shape or a node beyond its class.
pub fn apply(w: &mut World, fluxes: &[LinkFlux], moves: &[Move]) {
    let World { chem, counts, mesh, store, .. } = w;
    let n = mesh.cells().n();
    let cx = Ctx { chem, len: [n, mesh.level(AIR).n(), n, n, n, BOUNDARIES] };
    for x in fluxes {
        let links = x.a.len();
        let heat = if x.alike(chem) { x.heat.is_empty() || x.heat.len() == links } else { x.heat.len() == links };
        let shaped = x.class.iter().all(|&c| (c as usize) < BOOKED) && x.b.len() == links && x.mol.len() == x.ks.len() * links && heat;
        assert!(shaped, "a link flux of the wrong shape");
    }
    let lifted: Vec<_> = fluxes.iter().map(|x| x.lifted(chem)).collect();
    let mut books: Vec<&mut [f64]> = store.amounts.iter_mut().zip(&mut store.heat).flat_map(|(a, h)| [&mut a[..], &mut h[..]]).collect();
    // Each number that falls below zero on the way (those that end there are among them).
    let mut low = Vec::new();
    each_change(&cx, (fluxes, &lifted), moves, |change| match change {
        Change::One(s, d) => add(&mut books[s.arr][s.at], s, d, &mut low),
        Change::Column(c) if c.arr[0] == c.arr[1] => {
            let v = &mut *books[c.arr[0]];
            c.each(|s, d| add(&mut v[s.at], s, d, &mut low));
        }
        Change::Column(c) => {
            let [p, q] = books.get_disjoint_mut(c.arr).expect("two arrays");
            c.each(|s, d| add(if s.arr == c.arr[0] { &mut p[s.at] } else { &mut q[s.at] }, s, d, &mut low));
        }
    });
    low.sort_unstable();
    low.dedup();
    low.retain(|s| books[s.arr][s.at] < 0.0);
    if cfg!(debug_assertions) && !low.is_empty() {
        // Per number below zero: the sum of its changes, and of their sizes.
        let mut sums = vec![[0.0; 2]; low.len()];
        let mut sum = |s: Slot, d: f64| {
            if let Ok(x) = low.binary_search(&s) {
                sums[x] = [sums[x][0] + d, sums[x][1] + d.abs()];
            }
        };
        each_change(&cx, (fluxes, &lifted), moves, |change| match change {
            Change::One(s, d) => sum(s, d),
            Change::Column(c) => c.each(&mut sum),
        });
        for (s, [sum, size]) in low.iter().zip(sums) {
            let (v, len) = (&books[s.arr], cx.len[s.arr / 2]);
            let node: f64 = v[s.at % len..].iter().step_by(len).map(|x| x.abs()).sum();
            overdraw_check(v[s.at], node, [sum, size], s);
        }
    }
    for s in low {
        zero_rounding(&mut books[s.arr][s.at], &mut counts.rounded[s.arr % 2]);
    }
    let fell = from_life(chem.ns(), moves);
    if !fell.is_empty() {
        life_moved(w, &fell);
    }
}

/// The heat a fall gives each node of `class` that matter lands in, or a lift takes from the node that pays it, `(i,
/// q)`: `q` J to node `i` (negative for a lift), in the order given, booked on that node, and its other side on the
/// potential energy of the world's matter (`Store::lifted`), summed apart and added once (laws.md 0, "The books"). A
/// lift never takes more than its node holds; what rounding leaves below zero is set to zero and counted
/// (`Counters::rounded`), but of a boundary node, which gives all it is asked.
pub fn fall_heat(w: &mut World, class: Class, q: impl IntoIterator<Item = (usize, f64)>) {
    let World { store, counts, .. } = w;
    let (heat, mut fell) = (&mut store.heat[class as usize], 0.0);
    for (i, d) in q.into_iter().filter(|p| p.1 != 0.0) {
        let v = &mut heat[i];
        let was = *v;
        *v += d;
        fell += d;
        if *v < 0.0 && class != Class::Boundary {
            if cfg!(debug_assertions) {
                overdraw_check(*v, was.abs(), [d, d.abs()], (class, i));
            }
            zero_rounding(v, &mut counts.rounded[1]);
        }
    }
    store.lifted -= fell;
}

/// Matter that moved between the life of ground cells and their ground or bed, now booked: per cell `i`, `d[k]` and
/// `d[ns + k]` mol of substance `k` its ground and its bed gained from its life (gave it where negative). Life's matter
/// lies at its cell's head, and its potential energy counts with its ground node, as Flow books it (`flow::potential`),
/// so the ground node's heat takes what the move gave the potential energy of the world's matter back (`flow::gain`:
/// the cell as it was, its life heavier by the mass its ground and bed gained): a fall from the head heats the ground
/// it lands in, a lift to the head is paid by the ground it leaves (`fall_heat`, against `Store::lifted`), and the
/// stack and the air over it that the move raises or lowers are paid alike (laws.md 0, "The books").
pub(crate) fn life_moved(w: &mut World, d: &[(usize, Vec<f64>)]) {
    let ns = w.chem.ns();
    let back = |(i, d): &(usize, Vec<f64>)| {
        let kg: f64 = (0..ns).map(|k| (d[k] + d[ns + k]) * w.chem.subs[k].molar_mass).sum();
        let undo = |c: Class, k: usize| match c {
            Class::Ground => -d[k],
            Class::Bed => -d[ns + k],
            _ => 0.0,
        };
        (*i, crate::flow::gain(w, *i, undo, kg))
    };
    let q: Vec<(usize, f64)> = d.iter().map(back).collect();
    fall_heat(w, Class::Ground, q);
}

/// Life's matter that went from the head of one ground cell to another's, `(from, to, kg)` in the order given (a body
/// that moved and the structure it carried, a litter, seeds), now booked: it carries its potential energy from the head
/// it left, and the ground node of the cell it lands in takes what that holds over what it holds at its new head
/// (`flow::gain` of `kg` at each head): a fall heats that ground, a climb is paid by it (`fall_heat`, against
/// `Store::lifted`).
pub(crate) fn life_went(w: &mut World, went: &[(usize, usize, f64)]) {
    let at = |i: usize, kg: f64| crate::flow::gain(w, i, |_, _| 0.0, kg);
    let q: Vec<(usize, f64)> = went.iter().filter(|x| x.0 != x.1 && x.2 != 0.0).map(|&(from, to, kg)| (to, at(from, kg) - at(to, kg))).collect();
    fall_heat(w, Class::Ground, q);
}

/// Life's matter that went between the heads of ground cells, `(cell, [left, came])` the kg each lost from its head
/// and gained at it in all, in the order given, now booked: it carries its potential energy from the head it left, and
/// the ground node of the cell takes what the kg that left held over what the kg that came holds (`flow::gain` of each
/// at the cell's head): a fall heats that ground, a climb is paid by it (`fall_heat`, against `Store::lifted`). Two
/// calls to Flow at most for each cell, whatever the number of its sources and targets.
pub(crate) fn life_sent(w: &mut World, sent: impl IntoIterator<Item = (usize, [f64; 2])>) {
    let at = |w: &World, i: usize, kg: f64| if kg == 0.0 { 0.0 } else { crate::flow::gain(w, i, |_, _| 0.0, kg) };
    let q: Vec<(usize, f64)> = sent.into_iter().map(|(i, [left, came])| (i, at(w, i, left) - at(w, i, came))).collect();
    fall_heat(w, Class::Ground, q);
}

/// The moves of `moves` between a node of life (a body, a part, a cover) and a ground or bed node, per ground cell in
/// the order of the cells: what its ground (`[k]`) and its bed (`[ns + k]`) gained of each substance `k`, mol
/// (`life_moved`).
fn from_life(ns: usize, moves: &[Move]) -> Vec<(usize, Vec<f64>)> {
    let life = |c: Class| matches!(c, Class::Body | Class::Part | Class::Cover);
    let mut d: std::collections::BTreeMap<usize, Vec<f64>> = Default::default();
    for m in moves.iter().filter(|m| m.mol != 0.0) {
        for (x, other, mol) in [(m.to, m.from, m.mol), (m.from, m.to, -m.mol)] {
            let at = match x.class {
                Class::Ground => 0,
                Class::Bed => ns,
                _ => continue,
            };
            if life(other.class) {
                d.entry(x.index).or_insert_with(|| vec![0.0; 2 * ns])[at + m.k as usize] += mol;
            }
        }
    }
    d.into_iter().collect()
}
