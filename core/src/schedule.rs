//! The order of the laws, as processes of the clock: each runs on its own period, and at one time they run in the
//! order of `processes`, written once here (laws.md 0).

use crate::clock::{self, Every};
use crate::params::Config;
use crate::units::HOUR_S;
use crate::world::World;
use crate::{bond, change, check, flow, survey};

/// A stage of the month, as `step_marked` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// the parts of Flow (`flow`)
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

/// After a stage: `mark(stage)`; in debug builds, stop at the first stage that leaves the world broken.
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

/// A process: its stage, its name (for the debug check) and what it does for the seconds its run stands for.
type Run = (Stage, &'static str, fn(&mut World, u64));

/// The periods of the world's processes, each counted from the start of the longer period that holds it (`Every`).
struct Periods {
    physics: Every,
    air: Every,
    liquid: Every,
    creep: Every,
    refresh: Every,
    month: Every,
    day: Every,
}

impl Periods {
    fn of(cfg: &Config) -> Periods {
        let cal = cfg.calendar();
        let days = |d: u32| Every { period: d as u64 * cal.day, within: cal.month };
        // The slow parts count from the start of the world.
        let from_start = |period: u64| Every { period, within: period };
        Periods {
            physics: days(cfg.phys_days),
            air: Every { period: cfg.air_hours as u64 * HOUR_S, within: cal.month },
            liquid: days(cfg.flow_days),
            creep: from_start(flow::CREEP_YEARS * cal.year),
            refresh: from_start(flow::REFRESH_MONTHS * cal.month),
            month: from_start(cal.month),
            day: days(1),
        }
    }
}

/// The processes of the world in their fixed order at one time; Flow's state comes first, as the others read it.
fn processes(w: &World) -> [(Every, Run); 9] {
    let p = Periods::of(&w.cfg);
    [
        (p.physics, (Stage::Bodies, "survey", |w, _| survey::survey(w))),
        (p.physics, (Stage::Flow, "state", |w, _| flow::state(w, Periods::of(&w.cfg).liquid.span(w.time).is_some()))),
        (p.air, (Stage::Flow, "air", |w, s| flow::air(w, s as f64))),
        (p.liquid, (Stage::Flow, "liquid", |w, s| flow::liquid(w, s as f64))),
        (p.creep, (Stage::Flow, "creep", |w, s| flow::creep(w, s as f64))),
        (p.refresh, (Stage::Flow, "refresh", |w, _| flow::refresh(w))),
        (p.physics, (Stage::Change, "change", |w, s| change::step(w, s as f64))),
        // (Bond's own spans are months, `bond::step`; the bodies' intervals whole days, `survey::day`.)
        (p.month, (Stage::Parts, "bond", |w, _| bond::step(w))),
        (p.day, (Stage::Bodies, "body day", |w, _| survey::day(w))),
    ]
}

/// Run the processes due from the world's time on, for `span` seconds.
fn run_for(w: &mut World, span: u64, mark: &mut dyn FnMut(Stage)) {
    let (list, end) = (processes(w), w.time + span);
    while w.time < end {
        let t = w.time;
        for ((stage, what, run), span) in clock::due(&list, t) {
            run(w, span);
            after(w, stage, what, mark);
        }
        w.time = clock::next(&list, t).min(end);
    }
}
