//! Law 5 (Choose): one body learns to move to what it can take; each body learns into its own brain; giving; moving; a
//! body with no brain.
//!
//! The learning scene: flat ground, the energy-rich solid lying loose only in a strip on the east side, one body (of
//! the starting kind that burns that solid and takes it from the ground) just west of the strip, with a neutral
//! brain. Over its first 200 decisions, with learning on against off, over many seeds. Its children are taken out at
//! once.

use pcore::body::{self, Layout, INTERVAL, LEARN};
use pcore::rng::{Purpose, Rng};
use pcore::World;
use pcore::store::{Class, Node};
use worldgen::testkit::{self, dry_land, founder, scale, set_amount, set_heat, start, Books, Brain, Line, LIQUID, MATTER, MILD_LAT, OFF, RICH};

/// The scene's grid.
const W: usize = 12;
const H: usize = 8;
/// First column of the strip, and the loose energy-rich solid on each of its cells, mol/m².
const STRIP: usize = 6;
const STRIP_MOL: f64 = 20.0;
/// The body's mass, kg.
const BODY_KG: f64 = 30.0;
const DECISIONS: usize = 200;
/// Learning rate genes (log10): on, and off.
const LEARN_ON: f32 = -1.0;
const LEARN_OFF: f32 = -4.0;

/// The scene's world for `seed`: flat and mild (`MILD_LAT`), every cell a copy of one dry land cell, the strip added on top.
fn scene(seed: u64) -> World {
    let (chem, init) = worldgen::chemistry0();
    let cfg = pcore::Config { lat_south: MILD_LAT[0], lat_north: MILD_LAT[1], ..Default::default() };
    let mut w = worldgen::terrain::build(seed, worldgen::terrain::island(W, H), chem, &init, cfg);
    pcore::step(&mut w);
    let (n, ns) = (w.mesh.cells().n(), w.chem.ns());
    let land = dry_land(&w);
    for k in 0..ns {
        let (g, b) = (w.g(k, land), w.b(k, land));
        for i in 0..n {
            set_amount(&mut w, Node::ground(i), k, g);
            set_amount(&mut w, Node::new(Class::Bed, i), k, b);
        }
    }
    let e = w.store.heat(Class::Ground)[land];
    (0..n).for_each(|i| set_heat(&mut w, Node::ground(i), e));
    for i in (0..n).filter(|&i| i % W >= STRIP) {
        let rich = w.g(RICH, i) + STRIP_MOL * w.mesh.cells().area()[0];
        set_amount(&mut w, Node::ground(i), RICH, rich);
    }
    w.life.env.clear();
    pcore::survey::survey(&mut w);
    w
}

/// What a body perceives of the energy-rich solid in cell `i`.
fn seen(w: &World, i: usize) -> f32 {
    w.life.seen[i * w.chem.ns() + RICH]
}

/// Per decision: on the strip; moved; a neighbour was richer; moved to a richer one; the share of richer neighbours
/// (what a move in a random direction would give).
type Decision = (bool, bool, bool, bool, f64);

/// The scene for `seed` with the learning gene `learn`: the body's decisions until its 200th or its death.
fn run(seed: u64, learn: f32) -> Vec<Decision> {
    let mut w = scene(seed);
    // A neutral brain: small random weights, no bias among resting, putting, taking from the ground and moving (which
    // all start equally likely); the actions on parts and on other bodies kept rare, as they are not part of this
    // scene. The body scaled to `BODY_KG` (booked as an edit).
    let brain = Brain { fixed: [0.0, 0.0, 0.0, OFF, OFF, OFF, OFF], verbs: [OFF; pcore::brain::VERBS + pcore::brain::OVERBS], seed, ..Brain::neutral() };
    let start = w.map.idx(STRIP - 1, H / 2);
    let mut b = founder(&mut w, 1, start, &brain);
    let f = BODY_KG / b.mass(&w.chem);
    scale(&mut w, &mut b, f);
    // Twice the fuel of a starting eater besides, so that what it learns has the decisions to show before it starves.
    let fuel = 2.0 * testkit::kg(&w, &b, RICH);
    testkit::add(&mut w, &mut b, RICH, fuel);
    // Decisions every 5 days (the shortest interval).
    (b.genes[LEARN], b.genes[INTERVAL]) = (learn, 0.7);
    let slot = w.add(b, 1);
    let mut log = Vec::new();
    // Day by day, to see each decision.
    while log.len() < DECISIONS && w.life.bodies[slot].alive {
        let (u, from) = (w.counts.updates, w.life.bodies[slot].cell as usize);
        let near: Vec<usize> = w.mesh.cells().around(from).1.iter().map(|&j| j as usize).collect();
        let richer: Vec<usize> = near.iter().copied().filter(|&j| seen(&w, j) > seen(&w, from) + 0.02).collect();
        pcore::day(&mut w);
        // A body that has children: they die at once, their matter back to the ground where they were born, so the scene
        // stays one body.
        for s in 0..w.life.bodies.len() {
            if s != slot && w.life.bodies[s].alive {
                let c = std::mem::take(&mut w.life.bodies[s]);
                body::die(&mut w, s, c, body::LACK);
            }
        }
        if w.counts.updates > u && w.life.bodies[slot].alive {
            let to = w.life.bodies[slot].cell as usize;
            log.push((from % W >= STRIP, to != from, !richer.is_empty(), richer.contains(&to), richer.len() as f64 / near.len() as f64));
        }
    }
    log
}

/// What the scene's body did over many seeds.
struct Tally {
    /// The share of all decisions made on the strip
    strip: f64,
    /// Of the moves made from a cell next to a richer one: the share to a richer cell, and what random moves would give
    toward: f64,
    chance: f64,
    decisions: usize,
    moves: usize,
    /// The decisions each seed's body lived to make
    lives: Vec<f64>,
}

/// The scene over seeds 1 to `seeds`.
fn measure(learn: f32, seeds: u64) -> Tally {
    let (mut on, mut n, mut moves, mut toward, mut chance, mut lives) = (0, 0, 0usize, 0, 0.0, Vec::new());
    // (The seeds run on the machine's threads, each thread every so many of them; the tally adds them in seed order.)
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get()) as u64;
    let mut logs: Vec<(u64, Vec<Decision>)> = std::thread::scope(|s| {
        let parts: Vec<_> = (0..threads).map(|t| s.spawn(move || (1 + t..=seeds).step_by(threads as usize).map(|seed| (seed, run(seed, learn))).collect::<Vec<_>>())).collect();
        parts.into_iter().flat_map(|p| p.join().expect("a thread of seeds")).collect()
    });
    logs.sort_by_key(|l| l.0);
    for (_, log) in logs {
        lives.push(log.len() as f64);
        for (strip, moved, next, up, share) in log {
            (on, n) = (on + strip as usize, n + 1);
            if moved && next {
                (moves, toward, chance) = (moves + 1, toward + up as usize, chance + share);
            }
        }
    }
    Tally { strip: on as f64 / n as f64, toward: toward as f64 / moves as f64, chance: chance / moves as f64, decisions: n, moves, lives }
}

/// The mean of `x` and its standard error.
fn mean_and_error(x: &[f64]) -> (f64, f64) {
    let n = x.len() as f64;
    let mean = x.iter().sum::<f64>() / n;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    (mean, (var / n).sqrt())
}

fn report(on: &Tally, off: &Tally) -> String {
    format!(
        "learning on: on the strip {:.3}, toward {:.3} (chance {:.3}), {} decisions, {} moves; off: on the strip {:.3}, toward {:.3} (chance {:.3}), {} decisions, {} moves",
        on.strip, on.toward, on.chance, on.decisions, on.moves, off.strip, off.toward, off.chance, off.decisions, off.moves
    )
}

/// Over 200 seeds the body that learns lives to make more decisions than the one that does not, by more than three
/// standard errors of the difference of their means: it learns to take what it finds and to stay where it is. Of all
/// its decisions a clearly larger share is made on the strip. (Whether its moves go toward the richer cells is a
/// smaller effect, which needs the seeds of the slow version.)
#[test]
fn a_body_that_learns_lives_on_what_it_can_take() {
    let (on, off) = (measure(LEARN_ON, 200), measure(LEARN_OFF, 200));
    let ((life_on, e_on), (life_off, e_off)) = (mean_and_error(&on.lives), mean_and_error(&off.lives));
    let se = (e_on * e_on + e_off * e_off).sqrt();
    let report = format!("{}; decisions per body {life_on:.1} and {life_off:.1}, standard error of the difference {se:.1}", report(&on, &off));
    eprintln!("{report}");
    assert!(life_on - life_off > 3.0 * se, "{report}");
    assert!(on.strip > off.strip + 0.07, "{report}");
}

/// Over 1,600 seeds the body that learns ends up on the strip clearly more often than one that does not, and its moves
/// from a cell next to a richer one go to a richer cell more often than chance, and more than those of a body that
/// does not learn by one standard error of the difference of the two shares. Moving pays only through a take at a
/// later decision; it is learned because each row of the brain steps by its own size, so the row for the richness of
/// a neighbour learns when that matters, not drowned by the others.
#[test]
#[ignore = "slow: 3,200 runs of the learning scene"]
fn body_learns_to_move_toward_what_it_can_take() {
    let (on, off) = (measure(LEARN_ON, 1600), measure(LEARN_OFF, 1600));
    let report = report(&on, &off);
    eprintln!("{report}");
    assert!(on.strip > off.strip + 0.07, "{report}");
    let (up_on, up_off) = (on.toward - on.chance, off.toward - off.chance);
    let se = (on.toward * (1.0 - on.toward) / on.moves as f64 + off.toward * (1.0 - off.toward) / off.moves as f64).sqrt();
    assert!(up_on > 0.01 && up_on > up_off + se, "{report}, standard error {se:.4}");
}

/// A world of seed 1, 16 x 16, with the light-eaters and the eaters (neutral brains) placed after a month and run for
/// 12 months more (neutral brains starve soon); and its books at the start.
fn lived() -> (pcore::World, Books) {
    let mut w = worldgen::new_world(1, 16, 16);
    let books = Books::now(&w);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed_with(&mut w, 1, &worldgen::life::Seeding::neutral());
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    (w, books)
}

/// A brain grown (more hidden units and memory) and shrunk back is the brain it was.
#[test]
fn brain_grown_and_shrunk_back_is_unchanged() {
    let (a, b) = (pcore::lineage::shape(9, 12, 6), pcore::lineage::shape(9, 16, 8));
    let mut r = Rng::new(1, 0, 0, Purpose::Resize);
    let w = pcore::brain::random(a, &mut r, 0.1, [0.0; pcore::brain::FIXED], 0.0, [0.0; 9]);
    let (base, rows) = w.split_at(a.base());
    let grown = (pcore::brain::rebase(base, a, b, &mut r, 0.1), pcore::brain::rerow(rows, a, b, &mut r, 0.1));
    assert_eq!((grown.0.len(), grown.1.len()), (b.base(), b.rows() * b.r()));
    assert_eq!(pcore::brain::rebase(&grown.0, b, a, &mut r, 0.1), base);
    assert_eq!(pcore::brain::rerow(&grown.1, b, a, &mut r, 0.1), rows);
    // Down to no brain: no base and no born rows; back up from none: rows of new weights.
    let z = pcore::lineage::shape(9, 0, 6);
    assert!(pcore::brain::rebase(base, a, z, &mut r, 0.1).is_empty() && pcore::brain::rerow(rows, a, z, &mut r, 0.1).is_empty());
    assert_eq!(pcore::brain::rerow(&[], z, a, &mut r, 0.1).len(), a.rows() * a.r());
}

/// Each body with a brain learns into rows of its own: its rows are those it was born with plus what it learned, so
/// bodies that lived differ from their born rows, and they learned different things (a body drawn from a node,
/// `nodes::open`, carries the node's learned rows, so two may share them). Matter and energy still add up.
#[test]
fn each_body_learns_into_its_own_rows() {
    let (w, books) = lived();
    let minds: Vec<&pcore::body::Body> = w.life.alive().filter(|b| !b.rows.is_empty()).collect();
    assert!(minds.iter().all(|b| b.rows.len() == w.life.born(b).len()), "rows of another shape than the born rows");
    let learned: Vec<Vec<f32>> = minds.iter().map(|b| b.rows.iter().zip(w.life.born(b)).map(|(r, x)| r - x).collect()).collect();
    let moved = learned.iter().filter(|l| l.iter().any(|&x| x != 0.0)).count();
    assert!(moved >= 2, "{moved} of {} bodies learned", minds.len());
    let kinds = (0..learned.len()).filter(|&a| learned[a].iter().any(|&x| x != 0.0) && !(0..a).any(|b| learned[b] == learned[a])).count();
    assert!(kinds >= 2, "{kinds} different things learned by {moved} bodies");
    books.assert_kept(&w, 1e-9);
}

/// A body whose safe minimums are all off can give all its stores to a body with room (here handling costs nothing),
/// but not its body: its matter (body and brood) and the liquid its body keeps stay. No body is left empty and alive,
/// and nothing breaks.
#[test]
fn a_body_gives_its_stores_but_not_its_body() {
    use pcore::brain;
    use pcore::event::EventKind;
    let (mut w, i) = start();
    w.cfg.phys.c_handle = 0.0;
    let lay = Layout::of(&w.chem);
    // Brains that only give to a body they see, and that only rest.
    let mut giver = founder(&mut w, 1, i, &Brain::verb(brain::GIVE));
    // No safe minimums, and a long interval (316 days), so that one give may move all it holds.
    giver.genes[lay.min..lay.upt].fill(-0.1);
    giver.genes[INTERVAL] = 2.5;
    let mut taker = founder(&mut w, 1, i, &Brain::only(brain::REST));
    taker.genes[lay.keep..].fill(0.95);
    // (Body matter given is digested into the solid, so it fills the taker's room for the solid (`Body::digest_room`):
    // room for all the giver's solid and matter, 2 kg per kg of body.)
    taker.genes[lay.keep + RICH] = 2.0;
    let (m0, fuel0) = (giver.store[MATTER], giver.store[RICH]);
    let g = w.add(giver, 1);
    w.add(taker, 1);
    for _ in 0..3 {
        pcore::day(&mut w);
        for b in w.life.alive() {
            assert!(b.mass(&w.chem) >= 1e-3 * b.n(), "an empty body is alive: energy {}, stores {:?}", b.e, b.store);
        }
        assert_eq!(pcore::check::broken(&w), None);
    }
    assert!(w.events.iter().any(|e| matches!(e.kind, EventKind::Gave { .. })), "nothing was given");
    let b = &w.life.bodies[g];
    assert!(b.alive && b.store[MATTER] > 0.99 * m0 && b.store[RICH] < 0.5 * fuel0, "matter {m0} -> {}, fuel {fuel0} -> {}", b.store[MATTER], b.store[RICH]);
    assert!(b.store[LIQUID] >= 0.99 * b.level(&w.chem, LIQUID, b.mass(&w.chem)), "the liquid of its body was given");
}

/// A brain of no hidden units is no brain: a body of such a lineage takes no decision and pays nothing for a brain,
/// but lives its physics at its updates and has offspring from its brood. Matter and energy are kept.
#[test]
fn a_body_with_no_brain_lives_and_copies() {
    use pcore::body::{COPY_MASS, HIDDEN};
    let (mut w, i) = start();
    // The light-eaters' sizes but no hidden units.
    let line = Line::new(&mut w, &Brain { hidden: 0, ..Brain::neutral() }, 0);
    assert!(w.life.brains[line.lineage as usize].as_ref().expect("its lineage").base.is_empty());
    // A light-eater node whose members are mature: a body (0.7 kg of the 1 kg) of more than their copy mass (0.25 kg).
    let mut b = line.founder(&mut w, 0, i);
    (b.genes[HIDDEN], b.genes[COPY_MASS]) = (0.0, -0.6);
    // Twice the fuel of a starting light-eater: with what one holds, half its members cannot pay their upkeep in the
    // second month, and what is left with its children is fewer than it started with.
    let fuel = testkit::kg(&w, &b, RICH);
    testkit::add(&mut w, &mut b, RICH, fuel);
    let (n0, s) = (b.count, w.add(b, 1));
    // It pays nothing for a brain, however costly brains are.
    let mut x = w.life.bodies[s].clone();
    x.prepare(&w.chem, &w.cfg.phys);
    let (d, m, p0) = (x.interval(&w) as f64, x.mass(&w.chem), w.cfg.phys);
    let u0 = pcore::body::upkeep(&w, &x, d, m, x.temp(&w.chem));
    (w.cfg.phys.c_hidden, w.cfg.phys.c_memory, w.cfg.phys.c_seen, w.cfg.phys.c_sight, w.cfg.phys.c_learn) = (1e9, 1e9, 1e9, 1e9, 1e9);
    assert_eq!(pcore::body::upkeep(&w, &x, d, m, x.temp(&w.chem)), u0);
    w.cfg.phys = p0;
    let (books, u) = (Books::now(&w), w.counts.updates);
    for _ in 0..2 {
        pcore::step(&mut w);
        assert_eq!(pcore::check::broken(&w), None);
    }
    let b = &w.life.bodies[s];
    let born = testkit::born(&w);
    println!("updates {}, members {n0:.0} -> {:.0}, born {born:.0}", w.counts.updates - u, b.count);
    // (Its children are small against its members: young, so they are nodes of their own.)
    let all: f64 = w.life.alive().map(|x| x.n()).sum();
    assert!(b.alive && w.counts.updates - u >= 2 && born > 0.0 && all > n0, "no children: {born}");
    assert!(w.counts.decisions == 0.0 && b.trace.is_empty() && b.wb == 0.0, "the brain was called");
    books.assert_kept(&w, 1e-9);
}

/// Moving costs the real cost of transport: per kg and m a 30 kg member pays `c_move`, a smaller one more and a bigger
/// one less, as m^-0.32 (Taylor et al. 1982), so a metre costs a member of 100 times the mass 100^0.68 = 23 times as
/// much, not 100 times.
#[test]
fn moving_costs_the_real_cost_of_transport() {
    let p = pcore::Physics::default();
    assert_eq!(p.walk(30.0), p.c_move, "a 30 kg member pays c_move");
    let (small, big) = (p.walk(0.3) * 0.3, p.walk(30.0) * 30.0);
    assert!((big / small - 100f64.powf(0.68)).abs() < 1e-9, "100 times the mass, {} times the cost of a metre", big / small);
    assert!((p.walk(0.18) / p.walk(44.0) - (0.18f64 / 44.0).powf(-0.32)).abs() < 1e-9);
    // The real figure, 10.7 m^-0.32 J per kg per m, at 30 kg is 3.6: c_move 3 is the same order.
    assert!(p.c_move > 0.7 * 10.7 * 30f64.powf(-0.32) && p.c_move < 1.0 * 10.7 * 30f64.powf(-0.32));
}

