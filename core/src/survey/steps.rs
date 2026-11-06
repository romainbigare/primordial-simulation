//! The step follows the size (laws.md 0): a body's step lets it cross at most one cell. A move is one link a decision,
//! and a square of the map (`Square::square_depth`: 2 km on the island) steps a month (the pace the numbers of bodies
//! are anchored at), so a cell 2^k times finer than a square steps a month over 2^k and a square or a coarser cell a
//! month. The month's batch updates every body at its start (`batch`); then the bodies of the finer cells step again
//! inside the month (`substeps`), each over the time since its own last update (`Body::last`). A body's own physics
//! runs in sub-steps of one physiological step in every cell (`World::phys_grid`), its decisions at its cell's step.

use super::{batch_of, break_and_sum};
use crate::{choose, World};

/// A body's step is at least this long, s: the world's time is whole seconds (`Grid`).
const STEP_LEAST_S: u64 = 1;
/// The halvings of the month that give a body's physiological step, 7.5 days (`World::phys_grid`): the step its own
/// physics integrates in, the same in every cell. Its own times are a meal (its stores' room holds what it burns until
/// its next meal), the filling of its brood to a child and its turn between a cold and a warm state as its pace runs
/// away with its heat (laws.md 6.3), days to weeks; a step of a month crosses those thresholds once for a month at the
/// state it started in, and one of a week follows them as a cell of 500 m does, at four times the physics.
const PHYS_HALVINGS: u32 = 2;
/// The keys of the draws of a step shorter than a day start here (`World::key_at`): above every day's key (2^40 days
/// are 3e9 years), so a step of a day or longer keeps its day's key and no two shorter steps share one.
const SHORT_KEYS: u64 = 1 << 40;

/// The times a cell steps at when the month is cut into 2^`halvings` steps: step j of a month ends floor(j M / 2^halvings)
/// s after its start (M the month), so each is a whole number of seconds, and all are alike where 2^halvings divides
/// the month (2,592,000 s = 2^8 × 10,125: down to 2.8 hours); a grid of more halvings holds every time of one of fewer.
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
    /// The most halvings of the month that keep a step at least `STEP_LEAST_S`.
    fn most_halvings(&self) -> u32 {
        (self.cfg.calendar().month / STEP_LEAST_S).ilog2()
    }
    /// The halvings of the month that give the bodies' step of cell `i`: its depth below a square of the map (none for a
    /// square or a coarser cell), at most `most_halvings`.
    pub fn halvings(&self, i: usize) -> u32 {
        (self.map.leaves()[i].depth.saturating_sub(self.map.square_depth()) as u32).min(self.most_halvings())
    }
    /// The times the bodies of cell `i` update at: the month over 2^`halvings`.
    pub fn step_grid(&self, i: usize) -> Grid {
        Grid { month: self.cfg.calendar().month, halvings: self.halvings(i) }
    }
    /// The bodies' step of cell `i`, d: the time from an update there to the next.
    pub fn step_days(&self, i: usize) -> f64 {
        self.step_grid(i).days(self.cfg.calendar().day)
    }
    /// The physiological steps of the bodies of cell `i`: the month over 2^`PHYS_HALVINGS`, or the cell's step where
    /// that is shorter. An update runs its body's physics in sub-steps of it, each with a meal (`survey::updates`).
    pub fn phys_grid(&self, i: usize) -> Grid {
        Grid { month: self.cfg.calendar().month, halvings: self.halvings(i).max(PHYS_HALVINGS.min(self.most_halvings())) }
    }
    /// The physiological step of cell `i`, d: the time from a meal of its bodies to the next.
    pub fn phys_days(&self, i: usize) -> f64 {
        self.phys_grid(i).days(self.cfg.calendar().day)
    }
    /// The time from now to the next sub-step of cell `i`, d: what a body that lands there during a batch eats for
    /// (laws.md 0, 5.3).
    pub fn days_to_phys(&self, i: usize) -> f64 {
        (self.phys_grid(i).after(self.time) - self.time) as f64 / self.cfg.calendar().day as f64
    }
    /// The key of the random draws of the bodies of cell `i` at time `t` (s): its day where the cell steps a day or
    /// longer (two of its steps never fall on one day); else the time itself, from `SHORT_KEYS` on.
    pub fn key_at(&self, i: usize, t: u64) -> u64 {
        if self.step_days(i) >= 1.0 { self.cfg.calendar().days(t) } else { SHORT_KEYS + t }
    }
    /// The key of the draws of a span of cell `i` that ends at time `t` (s): that of its last second.
    pub fn key_ending(&self, i: usize, t: u64) -> u64 {
        self.key_at(i, t.saturating_sub(1))
    }
}

/// After the month's batch (`batch`), from the month's start: the sub-steps of the finest cells' step, each at the time it
/// ends. At each, the cells whose step ends then are surveyed again (`break_and_sum`, with every cell's numbers and
/// features, `choose::cells`, as the moves and meals since left them), and their bodies update (`batch_of`) on the same
/// chunks, a parallel pass and a serial pass; the other cells do nothing. A sub-step with no living body in a cell that
/// steps is skipped. The world's time is the month's start after.
pub fn substeps(w: &mut World) {
    let (t0, month) = (w.time, w.cfg.calendar().month);
    let halvings: Vec<u32> = (0..w.mesh.cells().n()).map(|i| w.halvings(i)).collect();
    let most = halvings.iter().copied().max().unwrap_or(0);
    let (grid, parts) = (Grid { month, halvings: most }, 1u64 << most);
    // The most halvings of a cell that holds a living body (none: no body lives).
    let deepest = |w: &World| w.life.living().map(|(h, _)| halvings[h.cell as usize]).max();
    let mut alive = deepest(w);
    for k in 1..parts {
        // (A cell of `h` halvings steps at every (`parts` >> h)-th sub-step: those of `most` less the trailing zeros of
        // `k` halvings or more step at `k`.)
        if alive.is_none_or(|d| most - k.trailing_zeros() > d) {
            continue;
        }
        let steps: Vec<bool> = halvings.iter().map(|&h| k.is_multiple_of(parts >> h)).collect();
        w.time = grid.at(grid.by(t0) + k);
        break_and_sum(w, Some(&steps));
        choose::cells(w);
        batch_of(w, Some(&steps));
        alive = deepest(w);
    }
    w.time = t0;
}
