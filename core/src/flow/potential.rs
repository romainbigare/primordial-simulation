//! The potential energy of the world's matter (laws.md 0, "The books"): one function of what the nodes hold and of the
//! ground's phases (`potential`), with the heights each node's matter leaves from (`Heights`); and its booking around
//! each part of Flow (`Lifts`): each move carries its potential energy from the height it leaves, and each node's heat
//! takes what came in less what left less the change of its own, so `Store::lifted` is the change of that function.

use super::Scratch;
use crate::store::{self, fall_heat, Class, LinkFlux, Move};
use crate::{World, AIR};

/// Numbers of a ground cell's row in `potential`: the potential energy of its stack and of its bed's store, J; the
/// heights its liquid, its surface and its store's liquid reach, m; and the potential energy of its bodies' node, J.
const ROW: usize = 6;

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

/// The height each node's matter leaves from, m: a ground node's and its bed's, the top of its liquid; an air node's,
/// the mean surface of its cells; a store's, its liquid's table; a bodies' node's, its cell's surface.
#[derive(Clone, Debug)]
struct Heights {
    ground: Vec<f64>,
    air: Vec<f64>,
    deep: Vec<f64>,
    surface: Vec<f64>,
}

impl Heights {
    /// The height matter leaves node `index` of `class` from, m (none for a node that holds no matter here).
    fn of(&self, class: Class, index: usize) -> Option<f64> {
        match class {
            Class::Ground | Class::Bed => Some(self.ground[index]),
            Class::Air => Some(self.air[index]),
            Class::Deep => Some(self.deep[index]),
            Class::Bodies => Some(self.surface[index]),
            _ => None,
        }
    }
}

/// The potential energy of every node (`Potential`). A ground cell is a stack from `World::base_z` up, each layer at
/// its middle: its bed, its hard loose solids, its liquid with what is dissolved in it (the gas above its boiling point
/// too), and the soft solids that lie on top (`soft`), each substance of the node in its phase by the node's temperature
/// and its plateau (`State`); its store lies `bed_depth` under its bed's top, filled from the bottom (`drain`). An air
/// node's matter lies at the mean surface of its cells: the height of its column above that is in its heat (its heat
/// capacity at constant pressure holds the work its column does as it warms), so lifting matter into it lifts it to
/// there. A bodies' node's gas lies at its cell's surface.
pub(super) fn potential(w: &World, s: &Scratch) -> Potential {
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    let (n, na, ns, g, p) = (cells.n(), air.n(), w.chem.ns(), w.cfg.gravity, w.cfg.phys);
    let (ground, bed, deep, bodies) = (w.store.amounts(Class::Ground), w.store.amounts(Class::Bed), w.store.amounts(Class::Deep), w.store.amounts(Class::Bodies));
    let mut rows = vec![0.0; n * ROW];
    crate::par::rows(&mut rows, ROW, || (), |_, i, row| {
        let (area, t, pk) = (cells.area()[i], s.state.tg[i], s.state.pk[i]);
        // Mass, kg, and volume, m³, of each layer: bed, hard loose solids, liquid, soft solids; and of the store.
        let (mut m, mut v, mut store) = ([0.0; 4], [0.0; 4], [0.0; 3]);
        for k in 0..ns {
            let (sp, x) = (&s.sp[k], k * n + i);
            (m[0], v[0]) = (m[0] + bed[x] * sp.mm, v[0] + bed[x] * sp.v[0]);
            let a = ground[x];
            if a > 0.0 {
                let wet = if t >= sp.boil { 1.0 } else if pk == k { s.state.phi[i] } else { (t >= sp.melt) as u8 as f64 };
                let layer = if t > p.soft * sp.melt { 3 } else { 1 };
                let liquid = if t < sp.boil { a * wet * sp.v[1] } else { 0.0 };
                (m[2], v[2]) = (m[2] + a * wet * sp.mm, v[2] + liquid);
                (m[layer], v[layer]) = (m[layer] + a * (1.0 - wet) * sp.mm, v[layer] + a * (1.0 - wet) * sp.v[0]);
            }
            (store[0], store[1]) = (store[0] + deep[x] * sp.mm, store[1] + if t < sp.boil { deep[x] * sp.v[1] } else { 0.0 });
            store[2] += bodies[x] * sp.mm;
        }
        // Each layer's bottom, m, and its potential energy at its middle.
        let (mut z, mut e) = (w.base_z, 0.0);
        for (m, v) in m.iter().zip(v) {
            e += m * g * (z + v / area / 2.0);
            z += v / area;
        }
        let liquid_top = z - v[3] / area;
        let floor = w.base_z + v[0] / area - p.bed_depth;
        let fill = store[1] / (p.bed_porosity * area);
        row.copy_from_slice(&[e, store[0] * g * (floor + fill / 2.0), liquid_top, z, floor + fill, store[2] * g * z]);
    });
    let at = |c: usize| rows.iter().skip(c).step_by(ROW).copied().collect::<Vec<f64>>();
    let (surface, amounts) = (at(3), w.store.amounts(Class::Air));
    let mut za = vec![0.0; na];
    cells.parent().iter().enumerate().for_each(|(i, &j)| za[j as usize] += surface[i] * cells.area()[i]);
    za.iter_mut().zip(air.area()).for_each(|(z, a)| *z /= a);
    let e_air = (0..na).map(|j| (0..ns).map(|k| amounts[k * na + j] * s.sp[k].mm).sum::<f64>() * g * za[j]).collect();
    Potential { ground: at(0), air: e_air, deep: at(1), bodies: at(5), heights: Heights { ground: at(2), air: za, deep: at(4), surface } }
}

/// The potential energy of the world's matter, J (`potential`), from Flow's state of its ground.
pub fn total(w: &World) -> f64 {
    let p = potential(w, &w.scratch);
    [&p.ground, &p.air, &p.deep, &p.bodies].iter().flat_map(|v| v.iter()).sum()
}

/// A part of Flow being booked: the potential energy of every node at its start, and what its moves carried into each
/// node so far, J (`carry`).
#[derive(Clone, Debug)]
pub(super) struct Lifts {
    start: Potential,
    carried: [Vec<f64>; 4],
}

/// The array of `Lifts::carried` and the index of node `index` of `class`, for a class that holds matter here.
fn slot(class: Class, index: usize) -> Option<(usize, usize)> {
    match class {
        Class::Ground | Class::Bed => Some((0, index)),
        Class::Air => Some((1, index)),
        Class::Deep => Some((2, index)),
        Class::Bodies => Some((3, index)),
        _ => None,
    }
}

impl Lifts {
    /// From the world now.
    pub(super) fn start(w: &World, s: &Scratch) -> Lifts {
        let start = potential(w, s);
        let carried = [start.ground.len(), start.air.len(), start.deep.len(), start.bodies.len()].map(|len| vec![0.0; len]);
        Lifts { start, carried }
    }
    /// `mol` of substance `k` (molar mass `mm`) go from node `from` to node `to`: they carry their potential energy at the
    /// height they leave from, as the part started.
    fn carry(&mut self, (from, to): ((Class, usize), (Class, usize)), mm: f64, mol: f64, g: f64) {
        let (src, dst) = if mol > 0.0 { (from, to) } else { (to, from) };
        let Some(h) = self.start.heights.of(src.0, src.1) else { return };
        let e = mol.abs() * mm * g * h;
        if let Some((c, i)) = slot(src.0, src.1) {
            self.carried[c][i] -= e;
        }
        if let Some((c, i)) = slot(dst.0, dst.1) {
            self.carried[c][i] += e;
        }
    }
    /// Book the part's lifts and falls: each node's heat takes what its moves carried in less what they carried out less
    /// the change of its own potential energy (`store::fall_heat`, which adds their sum to `Store::lifted`).
    pub(super) fn book(self, w: &mut World, s: &Scratch) {
        let now = potential(w, s);
        let (start, carried) = (self.start, self.carried);
        let olds = [start.ground, start.air, start.deep, start.bodies];
        let news = [now.ground, now.air, now.deep, now.bodies];
        for (((class, c), old), new) in [Class::Ground, Class::Air, Class::Deep, Class::Bodies].into_iter().zip(carried).zip(olds).zip(news) {
            let q: Vec<f64> = c.iter().zip(old.iter().zip(&new)).map(|(c, (o, n))| c - (n - o)).collect();
            fall_heat(w, class, &q);
        }
    }
}

/// Book `fluxes` and `moves` (`store::apply`), and what they carry for the part of Flow under way (`Lifts`).
pub(super) fn apply(w: &mut World, s: &mut Scratch, fluxes: &[LinkFlux], moves: &[Move]) {
    if let Some(l) = s.lifts.as_mut() {
        let g = w.cfg.gravity;
        for x in fluxes {
            let links = x.a.len();
            for (q, &k) in x.ks.iter().enumerate() {
                let mm = s.sp[k as usize].mm;
                for (lk, &mol) in x.mol[q * links..(q + 1) * links].iter().enumerate().filter(|p| *p.1 != 0.0) {
                    l.carry(((x.class[0], x.a[lk] as usize), (x.class[1], x.b[lk] as usize)), mm, mol, g);
                }
            }
        }
        for m in moves.iter().filter(|m| m.mol != 0.0) {
            l.carry(((m.from.class, m.from.index), (m.to.class, m.to.index)), s.sp[m.k as usize].mm, m.mol, g);
        }
    }
    store::apply(w, fluxes, moves);
}
