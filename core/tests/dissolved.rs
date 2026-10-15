//! The liquid's store of the gas (reactions 22 and 23 of chemistry 0, substance 23): a salt that
//! exists only dissolved. Substances by number: 0 the liquid, 3 the heat-absorbing gas, 10 the
//! rock that dissolves, 23 the salt.

use pcore::change::{reversible, Reactor};
use pcore::chem::Chemistry;

const LIQ: usize = 0;
const GAS: usize = 3;
const ROCK: usize = 10;
const SALT: usize = 23;
/// Moles of liquid in 1 m3 of it; the gas over 1 m2 at the start (its 400 ppm of the air's column).
const MOL_PER_M3: f64 = 1000.0 / 0.018;
const AIR_COLUMN: f64 = 101_325.0 / (9.81 * 0.0288);
const DAY: f64 = 86_400.0;

/// One m2 of a liquid column `depth` m deep over plenty of rock, with the gas of the air over it.
fn column(chem: &Chemistry, depth: f64, ppm: f64) -> (Vec<f64>, Vec<f64>) {
    let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
    n[LIQ] = depth * MOL_PER_M3;
    n[ROCK] = 1e4;
    (gas[1], gas[2], gas[GAS]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, ppm * 1e-6 * AIR_COLUMN);
    (n, gas)
}

/// Elements and (thermal + chemical) energy of one node, with its gas share.
fn totals(chem: &Chemistry, n: &[f64], gas: &[f64], e: f64) -> (Vec<f64>, f64) {
    let mut el = vec![0.0; chem.n_elements];
    let mut en = e;
    for k in 0..chem.ns() {
        chem.add_elems(k, n[k] + gas[k], &mut el);
        en += (n[k] + gas[k]) * chem.subs[k].hf;
    }
    (el, en)
}

/// The salt the chemistry holds per m2 at balance: `depth` m of liquid, `ppm` of the gas, `t` K.
fn at_balance(chem: &Chemistry, depth: f64, ppm: f64, t: f64) -> f64 {
    let (mut n, gas) = column(chem, depth, ppm);
    let set = Reactor::new(chem).balance(chem, &mut n, &gas, t);
    set.iter().find(|p| p.0 == SALT).map_or(0.0, |p| p.1)
}

#[test]
fn the_pair_is_found_from_the_chemistry_alone() {
    let (chem, _) = worldgen::chemistry0();
    assert!(reversible(&chem).contains(&(22, 23, 3, SALT)), "{:?}", reversible(&chem));
}

/// Dissolving and settling balance in elements and in energy, from either side.
#[test]
fn the_two_reactions_close_elements_and_energy() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    for salt in [0.0, 5000.0] {
        let (mut n, mut gas) = column(&chem, 300.0, 400.0);
        n[SALT] = salt;
        let (t, cap) = (288.0, 1e15);
        let mut e = t * (cap + n.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>());
        let (el0, en0) = totals(&chem, &n, &gas, e);
        for _ in 0..30 {
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, cap, None, 0.0, 120.0 * DAY);
        }
        let (el1, en1) = totals(&chem, &n, &gas, e);
        assert!((n[SALT] - salt).abs() > 1.0, "it did react: {salt} -> {}", n[SALT]);
        for j in 0..chem.n_elements {
            assert!((el1[j] - el0[j]).abs() <= 1e-10 * el0[j].max(1.0), "element {j}");
        }
        assert!((en1 - en0).abs() <= 1e-9 * en0.abs(), "energy {en0} -> {en1}");
        assert!(n.iter().chain(&gas).all(|&a| a >= 0.0));
    }
}

/// The balance holds what the real carbonate chemistry says at the anchor (1 mol per m3 of the
/// liquid, 500 m deep, 400 ppm, 288 K), goes with the gas to the power 1/3 and falls with warmth
/// as the reaction heat says (dH / R = 4,847 K, the salt as the cube root of the constant).
#[test]
fn the_balance_is_the_anchored_one_and_moves_as_the_law_says() {
    let (chem, _) = worldgen::chemistry0();
    let s = at_balance(&chem, 500.0, 400.0, 288.0) / 500.0;
    assert!((0.9..1.1).contains(&s), "{s} mol per m3 at the anchor");
    let twice = at_balance(&chem, 500.0, 800.0, 288.0) / (s * 500.0);
    assert!((twice - 2f64.powf(1.0 / 3.0)).abs() < 0.01, "twice the gas: x{twice}");
    let warm = at_balance(&chem, 500.0, 400.0, 298.0) / (s * 500.0);
    let law = (-(-(chem.dh(22) / 8.314) * (1.0 / 288.0 - 1.0 / 298.0)) / 3.0).exp();
    assert!((warm / law - 1.0).abs() < 0.02, "10 K warmer: x{warm}, the law says x{law}");
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
    n[SALT] = 100.0;
    let mut e = 288.0 * 1e15;
    rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, DAY);
    assert!(n[SALT] < 1e-6, "salt left in a dry cell: {}", n[SALT]);
}

/// A long run of the two reactions in lumps of 120 days (the pace of a calm cell), the air held at
/// 400 ppm, settles to the amount `balance` gives, from none and from twice as much.
#[test]
fn a_long_run_settles_to_the_balance() {
    let (chem, _) = worldgen::chemistry0();
    let want = at_balance(&chem, 500.0, 400.0, 288.0);
    let mut rc = Reactor::new(&chem);
    for start in [0.0, 2.0 * want] {
        let (mut n, gas0) = column(&chem, 500.0, 400.0);
        n[SALT] = start;
        let mut e = 288.0 * 1e15;
        for _ in 0..(60 * 3) {
            let mut gas = gas0.clone();
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, 120.0 * DAY);
        }
        assert!((n[SALT] / want - 1.0).abs() < 0.05, "from {start}: {} against the balance {want}", n[SALT]);
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

fn drift_after(years: u64) -> (f64, f64, pcore::World) {
    let mut w = worldgen::new_world(1, 24, 24);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    for _ in 0..years * 12 {
        pcore::step(&mut w);
    }
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    (de, dq, w)
}

/// The world starts with the salt in its low ground and none on dry land, takes no more than the
/// rock there is, and keeps its elements and energy through two years.
#[test]
fn a_world_starts_with_the_salt_in_the_liquid_and_keeps_its_books() {
    let w = worldgen::new_world(1, 24, 24);
    let n = w.grid.n();
    let (mut wet, mut dry) = (0.0, 0.0);
    for i in 0..n {
        if w.ground[LIQ * n + i] > 0.0 {
            assert!(w.ground[SALT * n + i] > 0.0, "cell {i} has liquid and no salt");
            wet += w.ground[SALT * n + i];
        } else {
            dry += w.ground[SALT * n + i];
        }
    }
    assert!(wet > 0.0 && dry == 0.0, "salt in the liquid {wet}, on dry land {dry}");
    let (de, dq, w2) = drift_after(2);
    assert!(de < 1e-9 && dq < 1e-9, "drift {de} {dq}");
    assert!(w2.ground[SALT * n..(SALT + 1) * n].iter().sum::<f64>() > 0.5 * wet);
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
    let wet = (0..n).filter(|&i| w.ground[SALT * n + i] > 0.0).count();
    let mut changes = 0;
    for _ in 0..39 {
        let before: Vec<f64> = w.ground[SALT * n..(SALT + 1) * n].to_vec();
        pcore::change::step(&mut w, 3.0 * DAY);
        w.day += 3;
        changes += (0..n).filter(|&i| w.ground[SALT * n + i] != before[i]).count();
    }
    eprintln!("{changes} changes of salt in 39 steps over {wet} cells");
    assert!(wet > 100, "{wet} cells hold salt");
    assert!(changes <= 3 * wet, "{changes} changes of salt in 39 steps over {wet} cells");
    assert!(changes >= wet / 2, "{changes} changes: the cells do react at their turn");
}

/// The salt moves with the liquid that runs off a high cell, by the liquid's share, and stays when
/// the liquid evaporates (it is no gas).
#[test]
fn the_salt_runs_with_the_liquid_and_stays_when_the_liquid_leaves() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, na, area) = (w.grid.n(), w.grid.na(), w.grid.area());
    let top = (0..n).max_by(|&a, &b| w.surface_z(a).total_cmp(&w.surface_z(b))).unwrap();
    let t = w.t_ground(top).max(w.chem.subs[LIQ].melt + 5.0);
    // (Liquid soaks into the pores of the loose ground first, 15 Oct 2026; what runs off is what is over them: three times their volume.)
    let pores: f64 = (0..w.chem.ns()).filter(|&k| w.chem.phase(k, w.t_ground(top)) == pcore::chem::Phase::Solid).map(|k| w.g(k, top) * w.chem.molar_volume(k, pcore::chem::Phase::Solid)).sum::<f64>() * w.cfg.phys.porosity / w.chem.molar_volume(LIQ, pcore::chem::Phase::Liquid);
    let (liquid, salt) = (3.0 * pores, 1e5);
    (w.ground[LIQ * n + top], w.ground[SALT * n + top]) = (liquid, salt);
    w.e_ground[top] = w.energy_ground(top, t);
    let all = |w: &pcore::World, k: usize| w.ground[k * n..(k + 1) * n].iter().sum::<f64>() + w.air[k * na..(k + 1) * na].iter().sum::<f64>();
    let (salt0, liq0) = (all(&w, SALT), all(&w, LIQ));
    pcore::flow::step(&mut w, 3.0 * DAY);
    let (left_liq, left_salt) = (w.ground[LIQ * n + top], w.ground[SALT * n + top]);
    assert!(left_liq < 0.5 * liquid && left_salt < 0.5 * salt, "stayed: liquid {left_liq}, salt {left_salt}");
    // The same share leaves: the salt left over the liquid left is the start's, within a tenth.
    assert!(((left_salt / salt) / (left_liq / liquid) - 1.0).abs() < 0.1, "left {} of the liquid, {} of the salt", left_liq / liquid, left_salt / salt);
    assert!((all(&w, SALT) - salt0).abs() < 1e-9 * salt0 && (all(&w, LIQ) - liq0).abs() < 1e-6 * liq0);
    // A thin film on warm ground in dry air: the liquid evaporates, the salt stays where it is.
    let mut w = worldgen::new_world(3, 16, 16);
    // (A pit of dry land, lower than all its neighbours: nothing runs off it.)
    let pit = |w: &pcore::World, i: usize| pcore::grid::DIRS.iter().all(|&d| w.grid.step(i, d).map_or(false, |j| w.surface_z(j) > w.surface_z(i)));
    let i = (0..n).find(|&i| w.ground[LIQ * n + i] == 0.0 && w.surface_z(i) > 20.0 && pit(&w, i)).expect("a pit of dry land");
    (w.ground[LIQ * n + i], w.ground[SALT * n + i]) = (0.05 * area / 0.018e-3 * 1.0, 1e3);
    w.e_ground[i] = w.energy_ground(i, 330.0);
    w.air[LIQ * na..(LIQ + 1) * na].fill(0.0);
    let (liq0, salt0) = (w.ground[LIQ * n + i], w.ground[SALT * n + i]);
    for _ in 0..4 {
        pcore::flow::step(&mut w, 3.0 * DAY);
    }
    assert!(w.ground[LIQ * n + i] < 0.99 * liq0, "the liquid did not evaporate: {} of {liq0}", w.ground[LIQ * n + i]);
    assert_eq!(w.ground[SALT * n + i], salt0);
    assert_eq!(w.air[SALT * na..(SALT + 1) * na].iter().sum::<f64>(), 0.0);
}
