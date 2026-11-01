//! The potential energy of the world's matter (laws.md 0, "The books"): one function of what the nodes hold and of the
//! ground's phases (`potential`), with the heights each node's matter leaves from (`Heights`); and its booking around
//! each part of Flow (`Lifts`): each move carries its potential energy from the height it leaves, and each node's heat
//! takes what came in less what left less the change of its own, so `Store::lifted` is the change of that function.

use super::Scratch;
use crate::store::{self, fall_heat, Class, LinkFlux, Move};
use crate::{World, AIR};

/// Numbers of a ground cell's row in `potential`: the potential energy of its stack and of its bed's store, J; the
/// heights its liquid and its store's liquid reach, m; and the potential energy of its bodies' node, J.
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
/// there. A bodies' node's gas lies at its cell's head.
pub(super) fn potential(w: &World, s: &Scratch) -> Potential {
    potential_of(w, s)
}

fn potential_of(w: &World, s: &Scratch) -> Potential {
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    let (n, na, g, p) = (cells.n(), air.n(), w.cfg.gravity, w.cfg.phys);
    // The substances Flow's last refresh found in the ground, its bed and the air, and those it added since
    // (`scratch::refresh`): what its stores and the bodies' nodes hold comes from them.
    let mut ks: Vec<usize> = s.state.loose.iter().chain(&s.state.bedk).chain(&s.state.gas).copied().collect();
    ks.sort_unstable();
    ks.dedup();
    let (ground, bed, deep, bodies) = (w.store.amounts(Class::Ground), w.store.amounts(Class::Bed), w.store.amounts(Class::Deep), w.store.amounts(Class::Bodies));
    let mut rows = vec![0.0; n.div_ceil(CELLS_PER_TASK) * CELLS_PER_TASK * ROW];
    crate::par::rows(&mut rows, CELLS_PER_TASK * ROW, || (), |_, task, out| {
        let cells_of = task * CELLS_PER_TASK..((task + 1) * CELLS_PER_TASK).min(n);
        // Per cell: mass, kg, and volume, m³, of each layer (bed, loose solids, liquid), then of its store, then the
        // mass of its bodies' node.
        let mut sums = vec![[0.0; 9]; cells_of.len()];
        // Where each substance lies in its class's amounts over the task's cells (`held`: none where it holds none).
        let at = |k: usize| k * n + cells_of.start..k * n + cells_of.end;
        let (tg, pk, phi) = (&s.state.tg[cells_of.clone()], &s.state.pk[cells_of.clone()], &s.state.phi[cells_of.clone()]);
        for &k in &ks {
            let sp = &s.sp[k];
            if let Some(r) = held(&bed[at(k)]) {
                sums.iter_mut().zip(r).for_each(|(x, &a)| (x[0], x[3]) = (x[0] + a * sp.mm, x[3] + a * sp.v[0]));
            }
            if let Some(r) = held(&ground[at(k)]) {
                for (c, (x, &a)) in sums.iter_mut().zip(r).enumerate() {
                    let t = tg[c];
                    let wet = if t >= sp.boil { 1.0 } else if pk[c] == k { phi[c] } else { (t >= sp.melt) as u8 as f64 };
                    let (wet_mol, dry_mol) = (a * wet, a - a * wet);
                    (x[2], x[5]) = (x[2] + wet_mol * sp.mm, x[5] + if t < sp.boil { wet_mol * sp.v[1] } else { 0.0 });
                    (x[1], x[4]) = (x[1] + dry_mol * sp.mm, x[4] + dry_mol * sp.v[0]);
                }
            }
            if let Some(r) = held(&deep[at(k)]) {
                for (c, (x, &a)) in sums.iter_mut().zip(r).enumerate() {
                    (x[6], x[7]) = (x[6] + a * sp.mm, x[7] + if tg[c] < sp.boil { a * sp.v[1] } else { 0.0 });
                }
            }
            if let Some(r) = held(&bodies[at(k)]) {
                sums.iter_mut().zip(r).for_each(|(x, &a)| x[8] += a * sp.mm);
            }
        }
        for ((x, i), row) in sums.iter().zip(cells_of.clone()).zip(out.chunks_exact_mut(ROW)) {
            let area = cells.area()[i];
            // Each layer's bottom, m, and its potential energy at its middle.
            let (mut z, mut e) = (w.base_z, 0.0);
            for (m, v) in x[..3].iter().zip(&x[3..6]) {
                e += m * g * (z + v / area / 2.0);
                z += v / area;
            }
            let (floor, fill) = (w.base_z + x[3] / area - p.bed_depth, x[7] / (p.bed_porosity * area));
            row.copy_from_slice(&[e, x[6] * g * (floor + fill / 2.0), z, floor + fill, x[8] * g * z]);
        }
    });
    rows.truncate(n * ROW);
    let at = |c: usize| rows.iter().skip(c).step_by(ROW).copied().collect::<Vec<f64>>();
    let (head, amounts) = (at(2), w.store.amounts(Class::Air));
    let mut za = vec![0.0; na];
    cells.parent().iter().enumerate().for_each(|(i, &j)| za[j as usize] += head[i] * cells.area()[i]);
    za.iter_mut().zip(air.area()).for_each(|(z, a)| *z /= a);
    let e_air = (0..na).map(|j| ks.iter().map(|&k| amounts[k * na + j] * s.sp[k].mm).sum::<f64>() * g * za[j]).collect();
    Potential { ground: at(0), air: e_air, deep: at(1), bodies: at(4), heights: Heights { ground: head, air: za, deep: at(3) } }
}

/// The potential energy of the world's matter, J (`potential`), from Flow's state of its ground.
pub fn total(w: &World) -> f64 {
    let p = potential(w, &w.scratch);
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

/// The array of `Potential` and the index of node `index` of `class`, for a class that holds matter here (the bed's
/// matter lies in its cell's stack).
fn slot(class: Class, index: u32) -> (u8, u32) {
    match class {
        Class::Ground | Class::Bed => (0, index),
        Class::Air => (1, index),
        Class::Deep => (2, index),
        Class::Bodies => (3, index),
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
            0 | 3 => self.heights.ground[index],
            1 => self.heights.air[index],
            _ => self.heights.deep[index],
        }
    }
}

impl Lifts {
    /// From the world now, whose potential energy is `known` where the last part left it so, its moves gathered in
    /// `moved` (empty; kept from part to part for its room).
    pub(super) fn start(w: &World, s: &Scratch, known: Option<Potential>, moved: Vec<Moved>) -> Lifts {
        Lifts { start: known.unwrap_or_else(|| potential(w, s)), moved }
    }
    /// `kg` go from node `from` to node `to` (back where negative).
    fn carry(&mut self, (from, to): ((Class, u32), (Class, u32)), kg: f64) {
        let (src, dst) = if kg > 0.0 { (from, to) } else { (to, from) };
        let (from, to) = (slot(src.0, src.1), slot(dst.0, dst.1));
        if from.0 != NOWHERE && kg != 0.0 {
            self.moved.push(Moved { from, to, kg: kg.abs() });
        }
    }
    /// Book the part's lifts and falls: each mass moved carries its potential energy from the height it left, the mean
    /// of its node's height at the part's start and at its end (so a node's own matter that leaves it, however much,
    /// takes what it held of it); each node's heat takes what came in less what left less the change of its own
    /// potential energy (`store::fall_heat`, which adds their sum to `Store::lifted`). Returns the potential energy now,
    /// and the room of its moves, emptied.
    pub(super) fn book(self, w: &mut World, s: &Scratch) -> (Potential, Vec<Moved>) {
        let (now, g) = (potential(w, s), w.cfg.gravity);
        let (start, mut moved) = (self.start, self.moved);
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
        for (((class, q), old), new) in [Class::Ground, Class::Air, Class::Deep, Class::Bodies].into_iter().zip(&mut q).zip(olds).zip(now.energies()) {
            q.iter_mut().zip(old.iter().zip(new)).for_each(|(q, (o, n))| *q -= n - o);
            fall_heat(w, class, q);
        }
        (now, moved)
    }
}

/// Book `fluxes` and `moves` (`store::apply`), and what they carry for the part of Flow under way (`Lifts`): over each
/// link the mass of all its substances that goes each way.
pub(super) fn apply(w: &mut World, s: &mut Scratch, fluxes: &[LinkFlux], moves: &[Move]) {
    if let Some(l) = s.lifts.as_mut() {
        for x in fluxes {
            let links = x.a.len();
            let mm: Vec<f64> = x.ks.iter().map(|&k| s.sp[k as usize].mm).collect();
            for lk in 0..links {
                let (mut ab, mut ba) = (0.0, 0.0);
                for (q, mm) in mm.iter().enumerate() {
                    let kg = x.mol[q * links + lk] * mm;
                    if kg > 0.0 { ab += kg } else { ba += kg }
                }
                let ends = ((x.class[0], x.a[lk]), (x.class[1], x.b[lk]));
                l.carry(ends, ab);
                l.carry(ends, ba);
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
