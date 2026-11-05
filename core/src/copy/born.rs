//! Children are born through each sub-step of their parent's update, as its brood fills (`Fill`), and feed and count
//! their time from their birth (`feed`).

use crate::body::Body;
use crate::choose::eat_for;
use crate::{life::Chunk, World};

/// How the brood of a body filled over a sub-step of its update (`body::live`): what it held before growth, mol, the
/// days the sub-step integrated, the share of them, at their end, over which growth filled it (`Grew::filling`), and
/// the time the sub-step ends at, s. Growth builds at one rate over a sub-step, so the brood filled evenly over that
/// share.
#[derive(Clone, Copy)]
pub struct Fill {
    pub before: f64,
    pub filling: f64,
    pub days: f64,
    pub at: u64,
}

impl Fill {
    /// The day of the update, from its start, at which the brood, `after` mol at its end, filled to `x` mol, at least what
    /// it held before.
    fn reached(&self, x: f64, after: f64) -> f64 {
        let start = (1.0 - self.filling) * self.days;
        let share = if after > self.before { ((x - self.before) / (after - self.before)).clamp(0.0, 1.0) } else { 0.0 };
        start + share * (self.days - start)
    }

    /// The mean age at the update's end, d, of the `births` children of `matter` mol each that the brood, `after` mol at
    /// its end, covers beyond the `held` mol it keeps: each is born when the brood covered it, at the update's start if
    /// it did before. A node's members are born as the brood fills past `held` (members are a continuous amount, so
    /// evenly from the time it passed it); an individual's whole children each when the brood reached it.
    pub(super) fn age(&self, after: f64, (held, matter, births): (f64, f64, f64), node: bool) -> f64 {
        let born = if node {
            // (What it held before past `held` was born at the start; the rest evenly from when it passed both.)
            let from = held.max(self.before);
            (after - from) * (self.reached(from, after) + self.days) / 2.0 / (after - held).max(1e-300)
        } else {
            let at = |j: f64| if j * matter <= self.before { 0.0 } else { self.reached(j * matter, after) };
            (1..=births as u64).map(|j| at(j as f64)).sum::<f64>() / births.max(1.0)
        };
        (self.days - born).max(0.0)
    }
}

/// Children `c` (a body not placed, in their parent's cell) born `age` d before `now`, the end of their parent's
/// sub-step, d: their time is counted from their birth (`Body::last`), so the first sub-step that integrates them,
/// their node's (`nodes::rejoin`: members who join count from when they came) or their own, runs them from then by the
/// physics of every body; and from their birth to `now` they take what lies there by the take law, as a body does for
/// the part of its step no act fills (`choose::eat_for`; laws.md 5.3; a body with no brain takes none, as it takes no
/// decision), and at each later sub-step as any body does (`choose::meal`).
pub(super) fn feed(w: &World, d: &mut Chunk, c: &mut Body, (now, age): (f64, f64)) {
    (c.last, c.decided) = (now - age, now - age);
    if age > 0.0 && !w.life.brainless(c) {
        eat_for(w, d, c, age);
    }
}
