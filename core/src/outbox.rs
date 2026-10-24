//! The outbox (plan 3.6): what a body's update in a batch does outside it is queued in its domain's outbox, in the
//! order made, and applied in the serial phase, domain by domain: its moves to another cell, its children who start in
//! another cell, its watches and its acts on parts. Each kind is applied by its law on the bodies and parts as they are
//! then; one whose body or target is gone does nothing, as its law says.

use crate::{choose, copy, store::NodeId, World};

/// One effect of an update outside it.
#[derive(Clone, Debug)]
pub enum Entry {
    /// Body `id` moves from cell `from` to `to` (`choose::go`), having spent `pushed` J pushing through the bodies
    /// there, which clears `lane` m² of its lane; with `kin`, it is members of a node who join their kind there
    Go { id: u64, from: u32, to: u32, pushed: f64, lane: f64, kin: bool },
    /// Children who start in another cell (`copy::emit`)
    Litter(Box<copy::Litter>),
    /// Body `by`, of `n` members who watched and of `lineage` then, watched body `of` (`choose::watched`)
    Watch { by: u64, n: f64, lineage: u32, of: u64 },
    /// Body `id` does verb `on.0` to part `on.1`, or forms a part of `shape` (none), as a part of node `node` if any
    /// (`choose::part_act`)
    Act { id: u64, on: Option<(usize, NodeId)>, shape: [f32; 2], node: Option<u64> },
}

impl Entry {
    /// The lineage and born rows it names become `to(id, rows)` (`Life::remap`).
    pub(crate) fn remap(&mut self, to: &impl Fn(u32, bool) -> u32) {
        match self {
            Entry::Litter(x) => (x.c.lineage, x.c.genome) = (to(x.c.lineage, false), to(x.c.genome, true)),
            Entry::Watch { lineage, .. } => *lineage = to(*lineage, false),
            _ => {}
        }
    }
}

/// The outboxes are applied, domain by domain, each in the order made.
pub fn deliver(w: &mut World) {
    for e in w.life.take_queued() {
        match e {
            Entry::Go { id, from, to, pushed, lane, kin } => choose::land(w, id, (from as usize, to as usize), (pushed, lane), kin),
            Entry::Litter(x) => copy::land(w, *x),
            Entry::Watch { by, n, lineage, of } => choose::watched(w, by, (n, lineage), of),
            Entry::Act { id, on, shape, node } => choose::part_act(w, id, (on, shape), node),
        }
    }
}
