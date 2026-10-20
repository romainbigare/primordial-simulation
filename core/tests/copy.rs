//! Law 4 (Copy): growth builds body matter, a mature body fills a brood and a child is made of it, and a hurt body
//! rebuilds. Two parents and population nodes: `populations.rs`.

use pcore::body::{Body, Layout, ALLOC, CHILD, COOL_T, COPY_MASS, DENATURE, DISPERSE, GAS, HEAT, KEEP_T, LIQ, MUT, STRENGTH};
use pcore::rng::{Purpose, Rng};
use pcore::units::DAY_S;
use pcore::{body, brain, copy, World};
use worldgen::testkit::{self, add, brood, life_totals, same_totals, children, eater, kg, only_the_solid, out, put, put_kg_m2, scale, start, took_to_burn, Books, BURN, DISSOLVED, LIQUID, MATTER, RICH};
use pcore::store::Class;

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
        b.prepare(&w.chem, &w.cfg.phys);
        let a = (100.0 * b.level(&w.chem, mk, b.mass(&w.chem)) - w.store.amounts(Class::Ground)[mk * n + i]).max(0.0);
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
    let (matter0, rich0, liquid0, e0) = (b.store[MATTER], b.store[RICH], b.store[LIQUID], w.store.heat(Class::Ground).iter().sum::<f64>());
    let minerals = w.chem.minerals().to_vec();
    let mineral0: Vec<f64> = minerals.iter().map(|m| b.store[m.0]).collect();
    // (Counts of the chemistry's growth: the solid in, the liquid out.)
    let bl = w.chem.build.as_ref().expect("growth");
    let (n_rich, n_liquid) = (bl.inputs.iter().find(|p| p.0 as usize == RICH).expect("the solid").1 as f64, bl.outputs.iter().find(|p| p.0 as usize == LIQUID).expect("the liquid").1 as f64);
    let days = b.interval(&w) as f64;
    assert!(testkit::moved(&mut w, (b.cell as usize, 0), |w| testkit::grow(w, &mut b, days)), "it did not grow");
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
    let heat = w.store.heat(Class::Ground).iter().sum::<f64>() - e0;
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
        let room = w.cfg.phys.c_build * b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem)) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
        let (built0, grew) = (b.built(&w.chem), testkit::moved(&mut w, (b.cell as usize, 0), |w| testkit::grow(w, &mut b, days)));
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
        let grew = testkit::moved(&mut w, (b.cell as usize, 0), |w| testkit::grow(w, &mut b, days));
        let built = (b.store[MATTER] - m0) * w.chem.subs[MATTER].molar_mass;
        let (up, release) = (body::upkeep(&w, &b, days, b.mass(&w.chem), b.temp(&w.chem)), testkit::release(&w, &b, days));
        let room = w.cfg.phys.c_build * b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem)) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
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
    assert!(!testkit::moved(&mut w, (b.cell as usize, 0), |w| testkit::grow(w, &mut b, days)) && b.store == stores && b.e == heat, "a body with no allowance built");

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
    let s = w.add(b, 1);
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
    let (before, ground0, built0, size0, temp0) = (b.store.clone(), w.store.heat(Class::Ground).iter().sum::<f64>(), b.built(&w.chem), b.size(&w.chem), b.temp(&w.chem));
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
    assert_eq!(w.store.heat(Class::Ground).iter().sum::<f64>(), ground0, "work was paid for a child");
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
    // A child is a body of half the copy mass: 5 kg, matter 1.25 kg; the members hold half a child each besides.
    n.brood = (7.3 + copy::BROOD_HELD * 5.0) * 5.0 * (1.0 - h) / mm;
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
    let held = copy::BROOD_HELD * 5.0 * 5.0 * (1.0 - h) / mm;
    assert!((n.brood - held).abs() < 1e-6 * held && (n.built(&w.chem) / n.n() - built0).abs() < 1e-9 * built0, "the brood is not what the members hold or the mean member fell: {built0} -> {}", n.built(&w.chem) / n.n());
    let given = c.store[MATTER] + other.store[MATTER];
    assert!((given - 7.3 * 1.25 / mm).abs() < 1e-6 * given && (before[MATTER] - n.store[MATTER] - given).abs() < 1e-9 * given);
    println!("node: {count0} members of {built0:.2} kg; children {} members of {:.2} kg", c.count, c.built(&w.chem) / c.n());
}

/// A mature node's children never lower its mean under maturity: they gather in a young node of their own (the second
/// brood joins the first, along the link of the two), which keeps the spread of its members' sizes; the monthly merge
/// never takes the young into the adult node. Members, matter and energy are kept.
#[test]
fn a_mature_nodes_children_gather_in_its_young_node() {
    let (mut w, i) = start();
    w.cfg.phys.c_copy = 1e18;
    let mut n = eater(&mut w, i, brain::REST, 10f32.log10());
    // Children of 0.3 of the copy mass: 3 kg a body, a quarter of that matter (h 0.75).
    (n.genes[CHILD], n.genes[DISPERSE]) = (0.3f32.log10(), 0.0);
    scale(&mut w, &mut n, 5.0);
    n.count = 5.0;
    let (mm, h) = (w.chem.subs[MATTER].molar_mass, 0.75);
    let child_mol = 3.0 * (1.0 - h) / mm;
    let s = w.add(n, 1);
    let books = Books::now(&w);
    w.time += DAY_S;
    pcore::survey::survey(&mut w);
    for round in 0..3 {
        let mut n = std::mem::take(&mut w.life.bodies[s]);
        n.prepare(&w.chem, &w.cfg.phys);
        // A brood of 4 children for the members together, besides what they hold.
        n.brood = (4.0 + copy::BROOD_HELD * 5.0) * child_mol;
        assert!(n.adult(&w.chem), "round {round}: the node is mature: {} kg", n.built(&w.chem) / n.n());
        let before = n.built(&w.chem) / n.n();
        assert!(copy::copy(&mut w, s, &mut n), "round {round}: no children");
        assert_eq!(n.count, 5.0);
        // The mean member of the node is what it was, and mature.
        assert!(n.adult(&w.chem) && (n.built(&w.chem) / n.n() - before).abs() < 1e-9 * before, "round {round}: the mean member fell: {before} -> {}", n.built(&w.chem) / n.n());
        w.life.bodies[s] = n;
        w.time += DAY_S;
    }
    // All three broods are in one young node, linked to its adult node; its members are all of one size.
    let kids: Vec<usize> = (0..w.life.bodies.len()).filter(|&k| k != s && w.life.bodies[k].alive).collect();
    assert_eq!(kids.len(), 1, "the children are {} nodes", kids.len());
    let (k, young) = (kids[0], &w.life.bodies[kids[0]]);
    assert!((young.count - 12.0).abs() < 1e-4 && !young.adult(&w.chem), "young: {} members", young.count);
    assert_eq!((young.link, w.life.bodies[s].link), (Some(w.life.bodies[s].id(s)), Some(young.id(k))), "not linked");
    assert!(young.size_spread.abs() < 1e-9, "children of one size have a size spread {}", young.size_spread);
    // A month's merge: alike in genes, but the young are immature.
    let month = w.cfg.calendar().month;
    w.time = (w.time / month + 1) * month;
    pcore::survey::survey(&mut w);
    assert_eq!(w.life.alive().count(), 2, "the young merged into the mature node");
    books.assert_kept(&w, 1e-9);
}

/// Young members mature at the rate their growth gives (`copy::mature`): a young node of 1,000 members whose log10
/// bodies lie evenly over the decade below the copy mass (the steady state of a stage entered at one size, with growth
/// at one pace and no deaths), beside its adult node. Without growth none matures. When every member grows by 0.1
/// decades, the top tenth of them join the adult node, holding the share (1 - 10^-0.1) / (1 - 10^-1) of the body (the
/// bodies 10^x over the decade). Members, stores and heat are kept; the adult node stays mature and the young one is
/// still young.
#[test]
fn young_members_mature_at_the_rate_their_growth_gives() {
    let (mut w, sa, mut y) = young_beside_adult();
    let mm = w.chem.subs[MATTER].molar_mass;
    let top = 10f64.log10();
    y.size_spread = 1.0 / 12.0;
    y.size_lag = ((y.built(&w.chem) / y.count).log10() - (top - 0.5)) as f32;
    let (sy, mut y) = out(&mut w, y);
    assert!(!y.adult(&w.chem) && w.life.bodies[sa].adult(&w.chem));
    let x0 = y.member_body(&w.chem);
    // No growth: nobody matures.
    assert!(matches!(copy::mature(&mut w, sy, &mut y, x0), copy::Grown::Lives) && y.count == 1000.0, "members matured without growth");
    // Every member grows by 0.1 decades: the young node's matter times 10^0.1 (booked as an edit).
    let kg_in = (10f64.powf(0.1) - 1.0) * y.store[MATTER] * mm;
    add(&mut w, &mut y, MATTER, kg_in);
    w.life.bodies[sy] = y;
    let (books, before) = (Books::now(&w), life_totals(&w));
    let mut y = std::mem::take(&mut w.life.bodies[sy]);
    let (n0, matter0) = (y.count, y.store[MATTER]);
    let (members, body) = (0.1, (1.0 - 10f64.powf(-0.1)) / 0.9);
    let adult0 = w.life.bodies[sa].count;
    assert!(matches!(copy::mature(&mut w, sy, &mut y, x0), copy::Grown::Lives));
    let moved = n0 - y.count;
    println!("matured {moved:.3} of {n0} members (wanted {:.3}), {:.4} of the body (wanted {body:.4})", members * n0, 1.0 - y.store[MATTER] / matter0);
    assert!((moved - members * n0).abs() < 1e-5 * n0, "{moved} matured, wanted {}", members * n0);
    assert!((1.0 - y.store[MATTER] / matter0 - body).abs() < 1e-5, "the body share that left");
    assert!((w.life.bodies[sa].count - adult0 - moved).abs() < 1e-9 * n0, "the adult node took in {} members", w.life.bodies[sa].count - adult0);
    assert!(w.life.bodies[sa].adult(&w.chem) && !y.adult(&w.chem), "the stages mixed");
    w.life.bodies[sy] = y;
    same_totals(&before, &life_totals(&w));
    books.assert_kept(&w, 1e-9);
}

/// A world with an adult node of 5 eaters of copy mass 10 kg (its slot), and out of any slot a young node of 1,000
/// members of its kind whose mean body is 6 kg (matter over 1 - 0.75), their sizes not set.
fn young_beside_adult() -> (World, usize, Body) {
    let (mut w, i) = start();
    let mm = w.chem.subs[MATTER].molar_mass;
    let mut a = eater(&mut w, i, brain::REST, 10f32.log10());
    scale(&mut w, &mut a, 5.0);
    a.count = 5.0;
    let mut y = a.clone();
    let sa = testkit::place(&mut w, a);
    scale(&mut w, &mut y, 200.0);
    y.count = 1000.0;
    let kg_in = (0.6 * 10.0 * 0.25 * y.count / mm - y.store[MATTER]) * mm;
    add(&mut w, &mut y, MATTER, kg_in);
    (w, sa, y)
}

/// Any spread of a young node's sizes matures a share of its members and of its body between 0 and 1
/// (`copy::mature`), also one wider than the steady state of a stage can hold (`math::Pass`: then an exponential tail
/// below the copy mass, of their mean): two cohorts far apart, as members who starved for years and newborns make (sd 2
/// decades about a mean 1 decade below the copy mass). The rest keep a mean below the copy mass, and members and books
/// are kept.
#[test]
fn any_spread_of_sizes_matures_a_share_of_the_young() {
    let (mut w, sa, y) = young_beside_adult();
    let mm = w.chem.subs[MATTER].molar_mass;
    let (sy, mut y) = out(&mut w, y);
    let x0 = y.member_body(&w.chem);
    (y.size_spread, y.size_lag) = (4.0, (x0.log10() - (10f64.log10() - 1.0)) as f32);
    // Every member grows by 0.05 decades (booked as an edit).
    let kg_in = (10f64.powf(0.05) - 1.0) * y.store[MATTER] * mm;
    add(&mut w, &mut y, MATTER, kg_in);
    w.life.bodies[sy] = y;
    let books = Books::now(&w);
    let mut y = std::mem::take(&mut w.life.bodies[sy]);
    let (n0, matter0, adult0) = (y.count, y.store[MATTER], w.life.bodies[sa].count);
    assert!(matches!(copy::mature(&mut w, sy, &mut y, x0), copy::Grown::Lives));
    let (moved, body) = (n0 - y.count, 1.0 - y.store[MATTER] / matter0);
    let (m, v) = y.size_moments(&w.chem);
    println!("matured {moved:.3} of {n0} members, {body:.4} of the body");
    assert!(moved > 0.0 && moved < n0 && body > 0.0 && body < 1.0, "matured {moved} members, {body} of the body");
    assert!(m.is_finite() && v.is_finite() && y.e.is_finite() && (m as f64) < 10f64.log10(), "the rest: mean {m}, variance {v}, heat {}", y.e);
    assert!((w.life.bodies[sa].count - adult0 - moved).abs() < 1e-9 * n0, "the adult node took in {} members", w.life.bodies[sa].count - adult0);
    w.life.bodies[sy] = y;
    books.assert_kept(&w, 1e-9);
}

/// A young node's sizes stay between birth and the copy mass (`copy::copy`, `copy::mature`): members are born at one
/// size, grow alike and leave at the copy mass. An adult node of copy mass 10 kg has 10 children of 1 kg each round,
/// and its young members grow by 0.02 decades a round, so each member is young for 50 rounds. At every round the mean
/// of their log10 bodies lies between 0 and 1 and their variance is at most 1/4, that of half at each end (Popoviciu
/// 1935); once steady, as many mature as are born, so the young are about 10 x 50 members (Little 1961).
#[test]
fn a_young_nodes_sizes_stay_between_birth_and_the_copy_mass() {
    let (mut w, i) = start();
    w.cfg.phys.c_copy = 1e18;
    let mm = w.chem.subs[MATTER].molar_mass;
    let mut n = eater(&mut w, i, brain::REST, 10f32.log10());
    (n.genes[CHILD], n.genes[DISPERSE]) = (0.1f32.log10(), 0.0);
    scale(&mut w, &mut n, 5.0);
    n.count = 5.0;
    let s = w.add(n, 1);
    let mut young = None;
    for round in 0..300 {
        let mut n = std::mem::take(&mut w.life.bodies[s]);
        // A brood of 10 children besides what the members hold, and matter that keeps them at the copy mass.
        n.prepare(&w.chem, &w.cfg.phys);
        brood(&w, &mut n, 10.0);
        let short = (2.5 * n.count - n.matter_kg(&w.chem)).max(0.0);
        add(&mut w, &mut n, MATTER, short);
        n.prepare(&w.chem, &w.cfg.phys);
        assert!(copy::copy(&mut w, s, &mut n), "round {round}: no children");
        w.life.bodies[s] = n;
        let y = *young.get_or_insert_with(|| children(&w, w.life.bodies[s].id(s)).into_iter().find(|&k| k != s).expect("a young node"));
        let mut b = std::mem::take(&mut w.life.bodies[y]);
        let before = b.member_body(&w.chem);
        let grown = (10f64.powf(0.02) - 1.0) * b.store[MATTER] * mm;
        add(&mut w, &mut b, MATTER, grown);
        assert!(matches!(copy::mature(&mut w, y, &mut b, before), copy::Grown::Lives), "round {round}: the young left");
        let (m, v) = b.size_moments(&w.chem);
        assert!((-1e-6..=1.0).contains(&m) && v <= 0.25, "round {round}: mean {m} and variance {v} of log10 kg");
        if round >= 200 {
            assert!((b.count / 500.0 - 1.0).abs() < 0.1, "round {round}: {} young", b.count);
        }
        w.life.bodies[y] = b;
        w.time += DAY_S;
    }
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
        assert!(testkit::moved(&mut w, (b.cell as usize, 0), |w| testkit::grow(w, &mut b, days)), "it did not build");
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
    let s = w.add(b, 1);
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
    testkit::moved(&mut w, (i, mv), |w| pcore::choose::wear(w, i, mv, e));
    let mut b = std::mem::take(&mut w.life.bodies[v]);
    assert!(!b.adult(&w.chem), "a body torn by 40% is still mature: {} kg", b.built(&w.chem));
    let days = b.interval(&w) as f64;
    // (Counted in all the matter it holds: the update that crosses maturity puts a share of what it builds into a brood.)
    let (mut last, mut updates) = (kg(&w, &b, MATTER), 0);
    while !b.adult(&w.chem) && updates < 2000 {
        let room = w.cfg.phys.c_build * b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem)) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
        assert!(testkit::moved(&mut w, (b.cell as usize, 0), |w| testkit::grow(w, &mut b, days)), "it did not grow after {updates} updates");
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
        let s = w.add(b, 1);
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

/// The temperature the ground of the growing body is held at, K: mild, under the 295 K over which it evaporates.
const GROWTH_GROUND_K: f64 = 289.0;

/// Body size by an energy ceiling, not a cap: what a body takes goes with its mass^0.75 (`Body::equiv`) and the cost
/// of keeping its tissue with its body^1.0 (`body::upkeep`), so growth, which builds only from the spare after upkeep
/// (`copy::grow`), stops where the two meet, a size the body's own costs set. An eater (ALLOC 0: it builds only for
/// itself; strength 1; building fast, so that the allowance never binds) in a place with all the fuel it can take, at
/// every update, for 360 days. With a quarter more than the starting costs of tissue the ceiling is near its size: it
/// levels off (under a quarter more body between the 100th and the 300th day) and in its last 100 days takes about what
/// it pays to keep what it has; with a quarter of the starting costs, a ceiling many times higher, it is still growing
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
        // The eater founder's size of maturity: tissue costs per kg go by it across kinds (Kleiber). It does not die of age.
        (b.genes[COPY_MASS], w.cfg.phys.e_life) = (1.645, 1e30);
        let (s, b0) = out(&mut w, b);
        w.life.bodies[s] = b0;
        let area = w.mesh.cells().area()[0];
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
            // (an eater has them only from the matter it digests, and here eats none). Its ground is held at
            // `GROWTH_GROUND_K`, so the heat it can shed is the body's, not the test world's climate.
            feed_minerals(&mut w, s);
            testkit::hold_ground(&mut w, i, GROWTH_GROUND_K);
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
            ups[day] = body::upkeep(&w, &b, 1.0, b.mass(&w.chem), b.temp(&w.chem));
            if day % 50 == 49 {
                built.push(b.built(&w.chem));
            }
        }
        let ratio = takes[260..].iter().sum::<f64>() / ups[260..].iter().sum::<f64>();
        println!("costs x {cost}: body (kg) every 50 days {built:.1?}; in the last 100 days it took {ratio:.2} times its upkeep");
        (built, ratio)
    };
    let (full, r_full) = run(1.25);
    let (cheap, r_cheap) = run(0.25);
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
    assert!(testkit::moved(&mut w, (b.cell as usize, 0), |w| testkit::grow(w, &mut b, days)));
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
    assert!(testkit::moved(&mut w, (with.cell as usize, 0), |w| testkit::grow(w, &mut with, days)), "it did not grow with the minerals");
    let made = with.store[m] - m0;
    for &(k, per) in &minerals {
        assert!(made > 0.0 && ((b.store[k] - with.store[k]) / (per * made) - 1.0).abs() < 1e-9, "{made} mol of matter for {} of substance {k}", b.store[k] - with.store[k]);
    }
    for &(k, per) in &minerals {
        // Without it, none: not a gram of matter.
        let mut b = b.clone();
        b.store[k] = 0.0;
        assert!(!testkit::moved(&mut w, (b.cell as usize, 0), |w| testkit::grow(w, &mut b, days)), "it grew without substance {k}");
        assert_eq!(b.store[m], m0);
        // A little of it, a little matter: it binds, in proportion.
        b.store[k] = 0.5 * made * per;
        assert!(testkit::moved(&mut w, (b.cell as usize, 0), |w| testkit::grow(w, &mut b, days)));
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
        w.add(b, 1);
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
        w.add(b, 1);
    }
    let here = |w: &pcore::World| w.life.alive().filter(|b| b.cell as usize == i).map(|b| b.n()).collect::<Vec<f64>>();
    let before: f64 = here(&w).iter().sum();
    pcore::survey::survey(&mut w);
    let after = here(&w);
    assert!(after.len() <= w.cfg.crowd, "{} nodes left", after.len());
    assert!((after.iter().sum::<f64>() - before).abs() < 1e-9 * before, "members {before} then {after:?}");
    books.assert_kept(&w, 1e-9);
}
