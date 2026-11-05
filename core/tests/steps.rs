//! The step follows the size (laws.md 0): a body's step lets it cross at most one cell, a square stepping a month and a
//! cell 2^k times finer a month over 2^k; the bodies of the finer cells step again inside the month, each update over
//! the time since its last, and the books close through those sub-steps.

use pcore::body::{Body, Layout, COOL_T, DISPERSE, KEEP_T, STRENGTH};
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

/// The small world with its first dry square unsplit, or that square and the squares around it split two depths
/// (`fine`: cells of 500 m, stepping a quarter of a month). Returns the world and the cell a body starts in: the square,
/// or a fine cell of its second row and column.
fn split(fine: bool) -> (World, usize) {
    let mut w = testkit::small_world();
    let square = w.map.leaves()[testkit::dry_land(&w)];
    if fine {
        let (cells, i) = (w.mesh.cells(), w.map.cell(square).expect("the square"));
        let near = std::iter::once(i).chain(cells.around(i).1.iter().map(|&j| j as usize));
        let around: Vec<Leaf> = near.map(|j| w.map.leaves()[j]).filter(|l| l.depth == square.depth).collect();
        testkit::split_down(&mut w, &around, 2);
    }
    let quarter = square.side() / 4;
    let start = if fine { Leaf { x: square.x + quarter, y: square.y + quarter, depth: square.depth + 2 } } else { square };
    let start = w.map.cell(start).expect("the start");
    (w, start)
}

/// `split` after a month (so its cells have their surroundings), with `LYING` on the ground of the square's cells.
fn scene(fine: bool) -> (World, usize) {
    let (mut w, start) = split(fine);
    pcore::step(&mut w);
    let square = w.map.leaves()[start].parent().filter(|_| fine).and_then(|q| q.parent()).unwrap_or(w.map.leaves()[start]);
    let mine: Vec<usize> = square.split(w.map.leaves()[start].depth).map(|l| w.map.cell(l).expect("a cell of the square")).collect();
    mine.iter().for_each(|&c| testkit::put_kg_m2(&mut w, RICH, c, LYING));
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

/// How far what a body of sight 2 and more sees from a cell split two depths may stray from what it sees from the
/// square (the features are scaled to about one; the two worlds' climates differ by about 1e-6 after a month).
const SEEN_OFF: f32 = 1e-4;

/// The distance from the centre of cell `i` of `w` to that of cell `j`, m.
fn apart(w: &World, i: usize, j: usize) -> f64 {
    let o = w.mesh.cells().offset(i, j);
    o[0].hypot(o[1])
}

/// What a body sees and where its children land reach as far in metres from a cell of 500 m as from a square of 2 km:
/// the features a body of sight 2 to `SIGHTS` sees of a fine cell are those of the square that holds it, within
/// `SEEN_OFF`; the children who leave a node there land `spread_m` away (2 km across a side, its √2 times across a
/// corner), four fine cells or one square on.
#[test]
fn sight_and_children_reach_as_far_from_a_fine_cell() {
    // (Seen just after the split, surveyed, before the liquid of the fine cells finds its own ways.)
    let [(coarse, i), (fine, j)] = [false, true].map(|f| {
        let (mut w, i) = split(f);
        pcore::survey::survey(&mut w);
        (w, i)
    });
    for sight in 2..=pcore::choose::SIGHTS {
        let (a, b) = (coarse.life.feat(i, sight), fine.life.feat(j, sight));
        let off = a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max);
        assert!(off < SEEN_OFF, "sight {sight}: {b:?} from the fine cell against {a:?} from the square");
    }
    for (mut w, i) in [false, true].map(scene) {
        w.cfg.phys.c_copy = 1e18;
        let one = testkit::eater(&mut w, i, brain::REST, 1.0);
        let mut n = testkit::node_of(&mut w, one, 5.0);
        n.genes[DISPERSE] = 1.0;
        testkit::brood(&w, &mut n, 4.0);
        let (s, mut n) = testkit::out(&mut w, n);
        assert!(w.in_cell(i, |w, d| pcore::copy::copy(w, d, &mut n)), "the node makes children");
        let id = n.id;
        w.life[s] = n;
        pcore::life::land(&mut w);
        let kids = testkit::children(&w, id);
        let far = |h: &pcore::life::Handle| apart(&w, i, w.life[*h].cell as usize);
        let side = w.cfg.phys.spread_m;
        assert!(!kids.is_empty() && kids.iter().all(|h| [side, side * std::f64::consts::SQRT_2].iter().any(|d| (far(h) - d).abs() < 1.0)), "children land {:?} m away", kids.iter().map(far).collect::<Vec<_>>());
    }
}

/// Months a node of `a_node_lives_alike_in_a_square_and_in_its_cells` settles in its square before one of two worlds
/// splits that square, and the months the two are then compared over.
const SETTLE: usize = 12;
const YEAR: usize = 12;
/// Members of that node, and the cover placed on its square and the land around it, kg per m²: what it takes for its
/// years.
const MEMBERS: f64 = 200.0;
const COVER_KG_M2: f64 = 0.3;

/// What the node of `a_node_lives_alike_in_a_square_and_in_its_cells` does over the year after it settled, in its
/// square or in the square split two depths (`fine`): its members at the end, the kg it took of what it uses, the
/// members born and the members that died, and its mean temperature by members at the months' ends, K.
fn node_year(fine: bool) -> [f64; 5] {
    let mut w = testkit::small_world();
    let square = w.map.leaves()[testkit::dry_land(&w)];
    let i = w.map.cell(square).expect("the square");
    let near: Vec<usize> = std::iter::once(i).chain(w.mesh.cells().around(i).1.iter().map(|&j| j as usize)).filter(|&c| w.is_land(c)).collect();
    near.iter().for_each(|&c| testkit::cover(&mut w, c, COVER_KG_M2));
    let line = testkit::Line::new(&mut w, &Brain::only(brain::TAKE), 1);
    let mut b = line.founder(&mut w, 1, i);
    b.genes[DISPERSE] = 0.0;
    b.prepare(&w.chem, &w.cfg);
    let b = testkit::node_of(&mut w, b, MEMBERS);
    w.add(b);
    (0..SETTLE).for_each(|_| pcore::step(&mut w));
    if fine {
        testkit::split_down(&mut w, &[square], 2);
    }
    let (mut year, mut warmth) = ([0.0; 5], 0.0);
    for _ in 0..YEAR {
        w.drain_events();
        pcore::step(&mut w);
        (year[1], year[2]) = (year[1] + testkit::took_used(&w), year[2] + testkit::born(&w));
        year[3] += (0..CAUSES).map(|c| testkit::died(&w, c)).sum::<f64>();
        let members: f64 = w.life.alive().map(|b| b.n()).sum();
        warmth += w.life.alive().map(|b| b.temp(&w.chem) * b.n()).sum::<f64>() / members / YEAR as f64;
    }
    let inside = |l: Leaf| l.x >= square.x && l.x < square.x + square.side() && l.y >= square.y && l.y < square.y + square.side();
    assert!(w.life.alive().all(|b| inside(w.map.leaves()[b.cell as usize])), "a body left the square");
    (year[0], year[4]) = (w.life.alive().map(|b| b.n()).sum(), warmth);
    year
}

/// Death causes (`body::LACK` to `body::AGE`).
const CAUSES: u8 = 6;
/// How far what the node does in a year in the square's cells may stray from what it does in the square, a share
/// (measured: 0.9% at most, of its members, what it took and those born), and its mean temperature, K (measured:
/// 0.005).
const LIVES_ALIKE: f64 = 0.05;
const WARMTH_K: f64 = 0.05;

/// One law of life at every size of cell (spec 5.2): a node of one kind that only takes (no member leaves its square),
/// settled a year on cover in a square of 2 km, then a year there and in the same world with the square split two
/// depths (16 cells of 500 m, stepping a quarter month): its members at the end, what it took and the members born
/// alike within `LIVES_ALIKE`, and its temperature within `WARMTH_K`. Its heat over an update is the mean of the exact
/// solution at the pace that holds (laws.md 6.3), it drinks first (6.4), and what it takes feeds it to its next update
/// within its stores' room for that time (5.3), and children who start a node of their own decide at their birth, so
/// they feed from then to their cell's next step (0): so a month of a coarse body is four quarter months of a fine one.
/// The members that died, fewer than a tenth of those born, are printed, not held (measured: 4% more in the cells).
#[test]
fn a_node_lives_alike_in_a_square_and_in_its_cells() {
    let [coarse, fine] = [false, true].map(node_year);
    println!("a year in a square: {coarse:.4?}; in its cells: {fine:.4?} (members, took kg, born, died, K)");
    for (q, what) in ["members", "took", "born"].iter().enumerate() {
        assert!((fine[q] / coarse[q] - 1.0).abs() < LIVES_ALIKE, "{what}: {} in the cells against {} in the square", fine[q], coarse[q]);
    }
    assert!((fine[4] - coarse[4]).abs() < WARMTH_K, "{} K in the cells against {} K in the square", fine[4], coarse[4]);
}
