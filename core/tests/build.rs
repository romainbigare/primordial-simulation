//! Growth is building body matter (14 Oct 2026, decision after watch report 7): a body is its
//! matter and the liquid it keeps with it (`Body::size`), not the fuel it stores; growth turns the
//! energy-rich solid into matter and costs `c_copy` per kg of matter in work, from the `c_build`
//! allowance and what the body can spare (`copy::grow`); storing fuel costs nothing; a mature body
//! puts the share `ALLOC` of what it builds into a brood and a child is made of the brood (`copy::copy`),
//! at the size its `CHILD` gene sets, with provisions in its parent's proportions, and costs its parent
//! no more work (14 Oct 2026, reproduction by surplus); satiety is off by default (a store is bounded by
//! the body, `Body::level`); dead body matter on the ground oxidises at the rate chemistry 0 sets.

use pcore::body::{Body, Layout, ALLOC, CHILD, COPY_MASS, DISPERSE, GAS, HEAT, LIQ, STRENGTH};
use pcore::rng::{Purpose, Rng};
use pcore::{body, brain, change, choose, copy, flow, World};

/// A land cell with dry ground.
fn dry_land(w: &World) -> usize {
    (0..w.grid.n()).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land")
}

/// What chemistry 0 calls the roles: the energy-rich solid, the liquid, body matter, and the
/// reaction that burns the solid.
struct Roles {
    rich: usize,
    liquid: usize,
    matter: usize,
    burn: usize,
}

fn roles(w: &World) -> Roles {
    let (_, rich, liquid, burn) = worldgen::life::roles(&w.chem).expect("roles");
    Roles { rich, liquid, matter: w.chem.matter().expect("body matter"), burn }
}

/// A starting eater (30 kg: 9 kg of fuel, 15.75 kg of liquid, 5.25 kg of body matter, a body of
/// 21 kg) of a brain that only does fixed action `act` (0 rests, 3 takes from the ground), weak
/// (strength 0), staying where it is, soaking no liquid up, copying at 10^`copy` kg of body.
fn eater(w: &mut World, i: usize, act: usize, copy: f32) -> Body {
    let shape = choose::shape(w.chem.ns(), 12, 6);
    let mut fixed = [-8.0; brain::FIXED];
    fixed[act] = 8.0;
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, fixed, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let (l, g) = (w.life.new_lineage(body::Lineage { shape, seen: 3, sight: 1, base, root: 0 }), w.life.new_genome(rows));
    let mut b = worldgen::life::make(w, 1, i, l, g).expect("body");
    (b.genes[DISPERSE], b.genes[STRENGTH], b.genes[COPY_MASS], b.genes[LIQ]) = (0.0, 0.0, copy, -4.0);
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

/// `kg` of substance `k` added to a body at its temperature (booked as an edit), or taken out if
/// negative.
fn add(w: &mut World, b: &mut Body, k: usize, kg: f64) {
    let (a, t) = (kg / w.chem.subs[k].molar_mass, b.temp(&w.chem));
    w.chem.add_elems(k, a, &mut w.ledger.imposed_elems);
    w.ledger.imposed_energy += a * (w.chem.subs[k].hf + w.chem.subs[k].cp * t);
    b.store[k] += a;
    b.e += a * w.chem.subs[k].cp * t;
}

/// A body whose roots take the minerals growth is built of (the uptake genes at 0.01, so that a meal is still nearly all fuel; soaking at least a tenth a day, `body::live`), and the ground of its cell
/// given a hundred times the level it keeps of each (booked as an edit): what a test that builds for many days needs to be
/// free of the minerals. (The rock salt falls out of the liquid inside a body within days, so a store given from outside
/// is gone by the next update; roots give it at the update, just before the body builds.)
fn feed_minerals(w: &mut World, s: usize) {
    let (i, n, lay) = (w.life.bodies[s].cell as usize, w.grid.n(), Layout::of(&w.chem));
    for (mk, _) in w.chem.minerals() {
        let b = &mut w.life.bodies[s];
        (b.genes[lay.upt + mk], b.genes[LIQ]) = (0.01, b.genes[LIQ].max(-1.0));
        b.k.cat.clear();
        let a = (100.0 * b.level(&w.chem, mk, b.mass(&w.chem)) - w.ground[mk * n + i]).max(0.0);
        let t = w.t_ground(i);
        w.ground[mk * n + i] += a;
        w.chem.add_elems(mk, a, &mut w.ledger.imposed_elems);
        let e = w.energy_ground(i, t);
        (w.ledger.imposed_energy, w.e_ground[i]) = (w.ledger.imposed_energy + e - w.e_ground[i] + a * w.chem.subs[mk].hf, e);
    }
}

/// One day: the flow and change steps when due, then the bodies due.
fn day(w: &mut World) {
    let dt = w.cfg.phys_days as f64 * 86_400.0;
    if (w.day % w.cfg.days_per_month as u64).is_multiple_of(w.cfg.phys_days as u64) {
        body::survey(w);
        flow::step(w, dt);
        change::step(w, dt);
    }
    w.events.clear();
    body::day(w);
    w.day += 1;
}

fn start() -> (World, usize) {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let i = dry_land(&w);
    (w, i)
}

/// A body placed and taken out again, prepared, to call the laws on directly.
fn out(w: &mut World, b: Body) -> (usize, Body) {
    let s = w.life.add(b, w.day, 1);
    body::survey(w);
    w.day += 1;
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    b.prepare(&w.chem, &w.cfg.phys);
    (s, b)
}

fn kg(w: &World, b: &Body, k: usize) -> f64 {
    b.store[k] * w.chem.subs[k].molar_mass
}

/// Size is the body: its matter over 1 - h (h the share of the liquid it keeps, its keep gene below
/// 0.95), and no more than its matter and all its liquid; the fuel it stores and liquid past the
/// share it keeps are not body. Leaf (`visible`) and the open surface follow it, and so does the
/// height order of `shade`: a body of more matter and less mass is the taller.
#[test]
fn size_goes_by_body_not_by_fuel_or_free_liquid() {
    let (mut w, i) = start();
    let r = roles(&w);
    let lay = Layout::of(&w.chem);
    let mut b = eater(&mut w, i, 0, 6.0);
    // 4 kg of matter and 12 kg of liquid: the liquid is the share 0.75 the keep gene sets, so a body of 16 kg.
    b.store.iter_mut().for_each(|x| *x = 0.0);
    b.store[r.matter] = 4.0 / w.chem.subs[r.matter].molar_mass;
    b.store[r.liquid] = 12.0 / w.chem.subs[r.liquid].molar_mass;
    b.genes[lay.keep + r.liquid] = 0.75;
    b.wet = 1 << r.liquid;
    let size = |w: &World, b: &Body| b.size(&w.chem);
    assert!((size(&w, &b) - 16.0).abs() < 1e-9, "matter 4 kg over 1 - 0.75: {}", size(&w, &b));
    // Fuel is not body, however much of it: nor is liquid past the share kept.
    add(&mut w, &mut b, r.rich, 100.0);
    add(&mut w, &mut b, r.liquid, 50.0);
    assert!((size(&w, &b) - 16.0).abs() < 1e-9, "fuel and free liquid add no size: {}", size(&w, &b));
    assert!(b.mass(&w.chem) > 160.0);
    // A body short of liquid is as big as it is wet.
    add(&mut w, &mut b, r.liquid, -55.0);
    assert!((size(&w, &b) - 11.0).abs() < 1e-9, "matter 4 kg and 7 kg of liquid: {}", size(&w, &b));
    // The keep gene above 0.95 counts as 0.95: matter over 0.05.
    b.genes[lay.keep + r.liquid] = 1.0;
    add(&mut w, &mut b, r.liquid, 100.0);
    assert!((size(&w, &b) - 80.0).abs() < 1e-9, "matter 4 kg over 1 - 0.95: {}", size(&w, &b));
    // No key liquid: the body is its matter.
    b.wet = 0;
    assert!((size(&w, &b) - 4.0).abs() < 1e-9, "{}", size(&w, &b));
    // A chemistry with no body matter: the whole mass.
    let mut plain = w.chem.clone();
    plain.build = None;
    assert!((b.size(&plain) - b.mass(&plain)).abs() < 1e-9);

    // Leaf and open surface follow the size: the same body with 100 kg more fuel has the same leaf.
    let (mut a, mut c) = (eater(&mut w, i, 0, 6.0), eater(&mut w, i, 0, 6.0));
    (a.genes[body::LEAF], c.genes[body::LEAF]) = (0.0, 0.0);
    add(&mut w, &mut c, r.rich, 100.0);
    a.prepare(&w.chem, &w.cfg.phys);
    c.prepare(&w.chem, &w.cfg.phys);
    let (va, vc) = (a.visible(&w.chem, &w.cfg.phys), c.visible(&w.chem, &w.cfg.phys));
    assert!(va[1] > 0.0 && va[1] == vc[1], "leaf {} and {}", va[1], vc[1]);
    assert!(vc[0] > 4.0 * va[0], "the mass differs: {} and {}", va[0], vc[0]);
    let (ma, mc) = (a.mass(&w.chem), c.mass(&w.chem));
    assert!((a.open_area(&w.chem, &w.cfg.phys, ma) - c.open_area(&w.chem, &w.cfg.phys, ma)).abs() < 1e-9 * ma, "the open surface, with a mass of {ma} kg");
    let _ = mc;

    // Height: two nodes in one cell, B of 3 times the matter of A but lighter (A holds 100 kg of fuel
    // a member more): by mass A is the taller, by body B. Leaf area is set so that each catches a
    // good part of the cell's light and the layers differ.
    let n = 100.0;
    let (mut ha, mut hb) = (eater(&mut w, i, 0, 6.0), eater(&mut w, i, 0, 6.0));
    (ha.genes[body::LEAF], hb.genes[body::LEAF]) = (4.0, 4.0);
    scale(&mut w, &mut hb, 3.0 * n);
    scale(&mut w, &mut ha, n);
    add(&mut w, &mut ha, r.rich, 100.0 * n);
    (ha.count, hb.count) = (n, n);
    let (sa, sb) = (w.life.add(ha, w.day, 1), w.life.add(hb, w.day, 1));
    for s in [sa, sb] {
        let mut x = std::mem::take(&mut w.life.bodies[s]);
        x.prepare(&w.chem, &w.cfg.phys);
        x.vis = x.visible(&w.chem, &w.cfg.phys);
        w.life.bodies[s] = x;
    }
    // (Not at a month's start: alike nodes of a cell merge then.)
    w.day += 1;
    body::survey(&mut w);
    let (a, b) = (&w.life.bodies[sa], &w.life.bodies[sb]);
    let (ma, mb) = (a.mass(&w.chem) / n, b.mass(&w.chem) / n);
    println!("shade: A (member mass {ma:.0} kg, body {:.1}) {:.3}, B (member mass {mb:.0} kg, body {:.1}) {:.3}; leaf per ground area {:.2} and {:.2}", a.size(&w.chem) / n, w.life.shade[sa], b.size(&w.chem) / n, w.life.shade[sb], a.vis[1] as f64 * n / w.grid.area(), b.vis[1] as f64 * n / w.grid.area());
    assert!(ma > 1.3 * mb && a.size(&w.chem) < b.size(&w.chem), "A is heavier and smaller in body");
    assert!(w.life.shade[sb] > 1.2 * w.life.shade[sa], "the body of more matter is the taller: shade {} against {}", w.life.shade[sb], w.life.shade[sa]);
}

/// Growth makes body matter of the energy-rich solid, the nitrogen mineral and the rock salt, 118 of the solid, 3 of the mineral and 1 of
/// the salt for 1 of matter and 27 of liquid (15 Oct 2026; 40, 1 and none for 1 and 9 before the rock salt, 6 of the solid for 1 of matter
/// and 1 of liquid before the mineral), and pays `c_copy` per kg of matter made in work that burns the solid (the fuel's heat leaves the
/// body for the ground): elements and energy are kept exactly, and the stores change as the
/// chemistry says: matter up by x, the solid down by 118 x and by what the work burnt, each mineral down by its count times x, the liquid up
/// by 27 x and by the liquid the burning made.
#[test]
fn growth_makes_matter_of_the_solid_and_keeps_elements_and_energy() {
    let (mut w, i) = start();
    let r = roles(&w);
    w.cfg.phys.c_build = 1e6;
    let b = eater(&mut w, i, 0, 6.0);
    let (_, mut b) = out(&mut w, b);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let (matter0, rich0, liquid0, e0) = (b.store[r.matter], b.store[r.rich], b.store[r.liquid], w.e_ground.iter().sum::<f64>());
    let minerals = w.chem.minerals();
    let mineral0: Vec<f64> = minerals.iter().map(|m| b.store[m.0]).collect();
    // (Counts of the chemistry's growth: the solid in, the liquid out.)
    let bl = w.chem.build.as_ref().expect("growth");
    let (n_rich, n_liquid) = (bl.inputs.iter().find(|p| p.0 as usize == r.rich).expect("the solid").1 as f64, bl.outputs.iter().find(|p| p.0 as usize == r.liquid).expect("the liquid").1 as f64);
    assert!(n_rich == 118.0 && n_liquid == 27.0 && minerals.iter().map(|m| m.1).collect::<Vec<_>>() == [3.0, 1.0]);
    let days = b.interval() as f64;
    assert!(copy::grow(&mut w, &mut b, days), "it did not grow");
    let (x, c_copy) = (b.store[r.matter] - matter0, w.cfg.phys.c_copy);
    assert!(x > 0.0);
    // The work: c_copy per kg of matter, burnt as the solid is (heat per mol of the burning reaction).
    let burnt = c_copy * x * w.chem.subs[r.matter].molar_mass / -w.chem.dh(r.burn);
    let (dsolid, dliquid) = (rich0 - b.store[r.rich], b.store[r.liquid] - liquid0);
    let dmineral: Vec<f64> = minerals.iter().zip(&mineral0).map(|(m, a)| a - b.store[m.0]).collect();
    println!("matter made {x:.4} mol ({:.3} kg), solid down {dsolid:.4} mol (118 x = {:.4}, burnt {burnt:.4}), minerals down {dmineral:.4?}, liquid up {dliquid:.4}", x * w.chem.subs[r.matter].molar_mass, 118.0 * x);
    assert!((dsolid / (n_rich * x + burnt) - 1.0).abs() < 1e-6, "the solid fell by {dsolid}, 118 x + burnt = {}", n_rich * x + burnt);
    for (m, d) in minerals.iter().zip(&dmineral) {
        assert!((d / (m.1 * x) - 1.0).abs() < 1e-9, "substance {} fell by {d}, x = {x}", m.0);
    }
    // (The burning of the solid for the work makes liquid too, as its reaction says.)
    let burn_liquid = w.chem.reactions[r.burn].outputs.iter().find(|p| p.0 as usize == r.liquid).map_or(0.0, |p| p.1 as f64);
    assert!((dliquid / (n_liquid * x + burn_liquid * burnt) - 1.0).abs() < 1e-6, "the liquid rose by {dliquid}, 27 x + burnt = {}", n_liquid * x + burn_liquid * burnt);
    // The work left the body as heat for the ground: that heat is the burnt solid's, less the share the body keeps while it leaves
    // (4.7% before the rock salt and 5.1% after it, the body holding a little more of the heat: the old bound of 5% was at the edge).
    let heat = w.e_ground.iter().sum::<f64>() - e0;
    assert!(heat > 0.0 && (heat / (burnt * -w.chem.dh(r.burn)) - 1.0).abs() < 0.06, "heat to the ground {heat:e} J, the work {:e} J", burnt * -w.chem.dh(r.burn));
    // Growth is thermoneutral: the matter holds the energy of the solid it was made of.
    assert!(w.chem.dh_build().abs() < 1.0, "growth takes {} J", w.chem.dh_build());
    w.life.bodies[0] = b;
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Building takes time: with fuel and spare in plenty, a body builds `c_build` x its `equiv` /
/// `m_ref` x the days of its interval over `c_copy` kg of matter in an update, no more; twice the
/// allowance builds twice the matter. A mature body (its body at or past its copy mass) builds as much
/// and puts the share `ALLOC` of it into its brood: at 1 its body stays what it was.
#[test]
fn growth_is_limited_by_c_build() {
    let built = |c_build: f64, copy: f32, alloc: f32| {
        let (mut w, i) = start();
        let r = roles(&w);
        w.cfg.phys.c_build = c_build;
        let mut b = eater(&mut w, i, 0, copy);
        b.genes[ALLOC] = alloc;
        scale(&mut w, &mut b, 5.0);
        let (s, mut b) = out(&mut w, b);
        let m0 = b.store[r.matter];
        let days = b.interval() as f64;
        let room = w.cfg.phys.c_build * b.supply(&w.chem, &w.cfg.phys) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
        let (built0, grew) = (b.built(&w.chem), copy::grow(&mut w, &mut b, days));
        let kg_matter = (b.store[r.matter] - m0) * w.chem.subs[r.matter].molar_mass;
        let (brood, built1) = (b.brood * w.chem.subs[r.matter].molar_mass, b.built(&w.chem));
        w.life.bodies[s] = b;
        (grew, kg_matter, room, brood, built1 - built0)
    };
    let (g1, one, room1, brood, _) = built(1e4, 6.0, 0.8);
    let (g2, two, room2, _, _) = built(2e4, 6.0, 0.8);
    println!("built {one:.5} kg of matter with c_build 1e4 (allowance {room1:.5} kg), {two:.5} kg with 2e4 ({room2:.5})");
    assert!(g1 && g2);
    assert!((one / room1 - 1.0).abs() < 1e-3 && (two / room2 - 1.0).abs() < 1e-3, "the allowance binds: {one} of {room1}, {two} of {room2}");
    assert!((two / one - 2.0).abs() < 0.01, "twice the allowance, twice the matter: {one} and {two}");
    // A body below its copy mass is immature: all it builds is its own growth.
    assert_eq!(brood, 0.0, "an immature body put {brood} kg into a brood");
    // A mature one (matter 5 x 5.25 kg over 0.25 = 105 kg of body; copy mass 10^1.0 = 10 kg) builds as much; with ALLOC 1 all of it is
    // brood and its body stays; with 0.8 a fifth of it is body.
    let (grew, kg_matter, room, brood, grown) = built(1e4, 1.0, 1.0);
    assert!(grew && (kg_matter / room - 1.0).abs() < 1e-3, "a mature body built {kg_matter} kg of {room}");
    assert!((brood / kg_matter - 1.0).abs() < 1e-9 && grown.abs() < 1e-9 * 105.0, "ALLOC 1: brood {brood} of {kg_matter} kg, its body grew by {grown} kg");
}

/// Building takes what the body can spare: after what it pays and converts, its reactions must
/// still be able to release its upkeep over an interval. A body
/// with little fuel builds less than its allowance and keeps that; one with none builds nothing.
#[test]
fn growth_is_limited_by_what_the_body_can_spare() {
    let (mut w, i) = start();
    let r = roles(&w);
    w.cfg.phys.c_build = 1e9;
    let rich_kg = |keep: f64| {
        let mut b = eater(&mut w.clone(), i, 0, 6.0);
        // (the solid taken out is not booked: only its use is measured, on a clone of the world)
        let kgs = b.store[r.rich] * w.chem.subs[r.rich].molar_mass;
        let _ = (&mut b, kgs);
        keep
    };
    let _ = rich_kg;
    let mut results = Vec::new();
    for keep in [0.0, 0.08, 0.15, 1.0] {
        let mut w = w.clone();
        let mut b = eater(&mut w, i, 0, 6.0);
        // (This measures the spare rule: the solid's minimum, the reserve growth keeps, is set to
        // the 0.02 of the body it was before the starting kinds got 0.25, so it does not bind.)
        b.genes[Layout::of(&w.chem).min + r.rich] = 0.02;
        let out_kg = (1.0 - keep) * kg(&w, &b, r.rich);
        add(&mut w, &mut b, r.rich, -out_kg);
        let (s, mut b) = out(&mut w, b);
        let days = b.interval() as f64;
        let m0 = b.store[r.matter];
        let grew = copy::grow(&mut w, &mut b, days);
        let built = (b.store[r.matter] - m0) * w.chem.subs[r.matter].molar_mass;
        let (up, release) = (body::upkeep(&w, &b, days, b.mass(&w.chem)), body::release(&w, &b, days));
        let room = w.cfg.phys.c_build * b.supply(&w.chem, &w.cfg.phys) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
        println!("fuel kept {keep}: built {built:.4} kg of matter (allowance {room:.3} kg), release {release:.3e} J against upkeep {up:.3e} J");
        w.life.bodies[s] = b;
        results.push((keep, grew, built, room, up, release));
    }
    // With no fuel it builds nothing; with some, less than the allowance and still able to pay its upkeep; with a lot, the allowance (or its wanting).
    assert!(!results[0].1 && results[0].2 == 0.0, "built with no fuel: {:?}", results[0]);
    assert!(results[1].1 && results[1].2 < 0.5 * results[1].3, "{:?}", results[1]);
    for &(_, grew, _, _, up, release) in &results[1..] {
        assert!(!grew || release >= 0.99 * up, "it cannot pay its upkeep after growing: release {release:e}, upkeep {up:e}");
    }
    assert!(results[2].2 > results[1].2 && results[3].2 > results[2].2, "more fuel, more built: {results:?}");
}

/// Storing fuel costs no building work: a body with no allowance builds nothing however much fuel it
/// holds (`copy::grow` pays no work and changes no store), and a body that takes the energy-rich
/// solid from the ground with no allowance to build with stores it (the solid rises) and its matter
/// and size stay.
#[test]
fn storing_the_solid_costs_no_build() {
    let (mut w, i) = start();
    let r = roles(&w);
    // A mature body (copy mass 10 kg of body, its body 21 kg) with 100 kg more fuel and no allowance to build with: it pays
    // no work for the fuel and builds nothing.
    w.cfg.phys.c_build = 0.0;
    let mut b = eater(&mut w, i, 0, 1.0);
    add(&mut w, &mut b, r.rich, 100.0);
    assert!(b.size(&w.chem) > 20.0);
    let (_, mut b) = out(&mut w, b);
    let (stores, heat, days) = (b.store.clone(), b.e, b.interval() as f64);
    assert!(!copy::grow(&mut w, &mut b, days) && b.store == stores && b.e == heat, "a body with no allowance built");

    // A body taking the solid from the ground, with no allowance to build.
    let (mut w, i) = start();
    w.cfg.phys.c_handle = 0.0;
    w.cfg.phys.c_build = 0.0;
    // A kg of the solid per m² on its ground (booked as an edit).
    let (t, a, n) = (w.t_ground(i), w.grid.area() / w.chem.subs[r.rich].molar_mass, w.grid.n());
    w.ground[r.rich * n + i] += a;
    w.chem.add_elems(r.rich, a, &mut w.ledger.imposed_elems);
    let e = w.energy_ground(i, t);
    (w.ledger.imposed_energy, w.e_ground[i]) = (w.ledger.imposed_energy + e - w.e_ground[i] + a * w.chem.subs[r.rich].hf, e);
    let mut b = eater(&mut w, i, 3, 6.0);
    // (An eater of the solid only: it does not burn body matter, which would be burnt for its upkeep.)
    let lay = Layout::of(&w.chem);
    let (m, burn_matter) = worldgen::life::matter_roles(&w.chem).expect("matter");
    (b.genes[lay.cat + burn_matter], b.genes[lay.upt + m]) = (-1.0, -0.1);
    // Less of the solid than it holds at once, so that it has room to store.
    let out_kg = 0.5 * kg(&w, &b, r.rich);
    add(&mut w, &mut b, r.rich, -out_kg);
    let (rich0, matter0) = (kg(&w, &b, r.rich), kg(&w, &b, r.matter));
    let s = w.life.add(b, w.day, 1);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let mut took = 0.0;
    for _ in 0..30 {
        day(&mut w);
        took += w.events.iter().filter_map(|e| if let pcore::event::EventKind::Took { burn, .. } = e.kind { Some(burn as f64) } else { None }).sum::<f64>();
    }
    let b = &w.life.bodies[s];
    println!("took {took:.3} kg: solid {rich0:.3} -> {:.3} kg, matter {matter0:.3} -> {:.3} kg", kg(&w, b, r.rich), kg(&w, b, r.matter));
    assert!(took > 0.5 && kg(&w, b, r.rich) > rich0, "it stored nothing: took {took} kg, solid {rich0} -> {}", kg(&w, b, r.rich));
    // (Matter does not rise; it oxidises in the open at 0.2 a year, here a month: 1.5%.)
    assert!(kg(&w, b, r.matter) <= matter0 && kg(&w, b, r.matter) > 0.97 * matter0, "matter changed with no allowance: {matter0} -> {}", kg(&w, b, r.matter));
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Body matter taken from outside is digested into the energy-rich solid as it comes in, so it fills
/// the room of the solid (`Body::level`): an eater on ground covered in body matter carries no more of
/// the solid than its keep gene allows per kg of its body. Before (14 Oct 2026, the review of lives)
/// the matter was held only to its own room, nine times the mass, and an eater's solid rose past its
/// level and its mass with it (the heaviest eaters of seeds 1 to 3 held 3 times their level at year 60).
#[test]
fn digested_matter_fills_the_room_of_the_solid() {
    let (mut w, i) = start();
    let r = roles(&w);
    (w.cfg.phys.c_handle, w.cfg.phys.c_build) = (0.0, 0.0);
    // Ten kg of body matter per m² on its ground (booked as an edit).
    let (t, a, n) = (w.t_ground(i), 10.0 * w.grid.area() / w.chem.subs[r.matter].molar_mass, w.grid.n());
    w.ground[r.matter * n + i] += a;
    w.chem.add_elems(r.matter, a, &mut w.ledger.imposed_elems);
    let e = w.energy_ground(i, t);
    (w.ledger.imposed_energy, w.e_ground[i]) = (w.ledger.imposed_energy + e - w.e_ground[i] + a * w.chem.subs[r.matter].hf, e);
    let b = eater(&mut w, i, 3, 6.0);
    let keep = b.gene(Layout::of(&w.chem).keep + r.rich);
    let s = w.life.add(b, w.day, 1);
    let (mut took, mut most) = (0.0, 0.0f64);
    for _ in 0..60 {
        day(&mut w);
        took += w.events.iter().filter_map(|e| if let pcore::event::EventKind::Took { burn, .. } = e.kind { Some(burn as f64) } else { None }).sum::<f64>();
        w.events.clear();
        let b = &w.life.bodies[s];
        if b.alive {
            most = most.max(kg(&w, b, r.rich) / b.size(&w.chem));
        }
    }
    println!("took {took:.2} kg; the most of the solid it held: {most:.3} kg per kg of body (keep gene {keep:.2})");
    assert!(took > 1.0, "it took nothing: {took}");
    // (Within 2%: digesting uses the body's liquid, so its body may shrink a little after the room is found.)
    assert!(most <= keep * 1.02, "the solid rose past its level: {most} kg per kg of body, keep {keep}");
    assert_eq!(pcore::check::broken(&w), None);
}

/// Matter and energy are kept over a child's making, and what a child is made of: matter of the brood
/// at the size its `CHILD` gene sets (`10^CHILD` of its parent's copy mass in body, `Body::built`),
/// provisions of every other store in the proportion its parent's body holds them to its matter, the
/// heat of its parent's temperature, and no work paid (a copy cost nobody could pay and no allowance:
/// the matter was built and paid for as brood). The parent keeps its body: what it built (`built`) does
/// not fall. Births are the brood over a child's matter: whole for an individual (the rest of the brood
/// waits), fractional members for a node.
#[test]
fn a_child_is_made_of_the_brood_at_the_size_its_gene_sets() {
    let (mut w, i) = start();
    let r = roles(&w);
    // (A copy cost nobody could pay, no allowance: a child costs its parent no work.)
    (w.cfg.phys.c_copy, w.cfg.phys.c_build) = (1e18, 0.0);
    // A mature eater: a body of 21 kg, matter 5.25 kg and the liquid it keeps, its copy mass 10 kg; a child is 0.1 of that, a body of 1 kg.
    let mut b = eater(&mut w, i, 0, 10f32.log10());
    b.genes[CHILD] = -1.0;
    let (mm, h) = (w.chem.subs[r.matter].molar_mass, 0.75);
    // A brood of 2.5 children: matter 0.25 kg each (a body of 1 kg is its matter over 1 - h).
    let child_mol = 1.0 * (1.0 - h) / mm;
    b.brood = 2.5 * child_mol;
    let (s, mut b) = out(&mut w, b);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let (before, ground0, built0, size0, temp0) = (b.store.clone(), w.e_ground.iter().sum::<f64>(), b.built(&w.chem), b.size(&w.chem), b.temp(&w.chem));
    assert!(b.adult(&w.chem), "the parent is mature: built {} kg", built0);
    w.day += 1;
    assert!(copy::copy(&mut w, s, &mut b), "no child");
    let kids: Vec<usize> = (0..w.life.bodies.len()).filter(|&k| w.life.bodies[k].alive && w.life.bodies[k].parent == b.id(s)).collect();
    // Two whole children (an individual's), the rest of the brood waits for the next.
    let born: f64 = kids.iter().map(|&k| w.life.bodies[k].count).sum();
    assert_eq!(born, 2.0, "births {born}");
    assert!((b.brood - 0.5 * child_mol).abs() < 1e-9 * child_mol, "the brood left is {} of {}", b.brood, 0.5 * child_mol);
    // A child's body is 10^CHILD of its parent's copy mass: 1 kg a member; its matter was the brood's.
    let mut given = 0.0;
    for &k in &kids {
        let c = &w.life.bodies[k];
        // (Its matter over 1 - h of its parent: its own keep gene is its parent's plus noise.)
        let body = c.store[r.matter] * mm / c.n() / (1.0 - h);
        assert!((body - 1.0).abs() < 1e-9, "a child's body is {body} kg, not 10^CHILD x the copy mass = 1 kg");
        given += c.store[r.matter];
        // Provisions in the proportion the parent's body holds them to its matter (what it holds of matter: 5.25 - its brood).
        let per_mol = c.n() * child_mol / (before[r.matter] - 2.5 * child_mol);
        for q in (0..w.chem.ns()).filter(|&q| q != r.matter) {
            assert!((c.store[q] - per_mol * before[q]).abs() <= 1e-9 * before[q].max(1.0), "substance {q}: child {} of {}", c.store[q], before[q]);
        }
        // As warm as its parent was, with no work paid.
        assert!((c.temp(&w.chem) / temp0 - 1.0).abs() < 1e-9, "the child is as warm as the parent was");
    }
    assert!((given - 2.0 * child_mol).abs() < 1e-9 * child_mol, "the children got {given} mol of matter, not {}", 2.0 * child_mol);
    for q in 0..w.chem.ns() {
        let kids_hold: f64 = kids.iter().map(|&k| w.life.bodies[k].store[q]).sum();
        assert!((b.store[q] + kids_hold - before[q]).abs() <= 1e-9 * before[q].max(1.0), "substance {q} not kept");
    }
    assert_eq!(w.e_ground.iter().sum::<f64>(), ground0, "work was paid for a child");
    // The parent's body does not fall: the matter it built is what it was (the brood left its matter and its brood together), and
    // its size, which waits on the liquid it still holds, is no smaller either.
    println!("parent: built {built0:.3} -> {:.3} kg, size {size0:.3} -> {:.3} kg", b.built(&w.chem), b.size(&w.chem));
    assert!((b.built(&w.chem) - built0).abs() < 1e-9 * built0, "the parent's body fell: {built0} -> {}", b.built(&w.chem));
    assert!(b.size(&w.chem) >= size0 * (1.0 - 1e-9), "the parent's size fell: {size0} -> {}", b.size(&w.chem));
    // With less than a child in the brood it makes none.
    assert!(!copy::copy(&mut w, s, &mut b));
    w.life.bodies[s] = b;
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");

    // A node of five such members: the brood of 7.3 children makes 7.3 members, who are immature and so do not join their
    // mature parents but start a young node of their own; the parents' stores and members stay what they were but for what
    // they gave, and the mean member of the node is what it was.
    let (mut w, i) = start();
    w.cfg.phys.c_copy = 1e18;
    let mut n = eater(&mut w, i, 0, 10f32.log10());
    (n.genes[CHILD], n.genes[DISPERSE]) = (-0.301_029_995_7, 0.0);
    scale(&mut w, &mut n, 5.0);
    n.count = 5.0;
    // A child is a body of half the copy mass: 5 kg, matter 1.25 kg.
    n.brood = 7.3 * 5.0 * (1.0 - h) / mm;
    let (s, mut n) = out(&mut w, n);
    let (before, count0, built0) = (n.store.clone(), n.count, n.built(&w.chem) / n.n());
    assert!(n.adult(&w.chem), "the node is mature: built {} kg a member", n.built(&w.chem) / n.n());
    w.day += 1;
    let at = w.life.alive().count();
    assert!(copy::copy(&mut w, s, &mut n));
    w.life.bodies[s] = std::mem::take(&mut n);
    let n = &w.life.bodies[s];
    let kids: Vec<usize> = (0..w.life.bodies.len()).filter(|&k| k != s && w.life.bodies[k].alive).collect();
    assert_eq!((kids.len(), w.life.alive().count()), (1, at + 2), "the children are a node of their own");
    let c = &w.life.bodies[kids[0]];
    assert!((c.count - 7.3).abs() < 1e-6 && !c.adult(&w.chem), "{} young members", c.count);
    assert_eq!(n.count, count0, "the parent node took its children in");
    assert!(n.brood == 0.0 && (n.built(&w.chem) / n.n() - built0).abs() < 1e-9 * built0, "the brood is left or the mean member fell: {built0} -> {}", n.built(&w.chem) / n.n());
    let given = c.store[r.matter];
    assert!((given - 7.3 * 1.25 / mm).abs() < 1e-6 * given && (before[r.matter] - n.store[r.matter] - given).abs() < 1e-9 * given);
    println!("node: {count0} members of {built0:.2} kg; children {} members of {:.2} kg", c.count, c.built(&w.chem) / c.n());
}

/// A mature node's children never lower its mean under maturity: they gather in a young node of their own
/// (the second brood joins the first), and the young node merges into the adult only once it is mature
/// itself (`copy::near`: the same size class and both mature, or both immature). Grown to the size of the
/// adults' class but under the size of maturity, the young stay apart at the month's merge; mature, they join.
/// Members, matter and energy are kept.
#[test]
fn a_mature_nodes_mean_stays_mature_and_its_young_merge_in_only_once_mature() {
    let (mut w, i) = start();
    let r = roles(&w);
    w.cfg.phys.c_copy = 1e18;
    let mut n = eater(&mut w, i, 0, 10f32.log10());
    // Children of 0.3 of the copy mass: 3 kg a body, a quarter of that matter (h 0.75).
    (n.genes[CHILD], n.genes[DISPERSE]) = (0.3f32.log10(), 0.0);
    scale(&mut w, &mut n, 5.0);
    n.count = 5.0;
    let (mm, h) = (w.chem.subs[r.matter].molar_mass, 0.75);
    let child_mol = 3.0 * (1.0 - h) / mm;
    let s = w.life.add(n, w.day, 1);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    w.day += 1;
    body::survey(&mut w);
    let lay = Layout::of(&w.chem);
    let mut built0 = 0.0;
    for round in 0..3 {
        let mut n = std::mem::take(&mut w.life.bodies[s]);
        n.prepare(&w.chem, &w.cfg.phys);
        // A brood of 4 children for the members together (each mature, 3.2 kg of matter more than it needs)
        n.brood = 4.0 * child_mol;
        if round == 0 {
            built0 = (n.built(&w.chem) / n.n()).max(10.0);
        }
        assert!(n.adult(&w.chem), "round {round}: the node is mature: {} kg", n.built(&w.chem) / n.n());
        let before = n.built(&w.chem) / n.n();
        assert!(copy::copy(&mut w, s, &mut n), "round {round}: no children");
        assert_eq!(n.count, 5.0);
        // The mean member of the node is what it was, and mature.
        assert!(n.adult(&w.chem) && (n.built(&w.chem) / n.n() - before).abs() < 1e-9 * before, "round {round}: the mean member fell: {before} -> {}", n.built(&w.chem) / n.n());
        w.life.bodies[s] = n;
        w.day += 1;
    }
    // All three broods are in one young node.
    let kids: Vec<usize> = (0..w.life.bodies.len()).filter(|&k| k != s && w.life.bodies[k].alive).collect();
    assert_eq!(kids.len(), 1, "the children are {} nodes", kids.len());
    let k = kids[0];
    assert!((w.life.bodies[k].count - 12.0).abs() < 1e-4 && !w.life.bodies[k].adult(&w.chem), "young: {} members", w.life.bodies[k].count);
    let _ = built0;
    let month = w.cfg.days_per_month as u64;
    // A month's merge: alike in genes and in size class (3 kg and the parents' about 20), but the young are immature.
    w.day = (w.day / month + 1) * month;
    body::survey(&mut w);
    assert_eq!(w.life.alive().count(), 2, "the young merged into the mature node");
    // Grown to 8 kg a member (under the copy mass 10 kg, within the size class of the parents' 20): still apart.
    let grow_to = |w: &mut World, k: usize, body: f64| {
        let want = (body * (1.0 - h) * w.life.bodies[k].count - w.life.bodies[k].store[r.matter] * mm).max(0.0);
        let mut c = std::mem::take(&mut w.life.bodies[k]);
        add(w, &mut c, r.matter, want);
        add(w, &mut c, r.liquid, 3.0 * want);
        c.prepare(&w.chem, &w.cfg.phys);
        w.life.bodies[k] = c;
    };
    grow_to(&mut w, k, 8.0);
    assert!(!w.life.bodies[k].adult(&w.chem) && copy::alike_size(&lay, w.cfg.phys.alike, 8.0, w.life.bodies[s].built(&w.chem) / 5.0));
    assert!(!copy::near(&lay, &w.chem, w.cfg.phys.alike, &w.life.bodies[s], &w.life.bodies[k]));
    w.day += month;
    body::survey(&mut w);
    assert_eq!(w.life.alive().count(), 2, "an immature node merged into a mature one");
    // Mature (11 kg), it joins.
    grow_to(&mut w, k, 11.0);
    assert!(w.life.bodies[k].adult(&w.chem));
    w.day += month;
    body::survey(&mut w);
    assert_eq!(w.life.alive().count(), 1, "the mature young did not merge in");
    let all: f64 = w.life.alive().map(|b| b.n()).sum();
    assert!((all - 17.0).abs() < 1e-4, "members {all}");
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// A mature body splits what it builds in an update by `ALLOC`: the share to its brood, the rest its own
/// growth; one below the size of maturity puts everything into its own growth up to that size and splits
/// what it builds past it. Counted in matter: what it made is the matter that rose, the brood is the share of it, the body grew
/// by the rest (matter over 1 - h).
#[test]
fn a_mature_body_splits_its_building_by_alloc() {
    let build = |copy: f32, alloc: f32| {
        let (mut w, i) = start();
        let r = roles(&w);
        w.cfg.phys.c_build = 1e5;
        let mut b = eater(&mut w, i, 0, copy);
        b.genes[ALLOC] = alloc;
        let (s, mut b) = out(&mut w, b);
        let (m0, built0, mm) = (b.store[r.matter], b.built(&w.chem), w.chem.subs[r.matter].molar_mass);
        let days = b.interval() as f64;
        assert!(copy::grow(&mut w, &mut b, days), "it did not build");
        let made = (b.store[r.matter] - m0) * mm;
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
    // Immature: all of it is the body's. Just under maturity (the body lacks 0.2 of what it made), it grows to the size of maturity and splits the rest.
    let (made, ..) = build(1.0, 1.0);
    let (made_i, brood_i, grown_i) = build(30f32.log10(), 1.0);
    assert!((made_i - made).abs() < 1e-9 * made, "building does not depend on maturity: {made_i} and {made}");
    assert_eq!(brood_i, 0.0, "an immature body made brood");
    assert!((grown_i - made).abs() < 1e-9 * made, "an immature body's building is all its own growth: {grown_i} of {made}");
    // Copy mass such that the body (matter 5.25 kg) lacks 0.2 made of the matter of maturity: needs 0.2 x made.
    let mc = (5.25 + 0.2 * made) / (1.0 - 0.75);
    let (m2, brood2, grown2) = build(mc.log10() as f32, 0.5);
    assert!((m2 - made).abs() < 1e-9 * made);
    assert!((grown2 - (0.2 * made + 0.5 * 0.8 * made)).abs() < 1e-6 * made && (brood2 - 0.5 * 0.8 * made).abs() < 1e-6 * made, "crossing maturity: body {grown2}, brood {brood2} of {made}");
}

/// An immature body never reproduces: whatever its brood (a brood it would not have), `copy::copy` makes no child
/// while its body (`Body::built`) is under its copy mass, and over the updates of its life a body that
/// builds fast grows to maturity first and has a brood only from then on.
#[test]
fn an_immature_body_never_reproduces() {
    let brain_less = |w: &mut World, i: usize| {
        let mut b = eater(w, i, 0, 24f32.log10());
        b.genes[CHILD] = -1.0;
        b
    };
    let (mut w, i) = start();
    let r = roles(&w);
    // Copy mass 24 kg of body, a body of 21 kg.
    let mut b = brain_less(&mut w, i);
    let child = 0.1 * 24.0 * (1.0 - 0.75) / w.chem.subs[r.matter].molar_mass;
    b.brood = 3.0 * child;
    let (s, mut b) = out(&mut w, b);
    assert!(!b.adult(&w.chem));
    let at = w.life.alive().count();
    w.day += 1;
    assert!(!copy::copy(&mut w, s, &mut b) && w.life.alive().count() == at, "an immature body had children");
    // Updates: it builds with all it has and has no brood and no child until it is mature.
    let (mut w, i) = start();
    w.cfg.phys.c_build = 1e5;
    let (rich, mut b) = (roles(&w).rich, brain_less(&mut w, i));
    // (Fuel for a life of building: 60 kg more of it.)
    add(&mut w, &mut b, rich, 60.0);
    let s = w.life.add(b, w.day, 1);
    let (mut grown_up, mut born_before, mut born) = (None, 0.0, 0.0);
    for d in 0..400 {
        feed_minerals(&mut w, s);
        day(&mut w);
        let b = &w.life.bodies[s];
        if !b.alive {
            break;
        }
        let kids: f64 = w.events.iter().filter_map(|e| if let pcore::event::EventKind::Born { .. } = e.kind { Some(e.value) } else { None }).sum();
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

/// A child far from its parent node's size starts a node of its own: its body is a share of its parent's
/// copy mass of 0.01 (a tenth of a kg against about 18), more than `alike` steps of the copy-mass gene
/// (a factor 6.3) from the members', so it does not join the node it was born in, and the monthly merge
/// leaves them apart until the young have grown to the size of the old (the merge takes them back
/// once their members' bodies are within the same factor). Members, matter and energy are kept.
#[test]
fn a_small_child_in_a_node_starts_its_own_size_class_and_merges_back_once_grown() {
    let (mut w, i) = start();
    let r = roles(&w);
    w.cfg.phys.c_copy = 1e18;
    let mut n = eater(&mut w, i, 0, 10f32.log10());
    (n.genes[CHILD], n.genes[DISPERSE]) = (-2.0, 0.0);
    scale(&mut w, &mut n, 5.0);
    n.count = 5.0;
    let (mm, h) = (w.chem.subs[r.matter].molar_mass, 0.75);
    // A brood of 12.6 children of 0.1 kg (matter 0.025 kg each): 12.6 members are born, as a young node.
    let child_mol = 0.1 * (1.0 - h) / mm;
    n.brood = 12.6 * child_mol;
    let s = w.life.add(n, w.day, 1);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    // (One body: a month's start would otherwise be the day of the survey; not yet.)
    w.day += 1;
    body::survey(&mut w);
    let mut n = std::mem::take(&mut w.life.bodies[s]);
    n.prepare(&w.chem, &w.cfg.phys);
    let parent_body = n.built(&w.chem) / n.n();
    assert!(n.adult(&w.chem), "the node is mature: a member's body is {parent_body} kg");
    assert!(copy::copy(&mut w, s, &mut n), "no children");
    assert_eq!(n.count, 5.0, "the node took its children in");
    assert!(n.brood < 1e-9 * child_mol, "the brood left is {} child(ren)", n.brood / child_mol);
    w.life.bodies[s] = n;
    let kids: Vec<usize> = (0..w.life.bodies.len()).filter(|&k| k != s && w.life.bodies[k].alive).collect();
    assert_eq!(kids.len(), 1, "one node of children: {}", kids.len());
    let k = kids[0];
    let c = &w.life.bodies[k];
    println!("a node of {} members, a body of {parent_body:.2} kg each; its children: {} members of {:.3} kg", w.life.bodies[s].count, c.count, c.built(&w.chem) / c.n());
    assert!((c.count - 12.6).abs() < 1e-9);
    assert!((c.store[r.matter] * mm / c.n() / (1.0 - h) - 0.1).abs() < 1e-9, "a child's body is not 10^CHILD x the copy mass");
    let lay = Layout::of(&w.chem);
    assert!(!copy::near(&lay, &w.chem, w.cfg.phys.alike, &w.life.bodies[s], &w.life.bodies[k]), "a child of 0.1 kg is of the size class of members of {parent_body} kg");
    // A month's start: they are alike in genes but not in size: they stay two.
    let month = w.cfg.days_per_month as u64;
    w.day = (w.day / month + 1) * month;
    body::survey(&mut w);
    assert_eq!(w.life.alive().count(), 2, "the young and the old merged");
    // The young grow: matter and the liquid it keeps with it added (an edit, booked) up to the members' size, and the next
    // month's merge takes them back.
    let want = (parent_body * (1.0 - h) * 12.6 - w.life.bodies[k].store[r.matter] * mm).max(0.0);
    let mut c = std::mem::take(&mut w.life.bodies[k]);
    add(&mut w, &mut c, r.matter, want);
    add(&mut w, &mut c, r.liquid, 3.0 * want);
    c.prepare(&w.chem, &w.cfg.phys);
    w.life.bodies[k] = c;
    assert!(copy::near(&lay, &w.chem, w.cfg.phys.alike, &w.life.bodies[s], &w.life.bodies[k]), "grown to {} kg a member, not yet of the size class", w.life.bodies[k].built(&w.chem) / 12.6);
    w.day += month;
    body::survey(&mut w);
    assert_eq!(w.life.alive().count(), 1, "the grown young did not merge back");
    let all: f64 = w.life.alive().map(|b| b.n()).sum();
    assert!((all - 17.6).abs() < 1e-9, "members {all}");
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Matter and energy are kept over updates in which a body builds, fills a brood and has children
/// (the update path of `body::live`): a mature eater with fuel and an allowance, over 150 days.
#[test]
fn reproducing_updates_keep_elements_and_energy() {
    let (mut w, i) = start();
    w.cfg.phys.c_build = 1e5;
    let mut b = eater(&mut w, i, 0, 5f32.log10());
    // Children of 0.1 of the copy mass (0.5 kg) from a mature body of 21 kg.
    (b.genes[CHILD], b.genes[ALLOC]) = (-1.0, 0.8);
    let id = {
        let s = w.life.add(b, w.day, 1);
        w.life.bodies[s].id(s)
    };
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let mut born = 0.0;
    for _ in 0..150 {
        day(&mut w);
        born += w.events.iter().filter_map(|e| if let pcore::event::EventKind::Born { .. } = e.kind { Some(e.value) } else { None }).sum::<f64>();
        assert_eq!(pcore::check::broken(&w), None);
    }
    let kids = w.life.alive().filter(|b| b.parent == id).count();
    println!("born {born}, bodies of its children {kids}");
    assert!(born >= 1.0, "no child in 150 days");
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Satiety holds at every size with `Config::sate_all` (the decision after watch report 7; the
/// default holds it past `full` only, below) and counts what a body can build: asked for twice what
/// its size can take, bodies of 0.1, 1, 10 and 100 times a starting eater's mass (below and past
/// `full`) are left with the fuel their upkeep and their building use, never the size cap, and the
/// solid converted into the matter they build counts: the same body with a copy cost a million times
/// lower is left with more.
#[test]
fn satiety_caps_every_size_and_counts_the_matter_built() {
    let (mut w, i) = start();
    let r = roles(&w);
    w.cfg.phys.c_build = 3e4;
    w.cfg.sate_all = true;
    for times in [0.1, 1.0, 10.0, 100.0] {
        let mut b = eater(&mut w, i, 0, 6.0);
        scale(&mut w, &mut b, times);
        // Matter is not an eater's fuel here: it burns the solid only.
        let lay = Layout::of(&w.chem);
        let (m, burn_matter) = worldgen::life::matter_roles(&w.chem).expect("matter");
        (b.genes[lay.cat + burn_matter], b.genes[lay.upt + m]) = (-1.0, -0.1);
        b.prepare(&w.chem, &w.cfg.phys);
        let (p, days, mass) = (w.cfg.phys, b.interval() as f64, b.mass(&w.chem));
        let (equiv, up) = (b.supply(&w.chem, &p), body::upkeep(&w, &b, days, mass));
        let cap = p.take * days * equiv;
        let jpk = -w.chem.dh(r.burn) / w.chem.reactions[r.burn].inputs.iter().find(|x| x.0 as usize == r.rich).unwrap().1 as f64 / w.chem.subs[r.rich].molar_mass;
        let left = |w: &World, b: &Body| {
            let mut m = vec![0.0; w.chem.ns()];
            m[r.rich] = 2.0 * cap / w.chem.subs[r.rich].molar_mass;
            choose::sate(w, b, &mut m, days, mass);
            m[r.rich] * w.chem.subs[r.rich].molar_mass
        };
        let dear = left(&w, &b);
        let cheap = {
            let mut w2 = w.clone();
            w2.cfg.phys.c_copy = 1e-6 * w.cfg.phys.c_copy;
            left(&w2, &b)
        };
        println!("{times} x 30 kg: left {dear:.3} kg (upkeep {:.3} kg, size cap {cap:.3} kg), with a copy cost 10^6 lower {cheap:.3} kg", up / jpk);
        assert!(dear < cap && cheap <= cap + 1e-9, "the size cap {cap} binds: {dear}, {cheap}");
        assert!(dear >= up / jpk * (1.0 - 1e-9), "less than its upkeep's worth: {dear} of {}", up / jpk);
        assert!(cheap >= dear, "{cheap} against {dear}");
    }
    // A starved body (a store under the target) is the case where the building term shows: with
    // the conversion of the solid into the matter built counted, it is left more than with the
    // conversion nearly free (a copy cost 10^6 higher).
    for keep in [0.04] {
        let mut b = eater(&mut w, i, 0, 6.0);
        scale(&mut w, &mut b, 10.0);
        let out_kg = (1.0 - keep) * kg(&w, &b, r.rich);
        add(&mut w, &mut b, r.rich, -out_kg);
        let lay = Layout::of(&w.chem);
        let (m, burn_matter) = worldgen::life::matter_roles(&w.chem).expect("matter");
        (b.genes[lay.cat + burn_matter], b.genes[lay.upt + m]) = (-1.0, -0.1);
        b.prepare(&w.chem, &w.cfg.phys);
        let (days, mass) = (b.interval() as f64, b.mass(&w.chem));
        let left = |w: &World| {
            let mut m = vec![0.0; w.chem.ns()];
            m[r.rich] = 40.0 * w.cfg.phys.take * days * b.supply(&w.chem, &w.cfg.phys) / w.chem.subs[r.rich].molar_mass;
            choose::sate(w, &b, &mut m, days, mass);
            m[r.rich] * w.chem.subs[r.rich].molar_mass
        };
        let with = left(&w);
        let mut w2 = w.clone();
        w2.cfg.phys.c_copy *= 1e6;
        let without = left(&w2);
        println!("a starved body (fuel {:.0}% of the target's): left {with:.3} kg with the conversion counted, {without:.3} kg with a copy cost 10^6 higher", 100.0 * keep);
        assert!(with > without * 1.5, "{with} and {without}");
    }
}

/// Dead body matter on the ground oxidises in the open at the rate chemistry 0 sets: 0.2 of it a
/// year at 290 K in the starting air (18.6% lost in a year; a little slower than the energy-rich
/// solid's 0.27 a year, 24%: structure resists; both are litter decay constants, forest floor
/// 0.25 a year), by the oxygen-like gas to the 6th power (the power of the rate law, `Reaction::ord`; the matter takes 118 of
/// it in all, 0.21^6 = 8.6e-5) over a rate constant of 7.6e-5 /s at 290 K. The rate is per mol of matter, so it did not change when the
/// unit of matter grew three times (15 Oct 2026).
#[test]
fn dead_body_matter_oxidises_at_the_set_rate() {
    let (chem, init) = worldgen::chemistry0();
    let (m, rich) = (chem.matter().expect("body matter"), 5usize);
    let mut rc = change::Reactor::new(&chem);
    // The starting air in plenty (booked nowhere: a node of its own), a huge heat capacity.
    let mut lost = |k: usize| {
        let mut n = vec![0.0; chem.ns()];
        let mut gas = vec![0.0; chem.ns()];
        for &(g, x) in &init.air {
            gas[g] = x * 1e9;
        }
        n[k] = 1000.0;
        let mut e = 290.0 * 1e15;
        rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, 365.0 * 86_400.0);
        1.0 - n[k] / 1000.0
    };
    let (matter, solid) = (lost(m), lost(rich));
    println!("a year at 290 K in the starting air: body matter {:.2}% oxidised, the energy-rich solid {:.2}%", 100.0 * matter, 100.0 * solid);
    assert!(matter > 0.17 && matter < 0.205, "{matter}");
    assert!(solid > 0.22 && solid < 0.26 && matter < solid, "{solid}");
    // It burns: 55,124.7 kJ a mol (15.9 MJ a kg, a little over the solid's 15.6, which the matter was made of without a gain of heat;
    // 16.7 before the rock salt, whose carbon is already burnt and which leaves as lime), and balances in elements (validated on load).
    let burn = (0..chem.reactions.len()).find(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == m)).expect("a burning reaction");
    assert!((chem.dh(burn) / -55124.7e3 - 1.0).abs() < 1e-6 && chem.validate().is_ok(), "{}", chem.dh(burn));
    // Bodies catalyse it and pay for the enzyme as for the solid's: the same speed-up costs about
    // three quarters as much (the rate constants over the open rates, the air beyond first order).
    let cost = |r: usize| {
        let mut b = Body { genes: vec![0.0; Layout::of(&chem).len], store: vec![0.0; chem.ns()], ..Default::default() };
        b.genes[Layout::of(&chem).cat + r] = 2.6;
        b.prepare(&chem, &pcore::Physics::default());
        b.k.cats
    };
    let (c_matter, c_solid) = (cost(burn), cost(1));
    println!("enzyme capacity at a speed-up of 10^2.6: body matter {c_matter:.2e}, the solid {c_solid:.2e} per s");
    assert!(c_matter > 0.5 * c_solid && c_matter < 1.0 * c_solid, "{c_matter} and {c_solid}");
}

/// By default there is no satiety (14 Oct 2026, after watch report 8): a body is left everything it
/// is asked to take, up to its size cap, young or past `full`; what it carries is bounded by its
/// body (`Body::level`, below), so a full store is the satiety. With `Config::sate_all` it is held
/// to its upkeep and what it builds at every size. (With satiety at every size and the keep genes a
/// share of the whole mass, all the animals of the 64x64 runs died in their first year.)
#[test]
fn no_satiety_by_default_and_every_size_with_the_switch() {
    let (mut w, i) = start();
    let r = roles(&w);
    let left = |w: &World, copy: f32| {
        let mut w = w.clone();
        let mut b = eater(&mut w, i, 0, copy);
        let lay = Layout::of(&w.chem);
        let (m, burn_matter) = worldgen::life::matter_roles(&w.chem).expect("matter");
        (b.genes[lay.cat + burn_matter], b.genes[lay.upt + m]) = (-1.0, -0.1);
        b.prepare(&w.chem, &w.cfg.phys);
        let (days, mass) = (b.interval() as f64, b.mass(&w.chem));
        let cap = w.cfg.phys.take * days * b.supply(&w.chem, &w.cfg.phys);
        let mut m = vec![0.0; w.chem.ns()];
        m[r.rich] = 2.0 * cap / w.chem.subs[r.rich].molar_mass;
        choose::sate(&w, &b, &mut m, days, mass);
        (m[r.rich] * w.chem.subs[r.rich].molar_mass, cap)
    };
    // A body of 21 kg: copy mass 100 kg (below full, 200 kg) or 5 kg (past full, 10 kg).
    let (young, cap) = left(&w, 2.0);
    let (grown, _) = left(&w, 5f32.log10());
    println!("no satiety: left {young:.3} kg of {:.3} asked below full, {grown:.3} kg past it (size cap {cap:.3} kg)", 2.0 * cap);
    assert!((young / (2.0 * cap) - 1.0).abs() < 1e-9 && (grown / (2.0 * cap) - 1.0).abs() < 1e-9, "a body is held: {young} and {grown} of {}", 2.0 * cap);
    w.cfg.sate_all = true;
    let (young_all, _) = left(&w, 2.0);
    let (grown_all, _) = left(&w, 5f32.log10());
    assert!(young_all < 0.2 * cap && grown_all < 0.2 * cap, "satiety at every size holds the young and the grown: {young_all}, {grown_all}");
}

/// What a body carries is bounded by its body, not its whole mass (14 Oct 2026, after watch report
/// 8): the keep gene of the liquid is the share of the body it makes up (so it holds h / (1 - h) of
/// its matter), the keep gene of any other store (not body matter) is kg of it per kg of body
/// (`Body::size`). So more fuel does not raise the level of the liquid or of another store, and a
/// body that takes without limit stops at them: left on ground full of the solid and the liquid it
/// never holds more than its keep genes times its body, however long it takes.
#[test]
fn a_body_cannot_store_more_than_keep_times_body() {
    let (mut w, i) = start();
    let r = roles(&w);
    let lay = Layout::of(&w.chem);
    let mut b = eater(&mut w, i, 3, 6.0);
    let (mm_rich, mm_liq) = (w.chem.subs[r.rich].molar_mass, w.chem.subs[r.liquid].molar_mass);
    let (keep_rich, h) = (b.genes[lay.keep + r.rich] as f64, b.genes[lay.keep + r.liquid] as f64);
    let mass = b.mass(&w.chem);
    let size = b.size(&w.chem);
    // The levels: the solid keep x body, the liquid h / (1 - h) x matter.
    let (lv_rich, lv_liq) = (b.level(&w.chem, r.rich, mass) * mm_rich, b.level(&w.chem, r.liquid, mass) * mm_liq);
    println!("body {size:.2} kg of {mass:.2}: solid level {lv_rich:.2} kg (keep {keep_rich:.2}), liquid level {lv_liq:.2} kg (h {h:.2})");
    assert!((lv_rich / (keep_rich * size) - 1.0).abs() < 1e-9, "{lv_rich} of {keep_rich} x {size}");
    assert!((lv_liq / (h / (1.0 - h) * kg(&w, &b, r.matter)) - 1.0).abs() < 1e-9, "{lv_liq}");
    // More fuel does not raise the other levels, nor the size; scaling the whole body raises all.
    add(&mut w, &mut b, r.rich, 100.0);
    assert!((b.level(&w.chem, r.rich, b.mass(&w.chem)) * mm_rich / lv_rich - 1.0).abs() < 1e-9 && (b.size(&w.chem) / size - 1.0).abs() < 1e-9);
    add(&mut w, &mut b, r.rich, -100.0);
    let mut big = b.clone();
    scale(&mut w, &mut big, 4.0);
    assert!((big.level(&w.chem, r.rich, big.mass(&w.chem)) * mm_rich / lv_rich - 4.0).abs() < 1e-9);
    // A long take from ground rich in both: the stores stop at their levels.
    w.cfg.phys.c_handle = 0.0;
    w.cfg.phys.c_build = 0.0;
    let n = w.grid.n();
    let t = w.t_ground(i);
    let (a_rich, a_liq) = (50.0 * w.grid.area() / mm_rich, 50.0 * w.grid.area() / mm_liq);
    for (k, a) in [(r.rich, a_rich), (r.liquid, a_liq)] {
        w.ground[k * n + i] += a;
        w.chem.add_elems(k, a, &mut w.ledger.imposed_elems);
        w.ledger.imposed_energy += a * w.chem.subs[k].hf;
    }
    let e = w.energy_ground(i, t);
    (w.ledger.imposed_energy, w.e_ground[i]) = (w.ledger.imposed_energy + e - w.e_ground[i], e);
    let rich0 = b.store[r.rich];
    let s = w.life.add(b, w.day, 1);
    let (mut top_rich, mut top_liq) = (0.0f64, 0.0f64);
    for _ in 0..120 {
        day(&mut w);
        let b = &w.life.bodies[s];
        let mass = b.mass(&w.chem);
        let (x_rich, x_liq) = (b.store[r.rich] * mm_rich / (b.level(&w.chem, r.rich, mass) * mm_rich).max(1e-30), b.store[r.liquid] * mm_liq / (b.level(&w.chem, r.liquid, mass) * mm_liq).max(1e-30));
        top_rich = top_rich.max(x_rich);
        top_liq = top_liq.max(x_liq);
    }
    let b = &w.life.bodies[s];
    println!("after 120 days: solid {:.2} kg (was {:.2}), at most {top_rich:.3} of its level; liquid at most {top_liq:.3} of its level; body {:.2} kg, mass {:.2} kg", kg(&w, b, r.rich), rich0 * mm_rich, b.size(&w.chem), b.mass(&w.chem));
    assert!(b.store[r.rich] > rich0, "it took none of the solid");
    // (The liquid starts at its level, and burning the solid leaves liquid in the body, so it
    // goes a little past; taking without limit would take it to many times that.)
    assert!(top_rich < 1.0 + 1e-6 && top_liq < 1.1, "past its level: solid {top_rich}, liquid {top_liq}");
    assert!(b.mass(&w.chem) < 3.0 * b.size(&w.chem), "mass {} for a body of {}", b.mass(&w.chem), b.size(&w.chem));
}

/// The safe minimums (the min genes) are shares of the body, so a body is not killed by holding
/// fuel: a starting eater of 30 kg (body 21 kg, solid 9 kg, liquid 15.75 kg) with a minimum for the
/// liquid of 0.7 of its body (14.7 kg, under the 15.75 it has) is short of it when it is 0.7 of its
/// whole mass (21 kg), and still is not after it takes in 100 kg of the solid.
#[test]
fn the_safe_minimum_is_a_share_of_the_body() {
    let (mut w, i) = start();
    let r = roles(&w);
    let lay = Layout::of(&w.chem);
    let mut b = eater(&mut w, i, 0, 6.0);
    b.genes[lay.min + r.liquid] = 0.7;
    let (size, mass) = (b.size(&w.chem), b.mass(&w.chem));
    println!("body {size:.2} kg of {mass:.2}: liquid {:.2} kg against a minimum of {:.2} kg", kg(&w, &b, r.liquid), 0.7 * size);
    assert!(kg(&w, &b, r.liquid) > 0.7 * size && kg(&w, &b, r.liquid) < 0.7 * mass, "the setup");
    assert!(!b.lacking_at(&w.chem, mass), "lacking a minimum that is met");
    add(&mut w, &mut b, r.rich, 100.0);
    assert!(!b.lacking_at(&w.chem, b.mass(&w.chem)), "killed by holding fuel");
    // And a minimum above what it holds still counts: 0.8 of the body is 16.8 kg of the liquid.
    b.genes[lay.min + r.liquid] = 0.8;
    assert!(b.lacking_at(&w.chem, b.mass(&w.chem)));
}

/// Digestion: body matter that a body takes in from outside is broken into the energy-rich solid as
/// it comes in (1 of matter + 1 of liquid -> 6 of the solid, the reverse of growth), so it enters as
/// fuel, never as body, and the body builds its own matter from it at the cost of work. Elements and
/// energy are kept exactly. A body whose catalyst for burning matter is off takes none.
#[test]
fn matter_taken_from_outside_is_digested_into_the_solid() {
    let take_from_ground = |digests: bool| {
        let (mut w, i) = start();
        let r = roles(&w);
        w.cfg.phys.c_handle = 0.0;
        w.cfg.phys.c_build = 0.0;
        // 3 kg of body matter per m² on its ground (booked as an edit).
        let (t, a, n) = (w.t_ground(i), 3.0 * w.grid.area() / w.chem.subs[r.matter].molar_mass, w.grid.n());
        w.ground[r.matter * n + i] += a;
        w.chem.add_elems(r.matter, a, &mut w.ledger.imposed_elems);
        let e = w.energy_ground(i, t);
        (w.ledger.imposed_energy, w.e_ground[i]) = (w.ledger.imposed_energy + e - w.e_ground[i] + a * w.chem.subs[r.matter].hf, e);
        let mut b = eater(&mut w, i, 3, 6.0);
        let lay = Layout::of(&w.chem);
        let (_, burn_matter) = worldgen::life::matter_roles(&w.chem).expect("matter");
        if !digests {
            b.genes[lay.cat + burn_matter] = -1.0;
        }
        // Only matter on the ground to take: it takes neither the liquid nor the solid.
        (b.genes[lay.upt + r.liquid], b.genes[lay.upt + r.rich]) = (-0.1, -0.1);
        let (rich0, matter0, size0) = (b.store[r.rich], b.store[r.matter], b.size(&w.chem));
        let s = w.life.add(b, w.day, 1);
        let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
        // What the body took from the ground (the events), not what left it: dead matter on the ground also rots in the open.
        let mut took = 0.0;
        for _ in 0..10 {
            day(&mut w);
            took += w.events.iter().map(|e| if let pcore::event::EventKind::Took { from: u64::MAX, .. } = e.kind { e.value } else { 0.0 }).sum::<f64>();
        }
        let b = &w.life.bodies[s];
        assert_eq!(pcore::check::broken(&w), None);
        let (de, dq) = pcore::check::drift(&w, &t0, &l0);
        assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
        (b.store[r.rich] - rich0, b.store[r.matter] - matter0, took / w.chem.subs[r.matter].molar_mass, b.size(&w.chem) - size0)
    };
    let (rich, matter, taken, size) = take_from_ground(true);
    println!("an eater took {taken:.2} mol of matter from the ground: solid +{rich:.2} mol, its own matter {matter:+.3} mol, body {size:+.3} kg");
    assert!(taken > 0.5, "it took none");
    // The solid rose by 40 per mol taken (less what it burnt in ten days); its own matter did not rise.
    assert!(rich > 20.0 * taken, "solid +{rich} mol for {taken} mol of matter");
    assert!(matter <= 1e-9 && size <= 1e-9, "matter entered as body: {matter} mol, body {size} kg");
    let (_, _, none, _) = take_from_ground(false);
    assert!(none < 1e-12, "a body that cannot burn matter took {none} mol of it");
    // The reverse of growth, exactly, called directly.
    let (mut w, i) = start();
    let r = roles(&w);
    let mut b = eater(&mut w, i, 0, 6.0);
    add(&mut w, &mut b, r.matter, 34.56);
    let (s, mut b) = out(&mut w, b);
    let (t0, l0, before) = (pcore::check::totals(&w), w.ledger.clone(), b.store.clone());
    let x = b.digest(&w.chem, 10.0);
    assert!((x - 10.0).abs() < 1e-12);
    // 10 of matter and 270 of liquid become 1,180 of the solid, 30 of the nitrogen mineral and 10 of the rock salt.
    assert!((before[r.matter] - b.store[r.matter] - 10.0).abs() < 1e-9 && (b.store[r.rich] - before[r.rich] - 1180.0).abs() < 1e-9 && (before[r.liquid] - b.store[r.liquid] - 270.0).abs() < 1e-9);
    for (mineral, per) in w.chem.minerals() {
        assert!((b.store[mineral] - before[mineral] - 10.0 * per).abs() < 1e-9, "substance {mineral}");
    }
    w.life.bodies[s] = b;
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-12, "element drift {de:e}, energy drift {dq:e}");
}

/// Body size by an energy ceiling, not a cap (14 Oct 2026): what a body takes goes with its mass^0.75
/// (`Body::equiv`) and the cost of keeping its tissue with its body^1.0 (`body::upkeep`), so growth,
/// which builds only from the spare after upkeep (`copy::grow`), stops where the two meet, a size the
/// body's own costs set. An eater (ALLOC 0: it builds only for itself, nothing is given to children;
/// strength 1; building fast, so that the allowance never binds; its liquid put back to the ground) in
/// a place with all the fuel it can take, at every update. With tissue dear enough that the ceiling is
/// near (costs 1.4 times the starting ones, 1.7 before the nitrogen mineral: growth was 5% slower then and a body at 1.7 starved in the
/// cold season of its 200th day) it levels off (under a quarter more body
/// between the 100th and the 300th day) and in its last 100 days takes about what it pays to keep what
/// it has (within 5 to 10%); with the costs of the start, a ceiling many times higher, it is still
/// growing fast (more than twice the relative growth) and ends larger. (The bodies of this test die
/// of a cold season when held at the ceiling for a year: 360 days are run.)
#[test]
fn growth_stops_where_upkeep_meets_what_the_body_can_take() {
    use pcore::event::EventKind;
    let run = |cost: f64| {
        let (mut w, i) = start();
        w.cfg.tilt = 0.0;
        let r = roles(&w);
        // Building never binds; the costs of keeping tissue are `cost` times the starting ones.
        w.cfg.phys.c_build = 1e7;
        for c in [&mut w.cfg.phys.c_basal, &mut w.cfg.phys.c_cat, &mut w.cfg.phys.c_strength, &mut w.cfg.phys.c_learn, &mut w.cfg.phys.c_signal] {
            *c *= cost;
        }
        let mut b = eater(&mut w, i, 3, 0.0);
        // A body that is wet (soaking), breathes freely and sheds its heat, so that only its energy limits it.
        (b.genes[STRENGTH], b.genes[ALLOC], b.genes[LIQ], b.genes[GAS], b.genes[HEAT]) = (1.0, 0.0, -1.0, 3.0, 3.0);
        let (s, b0) = out(&mut w, b);
        w.life.bodies[s] = b0;
        let (n, area) = (w.grid.n(), w.grid.area());
        let joule = -w.chem.dh(r.burn) / w.chem.reactions[r.burn].inputs.iter().find(|x| x.0 as usize == r.rich).unwrap().1 as f64 / w.chem.subs[r.rich].molar_mass;
        let (mut built, mut takes, mut ups) = (Vec::new(), vec![0.0; 360], vec![0.0; 360]);
        for day in 0..360usize {
            // Fuel and liquid in plenty: 100 kg per m² of each on the cell at every update (booked as an edit).
            for k in [r.rich, r.liquid] {
                let (t, a) = (w.t_ground(i), (100.0 * area / w.chem.subs[k].molar_mass - w.g(k, i)).max(0.0));
                w.ground[k * n + i] += a;
                w.chem.add_elems(k, a, &mut w.ledger.imposed_elems);
                let e = w.energy_ground(i, t);
                (w.ledger.imposed_energy, w.e_ground[i]) = (w.ledger.imposed_energy + e - w.e_ground[i] + a * w.chem.subs[k].hf, e);
            }
            // And the minerals growth is built of (15 Oct 2026): this is a test of the energy ceiling, so its roots find them in plenty
            // (an eater has them only from the matter it digests, and here eats none).
            feed_minerals(&mut w, s);
            self::day(&mut w);
            assert!(w.life.bodies[s].alive, "the body died on day {day}: {:?}", w.events.iter().filter_map(|e| if let EventKind::Died { cause, .. } = e.kind { Some(cause) } else { None }).collect::<Vec<_>>());
            let b = &mut w.life.bodies[s];
            // What its burning made and it holds above the level it keeps goes back to the ground (a body that decides to put it
            // out, or breathes it out; this one only takes).
            let (mass, t) = (b.mass(&w.chem), b.temp(&w.chem));
            let over = (b.store[r.liquid] - b.level(&w.chem, r.liquid, mass)).max(0.0);
            let q = over * w.chem.subs[r.liquid].cp * t;
            (w.ground[r.liquid * n + i], b.store[r.liquid], w.e_ground[i], b.e) = (w.ground[r.liquid * n + i] + over, b.store[r.liquid] - over, w.e_ground[i] + q, b.e - q);
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
    let (full, r_full) = run(1.4);
    let (cheap, r_cheap) = run(1.0);
    let n = full.len();
    // Relative growth between the 100th and the 300th day (the first days only fill the body with the liquid it keeps).
    let rel = |x: &[f64]| x[5] / x[1] - 1.0;
    // Growth slows toward a size: the body with the dearer tissue is near it, the other far from it.
    assert!(rel(&full) < 0.25 && rel(&cheap) > 2.0 * rel(&full), "growth: {:.3} of the body between the 100th and the 300th day against {:.3} with cheaper tissue", rel(&full), rel(&cheap));
    // What it takes over what it pays to keep what it has falls toward 1 as it grows (supply goes with mass^0.75,
    // upkeep with the body): at the end it is within 10% of 1, for the body with dear tissue, and the other, far from its
    // ceiling, more.
    assert!(r_full > 0.95 && r_full < 1.1 && r_cheap > r_full, "took / upkeep at the end {r_full} and {r_cheap} with cheaper tissue");
    // Cheaper tissue, a larger body in the same time.
    assert!(cheap[n - 1] > 1.5 * full[n - 1], "a body with cheaper tissue grows larger: {} against {}", cheap[n - 1], full[n - 1]);
}
