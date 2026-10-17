//! The starting state worldgen makes: the dissolved mineral in the low ground's liquid, and the soil given when life starts.

use pcore::World;

/// Substances of chemistry 0: the common liquid, and the mineral that exists only dissolved in it.
const LIQUID: usize = 0;
const DISSOLVED: usize = 23;

/// All of substance `k` in the world's ground and bodies, mol.
fn total(w: &World, k: usize) -> f64 {
    let n = w.grid.n();
    // (With the store of the bed under the ground, where drained liquid takes what is dissolved in it.)
    w.ground[k * n..(k + 1) * n].iter().sum::<f64>() + w.deep.get(k * n..(k + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>()) + w.life.alive().map(|b| b.store[k]).sum::<f64>()
}

/// The world starts with the dissolved mineral in the liquid of its low ground and none on dry land.
#[test]
fn a_world_starts_with_the_dissolved_mineral_in_its_liquid() {
    let w = worldgen::new_world(1, 24, 24);
    let n = w.grid.n();
    let (mut wet, mut dry) = (0.0, 0.0);
    for i in 0..n {
        if w.ground[LIQUID * n + i] > 0.0 {
            assert!(w.ground[DISSOLVED * n + i] > 0.0, "cell {i} has liquid and none of the mineral");
            wet += w.ground[DISSOLVED * n + i];
        } else {
            dry += w.ground[DISSOLVED * n + i];
        }
    }
    assert!(wet > 0.0 && dry == 0.0, "the mineral in the liquid {wet}, on dry land {dry}");
}

/// The soil is given to the land when life starts: `Init::soil` mol per m² of the fixed gas's mineral (10 g of the
/// gas's element, 14 g a mol, to the m²) in whole cells of land, booked as new matter and energy.
#[test]
fn the_soil_is_given_to_the_land_and_booked() {
    let mut w = worldgen::new_world(1, 16, 16);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let (mineral, _) = w.chem.minerals()[0];
    let soil = worldgen::chemistry0().1.soil;
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let before = total(&w, mineral);
    println!("{}", worldgen::life::soil(&mut w, &soil));
    let (k, per_m2) = soil[0];
    let cells = (total(&w, mineral) - before) / (per_m2 * w.grid.area());
    assert!(k == mineral && (cells - cells.round()).abs() < 1e-9 && cells >= 10.0 && cells <= w.grid.n() as f64, "{cells} cells' worth");
    let grams = per_m2 * w.chem.subs[k].elems[2] as f64 * 14.0;
    assert!((grams - 10.0).abs() < 0.05, "{grams} g of the gas's element to the m²");
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
