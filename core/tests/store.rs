//! The store: every node has one id, `apply` keeps the books and every amount and heat at zero or above, and what
//! crosses the world's boundary is booked on its boundary nodes.

use pcore::check;
use pcore::rng::{Purpose, Rng};
use pcore::store::{apply, Boundary, Class, LinkFlux, Move, NodeId, Transform, BOUNDARIES};
use pcore::World;
use worldgen::testkit::{put, small_world, Books, LIQUID, ROCK};

/// Random changes the books test makes, in all (rounds of `PER_ROUND`).
const CHANGES: usize = 1_000_000;
const PER_ROUND: usize = 10_000;
/// The most a random change asks of what its node holds (so many ask too much and are scaled).
const GREED: f64 = 1.5;

/// The classes of the nodes of a cell.
const CLASSES: [Class; 3] = [Class::Ground, Class::Air, Class::Deep];

/// The amounts (`[k * len + index]`) and heats of the nodes of `class`, and their count.
fn arrays(w: &World, class: Class) -> (&[f64], &[f64], usize) {
    match class {
        Class::Ground => (&w.ground, &w.e_ground, w.grid.n()),
        Class::Air => (&w.air, &w.e_air, w.grid.na()),
        Class::Deep => (&w.deep, &w.e_deep, w.grid.n()),
        Class::Boundary => (&w.boundary, &w.e_boundary[..], BOUNDARIES),
    }
}

#[test]
fn every_node_has_one_id() {
    let w = small_world();
    let mut ids: Vec<NodeId> = w.table.ids().collect();
    for class in CLASSES.into_iter().chain([Class::Boundary]) {
        for i in 0..arrays(&w, class).2 {
            let id = w.table.id(class, i);
            assert_eq!((w.table.place(id), w.table.cell(id), w.table.parent(id)), ((class, i), i, None));
        }
    }
    let n = ids.len();
    ids.dedup();
    assert_eq!((ids.len(), n), (n, 2 * w.grid.n() + w.grid.na() + BOUNDARIES), "one id for each ground, air, deep and boundary node");
}

/// Two moves that each ask for all a node holds each get half of it.
#[test]
fn what_leaves_a_node_that_holds_too_little_is_scaled_down() {
    let mut w = small_world();
    let (k, i) = (0..w.chem.ns()).flat_map(|k| (0..w.grid.n()).map(move |i| (k, i))).find(|&(k, i)| w.g(k, i) > 0.0).unwrap();
    let (have, from) = (w.g(k, i), w.table.id(Class::Ground, i));
    let to = |j| Move { from, to: w.table.id(Class::Deep, j), k: k as u16, mol: have, heat: 0.0 };
    let (m0, m1) = (to(i), to(i + 1));
    let deep0 = [i, i + 1].map(|j| w.deep.get(k * w.grid.n() + j).copied().unwrap_or(0.0));
    apply(&mut w, &[], &[m0, m1], &[]);
    let n = w.grid.n();
    assert_eq!(w.g(k, i), 0.0);
    for (j, d0) in [i, i + 1].into_iter().zip(deep0) {
        assert!((w.deep[k * n + j] - d0 - have / 2.0).abs() <= 1e-12 * w.deep[k * n + j], "deep store {j}");
    }
    assert_eq!(w.counts.scaled[0], have);
}

#[test]
#[should_panic(expected = "does not keep its elements")]
fn a_transform_that_makes_elements_is_refused() {
    let mut w = small_world();
    let at = w.table.id(Class::Ground, 0);
    apply(&mut w, &[], &[], &[Transform { at, dmol: vec![(0, 1.0)] }]);
}

/// A node of a random class, and its index.
fn any_node(w: &World, r: &mut Rng) -> (Class, usize) {
    let class = CLASSES[r.below(3)];
    (class, r.below(arrays(w, class).2))
}

/// Up to `GREED` times the amount of `k` in node (`class`, `i`), mol.
fn some_of(w: &World, r: &mut Rng, (class, i): (Class, usize), k: usize) -> f64 {
    let (amount, _, len) = arrays(w, class);
    GREED * r.f64() * amount[k * len + i]
}

/// Up to `GREED` times the heat of node (`class`, `i`), J.
fn some_heat(w: &World, r: &mut Rng, (class, i): (Class, usize)) -> f64 {
    GREED * r.f64() * arrays(w, class).1[i]
}

/// One round of random changes, each asking up to `GREED` times what its node holds: a link flux of two substances and
/// heat over a third of them, then moves between nodes of any classes, then the chemistry's reactions in any node.
fn random_round(w: &mut World, r: &mut Rng) {
    let (ns, per) = (w.chem.ns(), PER_ROUND / 3);
    let class = CLASSES[r.below(3)];
    let len = arrays(w, class).2;
    let (a, b): (Vec<u32>, Vec<u32>) = (0..per).map(|_| (r.below(len) as u32, r.below(len) as u32)).unzip();
    let ks = [r.below(ns) as u16, r.below(ns) as u16];
    let (mut mol, mut heat) = (Vec::new(), Vec::new());
    // + from a to b, or back: each takes from the end it leaves.
    let sign = |r: &mut Rng, l: usize| if r.below(2) == 0 { (a[l], 1.0) } else { (b[l], -1.0) };
    for &k in &ks {
        for l in 0..per {
            let (from, s) = sign(r, l);
            mol.push(s * some_of(w, r, (class, from as usize), k as usize));
        }
    }
    for l in 0..per {
        let (from, s) = sign(r, l);
        heat.push(s * some_heat(w, r, (class, from as usize)));
    }
    let flux = LinkFlux { class, a: &a, b: &b, ks: &ks, mol: &mol, heat: &heat };
    let moves: Vec<Move> = (0..per)
        .map(|_| {
            let (from, to, k) = (any_node(w, r), any_node(w, r), r.below(ns));
            let (mol, heat) = (some_of(w, r, from, k), some_heat(w, r, from));
            Move { from: w.table.id(from.0, from.1), to: w.table.id(to.0, to.1), k: k as u16, mol, heat }
        })
        .collect();
    let transforms: Vec<Transform> = (0..PER_ROUND - 2 * per)
        .map(|_| {
            let (rx, at) = (&w.chem.reactions[r.below(w.chem.reactions.len())], any_node(w, r));
            let most = rx.inputs.iter().map(|&(k, n)| some_of(w, r, at, k as usize) / n as f64).fold(f64::MAX, f64::min);
            let side = |v: &[(u16, u16)], s: f64| v.iter().map(move |&(k, n)| (k, s * n as f64 * most)).collect::<Vec<_>>();
            Transform { at: w.table.id(at.0, at.1), dmol: [side(&rx.inputs, -1.0), side(&rx.outputs, 1.0)].concat() }
        })
        .collect();
    apply(w, &[flux], &moves, &transforms);
}

/// A million random moves, transforms and link fluxes keep every element and the energy to rounding, and leave no
/// amount and no heat below zero.
#[test]
fn random_changes_keep_the_books() {
    let mut w = small_world();
    let (t0, mut r) = (check::totals(&w), Rng::new(1, 0, 0, Purpose::Chem));
    for _ in 0..CHANGES / PER_ROUND {
        random_round(&mut w, &mut r);
    }
    let t1 = check::totals(&w);
    for (j, (e0, e1)) in t0.elems.iter().zip(&t1.elems).enumerate() {
        assert!((e1 - e0).abs() <= 1e-12 * e0.abs().max(1.0), "element {j}: {e0} mol before, {e1} after");
    }
    let (e0, e1) = (t0.thermal + t0.chemical, t1.thermal + t1.chemical);
    assert!((e1 - e0).abs() <= 1e-12 * t0.thermal.abs().max(t0.chemical.abs()), "energy {e0} J before, {e1} after");
    for class in CLASSES {
        let (amount, heat, _) = arrays(&w, class);
        assert!(amount.iter().chain(heat).all(|&x| x >= 0.0), "{class:?}: a number below zero");
    }
    assert!(w.counts.scaled.iter().all(|&s| s > 0.0), "the changes asked too much and were scaled");
}

/// A stream keyed by node id and seconds differs for every part of its key and from the stream keyed by day and index.
#[test]
fn node_streams_are_apart() {
    let first = |mut r: Rng| r.u64();
    let id = |slot, gen| NodeId { slot, gen };
    let base = first(Rng::of_node(1, 86_400, id(3, 0), Purpose::Choose));
    assert_eq!(base, first(Rng::of_node(1, 86_400, id(3, 0), Purpose::Choose)));
    let others = [
        first(Rng::of_node(2, 86_400, id(3, 0), Purpose::Choose)),
        first(Rng::of_node(1, 86_401, id(3, 0), Purpose::Choose)),
        first(Rng::of_node(1, 86_400, id(3, 1), Purpose::Choose)),
        first(Rng::of_node(1, 86_400, id(1, 3), Purpose::Choose)),
        first(Rng::of_node(1, 86_400, id(3, 0), Purpose::Bond)),
        first(Rng::new(1, 86_400, 3, Purpose::Choose)),
    ];
    assert!(others.iter().all(|&x| x != base));
}

/// Each crossing of the boundary keeps the books: matter and heat from outside (a move, and an edit), light in from the
/// sun, heat out to space. Heat that crosses unbooked does not.
#[test]
fn what_crosses_the_boundary_keeps_the_books() {
    let mut w = small_world();
    let books = Books::now(&w);
    let (from, to) = (w.table.id(Class::Boundary, Boundary::Outside as usize), w.table.id(Class::Ground, 0));
    apply(&mut w, &[], &[Move { from, to, k: LIQUID as u16, mol: 1e3, heat: 1e6 }], &[]);
    put(&mut w, ROCK, 1, 1e2);
    let q = 1e-3 * w.e_air[0];
    w.e_ground[2] += q;
    w.cross(Boundary::Sun, [], q);
    w.e_air[0] -= q;
    w.cross(Boundary::Space, [], -q);
    books.assert_kept(&w, 1e-12);
    w.e_ground[3] += q;
    assert!(books.drift(&w).1 > 1e-9, "heat that crossed unbooked was not seen");
}
