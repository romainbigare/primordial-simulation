//! The stack of a ground cell as the potential energy reads it (laws.md 0, "The books"; `potential`): the mass and
//! volume of each of its layers from what its nodes hold in their phases, its liquid in its pores and over its own
//! ground (`Column`), the matter of its life at its head, and the potential energy and heights they make.

use super::curve::{with_fill, Fill};
use super::potential::{deep_of, ground_of};
use super::{land::NONE, scratch::Sp, Scratch};
use crate::{store::Class, World, AIR};

/// Numbers of a ground cell's row in `potential`: the potential energy of its stack and of its bed's store, J; its head
/// (`Column::head`), m; the height its store's liquid reaches, m; the potential energy of the matter at its head, J; and
/// the height its liquid leaves from (`Column::top`), m.
pub(super) const ROW: usize = 6;
/// Where `ROW` holds the height the cell's liquid leaves from.
pub(super) const LEAVES: usize = 5;

/// What a ground cell's stack is made of: the mass, kg, and the volume, m³, of each layer (bed, solids, liquid), then
/// of its store, then the mass at its head, kg (its aside node's gas and its life), then the volume of its loose
/// solids, m³ (those not soft, whose pores take liquid, `Sp::soft`).
pub(super) const LAYERS: usize = 10;
pub(super) const HEAD: usize = 8;
pub(super) const PORED: usize = 9;

/// A cell's liquid as its stack holds it (laws.md 0, 1.2): `vol` m³ fill the pores of its loose solids first (`loose`
/// m³ of them, porosity `por`; of a cell of parts less `dry` m³, the room its land's pores have left, `parts::Parts`),
/// a table rising from the bed's top `bed` by the depth they fill over the porosity; the rest stands over its own ground
/// (`curve::Fill`) from its mean solid surface `z` (m), as Flow's level stands it (`land::stand`). Its area `area`,
/// m².
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Column {
    pub(super) bed: f64,
    pub(super) z: f64,
    pub(super) area: f64,
    pub(super) por: f64,
    pub(super) loose: f64,
    pub(super) dry: f64,
    pub(super) vol: f64,
}

impl Column {
    /// Of ground cell `i` of `w` made of `x` (`LAYERS`), by Flow's state `s`.
    pub(super) fn of(w: &World, s: &Scratch, x: &[f64; LAYERS], i: usize) -> Column {
        let area = w.mesh.cells().area()[i];
        let bed = w.base_z + x[3] / area;
        Column { bed, z: bed + x[4] / area, area, por: w.cfg.phys.porosity, loose: x[PORED], dry: s.dry(i), vol: x[5] }
    }
    /// What its pores hold and what stands over them, m³.
    fn parts(&self) -> [f64; 2] {
        let pore = self.vol.min((self.por * self.loose - self.dry).max(0.0));
        [pore, self.vol - pore]
    }
    /// The table of `pore` m³ in its pores, m.
    fn table(&self, pore: f64) -> f64 {
        self.bed + if pore > 0.0 { pore / (self.por * self.area) } else { 0.0 }
    }
    /// The height its liquid leaves from, its top, m: the table of its pores, or the level of what stands over its own
    /// ground (`fill`) where that lies higher. What stands in a pit of its ground under
    /// the table of its full pores lies under liquid that fills the ground round it, and leaves with it: so the top
    /// never rises as liquid leaves, and liquid leaves the cell, and sinks into its store, from no lower than its
    /// pores' table.
    pub(super) fn top(&self, fill: Fill) -> f64 {
        match self.parts() {
            [pore, stands] if stands > 0.0 => (self.z + fill.level(stands)).max(self.table(pore)),
            [pore, _] => self.table(pore),
        }
    }
    /// Its head, m: its solid surface and its liquid as a column of its area, as Flow's state stacks them (`FlowView`),
    /// the ground the air over it and its life stand on.
    pub(super) fn head(&self) -> f64 {
        self.z + self.vol / self.area
    }
    /// The potential energy of its liquid of mass `kg`, J, under gravity `g`: what its pores hold at the middle of the
    /// depth it fills, and what stands at its centroid over its own ground (`Fill::moment`); its mass at its top where
    /// it fills no volume (a gas).
    pub(super) fn energy(&self, kg: f64, g: f64, fill: Fill) -> f64 {
        if self.vol <= 0.0 {
            return kg * g * self.top(fill);
        }
        let [pore, stands] = self.parts();
        let mut moment = pore * (self.bed + self.table(pore)) / 2.0;
        if stands > 0.0 {
            moment += stands * self.z + fill.moment(stands);
        }
        kg * g * moment / self.vol
    }
}

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
    solid_layer(x, sp, t, dry_mol);
}

/// `a` mol of the solid of substance `sp` in a ground node at `t` K: loose, its pores taking liquid, unless soft.
#[inline]
pub(super) fn solid_layer(x: &mut [f64; LAYERS], sp: &Sp, t: f64, a: f64) {
    let v = a * sp.v[0];
    (x[1], x[4]) = (x[1] + a * sp.mm, x[4] + v);
    if t <= sp.soft {
        x[PORED] += v;
    }
}

/// `a` mol of substance `sp` in a store under ground at `t` K.
#[inline]
pub(super) fn deep_layer(x: &mut [f64; LAYERS], sp: &Sp, t: f64, a: f64) {
    (x[6], x[7]) = (x[6] + a * sp.mm, x[7] + if t < sp.boil { a * sp.v[1] } else { 0.0 });
}

/// The row of `potential` (`ROW`) of ground cell `i` made of `x` (`LAYERS`): its bed and its solids each at its middle,
/// its liquid in its pores and over its own ground (`Column`, the ground of `s`, `curve::with_fill`), and its life at its
/// head.
pub(super) fn stack(w: &World, s: &Scratch, x: &[f64; LAYERS], i: usize) -> [f64; ROW] {
    let (area, g, p) = (w.mesh.cells().area()[i], w.cfg.gravity, w.cfg.phys);
    let c = Column::of(w, s, x, i);
    let solids = x[0] * g * (w.base_z + c.bed) / 2.0 + x[1] * g * (c.bed + c.z) / 2.0;
    let (liquid, top) = with_fill(w, s, i, |f| (c.energy(x[2], g, f), c.top(f)));
    let (e, head) = (solids + liquid, c.head());
    let (floor, fill) = (w.base_z + x[3] / area - p.bed_depth, x[7] / (p.bed_porosity * area));
    [e, x[6] * g * (floor + fill / 2.0), head, floor + fill, x[HEAD] * g * head, top]
}

/// The matter of the cells' life, which lies at their heads: the substances of life's parts.
pub(super) struct Life {
    parts: Option<crate::life::Parts>,
}

impl Life {
    pub(super) fn of(w: &World) -> Life {
        Life { parts: crate::life::Parts::of(&w.chem) }
    }
    /// The mass of the life of each cell (`kg`).
    pub(super) fn every(&self, w: &World, sp: &[Sp]) -> Vec<f64> {
        crate::par::map_n(w.mesh.cells().n(), crate::par::UNCOUNTED, |i| self.kg(w, sp, i))
    }
    /// The mass of the life of cell `i`, kg: its cover, the seeds lying on its ground, the seeds it sends out until
    /// they land (`cover::land`), and its takers.
    pub(super) fn kg(&self, w: &World, sp: &[Sp], i: usize) -> f64 {
        let c = w.life.cover(i);
        let mut kg = 0.0;
        if let Some(p) = &self.parts {
            kg += c.all_mol().chain(c.seeds.iter().map(|s| &s.mol)).flat_map(|m| m.iter().zip(p.sub)).map(|(&a, k)| a * sp[k].mm).sum::<f64>();
            kg += w.life.cell_at(i).takers_kg(p, |k| sp[k].mm);
        }
        kg
    }
}

/// The row of `potential` (`ROW`) of ground cell `i` alone, its nodes changed by `d(class, k)` mol of substance `k`
/// (its ground, its bed, its store and its aside node) and `life` kg at its head (the mass of its life, `Life::kg`):
/// its layers as `potential` adds them up, substance by substance, so the same numbers come out.
pub(super) fn row_of(w: &World, s: &Scratch, same: &[Same], i: usize, change: (impl Fn(Class, usize) -> f64, f64)) -> [f64; ROW] {
    stack(w, s, &layers_of(w, s, same, i, change), i)
}

/// The layers (`LAYERS`) of ground cell `i` alone, changed as `row_of` says.
pub(super) fn layers_of(w: &World, s: &Scratch, same: &[Same], i: usize, (d, life): (impl Fn(Class, usize) -> f64, f64)) -> [f64; LAYERS] {
    let (n, mut x) = (w.mesh.cells().n(), [0.0; LAYERS]);
    let at = |class: Class, k: usize| w.store.amounts(class)[k * n + i] + d(class, k);
    for (k, sp) in s.sp.iter().enumerate() {
        bed_layer(&mut x, sp, at(Class::Bed, k));
        ground_of(&mut x, s, same[k], (k, i), at(Class::Ground, k));
        deep_of(&mut x, s, same[k], (k, i), at(Class::Deep, k));
        x[HEAD] += at(Class::Aside, k) * sp.mm;
    }
    x[HEAD] += life;
    x
}

/// What the potential energy of the world's matter gains, J, when the nodes of ground cell `i` change by `d(class, k)`
/// mol of substance `k` (its ground, its bed, its store and its aside node) and the mass of its life by `life` kg, by
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
