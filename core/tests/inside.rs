//! Inside a cell (laws.md 0, `pcore::inside`): the liquid in a cell's soil lies over its ground by how much liquid
//! gathers at each point, and keeps the cell's mean exactly.

use pcore::inside::{soil_held, soil_level, Means};
use pcore::Physics;

/// The liquid in its pores, its pores' room and what stands, m: a cell half wet, some liquid standing.
const MEANS: Means = Means { pore: 0.08, pores: 0.2, stands: 0.05 };

/// The soil's liquid over points of a wide spread of wetness index (from a ridge to a hollow) and unequal areas: its
/// mean by area is the cell's, to the rounding of a sum; no point holds less than none or more than its pores; and a
/// point where more gathers holds no less.
#[test]
fn the_soil_keeps_its_mean_over_a_cell() {
    let ph = Physics::default();
    let twi: Vec<f64> = (0..64).map(|p| -2.0 + 0.25 * p as f64).collect();
    let raw: Vec<f64> = (0..64).map(|p| 1.0 + (p % 5) as f64).collect();
    let area: Vec<f64> = raw.iter().map(|a| a / raw.iter().sum::<f64>()).collect();
    let c = soil_level(&ph, &twi, &area, MEANS).expect("pores neither dry nor full");
    let held: Vec<f64> = twi.iter().map(|&t| soil_held(&ph, MEANS, t, c)).collect();
    let mean: f64 = held.iter().zip(&area).map(|(h, a)| h * a).sum();
    assert!((mean - MEANS.pore).abs() < 1e-12, "mean {mean}, the cell's {}", MEANS.pore);
    assert!(held.iter().all(|&h| (0.0..=MEANS.pores).contains(&h)), "within none and full pores");
    assert!(held.windows(2).all(|p| p[1] >= p[0]), "more where more gathers");
}
