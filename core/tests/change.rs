use pcore::change::Reactor;
use pcore::chem::Chemistry;

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

#[test]
fn hot_reacts_faster() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    let mut used = |t: f64| {
        let mut n = vec![0.0; chem.ns()];
        let mut gas = vec![0.0; chem.ns()];
        n[5] = 1000.0;
        gas[2] = 1e6;
        let mut e = t * 1e12; // huge heat capacity: temperature stays put
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e12, None, 0.0, 1e5);
        1000.0 - n[5]
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
    n[5] = 1000.0;
    gas[2] = 5000.0;
    gas[1] = 20000.0;
    let mut e = 650.0 * 1000.0 * chem.subs[5].cp;
    let (el0, en0) = totals(&chem, &n, &gas, e);
    for _ in 0..4 {
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 0.0, None, 0.0, 3.0 * 86_400.0);
    }
    let (el1, en1) = totals(&chem, &n, &gas, e);
    assert!(n[5] < 10.0 && gas[3] > 500.0, "left {} released {}", n[5], gas[3]);
    assert!(n.iter().chain(&gas).all(|&a| a >= 0.0));
    for j in 0..chem.n_elements {
        assert!((el1[j] - el0[j]).abs() <= 1e-10 * el0[j].max(1.0), "element {j}");
    }
    assert!((en1 - en0).abs() <= 1e-9 * en0.abs(), "energy {en0} -> {en1}");
}
