//! Bodies: nodes that hold substances, heat and genes, each with an id that never changes (`life`). A body decides at
//! each of its cell's steps, and its physics runs over the step ahead in sub-steps of a week (`live`; laws.md 0, 6).

mod death;
mod exchange;
mod genes;
mod intake;
mod live;
mod metab;
mod pay;
mod tissue;
mod drink;

pub use tissue::solids_of;

pub use death::{die, wound};
use death::thin;
pub(crate) use death::{perish, remains, to_ground, torn_to_death};
pub use genes::*;
pub(crate) use live::{live, fade_meal_heat, newborn, LiveTmp};
pub use live::Lived;
pub use pay::{pay, upkeep};
pub(crate) use pay::ahead;
pub(crate) use pay::settle_heat;

use crate::brain::ln;
use crate::chem::{BodyChem, Chemistry, Phase, Stores};
use crate::life::Block;
use crate::math::Few;

use crate::units::{BODY_REACTIONS, BODY_STORES};
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
/// Most numbers of a brain's memory (the top of the `MEMORY` gene's range).
pub const MEMORY_MOST: usize = 12;
/// The visible traits of a body (`Body::vis`).
pub const VIS: usize = 5;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Body {
    /// Its id, given at birth (`Chunk::add`); 0 for none
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
    /// Day of the last update; for a node, the mean by members of the days its members' time was last integrated from
    /// (members who joined since count from when they came, `nodes::rejoin`)
    pub last: f64,
    /// Thermal energy, J
    pub e: f64,
    /// mol of each store (`Chemistry::body`: the substances bodies hold, each at its place)
    pub store: Stores,
    pub genes: Genes,
    /// How a node's members differ: the variance of each gene around `genes` (empty: none), and of their born rows, one
    /// number for every row
    pub spread: Genes,
    pub row_spread: f32,
    /// The brain rows it was born with (genes too), in `Life::genomes`
    pub genome: u32,
    /// Key stores (safe minimum > 0) that were liquid at birth, one bit per store
    pub wet: u32,
    /// Body matter built for children and held back, mol (part of `store`, not of the body: `size`, `built`; laws.md 4.2)
    pub brood: f64,
    /// Visible traits: mass kg, strength, colour (3 bands)
    pub vis: [f32; VIS],
    /// Whether what it holds changed since its visible traits were worked out: the next survey works them out again
    /// (`Chunk::see_changed`); a saved body's are as saved
    #[serde(skip)]
    pub stale: bool,
    /// Its brain in its cell's brains (`life::Brains`; none without a brain): its rows, the rows it was born with plus
    /// what it (a node: its members) learned; per learned row a moving average of the square of its share of the
    /// learning steps (`brain::learn`); the trace of its last decision, which holds one if `traced`
    pub brain: Block,
    pub traced: bool,
    /// Moving average of the square of its critic's error per month (0 until the first one)
    pub var: f32,
    /// The part it holds, and a part of the structure it is inside (their ids, `bond::Part::id`; both lie in its cell)
    pub hold: Option<u64>,
    pub inside: Option<u64>,
    /// Memory the brain reads and writes (its first numbers, as many as its lineage's brain has; the others 0)
    pub mem: [f32; MEMORY_MOST],
    /// Well-being at the last decision
    pub wb: f32,
    /// What its last sub-step left for its decision (`survey::updates`), and the day of its last decision (of its
    /// placing or birth before its first)
    pub lived: Lived,
    pub decided: f64,
    /// The share of its members whose act at its last decision fills its step (rest, a bite, a gift, a watch, an act on
    /// a part): they take nothing by the take law at the later sub-steps of its update (`choose::meal`; laws.md 5.3)
    pub busy: f32,
    /// It has a move queued in the batch running, or is a node's part with a part act queued (`life::Entry`): it stays in
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
    /// The reactions of its diet that release heat, with the heat per unit, J (not those that keep a part of a fuel's
    /// heat, `Chemistry::keeps`; only those bodies may run, `Chemistry::body`)
    pub burn: Few<(usize, f64), BODY_REACTIONS>,
    /// The substances taken, by selectivity
    pub upt: Few<(usize, f32), BODY_STORES>,
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

/// Amounts `m` of a body's stores as (substance, mol), in the order of the substances.
pub fn each<'a>(chem: &'a Chemistry, m: &'a Stores) -> impl Iterator<Item = (usize, f64)> + 'a {
    chem.body().subs().iter().copied().zip(m.iter().copied())
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
        chem.body().mass_of(&self.store)
    }
    pub fn heat_cap(&self, chem: &Chemistry) -> f64 {
        chem.body().heat_cap_of(&self.store)
    }
    /// mol of substance `k` it holds (none of one bodies do not hold, `Chemistry::body`).
    #[inline]
    pub fn amount(&self, bc: &BodyChem, k: usize) -> f64 {
        bc.place(k).map_or(0.0, |s| self.store[s])
    }
    /// Its store of substance `k`, mol. Panics for one bodies do not hold.
    pub fn store_of(&mut self, chem: &Chemistry, k: usize) -> &mut f64 {
        &mut self.store[chem.body().place(k).expect("bodies hold it")]
    }
    /// Its temperature, K: its heat less the fusion enthalpy its key liquids hold (`fused`), over its heat capacity.
    pub fn temp(&self, chem: &Chemistry) -> f64 {
        (self.e - self.fused(chem, &self.store)) / self.heat_cap(chem).max(1e-9)
    }
    /// The fusion enthalpy amounts `m` (mol of each store) hold: of the key liquids among them, J. A body holds its key
    /// liquids molten at any temperature: frost kills a share of its members (laws.md 6.5); it has no melting plateau.
    pub(crate) fn fused(&self, chem: &Chemistry, m: &Stores) -> f64 {
        let subs = chem.body().subs();
        self.wet_ks().map(|s| m[s] * chem.subs[subs[s]].lf).sum()
    }
    /// Its energy at `t`, J: the inverse of `temp`.
    pub fn energy_at(&self, chem: &Chemistry, t: f64) -> f64 {
        self.heat_cap(chem) * t + self.fused(chem, &self.store)
    }
    /// The heat that leaves with amounts `m` (mol of each store) at its temperature, with the fusion enthalpy of its key
    /// liquids among them, J.
    pub(crate) fn heat_with(&self, chem: &Chemistry, m: &Stores) -> f64 {
        chem.body().heat_cap_of(m) * self.temp(chem) + self.fused(chem, m)
    }
    /// The phase of its store `s` at `t`: of its substance, or liquid where its liquid holds what lowers its melting point
    /// (`Chemistry::phase_in`).
    pub(crate) fn phase_of(&self, chem: &Chemistry, s: usize, t: f64) -> Phase {
        let bc = chem.body();
        chem.phase_in(bc.subs()[s], t, |j| self.amount(bc, j))
    }
    /// Volume of its solids and liquids at its temperature `t`, m³.
    pub fn volume(&self, chem: &Chemistry, t: f64) -> f64 {
        let bc = chem.body();
        let held = bc.subs().iter().zip(&self.store).enumerate().filter(|x| *x.1 .1 != 0.0);
        held.map(|(s, (&k, a))| a * chem.molar_volume(k, self.phase_of(chem, s, t))).sum()
    }
    /// Work `k` out from the genes and the settings, at every change of the genes.
    pub fn prepare(&mut self, chem: &Chemistry, cfg: &crate::Config) {
        let p = &cfg.phys;
        let (lay, bc) = (Layout::of(chem), chem.body());
        let on = |r: usize| lay.diet(chem, r).is_some_and(|g| self.genes[g] > 0.0);
        let mut k = std::mem::take(&mut self.k);
        k.burn = chem.burners().iter().filter(|x| on(x.0)).copied().collect();
        k.fuel = k.burn.iter().flat_map(|x| &chem.reactions[x.0].inputs).fold(0, |m, p| m | 1 << p.0);
        k.broken = bc.reactions().iter().filter(|&&r| on(r)).flat_map(|&r| &chem.reactions[r].inputs).fold(0, |m, p| m | 1 << p.0);
        k.upt = (0..bc.n()).filter(|&s| self.genes[lay.upt + s] > 0.0).map(|s| (bc.subs()[s], self.genes[lay.upt + s])).collect();
        k.copy_mass = self.dec(COPY_MASS);
        k.tissue = (k.copy_mass / p.m_ref).powf(p.intake_exp - 1.0);
        k.pace_ref = k.burn.first().map_or(1.0, |&(r, _)| chem.reactions[r].arrhenius(p.t_cat));
        self.k = k;
    }
    /// Visible traits of one member (`Body::vis`); the colour is the reflectance of the stores by volume.
    pub fn visible(&self, chem: &Chemistry) -> [f32; VIS] {
        let (m1, t, bc) = (self.mass(chem) / self.n(), self.temp(chem), chem.body());
        let (mut v, mut c) = (1e-12, [0.0; 3]);
        for (&k, &a) in bc.subs().iter().zip(&self.store).filter(|x| *x.1 > 0.0) {
            let p = chem.phase(k, t);
            if p != Phase::Gas {
                let vol = a * chem.molar_volume(k, p);
                v += vol;
                for (x, r) in c.iter_mut().zip(chem.subs[k].refl[(p == Phase::Liquid) as usize]) {
                    *x += vol * r as f64;
                }
            }
        }
        [m1 as f32, self.genes[STRENGTH], (c[0] / v) as f32, (c[1] / v) as f32, (c[2] / v) as f32]
    }
    /// Well-being at `t` (laws.md 5.4): the log of the energy its reactions could release from a member's stores in
    /// months' worth of thinking (`think`, J), less its nearness to its limits: key stores to their minimums and
    /// reactions to upkeep (`spare`), in `MARGIN` form, and key liquids to freezing or boiling.
    pub(crate) fn wellbeing_at(&self, chem: &Chemistry, think: f64, t: f64, spare: f64) -> f32 {
        let (lay, mass, bc) = (Layout::of(chem), self.size(chem), chem.body());
        let mut w = ln(1.0 + (self.fuel(chem, t) / self.n() / think) as f32) + ln(spare.clamp(MARGIN_MIN, MARGIN) as f32 / MARGIN as f32);
        for (s, &k) in bc.subs().iter().enumerate() {
            let min = self.genes[lay.min + s];
            if min > 0.0 {
                w += ln(ln((self.store[s] * bc.mm[s] / mass) as f32 / min).clamp(MARGIN_MIN as f32, MARGIN as f32) / MARGIN as f32);
            }
            if self.wet >> s & 1 == 1 {
                let x = &chem.subs[k];
                w += (((t - x.melt).min(x.boil - t) as f32 - PHASE_MARGIN) / PHASE_MARGIN).min(0.0);
            }
        }
        w
    }
}
