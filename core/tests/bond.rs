//! Law 3 (Bond): parts and the bonds between them, structures and the shelter they give, and the bodies that make
//! and hold parts.

use pcore::bond;
use pcore::body::{die, Body, COOL_T, COPY_MASS, HEAT, LACK, STRENGTH};
use pcore::brain;
use pcore::chem::Phase;
use pcore::life::{Entry, Handle};
use pcore::units::DAY_S;
use pcore::store::{Class, Node};
use pcore::World;
use worldgen::testkit::{bite, dry_land, eater, node_of, part_of, place, plate, relocate, scale, set_amount, small_world, start as world, stores, Books, Brain, Line, FRAME, LIQUID, MATTER, MINERAL, ROCK};

/// A bond breaks when the weight it bears is more than it holds: of two parts hung on a held one by bonds that hold
/// 10 t weight, the one of 1 t stays and the one of 100 t falls.
#[test]
fn bond_breaks_under_too_much_load() {
    let (mut w, i) = world();
    let top = plate(&mut w, i, 1e3, false);
    let mut b = eater(&mut w, i, brain::REST, 1.0);
    b.hold = Some(top);
    let h = w.add(b);
    let id = w.life[h].id;
    w.life.part_mut(top).expect("the part").held = id;
    let (light, heavy) = (plate(&mut w, i, 1e3, false), plate(&mut w, i, 1e5, false));
    // Each hangs on the held part by a bond that holds 10 t weight.
    bond::link(&mut w, light, top, 1e5);
    bond::link(&mut w, heavy, top, 1e5);
    let books = Books::now(&w);
    bond::step(&mut w);
    let bonds = &w.life.cell_at(i).bonds;
    assert!(bonds.of(light).any(|l| l.0 == top), "the light part fell");
    assert!(bonds.of(heavy).next().is_none() && w.life.part(heavy).expect("the heavy part").root == heavy, "the heavy part still hangs");
    books.assert_within(&w, 1e-9, 1e-9);
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
    // (A lineage of its own, the same brain: it joins no body of its kind in cell `i` at the month's merge.)
    let own = w.life.brains[w.life[slot].lineage as usize].clone().expect("a brain");
    w.life[slot].lineage = w.life.new_lineage(own);
    // Six blocks joined around a small space.
    let parts: Vec<u64> = (0..6).map(|_| plate(&mut w, i, 1e3, true)).collect();
    for p in &parts[1..] {
        bond::link(&mut w, *p, parts[0], 1e9);
    }
    let r = w.life.part(parts[0]).expect("a part").root;
    let s = w.life.part(r).expect("the root").s;
    assert!(s.encl.closed > 0.99, "not closed: {s:?}");
    // Enough liquid for a month without rain.
    // It rests (the biases of its brain's action rows: resting up, every other action down), so it stays where it is put.
    let shape = w.life.brains[w.life[slot].lineage as usize].as_ref().expect("a brain").shape;
    let rows = (0..pcore::brain::FIXED).chain([shape.dir()]).chain((0..pcore::brain::VERBS + pcore::brain::OVERBS).map(|v| shape.verb(v)));
    for q in rows {
        w.life.rows_mut(slot)[q * shape.r() + shape.h] += if q == pcore::brain::REST { 3.0 } else { -3.0 };
    }
    let b = &mut w.life[slot];
    let k = (0..w.chem.ns()).max_by(|&x, &y| b.amount(w.chem.body(), x).total_cmp(&b.amount(w.chem.body(), y))).unwrap();
    // (At the body's temperature, with the fusion enthalpy its key liquids hold: `Body::energy_at`.)
    let t = b.temp(&w.chem);
    *b.store_of(&w.chem, k) *= 1.3;
    b.e = b.energy_at(&w.chem, t);
    // (A bare body that never opens a coat or sweats: where its own heat holds it is the walls' to say, not its genes'.)
    (b.genes[HEAT], b.genes[COOL_T]) = (0.0, 400.0);
    b.prepare(&w.chem, &w.cfg);
    // (It does not grow: a warm body builds faster, at the pace of its temperature, and would burn the fuel of its month.)
    w.cfg.phys.c_build = 0.0;
    // (Its update integrates a month, as in a run, not the day since it was placed: the walls slow how fast its heat
    // follows the ground, so a day shows only part of the warmth they keep.)
    w.life[slot].last -= w.cfg.days_per_month as f64;
    let mut inside = w.clone();
    inside.life[slot].inside = Some(parts[0]);
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
    let fed = b.store.map(|a| 2.0 * a);
    for _ in 0..8 {
        // Fed again each time to twice its starting stores (its matter and heat are not booked here), so that it has
        // solids above its minimums to spare, and its liquid with them (a dry body's pores close, and it could not
        // breathe to pay).
        let t = b.temp(&w.chem);
        let subs = w.chem.body().subs();
        (0..subs.len()).filter(|&s| w.chem.phase(subs[s], t) != Phase::Gas).for_each(|s| b.store[s] = b.store[s].max(fed[s]));
        b.e = b.energy_at(&w.chem, t);
        w.in_cell(i, |w, d| bond::form(w, d, &mut b, [0.0, 0.5, 0.0]));
        b.hold = None;
    }
    assert!(w.life.cell_at(i).parts.len() > 6, "{} parts in the cell", w.life.cell_at(i).parts.len());
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
    chem.body().subs().iter().copied().filter(|&x| solid(x) && x != k).for_each(|x| *b.store_of(chem, x) = 0.0);
    b.genes[pcore::body::Layout::of(chem).min + chem.body().place(k).expect("a store")] = 0.0;
    *b.store_of(&w.chem, k) = 0.05 * b.mass(chem) / chem.subs[k].molar_mass;
    b.e = b.energy_at(chem, t);
    // Half the fuel is to go out (`Physics::form` of its mass), and paying for it burns nearly all of it.
    let got = 0.5 * b.amount(w.chem.body(), k) * chem.subs[k].molar_mass;
    w.cfg.phys.form = got / b.mass(chem);
    let r = worldgen::testkit::release(&w, &w.life[slot]);
    (w.cfg.phys.min_kg, w.cfg.phys.c_form) = (0.5 * got, 0.99 * r / got);
    let books = Books::now(&w);
    let mut b = std::mem::take(&mut w.life[slot]);
    w.in_cell(b.cell as usize, |w, d| bond::form(w, d, &mut b, [0.0, 0.5, 0.0]));
    assert!(b.store.iter().all(|&a| a >= 0.0), "{:?}", b.store);
    w.life[slot] = b;
    assert!(w.life.parts().count() == 1 && pcore::check::broken(&w).is_none());
    books.assert_within(&w, 1e-9, 1e-9);
}

/// What lies alone on the ground is buried (laws.md 1.3): a lone flat plate of rock sinks `bury` m/s into the loose
/// ground and, buried past its smallest size, joins its cell's ground, its walls and its heat; two plates joined by a
/// bond stay on top. The books close.
#[test]
fn a_lone_part_is_buried_in_the_ground() {
    let (mut w, i) = world();
    let ns = w.chem.ns();
    let rock = |w: &mut World| {
        let mut wall = vec![0.0; ns];
        wall[ROCK] = 1.0 / w.chem.subs[ROCK].molar_mass;
        part_of(w, i, wall, vec![0.0; ns], -1.0, 0.0)
    };
    let (lone, a, b) = (rock(&mut w), rock(&mut w), rock(&mut w));
    bond::link(&mut w, a, b, 1e9);
    let size = w.life.part(lone).expect("the plate").size[2] as f64;
    let months = (size / (w.cfg.bury * w.cfg.calendar().month as f64)).ceil() as usize;
    let books = Books::now(&w);
    for m in 0..months {
        assert!(w.life.part(lone).is_some(), "buried after {m} of {months} months");
        bond::step(&mut w);
    }
    assert!(w.life.part(lone).is_none(), "the plate is on the ground after {months} months");
    assert!(w.life.part(a).is_some() && w.life.part(b).is_some(), "the joined plates were buried");
    books.assert_kept(&w, 1e-9);
}

/// A body may make a part of what lies on its ground: the share of its walls its brain asks for is of the ground's loose
/// solids in their proportions, with their heat, never more than the ground holds. On land that is mostly rock, harder
/// than any body, so the part is an edge that cuts a body like its maker. The books close.
#[test]
fn a_body_picks_up_its_walls_from_the_ground() {
    // A starting eater forms a flat part wholly of the ground (`all`: as much as it likes, for free): the world, the
    // eater, the ground's loose solids before (mol), the eater's mass then and the part.
    let scene = |all: bool| {
        let (mut w, i) = world();
        if all {
            (w.cfg.phys.form, w.cfg.phys.c_form, w.cfg.phys.c_handle) = (1e30, 0.0, 0.0);
        }
        let (n, ns, t) = (w.mesh.cells().n(), w.chem.ns(), w.t_ground_last(i));
        let loose: Vec<f64> = (0..ns).map(|k| if w.chem.phase(k, t) == Phase::Solid { w.store.amounts(Class::Ground)[k * n + i] } else { 0.0 }).collect();
        // (Strong enough to hold the part it makes.)
        let mut b = eater(&mut w, i, brain::REST, 1.0);
        b.genes[STRENGTH] = 0.5;
        let s = place(&mut w, b);
        let (books, mut b) = (Books::now(&w), std::mem::take(&mut w.life[s]));
        let mb = b.mass(&w.chem);
        w.in_cell(i, |w, d| bond::form(w, d, &mut b, [-1.0, 0.0, 1.0]));
        w.life[s] = b;
        books.assert_kept(&w, 1e-9);
        let p = w.life.cell_at(i).parts.last().expect("a part").clone();
        (w, s, loose, mb, p)
    };
    let (w, s, loose, mb, p) = scene(false);
    let lying: Vec<usize> = (0..loose.len()).filter(|&k| loose[k] > 0.0).collect();
    let share = |k: usize| p.wall[k] / loose[k];
    assert!((0..loose.len()).all(|k| p.wall[k] == 0.0 || loose[k] > 0.0), "the walls hold what does not lie on the ground");
    assert!(lying.iter().all(|&k| (share(k) / share(lying[0]) - 1.0).abs() < 1e-9), "the walls are not in the ground's proportions");
    let kg = p.walls(&w.chem).mass;
    assert!((kg / (w.cfg.phys.form * mb) - 1.0).abs() < 1e-9, "a part of {kg} kg from a body of {mb}");
    let i = w.life[s].cell as usize;
    let edge = bond::edge(w.life.cell_at(i), &w, &w.life[s]).expect("it holds an edge");
    assert!(p.walls(&w.chem).hardness > 0.2 && w.life[s].cut_by(&w.chem, &w.cfg, &edge) < 1.0, "the ground's part does not cut a body");
    // Asking for more than lies there takes all of it, and no more.
    let (w, s, loose, _, p) = scene(true);
    let (n, i) = (w.mesh.cells().n(), w.life[s].cell as usize);
    for k in (0..loose.len()).filter(|&k| loose[k] > 0.0) {
        assert!((p.wall[k] / loose[k] - 1.0).abs() < 1e-9 && w.store.amounts(Class::Ground)[k * n + i].abs() < 1e-9 * loose[k], "substance {k}: {} of {} mol", p.wall[k], loose[k]);
    }
}

/// The members of a node who do an action of Bond (all of them, when its brain gives it every chance) act together, as
/// one body of so many members (`choose::spread`). Strong enough together (its brain only holds parts, and their force
/// bears the part's weight), they hold the part and live on holding it; else (too weak to bear it) the node holds
/// nothing and is as it was: its members, genes and the spread of its born rows.
#[test]
fn a_node_acts_on_bonds_as_its_members_together() {
    // 0: holds a part of 5 kg, members at strength about 1 (they bear about 200 N, the part weighs 49 N); 1: the same at
    // strength 0 (they bear nothing).
    let scene = |case: u8| {
        let mut w = small_world();
        // (No member dies of age here, so the node's count is exact.)
        w.cfg.phys.e_life = 1e30;
        let books = Books::now(&w);
        let i = dry_land(&w);
        // A part of 5 kg of the cell's most plentiful loose substance.
        let p = plate(&mut w, i, 5.0, false);
        let line = Line::new(&mut w, &Brain::verb(brain::VERBS), 0);
        let (l, b) = (line.lineage, line.founder(&mut w, 1, i));
        let mut b = worldgen::testkit::node_of(&mut w, b, 800.0);
        (b.spread, b.row_spread) = (pcore::body::Genes::filled(0.01, b.genes.len()), 0.01);
        // (Mature past their copy mass, so the matter that rots is not starving, and putting nothing into a brood, so they
        // do not copy.)
        (b.genes[COPY_MASS], b.genes[pcore::body::ALLOC]) = (b.genes[COPY_MASS] - 1.0, 0.0);
        // (Bare, its own heat holds it near its ground, at a slow pace: it pays its month from the fuel it was placed with.)
        b.genes[HEAT] = 0.0;
        if case == 0 {
            (b.genes[STRENGTH], b.genes[COPY_MASS], b.count) = (1.0, 1.0, b.count / 40.0);
        } else {
            b.genes[STRENGTH] = 0.0;
        }
        // (Twenty members of the same size: in case 1 too weak together to bear the part.)
        let few = 20f64.min(b.count) / b.count;
        scale(&mut w, &mut b, few);
        b.count *= few;
        let (n0, genes) = (b.count, b.genes);
        w.add(b);
        pcore::survey::survey(&mut w);
        w.time += w.cfg.days_per_month as u64 * DAY_S;
        pcore::survey::batch(&mut w);
        let all: Vec<&Body> = w.life.alive().filter(|b| b.lineage == l).collect();
        let members: f64 = all.iter().map(|b| b.count).sum();
        assert!((members - n0).abs() < 1e-9 * n0, "case {case}: {members} members of {n0}");
        if case == 0 {
            assert!(all.iter().any(|b| Some(p) == b.hold), "nothing holds the part");
        } else {
            let node = all.iter().find(|b| b.count > 1.0).expect("the node");
            assert!(all.iter().all(|b| b.hold.is_none()), "case {case}: a body holds a part");
            assert!(node.genes.iter().zip(&genes).all(|(a, b)| (a - b).abs() < 1e-4), "genes changed");
            assert_eq!(node.row_spread, 0.01, "the spread of the born rows changed");
            assert!(w.life.part(p).expect("the part").held == pcore::bond::NOBODY, "the part is held");
        }
        books.assert_kept(&w, 1e-9);
    };
    scene(0);
    scene(1);
}

/// A part whose walls fall under `min_kg` falls apart, and all it still holds goes to its cell's ground: its walls, the
/// liquid in its hollow and its heat, with what their fall from the cell's head frees of the potential energy
/// (`Store::lifted`). So in its monthly turn, and when a body takes from it (what the body took aside).
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
    let holds = |w: &World, p: u64| {
        let pt = w.life.part(p).expect("the part");
        (pt.wall.iter().zip(&pt.fill).map(|(a, f)| a + f).collect::<Vec<f64>>(), pt.e)
    };

    // Its monthly turn.
    let (mut w, i) = world();
    let p = new_part(&mut w, i);
    let (books, before, (held, e), lifted) = (Books::now(&w), ground(&w, i), holds(&w, p), w.store.lifted());
    bond::step(&mut w);
    assert!(w.life.part(p).is_none(), "the part is still there");
    gained(&w, i, &before, &held, e - (w.store.lifted() - lifted));
    books.assert_within(&w, 1e-9, 1e-9);

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
        w.chem.body().subs().iter().enumerate().for_each(|(s, &k)| b.genes[up + s] = if k == MINERAL { 8.0 } else { -8.0 });
        (w.cfg.phys.c_handle, w.cfg.phys.take) = (0.0, 1e-3 * w.cfg.phys.take);
        (w, h, i, p, b)
    };
    let (mut w, h, i, p, mut b) = take_scene();
    let (books, before, (held, e), (store, eb), lifted) = (Books::now(&w), ground(&w, i), holds(&w, p), (stores(&w, &b), b.e), w.store.lifted());
    act_on(&mut w, &mut b, p);
    assert!(w.life.part(p).is_none(), "the part is still there");
    let took: Vec<f64> = stores(&w, &b).iter().zip(&store).map(|(a, s)| a - s).collect();
    assert!(took[MINERAL] > 0.0 && took[MINERAL] < 0.5 * held[MINERAL], "it took {} mol of {}", took[MINERAL], held[MINERAL]);
    let gain: Vec<f64> = held.iter().zip(&took).map(|(a, t)| a - t).collect();
    gained(&w, i, &before, &gain, e - (b.e - eb) - (w.store.lifted() - lifted));
    w.life[h] = b;
    books.assert_within(&w, 1e-9, 1e-9);

    // The body dies after the fall: the books are kept.
    let (mut w, h, i, p, mut b) = take_scene();
    let (books, id) = (Books::now(&w), b.id);
    act_on(&mut w, &mut b, p);
    w.in_cell(i, |w, d| die(w, d, h, b, LACK));
    assert!(w.life.part(p).is_none() && w.life.find(id).is_none(), "the part or the body is still there");
    books.assert_within(&w, 1e-9, 1e-9);
}

/// A part rots: each month the share of its walls the open world's decomposers rot at its cell ground's temperature
/// and moisture goes to that ground, where Change rots it on; the books are kept. A part of the frame rots; one of rock,
/// which no decomposer takes, does not.
#[test]
fn a_part_rots_to_its_ground() {
    let (mut w, i) = world();
    let (ns, n) = (w.chem.ns(), w.mesh.cells().n());
    let walls = |k: usize, kg: f64, w: &mut World| {
        let mut wall = vec![0.0; ns];
        wall[k] = kg / w.chem.subs[k].molar_mass;
        let a = w.store.amounts(Class::Ground)[k * n + i];
        set_amount(w, Node::ground(i), k, a + 2.0 * wall[k]);
        part_of(w, i, wall, vec![0.0; ns], 0.0, 0.0)
    };
    let (frame, rock) = (walls(FRAME, 1e3, &mut w), walls(ROCK, 1e3, &mut w));
    let held = |w: &World, p: u64, k: usize| w.life.part(p).expect("the part").wall[k];
    let (books, f0, r0) = (Books::now(&w), held(&w, frame, FRAME), held(&w, rock, ROCK));
    bond::step(&mut w);
    let lost = 1.0 - held(&w, frame, FRAME) / f0;
    assert!(lost > 0.0 && lost < 1.0, "the frame lost {lost} of its walls in a month");
    assert_eq!(held(&w, rock, ROCK), r0, "the rock rotted");
    books.assert_within(&w, 1e-9, 1e-9);
}

/// A body that moves carrying a structure takes it to its new cell, before any part act of the serial pass; another
/// body that held a part of it, left behind, lets go of it.
#[test]
fn a_carried_structure_leaves_its_other_holders_behind() {
    let (mut w, i) = world();
    let j = w.mesh.cells().around(i).1[0] as usize;
    let (p, q) = (plate(&mut w, i, 1.0, false), plate(&mut w, i, 1.0, false));
    bond::link(&mut w, p, q, 1e9);
    let mut ids = Vec::new();
    for part in [p, q] {
        let mut b = eater(&mut w, i, brain::REST, 1.0);
        // (Strong enough to bear the structure it carries, `bond::carry`.)
        b.genes[STRENGTH] = 1.0;
        b.prepare(&w.chem, &w.cfg);
        b.hold = Some(part);
        let h = w.add(b);
        w.life.part_mut(part).expect("the part").held = w.life[h].id;
        ids.push(w.life[h].id);
    }
    let (mover, other) = (ids[0], ids[1]);
    w.in_cell(i, |_, d| d.out.push(Entry::Go { id: mover, from: i as u32, to: j as u32, pushed: 0.0, lane: 0.0, carries: true, kin: false, eats: 0.0, brain: Vec::new() }));
    pcore::life::land(&mut w);
    let (m, o) = (w.life.find(mover).expect("the mover"), w.life.find(other).expect("the other holder"));
    let held = bond::held(w.life.cell_at(j), &w.life[m]).map(|x| w.life.cell_at(j).parts[x].id);
    assert!(w.life[m].cell as usize == j && held == Some(p), "the mover does not hold its part in its new cell");
    assert!(w.life.part_cell(p) == Some(j) && w.life.part_cell(q) == Some(j), "the structure stayed behind");
    assert!(w.life[o].hold.is_none() && w.life.part(q).expect("the part").held == bond::NOBODY, "a body left behind still holds a part of the moved structure");
}

/// An edge concentrates force (laws.md 3.3). A taker holding a flat plate of rock (scratch hardness 0.35, above body
/// matter's and the frame's 0.2) needs the share c of the energy and force to break its prey: the plate's thickness
/// over a member's width. Against an equal force its bite takes more. A plate of the frame and body matter, no harder
/// than the tissue (a fact of chemistry 0), changes nothing and does not wear. The rock wears by Archard's law,
/// `abrade` × work / (`TABOR` × breaking stress), its walls to the ground; the books close.
#[test]
fn an_edge_cuts_only_what_is_softer_than_it() {
    // The share the law gives for a prey `o` and an edge `thick` m thick harder than all it is built of.
    let law = |w: &World, o: &Body, thick: f64| (thick / o.width(&w.chem, &w.cfg)).min(1.0);
    // One bite with a flat plate of 5 kg of each of substances `of` held (none: nothing held): the world, its cell, the
    // taker, the share the edge leaves and the law's, the kg taken and the volume of walls worn.
    let scene = |of: &[usize]| {
        let (mut w, i) = world();
        w.cfg.phys.sweep = 1e12;
        let ns = w.chem.ns();
        let mut a = eater(&mut w, i, brain::REST, 1.0);
        let mut v = eater(&mut w, i, brain::REST, 1.0);
        (a.genes[STRENGTH], v.genes[STRENGTH]) = (0.5, 0.5);
        let v = node_of(&mut w, v, 10.0);
        let part = (!of.is_empty()).then(|| {
            let mut wall = vec![0.0; ns];
            for &k in of {
                wall[k] = 5.0 / w.chem.subs[k].molar_mass;
                let have = w.store.amounts(Class::Ground)[k * w.mesh.cells().n() + i];
                set_amount(&mut w, Node::ground(i), k, have + 2.0 * wall[k]);
            }
            part_of(&mut w, i, wall, vec![0.0; ns], -1.0, 0.0)
        });
        a.hold = part;
        let (a, v) = (place(&mut w, a), place(&mut w, v));
        if let Some(p) = part {
            w.life.part_mut(p).expect("the part").held = w.life[a].id;
        }
        let books = Books::now(&w);
        let c = bond::edge(w.life.cell_at(i), &w, &w.life[a]).map_or(1.0, |e| w.life[v].cut_by(&w.chem, &w.cfg, &e));
        let thick = part.map_or(f64::INFINITY, |p| w.life.part(p).expect("the part").thickness(&w.chem));
        let expected = law(&w, &w.life[v], thick);
        let walls = |w: &World| part.map_or(0.0, |p| w.life.part(p).expect("the part").walls(&w.chem).volume);
        let before = walls(&w);
        w.events.clear();
        bite(&mut w, a, v);
        pcore::life::land(&mut w);
        let got: f64 = w.events.iter().map(|e| if let pcore::event::EventKind::Took { .. } = e.kind { e.value } else { 0.0 }).sum();
        books.assert_kept(&w, 1e-9);
        let worn = before - walls(&w);
        (w, (i, a), [c, expected], got, worn)
    };
    let (_, _, [c0, _], none, _) = scene(&[]);
    let (_, _, [c_frame, _], frame, worn_frame) = scene(&[FRAME, MATTER]);
    let (mut w, (i, a), [c, expected], rock, worn) = scene(&[ROCK]);
    assert!(c0 == 1.0 && c_frame == 1.0 && frame == none && worn_frame == 0.0, "the frame cut: c {c_frame}, took {frame} kg of {none}, wore {worn_frame} m³");
    println!("rock: c {c:.4}; took {rock:.4} kg, blunt {none:.4} kg; wore {worn:e} m³");
    assert!(c < 0.99 && (c - expected).abs() < 1e-9, "c {c}, the law gives {expected}");
    assert!(rock > none && worn > 0.0, "the rock took {rock} kg (blunt {none}) and wore {worn} m³");
    // Archard's law on its own: the edge does 1 GJ of work.
    let id = w.life[a].hold.expect("it holds the rock");
    let (books, edge) = (Books::now(&w), bond::edge(w.life.cell_at(i), &w, &w.life[a]).expect("the rock is an edge"));
    let held = |w: &World| w.life.part(id).expect("the rock").wall[ROCK];
    let (before, work) = (held(&w), 1e9);
    w.in_cell(i, |_, d| bond::blunt(d, &edge, work));
    pcore::life::land(&mut w);
    let s = &w.chem.subs[ROCK];
    let archard = w.cfg.phys.abrade * work / (pcore::units::TABOR * s.rupture) * s.density[0] / s.molar_mass;
    assert!(((before - held(&w)) / archard - 1.0).abs() < 1e-9, "wore {} mol, Archard's law {archard}", before - held(&w));
    books.assert_kept(&w, 1e-9);
}

/// Body `b` (taken out of its slot) takes from the part named `p` of its cell (`bond::act`, verb 1), in a serial step.
fn act_on(w: &mut World, b: &mut Body, p: u64) {
    let i = b.cell as usize;
    w.in_cell(i, |w, d| {
        let x = bond::at(d.at(i), p).expect("the part");
        bond::act(w, d, b, 1, (x, w.step_days(i)))
    });
}
