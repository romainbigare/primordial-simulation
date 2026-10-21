//! The key numbers of a normal run: `ew run --seed N --size 64` for seeds 1, 2 and 3, 30 years each, on threads. The
//! mean of each number over the three seeds must lie in its range around its reference, the mean of the same three
//! seeds in the reference runs: the runs of the world after the foundation's law changes of Phases E, F and J, at the
//! steps of Phase J's end (decision 27); for the climate, the land worn down and the rivers, the runs since clouds absorb
//! heat radiation and the island lies at 40°, with the air starting at its mean height and the air's step taking the
//! surface's reflectance at its end (decision 34); for the heat-absorbing gas and the light-eaters, the runs after the
//! body costs and the light-eaters' walls (decision 53); for the eaters, the runs after bodies take only what they use
//! (decision 57). The climate and the land worn down are means over years 21 to 30; life
//! is read at year 30, ten years after the eaters came. Each run also keeps its books.
//!
//! The sleep of Change's nodes is a speed tool: a small world with life keeps the chemistry's key numbers
//! within their noise with every node awake every step (`Config::sleep_days` 0), and with the setting x2 and /2. So is
//! the pace of population nodes: the same world keeps its life's key numbers with every node at its own `INTERVAL`
//! gene (`Config::node_days` 0), and with the setting x2 and /2.
//!
//! The eaters live: at 48 x 48 they still have members at year 100 in seeds 1, 2 and 3, after the booms and busts
//! that year 30 hides; they have stopped growing by their last ten years, and none hold more than a few times the
//! densest soil animals. The ground under dense canopies
//! follows its neighbours from one month to the next.
//!
//! A globe of Earth's size without life keeps an Earth-like climate: its land near Earth's, steady, with no runaway ice.
//!
//! The steps of Flow are speed settings too (the long step against the short one): small worlds with life, each seed
//! against itself, keep their climate at the default steps and with the air's step at 6 hours and the physics step at
//! 3 days, within 0.3 K for a temperature and the year-to-year spread for the others.

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

/// Ground temperature on land, K (series `t_land`), in the reference runs.
const T_LAND: f64 = 287.83;
/// Seasonal swing on land, K (`t_land_summer` less `t_land_winter`), in the reference runs.
const SWING: f64 = 13.61;
/// Rain on land, mol/m²/s (`rain_land`, a moving average over about a year), in the reference runs.
const RAIN_LAND: f64 = 1.714e-3;
/// The heat-absorbing gas of the air, ppm (`air_<ABSORBER>`), in the reference runs.
const ABSORBER_PPM: f64 = 388.8;
/// Dry land cells with a liquid flux over the web viewer's threshold (`rivers`), in the reference runs.
const RIVERS: f64 = 136.1;
/// Rock lost on the land of year 20, mm/yr (`testkit::ground_of`, loose and bonded), in the reference runs.
const WORN: f64 = 0.0424;
/// Light-eaters (starting kind 0) at year 30, log10 kg (`root_0_kg`), in the reference runs.
const PLANT_KG: f64 = 10.38;
/// Eaters (starting kind 1) at year 30, log10 kg (`root_1_kg`), in the reference runs.
const EATER_KG: f64 = 4.86;
/// The life light-eaters live at year 30, yr (`root_0_life_real`: members standing over deaths), in the reference runs.
const PLANT_LIFE: f64 = 1.81;
/// The factor either way the light-eaters' life may lie from `PLANT_LIFE`.
const PLANT_LIFE_RANGE: f64 = 2.0;
/// How far the eaters' mass may lie from `EATER_KG`, log10: wider than the light-eaters' 0.3, as a few hundred eaters
/// make a noisy mean (decision 33).
const EATER_KG_RANGE: f64 = 0.5;
/// The life eaters live at year 30, yr (`root_1_life_real`), in the reference runs.
const EATER_LIFE: f64 = 10.47;

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

/// The long-step test's world: its side, its years (long enough for the plants to grow, as an error that comes with
/// them grows too), and the years from which its numbers are averaged.
const STEP_SIDE: usize = 32;
const STEP_YEARS: u64 = 20;
const STEP_FROM: u64 = 11;
/// How far the long step may move each temperature of the climate from the short step's, each seed against itself, K.
const STEP_K: f64 = 0.3;

/// The `CLIMATE` columns at the end of each year from `STEP_FROM`, on the long-step test's world of `seed`, with the
/// steps of Flow that `set` makes.
fn climate(seed: u64, set: fn(&mut Config)) -> Vec<[f64; CLIMATE.len()]> {
    let mut years = vec![];
    run(seed, STEP_SIDE, STEP_YEARS, false, set, |y, _, rec| {
        if y >= STEP_FROM {
            years.push(CLIMATE.map(|c| rec.last(c)));
        }
    });
    years
}

/// The air's step of 6 hours (`air_hours`), short against the air's changes in a day, and a physics step of 3 days
/// (`phys_days`).
const SHORT_AIR_HOURS: u32 = 6;
const SHORT_PHYS_DAYS: u32 = 3;

/// The long step against the short one, each seed against itself: at the default steps of Flow (`air_hours`,
/// `phys_days`, `flow_days`) the mean over the seeds of what the long step moves each of the climate's key numbers
/// (the mean of its last years) from the short steps (`SHORT_AIR_HOURS`, `SHORT_PHYS_DAYS`) is less than `STEP_K` for
/// a temperature, and less than its year-to-year spread at the short steps (the mean over the seeds of its highest
/// less its lowest yearly value) for the others.
#[test]
#[ignore = "slow: six 20-year runs of a 32 x 32 world with life"]
fn the_long_step_keeps_the_climate() {
    let short: fn(&mut Config) = |c| (c.air_hours, c.phys_days) = (SHORT_AIR_HOURS, SHORT_PHYS_DAYS);
    let default: fn(&mut Config) = |_| {};
    // Per seed, [short, default], all side by side.
    let runs: Vec<[Vec<[f64; CLIMATE.len()]>; 2]> = std::thread::scope(|s| {
        let runs: Vec<_> = SEEDS.iter().map(|&seed| [short, default].map(|set| s.spawn(move || climate(seed, set)))).collect();
        runs.into_iter().map(|pair| pair.map(|r| r.join().expect("a run"))).collect()
    });
    let seeds = runs.len() as f64;
    let mut out = vec![];
    for (q, name) in CLIMATE.iter().enumerate() {
        let mean = |run: &[[f64; CLIMATE.len()]]| run.iter().map(|y| y[q]).sum::<f64>() / run.len() as f64;
        let spread = |run: &[[f64; CLIMATE.len()]]| run.iter().map(|y| y[q]).fold(f64::MIN, f64::max) - run.iter().map(|y| y[q]).fold(f64::MAX, f64::min);
        let offs: Vec<f64> = runs.iter().map(|[base, long]| mean(long) - mean(base)).collect();
        let off = offs.iter().sum::<f64>() / seeds;
        let limit = if name.starts_with("t_") { STEP_K } else { runs.iter().map(|[base, _]| spread(base)).sum::<f64>() / seeds };
        let ok = off.abs() <= limit;
        let short: Vec<String> = runs.iter().map(|[base, _]| format!("{:.4e}", mean(base))).collect();
        let seeds: Vec<String> = offs.iter().map(|x| format!("{x:.3e}")).collect();
        println!("{name}: off by {off:.3e} (seeds {seeds:?}) at the default steps from the short ones (seeds {short:?}), limit {limit:.3e}{}", if ok { "" } else { ": OUT" });
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

/// The world of the eaters' check: its side, its years, and the first of its last years, over which the eaters must
/// have stopped growing: thirty years, as a settled seed's births less deaths swing by about 15% of births a decade.
const EATER_SIDE: usize = 48;
const EATER_YEARS: u64 = 100;
const EATER_FROM: u64 = 71;
/// The most the eaters' births may exceed their deaths over those years, as a share of their births: a population that
/// replaces itself to within a tenth has stopped growing.
const EATER_GROWTH: f64 = 0.1;
/// The most eaters a km² of land may hold at the end, kg: soil animals that live on dead matter, as these do, reach
/// 1e4 to 1e5 kg/km² (earthworms, `docs/numbers.md`), and this bound lies above them by a few times.
const EATER_MOST_KG_KM2: f64 = 3e5;
/// Square metres in a km².
const KM2: f64 = 1e6;
/// The causes of death (`docs/tape.md`).
const CAUSES: [&str; 6] = ["lack", "phase", "spent", "taken", "broken", "age"];

/// The eaters of a 48 x 48 world of `seed` after `EATER_YEARS`: their members, their mass per km² of land, kg, and
/// their births and deaths in the years from `EATER_FROM`.
fn eaters_at_end(seed: u64) -> [f64; 4] {
    let (mut born, mut died) = (0.0, 0.0);
    let (w, rec) = run(seed, EATER_SIDE, EATER_YEARS, false, |_| {}, |y, _, rec| {
        if y >= EATER_FROM {
            born += rec.last("root_1_births");
            died += CAUSES.iter().map(|c| rec.last(&format!("root_1_died_{c}"))).sum::<f64>();
        }
    });
    let land: f64 = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).map(|i| w.mesh.cells().area()[i]).sum();
    [rec.last("root_1_members"), rec.last("root_1_kg") / land * KM2, born, died]
}

/// The eaters live to year 100 in seeds 1, 2 and 3, after the booms and busts of their first decades; by their last
/// thirty years they have stopped growing (their births exceed their deaths by less than `EATER_GROWTH` of their
/// births); and none hold more than `EATER_MOST_KG_KM2`.
#[test]
#[ignore = "slow: three 100-year runs of a 48 x 48 world"]
fn the_eaters_live_and_stop_growing() {
    let eaters: Vec<[f64; 4]> = std::thread::scope(|s| {
        let runs: Vec<_> = SEEDS.iter().map(|&seed| s.spawn(move || eaters_at_end(seed))).collect();
        runs.into_iter().map(|r| r.join().expect("an eater run")).collect()
    });
    println!("eaters at year {EATER_YEARS}, seeds {SEEDS:?}: (members, kg per km2 of land, births and deaths from year {EATER_FROM}) {eaters:?}");
    assert!(eaters.iter().all(|e| e[0] > 0.0), "the eaters died out by year {EATER_YEARS}: {eaters:?}");
    let growing = eaters.iter().any(|e| e[2] - e[3] > EATER_GROWTH * e[2]);
    assert!(!growing, "the eaters still grow in years {EATER_FROM} to {EATER_YEARS}: {eaters:?}");
    let boomed = eaters.iter().any(|e| e[1] > EATER_MOST_KG_KM2);
    assert!(!boomed, "the eaters boomed past {EATER_MOST_KG_KM2} kg/km2: {eaters:?}");
}

/// The canopy check: the years before it, the body mass per area over which a cell is a dense canopy, kg/m², how far
/// its ground may move against the mean of its neighbours within 30 days, K, and the share of canopies that may move
/// more.
const CANOPY_YEARS: u64 = 20;
const CANOPY_KG: f64 = 20.0;
const CANOPY_SWING: f64 = 10.0;
const CANOPY_SHARE: f64 = 0.15;

/// Bodies draw their heat from the ground inside the air's implicit step (laws.md 1.4), so the ground under a dense
/// canopy follows its neighbours: through a year, day by day, after `CANOPY_YEARS` of seed 1, fewer than
/// `CANOPY_SHARE` of the dense canopies move more than `CANOPY_SWING` against their neighbours within 30 days.
#[test]
#[ignore = "slow: a 20-year run of a 64 x 64 world, then a year day by day"]
fn the_ground_under_dense_canopies_stays_near_its_neighbours() {
    let (mut w, _) = run(1, SIDE, CANOPY_YEARS, false, |_| {}, |_, _, _| {});
    let area = w.mesh.cells().area().to_vec();
    let dense: Vec<usize> = (0..area.len()).filter(|&i| w.is_land(i) && w.life.sum.get(i).map_or(0.0, |s| s[1]) / area[i] > CANOPY_KG).collect();
    // The gap of each canopy's ground to the mean of its neighbours, every 10 days.
    let gap = |w: &World, i: usize| {
        let around = w.mesh.cells().around(i).1;
        w.t_ground_last(i) - around.iter().map(|&j| w.t_ground_last(j as usize)).sum::<f64>() / around.len() as f64
    };
    let mut gaps: Vec<Vec<f64>> = vec![];
    for day in 0..w.days_per_year() {
        pcore::day(&mut w);
        if day % 10 == 9 {
            gaps.push(dense.iter().map(|&i| gap(&w, i)).collect());
        }
    }
    let swing = |c: usize| gaps.windows(4).map(|g| g.iter().map(|x| x[c]).fold(f64::MIN, f64::max) - g.iter().map(|x| x[c]).fold(f64::MAX, f64::min)).fold(0.0, f64::max);
    let swings: Vec<f64> = (0..dense.len()).map(swing).collect();
    let moved = swings.iter().filter(|&&x| x > CANOPY_SWING).count();
    println!("{moved} of {} dense canopies moved more than {CANOPY_SWING} K against their neighbours within 30 days; the most {:.1} K", dense.len(), swings.iter().fold(0.0f64, |a, &b| a.max(b)));
    assert!(!dense.is_empty() && (moved as f64) < CANOPY_SHARE * dense.len() as f64, "{moved} of {} dense canopies moved more than {CANOPY_SWING} K", dense.len());
}

/// The globe of the climate check: its rows (twice as many columns), its years, and the years its means are over.
const GLOBE_ROWS: usize = 32;
const GLOBE_YEARS: u64 = 20;
const GLOBE_FROM: u64 = 11;
/// Earth's land, K: its mean surface is about 282 K, its whole surface 288 K (from memory); the bounds leave a few K.
const EARTH_LAND: [f64; 2] = [280.0, 292.0];
/// The most the globe's land may drift in a year over those years, K, and the most of its surface ice may cover.
const GLOBE_DRIFT_K: f64 = 0.2;
const GLOBE_ICE: f64 = 0.25;

/// A globe of Earth's size under Earth's sun, without life, keeps an Earth-like climate: over years `GLOBE_FROM` to
/// `GLOBE_YEARS` its land stays within `EARTH_LAND` and drifts by less than `GLOBE_DRIFT_K` a year, and ice covers
/// less than `GLOBE_ICE` of it (no runaway of the ice's reflectance).
#[test]
#[ignore = "slow: a 20-year run of a 64 x 32 globe without life"]
fn a_globe_without_life_keeps_an_earth_like_climate() {
    let mut w = worldgen::terrain::new_globe(1, GLOBE_ROWS);
    worldgen::terrain::scale_absorbers(&mut w, 1.0);
    let mut rec = tape::Recorder::new(&w, 0, 0);
    common::record(&mut rec, &mut w);
    let mut years = vec![];
    for y in 1..=GLOBE_YEARS {
        for _ in 0..w.cfg.months_per_year {
            pcore::step(&mut w);
            common::record(&mut rec, &mut w);
        }
        if y >= GLOBE_FROM {
            years.push([rec.last("t_land"), rec.last("ice_frac")]);
        }
    }
    let (first, last) = (years[0], years[years.len() - 1]);
    let land = years.iter().map(|y| y[0]).sum::<f64>() / years.len() as f64;
    let drift = (last[0] - first[0]) / (GLOBE_YEARS - GLOBE_FROM) as f64;
    let ice = years.iter().map(|y| y[1]).fold(0.0, f64::max);
    let report = format!("land {land:.2} K, drift {drift:.3} K a year, ice at most {ice:.3}");
    println!("{report}");
    assert!((EARTH_LAND[0]..=EARTH_LAND[1]).contains(&land) && drift.abs() < GLOBE_DRIFT_K && ice < GLOBE_ICE, "{report}");
}
