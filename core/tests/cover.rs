//! The cover (laws.md 4.5): a field of light-eaters per cell grows in light and dies in frost, keeping the books, and
//! animals graze it by their take from the place.

use pcore::body::{PHASE, TAKEN};
use pcore::cover::{self, FROST};
use pcore::event::EventKind;
use worldgen::life::{Place, KINDS};
use worldgen::testkit::{dry_lands, eater, small_world, Books, MILD_LAT};

/// Latitudes of the south and north edges of a world whose land stays warm the year round, degrees.
const WARM_LAT: [f64; 2] = [0.0, 2.3];
/// The two frost genes of the selection test, K: above the key liquid's melting point (273 K), so the cover is active
/// (its ground's liquid is liquid) where winters fall below the higher one, and a winter can kill it.
const FROST_GENES: [f32; 2] = [285.0, 270.0];

/// The genes of the starting cover (kind 0).
fn genes() -> [f32; cover::NG] {
    let Place::Field(f) = &KINDS[0].place else { unreachable!("kind 0 is cover") };
    f.genes
}

/// The world's cover, kg.
fn cover_kg(w: &pcore::World) -> f64 {
    cover::sums(w).iter().map(|s| s.kg).sum()
}

/// A small world with its soil, after a month (Flow's state built), and a dry land cell that rain reaches.
fn start() -> (pcore::World, usize) {
    let mut w = small_world();
    pcore::step(&mut w);
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let i = dry_lands(&w).into_iter().find(|&i| w.rain[i] > 0.0).expect("dry land with rain");
    (w, i)
}

/// 0.1 kg per m² of the starting cover on one cell grows over a year of light (it builds from its light reaction and
/// the soil's minerals, and spreads seeds), and every element and the energy are kept.
#[test]
fn the_cover_grows_in_light_and_keeps_the_books() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    assert!(cover::place(&mut w, i, genes(), 0, 0.1));
    let kg0 = cover_kg(&w);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let kg = cover_kg(&w);
    println!("cover {kg0:.3e} kg, a year later {kg:.3e} kg in {} slots", cover::sums(&w)[0].slots);
    assert!(kg > 2.0 * kg0, "the cover did not grow: {kg0} then {kg}");
    assert_eq!(pcore::check::broken(&w), None);
    books.assert_kept(&w, 1e-9);
}

/// A cover far thicker than its light can keep (20 kg per m², a capture area 100 times its ground) does not thicken:
/// one more kg would not pay its upkeep at the bottom of its layer, so all it builds leaves as seeds, and over a
/// summer its cell holds less than it started with; the books are kept.
#[test]
fn a_thick_cover_does_not_thicken() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    assert!(cover::place(&mut w, i, genes(), 0, 20.0));
    let kg0 = cover::per_m2(&w)[i];
    for _ in 0..6 {
        pcore::step(&mut w);
    }
    let kg = cover::per_m2(&w)[i];
    println!("cover on its cell {kg0:.2} kg/m2, six months later {kg:.2} kg/m2");
    assert!(kg < kg0, "the thick cover thickened: {kg0} then {kg}");
    books.assert_kept(&w, 1e-9);
}

/// The light reaction goes with its gas in the air (`Physics::gas_factor`): nothing at the compensation point, all
/// it stores at `light_yield` at the reference share, and less in between.
#[test]
fn the_light_reaction_goes_with_its_gas() {
    let p = pcore::Physics::default();
    assert_eq!(p.gas_factor(p.gas_floor), 0.0);
    assert!((p.gas_factor(p.gas_ref) - 1.0).abs() < 1e-12);
    let half = p.gas_factor(0.5 * p.gas_ref);
    assert!(half > 0.0 && half < 1.0 && p.gas_factor(2.0 * p.gas_ref) > 1.0, "{half}");
}

/// Cover whose frost gene lies above its ground's temperature dies of frost in its month (`PHASE`), and what dies goes
/// to the ground: the books are kept.
#[test]
fn the_cover_dies_in_frost() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    let mut g = genes();
    g[FROST] = 300.0;
    assert!(w.t_ground_last(i) < 299.0, "the ground is warm: {} K", w.t_ground_last(i));
    assert!(cover::place(&mut w, i, g, 0, 1.0));
    let kg0 = cover_kg(&w);
    w.take_counters();
    pcore::step(&mut w);
    let lost = w.take_counters().cover_lost;
    println!("cover {kg0:.3e} kg, a month later {:.3e} kg; lost to frost {:.3e} kg", cover_kg(&w), lost[PHASE as usize]);
    assert!(lost[PHASE as usize] > 0.1 * kg0 && cover_kg(&w) < kg0, "no frost: lost {lost:?}");
    books.assert_kept(&w, 1e-9);
}

/// An animal whose brain only takes from the place, in a cell of thick cover, takes from the cover (grazing): its
/// event says so, the cover loses what was eaten (`TAKEN`), and the books are kept.
#[test]
fn grazing_takes_from_the_cover() {
    let (mut w, i) = start();
    assert!(cover::place(&mut w, i, genes(), 0, 1.0));
    let b = eater(&mut w, i, pcore::brain::TAKE, 1.0);
    w.add(b);
    let books = Books::now(&w);
    w.take_counters();
    w.events.clear();
    for _ in 0..60 {
        pcore::day(&mut w);
    }
    let grazed: f64 = w.events.iter().filter_map(|e| matches!(e.kind, EventKind::Took { from: pcore::choose::COVER, .. }).then_some(e.value)).sum();
    let lost = w.take_counters().cover_lost[TAKEN as usize];
    println!("grazed {grazed:.3} kg; the cover lost {lost:.3} kg to it");
    assert!(grazed > 0.0 && (lost - grazed).abs() <= 1e-9 * grazed, "grazed {grazed}, lost {lost}");
    books.assert_kept(&w, 1e-9);
}

/// The slots of cover in cell `i`: each one's genes and built cover, kg.
fn slots_of(w: &pcore::World, i: usize) -> Vec<([f32; cover::NG], f64)> {
    let Some(p) = cover::Parts::of(&w.chem) else { return Vec::new() };
    let (d, mm) = (w.life.domain(i), p.mm(&w.chem));
    let (c, k) = (&d.cover, d.slot(i));
    cover::Cover::slots(k).filter(|&x| c.root[x] != cover::NO_ROOT).map(|x| (c.genes[x], c.built(&mm, x))).collect()
}

/// Seeds land by their kind and genes (`seed::settle`): cover of the starting kind placed where that kind grows joins
/// its slot when within `APART` x `alike` steps (one slot, the genes the mean by built cover), and a variant farther
/// away founds a slot of its own and keeps it through a month (slots merge only within the same distance). The books
/// are kept.
#[test]
fn seeds_join_their_kind_or_found_a_slot() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    let (g, step) = (genes(), cover::RANGE[FROST].0);
    assert!(slots_of(&w, i).is_empty(), "the cell starts bare");
    assert!(cover::place(&mut w, i, g, 0, 0.1));
    let mut near = g;
    near[FROST] -= step;
    assert!(cover::place(&mut w, i, near, 0, 0.1));
    let one = slots_of(&w, i);
    assert_eq!(one.len(), 1, "a seed one step away joins: {one:?}");
    assert!((one[0].0[FROST] - (g[FROST] - 0.5 * step)).abs() < 1e-3 * step, "the joined frost gene {} is not the mean", one[0].0[FROST]);
    let mut far = g;
    far[FROST] -= 4.0 * step;
    assert!(cover::place(&mut w, i, far, 0, 0.1));
    assert_eq!(slots_of(&w, i).len(), 2, "a seed farther than {} steps founds its own slot", cover::APART * w.cfg.phys.alike);
    pcore::step(&mut w);
    let two = slots_of(&w, i);
    assert_eq!(two.len(), 2, "the variant was merged within its month: {two:?}");
    books.assert_kept(&w, 1e-9);
}

/// The frost gene of the cover moves down where winters kill (laws.md 4.5): two variants of the starting cover, of
/// frost genes `FROST_GENES` (3 steps, 15 K, apart), sown on every land cell of a world at the equator and of one at
/// mild latitudes, compete for 60 years. Where the ground's winter falls below the higher gene while its liquid is
/// still liquid, frost kills the active cover of that gene (4.5, 6.5), and the lower one (dearer to keep: `c_firm`)
/// wins; where it does not, the higher one wins. The cover's mean frost gene (by built cover) ends more than a step
/// lower in the world with winters. (Below the liquid's melting point the cover is dormant, its ground's liquid frozen,
/// and frost does not kill it: a frost gene under 273 K is never selected by frost.)
#[test]
fn the_frost_gene_falls_where_winters_are_cold() {
    let mean_frost = |lat: [f64; 2]| {
        let (chem, init) = worldgen::chemistry0();
        let cfg = pcore::Config { lat_south: lat[0], lat_north: lat[1], ..Default::default() };
        let mut w = worldgen::terrain::build(1, worldgen::terrain::island(16, 16), chem, &init, cfg);
        pcore::step(&mut w);
        worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
        let (mut g, mut low) = (genes(), genes());
        (g[FROST], low[FROST]) = (FROST_GENES[0], FROST_GENES[1]);
        let land: Vec<usize> = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).collect();
        for i in land {
            cover::place(&mut w, i, g, 0, 0.05);
            cover::place(&mut w, i, low, 0, 0.05);
        }
        let coldest = |w: &pcore::World| (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).map(|i| w.t_ground_last(i)).fold(f64::MAX, f64::min);
        let mut winter = f64::MAX;
        for _ in 0..60 * 12 {
            pcore::step(&mut w);
            winter = winter.min(coldest(&w));
        }
        let s = cover::sums(&w);
        let (frost, built) = s.iter().fold((0.0, 0.0), |a, x| (a.0 + x.genes[FROST], a.1 + x.built));
        (frost / built, winter, g[FROST] as f64, low[FROST] as f64)
    };
    let ((warm, warm_winter, high, low), (cold, cold_winter, ..)) = (mean_frost(WARM_LAT), mean_frost(MILD_LAT));
    let step = cover::RANGE[FROST].0 as f64;
    println!("mean frost gene {warm:.1} K at the equator (coldest land {warm_winter:.1} K), {cold:.1} K at mild latitudes ({cold_winter:.1} K); sown {high} and {low}");
    assert!(cold_winter < high - 3.0, "no winter below the higher gene: coldest land {cold_winter} K");
    assert!(warm - cold > step, "no selection: mean frost gene {warm} K (equator) and {cold} K (mild latitudes)");
}
