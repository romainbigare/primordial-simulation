//! Damage to a body: bites, trampling and heat break its body matter, and what it leaves.

use pcore::life::Handle;
use pcore::body::{self, Body, COPY_MASS, STRENGTH};
use pcore::rng::{Purpose, Rng};
use pcore::store::Class;
use pcore::units::DAY_S;
use pcore::{bond, brain, choose, World};
use worldgen::testkit::{node_of, eater, died, kg, place, scale, start, Books, LIQUID, MATTER, RICH};

/// A world with a dry land cell where a bite costs no handling, and the area a taker works does not bind (a node's
/// members are reached as an individual is).
fn bite_start() -> (World, usize) {
    let (mut w, i) = start();
    w.cfg.phys.sweep = 1e12;
    w.cfg.phys.c_handle = 0.0;
    (w, i)
}

/// A resting starting eater (`testkit::eater`) of strength `strength`, mature at 10^`copy` kg.
fn resting(w: &mut World, i: usize, copy: f32, strength: f32) -> Body {
    let mut b = eater(w, i, brain::REST, copy);
    b.genes[STRENGTH] = strength;
    b
}

/// Body `a` bites body `v` once.
fn bite(w: &mut World, a: Handle, v: Handle) {
    let mut b = std::mem::take(&mut w.life[a]);
    let r = &mut Rng::new(1, w.day(), 7, Purpose::Choose);
    w.in_cell(b.cell as usize, |w, d| choose::take_body(w, d, &mut b, v, r));
    w.life[a] = b;
}

/// What a body looked like: its body, its matter, solid and liquid (kg), and its force.
struct Seen {
    built: f64,
    matter: f64,
    solid: f64,
    liquid: f64,
    force: f64,
}

/// What the body at `s` looks like.
fn seen(w: &World, s: Handle) -> Seen {
    let b = &w.life[s];
    Seen { built: b.built(&w.chem), matter: kg(w, b, MATTER), solid: kg(w, b, RICH), liquid: kg(w, b, LIQUID), force: bond::bears(w, b) }
}

/// Damage: the energy of a bite (`choose::take_body`), of being pushed through (`choose::wear`) and of heat breaks body
/// matter first, which is what holds a body together, and spills the stores it held in proportion (`body::wound`). A
/// bite: the individual lives on smaller and weaker (its force goes by its body). Elements and energy are kept.
#[test]
fn an_attacked_individual_loses_body_and_lives_on_weaker() {
    let (mut w, i) = bite_start();
    let a = resting(&mut w, i, 6.0, 1.0);
    let v = resting(&mut w, i, 6.0, 0.3);
    let (a, v) = (place(&mut w, a), place(&mut w, v));
    let books = Books::now(&w);
    let before = seen(&w, v);
    let (n, ground0, gained0) = (w.mesh.cells().n(), w.store.amounts(Class::Ground)[RICH * w.mesh.cells().n() + i], kg(&w, &w.life[a], RICH) + kg(&w, &w.life[a], MATTER));
    bite(&mut w, a, v);
    assert!(w.life[v].alive, "the victim died of a bite of {} kg", before.built);
    let after = seen(&w, v);
    let gained = kg(&w, &w.life[a], RICH) + kg(&w, &w.life[a], MATTER) - gained0;
    let share = 1.0 - after.built / before.built;
    println!("bite: attacker took {gained:.3} kg, victim's body {:.2} -> {:.2} kg ({:.1}% torn), matter {:.3} -> {:.3}, solid {:.2} -> {:.2}, liquid {:.2} -> {:.2}, force {:.1} -> {:.1} N", before.built, after.built, 100.0 * share, before.matter, after.matter, before.solid, after.solid, before.liquid, after.liquid, before.force, after.force);
    // It lost body, and the stores that matter held in proportion (the same share of each, but where the taker took more
    // of one than that share).
    assert!(share > 0.02, "a bite tore only {share} of the body");
    assert!((1.0 - after.matter / before.matter - share).abs() < 1e-9, "matter and body tore different shares");
    for (x, y) in [(before.solid, after.solid), (before.liquid, after.liquid)] {
        assert!(1.0 - y / x >= share - 1e-9, "a store lost {} of the {share} torn", 1.0 - y / x);
    }
    // Weaker: force goes by the body, so it fell by the share of the body torn.
    assert!((after.force / before.force - after.built / before.built).abs() < 1e-9, "force {} -> {} N", before.force, after.force);
    // What the taker did not take went to the ground as dead matter.
    assert!(w.store.amounts(Class::Ground)[RICH * n + i] > ground0, "no dead matter on the ground");
    books.assert_kept(&w, 1e-9);
}

/// A bite of more than half of a body kills it: below half of what it was, an individual dies (cause TAKEN).
#[test]
fn a_bite_of_more_than_half_the_body_kills() {
    let (mut w, i) = bite_start();
    let mut a = resting(&mut w, i, 6.0, 1.0);
    scale(&mut w, &mut a, 20.0);
    let v = resting(&mut w, i, 6.0, 0.0);
    let (a, v) = (place(&mut w, a), place(&mut w, v));
    let books = Books::now(&w);
    w.events.clear();
    bite(&mut w, a, v);
    assert!(!w.life[v].alive && died(&w, body::TAKEN) > 0.0, "a victim bitten by a body 20 times its size lived");
    books.assert_kept(&w, 1e-9);
}

/// Grazing: bites smaller than one member's body take part of every member's matter, so the node keeps its members with
/// smaller bodies; a hurt adult node below its copy mass is immature until healed, and stays a node.
#[test]
fn a_node_grazed_by_small_bites_keeps_its_members_with_smaller_bodies() {
    let (mut w, i) = bite_start();
    // Members of 30 kg (body 21 kg, mature at a copy mass of 15 kg); a small taker (a bite far under a member's body).
    let mut a = resting(&mut w, i, 6.0, 1.0);
    scale(&mut w, &mut a, 0.05);
    let m = resting(&mut w, i, 15f32.log10(), 0.0);
    let v = node_of(&mut w, m, 100.0);
    let (a, v) = (place(&mut w, a), place(&mut w, v));
    let books = Books::now(&w);
    let b = &w.life[v];
    let (n0, mean0, adult0) = (b.n(), b.built(&w.chem) / b.n(), b.adult(&w.chem));
    assert!(adult0, "members of {mean0} kg are not mature");
    let before = seen(&w, v);
    for _ in 0..3 {
        bite(&mut w, a, v);
        w.time += DAY_S;
    }
    let (b, after) = (&w.life[v], seen(&w, v));
    let (mean1, took) = (b.built(&w.chem) / b.n(), before.built - after.built);
    println!("{} members of {mean0:.3} kg after three small bites: {} members of {mean1:.3} kg ({took:.4} kg of body torn)", n0, b.n());
    assert!(w.life[v].alive && b.n() == n0, "members died: {} of {n0}", b.n());
    assert!(took > 0.0 && mean1 < mean0, "no body torn");
    // A hurt adult node: below its copy mass, so immature until it has healed; still one node.
    let copy_mass = b.dec(COPY_MASS);
    assert!(adult0 && (b.adult(&w.chem) == (mean1 >= copy_mass * (1.0 - 1e-9))));
    books.assert_kept(&w, 1e-9);
}

/// The area a taker works in the big-bite test, m² per kg per day: it holds a few of the node's members.
const SWEEP_FEW: f64 = 50.0;

/// A bite reaches the members the taker's area holds (`choose::take_body`). A taker that takes body matter takes all
/// that thinly lying members hold and kills them: the count falls by those members (the body and the brood by the
/// same share), and the survivors keep their bodies.
#[test]
fn a_big_bite_kills_the_members_it_reaches() {
    let (mut w, i) = bite_start();
    w.cfg.phys.sweep = SWEEP_FEW;
    // Members of 0.3 kg (a body of 0.21 kg) against a taker of 240 kg; most of the members' matter is brood.
    let mut a = resting(&mut w, i, 6.0, 1.0);
    scale(&mut w, &mut a, 8.0);
    let mut m = resting(&mut w, i, 6.0, 0.0);
    scale(&mut w, &mut m, 0.01);
    let mut v = node_of(&mut w, m, 1000.0);
    v.brood = 0.5 * v.amount(w.chem.body(), MATTER);
    let (a, v) = (place(&mut w, a), place(&mut w, v));
    let books = Books::now(&w);
    let b = &w.life[v];
    let (n0, mean0, brood0, held0) = (b.n(), b.built(&w.chem) / b.n(), b.brood, b.mass(&w.chem) / b.n());
    let before = seen(&w, v);
    w.events.clear();
    bite(&mut w, a, v);
    let got: f64 = w.events.iter().map(|e| if let pcore::event::EventKind::Took { .. } = e.kind { e.value } else { 0.0 }).sum();
    let (b, after) = (&w.life[v], seen(&w, v));
    let (mean1, share, killed) = (b.built(&w.chem) / b.n(), 1.0 - after.built / before.built, n0 - b.n());
    println!("{n0} members of {mean0:.4} kg: {:.2} members left of {mean1:.4} kg ({:.2}% of the body torn by a bite of {got:.4} kg)", b.n(), 100.0 * share);
    assert!(killed > 1.0 && killed < n0 && (share - killed / n0).abs() < 1e-9, "count {} of {n0}, share {share}", b.n());
    assert!(got > 0.0 && got <= killed * held0 * (1.0 + 1e-9), "a bite of {got} kg from {killed} members of {held0} kg");
    assert!((mean1 / mean0 - 1.0).abs() < 1e-9, "survivors changed: {mean0} -> {mean1} kg");
    assert!((b.brood / brood0 - b.n() / n0).abs() < 1e-9, "brood {} of {brood0}, count {} of {n0}", b.brood, b.n());
    books.assert_kept(&w, 1e-9);
}

/// Being pushed through breaks body matter: the share broken is of the body, with the stores it held; an individual
/// lives on smaller, and a node is hurt in every member alike, whether its members are big or small.
#[test]
fn trampling_breaks_body_matter() {
    let (mut w, i) = bite_start();
    let v = resting(&mut w, i, 15f32.log10(), 0.0);
    let m = resting(&mut w, i, 6.0, 0.0);
    let big = node_of(&mut w, m, 10.0);
    let mut small = resting(&mut w, i, 6.0, 0.0);
    scale(&mut w, &mut small, 0.001);
    let small = node_of(&mut w, small, 10000.0);
    let (v, big, small) = (place(&mut w, v), place(&mut w, big), place(&mut w, small));
    let books = Books::now(&w);
    let (b0, nb0, ns0) = (seen(&w, v), w.life[big].n(), w.life[small].n());
    let (bb0, bs0) = (seen(&w, big), seen(&w, small));
    let (g0, e0) = (w.store.amounts(Class::Ground)[MATTER * w.mesh.cells().n() + i], w.store.heat(Class::Ground)[i]);
    // Movers spent 2 kJ pushing through the cell's bodies since the last survey.
    w.in_cell(i, |w, d| choose::wear(w, d, i, 2_000.0));
    let (b1, bb1, bs1) = (seen(&w, v), seen(&w, big), seen(&w, small));
    let (sv, sb, ss) = (1.0 - b1.built / b0.built, 1.0 - bb1.built / bb0.built, 1.0 - bs1.built / bs0.built);
    println!("trampled: individual body -{:.2}%, node of 10 big members -{:.2}%, node of 10,000 small ones -{:.2}% (members {} of {ns0})", 100.0 * sv, 100.0 * sb, 100.0 * ss, w.life[small].n());
    assert!(sv > 0.0 && w.life[v].alive, "an individual was not hurt");
    // The share of the matter broken is the share of the body and of every store (not gas).
    assert!((1.0 - b1.matter / b0.matter - sv).abs() < 1e-9 && (1.0 - b1.solid / b0.solid - sv).abs() < 1e-9 && (1.0 - b1.liquid / b0.liquid - sv).abs() < 1e-9);
    // Weaker.
    assert!((b1.force / b0.force).is_nan() || b0.force == 0.0 || (b1.force / b0.force - b1.built / b0.built).abs() < 1e-9);
    // Every member of a node is hurt alike: none dies short of half its body.
    assert!(w.life[big].n() == nb0 && sb > 0.0, "a node of heavy members lost members: {} of {nb0}", w.life[big].n());
    assert!(w.life[small].n() == ns0 && ss > 0.0, "a node of light members lost members: {} of {ns0}", w.life[small].n());
    assert!(w.store.amounts(Class::Ground)[MATTER * w.mesh.cells().n() + i] > g0 && w.store.heat(Class::Ground)[i] > e0, "nothing went to the ground");
    books.assert_kept(&w, 1e-9);
}
