//! Partial damage and healing (15 Oct 2026): the energy of a bite (`choose::take_body`), of
//! being pushed through (`choose::wear`) and of heat breaks body matter first, which is what holds a body
//! together, and spills the stores it held in proportion (`body::wound`); a body that keeps enough of itself
//! lives on smaller, weaker (force goes by its body, `Body::built`), and rebuilds what it lost as it grows
//! (`copy::grow`, at `c_copy` within `c_build`). In a node the size of the hit decides: less than one member's
//! body takes part of every member's matter, one member or more takes whole members.

use pcore::body::{Body, COPY_MASS, DISPERSE, LIQ, STRENGTH};
use pcore::rng::{Purpose, Rng};
use pcore::{body, bond, brain, choose, copy, World};
use worldgen::testkit::{add, founder, hold_ground, kg, place, scale, Books, Brain, LIQUID, MATTER, RICH};

/// A world with a dry land cell; a bite costs no handling.
fn start() -> (World, usize) {
    let (mut w, i) = worldgen::testkit::start();
    // The area a taker works is not what these tests are about: a node's members are reached as an individual is.
    w.cfg.phys.sweep = 1e12;
    w.cfg.phys.c_handle = 0.0;
    (w, i)
}

/// A starting eater (30 kg: 9 kg of fuel, 15.75 kg of liquid, 5.25 kg of body matter, a body of 21 kg) that does
/// nothing by its brain, staying where it is, soaking no liquid up, of strength `strength`, copying at 10^`copy` kg.
fn eater(w: &mut World, i: usize, copy: f32, strength: f32) -> Body {
    let mut b = founder(w, 1, i, &Brain::only(brain::REST));
    let m = b.mass(&w.chem);
    scale(w, &mut b, 30.0 / m);
    (b.genes[DISPERSE], b.genes[STRENGTH], b.genes[COPY_MASS], b.genes[LIQ]) = (0.0, strength, copy, -4.0);
    b
}

/// A node of `n` members of the body `b` (stores and heat of all of them; booked as an edit).
fn node(w: &mut World, mut b: Body, n: f64) -> Body {
    scale(w, &mut b, n);
    b.count = n;
    b
}

/// Body `a` bites body `v` once.
fn bite(w: &mut World, a: usize, v: usize) {
    let mut b = std::mem::take(&mut w.life.bodies[a]);
    choose::take_body(w, a, &mut b, v, &mut Rng::new(1, w.day, 7, Purpose::Choose));
    w.life.bodies[a] = b;
}

/// What the victim and the attacker looked like.
struct Seen {
    built: f64,
    matter: f64,
    solid: f64,
    liquid: f64,
    force: f64,
}

fn seen(w: &World, s: usize) -> Seen {
    let b = &w.life.bodies[s];
    Seen { built: b.built(&w.chem), matter: kg(w, b, MATTER), solid: kg(w, b, RICH), liquid: kg(w, b, LIQUID), force: bond::force(w, b) }
}

/// A bite breaks body matter and spills the stores it held in proportion; an individual lives on smaller and
/// weaker (its force goes by its body), and rebuilds the matter at the build rate. Elements and energy are kept.
#[test]
fn an_attacked_individual_loses_body_and_lives_on_weaker_then_rebuilds() {
    let (mut w, i) = start();
    let a = eater(&mut w, i, 6.0, 1.0);
    let v = eater(&mut w, i, 6.0, 0.3);
    let (a, v) = (place(&mut w, a), place(&mut w, v));
    let books = Books::now(&w);
    let before = seen(&w, v);
    let (n, ground0, gained0) = (w.grid.n(), w.ground[RICH * w.grid.n() + i], kg(&w, &w.life.bodies[a], RICH) + kg(&w, &w.life.bodies[a], MATTER));
    bite(&mut w, a, v);
    assert!(w.life.bodies[v].alive, "the victim died of a bite of {} kg", before.built);
    let after = seen(&w, v);
    let gained = kg(&w, &w.life.bodies[a], RICH) + kg(&w, &w.life.bodies[a], MATTER) - gained0;
    let share = 1.0 - after.built / before.built;
    println!("bite: attacker took {gained:.3} kg, victim's body {:.2} -> {:.2} kg ({:.1}% torn), matter {:.3} -> {:.3}, solid {:.2} -> {:.2}, liquid {:.2} -> {:.2}, force {:.1} -> {:.1} N", before.built, after.built, 100.0 * share, before.matter, after.matter, before.solid, after.solid, before.liquid, after.liquid, before.force, after.force);
    // It lost body, and the stores that matter held in proportion (the same share of each, but where the taker
    // took more of one than that share).
    assert!(share > 0.02, "a bite tore only {share} of the body");
    assert!((1.0 - after.matter / before.matter - share).abs() < 1e-9, "matter and body tore different shares");
    for (x, y) in [(before.solid, after.solid), (before.liquid, after.liquid)] {
        assert!(1.0 - y / x >= share - 1e-9, "a store lost {} of the {share} torn", 1.0 - y / x);
    }
    // Weaker: force goes by the body, so it fell by the share of the body torn.
    assert!((after.force / before.force - after.built / before.built).abs() < 1e-9, "force {} -> {} N", before.force, after.force);
    // What the taker did not take went to the ground as dead matter.
    assert!(w.ground[RICH * n + i] > ground0, "no dead matter on the ground");
    books.assert_kept(&w, 1e-9);

    // It rebuilds, at c_copy within c_build: each update by no more than the allowance, and over time to what it
    // was (given fuel: it is the fuel it converts, and it keeps its reserve).
    let mut b = std::mem::take(&mut w.life.bodies[v]);
    add(&mut w, &mut b, RICH, 6.0);
    let (m1, days) = (b.matter_kg(&w.chem), b.interval() as f64);
    let mut last = b.matter_kg(&w.chem);
    let mut updates = 0;
    while b.built(&w.chem) < before.built * (1.0 - 1e-3) && updates < 1000 {
        let room = w.cfg.phys.c_build * b.supply(&w.chem, &w.cfg.phys) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
        assert!(copy::grow(&mut w, &mut b, days), "it did not grow after {updates} updates");
        let made = b.matter_kg(&w.chem) - last;
        assert!(made > 0.0 && made <= room * (1.0 + 1e-9), "built {made} kg in an update of allowance {room}");
        last = b.matter_kg(&w.chem);
        updates += 1;
    }
    println!("rebuilt {:.3} kg of matter in {updates} updates of {days} days ({:.0} days)", last - m1, updates as f64 * days);
    assert!(b.built(&w.chem) >= before.built * (1.0 - 1e-3) && updates > 5, "rebuilt in {updates} updates");
    // (An immature body puts all it builds into itself: no brood.)
    assert_eq!(b.brood, 0.0);
    w.life.bodies[v] = b;
    books.assert_kept(&w, 1e-9);
}

/// A bite of more than half of a body kills it: below half of what it was, an individual dies (cause TAKEN).
#[test]
fn a_bite_of_more_than_half_the_body_kills() {
    let (mut w, i) = start();
    let a = eater(&mut w, i, 6.0, 1.0);
    let mut a = a;
    scale(&mut w, &mut a, 8.0);
    let v = eater(&mut w, i, 6.0, 0.0);
    let (a, v) = (place(&mut w, a), place(&mut w, v));
    let books = Books::now(&w);
    w.events.clear();
    bite(&mut w, a, v);
    let died = w.events.iter().any(|e| matches!(e.kind, pcore::event::EventKind::Died { cause, .. } if cause == body::TAKEN));
    assert!(!w.life.bodies[v].alive && died, "a victim bitten by a body 8 times its size lived");
    books.assert_kept(&w, 1e-9);
}

/// Grazing: bites smaller than one member's body take part of every member's matter, so the node keeps its
/// members with smaller bodies; a hurt adult node is below maturity, so it is immature until healed and stays a node.
#[test]
fn a_node_grazed_by_small_bites_keeps_its_members_with_smaller_bodies() {
    let (mut w, i) = start();
    // Members of 30 kg (body 21 kg, mature at a copy mass of 15 kg); a small taker (a bite far under a member's body).
    let mut a = eater(&mut w, i, 6.0, 1.0);
    scale(&mut w, &mut a, 0.05);
    let m = eater(&mut w, i, 15f32.log10(), 0.0);
    let v = node(&mut w, m, 100.0);
    let (a, v) = (place(&mut w, a), place(&mut w, v));
    let books = Books::now(&w);
    let b = &w.life.bodies[v];
    let (n0, mean0, adult0) = (b.n(), b.built(&w.chem) / b.n(), b.adult(&w.chem));
    assert!(adult0, "members of {mean0} kg are not mature");
    let before = seen(&w, v);
    for _ in 0..3 {
        bite(&mut w, a, v);
        w.day += 1;
    }
    let (b, after) = (&w.life.bodies[v], seen(&w, v));
    let (mean1, took) = (b.built(&w.chem) / b.n(), before.built - after.built);
    println!("{} members of {mean0:.3} kg after three small bites: {} members of {mean1:.3} kg ({took:.4} kg of body torn)", n0, b.n());
    assert!(w.life.bodies[v].alive && b.n() == n0, "members died: {} of {n0}", b.n());
    assert!(took > 0.0 && mean1 < mean0, "no body torn");
    // A hurt adult node: below its copy mass, so immature until it has healed; still one node.
    let copy_mass = b.dec(COPY_MASS);
    assert!(adult0 && (b.adult(&w.chem) == (mean1 >= copy_mass * (1.0 - 1e-9))));
    books.assert_kept(&w, 1e-9);
}

/// A bite of one member's body or more takes whole members: the count falls by the share of the node that was eaten (the
/// body falls by the same share), the survivors keep their bodies.
#[test]
fn a_big_bite_takes_whole_members() {
    let (mut w, i) = start();
    // Members of 0.3 kg (a body of 0.21 kg) against a taker of 240 kg.
    let mut a = eater(&mut w, i, 6.0, 1.0);
    scale(&mut w, &mut a, 8.0);
    let mut m = eater(&mut w, i, 6.0, 0.0);
    scale(&mut w, &mut m, 0.01);
    let v = node(&mut w, m, 1000.0);
    let (a, v) = (place(&mut w, a), place(&mut w, v));
    let books = Books::now(&w);
    let b = &w.life.bodies[v];
    let (n0, mean0) = (b.n(), b.built(&w.chem) / b.n());
    let before = seen(&w, v);
    bite(&mut w, a, v);
    let (b, after) = (&w.life.bodies[v], seen(&w, v));
    let (mean1, share) = (b.built(&w.chem) / b.n(), 1.0 - after.built / before.built);
    println!("{n0} members of {mean0:.4} kg: {:.1} members left of {mean1:.4} kg ({:.2}% of the body torn)", b.n(), 100.0 * share);
    assert!(b.n() < n0 && (b.n() / n0 - (1.0 - share)).abs() < 1e-9, "count {} of {n0}, share {share}", b.n());
    assert!((mean1 / mean0 - 1.0).abs() < 1e-9, "survivors changed: {mean0} -> {mean1} kg");
    books.assert_kept(&w, 1e-9);
}

/// Being pushed through breaks body matter: the share broken is of the body, with the stores it held; an
/// individual lives on smaller, a node of members that weigh less than the hit loses members, and one of
/// bigger members is hurt in every member.
#[test]
fn trampling_breaks_body_matter() {
    let (mut w, i) = start();
    let mover = eater(&mut w, i, 6.0, 1.0);
    let v = eater(&mut w, i, 15f32.log10(), 0.0);
    let m = eater(&mut w, i, 6.0, 0.0);
    let big = node(&mut w, m, 10.0);
    let mut small = eater(&mut w, i, 6.0, 0.0);
    scale(&mut w, &mut small, 0.001);
    let small = node(&mut w, small, 10000.0);
    let (mv, v, big, small) = (place(&mut w, mover), place(&mut w, v), place(&mut w, big), place(&mut w, small));
    let books = Books::now(&w);
    let (b0, nb0, ns0) = (seen(&w, v), w.life.bodies[big].n(), w.life.bodies[small].n());
    let (bb0, bs0) = (seen(&w, big), seen(&w, small));
    let (g0, e0) = (w.ground[MATTER * w.grid.n() + i], w.e_ground[i]);
    let broken = choose::wear(&mut w, i, mv, 2_000.0);
    let (b1, bb1, bs1) = (seen(&w, v), seen(&w, big), seen(&w, small));
    let (sv, sb, ss) = (1.0 - b1.built / b0.built, 1.0 - bb1.built / bb0.built, 1.0 - bs1.built / bs0.built);
    println!("trampled: individual body -{:.2}%, node of 10 big members -{:.2}%, node of 10,000 small ones -{:.2}% (members {} of {ns0}); {broken:.3} kg broken", 100.0 * sv, 100.0 * sb, 100.0 * ss, w.life.bodies[small].n());
    assert!(broken > 0.0 && sv > 0.0 && w.life.bodies[v].alive, "an individual was not hurt");
    // The share of the matter broken is the share of the body and of every store (not gas).
    assert!((1.0 - b1.matter / b0.matter - sv).abs() < 1e-9 && (1.0 - b1.solid / b0.solid - sv).abs() < 1e-9 && (1.0 - b1.liquid / b0.liquid - sv).abs() < 1e-9);
    // Weaker.
    assert!((b1.force / b0.force).is_nan() || b0.force == 0.0 || (b1.force / b0.force - b1.built / b0.built).abs() < 1e-9);
    // The same energy over a node of members each lighter than the hit: members lost; of heavier: all hurt.
    assert!(w.life.bodies[big].n() == nb0 && sb > 0.0, "a node of heavy members lost members: {} of {nb0}", w.life.bodies[big].n());
    assert!(w.life.bodies[small].n() < ns0, "a node of light members kept all {ns0}");
    assert!(w.ground[MATTER * w.grid.n() + i] > g0 && w.e_ground[i] > e0, "nothing went to the ground");
    books.assert_kept(&w, 1e-9);
}

/// A hurt adult is immature until healed: it follows from `Body::adult` reading the body, so it makes no
/// offspring while it rebuilds (everything it builds is its own growth), and is mature again once rebuilt.
#[test]
fn a_hurt_adult_is_immature_until_it_has_healed() {
    let (mut w, i) = start();
    let mover = eater(&mut w, i, 6.0, 1.0);
    // A body of 21 kg that is mature from 15 kg.
    let v = eater(&mut w, i, 15f32.log10(), 0.0);
    let (mv, v) = (place(&mut w, mover), place(&mut w, v));
    assert!(w.life.bodies[v].adult(&w.chem));
    let books = Books::now(&w);
    // The energy that breaks 40% of it (`Body::breaking`, 16 Oct 2026: by what it is built of; 40,000 J at 3e-5 kg/J before).
    let e = w.life.bodies[v].breaking(&w.chem, &w.cfg, 0.4);
    choose::wear(&mut w, i, mv, e);
    let mut b = std::mem::take(&mut w.life.bodies[v]);
    println!("hurt to {:.2} kg of a copy mass of {:.2}", b.built(&w.chem), b.dec(COPY_MASS));
    assert!(!b.adult(&w.chem), "a body torn by 40% is still mature");
    let days = b.interval() as f64;
    let mut updates = 0;
    while !b.adult(&w.chem) && updates < 2000 {
        assert!(copy::grow(&mut w, &mut b, days));
        // (The update that crosses the size of maturity puts what it built past it, by `ALLOC`, into a brood.)
        assert!(b.adult(&w.chem) || b.brood == 0.0, "a hurt body put matter into a brood");
        updates += 1;
    }
    println!("mature again after {updates} updates of {days} days (matter {:.2} kg)", kg(&w, &b, MATTER));
    assert!(b.adult(&w.chem) && updates > 5);
    w.life.bodies[v] = b;
    books.assert_kept(&w, 1e-9);
}

/// Heat burns body matter: a warm body that lives on (its liquid does not boil) burns matter by the reaction that burns
/// it, so it is smaller in body, not only in fuel, with no new rule (the reactions of `body::live` act on the matter in its
/// stores, and the body is its matter over 1 - h); it dies when the matter falls below its safe minimum (day 40 of
/// 350 K for this body since the litter constants of 15 Oct 2026). Elements and energy are kept.
///
/// (30 days, not 20: the solid now burns bare 4 times faster, 24% a year at 290 K, so in the first update of a hot body
/// it burns first and takes the oxygen the body has at hand, and the matter, which needs its sixth power, follows in
/// the next: 5.25 -> 4.90 kg of matter after the first update, 1.64 kg after the second, against 4.61 kg at day 20
/// before. What is tested is that heat makes a body smaller in body as well as in fuel, not how soon.)
#[test]
fn a_body_burnt_by_heat_and_alive_is_smaller_in_body() {
    let (mut w, i) = start();
    w.cfg.phys.c_build = 0.0;
    // Upkeep goes with the body's temperature (15 Oct 2026), about 100 times as much at 350 K as at 290 K: a body
    // that paid it would die of `SPENT` on the first days. This test is about what the heat does to the matter, so
    // the costs of keeping tissue are put to nothing.
    for c in [&mut w.cfg.phys.c_basal, &mut w.cfg.phys.c_cat, &mut w.cfg.phys.c_strength, &mut w.cfg.phys.c_learn, &mut w.cfg.phys.c_signal, &mut w.cfg.phys.c_leaf, &mut w.cfg.phys.c_brain] {
        *c = 0.0;
    }
    let mut b = eater(&mut w, i, 6.0, 0.0);
    // (Nor does it evaporate to cool, since 16 Oct 2026: above its `COOL_T` of 320 K it dried and died of it on day 20.)
    b.genes[pcore::body::COOL_T] = 400.0;
    let s = place(&mut w, b);
    let books = Books::now(&w);
    let before = seen(&w, s);
    for d in 0..30 {
        // The cell held at 350 K, booked as an edit.
        hold_ground(&mut w, i, 350.0);
        pcore::day(&mut w);
        if !w.life.bodies[s].alive {
            let cause = w.events.iter().find_map(|e| if let pcore::event::EventKind::Died { cause, .. } = e.kind { Some(cause) } else { None });
            panic!("it died on day {d}, cause {cause:?} (0 lack, 1 phase, 2 spent)");
        }
    }
    let alive = w.life.bodies[s].alive;
    let after = seen(&w, s);
    println!("alive {alive}: body {:.2} -> {:.2} kg, matter {:.3} -> {:.3}, solid {:.2} -> {:.2}", before.built, after.built, before.matter, after.matter, before.solid, after.solid);
    assert!(alive, "it died");
    assert!(after.matter < 0.9 * before.matter && after.built < 0.9 * before.built, "its body did not shrink with its fuel");
    books.assert_kept(&w, 1e-9);
}
