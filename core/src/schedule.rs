//! The order of the laws in one month.
//!
//! Every `Config::phys_days` days:
//! 1. `body::survey`: what each cell holds, its cover, light and heat set aside for its bodies;
//!    once a month the merge of alike nodes and each cell's surroundings.
//! 2. `flow::step`: radiation every step; once `air_days` have passed the exchange of the ground
//!    with the air (every `exchange_days`) and the wind; once `flow_days` have passed liquid and
//!    drainage, and creep once a year; the slow ground rechecked every few months.
//! 3. `change::step`: the reactions of ground and air nodes.
//!
//! On the first day of the month `bond::step` (parts and structures), and every day `body::day`
//! (the bodies whose update is due: their physics, decisions and copying).

use crate::units::DAY;
use crate::world::World;
use crate::{body, bond, change, check, flow};

/// A stage of the month, as `step_marked` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// `flow::step`
    Flow,
    /// `change::step`
    Change,
    /// `body::survey` and `body::day`
    Bodies,
    /// `bond::step`
    Parts,
}

/// Advance the world by one month. This is the whole step: `next_state = step(state)`.
pub fn step(w: &mut World) {
    step_marked(w, &mut |_| {});
}

/// Debug builds only: stop at the first stage that leaves the world broken (the yearly check of
/// the shell comes too late to say which stage did it). Release builds skip this.
#[inline]
fn audit(w: &World, stage: &str) {
    if cfg!(debug_assertions) {
        if let Some(e) = check::broken(w) {
            panic!("broken after {stage} (day {}): {e}", w.day);
        }
    }
}

/// `step`, calling `mark(stage)` after each stage, e.g. to time them.
pub fn step_marked(w: &mut World, mark: &mut dyn FnMut(Stage)) {
    let dpm = w.cfg.days_per_month;
    for d in 0..dpm {
        if d % w.cfg.phys_days == 0 {
            let dt = w.cfg.phys_days.min(dpm - d) as f64 * DAY;
            body::survey(w);
            mark(Stage::Bodies);
            audit(w, "survey");
            flow::step(w, dt);
            mark(Stage::Flow);
            audit(w, "flow");
            change::step(w, dt);
            mark(Stage::Change);
            audit(w, "change");
        }
        if d == 0 {
            bond::step(w);
            mark(Stage::Parts);
            audit(w, "bond");
        }
        body::day(w);
        w.day += 1;
        mark(Stage::Bodies);
        audit(w, "body day");
    }
}
