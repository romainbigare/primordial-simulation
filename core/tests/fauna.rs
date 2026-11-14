//! The takers (laws.md 3b): slots of members that pay their upkeep from their store and die by what they cannot pay,
//! keeping the books, their counts whole when few.

use pcore::fauna::{self, take, Fauna, BODY, FS, KEEP, N_WHOLE, REPAIR, STORE};
use pcore::life::{Parts, NO_ROOT, WET_TISSUE};
use pcore::units::DAY;
use pcore::World;
use worldgen::life::TAKER_KINDS;
use worldgen::testkit::{self, Books};

/// The row of the starting kind that keeps no heat of its own (`TAKER_KINDS`).
const COLD_ROW: usize = 4;
/// Members of the slot the law of the store is read on: real counts (over `N_WHOLE`).
const MANY: f64 = 1000.0;
/// The share of a month's upkeep its store holds (within the most a taker stores, `store_most`).
const STORE_MONTHS: f64 = 0.6;

/// A small world with no life after a month, and its mildest dry land cell, a pond standing on it (a metre: liquid for
/// any thirst) and no snow: takers there find nothing to take.
fn start() -> (World, usize) {
    let mut w = testkit::small_world();
    let i = testkit::mild_land(&w);
    w.life.env[i][1] = 1.0;
    w.life.env[i][3] = 0.0;
    (w, i)
}

/// The takers of cell `i`.
fn of(w: &World, i: usize) -> &Fauna {
    &w.life.cell_at(i).fauna
}

/// The energy a member of the starting kind of row `row` burns in a day on ground at `t` K, finding nothing, by the
/// law of upkeep, the walk and the heat, J, its living mass `m` kg: at rest Kleiber's (`c_basal` + `c_muscle`)
/// m^`upkeep_exp` and its gut's capacity (`c_capacity` of the fuel it could free), at the pace of its burning at its
/// body's temperature, times its repair factor; the walk of its whole working time searching (speed
/// (`walk_froude` g L)^0.5 at `c_move` (m / `move_ref`)^`move_exp` J per kg and m); no less than its coat loses to
/// colder ground.
fn burnt_a_day(w: &World, row: usize, m: f64, t: f64) -> f64 {
    let (ph, g, chem) = (&w.cfg.phys, &TAKER_KINDS[row].genes, &w.chem);
    let p = Parts::of(chem).expect("life's parts");
    let rx = &chem.reactions[p.burn];
    let tb = (g[KEEP] as f64).max(t);
    let pace = rx.arrhenius(tb) / rx.arrhenius(ph.t_cat);
    let makes = |v: &[(u16, u16)], k: usize| v.iter().any(|q| q.0 as usize == p.sub[k]);
    let split = chem.reactions.iter().find(|r| makes(&r.inputs, pcore::life::WALL) && makes(&r.outputs, pcore::life::FUEL)).expect("walls split into fuel");
    let fuel_a_wall = split.outputs.iter().find(|q| q.0 as usize == p.sub[pcore::life::FUEL]).map_or(0.0, |q| q.1 as f64) * per_kg(w) * chem.subs[p.sub[pcore::life::FUEL]].molar_mass;
    let gut = ph.c_capacity * 10f64.powf(g[fauna::ENZYME] as f64) * m * fuel_a_wall * DAY;
    let rest = ((ph.c_basal + ph.c_muscle) * m.powf(ph.upkeep_exp) + gut) * pace * ph.repair(g[REPAIR] as f64);
    let speed = (ph.walk_froude * w.cfg.gravity * (m / ph.rho_body).cbrt()).sqrt() * (rx.arrhenius(tb) / rx.arrhenius(fauna::WORK_T)).min(1.0);
    let walk = speed * ph.busy * DAY * ph.c_move * (m / ph.move_ref).powf(ph.move_exp) * m;
    let coat = ph.meeh * m.powf(2.0 / 3.0) / (ph.r_air + g[fauna::COAT] as f64 * ph.r_coat);
    (rest + walk).max(coat * (g[KEEP] as f64 - t).max(0.0) * DAY)
}

/// The energy a kg of the takers' store frees when burnt, J.
fn per_kg(w: &World) -> f64 {
    let p = Parts::of(&w.chem).expect("life's parts");
    let rx = &w.chem.reactions[p.burn];
    let per = rx.inputs.iter().find(|q| q.0 as usize == p.sub[pcore::life::FUEL]).map_or(1.0, |q| q.1 as f64);
    -w.chem.dh(p.burn) / per / w.chem.subs[p.sub[pcore::life::FUEL]].molar_mass
}

/// A slot with no cover to take starves by the law of upkeep: a store of three fifths of a month of its upkeep and
/// its walk (a kind that keeps no heat, on ground warm enough that it neither freezes nor sheds vapour) pays three
/// fifths of its first month, its members falling by their wear (1 − e^-wear) and by the two fifths unpaid, and none of
/// the second, when all die; their remains reach the ground and the books are kept. For every starting kind the month its store first runs short is the one the
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
    let store_most = w.cfg.phys.store_most * m;
    assert!(store <= store_most, "a store of {store} kg is over the most a member stores, {store_most} kg");
    for k in 1..=2 {
        testkit::takers_month(&mut w, i);
        n = if k == 1 { n * (1.0 - wear) * STORE_MONTHS } else { 0.0 };
        let f = of(&w, i);
        assert!((f.n[0] - n).abs() <= 1e-9 * n, "month {k}: {} members, the law says {n} (at {t} K)", f.n[0]);
        println!("month {k}: {:.3} members", f.n[0]);
    }
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
    let mut was = of(&w, i).n;
    for k in 0..50 {
        testkit::takers_month(&mut w, i);
        let f = of(&w, i);
        for (g, &n) in f.n.iter().enumerate() {
            assert!(n <= was[g], "month {k}, group {g}: {n} members after {}", was[g]);
            assert!(n >= N_WHOLE || n.fract() == 0.0, "month {k}, group {g}: {n} members, not whole");
            assert!(n > 0.0 || f.mol[g] == [0.0; 2], "month {k}, group {g}: a group of none holds {:?}", f.mol[g]);
            assert!(n == 0.0 || f.mol[g][BODY] > 0.0, "month {k}, group {g}: members with no body");
        }
        assert!((0..FS).all(|x| f.members(x) > 0.0 || f.root[x] == NO_ROOT), "month {k}: a slot of none is held");
        was = f.n;
    }
    let left: f64 = of(&w, i).n.iter().sum();
    println!("members left after 50 months: {left}; living kg {:.3e}", (0..FS).map(|x| of(&w, i).mol[x][BODY]).sum::<f64>() * w.chem.subs[10].molar_mass / (1.0 - WET_TISSUE));
    books.assert_kept(&w, 1e-9);
}

/// Members `n` of one eater of the scheme, searching `search` m²/s for `busy` s a day, chewing at `chew` W, reaching
/// all of `layers` layers.
fn eater(n: f64, (search, busy, chew): (f64, f64, f64), layers: usize) -> take::Eater {
    take::Eater { n, search, busy, chew, reach: vec![1.0; layers] }
}

/// The take in steps meets the exact answer (Holling's disk law over a month): one eater on one layer falls as
/// ln(D/D0) + a h (D − D0) = −(N/A) T a t (D the layer per m², a the area searched a s, h the handling time a kg, T the
/// working time a day, t in days), found by halving, within 1%; and with two layers the eater switches: of a hard
/// layer it reaches little of it takes far more once the one that filled its time runs out than the month's first rates
/// would give.
#[test]
fn the_take_in_steps_meets_the_exact_answer() {
    let (area, days, a, busy, chew) = (4e6, 30.0, 0.5, 43_200.0, 30.0);
    let (n, d0, breaking) = (200.0, 0.05, 4e4);
    let (taken, _) = take::scheme(&[eater(n, (a, busy, chew), 1)], (&[d0 * area], &[breaking]), area, days);
    let h = breaking / chew;
    let gap = |d: f64| (d / d0).ln() + a * h * (d - d0) + n / area * busy * a * days;
    let (mut lo, mut hi) = (0.0, d0);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if gap(mid) > 0.0 { hi = mid } else { lo = mid }
    }
    let exact = (d0 - lo) * area;
    println!("one layer: the steps take {:.4e} kg, the exact answer {exact:.4e} kg (left {:.1}%)", taken[0][0], 100.0 * lo / d0);
    assert!((taken[0][0] - exact).abs() <= 0.01 * exact && lo < 0.8 * d0, "the steps took {} kg, the exact answer is {exact} kg", taken[0][0]);
    // A layer that fills its handling time (a D h = 10) and runs out in about ten days, and one it reaches a tenth of.
    let (first_layer, second, w2) = ((0.2 * area, 3e3), (0.01 * area, 3e3), 0.1);
    let mut both = eater(n, (a, busy, chew), 2);
    both.reach[1] = w2;
    let (taken, _) = take::scheme(&[both], (&[first_layer.0, second.0], &[first_layer.1, second.1]), area, days);
    let slow = 1.0 + a * (first_layer.0 * first_layer.1 + w2 * second.0 * second.1) / area / chew;
    let first = n * busy * a * w2 * second.0 / area / slow * days;
    println!("two layers: the first {:.3e} of {:.3e} kg taken; of the second {:.3e} kg, at the month's first rates {first:.3e} kg", taken[0][0], first_layer.0, taken[0][1]);
    assert!(taken[0][0] > 0.9 * first_layer.0 && taken[0][1] > 2.0 * first, "the eater did not switch: {:?}", taken[0]);
}

/// A taker takes no more than it reaches: a slot of many members asks far more than a cell's cover holds within its
/// band and over its bite's floor; each layer loses no more than that reachable stock (`contract::reached` of what it
/// shows), the cover keeps the rest as a refuge, and the books are kept.
#[test]
fn a_taker_takes_no_more_than_it_reaches() {
    let mut w = testkit::living(8, 12);
    let i = testkit::mild_land(&w);
    (w.life.env[i][1], w.life.env[i][3]) = (1.0, 0.0);
    let (row, ph) = (3, w.cfg.phys);
    let (g, m) = (TAKER_KINDS[row].genes, fauna::adult(&TAKER_KINDS[row].genes));
    let len = (m / ph.rho_body).cbrt();
    let reach = 10f64.powf(g[fauna::REACH] as f64);
    let band = (reach / 10f64.powf(fauna::BAND), (reach * 10f64.powf(fauna::BAND)).min(ph.reach_len * len));
    let before: Vec<(f64, f64)> = pcore::contract::slots(&w, w.life.cell_at(i)).iter().map(|&(l, h)| (shown(&w, i, l), pcore::contract::reached(band, ph.bite_floor * len, h))).collect();
    let books = Books::now(&w);
    testkit::takers(&mut w, i, row, (1e4, 0.1 * m));
    testkit::takers_month(&mut w, i);
    let after: Vec<f64> = pcore::contract::slots(&w, w.life.cell_at(i)).iter().map(|&(l, _)| shown(&w, i, l)).collect();
    assert!(before.len() == after.len() && before.iter().any(|b| b.1 > 0.0), "no layer within reach: {before:?}");
    for ((kg, r), now) in before.iter().zip(&after) {
        println!("a layer of {kg:.3e} kg, {:.0}% within reach: {now:.3e} kg left", 100.0 * r);
        assert!(*now >= kg * (1.0 - r) * (1.0 - 1e-9), "a layer of {kg} kg, {r} of it within reach, kept {now} kg");
    }
    assert!(after.iter().zip(&before).any(|(a, b)| *a < b.0), "nothing was taken");
    books.assert_kept(&w, 1e-9);
}

/// The kg that `layer` of cell `i` shows.
fn shown(w: &World, i: usize, layer: pcore::contract::Layer) -> f64 {
    pcore::contract::shown(w, i, w.life.cell_at(i), layer).map_or(0.0, |s| s.kg)
}

/// The way to the liquid (`fauna::way::fetch`, `contract::ways_to_liquid`): a slot short of liquid fetches it from the nearest
/// open liquid at the work of the walks there and back, which grows with the way as the law of walking says (twice
/// the way, twice the work); beyond what the day's hours can walk it fetches less than it is short, and its work is cut
/// for the rest. The field of ways is none at the cells with open liquid and the shortest way along the links over land
/// elsewhere: no land cell's way is longer than a neighbour's and the link between them.
#[test]
fn the_way_to_the_liquid_costs_the_walk() {
    let (w, _) = start();
    let ph = &w.cfg.phys;
    let m = fauna::adult(&TAKER_KINDS[3].genes);
    let len = (m / ph.rho_body).cbrt();
    let (speed, per_m) = ((ph.walk_froude * w.cfg.gravity * len).sqrt(), ph.c_move * (m / ph.move_ref).powf(ph.move_exp) * m);
    let (n, short, mm, days) = (10.0, 3e4, 0.018, 30.0);
    let at = |way: f64| fauna::way::fetch((n, m, speed, per_m), (short, mm, days), way, ph.dry_most);
    let (near, far) = (at(1e3), at(2e3));
    let trips = short * mm / (n * days) / (ph.dry_most * m);
    println!("{trips:.2} trips a day; 1 km: {near:?}; 2 km: {far:?}");
    assert!((near.mol - short).abs() <= 1e-9 * short && (far.work - 2.0 * near.work).abs() <= 1e-9 * far.work, "the walks' work does not go as the way: {near:?}, {far:?}");
    assert!((near.work - trips * 2e3 * per_m * n * days).abs() <= 1e-9 * near.work, "the walks' work is not the law of walking's");
    let reach = pcore::units::DAY * speed / (2.0 * trips);
    let beyond = at(2.0 * reach);
    println!("its day walks {reach:.0} m to the liquid at most; at twice that it fetches {beyond:?}");
    assert!((beyond.mol - 0.5 * short).abs() <= 1e-9 * short, "beyond its reach a slot fetched {} of {short} mol", beyond.mol);
    let mut w = testkit::living(8, 1);
    pcore::step(&mut w);
    let cells = w.mesh.cells();
    let field = |need: f64| (0..cells.n()).map(|i| pcore::contract::to_liquid(&w, i, need)).collect::<Vec<f64>>();
    let mixed = |v: &[f64]| v.contains(&0.0) && v.iter().any(|&d| d > 0.0 && d.is_finite());
    let need = (0..pcore::contract::LEVELS as i32).map(|k| 10f64.powi(k)).find(|&x| mixed(&field(x))).expect("a field of ways with some cells at the liquid and some off it");
    let way = field(need);
    println!("ways to {need:e} mol a month: {:?}", way.iter().map(|d| d.round()).collect::<Vec<_>>());
    for i in (0..cells.n()).filter(|&i| w.is_land(i) && way[i] > 0.0) {
        let (links, around) = cells.around(i);
        let best = links.iter().zip(around).map(|(&l, &j)| way[j as usize] + cells.dist()[l as usize]).fold(f64::INFINITY, f64::min);
        assert!((way[i] - best).abs() <= 1e-9 * best.max(1.0), "cell {i}: a way of {} m where a neighbour gives {best} m", way[i]);
    }
}

/// A body at rest sits in a shell of what lies on the ground (`fauna::shell`): a small warm body's loss falls as the
/// remains it gathers grow, toward the loss of a shell without end, 4π k r ΔT; with nothing lying it is the open loss.
/// A large warm body's open loss is already under the heat its rest makes, so a shell changes nothing it burns.
#[test]
fn a_small_body_rests_in_what_lies_on_the_ground() {
    let ph = pcore::Config::default().phys;
    let (dt, four_pi) = (35.0, 4.0 * std::f64::consts::PI);
    for row in [0, 1] {
        let g = TAKER_KINDS[row].genes;
        let m = fauna::adult(&g);
        let area = ph.meeh * m.powf(2.0 / 3.0);
        let coat = area / (ph.r_air + g[fauna::COAT] as f64 * ph.r_coat);
        let floor = four_pi * ph.litter_k * (area / four_pi).sqrt();
        let losses: Vec<f64> = [0.0, 0.01, 0.1, 1.0, 1e6].iter().map(|&kg| fauna::shell(&ph, area, coat, kg) * dt).collect();
        println!("{m:.2} kg: open {:.3} W; in 0, 10 g, 100 g, 1 kg, 1000 t of remains {losses:.3?} W; a shell without end {:.3} W", coat * dt, floor * dt);
        assert!((losses[0] - coat * dt).abs() <= 1e-12 * coat * dt, "with nothing lying a body loses {} W, in the open {} W", losses[0], coat * dt);
        assert!(losses.windows(2).all(|p| p[1] < p[0]), "the loss does not fall as the shell grows: {losses:?}");
        let end = 1.0 / (1.0 / coat + 1.0 / floor) * dt;
        assert!((losses[4] - end).abs() <= 0.01 * end, "a deep shell loses {} W, a shell without end {end} W", losses[4]);
    }
    let g = TAKER_KINDS[3].genes;
    let m = fauna::adult(&g);
    let open = ph.meeh * m.powf(2.0 / 3.0) / (ph.r_air + g[fauna::COAT] as f64 * ph.r_coat) * (g[fauna::KEEP] as f64 - 275.0);
    let chem = worldgen::chemistry0().0;
    let rx = &chem.reactions[Parts::of(&chem).expect("life's parts").burn];
    let pace = rx.arrhenius(g[fauna::KEEP] as f64) / rx.arrhenius(ph.t_cat);
    let rest = m.powf(ph.upkeep_exp) * (ph.c_basal + ph.c_muscle) * pace / DAY;
    println!("{m:.0} kg at 275 K: open loss {open:.0} W, its rest makes {rest:.0} W");
    assert!(rest > open, "a large body loses more in the open than its rest makes: {open} W against {rest} W");
}

/// A mouth breaks what its jaw can rupture (`contract::cut`): a jaw of no force cuts nothing; it cuts a stem of width
/// d where its force is the frame's stress for a cut over the solid wall of the stem's section, so of a frame whose main
/// stem is wider it takes the share (d / width)^`thin_exp`, which grows with the jaw's force and so with the body's
/// mass; a frame no wider than d it takes whole.
#[test]
fn a_mouth_cuts_what_its_jaw_can_rupture() {
    let w = testkit::small_world();
    let (ph, p) = (&w.cfg.phys, Parts::of(&w.chem).expect("life's parts"));
    let shear = w.chem.subs[p.sub[pcore::life::WALL]].shear;
    assert_eq!(pcore::contract::cut(&w, 0.1, 0.0), 0.0, "a jaw of no force cut something");
    let mut last = 0.0;
    for row in [0, 4, 1, 2, 3] {
        let m = fauna::adult(&TAKER_KINDS[row].genes);
        let force = ph.jaw * m.powf(ph.jaw_exp);
        let d = (4.0 * force / (std::f64::consts::PI * shear * ph.stem_solid)).sqrt();
        let share = pcore::contract::cut(&w, 0.2, force);
        println!("{m:.2} kg: a jaw of {force:.0} N cuts stems up to {:.2} mm, {:.1}% of a frame whose main stem is 0.2 m wide", d * 1e3, share * 100.0);
        assert!((share - (d / 0.2).powf(ph.thin_exp)).abs() <= 1e-12 && share > last, "{m} kg: a share {share} of the frame");
        assert_eq!(pcore::contract::cut(&w, 0.5 * d, force), 1.0, "a frame thinner than d was not taken whole");
        last = share;
    }
}

/// The newborn mass a mother bears a unit of time over her cap, where her newborns hold the share `child` of her mass:
/// her unborn grow from nothing by dm/dt = a m^(3/4) (1 − (m/M)^(1/4)) (West, Brown and Enquist 2001), found here step
/// by step in u = (m/M)^(1/4), du/dt = a (1 − u) / (4 M^(1/4)), with a = M = 1; her cap feeds at once the litter whose
/// greatest need over its growth is her cap; she bears litter after litter.
fn bearing(child: f64) -> f64 {
    let (end, steps) = (child.powf(0.25), 100_000);
    let (mut t, mut most) = (0.0, 0.0f64);
    for k in 0..steps {
        let u = (k as f64 + 0.5) / steps as f64 * end;
        t += 4.0 / (1.0 - u) * end / steps as f64;
        let at = (k + 1) as f64 / steps as f64 * end;
        most = most.max(at.powi(3) * (1.0 - at));
    }
    child / most / t
}

/// Newborns grow at the law every body grows by, and the books are kept through births and growth: a slot of adults
/// with a store over the most it keeps, on a cell of cover rich in body matter, bears in its first month the living mass
/// its adults' cap feeds litter after litter (`bearing`; the cap: Kleiber's rest (`c_basal` + `c_muscle`)
/// m^`upkeep_exp` at the pace of its body's temperature over the energy a kg of living tissue holds and costs,
/// `Parts::living_cost`), as newborns of `CHILD` of their mass; over the next months its young grow and join its
/// adults, every count under `N_WHOLE` is whole, and every element and the energy are kept.
#[test]
fn newborns_grow_by_the_law_of_growth_and_the_books_are_kept() {
    let mut w = testkit::living(8, 12);
    let i = testkit::mild_land(&w);
    (w.life.env[i][1], w.life.env[i][3]) = (1.0, 0.0);
    let (row, ph) = (1, w.cfg.phys);
    let g = TAKER_KINDS[row].genes;
    let (m, child) = (fauna::adult(&g), fauna::adult(&g) * 10f64.powf(g[fauna::CHILD] as f64));
    let books = Books::now(&w);
    testkit::takers(&mut w, i, row, (MANY, 2.0 * ph.store_most * m));
    let p = Parts::of(&w.chem).expect("life's parts");
    let rx = &w.chem.reactions[p.burn];
    let pace = rx.arrhenius(g[KEEP] as f64) / rx.arrhenius(ph.t_cat);
    let cap = (ph.c_basal + ph.c_muscle) * m.powf(ph.upkeep_exp) * pace / p.living_cost(&w.chem, &ph);
    let mm = fauna::mm(&w.chem, &p);
    testkit::takers_month(&mut w, i);
    let f = of(&w, i);
    let (built, law) = (f.living_of(&mm, FS), cap * bearing(child / m) * f.n[0] * w.cfg.days_per_month as f64);
    println!("month 1: {:.1} adults bore {:.1} young, {built:.4e} kg; the law {law:.4e} kg (the share {:.4} of their cap)", f.n[0], f.n[FS], bearing(child / m));
    assert!((built - law).abs() <= 1e-6 * law, "the newborns hold {built} kg, the law of growth gives {law} kg");
    assert!((f.n[FS] - built / child).abs() <= 1e-9 * f.n[FS], "{} newborns of {built} kg at {child} kg each", f.n[FS]);
    for k in 2..=6 {
        testkit::takers_month(&mut w, i);
        let f = of(&w, i);
        println!("month {k}: {:.1} adults, {:.1} young", f.n[0], f.n[FS]);
        assert!(f.n.iter().all(|&n| n >= N_WHOLE || n.fract() == 0.0), "month {k}: a count under {N_WHOLE} not whole: {:?}", f.n);
    }
    assert!(of(&w, i).n[0] > 0.0 && of(&w, i).n[FS] > 0.0, "no adults or no young after six months");
    books.assert_kept(&w, 1e-9);
}

/// A body works at the pace of its own temperature (laws.md 3b): of two kinds alike but for the heat they keep, one at
/// its ground's temperature and one held at `WORK_T` in a thick coat, on the same cell, the cold one takes the share
/// pace(t) / pace(`WORK_T`) of what the warm one takes, the share its upkeep is of the warm one's: being cold does
/// not multiply what it takes over what it burns.
#[test]
fn a_cold_body_takes_in_the_proportion_it_burns() {
    let mut w0 = testkit::living(8, 12);
    let i = testkit::mild_land(&w0);
    (w0.life.env[i][1], w0.life.env[i][3]) = (1.0, 0.0);
    let t = pcore::contract::place(&w0, i).t;
    let cover = |w: &World| pcore::cover::per_m2(w, pcore::cover::Cover::kg)[i] + pcore::cover::lying(w)[i];
    let taken = |keep: f32, coat: f32| {
        let mut w = w0.clone();
        let mut genes = TAKER_KINDS[COLD_ROW].genes;
        (genes[KEEP], genes[fauna::COAT]) = (keep, coat);
        let m = fauna::adult(&genes);
        assert!(fauna::place(&mut w, i, (genes, COLD_ROW as u32), (1.0, 0.1 * m)));
        let before = cover(&w);
        testkit::takers_month(&mut w, i);
        before - cover(&w)
    };
    let (cold, warm) = (taken(250.0, 0.0), taken(fauna::WORK_T as f32, 2.0));
    let p = Parts::of(&w0.chem).expect("life's parts");
    let rx = &w0.chem.reactions[p.burn];
    let law = rx.arrhenius(t) / rx.arrhenius(fauna::WORK_T);
    println!("at {t:.1} K a cold body took {cold:.4e} kg/m2, a warm one {warm:.4e}: {:.4} of it; its upkeep is {law:.4} of the warm one's", cold / warm);
    assert!(warm > 0.0 && (cold / warm - law).abs() <= 1e-6 * law, "a cold body took {} of a warm one's take, its upkeep is {law} of the warm one's", cold / warm);
}
