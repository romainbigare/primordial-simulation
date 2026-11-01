//! Bodies: nodes that hold substances, heat and genes, each with an id that never changes (`life`). A body updates every
//! `interval` days and integrates the time since at its update (`live`; laws.md 6).

mod death;
mod exchange;
mod genes;
mod intake;
mod live;
mod metab;
mod pay;
mod tissue;

pub use tissue::solids_of;

pub use death::{die, wound};
use death::thin;
pub(crate) use death::{remains, to_ground, torn_to_death};
pub use genes::*;
pub(crate) use live::{live, LiveTmp};
pub use live::Lived;
pub use pay::{pay, upkeep};
pub(crate) use pay::settle_heat;

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
pub const BROKEN: u8 = 4;
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
    /// Its id, given at birth (`Domain::add`); 0 for none
    pub id: u64,
    /// Whether it is a living body (not an empty slot)
    pub alive: bool,
    pub cell: u32,
    /// Members this node stands for (1: an individual). Stores and heat are the members' totals, genes and brain their
    /// mean, so every law runs on the totals. Use `n()`.
    pub count: f64,
    pub lineage: u32,
    /// The body it was copied from (id; 0 for a founder) and its day of birth
    pub parent: u64,
    pub born: u64,
    /// Batch of its next update (`Life::schedule`)
    pub due: u64,
    /// Day of the last update; for a node, the mean by members of the days its members' time was last integrated from
    /// (members who joined since count from when they came, `nodes::rejoin`)
    pub last: f64,
    /// Thermal energy, J
    pub e: f64,
    /// mol of each substance
    pub store: Vec<f64>,
    pub genes: Vec<f32>,
    /// How a node's members differ: the variance of each gene around `genes` (empty: none), and of their born rows, one
    /// number for every row
    pub spread: Vec<f32>,
    pub row_spread: f32,
    /// The brain rows it was born with (genes too), in `Life::genomes`
    pub genome: u32,
    /// Key stores (safe minimum > 0) that were liquid at birth, one bit per substance
    pub wet: u64,
    /// Body matter built for children and held back, mol (part of `store`, not of the body: `size`, `built`; laws.md 4.2)
    pub brood: f64,
    /// For a node: the first batch of the world's clock at which it may make children (`copy::due`; 0: not set)
    pub lump_at: u64,
    /// Visible traits: mass kg, light-capture area m² (none: bodies catch no light), strength, colour (3 bands), height
    /// m (`height`)
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
    /// The day of its last decision (any body); for a node, what its members repeat until its next decision (`choose::Habit`)
    pub habit: Option<crate::choose::Habit>,
    /// It has a move queued in the batch running, or is a node's part with a part act queued (`outbox`): it stays in
    /// its cell until then, but the others there no longer deal with it (laws.md 0); false outside a batch, not saved
    #[serde(skip)]
    pub leaving: bool,
    /// Numbers that follow from the genes (`prepare`); not saved
    #[serde(skip)]
    pub k: Consts,
}

/// Numbers that follow from a body's genes, kept to save time.
#[derive(Clone, Debug, Default)]
pub struct Consts {
    /// 10 to the power of the coat gene (`HEAT`)
    pub heat: f64,
    /// Days between the updates its `INTERVAL` gene asks for, a whole number of batches (`Body::interval`)
    pub interval: u64,
    /// The reactions of its diet that release heat, with the heat per unit, J (not those that keep a part of a fuel's
    /// heat, `Chemistry::keeps`)
    pub burn: Vec<(usize, f64)>,
    /// The substances taken, by selectivity
    pub upt: Vec<(usize, f32)>,
    /// 10 to the power of the copy mass gene, kg
    pub copy_mass: f64,
    /// Factor of upkeep per kg of living body, (copy mass / `m_ref`)^(`intake_exp` - 1), 1
    pub tissue: f64,
    /// The bare rate constant of its main burning at `t_cat`, 1/s (`Body::pace`)
    pub pace_ref: f64,
    /// What its burnings take, and what the reactions of its diet take (their inputs, a bit per substance)
    pub fuel: u64,
    pub broken: u64,
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
    /// Its temperature, K (`thermal`).
    pub fn temp(&self, chem: &Chemistry) -> f64 {
        self.thermal(chem).t
    }
    /// Where the body stands on the melting of its key liquids (`ground::melting`, at each one's own melting point): its
    /// molten key liquids hold their fusion enthalpy in `e`; its other stores hold their sensible heat only (laws.md 6.3).
    pub fn thermal(&self, chem: &Chemistry) -> Melting {
        let (mut ks, mut m) = ([(0.0, 0usize, 0.0); MAX_SUBSTANCES], 0);
        for k in self.wet_ks(chem).filter(|&k| self.store[k] > 0.0 && chem.subs[k].lf > 0.0) {
            (ks[m], m) = ((chem.subs[k].melt, k, self.store[k] * chem.subs[k].lf), m + 1);
        }
        ks[..m].sort_by(|a, b| a.0.total_cmp(&b.0));
        melting(self.heat_cap(chem).max(1e-9), self.e, ks[..m].iter().copied())
    }
    /// The fusion enthalpy amounts `m` hold at the plateau state `th`: of the key liquids among them, as far as they are
    /// molten, J.
    pub(crate) fn fused_of(&self, chem: &Chemistry, th: &Melting, m: &[f64]) -> f64 {
        let molten = |k: usize| if th.bits >> k & 1 == 1 { th.molten(k) } else { (th.t >= chem.subs[k].melt) as u8 as f64 };
        self.wet_ks(chem).filter(|&k| m[k] > 0.0).map(|k| m[k] * chem.subs[k].lf * molten(k)).sum()
    }
    /// Its energy at `t`, its key liquids at or over their melting point molten, J: the inverse of `temp` off a plateau.
    pub fn energy_at(&self, chem: &Chemistry, t: f64) -> f64 {
        let fused: f64 = self.wet_ks(chem).filter(|&k| t >= chem.subs[k].melt).map(|k| self.store[k] * chem.subs[k].lf).sum();
        self.heat_cap(chem) * t + fused
    }
    /// The heat that leaves with amounts `m` (mol) at the plateau state `th`, with their molten shares' fusion enthalpy, J.
    pub(crate) fn heat_with(&self, chem: &Chemistry, th: &Melting, m: &[f64]) -> f64 {
        chem.heat_cap_of(m) * th.t + self.fused_of(chem, th, m)
    }
    /// Volume of its solids and liquids at its temperature `t`, m³.
    pub fn volume(&self, chem: &Chemistry, t: f64) -> f64 {
        let held = self.store.iter().enumerate().filter(|x| *x.1 != 0.0);
        held.map(|(k, a)| a * chem.molar_volume(k, chem.phase_in(k, t, |j| self.store[j]))).sum()
    }
    /// Days between its updates, a whole number of batches (`Config::batch_days`): its `INTERVAL` gene.
    pub fn interval(&self, _w: &crate::World) -> u64 {
        self.k.interval
    }
    /// How many of the intervals its `INTERVAL` gene asks for one update spans.
    pub(crate) fn paces(&self, w: &crate::World) -> f64 {
        self.interval(w) as f64 / self.k.interval as f64
    }
    /// The share that dies in an update when `f` would die in one interval of its gene: the rate -ln(1 - f) per
    /// interval over `paces` intervals, so as many die in a time whatever the length of the updates.
    pub(crate) fn over_span(&self, f: f64, w: &crate::World) -> f64 {
        let d = self.paces(w);
        if d == 1.0 { f } else { 1.0 - (1.0 - f).powf(d) }
    }
    /// Work `k` out from the genes and the settings, at every change of the genes.
    pub fn prepare(&mut self, chem: &Chemistry, cfg: &crate::Config) {
        let p = &cfg.phys;
        let (lay, nr) = (Layout::of(chem), chem.reactions.len());
        let on = |r: usize| self.genes[lay.cat + r] > 0.0;
        let mut k = std::mem::take(&mut self.k);
        k.burn.clear();
        k.burn.extend(chem.burners().iter().filter(|x| on(x.0)));
        k.fuel = k.burn.iter().flat_map(|x| &chem.reactions[x.0].inputs).fold(0, |m, p| m | 1 << p.0);
        k.broken = (0..nr).filter(|&r| on(r)).flat_map(|r| &chem.reactions[r].inputs).fold(0, |m, p| m | 1 << p.0);
        k.upt.clear();
        k.upt.extend((0..chem.ns()).filter(|&x| self.genes[lay.upt + x] > 0.0).map(|x| (x, self.genes[lay.upt + x])));
        // The gene's days to the nearest whole batch, at least one, so every update starts a batch.
        let batch = cfg.batch_days as f64;
        k.interval = ((self.dec(INTERVAL).clamp(MIN_INTERVAL, MAX_INTERVAL) / batch).round().max(1.0) * batch) as u64;
        (k.heat, k.copy_mass) = (self.dec(HEAT), self.dec(COPY_MASS));
        k.tissue = (k.copy_mass / p.m_ref).powf(p.intake_exp - 1.0);
        k.pace_ref = k.burn.first().map_or(1.0, |&(r, _)| chem.reactions[r].arrhenius(p.t_cat));
        self.k = k;
    }
    /// Visible traits of one member (`Body::vis`); the colour is the reflectance of the stores by volume.
    pub fn visible(&self, chem: &Chemistry, cfg: &crate::Config) -> [f32; 7] {
        let (m1, t) = (self.mass(chem) / self.n(), self.temp(chem));
        let (mut v, mut c) = (1e-12, [0.0; 3]);
        for (k, &a) in self.store.iter().enumerate().filter(|x| *x.1 > 0.0) {
            let p = chem.phase(k, t);
            if p != Phase::Gas {
                let vol = a * chem.molar_volume(k, p);
                v += vol;
                for (x, r) in c.iter_mut().zip(chem.subs[k].refl[(p == Phase::Liquid) as usize]) {
                    *x += vol * r as f64;
                }
            }
        }
        [m1 as f32, 0.0, self.genes[STRENGTH], (c[0] / v) as f32, (c[1] / v) as f32, (c[2] / v) as f32, self.height(chem, cfg) as f32]
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
                w += (((t - s.melt).min(s.boil - t) as f32 - PHASE_MARGIN) / PHASE_MARGIN).min(0.0);
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
