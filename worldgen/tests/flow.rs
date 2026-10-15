//! Two tiny scenes for Law 1 (Flow) on a small generated world.

use pcore::check::{drift, totals};

const DT: f64 = 3.0 * 86_400.0;

#[test]
fn liquid_runs_downhill() {
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, k) = (w.grid.n(), init.low);
    let top = (0..n).max_by(|&a, &b| w.surface_z(a).total_cmp(&w.surface_z(b))).unwrap();
    let t = w.t_ground(top).max(w.chem.subs[k].melt + 5.0);
    w.ground[k * n + top] += 1e9;
    w.e_ground[top] = w.energy_ground(top, t);
    let (t0, l0) = (totals(&w), w.ledger.clone());
    pcore::flow::step(&mut w, DT);
    assert!(w.ground[k * n + top] < 1e8, "liquid stayed on the top: {}", w.ground[k * n + top]);
    let (de, dq) = drift(&w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-9, "drift {de} {dq}");
}

/// A hot node held at a melting point with little fusion enthalpy left: radiating for a whole
/// step at that temperature would take more heat than it holds. It leaves the plateau and cools
/// by its heat capacity, toward the balance with sunlight and the air, not down to the floor.
#[test]
fn hot_plateau_cools_but_not_to_the_floor() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, ns) = (w.grid.n(), w.chem.ns());
    let k = (0..ns).find(|&k| (700.0..1000.0).contains(&w.chem.subs[k].melt)).expect("a substance melting near 850 K");
    let i = (0..n).max_by(|&a, &b| w.surface_z(a).total_cmp(&w.surface_z(b))).unwrap();
    (0..ns).for_each(|x| w.ground[x * n + i] = 0.0);
    w.ground[k * n + i] = w.grid.area();
    let melt = w.chem.subs[k].melt;
    w.e_ground[i] = w.energy_ground(i, melt) + 0.5 * w.ground[k * n + i] * w.chem.subs[k].lf;
    assert_eq!(w.thaw(i).1.map(|p| p.0), Some(k));
    let (t0, l0) = (totals(&w), w.ledger.clone());
    pcore::flow::step(&mut w, DT);
    let t = w.t_ground(i);
    assert!(t > 150.0 && t < melt, "{t} K after a step from {melt} K");
    assert_eq!(w.ledger.clamped_heat, 0.0);
    let (de, dq) = drift(&w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-9, "drift {de} {dq}");
}

#[test]
fn warm_air_spreads() {
    let mut cold = worldgen::new_world(3, 16, 16);
    let mut warm = cold.clone();
    let j = cold.grid.aw() + 1;
    warm.e_air[j] *= 1.2;
    let (t0, l0) = (totals(&warm), warm.ledger.clone());
    let excess = warm.t_air(j) - cold.t_air(j);
    pcore::flow::step(&mut cold, DT);
    pcore::flow::step(&mut warm, DT);
    assert!(warm.t_air(j) - cold.t_air(j) < 0.5 * excess, "warm node kept its heat");
    assert!(warm.t_air(j + 1) - cold.t_air(j + 1) > 0.5, "neighbour did not warm");
    let (de, dq) = drift(&warm, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-9, "drift {de} {dq}");
}

/// More heat-absorbing gas keeps warming the ground: under 240 W/m² of sunlight, with the air
/// following the ground by the lapse rate, the balance at optical depth 17 is clearly warmer than
/// at 9 (two seeds' late worlds; one slab at the air's mean temperature gave both the same).
#[test]
fn more_gas_warms_with_no_ceiling() {
    let ground = |x: f64| {
        let (mut lo, mut hi) = (150.0, 500.0);
        for _ in 0..60 {
            let t: f64 = (lo + hi) / 2.0;
            let up = (-x).exp() * 5.670e-8 * t.powi(4) + pcore::flow::glow(-(-x).exp_m1(), t, &pcore::flow::column(x, 0.0065, 1e4, 7600.0)[1]);
            (lo, hi) = if up > 240.0 { (lo, t) } else { (t, hi) };
        }
        lo
    };
    let (t1, t9, t17) = (ground(1.5), ground(9.0), ground(17.0));
    assert!(t9 > t1 + 20.0 && t17 > t9 + 3.0, "{t1} {t9} {t17}");
}

/// The most soluble gas of the air dissolves in the liquid of the deepest node toward Henry's
/// share (here with a fast liquid side, so within an exchange), and part of it comes out when that
/// liquid is warmed by 20 K (a fifth at balance); matter and energy are kept.
#[test]
fn gas_dissolves_and_comes_out_when_warmed() {
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::new_world(3, 16, 16);
    (w.cfg.dissolve, w.cfg.exchange_days) = (1.0, 3);
    let (n, na, c) = (w.grid.n(), w.grid.na(), &w.chem);
    let k = init.air.iter().map(|p| p.0).filter(|&k| c.subs[k].boil < 250.0).max_by(|&a, &b| c.subs[a].solubility.total_cmp(&c.subs[b].solubility)).unwrap();
    let i = (0..n).max_by(|&a, &b| w.ground[init.low * n + a].total_cmp(&w.ground[init.low * n + b])).unwrap();
    let (t0, l0) = (totals(&w), w.ledger.clone());
    // (Gases dissolve at every fourth exchange, for all the time since.)
    (0..4).for_each(|_| pcore::flow::step(&mut w, DT));
    // Dissolved per mol of liquid, against Henry's share under the gas's share of the reference pressure.
    let held = |w: &pcore::World| w.ground[k * n + i] / w.ground[init.low * n + i];
    let j = w.grid.air_of(i);
    let x = w.air[k * na + j] / (0..w.chem.ns()).map(|q| w.air[q * na + j]).sum::<f64>();
    let (h1, want) = (held(&w), w.chem.henry(k, w.t_ground(i), x * pcore::chem::P_REF));
    assert!(h1 > 0.5 * want && h1 < 2.0 * want, "{h1} dissolved per mol, Henry's share {want}");
    let e = w.energy_ground(i, w.t_ground(i) + 20.0) - w.e_ground[i];
    w.e_ground[i] += e;
    w.ledger.imposed_energy += e;
    (0..4).for_each(|_| pcore::flow::step(&mut w, DT));
    assert!(held(&w) < 0.9 * h1, "{} dissolved per mol after warming, {h1} before", held(&w));
    let (de, dq) = drift(&w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-9, "drift {de} {dq}");
}

/// A solid that dissolves in the liquid (the nitrogen mineral, 15 Oct 2026) moves with the liquid down the slope, a share of it
/// each step, as dissolved gas does, and is deposited nowhere as sediment; on dry ground it stays where it is. Matter and energy
/// are kept. (The share is that of the liquid that runs off of the standing liquid and the soil's moisture together,
/// `Physics::pore`.)
#[test]
fn a_dissolved_solid_moves_with_the_liquid_and_a_dry_one_stays() {
    let (_, init) = worldgen::chemistry0();
    let mut wet = worldgen::new_world(3, 16, 16);
    let (n, k) = (wet.grid.n(), init.low);
    let mineral = wet.chem.mineral().expect("the mineral").0;
    let top = (0..n).max_by(|&a, &b| wet.surface_z(a).total_cmp(&wet.surface_z(b))).unwrap();
    let t = wet.t_ground(top).max(wet.chem.subs[k].melt + 5.0);
    // The twin has no liquid on the highest cell; the other a sheet of it half a metre deep. Both 1e6 mol of the mineral there.
    let mut dry = wet.clone();
    let sheet = 0.5 * wet.grid.area() / wet.chem.molar_volume(k, pcore::chem::Phase::Liquid);
    wet.ground[k * n + top] += sheet;
    wet.e_ground[top] = wet.energy_ground(top, t);
    for w in [&mut wet, &mut dry] {
        let e = 1e6 * w.chem.subs[mineral].cp * w.t_ground(top);
        w.ground[mineral * n + top] += 1e6;
        w.e_ground[top] += e;
        w.chem.add_elems(mineral, 1e6, &mut w.ledger.imposed_elems);
        w.ledger.imposed_energy += e + 1e6 * w.chem.subs[mineral].hf;
    }
    let (t0, l0) = (totals(&wet), wet.ledger.clone());
    for _ in 0..8 {
        pcore::flow::step(&mut wet, DT);
    }
    // (One step for the dry twin: rain would wet it later.)
    pcore::flow::step(&mut dry, DT);
    let moved = 1.0 - wet.ground[mineral * n + top] / 1e6;
    let downstream: f64 = (0..n).filter(|&i| i != top).map(|i| wet.ground[mineral * n + i]).sum();
    println!("with the sheet {:.1}% of the mineral left the cell in 8 steps, {:.3e} mol are downstream; on dry ground {:.3e} mol stayed in a step", 100.0 * moved, downstream, dry.ground[mineral * n + top]);
    assert!(moved > 0.3, "only {moved} of the mineral moved with the liquid");
    assert!((downstream + wet.ground[mineral * n + top] - 1e6).abs() < 1e-6 * 1e6, "the mineral was lost: {} mol", downstream + wet.ground[mineral * n + top]);
    assert_eq!(dry.ground[mineral * n + top], 1e6, "the mineral moved on dry ground");
    let (de, dq) = drift(&wet, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "drift {de} {dq}");
}
