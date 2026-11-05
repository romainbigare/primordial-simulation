//! The tree of cells while a world runs (laws.md 0): between steps a cell splits into its quarters and four quarters
//! join into their parent (`World::split`, `World::join`), cells across a border one depth apart at most, the books
//! exact and the relief a join flattens given back by its split; a cell with life refuses for now.

use pcore::check;
use pcore::event::EventKind;
use pcore::remap::Refused;
use pcore::space::square::Square;
use pcore::space::tree::{Refused as Shape, DEPTH_MOST};
use pcore::space::Leaf;
use pcore::store::Class;
use pcore::{World, AIR};
use worldgen::testkit::{self, Books};

/// The small world, its events taken: a world without life, at a month's start.
fn world() -> World {
    let mut w = testkit::small_world();
    w.drain_events();
    w
}

/// The serial of cell `i` (`life::Cell::serial`).
fn serial(w: &World, i: usize) -> u64 {
    serde_json::to_value(&w.life).expect("life serialises")["cells"][i]["serial"].as_u64().expect("a serial")
}

/// The leaf of cell `i`.
fn leaf(w: &World, i: usize) -> Leaf {
    w.map.leaves()[i]
}

/// The saved state of `w` but its life (a split or a join gives its cells new serials) and the records of its joins.
fn saved(w: &World) -> serde_json::Value {
    let mut v = serde_json::to_value(w).expect("a world serialises");
    let o = v.as_object_mut().expect("an object");
    o.remove("life");
    o.remove("joined");
    v
}

/// The first place where `a` and `b` differ, by its path, and the two there.
fn difference(a: &serde_json::Value, b: &serde_json::Value, path: &str) -> Option<String> {
    use serde_json::Value::{Array, Object};
    match (a, b) {
        (Array(x), Array(y)) if x.len() == y.len() => x.iter().zip(y).enumerate().find_map(|(k, (p, q))| difference(p, q, &format!("{path}/{k}"))),
        (Object(x), Object(y)) if x.len() == y.len() => x.iter().find_map(|(k, p)| y.get(k).map_or(Some(format!("{path}/{k} missing")), |q| difference(p, q, &format!("{path}/{k}")))),
        _ => (a != b).then(|| format!("{path}: {a} against {b}")),
    }
}

/// Nothing is broken, cells across a border differ by one depth at most, and the cells under each air node cover it
/// (`Mesh::share` sums to 1).
fn sound(w: &World) {
    assert_eq!(check::broken(w), None);
    let (cells, leaves) = (w.mesh.cells(), w.map.leaves());
    let border = (0..cells.a().len()).filter(|&l| cells.width()[l] > 0.0);
    assert!(border.clone().all(|l| leaves[cells.a()[l] as usize].depth.abs_diff(leaves[cells.b()[l] as usize].depth) <= 1), "2:1 across every border");
    let mut cover = vec![0.0; w.mesh.level(AIR).n()];
    (0..cells.n()).for_each(|i| cover[w.mesh.up(i, 0, AIR)] += w.mesh.share(i, AIR));
    assert!(cover.iter().all(|s| (s - 1.0).abs() < 1e-12), "the cells under each air node cover it: {cover:?}");
}

/// What the nodes of cell `i` hold: each amount of each class of its nodes, then their heat.
fn holds(w: &World, i: usize) -> Vec<f64> {
    let (n, ns) = (w.mesh.cells().n(), w.chem.ns());
    let classes = [Class::Ground, Class::Deep, Class::Bed, Class::Bodies];
    let amounts = classes.iter().flat_map(|&c| (0..ns).map(move |k| w.store.amounts(c)[k * n + i]));
    amounts.chain(classes.iter().filter_map(|&c| w.store.heat(c).get(i).copied())).collect()
}

/// The world's potential energy less what the stacks hold now and what the joins keep as relief: a split or a join
/// changes none of it (`Store::lifted`).
fn unflattened(w: &World) -> f64 {
    w.store.lifted() - pcore::flow::potential(w) - w.joined.iter().flatten().map(|r| r.relief).sum::<f64>()
}

/// A cell split into flat quarters (each a quarter of what it held, at its temperature) and joined again is the world
/// it was, every number of it, bit for bit; each change is an event naming the cell by its serial.
#[test]
fn a_split_then_its_join_gives_back_the_world() {
    let mut w = world();
    let (i, n) = (testkit::dry_land(&w), w.mesh.cells().n());
    let (l, before, held, first) = (leaf(&w, i), saved(&w), holds(&w, i), serial(&w, i));
    assert_eq!(w.split(l), Ok(vec![l]));
    assert_eq!(w.mesh.cells().n(), n + 3);
    sound(&w);
    for q in l.quarters() {
        let j = w.map.cell(q).expect("a quarter");
        assert!(holds(&w, j).iter().zip(&held).all(|(&x, &y)| x == y / 4.0), "quarter {q:?} holds a quarter of what its cell held");
    }
    w.join(l).expect("its quarters join");
    assert_eq!(difference(&saved(&w), &before, ""), None);
    let j = w.map.cell(l).expect("the joined cell");
    let events: Vec<_> = w.drain_events().into_iter().map(|e| (e.kind, e.node)).collect();
    assert_eq!(events, [(EventKind::Split, first as u32), (EventKind::Join, serial(&w, j) as u32)]);
    assert!(serial(&w, j) > first, "the joined cell takes a serial never given before");
}

/// Four quarters of different heights joined and split again hold what they held, each amount and heat to the rounding
/// of x · share (4 ε of it, ε = 2^-53), and the relief the join kept comes back to the stacks: the world's potential
/// energy less the stacks' and the reliefs (`unflattened`) and the books do not move.
#[test]
fn a_join_then_its_split_gives_back_the_quarters() {
    let mut w = world();
    let parent = leaf(&w, testkit::dry_land(&w)).parent().expect("a square has a parent");
    let kids = parent.quarters().map(|q| w.map.cell(q).expect("a square"));
    let (held, books, rest) = (kids.map(|i| holds(&w, i)), Books::now(&w), unflattened(&w));
    w.join(parent).expect("four squares join");
    let relief = w.joined[w.map.cell(parent).expect("the joined cell")].as_ref().expect("its record").relief;
    assert!(relief > 0.0, "a join of uneven quarters flattens their stacks: relief {relief} J");
    assert_eq!(w.split(parent), Ok(vec![parent]));
    sound(&w);
    for (q, held) in parent.quarters().iter().zip(&held) {
        let now = holds(&w, w.map.cell(*q).expect("a quarter"));
        let worst = now.iter().zip(held).map(|(&x, &y)| (x - y).abs() / y.abs().max(f64::MIN_POSITIVE)).fold(0.0, f64::max);
        assert!(worst <= 4.0 * f64::EPSILON / 2.0, "quarter {q:?} back within {worst:e}");
    }
    assert!((unflattened(&w) - rest).abs() <= 1e-12 * w.store.lifted().abs().max(relief), "the potential energy kept: {} J", unflattened(&w) - rest);
    books.assert_kept(&w, 1e-12);
}

/// Months of splits and joins, the world stepping between them: down two depths and back on land, at the coast and on a
/// cell under 0 m, the books within `check::DRIFT`, nothing below zero or NaN, 2:1 kept.
#[test]
fn splits_and_joins_between_steps_keep_the_books() {
    run_splits(&mut world(), 1);
}

/// `splits_and_joins_between_steps_keep_the_books` for years.
#[test]
#[ignore]
fn years_of_splits_and_joins_keep_the_books() {
    run_splits(&mut world(), 12);
}

/// `rounds` rounds of splits and joins of `w`, a month between each change.
fn run_splits(w: &mut World, rounds: usize) {
    let books = Books::now(w);
    let land = testkit::dry_land(w);
    let coast = (0..w.mesh.cells().n()).find(|&i| w.is_land(i) && w.mesh.cells().around(i).1.iter().any(|&j| !w.is_land(j as usize))).expect("a coast");
    let low = (0..w.mesh.cells().n()).find(|&i| !w.is_land(i)).expect("a cell under 0 m");
    let cells = [land, coast, low].map(|i| leaf(w, i));
    let month = |w: &mut World| {
        pcore::step(w);
        w.drain_events();
        sound(w);
        books.assert_kept(w, check::DRIFT);
    };
    for _ in 0..rounds {
        for &l in &cells {
            w.split(l).expect("a split");
            month(w);
            let fine = l.quarters()[3];
            let forced = w.split(fine).expect("a split two depths down");
            assert_eq!(forced.last(), Some(&fine));
            month(w);
            for &f in forced.iter().rev() {
                w.join(f).expect("the join of what a split made");
            }
            month(w);
            w.join(l).expect("the join back");
            month(w);
        }
    }
}

/// A split two depths below its neighbours first splits them (and theirs), so cells across a border differ by one depth
/// at most after each split; a join that would leave a neighbour two depths finer is refused and changes nothing, as
/// are a split of a cell of the most depth and a join of what are not four cells.
#[test]
fn the_tree_keeps_cells_one_depth_apart() {
    let mut w = world();
    let l = leaf(&w, testkit::dry_land(&w));
    w.split(l).expect("a split");
    let fine = l.quarters()[0];
    let made = w.split(fine).expect("a split two depths down");
    assert!(made.len() > 1 && made.last() == Some(&fine) && made[..made.len() - 1].iter().all(|o| o.depth == l.depth), "the squares beside it split first: {made:?}");
    sound(&w);
    let before = saved(&w);
    assert_eq!(w.join(made[0]), Err(Refused::Shape(Shape::Unbalanced)));
    assert_eq!(w.join(l.parent().expect("a parent")), Err(Refused::Shape(Shape::NoCell)));
    assert_eq!(difference(&saved(&w), &before, ""), None);
    let deepest = Square::new(4, 4, 2000.0, false, 4).by_blocks(|_, _| DEPTH_MOST);
    assert_eq!(deepest.splits(deepest.leaves()[0]), Err(Shape::Deepest));
}

/// A cell whose life holds a body, cover or a part refuses to split or to join, and nothing changes.
#[test]
fn a_cell_with_life_refuses_to_split() {
    let mut w = world();
    let lands = testkit::dry_lands(&w);
    let body = testkit::eater(&mut w, lands[0], pcore::brain::REST, 0.0);
    w.add(body);
    testkit::cover(&mut w, lands[1], 1.0);
    testkit::plate(&mut w, lands[2], 1.0, true);
    let before = serde_json::to_value(&w).expect("a world serialises");
    for &i in &lands[..3] {
        assert_eq!(w.split(leaf(&w, i)), Err(Refused::Life(i)));
        assert!(matches!(w.join(leaf(&w, i).parent().expect("a parent")), Err(Refused::Life(_))));
    }
    assert_eq!(difference(&serde_json::to_value(&w).expect("a world serialises"), &before, ""), None);
}
