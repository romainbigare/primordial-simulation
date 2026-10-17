//! Law 3 (Bond): parts and the bonds between them, structures and the shelter they give, and the bodies that make
//! and hold parts.

use pcore::bond::{self, GROUND};
use pcore::body::{Body, COPY_MASS, LIQ, STRENGTH};
use pcore::brain;
use pcore::chem::Phase;
use worldgen::testkit::{dry_land, plate, small_world, start as world, Books, Brain, Line};

/// A bond breaks when the weight it bears is more than it holds: of two parts hung on an anchored one by bonds that
/// hold 10 t weight, the one of 1 t stays and the one of 100 t falls.
#[test]
fn bond_breaks_under_too_much_load() {
    let (mut w, i) = world();
    let top = plate(&mut w, i, 1e3, false);
    bond::link(&mut w, top, GROUND, 1e9);
    let (light, heavy) = (plate(&mut w, i, 1e3, false), plate(&mut w, i, 1e5, false));
    // Each hangs on the anchored part by a bond that holds 10 t weight.
    bond::link(&mut w, light, top as u32, 1e5);
    bond::link(&mut w, heavy, top as u32, 1e5);
    let books = Books::now(&w);
    bond::step(&mut w);
    let p = &w.parts.parts;
    assert!(p[light].links.iter().any(|l| l.0 == top as u32), "the light part fell");
    assert!(p[heavy].links.is_empty() && p[heavy].root as usize == heavy, "the heavy part still hangs");
    books.assert_within(&w, 1e-12, 1e-9);
}

/// Walls shelter a body's heat: a resting animal inside a closed structure of thick walls (which conduct less than the
/// body's surface) ends a month warmer over the ground than the same body outside.
#[test]
fn walls_keep_a_body_warm() {
    let (mut w, i) = world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    // Six blocks joined around a small space, one bonded to the ground.
    let parts: Vec<usize> = (0..6).map(|_| plate(&mut w, i, 2e3, true)).collect();
    bond::link(&mut w, parts[0], GROUND, 1e9);
    for p in &parts[1..] {
        bond::link(&mut w, *p, parts[0] as u32, 1e9);
    }
    let r = w.parts.parts[parts[0]].root as usize;
    assert!(w.parts.parts[r].s.encl > 0.99, "not closed: {:?}", w.parts.parts[r].s);
    // The same body outside and inside: one that does not capture light (its upkeep warms it), from the warmest place.
    let bs = &w.life.bodies;
    let slot = (0..bs.len()).filter(|&s| bs[s].alive && pcore::body::light_cat(&w.chem, &bs[s].genes) < 5.0).max_by(|&a, &b| bs[a].temp(&w.chem).total_cmp(&bs[b].temp(&w.chem))).expect("a body");
    let (from, g) = (w.life.bodies[slot].cell as usize, w.life.bodies[slot].group);
    w.life.relocate(slot, g, from, i);
    w.life.bodies[slot].cell = i as u32;
    // Enough liquid for a month without rain.
    let b = &mut w.life.bodies[slot];
    let k = (0..b.store.len()).max_by(|&x, &y| b.store[x].total_cmp(&b.store[y])).unwrap();
    // It rests (its own adjustment of the action scores), so it stays where it is put.
    b.adj.fill(-3.0);
    b.adj[pcore::brain::REST] = 3.0;
    let t = b.temp(&w.chem);
    b.e += 0.3 * b.store[k] * w.chem.subs[k].cp * t;
    b.store[k] *= 1.3;
    // (It does not grow: a warm body builds faster, at the pace of its temperature, and would burn the fuel of its month.)
    w.cfg.phys.c_build = 0.0;
    let mut inside = w.clone();
    inside.life.bodies[slot].inside = inside.parts.parts[parts[0]].id(parts[0]);
    let temp = |w: &pcore::World| w.life.bodies[slot].temp(&w.chem) - w.life.env[i][0];
    pcore::step(&mut w);
    pcore::step(&mut inside);
    assert!(w.life.bodies[slot].alive && inside.life.bodies[slot].alive);
    assert!(temp(&inside) > temp(&w) + 1.0, "inside {} K, outside {} K above the ground", temp(&inside), temp(&w));
}

/// A small body can make parts (a part is a share of the body's mass, at least `Physics::min_kg`), and a cell holds
/// many of them.
#[test]
fn a_small_body_can_make_parts_and_a_cell_holds_many() {
    let (mut w, i) = world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    let slot = (0..w.life.bodies.len()).find(|&s| w.life.bodies[s].alive && w.life.bodies[s].n() == 1.0).expect("an eater");
    let mut b = std::mem::take(&mut w.life.bodies[slot]);
    b.cell = i as u32;
    for _ in 0..8 {
        // Fed again each time (its matter and heat are not booked here), so that it has solids above its minimums to
        // spare, and its liquid with them (a dry body's pores close, and it could not breathe to pay).
        let t = b.temp(&w.chem);
        (0..b.store.len()).filter(|&k| w.chem.phase(k, t) != Phase::Gas).for_each(|k| b.store[k] *= 2.0);
        b.e = b.heat_cap(&w.chem) * t;
        bond::form(&mut w, slot, &mut b, [0.0, 0.5]);
        b.hold = 0;
    }
    assert!(w.parts.at[i].len() > 6, "{} parts in the cell", w.parts.at[i].len());
}

/// Forming pays first, and paying may burn the very solid put out: what goes out is cut to what is left, so no store
/// goes below 0 and matter and energy stay exact (a costly form by a body whose only spare solid is its fuel, which it
/// burns fast).
#[test]
fn forming_never_takes_more_than_is_left() {
    let (mut w, _) = world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    let slot = (0..w.life.bodies.len()).find(|&s| w.life.bodies[s].alive && w.life.bodies[s].n() == 1.0).expect("an eater");
    let (chem, b) = (&w.chem, &mut w.life.bodies[slot]);
    b.prepare(chem, &w.cfg.phys);
    let t = b.temp(chem);
    let solid = |k: usize| chem.phase(k, t) == Phase::Solid;
    let k = b.k.burn.iter().flat_map(|&(r, _)| chem.reactions[r].inputs.iter().map(|p| p.0 as usize)).find(|&k| solid(k)).expect("a solid fuel");
    (0..b.store.len()).filter(|&x| solid(x) && x != k).for_each(|x| b.store[x] = 0.0);
    b.genes[pcore::body::Layout::of(chem).min + k] = 0.0;
    b.store[k] = 0.05 * b.mass(chem) / chem.subs[k].molar_mass;
    b.e = b.heat_cap(chem) * t;
    b.k.cat.iter_mut().for_each(|c| *c *= 1e6);
    // Half the fuel is to go out, and paying for it burns nearly all of it.
    let got = 0.5 * b.store[k] * chem.subs[k].molar_mass;
    let r = pcore::body::release(&w, &w.life.bodies[slot], w.life.bodies[slot].interval() as f64);
    (w.cfg.phys.min_kg, w.cfg.phys.c_form) = (0.5 * got, 0.99 * r / got);
    let books = Books::now(&w);
    let mut b = std::mem::take(&mut w.life.bodies[slot]);
    bond::form(&mut w, slot, &mut b, [0.0, 0.5]);
    assert!(b.store.iter().all(|&a| a >= 0.0), "{:?}", b.store);
    w.life.bodies[slot] = b;
    assert!(w.parts.alive().count() == 1 && pcore::check::broken(&w).is_none());
    books.assert_within(&w, 1e-12, 1e-9);
}

/// A node whose brain draws an action of Bond splits one member off as an individual of its lineage, with its share of
/// the stores, to act. One that holds a part after (its brain only holds parts, and its force bears the part's weight)
/// lives on; one that holds nothing (too weak to bear the part, or its brain only breaks the ground) goes back into the
/// node at once, which is then as it was: its members, genes and the spread of its born rows.
#[test]
fn a_node_acts_on_bonds_through_one_member() {
    // 0: holds a part of 5 kg, members of 40 kg at strength about 1 (they bear about 200 N, the part weighs 49 N); 1: the
    // same with the light-eater's own members (1 kg at strength about 0.05: they bear about 0.25 N); 2: breaks the ground.
    let scene = |case: u8| {
        let hold = case < 2;
        let mut w = small_world();
        // (No member dies of age here, so the node's count is exact.)
        w.cfg.phys.e_life = 1e30;
        let books = Books::now(&w);
        let i = dry_land(&w);
        // A part of 5 kg of the cell's most plentiful loose substance.
        let p = plate(&mut w, i, 5.0, false);
        let brain = if hold { Brain::verb(brain::VERBS) } else { Brain::only(brain::DIG) };
        let line = Line::new(&mut w, &brain, 0);
        let (l, mut b) = (line.lineage, line.founder(&mut w, 0, i));
        (b.spread, b.row_spread) = (vec![0.01; b.genes.len()], 0.01);
        // (No liquid soaked up from the ground, which would add a third to their mass.)
        b.genes[LIQ] = -4.0;
        if case == 0 {
            // (Mature at 100 kg, so they do not copy instead.)
            (b.genes[STRENGTH], b.genes[COPY_MASS], b.count) = (1.0, 2.0, b.count / 40.0);
        }
        let (n0, genes) = (b.count, b.genes.clone());
        w.life.add(b, w.day, 1);
        pcore::survey::survey(&mut w);
        w.day += 1;
        pcore::survey::day(&mut w);
        assert_eq!(w.counts.splits[1], 1.0, "no member split off");
        let one: Vec<&Body> = w.life.alive().filter(|b| b.lineage == l && b.count == 1.0).collect();
        let node = w.life.alive().find(|b| b.lineage == l && b.count > 1.0).expect("the node");
        if case == 0 {
            assert!(one.len() == 1 && w.parts.parts[p].id(p) == one[0].hold, "the member holds nothing");
            // (Its share of the stores: both lost what their leaves give off in the update.)
            let kg = node.mass(&w.chem) / node.count;
            assert!((node.count + 1.0 - n0).abs() < 1e-9 * n0 && (one[0].mass(&w.chem) / kg - 1.0).abs() < 0.1, "{} of {n0} members, {} kg against {kg}", node.count, one[0].mass(&w.chem));
        } else {
            assert!(one.is_empty() && (node.count - n0).abs() < 1e-9 * n0, "{} members", node.count);
            assert!(node.genes.iter().zip(&genes).all(|(a, b)| (a - b).abs() < 1e-4), "genes changed");
            assert_eq!(node.row_spread, 0.01, "the spread of the born rows changed");
            assert!(w.parts.parts[p].held == pcore::bond::NOBODY, "the part is held");
        }
        books.assert_kept(&w, 1e-9);
    };
    scene(0);
    scene(1);
    scene(2);
}
