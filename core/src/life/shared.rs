//! What life shares: the causes it dies of (the cover's books count what it loses by them, `Counters::cover_lost`), the
//! liquid share of living tissue (laws.md 3).

/// Causes of death, the places of `Counters::cover_lost`: a store below its safe minimum; a key liquid froze; upkeep not
/// paid; taken by a taker (`contract::take`).
pub const LACK: u8 = 0;
pub const PHASE: u8 = 1;
pub const SPENT: u8 = 2;
pub const TAKEN: u8 = 3;
/// Broken loose by work pressed on its cell (`contract::press`).
pub const BROKEN: u8 = 4;
/// Worn out by living (its wear, `cover::live`).
pub const AGE: u8 = 5;
/// The liquid share of living tissue: a kg of matter makes 1 / (1 - this) kg of living tissue (`Cover::living`).
pub const WET_TISSUE: f64 = 0.75;
