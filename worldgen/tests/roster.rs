//! The roster of starting kinds (`worldgen::life::ROSTER`): every kind can be placed, each in its own
//! lineage, and the world stays whole (elements and energy booked, nothing negative).

use worldgen::life::{seed, KINDS, ROSTER};

#[test]
fn every_roster_kind_is_placed_and_the_world_holds() {
    let mut w = worldgen::new_world(3, 32, 32);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    for _ in 0..24 {
        pcore::step(&mut w);
    }
    for &k in &ROSTER {
        let (before, lineages) = (w.life.alive().count(), w.life.brains.len());
        let line = seed(&mut w, k);
        assert!(line.contains(KINDS[k].name), "no seeding line for kind {k}: {line:?}");
        assert!(w.life.alive().count() > before, "kind {} placed nothing", KINDS[k].name);
        assert_eq!(w.life.brains.len(), lineages + 1, "kind {} has its own lineage", KINDS[k].name);
        // Bodies are found in their cells after a month (animals are placed where bodies already are).
        pcore::step(&mut w);
    }
    for _ in 0..6 {
        pcore::step(&mut w);
    }
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
