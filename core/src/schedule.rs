//! The order of the laws in one month, as processes of the clock (`clock`).
//!
//! Every `Config::phys_days` days:
//! 1. `survey::survey`: what each cell holds, its cover, light and heat set aside for its bodies;
//!    once a month the merge of alike nodes and each cell's surroundings.
//! 2. `flow::step`: radiation every step; the exchange of the ground with the air every
//!    `exchange_days`, the wind every `air_days`, liquid and drainage every `flow_days`, creep
//!    once a year; the slow ground rechecked every few months.
//! 3. `change::step`: the reactions of ground and air nodes.
//!
//! On the first day of the month `bond::step` (parts and structures), and every day `survey::day`
//! (the bodies whose update is due: their physics, decisions and copying).

use crate::clock::{self, Every};
use crate::world::World;
use crate::{bond, change, check, flow, survey};

/// A stage of the month, as `step_marked` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// `flow::step`
    Flow,
    /// `change::step`
    Change,
    /// `survey::survey` and `survey::day`
    Bodies,
    /// `bond::step`
    Parts,
}

/// Advance the world by one month. This is the whole step: `next_state = step(state)`.
pub fn step(w: &mut World) {
    step_marked(w, &mut |_| {});
}

/// After a stage: `mark(stage)`; and, in debug builds only, stop at the first stage that leaves the world broken
/// (the yearly check of the shell comes too late to say which stage did it, `what`). Release builds skip the check.
#[inline]
fn after(w: &World, stage: Stage, what: &str, mark: &mut dyn FnMut(Stage)) {
    mark(stage);
    if cfg!(debug_assertions) {
        if let Some(e) = check::broken(w) {
            panic!("broken after {what} (day {}): {e}", w.day());
        }
    }
}

/// `step`, calling `mark(stage)` after each stage, e.g. to time them.
pub fn step_marked(w: &mut World, mark: &mut dyn FnMut(Stage)) {
    run_for(w, w.cfg.calendar().month, mark);
}

/// Advance the world by one day: what `step` does on that day.
pub fn day(w: &mut World) {
    run_for(w, w.cfg.calendar().day, &mut |_| {});
}

/// What a process does at one time: the world, the seconds the run stands for, and the mark of its stages.
type Run = fn(&mut World, u64, &mut dyn FnMut(Stage));

/// The processes of the world in their fixed order at one time: the physics every `phys_days` days from the month's
/// start (the last span cut to the month's end), Bond at the month's start, then the bodies due, every day.
fn processes(w: &World) -> [(Every, Run); 3] {
    let cal = w.cfg.calendar();
    let every = |period| Every { period, within: cal.month };
    [(every(w.cfg.phys_days as u64 * cal.day), physics), (every(cal.month), parts), (every(cal.day), bodies)]
}

/// Run the processes due from the world's time on, for `span` seconds.
fn run_for(w: &mut World, span: u64, mark: &mut dyn FnMut(Stage)) {
    let (list, end) = (processes(w), w.time + span);
    while w.time < end {
        let t = w.time;
        for (run, span) in clock::due(&list, t) {
            run(w, span, mark);
        }
        w.time = clock::next(&list, t).min(end);
    }
    // The groups' rows follow their members before anything outside the laws reads them.
    w.life.settle_all();
}

/// The survey, Flow and Change, for `span` seconds.
fn physics(w: &mut World, span: u64, mark: &mut dyn FnMut(Stage)) {
    let dt = span as f64;
    survey::survey(w);
    after(w, Stage::Bodies, "survey", mark);
    flow::step(w, dt);
    after(w, Stage::Flow, "flow", mark);
    change::step(w, dt);
    after(w, Stage::Change, "change", mark);
}

/// Bond: parts and structures (its own spans are months, `bond::step`).
fn parts(w: &mut World, _: u64, mark: &mut dyn FnMut(Stage)) {
    bond::step(w);
    after(w, Stage::Parts, "bond", mark);
}

/// The bodies whose update is due (their own intervals are whole days, `survey::day`).
fn bodies(w: &mut World, _: u64, mark: &mut dyn FnMut(Stage)) {
    survey::day(w);
    after(w, Stage::Bodies, "body day", mark);
}
