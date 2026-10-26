//! Booking by array: many sources of changes (the views of the domains that ran, the rows of a law) are added to the
//! store with one task for each block of its arrays (the amounts of a few substances of a class, or its heat), so each
//! number has one writer (CLAUDE.md 3.6) and gets its changes in the order of the sources, whatever thread runs the
//! task. It adds as `apply` adds, counts the rounding it sets to zero and checks it as `apply` checks. Here too the
//! booking of the moves the domains gathered (`World::book_moves`).

use super::apply::{add_carried, overdraw_check, zero_rounding};
use super::{Class, Moves, Store};
use crate::{par::SERIAL_BELOW, World};
use std::ops::Range;

/// Substances whose amounts make one booking task on the pool: a few, so a task is worth its start and each thread
/// still gets several.
const PER_TASK: usize = 4;

/// A booking task: the arrays of one class of nodes it owns, the amounts of substances `ks` (`[k * len + index]`, mol)
/// or with `ks` none the heat (`[index]`, J), with what rounding took of each amount (a carried class, `Store::carry`);
/// the numbers that fell below zero on the way, and what rounding left below zero, set to zero at the task's end. In
/// debug builds, per number the sum of its changes and of their sizes, and what each node of the class held of all
/// substances before the booking (`apply::overdraw_check`).
pub struct Booker<'s> {
    pub class: Class,
    pub ks: Option<Range<usize>>,
    /// The count of nodes of the class
    pub len: usize,
    /// Its numbers, from `[ks.start * len]`
    v: &'s mut [f64],
    carry: Option<&'s mut [f64]>,
    from: usize,
    low: Vec<usize>,
    rounded: f64,
    changed: Vec<[f64; 2]>,
    node: &'s [f64],
}

impl Booker<'_> {
    /// Number `x` of the class's array (`[k * len + index]` of an amount, `[index]` of the heat) changes by `d`, as
    /// `apply` adds it (`add_carried`). Only what it holds at the end must be at zero or above.
    #[inline(always)]
    pub fn add(&mut self, x: usize, d: f64) {
        let x = x - self.from;
        let was = self.v[x];
        add_carried(&mut self.v[x], self.carry.as_deref_mut().map(|c| &mut c[x]), d);
        if self.v[x] < 0.0 && was >= 0.0 {
            self.low.push(x);
        }
        if cfg!(debug_assertions) {
            self.changed[x] = [self.changed[x][0] + d, self.changed[x][1] + d.abs()];
        }
    }
    /// Each number `at(i)` changes by `sign` x `d[i]`, but where `d[i]` is zero (`add`).
    #[inline]
    pub fn column(&mut self, at: impl Fn(usize) -> usize, d: &[f64], sign: f64) {
        for (i, &d) in d.iter().enumerate().filter(|(_, d)| **d != 0.0) {
            self.add(at(i), sign * d);
        }
    }
    /// What rounding left below zero is set to zero and counted; more than rounding is a law that overdrew a node
    /// (checked in debug builds, against what the node held of all substances, or its heat).
    fn zero_low(&mut self) {
        for &x in &self.low {
            if cfg!(debug_assertions) && self.v[x] < 0.0 {
                let node = if self.ks.is_some() { self.node[(x + self.from) % self.len] } else { self.v[x].abs() };
                overdraw_check(self.v[x], node, self.changed[x], (self.class, self.ks.is_some(), x + self.from));
            }
            zero_rounding(&mut self.v[x], &mut self.rounded);
        }
    }
}

/// Book into the arrays of the nodes of `classes` (their amounts of each of the `ns` substances, then their heat) the
/// changes `f` adds to them (`Booker::add`). With work enough for the pool (`work`, `par::each_init`), the amounts of
/// `PER_TASK` substances of a class are one task; else all of a class's amounts, on the calling thread. Then what
/// rounding left below zero in each is set to zero (`zero_rounding`). Returns what was set to zero, each task's in
/// task order: of the amounts, mol, and of the heat, J (`Counters::rounded`).
pub(crate) fn book_by_array(store: &mut Store, ns: usize, classes: &[Class], work: usize, f: impl Fn(&mut Booker) + Sync + Send) -> [f64; 2] {
    let Store { amounts, heat, carry } = store;
    // In debug builds, what each node of each class held of all substances before the booking (`zero_low`).
    let nodes: Vec<Vec<f64>> = amounts
        .iter()
        .enumerate()
        .map(|(c, a)| {
            let len = a.len() / ns;
            let held = |i: usize| (0..ns).map(|k| a[k * len + i].abs()).sum::<f64>();
            if cfg!(debug_assertions) && classes.contains(&Class::ALL[c]) { (0..len).map(held).collect() } else { Vec::new() }
        })
        .collect();
    // Substances per task.
    let per = if work >= crate::par::SERIAL_BELOW { PER_TASK } else { ns };
    let mut tasks = Vec::new();
    let carries = carry.iter_mut().map(Some).chain(std::iter::repeat_with(|| None));
    for ((((c, a), h), carry), node) in amounts.iter_mut().enumerate().zip(heat.iter_mut()).zip(carries).zip(&nodes) {
        let class = Class::ALL[c];
        if !classes.contains(&class) {
            continue;
        }
        let task = |ks, len, from, v, carry| Booker { class, ks, len, v, carry, from, low: Vec::new(), rounded: 0.0, changed: Vec::new(), node };
        if !a.is_empty() {
            let len = a.len() / ns;
            let mut carry = carry.map(|x| x.chunks_mut(per * len));
            for (q, v) in a.chunks_mut(per * len).enumerate() {
                let ks = q * per..((q + 1) * per).min(ns);
                tasks.push(task(Some(ks), len, q * per * len, v, carry.as_mut().and_then(Iterator::next)));
            }
        }
        if !h.is_empty() {
            tasks.push(task(None, h.len(), 0, h, None));
        }
    }
    crate::par::each_init(&mut tasks, work, || (), |_, b| {
        if cfg!(debug_assertions) {
            b.changed = vec![[0.0; 2]; b.v.len()];
        }
        f(b);
        b.zero_low();
    });
    tasks.iter().fold([0.0; 2], |mut r, b| {
        r[b.ks.is_none() as usize] += b.rounded;
        r
    })
}

/// The classes whose nodes a domain's view holds, which its booking writes.
const VIEWED: [Class; 4] = [Class::Ground, Class::Air, Class::Bed, Class::Bodies];

impl World {
    /// The moves gathered by the domains that ran since the last booking are booked (`Moves::book_into`): each array of
    /// the store by one task, which adds each domain's changes in the domains' order (`store::book_by_array`); what
    /// rounding leaves below zero is set to zero and counted (`Counters::rounded`), then the lift into the potential
    /// and the heat the ground nodes could not give (`Counters::clamped_heat`), domain by domain. With work enough
    /// for the pool, `f` runs beside it in the same call, the store out of the world meanwhile: `f` must not read the
    /// store (a landing reads none: it moves no matter of the store, laws.md 0) nor open a view.
    pub(crate) fn book_moves_beside<R: Send>(&mut self, f: impl FnOnce(&mut World) -> R + Send) -> R {
        self.book_gathered(true, f)
    }
    /// The moves gathered by the domains that ran since the last booking are booked on the calling thread: a serial
    /// step, or the turns of the parts, gather a few numbers a domain, and a pool call costs more than their booking.
    pub fn book_moves(&mut self) {
        self.book_gathered(false, |_| ());
    }
    /// `book_moves_beside` (`pool`: on the pool when there are domains enough) or `book_moves`.
    fn book_gathered<R: Send>(&mut self, pool: bool, f: impl FnOnce(&mut World) -> R + Send) -> R {
        let crate::life::Life { unbooked, domains, .. } = &mut self.life;
        // (A domain that ran but gathered nothing has no view open.)
        let open: Vec<usize> = unbooked.take().filter(|&k| domains[k].moves.open.is_some()).collect();
        let views: Vec<(usize, Moves)> = open.into_iter().map(|k| (k, std::mem::take(&mut domains[k].moves))).collect();
        let (ns, work) = (self.chem.ns(), if pool { views.len() } else { 0 });
        let book = |store: &mut Store| book_by_array(store, ns, &VIEWED, work, |b| views.iter().for_each(|(_, m)| m.book_into(b)));
        let (r, rounded) = if views.is_empty() {
            (f(self), [0.0; 2])
        } else if work < SERIAL_BELOW {
            let rounded = book(&mut self.store);
            (f(self), rounded)
        } else {
            let mut store = std::mem::take(&mut self.store);
            let both = crate::par::join(|| f(self), || book(&mut store));
            self.store = store;
            both
        };
        let World { life, counts, store, .. } = self;
        counts.add_rounded(rounded);
        for (k, mut m) in views {
            let (lift, short) = m.close();
            store.add_far_heat(Class::Potential, 0, lift);
            counts.clamped_heat += short;
            debug_assert!(life.domains[k].moves.open.is_none(), "a view opened while the moves gathered were booked");
            life.domains[k].moves = m;
        }
        r
    }
}
