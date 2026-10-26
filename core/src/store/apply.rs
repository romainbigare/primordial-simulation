//! `apply`: books the link fluxes, moves and transforms of a call as they are given, then sets what rounding left below
//! zero to zero and counts it. A law never asks a node for more than it holds (laws.md 0, "No overdraw"), so the store
//! only adds, checks and counts.

use super::{held, held_more};
use super::moves::{LinkFlux, Move, Transform};
use super::nodes::{Class, Node, BOUNDARIES};
use super::CARRIED;
use crate::chem::Chemistry;
use crate::math::rounding_lost;
use crate::{World, AIR};

/// What a transform may lose or gain of an element to rounding, relative to all the elements it changes.
const ELEMENT_ROUNDING: f64 = 1e-9;
/// What a number may end below zero by to rounding, relative to what its node holds of that unit, what it held and the
/// call's changes to it: more is a law that overdrew (checked in debug builds).
pub(crate) const ROUNDING: f64 = 1e-9;
/// The classes `apply` books, the first ones: those whose amounts and heat lie in the `Store` (a part or a body books
/// its own).
const BOOKED: usize = Class::Part as usize;
/// The first array of the nodes that give all they are asked and may hold less than nothing: the boundary nodes and
/// the potential (`Class::Boundary`, `Class::Potential`).
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

/// A change of a call: a whole column of a link flux, or one number's change by a move or a transform.
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
/// the `b` end), then each move's and each transform's changes.
fn each_change<'x>(cx: &Ctx, (fluxes, lifted): (&'x [LinkFlux], &'x [Vec<f64>]), moves: &[Move], transforms: &[Transform], mut f: impl FnMut(Change<'x>)) {
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
        for (x, mol, heat) in [(m.from, -m.mol, -m.heat), (m.to, m.mol, m.heat - m.mol * held_more(chem, m.from.class, m.to.class, k))] {
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
    for t in transforms {
        let (c, i) = booked(cx, t.at).expect("a transform in a node of the store");
        let mut q = 0.0;
        for &(k, d) in &t.dmol {
            one(2 * c, k as usize * len[c] + i, d);
            q -= d * held(chem, Class::ALL[c], k as usize);
        }
        one(2 * c + 1, i, q);
    }
}

/// Adds `d` to number `s`, `v`, noting it in `low` when it falls below zero (but of a node that gives all it is asked).
/// A ground node under deep standing liquid holds much of it and an air node much of the main gases, so a small move of
/// a body would round away against them: what rounding takes from an amount of a carried class (`CARRIED`) is kept in
/// `carry` and folded back as far as the amount can hold it (Neumaier 1974).
#[inline(always)]
fn add(v: &mut f64, s: Slot, d: f64, low: &mut Vec<Slot>, carry: &mut [Vec<f64>]) {
    let was = *v;
    add_carried(v, (s.arr.is_multiple_of(2) && s.arr < 2 * CARRIED).then(|| &mut carry[s.arr / 2][s.at]), d);
    if *v < 0.0 && was >= 0.0 && s.arr < BOUNDARY_ARRAYS {
        low.push(s);
    }
}

/// Adds `d` to number `v`; to an amount of a carried class with what rounding took so far, `carry`, which keeps what it
/// takes now (Neumaier 1974).
#[inline(always)]
pub(super) fn add_carried(v: &mut f64, carry: Option<&mut f64>, d: f64) {
    let was = *v;
    *v += d;
    if let Some(carry) = carry {
        let lost = *carry + rounding_lost(was, d, *v);
        let folded = *v + lost;
        (*carry, *v) = (rounding_lost(*v, lost, folded), folded);
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

/// Does `dmol` keep every element of the chemistry's table, to rounding? `sums` is room for the element totals.
pub(crate) fn keeps_elements(chem: &Chemistry, dmol: impl IntoIterator<Item = (u16, f64)>, sums: &mut [f64]) -> bool {
    let mut size = 0.0;
    sums.fill(0.0);
    for (k, d) in dmol {
        chem.add_elems(k as usize, d, sums);
        size += d.abs() * chem.subs[k as usize].elems.iter().map(|&c| c as f64).sum::<f64>();
    }
    sums.iter().all(|e| e.abs() <= ELEMENT_ROUNDING * size)
}

/// Book the link fluxes, then the moves, then the transforms, as given: what leaves one node enters another, so the
/// books are kept by their form. Only what each number holds at the end counts, so what passes through a node in one
/// call needs nothing held. Then each number below zero, the rounding of the sums, is set to zero and counted in
/// `Counters::rounded` (handover 12.1; checked in debug builds). Boundary nodes and the potential give all they are
/// asked. Panics on a flux of the wrong shape, a transform that breaks its elements, or a node beyond its class. No
/// domain's view of the store is open then (`Moves::open`; checked in debug builds).
pub fn apply(w: &mut World, fluxes: &[LinkFlux], moves: &[Move], transforms: &[Transform]) {
    debug_assert!(w.life.domains.iter().all(|d| d.moves.open.is_none()), "the store changes while a domain is open");
    let World { chem, counts, mesh, store, .. } = w;
    let n = mesh.cells().n();
    let cx = Ctx { chem, len: [n, mesh.level(AIR).n(), n, n, n, BOUNDARIES, 1] };
    for x in fluxes {
        let links = x.a.len();
        let heat = if x.alike(chem) { x.heat.is_empty() || x.heat.len() == links } else { x.heat.len() == links };
        let shaped = x.class.iter().all(|&c| (c as usize) < BOOKED) && x.b.len() == links && x.mol.len() == x.ks.len() * links && heat;
        assert!(shaped, "a link flux of the wrong shape");
    }
    let mut sums = vec![0.0; if transforms.is_empty() { 0 } else { chem.n_elements }];
    for t in transforms {
        assert!(keeps_elements(chem, t.dmol.iter().copied(), &mut sums), "a transform that does not keep its elements: {t:?}");
    }
    let lifted: Vec<_> = fluxes.iter().map(|x| x.lifted(chem)).collect();
    // The heat of the boundary nodes and the potential is a whole history's sum: their changes are summed apart and added
    // once, so small changes do not round away against it.
    let (mut far_heat, mut far_lift) = ([0.0; BOUNDARIES], [0.0]);
    let carry = &mut store.carry[..];
    let mut books: Vec<&mut [f64]> = store.amounts.iter_mut().zip(&mut store.heat).flat_map(|(a, h)| [&mut a[..], &mut h[..]]).collect();
    (books[2 * Class::Boundary as usize + 1], books[2 * Class::Potential as usize + 1]) = (&mut far_heat, &mut far_lift);
    // Each number that falls below zero on the way (those that end there are among them).
    let mut low = Vec::new();
    each_change(&cx, (fluxes, &lifted), moves, transforms, |change| match change {
        Change::One(s, d) => add(&mut books[s.arr][s.at], s, d, &mut low, carry),
        Change::Column(c) if c.arr[0] == c.arr[1] => {
            let v = &mut *books[c.arr[0]];
            c.each(|s, d| add(&mut v[s.at], s, d, &mut low, carry));
        }
        Change::Column(c) => {
            let [p, q] = books.get_disjoint_mut(c.arr).expect("two arrays");
            c.each(|s, d| add(if s.arr == c.arr[0] { &mut p[s.at] } else { &mut q[s.at] }, s, d, &mut low, carry));
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
        each_change(&cx, (fluxes, &lifted), moves, transforms, |change| match change {
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
    for (c, far) in [(Class::Boundary, &far_heat[..]), (Class::Potential, &far_lift[..])] {
        far.iter().enumerate().for_each(|(at, &d)| store.add_far_heat(c, at, d));
    }
}
