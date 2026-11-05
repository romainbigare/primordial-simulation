//! Coarse against fine: the laws do not depend on the size of the cells. The same island of 64 x 64 squares of 1 km,
//! once in cells of 2 x 2 squares (32 x 32 cells of 2 km) and once on its squares, each seed against itself: the mean
//! over the seeds of what the squares move each key number per area (the mean of its last years) is less than its
//! year-to-year spread in the coarse cells (the mean over the seeds of its highest less its lowest yearly value). The length of the rivers is printed, not held: it still moves by more than its noise
//! (decision 31). And the life of a roster island whose region splits while it runs stays within the noise of the
//! unsplit island, its bodies stepping four times a month there (`life_does_not_depend_on_a_split_region`; the life in
//! the region itself is printed).

mod common;

use common::run_on;
use pcore::World;

const SEEDS: [u64; 3] = [1, 2, 3];
const SIDE: usize = 32;
const YEARS: u64 = 15;
/// The numbers are the means of the years from this one to `YEARS`.
const FROM: u64 = 6;
/// The key numbers: the land's ground temperature, its warmest and coldest month, rain on land, the heat-absorbing gas,
/// the cover's mass and the starting cover's capture area, m² (the first `HELD`, held to the noise), and the length of
/// the rivers, km (river cells times the cells' side).
const KEYS: [&str; 8] = ["t_land", "t_land_summer", "t_land_winter", "rain_land", "air_3", "cover_kg", "root_0_cover_area", "river_km"];
const HELD: usize = 7;

/// The `KEYS` at the end of each year from `FROM` of the island of `seed`, in cells of 2 x 2 squares or on its
/// squares (`fine`).
fn keys(seed: u64, fine: bool) -> Vec<[f64; KEYS.len()]> {
    let island = worldgen::terrain::island(2 * SIDE, 2 * SIDE);
    let map = pcore::space::square::Square::new(island.w, island.h, island.side / 2.0, false, island.scale);
    let square = map.square_depth();
    let map = map.by_blocks(|_, _| if fine { square } else { square - 1 });
    let km = map.side * if fine { 1.0 } else { 2.0 } / 1e3;
    let mut years = vec![];
    run_on(worldgen::terrain::new_map(seed, map), YEARS, false, |_| {}, |y, _, rec| {
        if y >= FROM {
            years.push(KEYS.map(|c| if c == "river_km" { rec.last("rivers") * km } else { rec.last(c) }));
        }
    });
    years
}

#[test]
#[ignore = "slow: three 15-year runs of a 32 x 32 island with life, and three of it on 64 x 64 squares"]
fn the_laws_do_not_depend_on_the_size_of_the_cells() {
    // Per seed, [coarse, fine], all side by side.
    let runs: Vec<[Vec<[f64; KEYS.len()]>; 2]> = std::thread::scope(|s| {
        let runs: Vec<_> = SEEDS.iter().map(|&seed| [false, true].map(|fine| s.spawn(move || keys(seed, fine)))).collect();
        runs.into_iter().map(|pair| pair.map(|r| r.join().expect("a run"))).collect()
    });
    let seeds = runs.len() as f64;
    let mut out = vec![];
    for (q, name) in KEYS.iter().enumerate() {
        let mean = |run: &[[f64; KEYS.len()]]| run.iter().map(|y| y[q]).sum::<f64>() / run.len() as f64;
        let spread = |run: &[[f64; KEYS.len()]]| run.iter().map(|y| y[q]).fold(f64::MIN, f64::max) - run.iter().map(|y| y[q]).fold(f64::MAX, f64::min);
        let offs: Vec<f64> = runs.iter().map(|[coarse, fine]| mean(fine) - mean(coarse)).collect();
        let (off, limit) = (offs.iter().sum::<f64>() / seeds, runs.iter().map(|[coarse, _]| spread(coarse)).sum::<f64>() / seeds);
        let ok = off.abs() <= limit || q >= HELD;
        let coarse: Vec<String> = runs.iter().map(|[coarse, _]| format!("{:.4e}", mean(coarse))).collect();
        let by_seed: Vec<String> = offs.iter().map(|x| format!("{x:.3e}")).collect();
        println!("{name}: off by {off:.3e} (seeds {by_seed:?}) on squares from the coarse cells (seeds {coarse:?}), limit {limit:.3e}{}", if off.abs() <= limit { "" } else if ok { ": out, not held" } else { ": OUT" });
        if !ok {
            out.push(*name);
        }
    }
    assert!(out.is_empty(), "the size of the cells changed: {out:?}");
}

/// The roster island of the check of a split region (spec 5.2 for life): its side in cells, its years, the year at
/// whose end its region splits, and the seeds, each run unsplit and split, whose unsplit runs measure the noise: the
/// range of their means (two seeds give one gap, which two runs that happen to agree make no band at all).
const REGION_SIDE: usize = 32;
const REGION_YEARS: u64 = 50;
const SPLIT_AT: u64 = 25;
const NOISE_SEEDS: [u64; 4] = [1, 2, 3, 4];
/// The numbers compared, each the mean of the years after the split: the animals' mass, kg, the cover's, kg, and the
/// starting kinds alive (in bodies or cover), the first `LIFE_HELD`, held to the noise; then in the region (where its
/// bodies step four times a month once it is split) the animals' mass, kg, their members and their mean temperature by
/// members, K, printed, not held: the region is one air node of a small island, whose seeds hold animals fifty times
/// apart there, so the range of its seeds is no band. One body law at every grain is held where a node lives
/// on: `core/tests/steps.rs`.
const LIFE: [&str; 6] = ["animal_kg", "cover_kg", "kinds_alive", "region_animal_kg", "region_members", "region_t"];
const LIFE_HELD: usize = 3;

/// The `LIFE` numbers of `w`, whose region is the cells inside the leaves `region`.
fn life(w: &World, region: &[pcore::space::Leaf]) -> [f64; LIFE.len()] {
    let inside = |i: usize| {
        let l = w.map.leaves()[i];
        region.iter().any(|r| l.x >= r.x && l.x < r.x + r.side() && l.y >= r.y && l.y < r.y + r.side())
    };
    let animals: f64 = w.life.alive().map(|b| b.mass(&w.chem)).sum();
    let here: Vec<&pcore::body::Body> = w.life.alive().filter(|b| inside(b.cell as usize)).collect();
    let members: f64 = here.iter().map(|b| b.n()).sum();
    let t = here.iter().map(|b| b.temp(&w.chem) * b.n()).sum::<f64>() / members.max(f64::MIN_POSITIVE);
    let cover: f64 = pcore::cover::sums(w).iter().map(|s| s.kg).sum();
    let mut kinds: std::collections::BTreeSet<u32> = w.life.alive().filter_map(|b| w.life.brain(b.lineage).map(|l| l.root)).collect();
    kinds.extend(pcore::cover::slots(w).map(|s| s.0));
    [animals, cover, kinds.len() as f64, here.iter().map(|b| b.mass(&w.chem)).sum(), members, t]
}

/// The `LIFE` numbers at the end of each year after `SPLIT_AT` of the roster island of `seed`, its region (the cells of
/// its first air node over land only) split two depths at the end of year `SPLIT_AT` if `split`.
fn life_years(seed: u64, split: bool) -> Vec<[f64; LIFE.len()]> {
    let w = worldgen::new_world(seed, REGION_SIDE, REGION_SIDE);
    let (region, at) = (worldgen::testkit::land_block(&w), SPLIT_AT * w.cfg.calendar().year);
    let cut = region.clone();
    let step = move |w: &mut World| {
        if split && w.time == at {
            worldgen::testkit::split_down(w, &cut, 2);
        }
        pcore::step(w);
    };
    let mut years = vec![];
    common::run_stepped(w, REGION_YEARS, true, |_| {}, step, |y, w, _| {
        if y > SPLIT_AT {
            years.push(life(w, &region));
        }
    });
    years
}

/// Spec 5.2 for life: a roster island whose region of one air node splits two depths (its cells a quarter of the side
/// of the others, its bodies stepping four times a month) at the end of year `SPLIT_AT` keeps its animals' mass, its
/// cover's mass and its kinds alive (the means of the years after) within the noise of the unsplit island: the mean over
/// the seeds of what the split moves each number by is less than the range of the unsplit means of the seeds (their
/// highest less their lowest).
/// The life in the region is printed (`LIFE`).
#[test]
#[ignore = "slow: eight 50-year roster runs of a 32 x 32 island, four with a region split two depths at year 25"]
fn life_does_not_depend_on_a_split_region() {
    let [unsplit, split] = std::thread::scope(|s| {
        let runs = [false, true].map(|cut| NOISE_SEEDS.map(|seed| s.spawn(move || life_years(seed, cut))));
        runs.map(|r| r.map(|r| r.join().expect("a run")))
    });
    let mut out = vec![];
    for (q, name) in LIFE.iter().enumerate() {
        let mean = |run: &[[f64; LIFE.len()]]| run.iter().map(|y| y[q]).sum::<f64>() / run.len() as f64;
        let (base, cut) = (unsplit.each_ref().map(|r| mean(r)), split.each_ref().map(|r| mean(r)));
        let offs: Vec<f64> = cut.iter().zip(&base).map(|(c, u)| c - u).collect();
        let off = offs.iter().sum::<f64>() / offs.len() as f64;
        let noise = base.iter().cloned().fold(f64::MIN, f64::max) - base.iter().cloned().fold(f64::MAX, f64::min);
        let held = if q >= LIFE_HELD { ", not held" } else if off.abs() > noise { ": OUT" } else { "" };
        let list = |v: &[f64]| v.iter().map(|x| format!("{x:.4e}")).collect::<Vec<_>>().join(", ");
        println!("{name}: unsplit [{}] (seeds {NOISE_SEEDS:?}), split [{}], off by {off:.3e} ([{}]); noise {noise:.3e}{held}", list(&base), list(&cut), list(&offs));
        if q < LIFE_HELD && off.abs() > noise {
            out.push(*name);
        }
    }
    assert!(out.is_empty(), "the split region changed the life: {out:?}");
}
