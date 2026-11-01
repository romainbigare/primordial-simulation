//! The world lives and keeps its books: a small island with the roster keeps every element and the energy for 26
//! years, holds no number that is not finite, and has cover and two kinds of animals alive at the end.
//!
//! The key numbers of a normal run: `ew run --seed N --size 64` for seeds 1, 2 and 3, 30 years each, on threads. The
//! mean of each number over the three seeds must lie in its range around its reference, the mean of the same three
//! seeds in the reference runs: for the climate, the land worn down and the rivers, the runs of the world before the
//! cover and the short body model; for the cover and the eaters, the runs of the cover and the short body model. The
//! climate and the land worn down are means over years 21 to 30; life is read at year 30, ten years after the eaters
//! came. Each run also keeps its books. Two numbers are also held to wide bounds around Earth's, as the references only
//! keep a world near itself: the rain on land, and the share of the cover's dry mass it holds as its stored solid.
//!
//! The sleep of Change's nodes is a speed tool: a small world with life keeps the chemistry's key numbers
//! within their noise with every node awake every step (`Config::sleep_days` 0), and with the setting x2 and /2.
//!
//! The eaters live: at 48 x 48 they still have members at year 100 in seeds 1, 2 and 3, after the booms and busts
//! that year 30 hides; they have stopped growing by their last ten years, and none hold more than a few times the
//! densest soil animals.
//!
//! A globe of Earth's size without life keeps an Earth-like climate: its land near Earth's, steady, with no runaway
//! ice, and its surface, vapour and rain on land within wide bounds around Earth's.

mod common;

use common::run;
use pcore::chem::Phase;
use pcore::{Config, World};
use std::fmt;
use worldgen::testkit::{ground_of, ABSORBER, LIQUID};

const SEEDS: [u64; 3] = [1, 2, 3];
const SIDE: usize = 64;
const YEARS: u64 = 30;
/// The climate is the mean of the years from this one to `YEARS`; the land worn down is over the same years.
const FROM: u64 = 21;
/// Parts per million in one; millimetres in a metre.
const PPM: f64 = 1e6;
const MM: f64 = 1e3;

/// Ground temperature on land, K (series `t_land`), in the reference runs.
const T_LAND: f64 = 291.79;
/// Seasonal swing on land, K (`t_land_summer` less `t_land_winter`), in the reference runs.
const SWING: f64 = 15.13;
/// Rain on land, mol/m²/s (`rain_land`, a moving average over about a year), in the reference runs.
const RAIN_LAND: f64 = 1.714e-3;
/// The heat-absorbing gas of the air, ppm (`air_<ABSORBER>`), in the reference runs.
const ABSORBER_PPM: f64 = 388.8;
/// Dry land cells with a liquid flux over the web viewer's threshold (`rivers`), in the reference runs.
const RIVERS: f64 = 712.4;
/// Rock lost on the land of year 20, mm/yr (`testkit::ground_of`, loose and bonded), in the reference runs.
const WORN: f64 = 0.141;
/// The cover (starting kind 0) at year 30, log10 kg (`root_0_cover_kg`), in the reference runs.
const COVER_KG: f64 = 9.985;
/// Eaters (starting kind 1) at year 30, log10 kg (`root_1_kg`), in the reference runs.
const EATER_KG: f64 = 9.166;
/// How far the eaters' mass may lie from `EATER_KG`, log10: wider than the cover's 0.3, as a boom or bust of the eaters
/// moves a mean of three seeds far.
const EATER_KG_RANGE: f64 = 0.5;
/// The life eaters live at year 30, yr (`root_1_life_real`), in the reference runs.
const EATER_LIFE: f64 = 2.43;

/// Earth's rain on land, m/yr (Trenberth et al. 2007), and the factor either way a run's may lie from it: the island
/// lies at 40°, where land gets 0.5 to 1 m (from memory), and a globe's land spans every belt.
const EARTH_RAIN_LAND: f64 = 0.76;
const EARTH_RAIN_RANGE: f64 = 3.0;
/// The most share of the cover's dry mass its stored solid may make: plants hold about a tenth of theirs as
/// stored sugars and starch, rarely over a third in storage organs (from memory).
const STORED_MOST: f64 = 0.5;

/// Rain of the key liquid, mol/m²/s, in m of liquid a year of `w`.
fn rain_m_yr(w: &World, rain: f64) -> f64 {
    rain * w.chem.molar_volume(LIQUID, Phase::Liquid) * w.cfg.calendar().year as f64
}

/// The share of the cover's dry mass that is its stored solid.
fn stored_share(w: &World) -> f64 {
    let s = pcore::cover::sums(w);
    let (kg, built) = (s.iter().map(|x| x.kg).sum::<f64>(), s.iter().map(|x| x.built).sum::<f64>());
    (kg - built) / kg.max(f64::MIN_POSITIVE)
}

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

/// The liveness check's island: its side and years.
const LIVE_SIDE: usize = 16;
const LIVE_YEARS: u64 = 26;
/// The starting kinds that are animals (bodies) in the roster.
fn animal_kinds() -> Vec<usize> {
    worldgen::life::KINDS.iter().enumerate().filter(|k| matches!(k.1.place, worldgen::life::Place::Bodies(_))).map(|k| k.0).collect()
}

/// `LIVE_YEARS` of a `LIVE_SIDE` island with the roster (seed 1) keep every element and the energy (`common::run`
/// checks the books to 1e-12), leave no number that is not finite (`check::broken`), and end with cover and at least
/// two kinds of animals alive.
#[test]
fn the_world_lives_and_keeps_its_books() {
    let (w, rec) = run(1, LIVE_SIDE, LIVE_YEARS, true, |_| {}, |_, w, _| assert_eq!(pcore::check::broken(w), None));
    let cover = rec.last("cover_kg");
    let alive: Vec<usize> = animal_kinds().into_iter().filter(|k| w.life.alive().any(|b| w.life.brains[b.lineage as usize].as_ref().is_some_and(|l| l.root as usize == *k))).collect();
    println!("year {LIVE_YEARS}: cover {cover:.3e} kg, animal kinds alive {alive:?}");
    assert!(cover > 0.0, "the cover died out");
    assert!(alive.len() >= 2, "animal kinds alive: {alive:?}");
}

/// The key numbers in the order `key_numbers` gives them: name, unit, reference and range.
const KEYS: [(&str, &str, f64, Range); 9] = [
    ("ground temperature on land", "K", T_LAND, Range::Plus(1.0)),
    ("seasonal swing on land", "K", SWING, Range::Plus(1.5)),
    ("rain on land", "mol/m2/s", RAIN_LAND, Range::Times(1.3)),
    ("heat-absorbing gas", "ppm", ABSORBER_PPM, Range::Plus(40.0)),
    ("river cells", "", RIVERS, Range::Times(1.5)),
    ("land worn down", "mm/yr", WORN, Range::Times(2.0)),
    ("cover", "log10 kg", COVER_KG, Range::Plus(0.3)),
    ("eaters", "log10 kg", EATER_KG, Range::Plus(EATER_KG_RANGE)),
    ("life of eaters", "yr", EATER_LIFE, Range::Times(2.0)),
];

/// The mean thickness of rock on the cells `land`, m.
fn rock(w: &World, land: &[usize]) -> f64 {
    land.iter().map(|&i| ground_of(w, i)).map(|g| g.0 + g.1).sum::<f64>() / land.len() as f64
}

/// `YEARS` years of `ew run --seed <seed> --size 64`: the key numbers in the order of `KEYS`, then the rain on land in
/// m/yr and the cover's stored share (`stored_share`) at the end.
fn key_numbers(seed: u64) -> ([f64; KEYS.len()], [f64; 2]) {
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
    let (cover, eaters) = (rec.last("root_0_cover_kg").log10(), rec.last("root_1_kg").log10());
    let keys = [t, summer - winter, rain, air * PPM, rivers, worn, cover, eaters, rec.last("root_1_life_real")];
    (keys, [rain_m_yr(&w, rain), stored_share(&w)])
}

/// The key numbers of three seeds, each mean in its range around its reference; the rain on land within
/// `EARTH_RAIN_RANGE` of Earth's, and the cover's stored share under `STORED_MOST`.
#[test]
#[ignore = "slow: three 30-year runs of a 64 x 64 world"]
fn the_key_numbers_stay_in_their_ranges() {
    let runs: Vec<([f64; KEYS.len()], [f64; 2])> = std::thread::scope(|s| {
        let runs: Vec<_> = SEEDS.iter().map(|&seed| s.spawn(move || key_numbers(seed))).collect();
        runs.into_iter().map(|r| r.join().expect("a run")).collect()
    });
    let mut out = vec![];
    for (k, &(name, unit, reference, range)) in KEYS.iter().enumerate() {
        let seeds: Vec<f64> = runs.iter().map(|r| r.0[k]).collect();
        let mean = seeds.iter().sum::<f64>() / seeds.len() as f64;
        let ok = range.holds(mean, reference);
        println!("{name}: {mean:.6} {unit} (seeds {seeds:.6?}), reference {reference} {range}{}", if ok { "" } else { ": OUT" });
        if !ok {
            out.push(name);
        }
    }
    let earth = |q: usize| runs.iter().map(|r| r.1[q]).collect::<Vec<f64>>();
    let (rain, stored) = (earth(0).iter().sum::<f64>() / 3.0, earth(1).iter().sum::<f64>() / 3.0);
    let rain_ok = Range::Times(EARTH_RAIN_RANGE).holds(rain, EARTH_RAIN_LAND);
    println!("rain on land {rain:.3} m/yr, against Earth's {EARTH_RAIN_LAND} x/ {EARTH_RAIN_RANGE}");
    println!("the cover's stored share {stored:.3} (seeds {:.3?}), under {STORED_MOST}", earth(1));
    out.extend((!rain_ok).then_some("rain on land against Earth's"));
    out.extend((stored >= STORED_MOST).then_some("the cover's stored share"));
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

/// The `columns` at the end of each year from `SLEEP_FROM` of the sleep test's world of `seed` with the settings `set`
/// makes.
fn yearly(seed: u64, columns: &[&str], set: impl Fn(&mut Config)) -> Vec<Vec<f64>> {
    let mut years = vec![];
    run(seed, SLEEP_SIDE, SLEEP_YEARS, false, set, |y, _, rec| {
        if y >= SLEEP_FROM {
            years.push(columns.iter().map(|c| rec.last(c)).collect());
        }
    });
    years
}

/// For each of `columns`, the mean over the years of each run of `runs` after the first, against the first's: it must
/// move by less than its noise, the spread of the first run's values over those years. Prints each; returns the
/// columns and settings (named by `name`) that moved more. The noise is the year-to-year range, trend included: for
/// numbers that still grow in these years (plant members, kg and births) it is 66 to 84% of the value, so the test is
/// loose on those three and tight on the others. A column in `accepted` is held to the bound given there, a known bias
/// the owner accepts, instead of to its noise.
fn off_by_noise<S: Copy>(columns: &[&str], runs: &[Vec<Vec<f64>>], settings: &[S], accepted: &[(&str, f64)], name: impl Fn(S) -> String) -> Vec<String> {
    let mut out = vec![];
    for (q, column) in columns.iter().enumerate() {
        let values = |run: &[Vec<f64>]| run.iter().map(|y| y[q]).collect::<Vec<f64>>();
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let first = values(&runs[0]);
        let noise = first.iter().fold(f64::MIN, |m, &x| m.max(x)) - first.iter().fold(f64::MAX, |m, &x| m.min(x));
        let limit = accepted.iter().find(|a| a.0 == *column).map_or(noise, |a| a.1);
        for (run, &set) in runs.iter().zip(settings).skip(1) {
            let off = mean(&values(run)) - mean(&first);
            let ok = off.abs() <= limit;
            println!("{column}, {}: {:.6e}, against {:.6e}, off by {off:.3e}, noise {noise:.3e}, limit {limit:.3e}{}", name(set), mean(&values(run)), mean(&first), if ok { "" } else { ": OUT" });
            if !ok {
                out.push(format!("{column}, {}", name(set)));
            }
        }
    }
    out
}

/// The sleep's known bias on the heat-absorbing gas, as a share of the air: the sleep of Change leaves it 0.8e-6 to
/// 1.6e-6 high in this world (seeds 1 to 6), about +0.9% of the gas and +0.04 K of the land over a long run, which the
/// owner accepts (`docs/lighter/measures.md`, "Study: the sleep's bias on the gas"). Held to twice that, so a growing
/// bias fails the test.
const GAS_SLEEP_BIAS: f64 = 3e-6;

/// Sleep against no sleep, and each speed setting of the sleep x2 and /2 (`docs/numbers.md`, speed settings): the
/// longest sleep (`sleep_days`) and the gain that wakes a node (`wake_share`). Against every node awake every step, the
/// mean of each of the chemistry's key numbers over the last years moves by less than its noise, the spread of its
/// values over those years with every node awake; the heat-absorbing gas by less than `GAS_SLEEP_BIAS`.
#[test]
#[ignore = "slow: six 8-year runs of a 32 x 32 world with life"]
fn the_sleep_of_change_keeps_the_chemistry() {
    let (set, share) = (Config::default().sleep_days, Config::default().wake_share);
    let settings = [(0, share), (set, share), (2 * set, share), (set / 2, share), (set, 2.0 * share), (set, share / 2.0)];
    let runs: Vec<Vec<Vec<f64>>> = std::thread::scope(|s| {
        let runs: Vec<_> = settings.iter().map(|&(days, wake)| s.spawn(move || yearly(SEEDS[0], &CHEMISTRY, |c| (c.sleep_days, c.wake_share) = (days, wake)))).collect();
        runs.into_iter().map(|r| r.join().expect("a run")).collect()
    });
    let out = off_by_noise(&CHEMISTRY, &runs, &settings, &[("air_3", GAS_SLEEP_BIAS)], |(days, wake)| format!("sleep {days} days, wake at {wake}"));
    assert!(out.is_empty(), "the sleep changed: {out:?}");
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

/// The climate's key numbers (series columns): the ground temperature on land, the warmest and the coldest month's,
/// the rain on land, the heat-absorbing gas and the river cells.
const CLIMATE: [&str; 6] = ["t_land", "t_land_summer", "t_land_winter", "rain_land", "air_3", "rivers"];

/// The long-step test's world: its side, its years, the years from which its numbers are averaged, its seeds, and how
/// far the default step may move each temperature of the climate from a shorter one's, each seed against itself, K.
const STEP_SIDE: usize = 32;
const STEP_YEARS: u64 = 20;
const STEP_FROM: u64 = 11;
const STEP_SEEDS: [u64; 3] = [1, 2, 3];
const STEP_K: f64 = 0.3;
/// The shorter air steps, hours: a day, the air's own step, and a quarter of one.
const SHORT_AIR_HOURS: [u32; 2] = [24, 6];

/// The `CLIMATE` columns at the end of each year from `STEP_FROM` of seed `seed` with the air's step `hours` long.
fn climate(seed: u64, hours: u32) -> Vec<[f64; CLIMATE.len()]> {
    let mut years = vec![];
    run(seed, STEP_SIDE, STEP_YEARS, false, |c: &mut Config| c.air_hours = hours, |y, _, rec| {
        if y >= STEP_FROM {
            years.push(CLIMATE.map(|c| rec.last(c)));
        }
    });
    years
}

/// The long step against shorter ones (the air's step is a speed setting, `docs/numbers.md`): on small worlds with life,
/// the mean over the seeds of what the default air step moves each of the climate's key numbers (the mean of its last
/// years) from the step of each of `SHORT_AIR_HOURS`, each seed against itself, is less than `STEP_K` for a temperature
/// and less than its year-to-year spread at the short step for the others.
#[test]
#[ignore = "slow: nine 20-year runs of a 32 x 32 world with life"]
fn the_long_step_keeps_the_climate() {
    let hours = [Config::default().air_hours, SHORT_AIR_HOURS[0], SHORT_AIR_HOURS[1]];
    let runs: Vec<Vec<Vec<[f64; CLIMATE.len()]>>> = std::thread::scope(|s| {
        let runs: Vec<Vec<_>> = STEP_SEEDS.iter().map(|&seed| hours.iter().map(|&h| s.spawn(move || climate(seed, h))).collect()).collect();
        runs.into_iter().map(|seed| seed.into_iter().map(|r| r.join().expect("a run")).collect()).collect()
    });
    let mut out = vec![];
    for (q, name) in CLIMATE.iter().enumerate() {
        let mean = |run: &[[f64; CLIMATE.len()]]| run.iter().map(|y| y[q]).sum::<f64>() / run.len() as f64;
        let spread = |run: &[[f64; CLIMATE.len()]]| run.iter().map(|y| y[q]).fold(f64::MIN, f64::max) - run.iter().map(|y| y[q]).fold(f64::MAX, f64::min);
        for (h, &short) in SHORT_AIR_HOURS.iter().enumerate() {
            let off = runs.iter().map(|seed| mean(&seed[0]) - mean(&seed[1 + h])).sum::<f64>() / STEP_SEEDS.len() as f64;
            let limit = if name.starts_with("t_") { STEP_K } else { runs.iter().map(|seed| spread(&seed[1 + h])).sum::<f64>() / STEP_SEEDS.len() as f64 };
            let ok = off.abs() <= limit;
            println!("{name}: the default step off by {off:.3e} from {short}-hour steps, limit {limit:.3e}{}", if ok { "" } else { ": OUT" });
            if !ok {
                out.push(format!("{name} at {short} hours"));
            }
        }
    }
    assert!(out.is_empty(), "the long step changed: {out:?}");
}

/// The globe of the climate check: its rows (twice as many columns), its years, and the years its means are over.
const GLOBE_ROWS: usize = 32;
const GLOBE_YEARS: u64 = 100;
const GLOBE_FROM: u64 = 71;
/// Earth's land, K: its mean surface is about 282 K, its whole surface 288 K (from memory); the bounds leave a few K.
const EARTH_LAND: [f64; 2] = [280.0, 292.0];
/// Earth's whole surface, 288 K, and how far a globe's may lie from it, K.
const EARTH_SURFACE: f64 = 288.0;
const EARTH_SURFACE_RANGE: f64 = 4.0;
/// Earth's vapour, mole fraction of the air: 25 kg/m² of precipitable water over 1e4 kg/m² of air (Trenberth et al.
/// 2005) times 29 / 18; and the factor either way a globe's may lie from it (about 7% a K, Clausius-Clapeyron).
const EARTH_VAPOUR: f64 = 0.004;
const EARTH_VAPOUR_RANGE: f64 = 2.0;
/// The most the globe's land may drift in a year over those years, K (it has settled), and the most of its surface ice
/// may cover.
const GLOBE_DRIFT_K: f64 = 0.02;
const GLOBE_ICE: f64 = 0.25;

/// A globe of Earth's size under Earth's sun, without life, keeps an Earth-like climate: over years `GLOBE_FROM` to
/// `GLOBE_YEARS` its land stays within `EARTH_LAND` and drifts by less than `GLOBE_DRIFT_K` a year, and ice covers
/// less than `GLOBE_ICE` of it (no runaway of the ice's reflectance); its whole surface, its vapour and its rain on
/// land lie within their bounds around Earth's.
#[test]
#[ignore = "slow: a 100-year run of a 64 x 32 globe without life"]
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
            let earth = [rec.last("t_ground"), rec.last(&format!("air_{LIQUID}")), rec.last("rain_land")];
            years.push([rec.last("t_land"), rec.last("ice_frac"), earth[0], earth[1], earth[2]]);
        }
    }
    let mean = |q: usize| years.iter().map(|y| y[q]).sum::<f64>() / years.len() as f64;
    let (surface, vapour, rain) = (mean(2), mean(3), rain_m_yr(&w, mean(4)));
    let earth = format!("surface {surface:.2} K, vapour {vapour:.4}, rain on land {rain:.3} m/yr");
    println!("{earth}");
    let warm_ok = Range::Plus(EARTH_SURFACE_RANGE).holds(surface, EARTH_SURFACE);
    let vapour_ok = Range::Times(EARTH_VAPOUR_RANGE).holds(vapour, EARTH_VAPOUR);
    assert!(warm_ok && vapour_ok && Range::Times(EARTH_RAIN_RANGE).holds(rain, EARTH_RAIN_LAND), "{earth}");
    let (first, last) = (years[0], years[years.len() - 1]);
    let land = years.iter().map(|y| y[0]).sum::<f64>() / years.len() as f64;
    let drift = (last[0] - first[0]) / (GLOBE_YEARS - GLOBE_FROM) as f64;
    let ice = years.iter().map(|y| y[1]).fold(0.0, f64::max);
    let report = format!("land {land:.2} K, drift {drift:.3} K a year, ice at most {ice:.3}");
    println!("{report}");
    assert!((EARTH_LAND[0]..=EARTH_LAND[1]).contains(&land) && drift.abs() < GLOBE_DRIFT_K && ice < GLOBE_ICE, "{report}");
}
