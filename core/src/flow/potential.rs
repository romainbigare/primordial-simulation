//! The potential energy of the world's matter (laws.md 0, "The books"): one function of what the nodes hold and of the
//! ground's phases (`potential`), with the heights each node's matter leaves from (`Heights`); and its booking around
//! each part of Flow (`Lifts`): each move carries its potential energy from the height it leaves, and each node's heat
//! takes what came in less what left less the change of its own, so `Store::lifted` is the change of that function.

use super::{land::NONE, scratch::Sp, Scratch};
use crate::store::{self, fall_heat, Class, LinkFlux, Move};
use crate::{World, AIR};

/// Numbers of a ground cell's row in `potential`: the potential energy of its stack and of its bed's store, J; the
/// heights its liquid and its store's liquid reach, m; and the potential energy of the matter at its head, J.
const ROW: usize = 5;
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
/// there. A bodies' node's gas, and the matter of the cell's life (its bodies, its cover and its parts), lie at its
/// cell's head.
fn potential(w: &World, s: &Scratch, found: Option<Found>) -> Potential {
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    let (n, na, ns) = (cells.n(), air.n(), w.chem.ns());
    let (ground, bed, deep, bodies) = (w.store.amounts(Class::Ground), w.store.amounts(Class::Bed), w.store.amounts(Class::Deep), w.store.amounts(Class::Bodies));
    let found = found.unwrap_or_else(|| Found::of(w, s));
    let (life, holds) = (&found.life, found.holds);
    let same = Same::of(s);
    let mut rows = vec![0.0; n.div_ceil(CELLS_PER_TASK) * CELLS_PER_TASK * ROW];
    crate::par::rows(&mut rows, CELLS_PER_TASK * ROW, || (), |_, task, out| {
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
                let each = sums.iter_mut().zip(r);
                match same[k] {
                    // What `ground_layers` gives where the phase is the same in every node.
                    Same::Gas => each.for_each(|(x, &a)| x[2] += a * sp.mm),
                    Same::Liquid => each.for_each(|(x, &a)| (x[2], x[5]) = (x[2] + a * sp.mm, x[5] + a * sp.v[1])),
                    Same::Solid => each.for_each(|(x, &a)| (x[1], x[4]) = (x[1] + a * sp.mm, x[4] + a * sp.v[0])),
                    Same::Varies => each.zip(cells_of.clone()).for_each(|((x, &a), i)| ground_layers(x, s, (k, i), a)),
                }
            }
            if let Some(r) = has(2).then(|| held(&deep[at(k)])).flatten() {
                let each = sums.iter_mut().zip(r);
                match same[k] {
                    Same::Gas => each.for_each(|(x, &a)| x[6] += a * sp.mm),
                    Same::Liquid | Same::Solid => each.for_each(|(x, &a)| (x[6], x[7]) = (x[6] + a * sp.mm, x[7] + a * sp.v[1])),
                    Same::Varies => each.zip(cells_of.clone()).for_each(|((x, &a), i)| deep_layer(x, sp, s.state.tg[i], a)),
                }
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

/// What a ground cell's stack is made of: the mass, kg, and the volume, m³, of each layer (bed, loose solids, liquid),
/// then of its store, then the mass at its head, kg (its bodies' node's gas and its life).
const LAYERS: usize = 9;
const HEAD: usize = 8;

/// How a substance stands in every ground node now, where it stands alike in all: a gas (at or over its boiling point),
/// a liquid or a solid (under it, at or over its melting point or under it, on no node's plateau); else it varies.
#[derive(Clone, Copy)]
enum Same {
    Gas,
    Liquid,
    Solid,
    Varies,
}

impl Same {
    /// Each substance, by Flow's state of the ground nodes (`ground_layers`).
    fn of(s: &Scratch) -> Vec<Same> {
        let (tmin, tmax) = super::range(&s.state.tg);
        let mut plateau = vec![false; s.sp.len()];
        s.state.pk.iter().filter(|&&k| k != NONE).for_each(|&k| plateau[k] = true);
        (s.sp.iter().zip(plateau))
            .map(|(sp, on)| match () {
                _ if sp.boil <= tmin => Same::Gas,
                _ if sp.boil > tmax && !on && sp.melt <= tmin => Same::Liquid,
                _ if sp.boil > tmax && !on && sp.melt > tmax => Same::Solid,
                _ => Same::Varies,
            })
            .collect()
    }
}

/// `a` mol of substance `sp` in a bed.
#[inline]
fn bed_layer(x: &mut [f64; LAYERS], sp: &Sp, a: f64) {
    (x[0], x[3]) = (x[0] + a * sp.mm, x[3] + a * sp.v[0]);
}

/// `a` mol of substance `k` in ground node `i`, in its phase by the node's temperature and plateau (`State`).
#[inline]
fn ground_layers(x: &mut [f64; LAYERS], s: &Scratch, (k, i): (usize, usize), a: f64) {
    let (sp, t) = (&s.sp[k], s.state.tg[i]);
    let wet = if t >= sp.boil { 1.0 } else if s.state.pk[i] == k { s.state.phi[i] } else { (t >= sp.melt) as u8 as f64 };
    let (wet_mol, dry_mol) = (a * wet, a - a * wet);
    (x[2], x[5]) = (x[2] + wet_mol * sp.mm, x[5] + if t < sp.boil { wet_mol * sp.v[1] } else { 0.0 });
    (x[1], x[4]) = (x[1] + dry_mol * sp.mm, x[4] + dry_mol * sp.v[0]);
}

/// `a` mol of substance `sp` in a store under ground at `t` K.
#[inline]
fn deep_layer(x: &mut [f64; LAYERS], sp: &Sp, t: f64, a: f64) {
    (x[6], x[7]) = (x[6] + a * sp.mm, x[7] + if t < sp.boil { a * sp.v[1] } else { 0.0 });
}

/// The row of `potential` (`ROW`) of ground cell `i` made of `x` (`LAYERS`).
fn stack(w: &World, x: &[f64; LAYERS], i: usize) -> [f64; ROW] {
    let (area, g, p) = (w.mesh.cells().area()[i], w.cfg.gravity, w.cfg.phys);
    // Each layer's bottom, m, and its potential energy at its middle.
    let (mut z, mut e) = (w.base_z, 0.0);
    for (m, v) in x[..3].iter().zip(&x[3..6]) {
        e += m * g * (z + v / area / 2.0);
        z += v / area;
    }
    let (floor, fill) = (w.base_z + x[3] / area - p.bed_depth, x[7] / (p.bed_porosity * area));
    [e, x[6] * g * (floor + fill / 2.0), z, floor + fill, x[HEAD] * g * z]
}

/// The matter of the cells' life, which lies at their heads: the substances of a body and of the cover's parts.
struct Life {
    body: Vec<usize>,
    cover: Option<[usize; crate::cover::NP]>,
}

impl Life {
    fn of(w: &World) -> Life {
        Life { body: w.chem.body().subs().to_vec(), cover: crate::cover::Parts::of(&w.chem).map(|p| p.sub) }
    }
    /// The mass of the life of each cell (`kg`).
    fn every(&self, w: &World, sp: &[Sp]) -> Vec<f64> {
        crate::par::map_n(w.mesh.cells().n(), |i| self.kg(w, sp, i))
    }
    /// The mass of the life of cell `i`, kg: its living bodies, its cover and its parts.
    fn kg(&self, w: &World, sp: &[Sp], i: usize) -> f64 {
        let cell = w.life.cell_at(i);
        let mut kg = 0.0;
        for b in cell.bodies.iter().filter(|b| b.alive) {
            kg += self.body.iter().zip(&b.store).map(|(&k, &a)| a * sp[k].mm).sum::<f64>();
        }
        if let Some(sub) = self.cover {
            kg += cell.cover.mol.iter().flat_map(|m| m.iter().zip(sub)).map(|(&a, k)| a * sp[k].mm).sum::<f64>();
        }
        for p in &cell.parts {
            kg += p.wall.iter().zip(&p.fill).zip(sp).map(|((&a, &f), sp)| (a + f) * sp.mm).sum::<f64>();
        }
        kg
    }
}

/// What the potential energy of ground cell `i` and the matter at its head gains, J, when its ground's amounts change
/// by `d(k)` mol, by Flow's state of its ground (`potential`): how a change inside a node moves its stack.
pub(crate) fn stack_gain(w: &World, i: usize, d: impl Fn(usize) -> f64) -> f64 {
    let (s, n, ns) = (&w.scratch, w.mesh.cells().n(), w.chem.ns());
    let at = |class: Class, k: usize| w.store.amounts(class)[k * n + i];
    let mut x = [0.0; LAYERS];
    for (k, sp) in s.sp.iter().enumerate() {
        bed_layer(&mut x, sp, at(Class::Bed, k));
        deep_layer(&mut x, sp, s.state.tg[i], at(Class::Deep, k));
        x[HEAD] += at(Class::Bodies, k) * sp.mm;
    }
    x[HEAD] += Life::of(w).kg(w, &s.sp, i);
    let (mut before, mut after) = (x, x);
    for k in 0..ns {
        ground_layers(&mut before, s, (k, i), at(Class::Ground, k));
        ground_layers(&mut after, s, (k, i), at(Class::Ground, k) + d(k));
    }
    let (a, b) = (stack(w, &after, i), stack(w, &before, i));
    a[0] + a[4] - b[0] - b[4]
}

/// The potential energy of the world's matter, J (`potential`), from Flow's state of its ground.
pub fn total(w: &World) -> f64 {
    let p = potential(w, &w.scratch, None);
    [&p.ground, &p.air, &p.deep, &p.bodies].iter().flat_map(|v| v.iter()).sum()
}

/// A part of Flow being booked: the potential energy of every node at its start, and the mass its moves carried so far
/// (`carry`).
#[derive(Clone, Debug)]
pub(super) struct Lifts {
    start: Potential,
    moved: Vec<Moved>,
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
        Lifts { start: known.unwrap_or_else(|| potential(w, s, None)), moved }
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
    /// Book the part's lifts and falls: each mass moved carries its potential energy from the height it left, the mean
    /// of its node's height at the part's start and at its end (so a node's own matter that leaves it, however much,
    /// takes what it held of it); each node's heat takes what came in less what left less the change of its own
    /// potential energy (`store::fall_heat`, which adds their sum to `Store::lifted`). A bodies' node's gas lies at its
    /// cell's head, which the part moves with the ground's matter, not its own: what its lifts and falls give or take
    /// is its ground node's (a bodies' node holds little heat of its own). Returns the potential energy now, and the
    /// room of its moves, emptied.
    pub(super) fn book(self, w: &mut World, s: &Scratch) -> (Potential, Vec<Moved>) {
        let (mut start, mut moved) = (self.start, self.moved);
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
        for (class, q) in [(Class::Ground, ground), (Class::Air, air), (Class::Deep, deep)] {
            fall_heat(w, class, q);
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
        for x in fluxes {
            // Per link the mass that goes each way, summed over its substances in their order.
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
