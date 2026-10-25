//! Booking by array: many sources of changes (the views of the domains that ran, the rows of a law) are added to the
//! store with one task for each block of its arrays (the amounts of a few substances of a class, or its heat), so each
//! number has one writer (CLAUDE.md 3.6) and gets its changes in the order of the sources, whatever thread runs the
//! task. It adds as `apply` adds and counts the rounding it sets to zero.

use super::apply::{add_carried, zero_rounding};
use super::{Class, Store};
use std::ops::Range;

/// Substances whose amounts make one booking task on the pool: a few, so a task is worth its start and each thread
/// still gets several.
const PER_TASK: usize = 4;

/// A booking task: the arrays of one class of nodes it owns, the amounts of substances `ks` (`[k * len + index]`, mol)
/// or with `ks` none the heat (`[index]`, J), with what rounding took of each amount (a carried class, `Store::carry`);
/// the numbers that fell below zero on the way, and what rounding left below zero, set to zero at the task's end.
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
}

impl Booker<'_> {
    /// Number `x` of the class's array (`[k * len + index]` of an amount, `[index]` of the heat) changes by `d`, as
    /// `apply` adds it (`add_carried`). Only what it holds at the end must be at zero or above.
    #[inline]
    pub fn add(&mut self, x: usize, d: f64) {
        let x = x - self.from;
        let was = self.v[x];
        add_carried(&mut self.v[x], self.carry.as_deref_mut().map(|c| &mut c[x]), d);
        if self.v[x] < 0.0 && was >= 0.0 {
            self.low.push(x);
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
    // Substances per task.
    let per = if work >= crate::par::SERIAL_BELOW { PER_TASK } else { ns };
    let mut tasks = Vec::new();
    let carries = carry.iter_mut().map(Some).chain(std::iter::repeat_with(|| None));
    for (((c, a), h), carry) in amounts.iter_mut().enumerate().zip(heat.iter_mut()).zip(carries) {
        let class = Class::ALL[c];
        if !classes.contains(&class) {
            continue;
        }
        let task = |ks, len, from, v, carry| Booker { class, ks, len, v, carry, from, low: Vec::new(), rounded: 0.0 };
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
        f(b);
        for &x in &b.low {
            zero_rounding(&mut b.v[x], &mut b.rounded);
        }
    });
    tasks.iter().fold([0.0; 2], |mut r, b| {
        r[b.ks.is_none() as usize] += b.rounded;
        r
    })
}
