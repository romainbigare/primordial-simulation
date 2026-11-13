//! The roster of starting kinds (`worldgen::life::ROSTER`): every kind can be placed, its cover labelled by its row,
//! and the world stays whole (elements and energy booked, nothing negative).

use worldgen::life::{seed, KINDS, ROSTER};

#[test]
fn every_roster_kind_is_placed_and_the_world_holds() {
    let mut w = worldgen::new_world(3, 32, 32);
    let t0 = pcore::check::totals(&w);
    pcore::step(&mut w);
    for &k in &ROSTER {
        let line = seed(&mut w, k);
        let name = KINDS[k].name;
        assert!(line.contains(name), "no seeding line for kind {k}: {line:?}");
        assert!(pcore::cover::sums(&w).get(k).is_some_and(|s| s.slots > 0.0), "kind {name} placed no cover");
        pcore::step(&mut w);
    }
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
