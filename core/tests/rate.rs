//! Law 2's rate is mass action with the contact of its phases (`change::pace`): no jump when
//! another input becomes the scarcest, first order in a trace input, and with a solid as far as
//! the solid goes. Substances by number (chemistry 0): 0 the liquid, 2 the oxygen-like gas, 3 the
//! heat-absorbing gas, 5 the energy-rich solid, 10 the rock that dissolves, 23 the salt.

use pcore::change::Reactor;
use pcore::chem::Chemistry;

const LIQ: usize = 0;
const OX: usize = 2;
const GAS: usize = 3;
const RICH: usize = 5;
const ROCK: usize = 10;
const SALT: usize = 23;
const AIR_COLUMN: f64 = 101_325.0 / (9.81 * 0.0288);
const DAY: f64 = 86_400.0;

/// One m2 of wet land (0.4 m of liquid in its pores) under the air at 400 ppm, with `rock` mol of
/// the rock; the reactions run for `dt` at 288 K. Returns the salt made per second and the books
/// (elements, energy) before and after.
fn wet_land(chem: &Chemistry, rock: f64, dt: f64) -> (f64, bool) {
    let mut rc = Reactor::new(chem);
    let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
    (n[LIQ], n[ROCK]) = (0.4 * 1000.0 / 0.018, rock);
    (gas[1], gas[OX], gas[GAS]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, 400e-6 * AIR_COLUMN);
    let cap = 1e15;
    let mut e = 288.0 * (cap + n.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>());
    let books = |n: &[f64], gas: &[f64], e: f64| {
        let mut el = vec![0.0; chem.n_elements];
        let mut en = e;
        for k in 0..chem.ns() {
            chem.add_elems(k, n[k] + gas[k], &mut el);
            en += (n[k] + gas[k]) * chem.subs[k].hf;
        }
        (el, en)
    };
    let (el0, en0) = books(&n, &gas, e);
    rc.react(chem, &mut n, Some(&mut gas), &mut e, cap, None, 0.0, dt);
    let (el1, en1) = books(&n, &gas, e);
    let closed = (0..chem.n_elements).all(|j| (el1[j] - el0[j]).abs() <= 1e-10 * el0[j].max(1.0)) && (en1 - en0).abs() <= 1e-9 * en0.abs();
    (n[SALT] / dt, closed && n.iter().chain(&gas).all(|&a| a >= 0.0))
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

/// A trace gas against a solid is first order in the gas: the share of it the rock takes in a day
/// is the same for 1 or 100 mol of the gas, the other amounts the same.
#[test]
fn a_trace_gas_against_a_solid_goes_first_order() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    let mut lost = |a: f64| {
        let (mut n, mut gas) = (vec![0.0; chem.ns()], vec![0.0; chem.ns()]);
        (n[LIQ], n[ROCK]) = (1e5, 1e4);
        (gas[1], gas[OX], gas[GAS]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, a);
        let mut e = 288.0 * 1e15;
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, 30.0 * DAY);
        1.0 - gas[GAS] / a
    };
    let (few, many) = (lost(1.0), lost(100.0));
    assert!(few > 1e-4 && few < 0.9, "share lost {few}");
    assert!((many - few).abs() < 0.01 * few, "1 mol lost {few}, 100 mol lost {many}");
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
        (gas[1], gas[OX], gas[GAS]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, 400e-6 * AIR_COLUMN);
        let mut e = 290.0 * 1e15;
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, year);
        1.0 - n[RICH] / a
    };
    let (few, many) = (lost(1.0), lost(1000.0));
    assert!((many - few).abs() < 1e-3 * few, "1 mol lost {few}, 1,000 mol lost {many}");
    // 0.27 a year, the rest of the bare decay (reaction 4 and 9, far slower) aside.
    assert!((0.235..0.25).contains(&few), "share lost in a year {few} (1 - e^-0.27 = 0.237)");
}

/// The founders' light gene is raised by `LIGHT_X`, the log10 of the gas a founder light-eater
/// exchanges in its interval over the liquid it holds (the contact of the light reaction's phases
/// in a body is its liquid): the constant is that measure.
#[test]
fn the_light_shift_is_the_founders_gas_over_their_liquid() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let i = (0..w.grid.n()).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
    let shape = pcore::choose::shape(w.chem.ns(), 12, 6);
    let (l, g) = (w.life.new_lineage(pcore::body::Lineage { shape, seen: 3, sight: 1, base: vec![], root: 0 }), w.life.new_genome(vec![]));
    let mut b = worldgen::life::make(&mut w, 0, i, l, g).expect("a founder");
    b.prepare(&w.chem, &w.cfg.phys);
    let reach = b.k.gas * b.open_area(&w.chem, &w.cfg.phys, b.mass(&w.chem)) * b.interval() as f64;
    let x = (reach / b.store[LIQ]).log10();
    assert!((x - worldgen::life::LIGHT_X as f64).abs() < 0.01, "a founder exchanges 10^{x} times its liquid");
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
        (n[LIQ], n[SALT]) = (1250.0, 4.35);
        (gas[1], gas[OX], gas[GAS]) = (0.78 * AIR_COLUMN, 0.21 * AIR_COLUMN, 400e-6 * AIR_COLUMN);
        let mut e = 305.0 * 1e7;
        for _ in 0..calls {
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e7, None, 0.0, 10.0 * DAY / calls as f64);
        }
        n[SALT]
    };
    let rx = &chem.reactions[23];
    let z = rx.k * (-rx.ea / 305.0f64).exp() * (4.35f64 / 1250.0).powi(2) * 10.0 * DAY;
    let (one, ten, law) = (left(1), left(10), 4.35 / (1.0 + 2.0 * z).sqrt());
    println!("salt left after 10 days: one call {one:.4}, ten calls {ten:.4}, the rate law {law:.4} mol");
    assert!((0.9..1.25).contains(&(one / law)) && (one / ten - 1.0).abs() < 0.05, "one call {one}, ten calls {ten}, the law {law}");
}
