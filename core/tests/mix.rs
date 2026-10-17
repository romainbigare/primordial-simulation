//! Two-parent copying (15 Oct 2026, handover 6.4): when a mature body makes children it looks in its cell for a
//! partner, another mature body alike to it (the likeness that lets bodies join, `body::alike`); each gene of the
//! children is one parent's, a keyed draw gene by gene, and each born brain row too, then the generation's
//! mutation as for one parent. Unlike bodies never mix; with no partner a child is a copy of one parent; the
//! partner loses nothing; a node's children get the spread a merge of the two would pool.

use pcore::body::{Body, Layout, CHILD, COPY_MASS, DISPERSE, LIQ, MUT, STRENGTH};
use pcore::event::EventKind;
use pcore::rng::{Purpose, Rng};
use pcore::{body, brain, copy, lineage, World};

/// Gene value of the first body in the genes under test, and the step the catalyst genes move in (0.3).
const BASE: f32 = 3.0;
const STEP: f32 = 0.3;

fn dry_land(w: &World) -> usize {
    (0..w.grid.n()).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land")
}

fn start() -> (World, usize) {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let i = dry_land(&w);
    (w, i)
}

/// A starting eater (30 kg, a body of 21 kg) of a brain that only rests, weak, staying where it is, copying at
/// 10^`copy` kg of body, mutating little (so the noise on a gene is far less than the steps tested), with children
/// 10^-3 of its copy mass.
fn eater(w: &mut World, i: usize, copy: f32) -> Body {
    let shape = lineage::shape(w.chem.ns(), 12, 6);
    let mut fixed = [-8.0; brain::FIXED];
    fixed[brain::REST] = 8.0;
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, fixed, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let (l, g) = (w.life.new_lineage(pcore::lineage::Lineage { shape, seen: 3, sight: 1, base, root: 0 }), w.life.new_genome(rows));
    let mut b = worldgen::life::make(w, 1, i, l, g).expect("body");
    (b.genes[DISPERSE], b.genes[STRENGTH], b.genes[COPY_MASS], b.genes[LIQ]) = (0.0, 0.0, copy, -4.0);
    (b.genes[MUT], b.genes[CHILD]) = (-3.0, -3.0);
    b
}

/// A body `f` times the size (stores and heat; booked as an edit).
fn scale(w: &mut World, b: &mut Body, f: f64) {
    for k in 0..w.chem.ns() {
        w.chem.add_elems(k, (f - 1.0) * b.store[k], &mut w.ledger.imposed_elems);
        w.ledger.imposed_energy += (f - 1.0) * b.store[k] * w.chem.subs[k].hf;
        b.store[k] *= f;
    }
    w.ledger.imposed_energy += (f - 1.0) * b.e;
    b.e *= f;
}

/// A body placed and taken out again, prepared, to call the laws on directly.
fn out(w: &mut World, b: Body) -> (usize, Body) {
    let s = w.life.add(b, w.day, 1);
    pcore::survey::survey(w);
    w.day += 1;
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    b.prepare(&w.chem, &w.cfg.phys);
    (s, b)
}

/// Two mature eaters in one cell whose catalyst genes are `BASE` and `BASE` + `gap` steps; the second (the
/// partner) is placed, the first is taken out to copy. The second's born rows are the first's plus 1.
fn pair(w: &mut World, i: usize, gap: f32) -> (usize, Body, usize) {
    let lay = Layout::of(&w.chem);
    let (mut a, mut b) = (eater(w, i, 1.0), eater(w, i, 1.0));
    for g in lay.cat..lay.min {
        (a.genes[g], b.genes[g]) = (BASE, BASE + gap * STEP);
    }
    let rows = w.life.born(&b).iter().map(|x| x + 1.0).collect();
    w.life.genomes[b.genome as usize].rows = rows;
    let sb = w.life.add(b, w.day, 1);
    let (sa, a) = out(w, a);
    (sa, a, sb)
}

/// The brood of one child of `a`: its body is 10^-3 of the copy mass, of matter over 1 - h.
fn one_child(w: &World, a: &Body) -> f64 {
    let mm = w.chem.subs[w.chem.matter().expect("matter")].molar_mass;
    10f64.powf(a.gene(CHILD)) * a.dec(COPY_MASS) * (1.0 - a.wet_h(&w.chem).1) / mm
}

/// Make `n` children of `a` one after another (a day apart, each from a new brood), and return them.
fn children(w: &mut World, s: usize, a: &mut Body, n: usize) -> Vec<Body> {
    let child = one_child(w, a);
    let mut out = Vec::new();
    for _ in 0..n {
        w.day += 1;
        a.brood = child;
        assert!(copy::copy(w, s, a), "no child");
    }
    for k in 0..w.life.bodies.len() {
        let c = &w.life.bodies[k];
        if c.alive && c.parent == a.id(s) {
            out.push(c.clone());
        }
    }
    assert_eq!(out.len(), n);
    out
}

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
    let (s, mut a, sb) = pair(&mut w, i, 2.0);
    let (rows_a, rows_b) = (w.life.born(&a).to_vec(), w.life.born(&w.life.bodies[sb]).to_vec());
    w.events.clear();
    let kids = children(&mut w, s, &mut a, 200);
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

/// With no partner, or none mature, a child is a copy of one parent as before: every gene and row is its parent's
/// plus noise, and the birth is not marked.
#[test]
fn with_no_partner_a_child_has_one_parent() {
    let (mut w, i) = start();
    let lay = Layout::of(&w.chem);
    // Alone in its cell.
    let mut a = eater(&mut w, i, 1.0);
    (lay.cat..lay.min).for_each(|g| a.genes[g] = BASE);
    let (s, mut a) = out(&mut w, a);
    w.events.clear();
    let kids = children(&mut w, s, &mut a, 30);
    for c in &kids {
        assert!((lay.cat..lay.min).all(|g| (c.genes[g] - BASE).abs() < 0.05), "a gene that is not its parent's");
        // (A child's rows change a little now and then: the generation's mutation.)
        let drift = w.life.born(c).iter().zip(w.life.born(&a)).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert!(drift < 0.01, "rows {drift} from its parent's");
    }
    assert_eq!(mixed(&w), (0.0, 30.0));
    // A partner that is not mature (its body under its copy mass) is not one.
    let (mut w, i) = start();
    let (s, mut a, sb) = pair(&mut w, i, 2.0);
    w.life.bodies[sb].genes[COPY_MASS] = 3.0;
    assert!(!w.life.bodies[sb].adult(&w.chem));
    w.events.clear();
    let kids = children(&mut w, s, &mut a, 30);
    assert!(kids.iter().all(|c| (lay.cat..lay.min).all(|g| (c.genes[g] - BASE).abs() < 0.05)));
    assert_eq!(mixed(&w), (0.0, 30.0));
    // The only body in the cell that is mature is the parent itself: it is never its own partner.
    let (mut w, i) = start();
    let a = eater(&mut w, i, 1.0);
    let (s, mut a) = out(&mut w, a);
    w.events.clear();
    children(&mut w, s, &mut a, 5);
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
    let (s, mut a, _) = pair(&mut w, i, limit + 1.0);
    w.events.clear();
    let kids = children(&mut w, s, &mut a, 30);
    assert!(kids.iter().all(|c| (lay.cat..lay.min).all(|g| (c.genes[g] - BASE).abs() < 0.05)), "unlike bodies mixed");
    assert_eq!(mixed(&w).0, 0.0);
    // Alike in all but one gene, which is past the limit: still unlike.
    let (mut w, i) = start();
    let (s, mut a, sb) = pair(&mut w, i, 1.0);
    w.life.bodies[sb].genes[lay.cat] = BASE + (limit + 0.5) * STEP;
    w.events.clear();
    children(&mut w, s, &mut a, 10);
    assert_eq!(mixed(&w).0, 0.0, "one gene past the limit is unlike");
    // Brain sizes count by whole units: a partner 6 units bigger in brain (past 4) is unlike, 3 units is alike.
    for (units, mixes) in [(6.0, false), (3.0, true)] {
        let (mut w, i) = start();
        let (s, mut a, sb) = pair(&mut w, i, 1.0);
        w.life.bodies[sb].genes[body::HIDDEN] += units;
        w.events.clear();
        children(&mut w, s, &mut a, 10);
        assert_eq!(mixed(&w).0 > 0.0, mixes, "{units} units of brain apart");
    }
    // Two partners, one near and one further but alike: all the children take the near one's genes.
    let (mut w, i) = start();
    let (s, mut a, _) = pair(&mut w, i, 3.0);
    let mut near = eater(&mut w, i, 1.0);
    for g in lay.cat..lay.min {
        near.genes[g] = BASE + 1.0 * STEP;
    }
    w.life.add(near, w.day, 1);
    let kids = children(&mut w, s, &mut a, 60);
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
    let before = w.life.bodies[sb].clone();
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    w.events.clear();
    children(&mut w, s, &mut a, 20);
    assert_eq!(mixed(&w).0, 20.0);
    let after = &w.life.bodies[sb];
    assert_eq!((&after.store, after.e, &after.genes, after.brood, after.count), (&before.store, before.e, &before.genes, before.brood, before.count));
    assert_eq!(w.life.born(after), w.life.born(&before));
    w.life.bodies[s] = a;
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// A node's children by two nodes of as many members have the mean between the two and the spread a merge of the
/// two would pool: each one's spread by its share plus the squared gap of the means times the share of each, per
/// gene (a quarter at halves); a node with a partner of a tenth of its members takes a tenth of its genes.
#[test]
fn a_nodes_mixed_children_have_the_pooled_spread() {
    let (mut w, i) = start();
    let lay = Layout::of(&w.chem);
    let (sa_, sb_, gap) = (0.01f32, 0.03f32, 2.0f32);
    let members = 20_000.0;
    for (n_b, p) in [(members, 0.5f32), (members / 10.0, 1.0 / 11.0)] {
        let (s, mut a, sb) = pair(&mut w, i, gap);
        // Both are nodes: the stores of their members, a spread in every catalyst gene.
        scale(&mut w, &mut a, members);
        a.count = members;
        a.spread = vec![0.0; lay.len];
        let mut b = std::mem::take(&mut w.life.bodies[sb]);
        scale(&mut w, &mut b, n_b);
        b.count = n_b;
        b.spread = vec![0.0; lay.len];
        for g in lay.cat..lay.min {
            (a.spread[g], b.spread[g]) = (sa_, sb_);
        }
        w.life.bodies[sb] = b;
        // A brood of 2,000 children (members of the new node).
        a.brood = 2000.0 * one_child(&w, &a);
        w.day += 1;
        assert!(a.adult(&w.chem) && w.life.bodies[sb].adult(&w.chem));
        w.events.clear();
        assert!(copy::copy(&mut w, s, &mut a));
        // (The brood over the child's matter, a float: 1999.9999999999998 with the matter of 3,456 g a mol, 2000 with the one before.)
        assert!((mixed(&w).0 - 2000.0).abs() < 1e-9 * 2000.0, "{} children", mixed(&w).0);
        let c = w.life.bodies.iter().find(|c| c.alive && c.parent == a.id(s)).expect("the children").clone();
        assert!((c.count - 2000.0).abs() < 1e-6 * 2000.0, "{} members", c.count);
        for g in lay.cat..lay.min {
            // The mean: a's plus the share p of the gap (the share drawn is p within a few thousandths).
            let mean = BASE + p * gap * STEP;
            let sd = (p * (1.0 - p) / 2000.0).sqrt() * STEP * gap;
            assert!((c.genes[g] - mean).abs() < 4.5 * sd + 0.002, "gene {g}: mean {} not {mean} (sd {sd})", c.genes[g]);
            // The spread: pooled, less the share of it a group of 2,000 does not keep among itself.
            let want = ((1.0 - p) * sa_ + p * sb_ + p * (1.0 - p) * (gap * STEP).powi(2)) * (1.0 - 1.0 / 2000.0);
            // (The share drawn is p within a few of its standard deviations, which moves the pooled spread by their effect.)
            let sd = (p * (1.0 - p) / 2000.0).sqrt();
            let tol = 4.5 * sd * ((1.0 - 2.0 * p).abs() * (gap * STEP).powi(2) + (sb_ - sa_)) + 0.01 * want;
            assert!((c.spread[g] - want).abs() < tol, "gene {g}: spread {} not {want} (p {p}, tolerance {tol})", c.spread[g]);
        }
        // The partner's spread is unchanged, and the other genes are as theirs (the same in both).
        assert_eq!(w.life.bodies[sb].spread[lay.cat], sb_);
        w.life.bodies[s] = a;
        // (The next pair is in a world of its own.)
        let (w2, _) = start();
        w = w2;
    }
}
