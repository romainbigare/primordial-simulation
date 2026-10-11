//! Raw physical facts the shell can record. Nothing here interprets them.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    /// Substance `k` has a non-zero amount somewhere for the first time
    FirstSubstance(u16),
    Born,
    Died { cause: u8 },
    /// A body of group `by` watched a body of group `of` (value 1 if of the same lineage)
    Watched { by: u32, of: u32 },
    /// Body `from` put stores into body `to` (value: kg)
    Gave { from: u64, to: u64 },
    BondMade,
    BondBroken,
    Imposed,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub day: u64,
    pub kind: EventKind,
    pub node: u32,
    pub value: f64,
}
