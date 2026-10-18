//! The three ways a law changes the world (`docs/foundation/plan.md`, Idea 2): a move between two nodes, a transform
//! of the amounts inside one node, and a link flux over a whole field; how a flux's numbers lie in the world's arrays
//! (its columns); and the fast way `apply` books link fluxes (`book_first`).

use super::nodes::{Class, NodeId, Nodes};
use crate::chem::Chemistry;
use crate::clock::Sleep;

/// A link flux's net change of a number within this share of all that passes through it is the rounding of its sum, not
/// a change: liquid that passes through a dry cell would else take from it what it never held.
pub(super) const NET_ROUNDING: f64 = 1e-12;
/// The classes `apply` books, the first ones: those whose amounts lie in arrays of `World` (a part's lie in the part,
/// `Class::Part`, which Bond books).
pub(super) const BOOKED: usize = Class::Part as usize;

/// `mol` of substance `k` and `heat` J leave node `from` for node `to`. `to` receives `heat` less the energy the mol
/// hold apart from heat more there than in `from` (`held`): so the law chooses who pays the latent heat of a gas that
/// enters the air, `from` by giving it with the heat, or `to` by being given only the heat the gas carries.
#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub from: NodeId,
    pub to: NodeId,
    pub k: u16,
    pub mol: f64,
    pub heat: f64,
}

/// The amounts of node `at` change by `dmol` (substance, mol), keeping every element; the energy they held apart from
/// heat (`held`) and no longer hold becomes the node's heat (the heat of reaction).
#[derive(Clone, Debug)]
pub struct Transform {
    pub at: NodeId,
    pub dmol: Vec<(u16, f64)>,
}

/// What crosses each link of a field of nodes at once: link `l` joins the node at index `a[l]` of class `class[0]` and
/// the one at `b[l]` of `class[1]` (one class for the links of a level, two for the vertical links of a stack, which
/// must hold substances alike: both air or neither, `held`) and carries `mol[q * links + l]` mol of substance `ks[q]`
/// (each substance once) and `heat[l]` J, + from `a` to `b` (`heat` empty: no heat). What leaves one end enters the
/// other, so a flux keeps the books by its form. It takes from a node only what the node loses over all its links (so
/// what passes through a node, in by one link and out by another, as liquid down a slope, needs nothing held), and it
/// is scaled as a whole.
#[derive(Clone, Copy, Debug)]
pub struct LinkFlux<'a> {
    pub class: [Class; 2],
    pub a: &'a [u32],
    pub b: &'a [u32],
    pub ks: &'a [u16],
    pub mol: &'a [f64],
    pub heat: &'a [f64],
}

/// Energy a mol of substance `k` holds in a node of `class` apart from the node's heat, J/mol: its formation enthalpy,
/// and in the air its fusion and vaporisation enthalpy (`Substance::latent_gas`), as `check::totals` books them.
pub fn held(chem: &Chemistry, class: Class, k: usize) -> f64 {
    let s = &chem.subs[k];
    s.hf + if class == Class::Air { s.latent_gas() } else { 0.0 }
}

/// The arrays `apply` books: `2 * class` holds the amounts of a class and `2 * class + 1` its heat, the order in which
/// it counts what it rounds.
pub(super) const ARRAYS: usize = 2 * BOOKED;
/// The first array of the boundary nodes, which give all they are asked (`Class::Boundary`).
pub(super) const BOUNDARY_ARRAYS: usize = 2 * Class::Boundary as usize;

/// One number of the world: entry `at` of array `arr` (`ARRAYS`); an amount is at `k * len + index`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Slot {
    pub(super) arr: usize,
    pub(super) at: usize,
}

impl Slot {
    /// 0 for an amount, 1 for a heat: its place in `Counters::scaled` and `Counters::rounded`.
    pub(super) fn unit(self) -> usize {
        self.arr % 2
    }
    /// Whether a change of `x` to it takes from what a node holds, so that it may have to be scaled down.
    pub(super) fn takes(self, x: f64) -> bool {
        x < 0.0 && self.arr < BOUNDARY_ARRAYS
    }
}

/// Per array (`ARRAYS`), two sums for each number.
pub(super) type Sums = [Vec<[f64; 2]>; ARRAYS];

/// One column of a link flux: what crosses each link, from array `arr[0]` (`ARRAYS`) at its `a` end to `arr[1]` at its
/// `b` end, where the number of node `i` is at `at[end] + i`.
#[derive(Clone, Copy)]
pub(super) struct Column<'a> {
    pub(super) arr: [usize; 2],
    at: [usize; 2],
    by_link: &'a [f64],
}

impl<'a> LinkFlux<'a> {
    /// Its columns: one for each substance, then the heat.
    pub(super) fn columns(&self, len: &[usize; BOOKED]) -> Vec<Column<'a>> {
        let (c, links, mol) = (self.class.map(|c| c as usize), self.a.len(), self.mol);
        let amount = |q: usize, k: u16| Column { arr: c.map(|c| 2 * c), at: c.map(|c| k as usize * len[c]), by_link: &mol[q * links..(q + 1) * links] };
        let heat = (!self.heat.is_empty()).then_some(Column { arr: c.map(|c| 2 * c + 1), at: [0; 2], by_link: self.heat });
        self.ks.iter().enumerate().map(|(q, &k)| amount(q, k)).chain(heat).collect()
    }
}

/// The arrays of a column's two ends: one (`One`), or two apart (`Two`), so the loops over a column are compiled for
/// each and the common one, a field of one class, pays nothing for the other.
pub(super) trait Ends<T> {
    /// The number at index `i` of the array of end `end`.
    fn at(&mut self, end: usize, i: usize) -> &mut T;
}

pub(super) struct One<'v, T>(pub(super) &'v mut [T]);
pub(super) struct Two<'v, T>(pub(super) &'v mut [T], pub(super) &'v mut [T]);

impl<T> Ends<T> for One<'_, T> {
    #[inline(always)]
    fn at(&mut self, _: usize, i: usize) -> &mut T {
        &mut self.0[i]
    }
}

impl<T> Ends<T> for Two<'_, T> {
    #[inline(always)]
    fn at(&mut self, end: usize, i: usize) -> &mut T {
        if end == 0 { &mut self.0[i] } else { &mut self.1[i] }
    }
}

/// Runs `$body` with `$e` the ends (`Ends`) of arrays `$arr` of each of `$v`, one array or two.
macro_rules! with_ends {
    ($arr:expr, $(($v:expr, $e:ident)),+ => $body:expr) => {{
        let arr: [usize; 2] = $arr;
        if arr[0] == arr[1] {
            $(let mut $e = One(&mut $v[arr[0]][..]);)+
            $body
        } else {
            $(let [p, q] = $v.get_disjoint_mut(arr).expect("two arrays"); let mut $e = Two(&mut p[..], &mut q[..]);)+
            $body
        }
    }};
}
pub(super) use with_ends;

impl Column<'_> {
    /// What link `l`, from node `a` to node `b`, changes in it: what leaves `a` enters `b`.
    #[inline(always)]
    pub(super) fn deltas(&self, l: usize, a: u32, b: u32) -> [(Slot, f64); 2] {
        let (arr, at, m) = (self.arr, self.at, self.by_link[l]);
        [(Slot { arr: arr[0], at: at[0] + a as usize }, -m), (Slot { arr: arr[1], at: at[1] + b as usize }, m)]
    }
    /// Adds to `net` the net change of each number it touches and all that passes through it, and notes each number
    /// once in `touched` (its index times 2, plus its end); returns their count.
    #[inline(always)]
    pub(super) fn sum(&self, x: &LinkFlux, net: &mut impl Ends<[f64; 2]>, touched: &mut [usize]) -> usize {
        let mut count = 0;
        for (l, (&a, &b)) in x.a.iter().zip(x.b).enumerate() {
            for (end, (s, d)) in self.deltas(l, a, b).into_iter().enumerate() {
                let [sum, through] = net.at(end, s.at);
                touched[count] = 2 * s.at + end;
                count += ((*through == 0.0) & (d != 0.0)) as usize;
                *through += d.abs();
                *sum += d;
            }
        }
        count
    }
}

/// What gives each change its scaled amount: the counts of what scaling left, and the sleep of the nodes it wakes.
pub(super) struct Giver<'w> {
    pub(super) wake_share: f64,
    pub(super) scaled: &'w mut [f64; 2],
    pub(super) woken: &'w mut Vec<Slot>,
    pub(super) sleep: &'w mut Sleep,
    pub(super) table: &'w Nodes,
    pub(super) time: u64,
    pub(super) len: [usize; BOOKED],
}

impl Giver<'_> {
    /// Changes number `s`, `v`, by `x` scaled by `f`, and notes the node to wake when that brings it more than
    /// `wake_share` of what it held of an amount. Heat wakes no node: a node's temperature changes at every step outside
    /// `apply` (radiation, the exchange with the air), and its own alarm bounds how long it reacts at an old one.
    #[inline(always)]
    pub(super) fn give(&mut self, v: &mut f64, s: Slot, x: f64, f: f64) {
        let (held, given) = (*v, x * f);
        *v += given;
        if s.unit() == 0 && given > self.wake_share * held.max(0.0) {
            self.wake(s);
        }
    }
    /// Counts what scaling a change of `x` to number `s` by `f` left in place.
    pub(super) fn left(&mut self, s: Slot, x: f64, f: f64) {
        if f < 1.0 && x < 0.0 {
            self.scaled[s.unit()] -= x * (1.0 - f);
        }
    }
    #[cold]
    fn wake(&mut self, s: Slot) {
        self.woken.push(s);
    }
    /// Wakes the nodes noted (in any order: waking is the same).
    pub(super) fn flush(&mut self) {
        for s in self.woken.drain(..) {
            let c = s.arr / 2;
            let (class, i) = (Class::ALL[c], s.at % self.len[c]);
            self.sleep.wake(class, self.table.id(class, i), self.time);
        }
    }
}

/// Each number the columns of `fluxes` change, with its change, as `f(number, slot, change)`: column by column and link
/// by link in their order (the order in which they are given), or from the last back (`BACK`).
#[inline(always)]
fn each_end<const BACK: bool>(fluxes: &[LinkFlux], columns: &[Vec<Column>], books: &mut [&mut [f64]; ARRAYS], mut f: impl FnMut(&mut f64, Slot, f64)) {
    let at = |k: usize, n: usize| if BACK { n - 1 - k } else { k };
    for xi in 0..fluxes.len() {
        let (x, cols) = (&fluxes[at(xi, fluxes.len())], &columns[at(xi, fluxes.len())]);
        for ci in 0..cols.len() {
            let col = cols[at(ci, cols.len())];
            with_ends!(col.arr, (books, v) => {
                for k in 0..x.a.len() {
                    let l = at(k, x.a.len());
                    let ds = col.deltas(l, x.a[l], x.b[l]);
                    for e in 0..2 {
                        let (s, d) = ds[at(e, 2)];
                        f(v.at(at(e, 2), s.at), s, d);
                    }
                }
            })
        }
    }
}

/// The fast path of `apply` for link fluxes alone: books them unscaled, in the order in which the careful path gives
/// them when nothing is short, keeping the value of each number before each change (`saved`); then looks at the
/// numbers that ended below zero (`low`). One below the rounding of all that passed through it (`NET_ROUNDING`) is
/// short: then every number is set back to what it held and it returns false. Else the rounding below zero is set to
/// zero and counted in `rounded`, as the careful path does, so the result is the same.
pub(super) fn book_first(fluxes: &[LinkFlux], columns: &[Vec<Column>], books: &mut [&mut [f64]; ARRAYS], giver: &mut Giver, (saved, net, low): (&mut Vec<f64>, &mut Sums, &mut Vec<Slot>), rounded: &mut [f64; 2]) -> bool {
    saved.clear();
    low.clear();
    each_end::<false>(fluxes, columns, books, |v, s, d| {
        saved.push(*v);
        giver.give(v, s, d, 1.0);
    });
    // Each number below zero once (marked in `net`), then all that passed through it.
    each_end::<false>(fluxes, columns, books, |v, s, _| {
        if *v < 0.0 && net[s.arr][s.at][0] == 0.0 {
            net[s.arr][s.at][0] = 1.0;
            low.push(s);
        }
    });
    if low.is_empty() {
        return true;
    }
    each_end::<false>(fluxes, columns, books, |_, s, d| {
        let m = &mut net[s.arr][s.at];
        m[1] += if m[0] != 0.0 { d.abs() } else { 0.0 };
    });
    let mut short = false;
    for s in low.iter() {
        let [_, all] = std::mem::take(&mut net[s.arr][s.at]);
        short |= books[s.arr][s.at] < -NET_ROUNDING * all;
    }
    if short {
        let mut i = saved.len();
        each_end::<true>(fluxes, columns, books, |v, _, _| {
            i -= 1;
            *v = saved[i];
        });
        return false;
    }
    low.sort_unstable();
    for &s in low.iter() {
        rounded[s.unit()] -= std::mem::take(&mut books[s.arr][s.at]);
    }
    true
}

