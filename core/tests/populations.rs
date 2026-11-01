//! Law 4 (Copy) in populations: two parents mix, and population nodes spread, split and merge.

use pcore::life::Handle;
use pcore::body::{Body, Layout, CHILD, COPY_MASS, DISPERSE, MUT, STRENGTH};
use pcore::event::EventKind;
use pcore::units::DAY_S;
use pcore::{body, brain, copy, life, nodes, World};
use worldgen::testkit::{brood, life_totals, children, founder, out, small_world, start, Books, Brain};

/// Two parents: the gene value of the first body in the genes under test, and the step the catalyst genes move in.
const BASE: f32 = 3.0;
const STEP: f32 = 0.3;

/// A starting eater of a brain that only rests, weak, staying where it is, mature at 10^`copy` kg of body, mutating
/// little (so the noise on a gene is far less than the steps tested), with children 10^-3 of its copy mass.
fn mixer(w: &mut World, i: usize, copy: f32) -> Body {
    let mut b = founder(w, 1, i, &Brain::only(brain::REST));
    (b.genes[DISPERSE], b.genes[STRENGTH], b.genes[COPY_MASS]) = (0.0, 0.0, copy);
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
    let sb = w.add(b);
    let (sa, a) = out(w, a);
    (sa, a, sb)
}

/// Make `n` children of `a` one after another (a day apart, each from a new brood of one), and return them.
fn copies(w: &mut World, a: &mut Body, n: usize) -> Vec<Body> {
    for _ in 0..n {
        w.time += DAY_S;
        brood(w, a, 1.0);
        assert!(w.in_cell(a.cell as usize, |w, d| copy::copy(w, d, a)), "no child");
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
    w.add(near);
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

/// A mature node's children start a node of their own (they are immature) that carries the variance
/// left among them; when they join their parents' node (`nodes::merge`) its spread is widened by it.
#[test]
fn a_node_spreads_with_its_children() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::testkit::eater_nodes(&mut w, 50.0);
    let books = Books::now(&w);
    let s = live(&w).into_iter().find(|&s| w.life[s].count > 10.0).expect("a node");
    let mut b = std::mem::take(&mut w.life[s]);
    // Members are mature (a low copy mass), each has a child and all children stay in the cell.
    (b.genes[COPY_MASS], b.genes[DISPERSE]) = (-0.4, 0.0);
    b.prepare(&w.chem, &w.cfg);
    let n0 = b.count;
    brood(&w, &mut b, n0);
    let before = live(&w);
    assert!(b.spread.is_empty() && w.in_cell(b.cell as usize, |w, d| copy::copy(w, d, &mut b)), "no children");
    assert_eq!(b.count, n0, "the children joined their mature parents");
    w.life[s] = b;
    let kids: Vec<Handle> = live(&w).into_iter().filter(|x| !before.contains(x) && *x != s).collect();
    assert_eq!(kids.len(), 1, "the children are one node of their own");
    let c = &w.life[kids[0]];
    assert!((c.count - n0).abs() < 1e-6 * n0 && !c.adult(&w.chem), "{} young members", c.count);
    assert!(c.spread.iter().all(|&v| v >= 0.0) && c.spread.iter().any(|&v| v > 0.0) && c.row_spread > 0.0, "children keep no spread");
    // Grown, they join the node (as the month's merge does): its spread is widened.
    w.in_cell(w.life[s].cell as usize, |w, d| nodes::merge(w, d, kids[0], s));
    let b = std::mem::take(&mut w.life[s]);
    assert!(b.count > 1.5 * n0 && b.spread.iter().any(|&v| v > 0.0) && b.row_spread > 0.0, "the merge widened nothing");

    w.life[s] = b;
    books.assert_kept(&w, 1e-9);
}

/// Children that leave a varied node join the node of their own kind in the next cell: pairs of children drawn from a
/// node held at the split threshold in every gene, sent to a cell whose node has the same genes and spread, join it
/// nearly always, as they lie within its spread (a pair's mean can differ from the node's by more than `alike` steps).
#[test]
fn dispersing_children_of_a_varied_node_join_their_kind() {
    let mut w = small_world();
    worldgen::testkit::eater_nodes(&mut w, 50.0);
    w.cfg.phys.c_copy = 0.0;
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
        assert!(w.in_cell(c.cell as usize, |w, d| copy::copy(w, d, &mut c)), "no children");
        // (They land in the next cell, as at a batch's end.)
        life::land(&mut w);
        (joined, tries) = (joined + (live(&w).len() == n0) as u32, tries + 1);
        w.events.clear();
    }
    println!("pairs of children that joined the node of their kind: {joined} of {tries}");
    assert!(joined as f64 >= 0.95 * tries as f64, "{joined} of {tries} joined");
}

/// Members who join a node are kept (`nodes::merge`): a body of 1.4 members adds 1.4 to its count, and the genes become
/// the means by members.
#[test]
fn joining_a_node_keeps_members_and_weighs_genes_by_them() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::testkit::eater_nodes(&mut w, 50.0);
    let t = live(&w).into_iter().find(|&s| w.life[s].count > 1.0).expect("a node");
    w.life[t].count = 3.0;
    let (o, before) = (w.life[t].clone(), life_totals(&w).0);
    let mut c = o.clone();
    (c.count, c.genes) = (1.4, o.genes.iter().map(|g| g + 0.1).collect());
    let s = w.add(c);
    w.in_cell(w.life[t].cell as usize, |w, d| nodes::merge(w, d, s, t));
    let (after, node) = (life_totals(&w).0, &w.life[t]);
    assert!((after - (before + 1.4)).abs() < 1e-12 * after && (node.count - 4.4).abs() < 1e-12, "members {before} + 1.4 then {after}");
    for (g, (x, y)) in o.genes.iter().zip(&node.genes).enumerate() {
        let want = x + 0.1 * 1.4 / 4.4;
        assert!((y - want).abs() < 1e-5 * (1.0 + x.abs()), "gene {g}: {x} then {y}, wanted {want}");
    }
}
