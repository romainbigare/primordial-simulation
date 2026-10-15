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
        // (2.5e4 s: with the solid's litter constant 4 times higher, 1e5 s burns the hot node out and the ratio cannot show.)
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e12, None, 0.0, 2.5e4);
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

/// A reaction is first order in its scarce input: a trace of the methane-like gas (4) in plenty of
/// the oxygen-like gas (2) loses the same share of itself in the same time whatever its amount.
#[test]
fn scarce_gas_goes_first_order() {
    let (chem, _) = worldgen::chemistry0();
    let mut rc = Reactor::new(&chem);
    let mut lost = |a: f64| {
        let mut n = vec![0.0; chem.ns()];
        n[2] = 1e6;
        n[4] = a;
        let mut e = 300.0 * 1e12; // huge heat capacity: temperature stays put
        rc.react(&chem, &mut n, None, &mut e, 1e12, None, 0.0, 3e4);
        1.0 - n[4] / a
    };
    let (few, many) = (lost(1.0), lost(100.0));
    assert!(few > 0.1 && few < 0.9, "share lost {few}");
    assert!((many - few).abs() < 0.01 * few, "1 mol lost {few}, 100 mol lost {many}");
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
        (n[5], gas[2]) = (1000.0, 1e6);
        let mut e = t * 1e12; // huge heat capacity: temperature stays put
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e12, c, 0.0, 1.0);
        1e6 - gas[2]
    };
    let (cool, cool_bare) = (used(310.0, Some((&cat, 320.0, 3.0))), used(310.0, None));
    let (hot, hot_bare) = (used(340.0, Some((&cat, 320.0, 3.0))), used(340.0, None));
    assert!(cool > 1e5 * cool_bare && hot < 1.2 * hot_bare, "310 K: {cool:e} vs bare {cool_bare:e}; 340 K: {hot:e} vs bare {hot_bare:e}");
}
