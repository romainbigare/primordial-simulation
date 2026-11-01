//! Booking by array: many sources of changes (the views of the chunks that ran, `World::take_chunks`; the rows of a law) are
//! added to the store with one task for each block of its arrays (the amounts of a few substances of a class, or its
//! heat), so each number has one writer (CLAUDE.md 3.6) and gets its changes in the order of the sources, whatever
//! thread runs the task. It adds as `apply` adds, counts the rounding it sets to zero and checks it as `apply` checks.

use super::apply::{overdraw_check, zero_rounding};
use super::{Class, Store};
use std::ops::Range;

/// Substances whose amounts make one booking task on the pool: a few, so a task is worth its start and each thread
/// still gets several.
const PER_TASK: usize = 4;

/// A booking task: the arrays of one class of nodes it owns, the amounts of substances `ks` (`[k * len + index]`, mol)
/// or with `ks` none the heat (`[index]`, J); the numbers that fell below zero on the way, and what rounding left below zero, set to zero at the task's end. In
/// debug builds, per number the sum of its changes and of their sizes, and what each node of the class held of all
/// substances before the booking (`apply::overdraw_check`).
pub struct Booker<'s> {
    pub class: Class,
    pub ks: Option<Range<usize>>,
    /// The count of nodes of the class
    pub len: usize,
    /// Its numbers, from `[ks.start * len]`
    v: &'s mut [f64],
    from: usize,
    low: Vec<usize>,
    rounded: f64,
    changed: Vec<[f64; 2]>,
    node: &'s [f64],
}

impl Booker<'_> {
    /// Number `x` of the class's array (`[k * len + index]` of an amount, `[index]` of the heat) changes by `d`, as
    /// `apply` adds it. Only what it holds at the end must be at zero or above.
    #[inline(always)]
    pub fn add(&mut self, x: usize, d: f64) {
        let x = x - self.from;
        let was = self.v[x];
        self.v[x] += d;
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
    let Store { amounts, heat, .. } = store;
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
    for (((c, a), h), node) in amounts.iter_mut().enumerate().zip(heat.iter_mut()).zip(&nodes) {
        let class = Class::ALL[c];
        if !classes.contains(&class) {
            continue;
        }
        let task = |ks, len, from, v| Booker { class, ks, len, v, from, low: Vec::new(), rounded: 0.0, changed: Vec::new(), node };
        if !a.is_empty() {
            let len = a.len() / ns;
            for (q, v) in a.chunks_mut(per * len).enumerate() {
                let ks = q * per..((q + 1) * per).min(ns);
                tasks.push(task(Some(ks), len, q * per * len, v));
            }
        }
        if !h.is_empty() {
            tasks.push(task(None, h.len(), 0, h));
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
