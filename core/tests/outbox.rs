//! The outbox and the serial phase of a batch (plan 3.6): an entry whose body or target is gone does nothing and keeps
//! the books, a holder killed in the parallel phase lets go of its part in the serial phase, and the order in which the
//! domains run and the threads that run them change nothing.

use pcore::body::{die, Body, BROKEN, CHILD, COPY_MASS, DISPERSE, HIDDEN, LACK, LIQ};
use pcore::event::EventKind;
use pcore::life::Handle;
use pcore::outbox::{deliver, Entry};
use pcore::store::NodeId;
use pcore::units::DAY_S;
use pcore::{bond, brain, copy, nodes, World};
use worldgen::schedule::Schedule;
use worldgen::testkit::{brood, children, dry_lands, eater, founder, moved, node_of, plate, scale, start, Books, Brain, EATER_KG, MATTER};

/// The body at `h` dies where it is.
fn kill(w: &mut World, h: Handle) {
    let b = std::mem::take(&mut w.life[h]);
    moved(w, (b.cell as usize, h), |w, d| die(w, d, h, b, LACK));
}

/// A node of 5 resting members in cell `i` whose children all start in a neighbour cell, with a brood of `k` children
/// ready.
fn dispersing(w: &mut World, i: usize, k: f64) -> Body {
    let mut n = eater(w, i, brain::REST, 1.0);
    (n.genes[CHILD], n.genes[DISPERSE]) = (-std::f32::consts::LOG10_2, 1.0);
    let mut n = node_of(w, n, 5.0);
    brood(w, &mut n, k);
    n
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

    // Moves of a body that died before its batch ended.
    let (e, at_j) = (rest(&mut w), w.life.in_cell(j));
    let a = w.add(e, 1);
    for kin in [false, true] {
        let id = w.life[a].id;
        w.in_domain(i, |_, d| d.out.push(Entry::Go { id, from: i as u32, to: j as u32, pushed: 1.0, lane: 1.0, carries: false, kin }));
    }
    kill(&mut w, a);
    deliver(&mut w);
    assert!(w.life.in_cell(j) == at_j && w.life.pushed.iter().all(|&e| e == 0.0), "a move of a dead body landed");

    // Less than one member of a node's children, all starting in a neighbour cell where no node of theirs is: back
    // into the parent's brood, or to the ground of its cell if it died.
    for dies in [false, true] {
        let n = dispersing(&mut w, i, 0.6);
        let s = w.add(n, 1);
        let mut n = std::mem::take(&mut w.life[s]);
        n.brood = n.brooded(&w.chem);
        let (id, before) = (n.id, [n.store[MATTER], n.brood, n.e]);
        assert!(w.in_domain(n.cell as usize, |w, d| copy::copy(w, d, &mut n)), "no children");
        assert!(n.store[MATTER] < before[0], "the litter was not taken from its parent");
        w.life[s] = n;
        if dies {
            kill(&mut w, s);
        }
        deliver(&mut w);
        assert!(children(&w, id).is_empty(), "a litter of less than one member started a node of its own");
        let p = &w.life[s];
        let now = if dies { before } else { [p.store[MATTER], p.brood, p.e] };
        for (now, was) in now.iter().zip(before) {
            assert!((now / was - 1.0).abs() < 1e-12, "the litter did not come back to its parent: {now} against {was}");
        }
    }

    // Watches of a body that died, and by a body no longer of the lineage it watched in.
    let (e, f, g) = (rest(&mut w), rest(&mut w), rest(&mut w));
    let (by, of, gone) = (w.add(e, 1), w.add(f, 1), w.add(g, 1));
    let (rows, lineage, ids) = (w.life[by].rows.clone(), w.life[by].lineage, [by, of, gone].map(|h| w.life[h].id));
    kill(&mut w, gone);
    w.in_domain(i, |_, d| d.out.push(Entry::Watch { by: ids[0], n: 1.0, lineage, of: ids[2] }));
    w.in_domain(i, |_, d| d.out.push(Entry::Watch { by: ids[0], n: 1.0, lineage: lineage + 1, of: ids[1] }));
    deliver(&mut w);
    assert!(w.life[by].rows == rows && !w.events.iter().any(|e| matches!(e.kind, EventKind::Watched { .. })), "a watch was applied");

    // A part act its body cannot pay, and one on a part that is gone.
    let p = plate(&mut w, i, 5.0, false);
    let (part, kg) = (bond::id(&w, p), w.parts.parts[p].mass(&w.chem));
    let e = rest(&mut w);
    let (a, c_handle) = (w.add(e, 1), w.cfg.phys.c_handle);
    let id = w.life[a].id;
    w.cfg.phys.c_handle = 1e30;
    w.in_domain(i, |_, d| d.out.push(Entry::Act { id, on: Ok((1, part)), node: None }));
    w.in_domain(i, |_, d| d.out.push(Entry::Act { id, on: Ok((1, NodeId { gen: part.gen + 1, ..part })), node: None }));
    deliver(&mut w);
    w.cfg.phys.c_handle = c_handle;
    assert_eq!(w.parts.parts[p].mass(&w.chem), kg, "a part act was applied");

    // Two members of a node with a part act queued: one takes from the anchored part and comes back to its node, the
    // other takes hold of it and lives on as a body.
    bond::link(&mut w, p, bond::GROUND, 1e9);
    let e = rest(&mut w);
    let n = node_of(&mut w, e, 5.0);
    let s = w.add(n, 1);
    let parts = w.in_domain(w.life[s].cell as usize, |w, d| nodes::open(w, d, s, 2));
    let (node, count) = (w.life[s].id, w.life[s].count);
    let ids: Vec<u64> = parts.iter().map(|&h| w.life[h].id).collect();
    for (&h, verb) in parts.iter().zip([1, 0]) {
        w.life[h].leaving = true;
        let id = w.life[h].id;
        w.in_domain(i, |_, d| d.out.push(Entry::Act { id, on: Ok((verb, part)), node: Some(node) }));
    }
    deliver(&mut w);
    let s = w.life.find(node).unwrap();
    assert!(w.life.find(ids[0]).is_none() && w.life[s].count == count + 1.0, "the part that holds nothing did not come back");
    let h = w.life.find(ids[1]).expect("the part that holds the part lives on");
    assert!(w.life[h].hold == Some(part) && !w.life[h].leaving, "the part did not take hold of the part");
    books.assert_kept(&w, 1e-12);
}

/// A body of the parallel phase that kills a body holding a part: the dead holder lets go of the part in the serial
/// phase (`bond::LET_GO`), as parts change only there, and elements and energy are kept.
#[test]
fn a_holder_killed_in_the_parallel_phase_lets_go_in_the_serial_phase() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    let p = plate(&mut w, i, 5.0, false);
    // A small holder, not due for a long time, and a taker due at the next batch whose area holds the whole cell.
    let mut h = eater(&mut w, i, brain::REST, 1.0);
    scale(&mut w, &mut h, 0.01);
    h.hold = Some(bond::id(&w, p));
    let h = w.add(h, 300);
    let id = w.life[h].id;
    w.parts.parts[p].held = id;
    let mut t = founder(&mut w, 1, i, &Brain::verb(brain::TAKE_FROM));
    let m = t.mass(&w.chem);
    scale(&mut w, &mut t, EATER_KG / m);
    (t.genes[DISPERSE], t.genes[COPY_MASS], t.genes[LIQ]) = (0.0, 1.0, -4.0);
    w.add(t, 1);
    w.cfg.phys.sweep *= 1e9;
    for _ in 0..10 {
        w.time += w.cfg.batch_days as u64 * DAY_S;
        pcore::survey::batch(&mut w, false);
    }
    assert!(w.life.find(id).is_none(), "the taker did not kill the holder");
    // (It let go after its death, when the outboxes were applied.)
    let at = |f: &dyn Fn(&EventKind) -> bool| w.events.iter().position(|e| f(&e.kind));
    assert!(at(&|k| matches!(k, EventKind::BondBroken)) > at(&|k| matches!(k, EventKind::Died { .. })), "it let go before it died");
    assert_eq!(w.parts.parts[p].held, bond::NOBODY, "the dead holder still holds the part");
    books.assert_kept(&w, 1e-12);
}

/// The order in which the domains run in the parallel phase of a batch and of the survey, and the threads that run them,
/// change nothing: the lineages and born rows made there get the same global ids (plan 3.3), and the world ends the
/// same, bit for bit, with its events and counters. In the batches each kind of the serial phase happens: a dispersing
/// litter lands, a watch and a part act are applied, a body that holds a part runs, one anchors its part, lineages are
/// made; in the first survey, movers kill a body in a cell with no part and a holder in its cell with a part, which lets
/// go of the part, and the month's merge joins two nodes with different born rows into new born rows; the books are
/// kept.
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
    // A dispersing node, a watcher, a body that takes hold of an anchored part, one that holds a part and one that
    // anchors the part it holds, in cells spread over the island.
    let land = dry_lands(&w);
    let at = |k: usize| land[k * land.len() / 5];
    let n = dispersing(&mut w, at(0), 3.0);
    let parent = w.add(n, 1);
    let parent = w.life[parent].id;
    let watcher = founder(&mut w, 1, at(1), &Brain::verb(brain::WATCH));
    w.add(watcher, 1);
    let (p, q) = (plate(&mut w, at(2), 5.0, false), plate(&mut w, at(3), 5.0, false));
    bond::link(&mut w, p, bond::GROUND, 1e9);
    let grabber = founder(&mut w, 1, at(2), &Brain::verb(brain::VERBS));
    w.add(grabber, 1);
    let mut holder = eater(&mut w, at(3), brain::REST, 1.0);
    holder.hold = Some(bond::id(&w, q));
    let holder = w.add(holder, 1);
    let (holder, last) = (w.life[holder].id, w.life[holder].last);
    w.parts.parts[q].held = holder;
    let r = plate(&mut w, at(4), 5.0, false);
    let mut anchorer = eater(&mut w, at(4), brain::ANCHOR, 1.0);
    anchorer.hold = Some(bond::id(&w, r));
    let anchorer = w.add(anchorer, 1);
    w.parts.parts[r].held = w.life[anchorer].id;
    // A holder in a cell of its own with a part, and two alike nodes of one lineage whose born rows differ.
    let busy: Vec<usize> = (0..5).map(at).collect();
    let free: Vec<usize> = land.iter().copied().filter(|c| !busy.contains(c) && c % 3 != 0).collect();
    let (x, m) = (free[0], free[free.len() / 2]);
    let s = plate(&mut w, x, 5.0, false);
    let mut doomed = eater(&mut w, x, brain::REST, 1.0);
    doomed.hold = Some(bond::id(&w, s));
    let doomed = w.add(doomed, 300);
    let doomed_id = w.life[doomed].id;
    w.parts.parts[s].held = doomed_id;
    let e = eater(&mut w, m, brain::REST, 1.0);
    let (big, mut small) = (node_of(&mut w, e.clone(), 8.0), node_of(&mut w, e, 4.0));
    let rows = [w.life.born(&big).to_vec(), w.life.born(&small).iter().map(|r| r + 0.5).collect()];
    small.genome = w.life.new_genome(rows[1].clone());
    let big = w.add(big, 300);
    let big = w.life[big].id;
    w.add(small, 300);
    // Movers pushed through every third cell and the holder's hard enough to kill (`Body::breaking`; each body there
    // takes its share by mass).
    let (h, all) = (w.life.find(doomed_id).unwrap(), w.life.cell(x).map(|o| w.life[o].mass(&w.chem)).sum::<f64>());
    let kill_holder = 2.0 * w.life[h].breaking(&w.chem, &w.cfg, 1.0) * all / w.life[h].mass(&w.chem);
    w.life.pushed = (0..w.mesh.cells().n()).map(|i| if i == x { kill_holder } else if i % 3 == 0 && !busy.contains(&i) { 3e6 } else { 0.0 }).collect();
    w.drain_events();
    let books = Books::now(&w);
    let lineages = w.life.brains.len();
    // On pools of 1 and 4 threads (with the feature `threads`; in order without it), in order and reversed.
    let ends = [(1, false), (1, true), (4, false), (4, true)].map(|(threads, reversed)| {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        pool.install(|| {
            let mut v = w.clone();
            pcore::survey::survey(&mut v, reversed);
            // What the first survey did: a death by breaking in a cell with no part (on the pool), a part let go of,
            // the merged node's born rows.
            let did = |f: &dyn Fn(&pcore::event::Event) -> bool| v.events.iter().any(f);
            let merged = v.life.find(big).map(|h| v.life.born(&v.life[h]).to_vec());
            let broken = did(&|e| matches!(e.kind, EventKind::Died { cause: BROKEN, .. }) && e.node != x as u32);
            let first = (broken, did(&|e| matches!(e.kind, EventKind::BondBroken)), merged);
            for _ in 0..30 {
                pcore::survey::batch(&mut v, reversed);
                v.time += v.cfg.batch_days as u64 * DAY_S;
                pcore::survey::survey(&mut v, reversed);
            }
            (v, first)
        })
    });
    let (killed, let_go, merged) = &ends[0].1;
    let new_rows = merged.as_ref().is_some_and(|r| !rows.contains(r));
    assert!(*killed && *let_go && new_rows, "the first survey: killed {killed}, let go {let_go}, new born rows {new_rows}");
    assert!(ends.iter().all(|e| e.1 == ends[0].1), "the threads or the order of the domains changed the first survey");
    let ends = ends.map(|e| e.0);
    // (The whole saved world, and the events and counters of the batches, which are not saved.)
    let state = |v: &World| (serde_json::to_string(v).unwrap(), format!("{:?} {:?}", v.events, v.counts));
    assert!(ends.iter().all(|v| state(v) == state(&ends[0])), "the threads or the order of the domains changed the world");
    let v = &ends[0];
    let kind = |f: &dyn Fn(&EventKind) -> bool| v.events.iter().filter(|e| f(&e.kind)).count();
    let landed = children(v, parent).iter().filter(|&&c| v.life[c].cell as usize != at(0)).count();
    let (watched, held) = (kind(&|k| matches!(k, EventKind::Watched { .. })), kind(&|k| matches!(k, EventKind::BondMade)));
    let ran = v.life.find(holder).is_none_or(|h| v.life[h].last > last);
    let made = v.life.brains.len() - lineages;
    assert!(landed > 0 && watched > 0 && held > 0 && ran && made > 1, "litters landed {landed}, watches {watched}, holds {held}, holder ran {ran}, lineages {made}");
    let anchored = v.parts.bonds.of(r).any(|b| b.0 == bond::GROUND);
    assert!(anchored && v.parts.parts[r].held == bond::NOBODY, "the part was not anchored and let go of (anchored {anchored})");
    books.assert_kept(v, 1e-12);
}
