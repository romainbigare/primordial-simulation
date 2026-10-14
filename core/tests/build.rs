//! Growth is building body matter (14 Oct 2026, decision after watch report 7): a body is its
//! matter and the liquid it keeps with it (`Body::size`), not the fuel it stores; growth turns the
//! energy-rich solid into matter and costs `c_copy` per kg of matter in work, from the `c_build`
//! allowance and what the body can spare (`copy::grow`); storing fuel costs nothing; a child is
//! given a share of every store, matter included, and costs its parent no more work; satiety is off
//! by default (a store is bounded by the body, `Body::level`); dead body matter on the ground oxidises at the rate chemistry 0 sets.

use pcore::body::{Body, Layout, COPY_MASS, DISPERSE, LIQ, STRENGTH};
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

/// Growth makes body matter of the energy-rich solid, 6 of it for 1 of matter and 1 of liquid, and
/// pays `c_copy` per kg of matter made in work that burns the solid (the fuel's heat leaves the
/// body for the ground): elements and energy are kept exactly, and the stores change as the
/// chemistry says: matter up by x, the solid down by 6 x and by what the work burnt, the liquid up
/// by x and by the liquid the burning made.
#[test]
fn growth_makes_matter_of_the_solid_and_keeps_elements_and_energy() {
    let (mut w, i) = start();
    let r = roles(&w);
    w.cfg.phys.c_build = 1e6;
    let b = eater(&mut w, i, 0, 6.0);
    let (_, mut b) = out(&mut w, b);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let (matter0, rich0, liquid0, e0) = (b.store[r.matter], b.store[r.rich], b.store[r.liquid], w.e_ground.iter().sum::<f64>());
    let days = b.interval() as f64;
    assert!(copy::grow(&mut w, &mut b, days), "it did not grow");
    let (x, c_copy) = (b.store[r.matter] - matter0, w.cfg.phys.c_copy);
    assert!(x > 0.0);
    // The work: c_copy per kg of matter, burnt as the solid is (heat per mol of the burning reaction).
    let burnt = c_copy * x * w.chem.subs[r.matter].molar_mass / -w.chem.dh(r.burn);
    let (dsolid, dliquid) = (rich0 - b.store[r.rich], b.store[r.liquid] - liquid0);
    println!("matter made {x:.4} mol ({:.3} kg), solid down {dsolid:.4} mol (6 x = {:.4}, burnt {burnt:.4}), liquid up {dliquid:.4}", x * w.chem.subs[r.matter].molar_mass, 6.0 * x);
    assert!((dsolid / (6.0 * x + burnt) - 1.0).abs() < 1e-6, "the solid fell by {dsolid}, 6 x + burnt = {}", 6.0 * x + burnt);
    assert!((dliquid / (x + burnt) - 1.0).abs() < 1e-6, "the liquid rose by {dliquid}, x + burnt = {}", x + burnt);
    // The work left the body as heat for the ground: that heat is the burnt solid's.
    let heat = w.e_ground.iter().sum::<f64>() - e0;
    assert!(heat > 0.0 && (heat / (burnt * -w.chem.dh(r.burn)) - 1.0).abs() < 0.05, "heat to the ground {heat:e} J, the work {:e} J", burnt * -w.chem.dh(r.burn));
    // Growth is thermoneutral: the matter holds the energy of the solid it was made of.
    assert!(w.chem.dh_build().abs() < 1.0, "growth takes {} J", w.chem.dh_build());
    w.life.bodies[0] = b;
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Building takes time: with fuel and spare in plenty, a body builds `c_build` x its `equiv` /
/// `m_ref` x the days of its interval over `c_copy` kg of matter in an update, no more; twice the
/// allowance builds twice the matter; and a body that has built its body (size at `full` times its
/// copy mass) builds nothing more.
#[test]
fn growth_is_limited_by_c_build() {
    let built = |c_build: f64, copy: f32| {
        let (mut w, i) = start();
        let r = roles(&w);
        w.cfg.phys.c_build = c_build;
        let mut b = eater(&mut w, i, 0, copy);
        scale(&mut w, &mut b, 5.0);
        let (s, mut b) = out(&mut w, b);
        let m0 = b.store[r.matter];
        let days = b.interval() as f64;
        let room = w.cfg.phys.c_build * b.equiv(&w.cfg.phys, b.mass(&w.chem)) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
        let grew = copy::grow(&mut w, &mut b, days);
        let kg_matter = (b.store[r.matter] - m0) * w.chem.subs[r.matter].molar_mass;
        w.life.bodies[s] = b;
        (grew, kg_matter, room)
    };
    let (g1, one, room1) = built(1e4, 6.0);
    let (g2, two, room2) = built(2e4, 6.0);
    println!("built {one:.5} kg of matter with c_build 1e4 (allowance {room1:.5} kg), {two:.5} kg with 2e4 ({room2:.5})");
    assert!(g1 && g2);
    assert!((one / room1 - 1.0).abs() < 1e-3 && (two / room2 - 1.0).abs() < 1e-3, "the allowance binds: {one} of {room1}, {two} of {room2}");
    assert!((two / one - 2.0).abs() < 0.01, "twice the allowance, twice the matter: {one} and {two}");
    // A body at its full size (matter 5 x 5.25 kg over 0.25 = 105 kg; copy mass 10^1.0 = 10 kg, full 20 kg) builds nothing.
    let (grew, none, _) = built(1e4, 1.0);
    assert!(!grew && none == 0.0, "a body past full built {none} kg");
}

/// Building takes what the body can spare: after what it pays and converts, its reactions must
/// still be able to release its upkeep over an interval (the rule of copying before `full`). A body
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
        let out_kg = (1.0 - keep) * kg(&w, &b, r.rich);
        add(&mut w, &mut b, r.rich, -out_kg);
        let (s, mut b) = out(&mut w, b);
        let days = b.interval() as f64;
        let m0 = b.store[r.matter];
        let grew = copy::grow(&mut w, &mut b, days);
        let built = (b.store[r.matter] - m0) * w.chem.subs[r.matter].molar_mass;
        let (up, release) = (body::upkeep(&w, &b, days, b.mass(&w.chem)), body::release(&w, &b, days));
        let room = w.cfg.phys.c_build * b.equiv(&w.cfg.phys, b.mass(&w.chem)) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
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

/// Storing fuel costs no building work: a body at its full size builds nothing however much fuel it
/// holds (`copy::grow` pays no work and changes no store), and a body that takes the energy-rich
/// solid from the ground with no allowance to build with stores it (the solid rises) and its matter
/// and size stay.
#[test]
fn storing_the_solid_costs_no_build() {
    let (mut w, i) = start();
    let r = roles(&w);
    // At its full size (copy mass 10 kg of body, full 20 kg, its body 21 kg), with 100 kg more fuel.
    let mut b = eater(&mut w, i, 0, 1.0);
    add(&mut w, &mut b, r.rich, 100.0);
    assert!(b.size(&w.chem) > 20.0);
    let (_, mut b) = out(&mut w, b);
    let (stores, heat, days) = (b.store.clone(), b.e, b.interval() as f64);
    assert!(!copy::grow(&mut w, &mut b, days) && b.store == stores && b.e == heat, "a body at full size built");

    // A body below its full size, taking the solid from the ground, with no allowance to build.
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
    // (Matter does not rise; it oxidises in the open at 5% a year, here a month: 0.4%.)
    assert!(kg(&w, b, r.matter) <= matter0 && kg(&w, b, r.matter) > 0.99 * matter0, "matter changed with no allowance: {matter0} -> {}", kg(&w, b, r.matter));
    assert_eq!(pcore::check::broken(&w), None);
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// A child gets a share of every store, body matter included, which its parent built and paid for,
/// and costs it no more work: a copy needs only the matter to exist, so it happens with a copy cost
/// nobody could pay and no allowance, and no heat is paid to the ground. The parent keeps the rest
/// of its body, below its copy mass, and has no second child until growth rebuilds what it gave:
/// the intervals that takes are the matter given over the allowance, so children come no faster
/// than bodies are built.
#[test]
fn a_child_gets_matter_without_the_parent_paying_again() {
    let (mut w, i) = start();
    let r = roles(&w);
    // (A copy cost nobody could pay, no allowance: under the old law this was no child.)
    (w.cfg.phys.c_copy, w.cfg.phys.c_build) = (1e18, 0.0);
    // A body of 21 kg copying at 20 kg (full 40 kg): it gives the share CHILD of every store.
    let b = eater(&mut w, i, 0, 20f32.log10());
    let share = b.gene(body::CHILD).clamp(0.05, 0.5);
    let (s, mut b) = out(&mut w, b);
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let (before, ground0) = (b.store.clone(), w.e_ground.iter().sum::<f64>());
    w.day += 1;
    assert!(copy::copy(&mut w, s, &mut b), "no child");
    let kids: Vec<usize> = (0..w.life.bodies.len()).filter(|&k| w.life.bodies[k].alive && w.life.bodies[k].parent == b.id(s)).collect();
    assert_eq!(kids.len(), 1, "children {}", kids.len());
    let c = &w.life.bodies[kids[0]];
    for k in 0..w.chem.ns() {
        assert!((c.store[k] - share * before[k]).abs() <= 1e-9 * before[k].max(1.0), "substance {k}: child {} of {}", c.store[k], before[k]);
        assert!((b.store[k] - (1.0 - share) * before[k]).abs() <= 1e-9 * before[k].max(1.0));
    }
    println!("matter: parent {:.3} -> {:.3} kg, child {:.3} kg; size {:.2} -> {:.2} kg", before[r.matter] * 0.162, kg(&w, &b, r.matter), kg(&w, c, r.matter), 21.0, b.size(&w.chem));
    assert!(c.store[r.matter] > 0.0);
    assert_eq!(w.e_ground.iter().sum::<f64>(), ground0, "work was paid for a child");
    // The parent is below its copy mass now, and cannot copy again.
    assert!(b.size(&w.chem) < 20.0 && !copy::copy(&mut w, s, &mut b));
    // With an allowance (and the copy cost of the world) it rebuilds in the intervals the matter
    // given needs: given x (1 - h) is the matter, over the allowance a day (c_build x equiv /
    // m_ref / c_copy) and the days of an interval.
    let given = share * before[r.matter] * w.chem.subs[r.matter].molar_mass;
    // (Liquid comes by soaking, as in `live`: here it is there.)
    add(&mut w, &mut b, r.liquid, 20.0);
    w.life.bodies[s] = b;
    w.cfg.phys = pcore::Physics { c_build: 1e4, ..pcore::Physics::default() };
    let (mut b, mut intervals, mut regrown) = (std::mem::take(&mut w.life.bodies[s]), 0, 0.0);
    b.prepare(&w.chem, &w.cfg.phys);
    let (days, m0) = (b.interval() as f64, kg(&w, &b, r.matter));
    while b.size(&w.chem) < 20.0 && intervals < 200 {
        w.day += b.interval();
        copy::grow(&mut w, &mut b, days);
        regrown = kg(&w, &b, r.matter) - m0;
        intervals += 1;
    }
    let per = w.cfg.phys.c_build * b.equiv(&w.cfg.phys, b.mass(&w.chem)) / w.cfg.phys.m_ref * days / w.cfg.phys.c_copy;
    println!("the parent regrew {regrown:.3} kg of matter (it gave {given:.3} kg) in {intervals} intervals of {days} days, {per:.4} kg an interval");
    assert!(intervals > 3 && intervals < 200, "{intervals} intervals");
    assert!((regrown / per / intervals as f64 - 1.0).abs() < 0.15, "it built {regrown} kg in {intervals} intervals, the allowance is {per} kg each");
    assert!(copy::copy(&mut w, s, &mut b), "a rebuilt body copies");
    w.life.bodies[s] = b;
    assert_eq!(pcore::check::broken(&w), None);
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
        let (equiv, up) = (b.equiv(&p, mass), body::upkeep(&w, &b, days, mass));
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
            m[r.rich] = 40.0 * w.cfg.phys.take * days * b.equiv(&w.cfg.phys, mass) / w.chem.subs[r.rich].molar_mass;
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

/// Dead body matter on the ground oxidises in the open at the rate chemistry 0 sets: 5% of it a
/// year at 290 K in the starting air (a little slower than the energy-rich solid's 6.6%: structure
/// resists), by mass action of the oxygen-like gas to the 6th power (0.21^6 = 8.6e-5) over a rate
/// constant of 1.9e-5 /s at 290 K.
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
    assert!(matter > 0.045 && matter < 0.055, "{matter}");
    assert!(solid > 0.06 && solid < 0.075 && matter < solid, "{solid}");
    // It burns: 2805 kJ a mol, the heat of 6 of the solid, and balances in elements (validated on load).
    let burn = (0..chem.reactions.len()).find(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == m)).expect("a burning reaction");
    assert!((chem.dh(burn) / -2805e3 - 1.0).abs() < 1e-9 && chem.validate().is_ok());
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
        let cap = w.cfg.phys.take * days * b.equiv(&w.cfg.phys, mass);
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
        let on_ground0 = w.ground[r.matter * n + i];
        for _ in 0..10 {
            day(&mut w);
        }
        let b = &w.life.bodies[s];
        assert_eq!(pcore::check::broken(&w), None);
        let (de, dq) = pcore::check::drift(&w, &t0, &l0);
        assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
        (b.store[r.rich] - rich0, b.store[r.matter] - matter0, on_ground0 - w.ground[r.matter * n + i], b.size(&w.chem) - size0)
    };
    let (rich, matter, taken, size) = take_from_ground(true);
    println!("an eater took {taken:.2} mol of matter from the ground: solid +{rich:.2} mol, its own matter {matter:+.3} mol, body {size:+.3} kg");
    assert!(taken > 0.5, "it took none");
    // The solid rose by 6 per mol taken (less what it burnt in ten days); its own matter did not rise.
    assert!(rich > 3.0 * taken, "solid +{rich} mol for {taken} mol of matter");
    assert!(matter <= 1e-9 && size <= 1e-9, "matter entered as body: {matter} mol, body {size} kg");
    let (_, _, none, _) = take_from_ground(false);
    assert!(none < 1e-12, "a body that cannot burn matter took {none} mol of it");
    // The reverse of growth, exactly, called directly.
    let (mut w, i) = start();
    let r = roles(&w);
    let mut b = eater(&mut w, i, 0, 6.0);
    add(&mut w, &mut b, r.matter, 1.62);
    let (s, mut b) = out(&mut w, b);
    let (t0, l0, before) = (pcore::check::totals(&w), w.ledger.clone(), b.store.clone());
    let x = b.digest(&w.chem, 10.0);
    assert!((x - 10.0).abs() < 1e-12);
    assert!((before[r.matter] - b.store[r.matter] - 10.0).abs() < 1e-9 && (b.store[r.rich] - before[r.rich] - 60.0).abs() < 1e-9 && (before[r.liquid] - b.store[r.liquid] - 10.0).abs() < 1e-9);
    w.life.bodies[s] = b;
    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-12, "element drift {de:e}, energy drift {dq:e}");
}
