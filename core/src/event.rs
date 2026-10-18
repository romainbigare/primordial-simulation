//! Raw physical facts the shell can record. Nothing here interprets them.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    /// Substance `k` has a non-zero amount somewhere for the first time
    FirstSubstance(u16),
    /// Members born (value), the starting kind the parent descends from (`Lineage::root`; only a label for tallies;
    /// `u32::MAX` if none), and whether they had two parents (a partner, `copy::copy`)
    Born { root: u32, mixed: bool },
    /// Members died (value) of `cause`, the highest speed-up (log10) of any light reaction in their genes, and their
    /// starting kind (a label, as for `Born`)
    Died { cause: u8, cat: f32, root: u32 },
    /// Body `by` watched body `of` (value 1 if of the same lineage)
    Watched { by: u64, of: u64 },
    /// Body `from` put stores into body `to` (value: kg)
    Gave { from: u64, to: u64 },
    /// Body `by` took from body `from` (`u64::MAX`: from the ground); value: kg taken. `burn` is
    /// the part of it that `by`'s own reactions can burn; `cat` the highest speed-up (log10) of
    /// any light reaction in the genes of the taker and of the one taken from (0 for ground)
    Took { by: u64, from: u64, burn: f32, cat: [f32; 2] },
    BondMade,
    BondBroken,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub day: u64,
    pub kind: EventKind,
    pub node: u32,
    pub value: f64,
}
