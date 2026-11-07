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
/// parts, joining the nearest designs, and the monthly pass over parts (`bond::step`: the join, the turns and the loads
/// of a cell) goes over each record of its cells a fixed number of times, so its work is in proportion to the records.
/// A world of 1e6 parts of random designs placed by an edit, whose cells hold parts of more designs than the cap, holds
/// after the month that joins them (which goes over the 1e6 records the edit made, once) and in each of the 60 months
/// after at most `MAX_LOTS` records a cell, as the cap allows: no part is held, bonded or lived in, the only records the
/// cap does not join. So its records stay within `MAX_LOTS` times the cells holding parts, as a world of 1e3 parts's do,
/// whatever the parts. The month that joins them keeps every part (counted one by one); the books are kept throughout,
/// as thin plates are buried in the months after. The time of 60 months is printed for
/// both, not held (a shared machine moves it).
#[test]
#[ignore = "slow: a world of a million parts (400 MB)"]
fn a_million_parts_cost_the_records_of_the_cells() {
    // Of the world with `n` parts: the most designs one cell holds as placed, the most records a cell held and the
    // records summed over the 60 months after the first, the most cells that held parts, and the seconds of those months.
    let run = |n: usize| {
        let mut w = small_world();
        scatter(&mut w, n, 1);
        let cells = w.mesh.cells().n();
        let designs = (0..cells).map(|i| bond::designs(&w.chem, w.life.cell_at(i)).into_values().collect::<std::collections::BTreeSet<u64>>().len()).max().unwrap_or(0);
        let books = Books::now(&w);
        let (mut most, mut visited, mut holding) = (0, 0, 0);
        pcore::step(&mut w);
        let parts: f64 = w.life.parts().map(|p| p.count).sum();
        assert!((parts - n as f64).abs() < 1e-6 * n as f64, "{parts} parts of {n} after the first month");
        let start = std::time::Instant::now();
        for _ in 0..60 {
            most = most.max((0..cells).map(|i| w.life.cell_at(i).parts.len()).max().unwrap_or(0));
            visited += w.life.parts().count();
            holding = holding.max((0..cells).filter(|&i| !w.life.cell_at(i).parts.is_empty()).count());
            pcore::step(&mut w);
        }
        let secs = start.elapsed().as_secs_f64();
        books.assert_within(&w, 1e-9, 1e-9);
        (designs, most, visited, holding, secs)
    };
    let (designs, most, visited, holding, secs) = run(1_000_000);
    let (few_designs, few_most, few_visited, few_holding, few_secs) = run(1_000);
    println!("1e6 parts: {designs} designs in a cell at most, {most} records, {visited} visited over 60 months in {holding} cells, {secs:.3} s");
    println!("1e3 parts: {few_designs} designs in a cell at most, {few_most} records, {few_visited} visited over 60 months in {few_holding} cells, {few_secs:.3} s");
    assert!(designs > bond::MAX_LOTS, "the cap does not bind: {designs} designs in a cell at most");
    assert!(most <= bond::MAX_LOTS && few_most <= bond::MAX_LOTS, "a cell held {most} and {few_most} records, the cap {}", bond::MAX_LOTS);
    let bound = 60 * bond::MAX_LOTS * holding;
    assert!(visited <= bound && few_visited <= bound, "{visited} and {few_visited} records visited, the cap's bound {bound}");
}
