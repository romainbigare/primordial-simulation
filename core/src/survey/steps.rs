//! The step follows the size (laws.md 0): a body's step lets it cross at most one cell. A move is one link a decision,
//! and a square of the map (`Square::square_depth`: 2 km on the island) steps a month, today's pace and speed limit, so
//! a cell 2^k times finer than a square steps a month over 2^k and a square or a coarser cell a month. The month's batch
//! updates every body at its start (`batch`); then the bodies of the finer cells step again inside the month
//! (`substeps`), each over the time since its own last update (`Body::last`).

use super::{batch_of, break_and_sum};
use crate::units::DAY_S;
use crate::{choose, World};

/// A body's step is at least this long, s (a day: the draws of the laws of bodies are keyed by the day, `World::day`),
/// and a whole number of seconds; a cell too fine for that steps as the finest cell that keeps both.
const STEP_LEAST_S: u64 = DAY_S;

impl World {
    /// The halvings of the month that give the bodies' step of cell `i`: its depth below a square of the map (none for a
    /// square or a coarser cell), at most as many as keep the step a whole number of seconds and at least
    /// `STEP_LEAST_S`.
    pub fn halvings(&self, i: usize) -> u32 {
        let month = self.cfg.calendar().month;
        let most = (0..month.trailing_zeros()).take_while(|&k| month >> (k + 1) >= STEP_LEAST_S).count() as u32;
        (self.map.leaves()[i].depth.saturating_sub(self.map.square_depth()) as u32).min(most)
    }
    /// The bodies' step of cell `i`, s: the month over 2^`halvings`.
    pub fn body_step(&self, i: usize) -> u64 {
        self.cfg.calendar().month >> self.halvings(i)
    }
    /// The bodies' step of cell `i`, d: the time from an update there to the next.
    pub fn step_days(&self, i: usize) -> f64 {
        self.body_step(i) as f64 / self.cfg.calendar().day as f64
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
    let parts = 1u64 << most;
    for k in 1..parts {
        // (A cell of `h` halvings steps at every (`parts` >> h)-th sub-step.)
        let steps: Vec<bool> = halvings.iter().map(|&h| k.is_multiple_of(parts >> h)).collect();
        if !w.life.living().any(|(h, _)| steps[h.cell as usize]) {
            continue;
        }
        w.time = t0 + k * (month >> most);
        break_and_sum(w, Some(&steps));
        choose::cells(w);
        batch_of(w, Some(&steps));
    }
    w.time = t0;
}
