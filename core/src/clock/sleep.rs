//! One sleep and wake for every node (`docs/foundation/plan.md`, Idea 6). A node's own process (the law that runs
//! inside it, as reactions run inside a ground or an air node) need not run while the node is calm: it sleeps until an
//! alarm time, or until a move into it brings more than a share of what it holds (`Config::wake_share`, checked by
//! `store::apply`). What moves between nodes never sleeps. When a node is due, its law brings it up to date for all the
//! time since it last did (`since`) in one span, as its integration over a long step allows, and sets its next alarm.
//!
//! A node sleeps or wakes by its own alarm alone; the nodes due at one time are found by a look over all the alarms (a
//! few numbers a node, once a step) and come in a fixed order, by slot. The alarms are saved, so a resumed run sleeps
//! the same.

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
