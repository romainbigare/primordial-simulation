//! Population nodes (plan 7.1): how their members differ, how they split, how they act on bonds.

use pcore::body::{Body, Layout, CHILD, COPY_MASS, DISPERSE, GAS, HIDDEN, MEMORY, MUT, SEEN, SIGHT, STRENGTH};
use pcore::life::Lineage;
use pcore::rng::{Purpose, Rng};
use pcore::{brain, check, choose, copy};

/// Set aside a brood of `k` children for `b` (all its members together) out of the matter it holds: what
/// `copy::grow` would have put there (14 Oct 2026: a body makes children from its brood).
fn brood(w: &pcore::World, b: &mut Body, k: f64) {
    let (chem, m) = (&w.chem, w.chem.matter().expect("body matter"));
    b.brood = k * b.dec(CHILD) * b.dec(COPY_MASS) * (1.0 - pcore::body::WET_TISSUE) / chem.subs[m].molar_mass;
}

/// Slots of the live bodies.
fn live(w: &pcore::World) -> Vec<usize> {
    (0..w.life.bodies.len()).filter(|&s| w.life.bodies[s].alive).collect()
}

/// A mature node's children start a node of their own (they are immature, `copy::class`) that carries the variance left
/// among them; when they have grown and join their parents' node (the month's merge, `copy::merge`) its spread
/// is widened by it; a node whose members differ so much that its halves would not be alike splits in
/// two along that gene, keeping its members, stores, heat and pooled spread.
#[test]
fn a_node_spreads_with_its_children_and_splits_when_too_varied() {
    let mut w = worldgen::new_world(1, 32, 32);
    pcore::step(&mut w);
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
    let (t0, l0) = (check::totals(&w), w.ledger.clone());
    let s = live(&w).into_iter().find(|&s| w.life.bodies[s].count > 10.0).expect("a node");
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    // Members are mature (a low copy mass), each has a child and all children stay in the cell.
    (b.genes[COPY_MASS], b.genes[DISPERSE]) = (-0.4, 0.0);
    b.prepare(&w.chem, &w.cfg.phys);
    let n0 = b.count;
    brood(&w, &mut b, n0);
    let before = live(&w);
    assert!(b.spread.is_empty() && copy::copy(&mut w, s, &mut b), "no children");
    assert_eq!(b.count, n0, "the children joined their mature parents");
    w.life.bodies[s] = b;
    let kids: Vec<usize> = live(&w).into_iter().filter(|x| !before.contains(x) && *x != s).collect();
    assert_eq!(kids.len(), 1, "the children are one node of their own");
    let c = &w.life.bodies[kids[0]];
    assert!((c.count - n0).abs() < 1e-6 * n0 && !c.adult(&w.chem), "{} young members", c.count);
    assert!(c.spread.iter().all(|&v| v >= 0.0) && c.spread.iter().any(|&v| v > 0.0) && c.row_spread > 0.0, "children keep no spread");
    // Grown, they join the node (as the month's merge does): its spread is widened.
    copy::merge(&mut w, kids[0], s);
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    assert!(b.count > 1.5 * n0 && b.spread.iter().any(|&v| v > 0.0) && b.row_spread > 0.0, "the merge widened nothing");

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
    for (k, &x) in store.iter().enumerate() {
        assert!((a.store[k] + c.store[k] - x).abs() <= 1e-12 * x, "store {k}");
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

/// Light-eater nodes with a fast mutation (their spreads reach the split threshold) and members
/// of 100 kg (a body of 70 kg) that are mature from 28 kg, with children of 14 kg (no copy cost; the copy-mass and child
/// genes are set again at each update so that every node is mature, and its children are of its size class, whatever
/// its other genes: no selection by when a body may reproduce) copy, split, move to the lineage
/// their brain sizes ask for, send children to join or start nodes in neighbour cells, and merge
/// each month, for `updates` updates of 10 days (`wide`: from spreads held at the split threshold). Every member has one child per update, and then
/// a share `live(b)` of each body's members lives on (0.5: as many die as were born). Returns the
/// mean genes by members at the start and at the end.
fn churn(crowd: usize, updates: usize, wide: bool, live_on: impl Fn(&Body) -> f64) -> (pcore::World, Vec<f64>, Vec<f64>) {
    let mut w = worldgen::new_world(1, 4, 4);
    pcore::step(&mut w);
    (w.cfg.crowd, w.cfg.phys.c_copy) = (crowd, 0.0);
    // One parent: the node math alone is measured. With two (15 Oct 2026) a body's partner is the most alike of its cell, which
    // need not choose it in return, so children are pulled toward the nodes that are many bodies' nearest, and the mean over all
    // members wanders (1.5 to 2.3 steps in 1,000 updates here, against 0.2 with one parent), as it does when the nearest of
    // each is not the one that chose it.
    w.cfg.two_parents = false;
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
    let len = Layout::of(&w.chem).len;
    let b0 = &w.life.bodies[live(&w)[0]];
    let (per, e1) = (b0.store.iter().map(|a| 100.0 * a / b0.count).collect::<Vec<f64>>(), 100.0 * b0.e / b0.count);
    for s in live(&w) {
        (w.life.bodies[s].genes[COPY_MASS], w.life.bodies[s].genes[MUT], w.life.bodies[s].genes[CHILD]) = (1.45, -0.8, -std::f32::consts::LOG10_2);
        if wide {
            // (the first children that stay bring every spread down to the split threshold)
            w.life.bodies[s].spread = vec![1e9; len];
        }
    }
    let mean = |w: &pcore::World| {
        let (mut g, mut n) = (vec![0.0f64; len], 0.0);
        for b in w.life.alive() {
            g.iter_mut().zip(&b.genes).for_each(|(x, &y)| *x += y as f64 * b.n());
            n += b.n();
        }
        g.iter().map(|x| x / n).collect::<Vec<f64>>()
    };
    let m0 = mean(&w);
    for _ in 0..updates {
        w.day += 10;
        if w.day.is_multiple_of(w.cfg.days_per_month as u64) {
            pcore::survey::survey(&mut w);
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
            // Mature with a body of twice its copy mass after the brood, children half the copy mass: of its size class.
            let (body, n) = (b.built(&w.chem) / b.n(), b.count);
            (b.genes[COPY_MASS], b.genes[CHILD], b.brood) = ((0.4 * body).log10() as f32, -std::f32::consts::LOG10_2, 0.0);
            brood(&w, &mut b, n);
            let (before, id) = (live(&w), b.id(s));
            copy::copy(&mut w, s, &mut b);
            copy::resize(&mut w, s, &mut b);
            copy::split(&mut w, s, &mut b);
            w.life.bodies[s] = b;
            // The children are young nodes of their own (immature, `copy::class`); here they grow up at once, and the month's
            // merge would take them back into their parent's node: they do now, so the update's node math is the same.
            let kids: Vec<usize> = live(&w).into_iter().filter(|k| *k != s && !before.contains(k) && w.life.bodies[*k].parent == id).collect();
            for k in kids {
                copy::merge(&mut w, k, s);
            }
        }
        for s in live(&w) {
            let f = live_on(&w.life.bodies[s]);
            w.life.bodies[s].count *= f;
        }
        w.events.clear();
    }
    let m1 = mean(&w);
    (w, m0, m1)
}

/// Without selection the node math moves no gene (review 3, M1), the brain's sizes included
/// (review 4, finding 1: they now split too): every body's members live on alike, so a gene's
/// mean over all members may move only by the noise of the children that leave. Over 1,000
/// updates every gene's mean stays within a mutation step of its start, in cells over `crowd`
/// and not.
#[test]
fn genes_do_not_drift_without_selection() {
    for crowd in [1, 64] {
        let w0 = worldgen::new_world(1, 4, 4);
        let lay = Layout::of(&w0.chem);
        // Gene steps (from `steps` of all ones: 1 / step; the brain's sizes, which `steps` leaves
        // out of likeness, by their own steps).
        let mut inv = copy::steps(&lay, &vec![1.0; lay.len]);
        for (g, step) in [(HIDDEN, 0.3), (MEMORY, 0.15), (SEEN, 0.1), (SIGHT, 0.07)] {
            inv[g] = 1.0 / step;
        }
        // (The copy mass and the child's size are set again at each update, see `churn`: not measured.)
        (inv[COPY_MASS], inv[CHILD]) = (0.0, 0.0);
        let (w, m0, m1) = churn(crowd, 1000, false, |_| 0.5);
        let worst = (0..lay.len).filter(|&g| inv[g] > 0.0).map(|g| ((m1[g] - m0[g]) * inv[g] as f64).abs()).fold(0.0, f64::max);
        let which = (0..lay.len).filter(|&g| inv[g] > 0.0).max_by(|&a, &b| ((m1[a] - m0[a]) * inv[a] as f64).abs().total_cmp(&((m1[b] - m0[b]) * inv[b] as f64).abs())).unwrap();
        println!("crowd {crowd}: bodies {}, splits {}, largest move {worst:.3} steps (gene {which}: {} -> {})", live(&w).len(), w.diag.splits[0], m0[which], m1[which]);
        assert!(crowd < 4 || w.diag.splits[0] > 10.0, "too few splits");
        assert!(worst < 1.0, "a gene moved {worst} steps without selection");
    }
}

/// With selection a node's brain size moves (review 4, finding 1): from spreads held at the split
/// threshold, members live on a little less the more hidden units their genes ask for (5% per unit
/// and update, as if they paid for a brain they do not use), and nothing else differs. Within
/// 1,000 updates the mean of the hidden-units gene falls by more than a unit (12 to 10.9) and the
/// nodes move to a lineage with fewer hidden units. Before, a node never split along a size gene,
/// so its mean stayed at 12 whatever the selection. Slow, as a split exposes one gene at a time (a
/// random pick among the widest): about 3% of what selection on the whole spread would move (0.07
/// units in 300 updates at 1% per unit, against 0.01 x 0.655 x 300 = 2).
#[test]
fn selection_moves_a_nodes_brain_size() {
    let w0 = worldgen::new_world(1, 4, 4);
    let h0 = 12.0;
    let (w, m0, m1) = churn(64, 1000, true, |b| 0.5 * (1.0 - 0.05 * (b.genes[HIDDEN] as f64 - h0)));
    let smaller = w.life.alive().filter(|b| w.life.brains[b.lineage as usize].as_ref().is_some_and(|l| (l.shape.h as f64) < h0)).count();
    println!("hidden {:.2} -> {:.2}, bodies of a smaller lineage {smaller} of {}", m0[HIDDEN], m1[HIDDEN], live(&w).len());
    assert!((m0[HIDDEN] - h0).abs() < 0.1 && Layout::of(&w0.chem).len == m0.len());
    assert!(m1[HIDDEN] < m0[HIDDEN] - 1.0, "hidden gene {:.2} -> {:.2}: selection did not move it", m0[HIDDEN], m1[HIDDEN]);
    assert!(smaller > 0, "no node moved to a smaller brain");
}

/// Children that leave a varied node join the node of their own kind in the next cell (review 3,
/// M3): pairs of children drawn from a node held at the split threshold in every gene (2.7 steps),
/// sent to a cell whose node has the same genes and spread, join it nearly always, as they lie
/// within its spread. With the means within `alike` steps alone (the pair's mean differs from the
/// node's by 2 steps in each gene) about half of them started a node of their own (108 of 200
/// joined), and cells filled up with variants of one kind.
#[test]
fn dispersing_children_of_a_varied_node_join_their_kind() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
    (w.cfg.phys.c_copy, w.cfg.crowd) = (0.0, 64);
    let lay = Layout::of(&w.chem);
    // Every gene's spread at the split threshold, (alike step)² π / 8 + step² (`copy::most`).
    let inv = copy::steps(&lay, &vec![1.0; lay.len]);
    let a = w.cfg.phys.alike;
    let spread: Vec<f32> = inv.iter().map(|&x| if x > 0.0 { (a * a * std::f32::consts::FRAC_PI_8 + 1.0) / (x * x) } else { 0.0 }).collect();
    let s = live(&w).into_iter().find(|&s| {
        let i = w.life.bodies[s].cell as usize;
        pcore::grid::DIRS.iter().all(|&d| w.grid.step(i, d).is_some_and(|j| !w.life.at[j].is_empty()))
    }).expect("a node with nodes all around");
    // Members mature from 0.65 of their body, all children leave.
    let mut genes = w.life.bodies[s].genes.clone();
    (genes[COPY_MASS], genes[DISPERSE]) = ((0.65 * w.life.bodies[s].member_body(&w.chem) as f32).log10(), 1.0);
    for t in live(&w) {
        (w.life.bodies[t].genes, w.life.bodies[t].spread) = (genes.clone(), spread.clone());
        w.life.bodies[t].k = Default::default();
        // The nodes around are immature (half the matter: a member's body half, under its copy mass at 0.65), as nodes of
        // children are: a child joins no mature node (`copy::class`).
        if t != s {
            let m = w.chem.matter().expect("matter");
            w.life.bodies[t].store[m] *= 0.5;
        }
    }
    // A node of two members: two children.
    let b = w.life.bodies[s].clone();
    let per: Vec<f64> = b.store.iter().map(|x| x / b.count).collect();
    let (mut joined, mut tries) = (0, 0);
    for _ in 0..200 {
        let mut c = b.clone();
        (c.count, c.store, c.e) = (2.0, per.iter().map(|x| 2.0 * x).collect(), 2.0 * b.e / b.count);
        c.prepare(&w.chem, &w.cfg.phys);
        brood(&w, &mut c, 2.0);
        let n0 = live(&w).len();
        w.day += 1;
        assert!(copy::copy(&mut w, s, &mut c), "no children");
        (joined, tries) = (joined + (live(&w).len() == n0) as u32, tries + 1);
        w.events.clear();
    }
    println!("pairs of children that joined the node of their kind: {joined} of {tries}");
    assert!(joined as f64 >= 0.95 * tries as f64, "{joined} of {tries} joined");
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
        // (no member dies of age here, so the node's count is exact)
        w.cfg.phys.e_life = 1e30;
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
        let (l, g) = (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 0 }), w.life.new_genome(rows));
        let mut b = worldgen::life::make(&mut w, 0, i, l, g).expect("a node");
        b.spread = vec![0.01; b.genes.len()];
        // (their mass stays: no liquid soaked up from the ground, which would add a third to it, and little lost through their
        // pores, which since 16 Oct 2026 are a leaf's and would take a quarter of it in an update: the old gene)
        (b.genes[pcore::body::LIQ], b.genes[pcore::body::GAS]) = (-4.0, 2.187);
        if case == 0 {
            // (copying at 100 kg, so they do not copy instead)
            (b.genes[STRENGTH], b.genes[COPY_MASS], b.count) = (1.0, 2.0, b.count / 40.0);
        }
        let (n0, genes, kg) = (b.count, b.genes.clone(), b.mass(&w.chem) / b.count);
        w.life.add(b, w.day, 1);
        pcore::survey::survey(&mut w);
        w.day += 1;
        pcore::survey::day(&mut w);
        assert_eq!(w.diag.splits[1], 1.0, "no member split off");
        let one: Vec<&Body> = w.life.alive().filter(|b| b.lineage == l && b.count == 1.0).collect();
        let node = w.life.alive().find(|b| b.lineage == l && b.count > 1.0).expect("the node");
        if case == 0 {
            assert!(one.len() == 1 && w.parts.parts[p].id(p) == one[0].hold, "the member holds nothing");
            assert!((node.count + 1.0 - n0).abs() < 1e-9 * n0 && (one[0].mass(&w.chem) / kg - 1.0).abs() < 0.1, "{} of {n0} members, {} kg against {kg}", node.count, one[0].mass(&w.chem));
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

/// Totals over all live bodies: members, each store, heat (`same` compares the stores and heat; a copy makes members).
fn held(w: &pcore::World) -> (f64, Vec<f64>, f64) {
    let (mut n, mut store, mut e) = (0.0, vec![0.0; w.chem.ns()], 0.0);
    for b in w.life.alive() {
        n += b.n();
        e += b.e;
        store.iter_mut().zip(&b.store).for_each(|(s, x)| *s += x);
    }
    (n, store, e)
}

fn same(a: &(f64, Vec<f64>, f64), b: &(f64, Vec<f64>, f64)) {
    assert!((a.2 - b.2).abs() <= 1e-9 * a.2.abs().max(1.0), "heat {} then {}", a.2, b.2);
    for (k, (x, y)) in a.1.iter().zip(&b.1).enumerate() {
        assert!((x - y).abs() <= 1e-9 * x.abs().max(1.0), "store {k}: {x} then {y}");
    }
}

/// `open` draws members of a node as individuals (plan 7.1, "split it back into individuals when
/// the player looks closely"): they are in the node's cell, of count 1, with a wear in [0, 1), and
/// members, stores and heat are kept exactly; the node keeps its place with the members left; the
/// genes of the drawn are the node's plus noise of its spread (their mean near the node's, within
/// four standard errors, and as wide as the spread) and the mean of all members, drawn and left,
/// is the node's, as before.
#[test]
fn opening_a_node_draws_members_and_keeps_everything() {
    let mut w = worldgen::new_world(1, 16, 16);
    let (t0, l0) = (check::totals(&w), w.ledger.clone());
    pcore::step(&mut w);
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::old());
    let s = live(&w).into_iter().find(|&s| w.life.bodies[s].count > 1000.0).expect("a node");
    // A spread of one step in the copy mass (0.2), in the gas gene (0.3) and in a catalyst gene.
    let (lay, genes0) = (Layout::of(&w.chem), w.life.bodies[s].genes.clone());
    let cat = lay.cat + 1;
    let wide = [(COPY_MASS, 0.04f64), (GAS, 0.09), (cat, 0.09)];
    w.life.bodies[s].spread = (0..lay.len).map(|g| wide.iter().find(|x| x.0 == g).map_or(0.0, |x| x.1 as f32)).collect();
    let (before, count0, cell) = (held(&w), w.life.bodies[s].count, w.life.bodies[s].cell);
    let n = 400;
    let drawn = copy::open(&mut w, s, n);
    assert_eq!(drawn.len(), n);
    same(&before, &held(&w));
    assert!((held(&w).0 - before.0).abs() <= 1e-9 * before.0, "members {} then {}", before.0, held(&w).0);
    let node = &w.life.bodies[s];
    assert!((node.count + n as f64 - count0).abs() < 1e-6 && node.count >= 1.0 && node.alive);
    let mut mean = vec![0.0f64; lay.len];
    for &m in &drawn {
        let b = &w.life.bodies[m];
        assert!(b.alive && b.count == 1.0 && b.cell == cell && b.spread.is_empty() && (0.0..1.0).contains(&b.wear));
        mean.iter_mut().zip(&b.genes).for_each(|(x, &y)| *x += y as f64 / n as f64);
    }
    for (g, var) in wide {
        let (se, off) = ((var / n as f64).sqrt(), mean[g] - genes0[g] as f64);
        assert!(off.abs() < 4.0 * se, "gene {g}: drawn mean {} against {} (standard error {se})", mean[g], genes0[g]);
        let sd = (drawn.iter().map(|&m| (w.life.bodies[m].genes[g] as f64 - mean[g]).powi(2)).sum::<f64>() / n as f64).sqrt();
        assert!(sd > 0.8 * var.sqrt() && sd < 1.2 * var.sqrt(), "gene {g}: spread {sd}, wanted {}", var.sqrt());
    }
    // The mean of all members, by count, is what it was.
    for g in 0..lay.len {
        let all = (mean[g] * n as f64 + node.genes[g] as f64 * node.count) / count0;
        assert!((all - genes0[g] as f64).abs() < 1e-4 * (1.0 + genes0[g].abs() as f64), "gene {g}: mean of all {all} against {}", genes0[g]);
    }
    // A node keeps one member: asked for more than it has, it gives all but one.
    let rest = w.life.bodies[s].count.floor() as usize;
    assert_eq!(copy::open(&mut w, s, rest + 5).len(), rest - 1);
    assert!(w.life.bodies[s].count < 2.0);
    assert_eq!(check::broken(&w), None);
    let (de, dq) = check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// With `Config::nodes` off an animal's children are individuals: a child joins no node, even
/// when one of its own kind is in its cell (with nodes on it joins it), and a member whose brood
/// covers many children has that many individuals, not a node of them.
/// Members, stores and heat are kept exactly.
#[test]
fn individuals_mode_children_of_an_animal_are_individuals() {
    for nodes in [true, false] {
        let mut w = worldgen::new_world(1, 32, 32);
        pcore::step(&mut w);
        worldgen::life::seed(&mut w, 0);
        worldgen::life::seed_with(&mut w, 1, &worldgen::life::Seeding::neutral());
        (w.cfg.nodes, w.cfg.phys.c_copy) = (nodes, 0.0);
        let s = live(&w).into_iter().find(|&s| !pcore::body::light_eater(&w.chem, &w.life.bodies[s].genes)).expect("an animal");
        // A node of its own kind beside it, in its cell; its children stay in their cell.
        let mut node = w.life.bodies[s].clone();
        (node.count, node.genes[DISPERSE]) = (5.0, 0.0);
        node.store.iter_mut().for_each(|x| *x *= 5.0);
        // Young (half the matter: founders are adults since 15 Oct 2026), as a node children join is.
        node.store[w.chem.matter().expect("matter")] *= 0.5;
        node.e *= 5.0;
        w.life.bodies[s].genes[DISPERSE] = 0.0;
        let t = w.life.add(node, w.day, 1);
        // One child: its parent is mature (its copy mass is half its body, `Body::built`, which maturity reads, so it stays mature with the brood
        // out of its body whatever its sprayed `CHILD` gene), the child
        // 0.3 of that.
        let (at, before) = (live(&w).len(), held(&w));
        let mut b = std::mem::take(&mut w.life.bodies[s]);
        b.genes[COPY_MASS] = (0.5 * b.built(&w.chem) as f32).log10();
        b.prepare(&w.chem, &w.cfg.phys);
        brood(&w, &mut b, 1.0);
        w.day += 1;
        assert!(copy::copy(&mut w, s, &mut b), "no child");
        w.life.bodies[s] = b;
        let kids: Vec<usize> = live(&w).into_iter().skip(at).collect();
        println!("nodes {nodes}: bodies added {}, node of the kind now {}", kids.len(), w.life.bodies[t].count);
        same(&before, &held(&w));
        if nodes {
            assert!(kids.is_empty() && w.life.bodies[t].count > 5.0, "the child did not join the node of its kind");
            continue;
        }
        assert!(kids.len() == 1 && w.life.bodies[t].count == 5.0 && w.life.bodies[kids[0]].count == 1.0, "the child joined a node");
        // Many children at once: its brood covers 3.5 of them.
        let (at, before) = (live(&w).len(), held(&w));
        let mut b = std::mem::take(&mut w.life.bodies[s]);
        b.genes[COPY_MASS] = (0.3 * b.built(&w.chem) as f32).log10();
        b.prepare(&w.chem, &w.cfg.phys);
        brood(&w, &mut b, 3.5);
        w.day += 1;
        assert!(copy::copy(&mut w, s, &mut b), "no children");
        w.life.bodies[s] = b;
        let kids: Vec<usize> = live(&w).into_iter().skip(at).collect();
        println!("many at once: {} children", kids.len());
        same(&before, &held(&w));
        assert!(kids.len() >= 3, "{} children", kids.len());
        for k in kids {
            let c = &w.life.bodies[k];
            assert!(c.alive && c.count == 1.0 && c.spread.is_empty() && c.wear == 0.0, "child {k}: count {}", c.count);
        }
    }
}

/// With `Config::nodes` off, more than `crowd` alike animals in one cell stay individuals at the
/// month's start (with nodes on they merge, `crowded_individuals_merge_into_a_node`), and over
/// three years of the whole world no animal becomes a node; light-eaters still are nodes. The
/// animals are the hand-set eaters (`Seeding::old`, which find what they eat), so that some live and breed
/// through the window: the check is empty when none do (review 6, finding 10: with the default
/// neutral start 7, 2, 1, 1, 1 and then 0 animals were alive at months 6 to 36). Matter and
/// energy are kept exactly.
#[test]
fn individuals_mode_merges_no_animal() {
    use pcore::event::EventKind;
    let mut w = worldgen::new_world(1, 32, 32);
    w.cfg.nodes = false;
    let (t0, l0) = (check::totals(&w), w.ledger.clone());
    pcore::step(&mut w);
    let old = worldgen::life::Seeding::old();
    worldgen::life::seed_with(&mut w, 0, &old);
    worldgen::life::seed_with(&mut w, 1, &old);
    let animal = |w: &pcore::World, b: &Body| !pcore::body::light_eater(&w.chem, &b.genes);
    assert!(w.life.alive().filter(|b| animal(&w, b)).all(|b| b.count == 1.0), "an animal was placed as a node");
    let a = w.life.alive().find(|b| animal(&w, b)).expect("an animal");
    let (i, lineage, genome) = (a.cell as usize, a.lineage, a.genome);
    let here = |w: &pcore::World| w.life.alive().filter(|b| b.cell as usize == i && animal(w, b)).count();
    while here(&w) < w.cfg.crowd + 4 {
        let b = worldgen::life::make(&mut w, 1, i, lineage, genome).expect("body");
        w.life.add(b, w.day, 1);
    }
    let n = here(&w);
    pcore::survey::survey(&mut w);
    assert_eq!(here(&w), n, "animals merged");
    // Animals alive at the end of each month, and animals born in the window (members).
    let (mut alive, mut born) = (Vec::new(), 0.0);
    w.events.clear();
    for _ in 0..36 {
        pcore::step(&mut w);
        assert!(w.life.alive().filter(|b| animal(&w, b)).all(|b| b.count == 1.0), "an animal became a node");
        born += w.events.iter().filter(|e| matches!(e.kind, EventKind::Born { cat, .. } if cat < pcore::body::LIGHT_EATER)).map(|e| e.value).sum::<f64>();
        w.events.clear();
        alive.push(w.life.alive().filter(|b| animal(&w, b)).count());
    }
    println!("animals alive at the end of each month: {alive:?}; born in the window {born}");
    assert!(alive.last().is_some_and(|&a| a > 0), "no animal was alive at the end: {alive:?}");
    assert!(born > 0.0, "no animal was born in the window: {alive:?}");
    assert!(w.life.alive().any(|b| !animal(&w, b) && b.count > 1.0), "no light-eater node");
    assert_eq!(check::broken(&w), None);
    let (de, dq) = check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}
