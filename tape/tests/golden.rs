//! Whole runs: small worlds with every starting kind placed by worldgen's schedule, kept whole and in their books
//! for a century, and the speed of a run.

use pcore::World;
use std::time::Instant;
use worldgen::schedule::Schedule;
use worldgen::testkit::Books;

/// A world of seed `seed`, `side` x `side` cells, and the schedule of `ew run --roster` with life from year 0; with
/// `soon`, every kind due by year 1.
fn scheduled(seed: u64, side: usize, soon: bool) -> (World, Schedule) {
    let mut plan = Schedule::new(Some(0), true, None, worldgen::life::Seeding::default());
    if soon {
        plan.kinds.iter_mut().for_each(|k| k.1 = k.1.min(1));
    }
    (worldgen::new_world(seed, side, side), plan)
}

/// `years` years of `w`, placing what `plan` says at each year's start, calling `month` after every month.
fn run(w: &mut World, plan: &Schedule, years: u64, mut month: impl FnMut(&World)) {
    for _ in 0..years {
        plan.place(w);
        for _ in 0..w.cfg.months_per_year {
            pcore::step(w);
            month(w);
        }
    }
}

/// A century of a small world keeps its elements and energy, and never holds a negative, infinite or undefined amount
/// (checked every year).
#[test]
#[ignore = "slow: a hundred years of a 16 x 16 world"]
fn a_century_keeps_its_books() {
    let (mut w, plan) = scheduled(1, 16, false);
    let books = Books::now(&w);
    let mpy = w.cfg.months_per_year as u64;
    let mut months = 0;
    run(&mut w, &plan, 100, |w| {
        months += 1;
        if months % mpy == 0 {
            assert_eq!(pcore::check::broken(w), None, "month {months}");
        }
    });
    let (de, dq) = books.drift(&w);
    println!("a hundred years: element drift {de:e}, energy drift {dq:e}; {} bodies alive", w.life.alive().count());
    books.assert_kept(&w, 1e-11);
}

/// The speed of a run: a 32 x 32 world with every kind, five years; prints years per second.
#[test]
#[ignore = "slow: a benchmark"]
fn speed() {
    let (mut w, plan) = scheduled(1, 32, true);
    let (years, start) = (5, Instant::now());
    run(&mut w, &plan, years, |_| {});
    let s = start.elapsed().as_secs_f64();
    println!("{years} years of a 32 x 32 world in {s:.2} s: {:.2} years per second, {} bodies alive", years as f64 / s, w.life.alive().count());
}
