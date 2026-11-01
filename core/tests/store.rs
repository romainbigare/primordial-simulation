//! The store: `apply` keeps the books of changes that ask no node for more than it holds (what
//! rounding leaves below zero is set to zero), what crosses the world's boundary is booked on its boundary nodes, and
//! a sample member's update leaves the bodies' view of the store as it found it.

use pcore::check;
use pcore::rng::{Purpose, Rng};
use pcore::store::{apply, held, fall_heat, Boundary, Class, LinkFlux, Move, Node, NodeId, BOUNDARIES};
use pcore::{World, AIR};
use worldgen::testkit::{put, set_amount, small_world, Books, LIQUID, ROCK};

/// Random changes the books test makes, in all (rounds of `PER_ROUND`).
const CHANGES: usize = 1_000_000;
const PER_ROUND: usize = 10_000;

/// The classes of the nodes of a cell, and those of them that hold each substance apart from heat alike (`store::held`).
const CLASSES: [Class; 3] = [Class::Ground, Class::Air, Class::Deep];
const ALIKE: [Class; 2] = [Class::Ground, Class::Deep];

/// The amounts (`[k * len + index]`) and heats of the nodes of `class`, and their count.
fn arrays(w: &World, class: Class) -> (&[f64], &[f64], usize) {
    let len = match class {
        Class::Air => w.mesh.level(AIR).n(),
        Class::Boundary => BOUNDARIES,
        Class::Part | Class::Body => unreachable!("a part's or a body's amounts lie in it"),
        _ => w.mesh.cells().n(),
    };
    (w.store.amounts(class), w.store.heat(class), len)
}

/// What passes through a node in one call needs nothing held: liquid along a chain of links through nodes that hold
/// little or nothing (in by one link, out by the next) leaves them holding what they held.
#[test]
fn what_passes_through_a_node_needs_nothing_held() {
    const CHAIN: usize = 6;
    let mut w = small_world();
    let (n, have) = (w.mesh.cells().n(), 1e3);
    let held: Vec<f64> = (0..CHAIN).map(|i| if i == 0 { have } else { 1e-3 * have * (i % 2) as f64 }).collect();
    held.iter().enumerate().for_each(|(i, &m)| set_amount(&mut w, Node::ground(i), LIQUID, m));
    let (a, b): (Vec<u32>, Vec<u32>) = (0..CHAIN as u32 - 1).map(|i| (i, i + 1)).unzip();
    let mol = vec![have; CHAIN - 1];
    let (books, rounded) = (Books::now(&w), w.counts.rounded);
    apply(&mut w, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &[LIQUID as u16], mol: &mol, heat: &[] }], &[]);
    books.assert_kept(&w, 1e-9);
    let after = &w.store.amounts(Class::Ground)[LIQUID * n..LIQUID * n + CHAIN];
    assert_eq!((after[0], w.counts.rounded), (0.0, rounded), "nothing is rounded");
    for i in 1..CHAIN - 1 {
        assert!((after[i] - held[i]).abs() <= 1e-12 * have, "node {i} holds {} of {}", after[i], held[i]);
    }
    assert!((after[CHAIN - 1] - held[CHAIN - 1] - have).abs() <= 1e-12 * have);
}

/// A link may join the nodes of two classes: liquid and its heat cross the vertical link from a ground node into its
/// deep store, and the books are kept.
#[test]
fn a_link_joins_a_ground_node_to_its_store() {
    let mut w = small_world();
    let (n, i) = (w.mesh.cells().n(), 3);
    let (have, deep, heat) = (w.g(LIQUID, i), w.store.amounts(Class::Deep)[LIQUID * n + i], 1e-3 * w.store.heat(Class::Ground)[i]);
    let books = Books::now(&w);
    let cell = [i as u32];
    apply(&mut w, &[LinkFlux { class: [Class::Ground, Class::Deep], a: &cell, b: &cell, ks: &[LIQUID as u16], mol: &[have / 2.0], heat: &[heat] }], &[]);
    books.assert_kept(&w, 1e-9);
    assert!((w.g(LIQUID, i) - have / 2.0).abs() <= 1e-12 * have && (w.store.amounts(Class::Deep)[LIQUID * n + i] - deep - have / 2.0).abs() <= 1e-12 * have);
}

/// A link may join a ground node to its air node: the gas that rises takes from the heat the air receives what the air
/// holds of it apart from heat (its latent heat), and the air pays its lift from its heat into the potential energy
/// (`store::fall_heat`, `Store::lifted`).
#[test]
fn a_gas_that_rises_pays_its_latent_heat_and_its_lift() {
    let mut w = small_world();
    let i = 3;
    let (j, latent) = (w.mesh.up(i, 0, AIR) as u32, w.chem.subs[LIQUID].latent_gas());
    let (e_air, e_ground, pot, books) = (w.store.heat(Class::Air)[j as usize], w.store.heat(Class::Ground)[i], w.store.lifted(), Books::now(&w));
    let (heat, lift) = (1e-6 * e_ground, 1e-9 * e_ground);
    let m = 0.5 * heat / latent;
    let rise = LinkFlux { class: [Class::Ground, Class::Air], a: &[i as u32], b: &[j], ks: &[LIQUID as u16], mol: &[m], heat: &[heat] };
    apply(&mut w, &[rise], &[]);
    let mut paid = vec![0.0; w.mesh.level(AIR).n()];
    paid[j as usize] = -lift;
    fall_heat(&mut w, Class::Air, &paid);
    books.assert_kept(&w, 1e-9);
    let near = |x: f64, y: f64, of: f64| (x - y).abs() <= 1e-12 * of;
    assert!(near(w.store.heat(Class::Air)[j as usize] - e_air, heat - m * latent - lift, e_air) && near(e_ground - w.store.heat(Class::Ground)[i], heat, e_ground));
    assert!(near(w.store.lifted() - pot, lift, pot.abs()), "the potential energy gained {} of {lift} J", w.store.lifted() - pot);
}

/// A node of a random class, and its index.
fn any_node(w: &World, r: &mut Rng) -> (Class, usize) {
    let class = CLASSES[r.below(3)];
    (class, r.below(arrays(w, class).2))
}

/// What a round of random changes has not yet asked of each node of `CLASSES`: of each amount (`[k * len + index]`),
/// and of its heat. Each change asks only for what is left, so no node is asked for more than it holds.
struct Left {
    amount: Vec<Vec<f64>>,
    heat: Vec<Vec<f64>>,
}

impl Left {
    fn of(w: &World) -> Left {
        let (amount, heat) = CLASSES.iter().map(|&c| (arrays(w, c).0.to_vec(), arrays(w, c).1.to_vec())).unzip();
        Left { amount, heat }
    }
    /// What is left of amount `k` of node (`class`, `i`), or of its heat (`k` none).
    fn at(&mut self, w: &World, (class, i): (Class, usize), k: Option<usize>) -> &mut f64 {
        let c = CLASSES.iter().position(|&x| x == class).unwrap();
        match k {
            Some(k) => &mut self.amount[c][k * arrays(w, class).2 + i],
            None => &mut self.heat[c][i],
        }
    }
    /// Asks a random share of what is left of it: what it gives.
    fn some(&mut self, w: &World, r: &mut Rng, node: (Class, usize), k: Option<usize>) -> f64 {
        let want = r.f64() * *self.at(w, node, k);
        self.take(w, node, k, want)
    }
    /// Asks up to `want` of it: what it gives.
    fn take(&mut self, w: &World, node: (Class, usize), k: Option<usize>, want: f64) -> f64 {
        let left = self.at(w, node, k);
        let given = want.min(*left);
        *left -= given;
        given
    }
}

/// One round of random changes, each asking a random share of what its node has left (`Left`): a link flux of two
/// substances and heat over half of them (between ground nodes and deep stores), then moves between nodes of any
/// classes; in one call, or the flux alone first.
fn random_round(w: &mut World, r: &mut Rng) {
    let (ns, per, mut left) = (w.chem.ns(), PER_ROUND / 2, Left::of(w));
    let class = [ALIKE[r.below(2)], ALIKE[r.below(2)]];
    let len = class.map(|c| arrays(w, c).2);
    let (a, b): (Vec<u32>, Vec<u32>) = (0..per).map(|_| (r.below(len[0]) as u32, r.below(len[1]) as u32)).unzip();
    let k = r.below(ns);
    let ks = [k, (k + 1 + r.below(ns - 1)) % ns].map(|k| k as u16);
    // + from a to b, or back: each takes from the end it leaves.
    let end = |r: &mut Rng, l: usize| if r.below(2) == 0 { ((class[0], a[l] as usize), 1.0) } else { ((class[1], b[l] as usize), -1.0) };
    let (mut mol, mut heat) = (Vec::new(), Vec::new());
    for &k in &ks {
        for l in 0..per {
            let (from, s) = end(r, l);
            mol.push(s * left.some(w, r, from, Some(k as usize)));
        }
    }
    for l in 0..per {
        let (from, s) = end(r, l);
        heat.push(s * left.some(w, r, from, None));
    }
    let flux = LinkFlux { class, a: &a, b: &b, ks: &ks, mol: &mol, heat: &heat };
    // A move gives `to` its heat less what the mol hold more there (`store::held`): `to` pays the rest, as far as it can.
    let moves: Vec<Move> = (0..per)
        .map(|_| {
            let (from, to, k) = (any_node(w, r), any_node(w, r), r.below(ns));
            let (m, h) = (left.some(w, r, from, Some(k)), left.some(w, r, from, None));
            let lift = m * (held(&w.chem, to.0, k) - held(&w.chem, from.0, k));
            let mol = if lift > h { m * (h + left.take(w, to, None, lift - h)) / lift } else { m };
            Move { from: Node::new(from.0, from.1), to: Node::new(to.0, to.1), k: k as u16, mol, heat: h }
        })
        .collect();
    // In one call, or the flux alone and then the moves.
    if r.below(2) == 0 {
        apply(w, &[flux], &moves);
    } else {
        apply(w, &[flux], &[]);
        apply(w, &[], &moves);
    }
}

/// A million random moves and link fluxes that ask no node for more than it holds keep every element and the energy to
/// rounding, and leave no amount and no heat below zero; what was set to zero is rounding.
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
    let mol: f64 = CLASSES.iter().flat_map(|&c| arrays(&w, c).0).sum();
    assert!(w.counts.rounded[0] <= 1e-12 * mol && w.counts.rounded[1] <= 1e-12 * t1.thermal, "rounded {:?}", w.counts.rounded);
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
/// sun, heat out to space.
#[test]
fn what_crosses_the_boundary_keeps_the_books() {
    let mut w = small_world();
    let books = Books::now(&w);
    let (from, to) = (Node::new(Class::Boundary, Boundary::Outside as usize), Node::new(Class::Ground, 0));
    apply(&mut w, &[], &[Move { from, to, k: LIQUID as u16, mol: 1e3, heat: 1e6 }]);
    put(&mut w, ROCK, 1, 1e2);
    let q = 1e-3 * w.store.heat(Class::Air)[0];
    w.cross((Boundary::Sun, Node::ground(2)), [], q);
    w.cross((Boundary::Space, Node::air(0)), [], -q);
    books.assert_kept(&w, 1e-9);
}
