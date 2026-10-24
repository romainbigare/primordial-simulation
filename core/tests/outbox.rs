//! The outbox and the serial phase of a batch (plan 3.6): an entry whose body or target is gone does nothing and keeps
//! the books, and the order in which the domains run changes nothing.

use pcore::body::{die, CHILD, DISPERSE, HIDDEN, LACK};
use pcore::event::EventKind;
use pcore::life::Handle;
use pcore::outbox::{deliver, Entry};
use pcore::store::NodeId;
use pcore::units::DAY_S;
use pcore::{bond, brain, copy, nodes, World};
use worldgen::schedule::Schedule;
use worldgen::testkit::{brood, children, eater, moved, node_of, plate, start, Books, MATTER};

/// The body at `h` dies where it is.
fn kill(w: &mut World, h: Handle) {
    let b = std::mem::take(&mut w.life[h]);
    moved(w, (b.cell as usize, h), |w| die(w, h, b, LACK));
}

/// Each kind of entry fails once and changes nothing but what its failure rule says: a move of a body that died, whole
/// or of members who would join their kind; children with no node to join and less than one member, back into their
/// living parent's brood, or to the ground of the cell their dead parent was in; a watch of a body that died, and one
/// whose watcher's lineage changed; a part act that cannot be paid, and one on a part that is gone; and a node's part
/// whose part act left it holding nothing, which comes back to its node, and one that took hold of the part, which
/// cannot. Elements and energy are kept.
#[test]
fn an_entry_whose_target_is_gone_does_nothing_and_keeps_the_books() {
    let (mut w, i) = start();
    let j = w.mesh.cells().around(i).1[0] as usize;
    let books = Books::now(&w);
    let rest = |w: &mut World| eater(w, i, brain::REST, 1.0);
    let shape = [0.0, 0.5];

    // Moves of a body that died before its batch ended.
    let (e, at_j) = (rest(&mut w), w.life.in_cell(j));
    let a = w.add(e, 1);
    for kin in [false, true] {
        w.life.queue(i, Entry::Go { id: w.life[a].id, from: i as u32, to: j as u32, pushed: 1.0, lane: 1.0, kin });
    }
    kill(&mut w, a);
    deliver(&mut w);
    assert!(w.life.in_cell(j) == at_j && w.life.pushed.iter().all(|&e| e == 0.0), "a move of a dead body landed");

    // Less than one member of a node's children, all starting in a neighbour cell where no node of theirs is: back
    // into the parent's brood, or to the ground of its cell if it died.
    for dies in [false, true] {
        let mut n = rest(&mut w);
        (n.genes[CHILD], n.genes[DISPERSE]) = (-std::f32::consts::LOG10_2, 1.0);
        let mut n = node_of(&mut w, n, 5.0);
        brood(&w, &mut n, 0.6);
        let s = w.add(n, 1);
        let mut n = std::mem::take(&mut w.life[s]);
        let (id, matter) = (n.id, n.store[MATTER]);
        assert!(copy::copy(&mut w, &mut n), "no children");
        assert!(n.store[MATTER] < matter, "the litter was not taken from its parent");
        w.life[s] = n;
        if dies {
            kill(&mut w, s);
        }
        deliver(&mut w);
        assert!(children(&w, id).is_empty(), "a litter of less than one member started a node of its own");
        if !dies {
            assert!((w.life[s].store[MATTER] / matter - 1.0).abs() < 1e-12, "the litter did not come back to its parent");
        }
    }

    // Watches of a body that died, and by a body no longer of the lineage it watched in.
    let (e, f, g) = (rest(&mut w), rest(&mut w), rest(&mut w));
    let (by, of, gone) = (w.add(e, 1), w.add(f, 1), w.add(g, 1));
    let (rows, lineage, ids) = (w.life[by].rows.clone(), w.life[by].lineage, [by, of, gone].map(|h| w.life[h].id));
    kill(&mut w, gone);
    w.life.queue(i, Entry::Watch { by: ids[0], n: 1.0, lineage, of: ids[2] });
    w.life.queue(i, Entry::Watch { by: ids[0], n: 1.0, lineage: lineage + 1, of: ids[1] });
    deliver(&mut w);
    assert!(w.life[by].rows == rows && !w.events.iter().any(|e| matches!(e.kind, EventKind::Watched { .. })), "a watch was applied");

    // A part act its body cannot pay, and one on a part that is gone.
    let p = plate(&mut w, i, 5.0, false);
    let (part, kg) = (bond::id(&w, p), w.parts.parts[p].mass(&w.chem));
    let e = rest(&mut w);
    let (a, c_handle) = (w.add(e, 1), w.cfg.phys.c_handle);
    let id = w.life[a].id;
    w.cfg.phys.c_handle = 1e30;
    w.life.queue(i, Entry::Act { id, on: Some((1, part)), shape, node: None });
    w.life.queue(i, Entry::Act { id, on: Some((1, NodeId { gen: part.gen + 1, ..part })), shape, node: None });
    deliver(&mut w);
    w.cfg.phys.c_handle = c_handle;
    assert_eq!(w.parts.parts[p].mass(&w.chem), kg, "a part act was applied");

    // Two members of a node with a part act queued: one takes from the anchored part and comes back to its node, the
    // other takes hold of it and lives on as a body.
    bond::link(&mut w, p, bond::GROUND, 1e9);
    let e = rest(&mut w);
    let n = node_of(&mut w, e, 5.0);
    let s = w.add(n, 1);
    let parts = nodes::open(&mut w, s, 2);
    let (node, count) = (w.life[s].id, w.life[s].count);
    let ids: Vec<u64> = parts.iter().map(|&h| w.life[h].id).collect();
    for (&h, verb) in parts.iter().zip([1, 0]) {
        w.life[h].leaving = true;
        w.life.queue(i, Entry::Act { id: w.life[h].id, on: Some((verb, part)), shape, node: Some(node) });
    }
    deliver(&mut w);
    let s = w.life.find(node).unwrap();
    assert!(w.life.find(ids[0]).is_none() && w.life[s].count == count + 1.0, "the part that holds nothing did not come back");
    let h = w.life.find(ids[1]).expect("the part that holds the part lives on");
    assert!(w.life[h].hold == Some(part) && !w.life[h].leaving, "the part did not take hold of the part");
    books.assert_kept(&w, 1e-12);
}

/// The order in which the domains run in the parallel phase of a batch changes nothing: the lineages and born rows
/// made there get the same global ids (plan 3.3), and the world ends the same, bit for bit.
#[test]
fn the_order_of_the_domains_changes_nothing() {
    let mut w = worldgen::new_world(1, 16, 16);
    let mut plan = Schedule::new(Some(0), true, worldgen::life::Seeding::default());
    plan.kinds.iter_mut().for_each(|k| k.1 = k.1.min(1));
    for _ in 0..2 {
        plan.place(&mut w);
        (0..w.cfg.months_per_year).for_each(|_| pcore::step(&mut w));
    }
    // Every body with a brain moves to a new lineage at its next update (`lineage::resize`).
    w.life.each_mut(|b| b.genes[HIDDEN] += if b.rows.is_empty() { 0.0 } else { 1.5 });
    let lineages = w.life.brains.len();
    let ends = [false, true].map(|reversed| {
        let mut v = w.clone();
        for _ in 0..30 {
            pcore::survey::batch(&mut v, reversed);
            v.time += v.cfg.batch_days as u64 * DAY_S;
        }
        v
    });
    let state = |v: &World| (serde_json::to_string(&v.life).unwrap(), serde_json::to_string(&v.store).unwrap());
    assert!(state(&ends[0]) == state(&ends[1]), "the order of the domains changed the world");
    let (made, crossed) = (ends[0].life.brains.len() - lineages, ends[0].counts.crossings.iter().sum::<f32>());
    let born = ends[0].events.iter().filter(|e| matches!(e.kind, EventKind::Born { .. })).count();
    assert!(made > 1 && crossed > 0.0 && born > 0, "{made} lineages made, {crossed} members moved, {born} litters born");
}
