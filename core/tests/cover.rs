//! The cover (laws.md 3): a field of light-eaters per cell grows in light and dies in frost, keeping the books; what
//! takes from it or presses on it through the contract takes what it loses; its seeds land by the dispersal kernel.

use pcore::contract::{self, Layer};
use pcore::cover::{self, FROST};
use pcore::life::{BROKEN, PHASE, TAKEN};
use pcore::rng::{Purpose, Rng};
use pcore::space::Leaf;
use pcore::store::Class;
use pcore::World;
use std::collections::BTreeMap;
use worldgen::life::KINDS;
use worldgen::testkit::{self, dry_lands, small_world, Books, Taker};

/// Latitudes of the south and north edges of the frost test's island, degrees: it reaches from the warm latitudes to
/// the cold, so its air brings every latitude the heat of the lower ones, as a real place gets.
const FROST_WORLD_LAT: [f64; 2] = [20.0, 82.3];
/// The fewest land cells of the frost test's island its winters kill the tender variant in, and spare it in.
const FROST_CELLS: usize = 5;
/// A month's frost kills the tender variant in a cell where it would kill more than this share of it (`cover::frozen`).
const KILLS: f64 = 0.5;
/// The last years of the frost test's run, over which a cell's winters and its cover are weighed.
const LAST_YEARS: usize = 10;
/// The two frost genes of the selection test, K, three steps (15 K) apart: the tender one above the key liquid's melting
/// point (273 K), so its cover is still active (its ground's liquid liquid) when a winter's ground falls below it and
/// frost can kill it before dormancy hardens it.
const FROST_GENES: [f32; 2] = [276.0, 261.0];

/// The row of the big slow kind of cover (a tree), its label.
const TREE_ROW: usize = 2;
/// The big slow kind of cover.
const TREE: &worldgen::life::Kind = &KINDS[TREE_ROW];
/// The row of the woody kind of dry ground.
const DRY_ROW: usize = 4;

/// The genes of the light-eaters.
fn genes() -> [f32; cover::NG] {
    KINDS[worldgen::life::LIGHT_ROW].genes
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

/// 0.1 kg per m² of the starting cover on one cell grows over a year of light: it builds from its light reaction and
/// the soil's minerals, and spreads seeds. What the cover's parts hold as fuel (`cover::content`) changes by exactly the
/// flows its budget books (`cover::BUDGET`): what its light reaction stored, less each part of its upkeep, the work of
/// building, the layers it shed, what died and what its roots gave its ground; and every element and the energy of
/// the world are kept. Every cell has a loam's pores.
#[test]
fn the_cover_grows_by_what_its_budget_books() {
    let (mut w, i) = start();
    worldgen::testkit::set_loam(&mut w);
    let books = Books::now(&w);
    assert!(cover::place(&mut w, i, genes(), 0, 0.1));
    w.take_counters();
    let (kg0, e0) = (cover_kg(&w), cover::content(&w));
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let (kg, e) = (cover_kg(&w), cover::content(&w));
    let flows = w.take_counters().cover_budget;
    for (name, f) in cover::BUDGET.iter().zip(&flows) {
        println!("{name:12} {f:+.4e} J");
    }
    let booked: f64 = flows.iter().sum();
    println!("cover {kg0:.3e} kg, a year later {kg:.3e} kg; content {e0:.6e} J, a year later {e:.6e} J, booked {booked:+.6e} J");
    assert!(kg > kg0, "the cover did not grow: {kg0} then {kg}");
    let scale = flows.iter().map(|f| f.abs()).sum::<f64>().max(e0);
    assert!((e - e0 - booked).abs() <= 1e-9 * scale, "the cover's content moved by {} J, its budget booked {booked} J", e - e0);
    assert_eq!(pcore::check::broken(&w), None);
    books.assert_kept(&w, 1e-9);
}

/// A cover far thicker than its light can keep (20 kg per m², a capture area 100 times its ground) does not thicken:
/// one more kg would not pay its upkeep at the bottom of its layer, so all it builds leaves as seeds, and it sheds the
/// layers that do not pay (`season::shed`), so over a summer its cell holds less than it started with and its capture area
/// falls to under half; the books are kept.
#[test]
fn a_thick_cover_does_not_thicken() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    assert!(cover::place(&mut w, i, genes(), 0, 20.0));
    let (kg0, leaf0) = (cover::per_m2(&w, cover::Cover::kg)[i], cover::per_m2(&w, cover::Cover::leaf)[i]);
    for _ in 0..6 {
        pcore::step(&mut w);
    }
    let (kg, leaf) = (cover::per_m2(&w, cover::Cover::kg)[i], cover::per_m2(&w, cover::Cover::leaf)[i]);
    println!("cover on its cell {kg0:.2} kg/m2 (capture area {leaf0:.1}), six months later {kg:.2} kg/m2 ({leaf:.1})");
    assert!(kg < kg0, "the thick cover thickened: {kg0} then {kg}");
    assert!(leaf < 0.5 * leaf0, "the thick cover kept its layers that do not pay: {leaf0} then {leaf}");
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

/// A plant feels drought against the pores of its own ground (`Pores::available`, `cover::activity`): it is fully active
/// while its ground holds the share of what it holds for roots its moisture gene sets, none at the wilting point
/// (`wilt`), between the two it slows; a kind of dry ground (`MOIST` -1) is still fully active where the light-eaters
/// have slowed.
#[test]
fn a_plant_slows_from_its_share_to_the_wilting_point() {
    let soil = worldgen::testkit::loam();
    let (herb, dry) = (genes(), KINDS[DRY_ROW].genes);
    let act = |g: &[f32; cover::NG], filled: f64| cover::activity(g, soil.available(filled));
    let filled = |share: f64| soil.wilt + share * (soil.field - soil.wilt);
    let slows = cover::dec(herb[cover::MOIST]);
    let (wilting, full, half) = (act(&herb, soil.wilt), act(&herb, filled(slows * (1.0 + 1e-9))), act(&herb, filled(0.5 * slows)));
    println!("activity of the light-eaters at the wilting point {wilting}, at the share {slows:.2} {full}, at half that {half:.3}");
    assert!(wilting == 0.0 && full == 1.0 && half > 0.0 && half < 1.0, "{wilting} {full} {half}");
    assert_eq!(act(&dry, filled(0.5 * slows)), 1.0, "the kind of dry ground slowed where the light-eaters slow");
}

/// A leaf loses the key liquid for the gas it takes in (`cover::thirst`): nothing in air saturated at its temperature,
/// more the drier the air and the warmer the leaf, and at a mild leaf in half-saturated air some hundred mol a mol, the
/// transpiration ratio of real leaves (a few hundred g of liquid a g of dry matter; Farquhar and Sharkey 1982).
#[test]
fn leaves_lose_liquid_for_gas() {
    let (chem, _) = worldgen::chemistry0();
    let (p, ph) = (cover::Parts::of(&chem).expect("cover parts"), pcore::Physics::default());
    let gas = chem.reactions[p.light].inputs.iter().map(|q| q.0 as usize).find(|&k| chem.phase(k, 290.0) == pcore::chem::Phase::Gas).expect("a gas input");
    let at = |t: f64, rh: f64| {
        let p_air = pcore::units::P_REF;
        cover::thirst(&chem, &ph, (p.liquid, gas), t, [rh * chem.p_sat(p.liquid, t), ph.gas_ref * p_air])
    };
    let (wet, half, warm) = (at(290.0, 1.0), at(290.0, 0.5), at(300.0, 0.5));
    println!("mol of liquid a mol of gas: saturated {wet:.1}, half-saturated {half:.1}, warmer {warm:.1}");
    assert!(wet == 0.0 && half > 50.0 && half < 500.0 && warm > half, "{wet} {half} {warm}");
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

/// What `layer` of the cover of cell `i` shows (`contract::shown`).
fn shown(w: &pcore::World, i: usize, layer: Layer) -> contract::Shown {
    contract::shown(w, i, w.life.cell_at(i), layer).expect("the cover's parts")
}

/// A taker on the contract (`testkit::Taker`) takes each month a share of the lowest slot of a cell's cover
/// (`contract::slots`) and passes it to the ground: what it takes is what that slot loses, to rounding, and is counted
/// as taken; every element and the energy are kept; and the cell grazed harder holds less cover after a year.
#[test]
fn what_a_taker_takes_its_slot_loses() {
    let (mut w, i) = start();
    assert!(cover::place(&mut w, i, genes(), 0, 1.0));
    let mut left = vec![];
    for share in [0.05, 0.25] {
        let mut w = w.clone();
        let (books, mut taker) = (Books::now(&w), Taker::new(i, Layer::Cover, share));
        w.take_counters();
        for _ in 0..12 {
            pcore::step(&mut w);
            // (In the open: no snow hides any of it, `snow_hides_what_lies_under_it`.)
            w.life.env[i][3] = 0.0;
            let Some(&(lowest, _)) = contract::slots(&w, i, w.life.cell_at(i)).first() else { break };
            taker.layer = lowest;
            let before = shown(&w, i, taker.layer);
            let took = taker.month(&mut w);
            let after = shown(&w, i, taker.layer);
            for (k, &t) in took.iter().enumerate() {
                let lost = before.mol[k] - after.mol[k];
                assert!((lost - t).abs() <= 1e-12 * before.mol[k], "substance {k}: took {t:e} mol, the slot lost {lost:e}");
            }
        }
        books.assert_kept(&w, 1e-9);
        let kg: f64 = taker.took.iter().zip(&w.chem.subs).map(|(m, s)| m * s.molar_mass).sum();
        let counted = w.take_counters().cover_lost[TAKEN as usize];
        assert!((counted - kg).abs() <= 1e-9 * kg, "took {kg} kg, counted {counted} kg as taken");
        left.push(shown(&w, i, Layer::Cover).kg);
        println!("a share {share} taken a month: took {:.3e} mol, {:.3e} J; the cell keeps {:.3e} kg", taker.took.iter().sum::<f64>(), taker.heat, left[left.len() - 1]);
        assert!(taker.took.iter().sum::<f64>() > 0.0, "the taker took nothing");
    }
    assert!(left[1] < left[0], "the cell grazed harder holds more: {left:?}");
}

/// Work pressed on a cell of cover breaks it (`contract::press`): what it loses of each substance its ground gains, to
/// rounding; what it lost is counted as broken; the books are kept; and work enough to break all of it breaks all of it.
#[test]
fn what_is_pressed_loose_the_ground_gains() {
    let (mut w, i) = start();
    assert!(cover::place(&mut w, i, genes(), 0, 1.0));
    let (n, books) = (w.mesh.cells().n(), Books::now(&w));
    let ground = |w: &World, k: usize| w.store.amounts(Class::Ground)[k * n + i];
    // (In the open: no snow hides any of it.)
    w.life.env[i][3] = 0.0;
    let all = shown(&w, i, Layer::Cover);
    let breaking = all.breaking;
    w.take_counters();
    let before: Vec<f64> = (0..w.chem.ns()).map(|k| ground(&w, k)).collect();
    w.in_cell(i, |w, d| contract::press(w, d, i, 0.25 * breaking));
    let after = shown(&w, i, Layer::Cover);
    for (k, &g) in before.iter().enumerate() {
        let (lost, gained) = (all.mol[k] - after.mol[k], ground(&w, k) - g);
        assert!((lost - gained).abs() <= 1e-9 * all.mol[k].max(g), "substance {k}: the cover lost {lost:e} mol, its ground gained {gained:e}");
    }
    let broken = w.take_counters().cover_lost[BROKEN as usize];
    assert!((after.kg - 0.75 * all.kg).abs() <= 1e-9 * all.kg && (broken - 0.25 * all.kg).abs() <= 1e-9 * all.kg, "a quarter of the work broke {broken} of {} kg", all.kg);
    w.in_cell(i, |w, d| contract::press(w, d, i, 2.0 * breaking));
    assert!(shown(&w, i, Layer::Cover).kg <= 1e-9 * all.kg, "work enough to break all of it left some");
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

/// A kind new to a cell takes hold where variants of another kind hold every slot its world uses (`Config::variety`,
/// here 3) and all pay their way (`seed::lodge`): the two nearest each other by their genes join to make room, the cell
/// never holds more slots than its world's setting, and the cover keeps its mass.
#[test]
fn a_new_kind_enters_a_full_cell() {
    let (mut w, i) = start();
    w.cfg.variety = 3;
    let (g, step) = (genes(), cover::RANGE[FROST].0);
    for k in 0..w.cfg.variety {
        let mut v = g;
        v[FROST] -= 3.0 * step * k as f32;
        assert!(cover::place(&mut w, i, v, 0, 0.1));
    }
    let before: f64 = slots_of(&w, i).iter().map(|s| s.1).sum();
    assert_eq!(slots_of(&w, i).len(), w.cfg.variety, "the variants hold every slot");
    assert!(cover::place(&mut w, i, g, 1, 0.01), "seeds of a second kind find no room");
    assert_eq!(slots_of(&w, i).len(), w.cfg.variety, "the cell holds more slots than its world's setting");
    let c = w.life.cover(i);
    assert_eq!(c.held().filter(|&x| c.root[x] == 1).count(), 1, "the new kind holds a slot");
    let after: f64 = slots_of(&w, i).iter().map(|s| s.1).sum();
    assert!(after > before, "the joined variants lost cover: {before} kg, then {after} kg");
}

/// A stand of the tree kind sown on bare ground as seedlings (0.001 kg per m²) grows over two years of light: it never
/// holds more plants than the self-thinning law allows for its mass (`thin_kg`), its large plants grow taller
/// (`Cover::height`), and the books are kept.
#[test]
fn a_stand_never_passes_the_thinning_law_as_it_grows_tall() {
    let (mut w, i) = start();
    let books = Books::now(&w);
    assert!(cover::place(&mut w, i, TREE.genes, TREE_ROW as u32, 0.001));
    let p = cover::Parts::of(&w.chem).expect("cover parts");
    let (mm, area, rho) = (p.mm(&w.chem), w.mesh.cells().area()[i], w.chem.subs[p.sub[cover::WALL]].density[0]);
    let stand = |w: &pcore::World| {
        let c = w.life.cover(i);
        let x = c.held().next().expect("the stand lives");
        (c.built(&mm, x) / area, c.count(x) / area, c.height(&w.cfg.phys, &mm, rho, x))
    };
    let first = stand(&w);
    for _ in 0..24 {
        pcore::step(&mut w);
        let (kg, plants, _) = stand(&w);
        assert!(plants <= (w.cfg.phys.thin_kg / kg).powi(2) * (1.0 + 1e-9), "{plants} plants a m² on {kg} kg/m²");
    }
    let last = stand(&w);
    println!("kg/m², plants a m², height m: {first:?} then {last:?}");
    assert!(last.2 > 2.0 * first.2, "the plants did not grow tall: {first:?} then {last:?}");
    books.assert_kept(&w, 1e-9);
}

/// A kind's plants hold the share of their cell whose loose ground reaches their roots' need (`Cover::rooted`): on the
/// land cell of the thinnest loose ground, tall plants (the tree kind placed as a stand of 10 kg per m²) hold less of
/// it than low ones (the starting cover at 0.1 kg per m²), and less than all of it.
#[test]
fn tall_plants_hold_less_of_thin_ground() {
    let (mut w, _) = start();
    let mean = |b: &[f32; cover::BINS]| b.iter().enumerate().map(|(k, &s)| s as f64 * k as f64).sum::<f64>();
    let i = dry_lands(&w).into_iter().min_by(|&a, &b| mean(&w.loose_bins[a]).total_cmp(&mean(&w.loose_bins[b]))).expect("dry land");
    assert!(cover::place(&mut w, i, genes(), 0, 0.1) && cover::place(&mut w, i, TREE.genes, TREE_ROW as u32, 10.0));
    pcore::step(&mut w);
    let c = w.life.cover(i);
    let held = |root: u32| c.held().find(|&x| c.root[x] == root).map(|x| c.rooted[x]).expect("the kind lives");
    let (low, tall) = (held(0), held(TREE_ROW as u32));
    println!("rooted share of the low kind {low:.3}, of the tall kind {tall:.3}");
    assert!(tall < low && tall < 1.0, "the tall kind holds {tall} of the cell, the low kind {low}");
}

/// Seeds of a new kind arriving in a cell whose slots are all held take the slot in deficit (`Cover::net` below zero,
/// the worst), whose cover dies (`seed::lodge` returns it), and no other: the slots that pay their way keep their cover.
#[test]
fn seeds_take_the_slot_in_deficit() {
    let (chem, _) = worldgen::chemistry0();
    let (p, cfg) = (cover::Parts::of(&chem).expect("chemistry 0 has cover"), pcore::Config::default());
    let ph = &cfg.phys;
    let (mm, g, step) = (p.mm(&chem), genes(), cover::RANGE[FROST].0);
    let seeds = |v, root, kg| cover::sown(&chem, ph, &p, (0, 1.0, 280.0), v, root, kg);
    let mut c = cover::Cover::default();
    for k in 0..cfg.variety {
        let mut v = g;
        v[FROST] -= 3.0 * step * k as f32;
        assert!(matches!(cover::lodge(&mut c, &mm, &seeds(v, k as u32, 0.5), &cfg, (1.0, 1.0), &mut [0.0; cover::SLOTS]), cover::Landed::In));
    }
    (c.net[1], c.net[2]) = (-1.0, -3.0);
    let (held, worst) = (c.clone(), c.mol[2]);
    let cover::Landed::Out(died) = cover::lodge(&mut c, &mm, &seeds(g, 9, 0.01), &cfg, (1.0, 1.0), &mut [0.0; cover::SLOTS]) else { panic!("the seeds took no slot in deficit") };
    assert_eq!(died, worst, "the worst in deficit gave way");
    assert_eq!(c.root[2], 9, "the new kind holds the slot");
    for x in [0, 1, 3] {
        assert_eq!((c.root[x], c.mol[x]), (held.root[x], held.mol[x]), "slot {x} lost its cover");
    }
}

/// Frost selects the hardy where it kills the tender (laws.md 3): two variants of the starting cover, of frost genes
/// `FROST_GENES`, sown on every land cell of an island from the warm latitudes to the cold (`FROST_WORLD_LAT`), compete
/// for 60 years. In each cell the frost law says each month what share of the tender variant its ground's cold would
/// kill (`cover::frozen` at the tender gene and the cell's activity, dormancy's hardening included), whatever lives
/// there. Where it kills most of it in some month of the last `LAST_YEARS` years, the hardy variant (the lower gene,
/// dearer to keep: `c_firm`) holds most of the built cover over those years; where it never kills any of it, the
/// tender variant does. The scene's premise is asserted: the island holds cells of both.
#[test]
#[ignore = "slow: a 60-year run of a 16 x 16 world"]
fn frost_selects_the_hardy_where_it_kills_the_tender() {
    let cfg = pcore::Config { lat_south: FROST_WORLD_LAT[0], lat_north: FROST_WORLD_LAT[1], ..Default::default() };
    let (mut w, ()) = testkit::island_in_with(1, cfg, |w| (0..w.mesh.cells().n()).any(|i| w.is_land(i)).then_some(()));
    pcore::step(&mut w);
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let (mut tender, mut hardy) = (genes(), genes());
    (tender[FROST], hardy[FROST]) = (FROST_GENES[0], FROST_GENES[1]);
    let land: Vec<usize> = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).collect();
    for &i in &land {
        cover::place(&mut w, i, tender, 0, 0.05);
        cover::place(&mut w, i, hardy, 0, 0.05);
    }
    let parts = cover::Parts::of(&w.chem).expect("cover parts");
    let mm = parts.mm(&w.chem);
    // The share of the tender variant a month's frost kills in cell `i`, by the frost law alone.
    let kills = |w: &pcore::World, i: usize| {
        let act = cover::activity(&tender, cover::soak(w, &parts, i, w.g(parts.liquid, i)).1.moist);
        cover::frozen(&w.cfg.phys, w.chem.subs[parts.liquid].melt, tender[FROST] as f64, (w.t_ground_last(i), act))
    };
    // The share of the built cover of cell `i` whose frost gene lies nearer the hardy variant's (NaN where none stands).
    let middle = (FROST_GENES[0] + FROST_GENES[1]) / 2.0;
    let hardy_share = |w: &pcore::World, i: usize| {
        let c = w.life.cover(i);
        let all: f64 = c.held().map(|x| c.built(&mm, x)).sum();
        c.held().filter(|&x| c.genes[x][FROST] < middle).map(|x| c.built(&mm, x)).sum::<f64>() / all
    };
    let n = w.mesh.cells().n();
    let (months, last) = (60 * 12, LAST_YEARS * 12);
    let (mut killed_late, mut killed_ever, mut share) = (vec![0.0f64; n], vec![0.0f64; n], vec![0.0f64; n]);
    for m in 0..months {
        pcore::step(&mut w);
        for &i in &land {
            let k = kills(&w, i);
            killed_ever[i] = killed_ever[i].max(k);
            if m >= months - last {
                killed_late[i] = killed_late[i].max(k);
                share[i] += hardy_share(&w, i) / last as f64;
            }
        }
    }
    let mean = |cells: &[usize]| cells.iter().map(|&i| share[i]).sum::<f64>() / cells.len() as f64;
    let killing: Vec<usize> = land.iter().copied().filter(|&i| killed_late[i] > KILLS && share[i].is_finite()).collect();
    let sparing: Vec<usize> = land.iter().copied().filter(|&i| killed_ever[i] == 0.0 && share[i].is_finite()).collect();
    let (where_killed, where_spared) = (mean(&killing), mean(&sparing));
    println!("the hardy variant holds {where_killed:.2} of the cover in the {} cells whose winters kill the tender, {where_spared:.2} in the {} that spare it", killing.len(), sparing.len());
    assert!(killing.len() >= FROST_CELLS && sparing.len() >= FROST_CELLS, "cells whose winters kill the tender: {}, that spare it: {}", killing.len(), sparing.len());
    assert!(where_killed > 0.5, "where frost kills the tender, the hardy holds {where_killed} of the cover");
    assert!(where_spared < 0.5, "where frost spares the tender, the hardy holds {where_spared} of the cover");
}

/// Lots drawn from a cell by the tests of the dispersal kernel.
const DRAWS: usize = 400_000;
/// Points along a side of the cells in the numerical integral of the kernel.
const POINTS: usize = 2000;
/// Standard errors within which what lands must match what the kernel says: four, as the test makes about thirty
/// comparisons (three would fail one run in ten by chance).
const SIGMAS: f64 = 4.0;
/// Fewest lots expected in a cell for its share to be tested (so the normal law holds for its count).
const ENOUGH: f64 = 30.0;

/// The lots of `DRAWS` that land in each cell, by its leaf, drawn from the cell `from` (stream `stream`).
fn landed(w: &World, from: Leaf, draws: usize, stream: u64) -> BTreeMap<Leaf, f64> {
    let i = w.map.cell(from).expect("a cell");
    let mut r = Rng::new(w.seed, stream, i as u64, Purpose::Cover);
    let mut by_leaf = BTreeMap::new();
    for _ in 0..draws {
        if let Some(j) = pcore::life::lot(w, i, &mut r) {
            *by_leaf.entry(w.map.leaves()[j]).or_insert(0.0) += 1.0 / draws as f64;
        }
    }
    by_leaf
}

/// The share of what starts evenly along a line of `side` m that a Gaussian of `sigma` m puts along a line of the same
/// length `o` m on, by the midpoint rule over `POINTS` points on each.
fn along(sigma: f64, side: f64, o: f64) -> f64 {
    let at = |k: usize| ((k as f64 + 0.5) / POINTS as f64 - 0.5) * side;
    let density = |d: f64| (-0.5 * d * d / (sigma * sigma)).exp() / (std::f64::consts::TAU.sqrt() * sigma);
    let sum: f64 = (0..POINTS).flat_map(|a| (0..POINTS).map(move |c| (a, c))).map(|(a, c)| density(o + at(c) - at(a))).sum();
    sum * side / (POINTS * POINTS) as f64
}

/// The share of what leaves cell `i` that the kernel puts in cell `j` of the same size: its two Gaussians, each a product
/// over east and south, integrated over `j` from points spread evenly over `i`.
fn kernel_share(w: &World, i: usize, j: usize) -> f64 {
    let (p, o) = (&w.cfg.phys, w.mesh.cells().offset(i, j));
    let south = w.map.length(w.map.leaves()[i].depth);
    let side = [w.mesh.cells().area()[i] / south, south];
    let by = |sigma: f64| along(sigma, side[0], o[0]) * along(sigma, side[1], o[1]);
    (1.0 - p.kernel_far_share) * by(p.kernel_near) + p.kernel_far_share * by(p.kernel_far)
}

/// What disperses lands by one kernel (`life::lot`), in expectation: over many lots, the share in each cell near the
/// source is the kernel's integral over it, within `SIGMAS` standard errors.
#[test]
fn lots_land_by_the_kernel() {
    let coarse = testkit::small_world();
    let source = coarse.map.leaves()[coarse.mesh.tree().cell_at(coarse.map.point_at(8.0, 8.0))];
    let whole = landed(&coarse, source, DRAWS, 1);
    let i = coarse.map.cell(source).expect("a cell");
    for (leaf, &share) in whole.iter().filter(|(_, s)| **s * DRAWS as f64 > ENOUGH) {
        let want = kernel_share(&coarse, i, coarse.map.cell(*leaf).expect("a cell"));
        let se = (want * (1.0 - want) / DRAWS as f64).sqrt();
        assert!((share - want).abs() <= SIGMAS * se, "{leaf:?}: {share} lands, the kernel says {want}, {SIGMAS} standard errors are {}", SIGMAS * se);
    }
}

/// Seeds keep the books: elements and energy over two years of a living world, which sends seeds every month.
#[test]
fn seeds_keep_the_books() {
    let mut w = testkit::living(8, 0);
    let books = Books::now(&w);
    (0..24).for_each(|_| pcore::step(&mut w));
    books.assert_kept(&w, 1e-9);
}

/// A taker on the contract takes from the seeds lying on a cell's ground (`Layer::Seeds`): what it takes is what they
/// lose, to rounding, and is counted as seeds taken; every element and the energy are kept.
#[test]
fn what_a_taker_takes_of_the_seeds_lying_they_lose() {
    let mut w = testkit::living(8, 12);
    let books = Books::now(&w);
    let lying = cover::lying(&w);
    let i = (0..lying.len()).max_by(|&a, &b| lying[a].total_cmp(&lying[b])).expect("cells");
    let before = shown(&w, i, Layer::Seeds);
    assert!(before.kg > 0.0, "no seeds lie on the ground after a year");
    w.take_counters();
    let mut taker = Taker::new(i, Layer::Seeds, 0.5);
    let took = taker.month(&mut w);
    let after = shown(&w, i, Layer::Seeds);
    for (k, &t) in took.iter().enumerate() {
        let lost = before.mol[k] - after.mol[k];
        assert!((lost - t).abs() <= 1e-12 * before.mol[k], "substance {k}: took {t:e} mol, the seeds lost {lost:e}");
    }
    let kg: f64 = took.iter().zip(&w.chem.subs).map(|(m, s)| m * s.molar_mass).sum();
    let counted = w.take_counters().seeds[cover::TAKEN_LYING];
    assert!(kg > 0.0 && (counted - kg).abs() <= 1e-9 * kg, "took {kg} kg, counted {counted} kg as taken");
    books.assert_kept(&w, 1e-9);
}

/// Snow hides what lies under it (`contract::above`): of a slot whose crown is h deep under snow to half its depth, a taker
/// sees and takes half; the seeds lying show nothing under any snow.
#[test]
fn snow_hides_what_lies_under_it() {
    let mut w = testkit::living(8, 12);
    let lying = cover::lying(&w);
    let i = (0..lying.len()).max_by(|&a, &b| lying[a].total_cmp(&lying[b])).expect("cells");
    let (layer, (base, top)) = *contract::slots(&w, i, w.life.cell_at(i)).iter().find(|s| s.1 .1 > s.1 .0).expect("a slot with a crown");
    let h = top - base;
    let open = shown(&w, i, layer);
    w.life.env[i][3] = base + 0.5 * h;
    let hidden = shown(&w, i, layer);
    assert!((hidden.kg - 0.5 * open.kg).abs() <= 1e-9 * open.kg, "under snow half its height a slot shows {} of {} kg", hidden.kg, open.kg);
    assert_eq!(shown(&w, i, Layer::Seeds).kg, 0.0, "seeds show under snow");
    let took: f64 = Taker::new(i, layer, 1.0).month(&mut w).iter().zip(&w.chem.subs).map(|(m, s)| m * s.molar_mass).sum();
    assert!(took <= 0.5 * open.kg * (1.0 + 1e-9) && took > 0.0, "a taker took {took} kg of the {} kg above the snow", hidden.kg);
}

/// A taker's vapour (`contract::vapour`): of the key liquid a cell shows (`contract::liquid`, its parts adding to what
/// its ground holds), what it sheds as vapour leaves the ground for the aside node, and the taker pays its heat of
/// vaporisation; with that heat taken back from the ground every element and the energy are kept.
#[test]
fn a_takers_vapour_leaves_the_ground_for_the_air() {
    let (mut w, i) = start();
    let (k, n, books) = (cover::Parts::of(&w.chem).expect("the cover's parts").liquid, w.mesh.cells().n(), Books::now(&w));
    let (ground, aside) = (|w: &World| w.store.amounts(Class::Ground)[k * n + i], |w: &World| w.store.amounts(Class::Aside)[k * n + i]);
    // (A pond stands on it: a metre deep.)
    w.life.env[i][1] = 1.0;
    let found = w.in_cell(i, |w, d| contract::liquid(w, d, i));
    let all = found.open + found.pores + found.soft + found.frozen;
    assert!((all - ground(&w)).abs() <= 1e-12 * all && found.open > 0.0, "the key liquid shows {found:?} of {} mol", ground(&w));
    let (g0, a0, mol) = (ground(&w), aside(&w), 0.5 * found.open);
    let paid = w.in_cell(i, |w, d| {
        let paid = contract::vapour(w, d, i, [mol, 0.0]);
        contract::give(w, d, i, [], -paid);
        paid
    });
    assert!((g0 - ground(&w) - mol).abs() <= 1e-9 * mol && (aside(&w) - a0 - mol).abs() <= 1e-9 * mol, "{mol} mol of vapour moved {} from the ground, {} to the air", g0 - ground(&w), aside(&w) - a0);
    let lv = mol * w.chem.subs[k].lv;
    assert!((paid - lv).abs() <= 1e-9 * lv, "the taker paid {paid} J for vapour whose heat of vaporisation is {lv} J");
    books.assert_kept(&w, 1e-9);
}

/// The cover of a cell lives on the cell's land (`contract::land`, `cover::ground`): the same kg per m² placed on a
/// whole land cell and on a shore cell whose land is a part of it stands as the same kg per m² of their land.
#[test]
fn the_cover_lives_on_its_cells_land() {
    let mut w = small_world();
    pcore::step(&mut w);
    let area = w.mesh.cells().area().to_vec();
    let land = |w: &World, i: usize| contract::land(w, i);
    let whole = (0..area.len()).find(|&i| land(&w, i) == area[i]).expect("a whole land cell");
    let shore = (0..area.len()).find(|&i| land(&w, i) > 0.0 && land(&w, i) < 0.9 * area[i]).expect("a shore cell with land");
    for i in [whole, shore] {
        assert!(cover::place(&mut w, i, genes(), 0, 0.1));
    }
    let kg = cover::per_m2(&w, |c, mm, x| c.kg(mm, x));
    let [a, b] = [whole, shore].map(|i| kg[i] * area[i] / land(&w, i));
    assert!((a / b - 1.0).abs() < 1e-9, "{a} kg per m2 of land on the whole cell, {b} on the shore cell");
}

/// Plants do not move (`cover::footing`): a stand holds its cell's ground above its line (`Cover::line`). Its plants on
/// ground under the liquid past their roots' reach gain nothing and live on their store (`cover::upkeep`): a flood
/// shorter than their store kills none of them, and once a longer one has spent it they die, drowned, as whole plants
/// of every group alike, and the line rises to where none stands so. Ground the liquid leaves is taken by seedlings
/// only: a month with no seeds leaves the stand's line and its large plants as they were, and seeds of its kind that
/// wake there lower its line.
#[test]
fn plants_under_the_liquid_live_on_their_store_and_seedlings_take_the_ground_it_leaves() {
    let (mut w, _) = start();
    let area = w.mesh.cells().area().to_vec();
    let p = cover::Parts::of(&w.chem).expect("cover parts");
    let shore = (0..area.len()).find(|&j| (0.3..0.9).contains(&(contract::land(&w, j) / area[j])) && cover::soak(&w, &p, j, w.g(p.liquid, j)).1.loose > 0.0).expect("a shore cell");
    assert!(cover::place(&mut w, shore, TREE.genes, TREE_ROW as u32, 2.0));
    let (mm, month, drowned) = (p.mm(&w.chem), w.cfg.calendar().month as f64, pcore::life::DROWNED as usize);
    let mut c = w.life.cover(shore).clone();
    let x = c.held().next().expect("the stand");
    // The flood, in a cool season: its line at the cell's lowest ground, and its roots get no gas under any liquid (the
    // garden's soak).
    c.line[x] = 0.0;
    let flood = testkit::garden(&w, &c, (273.0, 1.0, 200.0, 0));
    let mut aired = flood.clone();
    aired.soak.breath = 1e6;
    let month_of = |c: &mut cover::Cover, a: &cover::Around| {
        let mut counts = w.counts.clone();
        let _ = cover::live(&w, c, &p, a, (shore, month), &mut counts);
        counts.cover_lost[drowned]
    };
    assert!(month_of(&mut c, &flood) == 0.0 && c.airless[x] > 0.0 && (0.0..1.0).contains(&c.drained[x]), "a month's flood: airless {}, drained {}", c.airless[x], c.drained[x]);
    assert!(month_of(&mut c, &aired) == 0.0 && c.airless[x] == 0.0 && c.drained[x] == 0.0, "the flood is over: none drowned, none drained");
    let (mut months, mut died) = (0, 0.0);
    let mut was = c.clone();
    while died == 0.0 {
        assert!(months < 600, "a flood of {months} months spent no store");
        was = c.clone();
        (months, died) = (months + 1, month_of(&mut c, &flood));
    }
    println!("a flood killed the plants under the liquid after {months} months: {died:.3e} kg, line {} m", c.line[x]);
    assert!(c.line[x] > 0.0, "line {}", c.line[x]);
    let kept: Vec<f64> = (0..was.sizes(&w.cfg.phys, &mm, x).held).map(|g| c.plants[x][g] / was.plants[x][g]).collect();
    assert!(kept.iter().all(|k| (k / kept[0] - 1.0).abs() < 1e-9 && *k < 1.0 - 0.5 * was.airless[x] as f64), "each group kept {kept:?} of its plants");
    let soak = cover::soak(&w, &p, shore, w.g(p.liquid, shore)).1;
    let foot = cover::footing(&w, shore, c.genes[x][cover::GAS] as f64, w.cfg.phys.root_least, &cover::Soak { breath: 0.0, ..soak }, Some(c.line[x] as f64));
    assert!(foot.airless == 0.0, "after the line rose, {} of the stand is still under the liquid past its reach", foot.airless);
    // The liquid leaves: its roots get the gas everywhere, the ground below its line is free.
    let (line, large) = (c.line[x], c.sizes(&w.cfg.phys, &mm, x));
    let _ = month_of(&mut c, &aired);
    let now = c.sizes(&w.cfg.phys, &mm, x);
    assert!(c.line[x] == line && now.count[now.large()] <= large.count[large.large()], "no seeds: line {} from {line}, large plants {} from {}", c.line[x], now.count[now.large()], large.count[large.large()]);
    let seeds = cover::sown(&w.chem, &w.cfg.phys, &p, (shore, cover::ground(&w, shore), 290.0), TREE.genes, TREE_ROW as u32, 1e-3);
    c.lying.lodge(&mm, &seeds, w.cfg.phys.alike);
    let large = c.sizes(&w.cfg.phys, &mm, x);
    let _ = month_of(&mut c, &aired);
    let now = c.sizes(&w.cfg.phys, &mm, x);
    assert!(c.line[x] < line && now.count[now.large()] <= large.count[large.large()], "seedlings: line {} from {line}, large plants {} from {}", c.line[x], now.count[now.large()], large.count[large.large()]);
    // A cell all under the liquid past the kind's reach: a stand founded there still holds ground (its highest floor,
    // `Shore::found`), all of it without the gas, and dies whole, drowned, once its store is spent.
    let deep = |j: usize| {
        let s = cover::Soak { breath: 0.0, ..cover::soak(&w, &p, j, w.g(p.liquid, j)).1 };
        let f = cover::footing(&w, j, TREE.genes[cover::GAS] as f64, w.cfg.phys.root_least, &s, None);
        f.share > 0.0 && f.airless == 1.0 && s.loose > 0.0
    };
    let under = (0..area.len()).find(|&j| j != shore && deep(j)).expect("a cell all under the liquid");
    // A line over all the ground (the ground worn below it) holds the cell's highest ground.
    let over = cover::footing(&w, under, TREE.genes[cover::GAS] as f64, w.cfg.phys.root_least, &cover::soak(&w, &p, under, w.g(p.liquid, under)).1, Some(1e6));
    assert!(over.share > 0.0 && over.work.is_finite(), "a line over all the ground: {over:?}");
    let mut c = w.life.cover(shore).clone();
    let x = c.held().next().expect("the stand");
    let kg = c.kg(&mm, x);
    let mut counts = w.counts.clone();
    let flood = testkit::garden(&w, &c, (273.0, 1.0, 200.0, 0));
    c.line[x] = cover::UNSET;
    for months in 0.. {
        assert!(months < 600, "a stand all under the liquid lives {months} months");
        let _ = cover::live(&w, &mut c, &p, &flood, (under, month), &mut counts);
        assert!(c.mol.iter().flatten().all(|m| m.is_finite()) && c.stands.iter().all(|s| s.is_finite()), "month {months}: {:?}", c.mol[x]);
        if c.held().next().is_none() {
            break;
        }
    }
    let drowned = counts.cover_lost[pcore::life::DROWNED as usize] - w.counts.cover_lost[pcore::life::DROWNED as usize];
    assert!(drowned > 0.5 * kg, "{drowned} kg of {kg} drowned");
}

/// A stand of the tree kind (2 kg per m²) on ground that held nothing for roots half of the last year
/// (`Cover::idle`) lives through two years of six good months and six months without gain (`cover::live` with a
/// garden's surroundings: warm sun, its ground full; then either its ground holds nothing for roots at the same warmth,
/// or its ground is frozen). A kg of leaf costs more to keep through six warm dry months than to build anew, and less
/// through six frozen ones (the burning's pace is low in the cold): so in the dry years it drops all its leaves as its
/// dry season starts, keeps its walls and its store, and grows its leaves again from its store when its ground is wet
/// again; in the frozen years it keeps its leaves (Kikuzawa 1991, `season::shed`).
#[test]
fn a_stand_drops_its_leaves_for_a_long_dry_season_but_not_for_a_frozen_one() {
    let (mut w, i) = start();
    assert!(cover::place(&mut w, i, TREE.genes, TREE_ROW as u32, 2.0));
    let p = cover::Parts::of(&w.chem).expect("cover parts");
    let (mm, area) = (p.mm(&w.chem), cover::ground(&w, i));
    let (sun, month) = (200.0, w.cfg.calendar().month as f64);
    let seasons = |frozen: bool| {
        let mut c = w.life.cover(i).clone();
        c.idle = 0.5;
        let mut counts = w.counts.clone();
        let mut held = Vec::new();
        for m in 0..24u64 {
            let good = m % 12 < 6;
            let a = testkit::garden(&w, &c, (if good || !frozen { 295.0 } else { 270.0 }, if good { 1.0 } else { 0.0 }, sun, m));
            let _ = cover::live(&w, &mut c, &p, &a, (i, month), &mut counts);
            let Some(x) = c.held().find(|&x| c.root[x] == TREE_ROW as u32) else { panic!("month {m}: the stand died; {held:?}") };
            held.push((c.living(&mm, x) / area, c.walls(&mm, x) / area, c.mol[x][cover::FUEL] * mm[cover::FUEL] / area));
        }
        held
    };
    let (dry, cold) = (seasons(false), seasons(true));
    for (m, (d, f)) in dry.iter().zip(&cold).enumerate() {
        println!("month {m:2}: dry year living, walls, fuel kg/m² {:.3} {:.3} {:.3}; frozen year {:.3} {:.3} {:.3}", d.0, d.1, d.2, f.0, f.1, f.2);
    }
    // Its dry season: no leaves, its walls and its store kept; the wet months after: leaves again.
    let (wet, bare) = (dry[5], dry[6..12].iter().map(|d| d.0).fold(0.0, f64::max));
    assert!(bare < 1e-3 * wet.0, "it kept {bare} kg/m² of leaves through its dry season (had {})", wet.0);
    assert!(dry[11].1 > 0.9 * wet.1 && dry[11].2 > 0.0, "it lost its walls or its store: {wet:?} then {:?}", dry[11]);
    assert!(dry[12].0 > 0.0 && dry[17].0 > 0.1 * wet.0, "it grew no leaves again: {:?} then {:?}", dry[12], dry[17]);
    // Frozen seasons: its leaves are kept.
    assert!(cold[11].0 > 0.5 * cold[5].0, "it dropped its leaves for a frozen season: {:?} then {:?}", cold[5], cold[11]);
}
