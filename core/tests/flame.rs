//! Flames in the remains on the ground (laws.md 3c): dry remains that cover the ground burn by the chemistry's burning
//! reactions, wet ones do not, the moisture of extinction falls out of the heat balance, and a crown under the scorch
//! height loses its plants where one above loses none; the books are kept.

use pcore::cover::{Parts, FUEL, MATTER, WALL};
use pcore::flame::{self, Bed};
use pcore::store::{Class, Node};
use pcore::World;
use worldgen::life::{KINDS, LIGHT_ROW};
use worldgen::testkit::{self, dry_lands, small_world, Books};

/// The remains put on the test's cell, kg a m² of their two parts (body matter, walls): a forest floor's load.
const REMAINS_KG: [f64; 2] = [1.0, 2.0];
/// Strikes a m² of land a year in the tests: so many that a flame lights every region that can burn.
const ALL_STRIKES: f64 = 1.0;
/// How much warmer than the small world's month the test's ground is, K: a hot dry month, its air near the ground far
/// from saturation.
const WARMER: f64 = 15.0;
/// The share either side of the flux at which the remains' moisture is their moisture of extinction.
const NEAR: f64 = 1e-3;
/// The rounding of the liquid set to field capacity: the moisture share it gives is 1 to within it.
const ROUNDING: f64 = 1e-12;

/// A small world with its soil and some cover on its first dry land cell (so its remains have a leaf area), its
/// remains put on that cell, and strikes so dense that what can burn burns; and that cell.
fn start() -> (World, usize) {
    let mut w = small_world();
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let i = dry_lands(&w)[0];
    testkit::cover(&mut w, i, KINDS[LIGHT_ROW].kg);
    let p = Parts::of(&w.chem).unwrap();
    for (q, kg) in [MATTER, WALL].into_iter().zip(REMAINS_KG) {
        testkit::put_kg_m2(&mut w, p.sub[q], i, kg);
    }
    w.cfg.phys.strikes = ALL_STRIKES;
    (w, i)
}

/// The key liquid of the ground of cell `i` set to `mol`, its ground `WARMER` than the world left it (booked as edits),
/// and Flow's state and the wet ground the laws read found again from them.
fn liquid_to(w: &mut World, i: usize, mol: f64) {
    let (p, t) = (Parts::of(&w.chem).unwrap(), w.t_ground_last(i));
    testkit::set_amount(w, Node::ground(i), p.liquid, mol);
    testkit::hold_ground(w, i, t + WARMER);
    pcore::flow::state(w, true);
    pcore::contract::wet_grounds(w);
}

/// The bed of cell `i` as the flames read it now.
fn bed_of(w: &World, i: usize) -> Bed {
    flame::bed(w, &Parts::of(&w.chem).unwrap(), i).expect("a bed")
}

/// The remains of cell `i` (body matter and walls), mol.
fn remains(w: &World, i: usize) -> [f64; 2] {
    let p = Parts::of(&w.chem).unwrap();
    [MATTER, WALL].map(|q| w.g(p.sub[q], i))
}

/// The same remains on ground at field capacity and on ground holding nothing for roots: on the dry ground all of them
/// burn, on the wet one only their top, down to where the liquid their bottom draws from the ground stops them
/// smouldering; the burnt mass leaves by the burning reactions (the frame's gas to the air), and every element and the
/// energy are kept.
#[test]
fn dry_remains_burn_through_and_wet_ones_only_at_their_top() {
    let (mut wet, i) = start();
    let p = Parts::of(&wet.chem).unwrap();
    // Its pores filled to field capacity: wet, short of the saturation that would keep a flame off the ground.
    let (held, filled) = (wet.g(p.liquid, i), pcore::cover::fill_now(&wet, &p, i, wet.g(p.liquid, i))[1]);
    let field = wet.pores(i).field;
    liquid_to(&mut wet, i, held * field / filled);
    let moist = pcore::cover::soak(&wet, &p, i, wet.g(p.liquid, i)).1.moist;
    assert!((moist - 1.0).abs() < ROUNDING, "the wet ground is at field capacity: {moist}");
    let mut dry = wet.clone();
    liquid_to(&mut dry, i, 0.0);
    // The gas the frame's burning gives.
    let burner = &dry.chem.reactions[dry.chem.burner(p.sub[WALL]).unwrap()];
    let gas = burner.outputs.iter().map(|o| o.0 as usize).find(|&k| dry.chem.phase(k, dry.t_ground(i)) == pcore::chem::Phase::Gas).unwrap();
    let air = |w: &World| (0..w.mesh.level(pcore::AIR).n()).map(|j| w.a(gas, j)).sum::<f64>() + w.store.amounts(Class::Aside).iter().skip(gas * w.mesh.cells().n()).take(w.mesh.cells().n()).sum::<f64>();
    let mm = p.mm(&dry.chem);
    let load: f64 = remains(&dry, i).iter().zip([MATTER, WALL]).map(|(m, q)| m * mm[q]).sum();
    let mut smouldered = vec![];
    for w in [&mut wet, &mut dry] {
        let (bed, books, gas0) = (bed_of(w, i), Books::now(w), air(w));
        assert!(bed.carries(), "{bed:?}");
        flame::month(w);
        let burnt = (w.counts.flame[flame::REMAINS] + w.counts.flame[flame::SMOULDERED]) / load;
        assert!((burnt - bed.cover * bed.smoulder).abs() < 1e-9, "burnt {burnt} of the remains, their cover {} and dry share {}", bed.cover, bed.smoulder);
        assert!(air(w) > gas0, "the gas to the air");
        books.assert_kept(w, 1e-9);
        smouldered.push(bed.smoulder);
    }
    assert!(smouldered[0] < 1.0 && smouldered[1] == 1.0, "dry shares {smouldered:?}");
}

/// Remains whose moisture is just under their moisture of extinction burn, just over it do not: the flux a burning kg
/// sends ahead (`flux_ahead`) that just lights the next kg at their moisture is what lighting a kg takes over what a kg
/// frees burning.
#[test]
fn remains_burn_just_under_their_moisture_of_extinction() {
    let (mut w, i) = start();
    liquid_to(&mut w, i, 0.0);
    let bed = bed_of(&w, i);
    let just = bed.light / bed.heat;
    for (share, burns) in [(1.0 - NEAR, false), (1.0 + NEAR, true)] {
        let mut v = w.clone();
        v.cfg.phys.flux_ahead = just * share;
        let before = remains(&v, i);
        flame::month(&mut v);
        assert_eq!(remains(&v, i) != before, burns, "flux {} of {just}: moisture {} against {}", just * share, bed.moist, bed_of(&v, i).extinct);
    }
}

/// A bed just under its least load carries no flame and one just over it does: what it frees burning while it flames
/// against what its top sends to the open sky at the flaming bed's temperature (`t_burn`).
#[test]
fn no_flame_below_the_least_load() {
    let (mut w, i) = start();
    liquid_to(&mut w, i, 0.0);
    let (bed, p) = (bed_of(&w, i), Parts::of(&w.chem).unwrap());
    assert!(bed.least > 0.0 && bed.least < bed.load, "{bed:?}");
    for (share, burns) in [(1.0 - NEAR, false), (1.0 + NEAR, true)] {
        let mut v = w.clone();
        for q in [MATTER, WALL] {
            testkit::set_amount(&mut v, Node::ground(i), p.sub[q], w.g(p.sub[q], i) * bed.least * share / bed.load);
        }
        assert!(bed_of(&v, i).cover > flame::CARRY_SHARE, "{:?}", bed_of(&v, i));
        let before = remains(&v, i);
        flame::month(&mut v);
        assert_eq!(remains(&v, i) != before, burns, "a load of {} against {}", bed_of(&v, i).load, bed.least);
    }
}

/// The mineral the burning of body matter gives breaks down in the flame by the chemistry's own reaction that takes it
/// alone, over the share 1 − e^(−k τ) of it at the flaming bed's temperature: its fixed element goes to the air as the
/// gas, the rest lies on the ground as ash. Here the flame's temperature is set so that half breaks down.
#[test]
fn the_fixed_element_goes_to_the_air_and_the_rest_to_the_ground() {
    let (mut w, i) = start();
    liquid_to(&mut w, i, 0.0);
    let ash = w.chem.minerals()[0].0;
    let rx = w.chem.reactions.iter().find(|rx| rx.inputs.iter().all(|q| q.0 as usize == ash)).expect("a breaking down").clone();
    let (gas, made) = (rx.outputs[0].0 as usize, rx.outputs[0].1 as f64);
    let tau = flame::residence(&w.cfg.phys);
    w.cfg.phys.t_burn = rx.ea / (rx.k * tau / std::f64::consts::LN_2).ln();
    let ash0 = w.g(ash, i);
    flame::month(&mut w);
    let to_air = w.counts.flame_air[gas] * rx.inputs[0].1 as f64 / made;
    let to_ground = w.g(ash, i) - ash0;
    let share = to_air / (to_air + to_ground);
    assert!((share - 0.5).abs() < 1e-9, "{share} of the mineral broke down, {to_air} mol to the air, {to_ground} to the ground");
}

/// A low slot whose crown stands under the scorch height dies above the ground on the burnt share and sprouts again:
/// it keeps its plants and its store, and loses no more of its built cover than the share burnt of what stands above
/// its base; a slot of one tall plant, whose crown rises above it, loses no plants.
#[test]
fn a_low_plant_burnt_to_its_base_keeps_its_plants_and_store() {
    let (mut w, i) = start();
    liquid_to(&mut w, i, 0.0);
    let tree = KINDS.iter().position(|k| k.name == "big slow light-eaters").unwrap();
    assert!(pcore::cover::place(&mut w, i, 0, KINDS[tree].genes, tree as u32, KINDS[tree].kg));
    let slot = |w: &World, row: usize| w.life.cover(i).held().find(|&x| w.life.cover(i).root[x] == row as u32).unwrap();
    let (low, tall) = (slot(&w, LIGHT_ROW), slot(&w, tree));
    // All of the tall slot's mass in one plant: a tree.
    w.in_cell(i, |_, d| d.cover(i).plants[tall] = [1.0, 0.0, 0.0, 0.0]);
    let p = Parts::of(&w.chem).unwrap();
    let (bed, cell) = (bed_of(&w, i), w.life.cell_at(i).clone());
    let scorch = flame::reach(&w.cfg.phys, &bed).scorch;
    let top = |x: usize| pcore::contract::crown(&w, i, &cell, &p, x).1;
    assert!(top(low) < scorch && scorch < top(tall), "tops {} and {} against the scorch height {scorch}", top(low), top(tall));
    let plants = |w: &World, x: usize| w.life.cover(i).count(x);
    let mm = p.mm(&w.chem);
    let built = |w: &World| w.life.cover(i).built(&mm, low);
    let (low0, tall0, built0, fuel0) = (plants(&w, low), plants(&w, tall), built(&w), w.life.cover(i).mol[low][FUEL]);
    let share = flame::shares(&w, &(0..w.mesh.cells().n()).map(|j| flame::bed(&w, &p, j).ok()).collect::<Vec<_>>(), w.cfg.calendar().month as f64)[i];
    assert!(share > 0.0, "the cell burns");
    flame::month(&mut w);
    assert_eq!((plants(&w, low), w.life.cover(i).mol[low][FUEL]), (low0, fuel0), "the low slot lost plants or store");
    let base = w.cfg.phys.base_share;
    assert!(built(&w) < built0 && built(&w) >= built0 * (1.0 - share * (1.0 - base)) - 1e-12, "built {} of {built0}, burnt {share}", built(&w));
    assert_eq!(plants(&w, tall), tall0, "the tall slot lost plants");
}

/// Every edit keeps the books (handover.md 11: an action is a physics edit): a split, a clearing, a planting and a
/// strike on the test's dry cell, made through `World::edit`, are made and told as events, and keep every element and
/// the energy; with no strikes drawn (`strikes` 0) the strike alone lights the cell in the flames' month.
#[test]
fn every_edit_keeps_the_books_and_a_strike_lights_its_cell() {
    use pcore::edit::{Act, Edit};
    let (mut w, i) = start();
    liquid_to(&mut w, i, 0.0);
    w.cfg.phys.strikes = 0.0;
    let [w0, n0, e0, s0] = w.map.bounds(i);
    let place = [(w0 + e0) / 2.0, (n0 + s0) / 2.0];
    let books = Books::now(&w);
    let acts = [Act::Split(0.2), Act::Clear(0.1), worldgen::schedule::plant(LIGHT_ROW), Act::Strike];
    assert_eq!(w.edit(&acts.map(|act| Edit { place, act })), [true; 4], "the edits on cell {i}");
    let told = w.drain_events().iter().filter(|e| matches!(e.kind, pcore::event::EventKind::Edit(_)) && e.node == i as u32).count();
    assert_eq!((told, w.life.cover(i).patch_count()), (4, 3), "the edits told, and the patches");
    books.assert_kept(&w, 1e-9);
    flame::month(&mut w);
    assert!(w.counts.flame[flame::REMAINS] > 0.0 && w.strikes.is_empty(), "the strike burnt {} kg", w.counts.flame[flame::REMAINS]);
    books.assert_kept(&w, 1e-9);
}
