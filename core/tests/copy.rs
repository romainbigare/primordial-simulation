//! Law 4 (Copy): growth builds body matter, a mature body fills a brood and a child is made of it, two parents mix,
//! population nodes spread, split, merge and open, and a hurt body rebuilds.

use pcore::body::{Body, Layout, ALLOC, CHILD, COOL_T, COPY_MASS, DENATURE, DISPERSE, GAS, HEAT, HIDDEN, KEEP_T, LEAF, LIQ, MEMORY, MUT, SEEN, SIGHT, STRENGTH};
use pcore::event::EventKind;
use pcore::rng::{Purpose, Rng};
use pcore::units::DAY_S;
use pcore::{body, brain, copy, lineage, nodes, World};
use worldgen::testkit::{self, add, brood, children, eater, founder, kg, only_the_solid, out, put, put_kg_m2, scale, small_world, start, took_to_burn, Books, Brain, BURN, DISSOLVED, LIQUID, MATTER, RICH};

/// Bodies `a` and `b` are in one size class (`body::class`): alike in size and in maturity.
fn near(lay: &Layout, chem: &pcore::chem::Chemistry, alike: f32, a: &Body, b: &Body) -> bool {
    let (x, y) = (body::class(chem, a), body::class(chem, b));
    x.1 == y.1 && body::alike_log(lay, alike, x.0, y.0)
}

/// A body whose roots take the minerals growth is built of (the uptake genes at 0.01, so that a meal is still nearly
/// all fuel; soaking at least a tenth a day, `body::live`), and the ground of its cell given a hundred times the level
/// it keeps of each (booked as an edit): what a test that builds for many days needs to be free of the minerals. (The
/// dissolved mineral falls out of the liquid inside a body within days, so a store given from outside is gone by the
/// next update; roots give it at the update, just before the body builds.)
fn feed_minerals(w: &mut World, s: usize) {
    let (i, n, lay) = (w.life.bodies[s].cell as usize, w.mesh.cells().n(), Layout::of(&w.chem));
    for (mk, _) in w.chem.minerals().to_vec() {
        let b = &mut w.life.bodies[s];
        (b.genes[lay.upt + mk], b.genes[LIQ]) = (0.01, b.genes[LIQ].max(-1.0));
        b.k.cat.clear();
        let a = (100.0 * b.level(&w.chem, mk, b.mass(&w.chem)) - w.ground[mk * n + i]).max(0.0);
        put(w, mk, i, a);
    }
}

/// Growth makes body matter of the energy-rich solid and the two minerals (`chemistry0.rs` has the counts), and pays
/// `c_copy` per kg of matter made in work that burns the solid (the fuel's heat leaves the body for the ground):
/// elements and energy are kept, and the stores change as the chemistry says: matter up by x, the solid down by its
/// count times x and by what the work burnt, each mineral down by its count times x, the liquid up by its count times
/// x and by the liquid the burning made.
#[test]
fn growth_makes_matter_of_the_solid_and_keeps_elements_and_energy() {
    let (mut w, i) = start();
    w.cfg.phys.c_build = 1e6;
    let b = eater(&mut w, i, brain::REST, 6.0);
    let (_, mut b) = out(&mut w, b);
    let books = Books::now(&w);
    let (matter0, rich0, liquid0, e0) = (b.store[MATTER], b.store[RICH], b.store[LIQUID], w.e_ground.iter().sum::<f64>());
    let minerals = w.chem.minerals().to_vec();
    let mineral0: Vec<f64> = minerals.iter().map(|m| b.store[m.0]).collect();
    // (Counts of the chemistry's growth: the solid in, the liquid out.)
    let bl = w.chem.build.as_ref().expect("growth");
    let (n_rich, n_liquid) = (bl.inputs.iter().find(|p| p.0 as usize == RICH).expect("the solid").1 as f64, bl.outputs.iter().find(|p| p.0 as usize == LIQUID).expect("the liquid").1 as f64);
    let days = b.interval(&w) as f64;
    assert!(copy::grow(&mut w, &mut b, days), "it did not grow");
    let (x, c_copy) = (b.store[MATTER] - matter0, w.cfg.phys.c_copy);
    assert!(x > 0.0);
    // The work: c_copy per kg of matter, burnt as the solid is (heat per mol of the burning reaction).
    let burnt = c_copy * x * w.chem.subs[MATTER].molar_mass / -w.chem.dh(BURN);
    let (dsolid, dliquid) = (rich0 - b.store[RICH], b.store[LIQUID] - liquid0);
    let dmineral: Vec<f64> = minerals.iter().zip(&mineral0).map(|(m, a)| a - b.store[m.0]).collect();
    println!("matter made {x:.4} mol ({:.3} kg), solid down {dsolid:.4} mol ({:.4} for the matter, burnt {burnt:.4}), minerals down {dmineral:.4?}, liquid up {dliquid:.4}", x * w.chem.subs[MATTER].molar_mass, n_rich * x);
    assert!((dsolid / (n_rich * x + burnt) - 1.0).abs() < 1e-6, "the solid fell by {dsolid}, not {}", n_rich * x + burnt);
    for (m, d) in minerals.iter().zip(&dmineral) {
        assert!((d / (m.1 * x) - 1.0).abs() < 1e-9, "substance {} fell by {d}, x = {x}", m.0);
    }
    // (The burning of the solid for the work makes liquid too, as its reaction says.)
    let burn_liquid = w.chem.reactions[BURN].outputs.iter().find(|p| p.0 as usize == LIQUID).map_or(0.0, |p| p.1 as f64);
    assert!((dliquid / (n_liquid * x + burn_liquid * burnt) - 1.0).abs() < 1e-6, "the liquid rose by {dliquid}, not {}", n_liquid * x + burn_liquid * burnt);
    // The work left the body as heat for the ground: that heat is the burnt solid's, less the fusion enthalpy the liquid it
    // made holds in the body, less the share the body keeps while it leaves (about 5%).
    let heat = w.e_ground.iter().sum::<f64>() - e0;
    let work = burnt * (-w.chem.dh(BURN) - burn_liquid * w.chem.subs[LIQUID].lf);
    assert!(heat > 0.0 && (heat / work - 1.0).abs() < 0.06, "heat to the ground {heat:e} J, the work {work:e} J");
    // Growth is thermoneutral in a body: the matter holds the energy of the solid it was made of (the liquids holding their
    // fusion enthalpy).
    let (chem, bl) = (&w.chem, w.chem.build.as_ref().unwrap());
    let fused = |v: &[(u16, u16)]| v.iter().map(|&(k, n)| n as f64 * chem.fused_heat(k as usize, pcore::units::T_SURF, |_| 0.0)).sum::<f64>();
    let growth = chem.dh_of(bl) + fused(&bl.outputs) - fused(&bl.inputs);
    assert!(growth.abs() < 1.0, "growth takes {growth} J");
    w.life.bodies[0] = b;
    books.assert_kept(&w, 1e-9);
}

/// Building takes time: with fuel and spare in plenty, a body builds `c_build` x its supply / `m_ref` x the days of
/// its interval over `c_copy` kg of matter in an update, no more; twice the allowance builds twice the matter. A
/// mature body (its body at or past its copy mass) builds as much and puts the share `ALLOC` of it into its brood: at
/// 1 its body stays what it was.
#[test]
fn growth_is_limited_by_c_build() {
    let built = |c_build: f64, copy: f32, alloc: f32| {
        let (mut w, i) = start();
        w.cfg.phys.c_build = c_build;
        let mut b = eater(&mut w, i, brain::REST, copy);
        b.genes[ALLOC] = alloc;
        scale(&mut w, &mut b, 5.0);
        let (s, mut b) = out(&mut w, b);
        let m0 = b.store[MATTER];
        let days = b.interval(&w) as f64;
        let room = w.cfg.phys.c_build * b.supply(&w.chem, &w.cfg.phys) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
        let (built0, grew) = (b.built(&w.chem), copy::grow(&mut w, &mut b, days));
        let kg_matter = (b.store[MATTER] - m0) * w.chem.subs[MATTER].molar_mass;
        let (brood, built1) = (b.brood * w.chem.subs[MATTER].molar_mass, b.built(&w.chem));
        w.life.bodies[s] = b;
        (grew, kg_matter, room, brood, built1 - built0)
    };
    let (g1, one, room1, _, _) = built(1e4, 6.0, 0.8);
    let (g2, two, room2, _, _) = built(2e4, 6.0, 0.8);
    println!("built {one:.5} kg of matter with c_build 1e4 (allowance {room1:.5} kg), {two:.5} kg with 2e4 ({room2:.5})");
    assert!(g1 && g2);
    assert!((one / room1 - 1.0).abs() < 1e-3 && (two / room2 - 1.0).abs() < 1e-3, "the allowance binds: {one} of {room1}, {two} of {room2}");
    assert!((two / one - 2.0).abs() < 0.01, "twice the allowance, twice the matter: {one} and {two}");
    // A mature one (matter 5 x 5.25 kg over 0.25 = 105 kg of body; copy mass 10^1.0 = 10 kg) builds as much; with
    // ALLOC 1 all of it is brood and its body stays.
    let (grew, kg_matter, room, brood, grown) = built(1e4, 1.0, 1.0);
    assert!(grew && (kg_matter / room - 1.0).abs() < 1e-3, "a mature body built {kg_matter} kg of {room}");
    assert!((brood / kg_matter - 1.0).abs() < 1e-9 && grown.abs() < 1e-9 * 105.0, "ALLOC 1: brood {brood} of {kg_matter} kg, its body grew by {grown} kg");
}

/// Building takes what the body can spare: after what it pays and converts, its reactions must still be able to
/// release its upkeep over an interval. A body with little fuel builds less than its allowance and keeps that; one
/// with none builds nothing.
#[test]
fn growth_is_limited_by_what_the_body_can_spare() {
    let (mut w, i) = start();
    w.cfg.phys.c_build = 1e9;
    let mut results = Vec::new();
    for keep in [0.0, 0.08, 0.15, 1.0] {
        let mut w = w.clone();
        let mut b = eater(&mut w, i, brain::REST, 6.0);
        // (This measures the spare rule: the solid's minimum, the reserve growth keeps, is set low so that it does not
        // bind.)
        b.genes[Layout::of(&w.chem).min + RICH] = 0.02;
        let out_kg = (1.0 - keep) * kg(&w, &b, RICH);
        add(&mut w, &mut b, RICH, -out_kg);
        let (s, mut b) = out(&mut w, b);
        let days = b.interval(&w) as f64;
        let m0 = b.store[MATTER];
        let grew = copy::grow(&mut w, &mut b, days);
        let built = (b.store[MATTER] - m0) * w.chem.subs[MATTER].molar_mass;
        let (up, release) = (body::upkeep(&w, &b, days, b.mass(&w.chem)), body::release(&w, &b, days));
        let room = w.cfg.phys.c_build * b.supply(&w.chem, &w.cfg.phys) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
        println!("fuel kept {keep}: built {built:.4} kg of matter (allowance {room:.3} kg), release {release:.3e} J against upkeep {up:.3e} J");
        w.life.bodies[s] = b;
        results.push((keep, grew, built, room, up, release));
    }
    // With no fuel it builds nothing; with some, less than the allowance and still able to pay its upkeep; with a lot, the
    // allowance (or what it wants).
    assert!(!results[0].1 && results[0].2 == 0.0, "built with no fuel: {:?}", results[0]);
    assert!(results[1].1 && results[1].2 < 0.5 * results[1].3, "{:?}", results[1]);
    for &(_, grew, _, _, up, release) in &results[1..] {
        assert!(!grew || release >= 0.99 * up, "it cannot pay its upkeep after growing: release {release:e}, upkeep {up:e}");
    }
    assert!(results[2].2 > results[1].2 && results[3].2 > results[2].2, "more fuel, more built: {results:?}");
}

/// Storing fuel costs no building work: a body with no allowance builds nothing however much fuel it holds
/// (`copy::grow` pays no work and changes no store), and a body that takes the energy-rich solid from the ground with
/// no allowance to build with stores it (the solid rises) and its matter and size stay.
#[test]
fn storing_the_solid_costs_no_build() {
    let (mut w, i) = start();
    // A mature body (copy mass 10 kg of body, its body 21 kg) with 100 kg more fuel and no allowance to build with: it
    // pays no work for the fuel and builds nothing.
    w.cfg.phys.c_build = 0.0;
    let mut b = eater(&mut w, i, brain::REST, 1.0);
    add(&mut w, &mut b, RICH, 100.0);
    assert!(b.size(&w.chem) > 20.0);
    let (_, mut b) = out(&mut w, b);
    let (stores, heat, days) = (b.store.clone(), b.e, b.interval(&w) as f64);
    assert!(!copy::grow(&mut w, &mut b, days) && b.store == stores && b.e == heat, "a body with no allowance built");

    // A body taking the solid from the ground, with no allowance to build.
    let (mut w, i) = start();
    w.cfg.phys.c_handle = 0.0;
    w.cfg.phys.c_build = 0.0;
    // A kg of the solid per m² on its ground (booked as an edit).
    put_kg_m2(&mut w, RICH, i, 1.0);
    let mut b = eater(&mut w, i, brain::TAKE, 6.0);
    // (An eater of the solid only: it does not burn body matter, which would be burnt for its upkeep.)
    only_the_solid(&w, &mut b);
    // Less of the solid than it holds at once, so that it has room to store.
    let out_kg = 0.5 * kg(&w, &b, RICH);
    add(&mut w, &mut b, RICH, -out_kg);
    let (rich0, matter0) = (kg(&w, &b, RICH), kg(&w, &b, MATTER));
    let s = w.life.add(b, w.day(), 1);
    let books = Books::now(&w);
    let mut took = 0.0;
    for _ in 0..30 {
        w.events.clear();
        pcore::day(&mut w);
        took += took_to_burn(&w);
    }
    let b = &w.life.bodies[s];
    println!("took {took:.3} kg: solid {rich0:.3} -> {:.3} kg, matter {matter0:.3} -> {:.3} kg", kg(&w, b, RICH), kg(&w, b, MATTER));
    assert!(took > 0.5 && kg(&w, b, RICH) > rich0, "it stored nothing: took {took} kg, solid {rich0} -> {}", kg(&w, b, RICH));
    // (Matter does not rise; it rots in the open at 0.2 a year: 1.5% in a month.)
    assert!(kg(&w, b, MATTER) <= matter0 && kg(&w, b, MATTER) > 0.97 * matter0, "matter changed with no allowance: {matter0} -> {}", kg(&w, b, MATTER));
    books.assert_kept(&w, 1e-9);
}

/// Matter and energy are kept over a child's making, and what a child is made of: matter of the brood at the size its
/// `CHILD` gene sets (`10^CHILD` of its parent's copy mass in body, `Body::built`), provisions of every other store in
/// the proportion its parent's body holds them to its matter, the heat of its parent's temperature, and no work paid
/// (a copy cost nobody could pay and no allowance: the matter was built and paid for as brood). The parent keeps its
/// body: what it built (`built`) does not fall. Births are the brood over a child's matter: whole for an individual
/// (the rest of the brood waits), fractional members for a node.
#[test]
fn a_child_is_made_of_the_brood_at_the_size_its_gene_sets() {
    let (mut w, i) = start();
    // (A copy cost nobody could pay, no allowance: a child costs its parent no work.)
    (w.cfg.phys.c_copy, w.cfg.phys.c_build) = (1e18, 0.0);
    // A mature eater: a body of 21 kg, matter 5.25 kg and the liquid it keeps, its copy mass 10 kg; a child is 0.1 of
    // that, a body of 1 kg.
    let mut b = eater(&mut w, i, brain::REST, 10f32.log10());
    b.genes[CHILD] = -1.0;
    let (mm, h) = (w.chem.subs[MATTER].molar_mass, 0.75);
    // A brood of 2.5 children: matter 0.25 kg each (a body of 1 kg is its matter over 1 - h).
    let child_mol = 1.0 * (1.0 - h) / mm;
    b.brood = 2.5 * child_mol;
    let (s, mut b) = out(&mut w, b);
    let books = Books::now(&w);
    let (before, ground0, built0, size0, temp0) = (b.store.clone(), w.e_ground.iter().sum::<f64>(), b.built(&w.chem), b.size(&w.chem), b.temp(&w.chem));
    assert!(b.adult(&w.chem), "the parent is mature: built {} kg", built0);
    w.time += DAY_S;
    assert!(copy::copy(&mut w, s, &mut b), "no child");
    let kids = children(&w, b.id(s));
    // Two whole children (an individual's), the rest of the brood waits for the next.
    let born: f64 = kids.iter().map(|&k| w.life.bodies[k].count).sum();
    assert_eq!(born, 2.0, "births {born}");
    assert!((b.brood - 0.5 * child_mol).abs() < 1e-9 * child_mol, "the brood left is {} of {}", b.brood, 0.5 * child_mol);
    // A child's body is 10^CHILD of its parent's copy mass: 1 kg a member; its matter was the brood's.
    let mut given = 0.0;
    for &k in &kids {
        let c = &w.life.bodies[k];
        // (Its matter over 1 - h of its parent: its own keep gene is its parent's plus noise.)
        let body = c.store[MATTER] * mm / c.n() / (1.0 - h);
        assert!((body - 1.0).abs() < 1e-9, "a child's body is {body} kg, not 10^CHILD x the copy mass = 1 kg");
        given += c.store[MATTER];
        // Provisions in the proportion the parent's body holds them to its matter (what it holds of matter: 5.25 - its brood).
        let per_mol = c.n() * child_mol / (before[MATTER] - 2.5 * child_mol);
        for q in (0..w.chem.ns()).filter(|&q| q != MATTER) {
            assert!((c.store[q] - per_mol * before[q]).abs() <= 1e-9 * before[q].max(1.0), "substance {q}: child {} of {}", c.store[q], before[q]);
        }
        // As warm as its parent was, with no work paid.
        assert!((c.temp(&w.chem) / temp0 - 1.0).abs() < 1e-9, "the child is as warm as the parent was");
    }
    assert!((given - 2.0 * child_mol).abs() < 1e-9 * child_mol, "the children got {given} mol of matter, not {}", 2.0 * child_mol);
    for (q, &was) in before.iter().enumerate().take(w.chem.ns()) {
        let kids_hold: f64 = kids.iter().map(|&k| w.life.bodies[k].store[q]).sum();
        assert!((b.store[q] + kids_hold - was).abs() <= 1e-9 * was.max(1.0), "substance {q} not kept");
    }
    assert_eq!(w.e_ground.iter().sum::<f64>(), ground0, "work was paid for a child");
    // The parent's body does not fall: the matter it built is what it was (the brood left its matter and its brood
    // together), and its size, which waits on the liquid it still holds, is no smaller either.
    println!("parent: built {built0:.3} -> {:.3} kg, size {size0:.3} -> {:.3} kg", b.built(&w.chem), b.size(&w.chem));
    assert!((b.built(&w.chem) - built0).abs() < 1e-9 * built0, "the parent's body fell: {built0} -> {}", b.built(&w.chem));
    assert!(b.size(&w.chem) >= size0 * (1.0 - 1e-9), "the parent's size fell: {size0} -> {}", b.size(&w.chem));
    // With less than a child in the brood it makes none.
    assert!(!copy::copy(&mut w, s, &mut b));
    w.life.bodies[s] = b;
    books.assert_kept(&w, 1e-9);

    // A node of five such members: the brood of 7.3 children makes 7.3 members, who are immature and so do not join
    // their mature parents but start a young node of their own, a third of them in a neighbour cell; the parents' stores
    // and members stay what they were but for what they gave, the mean member of the node is what it was, and the
    // children who left got what those who stayed got.
    let (mut w, i) = start();
    w.cfg.phys.c_copy = 1e18;
    let mut n = eater(&mut w, i, brain::REST, 10f32.log10());
    (n.genes[CHILD], n.genes[DISPERSE]) = (-std::f32::consts::LOG10_2, 1.0 / 3.0);
    scale(&mut w, &mut n, 5.0);
    n.count = 5.0;
    // A child is a body of half the copy mass: 5 kg, matter 1.25 kg.
    n.brood = 7.3 * 5.0 * (1.0 - h) / mm;
    let (s, mut n) = out(&mut w, n);
    let (before, count0, built0) = (n.store.clone(), n.count, n.built(&w.chem) / n.n());
    assert!(n.adult(&w.chem), "the node is mature: built {} kg a member", n.built(&w.chem) / n.n());
    w.time += DAY_S;
    let at = w.life.alive().count();
    assert!(copy::copy(&mut w, s, &mut n));
    w.life.bodies[s] = std::mem::take(&mut n);
    let n = &w.life.bodies[s];
    let kids: Vec<usize> = (0..w.life.bodies.len()).filter(|&k| k != s && w.life.bodies[k].alive).collect();
    assert_eq!((kids.len(), w.life.alive().count()), (2, at + 3), "the children are two nodes of their own");
    let (c, other) = (&w.life.bodies[kids[0]], &w.life.bodies[kids[1]]);
    assert!(c.cell != other.cell && (c.count + other.count - 7.3).abs() < 1e-6 && !c.adult(&w.chem), "{} and {} young members", c.count, other.count);
    for q in 0..w.chem.ns() {
        let (x, y) = (c.store[q] / c.count, other.store[q] / other.count);
        assert!((x - y).abs() <= 1e-9 * x.abs().max(1e-30), "substance {q}: {x} and {y} a child");
    }
    assert_eq!(n.count, count0, "the parent node took its children in");
    assert!(n.brood == 0.0 && (n.built(&w.chem) / n.n() - built0).abs() < 1e-9 * built0, "the brood is left or the mean member fell: {built0} -> {}", n.built(&w.chem) / n.n());
    let given = c.store[MATTER] + other.store[MATTER];
    assert!((given - 7.3 * 1.25 / mm).abs() < 1e-6 * given && (before[MATTER] - n.store[MATTER] - given).abs() < 1e-9 * given);
    println!("node: {count0} members of {built0:.2} kg; children {} members of {:.2} kg", c.count, c.built(&w.chem) / c.n());
}

/// A mature node's children never lower its mean under maturity: they gather in a young node of their own (the second
/// brood joins the first), and the young node merges into the adult only once it is mature itself (`body::class`: the
/// same size class and both mature, or both immature). Grown to the size of the adults' class but under the size of
/// maturity, the young stay apart at the month's merge; mature, they join. Members, matter and energy are kept.
#[test]
fn a_mature_nodes_mean_stays_mature_and_its_young_merge_in_only_once_mature() {
    let (mut w, i) = start();
    w.cfg.phys.c_copy = 1e18;
    let mut n = eater(&mut w, i, brain::REST, 10f32.log10());
    // Children of 0.3 of the copy mass: 3 kg a body, a quarter of that matter (h 0.75).
    (n.genes[CHILD], n.genes[DISPERSE]) = (0.3f32.log10(), 0.0);
    scale(&mut w, &mut n, 5.0);
    n.count = 5.0;
    let (mm, h) = (w.chem.subs[MATTER].molar_mass, 0.75);
    let child_mol = 3.0 * (1.0 - h) / mm;
    let s = w.life.add(n, w.day(), 1);
    let books = Books::now(&w);
    w.time += DAY_S;
    pcore::survey::survey(&mut w);
    let lay = Layout::of(&w.chem);
    for round in 0..3 {
        let mut n = std::mem::take(&mut w.life.bodies[s]);
        n.prepare(&w.chem, &w.cfg.phys);
        // A brood of 4 children for the members together.
        n.brood = 4.0 * child_mol;
        assert!(n.adult(&w.chem), "round {round}: the node is mature: {} kg", n.built(&w.chem) / n.n());
        let before = n.built(&w.chem) / n.n();
        assert!(copy::copy(&mut w, s, &mut n), "round {round}: no children");
        assert_eq!(n.count, 5.0);
        // The mean member of the node is what it was, and mature.
        assert!(n.adult(&w.chem) && (n.built(&w.chem) / n.n() - before).abs() < 1e-9 * before, "round {round}: the mean member fell: {before} -> {}", n.built(&w.chem) / n.n());
        w.life.bodies[s] = n;
        w.time += DAY_S;
    }
    // All three broods are in one young node.
    let kids: Vec<usize> = (0..w.life.bodies.len()).filter(|&k| k != s && w.life.bodies[k].alive).collect();
    assert_eq!(kids.len(), 1, "the children are {} nodes", kids.len());
    let k = kids[0];
    assert!((w.life.bodies[k].count - 12.0).abs() < 1e-4 && !w.life.bodies[k].adult(&w.chem), "young: {} members", w.life.bodies[k].count);
    let month = w.cfg.calendar().month;
    // A month's merge: alike in genes and in size class (3 kg and the parents' about 20), but the young are immature.
    w.time = (w.time / month + 1) * month;
    pcore::survey::survey(&mut w);
    assert_eq!(w.life.alive().count(), 2, "the young merged into the mature node");
    // Grown to 8 kg a member (under the copy mass 10 kg, within the size class of the parents' 20): still apart.
    let grow_to = |w: &mut World, k: usize, body: f64| {
        let want = (body * (1.0 - h) * w.life.bodies[k].count - w.life.bodies[k].store[MATTER] * mm).max(0.0);
        let mut c = std::mem::take(&mut w.life.bodies[k]);
        add(w, &mut c, MATTER, want);
        add(w, &mut c, LIQUID, 3.0 * want);
        c.prepare(&w.chem, &w.cfg.phys);
        w.life.bodies[k] = c;
    };
    grow_to(&mut w, k, 8.0);
    assert!(!w.life.bodies[k].adult(&w.chem) && body::alike_size(&lay, w.cfg.phys.alike, 8.0, w.life.bodies[s].built(&w.chem) / 5.0));
    assert!(!near(&lay, &w.chem, w.cfg.phys.alike, &w.life.bodies[s], &w.life.bodies[k]));
    w.time += month;
    pcore::survey::survey(&mut w);
    assert_eq!(w.life.alive().count(), 2, "an immature node merged into a mature one");
    // Mature (11 kg), it joins.
    grow_to(&mut w, k, 11.0);
    assert!(w.life.bodies[k].adult(&w.chem));
    w.time += month;
    pcore::survey::survey(&mut w);
    assert_eq!(w.life.alive().count(), 1, "the mature young did not merge in");
    let all: f64 = w.life.alive().map(|b| b.n()).sum();
    assert!((all - 17.0).abs() < 1e-4, "members {all}");
    books.assert_kept(&w, 1e-9);
}

/// A mature body splits what it builds in an update by `ALLOC`: the share to its brood, the rest its own growth; one
/// just below the size of maturity puts everything into its own growth up to that size and splits what it builds past
/// it. Counted in matter: what it made is the matter that rose, the brood is the share of it, the body grew by the
/// rest (matter over 1 - h).
#[test]
fn a_mature_body_splits_its_building_by_alloc() {
    let build = |copy: f32, alloc: f32| {
        let (mut w, i) = start();
        w.cfg.phys.c_build = 1e5;
        let mut b = eater(&mut w, i, brain::REST, copy);
        b.genes[ALLOC] = alloc;
        let (s, mut b) = out(&mut w, b);
        let (m0, built0, mm) = (b.store[MATTER], b.built(&w.chem), w.chem.subs[MATTER].molar_mass);
        let days = b.interval(&w) as f64;
        assert!(copy::grow(&mut w, &mut b, days), "it did not build");
        let made = (b.store[MATTER] - m0) * mm;
        let (brood, grown) = (b.brood * mm, (b.built(&w.chem) - built0) * (1.0 - 0.75));
        w.life.bodies[s] = b;
        (made, brood, grown)
    };
    // Mature (a body of 21 kg, copy mass 10 kg): the share ALLOC of what it made is brood, the rest its body.
    for alloc in [0.0f32, 0.25, 0.8, 1.0] {
        let (made, brood, grown) = build(1.0, alloc);
        println!("ALLOC {alloc}: made {made:.4} kg of matter, brood {brood:.4}, body grew by {grown:.4} (in matter)");
        assert!(made > 0.0 && (brood - alloc as f64 * made).abs() < 1e-9 * made, "ALLOC {alloc}: brood {brood} of {made}");
        assert!((grown - (1.0 - alloc as f64) * made).abs() < 1e-9 * made, "ALLOC {alloc}: the body grew by {grown} of {made}");
    }
    // Just under maturity (the body lacks 0.2 of what it makes), it grows to the size of maturity and splits the rest.
    let (made, ..) = build(1.0, 1.0);
    // Copy mass such that the body (matter 5.25 kg) lacks 0.2 made of the matter of maturity: needs 0.2 x made.
    let matter0 = {
        let (mut w, i) = start();
        let b = eater(&mut w, i, brain::REST, 1.0);
        b.matter_kg(&w.chem)
    };
    let mc = (matter0 + 0.2 * made) / (1.0 - 0.75);
    let (m2, brood2, grown2) = build(mc.log10() as f32, 0.5);
    assert!((m2 - made).abs() < 1e-9 * made);
    assert!((grown2 - (0.2 * made + 0.5 * 0.8 * made)).abs() < 1e-6 * made && (brood2 - 0.5 * 0.8 * made).abs() < 1e-6 * made, "crossing maturity: body {grown2}, brood {brood2} of {made}");
}

/// An immature body never reproduces: whatever its brood (a brood it would not have), `copy::copy` makes no child while
/// its body (`Body::built`) is under its copy mass, and over the updates of its life a body that builds fast grows to
/// maturity first and has a brood only from then on.
#[test]
fn an_immature_body_never_reproduces() {
    let immature = |w: &mut World, i: usize| {
        let mut b = eater(w, i, brain::REST, 24f32.log10());
        b.genes[CHILD] = -1.0;
        b
    };
    let (mut w, i) = start();
    // Copy mass 24 kg of body, a body of 21 kg.
    let mut b = immature(&mut w, i);
    let child = 0.1 * 24.0 * (1.0 - 0.75) / w.chem.subs[MATTER].molar_mass;
    b.brood = 3.0 * child;
    let (s, mut b) = out(&mut w, b);
    assert!(!b.adult(&w.chem));
    let at = w.life.alive().count();
    w.time += DAY_S;
    assert!(!copy::copy(&mut w, s, &mut b) && w.life.alive().count() == at, "an immature body had children");
    // Updates: it builds with all it has and has no brood and no child until it is mature.
    let (mut w, i) = start();
    w.cfg.phys.c_build = 1e5;
    let mut b = immature(&mut w, i);
    // (Fuel for a life of building: 60 kg more of it.)
    add(&mut w, &mut b, RICH, 60.0);
    let s = w.life.add(b, w.day(), 1);
    let (mut grown_up, mut born_before, mut born) = (None, 0.0, 0.0);
    for d in 0..400 {
        feed_minerals(&mut w, s);
        w.events.clear();
        pcore::day(&mut w);
        let b = &w.life.bodies[s];
        if !b.alive {
            break;
        }
        let kids = testkit::born(&w);
        if grown_up.is_none() && b.adult(&w.chem) {
            grown_up = Some(d);
        }
        if grown_up.is_none() {
            born_before += kids;
            assert_eq!(b.brood, 0.0, "day {d}: an immature body (built {:.2} kg of 24) put matter into a brood", b.built(&w.chem));
        }
        born += kids;
    }
    println!("mature after {grown_up:?} days, children before: {born_before}; born in all {born}");
    assert!(grown_up.is_some() && born_before == 0.0, "mature after {grown_up:?}, children before: {born_before}");
    assert!(born > 0.0, "a mature body had no children in 400 days");
}

/// A hurt body rebuilds what it lost (`copy::grow`, at `c_copy` within `c_build`), each update by no more than its
/// allowance. A hurt adult under its copy mass is immature until healed (`Body::adult` reads the body): it puts nothing
/// into a brood while it rebuilds, and is mature again once rebuilt. Elements and energy are kept.
#[test]
fn a_hurt_body_rebuilds_within_its_allowance_and_is_immature_until_healed() {
    let (mut w, i) = start();
    let mover = eater(&mut w, i, brain::REST, 6.0);
    // A body of 21 kg that is mature from 15 kg.
    let v = eater(&mut w, i, brain::REST, 15f32.log10());
    let (mv, v) = (testkit::place(&mut w, mover), testkit::place(&mut w, v));
    let books = Books::now(&w);
    assert!(w.life.bodies[v].adult(&w.chem));
    // The energy that breaks 40% of it (`Body::breaking`).
    let e = w.life.bodies[v].breaking(&w.chem, &w.cfg, 0.4);
    pcore::choose::wear(&mut w, i, mv, e);
    let mut b = std::mem::take(&mut w.life.bodies[v]);
    assert!(!b.adult(&w.chem), "a body torn by 40% is still mature: {} kg", b.built(&w.chem));
    let days = b.interval(&w) as f64;
    // (Counted in all the matter it holds: the update that crosses maturity puts a share of what it builds into a brood.)
    let (mut last, mut updates) = (kg(&w, &b, MATTER), 0);
    while !b.adult(&w.chem) && updates < 2000 {
        let room = w.cfg.phys.c_build * b.supply(&w.chem, &w.cfg.phys) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
        assert!(copy::grow(&mut w, &mut b, days), "it did not grow after {updates} updates");
        assert!(b.adult(&w.chem) || b.brood == 0.0, "a hurt body put matter into a brood");
        let made = kg(&w, &b, MATTER) - last;
        assert!(made > 0.0 && made <= room * (1.0 + 1e-9), "built {made} kg in an update of allowance {room}");
        last = kg(&w, &b, MATTER);
        updates += 1;
    }
    println!("mature again after {updates} updates of {days} days ({:.2} kg of body)", b.built(&w.chem));
    assert!(b.adult(&w.chem) && updates > 5, "mature after {updates} updates");
    w.life.bodies[v] = b;
    books.assert_kept(&w, 1e-9);
}

/// Matter and energy are kept over updates in which a body builds, fills a brood and has children (the update path of
/// `body::live`): a mature eater with fuel and an allowance, over 150 days.
#[test]
fn reproducing_updates_keep_elements_and_energy() {
    let (mut w, i) = start();
    w.cfg.phys.c_build = 1e5;
    let mut b = eater(&mut w, i, brain::REST, 5f32.log10());
    // Children of 0.1 of the copy mass (0.5 kg) from a mature body of 21 kg.
    (b.genes[CHILD], b.genes[ALLOC]) = (-1.0, 0.8);
    let id = {
        let s = w.life.add(b, w.day(), 1);
        w.life.bodies[s].id(s)
    };
    let books = Books::now(&w);
    let mut born = 0.0;
    for _ in 0..150 {
        w.events.clear();
        pcore::day(&mut w);
        born += testkit::born(&w);
        assert_eq!(pcore::check::broken(&w), None);
    }
    let kids = w.life.alive().filter(|b| b.parent == id).count();
    println!("born {born}, bodies of its children {kids}");
    assert!(born >= 1.0, "no child in 150 days");
    books.assert_kept(&w, 1e-9);
}

/// Body size by an energy ceiling, not a cap: what a body takes goes with its mass^0.75 (`Body::equiv`) and the cost
/// of keeping its tissue with its body^1.0 (`body::upkeep`), so growth, which builds only from the spare after upkeep
/// (`copy::grow`), stops where the two meet, a size the body's own costs set. An eater (ALLOC 0: it builds only for
/// itself; strength 1; building fast, so that the allowance never binds) in a place with all the fuel it can take, at
/// every update, for 360 days. With tissue dear enough that the ceiling is near (costs 1.6 times the starting ones)
/// it levels off (under a quarter more body between the 100th and the 300th day) and in its last 100 days takes about
/// what it pays to keep what it has; with the costs of the start, a ceiling many times higher, it is still growing
/// fast and ends larger.
#[test]
fn growth_stops_where_upkeep_meets_what_the_body_can_take() {
    use pcore::event::EventKind;
    let run = |cost: f64| {
        let (mut w, i) = start();
        w.cfg.tilt = 0.0;
        // Building never binds; the costs of keeping tissue are `cost` times the starting ones.
        w.cfg.phys.c_build = 1e7;
        for c in [&mut w.cfg.phys.c_basal, &mut w.cfg.phys.c_cat, &mut w.cfg.phys.c_strength, &mut w.cfg.phys.c_learn, &mut w.cfg.phys.c_signal] {
            *c *= cost;
        }
        let mut b = eater(&mut w, i, brain::TAKE, 0.0);
        // A body that is wet (soaking), breathes freely and sheds its heat, so that only its energy limits it: a bare body
        // sheds no more than its still air and radiation take, so growing it sheds the rest by evaporating above 295 K
        // (`COOL_T`), and soaks fast to stay wet.
        (b.genes[STRENGTH], b.genes[ALLOC], b.genes[LIQ], b.genes[GAS], b.genes[HEAT], b.genes[body::COOL_T]) = (1.0, 0.0, 1.0, 3.0, 3.0, 295.0);
        // The eater founder's size of maturity: tissue costs per kg go by it across kinds (Kleiber).
        b.genes[COPY_MASS] = 1.645;
        let (s, b0) = out(&mut w, b);
        w.life.bodies[s] = b0;
        let area = w.map.area();
        let joule = -w.chem.dh(BURN) / w.chem.reactions[BURN].inputs.iter().find(|x| x.0 as usize == RICH).unwrap().1 as f64 / w.chem.subs[RICH].molar_mass;
        let (mut built, mut takes, mut ups) = (Vec::new(), vec![0.0; 360], vec![0.0; 360]);
        for day in 0..360usize {
            // Fuel and liquid in plenty: 100 kg per m² of each on its cell at every update (booked as an edit); the cell it
            // is in, as it has a brain and may step to a neighbour.
            let i = w.life.bodies[s].cell as usize;
            for k in [RICH, LIQUID] {
                let a = (100.0 * area / w.chem.subs[k].molar_mass - w.g(k, i)).max(0.0);
                put(&mut w, k, i, a);
            }
            // And the minerals growth is built of: this is a test of the energy ceiling, so its roots find them in plenty
            // (an eater has them only from the matter it digests, and here eats none).
            feed_minerals(&mut w, s);
            w.events.clear();
            pcore::day(&mut w);
            assert!(w.life.bodies[s].alive, "the body died on day {day}: {:?}", w.events.iter().filter_map(|e| if let EventKind::Died { cause, .. } = e.kind { Some(cause) } else { None }).collect::<Vec<_>>());
            // What its burning made and it holds above the level it keeps drains to the ground at its update (`live::shed`);
            // what a decision burns after it adds a trace.
            let b = &w.life.bodies[s];
            let level = b.level(&w.chem, LIQUID, b.mass(&w.chem));
            assert!(b.store[LIQUID] <= level * (1.0 + 1e-3), "day {day}: liquid over its level by a share {:.3e}", b.store[LIQUID] / level - 1.0);
            takes[day] = w.events.iter().map(|e| if let EventKind::Took { from: u64::MAX, burn, .. } = e.kind { burn as f64 * joule } else { 0.0 }).sum();
            let mut b = w.life.bodies[s].clone();
            b.prepare(&w.chem, &w.cfg.phys);
            ups[day] = body::upkeep(&w, &b, 1.0, b.mass(&w.chem));
            if day % 50 == 49 {
                built.push(b.built(&w.chem));
            }
        }
        let ratio = takes[260..].iter().sum::<f64>() / ups[260..].iter().sum::<f64>();
        println!("costs x {cost}: body (kg) every 50 days {built:.1?}; in the last 100 days it took {ratio:.2} times its upkeep");
        (built, ratio)
    };
    let (full, r_full) = run(1.6);
    let (cheap, r_cheap) = run(1.0);
    let n = full.len();
    // Relative growth between the 100th and the 300th day (the first days only fill the body with the liquid it keeps).
    let rel = |x: &[f64]| x[5] / x[1] - 1.0;
    // Growth slows toward a size: the body with the dearer tissue is near it, the other far from it.
    assert!(rel(&full) < 0.25 && rel(&cheap) > 1.5 * rel(&full), "growth: {:.3} of the body between the 100th and the 300th day against {:.3} with cheaper tissue", rel(&full), rel(&cheap));
    // What it takes over what it pays to keep what it has falls toward 1 as it grows (supply goes with mass^0.75, upkeep
    // with the body): near 1 at the end for the body with dear tissue, and more for the other, far from its ceiling. (The
    // bound is 0.8 below: the upkeep here is counted each day at the temperature of that day, while the body still
    // levels off.)
    assert!(r_full > 0.8 && r_full < 1.1 && r_cheap > r_full, "took / upkeep at the end {r_full} and {r_cheap} with cheaper tissue");
    // Cheaper tissue, a larger body in the same time.
    assert!(cheap[n - 1] > 1.5 * full[n - 1], "a body with cheaper tissue grows larger: {} against {}", cheap[n - 1], full[n - 1]);
}

/// Growth builds each solid a body can build besides matter (`Chemistry::builds`) in the share its keep gene of it sets
/// (`Body::share`), by the same work, the gas the rock's making gives off going to the air, elements and energy kept;
/// and what it is built of holds it up and resists breaking: soft tissue alone holds a body low, a frame raises it
/// (`Body::height`) and costs more to break (`Body::breaking`).
#[test]
fn growth_builds_each_solid_in_its_share_and_hard_solids_hold_a_body_up() {
    let (mut w, i) = start();
    let lay = Layout::of(&w.chem);
    let mats: Vec<usize> = w.chem.materials().collect();
    let mut b = eater(&mut w, i, brain::REST, 6.0);
    // Shares of 0.2 and 0.1, none of either held yet, the dissolved mineral the rock is made of in plenty.
    for (&k, s) in mats.iter().zip([0.2, 0.1]) {
        b.genes[lay.keep + k] = s;
        let held = kg(&w, &b, k);
        add(&mut w, &mut b, k, -held);
    }
    add(&mut w, &mut b, DISSOLVED, 1.0);
    let (s, mut b) = out(&mut w, b);
    let books = Books::now(&w);
    let before: Vec<f64> = mats.iter().chain([&MATTER]).map(|&k| kg(&w, &b, k)).collect();
    let days = b.interval(&w) as f64;
    assert!(copy::grow(&mut w, &mut b, days));
    let made: Vec<f64> = mats.iter().chain([&MATTER]).zip(&before).map(|(&k, x)| kg(&w, &b, k) - x).collect();
    let all: f64 = made.iter().sum();
    println!("built {made:.4?} kg (frame, rock, matter)");
    assert!((made[0] / all - 0.2).abs() < 0.01 && (made[1] / all - 0.1).abs() < 0.01, "shares {:.3} and {:.3}", made[0] / all, made[1] / all);
    // The same body of soft tissue alone, and with 1 kg of frame (5% of its mass).
    let (mut soft, mut hard) = (b.clone(), b.clone());
    mats.iter().for_each(|&k| (soft.store[k], hard.store[k]) = (0.0, 0.0));
    hard.store[mats[0]] = 1.0 / w.chem.subs[mats[0]].molar_mass;
    let (h_soft, h, e_soft, e) = (soft.height(&w.chem, &w.cfg), hard.height(&w.chem, &w.cfg), soft.breaking(&w.chem, &w.cfg, 1.0), hard.breaking(&w.chem, &w.cfg, 1.0));
    println!("height {h_soft:.2} m of soft tissue alone, {h:.2} m with 1 kg of frame; breaking it all {e_soft:.0} J and {e:.0} J");
    assert!(h_soft < 1.5 && h > 2.0 * h_soft && e > 2.0 * e_soft);
    w.life.bodies[s] = b;
    books.assert_kept(&w, 1e-9);
}

/// A body without a mineral cannot build body matter (not a gram of it, whichever of the two is missing); with both, it
/// builds, and each falls by its count for the matter made; with a part of one, the matter made binds to it in
/// proportion.
#[test]
fn growth_stops_without_a_mineral_and_goes_on_with_them() {
    let (mut w, i) = start();
    w.cfg.phys.c_build = 1e6;
    let minerals = w.chem.minerals().to_vec();
    let m = w.chem.matter().expect("matter");
    let b = eater(&mut w, i, brain::REST, 6.0);
    let (_, b) = out(&mut w, b);
    assert!(minerals.iter().all(|&(k, _)| b.store[k] > 0.0), "a starting body holds the minerals its matter was made of");
    let days = b.interval(&w) as f64;
    // With them it builds, and for the matter made it spends each by its count.
    let m0 = b.store[m];
    let mut with = b.clone();
    assert!(copy::grow(&mut w, &mut with, days), "it did not grow with the minerals");
    let made = with.store[m] - m0;
    for &(k, per) in &minerals {
        assert!(made > 0.0 && ((b.store[k] - with.store[k]) / (per * made) - 1.0).abs() < 1e-9, "{made} mol of matter for {} of substance {k}", b.store[k] - with.store[k]);
    }
    for &(k, per) in &minerals {
        // Without it, none: not a gram of matter.
        let mut b = b.clone();
        b.store[k] = 0.0;
        assert!(!copy::grow(&mut w, &mut b, days), "it grew without substance {k}");
        assert_eq!(b.store[m], m0);
        // A little of it, a little matter: it binds, in proportion.
        b.store[k] = 0.5 * made * per;
        assert!(copy::grow(&mut w, &mut b, days));
        let grown = b.store[m] - m0;
        assert!((grown / (0.5 * made) - 1.0).abs() < 1e-6, "{grown} mol for a store of {} mol of substance {k}", 0.5 * made * per);
        assert!(b.store[k] < 1e-9 * made);
    }
}

/// A child's genes are its parent's plus noise of the size its mutation gene sets.
#[test]
fn child_has_parent_genes_plus_noise() {
    let mut w = worldgen::new_world(1, 16, 16);
    worldgen::life::seed(&mut w, 0);
    let parent = w.life.alive().next().expect("a body").genes.clone();
    let lay = Layout::of(&w.chem);
    let child = pcore::body::mutate(&lay, &parent, &mut Rng::new(1, 0, 0, Purpose::Mutate));
    let size = 10f32.powf(parent[MUT]);
    let diff: Vec<f32> = parent.iter().zip(&child).map(|(a, b)| (a - b).abs()).collect();
    assert!(diff.iter().any(|&d| d > 0.0), "no change");
    // (The two temperature genes step by 5 K, the others by at most 1.)
    assert!(diff.iter().enumerate().all(|(g, &d)| d <= 6.0 * size * if g == DENATURE || g == KEEP_T || g == COOL_T { 5.0 } else { 1.0 }), "change too big");
}

/// More than `cfg.crowd` alike individuals in one cell (of light-eaters, which they eat) merge into one population node
/// at the next month's start; matter and energy are kept.
#[test]
fn crowded_individuals_merge_into_a_node() {
    let mut w = worldgen::new_world(1, 16, 16);
    let books = Books::now(&w);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed_with(&mut w, 1, &worldgen::life::Seeding::neutral());
    let a = w.life.alive().find(|b| b.count == 1.0).expect("an individual");
    let (i, lineage, genome) = (a.cell as usize, a.lineage, a.genome);
    let placed = w.cfg.crowd + 4;
    for _ in 0..placed - w.life.alive().filter(|b| b.cell as usize == i && b.lineage == lineage).count() {
        let b = worldgen::life::make(&mut w, 1, i, lineage, genome).expect("body");
        w.life.add(b, w.day(), 1);
    }
    // At the month's start (looked at before they act: within the month the node may give its members up one at a time
    // to the actions of Bond, as the draws go).
    pcore::survey::survey(&mut w);
    let nodes: Vec<f64> = w.life.alive().filter(|b| b.cell as usize == i && b.lineage == lineage && b.count > 1.0).map(|b| b.count).collect();
    assert!(!nodes.is_empty(), "no node of the lineage at the month's start");
    pcore::step(&mut w);
    books.assert_kept(&w, 1e-9);
}

/// However unlike, the bodies of a cell merge at a month's start until at most `cfg.crowd` nodes are left; no member
/// is lost and matter and energy are kept.
#[test]
fn crowd_bounds_the_nodes_of_a_cell() {
    let mut w = worldgen::new_world(1, 16, 16);
    let books = Books::now(&w);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed_with(&mut w, 1, &worldgen::life::Seeding::neutral());
    let a = w.life.alive().find(|b| b.count == 1.0).expect("an individual");
    let (i, lineage, genome) = (a.cell as usize, a.lineage, a.genome);
    for x in 0..3 * w.cfg.crowd {
        let mut b = worldgen::life::make(&mut w, 1, i, lineage, genome).expect("body");
        b.genes[pcore::body::COPY_MASS] += 0.3 * x as f32;
        w.life.add(b, w.day(), 1);
    }
    let here = |w: &pcore::World| w.life.alive().filter(|b| b.cell as usize == i).map(|b| b.n()).collect::<Vec<f64>>();
    let before: f64 = here(&w).iter().sum();
    pcore::survey::survey(&mut w);
    let after = here(&w);
    assert!(after.len() <= w.cfg.crowd, "{} nodes left", after.len());
    assert!((after.iter().sum::<f64>() - before).abs() < 1e-9 * before, "members {before} then {after:?}");
    books.assert_kept(&w, 1e-9);
}

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
fn pair(w: &mut World, i: usize, gap: f32) -> (usize, Body, usize) {
    let lay = Layout::of(&w.chem);
    let (mut a, mut b) = (mixer(w, i, 1.0), mixer(w, i, 1.0));
    for g in lay.cat..lay.min {
        (a.genes[g], b.genes[g]) = (BASE, BASE + gap * STEP);
    }
    let rows = w.life.born(&b).iter().map(|x| x + 1.0).collect();
    w.life.genomes[b.genome as usize].rows = rows;
    let sb = w.life.add(b, w.day(), 1);
    let (sa, a) = out(w, a);
    (sa, a, sb)
}

/// Make `n` children of `a` one after another (a day apart, each from a new brood of one), and return them.
fn copies(w: &mut World, s: usize, a: &mut Body, n: usize) -> Vec<Body> {
    for _ in 0..n {
        w.time += DAY_S;
        brood(w, a, 1.0);
        assert!(copy::copy(w, s, a), "no child");
    }
    let out: Vec<Body> = children(w, a.id(s)).into_iter().map(|k| w.life.bodies[k].clone()).collect();
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
    let (s, mut a, sb) = pair(&mut w, i, 2.0);
    let (rows_a, rows_b) = (w.life.born(&a).to_vec(), w.life.born(&w.life.bodies[sb]).to_vec());
    w.events.clear();
    let kids = copies(&mut w, s, &mut a, 200);
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
    let (s, mut a) = out(&mut w, a);
    w.events.clear();
    let kids = copies(&mut w, s, &mut a, 30);
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
    let kids = copies(&mut w, s, &mut a, 30);
    assert!(kids.iter().all(|c| (lay.cat..lay.min).all(|g| (c.genes[g] - BASE).abs() < 0.05)));
    assert_eq!(mixed(&w), (0.0, 30.0));
    // The only body in the cell that is mature is the parent itself: it is never its own partner.
    let (mut w, i) = start();
    let a = mixer(&mut w, i, 1.0);
    let (s, mut a) = out(&mut w, a);
    w.events.clear();
    copies(&mut w, s, &mut a, 5);
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
    let kids = copies(&mut w, s, &mut a, 30);
    assert!(kids.iter().all(|c| (lay.cat..lay.min).all(|g| (c.genes[g] - BASE).abs() < 0.05)), "unlike bodies mixed");
    assert_eq!(mixed(&w).0, 0.0);
    // Alike in all but one gene, which is past the limit: still unlike.
    let (mut w, i) = start();
    let (s, mut a, sb) = pair(&mut w, i, 1.0);
    w.life.bodies[sb].genes[lay.cat] = BASE + (limit + 0.5) * STEP;
    w.events.clear();
    copies(&mut w, s, &mut a, 10);
    assert_eq!(mixed(&w).0, 0.0, "one gene past the limit is unlike");
    // Brain sizes count by whole units: a partner 6 units bigger in brain (past 4) is unlike, 3 units is alike.
    for (units, mixes) in [(6.0, false), (3.0, true)] {
        let (mut w, i) = start();
        let (s, mut a, sb) = pair(&mut w, i, 1.0);
        w.life.bodies[sb].genes[body::HIDDEN] += units;
        w.events.clear();
        copies(&mut w, s, &mut a, 10);
        assert_eq!(mixed(&w).0 > 0.0, mixes, "{units} units of brain apart");
    }
    // Two partners, one near and one further but alike: all the children take the near one's genes.
    let (mut w, i) = start();
    let (s, mut a, _) = pair(&mut w, i, 3.0);
    let mut near = mixer(&mut w, i, 1.0);
    for g in lay.cat..lay.min {
        near.genes[g] = BASE + 1.0 * STEP;
    }
    w.life.add(near, w.day(), 1);
    let kids = copies(&mut w, s, &mut a, 60);
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
    let books = Books::now(&w);
    w.events.clear();
    copies(&mut w, s, &mut a, 20);
    assert_eq!(mixed(&w).0, 20.0);
    let after = &w.life.bodies[sb];
    assert_eq!((&after.store, after.e, &after.genes, after.brood, after.count), (&before.store, before.e, &before.genes, before.brood, before.count));
    assert_eq!(w.life.born(after), w.life.born(&before));
    w.life.bodies[s] = a;
    books.assert_kept(&w, 1e-9);
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
        brood(&w, &mut a, 2000.0);
        w.time += DAY_S;
        assert!(a.adult(&w.chem) && w.life.bodies[sb].adult(&w.chem));
        w.events.clear();
        assert!(copy::copy(&mut w, s, &mut a));
        // (The brood over the child's matter is a float, within rounding of 2,000.)
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

/// Slots of the live bodies.
fn live(w: &pcore::World) -> Vec<usize> {
    (0..w.life.bodies.len()).filter(|&s| w.life.bodies[s].alive).collect()
}

/// A mature node's children start a node of their own (they are immature, `body::class`) that carries the variance
/// left among them; when they have grown and join their parents' node (the month's merge, `nodes::merge`) its spread
/// is widened by it; a node whose members differ so much that its halves would not be alike splits in two along that
/// gene, keeping its members, stores, heat and pooled spread; the half placed is seen as what it holds.
#[test]
fn a_node_spreads_with_its_children_and_splits_when_too_varied() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    let books = Books::now(&w);
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
    nodes::merge(&mut w, kids[0], s);
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    assert!(b.count > 1.5 * n0 && b.spread.iter().any(|&v| v > 0.0) && b.row_spread > 0.0, "the merge widened nothing");

    // Copy mass spread over 5 mutation steps (0.2 each): the halves are 2 sqrt(2/pi) 5 = 8 apart.
    b.spread[COPY_MASS] = 1.0;
    let (count, store, e, m) = (b.count, b.store.clone(), b.e, b.genes[COPY_MASS]);
    let before = live(&w);
    nodes::split(&mut w, s, &mut b);
    w.life.bodies[s] = b;
    let new: Vec<usize> = live(&w).into_iter().filter(|x| !before.contains(x) && *x != s).collect();
    assert_eq!(new.len(), 1, "not split in two");
    seen_as_it_is(&w, new[0]);
    let (a, c) = (&w.life.bodies[s], &w.life.bodies[new[0]]);
    assert_eq!((a.count + c.count, a.lineage, a.group, a.cell), (count, c.lineage, c.group, c.cell));
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
/// so that every node is mature, and its children are of its size class, whatever its other genes: no selection by
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
    let b0 = &w.life.bodies[live(&w)[0]];
    let (per, e1) = (b0.store.iter().map(|a| 100.0 * a / b0.count).collect::<Vec<f64>>(), 100.0 * b0.e / b0.count);
    for s in live(&w) {
        (w.life.bodies[s].genes[COPY_MASS], w.life.bodies[s].genes[MUT], w.life.bodies[s].genes[CHILD]) = (1.45, -0.8, -std::f32::consts::LOG10_2);
        if wide {
            // (The first children that stay bring every spread down to the split threshold.)
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
        w.time += 10 * DAY_S;
        if w.day().is_multiple_of(w.cfg.days_per_month as u64) {
            pcore::survey::survey(&mut w);
        }
        // In a random order, so no body is first by its slot.
        let (mut order, mut r) = (live(&w), Rng::new(1, w.day(), 0, Purpose::Choose));
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
            lineage::resize(&mut w, s, &mut b);
            nodes::split(&mut w, s, &mut b);
            w.life.bodies[s] = b;
            // The children are young nodes of their own (immature, `body::class`); here they grow up at once, and the
            // month's merge would take them back into their parent's node: they do now, so the update's node math is the
            // same.
            let kids: Vec<usize> = live(&w).into_iter().filter(|k| *k != s && !before.contains(k) && w.life.bodies[*k].parent == id).collect();
            for k in kids {
                nodes::merge(&mut w, k, s);
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
        println!("crowd {crowd}: bodies {}, splits {}, largest move {worst:.3} steps (gene {which}: {} -> {})", live(&w).len(), w.counts.splits[0], m0[which], m1[which]);
        assert!(crowd < 4 || w.counts.splits[0] > 10.0, "too few splits");
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
    let lay = Layout::of(&w.chem);
    // Every gene's spread at the split threshold, (alike step)² π / 8 + step² (`body::most`).
    let inv = body::steps(&lay, &vec![1.0; lay.len]);
    let a = w.cfg.phys.alike;
    let spread: Vec<f32> = inv.iter().map(|&x| if x > 0.0 { (a * a * std::f32::consts::FRAC_PI_8 + 1.0) / (x * x) } else { 0.0 }).collect();
    let s = live(&w).into_iter().find(|&s| {
        let i = w.life.bodies[s].cell as usize;
        pcore::space::square::DIRS.iter().all(|&d| w.map.step(i, d).is_some_and(|j| !w.life.at[j].is_empty()))
    }).expect("a node with nodes all around");
    // Members mature from 0.65 of their body, all children leave.
    let mut genes = w.life.bodies[s].genes.clone();
    (genes[COPY_MASS], genes[DISPERSE]) = ((0.65 * w.life.bodies[s].member_body(&w.chem) as f32).log10(), 1.0);
    for t in live(&w) {
        (w.life.bodies[t].genes, w.life.bodies[t].spread) = (genes.clone(), spread.clone());
        w.life.bodies[t].k = Default::default();
        // The nodes around are immature (half the matter: a member's body half, under its copy mass at 0.65), as nodes
        // of children are: a child joins no mature node (`body::class`).
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
        w.time += DAY_S;
        assert!(copy::copy(&mut w, s, &mut c), "no children");
        (joined, tries) = (joined + (live(&w).len() == n0) as u32, tries + 1);
        w.events.clear();
    }
    println!("pairs of children that joined the node of their kind: {joined} of {tries}");
    assert!(joined as f64 >= 0.95 * tries as f64, "{joined} of {tries} joined");
}

/// A node whose mean brain-size gene moved a whole unit from its lineage's size moves to a new lineage with that size
/// at its update. A light-eater node (no brain: no born rows, no group) grows one hidden unit: new born rows and a group
/// of its own. Grown once more, its brain is carried over, in a group of its own that keeps what its group learned (the
/// bias of a row is its last entry). Matter and energy as they were.
#[test]
fn a_node_whose_size_genes_moved_changes_lineage() {
    let mut w = small_world();
    worldgen::life::seed_with(&mut w, 0, &worldgen::life::Seeding::alike());
    let books = Books::now(&w);
    let s = live(&w).into_iter().find(|&s| w.life.bodies[s].count > 10.0).expect("a node");
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    let lin0 = b.lineage;
    assert!(w.life.born(&b).is_empty() && b.group == pcore::life::NO_GROUP, "a light-eater has a brain");
    b.genes[HIDDEN] += 0.9;
    lineage::resize(&mut w, s, &mut b);
    assert_eq!(b.lineage, lin0, "moved for less than a unit");
    b.genes[HIDDEN] += 0.3;
    lineage::resize(&mut w, s, &mut b);
    let shape = |w: &World, b: &Body| w.life.brains[b.lineage as usize].as_ref().expect("its lineage").shape;
    let (lin1, g1, s1) = (b.lineage, b.group, shape(&w, &b));
    assert!(lin1 != lin0 && g1 != pcore::life::NO_GROUP && s1.h == 1, "no new lineage of one hidden unit");
    assert_eq!((w.life.members[lin1 as usize], w.life.groups[g1 as usize].members), (1, 1));
    assert_eq!(w.life.born(&b).len(), s1.rows() * s1.r());
    w.life.groups[g1 as usize].rows[s1.h] += 0.5;
    let learned = w.life.groups[g1 as usize].rows[s1.h] - w.life.born(&b)[s1.h];
    b.genes[HIDDEN] += 1.0;
    lineage::resize(&mut w, s, &mut b);
    w.life.bodies[s] = b;
    let b = &w.life.bodies[s];
    let h = shape(&w, b).h;
    assert!(b.lineage != lin1 && b.group != g1 && h == 2, "no new lineage of two hidden units");
    assert!((w.life.groups[b.group as usize].rows[h] - w.life.born(b)[h] - learned).abs() < 1e-5, "what it learned is lost");
    for _ in 0..2 {
        pcore::step(&mut w);
    }
    books.assert_kept(&w, 1e-9);
}

/// Totals over all live bodies: members, each store, heat.
fn held(w: &pcore::World) -> (f64, Vec<f64>, f64) {
    let (mut n, mut store, mut e) = (0.0, vec![0.0; w.chem.ns()], 0.0);
    for b in w.life.alive() {
        n += b.n();
        e += b.e;
        store.iter_mut().zip(&b.store).for_each(|(s, x)| *s += x);
    }
    (n, store, e)
}

/// The stores and heat of two `held` totals are the same (a copy makes members, so they are not compared).
fn same(a: &(f64, Vec<f64>, f64), b: &(f64, Vec<f64>, f64)) {
    assert!((a.2 - b.2).abs() <= 1e-9 * a.2.abs().max(1.0), "heat {} then {}", a.2, b.2);
    for (k, (x, y)) in a.1.iter().zip(&b.1).enumerate() {
        assert!((x - y).abs() <= 1e-9 * x.abs().max(1.0), "store {k}: {x} then {y}");
    }
}

/// Body `s` is seen as what it holds now: its visible traits are those its stores, genes and members give.
fn seen_as_it_is(w: &World, s: usize) {
    let b = &w.life.bodies[s];
    assert_eq!(b.vis, b.visible(&w.chem, &w.cfg), "body {s} is seen as it was");
}

/// `open` draws members of a node as individuals: they are in the node's cell, of count 1, with a wear in [0, 1), and
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
    let s = w.life.add(b, w.day(), 1);
    assert!(w.life.bodies[s].count > 1000.0, "a node");
    // A spread of one step in the copy mass (0.2), in the gas gene (0.3) and in a catalyst gene; and one in the
    // light-capture gene, so each is seen with its own leaf area (near its wall: its noise shrinks, so it is not
    // measured).
    let (lay, genes0) = (Layout::of(&w.chem), w.life.bodies[s].genes.clone());
    let cat = lay.cat + 1;
    let wide = [(COPY_MASS, 0.04f64), (GAS, 0.09), (cat, 0.09)];
    w.life.bodies[s].spread = (0..lay.len).map(|g| wide.iter().find(|x| x.0 == g).map_or(0.0, |x| x.1 as f32)).collect();
    w.life.bodies[s].spread[LEAF] = 0.09;
    // And one of 0.1 in the born rows.
    w.life.bodies[s].row_spread = 0.01;
    let rows0 = w.life.born(&w.life.bodies[s]).to_vec();
    let (before, count0, cell) = (held(&w), w.life.bodies[s].count, w.life.bodies[s].cell);
    let n = 400;
    let drawn = nodes::open(&mut w, s, n);
    assert_eq!(drawn.len(), n);
    same(&before, &held(&w));
    assert!((held(&w).0 - before.0).abs() <= 1e-9 * before.0, "members {} then {}", before.0, held(&w).0);
    drawn.iter().chain([&s]).for_each(|&m| seen_as_it_is(&w, m));
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
    let (mut rows, mut var) = (w.life.born(node).iter().map(|&x| x as f64 * node.count / count0).collect::<Vec<f64>>(), 0.0);
    for b in drawn.iter().map(|&m| &w.life.bodies[m]) {
        for ((r, &x), &x0) in rows.iter_mut().zip(w.life.born(b)).zip(&rows0) {
            (*r, var) = (*r + x as f64 / count0, var + (x as f64 - x0 as f64).powi(2) / (n * rows0.len()) as f64);
        }
    }
    let off = rows.iter().zip(&rows0).map(|(x, &y)| (x - y as f64).abs()).fold(0.0, f64::max);
    assert!(off < 1e-4 && var.sqrt() > 0.08 && var.sqrt() < 0.12, "born rows: mean of all off by {off}, drawn as wide as {}", var.sqrt());
    // A node keeps one member: asked for more than it has, it gives all but one. (Its rows made alike first, so that its
    // many members do not each get rows of their own.)
    w.life.bodies[s].row_spread = 0.0;
    let rest = w.life.bodies[s].count.floor() as usize;
    assert_eq!(nodes::open(&mut w, s, rest + 5).len(), rest - 1);
    assert!(w.life.bodies[s].count < 2.0);
    books.assert_kept(&w, 1e-9);
}

/// Members who join a node are kept, however few: children of less than one member (a node's, `copy::copy`) add what
/// they are to its count, and the genes become the means by members.
#[test]
fn joining_a_node_keeps_members_and_weighs_genes_by_them() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    let t = live(&w).into_iter().find(|&s| w.life.bodies[s].count > 1.0).expect("a node");
    w.life.bodies[t].count = 3.0;
    let (o, before) = (w.life.bodies[t].clone(), held(&w).0);
    let mut c = o.clone();
    (c.count, c.genes) = (0.4, o.genes.iter().map(|g| g + 0.1).collect());
    let rows = w.life.born(&o).to_vec();
    nodes::join(&mut w, t, c, &rows);
    let (after, node) = (held(&w).0, &w.life.bodies[t]);
    assert!((after - (before + 0.4)).abs() < 1e-12 * after && (node.count - 3.4).abs() < 1e-12, "members {before} + 0.4 then {after}");
    for (g, (x, y)) in o.genes.iter().zip(&node.genes).enumerate() {
        let want = x + 0.1 * 0.4 / 3.4;
        assert!((y - want).abs() < 1e-5 * (1.0 + x.abs()), "gene {g}: {x} then {y}, wanted {want}");
    }
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
        let s = live(&w).into_iter().find(|&s| !pcore::body::light_eater(&w.chem, &w.life.bodies[s].genes)).expect("an animal");
        // A node of its own kind beside it, in its cell; its children stay in their cell.
        let mut node = w.life.bodies[s].clone();
        (node.count, node.genes[DISPERSE]) = (5.0, 0.0);
        node.store.iter_mut().for_each(|x| *x *= 5.0);
        // Young (half the matter of an adult founder), as a node children join is.
        node.store[w.chem.matter().expect("matter")] *= 0.5;
        node.e *= 5.0;
        w.life.bodies[s].genes[DISPERSE] = 0.0;
        let t = w.life.add(node, w.day(), 1);
        // One child: its parent is mature (its copy mass is half its body, `Body::built`, which maturity reads, so it stays
        // mature with the brood out of its body whatever its sprayed `CHILD` gene).
        let (at, before) = (live(&w).len(), held(&w));
        let mut b = std::mem::take(&mut w.life.bodies[s]);
        b.genes[COPY_MASS] = (0.5 * b.built(&w.chem) as f32).log10();
        b.prepare(&w.chem, &w.cfg.phys);
        brood(&w, &mut b, 1.0);
        w.time += DAY_S;
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
        w.time += DAY_S;
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

/// With `Config::nodes` off, more than `crowd` alike animals in one cell stay individuals at the month's start (with
/// nodes on they merge, `crowded_individuals_merge_into_a_node`), and over three years of the whole world no animal
/// becomes a node; light-eaters still are nodes. The animals are hand-set eaters (as `ew run` places them, which find
/// what they eat), so that some live and breed through the window: the check would be empty with none (neutral animal
/// founders mostly die in their first year). Matter and energy are kept.
#[test]
#[ignore = "slow: three years of a 32 x 32 world"]
fn individuals_mode_merges_no_animal() {
    let mut w = worldgen::new_world(1, 32, 32);
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
        w.life.add(b, w.day(), 1);
    }
    let n = here(&w);
    pcore::survey::survey(&mut w);
    assert_eq!(here(&w), n, "animals merged");
    // Animals alive at the end of each month, and animals born in the window (members).
    let (mut alive, mut born) = (Vec::new(), 0.0);
    let roots: Vec<u32> = w.life.alive().filter(|b| animal(&w, b)).map(|b| w.life.root(b)).collect();
    w.events.clear();
    for _ in 0..36 {
        pcore::step(&mut w);
        assert!(w.life.alive().filter(|b| animal(&w, b)).all(|b| b.count == 1.0), "an animal became a node");
        born += w.events.iter().filter(|e| matches!(e.kind, EventKind::Born { root, .. } if roots.contains(&root))).map(|e| e.value).sum::<f64>();
        w.events.clear();
        alive.push(w.life.alive().filter(|b| animal(&w, b)).count());
    }
    println!("animals alive at the end of each month: {alive:?}; born in the window {born}");
    assert!(alive.last().is_some_and(|&a| a > 0), "no animal was alive at the end: {alive:?}");
    assert!(born > 0.0, "no animal was born in the window: {alive:?}");
    assert!(w.life.alive().any(|b| !animal(&w, b) && b.count > 1.0), "no light-eater node");
    books.assert_kept(&w, 1e-9);
}
