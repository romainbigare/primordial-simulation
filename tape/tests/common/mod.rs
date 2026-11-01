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

/// `years` years of `ew run --seed <seed> --size <side>` (with `--roster` if `roster`) with the settings `set` makes,
/// recorded as `ew run` records it; `each_year(year, world, recorder)` after each year. Checks the books at the end.
#[allow(dead_code)]
pub fn run(seed: u64, side: usize, years: u64, roster: bool, set: impl Fn(&mut Config), each_year: impl FnMut(u64, &World, &Recorder)) -> (World, Recorder) {
    run_on(worldgen::new_world(seed, side, side), years, roster, set, each_year)
}

/// As `run`, on the new world `w`.
#[allow(dead_code)]
pub fn run_on(mut w: World, years: u64, roster: bool, set: impl Fn(&mut Config), mut each_year: impl FnMut(u64, &World, &Recorder)) -> (World, Recorder) {
    set(&mut w.cfg);
    w.cfg.validate().expect("a world `Config::validate` accepts");
    // `ew run` scales the air's heat-absorbing gases by `--absorber` (1), which sets each air node's heat again.
    worldgen::terrain::scale_absorbers(&mut w, 1.0);
    let plan = Schedule::new(Some(LIFE_YEAR), roster, Seeding::default());
    let (books, mut rec) = (Books::now(&w), Recorder::new(&w, 0, 0));
    record(&mut rec, &mut w);
    for y in 1..=years {
        plan.place(&mut w);
        for _ in 0..w.cfg.months_per_year {
            pcore::step(&mut w);
            record(&mut rec, &mut w);
        }
        each_year(y, &w, &rec);
    }
    books.assert_kept(&w, 1e-12);
    (w, rec)
}
