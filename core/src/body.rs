//! Bodies: nodes of the sparse graph that hold substances, heat and genes, with stable IDs
//! (slot and generation). Here: their storage, and Flow and Change on each body since its
//! last update: light, gas and liquid exchange with its cell, its catalysed reactions, heat
//! to the surface, upkeep, and death. Choose (`choose`) and Copy (`copy`) act on them.
//! A body updates every `interval` days; everything in between is integrated at its update.

use crate::brain::{ln, Shape, Thought, FIXED, OVERBS, SIG, VERBS};
use crate::change::Reactor;
use crate::chem::{Chemistry, Phase, P_REF};
use crate::event::{Event, EventKind};
use crate::{bond, choose, World};
use serde::{Deserialize, Serialize};

// Scalar genes, then one per reaction (catalyst, log10 speed-up) and three per substance
// (safe minimum as a share of mass; take selectivity; most share of mass kept). Below zero a
// gene is off.
/// Gas exchange, log10 mol of air per kg per day
pub const GAS: usize = 0;
/// Liquid exchange with the ground, log10 per day
pub const LIQ: usize = 1;
/// Heat conductance to the surface, log10 W/(K kg)
pub const HEAT: usize = 2;
/// Light-capture area, log10 m²/kg
pub const LEAF: usize = 3;
/// Strength, 0..1
pub const STRENGTH: usize = 4;
/// Share of each store given to a child
pub const CHILD: usize = 5;
/// Mass needed before copying, log10 kg
pub const COPY_MASS: usize = 6;
/// Days between updates, log10
pub const INTERVAL: usize = 7;
/// Mutation size, log10
pub const MUT: usize = 8;
/// Learning rate of the brain within a life, log10 (off at the lower end)
pub const LEARN: usize = 9;
/// Share of what its group learned that fades each month, log10 (off at the lower end): the
/// group forgets at the mean rate of its members
pub const FORGET: usize = 10;
/// Sizes of the brain and the senses (each rounded): hidden units (4..64), memory numbers
/// (0..12), bodies perceived (1..6), sight in cells (1..4). A body whose size genes are a whole
/// unit away from its lineage's sizes starts a new lineage with those sizes.
pub const HIDDEN: usize = 11;
pub const MEMORY: usize = 12;
pub const SEEN: usize = 13;
pub const SIGHT: usize = 14;
/// Share of the children that start in a neighbour cell, 0..1 (for an individual, the chance)
pub const DISPERSE: usize = 15;
pub const NSCALAR: usize = 16;

/// Death causes: a store below its safe minimum; a key liquid store froze or boiled; the
/// body could not pay its upkeep; taken apart by another body.
pub const LACK: u8 = 0;
pub const PHASE: u8 = 1;
pub const SPENT: u8 = 2;
pub const TAKEN: u8 = 3;
/// Bonds broken by bodies moving through its cell.
pub const BROKEN: u8 = 4;
/// Others a body remembers.
pub const KNOWN: usize = 4;

const DAY: f64 = 86_400.0;
/// Updates are kept in a ring of this many days (more than the longest interval).
const RING: u64 = 64;
/// Shortest and longest interval between updates, days (the shortest is a speed safeguard).
const MIN_INTERVAL: f64 = 5.0;
const MAX_INTERVAL: f64 = 60.0;
/// Liquid thickness over a body that holds back 63% of the light reaching it, m.
const LIGHT_DEPTH: f64 = 10.0;
/// Liquid thickness over a body that cuts its gas exchange by a factor e, m.
const GAS_DEPTH: f64 = 1.0;
/// Standing liquid depth at which the ground counts as half wet, m.
const WET_DEPTH: f64 = 0.01;
/// Upkeep of catalysts, J per kg per day per decade of speed-up.
const C_CAT: f64 = 2e2;
/// Upkeep of strength, J per kg per day at strength 1.
const C_STRENGTH: f64 = 5e3;
/// Upkeep of light-capture area, J per m² per day.
const C_LEAF: f64 = 2e3;
/// Cost of one decision of the starting brain (12 hidden units, 6 memory numbers, 3 bodies
/// perceived, sight of 1 cell), J per kg; it is the sum of the costs below at those sizes.
pub const C_BRAIN: f64 = 1e5;
/// Cost of one decision per hidden unit, per memory number, per body perceived, and per square
/// cell of sight, J per kg.
const C_HIDDEN: f64 = 5e3;
const C_MEMORY: f64 = 5e3 / 3.0;
const C_SEEN: f64 = 5e3;
const C_SIGHT: f64 = 1.5e4;
/// Upkeep of learning, J per kg per day at learning rate 1.
const C_LEARN: f64 = 3e4;
/// Cost of signalling, J per kg per day at signal size 1.
const C_SIGNAL: f64 = 2e2;

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
    /// The cell's cumulative light at the last update, J/m²
    pub mark: f64,
    /// Thermal energy, J
    pub e: f64,
    /// mol of each substance
    pub store: Vec<f64>,
    pub genes: Vec<f32>,
    /// The brain rows it was born with (genes too), in `Life::genomes`
    pub genome: u32,
    /// Key stores (safe minimum > 0) that were liquid at birth, one bit per substance
    pub wet: u64,
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
    /// Catalyst factor of each reaction (1 when off), and the summed speed-up (decades)
    pub cat: Vec<f64>,
    pub cats: f64,
    pub interval: u64,
    /// Catalysed reactions that release heat, with the heat per unit, J
    pub burn: Vec<(usize, f64)>,
    /// What a brain perceives of each safe minimum; the substances taken, by selectivity
    pub minln: Vec<f32>,
    pub upt: Vec<(usize, f32)>,
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
    pub fn interval(&self) -> u64 {
        if self.k.interval > 0 {
            return self.k.interval;
        }
        self.dec(INTERVAL).round().clamp(MIN_INTERVAL, MAX_INTERVAL) as u64
    }
    /// Fill `k` from the genes if it is missing.
    pub fn prepare(&mut self, chem: &Chemistry) {
        if !self.k.cat.is_empty() {
            return;
        }
        let cat = Layout::of(chem).cat;
        let f: Vec<f64> = (0..chem.reactions.len()).map(|r| if self.genes[cat + r] > 0.0 { self.dec(cat + r) } else { 1.0 }).collect();
        let cats = (0..chem.reactions.len()).map(|r| self.gene(cat + r).max(0.0)).sum();
        let burn = (0..chem.reactions.len()).filter(|&r| self.genes[cat + r] > 0.0 && chem.dh(r) < 0.0).map(|r| (r, -chem.dh(r))).collect();
        let lay = Layout::of(chem);
        let minln = (0..chem.ns()).map(|k| 0.25 * ln(self.genes[lay.min + k].max(0.0) + 1e-2)).collect();
        let upt = (0..chem.ns()).filter(|&k| self.genes[lay.upt + k] > 0.0).map(|k| (k, self.genes[lay.upt + k])).collect();
        self.k = Consts { gas: self.dec(GAS), liq: self.dec(LIQ), heat: self.dec(HEAT), leaf: self.dec(LEAF), cat: f, cats, interval: self.interval(), burn, minln, upt };
    }
    /// Visible traits of one member; the colour is the reflectance of the stores by volume.
    pub fn visible(&self, chem: &Chemistry) -> [f32; 6] {
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
        let leaf = m1 * self.dec(LEAF);
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
    /// Key stores that are liquid now.
    pub fn key_liquids(&self, chem: &Chemistry) -> u64 {
        let (t, lay) = (self.temp(chem), Layout::of(chem));
        (0..chem.ns()).filter(|&k| self.genes[lay.min + k] > 0.0 && chem.phase(k, t) == Phase::Liquid).fold(0, |m, k| m | 1 << k)
    }
    /// Most of substance `k` the body holds, mol: its keep gene is the largest share of the
    /// body's mass it may make up (below 0.95). Liquid caught, taken or put stops there.
    pub fn level(&self, chem: &Chemistry, k: usize, mass: f64) -> f64 {
        let f = self.gene(Layout::of(chem).keep + k).clamp(0.0, 0.95);
        let mm = chem.subs[k].molar_mass;
        (f / (1.0 - f) * (mass - self.store[k] * mm) / mm).max(0.0)
    }
    /// Well-being: the energy its catalysed reactions could release from a member's stores (log,
    /// in decisions' worth for a body of the mass it copies at, so shedding mass does not raise
    /// it), less how near its key stores are to their safe minimums (the log of each one's
    /// margin, which is the log of its share over its minimum, counted up to 2) and its key
    /// liquids to freezing or boiling (within 5 K). Needs `prepare`.
    pub fn wellbeing(&self, chem: &Chemistry, mass: f64) -> f32 {
        self.wellbeing_at(chem, mass, self.temp(chem))
    }
    /// `wellbeing` when the temperature `t` is known.
    pub fn wellbeing_at(&self, chem: &Chemistry, mass: f64, t: f64) -> f32 {
        let lay = Layout::of(chem);
        let mut e = 0.0;
        for &(r, q) in &self.k.burn {
            let solid = chem.reactions[r].inputs.iter().filter(|p| chem.phase(p.0 as usize, t) != Phase::Gas);
            e += q * solid.map(|&(k, nu)| self.store[k as usize] / nu as f64).fold(f64::MAX, f64::min).min(1e30);
        }
        let mut w = ln(1.0 + (e / self.n() / self.dec(COPY_MASS) / C_BRAIN) as f32);
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
    /// A store below its safe minimum, or nothing left of a member.
    pub fn lacking(&self, chem: &Chemistry) -> bool {
        self.lacking_at(chem, self.mass(chem))
    }
    /// `lacking` when the mass `m` is known.
    pub fn lacking_at(&self, chem: &Chemistry, m: f64) -> bool {
        let lay = Layout::of(chem);
        m < 1e-3 * self.n() || (0..chem.ns()).any(|k| self.store[k] * chem.subs[k].molar_mass < self.gene(lay.min + k) * m)
    }
}

/// The highest speed-up (log10) of any reaction driven by light in the genes (a raw number; 0
/// if the chemistry has none).
pub fn light_cat(chem: &Chemistry, genes: &[f32]) -> f32 {
    chem.reactions.iter().enumerate().filter(|(_, rx)| rx.light).map(|(r, _)| genes[NSCALAR + r]).fold(0.0, f32::max)
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

/// What the bodies of a lineage share: the sizes of their brain and senses, and the base of
/// the brain (its hidden layer).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Lineage {
    pub shape: Shape,
    pub seen: usize,
    pub sight: usize,
    pub base: Vec<f32>,
}

impl Lineage {
    /// Energy one decision takes, J per kg of body.
    pub fn cost(&self) -> f64 {
        let sq = |x: usize| (x * x) as f64;
        C_HIDDEN * self.shape.h as f64 + C_MEMORY * self.shape.mem as f64 + C_SEEN * self.seen as f64 + C_SIGHT * sq(self.sight)
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
    /// Rebuild the groups' sums of born rows from their members, in slot order.
    pub fn resum(&mut self) {
        for g in self.groups.iter_mut() {
            g.sum.clear();
            g.sum.resize(g.rows.len(), 0.0);
        }
        for b in self.bodies.iter().filter(|b| b.alive) {
            let (gr, born) = (&mut self.groups[b.group as usize], &self.genomes[b.genome as usize].rows);
            gr.sum.iter_mut().zip(born).for_each(|(s, &x)| *s += x);
        }
    }
    /// Move the centre of group `g` by a member at `cell` (`d` = 1 arrived, -1 left).
    fn shift(&mut self, g: u32, cell: u32, d: i64) {
        let (x, y) = ((cell % self.gw) as i64, (cell / self.gw) as i64);
        let gr = &mut self.groups[g as usize];
        (gr.sx, gr.sy) = (gr.sx + d * x, gr.sy + d * y);
    }
    /// Place a new body on `day`; it first updates `delay` days later (1..=60).
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
        let g = if rows == self.genomes[b.genome as usize].rows { b.genome } else { self.new_genome(rows) };
        let new = Body { cell: b.cell, genome: g, genes, ..Default::default() };
        self.genomes[g as usize].bodies += 1;
        self.enrol(b.group, &new, 1);
        self.enrol(b.group, b, -1);
        self.unref(b.genome);
        (b.genome, b.genes, b.k) = (g, new.genes, Consts::default());
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
/// light-capture area per ground area, and once a month each cell's surroundings.
pub fn survey(w: &mut World) {
    let n = w.grid.n();
    if w.life.at.len() != n {
        (w.life.at, w.life.due, w.life.gw) = (vec![Vec::new(); n], vec![Vec::new(); RING as usize], w.grid.w as u32);
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
    let life = &mut w.life;
    life.sum.clear();
    life.sum.resize(n, [0.0; 7]);
    for b in life.bodies.iter().filter(|b| b.alive) {
        let (s, n) = (&mut life.sum[b.cell as usize], b.n());
        s[0] += n;
        for (x, &v) in s[1..].iter_mut().zip(&b.vis) {
            *x += v as f64 * n;
        }
    }
    let area = w.grid.area();
    for (c, s) in w.canopy.iter_mut().zip(&life.sum) {
        *c = (s[2] / area) as f32;
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

/// A speed shortcut, not a law (plan 7.1; it must give the same patterns as individuals, to be
/// checked at M4): where a cell holds more than `cfg.crowd` individuals of one lineage and brain
/// group, those that hold and are inside nothing merge with the nodes of them there into one
/// population node (the largest body); and nodes of one lineage, or alike in genes
/// (`copy::alike`), merge. Stores and heat add up, genes and born rows become the means by count
/// (`copy::merge`); its lineage and brain group are the largest one's, and its memory.
fn crowd(w: &mut World) {
    let max = w.cfg.crowd;
    for i in 0..w.grid.n() {
        if w.life.at[i].len() <= 1 {
            continue;
        }
        let bodies = &w.life.bodies;
        let free = |s: usize| bodies[s].hold == 0 && bodies[s].inside == 0;
        let mut list: Vec<(u32, u32, u64, usize)> = w.life.at[i].iter().map(|&s| s as usize).filter(|&s| free(s)).map(|s| (bodies[s].lineage, bodies[s].group, u64::MAX - bodies[s].n().to_bits(), s)).collect();
        list.sort_unstable();
        for run in list.chunk_by(|a, b| (a.0, a.1) == (b.0, b.1)) {
            // Largest first: the nodes, then the individuals.
            let nodes = run.iter().take_while(|x| w.life.bodies[x.3].n() > 1.0).count();
            if run.len() - nodes > max {
                for &(_, _, _, s) in &run[1..] {
                    crate::copy::merge(w, s, run[0].3);
                }
            }
        }
        let (bodies, lay) = (&w.life.bodies, Layout::of(&w.chem));
        let mut nodes: Vec<(u64, usize)> = w.life.at[i].iter().map(|&s| s as usize).filter(|&s| bodies[s].n() > 1.0).map(|s| (u64::MAX - bodies[s].n().to_bits(), s)).collect();
        nodes.sort_unstable();
        for x in 1..nodes.len() {
            let (life, s) = (&w.life, nodes[x].1);
            let (a, b) = (&life.bodies[s], |t: usize| &life.bodies[t]);
            let like = |t: usize| b(t).alive && (b(t).lineage == a.lineage || crate::copy::alike(&lay, &b(t).genes, &a.genes));
            if let Some(&(_, t)) = nodes[..x].iter().find(|p| like(p.1)) {
                crate::copy::merge(w, s, t);
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
        b.prepare(&w.chem);
        w.life.updates += 1;
        match live(w, &mut b, &mut rc, &mut tmp) {
            Some(cause) => die(w, s, b, cause),
            None => {
                choose::act(w, s, &mut b, &mut tmp);
                b.vis = b.visible(&w.chem);
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
    let cat = light_cat(&w.chem, &b.genes);
    w.events.push(Event { day: w.day, kind: EventKind::Died { cause, cat }, node: b.cell, value: b.count });
    w.life.bodies[slot] = b;
    w.life.remove(slot);
}

/// Some of a body's matter goes to its cell's ground as loose amounts: `dead` (mol of each
/// substance), with its share of the heat. In a node these are the remains of a share `f` of
/// its members, who died of `cause` (the count falls). False if less than one member is left
/// (the caller lets the node die).
pub fn thin(w: &mut World, b: &mut Body, f: f64, dead: &[f64], cause: u8) -> bool {
    let (n, i, chem) = (w.grid.n(), b.cell as usize, &w.chem);
    let cap = b.heat_cap(chem).max(1e-9);
    let q = b.e * dead.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>() / cap;
    for (k, &a) in dead.iter().enumerate() {
        let a = a.min(b.store[k]);
        b.store[k] -= a;
        w.ground[k * n + i] += a;
    }
    b.e -= q;
    w.e_ground[i] += q;
    if b.n() > 1.0 && f > 0.0 {
        let cat = light_cat(chem, &b.genes);
        w.events.push(Event { day: w.day, kind: EventKind::Died { cause, cat }, node: b.cell, value: b.count * f });
        b.count *= 1.0 - f;
    }
    b.count >= 1.0
}

/// A node short of key stores: the members poorest in them die (members differ around the
/// mean), so the survivors keep those stores and the dead take their share of the rest. The
/// share that dies is the least that lifts every key store to its safe minimum; false if that
/// is all of them.
fn starve(w: &mut World, b: &mut Body, m: f64) -> bool {
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let x = |k: usize| b.store[k] * chem.subs[k].molar_mass;
    let short: Vec<usize> = (0..chem.ns()).filter(|&k| x(k) < b.gene(lay.min + k) * m).collect();
    let kept: f64 = short.iter().map(|&k| x(k)).sum();
    let live = short.iter().map(|&k| (x(k) / b.gene(lay.min + k) - kept) / (m - kept).max(1e-30)).fold(1.0, f64::min);
    let f = 1.0 - live.max(0.0);
    let dead: Vec<f64> = (0..chem.ns()).map(|k| if short.contains(&k) { 0.0 } else { b.store[k] * f }).collect();
    f < 1.0 && thin(w, b, f, &dead, LACK) && !b.lacking(&w.chem)
}

/// Flow and Change on one body since its last update. Returns a death cause.
fn live(w: &mut World, b: &mut Body, rc: &mut Reactor, tmp: &mut Tmp) -> Option<u8> {
    let i = b.cell as usize;
    // The ground's temperature as of the last flow step (the monthly survey's is too old to feel
    // a cell that turned hot).
    let [tg, depth, _] = w.life.env[i];
    let tg = w.scratch.temps().0.get(i).copied().unwrap_or(tg);
    let (depth, walls) = (bond::depth(w, b, depth), bond::walls_around(w, b));
    let open = 1.0 - walls.map_or(0.0, |x| x.0);
    let days = w.day.saturating_sub(b.last).max(1) as f64;
    let (mut mass, cap) = b.mass_cap(&w.chem);
    b.last = w.day;

    // Upkeep of catalysts, strength and capture area by the day, and of this decision; its
    // heat is spread over the time like the rest.
    let cats = b.k.cats;
    let sig = b.sig.iter().map(|s| s * s).sum::<f32>() as f64;
    let mind = C_LEARN * b.learn_rate() as f64 + C_SIGNAL * sig;
    let think = w.life.brains[b.lineage as usize].as_ref().map_or(C_BRAIN, |l| l.cost());
    let upkeep = days * (mass * (C_CAT * cats + C_STRENGTH * b.gene(STRENGTH) + mind) + C_LEAF * b.vis[1] as f64 * b.n()) + think * mass;
    // In a node, the members whose upkeep could not be paid die.
    let (e0, node) = (b.e, b.n() > 1.0);
    let short = pay_at(w, b, upkeep, days, mass, cap, node) / upkeep.max(1e-300);
    if short > 1e-9 && !node {
        return Some(SPENT);
    }
    let upkeep = b.e - e0;
    b.e = e0;
    if short > 1e-9 {
        let dead: Vec<f64> = b.store.iter().map(|a| a * short).collect();
        if !thin(w, b, short, &dead, SPENT) {
            b.e += upkeep;
            return Some(SPENT);
        }
        mass = b.mass(&w.chem);
    }

    // Heat the ground node can give, J (its capacity as of now; it only falls while the body soaks
    // up liquid): found when first needed, but before the body soaks anything up.
    let World { cfg, chem, grid, ground, bed, e_ground, air, e_air, ledger, light, diag, .. } = w;
    let mut gcap = None;
    macro_rules! cap {
        () => {
            *gcap.get_or_insert_with(|| crate::cap_ground(chem, grid, cfg, ground, bed, i))
        };
    }
    let (ns, n, na, j) = (chem.ns(), grid.n(), grid.na(), grid.air_of(i));
    let dt = days * DAY;

    // Light caught since the last update (through the openings of walls around it), less what
    // the body reflects; liquid over the body absorbs its share first.
    let refl = (b.vis[3] + b.vis[4] + b.vis[5]) as f64 / 3.0;
    let got = mass * b.k.leaf * (light[i] - b.mark) * (1.0 - refl) * open;
    b.mark = light[i];
    let reach = (-depth / LIGHT_DEPTH).exp();
    ledger.sun_in += got;
    e_ground[i] += got * (1.0 - reach);
    let lit = got * reach;
    tmp.lit = lit / mass / days;

    // The air the body exchanges gas with in this time (mol), from its air node; liquid over
    // the body slows it.
    let tot: f64 = (0..ns).map(|k| air[k * na + j]).sum();
    let reach_mol = b.k.gas * mass * days * (-depth / GAS_DEPTH).exp();
    tmp.x.clear();
    tmp.pool.clear();
    for k in 0..ns {
        let (a, x) = (air[k * na + j], air[k * na + j] / tot);
        tmp.x.push(x);
        tmp.pool.push((x * reach_mol).min(0.5 * a));
    }
    tmp.pool0.clone_from(&tmp.pool);

    // Catalysed reactions, at the body's temperature held fixed; their heat is spread over the
    // time below.
    let c0 = b.heat_cap(chem);
    let t0 = b.e / c0;
    let fixed = 1e6 * c0;
    let e_start = t0 * (fixed + c0);
    let mut e = e_start;
    let used = rc.react(chem, &mut b.store, Some(&mut tmp.pool), &mut e, fixed, Some(&b.k.cat), lit, dt);
    let mut heat = e - e_start + lit - used + upkeep;
    for k in 0..ns {
        let d = tmp.pool[k] - tmp.pool0[k];
        air[k * na + j] += d;
        heat -= d * held(chem, k);
    }

    // Liquids the body holds evaporate toward their vapour pressure at its temperature; what
    // falls as liquid on its capture area is caught, from the vapour it falls from. The net
    // goes in or out, up to the level the body keeps, with its latent heat. The latent heat
    // is paid by the light and heat released, and by what the surface can conduct into a body
    // that evaporating cools no lower than the vapour's condensing point.
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
            let owed = (lit + heat).min(0.0);
            let room = (crate::heat_above_floor(e_ground[i], cap!()) + owed).max(0.0);
            let paid = (lit + heat).max(0.0) + (b.k.heat * mass * dt * (tg - chem.t_sat(k, tmp.x[k])).max(0.0)).min(room);
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
    // up to the level the body keeps.
    let wet = depth / (depth + WET_DEPTH);
    let liq = |k: usize| chem.phase(k, tg) == Phase::Liquid && chem.subs[k].solubility > 0.0;
    let in_ground: f64 = (0..ns).filter(|&k| liq(k)).map(|k| ground[k * n + i]).sum();
    if in_ground > 0.0 {
        let _ = cap!();
    }
    let rate = -(-b.k.liq * days).exp_m1() * wet;
    let mass2 = if moved { b.mass(chem) } else { mass1 };
    for k in (0..ns).filter(|&k| in_ground > 0.0 && liq(k)) {
        let x = ground[k * n + i] / in_ground;
        let d = ((b.level(chem, k, mass2) - b.store[k]) * x * rate * chem.subs[k].solubility).min(0.5 * ground[k * n + i]).max(0.0);
        // The ground pays the heat that goes with the liquid and its fusion enthalpy; if it
        // cannot pay it all, the body soaks up less.
        let (per, f) = (chem.subs[k].cp * tg, fused(chem, k, tg));
        let want = d * (per + f);
        let d = if want > 0.0 { d * crate::share(crate::draw(&mut e_ground[i], ledger, cap!(), want), want) } else { 0.0 };
        b.store[k] += d;
        ground[k * n + i] -= d;
        b.e += d * per;
        heat += d * f;
    }

    // Heat to the surface the body stands on (which radiates it and warms the air): T relaxes
    // toward the ground's temperature plus the warming of the heat released, over the time.
    // Walls around it add their conductance (its share of it) in series.
    let (mass3, c) = b.mass_cap(chem);
    if c > 0.0 {
        let g = walls.map_or(b.k.heat * mass, |x| 1.0 / (1.0 / (b.k.heat * mass) + 1.0 / x.1));
        let teq = tg + heat / dt / g;
        let e1 = c * (teq + (b.e / c - teq) * (-g * dt / c).exp());
        // The ground pays the difference; if it cannot, the body ends colder.
        let give = e1 - b.e - heat;
        let got = crate::draw(&mut e_ground[i], ledger, cap!(), give);
        b.e = if got == give { e1 } else { e1 - (give - got) };
    }

    let (mut mass3, mut c) = (mass3, c);
    if b.lacking_at(&w.chem, mass3) {
        if !(b.n() > 1.0 && mass3 >= 1e-3 * b.n() && starve(w, b, mass3)) {
            return Some(LACK);
        }
        (mass3, c) = b.mass_cap(&w.chem);
    }
    let chem = &w.chem;
    (tmp.mass, tmp.cap) = (mass3, c);
    let t = b.e / c.max(1e-9);
    if (0..ns).any(|k| b.wet >> k & 1 == 1 && chem.phase(k, t) != Phase::Liquid) {
        return Some(PHASE);
    }
    None
}

/// Pay `cost` J by running the body's catalysed reactions that release heat, one after the
/// other as far as their inputs allow; gases come from the air the body exchanges with in
/// `days` and go to its cell's air, and the heat stays in the body. False, and nothing paid,
/// if they cannot release that much.
pub fn pay(w: &mut World, b: &mut Body, cost: f64, days: f64) -> bool {
    if cost <= 0.0 {
        return true;
    }
    let ((mass, cap), e0) = (b.mass_cap(&w.chem), b.e);
    let paid = pay_at(w, b, cost, days, mass, cap, false) == 0.0;
    // The heat released leaves the body for its cell's ground within the action (a body's heat
    // relaxes within seconds), so a costly action does not leave it hot.
    if b.e > e0 {
        w.e_ground[b.cell as usize] += b.e - e0;
        b.e = e0;
    }
    paid
}

/// `pay` when the body's mass and heat capacity are known; returns what was not paid. If
/// `part`, it pays what it can; else all or nothing.
fn pay_at(w: &mut World, b: &mut Body, cost: f64, days: f64, mass: f64, cap: f64, part: bool) -> f64 {
    if cost <= 0.0 {
        return 0.0;
    }
    b.prepare(&w.chem);
    let depth = bond::depth(w, b, w.life.env.get(b.cell as usize).map_or(0.0, |e| e[1]));
    let World { chem, grid, air, .. } = w;
    let (ns, na, i) = (chem.ns(), grid.na(), b.cell as usize);
    let j = grid.air_of(i);
    let tot: f64 = (0..ns).map(|k| air[k * na + j]).sum();
    let share = (b.k.gas * mass * days * (-depth / GAS_DEPTH).exp() / tot).min(0.5);
    let t = b.e / cap.max(1e-9);
    let gas = |k: usize| chem.phase(k, t) == Phase::Gas;
    let mut have = [0.0f64; 64];
    for &(r, _) in &b.k.burn {
        for &(k, _) in &chem.reactions[r].inputs {
            let k = k as usize;
            have[k] = if gas(k) { air[k * na + j] * share } else { b.store[k] };
        }
    }
    let (mut ext, mut left) = ([0.0f64; 64], cost);
    for (q, &(r, e)) in b.k.burn.iter().enumerate() {
        if left <= 0.0 {
            break;
        }
        let inputs = &chem.reactions[r].inputs;
        let lim = inputs.iter().map(|&(k, nu)| have[k as usize] / nu as f64).fold(f64::MAX, f64::min);
        ext[q] = lim.min(left / e);
        left -= e * ext[q];
        for &(k, nu) in inputs {
            have[k as usize] -= nu as f64 * ext[q];
        }
    }
    if left > 1e-9 * cost && !part {
        return cost;
    }
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
    b.e += cost - left - latent;
    if left > 1e-9 * cost { left } else { 0.0 }
}
