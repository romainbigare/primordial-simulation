//! Population nodes (plan 7.1): how their members differ, how they split, how they act on bonds.

use pcore::body::{Body, Layout, Lineage, COPY_MASS, DISPERSE, HIDDEN, MUT, STRENGTH};
use pcore::rng::{Purpose, Rng};
use pcore::{brain, check, choose, copy};

/// Slots of the live bodies.
fn live(w: &pcore::World) -> Vec<usize> {
    (0..w.life.bodies.len()).filter(|&s| w.life.bodies[s].alive).collect()
}

/// A node's children that stay widen its spread by their share of the mutation's variance; a node
/// whose members differ so much that its halves would not be alike splits in two along that gene,
/// keeping its members, stores, heat and pooled spread.
#[test]
fn a_node_spreads_with_its_children_and_splits_when_too_varied() {
    let mut w = worldgen::new_world(1, 32, 32);
    pcore::step(&mut w);
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
    let (t0, l0) = (check::totals(&w), w.ledger.clone());
    let s = live(&w).into_iter().find(|&s| w.life.bodies[s].count > 10.0).expect("a node");
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    // Members copy now (a low copy mass) and all children stay.
    (b.genes[COPY_MASS], b.genes[DISPERSE]) = (-1.0, 0.0);
    b.prepare(&w.chem, &w.cfg.phys);
    let n0 = b.count;
    assert!(b.spread.is_empty() && copy::copy(&mut w, s, &mut b) && b.count > n0, "no children");
    let (f, size) = (((b.count - n0) / b.count) as f32, 10f32.powf(b.genes[MUT]));
    let want = f * (w.cfg.phys.row_noise * size).powi(2);
    assert!(b.spread.iter().all(|&v| v > 0.0) && (b.row_spread - want).abs() <= 1e-5 * want, "{} vs {want}", b.row_spread);

    // Copy mass spread over 5 mutation steps (0.2 each): the halves are 2 sqrt(2/pi) 5 = 8 apart.
    b.spread[COPY_MASS] = 1.0;
    let (count, store, e, m) = (b.count, b.store.clone(), b.e, b.genes[COPY_MASS]);
    let before = live(&w);
    copy::split(&mut w, s, &mut b);
    w.life.bodies[s] = b;
    let new: Vec<usize> = live(&w).into_iter().filter(|x| !before.contains(x) && *x != s).collect();
    assert_eq!(new.len(), 1, "not split in two");
    let (a, c) = (&w.life.bodies[s], &w.life.bodies[new[0]]);
    assert_eq!((a.count + c.count, a.lineage, a.group, a.cell), (count, c.lineage, c.group, c.cell));
    for k in 0..store.len() {
        assert!((a.store[k] + c.store[k] - store[k]).abs() <= 1e-12 * store[k], "store {k}");
    }
    assert!((a.e + c.e - e).abs() <= 1e-12 * e);
    let d = (2.0 / std::f32::consts::PI).sqrt();
    // (which half keeps the node's place is drawn)
    let (x, y) = (a.genes[COPY_MASS] - m, c.genes[COPY_MASS] - m);
    assert!((x.abs() - d).abs() < 1e-5 && (x + y).abs() < 1e-5, "halves at {x} and {y}, not ±{d}");
    assert!((a.spread[COPY_MASS] + d * d - 1.0).abs() < 1e-5 && a.spread == c.spread, "pooled spread not kept");
    assert_eq!(check::broken(&w), None);
    let (de, dq) = check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Without selection the node math moves no gene (review 3, M1). Every member has one child and
/// dies at the same rate, whatever its genes (no copy cost, members far past their copy mass but
/// never `full`), so a gene's mean over all members may move only by the noise of the children
/// that leave. Light-eater nodes with a fast mutation (their spreads reach the split threshold)
/// copy, split, send children to join or start nodes in neighbour cells, and merge each month,
/// for 1,000 updates: every gene's mean stays within a mutation step of its start, in cells over
/// `crowd` and not.
#[test]
fn genes_do_not_drift_without_selection() {
    for crowd in [1, 64] {
        let mut w = worldgen::new_world(1, 4, 4);
        pcore::step(&mut w);
        (w.cfg.crowd, w.cfg.phys.c_copy, w.cfg.phys.full) = (crowd, 0.0, 1e12);
        worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
        let lay = Layout::of(&w.chem);
        // Members of 100 kg (stores of the start, per member) that copy from 0.1 kg.
        let b0 = &w.life.bodies[live(&w)[0]];
        let (per, e1) = (b0.store.iter().map(|a| 100.0 * a / b0.count).collect::<Vec<f64>>(), 100.0 * b0.e / b0.count);
        for s in live(&w) {
            (w.life.bodies[s].genes[COPY_MASS], w.life.bodies[s].genes[MUT]) = (-1.0, -0.8);
        }
        // Gene steps (from `steps` of all ones: 1 / step; the brain's sizes are not counted).
        let inv = copy::steps(&lay, &vec![1.0; lay.len]);
        let mean = |w: &pcore::World| {
            let (mut g, mut n) = (vec![0.0f64; lay.len], 0.0);
            for b in w.life.alive() {
                g.iter_mut().zip(&b.genes).for_each(|(x, &y)| *x += y as f64 * b.n());
                n += b.n();
            }
            g.iter().map(|x| x / n).collect::<Vec<f64>>()
        };
        let m0 = mean(&w);
        let (n0, splits0) = (live(&w).len(), w.diag.splits[0]);
        for _ in 0..1000 {
            w.day += 10;
            if w.day % w.cfg.days_per_month as u64 == 0 {
                pcore::body::survey(&mut w);
            }
            // In a random order, so no body is first by its slot.
            let (mut order, mut r) = (live(&w), Rng::new(1, w.day, 0, Purpose::Body));
            for k in (1..order.len()).rev() {
                order.swap(k, r.below(k + 1));
            }
            for s in order {
                let mut b = std::mem::take(&mut w.life.bodies[s]);
                (b.store, b.e) = (per.iter().map(|a| a * b.count).collect(), e1 * b.count);
                b.prepare(&w.chem, &w.cfg.phys);
                copy::copy(&mut w, s, &mut b);
                copy::split(&mut w, s, &mut b);
                w.life.bodies[s] = b;
            }
            // As many die as were born, the same share of every body.
            for s in live(&w) {
                w.life.bodies[s].count *= 0.5;
            }
            w.events.clear();
        }
        let m1 = mean(&w);
        let worst = (0..lay.len).filter(|&g| inv[g] > 0.0).map(|g| ((m1[g] - m0[g]) * inv[g] as f64).abs()).fold(0.0, f64::max);
        println!("crowd {crowd}: bodies {n0} -> {}, splits {}, largest move {worst:.3} steps", live(&w).len(), w.diag.splits[0] - splits0);
        assert!(crowd < 4 || w.diag.splits[0] - splits0 > 10.0, "too few splits");
        assert!(worst < 1.0, "a gene moved {worst} steps without selection");
    }
}

/// A node whose brain draws an action of Bond splits one member off as an individual of its
/// lineage, with its share of the stores, to act. One that holds a part after (its brain only
/// holds parts, and its force bears the part's weight) lives on; one that holds nothing (too weak
/// to bear the part, or its brain only breaks the ground) goes back into the node at once, which is
/// then as it was.
#[test]
fn a_node_acts_on_bonds_through_one_member() {
    // 0: holds a part of 5 kg, members of 40 kg at strength about 1 (they bear about 200 N, the
    // part weighs 49 N); 1: the same with the light-eater's own members (1 kg at strength about
    // 0.05: they bear about 0.25 N); 2: breaks the ground.
    let scene = |case: u8| {
        let hold = case < 2;
        let mut w = worldgen::new_world(1, 16, 16);
        pcore::step(&mut w);
        let (t0, l0) = (check::totals(&w), w.ledger.clone());
        let (n, ns) = (w.grid.n(), w.chem.ns());
        let i = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
        // A part of 5 kg of the cell's most plentiful loose substance.
        let k = (0..ns).max_by(|&a, &b| w.ground[a * n + i].total_cmp(&w.ground[b * n + i])).unwrap();
        let mut wall = vec![0.0; ns];
        wall[k] = 5.0 / w.chem.subs[k].molar_mass;
        let e = wall[k] * w.chem.subs[k].cp * w.life.env[i][0];
        (w.ground[k * n + i], w.e_ground[i]) = (w.ground[k * n + i] - wall[k], w.e_ground[i] - e);
        let p = pcore::bond::make(&mut w, i, wall, e, -1.0, 0.0);
        let (shape, mut fixed, mut verbs) = (choose::shape(ns, 12, 6), [-8.0; brain::FIXED], [-8.0; brain::VERBS + brain::OVERBS]);
        if hold { verbs[brain::VERBS] = 8.0 } else { fixed[7] = 8.0 }
        let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, fixed, -8.0, verbs);
        let rows = base.split_off(shape.base());
        let (l, g) = (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base }), w.life.new_genome(rows));
        let mut b = worldgen::life::make(&mut w, 0, i, l, g).expect("a node");
        b.spread = vec![0.01; b.genes.len()];
        if case == 0 {
            // (copying at 100 kg, so they do not copy instead)
            (b.genes[STRENGTH], b.genes[COPY_MASS], b.count) = (1.0, 2.0, b.count / 40.0);
        }
        let (n0, genes, kg) = (b.count, b.genes.clone(), b.mass(&w.chem) / b.count);
        w.life.add(b, w.day, 1);
        pcore::body::survey(&mut w);
        w.day += 1;
        pcore::body::day(&mut w);
        assert_eq!(w.diag.splits[1], 1.0, "no member split off");
        let one: Vec<&Body> = w.life.alive().filter(|b| b.lineage == l && b.count == 1.0).collect();
        let node = w.life.alive().find(|b| b.lineage == l && b.count > 1.0).expect("the node");
        if case == 0 {
            assert!(one.len() == 1 && w.parts.parts[p].id(p) == one[0].hold, "the member holds nothing");
            assert!((node.count + 1.0 - n0).abs() < 1e-9 * n0 && (one[0].mass(&w.chem) / kg - 1.0).abs() < 0.1);
        } else {
            assert!(one.is_empty() && (node.count - n0).abs() < 1e-9 * n0, "{} members", node.count);
            assert!(node.genes.iter().zip(&genes).all(|(a, b)| (a - b).abs() < 1e-4), "genes changed");
            assert!(w.parts.parts[p].held == pcore::bond::NOBODY, "the part is held");
        }
        assert_eq!(check::broken(&w), None);
        let (de, dq) = check::drift(&w, &t0, &l0);
        assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
    };
    scene(0);
    scene(1);
    scene(2);
}

/// A node whose mean brain-size gene moved a whole unit from its lineage's size moves to a new
/// lineage with that size at its update (review 3, M4): its brain carried over, in a group of its
/// own that keeps what its group learned; matter and energy as they were.
#[test]
fn a_node_whose_size_genes_moved_changes_lineage() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
    let (t0, l0) = (check::totals(&w), w.ledger.clone());
    let s = live(&w).into_iter().find(|&s| w.life.bodies[s].count > 10.0).expect("a node");
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    let (lin0, g0) = (b.lineage, b.group);
    let h0 = w.life.brains[lin0 as usize].as_ref().unwrap().shape.h;
    w.life.groups[g0 as usize].rows[0] += 0.5;
    let learned = w.life.groups[g0 as usize].rows[0] - w.life.born(&b)[0];
    b.genes[HIDDEN] += 0.9;
    copy::resize(&mut w, s, &mut b);
    assert_eq!(b.lineage, lin0, "moved for less than a unit");
    b.genes[HIDDEN] += 0.3;
    copy::resize(&mut w, s, &mut b);
    w.life.bodies[s] = b;
    let b = &w.life.bodies[s];
    let lin = w.life.brains[b.lineage as usize].as_ref().expect("its lineage");
    assert!(b.lineage != lin0 && b.group != g0 && lin.shape.h == h0 + 1, "no new lineage of {} hidden units", h0 + 1);
    assert_eq!((w.life.members[b.lineage as usize], w.life.groups[b.group as usize].members), (1, 1));
    assert_eq!(w.life.born(b).len(), lin.shape.rows() * lin.shape.r());
    assert!((w.life.groups[b.group as usize].rows[0] - w.life.born(b)[0] - learned).abs() < 1e-5, "what it learned is lost");
    for _ in 0..2 {
        pcore::step(&mut w);
    }
    assert_eq!(check::broken(&w), None);
    let (de, dq) = check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
