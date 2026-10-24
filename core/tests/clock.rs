//! The clock: processes run at their periods inside longer ones, in a fixed order at one time; the day, the month and
//! the season come from the seconds; the month of `step` is built on it. A calm node sleeps until its alarm or until
//! it has gained more than a share of what it held, and then catches up.

use pcore::clock::{self, Every, Sleep};
use pcore::store::Class;
use pcore::units::DAY_S;
use pcore::{Config, Physics, Stage};
use worldgen::testkit::small_world;

/// A fast process runs a whole number of times inside a slow one, each run standing for the time to the next; a span
/// that does not fill the slow period is cut at its end, and the next run starts the next slow period.
#[test]
fn a_fast_process_runs_several_times_inside_a_slow_one() {
    let fast = Every { period: 3, within: 10 };
    let runs: Vec<(u64, u64)> = (0..20).filter_map(|t| fast.span(t).map(|s| (t, s))).collect();
    assert_eq!(runs, [(0, 3), (3, 3), (6, 3), (9, 1), (10, 3), (13, 3), (16, 3), (19, 1)]);
    assert_eq!([0, 2, 8, 9, 10].map(|t| fast.next(t)), [3, 3, 9, 10, 13]);
}

/// At one time the processes due run in the order of their list, and the clock then goes to the next time any is
/// due: over a slow period each process runs period / its period times, and its spans add up to the whole period.
#[test]
fn the_processes_due_at_one_time_run_in_the_order_of_their_list() {
    let list = [(Every { period: 5, within: 10 }, 'a'), (Every { period: 10, within: 10 }, 'b'), (Every { period: 2, within: 10 }, 'c')];
    assert_eq!(clock::due(&list, 0).collect::<Vec<_>>(), [('a', 5), ('b', 10), ('c', 2)]);
    assert_eq!(clock::due(&list, 5).collect::<Vec<_>>(), [('a', 5)]);
    let (mut t, mut ran) = (0, vec![]);
    while t < 20 {
        ran.extend(clock::due(&list, t));
        t = clock::next(&list, t);
    }
    for (p, runs) in [('a', 4), ('b', 2), ('c', 10)] {
        assert_eq!(ran.iter().filter(|r| r.0 == p).count(), runs, "{p}");
        assert_eq!(ran.iter().filter(|r| r.0 == p).map(|r| r.1).sum::<u64>(), 20, "{p}");
    }
}

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

/// A month of `step`: on its first day the survey, the parts of Flow due (at a month's start at least its state, the air
/// and liquid) and Change, then Bond, then the bodies; the physics every `phys_days` days and the
/// bodies every `batch_days` days. The clock ends a whole month on, and `day` moves it by one day.
#[test]
fn a_month_runs_its_processes_in_their_order() {
    let mut w = small_world();
    let (t0, cal) = (w.time, w.cfg.calendar());
    let mut marks = vec![];
    pcore::step_marked(&mut w, &mut |s| marks.push(s));
    assert_eq!(w.time, t0 + cal.month);
    let flow = marks[1..].iter().take_while(|&&m| m == Stage::Flow).count();
    assert!(marks[0] == Stage::Bodies && flow >= 3, "{marks:?}");
    assert_eq!(marks[1 + flow..][..3], [Stage::Change, Stage::Parts, Stage::Bodies]);
    let count = |s: Stage| marks.iter().filter(|&&m| m == s).count() as u32;
    let (dpm, phys) = (w.cfg.days_per_month, w.cfg.phys_days);
    assert_eq!((count(Stage::Change), count(Stage::Parts)), (dpm.div_ceil(phys), 1));
    assert_eq!(count(Stage::Bodies), dpm / w.cfg.batch_days + dpm.div_ceil(phys));
    let (d, m) = (w.day(), w.month());
    pcore::day(&mut w);
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

/// The default settings pass; a time step of 0, an air step that does not divide the physics step, a batch that does not
/// divide the physics step or the month, a pace of nodes that is not whole batches, or a dormancy of 1 is refused.
#[test]
fn the_default_settings_pass_and_a_step_off_its_base_or_out_of_step_or_a_dormancy_of_1_is_refused() {
    assert_eq!(Config::default().validate(), Ok(()));
    let dormant = Config { phys: Physics { dormancy: 1.0, ..Default::default() }, ..Default::default() };
    let refused = [
        Config { flow_days: 0, ..Default::default() },
        Config { air_hours: 0, ..Default::default() },
        Config { air_hours: 480, ..Default::default() },
        Config { phys_days: 0, ..Default::default() },
        Config { batch_days: 0, ..Default::default() },
        Config { batch_days: 4, ..Default::default() },
        Config { batch_days: 5, node_days: 36, ..Default::default() },
        Config { batch_days: 10, node_days: 36, phys_days: 30, ..Default::default() },
        dormant,
    ];
    for c in refused {
        assert!(c.validate().is_err(), "{} {} {} {} {}", c.phys_days, c.air_hours, c.flow_days, c.batch_days, c.phys.dormancy);
    }
}
