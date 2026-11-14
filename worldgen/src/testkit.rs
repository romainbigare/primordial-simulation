//! What the tests of the laws share (the `testkit` feature, for dev-dependencies only): a small world, edits booked from
//! outside (cover and takers among them), the books, the substances of chemistry 0 by number, and a plain taker on the
//! contract (`taker`).

mod taker;
pub use taker::Taker;

use pcore::check::{self, Totals};
use pcore::chem::{Chemistry, Phase};
use pcore::store::{Boundary, Class, Node};
use pcore::{Config, World};

/// An island of `w × h` squares from a seed, each cell a square (`terrain::island`): for the tests that read the map's
/// lattice.
pub fn squares(seed: u64, w: usize, h: usize) -> World {
    crate::terrain::new_map(seed, crate::terrain::island(w, h))
}

/// An island of one rise in the middle of its map (`Shape::One`) of `w × h` squares from a seed, its low ground away
/// from land in coarse cells (`terrain::new_island`): the world the tests of the laws stand on, as `ew run --shape one`
/// makes it.
pub fn one_rise(seed: u64, w: usize, h: usize) -> World {
    crate::terrain::new_island(seed, (w, h), pcore::space::terrain::Shape::One, Config::default())
}

/// The sides, in cells, of the islands `island_with` tries, smallest first.
const ISLAND_SIDES: [usize; 4] = [16, 24, 32, 48];

/// The smallest island of seed `seed` (`squares`, of `ISLAND_SIDES` cells a side) whose ground has what a scene needs,
/// with what `has` found there: a scene states the ground it needs as a condition, and the island grows until its
/// ground holds it.
pub fn island_with<T>(seed: u64, has: impl Fn(&World) -> Option<T>) -> (World, T) {
    island_in_with(seed, Config::default(), has)
}

/// `island_with` for an island of the settings `cfg` (its latitudes).
pub fn island_in_with<T>(seed: u64, cfg: Config, has: impl Fn(&World) -> Option<T>) -> (World, T) {
    ISLAND_SIDES.iter().find_map(|&side| {
        let (chem, init) = crate::chemistry0();
        let w = crate::terrain::build(seed, crate::terrain::island(side, side), chem, &init, cfg.clone());
        has(&w).map(|found| (w, found))
    }).expect("an island whose ground has what the scene needs")
}

/// Substances of chemistry 0: the common liquid.
pub const LIQUID: usize = 0;
/// The gas the fixing reaction (10) takes from the air.
pub const FIX_GAS: usize = 1;
/// The gas the energy-rich solid burns with.
pub const OX: usize = 2;
/// The heat-absorbing gas.
pub const ABSORBER: usize = 3;
/// A gas that burns with `OX` (reaction 4).
pub const BURNING_GAS: usize = 4;
/// The energy-rich solid the light reaction makes.
pub const RICH: usize = 5;
/// The rock that dissolves in the liquid with the absorbing gas (reactions 8 and 9).
pub const ROCK: usize = 9;
/// Body matter.
pub const MATTER: usize = 10;
/// The mineral that exists only dissolved in the liquid, its store of the absorbing gas.
pub const DISSOLVED: usize = 11;
/// The mineral of the fixed gas.
pub const MINERAL: usize = 12;
/// The frame, a solid bodies build.
pub const FRAME: usize = 13;
/// The reaction that burns the energy-rich solid with `OX`.
pub const BURN: usize = 1;
/// The reactions that dissolve `ROCK` with the heat-absorbing gas and the liquid into `DISSOLVED`, and settle it back.
pub const DISSOLVING: usize = 8;
pub const SETTLING: usize = 9;
/// The element of `ROCK` (its cation).
pub const ROCK_ELEMENT: usize = 5;
/// The reaction that breaks `MINERAL` down when hot (its fixed element goes back to the air as `FIX_GAS`).
pub const MINERAL_BREAKS: usize = 11;

/// Latitudes of the south and north edges of the world most tests stand in, degrees: a low latitude whose first winter
/// is mild (about 290 K), so the tests' bodies live through it; real runs place the island by `Config::lat_south`.
pub const MILD_LAT: [f64; 2] = [31.0, 33.3];

/// The world most tests stand in: seed 1, 16 x 16 cells at `MILD_LAT`, after one month (so its cells have their
/// surroundings).
pub fn small_world() -> World {
    mild_world(ISLAND_SIDES[0])
}

/// The world of `small_world` with `side` x `side` cells.
fn mild_world(side: usize) -> World {
    let (chem, init) = crate::chemistry0();
    let cfg = Config { lat_south: MILD_LAT[0], lat_north: MILD_LAT[1], ..Default::default() };
    let mut w = crate::terrain::build(1, crate::terrain::island(side, side), chem, &init, cfg);
    pcore::step(&mut w);
    w
}

/// An island of `side` x `side` cells of seed 1 with the kind of `ew run` placed at its start as `ew run` places it
/// (the soil, the light-eaters: `life::seed`), after `months` months.
pub fn living(side: usize, months: u32) -> World {
    let mut w = one_rise(1, side, side);
    crate::life::soil(&mut w, &crate::chemistry0().1.soil);
    _ = crate::life::seed(&mut w, crate::life::LIGHT_ROW);
    (0..months).for_each(|_| pcore::step(&mut w));
    w
}

/// The land cells with no liquid standing on them, their lowest ground above 0 m (no low ground the low basin fills).
pub fn dry_lands(w: &World) -> Vec<usize> {
    (0..w.mesh.cells().n()).filter(|&i| w.is_land(i) && w.life.env[i][1] < 1e-3 && pcore::flow::lowest(w, i) > 0.0).collect()
}

/// The first land cell with no liquid standing on it.
pub fn dry_land(w: &World) -> usize {
    dry_lands(w).first().copied().expect("dry land")
}

/// Of `cells`, the one of the mildest climate: the least height.
fn lowest(w: &World, cells: impl Iterator<Item = usize>) -> Option<usize> {
    cells.min_by(|&a, &b| w.surface_z(a).total_cmp(&w.surface_z(b)))
}

/// The lowest dry land cell (`dry_lands`): of the dry land, the one of the mildest climate.
pub fn mild_land(w: &World) -> usize {
    lowest(w, dry_lands(w).into_iter()).expect("dry land")
}

/// The lowest dry land cell whose neighbours are all dry land: inland, no standing liquid beside it, and of the inland
/// cells the one of the mildest climate, as the tests of bodies want.
pub fn inland(w: &World) -> usize {
    let dry = dry_lands(w);
    lowest(w, dry.iter().copied().filter(|&i| w.mesh.cells().around(i).1.iter().all(|&j| dry.contains(&(j as usize))))).expect("an inland cell")
}

/// `small_world` and its first dry land cell.
pub fn start() -> (World, usize) {
    let w = small_world();
    let i = dry_land(&w);
    (w, i)
}

/// `kg` per m² of the cover of the light-eaters (their genes and label, `life::LIGHT_ROW`) placed in cell `i`
/// (`cover::place`).
pub fn cover(w: &mut World, i: usize, kg: f64) {
    assert!(pcore::cover::place(w, i, crate::life::KINDS[crate::life::LIGHT_ROW].genes, crate::life::LIGHT_ROW as u32, kg), "no room for cover in cell {i}");
}

/// `members` takers of the starting kind of row `row` of `life::TAKER_KINDS` placed in cell `i`, each with `store` kg of
/// fuel (`fauna::place`).
pub fn takers(w: &mut World, i: usize, row: usize, (members, store): (f64, f64)) {
    let genes = crate::life::TAKER_KINDS[row].genes;
    assert!(pcore::fauna::place(w, i, (genes, row as u32), (members, store)), "no room for takers in cell {i}");
}

/// The takers' month of cell `i` alone, its surroundings as they stand (`fauna::step` on a chunk of that cell).
pub fn takers_month(w: &mut World, i: usize) {
    w.in_cell(i, pcore::fauna::step);
}

/// `mol` of substance `k` put on the ground of cell `i` at its temperature (booked as an edit).
pub fn put(w: &mut World, k: usize, i: usize, mol: f64) {
    let t = w.t_ground(i);
    w.cross((Boundary::Outside, Node::ground(i)), [(k, mol)], 0.0);
    hold_ground(w, i, t);
}

/// `kg` per m² of substance `k` put on the ground of cell `i` at its temperature (booked as an edit).
pub fn put_kg_m2(w: &mut World, k: usize, i: usize, kg: f64) {
    let mol = kg * w.mesh.cells().area()[i] / w.chem.subs[k].molar_mass;
    put(w, k, i, mol);
}

/// All of substance `k` in the world's ground and the store of its bed, mol.
pub fn in_all(w: &World, k: usize) -> f64 {
    let n = w.mesh.cells().n();
    w.store.amounts(Class::Ground)[k * n..(k + 1) * n].iter().sum::<f64>() + w.store.amounts(Class::Deep).get(k * n..(k + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>())
}

/// Thickness of rock-making solids (those that melt above 1000 K, but body matter, which chars before it melts) in the
/// loose ground of cell `i` and the bed below it, m, and the mean softness (1 / (hardness + `Config::hard_min`)) of the loose ones, by
/// volume.
pub fn ground_of(w: &World, i: usize) -> (f64, f64, f64) {
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[i]);
    let (mut loose, mut bed, mut soft) = (0.0, 0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.chem.subs[k].melt > 1000.0 && Some(k) != w.chem.matter()) {
        let v = w.chem.molar_volume(k, Phase::Solid);
        loose += w.store.amounts(Class::Ground)[k * n + i] * v / area;
        bed += w.store.amounts(Class::Bed)[k * n + i] * v / area;
        soft += w.store.amounts(Class::Ground)[k * n + i] * v / area / (w.chem.subs[k].hardness + w.cfg.hard_min);
    }
    (loose, bed, soft / loose.max(1e-300))
}

/// The ground of cell `i` brought to `t` K (booked as an edit), all of it: a mixed cell's land and liquid alike
/// (`World::even_ground`).
pub fn hold_ground(w: &mut World, i: usize, t: f64) {
    let e = w.energy_ground(i, t);
    set_heat(w, Node::ground(i), e);
    w.even_ground(i, t);
}

/// The heat of node `x` of the store set to `e` J (booked as an edit: all it held out, then `e` in, so it holds `e`
/// exactly).
pub fn set_heat(w: &mut World, x: Node, e: f64) {
    w.cross((Boundary::Outside, x), [], -w.store.heat(x.class)[x.index]);
    w.cross((Boundary::Outside, x), [], e);
}

/// Ground node `i` and the bed under it emptied of every substance (booked as an edit; its heat stays).
pub fn empty_cell(w: &mut World, i: usize) {
    for k in 0..w.chem.ns() {
        set_amount(w, Node::ground(i), k, 0.0);
        set_amount(w, Node::new(Class::Bed, i), k, 0.0);
    }
}

/// The amount of substance `k` in node `x` of the store set to `mol` (booked as an edit, as `set_heat`, with no heat:
/// set it after).
pub fn set_amount(w: &mut World, x: Node, k: usize, mol: f64) {
    let at = k * w.store.amounts(x.class).len() / w.chem.ns() + x.index;
    w.cross((Boundary::Outside, x), [(k, -w.store.amounts(x.class)[at])], 0.0);
    w.cross((Boundary::Outside, x), [(k, mol)], 0.0);
}

/// Cell `i` flat inside: its ground level at its mean (`World::level_inside`).
pub fn flatten(w: &mut World, i: usize) {
    w.level_inside(i);
}

/// Cell `i` and the cells round it made flat land at its height, with its liquid and soil: all flat inside (`flatten`),
/// and each round it holding per m² what `i` holds in its loose ground, its bed and the store under it, and their heat
/// per m² (booked as edits); then Flow's slow ground is rechecked and the state of the ground found again, so each
/// stands as one node (flat inside, it is all land or all under its liquid, laws.md 1.1): no liquid runs off a slope
/// into or out of it sideways.
pub fn flat_round(w: &mut World, i: usize) {
    let (cells, ns) = (w.mesh.cells(), w.chem.ns());
    let round: Vec<usize> = cells.around(i).1.iter().map(|&j| j as usize).collect();
    let area = cells.area().to_vec();
    std::iter::once(i).chain(round.iter().copied()).for_each(|j| flatten(w, j));
    for j in round {
        let f = area[j] / area[i];
        for class in [Class::Ground, Class::Bed, Class::Deep] {
            let held = w.store.amounts(class);
            if held.is_empty() {
                continue;
            }
            let per = held.len() / ns;
            let want: Vec<f64> = (0..ns).map(|k| f * held[k * per + i]).collect();
            want.into_iter().enumerate().for_each(|(k, mol)| set_amount(w, Node::new(class, j), k, mol));
            if let Some(&e) = w.store.heat(class).get(i) {
                set_heat(w, Node::new(class, j), f * e);
            }
        }
    }
    pcore::flow::refresh(w);
    pcore::flow::state(w, true);
}

/// Every part of Flow but creep once, for `dt` seconds, in the clock's order: the state rebuilt in full, the air's step
/// with the sink into the bed's store, liquid, and the refresh; then the state the step leaves. A law of Flow on its own.
pub fn flow_step(w: &mut World, dt: f64) {
    pcore::flow::state(w, true);
    pcore::flow::air(w, dt);
    pcore::flow::liquid(w, dt);
    pcore::flow::refresh(w);
    pcore::flow::state(w, false);
}

/// Heat conducted sideways through the ground and creep for `dt` seconds, from the state of the world now: the yearly
/// part of Flow.
pub fn creep(w: &mut World, dt: f64) {
    pcore::flow::state(w, false);
    pcore::flow::creep(w, dt);
}

/// The volume of the loose solids of cell `i`, m³.
pub fn loose_volume(w: &World, i: usize) -> f64 {
    let t = w.t_ground(i);
    (0..w.chem.ns()).filter(|&k| w.chem.phase(k, t) == Phase::Solid).map(|k| w.g(k, i) * w.chem.molar_volume(k, Phase::Solid)).sum()
}

/// The books of a world at a moment: its totals and its boundary nodes' (`check::totals`).
pub struct Books(Totals);

impl Books {
    pub fn now(w: &World) -> Self {
        Books(check::totals(w))
    }

    /// The worst relative drift of elements and of energy since then.
    pub fn drift(&self, w: &World) -> (f64, f64) {
        check::drift(w, &self.0)
    }

    /// Nothing is broken, and elements and energy are kept since then within `tol`.
    pub fn assert_kept(&self, w: &World, tol: f64) {
        self.assert_within(w, tol, tol);
    }

    /// Nothing is broken, elements are kept within `elems` and energy within `energy`.
    pub fn assert_within(&self, w: &World, elems: f64, energy: f64) {
        assert_eq!(check::broken(w), None);
        let (de, dq) = self.drift(w);
        assert!(de < elems && dq < energy, "element drift {de:e}, energy drift {dq:e}");
    }
}

/// Elements and energy (heat and formation enthalpy) of one reacting node with its gas.
pub fn node_totals(chem: &Chemistry, n: &[f64], gas: &[f64], e: f64) -> (Vec<f64>, f64) {
    let mut el = vec![0.0; chem.n_elements];
    let mut en = e;
    for k in 0..chem.ns() {
        chem.add_elems(k, n[k] + gas[k], &mut el);
        en += (n[k] + gas[k]) * chem.subs[k].hf;
    }
    (el, en)
}

/// The air over 1 m² at the reference pressure, mol (its weight over g, at the mean molar mass of the starting air).
pub const AIR_COLUMN: f64 = 101_325.0 / (9.81 * 0.0288);

/// The heat capacity of an open-air node, J/K: so large that its temperature stays put.
pub const OPEN_CAP: f64 = 1e15;

/// An open-air node at `t` K: no substance, the starting air in plenty (10^9 times its mole fractions), and its heat
/// at the capacity `OPEN_CAP`. Returns the amounts, the gas and the heat.
pub fn open_air(chem: &Chemistry, init: &crate::Init, t: f64) -> (Vec<f64>, Vec<f64>, f64) {
    let mut gas = vec![0.0; chem.ns()];
    for &(g, x) in &init.air {
        gas[g] = x * 1e9;
    }
    (vec![0.0; chem.ns()], gas, t * OPEN_CAP)
}
