//! The clock: the day, the month and the season come from the seconds; the month of `step` is a fixed list of passes.
//! A calm node sleeps until its alarm or until it has gained more than a share of what it held, and then catches up.

use pcore::clock::Sleep;
use pcore::store::Class;
use pcore::units::DAY_S;
use pcore::{Config, Physics, Stage};
use worldgen::testkit::small_world;

/// The day, the month, the year and the season are the world's seconds counted in its calendar.
#[test]
fn day_month_and_season_come_from_the_seconds() {
    let cal = Config::default().calendar();
    assert_eq!((cal.day, cal.month, cal.year), (DAY_S, 30 * DAY_S, 360 * DAY_S));
    let t = 400 * DAY_S + DAY_S / 2;
    assert_eq!((cal.days(t), cal.months(t), cal.years(t)), (400, 13, 1));
    assert_eq!(cal.season(t), (40.5 * DAY_S as f64) / (360 * DAY_S) as f64);
    assert_eq!(cal.season(cal.year / 2), 0.5);
}

/// A month of `step`: its start (what stands, the surroundings), Flow's state, the air (with the sink into the bed's
/// store) in steps of `AIR_STEP_DAYS`, the liquid (with creep and the refresh at their months), the cover's month, then
/// Change, each once. The clock ends a whole month on, and `day_with_month_at_start` moves it by one day, the month's passes only at a month's start.
#[test]
fn a_month_runs_its_passes_in_their_order() {
    let mut w = small_world();
    let (t0, cal) = (w.time, w.cfg.calendar());
    let mut marks = vec![];
    pcore::step_marked(&mut w, &mut |_, s| marks.push(s));
    assert_eq!(w.time, t0 + cal.month);
    let air = (w.cfg.days_per_month / pcore::AIR_STEP_DAYS) as usize;
    let flow = marks[1..].iter().take_while(|&&m| m == Stage::Flow).count();
    assert!(marks[0] == Stage::Cover && flow >= 3 + air, "{marks:?}");
    assert_eq!(marks[1 + flow..], [Stage::Cover, Stage::Change]);
    let (d, m) = (w.day(), w.month());
    pcore::day_with_month_at_start(&mut w);
    assert_eq!((w.day(), w.month(), w.time % cal.day), (d + 1, m, 0));
}

/// A sleeping node is not due before its alarm; at its alarm it is, with the other nodes of its class due, in the order
/// of their indices. A node with no alarm is never due by itself.
#[test]
fn a_calm_node_sleeps_until_its_alarm() {
    let mut s = Sleep::default();
    for (i, until) in [(3, 50), (1, 100), (2, 50), (4, u64::MAX)] {
        s.sleep(Class::Ground, i, 0, until);
    }
    s.sleep(Class::Air, 1, 0, 10);
    assert!(s.due(Class::Ground, 49).is_empty());
    assert_eq!(s.due(Class::Ground, 120), [1, 2, 3]);
    assert!(s.due(Class::Ground, u64::MAX - 1).is_empty());
    assert_eq!(s.due(Class::Air, 10), [1]);
}

/// A sleeping node wakes when one of its amounts has grown by more than `wake_share` of what it held when it fell
/// asleep, in as many gains as it took and whatever brought them: gains that add up to less leave it asleep, and so
/// does a loss.
#[test]
fn a_node_that_gained_more_than_its_share_wakes() {
    let (len, share, now) = (3, 0.05, 0);
    let (mut s, mut amounts) = (Sleep::default(), vec![1.0; 2 * len]);
    for i in 0..len {
        s.sleep(Class::Ground, i, now, now + DAY_S);
        s.keep(Class::Ground, i, len, &amounts);
    }
    let gain = 0.6 * share;
    amounts[len] += gain;
    s.watch(Class::Ground, len, &amounts, share, now);
    assert!(s.due(Class::Ground, now).is_empty(), "one small gain");
    (amounts[len], amounts[1], amounts[2]) = (amounts[len] + gain, amounts[1] + gain, amounts[2] - 0.5);
    s.watch(Class::Ground, len, &amounts, share, now);
    assert_eq!(s.due(Class::Ground, now), [0], "two small gains of node 0");
    assert_eq!(s.due(Class::Ground, now + DAY_S), [1, 2]);
}

/// What a law's own step may differ by between one span and many steps of the same time, relative: rounding, as the
/// test law integrates exactly.
const CATCH_UP: f64 = 1e-12;

/// A law that relaxes a node's amount towards 1 at a rate of 1e-7 /s, run on two nodes: one due every step of 3 days,
/// one asleep for a year but woken once by a gain. Each time it is due it is brought up to date for the time since
/// (`Sleep::since`) in one span; at the year's end both hold the same.
#[test]
fn a_sleeping_node_catches_up_as_an_awake_node_steps() {
    let (dt, year, rate) = (3 * DAY_S, 360 * DAY_S, 1e-7);
    let relax = |a: f64, span: u64| 1.0 + (a - 1.0) * (-rate * span as f64).exp();
    let (awake, asleep, g) = (0, 1, Class::Ground);
    let mut s = Sleep::default();
    let mut amount = [0.0, 0.0];
    s.sleep(g, awake, 0, dt);
    s.sleep(g, asleep, 0, year);
    let mut t = 0;
    while t < year {
        t += dt;
        if t == 100 * DAY_S {
            s.wake(g, asleep, t);
        }
        for i in s.due(g, t) {
            let a = &mut amount[i];
            *a = relax(*a, t - s.since(g, i));
            s.sleep(g, i, t, if i == awake { t + dt } else { year });
        }
    }
    assert_eq!((s.since(g, awake), s.since(g, asleep)), (year, year));
    assert!((amount[0] - amount[1]).abs() <= CATCH_UP * amount[0], "{amount:?}");
    assert!(amount[0] > 0.9, "the law ran: {amount:?}");
}

/// The default settings pass; a month that is not a whole number of the air's steps, or a dormancy of 1, is refused.
#[test]
fn the_default_settings_pass_and_a_month_off_the_air_steps_or_a_dormancy_of_1_is_refused() {
    assert_eq!(Config::default().validate(), Ok(()));
    let dormant = Config { phys: Physics { dormancy: 1.0, ..Default::default() }, ..Default::default() };
    for c in [Config { days_per_month: 25, ..Default::default() }, Config { days_per_month: 0, ..Default::default() }, dormant] {
        assert!(c.validate().is_err(), "{} {}", c.days_per_month, c.phys.dormancy);
    }
}
