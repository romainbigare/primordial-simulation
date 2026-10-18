//! `apply`: books the link fluxes, moves and transforms of a call as they are given, then sets what rounding left below
//! zero to zero and counts it. A law never asks a node for more than it holds (`docs/foundation/plan.md`, Idea 2), so
//! the store only adds, checks and counts.

use super::held;
use super::moves::{LinkFlux, Move, Transform};
use super::nodes::{Class, Node, BOUNDARIES};
use crate::chem::Chemistry;
use crate::{World, AIR};

/// What a transform may lose or gain of an element to rounding, relative to all the elements it changes.
const ELEMENT_ROUNDING: f64 = 1e-9;
/// What a number may end below zero by to rounding, relative to what its node holds of that unit (all its amounts, or
/// its heat), what the number held and all the changes of the call to it: more is a law that overdrew (checked in debug
/// builds). A law's sums round at the size of what it moves, which is the size of what the node holds or less.
const ROUNDING: f64 = 1e-9;
/// The classes `apply` books, the first ones: those whose amounts and heat lie in arrays of `World` (a part's lie in
/// the part, `Class::Part`, which Bond books).
const BOOKED: usize = Class::Part as usize;
/// The arrays `apply` books: `2 * class` holds the amounts of a class and `2 * class + 1` its heat, the order in which
/// it counts what it rounds.
const ARRAYS: usize = 2 * BOOKED;
/// The first array of the nodes that give all they are asked and may hold less than nothing: the boundary nodes and
/// the potential (`Class::Boundary`, `Class::Potential`).
const BOUNDARY_ARRAYS: usize = 2 * Class::Boundary as usize;

/// One number of the world: entry `at` of array `arr` (`ARRAYS`); an amount is at `k * len + index`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Slot {
    arr: usize,
    at: usize,
}

/// One column of a link flux (a substance, or the heat): from array `arr[0]` (`ARRAYS`) at the links' `a` ends to array
/// `arr[1]` at their `b` ends, where the number of node `i` is at `at[end] + i`; `out[l]` leaves the `a` end of link `l`
/// and `into[l]` enters its `b` end.
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

/// The class of node `x` and its index, in a class `apply` books.
fn booked(cx: &Ctx, x: Node) -> (usize, usize) {
    let c = x.class as usize;
    assert!(c < BOOKED && x.index < cx.len[c], "apply books no node {x:?}");
    (c, x.index)
}

/// Each change of the call, in order: each link flux's columns, its substances then its heat (the `b` end receives the
/// heat less what the mol hold more there, `lifted`); then each move's and each transform's changes, number by number.
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
    for m in moves {
        let (((cf, i), (ct, j)), k) = ((booked(cx, m.from), booked(cx, m.to)), m.k as usize);
        let into = m.heat - m.mol * (held(chem, Class::ALL[ct], k) - held(chem, Class::ALL[cf], k));
        one(2 * cf, k * len[cf] + i, -m.mol);
        one(2 * cf + 1, i, -m.heat);
        one(2 * ct, k * len[ct] + j, m.mol);
        one(2 * ct + 1, j, into);
    }
    for t in transforms {
        let (c, i) = booked(cx, t.at);
        let mut q = 0.0;
        for &(k, d) in &t.dmol {
            one(2 * c, k as usize * len[c] + i, d);
            q -= d * held(chem, Class::ALL[c], k as usize);
        }
        one(2 * c + 1, i, q);
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

/// Does `dmol` keep every element of the chemistry's table, to rounding? `sums` is room for the element totals.
fn keeps_elements(chem: &Chemistry, dmol: &[(u16, f64)], sums: &mut [f64]) -> bool {
    let mut size = 0.0;
    sums.fill(0.0);
    for &(k, d) in dmol {
        chem.add_elems(k as usize, d, sums);
        size += d.abs() * chem.subs[k as usize].elems.iter().map(|&c| c as f64).sum::<f64>();
    }
    sums.iter().all(|e| e.abs() <= ELEMENT_ROUNDING * size)
}

/// Book the link fluxes, then the moves, then the transforms, each in its order, as they are given: what leaves one
/// node enters another, so the books are kept by the form of the changes. A law asks no node for more than it holds
/// (`docs/handover.md` 6.1): what passes through a node in one call (in by one link, out by another) needs nothing held,
/// as only what each number holds at the end counts. Then each amount and heat of a node that ended below zero, which
/// is the rounding of the sums, is set to zero and counted in `Counters::rounded` (handover 12.1), in the order of the
/// arrays; a debug build checks that it is only rounding (`ROUNDING`). A boundary node and the potential give all they
/// are asked. Panics on a link flux of the wrong shape, on a transform that does not keep its elements, and on a node
/// beyond its class.
pub fn apply(w: &mut World, fluxes: &[LinkFlux], moves: &[Move], transforms: &[Transform]) {
    w.fit_deep();
    let World { chem, counts, mesh, ground, e_ground, air, e_air, deep, e_deep, boundary, e_boundary, potential, .. } = w;
    let cx = Ctx { chem, len: [mesh.cells().n(), mesh.level(AIR).n(), mesh.cells().n(), BOUNDARIES, 1] };
    for x in fluxes {
        let links = x.a.len();
        let heat = if x.alike(chem) { x.heat.is_empty() || x.heat.len() == links } else { x.heat.len() == links };
        let shaped = x.class.iter().all(|&c| (c as usize) < BOOKED) && x.b.len() == links && x.mol.len() == x.ks.len() * links && heat;
        assert!(shaped, "a link flux of the wrong shape");
    }
    let mut sums = vec![0.0; chem.n_elements];
    for t in transforms {
        assert!(keeps_elements(chem, &t.dmol, &mut sums), "a transform that does not keep its elements: {t:?}");
    }
    let lifted: Vec<_> = fluxes.iter().map(|x| x.lifted(chem)).collect();
    let mut books: [&mut [f64]; ARRAYS] = [&mut ground[..], &mut e_ground[..], &mut air[..], &mut e_air[..], &mut deep[..], &mut e_deep[..], &mut boundary[..], &mut e_boundary[..], &mut [], std::slice::from_mut(potential)];
    // Each number of a node that falls below zero on the way: those that end there are among them. A column books into
    // its one or two arrays.
    let mut low = Vec::new();
    each_change(&cx, (fluxes, &lifted), moves, transforms, |change| match change {
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
        each_change(&cx, (fluxes, &lifted), moves, transforms, |change| match change {
            Change::One(s, d) => sum(s, d),
            Change::Column(c) => c.each(&mut sum),
        });
        for (s, [sum, size]) in low.iter().zip(sums) {
            let (v, len) = (&books[s.arr], cx.len[s.arr / 2]);
            let node: f64 = v[s.at % len..].iter().step_by(len).map(|x| x.abs()).sum();
            let now = v[s.at];
            assert!(-now <= ROUNDING * (node + (now - sum).abs() + size), "a law overdrew a node: number {s:?} ends at {now}");
        }
    }
    for s in low {
        counts.rounded[s.arr % 2] -= std::mem::take(&mut books[s.arr][s.at]);
    }
}
