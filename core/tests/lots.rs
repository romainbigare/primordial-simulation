//! Lots (laws.md 3.1): alike parts that lie alone in a cell are one record of many parts, as alike members are one
//! node; the books stay exact as they join and split.

use pcore::body::{Body, STRENGTH};
use pcore::bond;
use pcore::brain;
use pcore::event::EventKind;
use pcore::life::Handle;
use pcore::store::{Class, Node};
use pcore::World;
use worldgen::testkit::{bite, eater, node_of, part_of, place, scatter, set_amount, small_world, start as world, Books, ROCK};

/// Body `b` (taken out of its slot) takes from the part named `p` of its cell (`bond::act`, verb 1), in a serial step.
fn act_on(w: &mut World, b: &mut Body, p: u64) {
    let i = b.cell as usize;
    w.in_cell(i, |w, d| {
        let x = bond::at(d.at(i), p).expect("the part");
        bond::act(w, d, b, 1, (x, w.step_days(i)))
    });
}

/// Alike parts that lie alone in a cell are one lot (laws.md 3.1): eight plates of one design join in their monthly
/// turn into one record of eight, which keeps the first one's id and holds their amounts and heat; a body that takes
/// from it acts on one part split out of it; a split of the cell shares the lot among its quarters by count, and their
/// join gathers it, so the cell holds again no more records than designs. The books close at every step.
#[test]
fn alike_lone_parts_are_one_lot() {
    let (mut w, i) = world();
    let ns = w.chem.ns();
    let ids: Vec<u64> = (0..8)
        .map(|_| {
            let mut wall = vec![0.0; ns];
            wall[ROCK] = 2.0 / w.chem.subs[ROCK].molar_mass;
            part_of(&mut w, i, wall, vec![0.0; ns], -1.0, 0.0)
        })
        .collect();
    let books = Books::now(&w);
    let records = |w: &World, i: usize| w.life.cell_at(i).parts.iter().map(|p| (p.id, p.count)).collect::<Vec<_>>();
    bond::step(&mut w);
    assert_eq!(records(&w, i), vec![(ids[0], 8.0)], "the plates are not one lot");
    books.assert_within(&w, 1e-9, 1e-9);
    // A body takes from one of the lot (whatever it gets).
    let b = eater(&mut w, i, brain::REST, 1.0);
    let s = place(&mut w, b);
    let mut b = std::mem::take(&mut w.life[s]);
    act_on(&mut w, &mut b, ids[0]);
    w.life[s] = b;
    let now = records(&w, i);
    assert!(now.len() == 2 && now[0] == (ids[0], 7.0) && now[1].1 == 1.0, "one part is not split out: {now:?}");
    books.assert_within(&w, 1e-9, 1e-9);
    // The cell splits: each quarter holds its share of the lot by count; joined, the cell holds them all.
    let leaf = w.map.leaves()[i];
    w.split(leaf).expect("the cell splits");
    let quarters: Vec<usize> = leaf.quarters().iter().map(|&q| w.map.cell(q).expect("a quarter")).collect();
    let lots: Vec<f64> = quarters.iter().map(|&q| w.life.cell_at(q).parts.iter().filter(|p| p.count > 1.0).map(|p| p.count).sum()).collect();
    assert!(lots.iter().all(|&n| n >= 1.0) && (lots.iter().sum::<f64>() - 7.0).abs() < 1e-12, "the lot by quarter: {lots:?}");
    books.assert_within(&w, 1e-9, 1e-9);
    w.join(leaf).expect("the quarters join");
    let j = w.map.cell(leaf).expect("the joined cell");
    bond::step(&mut w);
    let designs = bond::designs(&w.chem, w.life.cell_at(j)).into_values().collect::<std::collections::BTreeSet<u64>>().len();
    let all: f64 = w.life.cell_at(j).parts.iter().map(|p| p.count).sum();
    assert!(w.life.cell_at(j).parts.len() <= designs && (all - 8.0).abs() < 1e-12, "{:?} in {designs} designs", records(&w, j));
    books.assert_within(&w, 1e-9, 1e-9);
}

/// A node holds a lot of alike parts, one in each member's hand (laws.md 3.3): a node of 100 members holding a lot of
/// 100 flat rock plates bites a large prey as 100 individuals each holding one plate do. They take as much and wear
/// their edges as much, within 1% (the individuals bite one after the other, each a prey the others left a little
/// smaller).
#[test]
fn a_node_holding_a_lot_of_edges_bites_as_its_members() {
    const MEMBERS: usize = 100;
    // `takers` takers of `members` members each, each holding `members` plates of 5 kg as one lot, bite a node of 1e5
    // eaters: the kg taken and the volume of walls worn, m³.
    let scene = |takers: usize, members: usize| {
        let (mut w, i) = world();
        w.cfg.phys.sweep = 1e12;
        let (ns, mm) = (w.chem.ns(), w.chem.subs[ROCK].molar_mass);
        let have = w.store.amounts(Class::Ground)[ROCK * w.mesh.cells().n() + i];
        set_amount(&mut w, Node::ground(i), ROCK, have + 2.0 * MEMBERS as f64 * 5.0 / mm);
        let mut v = eater(&mut w, i, brain::REST, 1.0);
        v.genes[STRENGTH] = 0.5;
        let v = node_of(&mut w, v, 1e5);
        let v = place(&mut w, v);
        let lots: Vec<u64> = (0..takers)
            .map(|_| {
                let plates: Vec<u64> = (0..members)
                    .map(|_| {
                        let mut wall = vec![0.0; ns];
                        wall[ROCK] = 5.0 / mm;
                        part_of(&mut w, i, wall, vec![0.0; ns], -1.0, 0.0)
                    })
                    .collect();
                plates[0]
            })
            .collect();
        // (The node's plates, lying alone, join into a lot; the individuals' are held at once.)
        if members > 1 {
            bond::step(&mut w);
        }
        let held: Vec<(Handle, u64)> = lots
            .iter()
            .map(|&lot| {
                assert_eq!(w.life.part(lot).expect("the lot").count, members as f64, "the plates are not one lot");
                let mut a = eater(&mut w, i, brain::REST, 1.0);
                a.genes[STRENGTH] = 0.5;
                let mut a = if members > 1 { node_of(&mut w, a, members as f64) } else { a };
                a.hold = Some(lot);
                let a = place(&mut w, a);
                w.life.part_mut(lot).expect("the lot").held = w.life[a].id;
                (a, lot)
            })
            .collect();
        let walls = |w: &World| held.iter().map(|&(_, p)| w.life.part(p).expect("the lot").walls(&w.chem).volume).sum::<f64>();
        let (books, before) = (Books::now(&w), walls(&w));
        w.events.clear();
        for &(a, _) in &held {
            bite(&mut w, a, v);
        }
        pcore::life::land(&mut w);
        books.assert_kept(&w, 1e-9);
        let got: f64 = w.events.iter().map(|e| if let EventKind::Took { .. } = e.kind { e.value } else { 0.0 }).sum();
        (got, before - walls(&w))
    };
    let (node, worn_node) = scene(1, MEMBERS);
    let (ones, worn_ones) = scene(MEMBERS, 1);
    println!("node: took {node:.4} kg, wore {worn_node:e} m³; individuals: took {ones:.4} kg, wore {worn_ones:e} m³");
    assert!(node > 0.0 && worn_node > 0.0, "the node took {node} kg and wore {worn_node} m³");
    assert!((node / ones - 1.0).abs() < 0.01 && (worn_node / worn_ones - 1.0).abs() < 0.01, "node {node} kg {worn_node} m³, individuals {ones} kg {worn_ones} m³");
}

/// The cost of parts follows the cells, not the parts (laws.md 3.1): a cell keeps at most `MAX_LOTS` lots of lone
/// parts, joining the nearest designs. A world with 1e6 parts of random designs placed by an edit steps within 10% of the
/// time of one with 1e3, each timed as the least of three runs of 60 months after a first month that joins them, and
/// keeps every part (counted one by one) and the books.
#[test]
#[ignore = "slow: a world of a million parts (400 MB), and a timing"]
fn a_million_parts_step_as_fast_as_a_thousand() {
    // The world with `n` parts after its first month, and the seconds of its next 60 months, least of three.
    let timed = |n: usize| {
        let mut w = small_world();
        scatter(&mut w, n, 1);
        let books = Books::now(&w);
        pcore::step(&mut w);
        let cells = w.mesh.cells().n();
        assert!((0..cells).all(|i| w.life.cell_at(i).parts.len() <= bond::MAX_LOTS), "a cell holds more lots than the cap");
        let parts: f64 = w.life.parts().map(|p| p.count).sum();
        assert!((parts - n as f64).abs() < 1e-6 * n as f64, "{parts} parts of {n}");
        let best = (0..3)
            .map(|_| {
                let mut v = w.clone();
                let start = std::time::Instant::now();
                (0..60).for_each(|_| pcore::step(&mut v));
                start.elapsed().as_secs_f64()
            })
            .fold(f64::INFINITY, f64::min);
        books.assert_within(&w, 1e-9, 1e-9);
        (best, w.life.parts().count())
    };
    let (many, records) = timed(1_000_000);
    let (few, few_records) = timed(1_000);
    println!("1e6 parts: {many:.3} s in {records} records; 1e3 parts: {few:.3} s in {few_records} records");
    assert!(many < 1.1 * few, "1e6 parts step in {many} s, 1e3 in {few} s");
}
