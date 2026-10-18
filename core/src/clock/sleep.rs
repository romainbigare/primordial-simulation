//! One sleep and wake for every node (`docs/foundation/plan.md`, Idea 6). A node's own process (the law that runs
//! inside it, as reactions run inside a ground or an air node) need not run while the node is calm: it sleeps until an
//! alarm time, or until one of its amounts has grown by more than a share of what it held when it fell asleep
//! (`Config::wake_share`), whatever brought it (`watch`, a look its law makes at its own step). What moves between
//! nodes never sleeps. When a node is due, its law brings it up to date for all the time since it last did (`since`) in
//! one span, as its integration over a long step allows, and sets its next alarm.
//!
//! A node sleeps or wakes by its own alarm alone; the nodes due at one time are found by a look over all the alarms (a
//! few numbers a node, once a step) and come in a fixed order, by slot. The alarms and the amounts a node held when it
//! fell asleep are saved, so a resumed run sleeps the same.

use crate::store::{Class, NodeId};
use serde::{Deserialize, Serialize};

/// The classes whose nodes run a process of their own, and so may sleep: all those before `Class::Boundary` (a boundary
/// node only gives and takes what crosses).
const RUN: usize = Class::Boundary as usize;

/// The alarms of the nodes, and when each was last brought up to date.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sleep {
    /// Per node slot: its alarm time, s (`u64::MAX`: no alarm; at or before the time now: due, or running)
    alarm: Vec<u64>,
    /// Per node slot: the time its law last brought it up to date, s
    since: Vec<u64>,
    /// Per node slot: the node that sleeps there, and its class (none while it runs)
    asleep: Vec<Option<(NodeId, Class)>>,
    /// Per class that runs (`RUN`): how many of its nodes sleep
    count: [usize; RUN],
    /// Per class that runs: what each of its nodes held when it last fell asleep, `[k * len + index]` (`keep`)
    held: [Vec<f64>; RUN],
}

impl Sleep {
    /// Node `id` of `class`, brought up to date at `now`, sleeps until `until` (s; `u64::MAX`: until a move wakes it;
    /// `now`: it is due at once). Its earlier alarm, if any, is dropped. Panics for a boundary node.
    pub fn sleep(&mut self, class: Class, id: NodeId, now: u64, until: u64) {
        assert!((class as usize) < RUN, "a node of {class:?} runs no process, so it never sleeps");
        let s = id.slot as usize;
        if s >= self.alarm.len() {
            self.alarm.resize(s + 1, u64::MAX);
            self.since.resize(s + 1, 0);
            self.asleep.resize(s + 1, None);
        }
        if self.asleep[s].is_none() {
            self.count[class as usize] += 1;
        }
        (self.alarm[s], self.since[s], self.asleep[s]) = (until, now, Some((id, class)));
    }

    /// Node `id` of `class` is due at `now` if it sleeps until later (nothing if it is not asleep, as a boundary node
    /// never is).
    pub fn wake(&mut self, class: Class, id: NodeId, now: u64) {
        let s = id.slot as usize;
        if self.alarm.get(s).is_some_and(|&a| a > now) && self.asleep[s] == Some((id, class)) {
            self.alarm[s] = now;
        }
    }

    /// The nodes of `class` due at `now` (their alarm rang or a move woke them), in the order of their slots (none for
    /// a boundary node). Each runs; its law puts it back to sleep (`sleep`) with its next alarm.
    pub fn due(&mut self, class: Class, now: u64) -> Vec<NodeId> {
        let due: Vec<NodeId> = (self.asleep.iter().zip(&self.alarm)).filter_map(|(n, &t)| n.filter(|&(_, c)| c == class && t <= now).map(|e| e.0)).collect();
        due.iter().for_each(|id| self.asleep[id.slot as usize] = None);
        if let Some(n) = self.count.get_mut(class as usize) {
            *n -= due.len();
        }
        due
    }

    /// Node `i` of `class` (of `len` nodes, holding `amounts`, `[k * len + i]`) falls asleep holding what it holds:
    /// `watch` looks for a gain against it.
    pub fn keep(&mut self, class: Class, i: usize, len: usize, amounts: &[f64]) {
        let held = &mut self.held[class as usize];
        held.resize(amounts.len(), 0.0);
        for (h, &a) in held[i..].iter_mut().step_by(len).zip(amounts[i..].iter().step_by(len)) {
            *h = a;
        }
    }

    /// Wakes each sleeping node of `class` (the node at index `i` of its class is `id(i)`; `len` nodes, holding
    /// `amounts`, `[k * len + i]`) one of whose amounts has grown by more than `share` of what it held when it fell
    /// asleep (`keep`), so a node that many small gains have changed reacts as one large gain would make it.
    pub fn watch(&mut self, class: Class, id: impl Fn(usize) -> NodeId, len: usize, amounts: &[f64], share: f64, now: u64) {
        if self.none(class) {
            return;
        }
        let mut grew = vec![false; len];
        for (now_k, then_k) in amounts.chunks(len).zip(self.held[class as usize].chunks(len)) {
            for (g, (&a, &h)) in grew.iter_mut().zip(now_k.iter().zip(then_k)) {
                *g |= a - h > share * h.max(0.0);
            }
        }
        for i in (0..len).filter(|&i| grew[i]) {
            self.wake(class, id(i), now);
        }
    }

    /// Whether no node of `class` sleeps: its law has not run yet (after a run, it puts every node it ran back to sleep).
    pub fn none(&self, class: Class) -> bool {
        self.count.get(class as usize).is_none_or(|&n| n == 0)
    }

    /// The time node `id` (put to sleep at least once) was last brought up to date, s: its law catches it up for the
    /// time since.
    pub fn since(&self, id: NodeId) -> u64 {
        self.since[id.slot as usize]
    }
}
