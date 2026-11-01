//! The cover: the light-eaters as a field (laws.md 4). Each ground cell holds up to `SLOTS` kinds of cover, each the
//! amounts of a few substances (`Parts`: the fuel its light reaction makes, body matter, and the two solids it builds)
//! and a small vector of genes, as plain arrays per domain (`Cover`). Once a month each cell's cover catches light,
//! runs its light reaction, pays its upkeep, fixes, builds and dies (`grow`), and sends seeds to a neighbour cell
//! (`seed`); alike kinds share a slot. The cover of a cell is a node of the store (`Class::Cover`) whose law books its
//! own side; animals graze it (`choose::feed`) and movers break it (`choose::wear`).

mod grow;
mod seed;

pub use grow::step;
pub use seed::land;
pub(crate) use seed::Seed;

use crate::body::WET_TISSUE;
use crate::chem::Chemistry;
use crate::units::MAX_SUBSTANCES;
use serde::{Deserialize, Serialize};

/// Kinds of cover a cell holds at most.
pub const SLOTS: usize = 4;
/// The parts of the cover (`Parts`), by their place in `Cover::mol`: the fuel its light reaction makes, its body
/// matter, the solid it builds of the fuel alone (its walls) and the solid it builds of a mineral.
pub const NP: usize = 4;
pub const FUEL: usize = 0;
pub const MATTER: usize = 1;
pub const WALL: usize = 2;
pub const HARD: usize = 3;

// The genes of a kind of cover (laws.md 4.1).
/// Capacity of the light reaction, log10 mol per kg of built cover per s at `t_cat` (at a moisture need of 1)
pub const LIGHT: usize = 0;
/// Capture area, log10 m² per kg of built cover
pub const LEAF: usize = 1;
/// Walls, log10 kg of the solid built of fuel alone per kg of body matter
pub const WALLS: usize = 2;
/// Fuel kept, log10 kg per kg of built cover
pub const STORE: usize = 3;
/// The ground temperature below which its frozen share dies, K
pub const FROST: usize = 4;
/// The wetness of the ground at which it is fully active, log10 of the share (`Physics::wetness`)
pub const MOIST: usize = 5;
/// Share of what it builds that goes to seeds
pub const SEED: usize = 6;
/// Repair, log10, as a body's `REPAIR`
pub const REPAIR: usize = 7;
/// Capacity of the fixing reaction, log10 mol per kg of built cover per s at `t_cat`; off at or below `FIX_OFF`
pub const FIX: usize = 8;
pub const NG: usize = 9;
/// A fixing gene at or below this is off.
pub const FIX_OFF: f32 = -10.0;
/// Mutation step (times `Physics::cover_mut`) and range of each gene.
pub const RANGE: [(f32, f32, f32); NG] = [(0.3, -8.0, -2.0), (0.3, -2.0, 2.0), (0.2, -2.0, 2.0), (0.2, -3.0, 0.0), (5.0, 200.0, 300.0), (0.1, -2.0, 0.0), (0.05, 0.0, 1.0), (0.2, -3.0, 3.0), (0.3, -12.0, -6.0)];
/// A slot whose built cover is under this, kg, is empty.
pub const EMPTY_KG: f64 = 1e-9;
/// The root of an empty slot.
pub const NO_ROOT: u32 = u32::MAX;

/// The substances of the cover and the reactions it runs, found from the chemistry's numbers (`Chemistry::life_roles`).
#[derive(Clone, Copy, Debug)]
pub struct Parts {
    /// The substance of each part
    pub sub: [usize; NP],
    /// The light reaction, the burning of the fuel, the fixing reaction (if any), and the key liquid
    pub light: usize,
    pub burn: usize,
    pub fix: Option<usize>,
    pub liquid: usize,
}

impl Parts {
    /// The parts in `chem`: none unless it has the roles of life, body matter and two built solids, one built of the
    /// fuel alone.
    pub fn of(chem: &Chemistry) -> Option<Parts> {
        let ((light, rich, liquid, burn), matter) = (chem.life_roles()?, chem.matter()?);
        let alone = |b: &crate::chem::Reaction| b.inputs.iter().all(|p| p.0 as usize == rich);
        let wall = chem.builds.iter().find(|b| alone(b))?.outputs[0].0 as usize;
        let hard = chem.builds.iter().find(|b| !alone(b))?.outputs[0].0 as usize;
        Some(Parts { sub: [rich, matter, wall, hard], light, burn, fix: chem.fixing(), liquid })
    }
    /// The molar mass of each part, kg/mol.
    pub fn mm(&self, chem: &Chemistry) -> [f64; NP] {
        self.sub.map(|k| chem.subs[k].molar_mass)
    }
}

/// The cover of a domain's cells, slot `k * SLOTS + s` of its `k`th cell (by its order in `Domain::cells`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cover {
    /// Per slot: mol of each part
    pub mol: Vec<[f64; NP]>,
    /// Per slot: its genes, and the starting kind it descends from (a label for the tape; `NO_ROOT` when empty)
    pub genes: Vec<[f32; NG]>,
    pub root: Vec<u32>,
    /// Per slot: whether one more kg of it did not pay its upkeep in its last month (`grow::build`): seeds of its kind
    /// do not take hold there
    pub full: Vec<bool>,
    /// Per cell: the heat of its cover, J; the cell's light (`World::light`) at its last step, J/m²; the share of the
    /// light its cover absorbs that its bodies' node sets aside for its light reaction
    pub e: Vec<f64>,
    pub mark: Vec<f64>,
    pub keep: Vec<f32>,
    /// Seeds made in its cells' last step, landed after it (`land`); not saved
    #[serde(skip)]
    pub(crate) seeds: Vec<Seed>,
}

impl Cover {
    /// No cover on `cells` cells.
    pub fn new(cells: usize) -> Cover {
        let n = cells * SLOTS;
        Cover { mol: vec![[0.0; NP]; n], genes: vec![[0.0; NG]; n], root: vec![NO_ROOT; n], full: vec![false; n], e: vec![0.0; cells], mark: vec![0.0; cells], keep: vec![0.0; cells], seeds: Vec::new() }
    }
    /// Whether it holds any cover.
    pub fn any(&self) -> bool {
        self.root.iter().any(|&r| r != NO_ROOT)
    }
    /// The slots of the `k`th cell.
    pub fn slots(k: usize) -> std::ops::Range<usize> {
        k * SLOTS..(k + 1) * SLOTS
    }
    /// The built cover of slot `x` (body matter and solids), kg.
    pub fn built(&self, mm: &[f64; NP], x: usize) -> f64 {
        built_kg(mm, &self.mol[x])
    }
    /// All the cover of slot `x`, kg.
    pub fn kg(&self, mm: &[f64; NP], x: usize) -> f64 {
        self.built(mm, x) + self.mol[x][FUEL] * mm[FUEL]
    }
    /// Its capture area, m².
    pub fn leaf(&self, mm: &[f64; NP], x: usize) -> f64 {
        dec(self.genes[x][LEAF]) * self.built(mm, x)
    }
    /// The mol of substance `k` the cover of the `k`th cell holds (0 for a substance it is not made of).
    pub fn amount(&self, p: &Parts, cell: usize, k: usize) -> f64 {
        p.sub.iter().position(|&s| s == k).map_or(0.0, |q| Cover::slots(cell).map(|x| self.mol[x][q]).sum())
    }
    /// The heat capacity of the cover of the `k`th cell, J/K.
    pub fn cap(&self, chem: &Chemistry, p: &Parts, k: usize) -> f64 {
        Cover::slots(k).map(|x| (0..NP).map(|q| self.mol[x][q] * chem.subs[p.sub[q]].cp).sum::<f64>()).sum()
    }
    /// The temperature of the cover of the `k`th cell, K (0 for none).
    pub fn temp(&self, chem: &Chemistry, p: &Parts, k: usize) -> f64 {
        let c = self.cap(chem, p, k);
        if c > 0.0 { self.e[k] / c } else { 0.0 }
    }
    /// The energy to break the share `f` of the cover of the `k`th cell, J: its soft tissue (as a body's, with its
    /// liquid) and its solids by their breaking stresses (`body::solids_of`).
    pub fn breaking(&self, chem: &Chemistry, cfg: &crate::Config, p: &Parts, k: usize, f: f64) -> f64 {
        let sum = |q: usize| Cover::slots(k).map(|x| self.mol[x][q]).sum::<f64>();
        let built = sum(MATTER) * chem.subs[p.sub[MATTER]].molar_mass / (1.0 - WET_TISSUE);
        f * crate::body::solids_of(chem, cfg, built, |s| p.sub.iter().position(|&q| q == s).map_or(0.0, sum))[2]
    }
    /// The reflectance of the cover of slot `x`: the mean of its solid parts' three bands, by volume.
    pub fn reflectance(&self, chem: &Chemistry, p: &Parts, x: usize) -> f64 {
        let (mut v, mut r) = (1e-30, 0.0);
        for (q, &k) in p.sub.iter().enumerate() {
            let s = &chem.subs[k];
            let vol = self.mol[x][q] * s.molar_mass / s.density[0];
            (v, r) = (v + vol, r + vol * s.refl[0].iter().map(|&b| b as f64).sum::<f64>() / 3.0);
        }
        r / v
    }
    /// `mol` of substance `k` taken from the cover of the `k`th cell, from its slots in proportion to what each holds
    /// of it: returns the heat that goes with it, J (at the cover's temperature).
    pub(crate) fn take(&mut self, chem: &Chemistry, p: &Parts, cell: usize, k: usize, mol: f64) -> f64 {
        let Some(q) = p.sub.iter().position(|&s| s == k) else { return 0.0 };
        let all = self.amount(p, cell, k);
        if all <= 0.0 || mol <= 0.0 {
            return 0.0;
        }
        let (f, t) = ((mol / all).min(1.0), self.temp(chem, p, cell));
        Cover::slots(cell).for_each(|x| self.mol[x][q] -= self.mol[x][q] * f);
        let heat = (mol.min(all) * chem.subs[k].cp * t).min(self.e[cell]);
        self.e[cell] -= heat;
        heat
    }
    /// The amounts of the cover of the `k`th cell by substance, mol (`MAX_SUBSTANCES`).
    pub fn amounts(&self, p: &Parts, k: usize) -> [f64; MAX_SUBSTANCES] {
        let mut a = [0.0; MAX_SUBSTANCES];
        for x in Cover::slots(k) {
            (0..NP).for_each(|q| a[p.sub[q]] += self.mol[x][q]);
        }
        a
    }
}

/// The share `f` of the cover of the `k`th cell of `d` breaks: it goes to the cell's ground as remains, with its heat,
/// through the domain's moves (opened by the caller for the cover's node).
pub(crate) fn lose(w: &crate::World, d: &mut crate::life::Domain, p: &Parts, k: usize, f: f64) {
    let (i, c) = (d.cells[k] as usize, &mut d.cover);
    let t = c.temp(&w.chem, p, k);
    let mut dead = [0.0; NP];
    for x in Cover::slots(k) {
        (0..NP).for_each(|q| dead[q] += c.mol[x][q] * f);
        c.mol[x].iter_mut().for_each(|a| *a -= *a * f);
    }
    let heat = ((0..NP).map(|q| dead[q] * w.chem.subs[p.sub[q]].cp).sum::<f64>() * t).min(c.e[k]);
    c.e[k] -= heat;
    let ground = crate::store::Node::ground(i);
    (0..NP).for_each(|q| d.moves.give(w, ground, p.sub[q], dead[q], 0.0));
    d.moves.give(w, ground, 0, 0.0, heat);
}

/// Place `kg` per m² of cover of genes `genes` and starting kind `root` in cell `i`, an edit booked from the outside
/// node (`World::cross`): its built cover at the shares of its genes, its fuel at its `STORE` gene, at the cell's
/// ground temperature; it settles as seeds do (`seed`). False if it found no slot (nothing is placed then).
pub fn place(w: &mut crate::World, i: usize, genes: [f32; NG], root: u32, kg: f64) -> bool {
    let Some(p) = Parts::of(&w.chem) else { return false };
    let (mm, t, alike) = (p.mm(&w.chem), w.t_ground_last(i), w.cfg.phys.alike);
    let fm = 1.0 / (1.0 + dec(genes[WALLS]) + w.cfg.phys.cover_hard);
    let built = kg * w.mesh.cells().area()[i] / (1.0 + dec(genes[STORE]));
    let mol = [built * dec(genes[STORE]) / mm[FUEL], built * fm / mm[MATTER], built * fm * dec(genes[WALLS]) / mm[WALL], built * fm * w.cfg.phys.cover_hard / mm[HARD]];
    let heat = (0..NP).map(|q| mol[q] * w.chem.subs[p.sub[q]].cp).sum::<f64>() * t;
    let (d, k) = w.life.cell_slot(i);
    let s = Seed { to: i as u32, mol, genes, root, heat };
    let c = &mut w.life.domains[d].cover;
    let was = c.temp(&w.chem, &p, k);
    match seed::settle(c, &mm, k, &s, alike) {
        seed::Landed::Dead => return false,
        seed::Landed::In => {}
        // What it outweighed goes to the ground as remains, with its heat.
        seed::Landed::Out(m) => {
            let q = ((0..NP).map(|q| m[q] * w.chem.subs[p.sub[q]].cp).sum::<f64>() * was).min(c.e[k]);
            c.e[k] -= q;
            let (from, to) = (crate::store::Node::new(crate::store::Class::Cover, i), crate::store::Node::ground(i));
            let mut moves: Vec<_> = (0..NP).map(|x| crate::store::Move { from, to, k: p.sub[x] as u16, mol: m[x], heat: 0.0 }).collect();
            moves.push(crate::store::Move { from, to, k: 0, mol: 0.0, heat: q });
            crate::store::apply(w, &[], &moves);
        }
    }
    // (The cover's side is its own: the store books the outside node's.)
    w.cross((crate::store::Boundary::Outside, crate::store::Node::new(crate::store::Class::Cover, i)), (0..NP).map(|q| (p.sub[q], mol[q])), heat);
    w.life.domains[d].cover.mark[k] = w.light[i];
    true
}

/// The built cover of amounts `m` (mol of each part): body matter and solids, kg.
pub fn built_kg(mm: &[f64; NP], m: &[f64; NP]) -> f64 {
    m[MATTER] * mm[MATTER] + m[WALL] * mm[WALL] + m[HARD] * mm[HARD]
}

/// Genes `g` after one generation's mutation of size `size` (`Physics::cover_mut`): each moves by noise of its step
/// times that, within its range.
pub fn mutate(g: &[f32; NG], size: f32, r: &mut crate::rng::Rng) -> [f32; NG] {
    std::array::from_fn(|q| (g[q] + crate::body::noise(r) * RANGE[q].0 * size).clamp(RANGE[q].1, RANGE[q].2))
}

/// The cover of one starting kind (`Cover::root`), summed over the world for the shell: its slots, its cover (kg), its
/// built cover (kg), its capture area (m²), and its genes summed by built cover.
#[derive(Clone, Debug, Default)]
pub struct Sum {
    pub slots: f64,
    pub kg: f64,
    pub built: f64,
    pub leaf: f64,
    pub genes: [f64; NG],
}

/// The cover of the world by starting kind (index: the root), in the order of the domains and their slots.
pub fn sums(w: &crate::World) -> Vec<Sum> {
    let Some(p) = Parts::of(&w.chem) else { return Vec::new() };
    let mm = p.mm(&w.chem);
    let mut out: Vec<Sum> = Vec::new();
    for c in w.life.domains.iter().map(|d| &d.cover) {
        for x in (0..c.root.len()).filter(|&x| c.root[x] != NO_ROOT) {
            let r = c.root[x] as usize;
            if r >= out.len() {
                out.resize(r + 1, Sum::default());
            }
            let (s, built) = (&mut out[r], c.built(&mm, x));
            (s.slots, s.kg, s.built, s.leaf) = (s.slots + 1.0, s.kg + c.kg(&mm, x), s.built + built, s.leaf + c.leaf(&mm, x));
            s.genes.iter_mut().zip(c.genes[x]).for_each(|(a, g)| *a += built * g as f64);
        }
    }
    out
}

/// Each slot of cover in the world, in the order of the domains: its starting kind and its genes.
pub fn slots(w: &crate::World) -> impl Iterator<Item = (u32, &[f32; NG])> {
    w.life.domains.iter().flat_map(|d| d.cover.root.iter().zip(&d.cover.genes)).filter(|x| *x.0 != NO_ROOT).map(|(&r, g)| (r, g))
}

/// The cover of the whole world by substance, mol (`MAX_SUBSTANCES`).
pub fn totals(w: &crate::World) -> [f64; MAX_SUBSTANCES] {
    let mut a = [0.0; MAX_SUBSTANCES];
    let Some(p) = Parts::of(&w.chem) else { return a };
    for c in w.life.domains.iter().map(|d| &d.cover) {
        c.mol.iter().for_each(|m| (0..NP).for_each(|q| a[p.sub[q]] += m[q]));
    }
    a
}

/// The cover of each ground cell, kg per m².
pub fn per_m2(w: &crate::World) -> Vec<f64> {
    let mut out = vec![0.0; w.mesh.cells().n()];
    let Some(p) = Parts::of(&w.chem) else { return out };
    let (mm, area) = (p.mm(&w.chem), w.mesh.cells().area());
    for d in &w.life.domains {
        for (k, &i) in d.cells.iter().enumerate() {
            out[i as usize] = Cover::slots(k).map(|x| d.cover.kg(&mm, x)).sum::<f64>() / area[i as usize];
        }
    }
    out
}

/// 10 to the power of a gene.
#[inline]
pub fn dec(g: f32) -> f64 {
    (g as f64 * std::f64::consts::LN_10).exp()
}

/// How far apart two gene vectors are, in steps: the largest gap of a gene over its step.
/// A fixing gene that is off counts as `FIX_OFF`, where differences do nothing.
pub fn distance(a: &[f32; NG], b: &[f32; NG]) -> f32 {
    let at = |v: &[f32; NG], g: usize| if g == FIX { v[g].max(FIX_OFF) } else { v[g] };
    (0..NG).map(|g| (at(a, g) - at(b, g)).abs() / RANGE[g].0).fold(0.0, f32::max)
}
