//! The bodies' steps (laws.md 0): a move is one link a decision, and a cell of the map steps a month (the pace the
//! numbers of bodies are anchored at), so the month's batch updates every body at its start (`batch`). A body's own
//! physics runs in sub-steps of one physiological step (`World::phys_grid`), its decisions at the month's step.

use crate::World;

/// The halvings of the month that give a body's physiological step, 7.5 days (`World::phys_grid`): the step its own
/// physics integrates in. Its own times are a meal (its stores' room holds what it burns until its next meal), the
/// filling of its brood to a child and its turn between a cold and a warm state as its pace runs away with its heat
/// (laws.md 6.3), days to weeks; a step of a month crosses those thresholds once for a month at the state it started
/// in, and one of a week follows them.
const PHYS_HALVINGS: u32 = 2;

/// The times a step ends at when the month is cut into 2^`halvings` steps: step j of a month ends floor(j M / 2^halvings)
/// s after its start (M the month), so each is a whole number of seconds, and all are alike where 2^halvings divides
/// the month (2,592,000 s = 2^8 × 10,125).
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    month: u64,
    halvings: u32,
}

impl Grid {
    /// The time step `j` ends at, counted from the start of time, s.
    pub(crate) fn at(&self, j: u64) -> u64 {
        let mask = (1u64 << self.halvings) - 1;
        (j >> self.halvings) * self.month + (((j & mask) * self.month) >> self.halvings)
    }
    /// The last step that ends by time `t` (s), counted from the start of time.
    pub(crate) fn by(&self, t: u64) -> u64 {
        let r = t % self.month;
        ((t / self.month) << self.halvings) + (((r + 1) << self.halvings).div_ceil(self.month) - 1)
    }
    /// The first time after `t` a step ends, s.
    pub fn after(&self, t: u64) -> u64 {
        self.at(self.by(t) + 1)
    }
    /// The mean length of a step, d (`day` s a day).
    fn days(&self, day: u64) -> f64 {
        self.month as f64 / (1u64 << self.halvings) as f64 / day as f64
    }
}

impl World {
    /// The times the bodies update at: each month's start.
    pub fn step_grid(&self) -> Grid {
        Grid { month: self.cfg.calendar().month, halvings: 0 }
    }
    /// The bodies' step, d: the time from an update to the next.
    pub fn step_days(&self) -> f64 {
        self.step_grid().days(self.cfg.calendar().day)
    }
    /// The physiological steps of the bodies: the month over 2^`PHYS_HALVINGS`. An update runs its body's physics in
    /// sub-steps of it, each with a meal (`survey::updates`).
    pub fn phys_grid(&self) -> Grid {
        Grid { month: self.cfg.calendar().month, halvings: PHYS_HALVINGS }
    }
    /// The physiological step, d: the time from a meal of a body to the next.
    pub fn phys_days(&self) -> f64 {
        self.phys_grid().days(self.cfg.calendar().day)
    }
    /// The time from now to the next physiological sub-step, d: what a body that lands in a cell during a batch eats
    /// for (laws.md 0, 5.3).
    pub fn days_to_phys(&self) -> f64 {
        (self.phys_grid().after(self.time) - self.time) as f64 / self.cfg.calendar().day as f64
    }
    /// The key of the random draws of the bodies at time `t` (s): its day (two of their steps never fall on one day).
    pub fn key_at(&self, t: u64) -> u64 {
        self.cfg.calendar().days(t)
    }
    /// The key of the draws of a span that ends at time `t` (s): that of its last second.
    pub fn key_ending(&self, t: u64) -> u64 {
        self.key_at(t.saturating_sub(1))
    }
}
