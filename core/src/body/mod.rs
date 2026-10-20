//! Bodies: nodes that hold substances, heat and genes, with stable ids (slot and generation). A body updates every
//! `interval` days and integrates the time since at its update (`live`; laws.md 6).

mod death;
mod exchange;
mod genes;
mod intake;
mod live;
mod metab;
mod pay;
mod physics;
mod tissue;

pub use death::{die, wound};
use death::thin;
pub(crate) use death::{to_ground, torn_to_death};
pub use exchange::conductance_to_ground;
pub use genes::*;
pub(crate) use live::{gain_to_growth, live, LiveTmp};
pub use live::Lived;
pub use pay::{pay, plan, upkeep, Payer};
pub(crate) use pay::spend;

use crate::brain::{ln, SIG};
use crate::chem::{Chemistry, Phase};
use crate::ground::{melting, Melting};
use crate::store::NodeId;
use crate::units::MAX_SUBSTANCES;
use serde::{Deserialize, Serialize};

/// Death causes: a store below its safe minimum; a key liquid froze or boiled; upkeep not paid; taken by a body.
pub const LACK: u8 = 0;
pub const PHASE: u8 = 1;
pub const SPENT: u8 = 2;
pub const TAKEN: u8 = 3;
/// Bonds broken by bodies moving through its cell.
pub(crate) const BROKEN: u8 = 4;
/// Worn out by living (its wear, `body::live`).
pub const AGE: u8 = 5;
/// Margins of well-being (`Body::wellbeing_at`) count from `MARGIN_MIN` up to this: a key store's log share over its
/// minimum, and its reactions over its upkeep (`live` plans for this many upkeeps).
pub(crate) const MARGIN: f64 = 2.0;
const MARGIN_MIN: f64 = 1e-3;
/// Well-being falls as a key liquid comes within this of freezing or boiling, K.
const PHASE_MARGIN: f32 = 5.0;
/// The liquid share of living tissue: a kg of body matter makes 1 / (1 - this) kg of working body (`Body::built`).
pub const WET_TISSUE: f64 = 0.75;
/// Others a body remembers.
pub(crate) const KNOWN: usize = 4;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Body {
    pub gen: u32,
    pub alive: bool,
    pub cell: u32,
    /// Members this node stands for (1: an individual). Stores and heat are the members' totals, genes and brain their
    /// mean, so every law runs on the totals. Use `n()`.
    pub count: f64,
    pub lineage: u32,
    /// The body it was copied from (id; 0 for a founder) and its day of birth
    pub parent: u64,
    pub born: u64,
    /// Day of the last update; for a node, the mean by members of the days its members' time was last integrated from
    /// (members who joined since count from when they came, `nodes::add`)
    pub last: f64,
    /// The cell's cumulative light at the last update, J/m², moved since by what its shade gave
    /// it more or less than the cell's mean (`survey::shade`): it caught `light - mark` per m²
    pub mark: f64,
    /// The cell's cumulative rain at the last update, mol/m²: it caught a share of `rained - this` (`physics::rain_caught`)
    pub rained: f64,
    /// The share of the light it caught that its light reactions stored at its last update: its cell sets that share
    /// aside in its bodies' node (`store::Class::Bodies`)
    pub keep: f32,
    /// The heat it draws from the ground between updates, set aside in its cell's bodies' node (`survey::Draw`)
    pub draw: crate::survey::Draw,
    /// Thermal energy, J
    pub e: f64,
    /// mol of each substance
    pub store: Vec<f64>,
    pub genes: Vec<f32>,
    /// How a node's members differ: the variance of each gene around `genes` (empty: none), and of their born rows, one
    /// number for every row
    pub spread: Vec<f32>,
    pub row_spread: f32,
    /// The variance of log10 of its members' bodies, decades² (`Body::size_moments`, `copy::young`)
    pub size_spread: f32,
    /// How far the mean of log10 of its members' bodies lies below log10 of its mean member's body, decades
    /// (`Body::size_moments`): 0 when all are of one size
    pub size_lag: f32,
    /// The node of its kind in its cell at the other stage (young or adult; its id), along which births and maturing
    /// members flow (`copy::young`)
    pub link: Option<u64>,
    /// The brain rows it was born with (genes too), in `Life::genomes`
    pub genome: u32,
    /// Key stores (safe minimum > 0) that were liquid at birth, one bit per substance
    pub wet: u64,
    /// Body matter built for children and held back, mol (part of `store`, not of the body: `size`, `built`; laws.md 4.2)
    pub brood: f64,
    /// Visible traits: mass kg, light-capture area m², strength, colour (3 bands), height m (`height`)
    pub vis: [f32; 7],
    /// Its brain rows: the rows it was born with plus what it (a node: its members) learned; empty without a brain
    pub rows: Vec<f32>,
    /// Moving average of the square of its critic's error (0 until the first one), and per learned row a moving average
    /// of the square of its share of the learning steps (`brain::learn`)
    pub var: f32,
    pub ms: Vec<f32>,
    /// The part it holds, and a part of the structure it is inside
    pub hold: Option<NodeId>,
    pub inside: Option<NodeId>,
    /// Memory the brain reads and writes; the signal emitted at the last decision
    pub mem: Vec<f32>,
    pub sig: [f32; SIG],
    /// Others it knows (id) and how dealings with them went (-1 worse .. 1 better)
    pub known: [(u64, f32); KNOWN],
    /// Well-being at the last decision; the trace of that decision for learning
    pub wb: f32,
    pub trace: Vec<f32>,
    /// Numbers that follow from the genes (`prepare`); not saved
    #[serde(skip)]
    pub k: Consts,
}

/// Numbers that follow from a body's genes, kept to save time.
#[derive(Clone, Debug, Default)]
pub struct Consts {
    /// 10 to the power of the gas, liquid, heat and light-capture genes
    pub gas: f64,
    pub liq: f64,
    pub heat: f64,
    pub leaf: f64,
    /// Catalyst factor of each reaction (1 when off)
    pub cat: Vec<f64>,
    /// Days between the updates its `INTERVAL` gene asks for (`Body::interval`)
    pub interval: u64,
    /// Catalysed reactions that release heat, with the heat per unit, J (not those that keep a part of a fuel's heat,
    /// `Chemistry::keeps`)
    pub burn: Vec<(usize, f64)>,
    /// The substances taken, by selectivity
    pub upt: Vec<(usize, f32)>,
    /// Its catalysts' capacity (`upkeep`; an exponential per catalyst, so kept)
    pub cats: f64,
}

/// Store amount `x` of a body changes by `d` mol; what rounding leaves below zero is set to zero and counted, as
/// `store::apply` does (a larger overdraw is a law's fault, checked in debug builds).
pub(crate) fn add_store(x: &mut f64, d: f64, rounded: &mut f64) {
    let was = *x;
    *x += d;
    if *x < 0.0 {
        debug_assert!(-*x <= crate::store::ROUNDING * (was + d.abs()), "a law overdrew a body's store: {was} by {d}");
        *rounded -= std::mem::take(x);
    }
}

impl Body {
    /// Members (at least 1).
    #[inline]
    pub fn n(&self) -> f64 {
        self.count.max(1.0)
    }
    pub fn id(&self, slot: usize) -> u64 {
        (self.gen as u64) << 32 | slot as u64
    }
    #[inline]
    pub fn gene(&self, g: usize) -> f64 {
        self.genes[g] as f64
    }
    /// 10 to the power of a gene.
    #[inline]
    pub fn dec(&self, g: usize) -> f64 {
        (self.gene(g) * std::f64::consts::LN_10).exp()
    }
    pub fn mass(&self, chem: &Chemistry) -> f64 {
        self.store.iter().zip(&chem.subs).map(|(a, s)| a * s.molar_mass).sum()
    }
    pub fn heat_cap(&self, chem: &Chemistry) -> f64 {
        chem.heat_cap_of(&self.store)
    }
    /// `mass` and `heat_cap` in one pass over the stores (the same sums).
    pub(crate) fn mass_cap(&self, chem: &Chemistry) -> (f64, f64) {
        self.store.iter().zip(&chem.subs).fold((0.0, 0.0), |(m, c), (a, s)| (m + a * s.molar_mass, c + a * s.cp))
    }
    /// Its temperature, K (`thermal`).
    pub fn temp(&self, chem: &Chemistry) -> f64 {
        self.thermal(chem).t
    }
    /// Where the body stands among the melting plateaus of its stores (`ground::melting`): its molten stores hold their
    /// fusion enthalpy in `e`, each melting at its point lowered by what is dissolved in it (`melt`). What is dissolved
    /// only lowers a point, so the solve at the stores' own points holds when every such store is molten above it.
    pub fn thermal(&self, chem: &Chemistry) -> Melting {
        let (held, cap) = (|k: usize| self.store[k] > 0.0 && chem.subs[k].lf > 0.0, self.heat_cap(chem));
        let own = self.melting_at(chem, cap, |k| chem.subs[k].melt);
        if (0..chem.ns()).all(|k| !held(k) || chem.mixes(k).is_none() || own.t > chem.subs[k].melt) {
            return own;
        }
        self.melting_at(chem, cap, |k| self.melt(chem, k))
    }
    /// `thermal` with each store melting at `melt(k)`.
    fn melting_at(&self, chem: &Chemistry, cap: f64, melt: impl Fn(usize) -> f64) -> Melting {
        let (mut ks, mut m) = ([(0.0, 0usize); MAX_SUBSTANCES], 0);
        for k in (0..chem.ns()).filter(|&k| self.store[k] > 0.0 && chem.subs[k].lf > 0.0) {
            ks[m] = (melt(k), k);
            m += 1;
        }
        ks[..m].sort_by(|a, b| a.0.total_cmp(&b.0));
        melting(cap.max(1e-9), self.e, ks[..m].iter().map(|&(tm, k)| (tm, k, self.store[k] * chem.subs[k].lf)))
    }
    /// Its energy at the plateau state `th`: heat capacity times `th.t` plus the fusion enthalpy molten there, J.
    pub(crate) fn energy_in(&self, chem: &Chemistry, th: &Melting) -> f64 {
        self.heat_cap(chem) * th.t + self.fusion_in(chem, th)
    }
    /// The fusion enthalpy its stores hold at the plateau state `th`, J.
    pub(crate) fn fusion_in(&self, chem: &Chemistry, th: &Melting) -> f64 {
        self.fused_of(chem, th, &self.store)
    }
    /// The fusion enthalpy amounts `m` hold at the plateau state `th` (a store `th` does not know, by its melting point
    /// now), J.
    fn fused_of(&self, chem: &Chemistry, th: &Melting, m: &[f64]) -> f64 {
        let molten_at = |k: usize| th.t >= chem.subs[k].melt || th.t >= self.melt(chem, k);
        let molten = |k: usize| if th.bits >> k & 1 == 1 { th.molten(k) } else { molten_at(k) as u8 as f64 };
        m.iter().enumerate().filter(|&(k, &a)| a > 0.0 && chem.subs[k].lf > 0.0).map(|(k, a)| a * chem.subs[k].lf * molten(k)).sum()
    }
    /// Its energy at `t`, what is at or over its melting point molten, J: the inverse of `temp` off a plateau.
    pub fn energy_at(&self, chem: &Chemistry, t: f64) -> f64 {
        self.heat_cap(chem) * t + (0..chem.ns()).map(|k| self.store[k] * chem.fused_heat(k, t, |j| self.store[j])).sum::<f64>()
    }
    /// The heat that leaves with amounts `m` (mol) at the plateau state `th`, with their molten shares' fusion enthalpy, J.
    pub(crate) fn heat_with(&self, chem: &Chemistry, th: &Melting, m: &[f64]) -> f64 {
        chem.heat_cap_of(m) * th.t + self.fused_of(chem, th, m)
    }
    /// Volume of its solids and liquids at its temperature `t`, m³.
    pub fn volume(&self, chem: &Chemistry, t: f64) -> f64 {
        self.store.iter().enumerate().map(|(k, a)| a * chem.molar_volume(k, chem.phase_in(k, t, |j| self.store[j]))).sum()
    }
    /// Days between its updates: its `INTERVAL` gene, and at least `Config::node_days` for a population node without a
    /// brain, which takes no decisions (laws.md 4.4, the pace of nodes).
    pub fn interval(&self, w: &crate::World) -> u64 {
        let own = self.k.interval;
        if self.n() > 1.0 && w.life.brainless(self) { own.max(w.cfg.node_days as u64) } else { own }
    }
    /// How many of the intervals its `INTERVAL` gene asks for one update spans (1 but for a node that updates less often).
    pub(crate) fn paces(&self, w: &crate::World) -> f64 {
        self.interval(w) as f64 / self.k.interval as f64
    }
    /// The share that dies in an update when `f` would die in one interval of its gene: the rate -ln(1 - f) per
    /// interval over `paces` intervals, so as many die in a time whatever the length of the updates.
    pub(crate) fn over_span(&self, f: f64, w: &crate::World) -> f64 {
        let d = self.paces(w);
        if d == 1.0 { f } else { 1.0 - (1.0 - f).powf(d) }
    }
    /// Its genes are `genes` (`prepare` follows).
    pub(crate) fn set_genes(&mut self, genes: Vec<f32>, chem: &Chemistry, p: &crate::Physics) {
        self.genes = genes;
        self.prepare(chem, p);
    }
    /// Work `k` out from the genes and the physics, at every change of the genes.
    pub fn prepare(&mut self, chem: &Chemistry, p: &crate::Physics) {
        let (lay, nr) = (Layout::of(chem), chem.reactions.len());
        let (cat, on) = (lay.cat, |r: usize| self.genes[lay.cat + r] > 0.0);
        let mut k = std::mem::take(&mut self.k);
        k.cat.clear();
        k.cat.extend((0..nr).map(|r| if on(r) { self.dec(cat + r) } else { 1.0 }));
        k.burn.clear();
        k.burn.extend(chem.burners().iter().filter(|x| on(x.0)));
        k.upt.clear();
        k.upt.extend((0..chem.ns()).filter(|&x| self.genes[lay.upt + x] > 0.0).map(|x| (x, self.genes[lay.upt + x])));
        // Catalysts' capacity: the catalysed rate per unit of contact at `t_cat`, summed over the genes that are on, its
        // gases at their share of the open air to their order; dearer when firm past `t_firm` (laws.md 6.2).
        let cats: f64 = chem.contact_rates(p.t_cat).iter().zip(&k.cat).filter(|x| *x.1 > 1.0).map(|(c, f)| f * c).sum();
        k.cats = cats * (1.0 + (self.gene(DENATURE) - p.t_firm).max(0.0) / p.c_firm);
        k.interval = self.dec(INTERVAL).round().clamp(MIN_INTERVAL, MAX_INTERVAL) as u64;
        (k.gas, k.liq, k.heat, k.leaf) = (self.dec(GAS), self.dec(LIQ), self.dec(HEAT), self.dec(LEAF));
        self.k = k;
    }
    /// The reflectance of a member, the mean of its colour's three bands (`visible`).
    pub(crate) fn reflectance(&self) -> f64 {
        (self.vis[3] + self.vis[4] + self.vis[5]) as f64 / 3.0
    }
    /// Visible traits of one member (`Body::vis`); the colour is the reflectance of the stores by volume.
    pub fn visible(&self, chem: &Chemistry, cfg: &crate::Config) -> [f32; 7] {
        let (m1, p, t) = (self.mass(chem) / self.n(), &cfg.phys, self.temp(chem));
        let (mut v, mut c) = (1e-12, [0.0; 3]);
        for (k, &a) in self.store.iter().enumerate() {
            let p = chem.phase_in(k, t, |j| self.store[j]);
            if p != Phase::Gas && a > 0.0 {
                let vol = a * chem.molar_volume(k, p);
                v += vol;
                for (x, r) in c.iter_mut().zip(chem.subs[k].refl[(p == Phase::Liquid) as usize]) {
                    *x += vol * r as f64;
                }
            }
        }
        let leaf = self.leaf_area(chem, p) / self.n();
        [m1 as f32, leaf as f32, self.genes[STRENGTH], (c[0] / v) as f32, (c[1] / v) as f32, (c[2] / v) as f32, self.height(chem, cfg) as f32]
    }
    /// Well-being at `t` (laws.md 5.4): the log of the energy its reactions could release from a member's stores in
    /// decisions' worth (`think`, J), less its nearness to its limits: key stores to their minimums and reactions to upkeep
    /// (`spare`), in `MARGIN` form, and key liquids to freezing or boiling.
    pub(crate) fn wellbeing_at(&self, chem: &Chemistry, think: f64, t: f64, spare: f64) -> f32 {
        let (lay, mass) = (Layout::of(chem), self.size(chem));
        let mut w = ln(1.0 + (self.fuel(chem, t) / self.n() / think) as f32) + ln(spare.clamp(MARGIN_MIN, MARGIN) as f32 / MARGIN as f32);
        for k in 0..chem.ns() {
            let min = self.genes[lay.min + k];
            if min > 0.0 {
                w += ln(ln((self.store[k] * chem.subs[k].molar_mass / mass) as f32 / min).clamp(MARGIN_MIN as f32, MARGIN as f32) / MARGIN as f32);
            }
            if self.wet >> k & 1 == 1 {
                let s = &chem.subs[k];
                w += (((t - self.melt(chem, k)).min(s.boil - t) as f32 - PHASE_MARGIN) / PHASE_MARGIN).min(0.0);
            }
        }
        w
    }
    /// Remember a dealing with body `id`: better (+) or worse (-). A new one replaces the least marked.
    pub(crate) fn note(&mut self, id: u64, d: f32) {
        let s = self.known.iter().position(|k| k.0 == id).unwrap_or_else(|| {
            let s = (0..KNOWN).min_by(|&a, &b| self.known[a].1.abs().total_cmp(&self.known[b].1.abs())).unwrap_or(0);
            self.known[s] = (id, 0.0);
            s
        });
        self.known[s].1 = (self.known[s].1 + d).clamp(-1.0, 1.0);
    }
}
