//! One sleep and wake for every node (laws.md 0): a calm node's own process sleeps until an alarm, or until one of its
//! amounts has grown by more than `Config::wake_share` of what it held when it fell asleep (`watch`); when due, its law
//! brings it up to date for all the time since (`since`) in one span. A long integration is exact for a calm node, and
//! what moves between nodes never sleeps. Saved, so a resumed run sleeps the same.

use crate::store::Class;
use serde::{Deserialize, Serialize};

/// The classes whose nodes run a process of their own, and so may sleep: those before `Class::Boundary`.
const RUN: usize = Class::Boundary as usize;

/// The alarms of the nodes of one class, by index.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Alarms {
    /// Its alarm time, s (`u64::MAX`: no alarm; at or before the time now: due, or running)
    alarm: Vec<u64>,
    /// The time its law last brought it up to date, s
    since: Vec<u64>,
    /// Whether it sleeps (not while it runs)
    asleep: Vec<bool>,
    /// How many of the nodes sleep
    count: usize,
    /// What each node held when it last fell asleep, `[k * len + index]` (`keep`)
    held: Vec<f64>,
}

/// The alarms of the nodes, and when each was last brought up to date, per class that runs (`RUN`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sleep([Alarms; RUN]);

impl Sleep {
    /// The alarms of `class`. Panics for a class that runs no process (a boundary node, a part).
    fn of(&mut self, class: Class) -> &mut Alarms {
        assert!((class as usize) < RUN, "a node of {class:?} runs no process, so it never sleeps");
        &mut self.0[class as usize]
    }

    /// Node `i` of `class`, brought up to date at `now`, sleeps until `until` (s; `u64::MAX`: until a gain wakes it).
    pub fn sleep(&mut self, class: Class, i: usize, now: u64, until: u64) {
        let a = self.of(class);
        if i >= a.alarm.len() {
            a.alarm.resize(i + 1, u64::MAX);
            a.since.resize(i + 1, 0);
            a.asleep.resize(i + 1, false);
        }
        if !a.asleep[i] {
            a.count += 1;
        }
        (a.alarm[i], a.since[i], a.asleep[i]) = (until, now, true);
    }

    /// Node `i` of `class` is due at `now` if it sleeps until later (nothing if it is not asleep).
    pub fn wake(&mut self, class: Class, i: usize, now: u64) {
        let a = self.of(class);
        if a.asleep.get(i) == Some(&true) && a.alarm[i] > now {
            a.alarm[i] = now;
        }
    }

    /// The nodes of `class` due at `now`, in the order of their indices; their law puts each back to sleep.
    pub fn due(&mut self, class: Class, now: u64) -> Vec<usize> {
        let a = self.of(class);
        let due: Vec<usize> = (0..a.alarm.len()).filter(|&i| a.asleep[i] && a.alarm[i] <= now).collect();
        due.iter().for_each(|&i| a.asleep[i] = false);
        a.count -= due.len();
        due
    }

    /// Node `i` of `class` falls asleep holding what it holds (`amounts`, `[k * len + i]`), which `watch` reads against.
    pub fn keep(&mut self, class: Class, i: usize, len: usize, amounts: &[f64]) {
        let held = &mut self.of(class).held;
        held.resize(amounts.len(), 0.0);
        held[i..].iter_mut().step_by(len).zip(amounts[i..].iter().step_by(len)).for_each(|(h, &a)| *h = a);
    }

    /// Wakes each sleeping node of `class` one of whose amounts has grown by more than `share` of what it held when it
    /// fell asleep, so many small gains wake it as one large gain would.
    pub fn watch(&mut self, class: Class, len: usize, amounts: &[f64], share: f64, now: u64) {
        if self.none(class) {
            return;
        }
        let mut grew = vec![false; len];
        for (now_k, then_k) in amounts.chunks(len).zip(self.of(class).held.chunks(len)) {
            grew.iter_mut().zip(now_k.iter().zip(then_k)).for_each(|(g, (&a, &h))| *g |= a - h > share * h.max(0.0));
        }
        (0..len).filter(|&i| grew[i]).for_each(|i| self.wake(class, i, now));
    }

    /// Whether no node of `class` sleeps: its law has not run yet (after a run, it puts every node it ran back to sleep).
    pub fn none(&self, class: Class) -> bool {
        self.0.get(class as usize).is_none_or(|a| a.count == 0)
    }

    /// The nodes of each cell (`Class::CELLS`) of a class whose law has run follow their cells (`World::remap`): a new
    /// cell's is due at `now`, as every node is before its law first runs.
    pub(crate) fn remap(&mut self, r: &crate::remap::Renumber, now: u64) {
        for c in Class::CELLS {
            let Alarms { alarm, since, asleep, count, held } = self.of(c);
            if alarm.is_empty() {
                continue;
            }
            (*alarm, *since) = (r.apply(std::mem::take(alarm), || now), r.apply(std::mem::take(since), || now));
            (*asleep, *held) = (r.apply(std::mem::take(asleep), || true), r.apply(std::mem::take(held), || 0.0));
            *count = asleep.iter().filter(|&&a| a).count();
        }
    }

    /// The time node `i` of `class` was last brought up to date, s.
    pub fn since(&self, class: Class, i: usize) -> u64 {
        self.0[class as usize].since[i]
    }
}
