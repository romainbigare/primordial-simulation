//! The clock: processes run at their periods inside longer ones, in a fixed order at one time; the day, the month and
//! the season come from the seconds; the month of `step` is built on it. A calm node sleeps until its alarm or until a
//! large move into it, and then catches up.

use pcore::clock::{self, Every, Sleep};
use pcore::store::{apply, Boundary, Class, LinkFlux, Move, NodeId};
use pcore::units::DAY_S;
use pcore::{Config, Stage, World};
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

/// The node of slot `slot` (a test's own ids: `Sleep` asks nothing of the table).
fn node(slot: u32) -> NodeId {
    NodeId { slot, gen: 0 }
}

/// A sleeping node is not due before its alarm; at its alarm it is, after the nodes whose alarms rang earlier and
/// with those of the same alarm in slot order. A node with no alarm is never due by itself.
#[test]
fn a_calm_node_sleeps_until_its_alarm() {
    let mut s = Sleep::default();
    for (slot, until) in [(3, 50), (1, 100), (2, 50), (4, u64::MAX)] {
        s.sleep(Class::Ground, node(slot), 0, until);
    }
    s.sleep(Class::Air, node(5), 0, 10);
    assert!(s.due(Class::Ground, 49).is_empty());
    assert_eq!(s.due(Class::Ground, 120), [node(2), node(3), node(1)]);
    assert!(s.due(Class::Ground, u64::MAX - 1).is_empty());
    assert_eq!(s.due(Class::Air, 10), [node(5)]);
}

/// A move into a sleeping node that brings less than `wake_share` of what it holds leaves it asleep; one that brings
/// more wakes it (it is due at once). The node the move leaves sleeps on; so does a node that a link flux brings as
/// much as it takes (it wakes by its net change); a boundary node, which runs no process, gives and is never woken.
#[test]
fn a_large_move_into_a_node_wakes_it() {
    let mut w = small_world();
    let n = w.mesh.cells().n();
    let (k, i) = (0..w.chem.ns()).flat_map(|k| (0..n - 1).map(move |i| (k, i))).find(|&(k, i)| w.g(k, i) > 0.0 && w.g(k, i + 1) > 0.0).unwrap();
    let (from, to, now) = (w.table.id(Class::Ground, i), w.table.id(Class::Ground, i + 1), w.time);
    // Only these two nodes sleep.
    w.sleep = Default::default();
    for id in [from, to] {
        w.sleep.sleep(Class::Ground, id, now, now + DAY_S);
    }
    let send = |w: &World, share: f64| Move { from, to, k: k as u16, mol: share * w.cfg.wake_share * w.g(k, i + 1), heat: 0.0 };
    let small = send(&w, 0.5);
    apply(&mut w, &[], &[small], &[]);
    assert!(w.sleep.due(Class::Ground, now).is_empty());
    let large = send(&w, 2.0);
    apply(&mut w, &[], &[large], &[]);
    assert_eq!(w.sleep.due(Class::Ground, now), [to]);
    w.sleep.sleep(Class::Ground, to, now, now + DAY_S);
    let outside = w.table.id(Class::Boundary, Boundary::Outside as usize);
    let back = Move { from: outside, to, k: k as u16, mol: 2.0 * w.cfg.wake_share * w.g(k, i + 1), heat: 0.0 };
    let (cell, mol) = ([i as u32 + 1; 2], [back.mol, -back.mol]);
    let through = LinkFlux { class: [Class::Boundary, Class::Ground], a: &[Boundary::Outside as u32; 2], b: &cell, ks: &[k as u16], mol: &mol, heat: &[] };
    apply(&mut w, &[through], &[], &[]);
    assert!(w.sleep.due(Class::Ground, now).is_empty() && w.sleep.due(Class::Boundary, now).is_empty());
    apply(&mut w, &[], &[back], &[]);
    assert_eq!(w.sleep.due(Class::Ground, now), [to]);
    assert_eq!(w.sleep.due(Class::Ground, now + DAY_S), [from]);
}

/// What a law's own step may differ by between one span and many steps of the same time, relative: rounding, as the
/// test law integrates exactly.
const CATCH_UP: f64 = 1e-12;

/// A law that relaxes a node's amount towards 1 at a rate of 1e-7 /s, run on two nodes: one due every step of 3 days,
/// one asleep for a year but woken once by a move. Each time it is due it is brought up to date for the time since
/// (`Sleep::since`) in one span; at the year's end both hold the same.
#[test]
fn a_sleeping_node_catches_up_as_an_awake_node_steps() {
    let (dt, year, rate) = (3 * DAY_S, 360 * DAY_S, 1e-7);
    let relax = |a: f64, span: u64| 1.0 + (a - 1.0) * (-rate * span as f64).exp();
    let (awake, asleep) = (node(0), node(1));
    let mut s = Sleep::default();
    let mut amount = [0.0, 0.0];
    s.sleep(Class::Ground, awake, 0, dt);
    s.sleep(Class::Ground, asleep, 0, year);
    let mut t = 0;
    while t < year {
        t += dt;
        if t == 100 * DAY_S {
            s.wake(Class::Ground, asleep, t);
        }
        for id in s.due(Class::Ground, t) {
            let a = &mut amount[id.slot as usize];
            *a = relax(*a, t - s.since(id));
            s.sleep(Class::Ground, id, t, if id == awake { t + dt } else { year });
        }
    }
    assert_eq!((s.since(awake), s.since(asleep)), (year, year));
    assert!((amount[0] - amount[1]).abs() <= CATCH_UP * amount[0], "{amount:?}");
    assert!(amount[0] > 0.9, "the law ran: {amount:?}");
}
