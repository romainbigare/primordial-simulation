//! The cover (laws.md 4.5): a field of light-eaters per cell grows in light and dies in frost, keeping the books, and
//! animals graze it by their take from the place.

use pcore::body::{PHASE, TAKEN};
use pcore::cover::{self, FROST};
use pcore::event::EventKind;
use worldgen::life::{Place, KINDS};
use worldgen::testkit::{dry_lands, eater, small_world, Books};

/// Latitudes of the south and north edges of the frost test's island, degrees: it reaches from the tropics to the cold,
/// so its air brings every band the heat of the lower latitudes, as a real place gets.
const FROST_WORLD_LAT: [f64; 2] = [10.0, 72.3];
/// The warm band and the cold band of that island, between these latitudes, degrees.
const WARM_BAND: [f64; 2] = [20.0, 30.0];
const COLD_BAND: [f64; 2] = [50.0, 60.0];
/// The fewest land cells a band holds.
const BAND_CELLS: usize = 5;
/// The two frost genes of the selection test, K, three steps (15 K) apart: the tender one above the key liquid's melting
/// point (273 K), so its cover is still active (its ground's liquid liquid) when a winter's ground falls below it and
/// frost can kill it before dormancy hardens it, and below the warm band's coldest ground, so the warm band's winters
/// spare it (the test asserts both).
const FROST_GENES: [f32; 2] = [276.0, 261.0];

/// The genes of the starting cover (kind 0).
fn genes() -> [f32; cover::NG] {
    let Place::Field(f) = &KINDS[0].place else { unreachable!("kind 0 is cover") };
    f.genes
}

/// The world's cover, kg.
fn cover_kg(w: &pcore::World) -> f64 {
    cover::sums(w).iter().map(|s| s.kg).sum()
}

/// A small world with its soil, after a month (Flow's state built), and the dry land cell that rain reaches most.
fn start() -> (pcore::World, usize) {
    let mut w = small_world();
    pcore::step(&mut w);
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let i = dry_lands(&w).into_iter().filter(|&i| w.rain[i] > 0.0).max_by(|&a, &b| w.rain[a].total_cmp(&w.rain[b])).expect("dry land with rain");
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
    let kg0 = cover::per_m2(&w, cover::Cover::kg)[i];
    for _ in 0..6 {
        pcore::step(&mut w);
    }
    let kg = cover::per_m2(&w, cover::Cover::kg)[i];
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
        pcore::day_with_month_at_start(&mut w);
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
    let (c, mm) = (w.life.cover(i), p.mm(&w.chem));
    c.held().map(|x| (c.genes[x], c.built(&mm, x))).collect()
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

/// A kind new to a cell takes hold where four variants of another kind hold every slot (`seed::lodge`): the two most
/// alike of them join to make room, and the cover keeps its mass.
#[test]
fn a_new_kind_enters_a_full_cell() {
    let (mut w, i) = start();
    let (g, step) = (genes(), cover::RANGE[FROST].0);
    for k in 0..cover::SLOTS {
        let mut v = g;
        v[FROST] -= 3.0 * step * k as f32;
        assert!(cover::place(&mut w, i, v, 0, 0.1));
    }
    let before: f64 = slots_of(&w, i).iter().map(|s| s.1).sum();
    assert_eq!(slots_of(&w, i).len(), cover::SLOTS, "four variants hold every slot");
    assert!(cover::place(&mut w, i, g, 1, 0.01), "seeds of a second kind find no room");
    let c = w.life.cover(i);
    assert_eq!(c.held().filter(|&x| c.root[x] == 1).count(), 1, "the new kind holds a slot");
    let after: f64 = slots_of(&w, i).iter().map(|s| s.1).sum();
    assert!(after > before, "the joined variants lost cover: {before} kg, then {after} kg");
}

/// The frost gene of the cover moves down where winters kill (laws.md 4.5): two variants of the starting cover, of
/// frost genes `FROST_GENES` (3 steps, 15 K, apart), sown on every land cell of one island from the tropics to the
/// cold (`FROST_WORLD_LAT`), compete for 60 years. Where the ground's winter falls below a gene's lethal limit
/// (lowered by dormancy, 6.5), frost kills the cover of that gene (4.5), and the lower one (dearer to keep: `c_firm`)
/// wins; where it does not, the higher one wins. The cover's mean frost gene (by built cover) ends more than a step
/// lower in the cold band (`COLD_BAND`) than in the warm band (`WARM_BAND`). The scene's premises are asserted, so a
/// later ground that breaks them says so: the frost law (dormancy's hardening included) kills some of the tender kind in
/// the cold band's winters and none in the warm band's.
#[test]
#[ignore = "slow: a 60-year run of a 16 x 16 world"]
fn the_frost_gene_falls_where_winters_are_cold() {
    let (chem, init) = worldgen::chemistry0();
    let cfg = pcore::Config { lat_south: FROST_WORLD_LAT[0], lat_north: FROST_WORLD_LAT[1], ..Default::default() };
    let mut w = worldgen::terrain::build(1, worldgen::terrain::island(16, 16), chem, &init, cfg);
    pcore::step(&mut w);
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let (mut g, mut low) = (genes(), genes());
    (g[FROST], low[FROST]) = (FROST_GENES[0], FROST_GENES[1]);
    let land: Vec<usize> = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).collect();
    for &i in &land {
        cover::place(&mut w, i, g, 0, 0.05);
        cover::place(&mut w, i, low, 0, 0.05);
    }
    let band = |b: [f64; 2]| -> Vec<usize> { land.iter().copied().filter(|&i| (b[0]..b[1]).contains(&w.mesh.cells().lat()[i])).collect() };
    let (warm, cold) = (band(WARM_BAND), band(COLD_BAND));
    assert!(warm.len() >= BAND_CELLS && cold.len() >= BAND_CELLS, "land cells in the bands: {} warm, {} cold", warm.len(), cold.len());
    let coldest = |w: &pcore::World, cells: &[usize]| cells.iter().map(|&i| w.t_ground_last(i)).fold(f64::MAX, f64::min);
    // The most of the tender kind a month's frost would kill in a band's cells, by the frost law at each cell's ground
    // and activity (`Physics::frozen`, dormancy's hardening included).
    let parts = cover::Parts::of(&w.chem).expect("cover parts");
    let tender = g;
    let kills = |w: &pcore::World, cells: &[usize]| {
        let at = |i: usize| {
            let act = cover::activity(&w.cfg.phys, &tender, cover::wet(w, &parts, i, w.g(parts.liquid, i)).1);
            w.cfg.phys.frozen(tender[FROST] as f64, w.t_ground_last(i), act)
        };
        cells.iter().map(|&i| at(i)).fold(0.0, f64::max)
    };
    let (mut warm_winter, mut cold_winter, mut warm_kill, mut cold_kill) = (f64::MAX, f64::MAX, 0.0f64, 0.0f64);
    for _ in 0..60 * 12 {
        pcore::step(&mut w);
        warm_winter = warm_winter.min(coldest(&w, &warm));
        cold_winter = cold_winter.min(coldest(&w, &cold));
        (warm_kill, cold_kill) = (warm_kill.max(kills(&w, &warm)), cold_kill.max(kills(&w, &cold)));
    }
    // The mean frost gene of a band's cells, by built cover.
    let mm = parts.mm(&w.chem);
    let mean_frost = |cells: &[usize]| {
        let (frost, built) = cells.iter().flat_map(|&i| {
            let c = &w.life.cell_at(i).cover;
            c.held().map(move |x| (c.built(&mm, x), c.genes[x][FROST] as f64)).collect::<Vec<_>>()
        }).fold((0.0, 0.0), |a, (b, f)| (a.0 + b * f, a.1 + b));
        frost / built
    };
    let (warm_mean, cold_mean, step) = (mean_frost(&warm), mean_frost(&cold), cover::RANGE[FROST].0 as f64);
    println!("mean frost gene {warm_mean:.1} K in the warm band (coldest ground {warm_winter:.1} K), {cold_mean:.1} K in the cold band ({cold_winter:.1} K); sown {} and {}", FROST_GENES[0], FROST_GENES[1]);
    // The scene's premises: the cold band's winters kill the tender kind, the warm band's spare it.
    println!("the most of the tender kind a month's frost kills: {warm_kill:.3} in the warm band, {cold_kill:.3} in the cold band");
    assert!(cold_kill > 0.0, "the cold band's winters do not kill the tender kind: coldest ground {cold_winter} K");
    assert!(warm_kill == 0.0, "the warm band's winters kill the tender kind ({warm_kill} a month): coldest ground {warm_winter} K");
    assert!(warm_mean - cold_mean > step, "no selection: mean frost gene {warm_mean} K (warm band) and {cold_mean} K (cold band)");
}
