//! The step follows the size (laws.md 0): a body's step lets it cross at most one cell, a square stepping a month and a
//! cell 2^k times finer a month over 2^k; the bodies of the finer cells step again inside the month, each update over
//! the time since its last, and the books close through those sub-steps.

use pcore::body::{Body, Layout, ALLOC, COOL_T, DISPERSE, HEAT, KEEP_T, LACK, LEARN, SPENT, STRENGTH};
use pcore::space::Leaf;
use pcore::event::EventKind;
use pcore::{brain, World};
use worldgen::testkit::{self, kg, Books, Brain, MATTER, RICH};

/// A square steps a month, a cell of half its side half a month and one of a quarter a quarter of a month. A cell
/// `SHORT` depths finer steps in minutes: its steps are whole seconds, at least one, and fill the month; the draws of
/// its bodies take a new key at each of its steps, while a square's take its day's.
#[test]
fn the_step_follows_the_size() {
    let mut w = testkit::small_world();
    let (i, month) = (testkit::dry_land(&w), w.cfg.calendar().month);
    let l = w.map.leaves()[i];
    assert_eq!(w.step_grid(i).after(0), month, "a square steps a month");
    assert_eq!(w.key_at(i, month), w.cfg.calendar().days(month), "a square's draws keyed by the day");
    testkit::split_down(&mut w, &[l], 2);
    for (depth, part) in [(l.depth + 1, 2), (l.depth + 2, 4)] {
        let cells: Vec<usize> = l.split(depth).filter_map(|c| w.map.cell(c)).collect();
        assert!(cells.iter().all(|&c| w.step_grid(c).after(0) * part == month), "a cell {part} times finer steps a month over {part}");
    }
    let fine = l.split(l.depth + SHORT).next().expect("a leaf");
    for depth in l.depth + 2..fine.depth {
        w.split(l.split(depth).next().expect("a leaf")).expect("a split");
    }
    let c = w.map.cell(fine).expect("a cell of the chain");
    let grid = w.step_grid(c);
    let ends: Vec<u64> = std::iter::successors(Some(0), |&t| Some(grid.after(t))).take_while(|&t| t <= month).collect();
    assert!(ends.len() == (1 << SHORT) + 1 && ends.last() == Some(&month), "{} steps fill the month", ends.len() - 1);
    let step = month >> SHORT;
    assert!(ends.windows(2).all(|t| (step..=step + 1).contains(&(t[1] - t[0]))), "steps of {step} or {} s", step + 1);
    let keys: std::collections::BTreeSet<u64> = ends.iter().map(|&t| w.key_at(c, t)).collect();
    assert_eq!(keys.len(), ends.len(), "a key a step");
}

/// Depths below a square of the cell of `the_step_follows_the_size` that steps in minutes.
const SHORT: u8 = 12;

/// Months a living island runs with a block split two depths in `the_books_close_through_the_sub_steps`.
const SPLIT_MONTHS: usize = 6;

/// A body in a cell split two depths updates four times a month, at the month's start and at each quarter of it, and
/// the physics of its last update reaches the next month's start; on a living island with a block of land split two
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
    let updates = w.counts.updates;
    pcore::step(&mut w);
    let h = w.life.find(id).expect("the eater lives");
    assert_eq!(w.counts.updates - updates, 4, "four updates in a month");
    assert_eq!(w.life[h].last, w.time as f64 / w.cfg.calendar().day as f64, "its physics reaches the next month's start");

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
/// The cover on the ground of the test of a mover, kg per m²: a field that does not rot within the month, as the solid
/// lying on the ground does, so the cells a mover may land in hold alike.
const MOVER_COVER: f64 = 0.3;

/// `LYING` on the ground of cell `i` of `w`.
fn lying(w: &mut World, i: usize) {
    testkit::put_kg_m2(w, RICH, i, LYING);
}

/// The small world with its inland square (`testkit::inland`) amid flat land at its height (`testkit::flat_round`), the
/// cells the split below flattens flat in both (`testkit::flat_as_split`), unsplit, or that square and the squares around
/// it split two depths (`fine`: cells of 500 m, stepping a quarter of a
/// month). Returns the world and the cell a body starts in: the square, or a fine cell of its second row and column.
fn split(fine: bool) -> (World, usize) {
    let mut w = testkit::small_world();
    let i = testkit::inland(&w);
    let square = w.map.leaves()[i];
    // Flat inside, as the quarters a flat split makes, in both: the same ground at both sizes.
    testkit::flat_round(&mut w, i);
    let near: Vec<usize> = std::iter::once(i).chain(w.mesh.cells().around(i).1.iter().map(|&j| j as usize)).collect();
    let around: Vec<Leaf> = near.iter().map(|&j| w.map.leaves()[j]).filter(|l| l.depth == square.depth).collect();
    testkit::flat_as_split(&mut w, &around, 2);
    if fine {
        testkit::split_flat(&mut w, &around, 2);
    }
    let quarter = square.side() / 4;
    let start = if fine { Leaf { x: square.x + quarter, y: square.y + quarter, depth: square.depth + 2 } } else { square };
    let start = w.map.cell(start).expect("the start");
    (w, start)
}

/// `split` after a month (so its cells have their surroundings), with what `put` lays on each cell of the square and of
/// the squares around it, so a body that moves finds it where it lands.
fn scene(fine: bool, put: fn(&mut World, usize)) -> (World, usize) {
    let (mut w, start) = split(fine);
    pcore::step(&mut w);
    let square = w.map.leaves()[start].parent().filter(|_| fine).and_then(|q| q.parent()).unwrap_or(w.map.leaves()[start]);
    let (side, depth) = (square.side(), w.map.leaves()[start].depth);
    let near = (0..9).filter_map(|k| Some(Leaf { x: (square.x + k % 3 * side).checked_sub(side)?, y: (square.y + k / 3 * side).checked_sub(side)?, depth: square.depth }));
    let cells: Vec<usize> = near.flat_map(|q| q.split(depth)).filter_map(|l| w.map.cell(l)).collect();
    cells.iter().for_each(|&c| put(&mut w, c));
    (w, start)
}

/// What a body that `make` makes in the start of `scene` does in its second month: the kg it
/// takes of what it uses (`testkit::took_used`), the share its body matter grows and the change of its fuel, kg.
fn second_month(fine: bool, make: impl Fn(&mut World, usize) -> Body) -> [f64; 3] {
    let (mut w, start) = scene(fine, lying);
    let b = make(&mut w, start);
    let h = w.add(b);
    let id = w.life[h].id;
    pcore::step(&mut w);
    let held = |w: &World| {
        let b = &w.life[w.life.find(id).expect("the body lives")];
        [kg(w, b, MATTER), kg(w, b, RICH)]
    };
    let before = held(&w);
    w.drain_events();
    pcore::step(&mut w);
    let (took, after) = (testkit::took_used(&w), held(&w));
    [took, after[0] / before[0] - 1.0, after[1] - before[1]]
}

/// What a strong eater that does not grow, made in the start of `scene` on
/// `MOVER_COVER`, its draws keyed by `seed` (`World::seed`), does in its first month, moving at every decision if
/// `moves`, else only taking: the kg it takes of what it uses (`testkit::took_used`), the metres it walks (from cell to
/// cell where it took in turn, centre to centre: a mover takes where it lands, `choose::land_move`) and its moves
/// (the members that moved into cells, from the yearly traffic of the next survey, `Counters::traffic`).
fn mover_month(fine: bool, seed: u64, moves: bool) -> [f64; 3] {
    let (mut w, start) = scene(fine, |w, c| testkit::cover(w, c, MOVER_COVER));
    (w.seed, w.cfg.phys.c_build) = (seed, 0.0);
    let brain = if moves { Brain { fixed: [testkit::OFF; brain::FIXED], dir: testkit::ON, ..Brain::only(brain::REST) } } else { Brain::only(brain::TAKE) };
    let mut b = testkit::eater_of(&mut w, start, &brain, 6.0);
    (b.genes[STRENGTH], b.genes[Layout::of(&w.chem).keep + w.chem.body().place(RICH).expect("a store")]) = (1.0, ROOM);
    held(&w, &mut b);
    let h = w.add(b);
    let id = w.life[h].id;
    w.drain_events();
    pcore::step(&mut w);
    let path = w.events.iter().filter_map(|e| matches!(e.kind, EventKind::Took { by, .. } if by == id).then_some(e.node as usize));
    let metres = path.fold((0.0, start), |(m, i), j| (m + apart(&w, i, j), j)).0;
    let took = testkit::took_used(&w);
    pcore::survey::survey(&mut w);
    [took, metres, w.counts.traffic.iter().map(|&t| t as f64).sum()]
}

/// The temperature the bodies of the test of one body law at every size and the node of
/// `a_node_lives_alike_in_a_square_and_in_its_cells` hold (their `KEEP_T` and `COOL_T`), K: their pace, and so their
/// supply and upkeep, stay as they are, whatever the length of their updates, and they have one heat state, not two.
const HELD: f32 = 300.0;
/// Kg of the solid per kg of living body its eater keeps: a month's meals fit its store, so the longer step does not
/// cut them.
const ROOM: f32 = 5.0;
/// How far what a body takes, stores and grows by in a month may stray between a square and a cell of it split two
/// depths, a share (measured: 0.7% at most): in the fine cell its month is four updates, so what it builds and stores
/// compounds within the month.
const ALIKE: f64 = 0.02;

/// The temperature genes of body `b` at `HELD`, its constants worked out again.
fn held(w: &World, b: &mut Body) {
    (b.genes[KEEP_T], b.genes[COOL_T]) = (HELD, HELD);
    b.prepare(&w.chem, &w.cfg);
}

/// One body law at every size: in a square of 2 km and in a cell of 500 m of it (on the same solid per m²), over a month,
/// an eater that does not grow takes and stores its fuel alike, and one that grows grows alike (within `ALIKE`).
#[test]
fn a_body_lives_alike_in_a_cell_split_two_depths() {
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
}

/// Movers of `a_mover_walks_and_eats_alike_in_a_cell_split_two_depths`, each with its own draws (`World::seed`).
const MOVERS: u64 = 24;
/// How far the mean metres the movers of the fine cells walk in a month may stray from those of the square, a share: a
/// link is straight or across a corner (√2 as long) by its draw, so the mean of `MOVERS` one-link months has a standard
/// error of 4% of it and that of four-link months half that; three standard errors of the gap (measured: 6%).
const WALKED_ALIKE: f64 = 0.12;
/// How far what they take in a month may stray from each other and from what a body that only takes takes, a share
/// (measured: 0.3%): the walk takes an hour of the month (a member of 30 kg walks 0.9 m/s, `Physics::speed`), and they
/// eat for the rest.
const ATE_ALIKE: f64 = 0.01;

/// A move takes its walk of the step and the body eats for the rest where it lands (laws.md 5.3), so the step of its
/// cell changes neither how far it goes nor what it eats: `MOVERS` bodies that move at every decision, in a square of
/// 2 km (one link a month) and in a cell of 500 m of it (four links of a quarter of the length), walk alike within
/// `WALKED_ALIKE`, the mean over the movers, and take alike, and as much as a body that only takes, within
/// `ATE_ALIKE`.
#[test]
fn a_mover_walks_and_eats_alike_in_a_cell_split_two_depths() {
    let [coarse, fine] = [false, true].map(|f| {
        let months: Vec<[f64; 3]> = (1..=MOVERS).map(|seed| mover_month(f, seed, true)).collect();
        let links = if f { 4.0 } else { 1.0 };
        assert!(months.iter().all(|m| (m[2] - links).abs() < 1e-3), "moves a month: {:?}, not {links}", months.iter().map(|m| m[2]).collect::<Vec<_>>());
        [0, 1].map(|q| months.iter().map(|m| m[q]).sum::<f64>() / MOVERS as f64)
    });
    let taker = mover_month(false, 1, false)[0];
    println!("a month of a mover in a square: took {:.4} kg, walked {:.0} m; in a cell of 500 m: {:.4} kg, {:.0} m; a taker took {taker:.4} kg", coarse[0], coarse[1], fine[0], fine[1]);
    assert!((fine[1] / coarse[1] - 1.0).abs() < WALKED_ALIKE, "walked {} m in the fine cells against {} m in the square", fine[1], coarse[1]);
    for (took, what) in [(fine[0], "a mover in the fine cells"), (taker, "a body that only takes in the square")] {
        assert!((took / coarse[0] - 1.0).abs() < ATE_ALIKE, "{what} took {took} kg against {} kg of a mover in the square", coarse[0]);
    }
}

/// How far what a body of sight 2 and more sees from a cell split two depths may stray from what it sees from the
/// square (the features are scaled to about one; the two worlds' climates differ by about 1e-6 after a month).
const SEEN_OFF: f32 = 1e-4;

/// The distance from the centre of cell `i` of `w` to that of cell `j`, m.
fn apart(w: &World, i: usize, j: usize) -> f64 {
    let o = w.mesh.cells().offset(i, j);
    o[0].hypot(o[1])
}

/// What a body sees reaches as far in metres from a cell of 500 m as from a square of 2 km: the features a body of
/// sight 2 to `SIGHTS` sees of a fine cell are those of the square that holds it, within `SEEN_OFF`.
#[test]
fn sight_reaches_as_far_from_a_fine_cell() {
    // (Seen just after the split, surveyed, before the liquid of the fine cells finds its own ways. No body lives, so
    // the survey makes no features: they are made here as it makes them for a body.)
    let [(coarse, i), (fine, j)] = [false, true].map(|f| {
        let (mut w, i) = split(f);
        pcore::survey::survey(&mut w);
        pcore::choose::cells(&mut w);
        (w, i)
    });
    for sight in 2..=pcore::choose::SIGHTS {
        let (a, b) = (coarse.life.feat(i, sight), fine.life.feat(j, sight));
        let off = a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max);
        assert!(off < SEEN_OFF, "sight {sight}: {b:?} from the fine cell against {a:?} from the square");
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

/// The learning gene of that node: off, so its brain stays one that only takes (what it would learn is no law of cells).
const LEARN_OFF: f32 = -4.0;

/// A node of `MEMBERS` that only takes and keeps its children in its square, its genes as `set` leaves them, settled
/// `SETTLE` months on `cover` kg per m² in the inland square of `w` (`testkit::inland`), it and its neighbours made flat
/// land at its height with its liquid and soil (`testkit::flat_round`) and the cells its split flattens flat in both
/// (`testkit::flat_as_split`), then a year there or in that square split two depths (`fine`): per month, its members at the month's end, the kg it took of what it uses, the members born and the
/// members that starved (died of a store below its minimum, `LACK`, or of upkeep unpaid, `SPENT`), its mean
/// temperature by members at the month's end, K; and whether no body left the square (a member's move has a chance near
/// none, not none).
fn node_months(fine: bool, mut w: World, cover: f64, set: fn(&mut Body)) -> (Vec<[f64; 5]>, bool) {
    let i = testkit::inland(&w);
    let square = w.map.leaves()[i];
    testkit::flat_round(&mut w, i);
    let near: Vec<usize> = std::iter::once(i).chain(w.mesh.cells().around(i).1.iter().map(|&j| j as usize)).collect();
    near.iter().for_each(|&c| testkit::cover(&mut w, c, cover));
    let line = testkit::Line::new(&mut w, &Brain::only(brain::TAKE), 1);
    let mut b = line.founder(&mut w, 1, i);
    b.genes[DISPERSE] = 0.0;
    b.genes[LEARN] = LEARN_OFF;
    set(&mut b);
    b.prepare(&w.chem, &w.cfg);
    let b = testkit::node_of(&mut w, b, MEMBERS);
    w.add(b);
    (0..SETTLE).for_each(|_| pcore::step(&mut w));
    testkit::flat_as_split(&mut w, &[square], 2);
    if fine {
        testkit::split_flat(&mut w, &[square], 2);
    }
    let months = (0..YEAR).map(|_| {
        w.drain_events();
        pcore::step(&mut w);
        // The mean temperature by members of its bodies, K, and their members.
        let (mut heat, mut members) = (0.0, 0.0);
        for b in w.life.alive() {
            (heat, members) = (heat + b.temp(&w.chem) * b.n(), members + b.n());
        }
        [members, testkit::took_used(&w), testkit::born(&w), testkit::died(&w, LACK) + testkit::died(&w, SPENT), heat / members]
    }).collect();
    let inside = |l: Leaf| l.x >= square.x && l.x < square.x + square.side() && l.y >= square.y && l.y < square.y + square.side();
    (months, w.life.alive().all(|b| inside(w.map.leaves()[b.cell as usize])))
}

/// What the node of `a_node_lives_alike_in_a_square_and_in_its_cells` (`node_months` on `COVER_KG_M2` in the small
/// world, with children, its temperature genes at `HELD`) does over the year after it settled: its members at the end,
/// the kg it took of what it uses, the members born and the members that starved, and its mean temperature by members
/// at the months' ends, K.
fn node_year(fine: bool) -> [f64; 5] {
    let (months, stayed) = node_months(fine, testkit::small_world(), COVER_KG_M2, |b| (b.genes[KEEP_T], b.genes[COOL_T]) = (HELD, HELD));
    assert!(stayed, "a body left the square");
    let sum = |q: usize| months.iter().map(|m| m[q]).sum::<f64>();
    [months[YEAR - 1][0], sum(1), sum(2), sum(3), sum(4) / YEAR as f64]
}

/// How far what the node does in a year in the square's cells may stray from what it does in the square, a share
/// (measured: 1.4% at most, of its members, what it took and those born, against a spread of 0.1% between streams of
/// draws), and its mean temperature, K (measured: 0.002).
const LIVES_ALIKE: f64 = 0.05;
const WARMTH_K: f64 = 0.05;

/// One law of life at every size of cell (spec 5.2): a node of one kind that only takes (no member leaves its square),
/// settled a year on cover in a square of 2 km amid flat land at its height (liquid that runs sideways on real slopes
/// differs with the size of the cells on its own, which `tape/tests/scale.rs` checks; here only the bodies' laws are
/// compared), then a year there and in the same world with the square split two
/// depths (16 cells of 500 m, stepping a quarter month): its members at the end, what it took and the members born
/// alike within `LIVES_ALIKE`, and its temperature within `WARMTH_K`. After each decision its physics runs over the
/// step ahead in sub-steps of a week in both, each with a meal by the take law within its stores' room for the week
/// (laws.md 0, 5.3); its heat over a sub-step is the mean of the exact solution at the pace that holds (6.3), it drinks
/// first (6.4), and its children are born through the sub-steps and live the rest of them (4.2): so a month of a
/// coarse body is four quarter months of a fine one. The scene is one the law speaks of, by two conditions the test
/// holds: no member starves through the year (a node that eats its cover out ends as the residue of a famine, which
/// chance decides), and its temperature genes hold it at `HELD`, so it has no heat turning point (where a node's
/// members sit near it, a node and the same members over finer cells may settle in different states, laws.md 6.3).
#[test]
fn a_node_lives_alike_in_a_square_and_in_its_cells() {
    let [coarse, fine] = [false, true].map(node_year);
    println!("a year in a square: {coarse:.4?}; in its cells: {fine:.4?} (members, took kg, born, starved, K)");
    assert!(coarse[3] == 0.0 && fine[3] == 0.0, "members starved: {} in the square, {} in the cells", coarse[3], fine[3]);
    for (q, what) in ["members", "took", "born"].iter().enumerate() {
        assert!((fine[q] / coarse[q] - 1.0).abs() < LIVES_ALIKE, "{what}: {} in the cells against {} in the square", fine[q], coarse[q]);
    }
    assert!((fine[4] - coarse[4]).abs() < WARMTH_K, "{} K in the cells against {} K in the square", fine[4], coarse[4]);
}

/// The cover the node of `a_node_holds_its_heat_alike_through_a_cold_year` takes, kg per m²: plenty for its year; and
/// its coat (`HEAT`), thinner than the founders' 0.9, so its winter turns it cold and its summer warm within the year.
const PLENTY_KG_M2: f64 = 3.0;
const THIN_COAT: f32 = 0.3;
/// How far its temperature at a month's end (the mean by members) may stray between the square and its cells, K
/// (measured: 0.04), and the least swing between its coldest and warmest month that shows it crossed between its two
/// states.
const HELD_K: f64 = 0.5;
const SWING_K: f64 = 15.0;

/// The heat law at every size (laws.md 6.3): on the island of the real runs, whose winter is cold, in a square amid flat
/// land at its height (as `a_node_lives_alike_in_a_square_and_in_its_cells`, so the ground differs only by the size of
/// the cells), a node of adults with plenty of cover lies cold through its winter (held at its `KEEP_T` by burning), in
/// spring warms on its own heat to a warm state (its heat over its coat past the warmth at which its pace feeds itself)
/// and in autumn turns cold again, swinging more than `SWING_K` over the year; at each month's end its members hold the
/// same temperature in a square and in its 16 cells of 500 m (four updates a month) within `HELD_K`, and turn in the
/// same month: its own time C / g is hours, so each update ends at the stable state its heat leads to from where it
/// stood (`live::standing`), in one step or in four. The scene is one the law speaks of, by a condition the test holds:
/// its members make no children (`ALLOC` none), so all are adults that the seasons carry across their turning point
/// together; a young node, which its newborns keep light, sits near its turning point for months, where a node and the
/// same members over finer cells may settle in different states (laws.md 6.3).
#[test]
fn a_node_holds_its_heat_alike_through_a_cold_year() {
    let cold = || {
        let mut w = worldgen::new_world(1, 16, 16);
        pcore::step(&mut w);
        w
    };
    let [coarse, fine] = [false, true].map(|f| {
        let months = node_months(f, cold(), PLENTY_KG_M2, |b| (b.genes[HEAT], b.genes[ALLOC]) = (THIN_COAT, 0.0)).0;
        assert!(months.iter().all(|m| m[2] == 0.0), "the node of adults made children");
        months.iter().map(|m| m[4]).collect::<Vec<f64>>()
    });
    println!("a cold year in a square: {coarse:.2?} K; in its cells: {fine:.2?} K");
    let swing = coarse.iter().cloned().fold(f64::MIN, f64::max) - coarse.iter().cloned().fold(f64::MAX, f64::min);
    assert!(swing > SWING_K, "the node stays in one state all year: {coarse:?}");
    assert!(coarse.iter().zip(&fine).all(|(c, f)| (c - f).abs() < HELD_K), "{fine:?} K in the cells against {coarse:?} K in the square");
}
