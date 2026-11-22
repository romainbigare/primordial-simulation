//! The planet's air for an island that stands in it (laws.md 1.6, `pcore::flow::Planet`): a strip of the planet from
//! pole to pole, `COLUMNS` air blocks round, all of it low open ground, run by the laws from its starting state for
//! `YEARS` years, the air of its last year by row and month (`planet`). It reads only chemistry 0, the default laws and
//! its own seed, so one table serves every island: the build ships it (`table`, written by `ew planet`), and a test
//! holds it to the strip run again, bit for bit.

use crate::chemistry::chemistry0;
use crate::terrain::{build_on, coarsen, makes_of, AIR_SCALE};
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
/// Years the strip runs: its last is the table's. An island of 32 x 32 on the band 20° to 65° feels a table of 20
/// years within 0.2 K in every row.
const YEARS: u64 = 10;
/// The seed of the strip's terrain: its ground is all low, so the seed shapes only its depth, and one strip serves every
/// island of one chemistry.
const SEED: u64 = 1;

/// The island `w` stands in the planet's air: its map's edges open onto the air of a strip of the planet (`table`).
pub fn stand(w: &mut World) {
    w.planet = Some(table());
}

/// The file the build ships the planet's table in (`ew planet` writes it).
pub const TABLE_FILE: &str = "worldgen/src/planet.bin";

/// The planet's table (`planet`) as the build ships it (`TABLE_FILE`).
pub fn table() -> Planet {
    bincode::deserialize(include_bytes!("planet.bin")).expect("the planet's table reads (`ew planet` writes it)")
}

/// The bytes of a planet's table as the build ships it (bincode).
pub fn table_bytes(p: &Planet) -> Vec<u8> {
    bincode::serialize(p).expect("a planet's table serializes")
}

/// The planet's air (`Planet`) from the strip of the planet run `YEARS` years.
pub fn planet() -> Planet {
    let ((chem, init), seed) = (chemistry0(), SEED);
    let cfg = Config { lat_south: -90.0, lat_north: 90.0, ..Config::default() };
    let map = coarsen(Square::new(COLUMNS * AIR_SCALE, ROWS, PI * cfg.radius_m / ROWS as f64, true, AIR_SCALE), |_, _| false);
    let makes = makes_of(&chem, &init);
    let mut w = build_on(seed, map, Terrain::new(seed, ALL_LOW, makes, &cfg.phys), (chem, &init), cfg);
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
    // Per month each row's value.
    let of = |v: &dyn Fn(usize, usize) -> f64| (0..months).map(|m| (0..rows).map(|r| v(m, r)).collect::<Vec<f64>>()).collect::<Vec<_>>();
    let air: Vec<Vec<Vec<f64>>> = (0..3).map(|c| of(&|m, r| each[m].0[r][c])).collect();
    let shares: Vec<Vec<Vec<f64>>> = (0..ns).map(|k| of(&|m, r| each[m].1[r][k])).collect();
    let table = |v: &[Vec<f64>]| (0..rows).flat_map(|r| (0..months).map(move |m| v[m][r])).collect::<Vec<f64>>();
    let share = (0..rows).flat_map(|r| (0..months).flat_map(move |m| (0..ns).map(move |k| (r, m, k)))).map(|(r, m, k)| shares[k][m][r]).collect();
    let of = |c: usize| table(&air[c]);
    Planet { lat, row_m: w.cfg.radius_m * (180.0 / rows as f64).to_radians(), months, t: of(0), warmth: of(1), mol: of(2), share }
}

#[cfg(test)]
mod tests {
    /// The table the build ships is the strip run again now, bit for bit: a change of the laws or of chemistry 0 that
    /// moves it fails here (a few seconds).
    #[test]
    fn the_shipped_table_is_the_planet_now() {
        let now = super::table_bytes(&super::planet());
        assert!(now == include_bytes!("planet.bin"), "the planet's table is stale: run `cargo run --release -p cli --bin ew -- planet` (it writes {})", super::TABLE_FILE);
    }
}
