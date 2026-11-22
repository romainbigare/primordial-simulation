//! Raw physical facts the shell can record. Nothing here interprets them.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    /// Substance `k` has a non-zero amount somewhere for the first time
    FirstSubstance(u16),
    /// An edit made at a step boundary (`World::edit`): its node the cell it acted on, its value 1 where it was made, 0
    /// where the world could not make it
    Edit(crate::edit::Edit),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub day: u64,
    pub kind: EventKind,
    pub node: u32,
    pub value: f64,
}
