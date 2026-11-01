//! Law 3 (Bond): parts and the bonds between them, structures and the shelter they give, and the bodies that make
//! and hold parts.

use pcore::bond::{self, GROUND};
use pcore::body::{die, Body, COPY_MASS, LACK, STRENGTH};
use pcore::brain;
use pcore::chem::Phase;
use pcore::life::Handle;
use pcore::units::DAY_S;
use pcore::store::{Class, Node};
use pcore::World;
use worldgen::testkit::{dry_land, moved_serial, part_of, plate, relocate, scale, set_amount, small_world, start as world, Books, Brain, Line, LIQUID, MINERAL};

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
    let (p, bonds) = (&w.parts.parts, &w.parts.bonds);
    assert!(bonds.of(light).any(|l| l.0 == top as u32), "the light part fell");
    assert!(bonds.of(heavy).next().is_none() && p[heavy].root as usize == heavy, "the heavy part still hangs");
    books.assert_within(&w, 1e-12, 1e-9);
}

/// Walls shelter a body's heat: a resting animal inside a closed structure of thick walls (which conduct less than the
/// body's surface) ends a month warmer over the ground than the same body outside.
#[test]
fn walls_keep_a_body_warm() {
    let (mut w, i) = world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    // The same body outside and inside: one that does not capture light (its upkeep warms it), from the warmest place
    // among the cells next to cell `i` (a body steps to a neighbour).
    let bs = &w.life;
    let next = |s: Handle| bs[s].cell as usize == i || w.mesh.cells().link(bs[s].cell as usize, i).is_some();
    let slot = bs.living().map(|x| x.0).filter(|&s| next(s)).max_by(|&a, &b| bs[a].temp(&w.chem).total_cmp(&bs[b].temp(&w.chem))).expect("a body next to the cell");
    let slot = if w.life[slot].cell as usize == i { slot } else { relocate(&mut w, slot, i) };
    // Six blocks joined around a small space, one bonded to the ground.
    let parts: Vec<usize> = (0..6).map(|_| plate(&mut w, i, 1e3, true)).collect();
    bond::link(&mut w, parts[0], GROUND, 1e9);
    for p in &parts[1..] {
        bond::link(&mut w, *p, parts[0] as u32, 1e9);
    }
    let r = w.parts.parts[parts[0]].root as usize;
    assert!(w.parts.parts[r].s.encl.closed > 0.99, "not closed: {:?}", w.parts.parts[r].s);
    // Enough liquid for a month without rain.
    // It rests (the biases of its brain's action rows: resting up, every other action down), so it stays where it is put.
    let shape = w.life.brains[w.life[slot].lineage as usize].as_ref().expect("a brain").shape;
    let b = &mut w.life[slot];
    let k = (0..b.store.len()).max_by(|&x, &y| b.store[x].total_cmp(&b.store[y])).unwrap();
    let rows = (0..pcore::brain::FIXED).chain([shape.dir()]).chain((0..pcore::brain::VERBS + pcore::brain::OVERBS).map(|v| shape.verb(v)));
    for q in rows {
        b.rows[q * shape.r() + shape.h] += if q == pcore::brain::REST { 3.0 } else { -3.0 };
    }
    // (At the body's temperature, with the fusion enthalpy a molten store holds: `Body::energy_at`.)
    let t = b.temp(&w.chem);
    b.store[k] *= 1.3;
    b.e = b.energy_at(&w.chem, t);
    // (It does not grow: a warm body builds faster, at the pace of its temperature, and would burn the fuel of its month.)
    w.cfg.phys.c_build = 0.0;
    let mut inside = w.clone();
    inside.life[slot].inside = Some(bond::id(&inside, parts[0]));
    let temp = |w: &pcore::World| w.life[slot].temp(&w.chem) - w.life.env[i][0];
    pcore::step(&mut w);
    pcore::step(&mut inside);
    assert!(w.life[slot].alive && inside.life[slot].alive);
    println!("inside {:.2} K, outside {:.2} K above the ground", temp(&inside), temp(&w));
    assert!(temp(&inside) > temp(&w) + 1.0, "inside {} K, outside {} K above the ground", temp(&inside), temp(&w));
}

/// A small body can make parts (a part is a share of the body's mass, at least `Physics::min_kg`), and a cell holds
/// many of them.
#[test]
fn a_small_body_can_make_parts_and_a_cell_holds_many() {
    let (mut w, i) = world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    let slot = w.life.living().map(|x| x.0).find(|&s| w.life[s].n() == 1.0).expect("an eater");
    let mut b = std::mem::take(&mut w.life[slot]);
    b.cell = i as u32;
    let fed: Vec<f64> = b.store.iter().map(|a| 2.0 * a).collect();
    for _ in 0..8 {
        // Fed again each time to twice its starting stores (its matter and heat are not booked here), so that it has
        // solids above its minimums to spare, and its liquid with them (a dry body's pores close, and it could not
        // breathe to pay).
        let t = b.temp(&w.chem);
        (0..b.store.len()).filter(|&k| w.chem.phase(k, t) != Phase::Gas).for_each(|k| b.store[k] = b.store[k].max(fed[k]));
        b.e = b.energy_at(&w.chem, t);
        moved_serial(&mut w, (b.cell as usize, slot), |w| bond::form(w, &mut b, [0.0, 0.5]));
        b.hold = None;
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
    let slot = w.life.living().map(|x| x.0).find(|&s| w.life[s].n() == 1.0).expect("an eater");
    let (chem, b) = (&w.chem, &mut w.life[slot]);
    b.prepare(chem, &w.cfg);
    let t = b.temp(chem);
    let solid = |k: usize| chem.phase(k, t) == Phase::Solid;
    let k = b.k.burn.iter().flat_map(|&(r, _)| chem.reactions[r].inputs.iter().map(|p| p.0 as usize)).find(|&k| solid(k)).expect("a solid fuel");
    (0..b.store.len()).filter(|&x| solid(x) && x != k).for_each(|x| b.store[x] = 0.0);
    b.genes[pcore::body::Layout::of(chem).min + k] = 0.0;
    b.store[k] = 0.05 * b.mass(chem) / chem.subs[k].molar_mass;
    b.e = b.energy_at(chem, t);
    // Half the fuel is to go out (`Physics::form` of its mass), and paying for it burns nearly all of it.
    let got = 0.5 * b.store[k] * chem.subs[k].molar_mass;
    w.cfg.phys.form = got / b.mass(chem);
    let r = worldgen::testkit::release(&w, &w.life[slot]);
    (w.cfg.phys.min_kg, w.cfg.phys.c_form) = (0.5 * got, 0.99 * r / got);
    let books = Books::now(&w);
    let mut b = std::mem::take(&mut w.life[slot]);
    moved_serial(&mut w, (b.cell as usize, slot), |w| bond::form(w, &mut b, [0.0, 0.5]));
    assert!(b.store.iter().all(|&a| a >= 0.0), "{:?}", b.store);
    w.life[slot] = b;
    assert!(w.parts.alive().count() == 1 && pcore::check::broken(&w).is_none());
    books.assert_within(&w, 1e-12, 1e-9);
}

/// The members of a node who do an action of Bond (all of them, when its brain gives it every chance) act together, as
/// one body of so many members (`choose::spread`). Strong enough together (its brain only holds parts, and their force
/// bears the part's weight), they hold the part and live on holding it; else (too weak to bear it, or their brain only
/// breaks the ground) the node holds nothing and is as it was: its members, genes and the spread of its born rows.
#[test]
fn a_node_acts_on_bonds_as_its_members_together() {
    // 0: holds a part of 5 kg, members at strength about 1 (they bear about 200 N, the part weighs 49 N); 1: the same at
    // strength 0 (they bear nothing); 2: breaks the ground.
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
        let (l, b) = (line.lineage, line.founder(&mut w, 1, i));
        let mut b = worldgen::testkit::node_of(&mut w, b, 800.0);
        (b.spread, b.row_spread) = (vec![0.01; b.genes.len()], 0.01);
        // (Mature past their copy mass, so the matter that rots is not starving, and putting nothing into a brood, so they
        // do not copy.)
        (b.genes[COPY_MASS], b.genes[pcore::body::ALLOC]) = (b.genes[COPY_MASS] - 1.0, 0.0);
        if case == 0 {
            (b.genes[STRENGTH], b.genes[COPY_MASS], b.count) = (1.0, 1.0, b.count / 40.0);
        } else if case == 1 {
            b.genes[STRENGTH] = 0.0;
        }
        // (Twenty members of the same size: in case 1 too weak together to bear the part.)
        let few = 20f64.min(b.count) / b.count;
        scale(&mut w, &mut b, few);
        b.count *= few;
        let (n0, genes) = (b.count, b.genes.clone());
        w.add(b);
        pcore::survey::survey(&mut w, false);
        w.time += w.cfg.days_per_month as u64 * DAY_S;
        pcore::survey::batch(&mut w, false);
        let all: Vec<&Body> = w.life.alive().filter(|b| b.lineage == l).collect();
        let members: f64 = all.iter().map(|b| b.count).sum();
        assert!((members - n0).abs() < 1e-9 * n0, "case {case}: {members} members of {n0}");
        if case == 0 {
            assert!(all.iter().any(|b| Some(bond::id(&w, p)) == b.hold), "nothing holds the part");
        } else {
            let node = all.iter().find(|b| b.count > 1.0).expect("the node");
            assert!(all.iter().all(|b| b.hold.is_none()), "case {case}: a body holds a part");
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

/// A part whose walls fall under `min_kg` falls apart, and all it still holds goes to its cell's ground: its walls, the
/// liquid in its hollow and its heat. So in its monthly turn, and when a body takes from it (what the body took aside).
#[test]
fn a_part_that_falls_apart_gives_all_it_holds_to_its_ground() {
    // A hollow part of 5 t of the mineral holding 100 kg of the liquid, in cell `i`; walls under `min_kg` from now on.
    let new_part = |w: &mut World, i: usize| {
        let (ns, n) = (w.chem.ns(), w.mesh.cells().n());
        let (mut wall, mut fill) = (vec![0.0; ns], vec![0.0; ns]);
        (wall[MINERAL], fill[LIQUID]) = (5e3 / w.chem.subs[MINERAL].molar_mass, 1e2 / w.chem.subs[LIQUID].molar_mass);
        for k in [MINERAL, LIQUID] {
            let a = w.store.amounts(Class::Ground)[k * n + i];
            set_amount(w, Node::ground(i), k, a + 2.0 * (wall[k] + fill[k]));
        }
        w.cfg.phys.min_kg = 1e9;
        part_of(w, i, wall, fill, 0.0, 0.5)
    };
    // What the ground of cell `i` holds: of each substance, mol, and its heat, J.
    let ground = |w: &World, i: usize| {
        let n = w.mesh.cells().n();
        ((0..w.chem.ns()).map(|k| w.store.amounts(Class::Ground)[k * n + i]).collect::<Vec<f64>>(), w.store.heat(Class::Ground)[i])
    };
    // The ground gained `gain` mol of each substance and `heat` J, to rounding.
    let gained = |w: &World, i: usize, before: &(Vec<f64>, f64), gain: &[f64], heat: f64| {
        let now = ground(w, i);
        for (k, &g) in gain.iter().enumerate() {
            let d = now.0[k] - before.0[k];
            assert!((d - g).abs() <= 1e-12 * g.abs() + 1e-14 * before.0[k], "substance {k}: the ground gained {d} mol of {g}");
        }
        let d = now.1 - before.1;
        assert!((d - heat).abs() <= 1e-9 * heat.abs() + 1e-14 * before.1.abs(), "the ground gained {d} J of {heat}");
    };
    let holds = |w: &World, p: usize| {
        let pt = &w.parts.parts[p];
        (pt.wall.iter().zip(&pt.fill).map(|(a, f)| a + f).collect::<Vec<f64>>(), pt.e)
    };

    // Its monthly turn (in a month without reactions with the air).
    let (mut w, i) = world();
    let month = w.cfg.calendar().month;
    if w.month() % 6 == 0 {
        w.time += month;
    }
    let p = new_part(&mut w, i);
    let (books, before, (held, e)) = (Books::now(&w), ground(&w, i), holds(&w, p));
    bond::step(&mut w);
    assert!(!w.parts.parts[p].alive, "the part is still there");
    gained(&w, i, &before, &held, e);
    books.assert_within(&w, 1e-12, 1e-9);

    // A take: a body takes some of the mineral (it takes nothing else, and pays nothing). A world with the body (taken
    // out of its slot), its cell and a part in it.
    let take_scene = || {
        let (mut w, _) = world();
        worldgen::life::seed(&mut w, 0);
        worldgen::life::seed(&mut w, 1);
        let h = w.life.living().map(|x| x.0).find(|&s| w.life[s].n() == 1.0).expect("a body");
        let i = w.life[h].cell as usize;
        let p = new_part(&mut w, i);
        let mut b = std::mem::take(&mut w.life[h]);
        let up = pcore::body::Layout::of(&w.chem).upt;
        (0..w.chem.ns()).for_each(|k| b.genes[up + k] = if k == MINERAL { 8.0 } else { -8.0 });
        (w.cfg.phys.c_handle, w.cfg.phys.take) = (0.0, 1e-3 * w.cfg.phys.take);
        (w, h, i, p, b)
    };
    let (mut w, h, i, p, mut b) = take_scene();
    let (books, before, (held, e), (store, eb)) = (Books::now(&w), ground(&w, i), holds(&w, p), (b.store.clone(), b.e));
    moved_serial(&mut w, (i, h), |w| bond::act(w, &mut b, 1, p));
    assert!(!w.parts.parts[p].alive, "the part is still there");
    let took: Vec<f64> = b.store.iter().zip(&store).map(|(a, s)| a - s).collect();
    assert!(took[MINERAL] > 0.0 && took[MINERAL] < 0.5 * held[MINERAL], "it took {} mol of {}", took[MINERAL], held[MINERAL]);
    let gain: Vec<f64> = held.iter().zip(&took).map(|(a, t)| a - t).collect();
    gained(&w, i, &before, &gain, e - (b.e - eb));
    w.life[h] = b;
    books.assert_within(&w, 1e-12, 1e-9);

    // The body dies in the same update, on the view the fall opened again: the books are kept.
    let (mut w, h, i, p, mut b) = take_scene();
    let (books, id) = (Books::now(&w), b.id);
    moved_serial(&mut w, (i, h), |w| {
        bond::act(w, &mut b, 1, p);
        w.in_domain(i, |w, d| die(w, d, h, b, LACK));
    });
    assert!(!w.parts.parts[p].alive && w.life.find(id).is_none(), "the part or the body is still there");
    books.assert_within(&w, 1e-12, 1e-9);
}
