use pcore::body::{Layout, COOL_T, DENATURE, KEEP_T, MUT};
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
    // (The two temperature genes step by 5 K, the others by at most 1.)
    assert!(diff.iter().enumerate().all(|(g, &d)| d <= 6.0 * size * if g == DENATURE || g == KEEP_T || g == COOL_T { 5.0 } else { 1.0 }), "change too big");
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
        // (No growth: building body matter takes the solid too, and this is about the burning. And no body can pay to move: with liquid
        // in the pores of the ground the cells they feel are not the dry ones of before, and a node that walks out of the hot cell
        // does not burn.)
        w.cfg.phys.c_build = 0.0;
        w.cfg.phys.c_move = 1e12;
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
    assert_eq!(w.diag.wide, 0.0, "alike bodies merged and some were counted wide");
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
    // Copy masses 0.3 apart are 1.5 steps apart each (`alike` is 4): the most alike pairs merge
    // first, so the cell may reach `crowd` with no wide merge; every unlike merge (past twice
    // `crowd` nodes) is wide too.
    assert!(w.diag.unlike <= w.diag.wide, "wide {} unlike {}", w.diag.wide, w.diag.unlike);
    assert!(after.len() <= w.cfg.crowd, "{} nodes left", after.len());
    assert!((after.iter().sum::<f64>() - before).abs() < 1e-9 * before, "members {before} then {after:?}");
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// The cost of living follows temperature (15 Oct 2026): what a starting body's reactions can release over
/// its interval falls with its temperature by Change's rate, and so does its upkeep, by the same law, so
/// the cold does not leave it short (before, upkeep was the same at every temperature and the cold bodies
/// could not pay: release over upkeep fell from 4.9 at 290 K to 0.01 at 250 K for an eater). In the warmth
/// upkeep rises faster than what the body can release (catalysts, the air it breathes), so a warm body
/// has less to spare.
#[test]
fn cold_bodies_release_and_pay_less() {
    let mut w = worldgen::new_world(1, 32, 32);
    pcore::step(&mut w);
    // (The starting bodies unsprayed: a sprayed one can be anything, and the first found is a lottery.)
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
    worldgen::life::seed_with(&mut w, 1, &worldgen::life::Seeding::old());
    for kind in [true, false] {
        let mut b = w.life.alive().find(|b| (pcore::body::light_cat(&w.chem, &b.genes) > 5.0) == kind).expect("a body").clone();
        b.prepare(&w.chem, &w.cfg.phys);
        let at = |t: f64| {
            let mut b = b.clone();
            b.e = b.heat_cap(&w.chem) * t;
            let days = b.interval() as f64;
            let (rel, up) = (pcore::body::release(&w, &b, days), pcore::body::upkeep(&w, &b, days, b.mass(&w.chem)));
            (rel, up, rel / up)
        };
        let t = [250.0, 270.0, 280.0, 290.0, 300.0];
        let r = t.map(at);
        println!("light-capturing {kind}: release / upkeep at 250, 270, 280, 290, 300 K: {:.3?}", r.map(|x| x.2));
        // Both rise with the temperature, the upkeep by 50 times or more (72 for the 7,760 K of burning the solid) between 250 K and 290 K
        // (2.5 per 10 K); the cold does not leave the body short, the warmth leaves it less to spare.
        assert!(r.windows(2).all(|p| p[1].0 > p[0].0 && p[1].1 > p[0].1), "{r:?}");
        assert!(r[3].1 > 50.0 * r[0].1, "{r:?}");
        assert!(r[0].2 > 1.0 && r[4].2 > 1.0 && r[4].2 < r[0].2, "{r:?}");
    }
}

/// A body whose safe minimums are all off can give all its stores to a body with room (here handling costs nothing), but
/// not its body: its matter (body and brood) and the liquid its body keeps stay (15 Oct 2026; before, it gave them too, and
/// left with nothing of a member it died at once). Nothing breaks the ledger (taking from an empty body once gave NaN).
#[test]
fn a_body_gives_its_stores_but_not_its_body() {
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
    // (Body matter given is digested into the solid, so it fills the taker's room for the solid
    // (`choose::digest_room`): room for all the giver's solid and matter, 2 kg per kg of body.)
    let rich = worldgen::life::roles(&w.chem).expect("roles").1;
    taker.genes[lay.keep + rich] = 2.0;
    let (matter, liquid) = (w.chem.matter().expect("matter"), worldgen::life::roles(&w.chem).expect("roles").2);
    let (m0, fuel0) = (giver.store[matter], giver.store[rich]);
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
    let b = &w.life.bodies[g];
    assert!(b.alive && b.store[matter] > 0.99 * m0 && b.store[rich] < 0.5 * fuel0, "matter {m0} -> {}, fuel {fuel0} -> {}", b.store[matter], b.store[rich]);
    assert!(b.store[liquid] >= 0.99 * b.level(&w.chem, liquid, b.mass(&w.chem)), "the liquid of its body was given");
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
    // The warmest animal that lives through the month on bare ground and under cover (a starting animal may starve in its
    // first month, a fifth of them do: the test is about the cover, so it takes the warmest one that lives).
    let bs = &w.life.bodies;
    let mut order: Vec<usize> = (0..bs.len()).filter(|&s| bs[s].alive && pcore::body::light_cat(&w.chem, &bs[s].genes) < 5.0).collect();
    order.sort_by(|&a, &b| bs[b].temp(&w.chem).total_cmp(&bs[a].temp(&w.chem)));
    let base = w.clone();
    let (w, covered, slot, i) = order
        .iter()
        .take(10)
        .find_map(|&slot| {
            let mut w = base.clone();
            let i = w.life.bodies[slot].cell as usize;
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
            pcore::step(&mut w);
            pcore::step(&mut covered);
            (w.life.bodies[slot].alive && covered.life.bodies[slot].alive).then_some((w, covered, slot, i))
        })
        .expect("one of the ten warmest animals lives through the month");
    let temp = |w: &pcore::World| w.life.bodies[slot].temp(&w.chem) - w.life.env[i][0];
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

/// A body takes no more than it can use (14 Oct 2026, review 5: satiety, in place of "past `full` it
/// takes at the rate of `full`", which review 3's M2 added so that a body that could not pay for its
/// children did not swell on what it held), at every size with `Config::sate_all` (the decision
/// after watch report 7, set here; by default only past `full`, see `tests/build.rs`): an eater of 1, 10 or 100 times the mass of a
/// starting one, holding fuel enough for many intervals, takes on the ground's energy-rich solid
/// only the heat of its upkeep over an interval, whatever `c_build`: never less (a hard stop below
/// upkeep starved the eaters, 13 Oct: burning leaves its liquid product in the body), and no more,
/// as its store already releases what it can use. A body whose store is below the target (upkeep
/// plus building over the share its reactions release, less what it holds) is asked directly:
/// `choose::sate` leaves what the formula says, more with a higher `c_build`, and never more than
/// its size takes in the time (review 6, findings 10 and 13). The bodies here neither grow nor
/// copy (a copy mass of a million kg, a copy cost that cannot be paid) nor soak liquid up, so that
/// nothing but satiety acts; what building adds to the target is in `tests/build.rs`. Matter and energy are
/// kept exactly.
#[test]
fn a_body_takes_no_more_than_it_can_use() {
    use pcore::body::{Lineage, DISPERSE};
    use pcore::event::EventKind;
    use pcore::{brain, choose};
    // Burnable kg taken in an update, the heat of the upkeep and build it may use, the size cap and
    // the target `choose::sate` is meant to give (all kg), for a body of `times` x 30 kg that keeps
    // the share `keep` of its fuel. With `direct`, instead of an update, `sate` is asked about
    // twice the size cap of the fuel and the first number is what it leaves.
    let took = |times: f64, c_build: f64, keep: f64, direct: bool| {
        let mut w = worldgen::new_world(1, 16, 16);
        (w.cfg.phys.c_build, w.cfg.phys.c_copy, w.cfg.sate_all) = (c_build, 1e15, true);
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
        // (A founder is placed as an adult since 15 Oct 2026, 63 kg: `times` a 30 kg one.)
        let times = times * 30.0 / b.mass(&w.chem);
        for k in 0..w.chem.ns() {
            w.chem.add_elems(k, (times - 1.0) * b.store[k], &mut w.ledger.imposed_elems);
            (w.ledger.imposed_energy, b.store[k]) = (w.ledger.imposed_energy + (times - 1.0) * b.store[k] * w.chem.subs[k].hf, times * b.store[k]);
        }
        // (An eater of the energy-rich solid only: it does not burn body matter, which would be fuel too.)
        let (matter, burn_matter) = worldgen::life::matter_roles(&w.chem).expect("matter");
        (b.genes[pcore::body::Layout::of(&w.chem).cat + burn_matter], b.genes[pcore::body::Layout::of(&w.chem).upt + matter]) = (-1.0, -0.1);
        (w.ledger.imposed_energy, b.e, b.genes[DISPERSE], b.genes[pcore::body::ALLOC], b.genes[pcore::body::LIQ]) = (w.ledger.imposed_energy + (times - 1.0) * b.e, times * b.e, 0.0, 0.0, -4.0);
        // Less of the fuel (the rest taken out as an edit).
        // (At the temperature it had: its upkeep goes with it, 15 Oct 2026.)
        let (out, t) = ((1.0 - keep) * b.store[rich], b.temp(&w.chem));
        w.chem.add_elems(rich, -out, &mut w.ledger.imposed_elems);
        (w.ledger.imposed_energy, b.store[rich]) = (w.ledger.imposed_energy - out * w.chem.subs[rich].hf, b.store[rich] - out);
        let e = t * b.heat_cap(&w.chem);
        (w.ledger.imposed_energy, b.e) = (w.ledger.imposed_energy + e - b.e, e);
        b.prepare(&w.chem, &w.cfg.phys);
        let (p, days, mass) = (w.cfg.phys, b.interval() as f64, b.mass(&w.chem));
        let (equiv, joule_per_kg) = (b.supply(&w.chem, &p), -w.chem.dh(burn) / w.chem.reactions[burn].inputs.iter().find(|x| x.0 as usize == rich).unwrap().1 as f64 / w.chem.subs[rich].molar_mass);
        let (up, build) = (pcore::body::upkeep(&w, &b, days, mass), p.c_build * equiv / p.m_ref * days);
        let can = up + build;
        // (up + build) x fuel / release - fuel, never under the upkeep, never over what its size takes.
        let (fuel, release) = (b.fuel(&w.chem, b.temp(&w.chem)), pcore::body::release(&w, &b, days));
        let target = ((can * fuel / release - fuel).max(up)).min(p.take * days * equiv * joule_per_kg) / joule_per_kg;
        if direct {
            let mut m = vec![0.0; w.chem.ns()];
            m[rich] = 2.0 * p.take * days * equiv / w.chem.subs[rich].molar_mass;
            choose::sate(&w, &b, &mut m, days, mass);
            return (m[rich] * w.chem.subs[rich].molar_mass, can / joule_per_kg, p.take * days * equiv, target);
        }
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
        // (Its upkeep goes with its temperature, which its update has settled by the time it takes: so the upkeep after.)
        let after = &w.life.bodies[s];
        let can = pcore::body::upkeep(&w, after, days, after.mass(&w.chem)) + build;
        (burnt, can / joule_per_kg, p.take * days * equiv, target)
    };
    // (Up to 30 times a starting eater, 900 kg: at 100 times upkeep, by the body, is above what it can take in an
    // interval, the energy ceiling of 14 Oct 2026 - it takes 119 kg of fuel against an upkeep of 141.)
    for times in [1.0, 10.0, 30.0] {
        let ((with, _, cap, _), (without, upkeep, _, _)) = (took(times, 3e4, 1.0, false), took(times, 0.0, 1.0, false));
        println!("{times} x 30 kg: takes {with:.3} kg of fuel (upkeep {upkeep:.3}, size cap {cap:.3}), with no building {without:.3}");
        assert!(upkeep < cap, "the size cap {cap} is not above its upkeep {upkeep}");
        // (3%, not 2%, since 16 Oct 2026: the 900 kg body, whose boundary layer goes with its size, is warmer and its upkeep
        // moves more between its decision and the end of its update; 2.0% measured.)
        assert!((with / upkeep - 1.0).abs() < 0.03 && (without / upkeep - 1.0).abs() < 0.03, "{with} and {without} of {upkeep} kg");
    }
    // With a store below the target, upkeep and build over the share its reactions
    // release in an interval, less what it holds: the target rises with `c_build`, between the
    // upkeep's worth and what its size takes (here a store of 10% of the fuel: 15.5%
    // before the body held matter and fuel apart, 17% before strength and catalysts went by mass^0.75,
    // 27% before they went by the kg of the body, 29.5% before upkeep went with the temperature, 15 Oct
    // 2026: the window is narrow, a few points of store; the body keeps its temperature when the fuel is taken out).
    // (A store of 0.3%, not 10%, since 16 Oct 2026: at a reptile's resting rate 10% of the fuel covers more than an interval's
    // upkeep and build, and the target is the upkeep's worth.)
    let sated: Vec<(f64, f64, f64, f64)> = [0.0, 3e4, 6e4].iter().map(|&c| took(10.0, c, 0.003, true)).collect();
    println!("sate asked for twice the size cap, store 0.3%: (left, upkeep+build, cap, target) {sated:?}");
    for &(left, _, cap, target) in &sated {
        assert!((left / target - 1.0).abs() < 1e-6, "sate left {left} kg, the formula says {target} kg");
        assert!(target < cap, "the size cap {cap} binds, the case shows nothing");
    }
    assert!(sated[0].0 > sated[0].1 * 1.2, "below the line of the upkeep it should take more than the upkeep's worth: {sated:?}");
    assert!(sated[1].0 > sated[0].0 * 1.1 && sated[2].0 > sated[1].0 * 1.1, "it takes no more with a higher c_build: {sated:?}");
    // With a store of 0.1% of the fuel and a building allowance of 6e5 (3e4 before the reptile's resting rate of 16 Oct 2026,
    // when its upkeep alone was enough) the target is far over the size cap: the cap is what it takes.
    let (left, _, cap, _) = took(10.0, 6e5, 0.001, true);
    assert!((left / cap - 1.0).abs() < 1e-6, "sate left {left} kg of the {cap} kg its size takes");
}

/// A brain of no hidden units is no brain (review 3, Q2): a body of such a lineage takes no
/// decision and pays nothing for a brain, but lives its physics at its updates and has offspring from its
/// brood (W3). Matter and energy are kept exactly.
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
    // A light-eater node whose members are mature: a body (0.7 kg of the 1 kg) of more than their copy mass (0.25 kg).
    let mut b = worldgen::life::make(&mut w, 0, i, l, g).expect("a node");
    (b.genes[HIDDEN], b.genes[COPY_MASS]) = (0.0, -0.6);
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
    // (Its children are small against its members, 0.075 kg against 0.7: another size class, so they are nodes of their own.)
    let all: f64 = w.life.alive().map(|x| x.n()).sum();
    assert!(b.alive && w.life.updates - u >= 2 && born > 0.0 && all > n0, "no children: {born}");
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
    use pcore::body::{Lineage, AGE, DISPERSE, LEARN, REPAIR, STRENGTH};
    use pcore::event::EventKind;
    use pcore::{brain, choose};
    let mut w = worldgen::new_world(1, 16, 16);
    // Bodies that neither grow nor soak liquid up (their mass, which the wear goes by, stays): no
    // building work, and no liquid exchange with the ground (14 Oct 2026: growth is building).
    w.cfg.phys.c_build = 0.0;
    // Upkeep goes with the temperature (15 Oct 2026): the bodies stand on ground held at the temperature the costs
    // are anchored to (`Physics::t_cat`, booked as an edit), so that they pay what the costs say, in two cells of
    // high ground (rain does not flood it) with other air nodes (a node would take the air an individual breathes).
    pcore::step(&mut w);
    let mut land: Vec<usize> = (0..w.grid.n()).filter(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).collect();
    land.sort_by(|&a, &b| w.life.env[b][2].total_cmp(&w.life.env[a][2]));
    let air = w.grid.air_of(land[0]);
    let cells = [land[0], *land.iter().find(|&&i| w.grid.air_of(i) != air).expect("another air node")];
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
        // (And do not learn: with liquid in the pores of the ground the cell they feel is another, and a body that learns from its
        // well-being may learn to walk away in the months of the test.)
        // (They soak liquid since 16 Oct 2026: at a reptile's resting rate their burning makes too little liquid to make up what
        // they breathe out, so they dried, paid less and lived a quarter longer; soaking keeps them at their level.)
        (b.genes[STRENGTH], b.genes[DISPERSE], b.genes[REPAIR], b.genes[pcore::body::LIQ], b.genes[LEARN]) = (0.0, 0.0, repair, 1.0, -6.0);
        // A node of `count` such members (its matter and energy booked as an edit), with three times the fuel (the
        // two months they settle and the long life of one of them cost more than the 9 kg a starting eater holds).
        let rich = worldgen::life::roles(&w.chem).expect("roles").1;
        b.store[rich] *= 3.0;
        w.chem.add_elems(rich, 2.0 * b.store[rich] / 3.0, &mut w.ledger.imposed_elems);
        w.ledger.imposed_energy += 2.0 * b.store[rich] / 3.0 * w.chem.subs[rich].hf;
        for k in 0..w.chem.ns() {
            w.chem.add_elems(k, (count - 1.0) * b.store[k], &mut w.ledger.imposed_elems);
            (w.ledger.imposed_energy, b.store[k]) = (w.ledger.imposed_energy + (count - 1.0) * b.store[k] * w.chem.subs[k].hf, count * b.store[k]);
        }
        (w.ledger.imposed_energy, b.e, b.count) = (w.ledger.imposed_energy + (count - 1.0) * b.e, count * b.e, count);
        b.prepare(&w.chem, &w.cfg.phys);
        let (d, m) = (b.interval() as f64, b.mass(&w.chem));
        let u = pcore::body::upkeep(w, &b, d, m) / (d * b.built(&w.chem));
        (w.life.add(b, w.day, 1), u)
    };
    let (sa, _) = make(&mut w, cells[0], 0.0, 1.0);
    let (sb, _) = make(&mut w, cells[0], 0.5, 1.0);
    let (c, _) = make(&mut w, cells[1], 0.0, 1000.0);
    // A day: the ground held, the flow and change steps when due, then the bodies due; the age deaths of the day.
    let t_cat = w.cfg.phys.t_cat;
    let day = |w: &mut pcore::World, deaths: &mut Vec<(u64, f64)>| {
        for &i in &cells {
            let e = w.energy_ground(i, t_cat);
            w.ledger.imposed_energy += e - w.e_ground[i];
            w.e_ground[i] = e;
        }
        if (w.day % w.cfg.days_per_month as u64).is_multiple_of(w.cfg.phys_days as u64) {
            let dt = w.cfg.phys_days as f64 * 86_400.0;
            pcore::body::survey(w);
            pcore::flow::step(w, dt);
            pcore::change::step(w, dt);
        }
        w.events.clear();
        pcore::body::day(w);
        deaths.extend(w.events.iter().filter(|e| matches!(e.kind, EventKind::Died { cause: AGE, .. })).map(|e| (e.day, e.value)));
        w.day += 1;
    };
    // Two months for the bodies to settle at the temperature their heat and the ground give them (an individual does
    // not age: `e_life` is large), then their upkeep per kg of body and day, their wear put back to nothing.
    let mut settle = Vec::new();
    for _ in 0..60 {
        day(&mut w, &mut settle);
    }
    assert!(w.life.bodies[sa].alive && w.life.bodies[sb].alive, "a body died while settling");
    let per_kg = |w: &mut pcore::World, s: usize| {
        let mut b = std::mem::take(&mut w.life.bodies[s]);
        b.prepare(&w.chem, &w.cfg.phys);
        let (d, m) = (b.interval() as f64, b.mass(&w.chem));
        let u = pcore::body::upkeep(w, &b, d, m) / (d * b.built(&w.chem));
        let act = b.activity(&w.chem, &w.cfg.phys);
        b.wear = 0.0;
        w.life.bodies[s] = b;
        (u, act)
    };
    let ((ua, aa), (ub, ab), (uc, ac)) = (per_kg(&mut w, sa), per_kg(&mut w, sb), per_kg(&mut w, c));
    println!("upkeep per kg of body and day: {ua:.1}, {ub:.1}, {uc:.1} J");
    // (The repair gene costs 0.75 + 0.25 x 2^0.5 times as much at 0.5; the second body makes a little more heat, so it is a little warmer.)
    assert!(((ub / ab) / (ua / aa) / (0.75 + 0.25 * 2f64.sqrt()) - 1.0).abs() < 0.03 && ((uc / ac) / (ua / aa) - 1.0).abs() < 0.07, "upkeep per kg {ua} {ub} {uc}, activity {aa} {ab} {ac}");
    // A life of 60 days at gene 0 (its upkeep per kg now), 10^0.5 ua / ub (10^0.5 / (0.75 + 0.25 x 2^0.5)) times that at 0.5.
    let (life, x) = (40.0, 10f64.sqrt() * ua / ub);
    w.cfg.phys.e_life = life * ua;
    // (Lives are counted from the last update of the first individual: wear counts from there.)
    let (t0, l0, start, n0) = (pcore::check::totals(&w), w.ledger.clone(), w.life.bodies[sa].last, w.life.bodies[c].count);
    let mut deaths = Vec::new();
    for _ in 0..240 {
        day(&mut w, &mut deaths);
    }
    // The individuals' deaths (one member each) and the node's (a share of its members).
    let (ones, node): (Vec<_>, Vec<_>) = deaths.into_iter().map(|(d, v)| (d - start, v)).partition(|x| x.1 == 1.0);
    // The node's mean life: the days its members lived over those who died (it loses a share at
    // update; it may die of other causes later, so only its age deaths count).
    let (mut n, mut t, mut lived) = (n0, 0, 0.0);
    for &(day, dead) in &node {
        (lived, n, t) = (lived + n * (day - t) as f64, n - dead, day);
    }
    let dc = lived / (n0 - n);
    println!("lives, days: individuals {ones:?} at genes 0 and 0.5 (predicted {life} and {:.0}); node {dc:.1} ({:.0} of {n0} members died of age)", life * x, n0 - n);
    assert!(ones.len() == 2 && (ones[0].0 as f64 / life - 1.0).abs() < 0.1 && (ones[1].0 as f64 / (life * x) - 1.0).abs() < 0.1, "lives {ones:?}");
    // (The node's members live the same time as an individual would at their own upkeep, `ua / uc` times as long.)
    assert!(node.len() >= 6 && (dc / (life * ua / uc) - 1.0).abs() < 0.1, "node: mean life {dc} days against {} predicted", life * ua / uc);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// What a member takes and can build goes with its mass to the power `intake_exp`
/// (14 Oct 2026, review 4, finding 5): 1 kg counts as 1 kg, 10 t takes 180 times what 10 kg does
/// (not 1,000), and a node of n members counts n times a member. `equiv` counts the mass it is given;
/// `Body::supply` gives it the body that is built, not the fuel stored (satiety stops what it takes,
/// `a_body_takes_no_more_than_it_can_use`).
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
    assert!(m.equiv(&p, 2000.0) > m.equiv(&p, 20.0), "equiv counts the mass it is given");
    // What a body takes and builds counts its body, not the fuel it stores (`Body::supply`, 14 Oct 2026,
    // the energy ceiling): the same body holding three times the fuel has the same supply.
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let rich = worldgen::life::roles(&w.chem).expect("roles").1;
    let mut b = Body { count: 1.0, genes: vec![0.0; pcore::body::Layout::of(&w.chem).len], store: vec![0.0; w.chem.ns()], ..Default::default() };
    b.genes[COPY_MASS] = 1.0;
    let matter = w.chem.matter().expect("body matter");
    (b.store[matter], b.store[rich]) = (10.0, 20.0);
    let thin = b.supply(&w.chem, &w.cfg.phys);
    b.store[rich] *= 3.0;
    assert!(thin > 0.0 && (b.supply(&w.chem, &w.cfg.phys) / thin - 1.0).abs() < 1e-12, "fuel changed the supply");
    b.store[matter] *= 8.0;
    assert!((b.supply(&w.chem, &w.cfg.phys) / thin - 8f64.powf(0.75)).abs() < 1e-9, "supply goes with the body to the power 0.75");
}

/// Every body pays a basal upkeep (14 Oct 2026, review 5): `c_basal` per day and kg of its body
/// (since 14 Oct 2026, the energy ceiling: mass^1.0, not the mass^0.75 of review 5; the cost of keeping
/// tissue), strength on top. A kind with strength 0 (the eaters of remains) used to pay none of the
/// body cost; now a body of 1 kg pays `c_basal` a day and one of 100 kg a hundred times that, the same
/// per kg.
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
    let days = 10.0;
    // The body `f` times as big (all its stores), alone or as a node of `n` such members.
    let (chem, t_cat) = (w.chem.clone(), w.cfg.phys.t_cat);
    let sized = |f: f64, n: f64| {
        let mut x = b.clone();
        x.count = n;
        x.store.iter_mut().for_each(|a| *a *= f * n);
        // (At the temperature the costs are anchored to, where upkeep is what the costs say, 15 Oct 2026.)
        x.e = x.heat_cap(&chem) * t_cat;
        x
    };
    let basal = |w: &mut pcore::World, x: &pcore::body::Body| {
        let m = x.mass(&w.chem);
        let with = pcore::body::upkeep(w, x, days, m);
        let p0 = w.cfg.phys;
        w.cfg.phys.c_basal = 0.0;
        let without = pcore::body::upkeep(w, x, days, m);
        w.cfg.phys = p0;
        (with - without) / days
    };
    let (one, hundred) = (sized(1.0, 1.0), sized(100.0, 1.0));
    let (e1, e100) = (basal(&mut w, &one), basal(&mut w, &hundred));
    let (k1, k100) = (one.built(&w.chem), hundred.built(&w.chem));
    println!("basal upkeep of a member of {} kg ({k1:.3} kg of body) {e1:.4e} J/day, of {} kg ({k100:.3}) {e100:.4e}", one.mass(&w.chem), hundred.mass(&w.chem));
    // (Per kg of body of a kind that matures at `m_ref`, times (copy mass / `m_ref`)^-0.25: Kleiber across kinds, 15 Oct 2026.)
    let kind = (b.dec(pcore::body::COPY_MASS) / w.cfg.phys.m_ref).powf(w.cfg.phys.intake_exp - 1.0);
    assert!((e1 / k1 / w.cfg.phys.c_basal / kind - 1.0).abs() < 1e-9, "it pays c_basal per kg of body: {}", e1 / k1);
    assert!((e100 / e1 / 100.0 - 1.0).abs() < 1e-9, "100 times the body pays 100 times as much, the same per kg: {}", e100 / e1);
    // A node of 5 members pays 5 times a member's.
    assert!((basal(&mut w, &sized(100.0, 5.0)) / (5.0 * e100) - 1.0).abs() < 1e-9, "a node pays for each member");
}

/// The cost of keeping tissue goes with the kg of the body, mass^1.0, at every mass (14 Oct 2026, the energy
/// ceiling; the supply of a body goes with mass^0.75, `equiv_goes_with_mass_to_the_power`): the same
/// genes, a body 1, 10, 100 and 10,000 times the size pay upkeep in proportion to their body, so per kg
/// of body the same, with basal, catalysts, strength and learning all in it. The brain's decision
/// (mass^0.25 per member) and the leaf (by area) are left out here (set to nothing), and fuel is no
/// body: the same body holding three times the fuel pays the same.
#[test]
fn upkeep_per_kg_of_body_is_the_same_at_every_size() {
    use pcore::body::{LEARN, REPAIR, STRENGTH};
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let i = (0..w.grid.n()).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
    let shape = pcore::choose::shape(w.chem.ns(), 12, 6);
    let mut base = pcore::brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, [0.0; pcore::brain::FIXED], 0.0, [0.0; pcore::brain::VERBS + pcore::brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let (l, g) = (w.life.new_lineage(pcore::body::Lineage { shape, seen: 3, sight: 1, base, root: 1 }), w.life.new_genome(rows));
    // A starting eater: strength 0.6, a burning catalyst, learning on.
    let mut b = worldgen::life::make(&mut w, 1, i, l, g).expect("body");
    (b.genes[STRENGTH], b.genes[LEARN], b.genes[REPAIR]) = (0.6, -1.0, 0.0);
    b.prepare(&w.chem, &w.cfg.phys);
    // At the temperature the costs are anchored to, where upkeep is what the costs say (15 Oct 2026).
    b.e = b.heat_cap(&w.chem) * w.cfg.phys.t_cat;
    let p = &mut w.cfg.phys;
    (p.c_brain, p.c_hidden, p.c_memory, p.c_seen, p.c_sight, p.c_leaf) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let rich = worldgen::life::roles(&w.chem).expect("roles").1;
    let days = 10.0;
    let up = |w: &pcore::World, x: &pcore::body::Body| pcore::body::upkeep(w, x, days, x.mass(&w.chem));
    let per_kg: Vec<f64> = [1.0, 10.0, 100.0, 1e4]
        .iter()
        .map(|&f| {
            let mut x = b.clone();
            x.store.iter_mut().for_each(|a| *a *= f);
            x.e = x.heat_cap(&w.chem) * w.cfg.phys.t_cat;
            up(&w, &x) / x.built(&w.chem)
        })
        .collect();
    println!("upkeep per day and kg of body at 21 kg x 1, 10, 100, 10,000: {:?}", per_kg.iter().map(|u| u / days).collect::<Vec<_>>());
    assert!(per_kg[0] > 0.0);
    for u in &per_kg {
        assert!((u / per_kg[0] - 1.0).abs() < 1e-9, "upkeep per kg of body differs with size: {per_kg:?}");
    }
    // Strength, catalysts, learning and basal are each in it, by the kg of body (at the temperature the costs are anchored to).
    let p = w.cfg.phys;
    let kind = (b.dec(pcore::body::COPY_MASS) / p.m_ref).powf(p.intake_exp - 1.0);
    let want = days * kind * (p.c_basal + p.c_cat * b.k.cats + p.c_strength * b.gene(STRENGTH) + p.c_learn * b.learn_rate() as f64);
    assert!((per_kg[0] / want - 1.0).abs() < 1e-9, "per kg of body {} against the costs summed {want}", per_kg[0]);
    // Across kinds by the size a kind matures at (Kleiber, 15 Oct 2026): the same body of a kind that matures 16 times
    // larger pays half as much per kg.
    let mut big = b.clone();
    big.genes[pcore::body::COPY_MASS] += 16f32.log10();
    assert!((up(&w, &big) / up(&w, &b) - 0.5).abs() < 1e-6, "a kind 16 times larger pays {} of the upkeep per kg", up(&w, &big) / up(&w, &b));
    // Fuel is no body: three times the fuel, the same upkeep.
    let mut fat = b.clone();
    fat.store[rich] *= 3.0;
    fat.e = fat.heat_cap(&w.chem) * w.cfg.phys.t_cat;
    assert!((up(&w, &fat) / up(&w, &b) - 1.0).abs() < 1e-9, "stored fuel adds to the upkeep: {} and {}", up(&w, &fat), up(&w, &b));
}

/// Leaf goes with mass^0.75 like intake (14 Oct 2026, review 5): the same `LEAF` gene gives a
/// member of 1 kg 10^LEAF m² per kg, a member of 1,000 kg 1000^0.75 times that in all (178 m², not
/// 1,000), so 5.6 times less per kg; a node counts its members; and the fuel a member holds adds no
/// leaf (it goes by the body, with no cap). The open surface follows (plus its skin).
#[test]
fn leaf_area_per_kg_falls_with_mass() {
    use pcore::body::{Body, COPY_MASS, LEAF};
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let (chem, p) = (w.chem.clone(), w.cfg.phys);
    // A member of `kg` (that copies at 10^`copy` kg), 0.5 m² of leaf per kg of 1 kg.
    let member = |count: f64, kg: f64, copy: f32| {
        let mut b = Body { count, genes: vec![0.0; pcore::body::Layout::of(&chem).len], store: vec![0.0; chem.ns()], ..Default::default() };
        (b.genes[LEAF], b.genes[COPY_MASS]) = (0.5f32.log10(), copy);
        // Body matter for a living body of `kg` per member (`Body::built`: matter over 1 - `WET_TISSUE`; no key liquid).
        let m = chem.matter().expect("body matter");
        b.store[m] = count * kg * (1.0 - pcore::body::WET_TISSUE) / chem.subs[m].molar_mass;
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
    let m = node.mass(&chem);
    let skin = 10.0 * 6.0 * (m / 10.0 / p.rho_body).powf(2.0 / 3.0);
    assert!((node.open_area(&chem, &p, m) - (10.0 * leaf(&tonne) + skin)).abs() < 1e-6 * (10.0 * leaf(&tonne) + skin), "open area {}", node.open_area(&chem, &p, m));
    // Nothing caps the mass that works (14 Oct 2026: a body is mature from its copy mass but keeps growing, and leaf goes by all of
    // its body): a member of 10 kg (its copy mass) and one of 1,000 kg have leaf in the ratio of their bodies to the power 0.75.
    let (at, past) = (member(1.0, 10.0, 1.0), member(1.0, 1000.0, 1.0));
    assert!((leaf(&past) / leaf(&at) / 100f64.powf(0.75) - 1.0).abs() < 1e-5 && (leaf(&at) / 0.5 / 10f64.powf(0.75) - 1.0).abs() < 1e-5, "past maturity: {} and {}", leaf(&at), leaf(&past));
}

/// Moving costs the real cost of transport (14 Oct 2026, review 6, finding 2): per kg and m a 30 kg
/// member pays `c_move`, a smaller one more and a bigger one less, as m^-0.32 (Taylor et al.), so a
/// metre costs a member of 100 times the mass 100^0.68 = 23 times as much, not 100 times.
#[test]
fn moving_costs_the_real_cost_of_transport() {
    let p = pcore::Physics::default();
    assert_eq!(p.walk(30.0), p.c_move, "a 30 kg member pays c_move");
    let (small, big) = (p.walk(0.3) * 0.3, p.walk(30.0) * 30.0);
    assert!((big / small - 100f64.powf(0.68)).abs() < 1e-9, "100 times the mass, {} times the cost of a metre", big / small);
    assert!((p.walk(0.18) / p.walk(44.0) - (0.18f64 / 44.0).powf(-0.32)).abs() < 1e-9);
    // The real figure, 10.7 m^-0.32 J per kg per m, at 30 kg is 3.6: c_move 3 is the same order.
    assert!(p.c_move > 0.7 * 10.7 * 30f64.powf(-0.32) && p.c_move < 1.0 * 10.7 * 30f64.powf(-0.32));
}
