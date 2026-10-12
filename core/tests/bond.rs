//! Two tiny scenes for Law 3 (Bond).

use pcore::bond::{self, GROUND};
use pcore::check::{drift, totals};

/// A land cell of a small world after one month (so the laws have their surroundings).
fn world() -> (pcore::World, usize) {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let i = (0..w.grid.n()).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 0.01).expect("land");
    (w, i)
}

/// A part of `kg` of the cell's most plentiful loose substance, taken from the ground at its
/// temperature: flat, or a block if `thick`.
fn plate(w: &mut pcore::World, i: usize, kg: f64, thick: bool) -> usize {
    let n = w.grid.n();
    let k = (0..w.chem.ns()).max_by(|&a, &b| w.ground[a * n + i].total_cmp(&w.ground[b * n + i])).unwrap();
    let mut wall = vec![0.0; w.chem.ns()];
    wall[k] = kg / w.chem.subs[k].molar_mass;
    assert!(w.ground[k * n + i] > wall[k]);
    let e = wall[k] * w.chem.subs[k].cp * w.life.env[i][0];
    w.ground[k * n + i] -= wall[k];
    w.e_ground[i] -= e;
    bond::make(w, i, wall, e, if thick { 0.0 } else { -1.0 }, 0.0)
}

#[test]
fn bond_breaks_under_too_much_load() {
    let (mut w, i) = world();
    let top = plate(&mut w, i, 1e3, false);
    bond::link(&mut w, top, GROUND, 1e9);
    let (light, heavy) = (plate(&mut w, i, 1e3, false), plate(&mut w, i, 1e5, false));
    // Each hangs on the anchored part by a bond that holds 10 t weight.
    bond::link(&mut w, light, top as u32, 1e5);
    bond::link(&mut w, heavy, top as u32, 1e5);
    let (t0, l0) = (totals(&w), w.ledger.clone());
    bond::step(&mut w);
    let p = &w.parts.parts;
    assert!(p[light].links.iter().any(|l| l.0 == top as u32), "the light part fell");
    assert!(p[heavy].links.is_empty() && p[heavy].root as usize == heavy, "the heavy part still hangs");
    let (de, dq) = drift(&w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-9, "drift {de:e} {dq:e}");
}

#[test]
fn walls_keep_a_body_warm() {
    let (mut w, i) = world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    // Six blocks joined around a small space, one bonded to the ground: thick walls, which
    // conduct less than the body's surface.
    let parts: Vec<usize> = (0..6).map(|_| plate(&mut w, i, 2e3, true)).collect();
    bond::link(&mut w, parts[0], GROUND, 1e9);
    for p in &parts[1..] {
        bond::link(&mut w, *p, parts[0] as u32, 1e9);
    }
    let r = w.parts.parts[parts[0]].root as usize;
    assert!(w.parts.parts[r].s.encl > 0.99, "not closed: {:?}", w.parts.parts[r].s);
    // The same body outside and inside: one that does not capture light (its upkeep warms it),
    // from the warmest place.
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
    b.adj[0] = 3.0;
    let t = b.temp(&w.chem);
    b.e += 0.3 * b.store[k] * w.chem.subs[k].cp * t;
    b.store[k] *= 1.3;
    let mut inside = w.clone();
    inside.life.bodies[slot].inside = inside.parts.parts[parts[0]].id(parts[0]);
    let temp = |w: &pcore::World| w.life.bodies[slot].temp(&w.chem) - w.life.env[i][0];
    for _ in 0..1 {
        pcore::step(&mut w);
        pcore::step(&mut inside);
    }
    assert!(w.life.bodies[slot].alive && inside.life.bodies[slot].alive);
    assert!(temp(&inside) > temp(&w) + 1.0, "inside {} K, outside {} K above the ground", temp(&inside), temp(&w));
}
