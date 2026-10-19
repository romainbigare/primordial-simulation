//! The clock: time is whole seconds since the start (`World::time`), so it adds up exactly; the calendar gives the day,
//! month, year and season. A process runs every `period` from the start of each longer period that holds it (`Every`);
//! a list of processes is their fixed order at one time. `sleep`: the sleep and wake of nodes.

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

/// A process that runs every `period` seconds from the start of each `within` seconds; its last run there stands for
/// the time to the end of `within`. Both are above 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Every {
    pub period: u64,
    pub within: u64,
}

impl Every {
    /// If the process runs at time `t`: the seconds that run stands for (to its next run, or to the end of `within`).
    pub fn span(&self, t: u64) -> Option<u64> {
        let into = t % self.within;
        into.is_multiple_of(self.period).then(|| self.period.min(self.within - into))
    }
    /// The first time after `t` at which the process runs, s.
    pub fn next(&self, t: u64) -> u64 {
        let start = t - t % self.within;
        (start + ((t - start) / self.period + 1) * self.period).min(start + self.within)
    }
}

/// The processes of `list` that run at time `t`, in the order of the list, each with the seconds its run stands for.
pub fn due<P: Copy>(list: &[(Every, P)], t: u64) -> impl Iterator<Item = (P, u64)> + '_ {
    list.iter().filter_map(move |&(every, p)| every.span(t).map(|span| (p, span)))
}

/// The first time after `t` at which a process of `list` runs, s (`u64::MAX` for an empty list).
pub fn next<P>(list: &[(Every, P)], t: u64) -> u64 {
    list.iter().map(|(every, _)| every.next(t)).min().unwrap_or(u64::MAX)
}
