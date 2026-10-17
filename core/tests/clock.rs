//! The clock: processes run at their periods inside longer ones, in a fixed order at one time; the day, the month and
//! the season come from the seconds; the month of `step` is built on it.

use pcore::clock::{self, Every};
use pcore::units::DAY_S;
use pcore::{Config, Stage};
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

/// A month of `step`: on its first day the survey, Flow and Change, then Bond, then the bodies; the physics every
/// `phys_days` days and the bodies every day. The clock ends a whole month on, and `day` moves it by one day.
#[test]
fn a_month_runs_its_processes_in_their_order() {
    let mut w = small_world();
    let (t0, cal) = (w.time, w.cfg.calendar());
    let mut marks = vec![];
    pcore::step_marked(&mut w, &mut |s| marks.push(s));
    assert_eq!(w.time, t0 + cal.month);
    assert_eq!(marks[..6], [Stage::Bodies, Stage::Flow, Stage::Change, Stage::Parts, Stage::Bodies, Stage::Bodies]);
    let count = |s: Stage| marks.iter().filter(|&&m| m == s).count() as u32;
    let (dpm, phys) = (w.cfg.days_per_month, w.cfg.phys_days);
    assert_eq!((count(Stage::Flow), count(Stage::Change), count(Stage::Parts)), (dpm.div_ceil(phys), dpm.div_ceil(phys), 1));
    assert_eq!(count(Stage::Bodies), dpm + dpm.div_ceil(phys));
    let (d, m) = (w.day(), w.month());
    pcore::day(&mut w);
    assert_eq!((w.day(), w.month(), w.time % cal.day), (d + 1, m, 0));
}
