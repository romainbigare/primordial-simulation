//! What the tests of the laws share (the `testkit` feature, for dev-dependencies only): a small world, founders of a
//! chosen brain, edits booked from outside, the books, and the substances of chemistry 0 by number.

use pcore::body::{Body, Layout, BRAIN_SIZES, CHILD, COPY_MASS, DISPERSE, STRENGTH, WET_TISSUE};
use pcore::bond;
use pcore::brain::{self, FIXED, OVERBS, VERBS};
use pcore::check::{self, Totals};
use pcore::chem::{Chemistry, Phase};
use pcore::event::EventKind;
use pcore::life::{Chunk, Handle};
use pcore::lineage::{self, Lineage};
use pcore::rng::{Purpose, Rng};
use pcore::store::{Boundary, Class, Move, Node};
use pcore::units::DAY_S;
use pcore::{Config, World};

/// An island of `w × h` squares from a seed, each cell a square (`terrain::island`): for the tests that read the map's
/// lattice.
pub fn squares(seed: u64, w: usize, h: usize) -> World {
    crate::terrain::new_map(seed, crate::terrain::island(w, h))
}

/// The sides, in cells, of the islands `island_with` tries, smallest first.
const ISLAND_SIDES: [usize; 4] = [16, 24, 32, 48];

/// The smallest island of seed `seed` (`squares`, of `ISLAND_SIDES` cells a side) whose ground has what a scene needs,
/// with what `has` found there: a scene states the ground it needs as a condition, and the island grows until its
/// ground holds it.
pub fn island_with<T>(seed: u64, has: impl Fn(&World) -> Option<T>) -> (World, T) {
    ISLAND_SIDES.iter().find_map(|&side| {
        let w = squares(seed, side, side);
        has(&w).map(|found| (w, found))
    }).expect("an island whose ground has what the scene needs")
}

/// Body `b` as the node an edit changes, named by its cell (its own end of the moves of an edit, which it books itself;
/// `World::cross`).
fn edited(b: &Body) -> Node {
    Node::new(Class::Body, b.cell as usize)
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

/// The smallest world of `small_world`'s kind (of `ISLAND_SIDES` cells a side) whose ground has what a scene needs, with
/// what `has` found there (`island_with`).
pub fn mild_with<T>(has: impl Fn(&World) -> Option<T>) -> (World, T) {
    ISLAND_SIDES.iter().find_map(|&side| {
        let w = mild_world(side);
        has(&w).map(|found| (w, found))
    }).expect("a world whose ground has what the scene needs")
}

/// An island of `side` x `side` cells of seed 1 with the life of `ew run` placed at its start (`life`), after `months`
/// months.
pub fn living(side: usize, months: u32) -> World {
    let mut w = crate::new_world(1, side, side);
    life(&mut w, &crate::life::TWO);
    (0..months).for_each(|_| pcore::step(&mut w));
    w
}

/// The start of life of a world, every kind at once: the soil, the cover the nursery sorts (`life::nursery`), then the
/// bodies of the starting kinds `rows` (`life::seed`), as `Schedule` places them when life starts, the later kinds
/// without their years.
pub fn life(w: &mut World, rows: &[usize]) {
    crate::life::soil(w, &crate::chemistry0().1.soil);
    crate::life::nursery::start(w);
    for &k in rows.iter().filter(|&&k| matches!(crate::life::KINDS[k].place, crate::life::Place::Bodies(_))) {
        crate::life::seed(w, k);
    }
}

/// The light-eaters of a scene that tests other laws, kg per m² (`light_eaters`).
pub const SCENE_COVER_KG: f64 = 0.1;

/// The light-eaters of a scene that tests other laws: a plain, explicit placing of the genes of starting kind 0,
/// `SCENE_COVER_KG` per m² on every dry land cell (`cover_lands`), the cells surveyed first. A world's own cover is the
/// nursery's (`life`).
pub fn light_eaters(w: &mut World) {
    pcore::survey::survey(w);
    cover_lands(w, SCENE_COVER_KG);
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

/// The bias that turns an action of a brain on, and off.
pub const ON: f32 = 8.0;
pub const OFF: f32 = -8.0;
/// Hidden units and memory of the founders' brains, and the size of their random weights.
const HIDDEN: usize = 12;
const MEMORY: usize = 6;
const WEIGHTS: f32 = 0.1;

/// A brain to found a lineage with: the biases of its fixed actions, of its move and of its verbs, its hidden units,
/// and the size and the seed of its random weights.
#[derive(Clone, Copy, Debug)]
pub struct Brain {
    pub fixed: [f32; FIXED],
    pub dir: f32,
    pub verbs: [f32; VERBS + OVERBS],
    pub hidden: usize,
    pub weights: f32,
    pub seed: u64,
}

impl Brain {
    /// A brain that only does fixed action `act`.
    pub fn only(act: usize) -> Self {
        let mut fixed = [OFF; FIXED];
        fixed[act] = ON;
        Brain { fixed, dir: OFF, verbs: [OFF; VERBS + OVERBS], hidden: HIDDEN, weights: WEIGHTS, seed: 1 }
    }

    /// A brain that only does verb `verb` (`brain::GIVE`, ..., or `brain::VERBS + k` for the verbs on parts).
    pub fn verb(verb: usize) -> Self {
        let mut verbs = [OFF; VERBS + OVERBS];
        verbs[verb] = ON;
        Brain { fixed: [OFF; FIXED], dir: OFF, verbs, hidden: HIDDEN, weights: WEIGHTS, seed: 1 }
    }

    /// Every bias 0: nothing preferred.
    pub fn neutral() -> Self {
        Brain { fixed: [0.0; FIXED], dir: 0.0, verbs: [0.0; VERBS + OVERBS], hidden: HIDDEN, weights: WEIGHTS, seed: 1 }
    }
}

/// A lineage made for a test, and the born rows of its brain.
pub struct Line {
    pub lineage: u32,
    pub rows: Vec<f32>,
}

impl Line {
    /// A new lineage of `brain`, labelled with starting kind `root`.
    pub fn new(w: &mut World, brain: &Brain, root: u32) -> Line {
        let shape = lineage::shape(w.chem.ns(), brain.hidden, MEMORY);
        let mut base = brain::random(shape, &mut Rng::new(brain.seed, 0, 0, Purpose::Brain), brain.weights, brain.fixed, brain.dir, brain.verbs);
        let rows = base.split_off(shape.base());
        let lineage = w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root });
        Line { lineage, rows }
    }

    /// A founder of starting kind `kind` in cell `i`, of this lineage and its brain sizes, booked as an edit; not placed.
    pub fn founder(&self, w: &mut World, kind: usize, i: usize) -> Body {
        let genome = w.life.new_genome(self.rows.clone());
        let mut b = crate::life::make(w, kind, i, self.lineage, genome).expect("a founder");
        let sizes = w.life.brains[self.lineage as usize].as_ref().map(lineage::sizes_of).expect("the lineage");
        BRAIN_SIZES.iter().zip(sizes).for_each(|(&g, x)| b.genes[g] = x as f32);
        b.prepare(&w.chem, &w.cfg);
        b
    }
}

/// A founder of starting kind `kind` in cell `i`, of a lineage of its own with `brain`.
pub fn founder(w: &mut World, kind: usize, i: usize, brain: &Brain) -> Body {
    Line::new(w, brain, kind as u32).founder(w, kind, i)
}

/// The mass the tests' starting eaters are scaled to, kg (9 of fuel, 15.75 of liquid, 5.25 of body matter).
pub const EATER_KG: f64 = 30.0;

/// A starting eater of `EATER_KG` in cell `i`, of a brain that only does fixed action `act`, weak (strength 0), staying
/// where it is, soaking no liquid up, mature at 10^`copy` kg of body.
pub fn eater(w: &mut World, i: usize, act: usize, copy: f32) -> Body {
    eater_of(w, i, &Brain::only(act), copy)
}

/// An eater (`eater`) of brain `brain`.
pub fn eater_of(w: &mut World, i: usize, brain: &Brain, copy: f32) -> Body {
    let mut b = founder(w, 1, i, brain);
    let m = b.mass(&w.chem);
    scale(w, &mut b, EATER_KG / m);
    (b.genes[DISPERSE], b.genes[STRENGTH], b.genes[COPY_MASS]) = (0.0, 0.0, copy);
    b.prepare(&w.chem, &w.cfg);
    b
}

/// Eater nodes of `members` members each (starting kind 1, with no brain: they decide nothing), one on each dry land
/// cell, placed: the nodes the tests of population nodes run on. Returns where they are stored.
pub fn eater_nodes(w: &mut World, members: f64) -> Vec<Handle> {
    let line = Line::new(w, &Brain { hidden: 0, ..Brain::only(brain::REST) }, 1);
    dry_lands(w).into_iter().map(|i| {
        let b = line.founder(w, 1, i);
        let b = node_of(w, b, members);
        w.add(b)
    }).collect()
}

/// `kg` per m² of the cover of starting kind 0 (its genes, `life::KINDS`) placed in cell `i` (`cover::place`).
pub fn cover(w: &mut World, i: usize, kg: f64) {
    let crate::life::Place::Field(f) = &crate::life::KINDS[0].place else { unreachable!("kind 0 is cover") };
    assert!(pcore::cover::place(w, i, f.genes, 0, kg), "no room for cover in cell {i}");
}

/// `kg` per m² of the cover of starting kind 0 on every dry land cell (`cover`; the cells' surroundings from the last
/// survey): what eaters placed then find to take from their first month.
pub fn cover_lands(w: &mut World, kg: f64) {
    for i in dry_lands(w) {
        cover(w, i, kg);
    }
}

/// Body `b` made `f` times as big: its stores and heat (booked as an edit).
pub fn scale(w: &mut World, b: &mut Body, f: f64) {
    let more: Vec<(usize, f64)> = pcore::body::each(&w.chem, &b.store).map(|(k, a)| (k, (f - 1.0) * a)).collect();
    w.cross((Boundary::Outside, edited(b)), more, (f - 1.0) * b.e);
    b.store.iter_mut().for_each(|a| *a *= f);
    b.e *= f;
}

/// `kg` of substance `k` added to body `b` at its temperature (booked as an edit), or taken out if negative.
pub fn add(w: &mut World, b: &mut Body, k: usize, kg: f64) {
    let (a, t, e0) = (kg / w.chem.subs[k].molar_mass, b.temp(&w.chem), b.e);
    *b.store_of(&w.chem, k) += a;
    b.e = b.energy_at(&w.chem, t);
    w.cross((Boundary::Outside, edited(b)), [(k, a)], b.e - e0);
}

/// What body `b` holds, mol of each substance.
pub fn stores(w: &World, b: &Body) -> Vec<f64> {
    (0..w.chem.ns()).map(|k| b.amount(w.chem.body(), k)).collect()
}

/// What body `b` holds of substance `k`, kg.
pub fn kg(w: &World, b: &Body, k: usize) -> f64 {
    b.amount(w.chem.body(), k) * w.chem.subs[k].molar_mass
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

/// All of substance `k` in the world's ground, the store of its bed and its bodies, mol.
pub fn in_all(w: &World, k: usize) -> f64 {
    let n = w.mesh.cells().n();
    w.store.amounts(Class::Ground)[k * n..(k + 1) * n].iter().sum::<f64>() + w.store.amounts(Class::Deep).get(k * n..(k + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>()) + w.life.alive().map(|b| b.store[k]).sum::<f64>()
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

/// One day of `pcore::day_with_month_at_start` with the ground of `cells` held at `t` K: before it, and after each pass of a month that
/// starts on it, with Flow's state rebuilt from it, so the laws after each pass read that ground (a test of a body's
/// law, not of the climate).
pub fn held_day(w: &mut World, cells: &[usize], t: f64) {
    let hold = |w: &mut World| {
        cells.iter().for_each(|&i| hold_ground(w, i, t));
        pcore::flow::state(w, true);
    };
    hold(w);
    pcore::day_with_month_at_start_marked(w, &mut |w, _| hold(w));
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

/// Every land cell of `w` made to hold what cell `like` holds per m², in its ground, its bed and the store under it, with
/// their heat (booked as edits, `set_amount`, `set_heat`): an even plateau, the same above and below ground.
pub fn even_land(w: &mut World, like: usize) {
    let n = w.mesh.cells().n();
    let (land, area): (Vec<usize>, Vec<f64>) = ((0..n).filter(|&i| w.is_land(i)).collect(), w.mesh.cells().area().to_vec());
    for &i in &land {
        for class in [Class::Ground, Class::Bed, Class::Deep] {
            for k in 0..w.chem.ns() {
                let mol = w.store.amounts(class)[k * n + like] * area[i] / area[like];
                set_amount(w, Node::new(class, i), k, mol);
            }
            if let Some(&e) = w.store.heat(class).get(like) {
                set_heat(w, Node::new(class, i), e * area[i] / area[like]);
            }
        }
    }
}

/// The store under cell `i` of `w` (`Class::Deep`) made to hold the share `share` of its room (`Physics::bed_porosity`
/// × `Physics::bed_depth`) of the common liquid, its heat in proportion (booked as edits); it must hold some.
pub fn fill_store(w: &mut World, i: usize, share: f64) {
    let n = w.mesh.cells().n();
    let room = w.cfg.phys.bed_porosity * w.cfg.phys.bed_depth * w.mesh.cells().area()[i] / w.chem.molar_volume(LIQUID, Phase::Liquid);
    let (was, heat) = (w.store.amounts(Class::Deep)[LIQUID * n + i], w.store.heat(Class::Deep)[i]);
    assert!(was > 0.0, "the store under cell {i} holds none");
    set_amount(w, Node::new(Class::Deep, i), LIQUID, share * room);
    set_heat(w, Node::new(Class::Deep, i), heat * share * room / was);
}

/// Body `b` brought to `t` K (booked as an edit).
pub fn warm(w: &mut World, b: &mut Body, t: f64) {
    let e = b.energy_at(&w.chem, t);
    w.cross((Boundary::Outside, edited(b)), [], e - b.e);
    b.e = e;
}

/// Body `b` placed and taken out again, prepared, to call the laws on directly: where it is stored, and the body.
pub fn out(w: &mut World, b: Body) -> (Handle, Body) {
    let s = w.add(b);
    pcore::survey::survey(w);
    w.time += DAY_S;
    let mut b = std::mem::take(&mut w.life[s]);
    b.prepare(&w.chem, &w.cfg);
    (s, b)
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

/// Body `b` makes the children its brood covers, all born now (`copy::copy`, of a brood no update filled). Returns
/// whether it made any.
pub fn copy(w: &World, d: &mut Chunk, b: &mut Body) -> bool {
    pcore::copy::copy(w, d, b, pcore::copy::Fill { before: b.brood, filling: 0.0, days: 0.0, at: w.time })
}

/// Body `b` grows for `days` days at its temperature (`copy::grow`). Returns whether it built anything.
pub fn grow(w: &World, d: &mut Chunk, b: &mut Body, days: f64) -> bool {
    let t = b.temp(&w.chem);
    pcore::copy::grow(w, d, b, days, t).built
}

/// Energy body `b`'s fuel can release at its temperature now, J (the most `body::pay` could pay).
pub fn release(w: &World, b: &Body) -> f64 {
    b.fuel(&w.chem, b.temp(&w.chem))
}

/// Heat conducted sideways through the ground and creep for `dt` seconds, from the state of the world now: the yearly
/// part of Flow.
pub fn creep(w: &mut World, dt: f64) {
    pcore::flow::state(w, false);
    pcore::flow::creep(w, dt);
}

/// A node of `n` members of the body `b` (stores and heat of all of them; booked as an edit).
pub fn node_of(w: &mut World, mut b: Body, n: f64) -> Body {
    scale(w, &mut b, n);
    b.count = n;
    b
}

/// Totals over all live bodies: members, mol of each substance, heat.
pub fn life_totals(w: &World) -> (f64, Vec<f64>, f64) {
    let (mut n, mut store, mut e) = (0.0, vec![0.0; w.chem.ns()], 0.0);
    for b in w.life.alive() {
        n += b.n();
        e += b.e;
        pcore::body::each(&w.chem, &b.store).for_each(|(k, x)| store[k] += x);
    }
    (n, store, e)
}

/// The stores and heat of two `life_totals` are the same (a copy makes members, so they are not compared).
pub fn same_totals(a: &(f64, Vec<f64>, f64), b: &(f64, Vec<f64>, f64)) {
    assert!((a.2 - b.2).abs() <= 1e-9 * a.2.abs().max(1.0), "heat {} then {}", a.2, b.2);
    for (k, (x, y)) in a.1.iter().zip(&b.1).enumerate() {
        assert!((x - y).abs() <= 1e-9 * x.abs().max(1.0), "store {k}: {x} then {y}");
    }
}

/// The body at `a` bites the body at `v` once (`choose::take_body`) over its cell's step, out of its slot meanwhile.
pub fn bite(w: &mut World, a: Handle, v: Handle) {
    let mut b = std::mem::take(&mut w.life[a]);
    let r = &mut Rng::new(1, w.day(), 7, Purpose::Choose);
    let days = w.step_days();
    w.in_cell(b.cell as usize, |w, d| pcore::choose::take_body(w, d, &mut b, (v, days), r));
    w.life[a] = b;
}

/// Body `b` placed and prepared where it is: where it is stored.
pub fn place(w: &mut World, b: Body) -> Handle {
    let (s, b) = out(w, b);
    w.life[s] = b;
    s
}

/// The brood of `k` children ready for body `b` (all its members together), of the matter a child is made of: a node
/// holds `copy::BROOD_HELD` children a member besides.
pub fn brood(w: &World, b: &mut Body, k: f64) {
    let held = if b.n() > 1.0 { pcore::copy::BROOD_HELD * b.n() } else { 0.0 };
    b.brood = (k + held) * b.dec(CHILD) * b.dec(COPY_MASS) * (1.0 - WET_TISSUE) / w.chem.subs[MATTER].molar_mass;
}

/// The live children of the body of id `id`.
pub fn children(w: &World, id: u64) -> Vec<Handle> {
    w.life.living().filter(|x| x.1.parent == id).map(|x| x.0).collect()
}

/// An eater that burns only the energy-rich solid: no catalyst for burning body matter, no roots for it.
pub fn only_the_solid(w: &World, b: &mut Body) {
    let lay = Layout::of(&w.chem);
    let (m, burn_matter) = w.chem.matter_roles().expect("body matter");
    let (diet, m) = (lay.diet(&w.chem, burn_matter).expect("a diet gene"), w.chem.body().place(m).expect("a store"));
    (b.genes[diet], b.genes[lay.upt + m]) = (-1.0, -0.1);
    b.prepare(&w.chem, &w.cfg);
}

/// The volume of the loose solids of cell `i`, m³.
pub fn loose_volume(w: &World, i: usize) -> f64 {
    let t = w.t_ground(i);
    (0..w.chem.ns()).filter(|&k| w.chem.phase(k, t) == Phase::Solid).map(|k| w.g(k, i) * w.chem.molar_volume(k, Phase::Solid)).sum()
}

/// A part of `kg` of the cell's most plentiful loose substance (of those solid at its ground's temperature: walls are
/// solid), taken from the ground at that temperature: flat, or a block if `thick`.
pub fn plate(w: &mut World, i: usize, kg: f64, thick: bool) -> u64 {
    let t = w.t_ground_last(i);
    let k = (0..w.chem.ns()).filter(|&k| w.chem.phase(k, t) == Phase::Solid).max_by(|&a, &b| w.g(a, i).total_cmp(&w.g(b, i))).expect("a solid in the ground");
    let mut wall = vec![0.0; w.chem.ns()];
    wall[k] = kg / w.chem.subs[k].molar_mass;
    part_of(w, i, wall, vec![0.0; w.chem.ns()], if thick { 0.0 } else { -1.0 }, 0.0)
}

/// A part of walls `wall` holding `fill` in its hollow (mol of each substance), taken from the ground of cell `i` with
/// their heat at its temperature, of elongation `x` and hollowness `h` (its structure's numbers count its walls only,
/// `bond::make`, until it is restructured). Returns its id.
pub fn part_of(w: &mut World, i: usize, wall: Vec<f64>, fill: Vec<f64>, x: f32, h: f32) -> u64 {
    let (n, t) = (w.mesh.cells().n(), w.t_ground_last(i));
    let moved: Vec<(usize, f64, f64)> = (0..w.chem.ns()).filter(|&k| wall[k] + fill[k] > 0.0).map(|k| (k, wall[k] + fill[k], (wall[k] + fill[k]) * w.chem.subs[k].cp * t)).collect();
    for &(k, mol, _) in &moved {
        assert!(w.store.amounts(Class::Ground)[k * n + i] > mol, "the cell holds less than the part");
    }
    let heat = moved.iter().map(|m| m.2).sum();
    let s = w.in_cell(i, |w, d| {
        let p = bond::make(w, d, i, wall, heat, x, h);
        d.at_mut(i).parts[p].fill = fill;
        p
    });
    let moves: Vec<Move> = moved.into_iter().map(|(k, mol, heat)| Move { from: Node::ground(i), to: Node::new(Class::Part, s), k: k as u16, mol, heat }).collect();
    pcore::store::apply(w, &[], &moves);
    w.life.cell_at(i).parts[s].id
}

/// `n` parts placed by an edit over the dry land cells of `w`, as many in each: each of the common rock, taken from its
/// ground with its heat at its temperature, of a design drawn by a stream keyed by `seed` (1 to 1,000 kg, any
/// elongation and hollowness), a record of its own until the parts' monthly turn joins the alike ones (laws.md 3.1).
pub fn scatter(w: &mut World, n: usize, seed: u64) {
    let cells = dry_lands(w);
    let mut x = seed;
    // SplitMix64 (Steele, Lea and Flood 2014): a draw in 0..1.
    let mut draw = move || {
        x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    let ns = w.chem.ns();
    for (q, &i) in cells.iter().enumerate() {
        let m = n / cells.len() + (q < n % cells.len()) as usize;
        let ground = |w: &World, k: usize| w.store.amounts(Class::Ground)[k * w.mesh.cells().n() + i];
        let k = ROCK;
        let (t, mm, cp) = (w.t_ground_last(i), w.chem.subs[k].molar_mass, w.chem.subs[k].cp);
        let made: Vec<(f64, f32, f32)> = (0..m).map(|_| (10f64.powf(3.0 * draw()) / mm, (2.0 * draw() - 1.0) as f32, (0.9 * draw()) as f32)).collect();
        let mol: f64 = made.iter().map(|x| x.0).sum();
        assert!(ground(w, k) > mol, "cell {i} holds less than its parts");
        let places = w.in_cell(i, |w, d| {
            made.iter()
                .map(|&(a, x, h)| {
                    let mut wall = vec![0.0; ns];
                    wall[k] = a;
                    bond::make(w, d, i, wall, a * cp * t, x, h)
                })
                .collect::<Vec<usize>>()
        });
        let moves: Vec<Move> = places.iter().zip(&made).map(|(&s, &(a, ..))| Move { from: Node::ground(i), to: Node::new(Class::Part, s), k: k as u16, mol: a, heat: a * cp * t }).collect();
        pcore::store::apply(w, &[], &moves);
    }
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

/// What the bodies took from the ground in the day's events, kg.
pub fn took_from_ground(w: &World) -> f64 {
    w.events.iter().map(|e| if let EventKind::Took { from: u64::MAX, .. } = e.kind { e.value } else { 0.0 }).sum()
}

/// What the bodies took of what they use (`Body::uses`), in the day's events, kg.
pub fn took_used(w: &World) -> f64 {
    w.events.iter().filter_map(|e| if let EventKind::Took { used, .. } = e.kind { Some(used as f64) } else { None }).sum()
}

/// The members born in the day's events.
pub fn born(w: &World) -> f64 {
    w.events.iter().filter(|e| matches!(e.kind, EventKind::Born { .. })).map(|e| e.value).sum()
}

/// The members that died of `cause` in the day's events.
pub fn died(w: &World, cause: u8) -> f64 {
    w.events.iter().filter(|e| matches!(e.kind, EventKind::Died { cause: c, .. } if c == cause)).map(|e| e.value).sum()
}
