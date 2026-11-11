//! The planet's air for an island that stands in it (laws.md 1.6, `pcore::flow::Planet`): a strip of the planet from
//! pole to pole, `COLUMNS` air blocks round, all of it low open ground, run by the laws from its starting state for
//! `YEARS` years, the air of its last year by row and month.

use crate::chemistry::chemistry0;
use crate::terrain::{build_on, coarsen, soft_of, AIR_SCALE, RADIUS_M};
use pcore::flow::Planet;
use pcore::space::square::Square;
use pcore::space::terrain::Terrain;
use pcore::{Config, World};
use std::f64::consts::PI;

/// Squares of the strip from pole to pole: rows of air blocks of 5° of latitude.
const ROWS: usize = 144;
/// Air blocks round the strip, each one cell: the fewest a globe's row may have.
const COLUMNS: usize = 3;
/// The rise at which the strip's ground stands at 0 m: above any rise of the noise (`space::terrain::rise`, under 1),
/// so all of it is low open ground.
const ALL_LOW: f64 = 1.0;
/// Years the strip runs: its last is the table's.
const YEARS: u64 = 20;

/// The island `w` stands in the planet's air: its map's edges open onto the air of a strip of the planet of its seed
/// (`planet`).
pub fn stand(w: &mut World) {
    w.planet = Some(planet(w.seed));
}

/// The planet's air (`Planet`) from a strip of the planet of `seed` run `YEARS` years.
pub fn planet(seed: u64) -> Planet {
    let (chem, init) = chemistry0();
    let map = coarsen(Square::new(COLUMNS * AIR_SCALE, ROWS, PI * RADIUS_M / ROWS as f64, true, AIR_SCALE), |_, _| false);
    let cfg = Config { lat_south: -90.0, lat_north: 90.0, ..Config::default() };
    let soft = soft_of(&chem, &init);
    let mut w = build_on(seed, map, Terrain::new(seed, ALL_LOW, soft), (chem, &init), cfg);
    pcore::flow::state(&mut w, true);
    let months = w.cfg.months_per_year as usize;
    for _ in 0..(YEARS - 1) * months as u64 {
        pcore::step(&mut w);
    }
    // Per month of the last year, per row: its air (`Planet::month_of`) and its shares.
    let (mut lat, mut each) = (Vec::new(), Vec::with_capacity(months));
    for _ in 0..months {
        pcore::step(&mut w);
        let (rows, air, shares) = Planet::month_of(&w);
        lat = rows;
        each.push((air, shares));
    }
    let (rows, ns) = (lat.len(), each[0].1[0].len());
    // Per month a row's value, each row the 1-2-1 mean of it and its neighbours (`smooth`).
    let of = |v: &dyn Fn(usize, usize) -> f64| each.iter().enumerate().map(|(m, _)| smooth(&(0..rows).map(|r| v(m, r)).collect::<Vec<f64>>())).collect::<Vec<_>>();
    let air: Vec<Vec<Vec<f64>>> = (0..3).map(|c| of(&|m, r| each[m].0[r][c])).collect();
    let shares: Vec<Vec<Vec<f64>>> = (0..ns).map(|k| of(&|m, r| each[m].1[r][k])).collect();
    let table = |v: &[Vec<f64>]| (0..rows).flat_map(|r| (0..months).map(move |m| v[m][r])).collect::<Vec<f64>>();
    let share = (0..rows).flat_map(|r| (0..months).flat_map(move |m| (0..ns).map(move |k| (r, m, k)))).map(|(r, m, k)| shares[k][m][r]).collect();
    let of = |c: usize| table(&air[c]);
    Planet { lat, row_m: RADIUS_M * (180.0 / rows as f64).to_radians(), months, t: of(0), warmth: of(1), mol: of(2), share }
}

/// The 1-2-1 mean of each row of `v` and the rows beside it (the poles' rows by themselves beyond them): the strip's
/// rows of air alternate warmer and colder in the low latitudes (a mode two rows long of the model's air, as on a whole
/// globe of the same rows), which the island's edges read as the planet's mean.
fn smooth(v: &[f64]) -> Vec<f64> {
    let n = v.len();
    (0..n).map(|r| (v[r.saturating_sub(1)] + 2.0 * v[r] + v[(r + 1).min(n - 1)]) / 4.0).collect()
}
