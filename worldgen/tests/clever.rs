//! The big-brain test (`worldgen::life::seed_clever`): copies of living eaters
//! in two new lineages that differ in brain size only, with the same cells, genes, born rows and
//! stores; the world stays whole.

use worldgen::life::{seed, seed_clever, CLEVER_ROOT, CLEVER_SIZES, CONTROL_ROOT};

#[test]
fn the_big_brain_and_its_control_differ_in_brain_size_only() {
    let mut w = worldgen::new_world(1, 32, 32);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    pcore::step(&mut w);
    seed(&mut w, 0);
    seed(&mut w, 1);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let line = seed_clever(&mut w, 20);
    println!("{line}");
    let of = |w: &pcore::World, root: u32| -> Vec<pcore::body::Body> { w.life.alive().filter(|b| w.life.root(b) == root).cloned().collect() };
    let (mut big, mut control) = (of(&w, CLEVER_ROOT), of(&w, CONTROL_ROOT));
    for v in [&mut big, &mut control] {
        v.sort_by(|a, b| (a.cell, a.e).partial_cmp(&(b.cell, b.e)).unwrap());
    }
    assert_eq!((big.len(), control.len()), (20, 20), "{line}");
    for (a, c) in big.iter().zip(&control) {
        // Pairs are placed together: the same cell and store, and the genes differ in the four brain sizes only.
        assert_eq!((a.cell, &a.store, a.e), (c.cell, &c.store, c.e));
        let diff: Vec<usize> = (0..a.genes.len()).filter(|&g| a.genes[g] != c.genes[g]).collect();
        assert!(diff.iter().all(|g| (pcore::body::HIDDEN..=pcore::body::SIGHT).contains(g)), "genes differ at {diff:?}");
        assert_eq!(pcore::lineage::sizes(&a.genes), CLEVER_SIZES);
        let (la, lc) = (w.life.brains[a.lineage as usize].as_ref().expect("lineage"), w.life.brains[c.lineage as usize].as_ref().expect("lineage"));
        assert_eq!(pcore::lineage::sizes_of(la), CLEVER_SIZES);
        assert_ne!(pcore::lineage::sizes_of(lc), CLEVER_SIZES, "the control has the big brain");
        assert_eq!(a.genes[pcore::body::MUT], c.genes[pcore::body::MUT]);
    }
    pcore::step(&mut w);
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
