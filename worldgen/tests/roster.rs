//! The roster of starting kinds (`worldgen::life::ROSTER`): every kind of bodies can be placed, each in its own lineage
//! labelled by its row, on the cover the nursery sorts, and the world stays whole (elements and energy booked, nothing
//! negative).

use worldgen::life::{seed, Place, KINDS, ROSTER};

#[test]
fn every_roster_kind_is_placed_and_the_world_holds() {
    let mut w = worldgen::new_world(3, 32, 32);
    let t0 = pcore::check::totals(&w);
    pcore::step(&mut w);
    worldgen::testkit::life(&mut w, &[]);
    for &k in ROSTER.iter().filter(|&&k| matches!(KINDS[k].place, Place::Bodies(_))) {
        let (lineages, old): (usize, Vec<u32>) = (w.life.brains.len(), w.life.alive().map(|b| b.lineage).collect());
        let line = seed(&mut w, k);
        let name = KINDS[k].name;
        assert!(line.contains(name), "no seeding line for kind {k}: {line:?}");
        assert_eq!(w.life.brains.len(), lineages + 1, "kind {name} has its own lineage");
        assert_eq!(w.life.brains[lineages].as_ref().map(|l| l.root), Some(k as u32), "kind {name} labels its lineage with its row");
        // Counted by its own lineage: placing also surveys the cells, where other bodies may merge or die.
        assert!(w.life.alive().any(|b| !old.contains(&b.lineage)), "kind {name} placed nothing");
        // Bodies are found in their cells after a month (animals are placed where life already is).
        pcore::step(&mut w);
    }
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
