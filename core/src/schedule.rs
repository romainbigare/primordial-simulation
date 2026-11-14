//! The order of the laws: one world step is one month, a fixed list of passes, written once here (laws.md 0).

use crate::{change, check, contract, flow, life, world::World};

/// The air runs in steps of this many days, a whole number of them in a month: each cell's exchange over a step is
/// solved once and taken as linear in its air (`flow::air`), so a longer step smooths the seasons. The long-step test
/// anchors it (`the_long_step_keeps_the_climate`: these steps against 1-day steps, the land within 0.3 K).
pub const AIR_STEP_DAYS: u32 = 15;
/// Heat conducted sideways and creep run once in this many months, for all that time: on
/// cells of kilometres both are slow, far below the limit of an explicit step.
const CREEP_MONTHS: u64 = 12;
/// The slow-changing ground and the substances present are rechecked once in this many months (`flow::refresh`).
const REFRESH_MONTHS: u64 = 3;

/// A stage of the month, as `step_marked` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// the parts of Flow (`flow`), and the surroundings it gives life (`contract::surroundings`)
    Flow,
    /// `change::step`
    Change,
    /// life: what stands on each cell and the month of its cover and its takers (`contract::stands`, `life::month`)
    Cover,
}

/// A pass of a month: its stage, its name and what it runs.
type Pass = (Stage, &'static str, fn(&mut World));

/// The passes of a month's start, in order: what stands on each cell (`contract::stands`), then
/// the surroundings, Flow's state built in full and each cell's place from it (`contract::surroundings`).
const START: [Pass; 2] = [(Stage::Cover, "stands", contract::stands), (Stage::Flow, "surroundings", contract::surroundings)];

/// The passes of a month's start alone (`START`): for a world placed or edited between months, which the laws read
/// as they would at a month's start.
pub fn month_start(w: &mut World) {
    START.iter().for_each(|&(_, _, run)| run(w));
}

/// Advance the world by one month. This is the whole step: `next_state = step(state)`.
pub fn step(w: &mut World) {
    step_marked(w, &mut |_, _| {});
}

/// `step`, calling `mark(world, stage)` after each pass, e.g. to time them.
pub fn step_marked(w: &mut World, mark: &mut dyn FnMut(&mut World, Stage)) {
    run_for(w, w.cfg.calendar().month, AIR_STEP_DAYS, mark);
}

/// `step` with the air in steps of `air_days` days (a whole number of them in a month): the control run the long step
/// of the air is checked against (`AIR_STEP_DAYS`).
pub fn step_with_air(w: &mut World, air_days: u32) {
    assert!(w.cfg.days_per_month.is_multiple_of(air_days), "the air's steps of {air_days} days fill no month");
    run_for(w, w.cfg.calendar().month, air_days, &mut |_, _| {});
}

/// Advance the world by one day; on a month's first day the whole month's passes run first, as they stand for the whole
/// month (the other days of a month run nothing).
pub fn day_with_month_at_start(w: &mut World) {
    day_with_month_at_start_marked(w, &mut |_, _| {});
}

/// `day_with_month_at_start`, calling `mark(world, stage)` after each pass.
pub fn day_with_month_at_start_marked(w: &mut World, mark: &mut dyn FnMut(&mut World, Stage)) {
    run_for(w, w.cfg.calendar().day, AIR_STEP_DAYS, mark);
}

/// Run `span` seconds from the world's time: the month's passes at each month's start on the way (`month`).
fn run_for(w: &mut World, span: u64, air_days: u32, mark: &mut dyn FnMut(&mut World, Stage)) {
    let (len, end) = (w.cfg.calendar().month, w.time + span);
    while w.time < end {
        if w.time.is_multiple_of(len) {
            month(w, air_days, mark);
        }
        w.time = (w.time - w.time % len + len).min(end);
    }
}

/// The passes of one month from its start, each for the whole month unless it says otherwise; after each,
/// `mark(world, stage)`, and in debug builds a stop at the first that leaves the world broken.
fn month(w: &mut World, air_days: u32, mark: &mut dyn FnMut(&mut World, Stage)) {
    let cal = w.cfg.calendar();
    let (t0, len, months) = (w.time, cal.month, cal.months(w.time));
    let mut pass = |w: &mut World, stage: Stage, what: &str, run: &dyn Fn(&mut World)| {
        run(w);
        mark(w, stage);
        if let Some(e) = cfg!(debug_assertions).then(|| check::broken(w)).flatten() {
            panic!("broken after {what} (day {}): {e}", w.day());
        }
    };
    // The month's start builds Flow's state in full for the cells' surroundings; Flow's own pass leaves it then.
    for (stage, what, run) in START {
        pass(w, stage, what, &run);
    }
    // Flow's parts follow each other with nothing between, from here to its refresh: their lifts and falls are booked
    // once (`flow::chain`).
    flow::chain(w, true);
    pass(w, Stage::Flow, "state", &|w| {
        if !flow::built_now(w) {
            flow::state(w, true);
        }
    });
    // The air in steps of `air_days`, each at its own time in the month (the sun's season), its state brought up to date
    // first and the sink of the liquid over field capacity into the bed's store after (`flow::air`).
    let air = air_days as u64 * cal.day;
    for k in 0..len / air {
        w.time = t0 + k * air;
        pass(w, Stage::Flow, "air", &|w| flow::air(w, air as f64));
    }
    w.time = t0;
    pass(w, Stage::Flow, "liquid", &|w| flow::liquid(w, len as f64));
    if months.is_multiple_of(CREEP_MONTHS) {
        pass(w, Stage::Flow, "creep", &|w| flow::creep(w, (CREEP_MONTHS * len) as f64));
    }
    if months.is_multiple_of(REFRESH_MONTHS) {
        pass(w, Stage::Flow, "refresh", &flow::refresh);
    }
    flow::chain(w, false);
    // The month of life (the cover, then the takers) reads Flow's state and the air as its last step left it; then
    // Change.
    pass(w, Stage::Cover, "life", &life::month);
    pass(w, Stage::Change, "change", &|w| change::step(w, len as f64));
}
