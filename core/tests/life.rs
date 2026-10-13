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
    // Heat that leaves with matter of most of the node's heat capacity may take the node down
    // to the floor of the heat capacity that stays.
    let (c, e1) = (0.9 * w.heat_cap_ground(i), w.e_ground[i]);
    let took = w.take_heat_with(i, e1, c);
    assert!((e1 - took - (w.heat_cap_ground(i) - c) * pcore::T_FLOOR).abs() <= 1e-9 * e1, "{took} of {e1}");
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
        worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
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
    // (They may have moved on together since.)
    let nodes: Vec<f64> = w.life.alive().filter(|b| b.lineage == lineage && b.count > 1.0).map(|b| b.count).collect();
    assert!(!nodes.is_empty(), "no node of the lineage after a month");
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// However unlike, the bodies of a cell merge at a month's start until at most `cfg.crowd` nodes
/// are left; no member is lost and matter and energy are kept exactly.
#[test]
fn crowd_bounds_the_nodes_of_a_cell() {
    let mut w = worldgen::new_world(1, 32, 32);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    let a = w.life.alive().find(|b| b.count == 1.0).expect("an individual");
    let (i, lineage, genome) = (a.cell as usize, a.lineage, a.genome);
    for x in 0..3 * w.cfg.crowd {
        let mut b = worldgen::life::make(&mut w, 1, i, lineage, genome).expect("body");
        b.genes[pcore::body::COPY_MASS] += 0.3 * x as f32;
        w.life.add(b, w.day, 1);
    }
    let here = |w: &pcore::World| w.life.alive().filter(|b| b.cell as usize == i).map(|b| b.n()).collect::<Vec<f64>>();
    let before: f64 = here(&w).iter().sum();
    pcore::body::survey(&mut w);
    let after = here(&w);
    assert!(after.len() <= w.cfg.crowd, "{} nodes left", after.len());
    assert!((after.iter().sum::<f64>() - before).abs() < 1e-9 * before, "members {before} then {after:?}");
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Cold costs energy: what a starting body's reactions can release over its interval, against
/// its upkeep, falls with its temperature by Change's rate. Warm bodies can pay; cold ones not.
#[test]
fn cold_bodies_release_too_little() {
    let mut w = worldgen::new_world(1, 32, 32);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    for kind in [true, false] {
        let mut b = w.life.alive().find(|b| (pcore::body::light_cat(&w.chem, &b.genes) > 5.0) == kind).expect("a body").clone();
        b.prepare(&w.chem);
        let ratio = |t: f64| {
            let mut b = b.clone();
            b.e = b.heat_cap(&w.chem) * t;
            let days = b.interval() as f64;
            pcore::body::release(&w, &b, days) / pcore::body::upkeep(&w, &b, days, b.mass(&w.chem))
        };
        let r = [250.0, 270.0, 280.0, 290.0, 300.0].map(ratio);
        println!("light-capturing {kind}: release / upkeep at 250, 270, 280, 290, 300 K: {r:.3?}");
        assert!(r[0] < 1.0 && r[3] > 1.0, "{r:?}");
    }
}

/// A body whose safe minimums are all off holds nothing it must keep, so it can give all it holds
/// to a body with room (here handling costs nothing, so no reaction leaves it anything). Left with
/// nothing of a member, it dies at once: no other body meets an empty body, which has no heat
/// capacity to share its heat by (taking from one gave NaN, which reached the air).
#[test]
fn a_body_that_gives_all_it_holds_dies_at_once() {
    use pcore::body::{Lineage, INTERVAL};
    use pcore::event::EventKind;
    use pcore::{body, bond, brain, change, choose, flow};
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    w.cfg.phys.c_handle = 0.0;
    let (n, ns, lay) = (w.grid.n(), w.chem.ns(), Layout::of(&w.chem));
    let i = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
    // Brains that only give to a body they see, and that only rest.
    let mut r = Rng::new(1, 0, 0, Purpose::Body);
    let shape = choose::shape(ns, 12, 6);
    let mut lineage = |w: &mut pcore::World, fixed: [f32; brain::FIXED], verbs: [f32; brain::VERBS + brain::OVERBS]| {
        let mut base = brain::random(shape, &mut r, 0.1, fixed, -8.0, verbs);
        let rows = base.split_off(shape.base());
        (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base }), w.life.new_genome(rows))
    };
    let mut give = [-8.0; brain::VERBS + brain::OVERBS];
    give[1] = 8.0;
    let (gl, gg) = lineage(&mut w, [-8.0; brain::FIXED], give);
    let mut rest = [-8.0; brain::FIXED];
    rest[0] = 8.0;
    let (rl, rg) = lineage(&mut w, rest, [-8.0; brain::VERBS + brain::OVERBS]);
    let mut giver = worldgen::life::make(&mut w, 1, i, gl, gg).expect("body");
    // No safe minimums, and a long interval, so one give may move all it holds.
    giver.genes[lay.min..lay.upt].fill(-0.1);
    giver.genes[INTERVAL] = 1.5;
    let mut taker = worldgen::life::make(&mut w, 1, i, rl, rg).expect("body");
    taker.genes[lay.keep..].fill(0.95);
    let g = w.life.add(giver, w.day, 1);
    w.life.add(taker, w.day, 1);
    let dt = w.cfg.phys_days as f64 * 86_400.0;
    for _ in 0..3 {
        let d = w.day % w.cfg.days_per_month as u64;
        if d.is_multiple_of(w.cfg.phys_days as u64) {
            body::survey(&mut w);
            flow::step(&mut w, dt);
            change::step(&mut w, dt);
        }
        if d == 0 {
            bond::step(&mut w);
        }
        body::day(&mut w);
        w.day += 1;
        for b in w.life.alive() {
            assert!(b.mass(&w.chem) >= 1e-3 * b.n(), "an empty body is alive: energy {}, stores {:?}", b.e, b.store);
        }
        assert_eq!(pcore::check::broken(&w), None);
    }
    assert!(w.events.iter().any(|e| matches!(e.kind, EventKind::Gave { .. })), "nothing was given");
    assert!(!w.life.bodies[g].alive, "the giver lives on");
}

/// Dense cover of other bodies shelters a body's heat like walls: a conductance in series with the
/// body's own, so a body under 10 kg/m² of cover ends warmer over the ground than one on bare
/// ground (its upkeep warms it more).
#[test]
fn cover_keeps_a_body_warm() {
    let mut w = worldgen::new_world(1, 32, 32);
    pcore::step(&mut w);
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
    worldgen::life::seed_with(&mut w, 1, &worldgen::life::Seeding::old());
    let bs = &w.life.bodies;
    let slot = (0..bs.len()).filter(|&s| bs[s].alive && pcore::body::light_cat(&w.chem, &bs[s].genes) < 5.0).max_by(|&a, &b| bs[a].temp(&w.chem).total_cmp(&bs[b].temp(&w.chem))).expect("a body");
    let i = bs[slot].cell as usize;
    // It rests, so it stays where it is.
    let b = &mut w.life.bodies[slot];
    b.adj.fill(-3.0);
    b.adj[0] = 3.0;
    let mut covered = w.clone();
    // Cover: a node of light-eaters, 10 kg/m² over the cell.
    let mut c = covered.life.alive().find(|b| pcore::body::light_cat(&w.chem, &b.genes) > 5.0).expect("a light-eater").clone();
    let f = 10.0 * w.grid.area() / c.mass(&w.chem);
    c.store.iter_mut().for_each(|a| *a *= f);
    c.e *= f;
    c.count = c.n() * f;
    c.cell = i as u32;
    c.adj.fill(-3.0);
    c.adj[0] = 3.0;
    covered.life.add(c, covered.day, 1);
    let temp = |w: &pcore::World| w.life.bodies[slot].temp(&w.chem) - w.life.env[i][0];
    pcore::step(&mut w);
    pcore::step(&mut covered);
    assert!(w.life.bodies[slot].alive && covered.life.bodies[slot].alive);
    println!("above the ground: bare {} K, under cover {} K", temp(&w), temp(&covered));
    assert!(temp(&covered) > temp(&w) + 1.0, "bare {} K, under cover {} K above the ground", temp(&w), temp(&covered));
}

/// The light a node's light reactions store is set aside in its cell as it falls (not drawn
/// back from the ground's heat at its update): a node a thousand times as large as seeded that
/// decides once a year takes it from there, no heat goes ungiven, and matter and energy are
/// kept exactly.
#[test]
fn a_big_node_stores_light_set_aside_as_it_fell() {
    let mut w = worldgen::new_world(1, 32, 32);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    for _ in 0..3 {
        pcore::step(&mut w);
    }
    let s = (0..w.life.bodies.len()).filter(|&s| w.life.bodies[s].alive).max_by(|&a, &b| w.life.bodies[a].count.total_cmp(&w.life.bodies[b].count)).expect("a node");
    let b = &mut w.life.bodies[s];
    b.count *= 1e3;
    b.store.iter_mut().for_each(|a| *a *= 1e3);
    b.e *= 1e3;
    // It decides once a year: a year's stored light at once.
    (b.genes[pcore::body::INTERVAL], b.k) = (2.556, Default::default());
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    for _ in 0..14 {
        pcore::step(&mut w);
        assert_eq!(pcore::check::broken(&w), None);
    }
    assert!(w.light_pool.iter().sum::<f64>() > 0.0 && w.ledger.clamped_heat == l0.clamped_heat, "heat not given {:e} J", w.ledger.clamped_heat - l0.clamped_heat);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
