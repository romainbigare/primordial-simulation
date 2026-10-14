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
    // Down to no brain: no base, the rows' biases kept.
    let z = pcore::choose::shape(9, 0, 6);
    let none = pcore::brain::rerow(rows, a, z, &mut r, 0.1);
    assert!(pcore::brain::rebase(base, a, z, &mut r, 0.1).is_empty() && none.len() == z.rows());
    assert_eq!(none[0], rows[a.h]);
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
    // At the month's start (looked at before they act: within the month the node may give its
    // members up one at a time to the actions of Bond, as the draws go).
    pcore::body::survey(&mut w);
    let nodes: Vec<f64> = w.life.alive().filter(|b| b.cell as usize == i && b.lineage == lineage && b.count > 1.0).map(|b| b.count).collect();
    assert!(!nodes.is_empty(), "no node of the lineage at the month's start");
    pcore::step(&mut w);
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
        b.prepare(&w.chem, &w.cfg.phys);
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
        (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 0 }), w.life.new_genome(rows))
    };
    let mut give = [-8.0; brain::VERBS + brain::OVERBS];
    give[1] = 8.0;
    let (gl, gg) = lineage(&mut w, [-8.0; brain::FIXED], give);
    let mut rest = [-8.0; brain::FIXED];
    rest[0] = 8.0;
    let (rl, rg) = lineage(&mut w, rest, [-8.0; brain::VERBS + brain::OVERBS]);
    let mut giver = worldgen::life::make(&mut w, 1, i, gl, gg).expect("body");
    // No safe minimums, and a long interval (316 days: the taker, of 30 kg, takes by 30^0.75 =
    // 12.8 kg, 0.05 x 316 x 12.8 = 200 kg of a 30 kg giver's spare), so one give may move all it holds.
    giver.genes[lay.min..lay.upt].fill(-0.1);
    giver.genes[INTERVAL] = 2.5;
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
    // The scene of the latitudes 5° to 55° the map had until 13 October 2026 (it picks the warmest body).
    let (chem, init) = worldgen::chemistry0();
    let mut w = worldgen::terrain::build(1, 32, 32, chem, &init, pcore::Config { lat_south: 5.0, lat_north: 55.0, ..Default::default() });
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

/// Light passes through a cell's bodies from the tallest member down (`body::shade`): of two
/// light-eater nodes with the same capture area, the one of 100 times heavier members catches
/// more light per m² of it (e^(its capture area per ground area) times more), the two together
/// catch what the cell's canopy intercepts, and matter and energy are kept exactly.
#[test]
fn a_tall_member_shades_a_short_one() {
    let mut w = worldgen::new_world(1, 32, 32);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    let short = (0..w.life.bodies.len()).filter(|&s| w.life.bodies[s].alive).max_by(|&a, &b| w.life.bodies[a].count.total_cmp(&w.life.bodies[b].count)).expect("a node");
    let (i, area) = (w.life.bodies[short].cell as usize, w.grid.area());
    // The cell's other nodes move next door.
    for s in w.life.at[i].clone().into_iter().map(|s| s as usize).filter(|&s| s != short) {
        let (g, j) = (w.life.bodies[s].group, i ^ 1);
        w.life.relocate(s, g, i, j);
        (w.life.bodies[s].cell, w.life.bodies[s].mark) = (j as u32, w.light[j]);
    }
    // Two nodes of capture area 0.5 per ground area each, the same but for member mass (and
    // the copy mass gene, so they are not alike and stay apart).
    let b = &mut w.life.bodies[short];
    let f = 0.5 * area / (b.vis[1] as f64 * b.n());
    b.store.iter_mut().for_each(|a| *a *= f);
    (b.e, b.count) = (b.e * f, b.n() * f);
    let mut tall = b.clone();
    // Leaf goes with mass^0.75 (14 Oct 2026): 100 times fewer members of 100 times the mass have
    // 100^-0.25 of the leaf, so the tall one's LEAF gene is raised by 0.5 to make the areas equal.
    (tall.count, tall.genes[pcore::body::COPY_MASS], tall.k) = (b.n() / 100.0, b.genes[pcore::body::COPY_MASS] + 2.0, Default::default());
    tall.genes[pcore::body::LEAF] += 0.5;
    (b.vis, tall.vis) = (b.visible(&w.chem, &w.cfg.phys), tall.visible(&w.chem, &w.cfg.phys));
    let slots = [w.life.add(tall, w.day, 1), short];
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    pcore::body::survey(&mut w);
    let (light0, dt) = (w.light[i], w.cfg.phys_days as f64 * 86_400.0);
    pcore::flow::step(&mut w, dt);
    let bs = &w.life.bodies;
    let per = |s: usize| w.light[i] - bs[s].mark;
    let cap = |s: usize| bs[s].vis[1] as f64 * bs[s].n();
    let (t, sh) = (per(slots[0]), per(slots[1]));
    println!("light per m² of capture area: tall {t:.4e}, short {sh:.4e} J");
    assert!(sh > 0.0 && (t / sh / (cap(slots[0]) / area).exp() - 1.0).abs() < 1e-6, "tall {t:e}, short {sh:e}");
    let (all, cell) = (cap(slots[0]) * t + cap(slots[1]) * sh, (w.light[i] - light0) * w.canopy[i] as f64 * area);
    assert!((all / cell - 1.0).abs() < 1e-6, "bodies caught {all:e} J, the canopy {cell:e} J");
    for _ in 0..3 {
        pcore::step(&mut w);
        assert_eq!(pcore::check::broken(&w), None);
    }
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// An individual far past `full` times its copy mass owes many children at once. It pays into its
/// child store what its reactions can release (and it can spare) at each update and has the whole
/// children the store covers, the rest later (review 3, P6; all or nothing, it could never pay and
/// grew without end). Matter and energy are kept exactly; the store is a count of work done.
#[test]
fn a_heavy_individual_copies_over_several_updates() {
    use pcore::body::{Lineage, DISPERSE};
    use pcore::event::EventKind;
    use pcore::{body, brain, change, choose, flow};
    let mut w = worldgen::new_world(1, 16, 16);
    // The build rate is out of this test (it has its own): what limits the child store here is
    // what the reactions can release.
    (w.cfg.phys.c_copy, w.cfg.phys.c_build) = (w.cfg.phys.c_copy * 5.0, 1e12);
    pcore::step(&mut w);
    let n = w.grid.n();
    let i = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
    // A starting eater whose brain only rests, ten times as heavy: 300 kg, 2.4 times `full`, so it
    // owes 5.6 children; at 5 times the copy cost (only what the children are built of costs,
    // not the liquid they carry) that is more than its reactions can release in one interval (it
    // does not eat).
    let shape = choose::shape(w.chem.ns(), 12, 6);
    let mut rest = [-8.0; brain::FIXED];
    rest[0] = 8.0;
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, rest, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let (l, g) = (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 0 }), w.life.new_genome(rows));
    let mut b = worldgen::life::make(&mut w, 1, i, l, g).expect("body");
    for k in 0..w.chem.ns() {
        w.chem.add_elems(k, 9.0 * b.store[k], &mut w.ledger.imposed_elems);
        (w.ledger.imposed_energy, b.store[k]) = (w.ledger.imposed_energy + 9.0 * b.store[k] * w.chem.subs[k].hf, 10.0 * b.store[k]);
    }
    (w.ledger.imposed_energy, b.e, b.genes[DISPERSE]) = (w.ledger.imposed_energy + 9.0 * b.e, 10.0 * b.e, 0.0);
    // Weak (strength costs most of its upkeep), so it lives on its stores a while.
    b.genes[pcore::body::STRENGTH] = 0.0;
    let (s, start) = (w.life.add(b, w.day, 1), w.day);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let (mut log, mut born, dt) = (Vec::new(), 0.0, w.cfg.phys_days as f64 * 86_400.0);
    for _ in 0..150 {
        if (w.day % w.cfg.days_per_month as u64).is_multiple_of(w.cfg.phys_days as u64) {
            body::survey(&mut w);
            flow::step(&mut w, dt);
            change::step(&mut w, dt);
        }
        w.events.clear();
        body::day(&mut w);
        let b = &w.life.bodies[s];
        if b.alive && b.last == w.day && w.day > start {
            let c: f64 = w.events.iter().filter(|e| matches!(e.kind, EventKind::Born { .. })).map(|e| e.value).sum();
            assert_eq!(c, c.round(), "part of a child");
            log.push((b.mass(&w.chem) as i64, b.paid as i64, c));
            born += c;
        }
        w.day += 1;
        assert_eq!(pcore::check::broken(&w), None);
    }
    println!("mass kg, child store J, children: {log:?}");
    let with = log.iter().filter(|x| x.2 > 0.0).count();
    assert!(with >= 2 && born >= 3.0 && log[0].2 < born, "children at once: {log:?}");
    assert!(log.iter().any(|x| x.1 > 0 && x.2 == 0.0), "the store never carried work over: {log:?}");
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// A grown body takes no more than it can use (14 Oct 2026, review 5: satiety, in place of "past
/// `full` it takes at the rate of `full`", which review 3's M2 added so that a body that could not
/// pay for its children did not swell on what it held): an eater of 10 or 100 times the mass of a
/// starting one, past `full` and holding fuel enough for many intervals, takes on the ground's
/// energy-rich solid only the heat of its upkeep over an interval, whatever `c_build`: never less
/// (a hard stop below upkeep starved the eaters, 13 Oct: burning leaves its liquid product in the
/// body), and no more, as its store already releases what it can use. A starting eater (below
/// `full`) takes more than that: it eats as its intake allows. Matter and energy are kept exactly.
#[test]
fn a_body_takes_no_more_than_it_can_use() {
    use pcore::body::{Lineage, DISPERSE};
    use pcore::event::EventKind;
    use pcore::{brain, choose};
    // Burnable kg taken in an update, the heat of the upkeep and build it may use, and the size cap, kg.
    let took = |times: f64, c_build: f64| {
        let mut w = worldgen::new_world(1, 16, 16);
        w.cfg.phys.c_build = c_build;
        pcore::step(&mut w);
        let n = w.grid.n();
        let i = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
        // A kg of the energy-rich solid per m² on its ground (booked as an edit).
        let (_, rich, _, burn) = worldgen::life::roles(&w.chem).expect("roles");
        let (t, a) = (w.t_ground(i), w.grid.area() / w.chem.subs[rich].molar_mass);
        w.ground[rich * n + i] += a;
        w.chem.add_elems(rich, a, &mut w.ledger.imposed_elems);
        let e = w.energy_ground(i, t);
        (w.ledger.imposed_energy, w.e_ground[i]) = (w.ledger.imposed_energy + e - w.e_ground[i] + a * w.chem.subs[rich].hf, e);
        // A starting eater (30 kg, copying at 63) whose brain only takes from the ground, `times`
        // as heavy.
        let shape = choose::shape(w.chem.ns(), 12, 6);
        let mut fixed = [-8.0; brain::FIXED];
        fixed[3] = 8.0;
        let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, fixed, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
        let rows = base.split_off(shape.base());
        let (l, g) = (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 0 }), w.life.new_genome(rows));
        let mut b = worldgen::life::make(&mut w, 1, i, l, g).expect("body");
        for k in 0..w.chem.ns() {
            w.chem.add_elems(k, (times - 1.0) * b.store[k], &mut w.ledger.imposed_elems);
            (w.ledger.imposed_energy, b.store[k]) = (w.ledger.imposed_energy + (times - 1.0) * b.store[k] * w.chem.subs[k].hf, times * b.store[k]);
        }
        (w.ledger.imposed_energy, b.e, b.genes[DISPERSE]) = (w.ledger.imposed_energy + (times - 1.0) * b.e, times * b.e, 0.0);
        b.prepare(&w.chem, &w.cfg.phys);
        let (p, days, mass) = (w.cfg.phys, b.interval() as f64, b.mass(&w.chem));
        let (equiv, joule_per_kg) = (b.equiv(&p, mass), -w.chem.dh(burn) / w.chem.reactions[burn].inputs.iter().find(|x| x.0 as usize == rich).unwrap().1 as f64 / w.chem.subs[rich].molar_mass);
        let can = pcore::body::upkeep(&w, &b, days, mass) + p.c_build * equiv / p.m_ref * days;
        let s = w.life.add(b, w.day, 1);
        let (t0, l0, id) = (pcore::check::totals(&w), w.ledger.clone(), w.life.bodies[s].id(s));
        pcore::body::survey(&mut w);
        w.day += 1;
        w.events.clear();
        pcore::body::day(&mut w);
        assert_eq!(pcore::check::broken(&w), None);
        let (de, dq) = pcore::check::drift(&w, &t0, &l0);
        assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
        let burnt: f64 = w.events.iter().filter_map(|e| if let EventKind::Took { by, from: u64::MAX, burn, .. } = e.kind { (by == id).then_some(burn as f64) } else { None }).sum();
        (burnt, can / joule_per_kg, p.take * days * equiv)
    };
    for times in [10.0, 100.0] {
        let ((with, _, cap), (without, upkeep, _)) = (took(times, 3e4), took(times, 0.0));
        println!("{times} x 30 kg: takes {with:.3} kg of fuel (upkeep {upkeep:.3}, size cap {cap:.3}), with no building {without:.3}");
        assert!(upkeep < cap, "the size cap {cap} is not above its upkeep {upkeep}");
        assert!((with / upkeep - 1.0).abs() < 0.02 && (without / upkeep - 1.0).abs() < 0.02, "{with} and {without} of {upkeep} kg");
    }
    let (young, can, _) = took(1.0, 3e4);
    println!("a starting eater takes {young:.3} kg of fuel (upkeep and build {can:.3})");
    assert!(young > can, "below full a body eats as its intake allows: {young} of {can} kg");
}

/// A brain of no hidden units is no brain (review 3, Q2): a body of such a lineage takes no
/// decision and pays nothing for a brain, but lives its physics at its updates and copies when
/// full (W3). Matter and energy are kept exactly.
#[test]
fn a_body_with_no_brain_lives_and_copies() {
    use pcore::body::{Lineage, COPY_MASS, HIDDEN};
    use pcore::event::EventKind;
    use pcore::{brain, choose};
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let n = w.grid.n();
    let i = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
    // The light-eaters' sizes but no hidden units.
    let shape = choose::shape(w.chem.ns(), 0, 6);
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, [0.0; brain::FIXED], 0.0, [0.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    assert!(base.is_empty());
    let (l, g) = (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 0 }), w.life.new_genome(rows));
    // A light-eater node whose members (1 kg) hold more than `full` times their copy mass (0.4 kg).
    let mut b = worldgen::life::make(&mut w, 0, i, l, g).expect("a node");
    (b.genes[HIDDEN], b.genes[COPY_MASS]) = (0.0, -0.4);
    let (n0, s) = (b.count, w.life.add(b, w.day, 1));
    // It pays nothing for a brain, however costly brains are.
    let mut x = w.life.bodies[s].clone();
    x.prepare(&w.chem, &w.cfg.phys);
    let (d, m, p0) = (x.interval() as f64, x.mass(&w.chem), w.cfg.phys);
    let u0 = pcore::body::upkeep(&w, &x, d, m);
    (w.cfg.phys.c_hidden, w.cfg.phys.c_memory, w.cfg.phys.c_seen, w.cfg.phys.c_sight, w.cfg.phys.c_learn) = (1e9, 1e9, 1e9, 1e9, 1e9);
    assert_eq!(pcore::body::upkeep(&w, &x, d, m), u0);
    w.cfg.phys = p0;
    let (t0, l0, u) = (pcore::check::totals(&w), w.ledger.clone(), w.life.updates);
    for _ in 0..2 {
        pcore::step(&mut w);
        assert_eq!(pcore::check::broken(&w), None);
    }
    let b = &w.life.bodies[s];
    let born: f64 = w.events.iter().filter(|e| matches!(e.kind, EventKind::Born { .. })).map(|e| e.value).sum();
    println!("updates {}, members {n0:.0} -> {:.0}, born {born:.0}", w.life.updates - u, b.count);
    assert!(b.alive && w.life.updates - u >= 2 && born > 0.0 && b.count > n0, "no children: {born}");
    assert!(w.diag.decisions == 0.0 && b.trace.is_empty() && b.wb == 0.0, "the brain was called");
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Living wears a body (14 Oct 2026): an individual dies of age when the energy its upkeep turned
/// over per kg reaches `e_life` x 10^REPAIR, and a node of the same members loses that share of
/// them at each update, so its members' mean life is the same. More repair: a longer life (8 times
/// per unit) at a higher upkeep (the repair share, a quarter at gene 0, doubles per unit). Matter and energy are kept exactly.
#[test]
fn bodies_die_of_age_by_their_rate_of_living() {
    use pcore::body::{Lineage, AGE, DISPERSE, REPAIR, STRENGTH};
    use pcore::event::EventKind;
    use pcore::{brain, choose};
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let land: Vec<usize> = (0..w.grid.n()).filter(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).collect();
    // Starting eaters (30 kg, under their copy mass) whose brain only rests, weak so their stores last.
    let shape = choose::shape(w.chem.ns(), 12, 6);
    let mut rest = [-8.0; brain::FIXED];
    rest[0] = 8.0;
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, rest, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let l = w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 0 });
    let make = |w: &mut pcore::World, i: usize, repair: f32, count: f64| {
        let g = w.life.new_genome(rows.clone());
        let mut b = worldgen::life::make(w, 1, i, l, g).expect("body");
        (b.genes[STRENGTH], b.genes[DISPERSE], b.genes[REPAIR]) = (0.0, 0.0, repair);
        // A node of `count` such members (its matter and energy booked as an edit).
        for k in 0..w.chem.ns() {
            w.chem.add_elems(k, (count - 1.0) * b.store[k], &mut w.ledger.imposed_elems);
            (w.ledger.imposed_energy, b.store[k]) = (w.ledger.imposed_energy + (count - 1.0) * b.store[k] * w.chem.subs[k].hf, count * b.store[k]);
        }
        (w.ledger.imposed_energy, b.e, b.count) = (w.ledger.imposed_energy + (count - 1.0) * b.e, count * b.e, count);
        b.prepare(&w.chem, &w.cfg.phys);
        let (d, m) = (b.interval() as f64, b.mass(&w.chem));
        let u = pcore::body::upkeep(w, &b, d, m) / (d * m);
        (w.life.add(b, w.day, 1), u)
    };
    let (_, ua) = make(&mut w, land[0], 0.0, 1.0);
    let (_, ub) = make(&mut w, land[land.len() / 2], 0.5, 1.0);
    let (c, uc) = make(&mut w, land[land.len() - 1], 0.0, 1000.0);
    assert!((ub / ua - (0.75 + 0.25 * 2f64.sqrt())).abs() < 1e-9 && (uc / ua - 1.0).abs() < 1e-9, "upkeep per kg {ua} {ub} {uc}");
    // A life of 60 days at gene 0 (its upkeep per kg now), 10^0.5 / (0.75 + 0.25 x 2^0.5) times that at 0.5.
    let (life, x) = (60.0, 10f64.sqrt() / (0.75 + 0.25 * 2f64.sqrt()));
    w.cfg.phys.e_life = life * ua;
    let (t0, l0, start, n0) = (pcore::check::totals(&w), w.ledger.clone(), w.day, w.life.bodies[c].count);
    for _ in 0..8 {
        pcore::step(&mut w);
    }
    let aged = |w: &pcore::World| w.events.iter().filter(|e| matches!(e.kind, EventKind::Died { cause: AGE, .. })).map(|e| (e.day - start, e.value)).collect::<Vec<_>>();
    // The individuals' deaths (one member each) and the node's (a share of its members).
    let (ones, node): (Vec<_>, Vec<_>) = aged(&w).into_iter().partition(|x| x.1 == 1.0);
    // The node's mean life: the days its members lived over those who died (it loses a share at
    // update; it may die of other causes later, so only its age deaths count).
    let (mut n, mut t, mut lived) = (n0, 0, 0.0);
    for &(day, dead) in &node {
        (lived, n, t) = (lived + n * (day - t) as f64, n - dead, day);
    }
    let dc = lived / (n0 - n);
    println!("lives, days: individuals {ones:?} at genes 0 and 0.5 (predicted {life} and {:.0}); node {dc:.1} ({:.0} of {n0} members died of age)", life * x, n0 - n);
    assert!(ones.len() == 2 && (ones[0].0 as f64 / life - 1.0).abs() < 0.1 && (ones[1].0 as f64 / (life * x) - 1.0).abs() < 0.1, "lives {ones:?}");
    assert!(node.len() >= 6 && (dc / ones[0].0 as f64 - 1.0).abs() < 0.1, "node: mean life {dc} days against {ones:?}");
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// What a member takes, can build and pays to be goes with its mass to the power `intake_exp`
/// (14 Oct 2026, review 4, finding 5): 1 kg counts as 1 kg, 10 t takes 180 times what 10 kg does
/// (not 1,000), and a node of n members counts n times a member. Past `full` times its copy mass
/// it counts by all its mass (satiety stops what it takes, `a_body_takes_no_more_than_it_can_use`).
#[test]
fn equiv_goes_with_mass_to_the_power() {
    use pcore::body::{Body, COPY_MASS};
    let p = pcore::Physics::default();
    let member = |count: f64, copy: f32| {
        let mut genes = vec![0.0f32; 64];
        genes[COPY_MASS] = copy;
        Body { count, genes, ..Default::default() }
    };
    let take = |kg: f64| member(1.0, 6.0).equiv(&p, kg);
    assert!((take(1.0) - 1.0).abs() < 1e-12, "a member of the reference mass counts as before");
    assert!((take(1e4) / take(10.0) - 177.8).abs() < 0.1, "10 t takes 180 times 10 kg: {}", take(1e4) / take(10.0));
    assert!((member(4.0, 6.0).equiv(&p, 4.0 * 30.0) / take(30.0) - 4.0).abs() < 1e-9, "a node counts n members' worth");
    let m = member(1.0, 1.0);
    assert!(m.equiv(&p, 2000.0) > m.equiv(&p, 20.0), "past full (20 kg) it counts by all its mass");
}

/// Building takes time (14 Oct 2026, review 5): a body pays into its child store at most
/// `c_build` x (member mass / `m_ref`)^`intake_exp` x members x the days of its interval,
/// whatever its reactions could release. An eater of 300 kg that owes many children (as in
/// `a_heavy_individual_copies_over_several_updates`) adds to its store `c_build` x 300^0.75 J a day, and
/// a body of half the mass adds 2^0.75 times less.
#[test]
fn a_body_builds_at_most_c_build_times_mass_to_the_power_a_day() {
    use pcore::body::{Lineage, DISPERSE, STRENGTH};
    use pcore::{body, brain, change, choose, flow};
    // The child store after each update of an eater of `times` x 30 kg, with the cap each time.
    let run = |times: f64| {
        let mut w = worldgen::new_world(1, 16, 16);
        w.cfg.phys.c_copy *= 100.0;
        pcore::step(&mut w);
        let n = w.grid.n();
        let i = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
        let shape = choose::shape(w.chem.ns(), 12, 6);
        let mut rest = [-8.0; brain::FIXED];
        rest[0] = 8.0;
        let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, rest, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
        let rows = base.split_off(shape.base());
        let (l, g) = (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 0 }), w.life.new_genome(rows));
        let mut b = worldgen::life::make(&mut w, 1, i, l, g).expect("body");
        for k in 0..w.chem.ns() {
            w.chem.add_elems(k, (times - 1.0) * b.store[k], &mut w.ledger.imposed_elems);
            (w.ledger.imposed_energy, b.store[k]) = (w.ledger.imposed_energy + (times - 1.0) * b.store[k] * w.chem.subs[k].hf, times * b.store[k]);
        }
        (w.ledger.imposed_energy, b.e, b.genes[DISPERSE], b.genes[STRENGTH]) = (w.ledger.imposed_energy + (times - 1.0) * b.e, times * b.e, 0.0, 0.0);
        let s = w.life.add(b, w.day, 1);
        let (mut log, mut last, dt) = (Vec::new(), 0.0, w.cfg.phys_days as f64 * 86_400.0);
        for _ in 0..60 {
            if (w.day % w.cfg.days_per_month as u64).is_multiple_of(w.cfg.phys_days as u64) {
                body::survey(&mut w);
                flow::step(&mut w, dt);
                change::step(&mut w, dt);
            }
            body::day(&mut w);
            let b = &mut w.life.bodies[s];
            if b.alive && b.last == w.day && b.paid > last {
                b.prepare(&w.chem, &w.cfg.phys);
                let cap = w.cfg.phys.c_build * b.equiv(&w.cfg.phys, b.mass(&w.chem)) / w.cfg.phys.m_ref * b.interval() as f64;
                log.push((b.paid - last, cap));
                last = b.paid;
            }
            w.day += 1;
        }
        log
    };
    let (big, small) = (run(10.0), run(5.0));
    println!("added to the child store, J, with its cap: 300 kg {big:?}, 150 kg {small:?}");
    assert!(big.len() >= 3 && small.len() >= 3, "the store never grew");
    // The cap binds (these two owe far more than their reactions release in an interval), and the
    // mass it is for falls a little as they burn (a few per cent between the two measures).
    for &(added, cap) in big.iter().chain(&small) {
        assert!(added <= cap * 1.05 && added >= cap * 0.9, "added {added:e} J, cap {cap:e}");
    }
    assert!((big[0].0 / small[0].0 / 2f64.powf(0.75) - 1.0).abs() < 0.05, "a body twice as heavy builds 2^0.75 times faster");
}


/// The pay-day cap (review 6, finding 6): `body::live` copies a member past `full` and then the
/// brain's copy (`choose::act`) calls `copy::copy` again in the same update; building takes time,
/// so the two together pay no more than one interval's allowance, `c_build` x `equiv` / `m_ref` x
/// the days of its interval, and the second pays nothing. An update on a later day pays again, and
/// a body that has not paid for a long time still gets one interval's worth, not the days since.
#[test]
fn two_copies_in_one_update_pay_one_intervals_allowance() {
    use pcore::body::{Lineage, DISPERSE, STRENGTH};
    use pcore::{brain, choose};
    let mut w = worldgen::new_world(1, 16, 16);
    w.cfg.phys.c_copy *= 100.0;
    pcore::step(&mut w);
    let n = w.grid.n();
    let i = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
    let shape = choose::shape(w.chem.ns(), 12, 6);
    let mut rest = [-8.0; brain::FIXED];
    rest[0] = 8.0;
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, rest, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let (l, g) = (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 0 }), w.life.new_genome(rows));
    let mut b = worldgen::life::make(&mut w, 1, i, l, g).expect("body");
    // Ten times a starting eater: it owes far more than one interval's allowance (the cap binds).
    let times = 10.0;
    for k in 0..w.chem.ns() {
        w.chem.add_elems(k, (times - 1.0) * b.store[k], &mut w.ledger.imposed_elems);
        (w.ledger.imposed_energy, b.store[k]) = (w.ledger.imposed_energy + (times - 1.0) * b.store[k] * w.chem.subs[k].hf, times * b.store[k]);
    }
    (w.ledger.imposed_energy, b.e, b.genes[DISPERSE], b.genes[STRENGTH]) = (w.ledger.imposed_energy + (times - 1.0) * b.e, times * b.e, 0.0, 0.0);
    let s = w.life.add(b, w.day, 1);
    pcore::body::survey(&mut w);
    w.day += 400;
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    b.prepare(&w.chem, &w.cfg.phys);
    let cap = w.cfg.phys.c_build * b.equiv(&w.cfg.phys, b.mass(&w.chem)) / w.cfg.phys.m_ref * b.interval() as f64;
    // Never paid, 400 days gone: one interval's worth, no more.
    assert!(!pcore::copy::copy(&mut w, s, &mut b), "no child is finished yet");
    let first = b.paid;
    assert!(first > 0.9 * cap && first <= cap * 1.0001, "the first call paid {first:e} J of {cap:e}");
    // The second call in the same update: nothing more.
    pcore::copy::copy(&mut w, s, &mut b);
    assert_eq!(b.paid, first, "the second call paid {:e} J more", b.paid - first);
    // The next interval pays again.
    w.day += b.interval();
    pcore::copy::copy(&mut w, s, &mut b);
    assert!(b.paid > 1.8 * first && b.paid <= 2.0001 * cap.max(first), "{:e} J after two intervals, one is {first:e}", b.paid);
    assert_eq!(pcore::check::broken(&w), None);
}


/// Every body pays a basal upkeep (14 Oct 2026, review 5): `c_basal` per day for a member of the
/// reference mass, times (member mass / reference)^0.75, per member, strength on top. A kind with
/// strength 0 (the eaters of remains) used to pay none of the body cost; now a 1 kg one pays
/// `c_basal` a day and a 100 kg one 100^0.75 = 31.6 times that, 3.2 times less per kg.
#[test]
fn a_body_with_no_strength_pays_a_basal_upkeep() {
    use pcore::body::{REPAIR, STRENGTH};
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let i = (0..w.grid.n()).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
    let shape = pcore::choose::shape(w.chem.ns(), 12, 6);
    let mut base = pcore::brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, [0.0; pcore::brain::FIXED], 0.0, [0.0; pcore::brain::VERBS + pcore::brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let (l, g) = (w.life.new_lineage(pcore::body::Lineage { shape, seen: 3, sight: 1, base, root: 5 }), w.life.new_genome(rows));
    // Eaters of remains (kind 5: strength 0, 1 kg); repair 0 so the factor is 1.
    let mut b = worldgen::life::make(&mut w, 5, i, l, g).expect("body");
    assert_eq!(b.genes[STRENGTH], 0.0);
    b.genes[REPAIR] = 0.0;
    b.prepare(&w.chem, &w.cfg.phys);
    let (days, mass) = (10.0, b.mass(&w.chem));
    let basal = |w: &mut pcore::World, b: &pcore::body::Body, m: f64| {
        let with = pcore::body::upkeep(w, b, days, m);
        let p0 = w.cfg.phys;
        w.cfg.phys.c_basal = 0.0;
        let without = pcore::body::upkeep(w, b, days, m);
        w.cfg.phys = p0;
        (with - without) / days
    };
    let (one, hundred) = (basal(&mut w, &b, 1.0), basal(&mut w, &b, 100.0));
    println!("basal upkeep of a 1 kg member {one:.4e} J/day, of 100 kg {hundred:.4e}, member of {mass} kg");
    assert!((one / w.cfg.phys.c_basal - 1.0).abs() < 1e-9, "1 kg pays c_basal: {one}");
    assert!((hundred / one / 100f64.powf(0.75) - 1.0).abs() < 1e-9, "100 kg pays 100^0.75 times that: {}", hundred / one);
    // A node of 5 members pays 5 times a member's.
    let mut n = b.clone();
    n.count = 5.0;
    assert!((basal(&mut w, &n, 500.0) / (5.0 * hundred) - 1.0).abs() < 1e-9, "a node pays for each member");
}

/// Leaf goes with mass^0.75 like intake (14 Oct 2026, review 5): the same `LEAF` gene gives a
/// member of 1 kg 10^LEAF m² per kg, a member of 1,000 kg 1000^0.75 times that in all (178 m², not
/// 1,000), so 5.6 times less per kg; a node counts its members; and past `full` times the copy
/// mass what a member holds adds no leaf. The open surface follows (plus its skin).
#[test]
fn leaf_area_per_kg_falls_with_mass() {
    use pcore::body::{Body, COPY_MASS, LEAF};
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let (chem, p) = (w.chem.clone(), w.cfg.phys);
    // A member of `kg` that copies at 10^6 kg (so it all works), 0.5 m² of leaf per kg of 1 kg.
    let member = |count: f64, kg: f64, copy: f32| {
        let mut b = Body { count, genes: vec![0.0; pcore::body::Layout::of(&chem).len], store: vec![0.0; chem.ns()], ..Default::default() };
        (b.genes[LEAF], b.genes[COPY_MASS]) = (0.5f32.log10(), copy);
        // Fill the stores of the first substance up to `kg` per member.
        b.store[0] = count * kg / chem.subs[0].molar_mass;
        b.prepare(&chem, &p);
        b
    };
    let leaf = |b: &Body| b.visible(&chem, &p)[1] as f64;
    let (one, tonne) = (member(1.0, 1.0, 6.0), member(1.0, 1000.0, 6.0));
    println!("leaf of a member: 1 kg {:.3} m², 1,000 kg {:.3} m²", leaf(&one), leaf(&tonne));
    assert!((leaf(&one) - 0.5).abs() < 1e-6, "1 kg: {}", leaf(&one));
    assert!((leaf(&tonne) / leaf(&one) / 1000f64.powf(0.75) - 1.0).abs() < 1e-6, "1,000 kg: {}", leaf(&tonne));
    assert!((leaf(&tonne) / 1000.0 / (leaf(&one) / 1.0) - 1000f64.powf(-0.25)).abs() < 1e-6, "per kg it falls");
    // A node of 10 members of 1,000 kg: the open surface is 10 members' leaf plus their skin.
    let node = member(10.0, 1000.0, 6.0);
    let (m, skin) = (node.mass(&chem), 10.0 * 6.0 * (1000.0 / p.rho_body).powf(2.0 / 3.0));
    assert!((node.open_area(&p, m) - (10.0 * leaf(&tonne) + skin)).abs() < 1e-6 * skin, "open area {}", node.open_area(&p, m));
    // Past full (2 x 10^1 = 20 kg) a member's extra mass adds no leaf: 20 kg and 2,000 kg alike.
    let (at, past) = (member(1.0, 20.0, 1.0), member(1.0, 2000.0, 1.0));
    assert!((leaf(&at) - leaf(&past)).abs() < 1e-9 && (leaf(&at) / 0.5 - 20f64.powf(0.75)).abs() < 1e-6, "past full: {} and {}", leaf(&at), leaf(&past));
}
