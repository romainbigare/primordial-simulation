//! What the tests of the laws share (the `testkit` feature, for dev-dependencies only): a small world, founders of a
//! chosen brain, edits booked from outside, the books, and the substances of chemistry 0 by number.

use pcore::body::{Body, Layout, BRAIN_SIZES, CHILD, COPY_MASS, DISPERSE, LIQ, STRENGTH, WET_TISSUE};
use pcore::bond;
use pcore::brain::{self, FIXED, OVERBS, VERBS};
use pcore::check::{self, Totals};
use pcore::chem::{Chemistry, Phase};
use pcore::event::EventKind;
use pcore::lineage::{self, Lineage};
use pcore::rng::{Purpose, Rng};
use pcore::store::{Boundary, Class, Move, Node};
use pcore::units::DAY_S;
use pcore::World;

/// A body an edit changes (its own end of the moves of an edit, which it books itself; `World::cross`).
const A_BODY: Node = Node { class: Class::Body, index: 0 };

/// Substances of chemistry 0: the common liquid.
pub const LIQUID: usize = 0;
/// The gas the fixing reaction (24) takes from the air.
pub const FIX_GAS: usize = 1;
/// The gas the energy-rich solid burns with.
pub const OX: usize = 2;
/// The heat-absorbing gas.
pub const ABSORBER: usize = 3;
/// A gas that burns with `OX` (reaction 8).
pub const BURNING_GAS: usize = 4;
/// The energy-rich solid the light reaction makes.
pub const RICH: usize = 5;
/// The rock that dissolves in the liquid with the absorbing gas (reactions 22 and 23).
pub const ROCK: usize = 10;
/// What `ROCK` leaves when it gives off the absorbing gas (reaction 15).
pub const BURNT_ROCK: usize = 11;
/// The gas the fixing reaction gives off.
pub const FIX_OUT: usize = 21;
/// Body matter.
pub const MATTER: usize = 22;
/// The mineral that exists only dissolved in the liquid, its store of the absorbing gas.
pub const DISSOLVED: usize = 23;
/// The mineral of the fixed gas.
pub const MINERAL: usize = 24;
/// The frame, a solid bodies build.
pub const FRAME: usize = 25;
/// The reaction that burns the energy-rich solid with `OX`.
pub const BURN: usize = 1;
/// The reaction that breaks `MINERAL` down when hot (its fixed element goes back to the air as `FIX_GAS`).
pub const MINERAL_BREAKS: usize = 25;

/// The world most tests stand in: seed 1, 16 x 16 cells, after one month (so its cells have their surroundings).
pub fn small_world() -> World {
    let mut w = crate::new_world(1, 16, 16);
    pcore::step(&mut w);
    w
}

/// The land cells with no liquid standing on them.
pub fn dry_lands(w: &World) -> Vec<usize> {
    (0..w.mesh.cells().n()).filter(|&i| w.is_land(i) && w.life.env[i][1] < 1e-3).collect()
}

/// The first land cell with no liquid standing on it.
pub fn dry_land(w: &World) -> usize {
    dry_lands(w).first().copied().expect("dry land")
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
/// and the seed of its random weights.
#[derive(Clone, Copy, Debug)]
pub struct Brain {
    pub fixed: [f32; FIXED],
    pub dir: f32,
    pub verbs: [f32; VERBS + OVERBS],
    pub hidden: usize,
    pub seed: u64,
}

impl Brain {
    /// A brain that only does fixed action `act`.
    pub fn only(act: usize) -> Self {
        let mut fixed = [OFF; FIXED];
        fixed[act] = ON;
        Brain { fixed, dir: OFF, verbs: [OFF; VERBS + OVERBS], hidden: HIDDEN, seed: 1 }
    }

    /// A brain that only does verb `verb` (`brain::GIVE`, ..., or `brain::VERBS + k` for the verbs on parts).
    pub fn verb(verb: usize) -> Self {
        let mut verbs = [OFF; VERBS + OVERBS];
        verbs[verb] = ON;
        Brain { fixed: [OFF; FIXED], dir: OFF, verbs, hidden: HIDDEN, seed: 1 }
    }

    /// Every bias 0: nothing preferred.
    pub fn neutral() -> Self {
        Brain { fixed: [0.0; FIXED], dir: 0.0, verbs: [0.0; VERBS + OVERBS], hidden: HIDDEN, seed: 1 }
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
        let mut base = brain::random(shape, &mut Rng::new(brain.seed, 0, 0, Purpose::Brain), WEIGHTS, brain.fixed, brain.dir, brain.verbs);
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
        b.prepare(&w.chem, &w.cfg.phys);
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
    let mut b = founder(w, 1, i, &Brain::only(act));
    let m = b.mass(&w.chem);
    scale(w, &mut b, EATER_KG / m);
    (b.genes[DISPERSE], b.genes[STRENGTH], b.genes[COPY_MASS], b.genes[LIQ]) = (0.0, 0.0, copy, -4.0);
    b.prepare(&w.chem, &w.cfg.phys);
    b
}

/// Body `b` made `f` times as big: its stores and heat (booked as an edit).
pub fn scale(w: &mut World, b: &mut Body, f: f64) {
    w.cross((Boundary::Outside, A_BODY), b.store.iter().map(|&a| (f - 1.0) * a).enumerate(), (f - 1.0) * b.e);
    b.store.iter_mut().for_each(|a| *a *= f);
    b.e *= f;
}

/// `kg` of substance `k` added to body `b` at its temperature (booked as an edit), or taken out if negative.
pub fn add(w: &mut World, b: &mut Body, k: usize, kg: f64) {
    let (a, t, e0) = (kg / w.chem.subs[k].molar_mass, b.temp(&w.chem), b.e);
    b.store[k] += a;
    b.e = b.energy_at(&w.chem, t);
    w.cross((Boundary::Outside, A_BODY), [(k, a)], b.e - e0);
}

/// What body `b` holds of substance `k`, kg.
pub fn kg(w: &World, b: &Body, k: usize) -> f64 {
    b.store[k] * w.chem.subs[k].molar_mass
}

/// `mol` of substance `k` put on the ground of cell `i` at its temperature (booked as an edit).
pub fn put(w: &mut World, k: usize, i: usize, mol: f64) {
    let t = w.t_ground(i);
    w.cross((Boundary::Outside, Node::ground(i)), [(k, mol)], 0.0);
    hold_ground(w, i, t);
}

/// `kg` per m² of substance `k` put on the ground of cell `i` at its temperature (booked as an edit).
pub fn put_kg_m2(w: &mut World, k: usize, i: usize, kg: f64) {
    let mol = kg * w.map.area() / w.chem.subs[k].molar_mass;
    put(w, k, i, mol);
}

/// All of substance `k` in the world's ground, the store of its bed and its bodies, mol.
pub fn in_all(w: &World, k: usize) -> f64 {
    let n = w.mesh.cells().n();
    w.store.amounts(Class::Ground)[k * n..(k + 1) * n].iter().sum::<f64>() + w.store.amounts(Class::Deep).get(k * n..(k + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>()) + w.life.alive().map(|b| b.store[k]).sum::<f64>()
}

/// The rock element (element 5) outside body matter on the ground of cell `i`, mol.
pub fn rock_outside_matter(w: &World, i: usize) -> f64 {
    let n = w.mesh.cells().n();
    (0..w.chem.ns()).filter(|&k| k != MATTER).map(|k| w.chem.subs[k].elems[5] as f64 * w.store.amounts(Class::Ground)[k * n + i]).sum()
}

/// Thickness of rock-making solids (those that melt above 1000 K, but body matter, which chars before it melts) in the
/// loose ground of cell `i` and the bed below it, m, and the mean softness (1 / (hardness + 0.05)) of the loose ones, by
/// volume.
pub fn ground_of(w: &World, i: usize) -> (f64, f64, f64) {
    let (n, area) = (w.mesh.cells().n(), w.map.area());
    let (mut loose, mut bed, mut soft) = (0.0, 0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.chem.subs[k].melt > 1000.0 && Some(k) != w.chem.matter()) {
        let v = w.chem.molar_volume(k, Phase::Solid);
        loose += w.store.amounts(Class::Ground)[k * n + i] * v / area;
        bed += w.store.amounts(Class::Bed)[k * n + i] * v / area;
        soft += w.store.amounts(Class::Ground)[k * n + i] * v / area / (w.chem.subs[k].hardness + 0.05);
    }
    (loose, bed, soft / loose.max(1e-300))
}

/// The ground of cell `i` brought to `t` K (booked as an edit).
pub fn hold_ground(w: &mut World, i: usize, t: f64) {
    let e = w.energy_ground(i, t);
    set_heat(w, Node::ground(i), e);
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

/// The amount of substance `k` in node `x` of the store set to `mol` (booked as an edit, as `set_heat`, with no heat: set
/// it after).
pub fn set_amount(w: &mut World, x: Node, k: usize, mol: f64) {
    let len = w.store.amounts(x.class).len() / w.chem.ns();
    w.cross((Boundary::Outside, x), [(k, -w.store.amounts(x.class)[k * len + x.index])], 0.0);
    w.cross((Boundary::Outside, x), [(k, mol)], 0.0);
}

/// Body `b` brought to `t` K (booked as an edit).
pub fn warm(w: &mut World, b: &mut Body, t: f64) {
    let e = b.energy_at(&w.chem, t);
    w.cross((Boundary::Outside, A_BODY), [], e - b.e);
    b.e = e;
}

/// Body `b` placed and taken out again, prepared, to call the laws on directly: its slot and the body.
pub fn out(w: &mut World, b: Body) -> (usize, Body) {
    let s = w.add(b, 1);
    pcore::survey::survey(w);
    w.time += DAY_S;
    let mut b = std::mem::take(&mut w.life.bodies[s]);
    b.prepare(&w.chem, &w.cfg.phys);
    (s, b)
}

/// What `f` does to `w` when it calls a law of bodies or parts on its own, outside an update: its moves in ground cell
/// `i` (`World::open_moves`, in the name of the body in slot `slot`) booked when it ends.
pub fn moved<R>(w: &mut World, (i, slot): (usize, usize), f: impl FnOnce(&mut World) -> R) -> R {
    w.open_moves(i, Node::new(Class::Body, slot));
    let r = f(w);
    w.book_moves();
    r
}

/// Every part of Flow but creep once, for `dt` seconds, in the clock's order: the state rebuilt in full, the air's step,
/// liquid, and the refresh; then the state the step leaves. A law of Flow on its own.
pub fn flow_step(w: &mut World, dt: f64) {
    pcore::flow::state(w, true);
    pcore::flow::air(w, dt);
    pcore::flow::liquid(w, dt);
    pcore::flow::refresh(w);
    pcore::flow::state(w, false);
}

/// Body `b` grows for `days` days at the plateau state it stands at (`copy::grow`). Returns whether it built anything.
pub fn grow(w: &mut World, b: &mut Body, days: f64) -> bool {
    let th = b.thermal(&w.chem);
    pcore::copy::grow(w, b, days, th)
}

/// Body `b` placed and prepared where it is: its slot.
pub fn place(w: &mut World, b: Body) -> usize {
    let (s, b) = out(w, b);
    w.life.bodies[s] = b;
    s
}

/// The brood of `k` children ready for body `b` (all its members together), of the matter a child is made of: a node
/// holds `copy::BROOD_HELD` children a member besides.
pub fn brood(w: &World, b: &mut Body, k: f64) {
    let held = if b.n() > 1.0 { pcore::copy::BROOD_HELD * b.n() } else { 0.0 };
    b.brood = (k + held) * b.dec(CHILD) * b.dec(COPY_MASS) * (1.0 - WET_TISSUE) / w.chem.subs[MATTER].molar_mass;
}

/// The live children of the body of id `id`.
pub fn children(w: &World, id: u64) -> Vec<usize> {
    (0..w.life.bodies.len()).filter(|&k| w.life.bodies[k].alive && w.life.bodies[k].parent == id).collect()
}

/// An eater that burns only the energy-rich solid: no catalyst for burning body matter, no roots for it.
pub fn only_the_solid(w: &World, b: &mut Body) {
    let lay = Layout::of(&w.chem);
    let (m, burn_matter) = w.chem.matter_roles().expect("body matter");
    (b.genes[lay.cat + burn_matter], b.genes[lay.upt + m]) = (-1.0, -0.1);
    b.prepare(&w.chem, &w.cfg.phys);
}

/// The volume of the loose solids of cell `i`, m³.
pub fn loose_volume(w: &World, i: usize) -> f64 {
    let t = w.t_ground(i);
    (0..w.chem.ns()).filter(|&k| w.chem.phase(k, t) == Phase::Solid).map(|k| w.g(k, i) * w.chem.molar_volume(k, Phase::Solid)).sum()
}

/// A part of `kg` of the cell's most plentiful loose substance, taken from the ground at its temperature: flat, or a
/// block if `thick`.
pub fn plate(w: &mut World, i: usize, kg: f64, thick: bool) -> usize {
    let n = w.mesh.cells().n();
    let k = (0..w.chem.ns()).max_by(|&a, &b| w.store.amounts(Class::Ground)[a * n + i].total_cmp(&w.store.amounts(Class::Ground)[b * n + i])).unwrap();
    let mut wall = vec![0.0; w.chem.ns()];
    wall[k] = kg / w.chem.subs[k].molar_mass;
    assert!(w.store.amounts(Class::Ground)[k * n + i] > wall[k], "the cell holds less than the part");
    let (e, mol) = (wall[k] * w.chem.subs[k].cp * w.t_ground_last(i), wall[k]);
    let s = bond::make(w, i, wall, e, if thick { 0.0 } else { -1.0 }, 0.0);
    pcore::store::apply(w, &[], &[Move { from: Node::ground(i), to: Node::new(Class::Part, s), k: k as u16, mol, heat: e }], &[]);
    s
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

/// What the bodies took that they can burn, in the day's events, kg.
pub fn took_to_burn(w: &World) -> f64 {
    w.events.iter().filter_map(|e| if let EventKind::Took { burn, .. } = e.kind { Some(burn as f64) } else { None }).sum()
}

/// The members born in the day's events.
pub fn born(w: &World) -> f64 {
    w.events.iter().filter(|e| matches!(e.kind, EventKind::Born { .. })).map(|e| e.value).sum()
}

/// The members that died of `cause` in the day's events.
pub fn died(w: &World, cause: u8) -> f64 {
    w.events.iter().filter(|e| matches!(e.kind, EventKind::Died { cause: c, .. } if c == cause)).map(|e| e.value).sum()
}
