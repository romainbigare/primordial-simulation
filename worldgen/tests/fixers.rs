//! The cover that fixes: a tenth of the cells of the starting cover carry the gene of the fixing reaction
//! (`cover::FIX`), an initial condition drawn by each cell, printed in the seeding line; zero means none.

use pcore::cover::{self, FIX};
use worldgen::life::{self, Seeding, FIXERS, FIX_GENE, LIGHT_ROW};

/// Slots of cover whose fixing gene is `FIX_GENE`, and slots of cover.
fn fixers_of(w: &pcore::World) -> (usize, usize) {
    cover::slots(w).fold((0, 0), |(on, all), (_, g)| (on + (g[FIX] == FIX_GENE) as usize, all + 1))
}

#[test]
fn a_tenth_of_the_cover_starts_with_the_fixing_gene() {
    let mut w = worldgen::testkit::one_rise(1, 64, 64);
    pcore::step(&mut w);
    let line = life::seed(&mut w, LIGHT_ROW);
    let (on, all) = fixers_of(&w);
    println!("{line}");
    let share = on as f64 / all as f64;
    // (A binomial: the sd of the share of about 1,500 cells at a tenth is 0.8 points.)
    assert!(all > 1000 && (share - FIXERS).abs() < 4.0 * (FIXERS * (1.0 - FIXERS) / all as f64).sqrt(), "{on} of {all} carry the gene");
    assert!(line.contains(&format!("{on} cells")) && line.contains("fixing gene"), "{line}");
    // The others start with it off, whatever the spray did.
    let off = cover::slots(&w).filter(|x| x.1[FIX] != FIX_GENE).all(|x| !cover::on(x.1, FIX));
    assert!(off, "a gene turned on by the spray");
}

#[test]
fn the_share_is_a_setting_of_the_start_and_zero_means_none() {
    for (share, want) in [(0.0, 0.0), (1.0, 1.0)] {
        let mut w = worldgen::testkit::one_rise(1, 32, 32);
        pcore::step(&mut w);
        life::seed_with(&mut w, LIGHT_ROW, &Seeding { fixers: share, ..Default::default() });
        let (on, all) = fixers_of(&w);
        assert_eq!(on as f64, want * all as f64, "share {share}: {on} of {all}");
    }
}
