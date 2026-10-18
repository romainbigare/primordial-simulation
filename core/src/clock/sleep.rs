//! One sleep and wake for every node (`docs/foundation/plan.md`, Idea 6). A node's own process (the law that runs
//! inside it, as reactions run inside a ground or an air node) need not run while the node is calm: it sleeps until an
//! alarm time, or until one of its amounts has grown by more than a share of what it held when it fell asleep
//! (`Config::wake_share`), whatever brought it (`watch`, a look its law makes at its own step). What moves between
//! nodes never sleeps. When a node is due, its law brings it up to date for all the time since it last did (`since`) in
//! one span, as its integration over a long step allows, and sets its next alarm.
//!
//! A node is named by its class and its index in that class's arrays. It sleeps or wakes by its own alarm alone; the
//! nodes due at one time are found by a look over the alarms of their class (a few numbers a node, once a step) and
//! come in a fixed order, by index. The alarms and the amounts a node held when it fell asleep are saved, so a resumed
//! run sleeps the same.

use crate::store::Class;
use serde::{Deserialize, Serialize};

/// The classes whose nodes run a process of their own, and so may sleep: all those before `Class::Boundary` (a boundary
/// node only gives and takes what crosses).
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
    /// The alarms of `class`. Panics for a class that runs no process (a boundary node, the potential, a part).
    fn of(&mut self, class: Class) -> &mut Alarms {
        assert!((class as usize) < RUN, "a node of {class:?} runs no process, so it never sleeps");
        &mut self.0[class as usize]
    }

    /// Node `i` of `class`, brought up to date at `now`, sleeps until `until` (s; `u64::MAX`: until a gain wakes it;
    /// `now`: it is due at once). Its earlier alarm, if any, is dropped.
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

    /// The nodes of `class` due at `now` (their alarm rang or a gain woke them), in the order of their indices. Each
    /// runs; its law puts it back to sleep (`sleep`) with its next alarm.
    pub fn due(&mut self, class: Class, now: u64) -> Vec<usize> {
        let a = self.of(class);
        let due: Vec<usize> = (0..a.alarm.len()).filter(|&i| a.asleep[i] && a.alarm[i] <= now).collect();
        due.iter().for_each(|&i| a.asleep[i] = false);
        a.count -= due.len();
        due
    }

    /// Node `i` of `class` (of `len` nodes, holding `amounts`, `[k * len + i]`) falls asleep holding what it holds:
    /// `watch` looks for a gain against it.
    pub fn keep(&mut self, class: Class, i: usize, len: usize, amounts: &[f64]) {
        let held = &mut self.of(class).held;
        held.resize(amounts.len(), 0.0);
        for (h, &a) in held[i..].iter_mut().step_by(len).zip(amounts[i..].iter().step_by(len)) {
            *h = a;
        }
    }

    /// Wakes each sleeping node of `class` (`len` nodes, holding `amounts`, `[k * len + i]`) one of whose amounts has
    /// grown by more than `share` of what it held when it fell asleep (`keep`), so a node that many small gains have
    /// changed reacts as one large gain would make it.
    pub fn watch(&mut self, class: Class, len: usize, amounts: &[f64], share: f64, now: u64) {
        if self.none(class) {
            return;
        }
        let mut grew = vec![false; len];
        for (now_k, then_k) in amounts.chunks(len).zip(self.of(class).held.chunks(len)) {
            for (g, (&a, &h)) in grew.iter_mut().zip(now_k.iter().zip(then_k)) {
                *g |= a - h > share * h.max(0.0);
            }
        }
        for i in (0..len).filter(|&i| grew[i]) {
            self.wake(class, i, now);
        }
    }

    /// Whether no node of `class` sleeps: its law has not run yet (after a run, it puts every node it ran back to sleep).
    pub fn none(&self, class: Class) -> bool {
        self.0.get(class as usize).is_none_or(|a| a.count == 0)
    }

    /// The time node `i` of `class` (put to sleep at least once) was last brought up to date, s: its law catches it up
    /// for the time since.
    pub fn since(&self, class: Class, i: usize) -> u64 {
        self.0[class as usize].since[i]
    }
}
