//! A slot is the bookkeeping of a kind in a cell, not a thing of the world (laws.md 3): the cover's laws act on the
//! stand, so a cell whose slot is cut into alike pieces stands as the uncut cell does.

use pcore::cover::{self, NO_ROOT, WALL};
use pcore::World;
use worldgen::life::{KINDS, LIGHT_ROW};
use worldgen::testkit::{dry_lands, small_world};

/// The row of the big slow kind of cover (a tree).
const TREE_ROW: usize = 2;

/// The cover of cell `i`: its kg, plants and height by built cover, and its seeds lying, kg.
fn stand(w: &World, i: usize) -> [f64; 4] {
    let p = cover::Parts::of(&w.chem).expect("cover parts");
    let (mm, rho, c) = (p.mm(&w.chem), w.chem.subs[p.sub[WALL]].density[0], w.life.cover(i));
    let built: f64 = c.held().map(|x| c.built(&mm, x)).sum();
    let h = c.held().map(|x| c.built(&mm, x) * c.height(&w.cfg.phys, &mm, rho, x)).sum::<f64>() / built.max(1e-300);
    [c.held().map(|x| c.kg(&mm, x)).sum(), c.held().map(|x| c.plants[x]).sum(), h, c.lying.kg(&mm)]
}

/// Cut the heaviest slot of cell `i` into `k` alike pieces: the same genes and state, its amounts and plants a k-th each.
fn cut(w: &mut World, i: usize, k: usize) {
    let mm = cover::Parts::of(&w.chem).expect("cover parts").mm(&w.chem);
    w.in_cell(i, |_, d| {
        let c = d.cover(i);
        let x = c.held().max_by(|&a, &b| c.kg(&mm, a).total_cmp(&c.kg(&mm, b))).expect("a stand");
        (c.mol[x], c.plants[x]) = (c.mol[x].map(|m| m / k as f64), c.plants[x] / k as f64);
        for _ in 1..k {
            let e = (0..cover::SLOTS).find(|&y| c.root[y] == NO_ROOT).expect("room");
            (c.mol[e], c.plants[e], c.genes[e], c.root[e], c.full[e], c.net[e], c.rooted[e], c.need[e]) = (c.mol[x], c.plants[x], c.genes[x], c.root[x], c.full[x], c.net[x], c.rooted[x], c.need[x]);
        }
    });
}

/// A stand of kind `row` grown a year from `kg` per m² on the dry land cell rain reaches most, the same cell with its
/// slot cut into two and into four alike pieces, and each a year later: the cell's cover, its plants, their height and
/// the seeds lying on its ground are the uncut cell's to 1e-9 (the pieces merge back as alike slots at their first
/// month's end; what is left is the rounding of sums taken in another order, carried through a year of a nonlinear
/// law: far under 1e-9).
fn cut_stands_as_whole(row: usize, kg: f64) {
    let mut w = small_world();
    w.cfg.variety = cover::SLOTS;
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let i = dry_lands(&w).into_iter().filter(|&i| w.rain[i] > 0.0).max_by(|&a, &b| w.rain[a].total_cmp(&w.rain[b])).expect("dry land with rain");
    assert!(cover::place(&mut w, i, KINDS[row].genes, row as u32, kg));
    (0..12).for_each(|_| pcore::step(&mut w));
    let year = |mut w: World| {
        (0..12).for_each(|_| pcore::step(&mut w));
        stand(&w, i)
    };
    let whole = year(w.clone());
    for k in [2, 4] {
        let mut c = w.clone();
        cut(&mut c, i, k);
        let cut = year(c);
        println!("kind {row}, cut in {k}: kg, plants, height m, seeds lying kg {cut:?}, uncut {whole:?}");
        for (a, b) in cut.iter().zip(&whole) {
            assert!((a / b - 1.0).abs() < 1e-9, "cut in {k}: {cut:?}, uncut {whole:?}");
        }
    }
}

#[test]
fn a_herb_stand_cut_into_slots_stands_as_the_whole() {
    cut_stands_as_whole(LIGHT_ROW, 0.1);
}

#[test]
fn a_tall_stand_cut_into_slots_stands_as_the_whole() {
    cut_stands_as_whole(TREE_ROW, 5.0);
}
