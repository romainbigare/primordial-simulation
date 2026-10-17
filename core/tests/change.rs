//! Law 2 (Change): the rate law of the reactions, burning and rotting in the open, catalysts, the liquid's store of the
//! heat-absorbing gas, and what dead matter gives back to the ground.

use pcore::change::{self, Reactor};
use pcore::chem::Chemistry;
use pcore::units::DAY;
use worldgen::testkit::{hold_ground, in_all, rock_outside_matter, node_totals, open_air, put, start, Books, ABSORBER, AIR_COLUMN, DISSOLVED, FIX_GAS, LIQUID, OPEN_CAP, OX, RICH, ROCK};

#[test]
fn hot_reacts_faster() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    let mut used = |t: f64| {
        let mut n = vec![0.0; chem.ns()];
        let mut gas = vec![0.0; chem.ns()];
        n[RICH] = 1000.0;
        gas[OX] = 1e6;
        let mut e = t * 1e12; // huge heat capacity: temperature stays put
        // (2.5e4 s: with the solid's litter constant 4 times higher, 1e5 s burns the hot node out and the ratio cannot show.)
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e12, None, 0.0, 2.5e4);
        1000.0 - n[RICH]
    };
    let (cold, hot) = (used(300.0), used(450.0));
    assert!(cold > 0.0 && hot > 100.0 * cold, "cold {cold}, hot {hot}");
}

#[test]
fn hot_energy_rich_node_keeps_burning() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    let mut n = vec![0.0; chem.ns()];
    let mut gas = vec![0.0; chem.ns()];
    n[RICH] = 1000.0;
    gas[OX] = 5000.0;
    gas[FIX_GAS] = 20000.0;
    let mut e = 650.0 * 1000.0 * chem.subs[RICH].cp;
    let (el0, en0) = node_totals(&chem, &n, &gas, e);
    for _ in 0..4 {
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 0.0, None, 0.0, 3.0 * 86_400.0);
    }
    let (el1, en1) = node_totals(&chem, &n, &gas, e);
    assert!(n[RICH] < 10.0 && gas[ABSORBER] > 500.0, "left {} released {}", n[RICH], gas[ABSORBER]);
    assert!(n.iter().chain(&gas).all(|&a| a >= 0.0));
    for j in 0..chem.n_elements {
        assert!((el1[j] - el0[j]).abs() <= 1e-10 * el0[j].max(1.0), "element {j}");
    }
    assert!((en1 - en0).abs() <= 1e-9 * en0.abs(), "energy {en0} -> {en1}");
}

/// A ground node at 600 K holding the energy-rich solid (the one the light reaction makes) with
/// the burning gas above it reacts in the very next step, whichever turn its block has.
#[test]
fn hot_node_reacts_in_the_next_step() {
    use pcore::chem::Phase;
    let base = worldgen::new_world(1, 16, 16);
    let chem = &base.chem;
    // The roles are found from the chemistry as `worldgen/src/life.rs` does.
    let at = |k: u16| chem.phase(k as usize, 290.0);
    let lit = (0..chem.reactions.len()).find(|&r| chem.reactions[r].light && chem.dh(r) > 0.0).expect("a light reaction");
    let rich = chem.reactions[lit].outputs.iter().find(|p| at(p.0) == Phase::Solid).expect("a solid made").0 as usize;
    let burn = (0..chem.reactions.len())
        .find(|&r| chem.dh(r) < 0.0 && chem.reactions[r].inputs.iter().any(|p| p.0 as usize == rich) && chem.reactions[r].inputs.iter().any(|p| at(p.0) == Phase::Gas))
        .expect("a burning reaction");
    let (n, na, i) = (base.grid.n(), base.grid.na(), 17);
    let j = base.grid.air_of(i);
    for tick in 0..40 {
        let mut w = base.clone();
        w.scratch = Default::default();
        w.day = 3 * tick;
        for k in 0..w.chem.ns() {
            w.ground[k * n + i] = 0.0;
            w.bed[k * n + i] = 0.0;
        }
        w.ground[rich * n + i] = 1000.0;
        let tot: f64 = (0..w.chem.ns()).map(|k| w.air[k * na + j]).sum();
        for p in w.chem.reactions[burn].inputs.clone().iter().filter(|p| at(p.0) == Phase::Gas) {
            w.air[p.0 as usize * na + j] += 0.2 * tot;
        }
        w.e_ground[i] = w.energy_ground(i, 600.0);
        pcore::change::step(&mut w, 3.0 * 86_400.0);
        assert!(w.ground[rich * n + i] < 999.0, "tick {tick}: amount left {}", w.ground[rich * n + i]);
    }
}

/// A body's catalyst works below the temperature where it loses its shape, and a few K above it
/// the reaction goes about as fast as bare.
#[test]
fn catalysts_fail_when_hot() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    let mut cat = vec![1.0; chem.reactions.len()];
    cat[1] = 1e6;
    let mut used = |t: f64, c: Option<(&[f64], f64, f64)>| {
        let mut n = vec![0.0; chem.ns()];
        let mut gas = vec![0.0; chem.ns()];
        (n[RICH], gas[OX]) = (1000.0, 1e6);
        let mut e = t * 1e12; // huge heat capacity: temperature stays put
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e12, c, 0.0, 1.0);
        1e6 - gas[OX]
    };
    let (cool, cool_bare) = (used(310.0, Some((&cat, 320.0, 3.0))), used(310.0, None));
    let (hot, hot_bare) = (used(340.0, Some((&cat, 320.0, 3.0))), used(340.0, None));
    assert!(cool > 1e5 * cool_bare && hot < 1.2 * hot_bare, "310 K: {cool:e} vs bare {cool_bare:e}; 340 K: {hot:e} vs bare {hot_bare:e}");
}

/// Cover is fuel when hot: on a cell held at 700 K for a month, a light-eater node loses most of
/// its energy-rich solid (it reacts with the gas, in the body or, once the body dies, in the
/// ground), while the same cell left as it was keeps it.
#[test]
fn hot_cover_burns() {
    let run = |hot: bool| {
        let mut w = worldgen::new_world(1, 16, 16);
        // (No growth: building body matter takes the solid too, and this is about the burning. And no body can pay to move: with liquid
        // in the pores of the ground the cells they feel are not the dry ones of before, and a node that walks out of the hot cell
        // does not burn.)
        w.cfg.phys.c_build = 0.0;
        w.cfg.phys.c_move = 1e12;
        pcore::step(&mut w);
        worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
        let (_, rich, _, _) = w.chem.life_roles().expect("roles");
        let i = w.life.alive().max_by(|a, b| a.count.total_cmp(&b.count)).expect("a node").cell as usize;
        let n = w.grid.n();
        let held = |w: &pcore::World| w.ground[rich * n + i] + w.life.alive().filter(|b| b.cell as usize == i).map(|b| b.store[rich]).sum::<f64>();
        let before = held(&w);
        // A month, the cell heated back to 700 K before each day (booked as an edit).
        for _ in 0..w.cfg.days_per_month {
            if hot {
                hold_ground(&mut w, i, 700.0);
            }
            pcore::day(&mut w);
        }
        assert_eq!(pcore::check::broken(&w), None);
        held(&w) / before
    };
    let (hot, cold) = (run(true), run(false));
    eprintln!("energy-rich solid left: {hot:.3} hot, {cold:.3} as it was");
    assert!(hot < 0.5 && cold > 0.9, "energy-rich solid left: {hot:.3} hot, {cold:.3} as it was");
}

/// One m2 of wet land (0.4 m of liquid in its pores) under the air at 400 ppm, with `rock` mol of
/// the rock; the reactions run for `dt` at 288 K. Returns the salt made per second and the books
/// (elements, energy) before and after.
fn wet_land(chem: &Chemistry, rock: f64, dt: f64) -> (f64, bool) {
    let mut rc = Reactor::new(chem);
    let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
    (n[LIQUID], n[ROCK]) = (0.4 * 1000.0 / 0.018, rock);
    (gas[FIX_GAS], gas[OX], gas[ABSORBER]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, 400e-6 * AIR_COLUMN);
    let cap = 1e15;
    let mut e = 288.0 * (cap + n.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>());
    let (el0, en0) = node_totals(chem, &n, &gas, e);
    rc.react(chem, &mut n, Some(&mut gas), &mut e, cap, None, 0.0, dt);
    let (el1, en1) = node_totals(chem, &n, &gas, e);
    let closed = (0..chem.n_elements).all(|j| (el1[j] - el0[j]).abs() <= 1e-10 * el0[j].max(1.0)) && (en1 - en0).abs() <= 1e-9 * en0.abs();
    (n[DISSOLVED] / dt, closed && n.iter().chain(&gas).all(|&a| a >= 0.0))
}

/// A sweep of the rock across the amount of the gas over the cell (144 mol/m2 at 400 ppm, where the
/// old law switched from the gas's amount to the rock's and the rate fell about 2,500 times): the
/// salt made goes with the rock, smoothly, and the books close at every step.
#[test]
fn no_jump_when_the_scarcest_input_changes() {
    let (chem, _) = worldgen::chemistry0();
    let co2 = 400e-6 * AIR_COLUMN;
    let (mut last, mut rock) = (None, 1.0);
    while rock < 1e5 {
        let (rate, closed) = wet_land(&chem, rock, DAY);
        assert!(closed, "the books at {rock} mol of rock");
        assert!(rate > 0.0, "no salt at {rock} mol of rock");
        if let Some((r0, x0)) = last {
            // 2% more rock: never less salt, never more than 2% more (a jump would be a factor of 2,500).
            let (up, more) = (rate / r0, rock / x0);
            assert!(up >= 1.0 && up <= more * 1.0001, "rock {x0:.1} -> {rock:.1} mol (gas {co2:.0}): rate x{up}");
        }
        last = Some((rate, rock));
        rock *= 1.02;
    }
    // Across the old break the rate is the rock's: 1% either side of the gas's amount, 2% apart.
    let (below, _) = wet_land(&chem, 0.99 * co2, DAY);
    let (above, _) = wet_land(&chem, 1.01 * co2, DAY);
    assert!((above / below - 1.01 / 0.99).abs() < 1e-3, "across the gas's amount: x{}", above / below);
}

/// Dead matter in the open goes with what there is of it (first order: the same share lost for 1
/// and 1,000 mol) at its litter constant, 0.27 a year at 290 K for the energy-rich solid (Olson
/// 1963; `docs/numbers.md`), the oxygen-like gas at its share of the air.
#[test]
fn litter_goes_first_order_at_its_constant() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    let year = 3.156e7;
    let mut lost = |a: f64| {
        let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
        n[RICH] = a;
        (gas[FIX_GAS], gas[OX], gas[ABSORBER]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, 400e-6 * AIR_COLUMN);
        let mut e = 290.0 * 1e15;
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, year);
        1.0 - n[RICH] / a
    };
    let (few, many) = (lost(1.0), lost(1000.0));
    assert!((many - few).abs() < 1e-3 * few, "1 mol lost {few}, 1,000 mol lost {many}");
    // 0.27 a year, the rest of the bare decay (reaction 4 and 9, far slower) aside.
    assert!((0.235..0.25).contains(&few), "share lost in a year {few} (1 - e^-0.27 = 0.237)");
}

/// A salt held far past its balance (an eater's rock salt: 4.35 mol in 1,250 mol of liquid at 305 K) falls out at the pace
/// of its rate law, its share squared, over a long step too: after 10 days what is left is the integral of that law,
/// 1 / sqrt(1 + 2 z) of it (4.5%), and the same in one call or ten. (It was 0: the rate was held over the step.)
#[test]
fn a_salt_past_its_balance_falls_out_by_its_rate_law_over_a_long_step() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    let mut left = |calls: usize| {
        let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
        (n[LIQUID], n[DISSOLVED]) = (1250.0, 4.35);
        (gas[FIX_GAS], gas[OX], gas[ABSORBER]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, 400e-6 * AIR_COLUMN);
        let mut e = 305.0 * 1e7;
        for _ in 0..calls {
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e7, None, 0.0, 10.0 * DAY / calls as f64);
        }
        n[DISSOLVED]
    };
    let rx = &chem.reactions[23];
    let z = rx.k * (-rx.ea / 305.0f64).exp() * (4.35f64 / 1250.0).powi(2) * 10.0 * DAY;
    let (one, ten, law) = (left(1), left(10), 4.35 / (1.0 + 2.0 * z).sqrt());
    println!("salt left after 10 days: one call {one:.4}, ten calls {ten:.4}, the rate law {law:.4} mol");
    assert!((0.9..1.25).contains(&(one / law)) && (one / ten - 1.0).abs() < 0.05, "one call {one}, ten calls {ten}, the law {law}");
}

/// Dead matter gives the minerals back: in the open at 290 K it rots (0.2 a year, the litter constant) and every mol that goes leaves three
/// of the nitrogen mineral and one of lime (the rock element, which has no liquid here to dissolve in) and no nitrogen gas; burning hot
/// (1000 K) the mineral it leaves breaks down as it is made, and most of the nitrogen of the matter goes to the air as the gas, while
/// the rock element stays in the solids (lime, then what the heat makes of it). Elements and energy are kept exactly.
#[test]
fn rotting_gives_the_minerals_back_and_burning_hot_gives_the_gas_to_the_air() {
    let (chem, init) = worldgen::chemistry0();
    let (m, minerals) = (chem.matter().expect("matter"), chem.minerals());
    let (mineral, lime, rock) = (minerals[0].0, 10usize, 5usize);
    let gas_n = 1usize;
    let mut rc = change::Reactor::new(&chem);
    let mut run = |t: f64, secs: f64| {
        let (mut n, mut gas, mut e) = open_air(&chem, &init, t);
        n[m] = 1000.0;
        let (el0, en0) = node_totals(&chem, &n, &gas, e);
        let gas0 = gas.clone();
        // (A product waits for the next call to react further: the mineral breaks down in the calls after the one that makes it.)
        for _ in 0..3 {
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, OPEN_CAP, None, 0.0, secs / 3.0);
        }
        let (el1, en1) = node_totals(&chem, &n, &gas, e);
        for j in 0..chem.n_elements {
            assert!((el1[j] - el0[j]).abs() <= 1e-9 * el0[j].max(1.0), "element {j}: {} -> {}", el0[j], el1[j]);
        }
        assert!((en1 - en0).abs() <= 1e-9 * en0.abs(), "energy {en0} -> {en1}");
        // The rock element outside the matter, in the solids (mol), by the elements of what is left.
        let ca: f64 = (0..chem.ns()).filter(|&k| k != m).map(|k| chem.subs[k].elems[rock] as f64 * n[k]).sum();
        (1000.0 - n[m], n[mineral], gas[gas_n] - gas0[gas_n], n[lime], ca)
    };
    let (lost, back, gas, lime_back, ca) = run(290.0, 365.0 * 86_400.0);
    println!("a year at 290 K: {lost:.2} mol of matter rotted, {back:.2} mol of the mineral back, {lime_back:.2} mol of lime, {gas:.4} mol of nitrogen gas");
    assert!(lost > 170.0 && lost < 205.0, "{lost}");
    assert!((back / (3.0 * lost) - 1.0).abs() < 1e-6 && gas.abs() < 1e-6 * lost, "mineral {back}, gas {gas}");
    // (A little of the lime meets the liquid the rotting makes and the gas, and is the salt: the rock element is counted in all solids and liquids.
    // Under the smooth rate of Law 2 that goes with the lime, its contact: 0.7% of it in this year, so the lime is held to 1%.)
    assert!((lime_back / lost - 1.0).abs() < 1e-2 && (ca / lost - 1.0).abs() < 1e-6, "lime {lime_back}, the rock element {ca}, for {lost} mol of matter");
    let (lost, back, gas, _, ca) = run(1000.0, 1e5);
    println!("hot, 1000 K: {lost:.1} mol of matter burned, {back:.1} mol of the mineral, {gas:.1} mol of nitrogen gas, {ca:.1} mol of the rock element in solids");
    assert!(lost > 990.0, "{lost}");
    assert!(gas > 0.8 * 3.0 * lost && back < 0.2 * 3.0 * lost, "mineral {back}, gas {gas} for {lost} mol of matter");
    assert!((ca / lost - 1.0).abs() < 1e-6, "the rock element in solids {ca} for {lost} mol of matter");
}

/// Dead matter on the ground returns its nitrogen and its rock element to the ground: in a world with no body, a year, the matter that
/// went is in the nitrogen mineral three for one (the mineral dissolves and runs with the liquid: it is counted over the whole ground)
/// and in the rock element outside the matter one for one (as lime, or the salt that lime and the liquid and gas make of it), more than
/// the same world without the matter makes (the open air fixes next to nothing), and elements and energy are kept.
#[test]
fn dead_matter_on_the_ground_returns_the_minerals_to_the_ground() {
    let (mut w, i) = start();
    let (mineral, per) = w.chem.minerals()[0];
    let m = w.chem.matter().expect("matter");
    // The same world with nothing on the ground: what the open makes of the mineral in a year.
    let mut twin = w.clone();
    let bare = in_all(&twin, mineral);
    for _ in 0..12 {
        pcore::step(&mut twin);
    }
    let (bare, bare_rock) = (in_all(&twin, mineral) - bare, rock_outside_matter(&twin, i));
    put(&mut w, m, i, 1e8);
    let books = Books::now(&w);
    let (m_before, n_before) = (in_all(&w, m), in_all(&w, mineral));
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let (lost, gained, rock) = (m_before - in_all(&w, m), in_all(&w, mineral) - n_before - bare, rock_outside_matter(&w, i) - bare_rock);
    assert!(bare >= 0.0 && bare < 1e-3 * lost, "the open fixed {bare} mol in a year");
    println!("a year on the ground: {lost:.0} of {m_before:.0} mol of matter rotted, {gained:.0} mol of the mineral back, {rock:.0} mol of the rock element more than the twin's cell");
    assert!(lost > 0.02 * m_before, "it did not rot: {lost}");
    assert!((gained / (per * lost) - 1.0).abs() < 1e-4, "the mineral {gained} for the matter {lost}");
    // (In the cell, to a tenth: the salt the rotting's gas and liquid make of the lime moves with the liquid, to and from the cells
    // around; the ledger of elements below is exact.)
    assert!((rock / lost - 1.0).abs() < 0.1, "the rock element {rock} for the matter {lost}");
    books.assert_kept(&w, 1e-9);
}

/// The rock salt in the pores of the land (15 Oct 2026): worldgen sets no salt on land (the ground has no liquid at the start, and a cell
/// with no liquid holds none), and a year of rain on loose ground that is a tenth lime makes it by the same two reactions as the
/// low ground's liquid cover (22 and 23): the cells whose pores hold liquid hold the salt, in the amount at which the two go at equal rates for their
/// liquid and the gas over them (`Reactor::balance`, to a half); a cell whose pores are dry holds none.
#[test]
fn the_pores_of_the_land_hold_the_rock_salt_where_lime_and_liquid_meet() {
    let mut w = worldgen::new_world(1, 16, 16);
    let (n, na, salt, liquid) = (w.grid.n(), w.grid.na(), 23usize, 0usize);
    let land: Vec<usize> = (0..n).filter(|&i| w.surface_z(i) >= 0.0).collect();
    assert!(land.iter().all(|&i| w.ground[salt * n + i] == 0.0), "land started with the salt");
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let mut rc = change::Reactor::new(&w.chem);
    let (mut wet, mut dry, mut ratio) = (0, 0, Vec::new());
    for &i in &land {
        if w.ground[liquid * n + i] > 1e6 {
            // The balance at the cell's own liquid, warmth and gas.
            let (t, j) = (w.t_ground(i), w.grid.air_of(i));
            let mut amt: Vec<f64> = (0..w.chem.ns()).map(|k| w.ground[k * n + i]).collect();
            amt[salt] = 0.0;
            let share = w.grid.air_share();
            let gas: Vec<f64> = (0..w.chem.ns()).map(|k| w.air[k * na + j] * share).collect();
            rc.balance(&w.chem, &mut amt, &gas, t);
            if amt[salt] > 0.0 {
                wet += 1;
                ratio.push(w.ground[salt * n + i] / amt[salt]);
            }
        } else if w.ground[liquid * n + i] == 0.0 {
            dry += 1;
            assert_eq!(w.ground[salt * n + i], 0.0, "a dry cell holds the salt");
        }
    }
    ratio.sort_by(|a, b| a.total_cmp(b));
    println!("{wet} land cells with liquid in the pores, {dry} dry; the salt over its balance, the cells at the 10th, 50th, 90th percent: {:.2} {:.2} {:.2}", ratio[ratio.len() / 10], ratio[ratio.len() / 2], ratio[ratio.len() * 9 / 10]);
    assert!(wet > 20, "only {wet} wet land cells");
    assert!(ratio[ratio.len() / 2] > 0.5 && ratio[ratio.len() / 2] < 2.0, "the cells' salt is far from its balance: {}", ratio[ratio.len() / 2]);
}

/// Moles of liquid in 1 m3 of it.
const MOL_PER_M3: f64 = 1000.0 / 0.018;

/// One m2 of a liquid column `depth` m deep over the rock of the start's loose ground (a tenth of
/// 1 m by volume, 2,710 mol: the rate goes with it, Law 2's contact), with the gas of the air over it.
fn column(chem: &Chemistry, depth: f64, ppm: f64) -> (Vec<f64>, Vec<f64>) {
    let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
    n[LIQUID] = depth * MOL_PER_M3;
    n[ROCK] = 0.1 / chem.molar_volume(ROCK, pcore::chem::Phase::Solid);
    (gas[FIX_GAS], gas[2], gas[ABSORBER]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, ppm * 1e-6 * AIR_COLUMN);
    (n, gas)
}

/// The salt the chemistry holds per m2 at balance: `depth` m of liquid, `ppm` of the gas, `t` K.
fn at_balance(chem: &Chemistry, depth: f64, ppm: f64, t: f64) -> f64 {
    let (mut n, gas) = column(chem, depth, ppm);
    let set = Reactor::new(chem).balance(chem, &mut n, &gas, t);
    set.iter().find(|p| p.0 == DISSOLVED).map_or(0.0, |p| p.1)
}

/// Dissolving and settling balance in elements and in energy, from either side.
#[test]
fn the_two_reactions_close_elements_and_energy() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    for salt in [0.0, 5000.0] {
        let (mut n, mut gas) = column(&chem, 300.0, 400.0);
        n[DISSOLVED] = salt;
        let (t, cap) = (288.0, 1e15);
        let mut e = t * (cap + n.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>());
        let (el0, en0) = node_totals(&chem, &n, &gas, e);
        for _ in 0..30 {
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, cap, None, 0.0, 120.0 * DAY);
        }
        let (el1, en1) = node_totals(&chem, &n, &gas, e);
        assert!((n[DISSOLVED] - salt).abs() > 1.0, "it did react: {salt} -> {}", n[DISSOLVED]);
        for j in 0..chem.n_elements {
            assert!((el1[j] - el0[j]).abs() <= 1e-10 * el0[j].max(1.0), "element {j}");
        }
        assert!((en1 - en0).abs() <= 1e-9 * en0.abs(), "energy {en0} -> {en1}");
        assert!(n.iter().chain(&gas).all(|&a| a >= 0.0));
    }
}

/// The balance holds what the real carbonate chemistry says at the anchor (1 mol per m3 of the
/// liquid, 500 m deep, 400 ppm, 288 K). The salt cubed, over the rock it left (the reaction's
/// contact, Law 2), goes with the gas and falls with warmth as the reaction heat says (dH / R =
/// 4,847 K): the real law, [Ca][HCO3]^2 = K pCO2, with the rock in it.
#[test]
fn the_balance_is_the_anchored_one_and_moves_as_the_law_says() {
    let (chem, _) = worldgen::chemistry0();
    let rock = column(&chem, 500.0, 400.0).0[ROCK];
    let held = |salt: f64| salt.powi(3) / (rock - salt);
    let s = at_balance(&chem, 500.0, 400.0, 288.0);
    assert!((0.9..1.1).contains(&(s / 500.0)), "{} mol per m3 at the anchor", s / 500.0);
    let twice = held(at_balance(&chem, 500.0, 800.0, 288.0)) / held(s);
    assert!((twice - 2.0).abs() < 0.02, "twice the gas: x{twice}");
    let warm = held(at_balance(&chem, 500.0, 400.0, 298.0)) / held(s);
    let law = ((chem.dh(22) / 8.314) * (1.0 / 288.0 - 1.0 / 298.0)).exp();
    assert!((warm / law - 1.0).abs() < 0.03, "10 K warmer: x{warm}, the law says x{law}");
}

/// The store goes with the liquid, not with the gas over it: a cell with no liquid holds none; a
/// deeper column holds more (to the power 2/3, the liquid's share counting twice), a little less
/// per m3; salt left in a dry cell settles out at once.
#[test]
fn a_dry_cell_holds_none_and_a_deeper_column_holds_more() {
    let (chem, _) = worldgen::chemistry0();
    assert_eq!(at_balance(&chem, 0.0, 400.0, 288.0), 0.0);
    let (shallow, deep) = (at_balance(&chem, 50.0, 400.0, 288.0), at_balance(&chem, 500.0, 400.0, 288.0));
    assert!(deep > 4.0 * shallow && deep < 5.0 * shallow, "50 m: {shallow}, 500 m: {deep} (10^(2/3) = 4.64)");
    // Salt in a cell with no liquid but what it makes: settles to the rock within a day.
    let mut rc = Reactor::new(&chem);
    let (mut n, mut gas) = column(&chem, 0.0, 400.0);
    n[DISSOLVED] = 100.0;
    let mut e = 288.0 * 1e15;
    rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, DAY);
    assert!(n[DISSOLVED] < 1e-6, "salt left in a dry cell: {}", n[DISSOLVED]);
}

/// A long run of the two reactions in lumps of 120 days (the pace of a calm cell), the air held at
/// 400 ppm, settles to the amount `balance` gives, from none and from twice as much (made of the
/// same rock, so the same calcium in all), with the gas reached through the liquid's film (500 m
/// at 5e-5 m/s, `Reactor::film`): at the balance the salt gives the liquid as much gas as it takes.
#[test]
fn a_long_run_settles_to_the_balance() {
    let (chem, _) = worldgen::chemistry0();
    let want = at_balance(&chem, 500.0, 400.0, 288.0);
    let mut rc = Reactor::new(&chem);
    rc.film = 500.0 / 5e-5;
    for start in [0.0, 2.0 * want] {
        let (mut n, gas0) = column(&chem, 500.0, 400.0);
        (n[DISSOLVED], n[ROCK]) = (start, n[ROCK] - start);
        let mut e = 288.0 * 1e15;
        for _ in 0..(60 * 3) {
            let mut gas = gas0.clone();
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, 120.0 * DAY);
        }
        assert!((n[DISSOLVED] / want - 1.0).abs() < 0.05, "from {start}: {} against the balance {want}", n[DISSOLVED]);
    }
}

/// Under a liquid the dead matter burns with the oxygen-like gas dissolved there, which the
/// liquid's film renews (`Reactor::film`): a pile that uses little of what the film can bring
/// burns as in the open air; one that uses the gas faster than the film brings it, slower by
/// 1 / (1 + used x film / Henry's share), the series law.
#[test]
fn under_a_liquid_dead_matter_burns_as_its_film_lets_the_gas_through() {
    let (chem, _) = worldgen::chemistry0();
    let (rich, ox, t, dt, depth) = (5, 2, 288.0, DAY, 100.0);
    let film = depth / 5e-5;
    let burn = |pile: f64, film: f64| {
        let (mut n, mut gas) = column(&chem, depth, 400.0);
        n[rich] = pile;
        let mut rc = Reactor::new(&chem);
        rc.film = film;
        let (mut e, o2) = (t * 1e15, gas[ox]);
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, dt);
        (o2 - gas[ox]) / dt
    };
    let air: f64 = column(&chem, depth, 400.0).1.iter().sum();
    let eq = depth * MOL_PER_M3 * chem.henry(ox, t, 0.21 * AIR_COLUMN / air * pcore::units::P_REF);
    for pile in [10.0, 1e4] {
        let open = burn(pile, 0.0);
        let law = 1.0 / (1.0 + open * film / eq);
        assert!((burn(pile, film) / open / law - 1.0).abs() < 0.05, "{pile} mol: x{} against the law's x{law}", burn(pile, film) / open);
        assert!(if pile < 100.0 { law > 0.99 } else { law < 0.5 }, "{pile} mol: x{law}");
    }
}

/// With the air's gas halved, the salt follows down the balance and gives back the gas it held
/// (the buffer): for each unit the air loses the liquid gives back about 3 (cube root: 21% of 1,000).
#[test]
fn the_liquid_gives_back_the_gas_it_holds_when_the_air_thins() {
    let (chem, _) = worldgen::chemistry0();
    let s400 = at_balance(&chem, 500.0, 400.0, 288.0);
    let s200 = at_balance(&chem, 500.0, 200.0, 288.0);
    let air_loss = 200e-6 * AIR_COLUMN;
    // The salt holds 2 units of the gas each.
    let given = 2.0 * (s400 - s200);
    assert!(given > 2.0 * air_loss, "gave {given} for an air loss of {air_loss}");
}

/// A cell at balance is calm: in a world whose only reactions are these two (others, such as the
/// burning of a trace of gas, make warm cells react every step), over 39 steps (just under the 40
/// a cell waits between turns) each cell of the liquid reacts about once, not every step.
#[test]
fn a_cell_at_balance_does_not_react_every_step() {
    let (mut chem, init) = worldgen::chemistry0();
    chem.reactions = chem.reactions[22..].to_vec();
    let mut w = worldgen::terrain::build(1, 24, 24, chem, &init, pcore::Config::default());
    let n = w.grid.n();
    let wet = (0..n).filter(|&i| w.ground[DISSOLVED * n + i] > 0.0).count();
    let mut changes = 0;
    for _ in 0..39 {
        let before: Vec<f64> = w.ground[DISSOLVED * n..(DISSOLVED + 1) * n].to_vec();
        pcore::change::step(&mut w, 3.0 * DAY);
        w.day += 3;
        changes += (0..n).filter(|&i| w.ground[DISSOLVED * n + i] != before[i]).count();
    }
    eprintln!("{changes} changes of salt in 39 steps over {wet} cells");
    assert!(wet > 100, "{wet} cells hold salt");
    assert!(changes <= 3 * wet, "{changes} changes of salt in 39 steps over {wet} cells");
    assert!(changes >= wet / 2, "{changes} changes: the cells do react at their turn");
}
