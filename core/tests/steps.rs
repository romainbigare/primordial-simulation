//! The step follows the size (laws.md 0): a body's step lets it cross at most one cell, a square stepping a month and a
//! cell 2^k times finer a month over 2^k; the bodies of the finer cells step again inside the month, each update over
//! the time since its last, and the books close through those sub-steps.

use pcore::body::{Body, Layout, COOL_T, KEEP_T, STRENGTH};
use pcore::space::Leaf;
use pcore::{brain, World};
use worldgen::testkit::{self, kg, Books, Brain, MATTER, RICH};

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
    let b = testkit::eater(&mut w, i, brain::REST, 6.0);
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

/// The energy-rich solid on the ground of the tests of one body law at every size, kg per m²: so much that a
/// month's meals leave it as it was.
const LYING: f64 = 1.0;

/// The small world after a month (so its cells have their surroundings), with its first dry square unsplit, or that
/// square and the squares around it split two depths (`fine`: cells of 500 m, stepping a quarter of a month), and
/// `LYING` on the ground of the square's cells. Returns the world and the cell a body starts in: the square, or a fine
/// cell of its second row and column.
fn scene(fine: bool) -> (World, usize) {
    let mut w = testkit::small_world();
    let square = w.map.leaves()[testkit::dry_land(&w)];
    if fine {
        let (cells, i) = (w.mesh.cells(), w.map.cell(square).expect("the square"));
        let near = std::iter::once(i).chain(cells.around(i).1.iter().map(|&j| j as usize));
        let around: Vec<Leaf> = near.map(|j| w.map.leaves()[j]).filter(|l| l.depth == square.depth).collect();
        testkit::split_down(&mut w, &around, 2);
    }
    pcore::step(&mut w);
    let depth = if fine { square.depth + 2 } else { square.depth };
    let mine: Vec<usize> = square.split(depth).map(|l| w.map.cell(l).expect("a cell of the square")).collect();
    mine.iter().for_each(|&c| testkit::put_kg_m2(&mut w, RICH, c, LYING));
    let quarter = square.side() / 4;
    let start = if fine { Leaf { x: square.x + quarter, y: square.y + quarter, depth } } else { square };
    let start = w.map.cell(start).expect("the start");
    (w, start)
}

/// What a body that `make` makes in the start of `scene` does in its second month: the kg it takes of what it uses
/// (`testkit::took_used`), the share its body matter grows, the change of its fuel, kg, and its moves (the members that
/// moved into cells, from the yearly traffic, `Counters::traffic`, whose weight of a month is 1 / 12).
fn second_month(fine: bool, make: impl Fn(&mut World, usize) -> Body) -> [f64; 4] {
    let (mut w, start) = scene(fine);
    let b = make(&mut w, start);
    let h = w.add(b);
    let id = w.life[h].id;
    pcore::step(&mut w);
    let held = |w: &World| {
        let b = &w.life[w.life.find(id).expect("the body lives")];
        [kg(w, b, MATTER), kg(w, b, RICH)]
    };
    let (before, mut traffic) = (held(&w), 0.0);
    w.drain_events();
    // (The traffic after the second month's survey, which adds the first month's moves.)
    pcore::step_marked(&mut w, &mut |w, _| {
        if traffic == 0.0 {
            traffic = w.counts.traffic.iter().map(|&t| t as f64).sum::<f64>().max(f64::MIN_POSITIVE);
        }
    });
    let (took, after) = (testkit::took_used(&w), held(&w));
    let a = 1.0 / w.cfg.months_per_year as f64;
    pcore::survey::survey(&mut w);
    let moves = w.counts.traffic.iter().map(|&t| t as f64).sum::<f64>() - (1.0 - a) * traffic;
    [took, after[0] / before[0] - 1.0, after[1] - before[1], moves]
}

/// The temperature the bodies of the test of one body law at every size hold (their `KEEP_T` and `COOL_T`), K: their
/// pace, and so their supply and upkeep, stay as they are, whatever the length of their updates.
const HELD: f32 = 300.0;
/// Kg of the solid per kg of living body its eater keeps: a month's meals fit its store, so the longer step does not
/// cut them.
const ROOM: f32 = 5.0;
/// How far what a body takes, stores and grows by in a month may stray between a square and a cell of it split two
/// depths, a share (measured: 0.7% at most): in the fine cell its month is four updates, so what it builds and stores
/// compounds within the month.
const ALIKE: f64 = 0.02;

/// One body law at every size: in a square of 2 km and in a cell of 500 m of it (on the same solid per m²), over a month,
/// an eater that does not grow takes and stores its fuel alike, one that grows grows alike (within `ALIKE`), and one
/// that moves at every decision moves one link in the square and four in the cell of a quarter of its side, as far.
#[test]
fn a_body_lives_alike_in_a_cell_split_two_depths() {
    let held = |w: &World, b: &mut Body| {
        (b.genes[KEEP_T], b.genes[COOL_T]) = (HELD, HELD);
        b.prepare(&w.chem, &w.cfg);
    };
    let grower = |w: &mut World, i: usize| {
        let mut b = testkit::eater(w, i, brain::TAKE, 2.0);
        b.genes[Layout::of(&w.chem).keep + w.chem.body().place(RICH).expect("a store")] = ROOM;
        held(w, &mut b);
        b
    };
    let feeder = |w: &mut World, i: usize| {
        w.cfg.phys.c_build = 0.0;
        grower(w, i)
    };
    let [coarse, fine] = [false, true].map(|f| second_month(f, feeder));
    let [grew, grew_fine] = [false, true].map(|f| second_month(f, grower)[1]);
    println!("a month in a square: took {:.4} kg, fuel {:+.4} kg, grew {grew:.4}; in a cell of 500 m: {:.4}, {:+.4}, {grew_fine:.4}", coarse[0], coarse[2], fine[0], fine[2]);
    for (q, what) in [(0, "took"), (2, "stored")] {
        assert!((fine[q] / coarse[q] - 1.0).abs() < ALIKE, "{what}: {} in the fine cell against {} in the square", fine[q], coarse[q]);
    }
    assert!(grew > 0.0 && (grew_fine / grew - 1.0).abs() < ALIKE, "grew by {grew_fine} in the fine cell against {grew} in the square");
    let mover = |w: &mut World, i: usize| {
        let moves = Brain { fixed: [testkit::OFF; brain::FIXED], dir: testkit::ON, ..Brain::only(brain::REST) };
        let mut b = testkit::eater_of(w, i, &moves, 6.0);
        b.genes[STRENGTH] = 1.0;
        b.prepare(&w.chem, &w.cfg);
        b
    };
    let [coarse, fine] = [false, true].map(|f| second_month(f, mover)[3]);
    assert!((coarse - 1.0).abs() < 1e-3 && (fine - 4.0).abs() < 1e-3, "moves a month: {coarse} in the square, {fine} in the fine cells");
}
