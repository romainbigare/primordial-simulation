//! Law 4 (Copy) in populations: two parents mix, and population nodes spread, split, merge and open.

use pcore::life::Handle;
use pcore::body::{Body, Layout, ALLOC, CHILD, COPY_MASS, DISPERSE, GAS, HIDDEN, LEAF, LIQ, MEMORY, MUT, SEEN, SIGHT, STRENGTH};
use pcore::event::EventKind;
use pcore::rng::{Purpose, Rng};
use pcore::units::DAY_S;
use pcore::{body, brain, copy, lineage, nodes, outbox, World};
use worldgen::testkit::{self, brood, life_totals, same_totals, children, founder, out, small_world, start, Books, Brain};

/// Two parents: the gene value of the first body in the genes under test, and the step the catalyst genes move in.
const BASE: f32 = 3.0;
const STEP: f32 = 0.3;

/// A starting eater of a brain that only rests, weak, staying where it is, mature at 10^`copy` kg of body, mutating
/// little (so the noise on a gene is far less than the steps tested), with children 10^-3 of its copy mass.
fn mixer(w: &mut World, i: usize, copy: f32) -> Body {
    let mut b = founder(w, 1, i, &Brain::only(brain::REST));
    (b.genes[DISPERSE], b.genes[STRENGTH], b.genes[COPY_MASS], b.genes[LIQ]) = (0.0, 0.0, copy, -4.0);
    (b.genes[MUT], b.genes[CHILD]) = (-3.0, -3.0);
    b
}

/// Two mature eaters in one cell whose catalyst genes are `BASE` and `BASE` + `gap` steps; the second (the partner) is
/// placed, the first is taken out to copy. The second's born rows are the first's plus 1.
fn pair(w: &mut World, i: usize, gap: f32) -> (Handle, Body, Handle) {
    let lay = Layout::of(&w.chem);
    let (mut a, mut b) = (mixer(w, i, 1.0), mixer(w, i, 1.0));
    for g in lay.cat..lay.min {
        (a.genes[g], b.genes[g]) = (BASE, BASE + gap * STEP);
    }
    let rows = w.life.born(&b).iter().map(|x| x + 1.0).collect();
    w.life.genomes[b.genome as usize].rows = rows;
    let sb = w.add(b, 1);
    let (sa, a) = out(w, a);
    (sa, a, sb)
}

/// Make `n` children of `a` one after another (a day apart, each from a new brood of one), and return them.
fn copies(w: &mut World, a: &mut Body, n: usize) -> Vec<Body> {
    for _ in 0..n {
        w.time += DAY_S;
        brood(w, a, 1.0);
        assert!(w.in_domain(a.cell as usize, |w, d| copy::copy(w, d, a)), "no child");
    }
    let out: Vec<Body> = children(w, a.id).into_iter().map(|k| w.life[k].clone()).collect();
    assert_eq!(out.len(), n);
    out
}

/// The members born in the events: of two parents, and in all.
fn mixed(w: &World) -> (f64, f64) {
    let (mut m, mut all) = (0.0, 0.0);
    for e in &w.events {
        if let EventKind::Born { mixed, .. } = e.kind {
            all += e.value;
            m += if mixed { e.value } else { 0.0 };
        }
    }
    (m, all)
}

/// A child of two parents with different genes has each gene from one or the other, plus a little noise, and about
/// half of them from each over many genes and children; each born brain row too; its lineage is its first parent's.
#[test]
fn each_gene_and_row_of_a_child_is_one_parents() {
    let (mut w, i) = start();
    // (No child starts a lineage of its own by chance, `Physics::new_lineage`: the lineage tested is the parents'.)
    w.cfg.phys.new_lineage = 0.0;
    let lay = Layout::of(&w.chem);
    let (_, mut a, sb) = pair(&mut w, i, 2.0);
    let (rows_a, rows_b) = (w.life.born(&a).to_vec(), w.life.born(&w.life[sb]).to_vec());
    w.events.clear();
    let kids = copies(&mut w, &mut a, 200);
    let (mut from_a, mut from_b, mut other) = (0, 0, 0);
    let (mut row_a, mut row_b, mut row_other) = (0, 0, 0);
    let r = w.life.brains[a.lineage as usize].as_ref().expect("brain").shape.r();
    for c in &kids {
        for g in lay.cat..lay.min {
            let (x, near) = (c.genes[g], |v: f32| (c.genes[g] - v).abs() < 0.05);
            if near(BASE) {
                from_a += 1;
            } else if near(BASE + 2.0 * STEP) {
                from_b += 1;
            } else {
                other += 1;
                println!("gene {g}: {x}");
            }
        }
        assert_eq!(c.lineage, a.lineage, "a child is of its first parent's lineage");
        let rows = w.life.born(c);
        for (k, row) in rows.chunks_exact(r).enumerate() {
            let d = |src: &[f32]| row.iter().zip(&src[k * r..(k + 1) * r]).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
            if d(&rows_a) < 0.05 {
                row_a += 1;
            } else if d(&rows_b) < 0.05 {
                row_b += 1;
            } else {
                row_other += 1;
            }
        }
    }
    let share = from_b as f64 / (from_a + from_b) as f64;
    println!("genes: {from_a} from the first parent, {from_b} from the second, {other} from neither: share {share:.3}; rows {row_a} / {row_b} / {row_other}");
    assert_eq!(other, 0, "a gene that is neither parent's");
    assert!((0.45..0.55).contains(&share), "the second parent gave {share} of the genes");
    assert_eq!(row_other, 0, "a row that is neither parent's");
    let rshare = row_b as f64 / (row_a + row_b) as f64;
    assert!((0.4..0.6).contains(&rshare), "the second parent gave {rshare} of the rows");
    // Every birth is marked as of two parents.
    assert_eq!(mixed(&w), (200.0, 200.0));
}

/// With no partner, or none mature, a child is a copy of one parent: every gene and row is its parent's plus noise,
/// and the birth is not marked.
#[test]
fn with_no_partner_a_child_has_one_parent() {
    let (mut w, i) = start();
    let lay = Layout::of(&w.chem);
    // Alone in its cell.
    let mut a = mixer(&mut w, i, 1.0);
    (lay.cat..lay.min).for_each(|g| a.genes[g] = BASE);
    let (_, mut a) = out(&mut w, a);
    w.events.clear();
    let kids = copies(&mut w, &mut a, 30);
    for c in &kids {
        assert!((lay.cat..lay.min).all(|g| (c.genes[g] - BASE).abs() < 0.05), "a gene that is not its parent's");
        // (A child's rows change a little now and then: the generation's mutation.)
        let drift = w.life.born(c).iter().zip(w.life.born(&a)).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert!(drift < 0.01, "rows {drift} from its parent's");
    }
    assert_eq!(mixed(&w), (0.0, 30.0));
    // A partner that is not mature (its body under its copy mass) is not one.
    let (mut w, i) = start();
    let (_, mut a, sb) = pair(&mut w, i, 2.0);
    w.life[sb].genes[COPY_MASS] = 3.0;
    w.life[sb].prepare(&w.chem, &w.cfg);
    assert!(!w.life[sb].adult(&w.chem));
    w.events.clear();
    let kids = copies(&mut w, &mut a, 30);
    assert!(kids.iter().all(|c| (lay.cat..lay.min).all(|g| (c.genes[g] - BASE).abs() < 0.05)));
    assert_eq!(mixed(&w), (0.0, 30.0));
    // The only body in the cell that is mature is the parent itself: it is never its own partner.
    let (mut w, i) = start();
    let a = mixer(&mut w, i, 1.0);
    let (_, mut a) = out(&mut w, a);
    w.events.clear();
    copies(&mut w, &mut a, 5);
    assert_eq!(mixed(&w), (0.0, 5.0));
}

/// Bodies that are not alike never mix: a partner more than `phys.alike` steps away in a gene is not one, however
/// near in the others; at the limit it is; and of two it takes the more alike.
#[test]
fn unlike_bodies_never_mix() {
    let (mut w, i) = start();
    let lay = Layout::of(&w.chem);
    let limit = w.cfg.phys.alike;
    // Five steps apart in every catalyst gene (past the limit of 4): asexual.
    let (_, mut a, _) = pair(&mut w, i, limit + 1.0);
    w.events.clear();
    let kids = copies(&mut w, &mut a, 30);
    assert!(kids.iter().all(|c| (lay.cat..lay.min).all(|g| (c.genes[g] - BASE).abs() < 0.05)), "unlike bodies mixed");
    assert_eq!(mixed(&w).0, 0.0);
    // Alike in all but one gene, which is past the limit: still unlike.
    let (mut w, i) = start();
    let (_, mut a, sb) = pair(&mut w, i, 1.0);
    w.life[sb].genes[lay.cat] = BASE + (limit + 0.5) * STEP;
    w.events.clear();
    copies(&mut w, &mut a, 10);
    assert_eq!(mixed(&w).0, 0.0, "one gene past the limit is unlike");
    // Brain sizes count by whole units: a partner 6 units bigger in brain (past 4) is unlike, 3 units is alike.
    for (units, mixes) in [(6.0, false), (3.0, true)] {
        let (mut w, i) = start();
        let (_, mut a, sb) = pair(&mut w, i, 1.0);
        w.life[sb].genes[body::HIDDEN] += units;
        w.events.clear();
        copies(&mut w, &mut a, 10);
        assert_eq!(mixed(&w).0 > 0.0, mixes, "{units} units of brain apart");
    }
    // Two partners, one near and one further but alike: all the children take the near one's genes.
    let (mut w, i) = start();
    let (_, mut a, _) = pair(&mut w, i, 3.0);
    let mut near = mixer(&mut w, i, 1.0);
    for g in lay.cat..lay.min {
        near.genes[g] = BASE + 1.0 * STEP;
    }
    w.add(near, 1);
    let kids = copies(&mut w, &mut a, 60);
    let (mut a_n, mut far) = (0, 0);
    for c in &kids {
        for g in lay.cat..lay.min {
            if (c.genes[g] - BASE).abs() < 0.05 {
                a_n += 1;
            } else if (c.genes[g] - BASE - STEP).abs() < 0.05 {
                far += 1;
            } else {
                panic!("gene {g} of a child is {}: neither its parent's nor the nearer partner's", c.genes[g]);
            }
        }
    }
    println!("from the parent {a_n}, from the nearer partner {far}");
    assert!(a_n > 0 && far > 0);
}

/// The partner loses nothing and pays nothing: its stores, heat, genes, brood and count are as they were, no matter
/// or energy moves to it or from it, and the world keeps its totals.
#[test]
fn the_partner_loses_nothing() {
    let (mut w, i) = start();
    let (s, mut a, sb) = pair(&mut w, i, 2.0);
    let before = w.life[sb].clone();
    let books = Books::now(&w);
    w.events.clear();
    copies(&mut w, &mut a, 20);
    assert_eq!(mixed(&w).0, 20.0);
    let after = &w.life[sb];
    assert_eq!((&after.store, after.e, &after.genes, after.brood, after.count), (&before.store, before.e, &before.genes, before.brood, before.count));
    assert_eq!(w.life.born(after), w.life.born(&before));
    w.life[s] = a;
    books.assert_kept(&w, 1e-9);
}

/// Slots of the live bodies.
fn live(w: &pcore::World) -> Vec<Handle> {
    w.life.living().map(|x| x.0).collect()
}

/// A mature node's children start a node of their own (they are immature, `copy::young`) that carries the variance
/// left among them; when they join their parents' node (`nodes::merge`) its spread is widened by it; a node whose
/// members differ so much that its halves would not be alike splits in two along that gene, keeping its members,
/// stores, heat and pooled spread; the half placed is seen as what it holds.
#[test]
fn a_node_spreads_with_its_children_and_splits_when_too_varied() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    // (A node may make children at every update: `lump_days` of one batch.)
    w.cfg.lump_days = w.cfg.batch_days;
    let books = Books::now(&w);
    let s = live(&w).into_iter().find(|&s| w.life[s].count > 10.0).expect("a node");
    let mut b = std::mem::take(&mut w.life[s]);
    // Members are mature (a low copy mass), each has a child and all children stay in the cell.
    (b.genes[COPY_MASS], b.genes[DISPERSE]) = (-0.4, 0.0);
    b.prepare(&w.chem, &w.cfg);
    let n0 = b.count;
    brood(&w, &mut b, n0);
    let before = live(&w);
    assert!(b.spread.is_empty() && w.in_domain(b.cell as usize, |w, d| copy::copy(w, d, &mut b)), "no children");
    assert_eq!(b.count, n0, "the children joined their mature parents");
    w.life[s] = b;
    let kids: Vec<Handle> = live(&w).into_iter().filter(|x| !before.contains(x) && *x != s).collect();
    assert_eq!(kids.len(), 1, "the children are one node of their own");
    let c = &w.life[kids[0]];
    assert!((c.count - n0).abs() < 1e-6 * n0 && !c.adult(&w.chem), "{} young members", c.count);
    assert!(c.spread.iter().all(|&v| v >= 0.0) && c.spread.iter().any(|&v| v > 0.0) && c.row_spread > 0.0, "children keep no spread");
    // Grown, they join the node (as the month's merge does): its spread is widened.
    w.in_domain(w.life[s].cell as usize, |w, d| nodes::merge(w, d, kids[0], s));
    let mut b = std::mem::take(&mut w.life[s]);
    assert!(b.count > 1.5 * n0 && b.spread.iter().any(|&v| v > 0.0) && b.row_spread > 0.0, "the merge widened nothing");

    // Copy mass spread over 5 mutation steps (0.2 each): the halves are 2 sqrt(2/pi) 5 = 8 apart.
    b.spread[COPY_MASS] = 1.0;
    let (count, store, e, m) = (b.count, b.store.clone(), b.e, b.genes[COPY_MASS]);
    let before = live(&w);
    w.in_domain(b.cell as usize, |w, d| nodes::split(w, d, &mut b));
    w.life[s] = b;
    let new: Vec<Handle> = live(&w).into_iter().filter(|x| !before.contains(x) && *x != s).collect();
    assert_eq!(new.len(), 1, "not split in two");
    seen_as_it_is(&w, new[0]);
    let (a, c) = (&w.life[s], &w.life[new[0]]);
    assert_eq!((a.count + c.count, a.lineage, a.cell), (count, c.lineage, c.cell));
    for (k, &x) in store.iter().enumerate() {
        assert!((a.store[k] + c.store[k] - x).abs() <= 1e-12 * x, "store {k}");
    }
    assert!((a.e + c.e - e).abs() <= 1e-12 * e);
    let d = (2.0 / std::f32::consts::PI).sqrt();
    // (Which half keeps the node's place is drawn.)
    let (x, y) = (a.genes[COPY_MASS] - m, c.genes[COPY_MASS] - m);
    assert!((x.abs() - d).abs() < 1e-5 && (x + y).abs() < 1e-5, "halves at {x} and {y}, not ±{d}");
    assert!((a.spread[COPY_MASS] + d * d - 1.0).abs() < 1e-5 && a.spread == c.spread, "pooled spread not kept");
    books.assert_kept(&w, 1e-9);
}

/// Light-eater nodes with a fast mutation (their spreads reach the split threshold) and members of 100 kg that are
/// mature from 28 kg, with children of 14 kg (no copy cost; the copy-mass and child genes are set again at each update
/// so that every node is mature, and its children join a young node, whatever its other genes: no selection by
/// when a body may reproduce) copy, split, move to the lineage their brain sizes ask for, send children to join or
/// start nodes in neighbour cells, and merge each month, for `updates` updates of 10 days (`wide`: from spreads held at
/// the split threshold). Every member has one child per update, and then a share `live_on(b)` of each body's members
/// lives on (0.5: as many die as were born). Returns the world and the mean genes by members at the start and at the
/// end.
fn churn(crowd: usize, updates: usize, wide: bool, live_on: impl Fn(&Body) -> f64) -> (pcore::World, Vec<f64>, Vec<f64>) {
    let mut w = worldgen::new_world(1, 4, 4);
    pcore::step(&mut w);
    (w.cfg.crowd, w.cfg.phys.c_copy) = (crowd, 0.0);
    // One parent: the node math alone is measured. With two a body's partner is the most alike of its cell, which need
    // not choose it in return, so children are pulled toward the nodes that are many bodies' nearest, and the mean over
    // all members wanders by more than the bound below.
    w.cfg.two_parents = false;
    worldgen::life::seed(&mut w, 0);
    let len = Layout::of(&w.chem).len;
    let b0 = &w.life[live(&w)[0]];
    let (per, e1) = (b0.store.iter().map(|a| 100.0 * a / b0.count).collect::<Vec<f64>>(), 100.0 * b0.e / b0.count);
    for s in live(&w) {
        (w.life[s].genes[COPY_MASS], w.life[s].genes[MUT], w.life[s].genes[CHILD]) = (1.45, -0.8, -std::f32::consts::LOG10_2);
        if wide {
            // (The first children that stay bring every spread down to the split threshold.) A brain of 12 hidden units
            // for selection to shrink (light-eaters start with none).
            w.life[s].spread = vec![1e9; len];
            w.life[s].genes[HIDDEN] = 12.0;
        }
        w.life[s].prepare(&w.chem, &w.cfg);
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
        w.time += 10 * DAY_S;
        if w.day().is_multiple_of(w.cfg.days_per_month as u64) {
            pcore::survey::survey(&mut w, false);
        }
        // In a random order, so no body is first by its slot.
        let (mut order, mut r) = (live(&w), Rng::new(1, w.day(), 0, Purpose::Choose));
        for k in (1..order.len()).rev() {
            order.swap(k, r.below(k + 1));
        }
        for s in order {
            let mut b = std::mem::take(&mut w.life[s]);
            (b.store, b.e) = (per.iter().map(|a| a * b.count).collect(), e1 * b.count);
            // Mature with a body of twice its copy mass after the brood, children half the copy mass.
            let (body, n) = (b.built(&w.chem) / b.n(), b.count);
            (b.genes[COPY_MASS], b.genes[CHILD], b.brood) = ((0.4 * body).log10() as f32, -std::f32::consts::LOG10_2, 0.0);
            b.prepare(&w.chem, &w.cfg);
            brood(&w, &mut b, n);
            let (before, id) = (live(&w), b.id);
            // (One serial step, which puts it back in its slot, so the ids it makes reach it.)
            w.in_domain(b.cell as usize, |w, d| {
                copy::copy(w, d, &mut b);
                lineage::resize(w, d, &mut b);
                nodes::split(w, d, &mut b);
                d[s] = b;
            });
            // (Children who start in another cell wait in the outbox until a batch ends: they land here, as there.)
            outbox::deliver(&mut w);
            // The children are young nodes of their own (immature, `copy::young`); here they grow up at once, and
            // would mature into their parent's node: they join it now, so the update's node math is the same.
            let kids: Vec<Handle> = live(&w).into_iter().filter(|k| *k != s && !before.contains(k) && w.life[*k].parent == id).collect();
            for k in kids {
                w.in_domain(w.life[s].cell as usize, |w, d| nodes::merge(w, d, k, s));
            }
        }
        for s in live(&w) {
            let f = live_on(&w.life[s]);
            w.life[s].count *= f;
        }
        w.events.clear();
    }
    let m1 = mean(&w);
    (w, m0, m1)
}

/// Without selection the node math moves no gene, the brain's sizes included: every body's members live on alike, so
/// a gene's mean over all members may move only by the noise of the children that leave. Over 1,000 updates every
/// gene's mean stays within a mutation step of its start, in cells over `crowd` and not.
#[test]
#[ignore = "slow: 2 x 1,000 updates of node churn"]
fn genes_do_not_drift_without_selection() {
    for crowd in [1, 64] {
        let w0 = worldgen::new_world(1, 4, 4);
        let lay = Layout::of(&w0.chem);
        // Gene steps (from `steps` of all ones: 1 / step; the brain's sizes, which `steps` leaves out of likeness, by
        // their own steps).
        let mut inv = body::steps(&lay, &vec![1.0; lay.len]);
        for (g, step) in [(HIDDEN, 0.3), (MEMORY, 0.15), (SEEN, 0.1), (SIGHT, 0.07)] {
            inv[g] = 1.0 / step;
        }
        // (The copy mass and the child's size are set again at each update, see `churn`: not measured.)
        (inv[COPY_MASS], inv[CHILD]) = (0.0, 0.0);
        let (w, m0, m1) = churn(crowd, 1000, false, |_| 0.5);
        let worst = (0..lay.len).filter(|&g| inv[g] > 0.0).map(|g| ((m1[g] - m0[g]) * inv[g] as f64).abs()).fold(0.0, f64::max);
        let which = (0..lay.len).filter(|&g| inv[g] > 0.0).max_by(|&a, &b| ((m1[a] - m0[a]) * inv[a] as f64).abs().total_cmp(&((m1[b] - m0[b]) * inv[b] as f64).abs())).unwrap();
        println!("crowd {crowd}: bodies {}, splits {}, largest move {worst:.3} steps (gene {which}: {} -> {})", live(&w).len(), w.counts.splits, m0[which], m1[which]);
        assert!(crowd < 4 || w.counts.splits > 10.0, "too few splits");
        assert!(worst < 1.0, "a gene moved {worst} steps without selection");
    }
}

/// With selection a node's brain size moves: from spreads held at the split threshold, members live on a little less
/// the more hidden units their genes ask for (5% per unit and update, as if they paid for a brain they do not use),
/// and nothing else differs. Within 1,000 updates the mean of the hidden-units gene falls by more than a unit and the
/// nodes move to a lineage with fewer hidden units. It is slow, as a split exposes one gene at a time (a random pick
/// among the widest).
#[test]
#[ignore = "slow: 1,000 updates of node churn"]
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

/// Children that leave a varied node join the node of their own kind in the next cell: pairs of children drawn from a
/// node held at the split threshold in every gene, sent to a cell whose node has the same genes and spread, join it
/// nearly always, as they lie within its spread (a pair's mean can differ from the node's by more than `alike` steps).
#[test]
fn dispersing_children_of_a_varied_node_join_their_kind() {
    let mut w = small_world();
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::alike());
    (w.cfg.phys.c_copy, w.cfg.crowd) = (0.0, 64);
    // (A node may make children at every update: `lump_days` of one batch.)
    w.cfg.lump_days = w.cfg.batch_days;
    let lay = Layout::of(&w.chem);
    // Every gene's spread at the split threshold, (alike step)² π / 8 + step² (`body::most`).
    let inv = body::steps(&lay, &vec![1.0; lay.len]);
    let a = w.cfg.phys.alike;
    let spread: Vec<f32> = inv.iter().map(|&x| if x > 0.0 { (a * a * std::f32::consts::FRAC_PI_8 + 1.0) / (x * x) } else { 0.0 }).collect();
    let s = live(&w).into_iter().find(|&s| {
        let i = w.life[s].cell as usize;
        let near = w.mesh.cells().around(i).1;
        near.len() == 8 && near.iter().all(|&j| w.life.in_cell(j as usize) > 0)
    }).expect("a node with nodes all around");
    // Members mature from 0.65 of their body, all children leave.
    let mut genes = w.life[s].genes.clone();
    (genes[COPY_MASS], genes[DISPERSE]) = ((0.65 * w.life[s].member_body(&w.chem) as f32).log10(), 1.0);
    for t in live(&w) {
        (w.life[t].genes, w.life[t].spread) = (genes.clone(), spread.clone());
        w.life[t].prepare(&w.chem, &w.cfg);
        // The nodes around are immature (half the matter: a member's body half, under its copy mass at 0.65), as nodes
        // of children are: a child joins no mature node (`copy::kin`).
        if t != s {
            let m = w.chem.matter().expect("matter");
            w.life[t].store[m] *= 0.5;
        }
    }
    // A node of two members: two children.
    let b = w.life[s].clone();
    let per: Vec<f64> = b.store.iter().map(|x| x / b.count).collect();
    let (mut joined, mut tries) = (0, 0);
    for _ in 0..200 {
        let mut c = b.clone();
        (c.count, c.store, c.e) = (2.0, per.iter().map(|x| 2.0 * x).collect(), 2.0 * b.e / b.count);
        c.prepare(&w.chem, &w.cfg);
        brood(&w, &mut c, 2.0);
        let n0 = live(&w).len();
        w.time += DAY_S;
        assert!(w.in_domain(c.cell as usize, |w, d| copy::copy(w, d, &mut c)), "no children");
        // (They land in the next cell, as at a batch's end.)
        outbox::deliver(&mut w);
        (joined, tries) = (joined + (live(&w).len() == n0) as u32, tries + 1);
        w.events.clear();
    }
    println!("pairs of children that joined the node of their kind: {joined} of {tries}");
    assert!(joined as f64 >= 0.95 * tries as f64, "{joined} of {tries} joined");
}

/// A node whose mean brain-size gene moved a whole unit from its lineage's size moves to a new lineage with that size
/// at its update. A light-eater node (no brain: no rows) grows one hidden unit: new born rows, and its rows those. Grown
/// once more, its brain is carried over and keeps what it learned (the bias of a row is its last entry). Matter and
/// energy as they were.
#[test]
fn a_node_whose_size_genes_moved_changes_lineage() {
    let mut w = small_world();
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::alike());
    let books = Books::now(&w);
    let s = live(&w).into_iter().find(|&s| w.life[s].count > 10.0).expect("a node");
    // (Its genes grow by `by` and it updates its lineage: a serial step, in its slot so the new ids reach it.)
    let grow = |w: &mut World, by: f32| {
        w.life[s].genes[HIDDEN] += by;
        w.in_domain(w.life[s].cell as usize, |w, d| {
            let mut b = std::mem::take(&mut d[s]);
            lineage::resize(w, d, &mut b);
            d[s] = b;
        });
    };
    let lin0 = w.life[s].lineage;
    assert!(w.life.born(&w.life[s]).is_empty() && w.life[s].rows.is_empty(), "a light-eater has a brain");
    grow(&mut w, 0.9);
    assert_eq!(w.life[s].lineage, lin0, "moved for less than a unit");
    grow(&mut w, 0.3);
    let shape = |w: &World, b: &Body| w.life.brains[b.lineage as usize].as_ref().expect("its lineage").shape;
    let b = &w.life[s];
    let (lin1, s1) = (b.lineage, shape(&w, b));
    assert!(lin1 != lin0 && s1.h == 1, "no new lineage of one hidden unit");
    assert_eq!(w.life.members[lin1 as usize], 1);
    assert_eq!(w.life.born(b).len(), s1.rows() * s1.r());
    assert_eq!(b.rows.as_slice(), w.life.born(b));
    w.life[s].rows[s1.h] += 0.5;
    let learned = w.life[s].rows[s1.h] - w.life.born(&w.life[s])[s1.h];
    grow(&mut w, 1.0);
    let b = &w.life[s];
    let h = shape(&w, b).h;
    assert!(b.lineage != lin1 && h == 2, "no new lineage of two hidden units");
    assert!((b.rows[h] - w.life.born(b)[h] - learned).abs() < 1e-5, "what it learned is lost");
    for _ in 0..2 {
        pcore::step(&mut w);
    }
    books.assert_kept(&w, 1e-9);
}

/// Body `s` is seen as what it holds now: its visible traits are those its stores, genes and members give.
fn seen_as_it_is(w: &World, s: Handle) {
    let b = &w.life[s];
    assert_eq!(b.vis, b.visible(&w.chem, &w.cfg), "body {s:?} is seen as it was");
}

/// `open` draws members of a node as individuals: they are in the node's cell, of count 1, and
/// members, stores and heat are kept exactly; the node keeps its place with the members left; each is seen as what it
/// holds (`Body::vis`); the genes of the drawn
/// are the node's plus noise of its spread (their mean near the node's, within four standard errors, and as wide as the
/// spread) and the mean of all members, drawn and left, is the node's; and so are their born rows, each member's its
/// own (a light-eater node of a test lineage with a brain: its own kind has no born rows).
#[test]
fn opening_a_node_draws_members_and_keeps_everything() {
    let mut w = worldgen::new_world(1, 16, 16);
    let books = Books::now(&w);
    pcore::step(&mut w);
    let i = testkit::dry_land(&w);
    let b = founder(&mut w, 0, i, &Brain::neutral());
    let s = w.add(b, 1);
    assert!(w.life[s].count > 1000.0, "a node");
    // A spread of one step in the copy mass (0.2), in the gas gene (0.3) and in a catalyst gene; and one in the
    // light-capture gene, so each is seen with its own leaf area (near its wall: its noise shrinks, so it is not
    // measured).
    let (lay, genes0) = (Layout::of(&w.chem), w.life[s].genes.clone());
    let cat = lay.cat + 1;
    let wide = [(COPY_MASS, 0.04f64), (GAS, 0.09), (cat, 0.09)];
    w.life[s].spread = (0..lay.len).map(|g| wide.iter().find(|x| x.0 == g).map_or(0.0, |x| x.1 as f32)).collect();
    w.life[s].spread[LEAF] = 0.09;
    // And one of 0.1 in the born rows.
    w.life[s].row_spread = 0.01;
    let rows0 = w.life.born(&w.life[s]).to_vec();
    let (before, count0, cell) = (life_totals(&w), w.life[s].count, w.life[s].cell);
    let n = 400;
    let drawn = w.in_domain(w.life[s].cell as usize, |w, d| nodes::open(w, d, s, n));
    assert_eq!(drawn.len(), n);
    same_totals(&before, &life_totals(&w));
    assert!((life_totals(&w).0 - before.0).abs() <= 1e-9 * before.0, "members {} then {}", before.0, life_totals(&w).0);
    drawn.iter().chain([&s]).for_each(|&m| seen_as_it_is(&w, m));
    let node = &w.life[s];
    assert!((node.count + n as f64 - count0).abs() < 1e-6 && node.count >= 1.0 && node.alive);
    let mut mean = vec![0.0f64; lay.len];
    for &m in &drawn {
        let b = &w.life[m];
        assert!(b.alive && b.count == 1.0 && b.cell == cell && b.spread.is_empty());
        mean.iter_mut().zip(&b.genes).for_each(|(x, &y)| *x += y as f64 / n as f64);
    }
    for (g, var) in wide {
        let (se, off) = ((var / n as f64).sqrt(), mean[g] - genes0[g] as f64);
        assert!(off.abs() < 4.0 * se, "gene {g}: drawn mean {} against {} (standard error {se})", mean[g], genes0[g]);
        let sd = (drawn.iter().map(|&m| (w.life[m].genes[g] as f64 - mean[g]).powi(2)).sum::<f64>() / n as f64).sqrt();
        assert!(sd > 0.8 * var.sqrt() && sd < 1.2 * var.sqrt(), "gene {g}: spread {sd}, wanted {}", var.sqrt());
    }
    // The mean of all members, by count, is what it was.
    for g in 0..lay.len {
        let all = (mean[g] * n as f64 + node.genes[g] as f64 * node.count) / count0;
        assert!((all - genes0[g] as f64).abs() < 1e-4 * (1.0 + genes0[g].abs() as f64), "gene {g}: mean of all {all} against {}", genes0[g]);
    }
    let (mut rows, mut var) = (w.life.born(node).iter().map(|&x| x as f64 * node.count / count0).collect::<Vec<f64>>(), 0.0);
    for b in drawn.iter().map(|&m| &w.life[m]) {
        for ((r, &x), &x0) in rows.iter_mut().zip(w.life.born(b)).zip(&rows0) {
            (*r, var) = (*r + x as f64 / count0, var + (x as f64 - x0 as f64).powi(2) / (n * rows0.len()) as f64);
        }
    }
    let off = rows.iter().zip(&rows0).map(|(x, &y)| (x - y as f64).abs()).fold(0.0, f64::max);
    assert!(off < 1e-4 && var.sqrt() > 0.08 && var.sqrt() < 0.12, "born rows: mean of all off by {off}, drawn as wide as {}", var.sqrt());
    // A node keeps one member: asked for more than it has, it gives all but one. (Its rows made alike first, so that its
    // many members do not each get rows of their own.)
    w.life[s].row_spread = 0.0;
    let rest = w.life[s].count.floor() as usize;
    assert_eq!(w.in_domain(w.life[s].cell as usize, |w, d| nodes::open(w, d, s, rest + 5)).len(), rest - 1);
    assert!(w.life[s].count < 2.0);
    books.assert_kept(&w, 1e-9);
}

/// Members who join a node are kept (`nodes::merge`): a body of 1.4 members adds 1.4 to its count, and the genes become
/// the means by members.
#[test]
fn joining_a_node_keeps_members_and_weighs_genes_by_them() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    let t = live(&w).into_iter().find(|&s| w.life[s].count > 1.0).expect("a node");
    w.life[t].count = 3.0;
    let (o, before) = (w.life[t].clone(), life_totals(&w).0);
    let mut c = o.clone();
    (c.count, c.genes) = (1.4, o.genes.iter().map(|g| g + 0.1).collect());
    let s = w.add(c, 1);
    w.in_domain(w.life[t].cell as usize, |w, d| nodes::merge(w, d, s, t));
    let (after, node) = (life_totals(&w).0, &w.life[t]);
    assert!((after - (before + 1.4)).abs() < 1e-12 * after && (node.count - 4.4).abs() < 1e-12, "members {before} + 1.4 then {after}");
    for (g, (x, y)) in o.genes.iter().zip(&node.genes).enumerate() {
        let want = x + 0.1 * 1.4 / 4.4;
        assert!((y - want).abs() < 1e-5 * (1.0 + x.abs()), "gene {g}: {x} then {y}, wanted {want}");
    }
}

/// A node that splits and joins again many times keeps its members, stores and heat to rounding, and comes back to
/// its genes and spread (`nodes::Lot`, `nodes::add`): 10,000 splits along a gene drawn at random, each half joined
/// back at once (`nodes::merge`), from a spread drawn at random between the split threshold and `body::most`.
#[test]
fn splitting_and_joining_give_the_node_back() {
    let mut w = small_world();
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::alike());
    w.cfg.crowd = 64;
    let books = Books::now(&w);
    let s = live(&w).into_iter().find(|&s| w.life[s].count > 10.0).expect("a node");
    let lay = Layout::of(&w.chem);
    let mut r = Rng::new(1, 0, 0, Purpose::Split);
    let (n0, before) = (w.life[s].count, life_totals(&w));
    for _ in 0..10_000 {
        let g = lay.cat + r.below(lay.min - lay.cat);
        let mut b = std::mem::take(&mut w.life[s]);
        // (A catalyst gene well clear of its walls, its spread past the split threshold and at most the most a node keeps,
        // `body::most`, which a join does not pass.)
        b.genes[g] = 6.0;
        b.spread = vec![0.0; lay.len];
        b.spread[g] = (1.0 - 0.1 * r.f64() as f32) * 0.3f32.powi(2) * (w.cfg.phys.alike.powi(2) * std::f32::consts::FRAC_PI_8 + 1.0);
        let (genes, spread) = (b.genes.clone(), b.spread.clone());
        let others = live(&w);
        w.in_domain(b.cell as usize, |w, d| nodes::split(w, d, &mut b));
        w.life[s] = b;
        let half = live(&w).into_iter().find(|x| *x != s && !others.contains(x)).expect("split in two");
        w.in_domain(w.life[s].cell as usize, |w, d| nodes::merge(w, d, half, s));
        let b = &w.life[s];
        assert!((b.genes[g] - genes[g]).abs() < 1e-5 && (b.spread[g] - spread[g]).abs() < 1e-5 * spread[g], "gene {g}: {} and {} back as {} and {}", genes[g], spread[g], b.genes[g], b.spread[g]);
    }
    same_totals(&before, &life_totals(&w));
    assert!((w.life[s].count - n0).abs() < 1e-9 * n0, "members {n0} then {}", w.life[s].count);
    books.assert_kept(&w, 1e-9);
}

/// With `Config::nodes` off an animal's children are individuals: a child joins no node, even when one of its own kind
/// is in its cell (with nodes on it joins it), and a member whose brood covers many children has that many
/// individuals, not a node of them. Members, stores and heat are kept.
#[test]
fn individuals_mode_children_of_an_animal_are_individuals() {
    for nodes in [true, false] {
        let mut w = worldgen::new_world(1, 16, 16);
        pcore::step(&mut w);
        worldgen::life::seed(&mut w, 0);
        worldgen::life::seed_with(&mut w, 1, &worldgen::life::Seeding::neutral());
        (w.cfg.nodes, w.cfg.phys.c_copy) = (nodes, 0.0);
        let s = live(&w).into_iter().find(|&s| !pcore::body::light_eater(&w.chem, &w.life[s].genes)).expect("an animal");
        // A node of its own kind beside it, in its cell; its children stay in their cell.
        let mut node = w.life[s].clone();
        (node.count, node.genes[DISPERSE]) = (5.0, 0.0);
        node.store.iter_mut().for_each(|x| *x *= 5.0);
        // Young (half the matter of an adult founder), as a node children join is.
        node.store[w.chem.matter().expect("matter")] *= 0.5;
        node.e *= 5.0;
        w.life[s].genes[DISPERSE] = 0.0;
        let t = w.add(node, 1);
        // One child: its parent is mature (its copy mass is half its body, `Body::built`, which maturity reads, so it stays
        // mature with the brood out of its body whatever its sprayed `CHILD` gene).
        let (was, before) = (live(&w), life_totals(&w));
        let mut b = std::mem::take(&mut w.life[s]);
        b.genes[COPY_MASS] = (0.5 * b.built(&w.chem) as f32).log10();
        b.prepare(&w.chem, &w.cfg);
        brood(&w, &mut b, 1.0);
        w.time += DAY_S;
        assert!(w.in_domain(b.cell as usize, |w, d| copy::copy(w, d, &mut b)), "no child");
        w.life[s] = b;
        let kids: Vec<Handle> = live(&w).into_iter().filter(|k| !was.contains(k)).collect();
        println!("nodes {nodes}: bodies added {}, node of the kind now {}", kids.len(), w.life[t].count);
        same_totals(&before, &life_totals(&w));
        if nodes {
            assert!(kids.is_empty() && w.life[t].count > 5.0, "the child did not join the node of its kind");
            continue;
        }
        assert!(kids.len() == 1 && w.life[t].count == 5.0 && w.life[kids[0]].count == 1.0, "the child joined a node");
        // Many children at once: its brood covers 3.5 of them.
        let (was, before) = (live(&w), life_totals(&w));
        let mut b = std::mem::take(&mut w.life[s]);
        b.genes[COPY_MASS] = (0.3 * b.built(&w.chem) as f32).log10();
        b.prepare(&w.chem, &w.cfg);
        brood(&w, &mut b, 3.5);
        w.time += DAY_S;
        assert!(w.in_domain(b.cell as usize, |w, d| copy::copy(w, d, &mut b)), "no children");
        w.life[s] = b;
        let kids: Vec<Handle> = live(&w).into_iter().filter(|k| !was.contains(k)).collect();
        println!("many at once: {} children", kids.len());
        same_totals(&before, &life_totals(&w));
        assert!(kids.len() >= 3, "{} children", kids.len());
        for k in kids {
            let c = &w.life[k];
            assert!(c.alive && c.count == 1.0 && c.spread.is_empty(), "child {k:?}: count {}", c.count);
        }
    }
}

/// With `Config::nodes` off, more than `crowd` alike animals in one cell stay individuals at the month's start (with
/// nodes on they merge, `crowded_individuals_merge_into_a_node`), and over three years of the whole world no animal
/// becomes a node; light-eaters still are nodes. The animals are hand-set eaters (as `ew run` places them, which find
/// what they eat), so that some live and breed through the window: the check would be empty with none (neutral animal
/// founders mostly die in their first year, and in one world all may die before they breed), so it runs three worlds
/// (seeds 1 to 3) and asks for births in them together. Matter and energy are kept.
#[test]
#[ignore = "slow: three years of three 32 x 32 worlds"]
fn individuals_mode_merges_no_animal() {
    let (mut born, mut alive_at_end) = (0.0, 0);
    for seed in 1..=3 {
        let (b, a) = individuals_window(seed);
        born += b;
        alive_at_end += a;
    }
    assert!(born > 0.0 && alive_at_end > 0, "no animal was born ({born}) or alive at the end ({alive_at_end}) in three worlds");
}

/// One world of `individuals_mode_merges_no_animal`: its checks, and the animals born in its window (members) and alive
/// at its end.
fn individuals_window(seed: u64) -> (f64, usize) {
    let mut w = worldgen::new_world(seed, 32, 32);
    w.cfg.nodes = false;
    let books = Books::now(&w);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    let animal = |w: &pcore::World, b: &Body| !pcore::body::light_eater(&w.chem, &b.genes);
    assert!(w.life.alive().filter(|b| animal(&w, b)).all(|b| b.count == 1.0), "an animal was placed as a node");
    let a = w.life.alive().find(|b| animal(&w, b)).expect("an animal");
    let (i, lineage, genome) = (a.cell as usize, a.lineage, a.genome);
    let here = |w: &pcore::World| w.life.alive().filter(|b| b.cell as usize == i && animal(w, b)).count();
    while here(&w) < w.cfg.crowd + 4 {
        let b = worldgen::life::make(&mut w, 1, i, lineage, genome).expect("body");
        w.add(b, 1);
    }
    let n = here(&w);
    pcore::survey::survey(&mut w, false);
    assert_eq!(here(&w), n, "animals merged");
    // Animals alive at the end of each month, and animals born in the window (members).
    let (mut alive, mut born) = (Vec::new(), 0.0);
    let roots: Vec<u32> = w.life.alive().filter(|b| animal(&w, b)).map(|b| w.life.brain(b.lineage).map_or(u32::MAX, |l| l.root)).collect();
    w.events.clear();
    for _ in 0..36 {
        pcore::step(&mut w);
        assert!(w.life.alive().filter(|b| animal(&w, b)).all(|b| b.count == 1.0), "an animal became a node");
        born += w.events.iter().filter(|e| matches!(e.kind, EventKind::Born { root, .. } if roots.contains(&root))).map(|e| e.value).sum::<f64>();
        w.events.clear();
        alive.push(w.life.alive().filter(|b| animal(&w, b)).count());
    }
    println!("seed {seed}: animals alive at the end of each month: {alive:?}; born in the window {born}");
    assert!(w.life.alive().any(|b| !animal(&w, b) && b.count > 1.0), "no light-eater node");
    books.assert_kept(&w, 1e-9);
    (born, alive.last().copied().unwrap_or(0))
}

/// What body `s` holds that can become body (`worth`), mol, and its gene `g`, after its next update (`pcore::day` until
/// it has updated).
fn after_update(w: &mut World, s: Handle, g: usize) -> (f64, f32) {
    let last = w.life[s].last;
    while w.life[s].last == last {
        pcore::day(w);
    }
    (worth(w, s), w.life[s].genes[g])
}

/// What the body at `s` holds that can become body (`copy::buildable`), at its ground's temperature.
fn worth(w: &World, s: Handle) -> f64 {
    let b = &w.life[s];
    copy::buildable(w, b, w.t_ground_last(b.cell as usize), b.activity(&w.chem, &w.cfg.phys))
}

/// Selection inside a node moves its mean as selection among its members would (`copy::sample`, `copy::select`): a
/// light-eater node at its first update (making no brood, so all it builds stays in it), whose members differ in one
/// gene by a standard deviation of 0.3 steps. The same node with that gene at its mean plus and minus the deviation,
/// and no spread, gains W+ and W- in an update (in what can become body, `copy::buildable`) (as two kinds of members would, each on its own); the covariance of the
/// gene with fitness in two such halves moves the mean by sd (W+ - W-) / (W+ + W-) (Robertson 1966). The node's own
/// update moves its mean as much, within a quarter, the same way, with a sample at each update (`sample_every` 1). It
/// costs the node one more update as far as its growth.
#[test]
fn selection_inside_a_node_moves_its_mean_as_its_members_would() {
    let mut w = small_world();
    w.cfg.sample_every = 1;
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::alike());
    let s = live(&w).into_iter().find(|&s| w.life[s].count > 10.0 && w.life[s].adult(&w.chem)).expect("an adult node");
    let books = Books::now(&w);
    let lay = Layout::of(&w.chem);
    for (g, name) in [(body::STRENGTH, "strength"), (body::REPAIR, "repair")] {
        let sd = 0.3 * pcore::body::steps(&lay, &vec![1.0; lay.len])[g].recip();
        let b = &mut w.life[s];
        (b.genes[ALLOC], b.brood) = (0.0, 0.0);
        b.prepare(&w.chem, &w.cfg);
        b.spread = vec![0.0; lay.len];
        b.spread[g] = sd * sd;
        let (z, m0) = (b.genes[g], worth(&w, s));
        // The two halves, each on its own in a copy of the world.
        let half = |d: f32| {
            let mut v = w.clone();
            let b = &mut v.life[s];
            (b.genes[g], b.spread) = (z + d, Vec::new());
            b.prepare(&v.chem, &v.cfg);
            (after_update(&mut v, s, g).0 / m0).ln()
        };
        let (up, down) = (half(sd), half(-sd));
        let pair = sd * ((up - down) / 2.0).tanh() as f32;
        let (m1, z1) = after_update(&mut w, s, g);
        println!("{name}: ln W+ {up:.3e}, ln W- {down:.3e}, node ln W {:.3e}; mean moved {:.3e}, the two halves {pair:.3e}", (m1 / m0).ln(), z1 - z);
        assert!((up - down).abs() > 1e-5, "{name}: no selection to measure ({up} and {down})");
        assert!(((z1 - z) / pair - 1.0).abs() < 0.25, "{name}: the node's mean moved {} where the two halves move it {pair}", z1 - z);
    }
    books.assert_kept(&w, 1e-9);
}
