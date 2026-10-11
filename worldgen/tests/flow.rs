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
    w.e_ground[top] = w.heat_cap_ground(top) * t;
    let (t0, l0) = (totals(&w), w.ledger.clone());
    pcore::flow::step(&mut w, DT);
    assert!(w.ground[k * n + top] < 1e8, "liquid stayed on the top: {}", w.ground[k * n + top]);
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
