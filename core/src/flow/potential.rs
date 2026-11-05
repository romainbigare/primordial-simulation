//! The potential energy of the world's matter (laws.md 0, "The books"): one function of what the nodes hold and of the
//! ground's phases (`potential`), with the heights each node's matter leaves from (`Heights`); and its booking around
//! a part of Flow, or the parts of a month's Flow as one (`Lifts`, `flow::chain`): each move carries its potential
//! energy from the height it leaves, and each node's heat takes what came in less what left less the change of its own,
//! so `Store::lifted` is the change of that function.

use super::stack::{bed_layer, deep_layer, ground_layers, row_of, stack, Life, Same, HEAD, LAYERS, ROW};
use super::Scratch;
use crate::store::{self, fall_heat, Class, LinkFlux, Move};
use crate::{World, AIR};
use std::borrow::Cow;

/// Cells of a task of `potential`: it walks the amounts of each substance over them in turn.
const CELLS_PER_TASK: usize = 256;

/// The potential energy of the nodes of each class that holds matter, J (the bed's lies in its cell's stack), and the
/// heights their matter leaves from (`Heights`).
#[derive(Clone, Debug)]
pub(super) struct Potential {
    ground: Vec<f64>,
    air: Vec<f64>,
    deep: Vec<f64>,
    bodies: Vec<f64>,
    heights: Heights,
    found: Found,
}

/// The classes of the store whose amounts `potential` reads per cell, in the order of `Found::holds`.
const HELD: [Class; 4] = [Class::Ground, Class::Bed, Class::Deep, Class::Bodies];

/// What the parts of Flow that follow each other (`flow::chain`) find once: the mass of each cell's life, kg
/// (`Life::kg`: no law but Flow's runs between them, and none of them moves it); and per class of `HELD` a bit for each
/// substance its nodes may hold, set as the parts book into it (`apply`), so `potential` reads no array that holds
/// nothing.
#[derive(Clone, Debug, Default)]
struct Found {
    life: Vec<f64>,
    holds: [u64; HELD.len()],
}

impl Found {
    fn of(w: &World, s: &Scratch) -> Found {
        let (n, mut holds) = (w.mesh.cells().n(), [0; HELD.len()]);
        for (bits, &class) in holds.iter_mut().zip(&HELD) {
            let v = w.store.amounts(class);
            *bits = (0..w.chem.ns()).filter(|&k| v[k * n..(k + 1) * n].iter().any(|&a| a != 0.0)).fold(0, |b, k| b | 1 << k);
        }
        Found { life: Life::of(w).every(w, &s.sp), holds }
    }
}

/// The height each node's matter leaves from, m: a ground node's, its bed's and its bodies' node's, the top of its
/// liquid (its head); an air node's, the mean head of its cells; a store's, its liquid's table.
#[derive(Clone, Debug)]
struct Heights {
    ground: Vec<f64>,
    air: Vec<f64>,
    deep: Vec<f64>,
}

/// The potential energy of every node (`Potential`). A ground cell is a stack from `World::base_z` up, each layer at
/// its middle: its bed, its loose solids, and its liquid with what is dissolved in it (the gas above its boiling point
/// too), as Flow's heads stack them (`land::layers`), each substance of the node in its phase by the node's temperature
/// and its plateau (`State`); its store lies `bed_depth` under its bed's top, filled from the bottom (`drain`). An air
/// node's matter lies at the mean head of its cells: the height of its column above that is in its heat (its heat
/// capacity at constant pressure holds the work its column does as it warms), so lifting matter into it lifts it to
/// there. A bodies' node's gas, and the matter of the cell's life (its bodies, its cover with its seeds until they land,
/// and its parts), lie at its cell's head.
fn potential(w: &World, s: &Scratch, found: Option<Found>) -> Potential {
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    let (n, na, ns) = (cells.n(), air.n(), w.chem.ns());
    let (ground, bed, deep, bodies) = (w.store.amounts(Class::Ground), w.store.amounts(Class::Bed), w.store.amounts(Class::Deep), w.store.amounts(Class::Bodies));
    let found = found.unwrap_or_else(|| Found::of(w, s));
    let (life, holds) = (&found.life, found.holds);
    let same = Same::of(s);
    let mut rows = vec![0.0; n.div_ceil(CELLS_PER_TASK) * CELLS_PER_TASK * ROW];
    crate::par::rows(&mut rows, CELLS_PER_TASK * ROW, crate::par::UNCOUNTED, || (), |_, task, out| {
        let cells_of = task * CELLS_PER_TASK..((task + 1) * CELLS_PER_TASK).min(n);
        let mut sums = vec![[0.0; LAYERS]; cells_of.len()];
        // Where each substance lies in its class's amounts over the task's cells (`held`: none where it holds none).
        let at = |k: usize| k * n + cells_of.start..k * n + cells_of.end;
        for k in 0..ns {
            let (sp, has) = (&s.sp[k], |c: usize| holds[c] >> k & 1 == 1);
            if let Some(r) = has(1).then(|| held(&bed[at(k)])).flatten() {
                sums.iter_mut().zip(r).for_each(|(x, &a)| bed_layer(x, sp, a));
            }
            if let Some(r) = has(0).then(|| held(&ground[at(k)])).flatten() {
                sums.iter_mut().zip(r).zip(cells_of.clone()).for_each(|((x, &a), i)| ground_of(x, s, same[k], (k, i), a));
            }
            if let Some(r) = has(2).then(|| held(&deep[at(k)])).flatten() {
                sums.iter_mut().zip(r).zip(cells_of.clone()).for_each(|((x, &a), i)| deep_of(x, s, same[k], (k, i), a));
            }
            if let Some(r) = has(3).then(|| held(&bodies[at(k)])).flatten() {
                sums.iter_mut().zip(r).for_each(|(x, &a)| x[HEAD] += a * sp.mm);
            }
        }
        for ((x, i), row) in sums.iter_mut().zip(cells_of.clone()).zip(out.chunks_exact_mut(ROW)) {
            x[HEAD] += life[i];
            row.copy_from_slice(&stack(w, x, i));
        }
    });
    rows.truncate(n * ROW);
    let at = |c: usize| rows.iter().skip(c).step_by(ROW).copied().collect::<Vec<f64>>();
    let (head, amounts, g) = (at(2), w.store.amounts(Class::Air), w.cfg.gravity);
    let mut za = vec![0.0; na];
    cells.parent().iter().enumerate().for_each(|(i, &j)| za[j as usize] += head[i] * cells.area()[i]);
    za.iter_mut().zip(air.area()).for_each(|(z, a)| *z /= a);
    let e_air = (0..na).map(|j| (0..ns).map(|k| amounts[k * na + j] * s.sp[k].mm).sum::<f64>() * g * za[j]).collect();
    Potential { ground: at(0), air: e_air, deep: at(1), bodies: at(4), heights: Heights { ground: head, air: za, deep: at(3) }, found }
}

/// `a` mol of substance `k` in ground node `i`, in its phase (`ground_layers`), where its phase is the same in every node
/// (`same`) without looking.
#[inline(always)]
pub(super) fn ground_of(x: &mut [f64; LAYERS], s: &Scratch, same: Same, (k, i): (usize, usize), a: f64) {
    let sp = &s.sp[k];
    match same {
        Same::Gas => x[2] += a * sp.mm,
        Same::Liquid => (x[2], x[5]) = (x[2] + a * sp.mm, x[5] + a * sp.v[1]),
        Same::Solid => (x[1], x[4]) = (x[1] + a * sp.mm, x[4] + a * sp.v[0]),
        Same::Varies => ground_layers(x, s, (k, i), a),
    }
}

/// `a` mol of substance `k` in the store under ground node `i` (`deep_layer`), where its phase is the same in every node
/// (`same`) without looking.
#[inline(always)]
pub(super) fn deep_of(x: &mut [f64; LAYERS], s: &Scratch, same: Same, (k, i): (usize, usize), a: f64) {
    let sp = &s.sp[k];
    match same {
        Same::Gas => x[6] += a * sp.mm,
        Same::Liquid | Same::Solid => (x[6], x[7]) = (x[6] + a * sp.mm, x[7] + a * sp.v[1]),
        Same::Varies => deep_layer(x, sp, s.state.tg[i], a),
    }
}

/// Flow's working memory as `potential` reads it: the world's, or its state with the table of the substances where a
/// remap or a load dropped the caches.
pub(super) fn scratch_of(w: &World) -> Cow<'_, Scratch> {
    if w.scratch.sp.is_empty() {
        Cow::Owned(Scratch { state: w.scratch.state.clone(), sp: super::scratch::substances(&w.chem), ..Default::default() })
    } else {
        Cow::Borrowed(&w.scratch)
    }
}

/// Whether Flow built the state of the ground, whose phases the potential energy reads (`potential`): not before its
/// first build, which finds the function then.
pub(crate) fn stated(w: &World) -> bool {
    w.scratch.state.tg.len() == w.mesh.cells().n()
}

/// The potential energy of the world's matter, J (`potential`), from Flow's state of its ground; none before Flow
/// built it (`stated`).
pub fn total(w: &World) -> Option<f64> {
    let p = stated(w).then(|| potential(w, &scratch_of(w), None))?;
    Some([&p.ground, &p.air, &p.deep, &p.bodies].iter().flat_map(|v| v.iter()).sum())
}

/// The potential energy of the matter of ground cells `cells` (their stacks, stores and heads) and of air node `air`,
/// J (`potential`): what a split or a join of cells under `air` changes (`World::split`). It reads those cells and the
/// cells under `air` alone (`row_of`), as `potential` reads them, by Flow's state of the ground, which gives a cell a
/// remap made the phases of its heat (`World::thawed`). None before Flow first built the state (`stated`).
pub(crate) fn potential_of(w: &World, cells: &[usize], air: usize) -> f64 {
    if !stated(w) {
        return 0.0;
    }
    let s = scratch_of(w);
    // The rows of `cells` and the mean head of `air` over its cells, as `potential` finds them.
    let (same, life, (level, g)) = (Same::of(&s), Life::of(w), (w.mesh.level(AIR), w.cfg.gravity));
    let row = |i: usize| row_of(w, &s, &same, i, (|_, _| 0.0, life.kg(w, &s.sp, i)));
    let (amounts, na, za) = (w.store.amounts(Class::Air), level.n(), mean_head(w, &s, &same, air));
    let e_air = (0..w.chem.ns()).map(|k| amounts[k * na + air] * s.sp[k].mm).sum::<f64>() * g * za;
    cells.iter().map(|&i| row(i)).map(|r| r[0] + r[1] + r[4]).sum::<f64>() + e_air
}

/// The head of each ground cell now, m, as `potential` stacks it (`row_of`), by Flow's state of the ground: the height
/// its matter leaves from.
pub(super) fn heads(w: &World, s: &Scratch) -> Vec<f64> {
    let same = Same::of(s);
    crate::par::map_n(w.mesh.cells().n(), crate::par::UNCOUNTED, |i| row_of(w, s, &same, i, (|_, _| 0.0, 0.0))[2])
}

/// The mean head of the cells of air node `j`, m, where its matter lies (`potential`), by the phases of `s`.
fn mean_head(w: &World, s: &Scratch, same: &[Same], j: usize) -> f64 {
    let (level, area) = (w.mesh.level(AIR), w.mesh.cells().area());
    level.children(j).iter().map(|&i| i as usize).fold(0.0, |z, i| z + row_of(w, s, same, i, (|_, _| 0.0, 0.0))[2] * area[i]) / level.area()[j]
}

/// The mean head of the cells of air node `j`, m, by Flow's state of the ground (`potential`): the height of matter that
/// enters it. None before Flow first built the state of the ground (`stated`).
pub(crate) fn air_head(w: &World, j: usize) -> f64 {
    if !stated(w) {
        return 0.0;
    }
    let s = scratch_of(w);
    mean_head(w, &s, &Same::of(&s), j)
}

/// A part of Flow being booked, or the parts that follow each other (`flow::chain`): the potential energy of every node
/// at their start, and the mass their moves carried so far (`carry`).
#[derive(Clone, Debug)]
pub(super) struct Lifts {
    start: Potential,
    moved: Vec<Moved>,
    given: Vec<f64>,
}

/// A mass that went from one node to another, kg: each named by its array of `Potential` and its index there (`slot`;
/// the node it went to may hold no matter here, `NOWHERE`).
#[derive(Clone, Copy, Debug)]
pub(super) struct Moved {
    from: (u8, u32),
    to: (u8, u32),
    kg: f64,
}

/// The array of `Moved::to` for a node that holds no matter here.
const NOWHERE: u8 = u8::MAX;
/// The array of the bodies' nodes (`slot`).
const BODIES: u8 = 3;

/// The array of `Potential` and the index of node `index` of `class`, for a class that holds matter here (the bed's
/// matter lies in its cell's stack).
fn slot(class: Class, index: u32) -> (u8, u32) {
    match class {
        Class::Ground | Class::Bed => (0, index),
        Class::Air => (1, index),
        Class::Deep => (2, index),
        Class::Bodies => (BODIES, index),
        _ => (NOWHERE, index),
    }
}

impl Potential {
    /// The potential energy of each node of each array (`slot`), J.
    fn energies(&self) -> [&Vec<f64>; 4] {
        [&self.ground, &self.air, &self.deep, &self.bodies]
    }
    /// The height the matter of node `index` of array `c` (`slot`) leaves from, m.
    fn height(&self, (c, index): (u8, u32)) -> f64 {
        let index = index as usize;
        match c {
            0 | BODIES => self.heights.ground[index],
            1 => self.heights.air[index],
            _ => self.heights.deep[index],
        }
    }
}

impl Lifts {
    /// From the world now, whose potential energy is `known` where the last part left it so, its moves gathered in
    /// `moved` (empty; kept from part to part for its room).
    pub(super) fn start(w: &World, s: &Scratch, known: Option<Potential>, moved: Vec<Moved>) -> Lifts {
        Lifts { start: known.unwrap_or_else(|| potential(w, s, None)), moved, given: Vec::new() }
    }
    /// The height ground node `i`'s matter left from at the start, m (its head).
    pub(super) fn head_at_start(&self, i: usize) -> f64 {
        self.start.heights.ground[i]
    }
    /// Heat of falls given on the way to the ground nodes the matter lands in, `q[i]` J (`store::fall_heat`, against
    /// `Store::lifted`), so what passes on takes its share; the booking at the end gives them only the rest.
    pub(super) fn give(&mut self, w: &mut World, q: &[f64]) {
        fall_heat(w, Class::Ground, q.iter().copied().enumerate());
        self.given.resize(q.len(), 0.0);
        self.given.iter_mut().zip(q).for_each(|(g, &x)| *g += x);
    }
    /// `kg` go from node `from` to node `to` (back where negative).
    fn carry(&mut self, (from, to): ((Class, u32), (Class, u32)), kg: f64) {
        self.carry_slots((slot(from.0, from.1), slot(to.0, to.1)), kg);
    }
    /// `carry`, between nodes named by their arrays (`slot`).
    #[inline]
    fn carry_slots(&mut self, (a, b): ((u8, u32), (u8, u32)), kg: f64) {
        let (from, to) = if kg > 0.0 { (a, b) } else { (b, a) };
        if from.0 != NOWHERE && kg != 0.0 {
            self.moved.push(Moved { from, to, kg: kg.abs() });
        }
    }
    /// Book the part's lifts and falls (or those of the parts that follow each other, `flow::chain`): each mass moved
    /// carries its potential energy from the height it left, the mean of its node's height at the start and at the end
    /// (so a node's own matter that leaves it, however much, takes what it held of it); each node's heat takes what
    /// came in less what left less the change of its own potential energy (`store::fall_heat`, which adds their sum to
    /// `Store::lifted`). A bodies' node's gas lies at its cell's head, which the part moves with the ground's matter,
    /// not its own: what its lifts and falls give or take is its ground node's (a bodies' node holds little heat of its
    /// own). Returns the potential energy now, and the room of its moves, emptied.
    pub(super) fn book(self, w: &mut World, s: &Scratch) -> (Potential, Vec<Moved>) {
        let (mut start, mut moved, given) = (self.start, self.moved, self.given);
        let (now, g) = (potential(w, s, Some(std::mem::take(&mut start.found))), w.cfg.gravity);
        let olds = start.energies();
        let mut q: [Vec<f64>; 4] = now.energies().map(|v| vec![0.0; v.len()]);
        for m in &moved {
            let e = m.kg * g * (start.height(m.from) + now.height(m.from)) / 2.0;
            q[m.from.0 as usize][m.from.1 as usize] -= e;
            if m.to.0 != NOWHERE {
                q[m.to.0 as usize][m.to.1 as usize] += e;
            }
        }
        moved.clear();
        for ((q, old), new) in q.iter_mut().zip(olds).zip(now.energies()) {
            q.iter_mut().zip(old.iter().zip(new)).for_each(|(q, (o, n))| *q -= n - o);
        }
        let [ground, air, deep, bodies] = &mut q;
        ground.iter_mut().zip(bodies.iter_mut()).for_each(|(g, b)| *g += std::mem::take(b));
        ground.iter_mut().zip(&given).for_each(|(g, given)| *g -= given);
        for (class, q) in [(Class::Ground, ground), (Class::Air, air), (Class::Deep, deep)] {
            fall_heat(w, class, q.iter().copied().enumerate());
        }
        (now, moved)
    }
}

/// Book `fluxes` and `moves` (`store::apply`), and what they carry for the part of Flow under way (`Lifts`): over each
/// link the mass of all its substances that goes each way.
pub(super) fn apply(w: &mut World, s: &mut Scratch, fluxes: &[LinkFlux], moves: &[Move]) {
    if let Some(l) = s.lifts.as_mut() {
        let holds = &mut l.start.found.holds;
        let mut into = |class: Class, k: u16| HELD.iter().zip(holds.iter_mut()).filter(|c| *c.0 == class).for_each(|(_, b)| *b |= 1 << k);
        for x in fluxes {
            x.class.iter().for_each(|&c| x.ks.iter().for_each(|&k| into(c, k)));
        }
        moves.iter().for_each(|m| [m.from.class, m.to.class].iter().for_each(|&c| into(c, m.k)));
        for x in fluxes.iter().filter(|x| !x.ks.is_empty()) {
            // Per link the mass that goes each way, summed over its substances in their order (a flux of heat alone
            // carries none).
            let links = x.a.len();
            let (mut ab, mut ba) = (vec![0.0; links], vec![0.0; links]);
            for (q, &k) in x.ks.iter().enumerate() {
                let mm = s.sp[k as usize].mm;
                for ((ab, ba), &m) in ab.iter_mut().zip(ba.iter_mut()).zip(&x.mol[q * links..(q + 1) * links]) {
                    let kg = m * mm;
                    if kg > 0.0 { *ab += kg } else { *ba += kg }
                }
            }
            let (ca, cb) = (slot(x.class[0], 0).0, slot(x.class[1], 0).0);
            for (lk, (ab, ba)) in ab.into_iter().zip(ba).enumerate() {
                let ends = ((ca, x.a[lk]), (cb, x.b[lk]));
                l.carry_slots(ends, ab);
                l.carry_slots(ends, ba);
            }
        }
        for m in moves {
            l.carry(((m.from.class, m.from.index as u32), (m.to.class, m.to.index as u32)), m.mol * s.sp[m.k as usize].mm);
        }
    }
    store::apply(w, fluxes, moves);
}

/// `v`, where it holds anything.
fn held(v: &[f64]) -> Option<&[f64]> {
    Some(v).filter(|r| r.iter().any(|&a| a != 0.0))
}
