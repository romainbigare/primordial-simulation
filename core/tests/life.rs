use pcore::body::{Layout, MUT};
use pcore::rng::{Purpose, Rng};

#[test]
fn child_has_parent_genes_plus_noise() {
    let mut w = worldgen::new_world(1, 32, 32);
    worldgen::life::seed(&mut w, 0);
    let parent = w.life.alive().next().expect("a body").genes.clone();
    let lay = Layout::of(&w.chem);
    let child = pcore::copy::mutate(&lay, &parent, &mut Rng::new(1, 0, 0, Purpose::Mutate));
    let size = 10f32.powf(parent[MUT]);
    let diff: Vec<f32> = parent.iter().zip(&child).map(|(a, b)| (a - b).abs()).collect();
    assert!(diff.iter().any(|&d| d > 0.0), "no change");
    assert!(diff.iter().all(|&d| d <= 6.0 * size), "change too big");
}

#[test]
fn bodies_keep_elements_and_energy() {
    let mut w = worldgen::new_world(1, 32, 32);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    for _ in 0..24 {
        pcore::step(&mut w);
    }
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    assert!(w.life.alive().count() > 0);
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// The same seed gives the same history, with brains shared by bodies that stayed together.
#[test]
fn same_seed_same_history_with_shared_brains() {
    let run = || {
        let mut w = worldgen::new_world(1, 32, 32);
        pcore::step(&mut w);
        worldgen::life::seed(&mut w, 0);
        worldgen::life::seed(&mut w, 1);
        for _ in 0..30 {
            pcore::step(&mut w);
        }
        w
    };
    let (a, b) = (run(), run());
    assert_eq!(pcore::check::hash(&a), pcore::check::hash(&b));
    let shared = a.life.groups.iter().filter(|g| g.members > 1).count();
    assert!(shared > 0, "no shared brains among {} bodies", a.life.alive().count());
    // Each group knows the mean of its members' born rows.
    for (k, g) in a.life.groups.iter().enumerate().filter(|(_, g)| g.members > 0) {
        let mut sum = vec![0f32; g.rows.len()];
        for b in a.life.alive().filter(|b| b.group as usize == k) {
            sum.iter_mut().zip(a.life.born(b)).for_each(|(s, x)| *s += x / g.members as f32);
        }
        let err = sum.iter().zip(g.innate()).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max);
        assert!(err < 1e-4, "group {k}: mean off by {err}");
    }
}

#[test]
fn take_heat_never_takes_a_node_below_the_floor() {
    let mut w = worldgen::new_world(1, 32, 32);
    let (i, e0) = (5, w.e_ground[5]);
    let floor = w.heat_cap_ground(i).max(1.0) * pcore::T_FLOOR;
    assert_eq!(w.take_heat(i, 0.25 * e0), 0.25 * e0);
    assert_eq!(w.ledger.clamped_heat, 0.0);
    // Asking for more than there is gives what is above the floor and counts the rest.
    let took = w.take_heat(i, 10.0 * e0);
    assert!((w.e_ground[i] - floor).abs() <= 1e-9 * e0, "{} vs {floor}", w.e_ground[i]);
    assert!((took + w.ledger.clamped_heat - 10.0 * e0).abs() <= 1e-9 * e0);
    assert!(pcore::check::broken(&w).is_none());
    // A negative request gives heat and is returned whole.
    assert_eq!(w.take_heat(i, -5.0), -5.0);
}

/// A brain grown (more hidden units and memory) and shrunk back is the brain it was.
#[test]
fn brain_grown_and_shrunk_back_is_unchanged() {
    let (a, b) = (pcore::choose::shape(9, 12, 6), pcore::choose::shape(9, 16, 8));
    let mut r = Rng::new(1, 0, 0, Purpose::Body);
    let w = pcore::brain::random(a, &mut r, 0.1, [0.0; 8], 0.0, [0.0; 9]);
    let (base, rows) = w.split_at(a.base());
    let grown = (pcore::brain::rebase(base, a, b, &mut r, 0.1), pcore::brain::rerow(rows, a, b, &mut r, 0.1));
    assert_eq!((grown.0.len(), grown.1.len()), (b.base(), b.rows() * b.r()));
    assert_eq!(pcore::brain::rebase(&grown.0, b, a, &mut r, 0.1), base);
    assert_eq!(pcore::brain::rerow(&grown.1, b, a, &mut r, 0.1), rows);
}

/// Without shared brains every body keeps a group of its own, and matter and energy still add up.
#[test]
fn own_brains_keep_one_body_per_group() {
    let mut w = worldgen::new_world(1, 32, 32);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    w.cfg.share_brains = false;
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    let first = w.life.alive().count();
    for _ in 0..30 {
        pcore::step(&mut w);
    }
    let born = w.life.alive().filter(|b| b.born > 0).count();
    assert!(born > 0 && w.life.alive().count() > 0, "{first} bodies placed, none born");
    assert!(w.life.groups.iter().all(|g| g.members <= 1), "a group with more than one member");
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Cover is fuel when hot: on a cell held at 700 K for a month, a light-eater node loses most of
/// its energy-rich solid (it reacts with the gas, in the body or, once the body dies, in the
/// ground), while the same cell left as it was keeps it.
#[test]
fn hot_cover_burns() {
    let run = |hot: bool| {
        let mut w = worldgen::new_world(1, 32, 32);
        pcore::step(&mut w);
        worldgen::life::seed(&mut w, 0);
        let (_, rich, _, _) = worldgen::life::roles(&w.chem).expect("roles");
        let i = w.life.alive().max_by(|a, b| a.count.total_cmp(&b.count)).expect("a node").cell as usize;
        let n = w.grid.n();
        let held = |w: &pcore::World| w.ground[rich * n + i] + w.life.alive().filter(|b| b.cell as usize == i).map(|b| b.store[rich]).sum::<f64>();
        let before = held(&w);
        // The days of `pcore::step`, the cell heated back to 700 K before each (booked as an edit).
        let dt = w.cfg.phys_days as f64 * 86_400.0;
        for d in 0..w.cfg.days_per_month {
            if hot {
                let e = w.energy_ground(i, 700.0);
                w.ledger.imposed_energy += e - w.e_ground[i];
                w.e_ground[i] = e;
            }
            if d % w.cfg.phys_days == 0 {
                pcore::body::survey(&mut w);
                pcore::flow::step(&mut w, dt);
                pcore::change::step(&mut w, dt);
            }
            pcore::body::day(&mut w);
            w.day += 1;
        }
        assert_eq!(pcore::check::broken(&w), None);
        held(&w) / before
    };
    let (hot, cold) = (run(true), run(false));
    eprintln!("energy-rich solid left: {hot:.3} hot, {cold:.3} as it was");
    assert!(hot < 0.5 && cold > 0.9, "energy-rich solid left: {hot:.3} hot, {cold:.3} as it was");
}

/// More than `cfg.crowd` alike individuals in one cell (of light-eaters, which they eat) merge
/// into one population node at the next month's start; matter and energy are kept exactly.
#[test]
fn crowded_individuals_merge_into_a_node() {
    let mut w = worldgen::new_world(1, 32, 32);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    let a = w.life.alive().find(|b| b.count == 1.0).expect("an individual");
    let (i, lineage, genome) = (a.cell as usize, a.lineage, a.genome);
    let placed = w.cfg.crowd + 4;
    for _ in 0..placed - w.life.alive().filter(|b| b.cell as usize == i && b.lineage == lineage).count() {
        let b = worldgen::life::make(&mut w, 1, i, lineage, genome).expect("body");
        w.life.add(b, w.day, 1);
    }
    pcore::step(&mut w);
    let here: Vec<f64> = w.life.alive().filter(|b| b.cell as usize == i && b.lineage == lineage).map(|b| b.count).collect();
    assert!(here.len() < placed && here.iter().any(|&c| c > 1.0), "bodies of the lineage after a month: {here:?}");
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
