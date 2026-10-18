//! `apply`: books the moves, transforms and link fluxes of a call, first unscaled where that is safe (`moves::book_first`),
//! else with care: each number gives only what it holds.

use super::moves::{book_first, with_ends, Column, Ends, Giver, LinkFlux, Move, One, Slot, Span, Sums, Transform, Two, ARRAYS, BOOKED, BOUNDARY_ARRAYS, NET_ROUNDING};
use super::nodes::{Class, NodeId, Nodes, BOUNDARIES};
use super::held;
use crate::chem::Chemistry;
use crate::ground::share;
use crate::{World, AIR};

/// What a transform may lose or gain of an element to rounding, relative to all the elements it changes.
const ELEMENT_ROUNDING: f64 = 1e-9;

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
    net: Sums,
    /// The numbers a column touches, at their first change (room for two per link)
    touched: Vec<usize>,
    /// The numbers a flux takes from (by flux: up to `ends[x]`), and those whose net change is rounding (`NET_ROUNDING`)
    takes: Vec<Slot>,
    ends: Vec<usize>,
    passed: Vec<Slot>,
    /// Per flux: what it takes from the nodes, mol and J (`Slot::unit`)
    lost: Vec<[f64; 2]>,
    /// The numbers the call's link fluxes may change as they were before it, and the nodes to wake (`Giver::flush`)
    spans: Vec<Span>,
    kept: Vec<f64>,
    woken: Vec<Slot>,
    /// Room for the element totals of a transform, and the atoms of each substance (the sum of its element counts)
    sums: Vec<f64>,
    atoms: Vec<f64>,
}

/// Adds to `n`, what the changes ask of number `s`, what a change of `x` takes from it, and notes `s` as the
/// `count`-th number asked the first time (written always and counted only then, so no branch is mispredicted).
#[inline(always)]
fn ask(n: &mut f64, s: Slot, x: f64, asked: &mut [Slot], count: &mut usize) {
    asked[*count] = s;
    *count += ((x < 0.0) & (*n == 0.0)) as usize;
    *n -= x.min(0.0);
}

/// What the numbers of a column, `touched` by it (`Column::sum`), ask: each once, without a branch, a net loss is
/// asked and noted in `takes` (from `nt` on), a change within rounding is noted in `passed` (from `np` on), but of a
/// node that gives all it is asked. Returns what the column takes from its nodes, and the counts `nt` and `np` after it.
#[inline(always)]
fn ask_column(col: &Column, touched: &[usize], (need, net): (&mut impl Ends<f64>, &mut impl Ends<[f64; 2]>), (asked, count): (&mut [Slot], &mut usize), (takes, passed): (&mut [Slot], &mut [Slot]), (mut nt, mut np): (usize, usize)) -> (f64, usize, usize) {
    let mut loss = 0.0;
    for &t in touched {
        let (end, at) = (t % 2, t / 2);
        let (s, [d, all]) = (Slot { arr: col.arr[end], at }, std::mem::take(net.at(end, at)));
        let mine = s.arr < BOUNDARY_ARRAYS;
        let (lose, round) = (d < -NET_ROUNDING * all && mine, d.abs() <= NET_ROUNDING * all && mine);
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

/// Gives each change of the links of flux `x` (of columns `cols`) to `books`, scaled by `f`, one column at a time.
#[inline(always)]
fn give_links(x: &LinkFlux, cols: &[Column], books: &mut [&mut [f64]; ARRAYS], f: f64) {
    for &col in cols {
        with_ends!(col.arr, (books, v) => {
            for (l, (&a, &b)) in x.a.iter().zip(x.b).enumerate() {
                for (end, (s, d)) in col.deltas(l, a, b).into_iter().enumerate() {
                    *v.at(end, s.at) += d * f;
                }
            }
        })
    }
}

/// Does `dmol` keep every element of the chemistry's table, to rounding? `sums` is room for the element totals, `atoms`
/// the atoms of each substance.
fn keeps_elements(chem: &Chemistry, dmol: &[(u16, f64)], sums: &mut Vec<f64>, atoms: &[f64]) -> bool {
    sums.clear();
    sums.resize(chem.n_elements, 0.0);
    let mut size = 0.0;
    for &(k, d) in dmol {
        chem.add_elems(k as usize, d, sums);
        size += d.abs() * atoms[k as usize];
    }
    sums.iter().all(|e| e.abs() <= ELEMENT_ROUNDING * size)
}

/// Book the link fluxes, then the moves, then the transforms, each in its order. A link flux takes from a node only its
/// net loss over all the flux's links (a net change within `NET_ROUNDING` of what passes through is none), and is
/// scaled as a whole: by the least share over what it takes from. Where all that would leave a number (an amount of a
/// node, or its heat) is more than it holds, each change that takes from it is scaled down by the share it holds (the
/// least share over all it takes from), so no amount and no heat falls below zero (`docs/handover.md` 6.1); a boundary
/// node and the potential give all they are asked. What the scaling left in place is counted in `Counters::scaled`, and
/// the last rounding below zero, set to zero, in `Counters::rounded`. A call that brings a node, net, more than
/// `Config::wake_share` of what it held of an amount its link fluxes may change wakes the node, as does a move or a
/// transform that alone does (`clock::Sleep::wake`, `Giver::flush`). Panics on a transform that does not keep its elements, and on the id of a node that is
/// gone.
///
/// It works on the numbers the changes touch alone (`Booking`), and books each flux by its columns, one array at a
/// time: each number still sees its changes in their order, so the result is the same. Link fluxes alone are booked
/// first and checked after (`book_first`), and booked with care only where that finds a number short.
pub fn apply(w: &mut World, fluxes: &[LinkFlux], moves: &[Move], transforms: &[Transform]) {
    w.fit_deep();
    let World { cfg, chem, table, sleep, time, counts, mesh, booking, ground, e_ground, air, e_air, deep, e_deep, boundary, e_boundary, potential, .. } = w;
    let (len, Booking { need, asked, net, touched, takes, ends, passed, lost, spans, kept, woken, sums, atoms }) = ([mesh.cells().n(), mesh.level(AIR).n(), mesh.cells().n(), BOUNDARIES, 1], booking);
    let mut books = [&mut ground[..], &mut e_ground[..], &mut air[..], &mut e_air[..], &mut deep[..], &mut e_deep[..], &mut boundary[..], &mut e_boundary[..], &mut [], std::slice::from_mut(potential)];
    for ((n, s), b) in need.iter_mut().zip(net.iter_mut()).zip(&books) {
        if n.len() != b.len() {
            (*n, *s) = (vec![0.0; b.len()], vec![[0.0; 2]; b.len()]);
        }
    }
    for x in fluxes {
        let (links, once) = (x.a.len(), x.ks.iter().enumerate().all(|(q, k)| !x.ks[..q].contains(k)));
        let heat = if x.alike(chem) { x.heat.is_empty() || x.heat.len() == links } else { x.heat.len() == links };
        let shaped = x.class.iter().all(|&c| (c as usize) < BOOKED) && x.b.len() == links && x.mol.len() == x.ks.len() * links && heat;
        assert!(shaped && once, "a link flux of the wrong shape");
    }
    if !transforms.is_empty() {
        atoms.clear();
        atoms.extend(chem.subs.iter().map(|s| s.elems.iter().map(|&c| c as f64).sum::<f64>()));
    }
    for t in transforms {
        assert!(keeps_elements(chem, &t.dmol, sums, atoms), "a transform that does not keep its elements: {t:?}");
    }
    let lifted: Vec<_> = fluxes.iter().map(|x| x.lifted(chem)).collect();
    let columns: Vec<_> = fluxes.iter().zip(&lifted).map(|(x, q)| x.columns(&len, q)).collect();
    let asleep = std::array::from_fn(|c| !sleep.none(Class::ALL[c]));
    let mut giver = Giver { wake_share: cfg.wake_share, scaled: &mut counts.scaled, spans, kept, woken, asleep, sleep, table, time: *time, len };
    giver.keep(&books, &columns);
    if moves.is_empty() && transforms.is_empty() && book_first(fluxes, &columns, &mut books, &giver, (net, passed), &mut counts.rounded) {
        giver.flush(&books);
        return;
    }
    // What each change asks of each number.
    let changes = fluxes.iter().map(|x| 2 * x.a.len() * (x.ks.len() + 1)).sum::<usize>() + 4 * moves.len();
    let changes = changes + transforms.iter().map(|t| t.dmol.len() + 1).sum::<usize>();
    if asked.len() < changes {
        asked.resize(changes, Slot { arr: 0, at: 0 });
    }
    let mut count = 0;
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
    let mut start = 0;
    for (((x, cols), &end), lost) in fluxes.iter().zip(&columns).zip(&*ends).zip(&*lost) {
        let f = if whole { 1.0 } else { takes[start..end].iter().fold(1.0, |f: f64, s| f.min(need[s.arr][s.at])) };
        start = end;
        give_links(x, cols, &mut books, f);
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
            giver.give_item(&mut books[s.arr][s.at], s, x, f);
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
    giver.flush(&books);
}