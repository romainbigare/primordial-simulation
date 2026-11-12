//! The start of the cover (`worldgen::life::nursery`): when life starts, the cover its gardens keep is placed on the
//! open land with the books kept, and the same seed gives the same start, bit for bit.

use pcore::cover;

#[test]
fn the_cover_is_placed_with_the_books_kept_and_the_same_for_a_seed() {
    let mut w = worldgen::new_world(1, 22, 22);
    pcore::flow::state(&mut w, true);
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let (t0, mut again) = (pcore::check::totals(&w), w.clone());
    let line = worldgen::life::nursery::start(&mut w);
    assert!(line.contains("nursery sorted the cover"), "{line}");
    let land: Vec<usize> = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i) && w.life.cover(i).any()).collect();
    assert!(land.len() * 10 >= (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).count() * 9, "the cover is on {} land cells: {line}", land.len());
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
    assert_eq!(worldgen::life::nursery::start(&mut again), line);
    let slots = |w: &pcore::World| cover::slots(w).map(|(r, g)| (r, g.map(f32::to_bits))).collect::<Vec<_>>();
    assert_eq!(slots(&w), slots(&again), "the same seed gave another start");
}
