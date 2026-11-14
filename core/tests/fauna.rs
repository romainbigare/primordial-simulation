//! The takers (laws.md 3b): slots of members that pay their upkeep from their store and die by what they cannot pay,
//! keeping the books, their counts whole when few.

use pcore::fauna::{self, Fauna, BODY, FS, KEEP, N_WHOLE, REPAIR, STORE};
use pcore::life::{Parts, NO_ROOT, WET_TISSUE};
use pcore::units::DAY;
use pcore::World;
use worldgen::life::TAKER_KINDS;
use worldgen::testkit::{self, Books};

/// The row of the starting kind that keeps no heat of its own (`TAKER_KINDS`).
const COLD_ROW: usize = 4;
/// Members of the slot the law of the store is read on: real counts (over `N_WHOLE`).
const MANY: f64 = 1000.0;
/// The months of upkeep its store holds: three are paid in full, the fourth half.
const STORE_MONTHS: f64 = 3.5;

/// A small world with its soil and the cover of `ew run` after a month, and its mildest dry land cell, a pond standing
/// on it (a metre: liquid for any thirst) and no snow.
fn start() -> (World, usize) {
    let mut w = testkit::living(8, 1);
    let i = testkit::mild_land(&w);
    w.life.env[i][1] = 1.0;
    w.life.env[i][3] = 0.0;
    (w, i)
}

/// The takers of cell `i`.
fn of(w: &World, i: usize) -> &Fauna {
    &w.life.cell_at(i).fauna
}

/// The energy a member of the starting kind of row `row` burns in a day on ground at `t` K by the law of upkeep and
/// heat, J, its living mass `m` kg: Kleiber's (`c_basal` + `c_muscle`) m^`upkeep_exp` at the pace of its burning at its
/// body's temperature, times its repair factor; more where its coat loses more to colder ground.
fn burnt_a_day(w: &World, row: usize, m: f64, t: f64) -> f64 {
    let (ph, g) = (&w.cfg.phys, &TAKER_KINDS[row].genes);
    let rx = &w.chem.reactions[Parts::of(&w.chem).expect("life's parts").burn];
    let tb = (g[KEEP] as f64).max(t);
    let rest = (ph.c_basal + ph.c_muscle) * m.powf(ph.upkeep_exp) * rx.arrhenius(tb) / rx.arrhenius(ph.t_cat) * ph.repair(g[REPAIR] as f64);
    let coat = ph.meeh * m.powf(2.0 / 3.0) / (ph.r_air + g[fauna::COAT] as f64 * ph.r_coat);
    rest.max(coat * (g[KEEP] as f64 - t).max(0.0) * DAY)
}

/// The energy a kg of the takers' store frees when burnt, J.
fn per_kg(w: &World) -> f64 {
    let p = Parts::of(&w.chem).expect("life's parts");
    let rx = &w.chem.reactions[p.burn];
    let per = rx.inputs.iter().find(|q| q.0 as usize == p.sub[pcore::life::FUEL]).map_or(1.0, |q| q.1 as f64);
    -w.chem.dh(p.burn) / per / w.chem.subs[p.sub[pcore::life::FUEL]].molar_mass
}

/// A slot with no reachable cover starves by the law of upkeep: a store of 3.5 months of Kleiber's upkeep (a kind
/// that keeps no heat, on ground warm enough that it neither freezes nor sheds vapour) pays three months in full, its
/// members falling only by their wear (1 − e^-wear), and half of the fourth, when half the rest die unpaid; the
/// remains of the dead reach the ground. For every starting kind the month its store first runs short is the one the
/// law of upkeep and heat gives.
#[test]
fn a_slot_that_takes_nothing_starves_as_its_upkeep_says() {
    let (w0, i) = start();
    let (t, days) = (pcore::contract::place(&w0, i).t, w0.cfg.days_per_month as f64);
    let m = fauna::adult(&TAKER_KINDS[COLD_ROW].genes);
    let month = burnt_a_day(&w0, COLD_ROW, m, t) * days;
    let mut w = w0.clone();
    let books = Books::now(&w);
    let store = STORE_MONTHS * month / per_kg(&w);
    testkit::takers(&mut w, i, COLD_ROW, (MANY, store));
    let wear = 1.0 - (-month / (m * w.cfg.phys.e_life * pcore::cover::dec(TAKER_KINDS[COLD_ROW].genes[REPAIR]))).exp();
    let mut n = MANY;
    for k in 1..=4 {
        testkit::takers_month(&mut w, i);
        n *= (1.0 - wear) * if k == 4 { 1.0 - (STORE_MONTHS - 3.0) } else { 1.0 };
        let f = of(&w, i);
        assert!((f.adults[0] - n).abs() <= 1e-9 * n, "month {k}: {} members, the law says {n}", f.adults[0]);
        let store = f.mol[0][STORE] / f.adults[0];
        println!("month {k}: {:.3} members, store {store:.3e} mol each", f.adults[0]);
    }
    assert!(of(&w, i).mol[0][STORE] <= 1e-9 * MANY, "the store outlived its months");
    books.assert_kept(&w, 1e-9);
    for (row, kind) in TAKER_KINDS.iter().enumerate() {
        let mut w = w0.clone();
        let m = fauna::adult(&kind.genes);
        let store = worldgen::life::START_STORE * m;
        let lasts = store * per_kg(&w) / burnt_a_day(&w, row, m, t);
        testkit::takers(&mut w, i, row, (MANY, store));
        let mut short = None;
        for k in 1..=12 {
            testkit::takers_month(&mut w, i);
            let f = of(&w, i);
            if f.root[0] == NO_ROOT || f.mol[0][STORE] <= 0.0 {
                short = Some(k);
                break;
            }
        }
        println!("{}: the start's store lasts {lasts:.1} days at {t:.0} K; it ran out in month {short:?}", kind.name);
        assert_eq!(short, Some((lasts / days).floor() as u64 + 1), "{}: its store of {lasts:.1} days ran out in another month", kind.name);
    }
}

/// Matter and energy are kept with takers: the starting kinds placed on all the open land of a small world, a year:
/// they pay their upkeep, shed their heat and die, and every element and the energy of the world are kept.
#[test]
fn matter_and_energy_are_kept_with_takers() {
    let mut w = testkit::living(8, 1);
    let books = Books::now(&w);
    let said = worldgen::life::place_takers(&mut w);
    assert!(said.len() == TAKER_KINDS.len() && (0..w.mesh.cells().n()).any(|i| of(&w, i).any()), "no takers placed");
    for _ in 0..w.cfg.months_per_year {
        pcore::step(&mut w);
    }
    books.assert_kept(&w, 1e-9);
}

/// Counts are never rounded up: slots of few members, and one of a real count just over `N_WHOLE`, over 50 months
/// with stores for all of them: every count under `N_WHOLE` is whole, no count grows (no births), a slot of none is
/// empty and its matter is on the ground, and the books are kept.
#[test]
fn counts_are_never_rounded_up() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    let t = pcore::contract::place(&w, i).t;
    for (row, n) in [(0, 3.0), (1, 9.0), (COLD_ROW, N_WHOLE + 0.5)] {
        let m = fauna::adult(&TAKER_KINDS[row].genes);
        let store = 60.0 * burnt_a_day(&w, row, m, t) * w.cfg.days_per_month as f64 / per_kg(&w);
        testkit::takers(&mut w, i, row, (n, store));
    }
    let mut was = of(&w, i).adults;
    for k in 0..50 {
        testkit::takers_month(&mut w, i);
        let f = of(&w, i);
        for (x, &n) in f.adults.iter().enumerate() {
            assert!(n <= was[x], "month {k}, slot {x}: {n} members after {}", was[x]);
            assert!(n >= N_WHOLE || n.fract() == 0.0, "month {k}, slot {x}: {n} members, not whole");
            assert!(n > 0.0 || (f.root[x] == NO_ROOT && f.mol[x] == [0.0; 2]), "month {k}, slot {x}: an empty slot holds {:?}", f.mol[x]);
            assert!(n == 0.0 || f.mol[x][BODY] > 0.0, "month {k}, slot {x}: members with no body");
        }
        was = f.adults;
    }
    let left: f64 = of(&w, i).adults.iter().sum();
    println!("members left after 50 months: {left}; living kg {:.3e}", (0..FS).map(|x| of(&w, i).mol[x][BODY]).sum::<f64>() * w.chem.subs[10].molar_mass / (1.0 - WET_TISSUE));
    books.assert_kept(&w, 1e-9);
}
