use pcore::body::{Layout, MUT};
use pcore::rng::{Purpose, Rng};

#[test]
fn child_has_parent_genes_plus_noise() {
    let mut w = worldgen::new_world(1, 32, 32);
    worldgen::life::seed(&mut w);
    let parent = w.life.alive().next().expect("a body").genes.clone();
    let lay = Layout::of(&w.chem);
    let child = pcore::copy::mutate(&lay, &parent, &mut Rng::new(1, 0, 0, Purpose::Mutate));
    let size = 10f32.powf(parent[MUT]);
    let diff: Vec<f32> = parent.iter().zip(&child).map(|(a, b)| (a - b).abs()).collect();
    assert!(diff.iter().any(|&d| d > 0.0), "no change");
    assert!(diff.iter().all(|&d| d <= 6.0 * size), "change too big");
}

#[test]
fn bodies_keep_elements_and_energy() {
    let mut w = worldgen::new_world(1, 32, 32);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    for _ in 0..24 {
        pcore::step(&mut w);
    }
    worldgen::life::seed(&mut w);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    assert!(w.life.alive().count() > 0);
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
