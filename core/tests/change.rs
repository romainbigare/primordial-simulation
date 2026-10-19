//! Law 2 (Change): the rate law of the reactions, burning and rotting in the open, catalysts, the liquid's store of
//! the heat-absorbing gas in the dissolved mineral, and what dead matter gives back to the ground.

use pcore::change::{self, Reactor};
use pcore::chem::{Chemistry, Phase};
use pcore::store::{Class, Node};
use pcore::units::{DAY, DAY_S, P_REF, R_GAS, T_SURF};
use worldgen::testkit::{empty_cell, hold_ground, in_all, set_amount, set_heat, BURN, BURNING_GAS, node_totals, open_air, put, rock_outside_matter, start, Books, ABSORBER, AIR_COLUMN, DISSOLVED, FIX_GAS, LIQUID, MATTER, MINERAL, OPEN_CAP, OX, RICH, ROCK};
use pcore::AIR;

/// The default world's settings (its year: a reaction too slow to matter over it is frozen).
fn cfg() -> pcore::Config {
    pcore::Config::default()
}

/// A heat capacity so large that a node's temperature stays put, J/K.
const HUGE_CAP: f64 = 1e12;
/// The element of the rock (its cation), and the reaction that makes the dissolved mineral of the rock, the gas and
/// the liquid.
const ROCK_ELEMENT: usize = 5;
const DISSOLVING: usize = 22;
/// The reaction that turns the dissolved mineral back into them.
const SETTLING: usize = 23;
/// Moles of liquid in 1 m³ of it.
const MOL_PER_M3: f64 = 1000.0 / 0.018;

/// The starting air over 1 m², with `ppm` of the heat-absorbing gas, mol.
fn air_over_m2(chem: &Chemistry, ppm: f64) -> Vec<f64> {
    let mut gas = vec![0.0; chem.ns()];
    (gas[FIX_GAS], gas[OX], gas[ABSORBER]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, ppm * 1e-6 * AIR_COLUMN);
    gas
}

/// The energy-rich solid burns with the gas far faster when hot (2.5e4 s: long enough for the cold node to burn a
/// little, short enough for the hot one not to burn out).
#[test]
fn hot_reacts_faster() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem, &cfg());
    let mut used = |t: f64| {
        let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
        (n[RICH], gas[OX]) = (1000.0, 1e6);
        let mut e = t * HUGE_CAP;
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, HUGE_CAP, None, 0.0, 2.5e4);
        1000.0 - n[RICH]
    };
    let (cold, hot) = (used(300.0), used(450.0));
    assert!(cold > 0.0 && hot > 100.0 * cold, "cold {cold}, hot {hot}");
}

/// A hot node of the energy-rich solid, with its own heat capacity only, keeps itself hot by its burning and burns
/// out; no amount goes negative, and elements and energy are kept.
#[test]
fn hot_energy_rich_node_keeps_burning() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem, &cfg());
    let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
    (n[RICH], gas[OX], gas[FIX_GAS]) = (1000.0, 5000.0, 20000.0);
    let mut e = 650.0 * 1000.0 * chem.subs[RICH].cp;
    let (el0, en0) = node_totals(&chem, &n, &gas, e);
    for _ in 0..4 {
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 0.0, None, 0.0, 3.0 * DAY);
    }
    let (el1, en1) = node_totals(&chem, &n, &gas, e);
    assert!(n[RICH] < 10.0 && gas[ABSORBER] > 500.0, "left {} released {}", n[RICH], gas[ABSORBER]);
    assert!(n.iter().chain(&gas).all(|&a| a >= 0.0));
    for j in 0..chem.n_elements {
        assert!((el1[j] - el0[j]).abs() <= 1e-10 * el0[j].max(1.0), "element {j}");
    }
    assert!((en1 - en0).abs() <= 1e-9 * en0.abs(), "energy {en0} -> {en1}");
}

/// A ground node sleeps by its own rates (`clock::Sleep`): a node at 600 K holding the energy-rich solid with the
/// burning gas above it burns far within a step, so it reacts again at the next step; a node holding only rock, where
/// nothing reacts (the air holds none of the gas that burns in it alone), sleeps `sleep_days` after its first step.
#[test]
fn a_node_sleeps_by_its_own_rates() {
    let mut w = worldgen::new_world(1, 16, 16);
    let (na, hot, cold) = (w.mesh.level(AIR).n(), 17, 40);
    for i in [hot, cold] {
        empty_cell(&mut w, i);
    }
    set_amount(&mut w, Node::ground(hot), RICH, 1000.0);
    set_amount(&mut w, Node::ground(cold), ROCK, 1e5);
    let j = w.mesh.up(hot, 0, AIR);
    let tot: f64 = (0..w.chem.ns()).map(|k| w.a(k, j)).sum();
    let (ox, e) = (w.a(OX, j) + 0.2 * tot, w.energy_ground(hot, 600.0));
    set_amount(&mut w, Node::air(j), OX, ox);
    (0..na).for_each(|j| set_amount(&mut w, Node::air(j), BURNING_GAS, 0.0));
    set_heat(&mut w, Node::ground(hot), e);
    let (step, sleep) = (w.cfg.phys_days as u64 * DAY_S, w.cfg.sleep_days as u64 * DAY_S);
    let start = w.time;
    let mut cold_ran = Vec::new();
    while w.time < start + sleep + step {
        change::step(&mut w, step as f64);
        w.time += step;
        if w.time <= start + 2 * step {
            assert_eq!(w.sleep.since(Class::Ground, hot), w.time, "the hot node reacted in this step");
        }
        if w.sleep.since(Class::Ground, cold) == w.time {
            cold_ran.push(w.time - start);
        }
    }
    assert_eq!(cold_ran, [step, sleep + step], "the steps the cold node reacted at");
}

/// Gas a ground reaction exchanges with the air crosses the height from the ground to the air's mean height: in a world
/// whose only reaction is the burning of the energy-rich solid, a hot node of it burns and takes more gas from the air
/// than it gives; the world's potential energy changes by the weight the air gained times a height within the air
/// (kilometres: the air's scale height), the air's heat pays for the difference, and energy is kept with the potential
/// energy counted.
#[test]
fn gas_a_ground_node_exchanges_with_the_air_crosses_the_air_s_height() {
    let (mut chem, init) = worldgen::chemistry0();
    let at = |k: u16| chem.phase(k as usize, 290.0);
    let burn = (0..chem.reactions.len())
        .find(|&r| chem.dh(r) < 0.0 && chem.reactions[r].inputs.iter().any(|p| p.0 as usize == RICH) && chem.reactions[r].inputs.iter().any(|p| at(p.0) == Phase::Gas))
        .expect("a burning reaction");
    chem.reactions = vec![chem.reactions[burn].clone()];
    let mut w = worldgen::terrain::build(1, worldgen::terrain::island(16, 16), chem, &init, pcore::Config::default());
    let (n, g) = (w.mesh.cells().n(), w.cfg.gravity);
    let i = (0..n).find(|&i| w.surface_z(i) > 0.0).expect("land");
    empty_cell(&mut w, i);
    set_amount(&mut w, Node::ground(i), RICH, 1000.0);
    let e = w.energy_ground(i, 600.0);
    set_heat(&mut w, Node::ground(i), e);
    let (books, air0, pot0) = (Books::now(&w), w.store.amounts(Class::Air).to_vec(), w.store.heat(Class::Potential)[0]);
    change::step(&mut w, 3.0 * DAY);
    let na = w.mesh.level(AIR).n();
    let kg: f64 = (0..w.chem.ns()).map(|k| (0..na).map(|j| w.store.amounts(Class::Air)[k * na + j] - air0[k * na + j]).sum::<f64>() * w.chem.subs[k].molar_mass).sum();
    let height = (w.store.heat(Class::Potential)[0] - pot0) / (g * kg);
    println!("the air gained {kg:.2} kg across {height:.0} m");
    assert!(kg.abs() > 1.0 && height > 1e3 && height < w.cfg.air_top_m, "{kg} kg across {height} m");
    books.assert_kept(&w, 1e-12);
}

/// A ground node reacts at its temperature: one half frozen, on the melting plateau of its liquid, burns the energy-rich
/// solid as fast as the same node all frozen a millionth of a kelvin below (the fusion enthalpy its liquid holds is no
/// heat the reactor sees). The burning is the world's only reaction, and the rock holds the node's heat and its liquid.
#[test]
fn a_node_on_its_melting_plateau_reacts_at_its_melting_point() {
    let (mut chem, init) = worldgen::chemistry0();
    chem.reactions = vec![chem.reactions[BURN].clone()];
    let base = worldgen::terrain::build(1, worldgen::terrain::island(16, 16), chem, &init, pcore::Config::default());
    let (n, i, liquid) = (base.mesh.cells().n(), 17, 2000.0);
    let burned = |half: bool| {
        let mut w = base.clone();
        empty_cell(&mut w, i);
        for (k, m) in [(LIQUID, liquid), (RICH, 10.0), (ROCK, 1e5)] {
            set_amount(&mut w, Node::ground(i), k, m);
        }
        let tm = w.chem.subs[LIQUID].melt;
        let e = if half { w.energy_ground(i, tm) - 0.5 * liquid * w.chem.subs[LIQUID].lf } else { w.energy_ground(i, tm - 1e-6) };
        set_heat(&mut w, Node::ground(i), e);
        w.scratch = Default::default();
        assert_eq!(w.thaw(i).1.map(|p| p.0), half.then_some(LIQUID));
        let dt = w.cfg.phys_days as f64 * DAY;
        change::step(&mut w, dt);
        10.0 - w.store.amounts(Class::Ground)[RICH * n + i]
    };
    let (half, frozen) = (burned(true), burned(false));
    assert!(frozen > 0.0 && (half - frozen).abs() <= 1e-5 * frozen, "half frozen {half}, all frozen {frozen}");
}

/// A body's catalyst works below the temperature where it loses its shape, and a few K above it the reaction goes
/// about as fast as bare (at once, for a catalyst that loses its shape over no width).
#[test]
fn catalysts_fail_when_hot() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem, &cfg());
    let mut cat = vec![1.0; chem.reactions.len()];
    cat[1] = 1e6;
    let mut used = |t: f64, c: Option<(&[f64], f64, f64)>| {
        let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
        (n[RICH], gas[OX]) = (1000.0, 1e6);
        let mut e = t * HUGE_CAP;
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, HUGE_CAP, c, 0.0, 1.0);
        1e6 - gas[OX]
    };
    let (cool, cool_bare) = (used(310.0, Some((&cat, 320.0, 3.0))), used(310.0, None));
    let (hot, hot_bare) = (used(340.0, Some((&cat, 320.0, 3.0))), used(340.0, None));
    assert!(cool > 1e5 * cool_bare && hot < 1.2 * hot_bare, "310 K: {cool:e} vs bare {cool_bare:e}; 340 K: {hot:e} vs bare {hot_bare:e}");
    // With no width it is whole up to its temperature and gone above it.
    let (sharp_cool, sharp_hot) = (used(310.0, Some((&cat, 320.0, 0.0))), used(340.0, Some((&cat, 320.0, 0.0))));
    assert!(sharp_cool >= cool && sharp_hot == hot_bare, "no width: 310 K {sharp_cool:e}, 340 K {sharp_hot:e}");
}

/// Dead matter in the open goes with what there is of it (first order: the same share lost for 1 and 1,000 mol) at
/// its litter constant, 0.27 a year at 290 K for the energy-rich solid (Olson 1963; `docs/numbers.md`), the burning
/// gas at its share of the air.
#[test]
fn litter_goes_first_order_at_its_constant() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem, &cfg());
    let year = 3.156e7;
    let mut lost = |a: f64| {
        let (mut n, mut gas) = (vec![0.0; chem.ns()], air_over_m2(&chem, 400.0));
        n[RICH] = a;
        let mut e = 290.0 * OPEN_CAP;
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, OPEN_CAP, None, 0.0, year);
        1.0 - n[RICH] / a
    };
    let (few, many) = (lost(1.0), lost(1000.0));
    assert!((many - few).abs() < 1e-3 * few, "1 mol lost {few}, 1,000 mol lost {many}");
    // 0.27 a year, the rest of the bare decay (reactions 4 and 9, far slower) aside.
    assert!((0.235..0.25).contains(&few), "share lost in a year {few} (1 - e^-0.27 = 0.237)");
}

/// 1 m² of wet land (0.4 m of liquid in its pores) under the air at 400 ppm, with `rock` mol of the rock; the
/// reactions run for `dt` at 288 K. Returns the dissolved mineral made per second, and whether the books closed and
/// no amount went negative.
fn wet_land(chem: &Chemistry, rock: f64, dt: f64) -> (f64, bool) {
    let mut rc = Reactor::new(chem, &cfg());
    let (mut n, mut gas) = (vec![0.0; chem.ns()], air_over_m2(chem, 400.0));
    (n[LIQUID], n[ROCK]) = (0.4 * MOL_PER_M3, rock);
    let mut e = 288.0 * (OPEN_CAP + n.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>());
    let (el0, en0) = node_totals(chem, &n, &gas, e);
    rc.react(chem, &mut n, Some(&mut gas), &mut e, OPEN_CAP, None, 0.0, dt);
    let (el1, en1) = node_totals(chem, &n, &gas, e);
    let closed = (0..chem.n_elements).all(|j| (el1[j] - el0[j]).abs() <= 1e-10 * el0[j].max(1.0)) && (en1 - en0).abs() <= 1e-9 * en0.abs();
    (n[DISSOLVED] / dt, closed && n.iter().chain(&gas).all(|&a| a >= 0.0))
}

/// Law 2's rate is mass action with the contact of its phases (`change::pace`): there is no jump when another input
/// becomes the scarcest. A sweep of the rock across the amount of the gas over the cell (144 mol/m² at 400 ppm): the
/// mineral made goes with the rock, smoothly, and the books close at every step.
#[test]
fn no_jump_when_the_scarcest_input_changes() {
    let (chem, _) = worldgen::chemistry0();
    let gas = 400e-6 * AIR_COLUMN;
    let (mut last, mut rock) = (None, 1.0);
    while rock < 1e5 {
        let (rate, closed) = wet_land(&chem, rock, DAY);
        assert!(closed, "the books at {rock} mol of rock");
        assert!(rate > 0.0, "no mineral made at {rock} mol of rock");
        if let Some((r0, x0)) = last {
            // 2% more rock: never less made, never more than 2% more.
            let (up, more) = (rate / r0, rock / x0);
            assert!(up >= 1.0 && up <= more * 1.0001, "rock {x0:.1} -> {rock:.1} mol (gas {gas:.0}): rate x{up}");
        }
        last = Some((rate, rock));
        rock *= 1.02;
    }
    // Either side of the gas's amount the rate is the rock's: 1% either side, 2% apart.
    let (below, _) = wet_land(&chem, 0.99 * gas, DAY);
    let (above, _) = wet_land(&chem, 1.01 * gas, DAY);
    assert!((above / below - 1.01 / 0.99).abs() < 1e-3, "across the gas's amount: x{}", above / below);
}

/// The dissolved mineral held far past its balance (as in an eater's liquid: 4.35 mol in 1,250 mol of liquid at 305 K)
/// falls out at the pace of its rate law, its share squared, over a long step too: after 10 days what is left is the
/// integral of that law, 1 / sqrt(1 + 2 z) of it, and the same in one call or ten.
#[test]
fn a_mineral_past_its_balance_falls_out_by_its_rate_law_over_a_long_step() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem, &cfg());
    let mut left = |calls: usize| {
        let (mut n, mut gas) = (vec![0.0; chem.ns()], air_over_m2(&chem, 400.0));
        (n[LIQUID], n[DISSOLVED]) = (1250.0, 4.35);
        let mut e = 305.0 * 1e7;
        for _ in 0..calls {
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e7, None, 0.0, 10.0 * DAY / calls as f64);
        }
        n[DISSOLVED]
    };
    let rx = &chem.reactions[SETTLING];
    let z = rx.k * (-rx.ea / 305.0f64).exp() * (4.35f64 / 1250.0).powi(2) * 10.0 * DAY;
    let (one, ten, law) = (left(1), left(10), 4.35 / (1.0 + 2.0 * z).sqrt());
    println!("left after 10 days: one call {one:.4}, ten calls {ten:.4}, the rate law {law:.4} mol");
    assert!((0.9..1.25).contains(&(one / law)) && (one / ten - 1.0).abs() < 0.05, "one call {one}, ten calls {ten}, the law {law}");
}

/// 1 m² of a liquid column `depth` m deep over the rock of the start's loose ground (a tenth of 1 m by volume: the rate
/// goes with it, Law 2's contact), with the air over it at `ppm` of the heat-absorbing gas.
fn column(chem: &Chemistry, depth: f64, ppm: f64) -> (Vec<f64>, Vec<f64>) {
    let mut n = vec![0.0; chem.ns()];
    n[LIQUID] = depth * MOL_PER_M3;
    n[ROCK] = 0.1 / chem.molar_volume(ROCK, Phase::Solid);
    (n, air_over_m2(chem, ppm))
}

/// The dissolved mineral the chemistry holds per m² at balance: `depth` m of liquid, `ppm` of the gas, `t` K.
fn at_balance(chem: &Chemistry, depth: f64, ppm: f64, t: f64) -> f64 {
    let (mut n, gas) = column(chem, depth, ppm);
    let set = Reactor::new(chem, &cfg()).balance(chem, &mut n, &gas, t);
    set.iter().find(|p| p.0 == DISSOLVED).map_or(0.0, |p| p.1)
}

/// Dissolving and settling balance in elements and in energy, from either side.
#[test]
fn the_two_reactions_close_elements_and_energy() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem, &cfg());
    for mineral in [0.0, 5000.0] {
        let (mut n, mut gas) = column(&chem, 300.0, 400.0);
        n[DISSOLVED] = mineral;
        let mut e = 288.0 * (OPEN_CAP + n.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>());
        let (el0, en0) = node_totals(&chem, &n, &gas, e);
        for _ in 0..30 {
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, OPEN_CAP, None, 0.0, 120.0 * DAY);
        }
        let (el1, en1) = node_totals(&chem, &n, &gas, e);
        assert!((n[DISSOLVED] - mineral).abs() > 1.0, "it did react: {mineral} -> {}", n[DISSOLVED]);
        for j in 0..chem.n_elements {
            assert!((el1[j] - el0[j]).abs() <= 1e-10 * el0[j].max(1.0), "element {j}");
        }
        assert!((en1 - en0).abs() <= 1e-9 * en0.abs(), "energy {en0} -> {en1}");
        assert!(n.iter().chain(&gas).all(|&a| a >= 0.0));
    }
}

/// The balance holds what the real chemistry of such a pair says at the anchor (1 mol per m³ of the liquid, 500 m
/// deep, 400 ppm, 288 K). The mineral cubed, over the rock it left (the reaction's contact, Law 2), goes with the gas
/// and falls with warmth as the reaction heat says (dH / R = 4,847 K): the real law, the cation times the anion
/// squared in proportion to the gas's pressure, with the rock in it.
#[test]
fn the_balance_is_the_anchored_one_and_moves_as_the_law_says() {
    let (chem, _) = worldgen::chemistry0();
    let rock = column(&chem, 500.0, 400.0).0[ROCK];
    let held = |x: f64| x.powi(3) / (rock - x);
    let s = at_balance(&chem, 500.0, 400.0, 288.0);
    assert!((0.9..1.1).contains(&(s / 500.0)), "{} mol per m³ at the anchor", s / 500.0);
    let twice = held(at_balance(&chem, 500.0, 800.0, 288.0)) / held(s);
    assert!((twice - 2.0).abs() < 0.02, "twice the gas: x{twice}");
    let warm = held(at_balance(&chem, 500.0, 400.0, 298.0)) / held(s);
    let law = ((chem.dh_at(DISSOLVING, T_SURF) / R_GAS) * (1.0 / 288.0 - 1.0 / 298.0)).exp();
    assert!((warm / law - 1.0).abs() < 0.03, "10 K warmer: x{warm}, the law says x{law}");
}

/// The store goes with the liquid, not with the gas over it: a cell with no liquid holds none; a deeper column holds
/// more (to the power 2/3, the liquid's share counting twice), a little less per m³; the mineral left in a dry cell
/// settles out at once.
#[test]
fn a_dry_cell_holds_none_and_a_deeper_column_holds_more() {
    let (chem, _) = worldgen::chemistry0();
    assert_eq!(at_balance(&chem, 0.0, 400.0, 288.0), 0.0);
    let (shallow, deep) = (at_balance(&chem, 50.0, 400.0, 288.0), at_balance(&chem, 500.0, 400.0, 288.0));
    assert!(deep > 4.0 * shallow && deep < 5.0 * shallow, "50 m: {shallow}, 500 m: {deep} (10^(2/3) = 4.64)");
    // The mineral in a cell with no liquid but what it makes: it settles to the rock within a day.
    let mut rc = Reactor::new(&chem, &cfg());
    let (mut n, mut gas) = column(&chem, 0.0, 400.0);
    n[DISSOLVED] = 100.0;
    let mut e = 288.0 * OPEN_CAP;
    rc.react(&chem, &mut n, Some(&mut gas), &mut e, OPEN_CAP, None, 0.0, DAY);
    assert!(n[DISSOLVED] < 1e-6, "left in a dry cell: {}", n[DISSOLVED]);
}

/// A long run of the two reactions in lumps of 120 days (the pace of a calm cell), the air held at 400 ppm, settles
/// to the amount `balance` gives, from none and from twice as much (made of the same rock, so the same rock element in
/// all), with the gas reached through the liquid's film (500 m at 5e-5 m/s, `Reactor::film`): at the balance the
/// mineral gives the liquid as much gas as it takes.
#[test]
fn a_long_run_settles_to_the_balance() {
    let (chem, _) = worldgen::chemistry0();
    let want = at_balance(&chem, 500.0, 400.0, 288.0);
    let mut rc = Reactor::new(&chem, &cfg());
    rc.film = 500.0 / 5e-5;
    for start in [0.0, 2.0 * want] {
        let (mut n, gas0) = column(&chem, 500.0, 400.0);
        (n[DISSOLVED], n[ROCK]) = (start, n[ROCK] - start);
        let mut e = 288.0 * OPEN_CAP;
        for _ in 0..(60 * 3) {
            let mut gas = gas0.clone();
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, OPEN_CAP, None, 0.0, 120.0 * DAY);
        }
        assert!((n[DISSOLVED] / want - 1.0).abs() < 0.05, "from {start}: {} against the balance {want}", n[DISSOLVED]);
    }
}

/// Under a liquid the dead matter burns with the burning gas dissolved there, which the liquid's film renews
/// (`Reactor::film`): a pile that uses little of what the film can bring burns as in the open air; one that uses the
/// gas faster than the film brings it, slower by 1 / (1 + used x film / Henry's share), the series law.
#[test]
fn under_a_liquid_dead_matter_burns_as_its_film_lets_the_gas_through() {
    let (chem, _) = worldgen::chemistry0();
    let (t, dt, depth) = (288.0, DAY, 100.0);
    let film = depth / 5e-5;
    let burn = |pile: f64, film: f64| {
        let (mut n, mut gas) = column(&chem, depth, 400.0);
        n[RICH] = pile;
        let mut rc = Reactor::new(&chem, &cfg());
        rc.film = film;
        let (mut e, gas0) = (t * OPEN_CAP, gas[OX]);
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, OPEN_CAP, None, 0.0, dt);
        (gas0 - gas[OX]) / dt
    };
    let air: f64 = column(&chem, depth, 400.0).1.iter().sum();
    let eq = depth * MOL_PER_M3 * chem.henry(OX, t, 0.21 * AIR_COLUMN / air * P_REF);
    for pile in [10.0, 1e4] {
        let open = burn(pile, 0.0);
        let law = 1.0 / (1.0 + open * film / eq);
        assert!((burn(pile, film) / open / law - 1.0).abs() < 0.05, "{pile} mol: x{} against the law's x{law}", burn(pile, film) / open);
        assert!(if pile < 100.0 { law > 0.99 } else { law < 0.5 }, "{pile} mol: x{law}");
    }
}

/// With the air's gas halved, the dissolved mineral follows down the balance and gives back the gas it held (the
/// buffer): for each unit the air loses the liquid gives back more than one.
#[test]
fn the_liquid_gives_back_the_gas_it_holds_when_the_air_thins() {
    let (chem, _) = worldgen::chemistry0();
    let (at400, at200) = (at_balance(&chem, 500.0, 400.0, 288.0), at_balance(&chem, 500.0, 200.0, 288.0));
    let air_loss = 200e-6 * AIR_COLUMN;
    // The mineral holds 2 units of the gas each.
    let given = 2.0 * (at400 - at200);
    assert!(given > 2.0 * air_loss, "gave {given} for an air loss of {air_loss}");
}

/// The dissolved mineral in the pores of the land: worldgen puts none on land (the ground has no liquid at the start,
/// and a cell with no liquid holds none), and a year of rain on loose ground that is a tenth the rock makes it by the
/// same two reactions as under the low ground's liquid: the cells whose pores hold liquid hold the mineral, in the
/// amount at which the two go at equal rates for their liquid and the gas over them (`Reactor::balance`, to a half);
/// a cell whose pores are dry holds none.
#[test]
fn the_pores_of_the_land_hold_the_dissolved_mineral_where_rock_and_liquid_meet() {
    let mut w = worldgen::new_world(1, 16, 16);
    let (n, na) = (w.mesh.cells().n(), w.mesh.level(AIR).n());
    let land: Vec<usize> = (0..n).filter(|&i| w.surface_z(i) >= 0.0).collect();
    assert!(land.iter().all(|&i| w.store.amounts(Class::Ground)[DISSOLVED * n + i] == 0.0), "land started with the mineral");
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let mut rc = Reactor::new(&w.chem, &w.cfg);
    let (mut wet, mut dry, mut ratio) = (0, 0, Vec::new());
    for &i in &land {
        if w.store.amounts(Class::Ground)[LIQUID * n + i] > 1e6 {
            // The balance at the cell's own liquid, warmth and gas.
            let (t, j) = (w.t_ground(i), w.mesh.up(i, 0, AIR));
            let mut amt: Vec<f64> = (0..w.chem.ns()).map(|k| w.store.amounts(Class::Ground)[k * n + i]).collect();
            amt[DISSOLVED] = 0.0;
            let share = w.mesh.share(i, AIR);
            let gas: Vec<f64> = (0..w.chem.ns()).map(|k| w.store.amounts(Class::Air)[k * na + j] * share).collect();
            rc.balance(&w.chem, &mut amt, &gas, t);
            if amt[DISSOLVED] > 0.0 {
                wet += 1;
                ratio.push(w.store.amounts(Class::Ground)[DISSOLVED * n + i] / amt[DISSOLVED]);
            }
        } else if w.store.amounts(Class::Ground)[LIQUID * n + i] == 0.0 {
            dry += 1;
            assert_eq!(w.store.amounts(Class::Ground)[DISSOLVED * n + i], 0.0, "a dry cell holds the mineral");
        }
    }
    ratio.sort_by(|a, b| a.total_cmp(b));
    println!("{wet} land cells with liquid in the pores, {dry} dry; the mineral over its balance at the 10th, 50th, 90th percent: {:.2} {:.2} {:.2}", ratio[ratio.len() / 10], ratio[ratio.len() / 2], ratio[ratio.len() * 9 / 10]);
    assert!(wet > 20, "only {wet} wet land cells");
    assert!(ratio[ratio.len() / 2] > 0.5 && ratio[ratio.len() / 2] < 2.0, "the cells' mineral is far from its balance: {}", ratio[ratio.len() / 2]);
}

/// Dead matter gives the minerals back: in the open at 290 K it rots (0.2 a year, its litter constant) and every mol
/// that goes leaves three of the fixed gas's mineral and one of the rock (the rock element, which has no liquid here
/// to dissolve in) and none of the gas; burning hot (1000 K) the mineral it leaves breaks down as it is made, and
/// most of the fixed gas's element goes to the air as the gas, while the rock element stays in the solids (the rock,
/// then what the heat makes of it). Elements and energy are kept exactly.
#[test]
fn rotting_gives_the_minerals_back_and_burning_hot_gives_the_gas_to_the_air() {
    let (chem, init) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem, &cfg());
    let mut run = |t: f64, secs: f64| {
        let (mut n, mut gas, mut e) = open_air(&chem, &init, t);
        n[MATTER] = 1000.0;
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
        // The rock element outside the matter (mol), by the elements of what is left.
        let rock: f64 = (0..chem.ns()).filter(|&k| k != MATTER).map(|k| chem.subs[k].elems[ROCK_ELEMENT] as f64 * n[k]).sum();
        (1000.0 - n[MATTER], n[MINERAL], gas[FIX_GAS] - gas0[FIX_GAS], n[ROCK], rock)
    };
    let (lost, back, gas, rock_back, rock) = run(290.0, 365.0 * DAY);
    println!("a year at 290 K: {lost:.2} mol of matter rotted, {back:.2} mol of the mineral back, {rock_back:.2} mol of the rock, {gas:.4} mol of the gas");
    assert!(lost > 170.0 && lost < 205.0, "{lost}");
    assert!((back / (3.0 * lost) - 1.0).abs() < 1e-6 && gas.abs() < 1e-6 * lost, "mineral {back}, gas {gas}");
    // (A little of the rock meets the liquid the rotting makes and the gas, and dissolves: the rock element is counted
    // in all that is left, and the rock itself is held to 1%.)
    assert!((rock_back / lost - 1.0).abs() < 1e-2 && (rock / lost - 1.0).abs() < 1e-6, "rock {rock_back}, the rock element {rock}, for {lost} mol of matter");
    let (lost, back, gas, _, rock) = run(1000.0, 1e5);
    println!("hot, 1000 K: {lost:.1} mol of matter burned, {back:.1} mol of the mineral, {gas:.1} mol of the gas, {rock:.1} mol of the rock element");
    assert!(lost > 990.0, "{lost}");
    assert!(gas > 0.8 * 3.0 * lost && back < 0.2 * 3.0 * lost, "mineral {back}, gas {gas} for {lost} mol of matter");
    assert!((rock / lost - 1.0).abs() < 1e-6, "the rock element {rock} for {lost} mol of matter");
}

/// Dead matter on the ground returns its minerals to the ground: in a world with no body, a year, the matter that went
/// is in the fixed gas's mineral three for one (the mineral dissolves and runs with the liquid: it is counted over the
/// whole ground) and in the rock element outside the matter one for one, more than the same world without the matter
/// makes (the open air fixes next to nothing); elements and energy are kept.
#[test]
fn dead_matter_on_the_ground_returns_the_minerals_to_the_ground() {
    let (mut w, i) = start();
    let per = w.chem.minerals()[0].1;
    // The same world with nothing on the ground: what the open makes of the mineral in a year.
    let mut twin = w.clone();
    let bare = in_all(&twin, MINERAL);
    for _ in 0..12 {
        pcore::step(&mut twin);
    }
    let (bare, bare_rock) = (in_all(&twin, MINERAL) - bare, rock_outside_matter(&twin, i));
    put(&mut w, MATTER, i, 1e8);
    let books = Books::now(&w);
    let (m_before, n_before) = (in_all(&w, MATTER), in_all(&w, MINERAL));
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let (lost, gained, rock) = (m_before - in_all(&w, MATTER), in_all(&w, MINERAL) - n_before - bare, rock_outside_matter(&w, i) - bare_rock);
    assert!(bare >= 0.0 && bare < 1e-3 * lost, "the open fixed {bare} mol in a year");
    println!("a year on the ground: {lost:.0} of {m_before:.0} mol of matter rotted, {gained:.0} mol of the mineral back, {rock:.0} mol of the rock element more than the twin's cell");
    assert!(lost > 0.02 * m_before, "it did not rot: {lost}");
    assert!((gained / (per * lost) - 1.0).abs() < 1e-4, "the mineral {gained} for the matter {lost}");
    // (In the cell, to a tenth: the dissolved mineral the rotting's gas and liquid make of the rock moves with the
    // liquid, to and from the cells around; the books of elements are exact.)
    assert!((rock / lost - 1.0).abs() < 0.1, "the rock element {rock} for the matter {lost}");
    books.assert_kept(&w, 1e-9);
}

/// Cover is fuel when hot: on a cell held at 700 K until its light-eater node has updated, the node loses most of its
/// energy-rich solid (it reacts with the gas, in the body or, once the body dies, in the ground), while the same cell
/// left as it was keeps it.
#[test]
fn hot_cover_burns() {
    let run = |hot: bool| {
        let mut w = worldgen::new_world(1, 16, 16);
        // (No growth: building body matter takes the solid too, and this is about the burning.)
        w.cfg.phys.c_build = 0.0;
        pcore::step(&mut w);
        worldgen::life::seed(&mut w, 0);
        let node = w.life.alive().max_by(|a, b| a.count.total_cmp(&b.count)).expect("a node");
        let (i, days) = (node.cell as usize, node.interval(&w) + 1);
        let n = w.mesh.cells().n();
        let held = |w: &pcore::World| w.store.amounts(Class::Ground)[RICH * n + i] + w.life.alive().filter(|b| b.cell as usize == i).map(|b| b.store[RICH]).sum::<f64>();
        let before = held(&w);
        // Its interval and a day (its first update comes within its interval), the cell heated back to 700 K before
        // each day (booked as an edit).
        for _ in 0..days {
            if hot {
                hold_ground(&mut w, i, 700.0);
            }
            pcore::day(&mut w);
        }
        assert_eq!(pcore::check::broken(&w), None);
        held(&w) / before
    };
    let (hot, cold) = (run(true), run(false));
    assert!(hot < 0.5 && cold > 0.9, "energy-rich solid left: {hot:.3} hot, {cold:.3} as it was");
}
