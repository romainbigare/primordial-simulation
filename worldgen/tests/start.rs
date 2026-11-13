//! The starting state worldgen makes: coarse cells only under the low liquid, the dissolved mineral in the low ground's
//! liquid, and the soil given when life starts.

use pcore::AIR;
use pcore::store::Class;
use worldgen::testkit::{in_all, DISSOLVED, LIQUID};

/// An island's coarse cells (more than one square) all lie below 0 m, in its low basin's liquid, on islands of five
/// seeds.
#[test]
fn coarse_cells_lie_under_the_low_liquid() {
    for seed in 1..=5 {
        let w = worldgen::testkit::one_rise(seed, 32, 32);
        let cells = w.mesh.cells();
        let coarse: Vec<usize> = (0..cells.n()).filter(|&i| cells.area()[i] > 1.5 * w.map.side * w.map.side).collect();
        assert!(!coarse.is_empty(), "seed {seed}: coarse cells");
        for i in coarse {
            assert!(w.surface_z(i) < 0.0 && !w.is_land(i), "seed {seed}: coarse cell {i} at {} m", w.surface_z(i));
        }
    }
}

/// The world starts with the dissolved mineral in the liquid of its low ground and none on dry land.
#[test]
fn a_world_starts_with_the_dissolved_mineral_in_its_liquid() {
    let w = worldgen::testkit::one_rise(1, 24, 24);
    let n = w.mesh.cells().n();
    let (mut wet, mut dry) = (0.0, 0.0);
    for i in 0..n {
        if w.store.amounts(Class::Ground)[LIQUID * n + i] > 0.0 {
            assert!(w.store.amounts(Class::Ground)[DISSOLVED * n + i] > 0.0, "cell {i} has liquid and none of the mineral");
            wet += w.store.amounts(Class::Ground)[DISSOLVED * n + i];
        } else {
            dry += w.store.amounts(Class::Ground)[DISSOLVED * n + i];
        }
    }
    assert!(wet > 0.0 && dry == 0.0, "the mineral in the liquid {wet}, on dry land {dry}");
}

/// The soil is given to the land when life starts: `Init::soil` mol per m² of the fixed gas's mineral (10 g of the
/// gas's element, 14 g a mol, to the m²) in whole cells of land, booked as new matter and energy.
#[test]
fn the_soil_is_given_to_the_land_and_booked() {
    let mut w = worldgen::testkit::one_rise(1, 16, 16);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let (mineral, _) = w.chem.minerals()[0];
    let soil = worldgen::chemistry0().1.soil;
    let t0 = pcore::check::totals(&w);
    let before = in_all(&w, mineral);
    println!("{}", worldgen::life::soil(&mut w, &soil));
    let (k, per_m2) = soil[0];
    let cells = (in_all(&w, mineral) - before) / (per_m2 * w.mesh.cells().area()[0]);
    assert!(k == mineral && (cells - cells.round()).abs() < 1e-9 && cells >= 10.0 && cells <= w.mesh.cells().n() as f64, "{cells} cells' worth");
    let grams = per_m2 * w.chem.subs[k].elems[2] as f64 * 14.0;
    assert!((grams - 10.0).abs() < 0.05, "{grams} g of the gas's element to the m²");
    let (de, dq) = pcore::check::drift(&w, &t0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// The air starts lying as Flow's law has it: each air node's column, against the column at height 0 (`terrain::air_column`), is thinner
/// by exp(-z / H) over its cells' mean height z by area, H the scale height of its own air (`flow::scale_height`, from its
/// temperature and the mean molar mass of its gases).
#[test]
fn the_starting_air_thins_with_height_by_its_own_scale_height() {
    let w = worldgen::testkit::one_rise(1, 24, 24);
    let (chem, init) = worldgen::chemistry0();
    let (n, na, g) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.cfg.gravity);
    let column = worldgen::terrain::air_column(&chem, &init, g);
    let (mut z, air_area) = (vec![0.0; na], w.mesh.level(AIR).area());
    for i in 0..n {
        z[w.mesh.up(i, 0, AIR)] += w.surface_z(i).max(0.0) * w.mesh.cells().area()[i];
    }
    let mut high = 0;
    for j in 0..na {
        let (mol, kg) = (0..w.chem.ns()).fold((0.0, 0.0), |(m, kg), k| (m + w.store.amounts(Class::Air)[k * na + j], kg + w.store.amounts(Class::Air)[k * na + j] * w.chem.subs[k].molar_mass));
        let (zj, h) = (z[j] / air_area[j], pcore::flow::scale_height(w.t_air(j), mol, kg, g));
        let ratio = mol / (column * air_area[j]);
        assert!((ratio / (-zj / h).exp() - 1.0).abs() < 1e-6, "node {j}: {ratio} of the column at height 0, at {zj} m, scale height {h} m");
        high += (zj > 500.0) as usize;
    }
    assert!(high > 0, "no air node over high ground");
}

/// Each dry cell's bed and loose ground add up to its height, the terrain's mean over it, and its loose ground is the
/// mean of what the wear left on its samples (`Samples::loose`), the top `skin_m` of each at most. (A cell that holds
/// the starting liquid, of the low basin or a body, is left out: the liquid at balance with the air dissolves some of
/// its loose ground, laws.md 2.5; an eighth of the cells or more are dry.)
#[test]
fn a_cells_bed_and_loose_ground_add_up_to_its_height() {
    let w = worldgen::testkit::one_rise(1, 16, 16);
    let (n, lat, seed) = (w.mesh.cells().n(), (w.cfg.lat_south, w.cfg.lat_north), w.terrain.seed);
    let dry: Vec<usize> = (0..n).filter(|&i| w.store.amounts(Class::Ground)[LIQUID * n + i] == 0.0).collect();
    assert!(dry.len() > n / 8, "{} dry cells of {n}", dry.len());
    for i in dry {
        let l = w.map.leaves()[i];
        let (loose, bed, _) = worldgen::testkit::ground_of(&w, i);
        let (z, left) = (w.terrain.inside(seed, &w.map, l, lat), w.terrain.over(seed, &w.map, l, lat, |s, k| s.loose[k].min(w.cfg.skin_m)));
        assert!((w.base_z + bed + loose - z).abs() < 1e-6, "cell {i}: bed {bed} m and loose {loose} m over {} m, at {z} m", w.base_z);
        assert!((loose - left).abs() < 1e-9 * left.max(1.0), "cell {i}: {loose} m of loose ground, the wear left {left} m");
    }
}
