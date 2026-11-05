//! The step follows the size (laws.md 0): a body's step lets it cross at most one cell, a square stepping a month and a
//! cell 2^k times finer a month over 2^k; the bodies of the finer cells step again inside the month, each update over
//! the time since its last, and the books close through those sub-steps.

use pcore::brain;
use worldgen::testkit::{self, Books};

/// A square steps a month, a cell of half its side half a month and one of a quarter a quarter of a month, each a whole
/// number of seconds.
#[test]
fn the_step_follows_the_size() {
    let mut w = testkit::small_world();
    let (i, month) = (testkit::dry_land(&w), w.cfg.calendar().month);
    let l = w.map.leaves()[i];
    assert_eq!(w.body_step(i), month, "a square steps a month");
    testkit::split_down(&mut w, &[l], 2);
    for (depth, part) in [(l.depth + 1, 2), (l.depth + 2, 4)] {
        let cells: Vec<usize> = l.split(depth).filter_map(|c| w.map.cell(c)).collect();
        assert!(cells.iter().all(|&c| w.body_step(c) * part == month), "a cell {part} times finer steps a month over {part}");
    }
}

/// Months a living island runs with a block split two depths in `the_books_close_through_the_sub_steps`.
const SPLIT_MONTHS: usize = 6;

/// A body in a cell split two depths updates four times a month, at the month's start and at each quarter of it, its
/// last update a quarter of a month before the next month's start; on a living island with a block of land split two
/// depths (its bodies stepping inside the months, moving in and out of it) the books close within `check::DRIFT`.
#[test]
fn the_books_close_through_the_sub_steps() {
    let mut w = testkit::small_world();
    let region = testkit::land_block(&w);
    testkit::split_down(&mut w, &region, 2);
    // (A month first, so the new cells have their surroundings.)
    pcore::step(&mut w);
    let i = (0..w.mesh.cells().n()).find(|&c| w.is_land(c) && w.halvings(c) == 2).expect("a cell two depths finer");
    let b = testkit::eater(&mut w, i, brain::REST, 0.0);
    let h = w.add(b);
    let id = w.life[h].id;
    let (updates, month) = (w.counts.updates, w.cfg.calendar().month);
    pcore::step(&mut w);
    let h = w.life.find(id).expect("the eater lives");
    assert_eq!(w.counts.updates - updates, 4, "four updates in a month");
    assert_eq!(w.life[h].last, (w.time - month / 4) as f64 / w.cfg.calendar().day as f64, "the last a quarter of a month ago");

    let mut w = testkit::living(16, 2);
    let region = testkit::land_block(&w);
    testkit::split_down(&mut w, &region, 2);
    let books = Books::now(&w);
    for _ in 0..SPLIT_MONTHS {
        pcore::step(&mut w);
    }
    assert!(w.life.alive().any(|b| w.halvings(b.cell as usize) == 2), "bodies live in the split block");
    books.assert_kept(&w, pcore::check::DRIFT);
}
