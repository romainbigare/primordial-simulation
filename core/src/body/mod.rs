//! Bodies: nodes of the sparse graph that hold substances, heat and genes, with stable IDs (slot and generation).
//! A body updates every `interval` days, and everything in between is integrated at its update (`live`): Flow and
//! Change on it (`physics`), its upkeep (`pay`), its death (`death`). Choose (`choose`) and Copy (`copy`) act on it.
//! Its genes, how alike they are and how they change are in `genes`, what it is made of in `tissue`, its chemistry in
//! `metab`, what it takes in in `intake`, its exchange with its surroundings in `exchange`. The registry of bodies is
//! `life`, its lineages `lineage`, the survey of cells before each step `survey`.

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
pub(crate) use death::to_ground;
pub use exchange::conductance_to_ground;
pub use genes::*;
pub(crate) use live::{gain_to_growth, live, LiveTmp};
pub use live::Lived;
pub use pay::{pay, release, upkeep};
pub(crate) use pay::{plan, spend, upkeep_at, Payer};

use crate::brain::{ln, SIG};
use crate::chem::{Chemistry, Phase};
use crate::ground::{melting, Melting};
use crate::store::NodeId;
use crate::units::MAX_SUBSTANCES;
use serde::{Deserialize, Serialize};

/// Death causes: a store below its safe minimum; a key liquid store froze or boiled; the
/// body could not pay its upkeep; taken apart by another body.
pub const LACK: u8 = 0;
pub const PHASE: u8 = 1;
pub const SPENT: u8 = 2;
pub const TAKEN: u8 = 3;
/// Bonds broken by bodies moving through its cell.
pub(crate) const BROKEN: u8 = 4;
/// Worn out by living (`Body::wear`).
pub const AGE: u8 = 5;
/// The most share of what its air node or ground node holds that a body takes in one update (the bodies of a cell
/// share it).
pub(crate) const MAX_TAKE_SHARE: f64 = 0.5;
/// Margins of well-being (`Body::wellbeing_at`) count up to this, and down to `MARGIN_MIN`: the margin of a key store
/// (the log of its share over its minimum) and that of its reactions over its upkeep (`live` plans for this many upkeeps).
pub(crate) const MARGIN: f64 = 2.0;
const MARGIN_MIN: f64 = 1e-3;
/// Well-being falls as a key liquid comes within this of freezing or boiling, K.
const PHASE_MARGIN: f32 = 5.0;
/// A brain perceives a share `x` of the body as `SHARE_SENSE` ln(x + floor): a safe minimum with the floor `MIN_FLOOR`.
pub(crate) const SHARE_SENSE: f32 = 0.25;
const MIN_FLOOR: f32 = 1e-2;
/// The liquid share of living tissue: a kg of body matter makes 1 / (1 - this) kg of working body (`Body::built`).
/// About 0.7 to 0.9 in real living tissue (docs/numbers.md).
pub const WET_TISSUE: f64 = 0.75;
/// Others a body remembers.
pub(crate) const KNOWN: usize = 4;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Body {
    pub gen: u32,
    pub alive: bool,
    pub cell: u32,
    /// Members this node stands for (1: an individual; more: a population node). Its
    /// stores and heat are the members' totals, its genes and brain one (their mean), so every
    /// law runs on the totals: rates are per kg or per member, times the count. Per member: the
    /// size a brain perceives, the mass compared with the copy mass. Use `n()`.
    pub count: f64,
    pub lineage: u32,
    /// The body it was copied from (id; 0 for a placed founder) and its day of birth: a record of
    /// descent, read by the checks that follow a parent's children
    pub parent: u64,
    pub born: u64,
    /// Day of the last update; for a node, the mean by members of the days its members' time was last integrated from
    /// (members who joined since count from when they came, `nodes::add`)
    pub last: f64,
    /// The cell's cumulative light at the last update, J/m², moved since by what its shade gave
    /// it more or less than the cell's mean (`survey::shade`): it caught `light - mark` per m²
    pub mark: f64,
    /// Thermal energy, J
    pub e: f64,
    /// mol of each substance
    pub store: Vec<f64>,
    pub genes: Vec<f32>,
    /// How a node's members differ: the variance of their genes around `genes`, per
    /// gene, in gene units squared (empty: none), and of their born brain rows around the node's,
    /// the same for every row (`copy`)
    pub spread: Vec<f32>,
    pub row_spread: f32,
    /// How a node's members differ in size: the variance of log10 of their bodies (`Body::size_moments`), decades²;
    /// members who joined earlier have grown more, and they mature first (`copy::young`)
    pub size_spread: f32,
    /// How far the mean of log10 of its members' bodies lies below log10 of its mean member's body, decades
    /// (`Body::size_moments`): 0 when all are of one size
    pub size_lag: f32,
    /// The node of its kind in its cell at the other stage: a young node's adult node, an adult node's young node (its
    /// id, `Body::id`; none: not known). Births flow along it one way, maturing members the other (`copy::young`)
    pub link: Option<u64>,
    /// The brain rows it was born with (genes too), in `Life::genomes`
    pub genome: u32,
    /// Key stores (safe minimum > 0) that were liquid at birth, one bit per substance
    pub wet: u64,
    /// Body matter built for offspring and held back, mol (all members; part of the matter in `store`,
    /// not counted in the body: `size`, `built`). A mature body puts the share `ALLOC` of what it
    /// builds here (`copy::grow`); `copy::copy` turns it into children of the size the `CHILD` gene sets
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
    /// An individual's wear, 0 at birth: the energy its upkeep turned over per kg, over what it
    /// can turn over in its life (`Physics::e_life` x 10^`REPAIR`); it dies of age at 1. A node
    /// has none: its members are of all ages (`live`).
    pub wear: f64,
    /// Numbers that follow from the genes (`prepare`, at every change of the genes; not saved: a world loaded works them
    /// out again)
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
    /// What a brain perceives of each safe minimum; the substances taken, by selectivity
    pub minln: Vec<f32>,
    pub upt: Vec<(usize, f32)>,
    /// Its catalysts' capacity (`upkeep`; an exponential per catalyst, so kept)
    pub cats: f64,
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
    /// Its temperature, K: its molten stores hold their fusion enthalpy in `e`, as the ground's do, so a body freezes and
    /// melts at a plateau (`thermal`).
    pub fn temp(&self, chem: &Chemistry) -> f64 {
        self.temp_at(chem, self.heat_cap(chem))
    }
    /// `temp` when the heat capacity `cap` is known.
    pub(crate) fn temp_at(&self, chem: &Chemistry, cap: f64) -> f64 {
        self.thermal(chem, cap).t
    }
    /// `thermal` at its heat capacity now.
    pub(crate) fn thermal_now(&self, chem: &Chemistry) -> Melting {
        self.thermal(chem, self.heat_cap(chem))
    }
    /// Where the body of heat capacity `cap` stands among the melting plateaus of its stores (`ground::melting`), each
    /// store melting at its point lowered by what is dissolved in it (`melt`). (What is dissolved only lowers a melting
    /// point: solved first at the stores' own points, the answer holds when every store that may hold something
    /// dissolved is molten above its own point, as a warm body's liquid is.)
    pub(crate) fn thermal(&self, chem: &Chemistry, cap: f64) -> Melting {
        let held = |k: usize| self.store[k] > 0.0 && chem.subs[k].lf > 0.0;
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
    /// Its energy with its stores as they are at the plateau state `th` (`thermal`): their heat capacity at `th.t` and
    /// the fusion enthalpy of their molten shares there, J.
    pub(crate) fn energy_in(&self, chem: &Chemistry, th: &Melting) -> f64 {
        self.heat_cap(chem) * th.t + self.fusion_in(chem, th)
    }
    /// The fusion enthalpy its stores hold at the plateau state `th`, J.
    pub(crate) fn fusion_in(&self, chem: &Chemistry, th: &Melting) -> f64 {
        self.fused_of(chem, th, &self.store)
    }
    /// The fusion enthalpy amounts `m` hold in the body at the plateau state `th`: of each its molten share there (what
    /// `th` does not hold molten, as a store it did not have then, by its melting point now), J.
    fn fused_of(&self, chem: &Chemistry, th: &Melting, m: &[f64]) -> f64 {
        let molten_at = |k: usize| th.t >= chem.subs[k].melt || th.t >= self.melt(chem, k);
        let molten = |k: usize| if th.bits >> k & 1 == 1 { th.molten(k) } else { molten_at(k) as u8 as f64 };
        m.iter().enumerate().filter(|&(k, &a)| a > 0.0 && chem.subs[k].lf > 0.0).map(|(k, a)| a * chem.subs[k].lf * molten(k)).sum()
    }
    /// Its energy at `t` with what is at or over its melting point molten (`Chemistry::fused_heat`), J: the inverse of
    /// `temp` off a plateau.
    pub fn energy_at(&self, chem: &Chemistry, t: f64) -> f64 {
        self.heat_cap(chem) * t + (0..chem.ns()).map(|k| self.store[k] * chem.fused_heat(k, t, |j| self.store[j])).sum::<f64>()
    }
    /// The heat that leaves with amounts `m` (mol of each substance) from the body at the plateau state `th` (`thermal`):
    /// their heat capacity at its temperature and the fusion enthalpy of their molten shares, J.
    pub(crate) fn heat_with(&self, chem: &Chemistry, th: &Melting, m: &[f64]) -> f64 {
        chem.heat_cap_of(m) * th.t + self.fused_of(chem, th, m)
    }
    /// Volume of its solids and liquids, m³.
    pub fn volume(&self, chem: &Chemistry) -> f64 {
        self.volume_at(chem, self.temp(chem))
    }
    /// `volume` when its temperature `t` is known.
    pub(crate) fn volume_at(&self, chem: &Chemistry, t: f64) -> f64 {
        self.store.iter().enumerate().map(|(k, a)| a * chem.molar_volume(k, chem.phase_in(k, t, |j| self.store[j]))).sum()
    }
    /// Days between its updates: its `INTERVAL` gene; a population node without a brain's at least `Config::node_days`,
    /// as a node of many members that takes no decisions needs no update more often (its update integrates the time
    /// since). A body with a brain keeps its gene's pace: its actions and its learning go by decisions.
    pub fn interval(&self, w: &crate::World) -> u64 {
        let own = self.own_interval();
        if self.n() > 1.0 && w.life.brainless(self) { own.max(w.cfg.node_days as u64) } else { own }
    }
    /// How many of the intervals its `INTERVAL` gene asks for one update spans (1 but for a node that updates less often).
    pub(crate) fn paces(&self, w: &crate::World) -> f64 {
        self.interval(w) as f64 / self.own_interval() as f64
    }
    /// The share of a node's members that dies in an update when the share `f` would die in one interval of its gene:
    /// they die at the rate -ln(1 - f) per such interval, so over the update 1 - (1 - f)^`paces`, as many in a time
    /// whatever the length of its updates.
    pub(crate) fn over_span(&self, f: f64, w: &crate::World) -> f64 {
        let d = self.paces(w);
        if d == 1.0 { f } else { 1.0 - (1.0 - f).powf(d) }
    }
    /// Days between the updates its `INTERVAL` gene asks for.
    fn own_interval(&self) -> u64 {
        self.k.interval
    }
    /// Its genes are `genes` (`prepare` follows).
    pub(crate) fn set_genes(&mut self, genes: Vec<f32>, chem: &Chemistry, p: &crate::Physics) {
        self.genes = genes;
        self.prepare(chem, p);
    }
    /// Work `k` out from the genes (and the physics, which stays the same through a run): at every change of the genes.
    pub fn prepare(&mut self, chem: &Chemistry, p: &crate::Physics) {
        let lay = Layout::of(chem);
        let cat = lay.cat;
        let f: Vec<f64> = (0..chem.reactions.len()).map(|r| if self.genes[cat + r] > 0.0 { self.dec(cat + r) } else { 1.0 }).collect();
        let burn = (0..chem.reactions.len()).filter(|&r| self.genes[cat + r] > 0.0 && chem.dh(r) < 0.0 && !chem.keeps(r)).map(|r| (r, -chem.dh(r))).collect();
        let minln = (0..chem.ns()).map(|k| SHARE_SENSE * ln(self.genes[lay.min + k].max(0.0) + MIN_FLOOR)).collect();
        let upt = (0..chem.ns()).filter(|&k| self.genes[lay.upt + k] > 0.0).map(|k| (k, self.genes[lay.upt + k])).collect();
        // Its catalysts' capacity: the catalysed rate per unit of the contact of its phases (`change`) at `t_cat`, summed
        // over the genes that are on, its gases at their share of the open air (`Chemistry::air`) to their order;
        // dearer if they keep their shape when warmer (`Physics::t_firm`).
        let open = |rx: &crate::chem::Reaction| -> f64 {
            rx.inputs.iter().enumerate().filter(|(_, x)| chem.phase(x.0 as usize, p.t_cat) == Phase::Gas).map(|(q, &(k, _))| chem.air.get(k as usize).copied().filter(|&x| x > 0.0).unwrap_or(1.0).powi(rx.order(q) as i32)).product()
        };
        let cats: f64 = chem.reactions.iter().zip(&f).filter(|x| *x.1 > 1.0).map(|(rx, f)| f * rx.arrhenius(p.t_cat) * open(rx)).sum();
        let cats = cats * (1.0 + (self.gene(DENATURE) - p.t_firm).max(0.0) / p.c_firm);
        let interval = self.dec(INTERVAL).round().clamp(MIN_INTERVAL, MAX_INTERVAL) as u64;
        self.k = Consts { gas: self.dec(GAS), liq: self.dec(LIQ), heat: self.dec(HEAT), leaf: self.dec(LEAF), cat: f, interval, burn, minln, upt, cats };
    }
    /// The reflectance of a member, the mean of its colour's three bands (`visible`).
    pub(crate) fn reflectance(&self) -> f64 {
        (self.vis[3] + self.vis[4] + self.vis[5]) as f64 / 3.0
    }
    /// Visible traits of one member (`Body::vis`); the colour is the reflectance of the stores by volume.
    pub fn visible(&self, chem: &Chemistry, cfg: &crate::Config) -> [f32; 7] {
        let ((m, cap), p) = (self.mass_cap(chem), &cfg.phys);
        let m1 = m / self.n();
        let t = self.temp_at(chem, cap);
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
    /// Well-being at temperature `t`: the energy its catalysed reactions could release from a member's stores (log, in
    /// decisions' worth: `think` is what one decision of the starting brain costs a member of the mass it copies at, J, so
    /// shedding mass does not raise it), less how near it is to its limits: each key store's margin over its safe minimum,
    /// the margin of its reactions over its upkeep (`spare`), both in `MARGIN` form, and its key liquids to freezing or
    /// boiling (`PHASE_MARGIN`).
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
    /// How past dealings with body `id` went, if it is known.
    pub(crate) fn recall(&self, id: u64) -> Option<f32> {
        self.known.iter().find(|k| k.0 == id && id != 0).map(|k| k.1)
    }
    /// Remember a dealing with body `id`: better (+) or worse (-). A new one replaces the least marked.
    pub(crate) fn note(&mut self, id: u64, d: f32) {
        let s = match self.known.iter().position(|k| k.0 == id) {
            Some(s) => s,
            None => {
                let s = (0..KNOWN).min_by(|&a, &b| self.known[a].1.abs().total_cmp(&self.known[b].1.abs())).unwrap_or(0);
                self.known[s] = (id, 0.0);
                s
            }
        };
        self.known[s].1 = (self.known[s].1 + d).clamp(-1.0, 1.0);
    }
}
