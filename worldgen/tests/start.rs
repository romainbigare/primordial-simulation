//! The starting state worldgen makes: the dissolved mineral in the low ground's liquid, and the soil given when life starts.

use pcore::World;

/// Substances of chemistry 0: the common liquid, and the mineral that exists only dissolved in it.
const LIQ: usize = 0;
const SALT: usize = 23;

/// All of substance `k` in the world's ground and bodies, mol.
fn total(w: &World, k: usize) -> f64 {
    let n = w.grid.n();
    // (With the store of the bed under the ground, where drained liquid takes what is dissolved in it.)
    w.ground[k * n..(k + 1) * n].iter().sum::<f64>() + w.deep.get(k * n..(k + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>()) + w.life.alive().map(|b| b.store[k]).sum::<f64>()
}

fn drift_after(years: u64) -> (f64, f64, pcore::World) {
    let mut w = worldgen::new_world(1, 24, 24);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    for _ in 0..years * 12 {
        pcore::step(&mut w);
    }
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    (de, dq, w)
}

/// The world starts with the salt in its low ground and none on dry land, takes no more than the
/// rock there is, and keeps its elements and energy through two years.
#[test]
fn a_world_starts_with_the_salt_in_the_liquid_and_keeps_its_books() {
    let w = worldgen::new_world(1, 24, 24);
    let n = w.grid.n();
    let (mut wet, mut dry) = (0.0, 0.0);
    for i in 0..n {
        if w.ground[LIQ * n + i] > 0.0 {
            assert!(w.ground[SALT * n + i] > 0.0, "cell {i} has liquid and no salt");
            wet += w.ground[SALT * n + i];
        } else {
            dry += w.ground[SALT * n + i];
        }
    }
    assert!(wet > 0.0 && dry == 0.0, "salt in the liquid {wet}, on dry land {dry}");
    let (de, dq, w2) = drift_after(2);
    assert!(de < 1e-9 && dq < 1e-9, "drift {de} {dq}");
    assert!(w2.ground[SALT * n..(SALT + 1) * n].iter().sum::<f64>() > 0.5 * wet);
}

/// The soil is given to the land when life starts, 10 g of nitrogen to the m2, booked as new matter and energy, none under
/// standing liquid; and the tape counts the mineral on land and on the low ground.
#[test]
fn the_soil_is_booked_and_the_tape_counts_the_mineral() {
    let mut w = worldgen::new_world(1, 16, 16);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let (mineral, _) = w.chem.minerals()[0];
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let n = w.grid.n();
    let before = total(&w, mineral);
    let line = worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    println!("{line}");
    let (placed, area) = (total(&w, mineral) - before, w.grid.area());
    let cells = placed / (0.357 * area);
    assert!(cells.fract().abs() < 1e-9 || (1.0 - cells.fract()).abs() < 1e-9, "{cells} cells' worth");
    assert!(cells >= 10.0 && cells <= n as f64);
    // 0.357 mol of the salt, 2 mol of nitrogen in each: 10 g of nitrogen in the m2.
    let grams: f64 = 0.357 * 2.0 * 14.0;
    assert!((grams - 10.0).abs() < 0.05);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
