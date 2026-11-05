//! The clock: time is whole seconds since the start (`World::time`), so it adds up exactly; the calendar gives the day,
//! month, year and season. `sleep`: the sleep and wake of nodes.

pub mod sleep;

pub use sleep::Sleep;

/// The lengths of a world's day, month and year, s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calendar {
    pub day: u64,
    pub month: u64,
    pub year: u64,
}

impl Calendar {
    /// Whole days since the start at time `t` (s).
    pub fn days(&self, t: u64) -> u64 {
        t / self.day
    }
    /// The day a span that ends at time `t` (s) ends on: that of its last second.
    pub fn day_ending(&self, t: u64) -> u64 {
        t.saturating_sub(1) / self.day
    }
    /// Whole months since the start at time `t` (s).
    pub fn months(&self, t: u64) -> u64 {
        t / self.month
    }
    /// Whole years since the start at time `t` (s).
    pub fn years(&self, t: u64) -> u64 {
        t / self.year
    }
    /// Share of the year gone at time `t` (s), 0..1.
    pub fn season(&self, t: u64) -> f64 {
        (t % self.year) as f64 / self.year as f64
    }
}
