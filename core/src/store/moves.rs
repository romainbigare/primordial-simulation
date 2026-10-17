//! The three ways a law changes the world (`docs/foundation/plan.md`, Idea 2): a move between two nodes, a transform
//! of the amounts inside one node, and a link flux over a whole field; and `apply`, which books them.

use super::nodes::{Class, NodeId, Nodes, BOUNDARIES, CLASSES};
use crate::chem::Chemistry;
use crate::clock::Sleep;
use crate::ground::share;
use crate::World;

/// What a transform may lose or gain of an element to rounding, relative to all the elements it changes.
const ELEMENT_ROUNDING: f64 = 1e-9;

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

/// What crosses each link of a field of nodes of one class at once: link `l` joins the nodes at index `a[l]` and
/// `b[l]` and carries `mol[q * links + l]` mol of substance `ks[q]` (each substance once) and `heat[l]` J, + from `a`
/// to `b` (`heat` empty: no heat). What leaves one end enters the other, so a flux keeps the books by its form.
#[derive(Clone, Copy, Debug)]
pub struct LinkFlux<'a> {
    pub class: Class,
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
const ARRAYS: usize = 2 * CLASSES;
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
    /// The numbers the call asks of, in the order it first asks (the first ones: room for one per change it may make)
    asked: Vec<Slot>,
    /// Per link of a flux: the least share over what it takes from
    least: Vec<f64>,
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

/// One column of a link flux: what crosses each link in one array (`ARRAYS`), where the number of node `i` is at
/// `at + i`.
#[derive(Clone, Copy)]
struct Column<'a> {
    arr: usize,
    at: usize,
    by_link: &'a [f64],
}

impl<'a> LinkFlux<'a> {
    /// Its columns: one for each substance, then the heat.
    fn columns(&self, len: &[usize; CLASSES]) -> Vec<Column<'a>> {
        let (c, links, at, mol) = (self.class as usize, self.a.len(), len[self.class as usize], self.mol);
        let amounts = self.ks.iter().enumerate().map(|(q, &k)| Column { arr: 2 * c, at: k as usize * at, by_link: &mol[q * links..(q + 1) * links] });
        amounts.chain((!self.heat.is_empty()).then_some(Column { arr: 2 * c + 1, at: 0, by_link: self.heat })).collect()
    }
}

impl Column<'_> {
    /// What link `l`, from node `a` to node `b`, changes in it: what leaves `a` enters `b`.
    #[inline(always)]
    fn deltas(&self, l: usize, a: u32, b: u32) -> [(Slot, f64); 2] {
        let (arr, m) = (self.arr, self.by_link[l]);
        [(Slot { arr, at: self.at + a as usize }, -m), (Slot { arr, at: self.at + b as usize }, m)]
    }
    /// The change of link `l` that takes from its node: the one below zero, or else one that takes nothing.
    #[inline(always)]
    fn take(&self, l: usize, a: u32, b: u32) -> (Slot, f64) {
        let [from, to] = self.deltas(l, a, b);
        if from.1 < 0.0 { from } else { to }
    }
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

/// Each number `item` changes, with its change unscaled, in order.
fn each_delta(item: Item, table: &Nodes, chem: &Chemistry, len: &[usize; CLASSES], mut f: impl FnMut(Slot, f64)) {
    let amount = |c: usize, at| Slot { arr: 2 * c, at };
    let heat = |c: usize, at| Slot { arr: 2 * c + 1, at };
    match item {
        Item::Move(m) => {
            let (((cf, i), (ct, j)), k) = ((table.place(m.from), table.place(m.to)), m.k as usize);
            let lift = m.mol * (held(chem, ct, k) - held(chem, cf, k));
            let (cf, ct) = (cf as usize, ct as usize);
            f(amount(cf, k * len[cf] + i), -m.mol);
            f(heat(cf, i), -m.heat);
            f(amount(ct, k * len[ct] + j), m.mol);
            f(heat(ct, j), m.heat - lift);
        }
        Item::Transform(t) => {
            let ((class, i), mut q) = (table.place(t.at), 0.0);
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
    len: [usize; CLASSES],
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

/// Gives each change of the links of flux `x` (of columns `cols`) to `books`, link `l`'s scaled by `f(l)`, one column
/// at a time.
#[inline(always)]
fn give_links(x: &LinkFlux, cols: &[Column], books: &mut [&mut [f64]; ARRAYS], giver: &mut Giver, f: impl Fn(usize) -> f64) {
    for &col in cols {
        let v = &mut books[col.arr][..];
        for (l, (&a, &b)) in x.a.iter().zip(x.b).enumerate() {
            col.deltas(l, a, b).into_iter().for_each(|(s, d)| giver.give(&mut v[s.at], s, d, f(l)));
        }
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

/// Book the link fluxes, then the moves, then the transforms, each in its order. Where all that would leave a number
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
    let (len, Booking { need, asked, least, sums }) = ([grid.n(), grid.na(), grid.n(), BOUNDARIES], booking);
    let mut books = [&mut ground[..], &mut e_ground[..], &mut air[..], &mut e_air[..], &mut deep[..], &mut e_deep[..], &mut boundary[..], &mut e_boundary[..]];
    for (n, b) in need.iter_mut().zip(&books) {
        if n.len() != b.len() {
            *n = vec![0.0; b.len()];
        }
    }
    for x in fluxes {
        let (links, once) = (x.a.len(), x.ks.iter().enumerate().all(|(q, k)| !x.ks[..q].contains(k)));
        let shaped = x.b.len() == links && x.mol.len() == x.ks.len() * links && (x.heat.is_empty() || x.heat.len() == links);
        assert!(shaped && once, "a link flux of the wrong shape");
    }
    for t in transforms {
        assert!(keeps_elements(chem, &t.dmol, sums), "a transform that does not keep its elements: {t:?}");
    }
    // What each change asks of each number.
    let changes = fluxes.iter().map(|x| x.a.len() * (x.ks.len() + 1)).sum::<usize>() + 4 * moves.len();
    let changes = changes + transforms.iter().map(|t| t.dmol.len() + 1).sum::<usize>();
    if asked.len() < changes {
        asked.resize(changes, Slot { arr: 0, at: 0 });
    }
    let (columns, mut count): (Vec<_>, _) = (fluxes.iter().map(|x| x.columns(&len)).collect(), 0);
    for (x, cols) in fluxes.iter().zip(&columns).filter(|(x, _)| x.class != Class::Boundary) {
        for &col in cols {
            let n = &mut need[col.arr][..];
            for (l, (&a, &b)) in x.a.iter().zip(x.b).enumerate() {
                let (s, d) = col.take(l, a, b);
                ask(&mut n[s.at], s, d, asked, &mut count);
            }
        }
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
    for (x, cols) in fluxes.iter().zip(&columns) {
        if whole {
            give_links(x, cols, &mut books, &mut giver, |_| 1.0);
            continue;
        }
        least.clear();
        least.resize(x.a.len(), 1.0);
        for col in cols {
            for (l, (&a, &b)) in x.a.iter().zip(x.b).enumerate() {
                let (s, d) = col.take(l, a, b);
                if s.takes(d) {
                    least[l] = least[l].min(need[s.arr][s.at]);
                }
            }
        }
        give_links(x, cols, &mut books, &mut giver, |l| least[l]);
        // What the scaling left in place, counted link by link.
        for (l, (&a, &b)) in x.a.iter().zip(x.b).enumerate() {
            cols.iter().flat_map(|col| col.deltas(l, a, b)).for_each(|(s, d)| giver.left(s, d, least[l]));
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
