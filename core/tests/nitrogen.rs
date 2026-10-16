//! The minerals of the ground (the nitrogen mineral, 15 Oct 2026, queued item 4 of handover 13.2, agreed on 14 Oct 2026; the rock salt,
//! 15 Oct 2026): body matter holds nitrogen at a carbon to nitrogen ratio of 20 and one atom of the rock element in 3,456 g, and is
//! built of the energy-rich solid and two minerals the ground holds, substance 24 (a salt that dissolves in the common liquid) and
//! substance 23 (the salt that lives only dissolved in the liquid and holds the liquid's carbon, which pores of loose ground
//! holding lime make); a body without a mineral cannot build; dead matter gives them back when it rots (the nitrogen as the mineral,
//! the rock element as lime), and gives nitrogen gas to the air when it burns hot; a body whose genes speed up the reaction that
//! fixes the air's nitrogen makes the nitrogen mineral, at the energy cost of the reaction; roots take the minerals from the ground by
//! the uptake genes; what a body holds of them past its level goes back to the ground; liquid carries them.

use pcore::body::{self, Body, Layout, Lineage, DISPERSE, LIQ, REPAIR, STRENGTH};
use pcore::chem::Chemistry;
use pcore::rng::{Purpose, Rng};
use pcore::{brain, change, check, choose, copy, flow, World};

/// A small world after one month, and a land cell with dry ground.
fn start() -> (World, usize) {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let i = (0..w.grid.n()).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
    (w, i)
}

/// A small world after a year, and a land cell with no liquid standing, whose pores hold liquid, and the rock salt in it.
fn start_wet() -> (World, usize) {
    let mut w = worldgen::new_world(1, 16, 16);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let n = w.grid.n();
    let i = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3 && w.ground[23 * n + i] > 1e5).expect("wet land");
    (w, i)
}

/// A starting body of kind `which` (0 light-eaters, 1 eaters) of a brain that only does fixed action `act` (0 rests, 3 takes
/// from the ground), weak, staying where it is, soaking no liquid up, repair 0.
fn body_of(w: &mut World, which: usize, i: usize, act: usize) -> Body {
    let shape = choose::shape(w.chem.ns(), 12, 6);
    let mut fixed = [-8.0; brain::FIXED];
    fixed[act] = 8.0;
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, fixed, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let (l, g) = (w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: which as u32 }), w.life.new_genome(rows));
    let mut b = worldgen::life::make(w, which, i, l, g).expect("body");
    (b.genes[DISPERSE], b.genes[STRENGTH], b.genes[LIQ], b.genes[REPAIR]) = (0.0, 0.0, -4.0, 0.0);
    b
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

/// Elements and (thermal + chemical) energy of one node, with its gas share.
fn node_totals(chem: &Chemistry, n: &[f64], gas: &[f64], e: f64) -> (Vec<f64>, f64) {
    let mut el = vec![0.0; chem.n_elements];
    let mut en = e;
    for k in 0..chem.ns() {
        chem.add_elems(k, n[k] + gas[k], &mut el);
        en += (n[k] + gas[k]) * chem.subs[k].hf;
    }
    (el, en)
}

/// `amount` mol of substance `k` added to the ground of cell `i` at its temperature, booked as an edit.
fn put(w: &mut World, k: usize, i: usize, amount: f64) {
    let n = w.grid.n();
    let e = amount * w.chem.subs[k].cp * w.t_ground(i);
    w.ground[k * n + i] += amount;
    w.e_ground[i] += e;
    w.chem.add_elems(k, amount, &mut w.ledger.imposed_elems);
    w.ledger.imposed_energy += e + amount * w.chem.subs[k].hf;
}

/// All of substance `k` in the world's ground and bodies, mol.
fn total(w: &World, k: usize) -> f64 {
    let n = w.grid.n();
    // (With the store of the bed under the ground, where drained liquid takes what is dissolved in it.)
    w.ground[k * n..(k + 1) * n].iter().sum::<f64>() + w.deep.get(k * n..(k + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>()) + w.life.alive().map(|b| b.store[k]).sum::<f64>()
}

/// What the chemistry says: the matter holds one nitrogen to 20 carbon (120 and 6 in its formula unit, three times the unit it was) and
/// one atom of the rock element, 1.16% of its mass, in the range of the dry matter of land plants (0.5 to 2%); it is made of 118 of the
/// energy-rich solid, 3 of the nitrogen mineral and 1 of the rock salt without a gain or loss of heat, and burns for about what it
/// did (15.9 MJ a kg; 16.7 before the rock salt) giving the nitrogen back as the mineral and the rock element as lime; the minerals dissolve
/// in the liquid; fixing the air's nitrogen takes 490 kJ a mol of the gas, what 16 molecules of the carrier of the enzyme's energy
/// (30.5 kJ each) hold; every reaction and growth balance in elements.
#[test]
fn the_chemistry_balances_and_fixing_costs_what_the_enzyme_spends() {
    let (chem, _) = worldgen::chemistry0();
    assert!(chem.validate().is_ok());
    let (m, minerals) = (chem.matter().expect("matter"), chem.minerals());
    let (mineral, salt, lime) = (24, 23, 10);
    assert_eq!((m, minerals.clone()), (22, vec![(mineral, 3.0), (salt, 1.0)]));
    // Carbon (element 1) to nitrogen (element 2) of the matter: 120 to 6; the rock element (5): one atom.
    assert_eq!((chem.subs[m].elems[1], chem.subs[m].elems[2], chem.subs[m].elems[5]), (120, 6, 1));
    // Its mass share, the rock element's mass found from the lime burnt (substance 11, 56 g a mol) less its oxygen (16).
    let rock = chem.subs[11].molar_mass - chem.subs[2].molar_mass / 2.0;
    let share = rock / chem.subs[m].molar_mass;
    println!("the rock element is {:.2}% of the dry matter ({} g of {} g)", 100.0 * share, 1e3 * rock, 1e3 * chem.subs[m].molar_mass);
    assert!(share > 0.005 && share < 0.02, "{share}");
    assert!(chem.dh_build().abs() < 1.0, "growth takes {} J", chem.dh_build());
    // Growth gives off only the liquid (the salt's carbon goes into the matter): nothing to the air.
    assert!(chem.build.as_ref().expect("growth").outputs.iter().all(|p| p.0 == m as u16 || chem.phase(p.0 as usize, 290.0) == pcore::chem::Phase::Liquid));
    let burn = (0..chem.reactions.len()).find(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == m) && chem.reactions[r].outputs.iter().any(|p| p.0 as usize == mineral)).expect("rotting");
    let per_kg = -chem.dh(burn) / chem.subs[m].molar_mass / 1e6;
    println!("body matter burns for {per_kg:.2} MJ/kg");
    assert!(per_kg > 15.5 && per_kg < 17.0, "{per_kg} MJ/kg");
    // Rotting gives back the mineral for the nitrogen (3 a mol) and the lime for the rock element (1 a mol), and no salt (no liquid goes in).
    let out = |k: usize| chem.reactions[burn].outputs.iter().find(|p| p.0 as usize == k).map_or(0, |p| p.1);
    assert!(out(mineral) == 3 && out(lime) == 1 && out(salt) == 0);
    assert!(chem.reactions[burn].inputs.iter().all(|p| p.0 as usize == m || chem.phase(p.0 as usize, 290.0) == pcore::chem::Phase::Gas));
    // The rock salt lives dissolved in the liquid and never boils; only the reaction that undoes its making takes it.
    assert!(chem.phase(salt, 290.0) == pcore::chem::Phase::Liquid && chem.subs[salt].boil > 2000.0);
    let takes: Vec<usize> = (0..chem.reactions.len()).filter(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == salt)).collect();
    assert!(takes.len() == 1 && chem.dh(takes[0]) > 0.0 && chem.reactions[takes[0]].outputs.iter().any(|p| p.0 as usize == lime));
    // The fixing reaction: air's nitrogen gas (1), the energy-rich solid and the oxygen-like gas -> the mineral, the reduced carbon gas,
    // the carbon gas and the liquid. It is the reaction that takes 490 kJ a mol of the gas (16 x 30.5, the 16 ATP of nitrogenase)
    // together with the burning of the units of the solid that real fixers respire for it: 4.7 to 6.6 g of carbon a g of nitrogen
    // (Layzell et al. 1979; models take 6). It
    // releases heat, but less from each unit of the solid than burning does: it keeps a part (`Chemistry::keeps`), so it is no burning
    // and a body never pays its upkeep with it; burning itself is a burning.
    let fix = worldgen::life::fixing(&chem).expect("fixing");
    let (_, rich, _, burn1) = worldgen::life::roles(&chem).expect("roles");
    let units = |r: usize| chem.reactions[r].inputs.iter().find(|p| p.0 as usize == rich).map_or(0.0, |p| p.1 as f64);
    let (unit, n) = (-chem.dh(burn1) / units(burn1), units(fix));
    let carbon = n * 12.0 / 28.0;
    println!("fixing burns {n} units of the solid a mol of nitrogen gas ({carbon:.1} g of carbon a g of nitrogen), {:.1} kJ; less the {:.1} kJ of each unit, {:.0}: 16 x 30.5 = {:.0}", chem.dh(fix) / 1e3, unit / 1e3, (chem.dh(fix) + n * unit) / 1e3, 16.0 * 30.5);
    assert!((4.7..=6.6).contains(&carbon), "{carbon} g of carbon a g of nitrogen");
    assert!(((chem.dh(fix) + n * unit) / (16.0 * 30.5e3) - 1.0).abs() < 0.01, "{} J", chem.dh(fix) + n * unit);
    assert!(chem.dh(fix) < 0.0 && chem.keeps(fix) && !chem.keeps(burn1) && !chem.keeps(burn));
    // The mineral is a salt that lives dissolved in the liquid (a liquid that never boils); nothing burns it in a gas, and hot it breaks down into the air's nitrogen gas (exothermic).
    assert!(chem.phase(mineral, 290.0) == pcore::chem::Phase::Liquid && chem.subs[mineral].boil > 2000.0, "a salt that lives in the liquid and never boils");
    let uses: Vec<usize> = (0..chem.reactions.len()).filter(|&r| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == mineral)).collect();
    assert_eq!(uses.len(), 1);
    assert!(chem.dh(uses[0]) < 0.0 && chem.reactions[uses[0]].outputs.iter().any(|p| p.0 == 1) && !chem.reactions[uses[0]].inputs.iter().any(|p| chem.phase(p.0 as usize, 290.0) == pcore::chem::Phase::Gas));
    assert!(chem.reactions[uses[0]].ea > 15000.0);
}

/// Dead matter gives the minerals back: in the open at 290 K it rots (0.2 a year, the litter constant) and every mol that goes leaves three
/// of the nitrogen mineral and one of lime (the rock element, which has no liquid here to dissolve in) and no nitrogen gas; burning hot
/// (1000 K) the mineral it leaves breaks down as it is made, and most of the nitrogen of the matter goes to the air as the gas, while
/// the rock element stays in the solids (lime, then what the heat makes of it). Elements and energy are kept exactly.
#[test]
fn rotting_gives_the_minerals_back_and_burning_hot_gives_the_gas_to_the_air() {
    let (chem, init) = worldgen::chemistry0();
    let (m, minerals) = (chem.matter().expect("matter"), chem.minerals());
    let (mineral, lime, rock) = (minerals[0].0, 10usize, 5usize);
    let gas_n = 1usize;
    let mut rc = change::Reactor::new(&chem);
    let mut run = |t: f64, secs: f64| {
        let mut n = vec![0.0; chem.ns()];
        let mut gas = vec![0.0; chem.ns()];
        for &(g, x) in &init.air {
            gas[g] = x * 1e9;
        }
        n[m] = 1000.0;
        let mut e = t * 1e15;
        let (el0, en0) = node_totals(&chem, &n, &gas, e);
        let gas0 = gas.clone();
        // (A product waits for the next call to react further: the mineral breaks down in the calls after the one that makes it.)
        for _ in 0..3 {
            rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, secs / 3.0);
        }
        let (el1, en1) = node_totals(&chem, &n, &gas, e);
        for j in 0..chem.n_elements {
            assert!((el1[j] - el0[j]).abs() <= 1e-9 * el0[j].max(1.0), "element {j}: {} -> {}", el0[j], el1[j]);
        }
        assert!((en1 - en0).abs() <= 1e-9 * en0.abs(), "energy {en0} -> {en1}");
        // The rock element outside the matter, in the solids (mol), by the elements of what is left.
        let ca: f64 = (0..chem.ns()).filter(|&k| k != m).map(|k| chem.subs[k].elems[rock] as f64 * n[k]).sum();
        (1000.0 - n[m], n[mineral], gas[gas_n] - gas0[gas_n], n[lime], ca)
    };
    let (lost, back, gas, lime_back, ca) = run(290.0, 365.0 * 86_400.0);
    println!("a year at 290 K: {lost:.2} mol of matter rotted, {back:.2} mol of the mineral back, {lime_back:.2} mol of lime, {gas:.4} mol of nitrogen gas");
    assert!(lost > 170.0 && lost < 205.0, "{lost}");
    assert!((back / (3.0 * lost) - 1.0).abs() < 1e-6 && gas.abs() < 1e-6 * lost, "mineral {back}, gas {gas}");
    // (A little of the lime meets the liquid the rotting makes and the gas, and is the salt: the rock element is counted in all solids and liquids.
    // Under the smooth rate of Law 2 that goes with the lime, its contact: 0.7% of it in this year, so the lime is held to 1%.)
    assert!((lime_back / lost - 1.0).abs() < 1e-2 && (ca / lost - 1.0).abs() < 1e-6, "lime {lime_back}, the rock element {ca}, for {lost} mol of matter");
    let (lost, back, gas, _, ca) = run(1000.0, 1e5);
    println!("hot, 1000 K: {lost:.1} mol of matter burned, {back:.1} mol of the mineral, {gas:.1} mol of nitrogen gas, {ca:.1} mol of the rock element in solids");
    assert!(lost > 990.0, "{lost}");
    assert!(gas > 0.8 * 3.0 * lost && back < 0.2 * 3.0 * lost, "mineral {back}, gas {gas} for {lost} mol of matter");
    assert!((ca / lost - 1.0).abs() < 1e-6, "the rock element in solids {ca} for {lost} mol of matter");
}

/// A body without a mineral cannot build body matter (not a gram of it, whichever of the two is missing), the tally says which one
/// held it back (the nitrogen mineral, or the rock salt); with both, it builds, and each falls by its count for the matter made (3 and 1
/// for 1); with a part of one, the matter made binds to it in proportion.
#[test]
fn growth_stops_without_a_mineral_and_goes_on_with_them() {
    let (mut w, i) = start();
    w.cfg.phys.c_build = 1e6;
    let minerals = w.chem.minerals();
    let m = w.chem.matter().expect("matter");
    let b = body_of(&mut w, 1, i, 0);
    let (_, b) = out(&mut w, b);
    assert!(minerals.iter().all(|&(k, _)| b.store[k] > 0.0), "a starting body holds the minerals its matter was made of");
    let days = b.interval() as f64;
    // With them it builds, and for the matter made it spends each by its count.
    let m0 = b.store[m];
    let mut with = b.clone();
    assert!(copy::grow(&mut w, &mut with, days), "it did not grow with the minerals");
    let made = with.store[m] - m0;
    for &(k, per) in &minerals {
        assert!(made > 0.0 && ((b.store[k] - with.store[k]) / (per * made) - 1.0).abs() < 1e-9, "{made} mol of matter for {} of substance {k}", b.store[k] - with.store[k]);
    }
    for (q, &(k, per)) in minerals.iter().enumerate() {
        // Without it, none: not a gram of matter, and the tally of the year counts it held back by that mineral and not the other.
        let mut b = b.clone();
        b.store[k] = 0.0;
        w.diag.nlim.clear();
        assert!(!copy::grow(&mut w, &mut b, days), "it grew without substance {k}");
        assert_eq!(b.store[m], m0);
        let tally = w.diag.nlim[w.life.root(&b) as usize].clone();
        println!("without substance {k}: held back {} (by the nitrogen mineral {}, by the rock salt {}) of {} members that could build", tally[1], tally[2], tally[3], tally[0]);
        assert!(tally[0] > 0.0 && tally[1] == tally[0] && tally[2 + q] == tally[0] && tally[3 - q] == 0.0);
        // A little of it, a little matter: it binds, in proportion.
        b.store[k] = 0.5 * made * per;
        assert!(copy::grow(&mut w, &mut b, days));
        let grown = b.store[m] - m0;
        assert!((grown / (0.5 * made) - 1.0).abs() < 1e-6, "{grown} mol for a store of {} mol of substance {k}", 0.5 * made * per);
        assert!(b.store[k] < 1e-9 * made);
    }
}

/// The rock element (5) outside body matter on the ground of cell `i`, mol (lime, the salt, quicklime...).
fn rock_outside(w: &World, i: usize) -> f64 {
    let (m, n) = (w.chem.matter().expect("matter"), w.grid.n());
    (0..w.chem.ns()).filter(|&k| k != m).map(|k| w.chem.subs[k].elems[5] as f64 * w.ground[k * n + i]).sum()
}

/// Dead matter on the ground returns its nitrogen and its rock element to the ground: in a world with no body, a year, the matter that
/// went is in the nitrogen mineral three for one (the mineral dissolves and runs with the liquid: it is counted over the whole ground)
/// and in the rock element outside the matter one for one (as lime, or the salt that lime and the liquid and gas make of it), more than
/// the same world without the matter makes (the open air fixes next to nothing), and elements and energy are kept.
#[test]
fn dead_matter_on_the_ground_returns_the_minerals_to_the_ground() {
    let (mut w, i) = start();
    let (mineral, per) = w.chem.minerals()[0];
    let m = w.chem.matter().expect("matter");
    // The same world with nothing on the ground: what the open makes of the mineral in a year.
    let mut twin = w.clone();
    let bare = total(&twin, mineral);
    for _ in 0..12 {
        pcore::step(&mut twin);
    }
    let (bare, bare_rock) = (total(&twin, mineral) - bare, rock_outside(&twin, i));
    put(&mut w, m, i, 1e8);
    let (t0, l0) = (check::totals(&w), w.ledger.clone());
    let (m_before, n_before) = (total(&w, m), total(&w, mineral));
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let (lost, gained, rock) = (m_before - total(&w, m), total(&w, mineral) - n_before - bare, rock_outside(&w, i) - bare_rock);
    assert!(bare >= 0.0 && bare < 1e-3 * lost, "the open fixed {bare} mol in a year");
    println!("a year on the ground: {lost:.0} of {m_before:.0} mol of matter rotted, {gained:.0} mol of the mineral back, {rock:.0} mol of the rock element more than the twin's cell");
    assert!(lost > 0.02 * m_before, "it did not rot: {lost}");
    assert!((gained / (per * lost) - 1.0).abs() < 1e-4, "the mineral {gained} for the matter {lost}");
    // (In the cell, to a tenth: the salt the rotting's gas and liquid make of the lime moves with the liquid, to and from the cells
    // around; the ledger of elements below is exact.)
    assert!((rock / lost - 1.0).abs() < 0.1, "the rock element {rock} for the matter {lost}");
    let (de, dq) = check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// Fixing the air's nitrogen: in the open it is nothing (1e-17 a second at 290 K), and a body whose genes speed the reaction up
/// makes the mineral from the air's gas and the energy-rich solid it burns, and puts the reduced carbon gas out, at the cost of the
/// reaction: it burns 14 units of its fuel for each mol of the mineral, and their heat pays none of its upkeep (it burns that much
/// more than its twin), and the enzyme costs upkeep by its capacity like the others. A body that has none fixes none. Elements and
/// energy are kept.
#[test]
fn a_body_that_fixes_makes_the_mineral_at_the_cost_of_the_reaction() {
    let (chem, init) = worldgen::chemistry0();
    let (mineral, _) = chem.minerals()[0];
    let fix = worldgen::life::fixing(&chem).expect("fixing");
    let (_, rich, _, _) = worldgen::life::roles(&chem).expect("roles");
    // In the open: ten years at 290 K, the air in plenty and the liquid and the fuel there.
    let mut rc = change::Reactor::new(&chem);
    let mut n = vec![0.0; chem.ns()];
    let mut gas = vec![0.0; chem.ns()];
    for &(g, x) in &init.air {
        gas[g] = x * 1e9;
    }
    (n[0], n[rich]) = (1e6, 1e6);
    let mut e = 290.0 * 1e15;
    rc.react(&chem, &mut n, Some(&mut gas), &mut e, 1e15, None, 0.0, 10.0 * 365.0 * 86_400.0);
    println!("in the open, ten years: {:.3e} mol of the mineral", n[mineral]);
    assert!(n[mineral] < 1e-6 * 1e6, "the open world fixed {} mol with 10^6 mol of the fuel there", n[mineral]);
    // A plant that fixes (the enzyme speeds the reaction up by 10^FIX_GENE) and its twin that does not, 60 days.
    const FIX: f32 = worldgen::life::FIX_GENE;
    let run = |cat: f32| {
        let (mut w, i) = start();
        let lay = Layout::of(&w.chem);
        let mut b = body_of(&mut w, 0, i, 0);
        // Its own mineral just short of nothing: what it holds past its level goes to the ground and is counted there.
        b.genes[lay.cat + fix] = cat;
        // (No rock salt, so neither builds: the two differ by the fixing alone. Before 15 Oct 2026 the salt fell out of the
        // body in its first update, so neither built anyway.)
        b.store[w.chem.minerals()[1].0] = 0.0;
        let s = w.life.add(b, w.day, 1);
        let (t0, l0) = (check::totals(&w), w.ledger.clone());
        let before = (total(&w, mineral), w.air[3 * w.grid.na()..4 * w.grid.na()].iter().sum::<f64>(), w.air[21 * w.grid.na()..22 * w.grid.na()].iter().sum::<f64>());
        let (mut up, mut fuel0, mut mass) = (0.0, 0.0, 0.0);
        let _ = (&mut fuel0, &mut mass);
        for _ in 0..60 {
            day(&mut w);
            let mut b = w.life.bodies[s].clone();
            b.prepare(&w.chem, &w.cfg.phys);
            // Its upkeep over that of the same body with the gene off.
            let mut off = b.clone();
            (off.genes[lay.cat + fix], off.k) = (-1.0, Default::default());
            off.prepare(&w.chem, &w.cfg.phys);
            up = body::upkeep(&w, &b, 1.0, b.mass(&w.chem)) / body::upkeep(&w, &off, 1.0, b.mass(&w.chem));
        }
        assert!(w.life.bodies[s].alive);
        let (de, dq) = check::drift(&w, &t0, &l0);
        assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
        let na = w.grid.na();
        let after = (total(&w, mineral), w.air[3 * na..4 * na].iter().sum::<f64>(), w.air[21 * na..22 * na].iter().sum::<f64>());
        let b = &w.life.bodies[s];
        (after.0 - before.0, after.2 - before.2, up, b.k.cats, b.store[rich], b.mass(&w.chem))
    };
    let (fixed, co, up_with, cats_with, fuel_with, mass) = run(FIX);
    let (none, co_none, up_without, cats_without, fuel_without, _) = run(-1.0);
    println!("60 days: the fixing plant made {fixed:.3e} mol of the mineral and put out {co:.3e} mol of the reduced carbon gas, upkeep {up_with:.4} times that with the gene off ({up_without:.4}); the other {none:.3e} mol; fuel {fuel_with:.4e} against {fuel_without:.4e}");
    // (What the enzyme made is the difference.)
    let made = fixed - none;
    assert!(made > 1.0, "the plant fixed {made} mol");
    println!("it fixed {:.2e} mol of the mineral a kg of plant and second", made / (mass * 60.0 * 86_400.0));
    // One mol of the reduced carbon gas for each mol of the mineral made; it is where the energy of the reaction went.
    assert!((co - co_none) / made > 0.9 && (co - co_none) / made < 1.1, "{} mol of the gas for {made} of the mineral", co - co_none);
    // And 14 mol of its fuel for each, the units the reaction burns (and the enzyme's upkeep, a little more): their heat paid no upkeep.
    let units = chem.reactions[fix].inputs.iter().find(|p| p.0 as usize == rich).expect("the fuel").1 as f64;
    assert!((fuel_without - fuel_with) / made > 0.95 * units && (fuel_without - fuel_with) / made < 1.2 * units, "{} mol of fuel for {made} of the mineral", fuel_without - fuel_with);
    // The enzyme costs by its capacity: the plant that fixes pays more upkeep than it would with the gene off.
    assert!(cats_with > cats_without && up_with > 1.0 && up_without == 1.0, "capacity {cats_with} against {cats_without}, upkeep {up_with} against {up_without}");
}

/// Roots take each mineral from the ground by the uptake genes, up to the level the body keeps and no more; a body whose gene is off
/// takes none. What a body holds of one past its level goes back to the ground at its update, with its heat (an eater that digested
/// matter, a plant that burned its own); the rock salt, which falls out of the liquid in a body in days as lime, goes back as lime past
/// the level the body keeps of lime (16 Oct 2026: lime is a solid bodies build, kept at its share, `Body::level`; before, all of it).
/// Elements and energy are kept.
#[test]
fn roots_take_the_minerals_up_to_the_level_and_the_excess_goes_back() {
    let (w0, _) = start();
    for (q, (mineral, _)) in w0.chem.minerals().into_iter().enumerate() {
        // (The salt lives only in the liquid of the pores, which lime and the gas make it in: no salt is put on this ground, it has its own.)
        let (mut w, i) = start_wet();
        let lay = Layout::of(&w.chem);
        let n = w.grid.n();
        let lime = 10;
        // A plant with its store of this mineral emptied, the ground full of it.
        let mut plant = body_of(&mut w, 0, i, 0);
        // (It soaks as a plant does, 1 a day, the rate roots take what is dissolved at.)
        plant.genes[LIQ] = 0.0;
        let mut idle = plant.clone();
        for b in [&mut plant, &mut idle] {
            let a = b.store[mineral];
            w.chem.add_elems(mineral, -a, &mut w.ledger.imposed_elems);
            w.ledger.imposed_energy -= a * w.chem.subs[mineral].hf;
            b.store[mineral] = 0.0;
        }
        idle.genes[lay.upt + mineral] = -0.1;
        assert!(plant.genes[lay.upt + mineral] > 0.0);
        if q == 0 {
            put(&mut w, mineral, i, 1e9);
        }
        let level = |w: &World, b: &Body| b.level(&w.chem, mineral, b.mass(&w.chem));
        let (sp, si) = (w.life.add(plant, w.day, 1), w.life.add(idle, w.day, 1));
        let (t0, l0) = (check::totals(&w), w.ledger.clone());
        // (The most it held in the days, at the end of an update: the salt is lost to lime between them.)
        let mut most = 0.0f64;
        for _ in 0..40 {
            day(&mut w);
            most = most.max(w.life.bodies[sp].store[mineral]);
        }
        let (took, lvl) = (w.life.bodies[sp].store[mineral], level(&w, &w.life.bodies[sp]));
        println!("substance {mineral}: a plant with the gene on held {took:.3e} mol (at most {most:.3e}), its level {lvl:.3e}, lime {:.3e}; the other {:.3e}", w.life.bodies[sp].store[lime], w.life.bodies[si].store[mineral]);
        assert!(most > 0.1 * lvl && most <= lvl * (1.0 + 1e-6), "held at most {most}, level {lvl}");
        let kept = |w: &World, s: usize| w.life.bodies[s].store[lime] <= w.life.bodies[s].level(&w.chem, lime, 0.0) * (1.0 + 1e-9);
        assert!(kept(&w, sp) && kept(&w, si), "lime past its level stayed in a body");
        assert!(w.life.bodies[si].store[mineral] < 1e-9 * lvl, "a body whose gene is off took substance {mineral}");
        let (de, dq) = check::drift(&w, &t0, &l0);
        assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
        // The excess: give the plant ten times its level, and it is back on the ground at its next update (as lime for the salt: the
        // rock element in the ground, counted in the salt and the lime, rises by what was over).
        let mut twin = w.clone();
        let more = 10.0 * lvl;
        w.chem.add_elems(mineral, more, &mut w.ledger.imposed_elems);
        w.ledger.imposed_energy += more * (w.chem.subs[mineral].hf + w.chem.subs[mineral].cp * w.life.bodies[sp].temp(&w.chem));
        w.life.bodies[sp].e += more * w.chem.subs[mineral].cp * w.life.bodies[sp].temp(&w.chem);
        w.life.bodies[sp].store[mineral] += more;
        for _ in 0..40 {
            day(&mut twin);
            day(&mut w);
        }
        let (held, lvl2) = (w.life.bodies[sp].store[mineral], level(&w, &w.life.bodies[sp]));
        // (Against its twin, the same world without the excess: the nitrogen mineral by itself, the rock element in all its forms.)
        let back = if q == 0 { w.ground[mineral * n + i] - twin.ground[mineral * n + i] } else { rock_outside(&w, i) - rock_outside(&twin, i) };
        println!("with ten times its level more: it holds {held:.3e} against a level of {lvl2:.3e}, lime {:.3e}; the ground gained {back:.3e} (the excess {more:.3e})", w.life.bodies[sp].store[lime]);
        assert!(held <= lvl2 * 1.05, "held {held} past the level {lvl2}");
        assert!(kept(&w, sp), "lime past its level stayed in the body");
        assert!(back > 0.5 * more, "the excess did not go back to the ground: {back} of {more}");
        let (de, dq) = check::drift(&w, &t0, &l0);
        assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
    }
}

/// An eater digests the matter it eats into the solid and the minerals (the reverse of growth) and takes it in even when the minerals
/// it makes would fill the room its level leaves many times over: what it holds of them past its level goes back to the ground at its
/// update (the nitrogen mineral, and the rock element as the salt or the lime it falls out as), so digestion never stops for them.
/// Elements and energy are kept.
#[test]
fn an_eater_digests_matter_into_the_solid_and_the_minerals_and_sheds_the_excess() {
    let (mut w, i) = start();
    let minerals = w.chem.minerals();
    let m = w.chem.matter().expect("matter");
    let (_, rich, _, _) = worldgen::life::roles(&w.chem).expect("roles");
    let n = w.grid.n();
    // A great store of matter on its ground, an eater that only takes from the ground.
    put(&mut w, m, i, 1e5);
    let mut b = body_of(&mut w, 1, i, 3);
    // It keeps little of the minerals (a thousandth of its body): what digestion makes of the matter it takes is many times that.
    for &(k, _) in &minerals {
        b.genes[Layout::of(&w.chem).keep + k] = 0.001;
    }
    let s = w.life.add(b, w.day, 1);
    let (t0, l0) = (check::totals(&w), w.ledger.clone());
    let (matter0, rich0, ground0) = (w.ground[m * n + i], w.life.bodies[s].store[rich], minerals.iter().map(|&(k, _)| w.ground[k * n + i]).collect::<Vec<_>>());
    let mut most = vec![0.0f64; minerals.len()];
    for _ in 0..60 {
        day(&mut w);
        let b = &w.life.bodies[s];
        assert!(b.alive);
        for (q, &(k, _)) in minerals.iter().enumerate() {
            most[q] = most[q].max(b.store[k] / b.level(&w.chem, k, b.mass(&w.chem)).max(1e-30));
        }
    }
    let b = &w.life.bodies[s];
    let taken = matter0 - w.ground[m * n + i];
    println!("in 60 days it took {taken:.2} mol of matter, the solid rose {:.1} mol; it held at most {most:.2?} of its levels", b.store[rich] - rich0);
    assert!(b.store[rich] > rich0, "the solid did not rise");
    for (q, &(k, per)) in minerals.iter().enumerate() {
        let lvl = b.level(&w.chem, k, b.mass(&w.chem));
        println!("substance {k}: level {lvl:.3} mol, the ground got {:.2} mol more (the salt's number includes what the ground itself makes)", w.ground[k * n + i] - ground0[q]);
        assert!(taken * per > 3.0 * lvl, "it took only {taken} mol of matter");
        // (The salt falls out of the liquid in the body in days, so what comes back is lime as much as salt: the nitrogen mineral by itself, the rock element by all its forms.)
        if q == 0 {
            assert!(w.ground[k * n + i] > ground0[q] + 0.5 * (taken * per - lvl), "the excess of substance {k} did not go back to the ground");
        }
    }
    // No lime past its level stays in the body (the salt falls out of the liquid in days, and the lime past the level it keeps, 16 Oct
    // 2026, goes back to the ground at the update), and what
    // it holds of the salt between two updates is a few times its level, the meals of the interval, not more and more.
    println!("the body holds {:.3} mol of salt and {} of lime at the end", b.store[23], b.store[10]);
    assert!(b.store[10] <= b.level(&w.chem, 10, 0.0) * (1.0 + 1e-9) && most[1] < 20.0, "lime {}, the salt at most {} of its level", b.store[10], most[1]);
    let (de, dq) = check::drift(&w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// The soil is given to the land when life starts, 10 g of nitrogen to the m2, booked as new matter and energy, none under
/// standing liquid; and the tape counts the mineral on land and on the low ground.
#[test]
fn the_soil_is_booked_and_the_tape_counts_the_mineral() {
    let mut w = worldgen::new_world(1, 16, 16);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let (mineral, _) = w.chem.minerals()[0];
    let (t0, l0) = (check::totals(&w), w.ledger.clone());
    let n = w.grid.n();
    let before = total(&w, mineral);
    let line = worldgen::life::soil(&mut w);
    println!("{line}");
    let (placed, area) = (total(&w, mineral) - before, w.grid.area());
    let cells = placed / (0.357 * area);
    assert!(cells.fract().abs() < 1e-9 || (1.0 - cells.fract()).abs() < 1e-9, "{cells} cells' worth");
    assert!(cells >= 10.0 && cells <= n as f64);
    // 0.357 mol of the salt, 2 mol of nitrogen in each: 10 g of nitrogen in the m2.
    let grams: f64 = 0.357 * 2.0 * 14.0;
    assert!((grams - 10.0).abs() < 0.05);
    let (de, dq) = check::drift(&w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// The rock salt in the pores of the land (15 Oct 2026): worldgen sets no salt on land (the ground has no liquid at the start, and a cell
/// with no liquid holds none), and a year of rain on loose ground that is a tenth lime makes it by the same two reactions as the
/// low ground's liquid cover (22 and 23): the cells whose pores hold liquid hold the salt, in the amount at which the two go at equal rates for their
/// liquid and the gas over them (`Reactor::balance`, to a half); a cell whose pores are dry holds none.
#[test]
fn the_pores_of_the_land_hold_the_rock_salt_where_lime_and_liquid_meet() {
    let mut w = worldgen::new_world(1, 16, 16);
    let (n, na, salt, liquid) = (w.grid.n(), w.grid.na(), 23usize, 0usize);
    let land: Vec<usize> = (0..n).filter(|&i| w.surface_z(i) >= 0.0).collect();
    assert!(land.iter().all(|&i| w.ground[salt * n + i] == 0.0), "land started with the salt");
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let mut rc = change::Reactor::new(&w.chem);
    let (mut wet, mut dry, mut ratio) = (0, 0, Vec::new());
    for &i in &land {
        if w.ground[liquid * n + i] > 1e6 {
            // The balance at the cell's own liquid, warmth and gas.
            let (t, j) = (w.t_ground(i), w.grid.air_of(i));
            let mut amt: Vec<f64> = (0..w.chem.ns()).map(|k| w.ground[k * n + i]).collect();
            amt[salt] = 0.0;
            let share = 1.0 / (w.grid.air_scale * w.grid.air_scale) as f64;
            let gas: Vec<f64> = (0..w.chem.ns()).map(|k| w.air[k * na + j] * share).collect();
            rc.balance(&w.chem, &mut amt, &gas, t);
            if amt[salt] > 0.0 {
                wet += 1;
                ratio.push(w.ground[salt * n + i] / amt[salt]);
            }
        } else if w.ground[liquid * n + i] == 0.0 {
            dry += 1;
            assert_eq!(w.ground[salt * n + i], 0.0, "a dry cell holds the salt");
        }
    }
    ratio.sort_by(|a, b| a.total_cmp(b));
    println!("{wet} land cells with liquid in the pores, {dry} dry; the salt over its balance, the cells at the 10th, 50th, 90th percent: {:.2} {:.2} {:.2}", ratio[ratio.len() / 10], ratio[ratio.len() / 2], ratio[ratio.len() * 9 / 10]);
    assert!(wet > 20, "only {wet} wet land cells");
    assert!(ratio[ratio.len() / 2] > 0.5 && ratio[ratio.len() / 2] < 2.0, "the cells' salt is far from its balance: {}", ratio[ratio.len() / 2]);
}

/// End-product inhibition (15 Oct 2026): a body's catalyst is slowed by its store of each product it keeps a level of, by
/// 1 / (1 + store / level): with none held it runs at full speed, at its level at half. Gases and the key liquid do not count,
/// so a burning (gas and the liquid) is not slowed.
#[test]
fn a_catalyst_is_slowed_by_the_product_the_body_holds() {
    let (mut w, i) = start();
    let (mineral, _) = w.chem.minerals()[0];
    let fix = worldgen::life::fixing(&w.chem).expect("fixing");
    let (light, rich, _, burn) = worldgen::life::roles(&w.chem).expect("roles");
    let b = body_of(&mut w, 0, i, 0);
    let (_, mut b) = out(&mut w, b);
    let (t, mass) = (b.temp(&w.chem), b.mass(&w.chem));
    for (r, k) in [(fix, mineral), (light, rich)] {
        b.store[k] = 0.0;
        let none = b.inhibited(&w.chem, r, mass, t);
        b.store[k] = b.level(&w.chem, k, mass);
        let full = b.inhibited(&w.chem, r, mass, t);
        assert!(none == 1.0 && (full - 0.5).abs() < 1e-9, "reaction {r}: {none} with none held, {full} at its level");
    }
    assert_eq!(b.inhibited(&w.chem, burn, mass, t), 1.0, "a burning makes gas and the liquid");
}
