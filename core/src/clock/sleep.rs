//! One sleep and wake for every node (`docs/foundation/plan.md`, Idea 6). A node's own process (the law that runs
//! inside it, as reactions run inside a ground or an air node) need not run while the node is calm: it sleeps until an
//! alarm time, or until a move into it brings more than a share of what it holds (`Config::wake_share`, checked by
//! `store::apply`). What moves between nodes never sleeps. When a node is due, its law brings it up to date for all the
//! time since it last did (`since`) in one span, as its integration over a long step allows, and sets its next alarm.
//!
//! The alarms of each class are a queue sorted by time, then node: the nodes due at one time come in that fixed order,
//! taking them costs only the nodes due, and the queue is saved in that order, so a resumed run sleeps the same.

use crate::store::{Class, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
    /// Per class that runs (`RUN`): the sleeping nodes, by alarm time, then node
    queue: [BTreeSet<(u64, NodeId)>; RUN],
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
        }
        let queue = &mut self.queue[class as usize];
        queue.remove(&(self.alarm[s], id));
        (self.alarm[s], self.since[s]) = (until, now);
        queue.insert((until, id));
    }

    /// Node `id` of `class` is due at `now` if it sleeps until later (nothing if it is not asleep, as a boundary node
    /// never is).
    pub fn wake(&mut self, class: Class, id: NodeId, now: u64) {
        let s = id.slot as usize;
        let Some(queue) = self.queue.get_mut(class as usize) else { return };
        if self.alarm.get(s).is_some_and(|&a| a > now) && queue.remove(&(self.alarm[s], id)) {
            self.alarm[s] = now;
            queue.insert((now, id));
        }
    }

    /// The nodes of `class` due at `now` (their alarm rang or a move woke them), taken out of the queue in the order of
    /// their alarms, then of their slots (none for a boundary node). Each runs; its law puts it back to sleep (`sleep`)
    /// with its next alarm.
    pub fn due(&mut self, class: Class, now: u64) -> Vec<NodeId> {
        let mut out = Vec::new();
        if let Some(queue) = self.queue.get_mut(class as usize) {
            while queue.first().is_some_and(|&(t, _)| t <= now) {
                out.extend(queue.pop_first().map(|e| e.1));
            }
        }
        out
    }

    /// The time node `id` (put to sleep at least once) was last brought up to date, s: its law catches it up for the
    /// time since.
    pub fn since(&self, id: NodeId) -> u64 {
        self.since[id.slot as usize]
    }
}
