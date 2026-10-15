//! Erosion at real rates (15 Oct 2026: `carry`, `wear` and `creep` anchored, not tuned; `docs/numbers.md`).
//! Creep is hillslope diffusion with a measured diffusivity (3e-3 m²/yr) for the starting loose ground; liquid carries off and breaks
//! the ground at the measured ratio of the sediment of running liquid to the potential energy of their runoff (2e-11 m³/J); and a small
//! island, run for years, loses ground at a rate in the real range (0.01 to 1 mm a year) and not 10 mm.

use pcore::chem::Phase;
use pcore::World;

/// Thickness of rock-making solids (those that melt above 1000 K) in the loose ground of cell `i` and bed below it, m, and the
/// mean softness (1 / (hardness + 0.05)) of the loose ones, by volume.
fn ground_of(w: &World, i: usize) -> (f64, f64, f64) {
    let (n, area) = (w.grid.n(), w.grid.area());
    let (mut loose, mut bed, mut soft) = (0.0, 0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.chem.subs[k].melt > 1000.0) {
        let v = w.chem.molar_volume(k, Phase::Solid);
        loose += w.ground[k * n + i] * v / area;
        bed += w.bed[k * n + i] * v / area;
        soft += w.ground[k * n + i] * v / area / (w.chem.subs[k].hardness + 0.05);
    }
    (loose, bed, soft / loose.max(1e-300))
}

/// The diffusivity of the starting loose ground is the measured one: creep x softness x thickness = 3e-3 m²/yr (hillslope soil
/// diffusivities are 1e-3 to 1e-2 m²/yr), within a tenth.
#[test]
fn the_starting_ground_creeps_at_a_measured_diffusivity() {
    let w = worldgen::new_world(1, 16, 16);
    let cell = (0..w.grid.n()).find(|&i| w.surface_z(i) > 0.0).expect("land");
    let (h, _, soft) = ground_of(&w, cell);
    let d = w.cfg.creep * soft * h * w.days_per_year() as f64 * 86_400.0;
    println!("loose ground {h:.2} m thick, softness {soft:.2}: diffusivity {d:.2e} m2/yr");
    assert!(h > 0.5 && h < 1.5, "{h} m of loose ground");
    assert!(d > 2.7e-3 && d < 3.3e-3, "{d} m2/yr");
}

/// The two rates of liquid are the measured yield: the sediment that running liquid carries to the low ground, about 15 to 20 Gt a year, is 7e9 m³, and the potential
/// energy of their runoff, 4e13 m³ a year falling 840 m, is 3.4e20 J: 2e-11 m³ per J. A bed of the starting rock (bond strength 0.7)
/// breaks at the same.
#[test]
fn liquid_carries_and_breaks_at_the_measured_yield_per_joule() {
    let w = worldgen::new_world(1, 16, 16);
    let yield_per_j = 7e9 / (4e13 * 840.0 * 1000.0 * 9.81);
    println!("measured yield {yield_per_j:.2e} m3/J; carry {:.1e}, wear {:.1e}", w.cfg.carry, w.cfg.wear);
    assert!((w.cfg.carry / yield_per_j - 1.0).abs() < 0.2, "carry {}", w.cfg.carry);
    // The bed breaks at wear / strength for each substance, by its share of the bed's volume.
    let (n, area) = (w.grid.n(), w.grid.area());
    let cell = (0..n).find(|&i| w.surface_z(i) > 0.0).expect("land");
    let (mut vol, mut inv) = (0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.bed[k * n + cell] > 0.0) {
        let v = w.bed[k * n + cell] * w.chem.molar_volume(k, Phase::Solid) / area;
        vol += v;
        inv += v / w.chem.subs[k].strength.max(0.05);
    }
    let per_j = w.cfg.wear * inv / vol;
    println!("bed breaks at {per_j:.2e} m3/J");
    assert!((per_j / yield_per_j - 1.0).abs() < 0.2, "{per_j}");
}

/// A small island, ten years: the land loses rock (the rock-making solids of its loose ground and bed, over its cells) at the real rate
/// of denudation, 0.01 to 1 mm a year, and not the 10 mm a year it lost when the numbers were first guesses.
#[test]
fn an_island_loses_ground_at_a_real_rate() {
    let mut w = worldgen::new_world(1, 32, 32);
    let land: Vec<usize> = (0..w.grid.n()).filter(|&i| w.surface_z(i) > 0.0).collect();
    let rock = |w: &World| land.iter().map(|&i| ground_of(w, i)).map(|g| g.0 + g.1).sum::<f64>() / land.len() as f64;
    let before = rock(&w);
    let years = 10;
    for _ in 0..12 * years {
        pcore::step(&mut w);
    }
    let rate = (before - rock(&w)) / years as f64 * 1000.0;
    println!("{} land cells lost {rate:.4} mm a year of rock", land.len());
    assert!(rate > 0.003 && rate < 1.0, "{rate} mm a year");
}
