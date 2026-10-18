//! The order of the laws, as processes of the clock (`clock`): each runs on its own period, and at one time they run
//! in the order of `processes`, written once here.
//!
//! Every `Config::phys_days` days:
//! 1. `survey::survey`: what each cell holds, its cover, light and heat set aside for its bodies;
//!    once a month the merge of alike nodes and each cell's surroundings.
//! 2. Flow (`flow`): the state of ground and air and radiation at every step; the exchange of the ground with the air
//!    every `exchange_days` (the trace gases at every `flow::TRACE_TURN`-th), the wind every `air_days`, liquid and
//!    drainage every `flow_days`, heat through the ground and creep every `flow::CREEP_YEARS` years, and the slow
//!    ground rechecked every `flow::REFRESH_MONTHS` months.
//! 3. `change::step`: the reactions of ground and air nodes.
//!
//! On the first day of the month `bond::step` (parts and structures), and every day `survey::day`
//! (the bodies whose update is due: their physics, decisions and copying).

use crate::clock::{self, Every};
use crate::params::Config;
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

/// What a process is at one time: the stage it belongs to, its name (for the debug check, `after`), and what it does,
/// given the seconds its run stands for.
type Run = (Stage, &'static str, fn(&mut World, u64));

/// The periods of the world's processes, each counted from the start of the longer period that holds it (`Every`):
/// the physics every `phys_days` days of the month (the last span cut to the month's end), and each part of Flow on its
/// own period inside it; Bond every month, the bodies every day.
struct Periods {
    physics: Every,
    exchange: Every,
    trace: Every,
    wind: Every,
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
            exchange: days(cfg.exchange_days),
            trace: days(flow::TRACE_TURN * cfg.exchange_days),
            wind: days(cfg.air_days),
            liquid: days(cfg.flow_days),
            creep: from_start(flow::CREEP_YEARS * cal.year),
            refresh: from_start(flow::REFRESH_MONTHS * cal.month),
            month: from_start(cal.month),
            day: days(1),
        }
    }
}

/// The processes of the world in their fixed order at one time. The state of ground and air is rebuilt in full at
/// each liquid step; the trace gases move at their own turns of the exchange.
fn processes(w: &World) -> [(Every, Run); 10] {
    let p = Periods::of(&w.cfg);
    [
        (p.physics, (Stage::Bodies, "survey", |w, _| survey::survey(w))),
        (p.physics, (Stage::Flow, "radiation", |w, s| flow::radiate(w, s as f64, Periods::of(&w.cfg).liquid.span(w.time).is_some()))),
        (p.exchange, (Stage::Flow, "exchange", |w, s| flow::exchange(w, s as f64, Periods::of(&w.cfg).trace.span(w.time).map(|t| t as f64)))),
        (p.wind, (Stage::Flow, "wind", |w, s| flow::wind(w, s as f64))),
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
    // The groups' rows follow their members before anything outside the laws reads them.
    w.life.settle_all();
}
