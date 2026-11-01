//! The cover (laws.md 4.5): a field of light-eaters per cell grows in light and dies in frost, keeping the books, and
//! animals graze it by their take from the place.

use pcore::body::{PHASE, TAKEN};
use pcore::cover::{self, FROST};
use pcore::event::EventKind;
use worldgen::life::{Place, KINDS};
use worldgen::testkit::{dry_land, eater, small_world, Books};

/// The genes of the starting cover (kind 0).
fn genes() -> [f32; cover::NG] {
    let Place::Field(f) = &KINDS[0].place else { unreachable!("kind 0 is cover") };
    f.genes
}

/// The world's cover, kg.
fn cover_kg(w: &pcore::World) -> f64 {
    cover::sums(w).iter().map(|s| s.kg).sum()
}

/// A small world with its soil, after a month (Flow's state built), and a dry land cell.
fn start() -> (pcore::World, usize) {
    let mut w = small_world();
    pcore::step(&mut w);
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let i = dry_land(&w);
    (w, i)
}

/// 0.1 kg per m² of the starting cover on one cell grows over a year of light (it builds from its light reaction and
/// the soil's minerals, and spreads seeds), and every element and the energy are kept.
#[test]
fn the_cover_grows_in_light_and_keeps_the_books() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    assert!(cover::place(&mut w, i, genes(), 0, 0.1));
    let kg0 = cover_kg(&w);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let kg = cover_kg(&w);
    println!("cover {kg0:.3e} kg, a year later {kg:.3e} kg in {} slots", cover::sums(&w)[0].slots);
    assert!(kg > 2.0 * kg0, "the cover did not grow: {kg0} then {kg}");
    assert_eq!(pcore::check::broken(&w), None);
    books.assert_kept(&w, 1e-9);
}

/// A cover far thicker than its light can keep (20 kg per m², a capture area 100 times its ground) does not thicken:
/// one more kg would not pay its upkeep at the bottom of its layer, so all it builds leaves as seeds, and over a
/// summer its cell holds less than it started with; the books are kept.
#[test]
fn a_thick_cover_does_not_thicken() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    assert!(cover::place(&mut w, i, genes(), 0, 20.0));
    let kg0 = cover::per_m2(&w)[i];
    for _ in 0..6 {
        pcore::step(&mut w);
    }
    let kg = cover::per_m2(&w)[i];
    println!("cover on its cell {kg0:.2} kg/m2, six months later {kg:.2} kg/m2");
    assert!(kg < kg0, "the thick cover thickened: {kg0} then {kg}");
    books.assert_kept(&w, 1e-9);
}

/// The light reaction goes with its gas in the air (`Physics::gas_factor`): nothing at the compensation point, all
/// it stores at `light_yield` at the reference share, and less in between.
#[test]
fn the_light_reaction_goes_with_its_gas() {
    let p = pcore::Physics::default();
    assert_eq!(p.gas_factor(p.gas_floor), 0.0);
    assert!((p.gas_factor(p.gas_ref) - 1.0).abs() < 1e-12);
    let half = p.gas_factor(0.5 * p.gas_ref);
    assert!(half > 0.0 && half < 1.0 && p.gas_factor(2.0 * p.gas_ref) > 1.0, "{half}");
}

/// Cover whose frost gene lies above its ground's temperature dies of frost in its month (`PHASE`), and what dies goes
/// to the ground: the books are kept.
#[test]
fn the_cover_dies_in_frost() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    let mut g = genes();
    g[FROST] = 300.0;
    assert!(w.t_ground_last(i) < 299.0, "the ground is warm: {} K", w.t_ground_last(i));
    assert!(cover::place(&mut w, i, g, 0, 1.0));
    let kg0 = cover_kg(&w);
    w.take_counters();
    pcore::step(&mut w);
    let lost = w.take_counters().cover_lost;
    println!("cover {kg0:.3e} kg, a month later {:.3e} kg; lost to frost {:.3e} kg", cover_kg(&w), lost[PHASE as usize]);
    assert!(lost[PHASE as usize] > 0.1 * kg0 && cover_kg(&w) < kg0, "no frost: lost {lost:?}");
    books.assert_kept(&w, 1e-9);
}

/// An animal whose brain only takes from the place, in a cell of thick cover, takes from the cover (grazing): its
/// event says so, the cover loses what was eaten (`TAKEN`), and the books are kept.
#[test]
fn grazing_takes_from_the_cover() {
    let (mut w, i) = start();
    assert!(cover::place(&mut w, i, genes(), 0, 1.0));
    let b = eater(&mut w, i, pcore::brain::TAKE, 1.0);
    w.add(b);
    let books = Books::now(&w);
    w.take_counters();
    w.events.clear();
    for _ in 0..60 {
        pcore::day(&mut w);
    }
    let grazed: f64 = w.events.iter().filter_map(|e| matches!(e.kind, EventKind::Took { from: pcore::choose::COVER, .. }).then_some(e.value)).sum();
    let lost = w.take_counters().cover_lost[TAKEN as usize];
    println!("grazed {grazed:.3} kg; the cover lost {lost:.3} kg to it");
    assert!(grazed > 0.0 && (lost - grazed).abs() <= 1e-9 * grazed, "grazed {grazed}, lost {lost}");
    books.assert_kept(&w, 1e-9);
}
