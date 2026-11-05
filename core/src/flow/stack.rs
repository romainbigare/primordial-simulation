//! The stack of a ground cell as the potential energy reads it (laws.md 0, "The books"; `potential`): the mass and
//! volume of each of its layers from what its nodes hold in their phases, the matter of its life at its head, and the
//! potential energy and heights they make.

use super::potential::{deep_of, ground_of};
use super::{land::NONE, scratch::Sp, Scratch};
use crate::{store::Class, World, AIR};

/// Numbers of a ground cell's row in `potential`: the potential energy of its stack and of its bed's store, J; the
/// heights its liquid and its store's liquid reach, m; and the potential energy of the matter at its head, J.
pub(super) const ROW: usize = 5;

/// What a ground cell's stack is made of: the mass, kg, and the volume, m³, of each layer (bed, loose solids, liquid),
/// then of its store, then the mass at its head, kg (its bodies' node's gas and its life).
pub(super) const LAYERS: usize = 9;
pub(super) const HEAD: usize = 8;

/// How a substance stands in every ground node now, where it stands alike in all: a gas (at or over its boiling point),
/// a liquid or a solid (under it, at or over its melting point or under it, on no node's plateau); else it varies.
#[derive(Clone, Copy)]
pub(super) enum Same {
    Gas,
    Liquid,
    Solid,
    Varies,
}

impl Same {
    /// Each substance, by Flow's state of the ground nodes (`ground_layers`).
    pub(super) fn of(s: &Scratch) -> Vec<Same> {
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
pub(super) fn bed_layer(x: &mut [f64; LAYERS], sp: &Sp, a: f64) {
    (x[0], x[3]) = (x[0] + a * sp.mm, x[3] + a * sp.v[0]);
}

/// `a` mol of substance `k` in ground node `i`, in its phase by the node's temperature and plateau (`State`).
#[inline]
pub(super) fn ground_layers(x: &mut [f64; LAYERS], s: &Scratch, (k, i): (usize, usize), a: f64) {
    let (sp, t) = (&s.sp[k], s.state.tg[i]);
    let wet = if t >= sp.boil { 1.0 } else if s.state.pk[i] == k { s.state.phi[i] } else { (t >= sp.melt) as u8 as f64 };
    let (wet_mol, dry_mol) = (a * wet, a - a * wet);
    (x[2], x[5]) = (x[2] + wet_mol * sp.mm, x[5] + if t < sp.boil { wet_mol * sp.v[1] } else { 0.0 });
    (x[1], x[4]) = (x[1] + dry_mol * sp.mm, x[4] + dry_mol * sp.v[0]);
}

/// `a` mol of substance `sp` in a store under ground at `t` K.
#[inline]
pub(super) fn deep_layer(x: &mut [f64; LAYERS], sp: &Sp, t: f64, a: f64) {
    (x[6], x[7]) = (x[6] + a * sp.mm, x[7] + if t < sp.boil { a * sp.v[1] } else { 0.0 });
}

/// The row of `potential` (`ROW`) of ground cell `i` made of `x` (`LAYERS`).
pub(super) fn stack(w: &World, x: &[f64; LAYERS], i: usize) -> [f64; ROW] {
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
pub(super) struct Life {
    body: Vec<usize>,
    cover: Option<[usize; crate::cover::NP]>,
}

impl Life {
    pub(super) fn of(w: &World) -> Life {
        Life { body: w.chem.body().subs().to_vec(), cover: crate::cover::Parts::of(&w.chem).map(|p| p.sub) }
    }
    /// The mass of the life of each cell (`kg`).
    pub(super) fn every(&self, w: &World, sp: &[Sp]) -> Vec<f64> {
        crate::par::map_n(w.mesh.cells().n(), crate::par::UNCOUNTED, |i| self.kg(w, sp, i))
    }
    /// The mass of the life of cell `i`, kg: its living bodies, its cover with the seeds it sends out until they land
    /// (`cover::land`), and its parts.
    pub(super) fn kg(&self, w: &World, sp: &[Sp], i: usize) -> f64 {
        let cell = w.life.cell_at(i);
        let mut kg = 0.0;
        for b in cell.bodies.iter().filter(|b| b.alive) {
            kg += self.body.iter().zip(&b.store).map(|(&k, &a)| a * sp[k].mm).sum::<f64>();
        }
        if let Some(sub) = self.cover {
            let (c, seeds) = (&cell.cover, cell.cover.seeds.iter().map(|s| &s.mol));
            kg += c.mol.iter().chain(seeds).flat_map(|m| m.iter().zip(sub)).map(|(&a, k)| a * sp[k].mm).sum::<f64>();
        }
        for p in &cell.parts {
            kg += p.wall.iter().zip(&p.fill).zip(sp).map(|((&a, &f), sp)| (a + f) * sp.mm).sum::<f64>();
        }
        kg
    }
}

/// The row of `potential` (`ROW`) of ground cell `i` alone, its nodes changed by `d(class, k)` mol of substance `k`
/// (its ground, its bed, its store and its bodies' node) and `life` kg at its head (the mass of its life, `Life::kg`):
/// its layers as `potential` adds them up, substance by substance, so the same numbers come out.
pub(super) fn row_of(w: &World, s: &Scratch, same: &[Same], i: usize, (d, life): (impl Fn(Class, usize) -> f64, f64)) -> [f64; ROW] {
    let (n, mut x) = (w.mesh.cells().n(), [0.0; LAYERS]);
    let at = |class: Class, k: usize| w.store.amounts(class)[k * n + i] + d(class, k);
    for (k, sp) in s.sp.iter().enumerate() {
        bed_layer(&mut x, sp, at(Class::Bed, k));
        ground_of(&mut x, s, same[k], (k, i), at(Class::Ground, k));
        deep_of(&mut x, s, same[k], (k, i), at(Class::Deep, k));
        x[HEAD] += at(Class::Bodies, k) * sp.mm;
    }
    x[HEAD] += life;
    stack(w, &x, i)
}

/// What the potential energy of the world's matter gains, J, when the nodes of ground cell `i` change by `d(class, k)`
/// mol of substance `k` (its ground, its bed, its store and its bodies' node) and the mass of its life by `life` kg, by
/// Flow's state of its ground (`potential`): its stack, its store and the matter at its head, and its air node's matter,
/// which lies at the mean head of its cells, so it rises by the cell's share of that node's area of what the head rises.
/// None before Flow first built the state of the ground (`potential::stated`).
pub(crate) fn gain(w: &World, i: usize, d: impl Fn(Class, usize) -> f64, life: f64) -> f64 {
    let (na, ns) = (w.mesh.level(AIR).n(), w.chem.ns());
    if !super::potential::stated(w) {
        return 0.0;
    }
    let s = super::potential::scratch_of(w);
    // (Each substance in its phase in this node: `Same::of` would read every node.)
    let (same, kg) = (vec![Same::Varies; ns], Life::of(w).kg(w, &s.sp, i));
    let (a, b) = (row_of(w, &s, &same, i, (d, kg + life)), row_of(w, &s, &same, i, (|_, _| 0.0, kg)));
    let (j, amounts) = (w.mesh.up(i, 0, AIR), w.store.amounts(Class::Air));
    let air: f64 = (0..ns).map(|k| amounts[k * na + j] * s.sp[k].mm).sum();
    (a[0] + a[1] + a[4]) - (b[0] + b[1] + b[4]) + air * w.cfg.gravity * (a[2] - b[2]) * w.mesh.share(i, AIR)
}
