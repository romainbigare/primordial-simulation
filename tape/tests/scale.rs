//! Coarse against fine: the laws do not depend on the size of the cells. The same island of 32 x 32 squares of 2 km,
//! once on its squares and once with every square in quarters (64 x 64 cells of 1 km, a refined region over the whole
//! map), each seed against itself: the mean over the seeds of what the quarters move each key number per area (the
//! mean of its last years) is less than its year-to-year spread on the squares (the mean over the seeds of its highest
//! less its lowest yearly value). The length of the rivers is printed, not held: it depends on the size of the cells
//! through the routing of liquid (decision 31).

mod common;

use common::run_on;

const SEEDS: [u64; 3] = [1, 2, 3];
const SIDE: usize = 32;
const YEARS: u64 = 15;
/// The numbers are the means of the years from this one to `YEARS`.
const FROM: u64 = 6;
/// The key numbers: the land's ground temperature, its warmest and coldest month, rain on land, the heat-absorbing gas,
/// the light-eaters' mass and members (the first `HELD`, held to the noise), and the length of the rivers, km (river
/// cells times the cells' side).
const KEYS: [&str; 8] = ["t_land", "t_land_summer", "t_land_winter", "rain_land", "air_3", "root_0_kg", "plant_members", "river_km"];
const HELD: usize = 7;

/// The `KEYS` at the end of each year from `FROM` of the island of `seed`, on its squares or in quarters (`fine`).
fn keys(seed: u64, fine: bool) -> Vec<[f64; KEYS.len()]> {
    let mut map = worldgen::terrain::island(SIDE, SIDE);
    map.region = fine.then_some([0, 0, SIDE, SIDE]);
    let km = map.side / if fine { 2.0 } else { 1.0 } / 1e3;
    let mut years = vec![];
    run_on(worldgen::terrain::new_map(seed, map), YEARS, false, |_| {}, |y, _, rec| {
        if y >= FROM {
            years.push(KEYS.map(|c| if c == "river_km" { rec.last("rivers") * km } else { rec.last(c) }));
        }
    });
    years
}

#[test]
#[ignore = "slow: three 15-year runs of a 32 x 32 island with life, and three of it in quarters"]
fn the_laws_do_not_depend_on_the_size_of_the_cells() {
    // Per seed, [squares, quarters], all side by side.
    let runs: Vec<[Vec<[f64; KEYS.len()]>; 2]> = std::thread::scope(|s| {
        let runs: Vec<_> = SEEDS.iter().map(|&seed| [false, true].map(|fine| s.spawn(move || keys(seed, fine)))).collect();
        runs.into_iter().map(|pair| pair.map(|r| r.join().expect("a run"))).collect()
    });
    let seeds = runs.len() as f64;
    let mut out = vec![];
    for (q, name) in KEYS.iter().enumerate() {
        let mean = |run: &[[f64; KEYS.len()]]| run.iter().map(|y| y[q]).sum::<f64>() / run.len() as f64;
        let spread = |run: &[[f64; KEYS.len()]]| run.iter().map(|y| y[q]).fold(f64::MIN, f64::max) - run.iter().map(|y| y[q]).fold(f64::MAX, f64::min);
        let offs: Vec<f64> = runs.iter().map(|[squares, quarters]| mean(quarters) - mean(squares)).collect();
        let (off, limit) = (offs.iter().sum::<f64>() / seeds, runs.iter().map(|[squares, _]| spread(squares)).sum::<f64>() / seeds);
        let ok = off.abs() <= limit || q >= HELD;
        let squares: Vec<String> = runs.iter().map(|[squares, _]| format!("{:.4e}", mean(squares))).collect();
        let by_seed: Vec<String> = offs.iter().map(|x| format!("{x:.3e}")).collect();
        println!("{name}: off by {off:.3e} (seeds {by_seed:?}) in quarters from the squares (seeds {squares:?}), limit {limit:.3e}{}", if off.abs() <= limit { "" } else if ok { ": out, not held" } else { ": OUT" });
        if !ok {
            out.push(*name);
        }
    }
    assert!(out.is_empty(), "the size of the cells changed: {out:?}");
}
