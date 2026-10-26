//! The order of the laws, as processes of the clock: each runs on its own period, and at one time they run in the
//! order of `processes`, written once here (laws.md 0).

use crate::{bond, change, check, clock::{self, Every}, flow, params::Config, survey, units::HOUR_S, world::World};

/// A stage of the month, as `step_marked` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// the parts of Flow (`flow`)
    Flow,
    /// `change::step`
    Change,
    /// `survey::survey` and `survey::batch`
    Bodies,
    /// `bond::step`
    Parts,
}

/// Advance the world by one month. This is the whole step: `next_state = step(state)`.
pub fn step(w: &mut World) {
    step_marked(w, &mut |_| {});
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

/// A process that runs every `d` days from the start of each month; one of more than a month counts its own period
/// from the start of the world, as the slow parts do (`Config::validate` makes it a whole number of months).
fn days(cfg: &Config, d: u32) -> Every {
    let cal = cfg.calendar();
    let period = d as u64 * cal.day;
    Every { period, within: period.max(cal.month) }
}

/// The processes of the world in their fixed order at one time, each on its period counted from the start of the longer
/// period that holds it (`Every`); Flow's state comes first, as the others read it.
fn processes(w: &World) -> [(Every, Run); 9] {
    let (cfg, cal) = (&w.cfg, w.cfg.calendar());
    // The slow parts count from the start of the world.
    let from_start = |period: u64| Every { period, within: period };
    let physics = days(cfg, cfg.phys_days);
    [
        (physics, (Stage::Bodies, "survey", |w, _| survey::survey(w, false))),
        // (A month's start that is a full state's has it already: the survey rebuilt it in full for the cells' surroundings.)
        (physics, (Stage::Flow, "state", |w, _| {
            let full = days(&w.cfg, w.cfg.flow_days).span(w.time).is_some();
            if !(full && survey::monthly(w)) {
                flow::state(w, full);
            }
        })),
        (Every { period: cfg.air_hours as u64 * HOUR_S, within: cal.month }, (Stage::Flow, "air", |w, s| flow::air(w, s as f64))),
        (days(cfg, cfg.flow_days), (Stage::Flow, "liquid", |w, s| flow::liquid(w, s as f64))),
        (from_start(flow::CREEP_YEARS * cal.year), (Stage::Flow, "creep", |w, s| flow::creep(w, s as f64))),
        (from_start(flow::REFRESH_MONTHS * cal.month), (Stage::Flow, "refresh", |w, _| flow::refresh(w))),
        (physics, (Stage::Change, "change", |w, s| change::step(w, s as f64))),
        // (Bond's own spans are months, `bond::step`; the bodies' intervals whole batches of days, `survey::batch`.)
        (from_start(cal.month), (Stage::Parts, "bond", |w, _| bond::step(w))),
        (days(cfg, cfg.batch_days), (Stage::Bodies, "body batch", |w, _| survey::batch(w, false))),
    ]
}

/// Run the processes due from the world's time on, for `span` seconds; after each, `mark(stage)`, and in debug builds a
/// stop at the first stage that leaves the world broken.
fn run_for(w: &mut World, span: u64, mark: &mut dyn FnMut(Stage)) {
    let (list, end) = (processes(w), w.time + span);
    while w.time < end {
        let t = w.time;
        for ((stage, what, run), span) in clock::due(&list, t) {
            run(w, span);
            mark(stage);
            if let Some(e) = cfg!(debug_assertions).then(|| check::broken(w)).flatten() {
                panic!("broken after {what} (day {}): {e}", w.day());
            }
        }
        w.time = clock::next(&list, t).min(end);
    }
}
