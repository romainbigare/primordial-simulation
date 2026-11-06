//! Lots (laws.md 3.1): alike parts that lie alone in a cell are one record of many parts, as alike members are one
//! node; the books stay exact as they join and split.

use pcore::body::Body;
use pcore::bond;
use pcore::brain;
use pcore::World;
use worldgen::testkit::{eater, part_of, place, start as world, Books, ROCK};

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
