//! The key numbers of a normal run: `ew run --seed N --size 64` for seeds 1, 2 and 3, 30 years each, on threads. The
//! mean of each number over the three seeds must lie in its range around its reference, the mean of the same three
//! seeds in the reference runs: for the climate and the land worn down, the runs of the converged world (the air's
//! implicit step at its default); for life, the A2 runs (`docs/reports/foundation-a/`). The climate and the land worn
//! down are means over years 21 to 30; life is read at year 30, ten years after the eaters came. Each run also keeps
//! its books.
//!
//! The sleep of Change's nodes is a speed tool: a small world with life keeps the chemistry's key numbers
//! within their noise with every node awake every step (`Config::sleep_days` 0), and with the setting x2 and /2. So is
//! the pace of population nodes: the same world keeps its life's key numbers with every node at its own `INTERVAL`
//! gene (`Config::node_days` 0), and with the setting x2 and /2.
//!
//! The steps of Flow are speed settings too (the long step against the short one): small worlds with life keep
//! their climate within the seeds' noise at the default steps and with the air's step at 6 hours and the physics step
//! at 3 days.

mod common;

use common::run;
use pcore::{Config, World};
use std::fmt;
use worldgen::testkit::{ground_of, ABSORBER};

const SEEDS: [u64; 3] = [1, 2, 3];
const SIDE: usize = 64;
const YEARS: u64 = 30;
/// The climate is the mean of the years from this one to `YEARS`; the land worn down is over the same years.
const FROM: u64 = 21;
/// Parts per million in one; millimetres in a metre.
const PPM: f64 = 1e6;
const MM: f64 = 1e3;

/// Ground temperature on land, K (series `t_land`), in the reference runs of the converged world.
const T_LAND: f64 = 288.45;
/// Seasonal swing on land, K (`t_land_summer` less `t_land_winter`), in the reference runs of the converged world.
const SWING: f64 = 12.68;
/// Rain on land, mol/m²/s (`rain_land`, a moving average over about a year), in the reference runs of the converged
/// world.
const RAIN_LAND: f64 = 1.842e-3;
/// The heat-absorbing gas of the air, ppm (`air_<ABSORBER>`), in the reference runs of the converged world.
const ABSORBER_PPM: f64 = 271.2;
/// Dry land cells with a liquid flux over the web viewer's threshold (`rivers`), in the reference runs of the converged
/// world.
const RIVERS: f64 = 784.5;
/// Rock lost on the land of year 20, mm/yr (`testkit::ground_of`, loose and bonded), in the reference runs of the
/// converged world.
const WORN: f64 = 0.0717;
/// Light-eaters (starting kind 0) at year 30, log10 kg (`root_0_kg`), in the A2 reference runs.
const PLANT_KG: f64 = 10.533;
/// Eaters (starting kind 1) at year 30, log10 kg (`root_1_kg`), in the A2 reference runs.
const EATER_KG: f64 = 4.497;
/// The life light-eaters live at year 30, yr (`root_0_life_real`: members standing over deaths), in the A2 runs.
const PLANT_LIFE: f64 = 1.895;
/// The factor either way the light-eaters' life may lie from `PLANT_LIFE`: wider than the others' 2, as the A2
/// light-eaters paid for a brain that light-eaters no longer have, so they live longer.
const PLANT_LIFE_RANGE: f64 = 3.0;
/// How far the eaters' mass may lie from `EATER_KG`, log10: wider than the light-eaters' 0.3, as the A2 eaters shared
/// the brain rows of their group and learned slower than a body that learns into its own rows.
const EATER_KG_RANGE: f64 = 0.5;
/// The life eaters live at year 30, yr (`root_1_life_real`), in the A2 reference runs.
const EATER_LIFE: f64 = 2.914;

/// How far a number may lie from its reference: by at most so much, or by at most a factor either way.
#[derive(Clone, Copy)]
enum Range {
    Plus(f64),
    Times(f64),
}

impl Range {
    fn holds(self, value: f64, reference: f64) -> bool {
        match self {
            Range::Plus(d) => (value - reference).abs() <= d,
            Range::Times(f) => (value / reference).ln().abs() <= f.ln(),
        }
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Range::Plus(d) => write!(f, "+-{d}"),
            Range::Times(x) => write!(f, "x/ {x}"),
        }
    }
}

/// The key numbers in the order `key_numbers` gives them: name, unit, reference and range.
const KEYS: [(&str, &str, f64, Range); 10] = [
    ("ground temperature on land", "K", T_LAND, Range::Plus(1.0)),
    ("seasonal swing on land", "K", SWING, Range::Plus(1.5)),
    ("rain on land", "mol/m2/s", RAIN_LAND, Range::Times(1.3)),
    ("heat-absorbing gas", "ppm", ABSORBER_PPM, Range::Plus(40.0)),
    ("river cells", "", RIVERS, Range::Times(1.5)),
    ("land worn down", "mm/yr", WORN, Range::Times(2.0)),
    ("light-eaters", "log10 kg", PLANT_KG, Range::Plus(0.3)),
    ("eaters", "log10 kg", EATER_KG, Range::Plus(EATER_KG_RANGE)),
    ("life of light-eaters", "yr", PLANT_LIFE, Range::Times(PLANT_LIFE_RANGE)),
    ("life of eaters", "yr", EATER_LIFE, Range::Times(2.0)),
];

/// The mean thickness of rock on the cells `land`, m.
fn rock(w: &World, land: &[usize]) -> f64 {
    land.iter().map(|&i| ground_of(w, i)).map(|g| g.0 + g.1).sum::<f64>() / land.len() as f64
}

/// `YEARS` years of `ew run --seed <seed> --size 64`: the key numbers in the order of `KEYS`.
fn key_numbers(seed: u64) -> [f64; KEYS.len()] {
    let gas = format!("air_{ABSORBER}");
    let climate = ["t_land", "t_land_summer", "t_land_winter", "rain_land", gas.as_str(), "rivers"];
    let (mut mean, mut land, mut rock_from) = ([0.0; 6], vec![], 0.0);
    let span = (YEARS - FROM + 1) as f64;
    let (w, rec) = run(seed, SIDE, YEARS, false, |_| {}, |y, w, rec| {
        if y == FROM - 1 {
            land = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).collect();
            rock_from = rock(w, &land);
        }
        if y >= FROM {
            mean.iter_mut().zip(climate).for_each(|(m, c)| *m += rec.last(c) / span);
        }
    });
    let worn = (rock_from - rock(&w, &land)) / span * MM;
    let [t, summer, winter, rain, air, rivers] = mean;
    let (plants, eaters) = (rec.last("root_0_kg").log10(), rec.last("root_1_kg").log10());
    [t, summer - winter, rain, air * PPM, rivers, worn, plants, eaters, rec.last("root_0_life_real"), rec.last("root_1_life_real")]
}

/// The key numbers of three seeds, each mean in its range around its reference.
#[test]
#[ignore = "slow: three 30-year runs of a 64 x 64 world"]
fn the_key_numbers_stay_in_their_ranges() {
    let runs: Vec<[f64; KEYS.len()]> = std::thread::scope(|s| {
        let runs: Vec<_> = SEEDS.iter().map(|&seed| s.spawn(move || key_numbers(seed))).collect();
        runs.into_iter().map(|r| r.join().expect("a run")).collect()
    });
    let mut out = vec![];
    for (k, &(name, unit, reference, range)) in KEYS.iter().enumerate() {
        let seeds: Vec<f64> = runs.iter().map(|r| r[k]).collect();
        let mean = seeds.iter().sum::<f64>() / seeds.len() as f64;
        let ok = range.holds(mean, reference);
        println!("{name}: {mean:.6} {unit} (seeds {seeds:.6?}), reference {reference} {range}{}", if ok { "" } else { ": OUT" });
        if !ok {
            out.push(name);
        }
    }
    assert!(out.is_empty(), "out of their ranges: {out:?}");
}

/// The sleep test's world: its side, its years, and the years from which its numbers are averaged (their values at the
/// end of each year, so at one season).
const SLEEP_SIDE: usize = 32;
const SLEEP_YEARS: u64 = 8;
const SLEEP_FROM: u64 = 5;

/// The chemistry's key numbers (series columns): the heat-absorbing gas (the carbon of the air), the two minerals in the
/// ground of land and of sea (24, the mineral of the fixed gas, and 23, the salt of the rock), and the dead body matter
/// and lime.
const CHEMISTRY: [&str; 7] = ["air_3", "min0_land", "min0_sea", "min1_land", "min1_sea", "dead_22", "dead_10"];

/// The `columns` at the end of each year from `SLEEP_FROM` of the sleep test's world with the settings `set` makes.
fn yearly(columns: &[&str], set: impl Fn(&mut Config)) -> Vec<Vec<f64>> {
    let mut years = vec![];
    run(1, SLEEP_SIDE, SLEEP_YEARS, false, set, |y, _, rec| {
        if y >= SLEEP_FROM {
            years.push(columns.iter().map(|c| rec.last(c)).collect());
        }
    });
    years
}

/// For each of `columns`, the mean over the years of each run of `runs` after the first, against the first's: it must
/// move by less than its noise, the spread of the first run's values over those years. Prints each; returns the
/// columns and settings (named by `name`) that moved more.
fn off_by_noise<S: Copy>(columns: &[&str], runs: &[Vec<Vec<f64>>], settings: &[S], name: impl Fn(S) -> String) -> Vec<String> {
    let mut out = vec![];
    for (q, column) in columns.iter().enumerate() {
        let values = |run: &[Vec<f64>]| run.iter().map(|y| y[q]).collect::<Vec<f64>>();
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let first = values(&runs[0]);
        let noise = first.iter().fold(f64::MIN, |m, &x| m.max(x)) - first.iter().fold(f64::MAX, |m, &x| m.min(x));
        for (run, &set) in runs.iter().zip(settings).skip(1) {
            let off = mean(&values(run)) - mean(&first);
            let ok = off.abs() <= noise;
            println!("{column}, {}: {:.6e}, against {:.6e}, off by {off:.3e}, noise {noise:.3e}{}", name(set), mean(&values(run)), mean(&first), if ok { "" } else { ": OUT" });
            if !ok {
                out.push(format!("{column}, {}", name(set)));
            }
        }
    }
    out
}

/// Sleep against no sleep, and each speed setting of the sleep x2 and /2 (`docs/numbers.md`, speed settings): the
/// longest sleep (`sleep_days`) and the gain that wakes a node (`wake_share`). Against every node awake every step, the
/// mean of each of the chemistry's key numbers over the last years moves by less than its noise, the spread of its
/// values over those years with every node awake.
#[test]
#[ignore = "slow: six 8-year runs of a 32 x 32 world with life"]
fn the_sleep_of_change_keeps_the_chemistry() {
    let (set, share) = (Config::default().sleep_days, Config::default().wake_share);
    let settings = [(0, share), (set, share), (2 * set, share), (set / 2, share), (set, 2.0 * share), (set, share / 2.0)];
    let runs: Vec<Vec<Vec<f64>>> = std::thread::scope(|s| {
        let runs: Vec<_> = settings.iter().map(|&(days, wake)| s.spawn(move || yearly(&CHEMISTRY, |c| (c.sleep_days, c.wake_share) = (days, wake)))).collect();
        runs.into_iter().map(|r| r.join().expect("a run")).collect()
    });
    let out = off_by_noise(&CHEMISTRY, &runs, &settings, |(days, wake)| format!("sleep {days} days, wake at {wake}"));
    assert!(out.is_empty(), "the sleep changed: {out:?}");
}

/// The climate's key numbers (series columns): the ground temperature on land, the warmest and the coldest month's,
/// the rain on land, the heat-absorbing gas and the river cells.
const CLIMATE: [&str; 6] = ["t_land", "t_land_summer", "t_land_winter", "rain_land", "air_3", "rivers"];

/// The means of the `CLIMATE` columns at the end of each year from `SLEEP_FROM`, on the sleep test's world of `seed`,
/// with the steps of Flow that `set` makes.
fn climate(seed: u64, set: fn(&mut Config)) -> [f64; CLIMATE.len()] {
    let mut mean = [0.0; CLIMATE.len()];
    let years = (SLEEP_YEARS - SLEEP_FROM + 1) as f64;
    run(seed, SLEEP_SIDE, SLEEP_YEARS, false, set, |y, _, rec| {
        if y >= SLEEP_FROM {
            mean.iter_mut().zip(CLIMATE).for_each(|(m, c)| *m += rec.last(c) / years);
        }
    });
    mean
}

/// The air's step of 6 hours (`air_hours`), short against the air's changes in a day, and a physics step of 3 days
/// (`phys_days`).
const SHORT_AIR_HOURS: u32 = 6;
const SHORT_PHYS_DAYS: u32 = 3;

/// The long step against the short one: at the default steps of Flow (`air_hours`, `phys_days`, `flow_days`) the mean
/// over the three seeds of each of the climate's key numbers (each the mean of its last years) moves by less than its
/// noise, the spread of the three seeds' values at the short steps (`SHORT_AIR_HOURS`, `SHORT_PHYS_DAYS`).
#[test]
#[ignore = "slow: six 8-year runs of a 32 x 32 world with life"]
fn the_long_step_keeps_the_climate() {
    let short: fn(&mut Config) = |c| (c.air_hours, c.phys_days) = (SHORT_AIR_HOURS, SHORT_PHYS_DAYS);
    let default: fn(&mut Config) = |_| {};
    // Per seed, [short, default], the short and the default step side by side.
    let runs: Vec<[[f64; CLIMATE.len()]; 2]> = SEEDS
        .iter()
        .map(|&seed| {
            std::thread::scope(|s| {
                let pair = [short, default].map(|set| s.spawn(move || climate(seed, set)));
                pair.map(|r| r.join().expect("a run"))
            })
        })
        .collect();
    let mut out = vec![];
    for (q, name) in CLIMATE.iter().enumerate() {
        let column = |k: usize| runs.iter().map(|r| r[k][q]).collect::<Vec<f64>>();
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let (base, long) = (column(0), column(1));
        let noise = base.iter().fold(f64::MIN, |m, &x| m.max(x)) - base.iter().fold(f64::MAX, |m, &x| m.min(x));
        let off = mean(&long) - mean(&base);
        let ok = off.abs() <= noise;
        println!("{name}: {:.6e} at the default steps (seeds {long:.4?}), {:.6e} at the short ones (seeds {base:.4?}), off by {off:.3e}, noise {noise:.3e}{}", mean(&long), mean(&base), if ok { "" } else { ": OUT" });
        if !ok {
            out.push(*name);
        }
    }
    assert!(out.is_empty(), "the long step changed: {out:?}");
}

/// Life's key numbers (series columns): the members and the mass of the light-eaters, the births of the year, the life
/// light-eaters live, and the heat-absorbing gas they take from the air.
const LIFE: [&str; 5] = ["plant_members", "root_0_kg", "births", "root_0_life_real", "air_3"];

/// Every population node at the pace of its own `INTERVAL` gene against the pace of nodes (`Config::node_days`), and
/// that setting x2 and /2 (`docs/numbers.md`, speed settings): the mean of each of life's key numbers over the last
/// years moves by less than its noise, the spread of its values over those years with every node at its own pace.
#[test]
#[ignore = "slow: four 8-year runs of a 32 x 32 world with life"]
fn the_pace_of_nodes_keeps_life() {
    let set = Config::default().node_days;
    let settings = [0, set, 2 * set, set / 2];
    let runs: Vec<Vec<Vec<f64>>> = std::thread::scope(|s| {
        let runs: Vec<_> = settings.iter().map(|&days| s.spawn(move || yearly(&LIFE, |c| c.node_days = days))).collect();
        runs.into_iter().map(|r| r.join().expect("a run")).collect()
    });
    let out = off_by_noise(&LIFE, &runs, &settings, |days| format!("nodes every {days} days at most"));
    assert!(out.is_empty(), "the pace of nodes changed life: {out:?}");
}
