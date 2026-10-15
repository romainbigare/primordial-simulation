//! Bodies: nodes of the sparse graph that hold substances, heat and genes, with stable IDs
//! (slot and generation). Here: their storage, and Flow and Change on each body since its
//! last update: light, gas and liquid exchange with its cell, its catalysed reactions, heat
//! to the surface, upkeep, death, and copying when full. Choose (`choose`) and Copy (`copy`) act on them.
//! A body updates every `interval` days; everything in between is integrated at its update.
//! The light it catches warms the surface as it falls, but for the share its cell sets aside for
//! what light reactions store (`World::light_pool`), which it takes at its update.

use crate::brain::{ln, Shape, Thought, FIXED, OVERBS, SIG, VERBS};
use crate::change::Reactor;
use crate::chem::{Chemistry, Phase, P_REF};
use crate::event::{Event, EventKind};
use crate::rng::{Purpose, Rng};
use crate::{bond, choose, World};
use serde::{Deserialize, Serialize};

// Scalar genes, then one per reaction (catalyst, log10 speed-up) and three per substance
// (safe minimum as a share of mass; take selectivity; most share of mass kept). Below zero a
// gene is off.
/// Gas exchange, log10 mol of air per m² of the surface open to the air (`Body::open_area`) per
/// day. Before (14 Oct 2026) it was per kg: a plant breathed 316 mol of air per kg per day, which
/// at 300 ppm of carbon gas is 2.8 g of sugar per kg per day, ten times less than a real leaf
/// (24 g per m² per day, so about 33 g per kg for 2 m²/kg), while the same plant caught 40 times
/// more light than that carbon gas could use. Shade then cost it nothing and height could not
/// pay (review 4, finding 4). Through the surface, capture area pays twice (light and gas), so a
/// plant that grows its area gains on both and a taller plant that shades the rest gains more:
/// the balance of gain and cost that makes a tall plant exists.
pub const GAS: usize = 0;
/// Liquid exchange with the ground, log10 per day
pub const LIQ: usize = 1;
/// Heat conductance to the surface through the body's surface, log10 W/(K kg) for a member of
/// `Physics::m_ref` kg (1 kg since 14 Oct 2026; 30 kg before, so every kind's gene is
/// log10(30^(1/3)) = 0.492 higher); it goes with mass^(2/3) (`Body::conductance`)
pub const HEAT: usize = 2;
/// Light-capture area, log10 m² per kg-equivalent of its body (`Body::size`, `Body::equiv`:
/// mass^0.75 from `Physics::m_ref`, 14 Oct 2026; per kg of a 1 kg member)
pub const LEAF: usize = 3;
/// Strength, 0..1
pub const STRENGTH: usize = 4;
/// A child's body, log10 of its share of its parent's copy mass (`COPY_MASS`): seeds and eggs at
/// -6, calves at -0.3 (14 Oct 2026, the decision to reproduce by surplus; before, the share of each store given to a
/// child, 0.05 to 0.5). A child is made whenever the brood (`Body::brood`) covers its matter
pub const CHILD: usize = 5;
/// Mass needed before copying, log10 kg
pub const COPY_MASS: usize = 6;
/// Days between updates, log10 (kept between 5 and 360 days)
pub const INTERVAL: usize = 7;
/// Mutation size, log10
pub const MUT: usize = 8;
/// Learning rate of the brain within a life, log10 (off at the lower end)
pub const LEARN: usize = 9;
/// Share of what its group learned that fades each month, log10 (off at the lower end): the
/// group forgets at the mean rate of its members
pub const FORGET: usize = 10;
/// Sizes of the brain and the senses (each rounded): hidden units (0..64; 0 is no brain, see
/// `Lineage::none`), memory numbers (0..12), bodies perceived (1..6), sight in cells (1..4). A
/// body whose size genes are a whole unit away from its lineage's sizes starts a new lineage with
/// those sizes.
pub const HIDDEN: usize = 11;
pub const MEMORY: usize = 12;
pub const SEEN: usize = 13;
pub const SIGHT: usize = 14;
/// Share of the children that start in a neighbour cell, 0..1 (for an individual, the chance)
pub const DISPERSE: usize = 15;
/// Temperature above which its catalysts lose their shape, K: their speed-up falls off within a
/// few K (`change::denature`). A firmer catalyst costs more upkeep (`Physics::t_firm`)
pub const DENATURE: usize = 16;
/// Repair, log10: a body wears out when the energy its upkeep turned over reaches
/// `Physics::e_life` x 10^gene per kg; each unit doubles the repair share `Physics::c_repair` of upkeep (`upkeep`)
pub const REPAIR: usize = 17;
/// The child's share of the key liquid, log10 of the factor on its provisions (14 Oct 2026, review 7
/// fix 6): a child gets provisions of every store but body matter in the parent's proportion to its
/// matter, and of the key liquid 10^`WET` times that (at most what the parent holds), so a body can make
/// dry children (below 0) that are nearly dormant (`Body::activity`) and wait until they soak liquid
/// up. At 0 a child gets the same share of everything
pub const WET: usize = 18;
/// Share of what a mature body builds in an update that goes to offspring (the brood), 0..1; the rest
/// is its own growth (14 Oct 2026, the decision to reproduce by surplus). With 1 it stops growing at maturity
pub const ALLOC: usize = 19;
pub const NSCALAR: usize = 20;

/// Death causes: a store below its safe minimum; a key liquid store froze or boiled; the
/// body could not pay its upkeep; taken apart by another body.
pub const LACK: u8 = 0;
pub const PHASE: u8 = 1;
pub const SPENT: u8 = 2;
pub const TAKEN: u8 = 3;
/// Bonds broken by bodies moving through its cell.
pub const BROKEN: u8 = 4;
/// Worn out by living (`Body::wear`).
pub const AGE: u8 = 5;
/// Others a body remembers.
pub const KNOWN: usize = 4;

const DAY: f64 = 86_400.0;
/// Updates are kept in a ring of this many days (more than the longest interval).
const RING: u64 = 384;
/// Shortest and longest interval between updates, days (the shortest is a speed safeguard; the
/// longest is one year, so a body that has nothing to decide can update once a year).
const MIN_INTERVAL: f64 = 5.0;
const MAX_INTERVAL: f64 = 360.0;

/// Where the genes of each kind start, for a chemistry.
#[derive(Clone, Copy)]
pub struct Layout {
    pub cat: usize,
    pub min: usize,
    pub upt: usize,
    pub keep: usize,
    pub len: usize,
}

impl Layout {
    pub fn of(chem: &Chemistry) -> Self {
        let (nr, ns) = (chem.reactions.len(), chem.ns());
        let cat = NSCALAR;
        Layout { cat, min: cat + nr, upt: cat + nr + ns, keep: cat + nr + 2 * ns, len: cat + nr + 3 * ns }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Body {
    pub gen: u32,
    pub alive: bool,
    pub cell: u32,
    /// Members this node stands for (1: an individual; more: a population node, plan 7.1). Its
    /// stores and heat are the members' totals, its genes and brain one (their mean), so every
    /// law runs on the totals: rates are per kg or per member, times the count. Per member: the
    /// size a brain perceives, the mass compared with the copy mass. Use `n()`.
    #[serde(default = "one")]
    pub count: f64,
    pub lineage: u32,
    pub parent: u64,
    pub born: u64,
    /// Day of the last update
    pub last: u64,
    /// The cell's cumulative light at the last update, J/m², moved since by what its shade gave
    /// it more or less than the cell's mean (`Life::shade`): it caught `light - mark` per m²
    pub mark: f64,
    /// Thermal energy, J
    pub e: f64,
    /// mol of each substance
    pub store: Vec<f64>,
    pub genes: Vec<f32>,
    /// How a node's members differ (plan 7.1): the variance of their genes around `genes`, per
    /// gene, in gene units squared (empty: none), and of their born brain rows around the node's,
    /// the same for every row (`copy`)
    #[serde(default)]
    pub spread: Vec<f32>,
    #[serde(default)]
    pub row_spread: f32,
    /// The brain rows it was born with (genes too), in `Life::genomes`
    pub genome: u32,
    /// Key stores (safe minimum > 0) that were liquid at birth, one bit per substance
    pub wet: u64,
    /// Body matter built for offspring and held back, mol (all members; part of the matter in `store`,
    /// not counted in the body: `size`, `built`). A mature body puts the share `ALLOC` of what it
    /// builds here (`copy::grow`); `copy::copy` turns it into children of the size the `CHILD` gene sets
    #[serde(default)]
    pub brood: f64,
    /// Visible traits: mass kg, light-capture area m², strength, colour (3 bands)
    pub vis: [f32; 6],
    /// Brain group; the other group it has stayed near, and since when (day)
    pub group: u32,
    pub near: u32,
    pub since: u64,
    /// Its own adjustment of the action scores: fixed actions, moving, the verbs
    pub adj: [f32; FIXED + 1 + VERBS + OVERBS],
    /// The part it holds, and a part of the structure it is inside (ids, 0 if none)
    #[serde(default)]
    pub hold: u64,
    #[serde(default)]
    pub inside: u64,
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
    #[serde(default)]
    pub wear: f64,
    /// Numbers that follow from the genes (rebuilt when missing)
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
    pub interval: u64,
    /// Catalysed reactions that release heat, with the heat per unit, J
    pub burn: Vec<(usize, f64)>,
    /// What a brain perceives of each safe minimum; the substances taken, by selectivity
    pub minln: Vec<f32>,
    pub upt: Vec<(usize, f32)>,
    /// Its catalysts' capacity (`upkeep`; an exponential per catalyst, so kept)
    pub cats: f64,
}

fn one() -> f64 {
    1.0
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
        self.store.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum()
    }
    /// `mass` and `heat_cap` in one pass over the stores (the same sums).
    pub fn mass_cap(&self, chem: &Chemistry) -> (f64, f64) {
        self.store.iter().zip(&chem.subs).fold((0.0, 0.0), |(m, c), (a, s)| (m + a * s.molar_mass, c + a * s.cp))
    }
    pub fn temp(&self, chem: &Chemistry) -> f64 {
        self.e / self.heat_cap(chem).max(1e-9)
    }
    /// Heat conductance to the surface, W/K: through each member's surface, which grows as
    /// mass^(2/3), so a small body cools faster per kg than a big one. Needs `prepare`.
    pub fn conductance(&self, m_ref: f64, mass: f64) -> f64 {
        let n = self.n();
        self.k.heat * n * m_ref.cbrt() * (mass / n).max(0.0).powf(2.0 / 3.0)
    }
    pub fn interval(&self) -> u64 {
        if self.k.interval > 0 {
            return self.k.interval;
        }
        self.dec(INTERVAL).round().clamp(MIN_INTERVAL, MAX_INTERVAL) as u64
    }
    /// Fill `k` from the genes (and the physics, which stays the same through a run) if it is missing.
    pub fn prepare(&mut self, chem: &Chemistry, p: &crate::Physics) {
        if !self.k.cat.is_empty() {
            return;
        }
        let cat = Layout::of(chem).cat;
        let f: Vec<f64> = (0..chem.reactions.len()).map(|r| if self.genes[cat + r] > 0.0 { self.dec(cat + r) } else { 1.0 }).collect();
        let burn = (0..chem.reactions.len()).filter(|&r| self.genes[cat + r] > 0.0 && chem.dh(r) < 0.0).map(|r| (r, -chem.dh(r))).collect();
        let lay = Layout::of(chem);
        let minln = (0..chem.ns()).map(|k| 0.25 * ln(self.genes[lay.min + k].max(0.0) + 1e-2)).collect();
        let upt = (0..chem.ns()).filter(|&k| self.genes[lay.upt + k] > 0.0).map(|k| (k, self.genes[lay.upt + k])).collect();
        // By capacity: the catalysed rate per unit of input at `t_cat`, over the genes that are on
        // (`Physics::c_cat`), dearer if they keep their shape when warmer (`Physics::t_firm`). A gas
        // counted beyond first order (its `nu` above 1: mass action) counts at its share of the open
        // air (`Chemistry::air`) to the power `nu` - 1: the first order is the supply, as the scarce
        // input counts once in the rate (`change`). Without it burning body matter (6 of the gas,
        // 0.21^6 = 8.6e-5 of its bare rate in the open) would cost 1,800 times what burning the
        // energy-rich solid does for the same speed-up, and no body could keep the enzyme.
        let open = |rx: &crate::chem::Reaction| -> f64 {
            rx.inputs.iter().filter(|q| q.1 > 1 && chem.phase(q.0 as usize, p.t_cat) == Phase::Gas).map(|&(k, nu)| chem.air.get(k as usize).copied().filter(|&x| x > 0.0).unwrap_or(1.0).powi(nu as i32 - 1)).product()
        };
        let cats: f64 = chem.reactions.iter().zip(&f).filter(|x| *x.1 > 1.0).map(|(rx, f)| f * rx.k * (-rx.ea / p.t_cat).exp() * open(rx)).sum();
        let cats = cats * (1.0 + (self.gene(DENATURE) - p.t_firm).max(0.0) / p.c_firm);
        self.k = Consts { gas: self.dec(GAS), liq: self.dec(LIQ), heat: self.dec(HEAT), leaf: self.dec(LEAF), cat: f, interval: self.interval(), burn, minln, upt, cats };
    }
    /// Visible traits of one member; the colour is the reflectance of the stores by volume. Its
    /// light-capture area is `LEAF` per kg-equivalent of its body (`size`, `equiv`: mass^0.75).
    pub fn visible(&self, chem: &Chemistry, p: &crate::Physics) -> [f32; 6] {
        let (m, cap) = self.mass_cap(chem);
        let m1 = m / self.n();
        let t = self.e / cap.max(1e-9);
        let (mut v, mut c) = (1e-12, [0.0; 3]);
        for (k, &a) in self.store.iter().enumerate() {
            let p = chem.phase(k, t);
            if p != Phase::Gas && a > 0.0 {
                let vol = a * chem.molar_volume(k, p);
                v += vol;
                for (x, r) in c.iter_mut().zip(chem.subs[k].refl[(p == Phase::Liquid) as usize]) {
                    *x += vol * r as f64;
                }
            }
        }
        let leaf = self.equiv(p, self.size(chem)) / self.n() * self.dec(LEAF);
        [m1 as f32, leaf as f32, self.genes[STRENGTH], (c[0] / v) as f32, (c[1] / v) as f32, (c[2] / v) as f32]
    }
    /// Learning rate of its brain within its life (0 when off).
    pub fn learn_rate(&self) -> f32 {
        if self.genes[LEARN] <= -3.9 { 0.0 } else { 10f32.powf(self.genes[LEARN]) }
    }
    /// Share of what its group learned that it would let fade each month (0 when off).
    pub fn forget(&self) -> f32 {
        if self.genes[FORGET] <= -3.9 { 0.0 } else { 10f32.powf(self.genes[FORGET]) }
    }
    /// Key stores (safe minimum above 0), one bit per substance.
    pub fn key_mask(&self, chem: &Chemistry) -> u64 {
        let lay = Layout::of(chem);
        (0..chem.ns()).filter(|&k| self.genes[lay.min + k] > 0.0).fold(0, |m, k| m | 1 << k)
    }
    /// Key stores that are liquid now.
    pub fn key_liquids(&self, chem: &Chemistry) -> u64 {
        let (t, lay) = (self.temp(chem), Layout::of(chem));
        (0..chem.ns()).filter(|&k| self.genes[lay.min + k] > 0.0 && chem.phase(k, t) == Phase::Liquid).fold(0, |m, k| m | 1 << k)
    }
    /// The brood, mol of body matter (all members), no more than the matter there is (a body that
    /// burnt or lost matter holds no more brood than matter).
    pub fn brooded(&self, chem: &Chemistry) -> f64 {
        chem.matter().map_or(0.0, |m| self.brood.max(0.0).min(self.store[m].max(0.0)))
    }
    /// The matter of the body itself, kg (all members): the matter in the store less the brood.
    pub fn matter_kg(&self, chem: &Chemistry) -> f64 {
        chem.matter().map_or(0.0, |m| (self.store[m] - self.brooded(chem)) * chem.subs[m].molar_mass)
    }
    /// The body, kg (all members): its body matter and the liquid it holds with it. The matter is
    /// `Chemistry::matter` (what growth makes, `copy::grow`) less the brood (matter held for offspring,
    /// `brooded`, which is not yet a body); the liquid is the key liquid it keeps,
    /// `h` the most share of its mass it keeps of it (the keep gene, below 0.95, `level`), so a body
    /// is its matter over 1 - `h`, and no more than its matter and all its liquid (a body short of
    /// liquid is as big as it is wet). The fuel it stores (the energy-rich solid, fat, whatever it
    /// burns) and liquid held past `h` are not body (14 Oct 2026, decision after watch
    /// report 7: a body's dry mass here was mostly its sugar store, so building work was charged on
    /// stored fuel and every animal died). The light it catches and the gas it breathes (its leaf,
    /// `visible`, `open_area`), the height order of `shade` and the safe minimums go by it;
    /// intake, upkeep and heat by the whole mass. A chemistry with no body matter
    /// (`Chemistry::build`): the whole mass.
    pub fn size(&self, chem: &Chemistry) -> f64 {
        if chem.matter().is_none() {
            return self.mass(chem);
        }
        let matter = self.matter_kg(chem);
        let (liquid, h) = self.wet_h(chem);
        (matter / (1.0 - h)).min(matter + liquid)
    }
    /// The body its matter makes, kg (all members): its matter (less the brood) over 1 - `h`, as if it
    /// held all the liquid it keeps (`size` without the cap of the liquid it holds now). What growth builds to
    /// (`copy::grow`), and so what maturity reads (`adult`). Without body matter: `size`.
    pub fn built(&self, chem: &Chemistry) -> f64 {
        if chem.matter().is_none() {
            return self.size(chem);
        }
        let (_, h) = self.wet_h(chem);
        self.matter_kg(chem) / (1.0 - h)
    }
    /// Whether the mean member's body (`built`) is at or past its copy mass (the size of maturity its
    /// genes set, `COPY_MASS`): grown, and it may reproduce (`copy::grow` puts a share of what it
    /// builds into offspring, `copy::copy` makes them). A node mixes ages, so its mean member says
    /// only what most of it is.
    pub fn adult(&self, chem: &Chemistry) -> bool {
        self.built(chem) / self.n() >= (1.0 - 1e-9) * self.dec(COPY_MASS)
    }
    /// Activity, 0..1 (14 Oct 2026, review 7 fix 6, a refinement of Change): a
    /// body's chemistry runs in its liquid, so it runs in proportion to the key liquid the body holds
    /// over the level it keeps, to the power `Physics::dormancy`: min(1, l / l*)^n, l the key liquids
    /// it holds, l* what `level` keeps of them (h / (1 - h) of its matter). A body that holds all it
    /// keeps (or more) has 1; a dry one is nearly dormant (n = 5.7: a third of the liquid is 512
    /// times less). The ratio is the mean over the members of a node. A body with no key liquid (the
    /// genes keep none), or none that it keeps a level of (no matter, or h = 0), has nothing to dry:
    /// 1. Upkeep and its decisions' cost (`body::upkeep`), the speed-up of its catalysts
    /// (`catalyst`) and its building (`copy::grow`) go with it; so does the share of a freezing that
    /// kills (`body::live`).
    pub fn activity(&self, chem: &Chemistry, p: &crate::Physics) -> f64 {
        if self.wet == 0 || chem.matter().is_none() {
            return 1.0;
        }
        let (mut have, mut keep) = (0.0, 0.0);
        for k in (0..chem.ns()).filter(|&k| self.wet >> k & 1 == 1) {
            let mm = chem.subs[k].molar_mass;
            have += self.store[k] * mm;
            keep += self.level(chem, k, 0.0) * mm;
        }
        if keep <= 0.0 {
            return 1.0;
        }
        (have / keep).min(1.0).max(0.0).powf(p.dormancy)
    }
    /// A catalyst's factor `f` (the speed-up of a reaction) at activity `a`: 1 + (f - 1) a. A dry
    /// body's enzymes do not work, but the bare reaction still runs (14 Oct 2026).
    #[inline]
    pub fn catalyst(f: f64, a: f64) -> f64 {
        1.0 + (f - 1.0) * a
    }
    /// The key liquids it holds, kg (all members), and the most share of its mass it keeps of any
    /// of them (its keep gene, 0..0.95).
    pub fn wet_h(&self, chem: &Chemistry) -> (f64, f64) {
        let keep = Layout::of(chem).keep;
        (0..chem.ns()).filter(|&k| self.wet >> k & 1 == 1).fold((0.0, 0.0), |(l, h), k| (l + self.store[k] * chem.subs[k].molar_mass, h.max(self.gene(keep + k).clamp(0.0, 0.95))))
    }
    /// The fuel growth converts into one kg of body matter, J (what its catalysed reactions would
    /// release from the inputs of `Chemistry::build`; 0 where bodies do not grow): 1.11 kg of the
    /// energy-rich solid, 17.3 MJ, in chemistry 0 for a body that burns it. Needs `prepare`.
    pub fn build_fuel(&self, chem: &Chemistry, t: f64) -> f64 {
        let Some(bl) = chem.build.as_ref() else { return 0.0 };
        let mm = chem.subs[bl.outputs[0].0 as usize].molar_mass * bl.outputs[0].1 as f64;
        bl.inputs.iter().map(|&(k, nu)| nu as f64 * self.heat_of(chem, k as usize, t)).sum::<f64>() / mm
    }
    /// Can it digest body matter: does it burn it (its catalyst for the reaction that burns the matter
    /// is on)? A body that cannot takes none in (`choose::wanted`). Read from the genes, so it holds
    /// for a body that has not updated yet (a child).
    pub fn digests(&self, chem: &Chemistry) -> bool {
        let (m, cat) = (chem.matter(), Layout::of(chem).cat);
        m.is_some_and(|m| (0..chem.reactions.len()).any(|r| self.genes[cat + r] > 0.0 && chem.dh(r) < 0.0 && chem.reactions[r].inputs.iter().any(|p| p.0 as usize == m)))
    }
    /// Digestion (14 Oct 2026): `taken` mol of body matter that this body took in from outside (the
    /// ground, another body, a part, a gift) are broken into its units as they come in, the reverse
    /// of growth (`Chemistry::build`): 1 of matter + 1 of liquid -> 6 of the energy-rich solid, element
    /// exact and thermoneutral in chemistry 0. So it enters as fuel, never as body; the body
    /// builds its own matter from that, at the cost of work (`copy::grow`). The liquid comes from the
    /// body's own store; with less of it than the matter (a body near death) the rest of the matter
    /// stays matter. Only what was taken is broken down, never the matter the body already had.
    /// Returns the units of growth reversed.
    pub fn digest(&mut self, chem: &Chemistry, taken: f64) -> f64 {
        let Some(bl) = chem.build.as_ref() else { return 0.0 };
        let m = bl.outputs[0].0 as usize;
        let mut x = (taken / bl.outputs[0].1 as f64).min(self.store[m] / bl.outputs[0].1 as f64);
        for &(k, nu) in &bl.outputs[1..] {
            x = x.min(self.store[k as usize] / nu as f64);
        }
        if x <= 0.0 {
            return 0.0;
        }
        for &(k, nu) in &bl.outputs {
            self.store[k as usize] = (self.store[k as usize] - nu as f64 * x).max(0.0);
        }
        for &(k, nu) in &bl.inputs {
            self.store[k as usize] += nu as f64 * x;
        }
        self.e -= -chem.dh_build() * x;
        x
    }
    /// Energy a mole of `k` gives its body's catalysed reactions at temperature `t` (J; 0 for a gas
    /// or what none of its reactions burns): the best over `Consts::burn`. Needs `prepare`.
    pub fn heat_of(&self, chem: &Chemistry, k: usize, t: f64) -> f64 {
        if chem.phase(k, t) == Phase::Gas {
            return 0.0;
        }
        self.k.burn.iter().filter_map(|&(r, e)| chem.reactions[r].inputs.iter().find(|x| x.0 as usize == k).map(|x| e / x.1 as f64)).fold(0.0, f64::max)
    }
    /// Mass-equivalent a body's size is counted by, kg (in place of the mass, for what goes with
    /// mass^0.75 in life): per member `m_ref` times its `mass` over `m_ref`, to the power
    /// `intake_exp` (0.75), times the count. What it takes, sweeps and receives as a gift, what it
    /// can build (`c_build`) and its leaf (`visible`, `open_area`:
    /// of its body, `size`) go by it: the supply. What it pays to be does not (mass^1.0 of its
    /// body, `body::upkeep`, 14 Oct 2026). A member of `m_ref` (1 kg) is 1 kg; one of
    /// 30 kg 12.8 kg-equivalents, 2.3 times less per kg; of 10 t 1,778, so it takes 180 times what
    /// a 10 kg one (10^0.75 = 5.6) does, not 1,000. Light, rain, liquid and gas go by area. How
    /// much of what it takes it can use is `choose::sate`, not a size: a member takes by all its mass,
    /// as far as it can use it (14 Oct 2026, review 5).
    pub fn equiv(&self, p: &crate::Physics, mass: f64) -> f64 {
        let n = self.n();
        n * p.m_ref * (mass / n / p.m_ref).powf(p.intake_exp)
    }
    /// What a body can take and build, in kg-equivalents: `equiv` of the body that is built
    /// (`Body::built`), not of the whole mass (14 Oct 2026, the energy ceiling): the fuel it stores
    /// is carried, not working tissue, so it raises neither what the body can take nor what it
    /// pays to keep (`body::upkeep`, by the same body). Intake (`take`, sweep, gifts), the `c_build`
    /// allowance (`copy::grow`) and satiety go by it; the leaf by the body too (`visible`).
    pub fn supply(&self, chem: &Chemistry, p: &crate::Physics) -> f64 {
        self.equiv(p, self.built(chem))
    }
    /// Surface the body exchanges gas with the air through, m²: per member its light-capture area
    /// (`LEAF` per kg-equivalent of its body, as `visible` and the light code count it)
    /// plus its skin, 6 (m1 / `rho_body`)^(2/3), as the surface of walls and the cover; times the
    /// count. A 1 kg plant of 2 m² per kg: 2.06 m²; a 30 kg eater with 0.03 m² of capture area per
    /// kg-equivalent (12.8): 0.38 + 0.58 m². Needs `prepare`. The `GAS` gene is the air exchanged
    /// per m² of it per day.
    pub fn open_area(&self, chem: &Chemistry, p: &crate::Physics, mass: f64) -> f64 {
        let m1 = mass / self.n();
        self.equiv(p, self.size(chem)) * self.k.leaf + self.n() * 6.0 * (m1 / p.rho_body).powf(2.0 / 3.0)
    }
    /// Most of substance `k` the body holds, mol (14 Oct 2026: stores are carried by the body).
    /// Its keep gene is how much of it the body carries: for a key liquid the largest share of the
    /// body it may make up, `h` (below 0.95), so it holds `h` / (1 - `h`) of its matter and its body
    /// is its matter over 1 - `h` (`size`); for every other substance but body matter, fuel and the
    /// rest, kg of it per kg of its body (`size`; 0.9 at the start, more for a kind placed with more
    /// fuel than that). Before it was a share of the whole mass for each store alone, so several
    /// stores together had no bound, and an eater swelled to 2 t with 2% of its mass matter (watch
    /// report 8). Body matter itself is bounded as before, a share of the whole mass. Liquid caught,
    /// taken or put stops there; the liquid a body keeps is that of its body matter, not of the brood
    /// (14 Oct 2026). It bounds the stores, not the size. `mass` is the whole mass, for matter and for a
    /// chemistry with none.
    pub fn level(&self, chem: &Chemistry, k: usize, mass: f64) -> f64 {
        let mm = chem.subs[k].molar_mass;
        let f = self.gene(Layout::of(chem).keep + k);
        let old = |f: f64| {
            let f = f.clamp(0.0, 0.95);
            (f / (1.0 - f) * (mass - self.store[k] * mm) / mm).max(0.0)
        };
        match chem.matter() {
            Some(m) if k != m => {
                if self.wet >> k & 1 == 1 {
                    let f = f.clamp(0.0, 0.95);
                    f / (1.0 - f) * self.matter_kg(chem) / mm
                } else {
                    f.max(0.0) * self.size(chem) / mm
                }
            }
            _ => old(f),
        }
    }
    /// Well-being at temperature `t`: the energy its catalysed reactions could release from a
    /// member's stores (log, in decisions' worth: `think` is what one decision of the starting
    /// brain costs a member of the mass it copies at, J, so shedding mass does not raise it), less
    /// how near it is to its limits: its key stores to their safe minimums (the log of each one's
    /// margin, which is the log of its share over its minimum, counted up to 2), how fast its reactions can release energy now to its upkeep (`spare`,
    /// the same margin form), and its key liquids to freezing or boiling (within 5 K). Needs
    /// `prepare`.
    pub fn wellbeing_at(&self, chem: &Chemistry, think: f64, _mass: f64, t: f64, spare: f64) -> f32 {
        let (lay, mass) = (Layout::of(chem), self.size(chem));
        let mut w = ln(1.0 + (self.fuel(chem, t) / self.n() / think) as f32) + ln(spare.clamp(1e-3, 2.0) as f32 / 2.0);
        for k in 0..chem.ns() {
            let min = self.genes[lay.min + k];
            if min > 0.0 {
                w += ln(ln((self.store[k] * chem.subs[k].molar_mass / mass) as f32 / min).clamp(1e-3, 2.0) / 2.0);
            }
            if self.wet >> k & 1 == 1 {
                let s = &chem.subs[k];
                w += (((t - s.melt).min(s.boil - t) as f32 - 5.0) / 5.0).min(0.0);
            }
        }
        w
    }
    /// Energy its catalysed reactions that release heat could release from its stores at
    /// temperature `t` (each as far as its inputs that are not gas go), J. Needs `prepare`.
    pub fn fuel(&self, chem: &Chemistry, t: f64) -> f64 {
        let mut e = 0.0;
        for &(r, q) in &self.k.burn {
            let solid = chem.reactions[r].inputs.iter().filter(|p| chem.phase(p.0 as usize, t) != Phase::Gas);
            e += q * solid.map(|&(k, nu)| self.store[k as usize] / nu as f64).fold(f64::MAX, f64::min).min(1e30);
        }
        e
    }
    /// How past dealings with body `id` went, if it is known.
    pub fn recall(&self, id: u64) -> Option<f32> {
        self.known.iter().find(|k| k.0 == id && id != 0).map(|k| k.1)
    }
    /// Remember a dealing with body `id`: better (+) or worse (-). A new one replaces the least
    /// marked.
    pub fn note(&mut self, id: u64, d: f32) {
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
    /// Nothing left of a member (under 1 g each) when the mass is `m`.
    pub fn empty_at(&self, m: f64) -> bool {
        m < 1e-3 * self.n()
    }
    /// A store below its safe minimum, or nothing left of a member.
    pub fn lacking(&self, chem: &Chemistry) -> bool {
        self.lacking_at(chem, self.mass(chem))
    }
    /// `lacking` when the mass `m` is known.
    pub fn lacking_at(&self, chem: &Chemistry, m: f64) -> bool {
        // (A safe minimum is a share of the body, not of the whole mass: a body is not killed by holding fuel.)
        let (lay, size) = (Layout::of(chem), self.size(chem));
        self.empty_at(m) || (0..chem.ns()).any(|k| !self.burns(chem, k) && self.store[k] * chem.subs[k].molar_mass < self.gene(lay.min + k) * size)
    }
    /// Does it burn substance `k` (an input that is not a gas of one of its reactions that release
    /// heat, `Consts::burn`)? A store it burns is fuel: a body short of fuel dies when it cannot pay
    /// its upkeep (`SPENT`), not when the store is below its safe minimum, which is then only the
    /// floor that growth and giving keep (`copy::grow`, `choose::give`): a reserve, not a death line.
    /// False before `prepare`.
    pub fn burns(&self, chem: &Chemistry, k: usize) -> bool {
        self.k.burn.iter().any(|&(r, _)| chem.reactions[r].inputs.iter().any(|p| p.0 as usize == k && chem.phase(k, crate::T_SURF) != Phase::Gas))
    }
}

/// The highest speed-up (log10) of any reaction driven by light in the genes (a raw number; 0
/// if the chemistry has none).
pub fn light_cat(chem: &Chemistry, genes: &[f32]) -> f32 {
    chem.reactions.iter().enumerate().filter(|(_, rx)| rx.light).map(|(r, _)| genes[NSCALAR + r]).fold(0.0, f32::max)
}

/// A body whose light reactions are sped up at least this much (log10) is a light-eater (5 with
/// chemistry 0's light reaction at k 10^-6 is what 8 was at k 10^-9). The tape counts by this, and
/// so does `Config::nodes`: the others are the animals.
pub const LIGHT_EATER: f32 = 5.0;

/// Does a body of these genes live on light (the rest are animals, `Config::nodes`)?
pub fn light_eater(chem: &Chemistry, genes: &[f32]) -> bool {
    light_cat(chem, genes) >= LIGHT_EATER
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Life {
    pub bodies: Vec<Body>,
    free: Vec<u32>,
    /// Bodies in each cell, in a fixed order
    pub at: Vec<Vec<u32>>,
    /// Per cell, from the last survey: members, then the visible traits summed over them (mass
    /// and light-capture area are the totals)
    pub sum: Vec<[f64; 7]>,
    /// Per cell, from the last survey: the share of the light reaching its bodies' capture area
    /// that they absorb (1 - their reflectance, weighted by the light each catches)
    #[serde(skip)]
    pub absorb: Vec<f32>,
    /// Per body slot, from the last survey: its light per m² of capture area over its cell's
    /// mean (`World::light`), by the shade of the taller members above it (`shade`)
    #[serde(skip)]
    pub shade: Vec<f64>,
    /// Per cell, from the last survey: the share of that absorbed light set aside for what its
    /// bodies' light reactions store (`World::keep_of`, weighted by the light each absorbs)
    #[serde(skip)]
    pub keep: Vec<f32>,
    /// Per cell, from the last monthly survey: ground temperature, liquid depth, surface height
    pub env: Vec<[f64; 3]>,
    /// Per cell, from the last monthly survey: its loose substances as a brain perceives them,
    /// [cell * substances + k]
    pub seen: Vec<f32>,
    /// Per cell, from the last survey: what a brain perceives of it as a place to move to
    #[serde(skip)]
    pub cellf: Vec<[f32; 8]>,
    /// Updates due, by day modulo RING: (slot, generation)
    due: Vec<Vec<(u32, u32)>>,
    /// The brain of each lineage (None once it died out), and its living bodies
    pub brains: Vec<Option<Lineage>>,
    pub members: Vec<u32>,
    /// Brain groups (a group with no members is free)
    pub groups: Vec<Group>,
    free_groups: Vec<u32>,
    /// Born brain rows, each shared by the bodies born with them (none: free)
    pub genomes: Vec<Genome>,
    free_genomes: Vec<u32>,
    /// Grid width (for the groups' centres)
    pub gw: u32,
    /// Body updates since the start
    pub updates: u64,
}

/// Born brain rows and how many living bodies were born with them. Most children are born
/// with their parent's; a few with a changed copy (`copy`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Genome {
    pub rows: Vec<f32>,
    pub bodies: u32,
}

/// What the bodies of a lineage share: the sizes of their brain and senses, the base of
/// the brain (its hidden layer), and where it came from.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Lineage {
    pub shape: Shape,
    pub seen: usize,
    pub sight: usize,
    pub base: Vec<f32>,
    /// The row of the starting kinds this lineage descends from (set where a kind is placed,
    /// inherited by every lineage made from another one): only a label, for tallies by origin.
    #[serde(default)]
    pub root: u32,
}

impl Lineage {
    /// A brain of no hidden units is no brain (zero is a size, review 3, Q2): its bodies take no
    /// decisions and pay nothing for a brain (no decision, learning or signal); their update is
    /// their physics only (`body::day`). A lineage gets there by evolution, as it saves the cost.
    pub fn none(&self) -> bool {
        self.shape.h == 0
    }
    /// Energy one decision takes, J per kg of a member of `Physics::m_ref` kg (`Physics::think`
    /// gives it for a member of another mass); none without a brain. It goes with the body's own
    /// size genes, not the lineage's rounded sizes it thinks with (they differ by less than a
    /// unit): so a smaller brain pays less at once, and selection can lead a node's sizes down,
    /// to none, a unit at a time (review 3, Q2; with the lineage's sizes nothing selected a size
    /// gene until it had moved a whole unit by chance).
    pub fn cost(&self, p: &crate::Physics, genes: &[f32]) -> f64 {
        if self.none() {
            return 0.0;
        }
        let g = |k: usize| genes[k].max(0.0) as f64;
        p.c_hidden * g(HIDDEN) + p.c_memory * g(MEMORY) + p.c_seen * g(SEEN) + p.c_sight * g(SIGHT).powi(2)
    }
}

/// Bodies that stayed near each other share the rows of one brain: the mean of the rows they
/// were born with, plus what the group learned (by learning and watching). `rows` holds that
/// sum, kept up to date as members come and go.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Group {
    pub lineage: u32,
    pub members: u32,
    /// Sums of the members' cell coordinates
    pub sx: i64,
    pub sy: i64,
    pub rows: Vec<f32>,
    /// Sums of the members' born rows (kept up to date as members come and go, and rebuilt
    /// from the members at the start of each month, so a run resumed from a snapshot goes on
    /// the same; not saved); and of their forgetting rates
    #[serde(skip)]
    pub sum: Vec<f32>,
    /// Members of its bodies (a node counts its count), as of the start of the month (not saved;
    /// rebuilt with `sum`): a body's decisions move the shared rows by its share of them
    #[serde(skip)]
    pub count: f64,
    pub forget: f32,
    /// Moving average of the square of the critic's error (0 until the first one)
    pub var: f32,
    /// Per learned row, a moving average of the square of its share of the learning steps
    pub ms: Vec<f32>,
}

impl Group {
    pub fn centre(&self) -> (f32, f32) {
        let m = self.members.max(1) as f32;
        (self.sx as f32 / m, self.sy as f32 / m)
    }
    /// The mean of its members' born rows.
    pub fn innate(&self) -> impl Iterator<Item = f32> + '_ {
        let k = 1.0 / self.members.max(1) as f32;
        self.sum.iter().map(move |&s| s * k)
    }
    /// What the group learned: its rows less the mean of its members' born rows.
    pub fn learned(&self) -> impl Iterator<Item = f32> + '_ {
        self.rows.iter().zip(self.innate()).map(|(r, m)| r - m)
    }
}

impl Life {
    pub fn alive(&self) -> impl Iterator<Item = &Body> {
        self.bodies.iter().filter(|b| b.alive)
    }
    pub fn lineages(&self) -> usize {
        self.members.iter().filter(|&&m| m > 0).count()
    }
    pub fn new_lineage(&mut self, l: Lineage) -> u32 {
        self.brains.push(Some(l));
        self.members.push(0);
        (self.brains.len() - 1) as u32
    }
    /// A new group of `lineage` with these rows (its first member brings its born rows, and
    /// the difference counts as learned).
    pub fn new_group(&mut self, lineage: u32, rows: Vec<f32>) -> u32 {
        let g = Group { lineage, rows, ..Default::default() };
        match self.free_groups.pop() {
            Some(i) => {
                self.groups[i as usize] = g;
                i
            }
            None => {
                self.groups.push(g);
                (self.groups.len() - 1) as u32
            }
        }
    }
    /// New born rows (not yet given to a body).
    pub fn new_genome(&mut self, rows: Vec<f32>) -> u32 {
        let g = Genome { rows, bodies: 0 };
        match self.free_genomes.pop() {
            Some(i) => {
                self.genomes[i as usize] = g;
                i
            }
            None => {
                self.genomes.push(g);
                (self.genomes.len() - 1) as u32
            }
        }
    }
    /// Body `b` has no brain (`Lineage::none`).
    pub fn brainless(&self, b: &Body) -> bool {
        self.brains.get(b.lineage as usize).and_then(|l| l.as_ref()).is_some_and(|l| l.none())
    }
    /// The starting kind body `b` descends from (`Lineage::root`); `u32::MAX` if its lineage is gone.
    pub fn root(&self, b: &Body) -> u32 {
        self.brains.get(b.lineage as usize).and_then(|l| l.as_ref()).map_or(u32::MAX, |l| l.root)
    }
    /// The rows body `b` was born with.
    pub fn born(&self, b: &Body) -> &[f32] {
        &self.genomes[b.genome as usize].rows
    }
    /// Count body `b` into group `g` (`d` = 1) or out of it (`d` = -1): its cell in the centre,
    /// its born rows in the sum (the rows shift by the change of the mean, so what the group
    /// learned stays), its forgetting rate.
    pub fn enrol(&mut self, g: u32, b: &Body, d: i64) {
        self.shift(g, b.cell, d);
        let (gr, born) = (&mut self.groups[g as usize], &self.genomes[b.genome as usize].rows);
        let n0 = gr.members as f32;
        gr.members = (gr.members as i64 + d) as u32;
        gr.forget += d as f32 * b.forget();
        if gr.members == 0 {
            (gr.rows, gr.sum, gr.ms, gr.sx, gr.sy, gr.forget) = (Vec::new(), Vec::new(), Vec::new(), 0, 0, 0.0);
            self.free_groups.push(g);
            return;
        }
        gr.sum.resize(gr.rows.len(), 0.0);
        // A new group keeps the rows it was given (the first member's difference counts as learned).
        let (k1, k0, d) = if n0 > 0.0 { (1.0 / gr.members as f32, 1.0 / n0, d as f32) } else { (0.0, 0.0, d as f32) };
        for ((r, s), &x) in gr.rows.iter_mut().zip(gr.sum.iter_mut()).zip(born) {
            let s1 = *s + d * x;
            *r += s1 * k1 - *s * k0;
            *s = s1;
        }
    }
    /// Rebuild the groups' sums of born rows and their counts of members from their bodies, in slot
    /// order.
    pub fn resum(&mut self) {
        for g in self.groups.iter_mut() {
            g.sum.clear();
            g.sum.resize(g.rows.len(), 0.0);
            g.count = 0.0;
        }
        for b in self.bodies.iter().filter(|b| b.alive) {
            let (gr, born) = (&mut self.groups[b.group as usize], &self.genomes[b.genome as usize].rows);
            gr.sum.iter_mut().zip(born).for_each(|(s, &x)| *s += x);
            gr.count += b.n();
        }
    }
    /// Move the centre of group `g` by a member at `cell` (`d` = 1 arrived, -1 left).
    fn shift(&mut self, g: u32, cell: u32, d: i64) {
        let (x, y) = ((cell % self.gw) as i64, (cell / self.gw) as i64);
        let gr = &mut self.groups[g as usize];
        (gr.sx, gr.sy) = (gr.sx + d * x, gr.sy + d * y);
    }
    /// Place a new body on `day`; it first updates `delay` days later (1..=360).
    pub fn add(&mut self, mut b: Body, day: u64, delay: u64) -> usize {
        let slot = match self.free.pop() {
            Some(s) => s as usize,
            None => {
                self.bodies.push(Body::default());
                self.bodies.len() - 1
            }
        };
        (b.gen, b.alive, b.last, b.count) = (self.bodies[slot].gen, true, day, b.n());
        self.at[b.cell as usize].push(slot as u32);
        self.members[b.lineage as usize] += 1;
        self.genomes[b.genome as usize].bodies += 1;
        self.enrol(b.group, &b, 1);
        self.due[((day + delay.clamp(1, MAX_INTERVAL as u64)) % RING) as usize].push((slot as u32, b.gen));
        self.bodies[slot] = b;
        slot
    }
    pub(crate) fn remove(&mut self, slot: usize) {
        let mut b = std::mem::take(&mut self.bodies[slot]);
        let (cell, lin, g) = (b.cell as usize, b.lineage as usize, b.group);
        self.enrol(g, &b, -1);
        self.unref(b.genome);
        (b.alive, b.store, b.genes, b.trace, b.mem) = (false, Vec::new(), Vec::new(), Vec::new(), Vec::new());
        b.gen += 1;
        self.bodies[slot] = b;
        self.at[cell].retain(|&s| s as usize != slot);
        self.members[lin] -= 1;
        if self.members[lin] == 0 {
            self.brains[lin] = None;
        }
        self.free.push(slot as u32);
    }
    /// One body fewer born with rows `g`.
    fn unref(&mut self, g: u32) {
        let gen = &mut self.genomes[g as usize];
        gen.bodies -= 1;
        if gen.bodies == 0 {
            gen.rows = Vec::new();
            self.free_genomes.push(g);
        }
    }
    /// Body `b` (taken out of its slot) now has these genes and born rows; its group follows.
    pub fn reborn(&mut self, b: &mut Body, genes: Vec<f32>, rows: Vec<f32>) {
        if rows == self.genomes[b.genome as usize].rows {
            return self.regene(b, genes);
        }
        let g = self.new_genome(rows);
        let new = Body { cell: b.cell, genome: g, genes, ..Default::default() };
        self.genomes[g as usize].bodies += 1;
        self.enrol(b.group, &new, 1);
        self.enrol(b.group, b, -1);
        self.unref(b.genome);
        (b.genome, b.genes, b.k) = (g, new.genes, Consts::default());
    }
    /// Body `b` (taken out of its slot) has these genes and the born rows it had (a member split
    /// off a node and back): of its group only the forgetting rate changes (counting the rows
    /// out and in again only rounded the group's rows, and cost two passes over them).
    pub fn regene(&mut self, b: &mut Body, genes: Vec<f32>) {
        let old = b.forget();
        (b.genes, b.k) = (genes, Consts::default());
        let gr = &mut self.groups[b.group as usize];
        gr.forget = gr.forget + b.forget() - old;
    }
    /// Body `b` (taken out of its slot) moves to `lineage`, born with `rows`, in group `group`
    /// (new, for it); its old lineage, group and born rows lose it.
    pub fn move_to(&mut self, b: &mut Body, lineage: u32, group: u32, rows: Vec<f32>) {
        self.enrol(b.group, b, -1);
        self.unref(b.genome);
        let old = b.lineage as usize;
        self.members[old] -= 1;
        if self.members[old] == 0 {
            self.brains[old] = None;
        }
        (b.lineage, b.group, b.genome) = (lineage, group, self.new_genome(rows));
        self.genomes[b.genome as usize].bodies += 1;
        self.members[lineage as usize] += 1;
        self.enrol(group, b, 1);
    }
    pub fn relocate(&mut self, slot: usize, g: u32, from: usize, to: usize) {
        self.at[from].retain(|&s| s as usize != slot);
        self.at[to].push(slot as u32);
        self.shift(g, to as u32, 1);
        self.shift(g, from as u32, -1);
    }
}

/// Energy a mole of `k` carries apart from its heat while in the air (fusion and vaporisation).
#[inline]
fn held(chem: &Chemistry, k: usize) -> f64 {
    chem.subs[k].lf + chem.subs[k].lv
}

/// Fusion enthalpy a mole of `k` holds in the ground's heat at temperature `t`.
#[inline]
pub fn fused(chem: &Chemistry, k: usize, t: f64) -> f64 {
    if t >= chem.subs[k].melt { chem.subs[k].lf } else { 0.0 }
}

/// Temperature (as of the last flow step), liquid depth and surface height of a ground cell.
pub fn env_of(w: &World, i: usize) -> [f64; 3] {
    let t = w.scratch.temps().0.get(i).copied().unwrap_or_else(|| w.t_ground(i));
    let (mut z, mut d) = (0.0, 0.0);
    for k in 0..w.chem.ns() {
        z += w.b(k, i) * w.chem.molar_volume(k, Phase::Solid);
        match w.chem.phase(k, t) {
            Phase::Solid => z += w.g(k, i) * w.chem.molar_volume(k, Phase::Solid),
            Phase::Liquid => d += w.g(k, i) * w.chem.molar_volume(k, Phase::Liquid),
            Phase::Gas => {}
        }
    }
    [t, d / w.grid.area(), w.base_z + z / w.grid.area()]
}

/// Before each physics step: what each cell holds (sums of visible traits), its bodies'
/// light-capture area per ground area, the share of the light they absorb and set aside, and
/// once a month each cell's surroundings.
pub fn survey(w: &mut World) {
    let n = w.grid.n();
    if w.life.at.len() != n {
        (w.life.at, w.life.due, w.life.gw) = (vec![Vec::new(); n], vec![Vec::new(); RING as usize], w.grid.w as u32);
    }
    if w.life.due.len() as u64 != RING {
        // A snapshot with a shorter ring: file each update again on the day it is due.
        let m = w.life.due.len() as u64;
        let old = std::mem::replace(&mut w.life.due, vec![Vec::new(); RING as usize]);
        for (s, v) in old.into_iter().enumerate() {
            w.life.due[((w.day + (s as u64 + m - w.day % m) % m) % RING) as usize].extend(v);
        }
    }
    if w.canopy.len() != n {
        (w.canopy, w.light) = (vec![0.0; n], vec![0.0; n]);
    }
    bond::ready(w);
    let monthly = w.day % w.cfg.days_per_month as u64 == 0;
    if monthly || w.life.groups.iter().any(|g| g.members > 0 && g.sum.is_empty()) {
        w.life.resum();
    }
    if monthly {
        crowd(w);
    }
    shade(w);
    let keep = |s: usize, g: u32| w.keep_of(s, g) as f64;
    // Per cell: light absorbed per J reaching the capture area, the share of it set aside, and
    // what its bodies can claim of the set-aside light at their updates (at most: walls aside).
    let (mut abs, mut kept, mut claim) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let mut sum = vec![[0.0; 7]; n];
    for (slot, b) in w.life.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
        let (i, m, k) = (b.cell as usize, b.n(), keep(slot, b.gen));
        sum[i][0] += m;
        for (x, &v) in sum[i][1..].iter_mut().zip(&b.vis) {
            *x += v as f64 * m;
        }
        let a = b.vis[1] as f64 * m * (1.0 - (b.vis[3] + b.vis[4] + b.vis[5]) as f64 / 3.0);
        let s = a * w.life.shade[slot];
        (abs[i], kept[i], claim[i]) = (abs[i] + s, kept[i] + s * k, claim[i] + a * k * (w.light[i] - b.mark));
    }
    let World { life, grid, canopy, light_pool, e_ground, .. } = w;
    life.sum = sum;
    (life.absorb.resize(n, 0.0), life.keep.resize(n, 0.0), light_pool.resize(n, 0.0));
    let area = grid.area();
    for i in 0..n {
        let s = life.sum[i][2];
        canopy[i] = (s / area) as f32;
        life.absorb[i] = if s > 0.0 { (abs[i] / s) as f32 } else { 0.0 };
        life.keep[i] = if abs[i] > 0.0 { (kept[i] / abs[i]) as f32 } else { 0.0 };
        // Set-aside light its bodies can no longer claim (one died, moved or merged before its
        // update, or caught less than its share) becomes the ground's heat.
        let over = (light_pool[i] - claim[i]).max(0.0);
        (light_pool[i], e_ground[i]) = (light_pool[i] - over, e_ground[i] + over);
    }
    if monthly && w.day > 0 {
        choose::regroup(w);
        let d = &mut w.diag;
        d.traffic.resize(n, 0.0);
        d.crossings.resize(n, 0.0);
        let a = 1.0 / w.cfg.months_per_year as f32;
        for (t, c) in d.traffic.iter_mut().zip(d.crossings.iter_mut()) {
            *t += a * (*c * w.cfg.months_per_year as f32 - *t);
            *c = 0.0;
        }
    }
    if monthly || w.life.env.len() != n || w.life.seen.len() != n * w.chem.ns() {
        w.life.env = (0..n).map(|i| env_of(w, i)).collect();
        let (ns, area) = (w.chem.ns(), w.grid.area());
        w.life.seen = (0..n * ns).map(|x| 0.2 * crate::brain::ln(1.0 + (w.ground[(x % ns) * n + x / ns] / area) as f32)).collect();
    }
    choose::cells(w);
}

/// Light passes through the bodies of a cell from the tallest member down: a member's height is
/// `(body / rho_body)^(1/3)` (a body has no shape yet; its body, `Body::size`, not its fuel or
/// the liquid it holds, 14 Oct 2026), so the order is that of member body mass, and
/// bodies of equal height share a layer. A layer of capture area `x` per ground area absorbs
/// `1 - exp(-x)` of what reaches it, so a big member shades a small one and size pays. Together
/// they still intercept `1 - exp(-canopy)`: the cell's light is only shared out (the shades,
/// weighted by capture area, have mean 1). `flow::air::radiation` moves each body's mark by it.
fn shade(w: &mut World) {
    let g = |x: f64| if x > 1e-9 { -(-x).exp_m1() / x } else { 1.0 - 0.5 * x };
    let (Life { bodies, at, shade, .. }, area, chem) = (&mut w.life, w.grid.area(), &w.chem);
    shade.resize(bodies.len(), 1.0);
    let mut l: Vec<(f32, f64, usize)> = Vec::new();
    for s in at.iter() {
        l.clear();
        l.extend(s.iter().map(|&s| (s as usize, &bodies[s as usize])).map(|(s, b)| ((b.size(chem) / b.n()) as f32, b.vis[1] as f64 * b.n() / area, s)));
        l.sort_by(|a, b| b.0.total_cmp(&a.0));
        let (all, mut above, mut k) = (g(l.iter().map(|x| x.1).sum()), 0f64, 0);
        while k < l.len() {
            let e = k + l[k..].iter().take_while(|x| x.0 == l[k].0).count();
            let x: f64 = l[k..e].iter().map(|x| x.1).sum();
            let f = (-above).exp() * g(x) / all;
            l[k..e].iter().for_each(|y| shade[y.2] = f);
            (above, k) = (above + x, e);
        }
    }
}

/// The speed shortcut of plan 7.1, not a law: it coarsens only how members are represented (as
/// population nodes), never how many there are or where, and must give the same patterns as
/// individuals (to be checked at M4). Monthly, in each cell, among the bodies that hold and are
/// inside nothing: a node joins the first larger one alike in genes (`copy::alike`, on the means
/// only, as a split tests its halves, so a split's halves stay apart for selection: a test within
/// their spreads, as children join, `copy::within`, would merge them back every month; never for
/// being of the same lineage, so variants of one lineage stay apart) and of its size (member bodies
/// within `alike` steps of the copy-mass gene, and both mature or both immature, `copy::near`: young and
/// adult nodes stay apart until the young have grown to adult size); then, while more than
/// `cfg.crowd` remain, the two most alike (squared distance of their genes in mutation steps,
/// `copy::steps`) merge, the smaller into the larger, if they are alike: unlike nodes are not
/// merged (merging them blended a new kind into the nearest one every month, review 3, M3),
/// unless the cell holds more than twice `crowd`: then the most alike merge however unlike, down
/// to that. That is a numerical safeguard, not a law: with about a hundred genes, variants of
/// individuals and immigrants are often unlike in one gene (fewer since children join the node
/// whose spread they lie in, `copy::within`), and with no ceiling one 64x64 world
/// gathered about 146,000 bodies in one cell within a few years. So individuals stay individuals
/// while their cell is not over `crowd`, and kinds stay apart up to twice that. Stores and heat add up, genes and born rows become the means by
/// count and the spread of both is kept in the merged node's (`copy::join`), the born rows carried
/// over to the larger's brain where its sizes differ (`copy::merge`); lineage, brain group and
/// memory are the larger's. A node that grew too varied splits again at its next update
/// (`copy::split`), so between two months its halves meet selection apart: the one that fits
/// grows, and a later merge pulls the mean toward it. With `cfg.nodes` off it leaves animals out of
/// the list: they never merge and never count toward `crowd`.
fn crowd(w: &mut World) {
    let (max, lay) = (w.cfg.crowd.max(1), Layout::of(&w.chem));
    for i in 0..w.grid.n() {
        if w.life.at[i].len() <= 1 {
            continue;
        }
        let b = &w.life.bodies;
        let mut list: Vec<(u64, usize)> = w.life.at[i].iter().map(|&s| s as usize).filter(|&s| b[s].hold == 0 && b[s].inside == 0 && (w.cfg.nodes || light_eater(&w.chem, &b[s].genes))).map(|s| (u64::MAX - b[s].n().to_bits(), s)).collect();
        list.sort_unstable();
        let mut head: Vec<usize> = (0..list.len()).collect();
        for x in 0..list.len() {
            let a = &b[list[x].1];
            if a.n() > 1.0 {
                head[x] = (0..x).find(|&y| head[y] == y && crate::copy::alike(&lay, w.cfg.phys.alike, &b[list[y].1].genes, &a.genes) && crate::copy::near(&lay, &w.chem, w.cfg.phys.alike, &b[list[y].1], a)).unwrap_or(x);
            }
        }
        for x in (0..list.len()).filter(|&x| head[x] != x) {
            crate::copy::merge(w, list[x].1, list[head[x]].1);
        }
        let mut left: Vec<usize> = (0..list.len()).filter(|&x| head[x] == x).map(|x| list[x].1).collect();
        if left.len() <= max {
            continue;
        }
        // The most alike pair first (ties: the earlier, larger ones), until `max` are left or no
        // pair is alike: here each within the other's spread, as children join a node
        // (`copy::within`), so variants of one kind that a split did not make go back together;
        // past twice `max`, unlike pairs too (the most alike of them first). A pair's key: unlike
        // or not, then its squared distance in steps. The halves of a split are not touched:
        // a node splits only while its cell is under `max` (`copy::split`).
        let (m, lim) = (left.len(), w.cfg.phys.alike);
        let steps = |w: &World, s: usize| (crate::copy::steps(&lay, &w.life.bodies[s].genes), crate::copy::spread_steps(&lay, &w.life.bodies[s]), (crate::copy::member_log(&w.chem, &w.life.bodies[s]), w.life.bodies[s].adult(&w.chem)));
        let mut v: Vec<(Vec<f32>, Vec<f32>, (f64, bool))> = left.iter().map(|&s| steps(w, s)).collect();
        let reach = lim as f64 * crate::copy::size_step(&lay) as f64;
        let dist = |p: &(Vec<f32>, Vec<f32>, (f64, bool)), q: &(Vec<f32>, Vec<f32>, (f64, bool))| {
            let gap = |g: usize| (p.0[g] - q.0[g]).abs() > lim + crate::copy::WITHIN * (p.1[g] + q.1[g]).sqrt();
            ((0..p.0.len()).any(gap) || (p.2 .0 - q.2 .0).abs() > reach || p.2 .1 != q.2 .1, p.0.iter().zip(&q.0).map(|(x, y)| (x - y) * (x - y)).sum::<f32>())
        };
        let mut d = vec![(true, f32::INFINITY); m * m];
        for p in 0..m {
            for q in p + 1..m {
                d[p * m + q] = dist(&v[p], &v[q]);
            }
        }
        for n in (max..m).rev() {
            let k = (0..m * m).fold(0, |k, x| if d[x] < d[k] { x } else { k });
            if d[k].1 == f32::INFINITY || d[k].0 && n < 2 * max {
                break;
            }
            let (p, q) = (k / m, k % m);
            let (s, t) = if w.life.bodies[left[q]].n() > w.life.bodies[left[p]].n() { (p, q) } else { (q, p) };
            // Count the members of the smaller node when the pair is unlike (a tally, read by the tape).
            if d[k].0 {
                w.diag.unlike += w.life.bodies[left[s]].n();
            }
            crate::copy::merge(w, left[s], left[t]);
            (left[s], v[t]) = (usize::MAX, steps(w, left[t]));
            for o in 0..m {
                let (lo, hi) = (o.min(s), o.max(s));
                d[lo * m + hi] = (true, f32::INFINITY);
                if o != t && left[o] != usize::MAX {
                    let (lo, hi) = (o.min(t), o.max(t));
                    d[lo * m + hi] = dist(&v[o], &v[t]);
                }
            }
        }
    }
}

/// Working memory for body updates.
#[derive(Default)]
pub struct Tmp {
    pool: Vec<f64>,
    pool0: Vec<f64>,
    x: Vec<f64>,
    /// Light caught at the last update per kg per day, J
    pub lit: f64,
    /// At the last update: what its reactions could release over the time against its upkeep
    /// (counted up to 2); the catalyst factors of the reactions that run on their own
    pub spare: f64,
    cat: Vec<f64>,
    /// The amounts of a share of its members that die (`thin`)
    dead: Vec<f64>,
    /// Mass and heat capacity of the body as `live` left it (valid until its stores change)
    pub mass: f64,
    pub cap: f64,
    pub input: Vec<f32>,
    pub thought: Thought,
    /// Features of each option a scorer scored (flat), for learning
    pub feat: Vec<f32>,
    /// By sight, the cells seen in each direction (`choose::act`)
    pub sector: Vec<choose::Sector>,
}

/// Update the bodies due today.
pub fn day(w: &mut World) {
    let due = std::mem::take(&mut w.life.due[(w.day % RING) as usize]);
    if due.is_empty() {
        return;
    }
    // Group by group, cell by cell (a fixed order that keeps memory reads close together).
    let bodies = &w.life.bodies;
    let mut list: Vec<(u64, u32, u32)> = due.iter().filter(|&&(s, g)| bodies[s as usize].alive && bodies[s as usize].gen == g).map(|&(s, g)| ((bodies[s as usize].group as u64) << 32 | bodies[s as usize].cell as u64, s, g)).collect();
    list.sort_unstable();
    let mut rc = Reactor::new(&w.chem);
    let mut tmp = Tmp::default();
    for (_, slot, gen) in list {
        let s = slot as usize;
        if !w.life.bodies[s].alive || w.life.bodies[s].gen != gen {
            continue;
        }
        let mut b = std::mem::take(&mut w.life.bodies[s]);
        b.prepare(&w.chem, &w.cfg.phys);
        w.life.updates += 1;
        match live(w, s, &mut b, &mut rc, &mut tmp) {
            Some(cause) => die(w, s, b, cause),
            None => {
                // A body with no brain takes no decision: its update is its physics only.
                if !w.life.brainless(&b) {
                    choose::act(w, s, &mut b, &mut tmp);
                    // An action can leave nothing of a member (all its stores put or given
                    // away): it dies at once, before another body deals with it.
                    if b.empty_at(b.mass(&w.chem)) {
                        die(w, s, b, LACK);
                        continue;
                    }
                }
                crate::copy::resize(w, s, &mut b);
                crate::copy::split(w, s, &mut b);
                b.vis = b.visible(&w.chem, &w.cfg.phys);
                w.life.due[((w.day + b.interval()) % RING) as usize].push((slot, gen));
                w.life.bodies[s] = b;
            }
        }
    }
}

/// Remains go to the ground node as loose amounts, with the body's heat.
pub fn die(w: &mut World, slot: usize, mut b: Body, cause: u8) {
    bond::let_go(w, &mut b, false);
    let (n, i) = (w.grid.n(), b.cell as usize);
    for (k, &a) in b.store.iter().enumerate() {
        w.ground[k * n + i] += a;
    }
    w.e_ground[i] += b.e;
    let (cat, root, adult) = (light_cat(&w.chem, &b.genes), w.life.root(&b), b.adult(&w.chem));
    w.events.push(Event { day: w.day, kind: EventKind::Died { cause, cat, root, adult }, node: b.cell, value: b.count });
    w.life.bodies[slot] = b;
    w.life.remove(slot);
}

/// Some of a body's matter goes to its cell's ground as loose amounts: `dead` (mol of each
/// substance), with its share of the heat. In a node these are the remains of a share `f` of
/// its members, who died of `cause` (the count falls). False if less than one member is left
/// (the caller lets the node die).
pub fn thin(w: &mut World, b: &mut Body, f: f64, dead: &[f64], cause: u8) -> bool {
    let (n, i, chem) = (w.grid.n(), b.cell as usize, &w.chem);
    // Whether the members who die were grown, judged before their matter leaves the node.
    let adult = b.adult(chem);
    let cap = b.heat_cap(chem).max(1e-9);
    let q = b.e * dead.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>() / cap;
    // The brood of the members who die goes with them: the same share of it as of the matter.
    let matter = chem.matter().map(|m| (m, b.store[m]));
    for (k, &a) in dead.iter().enumerate() {
        let a = a.min(b.store[k]);
        b.store[k] -= a;
        w.ground[k * n + i] += a;
    }
    if let Some((m, before)) = matter.filter(|x| x.1 > 0.0) {
        b.brood = b.brooded(chem).min(before) * (b.store[m] / before);
    }
    b.e -= q;
    w.e_ground[i] += q;
    if b.n() > 1.0 && f > 0.0 {
        let (cat, root) = (light_cat(chem, &b.genes), w.life.root(b));
        w.events.push(Event { day: w.day, kind: EventKind::Died { cause, cat, root, adult }, node: b.cell, value: b.count * f });
        b.count *= 1.0 - f;
    }
    b.count >= 1.0
}

/// A hit tears a share `f` of a body (15 Oct 2026, partial damage): the energy of a bite, of being
/// pushed through, breaks body matter first, which is what holds a body together, and spills the
/// stores that matter held (`dead`, mol of each, with their share of the heat, as `thin`). The torn
/// share is of the body (`Body::built`), so what the hit weighs against one member's body decides:
/// in a node, `f` of one member or more (`f` times the count at least 1) takes whole members, the
/// ones that die (`thin`); less takes part of every member's matter, so the node's mean body falls
/// and its members live. An individual always lives on smaller (its caller decides when it is too
/// little to live). `matter0` is the body matter, kg, it had before the hit. Returns `None` if less
/// than one member is left, else the body matter, kg, torn off members who live on (0 when members
/// died), which the caller counts with `hurt` once it knows the body lives.
pub fn tear(w: &mut World, b: &mut Body, f: f64, dead: &[f64], cause: u8, matter0: f64) -> Option<f64> {
    let whole = b.n() > 1.0 && f * b.n() >= 1.0;
    let alive = thin(w, b, if whole { f } else { 0.0 }, dead, cause);
    alive.then(|| if whole { 0.0 } else { (matter0 - b.matter_kg(&w.chem)).max(0.0) })
}

/// Count `kg` of body matter torn off a body that lived on (`tear`) for its starting kind (`Diag::hurt`).
pub fn hurt(w: &mut World, b: &Body, kg: f64) {
    let root = w.life.root(b) as usize;
    if kg > 0.0 && root != u32::MAX as usize {
        w.diag.hurt.resize(w.diag.hurt.len().max(root + 1), 0.0);
        w.diag.hurt[root] += kg;
    }
}

/// A node short of key stores: the members poorest in them die (members differ around the
/// mean), so the survivors keep those stores and the dead take their share of the rest. The
/// share that dies is the least that lifts every key store to its safe minimum; false if that
/// is all of them.
fn starve(w: &mut World, b: &mut Body, _m: f64, dead: &mut Vec<f64>) -> bool {
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let x = |k: usize| b.store[k] * chem.subs[k].molar_mass;
    // (Safe minimums are shares of the body; the survivors, a share `live` of the members, have that share of it.)
    let size = b.size(chem);
    let short = |&k: &usize| !b.burns(chem, k) && x(k) < b.gene(lay.min + k) * size;
    let live = (0..chem.ns()).filter(short).map(|k| x(k) / (b.gene(lay.min + k) * size).max(1e-30)).fold(1.0, f64::min);
    let f = 1.0 - live.max(0.0);
    dead.clear();
    dead.extend((0..chem.ns()).map(|k| if short(&k) { 0.0 } else { b.store[k] * f }));
    f < 1.0 && thin(w, b, f, dead, LACK) && !b.lacking(&w.chem)
}

/// Flow and Change on one body since its last update. Returns a death cause.
fn live(w: &mut World, slot: usize, b: &mut Body, rc: &mut Reactor, tmp: &mut Tmp) -> Option<u8> {
    let i = b.cell as usize;
    // The ground's temperature as of the last flow step (the monthly survey's is too old to feel
    // a cell that turned hot).
    let [tg, depth, _] = w.life.env[i];
    let tg = w.scratch.temps().0.get(i).copied().unwrap_or(tg);
    let (depth, walls) = (bond::depth(w, b, depth), bond::walls_around(w, b));
    let cover = (w.life.sum[i][1] - b.vis[0] as f64 * b.n()).max(0.0) / w.grid.area();
    let open = 1.0 - walls.map_or(0.0, |x| x.0);
    let days = w.day.saturating_sub(b.last).max(1) as f64;
    let (mut mass, cap) = b.mass_cap(&w.chem);
    b.last = w.day;
    // With nothing left of a member there is no heat capacity to hold a temperature: it dies.
    if b.empty_at(mass) {
        return Some(LACK);
    }

    // Upkeep, paid by the reactions that breathe, as fast as they go at the body's temperature
    // over the time (at least an interval: a decision takes one); its heat is spread over the
    // time like the rest.
    let (upkeep, span) = (upkeep(w, b, days, mass), days.max(b.interval() as f64));
    tmp.spare = plan(w, b, 2.0 * upkeep, span, mass, cap).1 / upkeep.max(1e-300);
    // In a node, the members whose upkeep could not be paid die.
    let (e0, node) = (b.e, b.n() > 1.0);
    let short = pay_at(w, b, upkeep, span, mass, cap, node) / upkeep.max(1e-300);
    if short > 1e-9 && !node {
        return Some(SPENT);
    }
    // Living wears a body (14 Oct 2026): by the energy its upkeep turns over per kg, over what it
    // can turn over in its life (`Physics::e_life`), so a body that lives fast wears fast. An
    // individual dies of age when its wear reaches 1. A node's members are of all ages: this share
    // of them dies now (the death rate of members who each live that long, so the same mean life).
    // (Over the body that is built, the tissue that wears, not the fuel it carries; 14 Oct 2026.)
    let worn = upkeep / (b.built(&w.chem).max(1e-9 * mass) * w.cfg.phys.e_life * b.dec(REPAIR));
    let upkeep = b.e - e0;
    b.e = e0;
    for (f, cause) in [(short, SPENT), (if node { worn.min(1.0) } else { 0.0 }, AGE)] {
        if f > 1e-9 {
            tmp.dead.clear();
            tmp.dead.extend(b.store.iter().map(|a| a * f));
            if !thin(w, b, f, &tmp.dead, cause) {
                b.e += upkeep;
                return Some(cause);
            }
            mass = b.mass(&w.chem);
        }
    }
    b.wear += if node { 0.0 } else { worn };
    if b.wear >= 1.0 {
        b.e += upkeep;
        return Some(AGE);
    }

    // Heat the ground node can give, J (its capacity as of now; it only falls while the body soaks
    // up liquid): found when first needed, but before the body soaks anything up.
    let keep = w.keep_of(slot, b.gen) as f64;
    let World { cfg, chem, grid, ground, bed, e_ground, air, e_air, ledger, light, light_pool, light_keep, diag, .. } = w;
    let mut gcap = None;
    macro_rules! cap {
        () => {
            *gcap.get_or_insert_with(|| crate::cap_ground(chem, grid, cfg, ground, bed, i))
        };
    }
    let (ns, n, na, j) = (chem.ns(), grid.n(), grid.na(), grid.air_of(i));
    let dt = days * DAY;

    // Its conductance to the surface it stands on, W/K (for its reactions, evaporation and the
    // heat below). Walls around it add their conductance (its share of it) in series. So does the cover of the
    // other bodies in its cell, but only for the part of its surface that lies under the cover:
    // its light-capture area is open to the air above (it is part of the canopy), so the body's
    // conductance splits in the shares of the two areas, and only the share of its own surface
    // (6 (mass / `rho_body`)^(2/3) per member, as the space of walls) goes through the cover. A
    // layer of `cover` kg/m² passes `phys.shelter` / `cover` W per K and m² of surface; bare ground
    // adds none. A member of 30 kg (surface 0.58 m², capture area 0.03 m²: 95% under the cover) with
    // 10 kg/m² of cover gets 0.58 x 220 / 10 = 12.8 W/K in series with 28.5 of its 30 W/K = 8.8
    // W/K, and 1.5 W/K stays open: 10.3 W/K in all (13.8 with what it radiates, below), so its
    // 52 W of upkeep warm it 3.8 K (1.6 K bare), 2.2 K more. A 1 kg plant member (surface 0.06
    // m², capture area 2 m²: 3% under the cover) keeps 475 of its 490 W/K open; its 14 W/K under
    // 10 kg/m² of cover (1.3 W/K of cover in series) become 1.2: 476 W/K in all.
    // In parallel, it radiates like every surface (σT⁴, `flow::air::radiation`) to the surface it
    // stands on, exchanging σ (T² + tg²)(T + tg) W per K and m² of its own surface: 6 W/K per
    // m² at 300 K (3.5 W/K for the 30 kg member, beside its 30), 1700 at 3000 K. Cover and walls
    // do not block it: they are matter about as warm as the surface, which takes up what the
    // body radiates and passes it on. Taken at its temperature at the start of the update (and
    // at the end, if it ends warmer).
    let work = b.size(chem);
    let (m1, nb, wk) = (mass / b.n(), b.n(), work / mass);
    // Its light-capture area, all members' (`visible`): of its body, by mass^0.75.
    let leaf = b.equiv(&cfg.phys, work) * b.k.leaf;
    let ab = 6.0 * (m1 / cfg.phys.rho_body).powf(2.0 / 3.0);
    let g0 = b.conductance(cfg.phys.m_ref, mass);
    let g0 = if cover > 0.0 {
        let s = ab / (ab + leaf / nb);
        g0 * (1.0 - s) + 1.0 / (1.0 / (g0 * s) + cover / (cfg.phys.shelter * nb * ab))
    } else {
        g0
    };
    let gc = walls.map_or(g0, |x| 1.0 / (1.0 / g0 + 1.0 / x.1));
    let rad = |t: f64| crate::flow::air::SIGMA * ab * nb * (t * t + tg * tg) * (t + tg);
    let c0 = b.heat_cap(chem);
    let t0 = b.e / c0;
    let g = gc + rad(t0);

    // Light caught since the last update (through the openings of walls around it), less what
    // the body reflects; liquid over the body absorbs its share first. Its energy already
    // warmed the surface as it fell (`flow::air::radiation`), but for the share set aside for
    // what light reactions store: so up to `MAX_INTERVAL` days of sunlight neither reach the
    // ground in one go nor are drawn back from it in one go.
    let refl = (b.vis[3] + b.vis[4] + b.vis[5]) as f64 / 3.0;
    let got = leaf * (light[i] - b.mark) * (1.0 - refl) * open;
    b.mark = light[i];
    let reach = (-depth / cfg.phys.light_depth).exp();
    let lit = got * reach;
    tmp.lit = lit / mass / days;

    // The air the body exchanges gas with in this time (mol), from its air node; liquid over
    // the body slows it.
    let tot: f64 = (0..ns).map(|k| air[k * na + j]).sum();
    let reach_mol = b.k.gas * b.open_area(chem, &cfg.phys, mass) * days * (-depth / cfg.phys.gas_depth).exp();
    tmp.x.clear();
    tmp.pool.clear();
    for k in 0..ns {
        let (a, x) = (air[k * na + j], air[k * na + j] / tot);
        tmp.x.push(x);
        tmp.pool.push((x * reach_mol).min(0.5 * a));
    }
    tmp.pool0.clone_from(&tmp.pool);

    // Catalysed reactions, at the body's temperature held by the surface it stands on; their
    // heat is spread over the time below. The surface holds it as a store of the heat it can
    // conduct into the body over the time (`g` dt `tg`, at the body's temperature): a body
    // well bound to it reacts at a fixed temperature, a big one under cover warms with what
    // its reactions release and cools with what they take, and they cannot take more heat than
    // the body and that store hold. Those that breathe ran on demand (upkeep, actions) and go
    // here at the open world's rate. Above the DENATURE gene its catalysts lose their shape.
    let fixed = g * dt * tg / t0.max(1.0);
    let e_start = t0 * (fixed + c0);
    let mut e = e_start;
    // The catalysts work as far as the body is wet (`Body::activity`): factor 1 + (f - 1) a. A dry
    // plant's light reaction slows with the rest.
    let act = b.activity(chem, &cfg.phys);
    tmp.cat.clear();
    tmp.cat.extend(b.k.cat.iter().map(|&f| Body::catalyst(f, act)));
    for &(r, _) in b.k.burn.iter().filter(|p| breathes(chem, p.0, t0)) {
        tmp.cat[r] = 1.0;
    }
    // Gas they take from the air brings its heat (at the air's temperature) and gives up its
    // fusion and vaporisation enthalpy. Gas they put out takes that enthalpy, as far as half of
    // what the body's heat, the heat released and that store hold pays for it (the rest stays
    // in the body), and leaves with its heat at the body's last temperature (`c_out`, below).
    let ta = e_air[j] / (0..ns).map(|k| air[k * na + j] * chem.subs[k].cp).sum::<f64>().max(1.0);
    let den = (&tmp.cat[..], b.gene(DENATURE), cfg.phys.unfold);
    let used = rc.react(chem, &mut b.store, Some(&mut tmp.pool), &mut e, fixed, Some(den), lit, dt);
    let mut heat = e - e_start + lit - used + upkeep;
    let (mut pay, mut q_in, mut c_out) = (0.0, 0.0, 0.0);
    for k in 0..ns {
        let d = tmp.pool[k] - tmp.pool0[k];
        (pay, heat, q_in) = (pay + d.max(0.0) * held(chem, k), heat - d.min(0.0) * held(chem, k), q_in - d.min(0.0) * chem.subs[k].cp * ta);
    }
    (b.e, e_air[j]) = (b.e + q_in, e_air[j] - q_in);
    let paid = crate::share(0.5 * (b.e + heat + g * dt * tg).max(0.0), pay);
    for k in 0..ns {
        let d = tmp.pool[k] - tmp.pool0[k];
        let sent = if d > 0.0 { d * paid } else { d };
        (b.store[k], air[k * na + j]) = (b.store[k] + d - sent, air[k * na + j] + sent);
        (heat, c_out) = (heat - sent.max(0.0) * held(chem, k), c_out + sent.max(0.0) * chem.subs[k].cp);
    }
    // What its light reactions stored (`LIGHT_YIELD` of the light they used) is paid by the
    // light set aside for it, as far as its share goes (the share it stored last time, of the
    // light it caught); the difference is settled with the ground below. It keeps the share it
    // stored now. The heat released is its own (`own`) but for the light, which is the ground's
    // heat already.
    let stored = crate::change::LIGHT_YIELD * used;
    let took = (keep * got).min(light_pool[i]).max(0.0);
    light_pool[i] -= took;
    if got > 0.0 {
        if light_keep.len() <= slot {
            light_keep.resize(slot + 1, (u32::MAX, 0.0));
        }
        light_keep[slot] = (b.gen, (stored / got) as f32);
    }

    // Liquids the body holds evaporate toward their vapour pressure at its temperature; what
    // falls as liquid on its capture area is caught, from the vapour it falls from. The net
    // goes in or out, up to the level the body keeps, with its latent heat. The latent heat
    // is paid by the body's own heat released (not the light: the ground has it), and by what
    // the surface can conduct into a body that evaporating cools no lower than the vapour's
    // condensing point: through the same conductance as the heat below (cover and walls in
    // series), and once for all the liquids (what one spent is owed by the next), so the heat
    // step never needs more than the surface gives and the body never ends below that point.
    let fall = diag.rain.get(i).map_or(0.0, |&r| r as f64) * b.vis[1] as f64 * b.n() * dt * open;
    tmp.pool.clear();
    tmp.pool.extend((0..ns).map(|k| if chem.phase(k, tg) == Phase::Liquid { air[k * na + j] } else { 0.0 }));
    let vap_tot: f64 = tmp.pool.iter().sum();
    let mass1 = b.mass(chem);
    let mut moved = false;
    for k in 0..ns {
        let (vap, wet) = (tmp.pool[k], b.store[k] > 0.0 && chem.phase(k, t0) == Phase::Liquid);
        if !wet && (fall <= 0.0 || vap <= 0.0) {
            continue;
        }
        let push = if wet { (chem.p_sat(k, t0) / P_REF - tmp.x[k]).max(0.0) } else { 0.0 };
        let out = if push > 0.0 {
            let (own, room) = (heat - lit + took, crate::heat_above_floor(e_ground[i], cap!()));
            let paid = (own + (g * dt * (tg - chem.t_sat(k, tmp.x[k])).max(0.0)).min(room)).max(0.0);
            (push * reach_mol).min(paid / held(chem, k))
        } else {
            0.0
        };
        let caught = if fall > 0.0 && vap > 0.0 { (fall * vap / vap_tot).min(0.5 * vap) } else { 0.0 };
        let net = caught - out;
        let (m, t) = if net > 0.0 { (net.min(b.level(chem, k, mass1) - b.store[k]).max(0.0), tg) } else { (net.max(-b.store[k]), t0) };
        if m != 0.0 {
            moved = true;
            let q = m * chem.subs[k].cp * t;
            air[k * na + j] -= m;
            b.store[k] += m;
            e_air[j] -= q;
            b.e += q;
            heat += m * held(chem, k);
        }
    }

    // Liquids standing on the ground soak in by solubility, faster where the ground is wet,
    // up to the level the body keeps, through the mass that works (`wk`).
    let wet = depth / (depth + cfg.phys.wet_depth);
    let liq = |k: usize| chem.phase(k, tg) == Phase::Liquid && chem.subs[k].solubility > 0.0;
    let in_ground: f64 = (0..ns).filter(|&k| liq(k)).map(|k| ground[k * n + i]).sum();
    if in_ground > 0.0 {
        let _ = cap!();
    }
    let rate = -(-b.k.liq * days).exp_m1() * wet * wk;
    let mass2 = if moved { b.mass(chem) } else { mass1 };
    for k in (0..ns).filter(|&k| in_ground > 0.0 && liq(k)) {
        let x = ground[k * n + i] / in_ground;
        let d = ((b.level(chem, k, mass2) - b.store[k]) * x * rate * chem.subs[k].solubility).min(0.5 * ground[k * n + i]).max(0.0);
        // The ground pays the heat that goes with the liquid and its fusion enthalpy; if it
        // cannot pay it all, the body soaks up less.
        let (per, f) = (chem.subs[k].cp * tg, fused(chem, k, tg));
        let want = d * (per + f);
        let d = if want > 0.0 { d * crate::share(crate::draw(&mut e_ground[i], ledger, cap!() - d * chem.subs[k].cp, want), want) } else { 0.0 };
        b.store[k] += d;
        ground[k * n + i] -= d;
        b.e += d * per;
        heat += d * f;
    }

    // Heat to the surface the body stands on (which radiates it and warms the air): T relaxes
    // toward the ground's temperature plus the warming of the heat released, over the time.
    // Heat taken beyond what the surface can conduct into it (down to 0 K) over the time is
    // taken from its own heat at once (its reactions took it at its temperature). The gas its
    // reactions put out (heat capacity `c_out`) goes the same way until it leaves at the end,
    // with its share of the heat.
    let (mass3, c) = b.mass_cap(chem);
    if c > 0.0 {
        let (r, ct, e0) = (heat.max(-g * dt * tg), c + c_out, b.e + heat - heat.max(-g * dt * tg));
        let relax = |g: f64| ct * (tg + r / dt / g + (e0 / ct - tg - r / dt / g) * (-g * dt / ct).exp());
        let e1 = relax(g);
        let e1 = if e1 > ct * t0 { relax(gc + rad(e1 / ct)) } else { e1 };
        // The ground pays the difference (the light is already in it, but for what was set aside
        // and taken); if it cannot, the body ends colder.
        let give = e1 - b.e - heat + lit - took;
        let got = crate::draw(&mut e_ground[i], ledger, cap!(), give);
        let e1 = if got == give { e1 } else { e1 - (give - got) };
        let out = e1 * c_out / ct;
        (b.e, e_air[j]) = (e1 - out, e_air[j] + out);
    }

    let (mut mass3, mut c) = (mass3, c);
    if b.lacking_at(&w.chem, mass3) {
        if !(b.n() > 1.0 && mass3 >= 1e-3 * b.n() && starve(w, b, mass3, &mut tmp.dead)) {
            return Some(LACK);
        }
        (mass3, c) = b.mass_cap(&w.chem);
    }
    let chem = &w.chem;
    (tmp.mass, tmp.cap) = (mass3, c);
    let t = b.e / c.max(1e-9);
    // A key liquid that boiled kills the body. One that froze kills the share of it that is free
    // liquid to freeze: the frozen share of its key liquid (all of it, the body being at one
    // temperature) times its activity, as a dry body holds little liquid to freeze (14 Oct 2026,
    // review 7 fix 6). A node loses that share of its members, with their stores (`thin`); an
    // individual dies with that chance.
    let key = |k: usize| b.wet >> k & 1 == 1;
    if (0..ns).any(|k| key(k) && chem.phase(k, t) == Phase::Gas) {
        return Some(PHASE);
    }
    if (0..ns).any(|k| key(k) && chem.phase(k, t) != Phase::Liquid) {
        let f = b.activity(chem, &w.cfg.phys);
        if f >= 1.0 - 1e-9 {
            return Some(PHASE);
        }
        if b.n() > 1.0 {
            tmp.dead.clear();
            tmp.dead.extend(b.store.iter().map(|a| a * f));
            if !thin(w, b, f, &tmp.dead, PHASE) {
                return Some(PHASE);
            }
            (tmp.mass, tmp.cap) = b.mass_cap(&w.chem);
        } else if Rng::new(w.seed, w.day, b.id(slot), Purpose::Frost).f64() < f {
            return Some(PHASE);
        }
    }
    // Growth is building body matter (14 Oct 2026, decision after watch report 7): a member
    // makes matter of the energy-rich solid, paid by work, as far as its allowance goes
    // (`copy::grow`). Below the size of maturity (its copy mass) all of it is its own growth; past
    // it a share `ALLOC` goes into the brood, matter held for offspring (14 Oct 2026). Then offspring are made whenever the brood covers a child, by physics and not by
    // choice (review 3, W3): at its update whatever its brain does (`copy::copy`).
    if crate::copy::grow(w, b, days) {
        (tmp.mass, tmp.cap) = b.mass_cap(&w.chem);
    }
    if b.brood > 0.0 && crate::copy::copy(w, slot, b) {
        (tmp.mass, tmp.cap) = b.mass_cap(&w.chem);
    }
    None
}

/// Upkeep over `days` of a body of `mass` kg, J: the cost of keeping tissue, by the kg of the body
/// itself (`Body::built`: its matter and the liquid it keeps with it, as if it held all it keeps,
/// not the fuel it stores nor the brood), mass^1.0: basal, which every body pays whatever its genes (`Physics::c_basal`), catalysts
/// (by their capacity, the catalysed rate per unit of input at `Physics::t_cat` summed over the genes
/// that are on, see `Physics::c_cat`, dearer if they keep their shape when warmer,
/// `Physics::t_firm`), strength, learning and signalling; capture area by the day; and one decision
/// (per member, by the brain's cost and weakly the member's mass, `Physics::think`, mass^0.25);
/// a body with no brain pays none of the learning, signal and decision. Repair is a share `c_repair`
/// of it at `REPAIR` 0 and doubles with each unit of the gene: all of it times 1 - c + c
/// 2^`REPAIR`, so a longer life costs more upkeep, a shorter one less, but never less than the rest
/// (`Physics::c_repair`).
///
/// **Why mass^1.0 while what a body takes and builds goes with mass^0.75** (14 Oct 2026, the growth law of
/// West, Brown and Enquist 2001, ontogenetic growth): supply (`Body::equiv`: intake, the
/// `c_build` allowance, leaf) grows more slowly than the tissue it must keep, so a body grows until
/// keeping what it has takes all it can supply and there is nothing left to build with
/// (`copy::grow` builds from the spare after upkeep): its size is set by its own costs, not by a cap.
/// Across kinds, adult costs then follow Kleiber's mass^0.75 by themselves. (Review 6 had moved these
/// costs to mass^0.75 because every animal evolved small; the main cause then was nodes mixing kinds,
/// since fixed.) The mass is the body that is built, not the whole mass: storing fuel is cheap, tissue
/// dear (decision of 14 Oct 2026 on body matter); and not `size`, which stops at the liquid held
/// now, a lack that dormancy (`activity`, below) already counts. Needs `prepare`.
pub fn upkeep(w: &World, b: &Body, days: f64, mass: f64) -> f64 {
    let sig = b.sig.iter().map(|s| s * s).sum::<f32>() as f64;
    let p = &w.cfg.phys;
    let mind = if w.life.brainless(b) { 0.0 } else { p.c_learn * b.learn_rate() as f64 + p.c_signal * sig };
    let think = w.life.brains.get(b.lineage as usize).and_then(|l| l.as_ref()).map_or(p.c_brain, |l| l.cost(p, &b.genes));
    // The cost of keeping tissue goes with the kg of the body (`Body::built`: not the fuel it
    // stores, nor the liquid it lacks, which dormancy counts below), supply with the mass to the
    // power 0.75 (see above): storing fuel is cheap, building and keeping tissue is dear.
    let tissue = b.built(&w.chem);
    let u = days * (tissue * (p.c_basal + p.c_cat * b.k.cats + p.c_strength * b.gene(STRENGTH) + mind) + p.c_leaf * b.vis[1] as f64 * b.n()) + p.think(think, mass / b.n()) * b.n();
    // Everything it pays is chemistry, which runs in its liquid: a dry body pays almost nothing (and,
    // as wear grows with what it pays, hardly ages): all of it, the decision's cost too, times its
    // activity (14 Oct 2026, review 7 fix 6).
    u * (1.0 - p.c_repair + p.c_repair * 2f64.powf(b.gene(REPAIR))) * b.activity(&w.chem, p)
}

/// Reaction `r` takes an input that is gas at `t`: it breathes, so it runs on demand.
fn breathes(chem: &Chemistry, r: usize, t: f64) -> bool {
    chem.reactions[r].inputs.iter().any(|p| chem.phase(p.0 as usize, t) == Phase::Gas)
}

/// Energy the body's reactions can release over `days` at its temperature now, J (the most
/// `pay` could pay). Needs `prepare`.
pub fn release(w: &World, b: &Body, days: f64) -> f64 {
    let (mass, cap) = b.mass_cap(&w.chem);
    plan(w, b, f64::MAX, days, mass, cap).1
}

/// Pay `cost` J by running the body's catalysed reactions that release heat and breathe, one
/// after the other as far as their inputs allow; gases come from the air the body exchanges
/// with in `days` and go to its cell's air, and the heat stays in the body. False, and nothing
/// paid, if they cannot release that much in that time.
pub fn pay(w: &mut World, b: &mut Body, cost: f64, days: f64) -> bool {
    if cost <= 0.0 {
        return true;
    }
    let (mass, cap) = b.mass_cap(&w.chem);
    let t0 = b.e / cap.max(1e-9);
    let paid = pay_at(w, b, cost, days, mass, cap, false) == 0.0;
    // The heat released leaves the body for its cell's ground within the action (a body's heat
    // relaxes within seconds), so a costly action does not leave it hot; but for what keeps it at
    // its temperature as what its reactions made holds more heat (a large cost, such as a child's,
    // left it cold, and slow).
    let keep = t0 * b.heat_cap(&w.chem);
    if b.e > keep {
        w.e_ground[b.cell as usize] += b.e - keep;
        b.e = keep;
    }
    paid
}

/// How far each reaction of `pay` runs (in the order of `Consts::burn`) to release up to `cost`
/// J, and the energy released. Each goes at most as far as Change's rate at the body's
/// temperature takes it over `days`: catalyst (as far as it keeps its shape at T) · k · exp(−Ea/T) · the activity of its gases (their
/// share of the air to their `nu`, the scarcest input's to `nu − 1`), per unit of its scarcest
/// input (its own stores, the air exchanged): the same law as `change`. So a cold body
/// releases little: the cold cost is the rate, not a rule.
fn plan(w: &World, b: &Body, cost: f64, days: f64, mass: f64, cap: f64) -> ([f64; 64], f64) {
    let (chem, i) = (&w.chem, b.cell as usize);
    let depth = bond::depth(w, b, w.life.env.get(i).map_or(0.0, |e| e[1]));
    let (ns, na, j, air) = (chem.ns(), w.grid.na(), w.grid.air_of(i), &w.air);
    let tot: f64 = (0..ns).map(|k| air[k * na + j]).sum();
    let share = (b.k.gas * b.open_area(chem, &w.cfg.phys, mass) * days * (-depth / w.cfg.phys.gas_depth).exp() / tot).min(0.5);
    let t = b.e / cap.max(1e-9);
    let dry = b.activity(chem, &w.cfg.phys);
    let gas = |k: usize| chem.phase(k, t) == Phase::Gas;
    let mut have = [0.0f64; 64];
    for &(r, _) in &b.k.burn {
        for &(k, _) in &chem.reactions[r].inputs {
            let k = k as usize;
            have[k] = if gas(k) { air[k * na + j] * share } else { b.store[k] };
        }
    }
    let (mut ext, mut got) = ([0.0f64; 64], 0.0);
    for (q, &(r, e)) in b.k.burn.iter().enumerate() {
        let rx = &chem.reactions[r];
        if got >= cost || !breathes(chem, r, t) {
            continue;
        }
        let (lim, who) = crate::change::scarce(&rx.inputs, |k| have[k]);
        let act: f64 = rx.inputs.iter().enumerate().filter(|p| gas(p.1.0 as usize)).map(|(q, &(k, nu))| (air[k as usize * na + j] / tot).powi(nu as i32 - (q == who) as i32)).product();
        let rate = crate::change::denature(Body::catalyst(b.k.cat[r], dry), t, b.gene(DENATURE), w.cfg.phys.unfold) * rx.k * (-rx.ea / t).exp() * act;
        ext[q] = (lim * crate::change::frac(rate * days * DAY)).min((cost - got) / e);
        got += e * ext[q];
        for &(k, nu) in &rx.inputs {
            have[k as usize] -= nu as f64 * ext[q];
        }
    }
    (ext, got.min(cost))
}

/// `pay` when the body's mass and heat capacity are known; returns what was not paid. If
/// `part`, it pays what it can; else all or nothing.
fn pay_at(w: &mut World, b: &mut Body, cost: f64, days: f64, mass: f64, cap: f64, part: bool) -> f64 {
    if cost <= 0.0 {
        return 0.0;
    }
    b.prepare(&w.chem, &w.cfg.phys);
    let (ext, paid) = plan(w, b, cost, days, mass, cap);
    let left = cost - paid;
    if left > 1e-9 * cost && !part {
        return cost;
    }
    let World { chem, grid, air, .. } = w;
    let (na, j, t) = (grid.na(), grid.air_of(b.cell as usize), b.e / cap.max(1e-9));
    let gas = |k: usize| chem.phase(k, t) == Phase::Gas;
    let mut latent = 0.0;
    for (q, &(r, _)) in b.k.burn.iter().enumerate().filter(|p| ext[p.0] > 0.0) {
        let rx = &chem.reactions[r];
        for &(k, nu) in &rx.inputs {
            let k = k as usize;
            let a = if gas(k) { &mut air[k * na + j] } else { &mut b.store[k] };
            let d = (nu as f64 * ext[q]).min(*a);
            *a -= d;
            if gas(k) {
                latent -= d * held(chem, k);
            }
        }
        for &(k, nu) in &rx.outputs {
            let (k, d) = (k as usize, nu as f64 * ext[q]);
            *if gas(k) { &mut air[k * na + j] } else { &mut b.store[k] } += d;
            if gas(k) {
                latent += d * held(chem, k);
            }
        }
    }
    b.e += paid - latent;
    if left > 1e-9 * cost { left } else { 0.0 }
}
