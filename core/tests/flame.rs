//! Flames in the remains on the ground (laws.md 3c): dry remains that cover the ground burn by the chemistry's burning
//! reactions, wet ones do not, the moisture of extinction falls out of the heat balance, and a crown under the scorch
//! height loses its plants where one above loses none; the books are kept.

use pcore::cover::{Parts, MATTER, WALL};
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
/// and Flow's state found again from them.
fn liquid_to(w: &mut World, i: usize, mol: f64) {
    let (p, t) = (Parts::of(&w.chem).unwrap(), w.t_ground_last(i));
    testkit::set_amount(w, Node::ground(i), p.liquid, mol);
    testkit::hold_ground(w, i, t + WARMER);
    pcore::flow::state(w, true);
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

/// The same remains on ground at field capacity and on ground holding nothing for roots: the wet one does not burn; the
/// dry one burns the share of them its remains cover, the burnt mass leaving by the burning reactions (the
/// frame's gas to the air, the mineral of body matter to the ground, as ash), and every element and the energy are
/// kept.
#[test]
fn dry_remains_burn_and_wet_ones_do_not() {
    let (mut wet, i) = start();
    let p = Parts::of(&wet.chem).unwrap();
    let held = wet.g(p.liquid, i);
    liquid_to(&mut wet, i, 10.0 * held);
    assert_eq!(pcore::cover::soak(&wet, &p, i, wet.g(p.liquid, i)).1.moist, 1.0, "the wet ground is at field capacity");
    let mut dry = wet.clone();
    liquid_to(&mut dry, i, 0.0);
    assert!(!bed_of(&wet, i).carries() && bed_of(&dry, i).carries(), "wet {:?}, dry {:?}", bed_of(&wet, i), bed_of(&dry, i));
    let before = remains(&wet, i);
    flame::month(&mut wet);
    assert_eq!(remains(&wet, i), before, "wet remains burn");
    let (bed, books) = (bed_of(&dry, i), Books::now(&dry));
    // The gas the frame's burning gives, and the mineral body matter's burning gives back.
    let burner = &dry.chem.reactions[dry.chem.burner(p.sub[WALL]).unwrap()];
    let gas = burner.outputs.iter().map(|o| o.0 as usize).find(|&k| dry.chem.phase(k, dry.t_ground(i)) == pcore::chem::Phase::Gas).unwrap();
    let ash = dry.chem.minerals()[0].0;
    let air = |w: &World| (0..w.mesh.level(pcore::AIR).n()).map(|j| w.a(gas, j)).sum::<f64>() + w.store.amounts(Class::Aside).iter().skip(gas * w.mesh.cells().n()).take(w.mesh.cells().n()).sum::<f64>();
    let (gas0, ash0) = (air(&dry), dry.g(ash, i));
    flame::month(&mut dry);
    // (What the flame killed of the cover lies on the ground after it: the remains it burnt are counted.)
    let mm = p.mm(&dry.chem);
    let load: f64 = before.iter().zip([MATTER, WALL]).map(|(m, q)| m * mm[q]).sum();
    let burnt = dry.counts.flame[flame::REMAINS] / load;
    assert!((burnt - bed.cover).abs() < 1e-9, "burnt {burnt} of the remains, their cover {}", bed.cover);
    assert!(air(&dry) > gas0 && dry.g(ash, i) > ash0, "the gas to the air, the ash to the ground");
    books.assert_kept(&dry, 1e-9);
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

/// A slot whose crown stands under the scorch height loses the burnt share of its plants; a slot of one tall plant,
/// whose crown rises above it, loses none.
#[test]
fn a_crown_under_the_scorch_height_loses_plants() {
    let (mut w, i) = start();
    liquid_to(&mut w, i, 0.0);
    let tree = KINDS.iter().position(|k| k.name == "big slow light-eaters").unwrap();
    assert!(pcore::cover::place(&mut w, i, KINDS[tree].genes, tree as u32, KINDS[tree].kg));
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
    let (low0, tall0) = (plants(&w, low), plants(&w, tall));
    let share = flame::shares(&w, &(0..w.mesh.cells().n()).map(|j| flame::bed(&w, &p, j)).collect::<Vec<_>>(), w.cfg.calendar().month as f64)[i];
    assert!(share > 0.0, "the cell burns");
    flame::month(&mut w);
    let kept = plants(&w, low) / low0;
    assert!((kept - (1.0 - share)).abs() < 1e-9, "the low slot kept {kept} of its plants, burnt {share}");
    assert_eq!(plants(&w, tall), tall0, "the tall slot lost plants");
}
