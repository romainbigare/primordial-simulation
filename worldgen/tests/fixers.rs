//! The founders that fix: a tenth of the starting light-eaters carry the gene of the fixing reaction (`Chemistry::fixing`), at
//! the speed-up a growing body needs; it is an initial condition, drawn by each body's own id, printed in the seeding line, and
//! the animals have none.

use pcore::body::Layout;
use worldgen::life::{self, Seeding, FIXERS, FIX_GENE};

fn fixers_of(w: &pcore::World, fix: usize) -> (usize, usize) {
    let lay = Layout::of(&w.chem);
    let on = w.life.alive().filter(|b| b.genes[lay.cat + fix] == FIX_GENE).count();
    (on, w.life.alive().count())
}

#[test]
fn a_tenth_of_the_light_eaters_start_with_the_fixing_gene() {
    let mut w = worldgen::new_world(1, 64, 64);
    pcore::step(&mut w);
    let fix = w.chem.fixing().expect("the fixing reaction");
    let line = life::seed(&mut w, 0);
    let (on, all) = fixers_of(&w, fix);
    println!("{line}");
    let share = on as f64 / all as f64;
    // (A binomial: the sd of the share of about 1,500 bodies at a tenth is 0.8 points.)
    assert!(all > 1000 && (share - FIXERS).abs() < 4.0 * (FIXERS * (1.0 - FIXERS) / all as f64).sqrt(), "{on} of {all} carry the gene");
    // The line says how many, and the speed-up (log10 `FIX_GENE` is in the genes).
    assert!(line.contains(&format!("{on} of them")) && line.contains("the fixing reaction"), "{line}");
    let lay = Layout::of(&w.chem);
    // (A gene the spray turned on speeds the reaction up at most ten times, against 10^`FIX_GENE`.)
    assert!(w.life.alive().all(|b| b.genes[lay.cat + fix] == FIX_GENE || b.genes[lay.cat + fix] < 1.0), "a gene turned on by mutation is small");
    // The animals placed after them have none; and the same seed gives the same bodies.
    life::seed(&mut w, 1);
    let (on_all, _) = fixers_of(&w, fix);
    assert_eq!(on_all, on, "an animal carries the fixing gene");
}

#[test]
fn the_share_is_a_setting_of_the_start_and_zero_means_none() {
    let mut w = worldgen::new_world(1, 32, 32);
    pcore::step(&mut w);
    let fix = w.chem.fixing().expect("the fixing reaction");
    let line = life::seed_with(&mut w, 0, &Seeding { fixers: 0.0, ..Default::default() });
    assert_eq!(fixers_of(&w, fix).0, 0);
    assert!(line.contains("0 of them"), "{line}");
    let mut w = worldgen::new_world(1, 32, 32);
    pcore::step(&mut w);
    life::seed_with(&mut w, 0, &Seeding { fixers: 1.0, ..Default::default() });
    let (on, all) = fixers_of(&w, fix);
    assert_eq!(on, all, "every light-eater should carry it");
}
