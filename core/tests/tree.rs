//! The tree of cells while a world runs (laws.md 0): between steps a cell splits into its quarters and four quarters
//! join into their parent (`World::split`, `World::join`), cells across a border one depth apart at most, the books
//! exact, a split's quarters standing as the terrain inside their cell does, paid by its relief, and the relief a join
//! flattens given back by its split; the life of a cell (its nodes, individuals, parts and
//! cover) is shared out to the quarters and gathered again, every element, the energy and the members kept. A region
//! split keeps the climate of the unsplit world (spec 5.2).

use pcore::body::Body;
use pcore::check;
use pcore::chem::Phase;
use pcore::event::EventKind;
use pcore::space::square::Square;
use pcore::space::terrain::SAMPLE_DEPTH;
use pcore::space::tree::{Refused as Shape, DEPTH_MOST};
use pcore::space::Leaf;
use pcore::store::Class;
use pcore::{brain, World};
use std::collections::BTreeMap;
use worldgen::life::{Place, KINDS};
use worldgen::testkit::{self, sound, Books};

/// The small world, its events taken: a world without life, at a month's start.
fn world() -> World {
    let mut w = testkit::small_world();
    w.drain_events();
    w
}

/// The leaf of cell `i`.
fn leaf(w: &World, i: usize) -> Leaf {
    w.map.leaves()[i]
}

/// The saved state of `w` but its life (a split or a join gives its cells new serials), the records of its joins and
/// the reliefs.
fn saved(w: &World) -> serde_json::Value {
    let mut v = serde_json::to_value(w).expect("a world serialises");
    let o = v.as_object_mut().expect("an object");
    ["life", "joined", "relief"].iter().for_each(|k| _ = o.remove(*k));
    v
}

/// The first place where `a` and `b` differ by more than `tol` of a number, by its path, and the two there.
fn difference(a: &serde_json::Value, b: &serde_json::Value, path: &str, tol: f64) -> Option<String> {
    use serde_json::Value::{Array, Number, Object};
    match (a, b) {
        (Array(x), Array(y)) if x.len() == y.len() => x.iter().zip(y).enumerate().find_map(|(k, (p, q))| difference(p, q, &format!("{path}/{k}"), tol)),
        (Object(x), Object(y)) if x.len() == y.len() => x.iter().find_map(|(k, p)| y.get(k).map_or(Some(format!("{path}/{k} missing")), |q| difference(p, q, &format!("{path}/{k}"), tol))),
        (Number(x), Number(y)) if x.is_f64() && y.is_f64() => {
            let (x, y) = (x.as_f64().expect("a number"), y.as_f64().expect("a number"));
            ((x - y).abs() > tol * x.abs().max(y.abs())).then(|| format!("{path}: {x} against {y}"))
        }
        _ => (a != b).then(|| format!("{path}: {a} against {b}")),
    }
}

/// What the nodes of cell `i` hold: each amount of each class of its nodes, then their heat.
fn holds(w: &World, i: usize) -> Vec<f64> {
    let (n, ns) = (w.mesh.cells().n(), w.chem.ns());
    let classes = [Class::Ground, Class::Deep, Class::Bed, Class::Bodies];
    let amounts = classes.iter().flat_map(|&c| (0..ns).map(move |k| w.store.amounts(c)[k * n + i]));
    amounts.chain(classes.iter().filter_map(|&c| w.store.heat(c).get(i).copied())).collect()
}

/// The world's potential energy less what the stacks hold now and the cells' reliefs: a split or a join changes none
/// of it (`Store::lifted`).
fn unflattened(w: &World) -> f64 {
    w.store.lifted() - pcore::flow::potential(w).expect("Flow's state") - w.relief.iter().sum::<f64>()
}

/// A cell with no relief (its ground flat inside it) split into flat quarters (each a quarter of what it held, at
/// its temperature) and joined again is the world it was, every number of it, where Flow's state of the ground is its
/// heat's (a cell a remap makes takes the state of its heat, `World::thawed`): bit for bit but the ground's heat, which
/// takes the rounding of the stacks' potential energy (`TIGHT`); each change is an event naming the cell by its serial.
/// Each quarter reads its cell's ground temperature, liquid depth and height at once (`Life::env`), and the joined cell
/// their mean.
#[test]
fn a_split_then_its_join_gives_back_the_world() {
    let mut w = world();
    pcore::flow::state(&mut w, true);
    let (i, n) = (testkit::dry_land(&w), w.mesh.cells().n());
    w.relief[i] = 0.0;
    let (l, before, held, first, env) = (leaf(&w, i), saved(&w), holds(&w, i), w.life.serial(i), w.life.env[i]);
    assert_eq!(w.split(l), Ok(vec![l]));
    assert_eq!(w.mesh.cells().n(), n + 3);
    sound(&w);
    for q in l.quarters() {
        let j = w.map.cell(q).expect("a quarter");
        assert!(holds(&w, j).iter().zip(&held).all(|(&x, &y)| x == y / 4.0), "quarter {q:?} holds a quarter of what its cell held");
        assert_eq!(w.life.env[j], env, "quarter {q:?} reads its cell's surroundings");
    }
    w.join(l).expect("its quarters join");
    assert_eq!(difference(&saved(&w), &before, "", TIGHT), None);
    let j = w.map.cell(l).expect("the joined cell");
    assert_eq!(w.life.env[j], env, "the joined cell reads its quarters' mean");
    let events: Vec<_> = w.drain_events().into_iter().map(|e| (e.kind, e.node)).collect();
    assert_eq!(events, [(EventKind::Split, first), (EventKind::Join, w.life.serial(j))]);
    assert!(w.life.serial(j) > first, "the joined cell takes a serial never given before");
}

/// Four quarters of different heights joined and split again hold what they held, each amount and heat to the rounding
/// of x · share (4 ε of it, ε = 2^-53), and the relief the join kept comes back to the stacks: the world's potential
/// energy less the stacks' and the reliefs (`unflattened`) and the books do not move.
#[test]
fn a_join_then_its_split_gives_back_the_quarters() {
    let mut w = world();
    let parent = leaf(&w, testkit::dry_land(&w)).parent().expect("a square has a parent");
    let rest = unflattened(&w);
    let relief = join_then_split(&mut w, parent);
    assert!(relief > 0.0, "a join of uneven quarters flattens their stacks: relief {relief} J");
    assert!((unflattened(&w) - rest).abs() <= 1e-12 * w.store.lifted().abs().max(relief), "the potential energy kept: {} J", unflattened(&w) - rest);
}

/// How far over the key liquid's boiling point `a_join_then_its_split_gives_back_quarters_of_two_phases` heats a
/// quarter, K.
const BOILING_BY: f64 = 20.0;

/// The same where the key liquid of one quarter boils (its ground heated over the boiling point) and the joined cell's
/// mean temperature is under it: a stack's phases are those of its heat, so the join and the split see the same
/// stacks, and the split gives no heat the quarters did not hold.
#[test]
fn a_join_then_its_split_gives_back_quarters_of_two_phases() {
    let mut w = world();
    let parent = leaf(&w, testkit::dry_land(&w)).parent().expect("a square has a parent");
    let hot = w.map.cell(parent.quarters()[0]).expect("a square");
    let boil = w.chem.subs[testkit::LIQUID].boil;
    let e = w.energy_ground(hot, boil + BOILING_BY);
    testkit::set_heat(&mut w, pcore::store::Node::ground(hot), e);
    pcore::flow::state(&mut w, true);
    let mean = parent.quarters().iter().map(|&q| w.t_ground(w.map.cell(q).expect("a square"))).sum::<f64>() / 4.0;
    assert!(w.t_ground(hot) > boil && mean < boil, "one quarter boils ({} K), their mean ({mean} K) does not", w.t_ground(hot));
    join_then_split(&mut w, parent);
}

/// Join the quarters of `parent` of `w` and split it again at once: each quarter holds what it held, each amount to
/// four roundings (2 ε of it), each heat to four roundings of it and of the potential energy the round trip books
/// through it (the split gives the quarters' ground heat what the relief has left over the stacks' potential energy,
/// `store::fall_heat`: 2 ε of the heat and of that potential energy, which is that of the quarters' stacks and their
/// air, `flow::potential_of`); and the books do not move. Returns the join's relief.
fn join_then_split(w: &mut World, parent: Leaf) -> f64 {
    let kids = parent.quarters().map(|q| w.map.cell(q).expect("a square"));
    let (held, books) = (kids.map(|i| holds(w, i)), Books::now(w));
    w.join(parent).expect("four squares join");
    let relief = w.relief[w.map.cell(parent).expect("the joined cell")];
    assert_eq!(w.split(parent), Ok(vec![parent]));
    sound(w);
    let cells = parent.quarters().map(|q| w.map.cell(q).expect("a quarter"));
    let booked = pcore::flow::potential_of(w, &cells, w.mesh.up(cells[0], 0, pcore::AIR)).abs();
    let amounts = w.chem.ns() * 4;
    println!("the round trip books {booked:.4e} J of potential energy through the ground's heat");
    for (q, held) in parent.quarters().iter().zip(&held) {
        let now = holds(w, w.map.cell(*q).expect("a quarter"));
        for (k, (&x, &y)) in now.iter().zip(held).enumerate() {
            let of = if k < amounts { y.abs() } else { y.abs() + booked };
            assert!((x - y).abs() <= 2.0 * f64::EPSILON * of, "quarter {q:?}, number {k}: {y:e} back as {x:e}, booked {booked:e} J");
        }
    }
    books.assert_kept(w, 1e-12);
    relief
}

/// The top of the bed of cell `i`, m: its bonded amounts as solids over its area, from the base up.
fn bed_top(w: &World, i: usize) -> f64 {
    let v: f64 = (0..w.chem.ns()).map(|k| w.b(k, i) * w.chem.molar_volume(k, Phase::Solid)).sum();
    w.base_z + v / w.mesh.cells().area()[i]
}

/// The ground's heat of cells `cells`, J.
fn ground_heat(w: &World, cells: &[usize]) -> f64 {
    cells.iter().map(|&i| w.store.heat(Class::Ground)[i]).sum()
}

/// The least spread of the quarters' bed tops a split of a land square gives, m: the terrain's relief inside 2 km.
const UNEVEN_M: f64 = 1.0;

/// A split of a land cell no join made gives its quarters the terrain inside it, drained: their beds stand at different
/// heights whose mean by area is the cell's, each has a neighbour no higher than it, the books close (`TIGHT`), and the
/// cell's relief is what the stacks gain and the quarters' reliefs (to the rounding of the world's potential energy),
/// none of it heat.
#[test]
fn a_split_gives_its_quarters_the_terrain() {
    let mut w = world();
    let i = testkit::dry_land(&w);
    let (l, books, relief, top) = (leaf(&w, i), Books::now(&w), w.relief[i], bed_top(&w, i));
    let (stacks, heat) = (pcore::flow::potential(&w).expect("Flow's state"), ground_heat(&w, &[i]));
    w.split(l).expect("a split");
    sound(&w);
    books.assert_kept(&w, TIGHT);
    let kids = l.quarters().map(|q| w.map.cell(q).expect("a quarter"));
    let (tops, area) = (kids.map(|j| bed_top(&w, j)), kids.map(|j| w.mesh.cells().area()[j]));
    let mean = tops.iter().zip(area).map(|(t, a)| t * a).sum::<f64>() / area.iter().sum::<f64>();
    let spread = tops.iter().fold(f64::MIN, |a, &b| a.max(b)) - tops.iter().fold(f64::MAX, |a, &b| a.min(b));
    let (spent, kept, warmed) = (pcore::flow::potential(&w).expect("Flow's state") - stacks, kids.map(|j| w.relief[j]).iter().sum::<f64>(), ground_heat(&w, &kids) - heat);
    println!("bed tops {tops:?} m (the cell's {top} m); relief {relief:.4e} J: the stacks {spent:.4e}, the quarters {kept:.4e}, the heat {warmed:.3e}");
    assert!((mean - top).abs() <= TIGHT * top.abs() && spread > UNEVEN_M, "the quarters' beds at {tops:?} m about the cell's {top} m");
    assert!((spent + kept - relief).abs() <= TIGHT * stacks.abs() && warmed.abs() <= TIGHT * heat, "the relief pays what the split gains, {warmed:e} J to heat");
    let drains = |j: usize| w.mesh.cells().around(j).1.iter().any(|&o| w.surface_z(o as usize) <= w.surface_z(j));
    assert!(kids.iter().all(|&j| drains(j)), "each quarter drains");
}

/// How far the ground of cell `i` stands over the terrain's mean inside it, m.
fn over_terrain(w: &World, i: usize) -> f64 {
    w.surface_z(i) - w.terrain.inside(w.seed, &w.map, leaf(w, i), (w.cfg.lat_south, w.cfg.lat_north)).0
}

/// The ground stands on the drained terrain at every size (laws.md 0, 8): no sample of land is a closed pit (each has a
/// lower neighbour), a world starts with each cell at the terrain's mean over it, and the cells two splits make of a
/// block of land stand at the terrain's mean over each as their cell did, to `UNEVEN_M`: no quarter carries its cell's
/// height as a step, and none is drained apart from its neighbours.
#[test]
fn split_cells_stand_on_the_terrain() {
    let mut w = world();
    let s = w.terrain.samples(w.seed, &w.map, (w.cfg.lat_south, w.cfg.lat_north));
    let lower = |k: usize| {
        let (r, c) = ((k / s.across) as i64, (k % s.across) as i64);
        let near = (-1..=1i64).flat_map(|a| (-1..=1i64).map(move |b| (r + a, c + b)));
        near.filter(|&(y, x)| (0..s.down as i64).contains(&y) && (0..s.across as i64).contains(&x)).any(|(y, x)| s.h[y as usize * s.across + x as usize] < s.h[k])
    };
    assert!((0..s.h.len()).filter(|&k| s.h[k] >= 0.0).all(lower), "every sample of land has a lower neighbour");
    let block = testkit::land_block(&w);
    let start: Vec<f64> = (0..w.mesh.cells().n()).map(|i| over_terrain(&w, i)).collect();
    let was: Vec<f64> = block.iter().map(|&l| over_terrain(&w, w.map.cell(l).expect("a cell"))).collect();
    testkit::split_down(&mut w, &block, 2);
    let kids: Vec<f64> = block.iter().zip(&was).flat_map(|(l, d)| l.split(l.depth + 2).map(|q| over_terrain(&w, w.map.cell(q).expect("a cell")) - d).collect::<Vec<f64>>()).collect();
    let most = |d: &[f64]| d.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    println!("the cells start within {} m of the terrain, the split cells within {} m beyond their cell", most(&start), most(&kids));
    assert!(most(&start) < UNEVEN_M && most(&kids) < UNEVEN_M, "the cells stand on the terrain");
}

/// The liquid of cell `i` as a depth over its area, m, and the depth its pores hold, m (the volume of its loose solids
/// times `porosity`).
fn liquid_and_pores(w: &World, i: usize) -> (f64, f64) {
    let (t, area) = (w.t_ground(i), w.mesh.cells().area()[i]);
    let liquid: f64 = (0..w.chem.ns()).filter(|&k| w.chem.phase(k, t) == Phase::Liquid).map(|k| w.g(k, i) * w.chem.molar_volume(k, Phase::Liquid)).sum();
    (liquid / area, w.cfg.phys.porosity * testkit::loose_volume(w, i) / area)
}

/// A split of the cell under 0 m of the most relief puts its liquid where it would stand on its quarters' floors: each
/// keeps its pores full (the least liquid a quarter holds is what the pores of its loose ground hold), and what stood
/// over them fills the quarters from the lowest up to one level, so a quarter is under that level or dry above it. The
/// books close (`TIGHT`), and the cell's relief pays what the stacks gain and the quarters' reliefs, none of it heat.
#[test]
fn a_split_stands_its_liquid_at_one_level() {
    let mut w = world();
    let n = w.mesh.cells().n();
    let i = (0..n).filter(|&i| !w.is_land(i)).max_by(|&a, &b| w.relief[a].total_cmp(&w.relief[b])).expect("a cell under 0 m");
    let (l, books, relief) = (leaf(&w, i), Books::now(&w), w.relief[i]);
    let (stacks, heat) = (pcore::flow::potential(&w).expect("Flow's state"), ground_heat(&w, &[i]));
    w.split(l).expect("a split");
    sound(&w);
    books.assert_kept(&w, TIGHT);
    let kids = l.quarters().map(|q| w.map.cell(q).expect("a quarter"));
    let (spent, kept, warmed) = (pcore::flow::potential(&w).expect("Flow's state") - stacks, kids.map(|j| w.relief[j]).iter().sum::<f64>(), ground_heat(&w, &kids) - heat);
    assert!((spent + kept - relief).abs() <= TIGHT * stacks.abs() && warmed.abs() <= TIGHT * heat, "the relief pays what the split gains, {warmed:e} J to heat");
    // Each quarter's solid surface and liquid, m; what the pores of each hold (the least liquid of a quarter), and the
    // level of what stands over them.
    let at = kids.map(|j| (w.surface_z(j), liquid_and_pores(&w, j)));
    let pores = at.iter().map(|x| x.1 .0).fold(f64::MAX, f64::min);
    let level = at.iter().filter(|x| x.1 .0 > pores + LEVEL_M).map(|&(z, (liquid, _))| z + liquid - pores).fold(f64::MIN, f64::max);
    println!("quarters (surface, (liquid, pores)) {at:?} m: the level {level:.2} m");
    assert!((pores - at[0].1 .1).abs() <= PORES * pores, "the pores full: {pores} m of {} m", at[0].1 .1);
    assert!(at.iter().any(|x| x.0 < level) && at.iter().any(|x| x.0 > level), "the scene has wet quarters and dry ones");
    for &(z, (liquid, _)) in &at {
        let (wet, dry) = ((z + liquid - pores - level).abs() <= LEVEL_M, liquid - pores <= LEVEL_M && z >= level);
        assert!(wet || dry, "a quarter at {z} m with {liquid} m of liquid: under the level or dry above it");
    }
}

/// How near one level the liquid of the quarters stands, m.
const LEVEL_M: f64 = 1e-6;
/// How near the depth the pores of the loose ground hold the least liquid of a quarter is, relative (Flow's state
/// counts the fixed solids apart).
const PORES: f64 = 1e-4;

/// A split, its join and its split again give the same quarters: each amount to the rounding of x · share and each heat
/// to that of the stacks' potential energy (`TIGHT`), and each one's relief, from the record of the join; the joined
/// cell's relief is theirs and what it flattened.
#[test]
fn a_split_its_join_and_its_split_give_the_same_quarters() {
    let mut w = world();
    let l = leaf(&w, testkit::dry_land(&w));
    let books = Books::now(&w);
    w.split(l).expect("a split");
    let kids = |w: &World| l.quarters().map(|q| w.map.cell(q).expect("a quarter"));
    let (held, relief) = (kids(&w).map(|i| holds(&w, i)), kids(&w).map(|i| w.relief[i]));
    w.join(l).expect("its quarters join");
    let joined = w.relief[w.map.cell(l).expect("the joined cell")];
    assert!(joined > relief.iter().sum::<f64>(), "a join of uneven quarters flattens their stacks: {joined} J over {relief:?}");
    w.split(l).expect("a split again");
    sound(&w);
    for ((i, held), r) in kids(&w).iter().zip(&held).zip(relief) {
        let worst = holds(&w, *i).iter().zip(held).map(|(&x, &y)| (x - y).abs() / y.abs().max(f64::MIN_POSITIVE)).fold(0.0, f64::max);
        assert!(worst <= TIGHT && w.relief[*i] == r, "quarter {i} back within {worst:e}, relief {} J against {r} J", w.relief[*i]);
    }
    books.assert_kept(&w, TIGHT);
}

/// The relief is one account at every size (`Terrain::inside`): a leaf's spread of heights is its quarters' and the
/// spread of the quarters' means about its own, to rounding (`TIGHT`), for a cell, a leaf coarser than the terrain's
/// samples, one of their depth and one below them, where the ground goes on as a midpoint displacement; a cell's relief
/// as built is its spread times the weight of a m³ of its bed (ρ g, between `BED_WEIGHT`).
#[test]
fn the_relief_of_a_cell_is_its_quarters_and_what_lies_between() {
    let w = world();
    let (lat, map) = ((w.cfg.lat_south, w.cfg.lat_north), &w.map);
    let low = (0..w.mesh.cells().n()).find(|&i| !w.is_land(i) && w.relief[i] > 0.0).expect("a cell under 0 m above the floor of the terrain");
    for i in [testkit::dry_land(&w), low] {
        let cell = leaf(&w, i);
        for l in [cell.depth, SAMPLE_DEPTH - 1, SAMPLE_DEPTH, SAMPLE_DEPTH + 2].map(|d| cell.split(d).last().expect("a leaf")) {
            let (mean, spread) = w.terrain.inside(w.seed, map, l, lat);
            let between: f64 = l.quarters().iter().map(|q| (map.place_of(q, lat.0, lat.1).0, w.terrain.inside(w.seed, map, *q, lat))).map(|(a, (m, s))| s + a * (m - mean) * (m - mean) / 2.0).sum();
            println!("cell {i}, depth {}: mean {mean:.2} m, spread {spread:.4e} m⁴", l.depth);
            assert!((between - spread).abs() <= TIGHT * spread, "cell {i}, depth {}: the quarters' {between:e} against the leaf's {spread:e}", l.depth);
        }
        let spread = w.terrain.inside(w.seed, map, cell, lat).1;
        assert!((BED_WEIGHT[0]..BED_WEIGHT[1]).contains(&(w.relief[i] / spread)), "cell {i}: relief {} J of a spread of {spread} m⁴", w.relief[i]);
    }
}

/// The weight of a m³ of a bed, N: rock of 2000 to 4000 kg/m³ under 9.81 m/s².
const BED_WEIGHT: [f64; 2] = [2e4, 4e4];

/// Months of splits and joins, the world stepping between them: down two depths and back on land, at the coast and on a
/// cell under 0 m, the books within `check::DRIFT`, nothing below zero or NaN, 2:1 kept.
#[test]
fn splits_and_joins_between_steps_keep_the_books() {
    let mut w = world();
    let cells = places(&w, testkit::dry_land(&w));
    run_splits(&mut w, cells, 1);
}

/// `splits_and_joins_between_steps_keep_the_books` for years.
#[test]
#[ignore]
fn years_of_splits_and_joins_keep_the_books() {
    let mut w = world();
    let cells = places(&w, testkit::dry_land(&w));
    run_splits(&mut w, cells, 12);
}

/// Cell `land`, the first cell of the coast and the first cell under 0 m of `w`, as leaves.
fn places(w: &World, land: usize) -> [Leaf; 3] {
    let coast = (0..w.mesh.cells().n()).find(|&i| w.is_land(i) && w.mesh.cells().around(i).1.iter().any(|&j| !w.is_land(j as usize))).expect("a coast");
    let low = (0..w.mesh.cells().n()).find(|&i| !w.is_land(i) && w.relief[i] > 0.0).expect("a cell under 0 m above the floor of the terrain");
    [land, coast, low].map(|i| leaf(w, i))
}

/// `rounds` rounds of splits and joins of `cells` of `w`, a month between each change: each split, then its first
/// quarter split again (which first splits the cells beside it), then those joined back in turn, then the cell; the
/// events of each change taken before the next (`World::drain_events`, as the shell does). Returns
/// the largest share of its quarters' cover that a join sent to the ground (the slots that found no slot in the joined
/// cell, `cover::land`).
fn run_splits(w: &mut World, cells: [Leaf; 3], rounds: usize) -> f64 {
    let books = Books::now(w);
    let mut lost = 0.0f64;
    let month = |w: &mut World| {
        pcore::step(w);
        w.drain_events();
        sound(w);
        books.assert_kept(w, check::DRIFT);
    };
    let mut join = |w: &mut World, l: Leaf| {
        let before: f64 = l.quarters().iter().map(|&q| cover_kg(w, w.map.cell(q).expect("a quarter"))).sum();
        w.join(l).expect("the join of what a split made");
        // (The events of a join's merge are taken before the next change, as the shell takes a month's and
        // `World::refine` sets a join's aside: `World::remap` runs only once they are.)
        w.drain_events();
        let after = cover_kg(w, w.map.cell(l).expect("the joined cell"));
        lost = lost.max(if before > 0.0 { 1.0 - after / before } else { 0.0 });
    };
    for _ in 0..rounds {
        for &l in &cells {
            w.split(l).expect("a split");
            month(w);
            let fine = l.quarters()[3];
            let forced = w.split(fine).expect("a split two depths down");
            assert_eq!(forced.last(), Some(&fine));
            month(w);
            forced.iter().rev().for_each(|&f| join(w, f));
            month(w);
            join(w, l);
            month(w);
        }
    }
    lost
}

/// The cover of cell `i`, kg.
fn cover_kg(w: &World, i: usize) -> f64 {
    let (c, mm) = (w.life.cover(i), pcore::cover::Parts::of(&w.chem).expect("the cover's parts").mm(&w.chem));
    c.held().map(|x| c.kg(&mm, x)).sum()
}

/// A split two depths below its neighbours first splits them (and theirs), so cells across a border differ by one depth
/// at most after each split; a join that would leave a neighbour two depths finer is refused and changes nothing, as
/// are a split of a cell of the most depth and a join of what are not four cells.
#[test]
fn the_tree_keeps_cells_one_depth_apart() {
    let mut w = world();
    let l = leaf(&w, testkit::dry_land(&w));
    w.split(l).expect("a split");
    let fine = l.quarters()[0];
    let made = w.split(fine).expect("a split two depths down");
    assert!(made.len() > 1 && made.last() == Some(&fine) && made[..made.len() - 1].iter().all(|o| o.depth == l.depth), "the squares beside it split first: {made:?}");
    sound(&w);
    let before = saved(&w);
    assert_eq!(w.join(made[0]), Err(Shape::Unbalanced));
    assert_eq!(w.join(l.parent().expect("a parent")), Err(Shape::NoCell));
    assert_eq!(difference(&saved(&w), &before, "", 0.0), None);
    // A block split at its north-west corner down to a leaf of the most depth.
    let mut chain = vec![Leaf { x: 0, y: 0, depth: 0 }];
    for _ in 0..DEPTH_MOST {
        chain = chain.into_iter().flat_map(|l| l.split(l.depth + u8::from(l.x == 0 && l.y == 0))).collect();
    }
    let deepest = Square::new(4, 4, 2000.0, false, 4).with(chain);
    assert_eq!(deepest.splits(deepest.leaves()[0]), Err(Shape::Deepest));
}

/// The members of the node of `with_life`.
const NODE: f64 = 40.0;
/// How near the books and the members stay through a split and a join, relative: the rounding of x · share.
const TIGHT: f64 = 1e-12;

/// The members of the living bodies by starting kind (`Lineage::root`).
fn members(w: &World) -> BTreeMap<u32, f64> {
    let mut m = BTreeMap::new();
    w.life.alive().for_each(|b| *m.entry(w.life.brain(b.lineage).expect("a lineage").root).or_insert(0.0) += b.count);
    m
}

/// The members of each kind are those of `then`, within `TIGHT`.
fn same_members(w: &World, then: &BTreeMap<u32, f64>) {
    let now = members(w);
    assert!(now.len() == then.len() && then.iter().all(|(k, x)| (now[k] - x).abs() <= TIGHT * x), "members by kind {then:?}, now {now:?}");
}

/// The slots of cover of cell `i` that hold cover, by starting kind: mol of each part.
fn slots(w: &World, i: usize) -> BTreeMap<u32, [f64; pcore::cover::NP]> {
    let c = w.life.cover(i);
    c.held().map(|x| (c.root[x], c.mol[x])).collect()
}

/// The small world with life in its first dry land cell: a node of `NODE` eaters and an individual eater, with brains,
/// the individual holding a part bonded to another (a structure), and cover of two kinds (`KINDS` rows 0 and 2).
/// Returns the world, the cell, the ids of the node and of the individual, and the ids of the parts.
fn with_life() -> (World, usize, [u64; 2], [u64; 2]) {
    let mut w = world();
    let i = testkit::dry_land(&w);
    testkit::cover(&mut w, i, 1.0);
    let Place::Field(f) = &KINDS[2].place else { unreachable!("kind 2 is cover") };
    assert!(pcore::cover::place(&mut w, i, f.genes, 2, 0.5), "room for a second kind of cover");
    let parts = [testkit::plate(&mut w, i, 10.0, false), testkit::plate(&mut w, i, 10.0, true)];
    pcore::bond::link(&mut w, parts[0], parts[1], 1e9);
    let node = testkit::eater(&mut w, i, brain::REST, 0.0);
    let node = testkit::node_of(&mut w, node, NODE);
    let node = w.add(node);
    let mut one = testkit::eater(&mut w, i, brain::REST, 0.0);
    one.hold = Some(parts[0]);
    let one = w.add(one);
    let ids = [w.life[node].id, w.life[one].id];
    w.life.part_mut(parts[0]).expect("the part").held = ids[1];
    w.drain_events();
    (w, i, ids, parts)
}

/// A cell with a node, an individual holding a structure and cover of two kinds, split and joined again, keeps every
/// element, the energy and the members of each kind (`TIGHT`). Split, each quarter holds a quarter of the node, with
/// the node's brain rows, and of each kind of cover what its growth law gives there, the four together all of it
/// (laws.md 0, "Inside a cell"); the individual and its structure lie whole in one quarter. Every id finds its body or
/// part, and once joined the cell holds its cover as it was.
#[test]
fn life_follows_a_split_and_its_join() {
    let (mut w, i, [node, one], parts) = with_life();
    let (books, kinds, l, cover) = (Books::now(&w), members(&w), leaf(&w, i), slots(&w, i));
    let (rows, lineage) = (w.life.rows_of(&w.life[w.life.find(node).expect("the node")]).to_vec(), w.life[w.life.find(node).expect("the node")].lineage);
    w.split(l).expect("a cell with life splits");
    sound(&w);
    books.assert_kept(&w, TIGHT);
    same_members(&w, &kinds);
    let h = w.life.find(one).expect("the individual");
    assert_eq!(w.life[h].count, 1.0, "the individual is not cut");
    assert!(parts.iter().all(|&p| w.life.part_cell(p) == Some(h.cell as usize)), "its structure goes with it");
    assert!(w.life.find(node).is_some(), "the node keeps its id");
    let mut kinds_held: BTreeMap<u32, [f64; pcore::cover::NP]> = BTreeMap::new();
    for q in l.quarters() {
        let c = w.map.cell(q).expect("a quarter");
        let shares: Vec<&Body> = w.life.cell(c).map(|s| &w.life[s]).filter(|b| b.lineage == lineage).collect();
        assert!(shares.len() == 1 && (shares[0].count - NODE / 4.0).abs() <= TIGHT * NODE, "a quarter of the node in quarter {q:?}");
        assert_eq!(w.life.rows_of(shares[0]), &rows[..], "with the node's rows");
        assert!(slots(&w, c).len() == 2 && slots(&w, c).values().all(|m| m.iter().all(|&x| x >= 0.0)), "each kind of cover, none below 0");
        slots(&w, c).iter().for_each(|(k, m)| (0..m.len()).for_each(|x| kinds_held.entry(*k).or_default()[x] += m[x]));
    }
    let near = |a: f64, b: f64| (a - b).abs() <= TIGHT * b;
    assert!(kinds_held.iter().all(|(k, m)| (0..m.len()).all(|x| near(m[x], cover[k][x]))), "the quarters hold all of each kind of cover");
    w.join(l).expect("its quarters join");
    sound(&w);
    books.assert_kept(&w, TIGHT);
    same_members(&w, &kinds);
    let j = w.map.cell(l).expect("the joined cell");
    assert!(w.life.find(one).is_some_and(|h| h.cell as usize == j) && parts.iter().all(|&p| w.life.part_cell(p) == Some(j)), "the individual and its parts in the joined cell");
    let now = slots(&w, j);
    assert!(now.len() == 2 && now.iter().all(|(k, m)| (0..m.len()).all(|x| (m[x] - cover[k][x]).abs() <= TIGHT * cover[k][x])), "the cover as it was: {now:?} against {cover:?}");
}

/// A split cuts no individual and no node of fewer members than one per quarter's share: each lies whole in one
/// quarter, drawn by area, so the individuals of a cell spread over its quarters; a larger node lies in each quarter, its
/// members there summing to its own.
#[test]
fn a_split_cuts_no_individual() {
    let mut w = world();
    let i = testkit::dry_land(&w);
    let counts = [1.0; 12].into_iter().chain([3.5, 4.0, NODE]);
    let made: Vec<(u32, f64)> = counts.map(|n| {
        let b = testkit::eater(&mut w, i, brain::REST, 0.0);
        let b = if n > 1.0 { testkit::node_of(&mut w, b, n) } else { b };
        let h = w.add(b);
        (w.life[h].lineage, n)
    }).collect();
    w.split(leaf(&w, i)).expect("a split");
    let mut cells = std::collections::BTreeSet::new();
    for &(lineage, n) in &made {
        let of: Vec<&Body> = w.life.alive().filter(|b| b.lineage == lineage).collect();
        let sum: f64 = of.iter().map(|b| b.count).sum();
        assert!((sum - n).abs() <= TIGHT * n, "a node of {n} keeps its members: {sum}");
        assert_eq!(of.len(), if n * 0.25 >= 1.0 { 4 } else { 1 }, "a body of {n} members lies in {} quarters", of.len());
        if n == 1.0 {
            assert_eq!(of[0].count, 1.0, "an individual is not cut");
            cells.insert(of[0].cell);
        }
    }
    assert!(cells.len() > 1, "the individuals spread over the quarters: {cells:?}");
}

/// Months of splits and joins of cells with life (the land cell with the most bodies, the coast, a cell under 0 m) on a
/// living island, the world stepping between them: the books within `check::DRIFT`, nothing below zero or NaN, 2:1
/// kept; the largest share of its quarters' cover a join sends to the ground is printed.
#[test]
fn splits_and_joins_of_cells_with_life_keep_the_books() {
    run_living(1);
}

/// `splits_and_joins_of_cells_with_life_keep_the_books` for years.
#[test]
#[ignore]
fn years_of_splits_and_joins_with_life_keep_the_books() {
    run_living(6);
}

/// `rounds` rounds of `run_splits` on a living island of 22 x 22 cells after 3 months.
fn run_living(rounds: usize) {
    let mut w = testkit::living(22, 3);
    w.drain_events();
    let busiest = (0..w.mesh.cells().n()).filter(|&i| w.life.cover(i).any()).max_by_key(|&i| (w.life.in_cell(i), std::cmp::Reverse(i))).expect("a cell");
    assert!(w.life.in_cell(busiest) > 1 && w.life.cover(busiest).any(), "a cell of bodies and cover");
    let cells = places(&w, busiest);
    let lost = run_splits(&mut w, cells, rounds);
    println!("the most a join sent to the ground: {lost:.3e} of its quarters' cover; bodies {} in {} cells, members {:?}", w.life.alive().count(), w.mesh.cells().n(), members(&w));
}

/// Where `climate` gives the store of the bed.
const STORE: usize = 2;
/// Which cells of a world a climate is the mean of (`climate`).
type Pick<'a> = &'a dyn Fn(&World, usize) -> bool;

/// The climate of the cells of `w` that `pick` takes, means by area: the ground's temperature, K; the liquid on and in the
/// ground and in the bed's store (`STORE`), mol/m²; the rain and what rises, mol/(m² s) (moving averages over a year).
fn climate(w: &World, pick: Pick) -> [f64; 5] {
    let cells = w.mesh.cells();
    let (mut sum, mut area) = ([0.0; 5], 0.0);
    for i in (0..cells.n()).filter(|&i| pick(w, i)) {
        let a = cells.area()[i];
        let v = [w.t_ground_last(i) * a, w.store.amounts(Class::Ground)[i], w.store.amounts(Class::Deep)[i], w.rain[i] as f64 * a, w.evap[i] as f64 * a];
        sum.iter_mut().zip(v).for_each(|(s, x)| *s += x);
        area += a;
    }
    sum.map(|s| s / area)
}

/// Whether leaf `l` lies inside one of the leaves `region` (or is one).
fn inside(region: &[Leaf], l: &Leaf) -> bool {
    region.iter().any(|r| l.x >= r.x && l.x < r.x + r.side() && l.y >= r.y && l.y < r.y + r.side())
}

/// `w` with each of the leaves `region` split `depths` depths down (`testkit::split_down`).
fn split_down(mut w: World, region: &[Leaf], depths: u8) -> World {
    testkit::split_down(&mut w, region, depths);
    w
}

/// The months an even region runs before it splits, and after.
const EVEN_MONTHS: usize = 12;
/// How far the climate of an even region split may stray from the unsplit one's, a share of each number but the store
/// of the bed (`climate`): its cells differ only by their latitude and the air they see.
const EVEN_SHARE: f64 = 1e-6;
/// The same for the store of the bed: in the year after the split the runoff over the plateau feeds some squares' stores
/// more than others again, and the store flows down those differences, which the cells of 500 m resolve better (a sharp
/// step falls over 500 m, not 2 km): 6e-5 in a year.
const STORE_SHARE: f64 = 1e-2;

/// The climate does not depend on the size of the cells where nothing in them differs (spec 5.2): on the small island
/// with all its land one even plateau (each land cell holding per m² what one does in its ground, its bed and the store
/// under it, `testkit::even_land`, with no relief inside it), made even again after a year (its runoff fed some squares'
/// stores more than others, a difference a finer grid resolves better, so the plateau would not be even below ground), a
/// block of 2 x 2 squares in its middle split two depths (into cells of 500 m, those beside it into cells of 1 km) keeps,
/// after a year, the climate (`climate`) of the block and of the island within `EVEN_SHARE` of each number
/// (`STORE_SHARE` for the store).
#[test]
fn an_even_region_split_two_depths_keeps_its_climate() {
    let mut w = world();
    let (n, like) = (w.mesh.cells().n(), testkit::dry_land(&w));
    let land: Vec<usize> = (0..n).filter(|&i| w.is_land(i)).collect();
    land.iter().for_each(|&i| w.relief[i] = 0.0);
    testkit::even_land(&mut w, like);
    (0..EVEN_MONTHS).for_each(|_| pcore::step(&mut w));
    // A year's runoff across the plateau feeds some squares' stores more than others; made even again, below ground too.
    testkit::even_land(&mut w, like);
    w.drain_events();
    // The block of 2 x 2 squares at the middle of the map, aligned to its size.
    let side = 2 * leaf(&w, 0).side();
    let block = [Leaf { x: (w.map.w as u64 / 2) / 2 * side, y: (w.map.h as u64 / 2) / 2 * side, depth: w.map.square_depth() - 1 }];
    assert!(block[0].quarters().iter().all(|&q| w.map.cell(q).is_some_and(|i| land.contains(&i))), "the block is four squares of land");
    let mut fine = split_down(w.clone(), &block, 2);
    for _ in 0..EVEN_MONTHS {
        pcore::step(&mut w);
        pcore::step(&mut fine);
    }
    let picks: [(&str, Pick); 2] = [("the block", &|w, i| inside(&block, &leaf(w, i))), ("the island", &|_, _| true)];
    for (what, pick) in picks {
        let (coarse, split) = (climate(&w, pick), climate(&fine, pick));
        let off: Vec<f64> = coarse.iter().zip(&split).map(|(a, b)| (b / a - 1.0).abs()).collect();
        println!("{what}: unsplit {coarse:?}, off by {off:?}");
        assert!(off.iter().enumerate().all(|(q, &x)| x <= if q == STORE { STORE_SHARE } else { EVEN_SHARE }), "{what}: unsplit {coarse:?}, split {split:?}");
    }
    assert_eq!(check::broken(&fine), None);
}

/// The coastal squares `the_split_coast_keeps_its_climate` splits, and the years it runs.
const COAST_SQUARES: usize = 6;
const COAST_YEARS: usize = 10;
/// How far the split run's ground temperature may stray from the unsplit one's, K, on the land and in the split region;
/// and its rain over the island, a share. The quarters are flat, so a slope becomes steps: the stores and the pores of
/// a region drain at its edges, not inside, and it holds more liquid (its store 2.3 times, its evaporation 5% more in
/// ten years), while the land and the rain move by 0.004 K and 0.06%.
const COAST_LAND_K: f64 = 0.02;
const COAST_REGION_K: f64 = 0.05;
const COAST_RAIN: f64 = 2e-3;

/// The coarse and the fine form of the same ground give the same climate (spec 5.2): the 22 x 22 island without life,
/// its first `COAST_SQUARES` coastal squares (land with a cell under 0 m across a side) in its west half split two
/// depths after a year into flat quarters (no relief inside the cells: the laws at two sizes, not a finer terrain), runs
/// `COAST_YEARS` years beside the unsplit island; the ground temperature of the land and of the region, and the rain,
/// stay within `COAST_LAND_K`, `COAST_REGION_K` and `COAST_RAIN`, and the books close.
#[test]
#[ignore = "slow: two 10-year runs of the 22 x 22 island"]
fn the_split_coast_keeps_its_climate() {
    let mut w = worldgen::new_world(1, 22, 22);
    (0..w.cfg.months_per_year).for_each(|_| pcore::step(&mut w));
    w.drain_events();
    let cells = w.mesh.cells();
    let coast = |i: usize| w.is_land(i) && cells.around(i).0.iter().zip(cells.around(i).1).any(|(&l, &j)| cells.width()[l as usize] > 0.0 && !w.is_land(j as usize));
    let west = |l: &Leaf| l.x < w.map.w as u64 / 2 * l.side();
    let region: Vec<Leaf> = (0..cells.n()).filter(|&i| coast(i)).map(|i| leaf(&w, i)).filter(|l| l.depth == w.map.square_depth() && west(l)).take(COAST_SQUARES).collect();
    let mut flat = w.clone();
    flat.relief.iter_mut().for_each(|r| *r = 0.0);
    let mut fine = split_down(flat, &region, 2);
    let books = Books::now(&fine);
    for _ in 0..COAST_YEARS * w.cfg.months_per_year as usize {
        pcore::step(&mut w);
        pcore::step(&mut fine);
        w.drain_events();
        fine.drain_events();
    }
    let (land, near): (Pick, Pick) = (&|w, i| w.is_land(i), &|w, i| inside(&region, &leaf(w, i)));
    let [a, b, ra, rb, all_a, all_b] = [(&w, land), (&fine, land), (&w, near), (&fine, near), (&w, &|_, _| true), (&fine, &|_, _| true)].map(|(w, pick)| climate(w, pick));
    println!("land {:+.4} K, the region {:+.4} K, rain {:+.3e}, the region's store {:+.3e} and what rises {:+.3e}", b[0] - a[0], rb[0] - ra[0], all_b[3] / all_a[3] - 1.0, rb[STORE] / ra[STORE] - 1.0, rb[4] / ra[4] - 1.0);
    assert!((b[0] - a[0]).abs() < COAST_LAND_K, "the land's ground {} K against {} K", b[0], a[0]);
    assert!((rb[0] - ra[0]).abs() < COAST_REGION_K, "the region's ground {} K against {} K", rb[0], ra[0]);
    assert!((all_b[3] / all_a[3] - 1.0).abs() < COAST_RAIN, "rain {} against {}", all_b[3], all_a[3]);
    books.assert_kept(&fine, check::DRIFT);
}
