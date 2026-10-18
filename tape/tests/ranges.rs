//! The key numbers of a normal run (`docs/foundation/plan.md` 6.3): `ew run --seed N --size 64` for seeds 1, 2 and 3,
//! 30 years each, on threads. The mean of each number over the three seeds must lie in its range around its
//! reference, the mean of the same three seeds in the reference runs (A2, `docs/reports/foundation-a/`). The climate
//! and the land worn down are means over years 21 to 30; life is read at year 30, ten years after the eaters came.
//! Each run also keeps its books.
//!
//! The sleep of Change's nodes is a speed tool (plan 6.4): a small world with life keeps the chemistry's key numbers
//! within their noise with every node awake every step (`Config::sleep_days` 0), and with the setting x2 and /2.

use pcore::{Config, World};
use std::fmt;
use tape::Recorder;
use worldgen::life::Seeding;
use worldgen::schedule::Schedule;
use worldgen::testkit::{ground_of, Books, ABSORBER};

const SEEDS: [u64; 3] = [1, 2, 3];
const SIDE: usize = 64;
const YEARS: u64 = 30;
/// The climate is the mean of the years from this one to `YEARS`; the land worn down is over the same years.
const FROM: u64 = 21;
/// The year life starts in `ew run` (`--life-year`).
const LIFE_YEAR: u64 = 2;
/// Parts per million in one; millimetres in a metre.
const PPM: f64 = 1e6;
const MM: f64 = 1e3;

/// Ground temperature on land, K (series `t_land`), in the A2 reference runs.
const T_LAND: f64 = 289.31;
/// Seasonal swing on land, K (`t_land_summer` less `t_land_winter`), in the A2 reference runs.
const SWING: f64 = 10.19;
/// Rain on land, mol/m²/s (`rain_land`, a moving average over about a year), in the A2 reference runs.
const RAIN_LAND: f64 = 1.496e-3;
/// The heat-absorbing gas of the air, ppm (`air_<ABSORBER>`), in the A2 reference runs.
const ABSORBER_PPM: f64 = 257.7;
/// Dry land cells with a liquid flux over the web viewer's threshold (`rivers`), in the A2 reference runs.
const RIVERS: f64 = 380.4;
/// Rock lost on the land of year 20, mm/yr (`testkit::ground_of`, loose and bonded), in the A2 reference runs.
const WORN: f64 = 0.0510;
/// Light-eaters (starting kind 0) at year 30, log10 kg (`root_0_kg`), in the A2 reference runs.
const PLANT_KG: f64 = 10.533;
/// Eaters (starting kind 1) at year 30, log10 kg (`root_1_kg`), in the A2 reference runs.
const EATER_KG: f64 = 4.497;
/// The life light-eaters live at year 30, yr (`root_0_life_real`: members standing over deaths), in the A2 runs.
const PLANT_LIFE: f64 = 1.895;
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
    ("eaters", "log10 kg", EATER_KG, Range::Plus(0.3)),
    ("life of light-eaters", "yr", PLANT_LIFE, Range::Times(2.0)),
    ("life of eaters", "yr", EATER_LIFE, Range::Times(2.0)),
];

/// The mean thickness of rock on the cells `land`, m.
fn rock(w: &World, land: &[usize]) -> f64 {
    land.iter().map(|&i| ground_of(w, i)).map(|g| g.0 + g.1).sum::<f64>() / land.len() as f64
}

/// `years` years of `ew run --seed <seed> --size <side>` with the settings `set` makes, recorded as `ew run` records it;
/// `each_year(year, world, recorder)` after each year. Checks the books at the end.
fn run(seed: u64, side: usize, years: u64, set: impl Fn(&mut Config), mut each_year: impl FnMut(u64, &World, &Recorder)) -> (World, Recorder) {
    let mut w = worldgen::new_world(seed, side, side);
    set(&mut w.cfg);
    // `ew run` scales the air's heat-absorbing gases by `--absorber` (1), which sets each air node's heat again.
    worldgen::terrain::scale_absorbers(&mut w, 1.0);
    let plan = Schedule::new(Some(LIFE_YEAR), false, None, Seeding::default());
    let (books, mut rec) = (Books::now(&w), Recorder::new(&w, 0, 0));
    rec.record_month(&mut w);
    for y in 1..=years {
        plan.place(&mut w);
        for _ in 0..w.cfg.months_per_year {
            pcore::step(&mut w);
            rec.record_month(&mut w);
        }
        each_year(y, &w, &rec);
    }
    books.assert_kept(&w, 1e-12);
    (w, rec)
}

/// `YEARS` years of `ew run --seed <seed> --size 64`: the key numbers in the order of `KEYS`.
fn key_numbers(seed: u64) -> [f64; KEYS.len()] {
    let gas = format!("air_{ABSORBER}");
    let climate = ["t_land", "t_land_summer", "t_land_winter", "rain_land", gas.as_str(), "rivers"];
    let (mut mean, mut land, mut rock_from) = ([0.0; 6], vec![], 0.0);
    let span = (YEARS - FROM + 1) as f64;
    let (w, rec) = run(seed, SIDE, YEARS, |_| {}, |y, w, rec| {
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

/// The `CHEMISTRY` columns at the end of each year from `SLEEP_FROM`, with Change's nodes sleeping at most `sleep_days`
/// and woken by a gain of `wake_share`.
fn chemistry((sleep_days, wake_share): (u32, f64)) -> Vec<Vec<f64>> {
    let mut years = vec![];
    run(1, SLEEP_SIDE, SLEEP_YEARS, |c| (c.sleep_days, c.wake_share) = (sleep_days, wake_share), |y, _, rec| {
        if y >= SLEEP_FROM {
            years.push(CHEMISTRY.map(|c| rec.last(c)).to_vec());
        }
    });
    years
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
        let runs: Vec<_> = settings.iter().map(|&d| s.spawn(move || chemistry(d))).collect();
        runs.into_iter().map(|r| r.join().expect("a run")).collect()
    });
    let mut out = vec![];
    for (q, name) in CHEMISTRY.iter().enumerate() {
        let column = |run: &[Vec<f64>]| run.iter().map(|y| y[q]).collect::<Vec<f64>>();
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let awake = column(&runs[0]);
        let noise = awake.iter().fold(f64::MIN, |m, &x| m.max(x)) - awake.iter().fold(f64::MAX, |m, &x| m.min(x));
        for (run, (days, wake)) in runs.iter().zip(settings).skip(1) {
            let off = mean(&column(run)) - mean(&awake);
            let ok = off.abs() <= noise;
            println!("{name}, sleep {days} days, wake at {wake}: {:.6e}, awake {:.6e}, off by {off:.3e}, noise {noise:.3e}{}", mean(&column(run)), mean(&awake), if ok { "" } else { ": OUT" });
            if !ok {
                out.push((*name, days, wake));
            }
        }
    }
    assert!(out.is_empty(), "the sleep changed: {out:?}");
}
