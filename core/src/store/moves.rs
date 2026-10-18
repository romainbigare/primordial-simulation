//! The three ways a law changes the world (`docs/foundation/plan.md`, Idea 2): a move between two nodes, a transform
//! of the amounts inside one node, and a link flux over a whole field; and `apply`, which books them.

use super::nodes::{Class, NodeId, Nodes, BOUNDARIES};
use crate::chem::Chemistry;
use crate::clock::Sleep;
use crate::ground::share;
use crate::World;

/// What a transform may lose or gain of an element to rounding, relative to all the elements it changes.
const ELEMENT_ROUNDING: f64 = 1e-9;
/// A link flux's net change of a number within this share of all that passes through it is the rounding of its sum, not
/// a change: liquid that passes through a dry cell would else take from it what it never held.
const NET_ROUNDING: f64 = 1e-12;
/// The classes `apply` books, the first ones: those whose amounts lie in arrays of `World` (a part's lie in the part,
/// `Class::Part`, which Bond books).
const BOOKED: usize = Class::Part as usize;

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
const ARRAYS: usize = 2 * BOOKED;
/// The first array of the boundary nodes, which give all they are asked (`Class::Boundary`).
const BOUNDARY_ARRAYS: usize = 2 * Class::Boundary as usize;

/// One number of the world: entry `at` of array `arr` (`ARRAYS`); an amount is at `k * len + index`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Slot {
    arr: usize,
    at: usize,
}

impl Slot {
    /// 0 for an amount, 1 for a heat: its place in `Counters::scaled` and `Counters::rounded`.
    fn unit(self) -> usize {
        self.arr % 2
    }
    /// Whether a change of `x` to it takes from what a node holds, so that it may have to be scaled down.
    fn takes(self, x: f64) -> bool {
        x < 0.0 && self.arr < BOUNDARY_ARRAYS
    }
}

/// The working space of `apply`, kept between calls so that a call costs only the numbers it changes.
#[derive(Clone, Debug, Default)]
pub struct Booking {
    /// Per array (`ARRAYS`): what the changes ask of each number, then the share of it the number can give; 0 between
    /// calls
    need: [Vec<f64>; ARRAYS],
    /// The numbers the call asks of, in the order it first asks (the first ones: room for one per number a change may
    /// ask of, two per link of a flux's column)
    asked: Vec<Slot>,
    /// Per array: the net change a flux's column makes to each number, and all that passes through it; 0 between columns
    net: [Vec<[f64; 2]>; ARRAYS],
    /// The numbers a column touches, at their first change (room for two per link)
    touched: Vec<usize>,
    /// The numbers a flux takes from (by flux: up to `ends[x]`), and those whose net change is rounding (`NET_ROUNDING`)
    takes: Vec<Slot>,
    ends: Vec<usize>,
    passed: Vec<Slot>,
    /// Per flux: what it takes from the nodes, mol and J (`Slot::unit`)
    lost: Vec<[f64; 2]>,
    /// Room for the element totals of a transform
    sums: Vec<f64>,
}

/// Adds to `n`, what the changes ask of number `s`, what a change of `x` takes from it, and notes `s` as the
/// `count`-th number asked the first time (written always and counted only then, so no branch is mispredicted).
#[inline(always)]
fn ask(n: &mut f64, s: Slot, x: f64, asked: &mut [Slot], count: &mut usize) {
    asked[*count] = s;
    *count += ((x < 0.0) & (*n == 0.0)) as usize;
    *n -= x.min(0.0);
}

/// One column of a link flux: what crosses each link, from array `arr[0]` (`ARRAYS`) at its `a` end to `arr[1]` at its
/// `b` end, where the number of node `i` is at `at[end] + i`.
#[derive(Clone, Copy)]
struct Column<'a> {
    arr: [usize; 2],
    at: [usize; 2],
    by_link: &'a [f64],
}

impl<'a> LinkFlux<'a> {
    /// Its columns: one for each substance, then the heat.
    fn columns(&self, len: &[usize; BOOKED]) -> Vec<Column<'a>> {
        let (c, links, mol) = (self.class.map(|c| c as usize), self.a.len(), self.mol);
        let amount = |q: usize, k: u16| Column { arr: c.map(|c| 2 * c), at: c.map(|c| k as usize * len[c]), by_link: &mol[q * links..(q + 1) * links] };
        let heat = (!self.heat.is_empty()).then_some(Column { arr: c.map(|c| 2 * c + 1), at: [0; 2], by_link: self.heat });
        self.ks.iter().enumerate().map(|(q, &k)| amount(q, k)).chain(heat).collect()
    }
}

/// The arrays of a column's two ends: one (`One`), or two apart (`Two`), so the loops over a column are compiled for
/// each and the common one, a field of one class, pays nothing for the other.
trait Ends<T> {
    /// The number at index `i` of the array of end `end`.
    fn at(&mut self, end: usize, i: usize) -> &mut T;
}

struct One<'v, T>(&'v mut [T]);
struct Two<'v, T>(&'v mut [T], &'v mut [T]);

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

impl Column<'_> {
    /// What link `l`, from node `a` to node `b`, changes in it: what leaves `a` enters `b`.
    #[inline(always)]
    fn deltas(&self, l: usize, a: u32, b: u32) -> [(Slot, f64); 2] {
        let (arr, at, m) = (self.arr, self.at, self.by_link[l]);
        [(Slot { arr: arr[0], at: at[0] + a as usize }, -m), (Slot { arr: arr[1], at: at[1] + b as usize }, m)]
    }
    /// Adds to `net` the net change of each number it touches and all that passes through it, and notes each number
    /// once in `touched` (its index times 2, plus its end); returns their count.
    #[inline(always)]
    fn sum(&self, x: &LinkFlux, net: &mut impl Ends<[f64; 2]>, touched: &mut [usize]) -> usize {
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

/// What the numbers of a column, `touched` by it (`Column::sum`), ask: each once, without a branch, a net loss is
/// asked (but of a boundary node) and noted in `takes` (from `nt` on), a change within rounding is noted in `passed`
/// (from `np` on). Returns what the column takes from its nodes, and the counts `nt` and `np` after it.
#[inline(always)]
fn ask_column(col: &Column, touched: &[usize], (need, net): (&mut impl Ends<f64>, &mut impl Ends<[f64; 2]>), (asked, count): (&mut [Slot], &mut usize), (takes, passed): (&mut [Slot], &mut [Slot]), (mut nt, mut np): (usize, usize)) -> (f64, usize, usize) {
    let mut loss = 0.0;
    for &t in touched {
        let (end, at) = (t % 2, t / 2);
        let (s, [d, all]) = (Slot { arr: col.arr[end], at }, std::mem::take(net.at(end, at)));
        let (lose, round) = (d < -NET_ROUNDING * all && s.arr < BOUNDARY_ARRAYS, d.abs() <= NET_ROUNDING * all);
        let d = if lose { d } else { 0.0 };
        ask(need.at(end, at), s, d, asked, count);
        (takes[nt], passed[np]) = (s, s);
        (nt, np, loss) = (nt + lose as usize, np + round as usize, loss - d);
    }
    (loss, nt, np)
}

/// A move or a transform.
#[derive(Clone, Copy)]
enum Item<'i> {
    Move(&'i Move),
    Transform(&'i Transform),
}

/// Each move, then each transform, in their order.
fn each_item<'i>(moves: &'i [Move], transforms: &'i [Transform], f: impl FnMut(Item<'i>)) {
    moves.iter().map(Item::Move).chain(transforms.iter().map(Item::Transform)).for_each(f);
}

/// Where the amounts and heat of node `id` lie (`Nodes::place`), in a class `apply` books.
fn booked(table: &Nodes, id: NodeId) -> (Class, usize) {
    let p = table.place(id);
    assert!((p.0 as usize) < BOOKED, "apply books no node of {:?}", p.0);
    p
}

/// Each number `item` changes, with its change unscaled, in order.
fn each_delta(item: Item, table: &Nodes, chem: &Chemistry, len: &[usize; BOOKED], mut f: impl FnMut(Slot, f64)) {
    let amount = |c: usize, at| Slot { arr: 2 * c, at };
    let heat = |c: usize, at| Slot { arr: 2 * c + 1, at };
    match item {
        Item::Move(m) => {
            let (((cf, i), (ct, j)), k) = ((booked(table, m.from), booked(table, m.to)), m.k as usize);
            let lift = m.mol * (held(chem, ct, k) - held(chem, cf, k));
            let (cf, ct) = (cf as usize, ct as usize);
            f(amount(cf, k * len[cf] + i), -m.mol);
            f(heat(cf, i), -m.heat);
            f(amount(ct, k * len[ct] + j), m.mol);
            f(heat(ct, j), m.heat - lift);
        }
        Item::Transform(t) => {
            let ((class, i), mut q) = (booked(table, t.at), 0.0);
            let c = class as usize;
            for &(k, d) in &t.dmol {
                f(amount(c, k as usize * len[c] + i), d);
                q -= d * held(chem, class, k as usize);
            }
            f(heat(c, i), q);
        }
    }
}

/// What gives each change its scaled amount: the counts of what scaling left, and the sleep of the nodes it wakes.
struct Giver<'w> {
    wake_share: f64,
    scaled: &'w mut [f64; 2],
    sleep: &'w mut Sleep,
    table: &'w Nodes,
    time: u64,
    len: [usize; BOOKED],
}

impl Giver<'_> {
    /// Changes number `s`, `v`, by `x` scaled by `f`, and wakes the node when that brings it more than `wake_share`
    /// of what it held.
    #[inline(always)]
    fn give(&mut self, v: &mut f64, s: Slot, x: f64, f: f64) {
        let (held, given) = (*v, x * f);
        *v += given;
        if given > self.wake_share * held.max(0.0) {
            self.wake(s);
        }
    }
    /// Counts what scaling a change of `x` to number `s` by `f` left in place.
    fn left(&mut self, s: Slot, x: f64, f: f64) {
        if f < 1.0 && x < 0.0 {
            self.scaled[s.unit()] -= x * (1.0 - f);
        }
    }
    #[cold]
    fn wake(&mut self, s: Slot) {
        let c = s.arr / 2;
        let (class, i) = (Class::ALL[c], if s.unit() == 0 { s.at % self.len[c] } else { s.at });
        self.sleep.wake(class, self.table.id(class, i), self.time);
    }
}

/// Gives each change of the links of flux `x` (of columns `cols`) to `books`, scaled by `f`, one column at a time.
#[inline(always)]
fn give_links(x: &LinkFlux, cols: &[Column], books: &mut [&mut [f64]; ARRAYS], giver: &mut Giver, f: f64) {
    for &col in cols {
        with_ends!(col.arr, (books, v) => {
            for (l, (&a, &b)) in x.a.iter().zip(x.b).enumerate() {
                for (end, (s, d)) in col.deltas(l, a, b).into_iter().enumerate() {
                    giver.give(v.at(end, s.at), s, d, f);
                }
            }
        })
    }
}

/// Does `dmol` keep every element of the chemistry's table, to rounding? `sums` is room for the element totals.
fn keeps_elements(chem: &Chemistry, dmol: &[(u16, f64)], sums: &mut Vec<f64>) -> bool {
    sums.clear();
    sums.resize(chem.n_elements, 0.0);
    let mut size = 0.0;
    for &(k, d) in dmol {
        chem.add_elems(k as usize, d, sums);
        size += d.abs() * chem.subs[k as usize].elems.iter().map(|&c| c as f64).sum::<f64>();
    }
    sums.iter().all(|e| e.abs() <= ELEMENT_ROUNDING * size)
}

/// Book the link fluxes, then the moves, then the transforms, each in its order. A link flux takes from a node only its
/// net loss over all the flux's links (a net change within `NET_ROUNDING` of what passes through is none), and is
/// scaled as a whole: by the least share over what it takes from. Where all that would leave a number
/// (an amount of a node, or its heat) is more than it holds, each change that takes from it is scaled down by the
/// share it holds (the least share over all it takes from), so no amount and no heat falls below zero
/// (`docs/handover.md` 6.1); a boundary node gives all it is asked. What the scaling left in place is counted in
/// `Counters::scaled`, and the last rounding below zero, set to zero, in `Counters::rounded`. A change that brings a
/// node more than `Config::wake_share` of what it held of that amount or heat wakes the node (`clock::Sleep::wake`; a
/// boundary node never sleeps). Panics on a transform that does not keep its elements, and on the id of a node that is
/// gone.
///
/// It works on the numbers the changes touch alone (`Booking`), and books each flux by its columns, one array at a
/// time: each number still sees its changes in their order, so the result is the same.
pub fn apply(w: &mut World, fluxes: &[LinkFlux], moves: &[Move], transforms: &[Transform]) {
    w.fit_deep();
    let World { cfg, chem, table, sleep, time, counts, grid, booking, ground, e_ground, air, e_air, deep, e_deep, boundary, e_boundary, .. } = w;
    let (len, Booking { need, asked, net, touched, takes, ends, passed, lost, sums }) = ([grid.n(), grid.na(), grid.n(), BOUNDARIES], booking);
    let mut books = [&mut ground[..], &mut e_ground[..], &mut air[..], &mut e_air[..], &mut deep[..], &mut e_deep[..], &mut boundary[..], &mut e_boundary[..]];
    for ((n, s), b) in need.iter_mut().zip(net.iter_mut()).zip(&books) {
        if n.len() != b.len() {
            (*n, *s) = (vec![0.0; b.len()], vec![[0.0; 2]; b.len()]);
        }
    }
    for x in fluxes {
        let (links, once) = (x.a.len(), x.ks.iter().enumerate().all(|(q, k)| !x.ks[..q].contains(k)));
        let classes = x.class.iter().all(|&c| (c as usize) < BOOKED) && (x.class[0] == Class::Air) == (x.class[1] == Class::Air);
        let shaped = classes && x.b.len() == links && x.mol.len() == x.ks.len() * links && (x.heat.is_empty() || x.heat.len() == links);
        assert!(shaped && once, "a link flux of the wrong shape");
    }
    for t in transforms {
        assert!(keeps_elements(chem, &t.dmol, sums), "a transform that does not keep its elements: {t:?}");
    }
    // What each change asks of each number.
    let changes = fluxes.iter().map(|x| 2 * x.a.len() * (x.ks.len() + 1)).sum::<usize>() + 4 * moves.len();
    let changes = changes + transforms.iter().map(|t| t.dmol.len() + 1).sum::<usize>();
    if asked.len() < changes {
        asked.resize(changes, Slot { arr: 0, at: 0 });
    }
    let (columns, mut count): (Vec<_>, _) = (fluxes.iter().map(|x| x.columns(&len)).collect(), 0);
    ends.clear();
    lost.clear();
    let (mut nt, mut np) = (0, 0);
    for (x, cols) in fluxes.iter().zip(&columns) {
        lost.push([0.0; 2]);
        for &col in cols {
            if touched.len() <= 2 * x.a.len() {
                touched.resize(2 * x.a.len() + 1, 0);
            }
            let loss = with_ends!(col.arr, (need, n), (net, m) => {
                let count_touched = col.sum(x, &mut m, touched);
                let room = nt.max(np) + count_touched + 1;
                for v in [&mut *takes, &mut *passed].into_iter().filter(|v| v.len() < room) {
                    v.resize(room, Slot { arr: 0, at: 0 });
                }
                let loss;
                (loss, nt, np) = ask_column(&col, &touched[..count_touched], (&mut n, &mut m), (asked, &mut count), (takes, passed), (nt, np));
                loss
            });
            lost.last_mut().unwrap()[col.arr[0] % 2] += loss;
        }
        ends.push(nt);
    }
    each_item(moves, transforms, |item| {
        each_delta(item, table, chem, &len, |s, x| {
            if s.arr < BOUNDARY_ARRAYS {
                ask(&mut need[s.arr][s.at], s, x, asked, &mut count);
            }
        })
    });
    // The share of it each number can give.
    let (asked, mut whole) = (&asked[..count], true);
    for s in asked {
        let n = &mut need[s.arr][s.at];
        *n = share(books[s.arr][s.at].max(0.0), *n);
        whole &= *n == 1.0;
    }
    // Each change, scaled by the least share of what it takes from (by none when each number gives all it is asked).
    let mut giver = Giver { wake_share: cfg.wake_share, scaled: &mut counts.scaled, sleep, table, time: *time, len };
    let mut start = 0;
    for (((x, cols), &end), lost) in fluxes.iter().zip(&columns).zip(&*ends).zip(&*lost) {
        let f = if whole { 1.0 } else { takes[start..end].iter().fold(1.0, |f: f64, s| f.min(need[s.arr][s.at])) };
        start = end;
        give_links(x, cols, &mut books, &mut giver, f);
        // What the scaling left in place: the share of the net losses it did not take.
        if f < 1.0 {
            (0..2).for_each(|u| giver.scaled[u] += lost[u] * (1.0 - f));
        }
    }
    each_item(moves, transforms, |item| {
        let mut f: f64 = 1.0;
        if !whole {
            each_delta(item, table, chem, &len, |s, x| {
                if s.takes(x) {
                    f = f.min(need[s.arr][s.at]);
                }
            });
        }
        each_delta(item, table, chem, &len, |s, x| {
            giver.give(&mut books[s.arr][s.at], s, x, f);
            giver.left(s, x, f);
        });
    });
    // What rounding left just below zero, set to zero and counted in the order of the arrays.
    let mut below = Vec::new();
    for &s in asked {
        need[s.arr][s.at] = 0.0;
    }
    for &s in asked.iter().chain(&passed[..np]) {
        let v = &mut books[s.arr][s.at];
        if *v < 0.0 {
            below.push((s, *v));
            *v = 0.0;
        }
    }
    below.sort_unstable_by_key(|e| e.0);
    for (s, v) in below {
        counts.rounded[s.unit()] -= v;
    }
}
