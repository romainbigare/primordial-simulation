//! What the tape's tests share: giving the recorder a month as the shell does, and a run as `ew run` makes it.

use pcore::{Config, World};
use tape::{Recorder, Spent};
use worldgen::life::Seeding;
use worldgen::schedule::Schedule;
use worldgen::testkit::Books;

/// The year life starts in `ew run` (`--life-year`).
pub const LIFE_YEAR: u64 = 2;

/// Record the month that has just ended: the world's events, at a year's end its counts, and no time spent.
pub fn record(rec: &mut Recorder, w: &mut World) {
    let events = w.drain_events();
    let taken = Recorder::wants_counters(w).then(|| w.take_counters());
    rec.record_month(w, events, taken, &Spent::default());
}

/// `years` years of `ew run --seed <seed> --size <side>` placing the kinds `rows` (rows of `worldgen::life::KINDS`:
/// `TWO`, or `ROSTER` for `--roster`, or the cover of either for `--plants`, `life::plants`) with the settings `set`
/// makes, recorded as `ew run` records it; `each_year(year, world, recorder)` after each year. Checks the books at the
/// end.
#[allow(dead_code)]
pub fn run(seed: u64, side: usize, years: u64, rows: &[usize], set: impl Fn(&mut Config), each_year: impl FnMut(u64, &World, &Recorder)) -> (World, Recorder) {
    run_on(worldgen::testkit::one_rise(seed, side, side), years, rows, set, each_year)
}

/// As `run`, on the new world `w`.
#[allow(dead_code)]
pub fn run_on(w: World, years: u64, rows: &[usize], set: impl Fn(&mut Config), each_year: impl FnMut(u64, &World, &Recorder)) -> (World, Recorder) {
    run_stepped(w, years, rows, set, pcore::step, each_year)
}

/// As `run_on`, each month advanced by `step` (`pcore::step`, or a control run of it).
#[allow(dead_code)]
pub fn run_stepped(mut w: World, years: u64, rows: &[usize], set: impl Fn(&mut Config), step: impl Fn(&mut World), each_year: impl FnMut(u64, &World, &Recorder)) -> (World, Recorder) {
    set(&mut w.cfg);
    w.cfg.validate().expect("a world `Config::validate` accepts");
    // `ew run` scales the air's heat-absorbing gases by `--absorber` (1), which sets each air node's heat again.
    worldgen::terrain::scale_absorbers(&mut w, 1.0);
    let mut rec = Recorder::new(&w, 0, 0);
    record(&mut rec, &mut w);
    let w = go_on(w, &mut rec, years, rows, step, each_year);
    (w, rec)
}

/// `years` more years of `w`, a world a run of `run_stepped` has brought to the end of a year, recorded by `rec`, each
/// month advanced by `step`, as the run goes on: `each_year(year, world, recorder)` after each year, counted from the
/// run's start. Checks the books from now to the end.
#[allow(dead_code)]
pub fn go_on(mut w: World, rec: &mut Recorder, years: u64, rows: &[usize], step: impl Fn(&mut World), mut each_year: impl FnMut(u64, &World, &Recorder)) -> World {
    let plan = Schedule::new(Some(LIFE_YEAR), rows, Seeding::default());
    let (books, done) = (Books::now(&w), w.cfg.calendar().years(w.time));
    for y in done + 1..=done + years {
        plan.place(&mut w);
        for _ in 0..w.cfg.months_per_year {
            step(&mut w);
            record(rec, &mut w);
        }
        each_year(y, &w, rec);
    }
    books.assert_kept(&w, 1e-9);
    w
}
