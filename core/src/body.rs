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
pub const NSCALAR: usize = 15;

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
    /// The brain rows it was born with (genes too: a child gets them with small noise)
    pub rows: Vec<f32>,
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

impl Body {
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
    /// Visible traits; the colour is the reflectance of the stores by volume.
    pub fn visible(&self, chem: &Chemistry) -> [f32; 6] {
        let (t, m) = (self.temp(chem), self.mass(chem));
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
        let leaf = m * self.dec(LEAF);
        [m as f32, leaf as f32, self.genes[STRENGTH], (c[0] / v) as f32, (c[1] / v) as f32, (c[2] / v) as f32]
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
    /// Well-being: the energy its catalysed reactions could release from its stores (log, in
    /// decisions' worth for a body of the mass it copies at, so shedding mass does not raise
    /// it), less how near its key stores are to their safe minimums (the log of each one's
    /// margin, which is the log of its share over its minimum, counted up to 2) and its key
    /// liquids to freezing or boiling (within 5 K). Needs `prepare`.
    pub fn wellbeing(&self, chem: &Chemistry, mass: f64) -> f32 {
        let (lay, t) = (Layout::of(chem), self.temp(chem));
        let mut e = 0.0;
        for &(r, q) in &self.k.burn {
            let solid = chem.reactions[r].inputs.iter().filter(|p| chem.phase(p.0 as usize, t) != Phase::Gas);
            e += q * solid.map(|&(k, nu)| self.store[k as usize] / nu as f64).fold(f64::MAX, f64::min).min(1e30);
        }
        let mut w = ln(1.0 + (e / self.dec(COPY_MASS) / C_BRAIN) as f32);
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
    /// A store below its safe minimum, or nothing left.
    pub fn lacking(&self, chem: &Chemistry) -> bool {
        let (m, lay) = (self.mass(chem), Layout::of(chem));
        m < 1e-3 || (0..chem.ns()).any(|k| self.store[k] * chem.subs[k].molar_mass < self.gene(lay.min + k) * m)
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
    /// Per cell, from the last survey: count, then the visible traits summed over its bodies
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
    /// Grid width (for the groups' centres)
    pub gw: u32,
    /// Body updates since the start
    pub updates: u64,
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
    /// Sums of the members' born rows and of their forgetting rates
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
    /// What the group learned: its rows less the mean of its members' born rows.
    pub fn learned(&self) -> impl Iterator<Item = f32> + '_ {
        let n = self.members.max(1) as f32;
        self.rows.iter().zip(&self.sum).map(move |(r, s)| r - s / n)
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
    /// Count body `b` into group `g` (`d` = 1) or out of it (`d` = -1): its cell in the centre,
    /// its born rows in the sum (the rows shift by the change of the mean, so what the group
    /// learned stays), its forgetting rate.
    pub fn enrol(&mut self, g: u32, b: &Body, d: i64) {
        self.shift(g, b.cell, d);
        let gr = &mut self.groups[g as usize];
        let n0 = gr.members as f32;
        gr.members = (gr.members as i64 + d) as u32;
        gr.forget += d as f32 * b.forget();
        if gr.members == 0 {
            (gr.rows, gr.sum, gr.ms, gr.sx, gr.sy, gr.forget) = (Vec::new(), Vec::new(), Vec::new(), 0, 0, 0.0);
            self.free_groups.push(g);
            return;
        }
        gr.sum.resize(gr.rows.len(), 0.0);
        let (n1, d) = (gr.members as f32, d as f32);
        for ((r, s), x) in gr.rows.iter_mut().zip(gr.sum.iter_mut()).zip(&b.rows) {
            let s1 = *s + d * x;
            if n0 > 0.0 {
                *r += s1 / n1 - *s / n0;
            }
            *s = s1;
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
        (b.gen, b.alive, b.last) = (self.bodies[slot].gen, true, day);
        self.at[b.cell as usize].push(slot as u32);
        self.members[b.lineage as usize] += 1;
        self.enrol(b.group, &b, 1);
        self.due[((day + delay.clamp(1, MAX_INTERVAL as u64)) % RING) as usize].push((slot as u32, b.gen));
        self.bodies[slot] = b;
        slot
    }
    fn remove(&mut self, slot: usize) {
        let mut b = std::mem::take(&mut self.bodies[slot]);
        let (cell, lin, g) = (b.cell as usize, b.lineage as usize, b.group);
        self.enrol(g, &b, -1);
        (b.alive, b.store, b.genes, b.rows, b.trace, b.mem) = (false, Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        b.gen += 1;
        self.bodies[slot] = b;
        self.at[cell].retain(|&s| s as usize != slot);
        self.members[lin] -= 1;
        if self.members[lin] == 0 {
            self.brains[lin] = None;
        }
        self.free.push(slot as u32);
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
    let life = &mut w.life;
    life.sum.clear();
    life.sum.resize(n, [0.0; 7]);
    for b in life.bodies.iter().filter(|b| b.alive) {
        let s = &mut life.sum[b.cell as usize];
        s[0] += 1.0;
        for (x, &v) in s[1..].iter_mut().zip(&b.vis) {
            *x += v as f64;
        }
    }
    let area = w.grid.area();
    for (c, s) in w.canopy.iter_mut().zip(&life.sum) {
        *c = (s[2] / area) as f32;
    }
    let monthly = w.day % w.cfg.days_per_month as u64 == 0;
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

/// Working memory for body updates.
#[derive(Default)]
pub struct Tmp {
    pool: Vec<f64>,
    pool0: Vec<f64>,
    x: Vec<f64>,
    /// Light caught at the last update per kg per day, J
    pub lit: f64,
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
    w.events.push(Event { day: w.day, kind: EventKind::Died { cause }, node: b.cell, value: b.id(slot) as f64 });
    w.life.bodies[slot] = b;
    w.life.remove(slot);
}

/// Flow and Change on one body since its last update. Returns a death cause.
fn live(w: &mut World, b: &mut Body, rc: &mut Reactor, tmp: &mut Tmp) -> Option<u8> {
    let i = b.cell as usize;
    let [tg, depth, _] = w.life.env[i];
    let (depth, walls) = (bond::depth(w, b, depth), bond::walls_around(w, b));
    let open = 1.0 - walls.map_or(0.0, |x| x.0);
    let days = w.day.saturating_sub(b.last).max(1) as f64;
    let mass = b.mass(&w.chem);
    b.last = w.day;

    // Upkeep of catalysts, strength and capture area by the day, and of this decision; its
    // heat is spread over the time like the rest.
    let cats = b.k.cats;
    let sig = b.sig.iter().map(|s| s * s).sum::<f32>() as f64;
    let mind = C_LEARN * b.learn_rate() as f64 + C_SIGNAL * sig;
    let think = w.life.brains[b.lineage as usize].as_ref().map_or(C_BRAIN, |l| l.cost());
    let upkeep = days * (mass * (C_CAT * cats + C_STRENGTH * b.gene(STRENGTH) + mind) + C_LEAF * b.vis[1] as f64) + think * mass;
    let e0 = b.e;
    if !pay(w, b, upkeep, days) {
        return Some(SPENT);
    }
    let upkeep = b.e - e0;
    b.e = e0;

    // Heat the ground node can give, J (its capacity as of now; it only falls while the body soaks up liquid).
    let cap = w.heat_cap_ground(i);
    let World { chem, grid, ground, e_ground, air, e_air, ledger, light, diag, .. } = w;
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
    tmp.x.extend((0..ns).map(|k| air[k * na + j] / tot));
    tmp.pool.clear();
    tmp.pool.extend((0..ns).map(|k| (tmp.x[k] * reach_mol).min(0.5 * air[k * na + j])));
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
    let fall = diag.rain.get(i).map_or(0.0, |&r| r as f64) * b.vis[1] as f64 * dt * open;
    tmp.pool.clear();
    tmp.pool.extend((0..ns).map(|k| if chem.phase(k, tg) == Phase::Liquid { air[k * na + j] } else { 0.0 }));
    let vap_tot: f64 = tmp.pool.iter().sum();
    let mass1 = b.mass(chem);
    for k in 0..ns {
        let (vap, wet) = (tmp.pool[k], b.store[k] > 0.0 && chem.phase(k, t0) == Phase::Liquid);
        if !wet && (fall <= 0.0 || vap <= 0.0) {
            continue;
        }
        let push = if wet { (chem.p_sat(k, t0) / P_REF - tmp.x[k]).max(0.0) } else { 0.0 };
        let out = if push > 0.0 {
            let owed = (lit + heat).min(0.0);
            let room = (crate::heat_above_floor(e_ground[i], cap) + owed).max(0.0);
            let paid = (lit + heat).max(0.0) + (b.k.heat * mass * dt * (tg - chem.t_sat(k, tmp.x[k])).max(0.0)).min(room);
            (push * reach_mol).min(paid / held(chem, k))
        } else {
            0.0
        };
        let caught = if fall > 0.0 && vap > 0.0 { (fall * vap / vap_tot).min(0.5 * vap) } else { 0.0 };
        let net = caught - out;
        let (m, t) = if net > 0.0 { (net.min(b.level(chem, k, mass1) - b.store[k]).max(0.0), tg) } else { (net.max(-b.store[k]), t0) };
        if m != 0.0 {
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
    let rate = -(-b.k.liq * days).exp_m1() * wet;
    let mass2 = b.mass(chem);
    for k in (0..ns).filter(|&k| in_ground > 0.0 && liq(k)) {
        let x = ground[k * n + i] / in_ground;
        let d = ((b.level(chem, k, mass2) - b.store[k]) * x * rate * chem.subs[k].solubility).min(0.5 * ground[k * n + i]).max(0.0);
        // The ground pays the heat that goes with the liquid and its fusion enthalpy; if it
        // cannot pay it all, the body soaks up less.
        let (per, f) = (chem.subs[k].cp * tg, fused(chem, k, tg));
        let want = d * (per + f);
        let d = if want > 0.0 { d * crate::share(crate::draw(&mut e_ground[i], ledger, cap, want), want) } else { 0.0 };
        b.store[k] += d;
        ground[k * n + i] -= d;
        b.e += d * per;
        heat += d * f;
    }

    // Heat to the surface the body stands on (which radiates it and warms the air): T relaxes
    // toward the ground's temperature plus the warming of the heat released, over the time.
    // Walls around it add their conductance (its share of it) in series.
    let c = b.heat_cap(chem);
    if c > 0.0 {
        let g = walls.map_or(b.k.heat * mass, |x| 1.0 / (1.0 / (b.k.heat * mass) + 1.0 / x.1));
        let teq = tg + heat / dt / g;
        let e1 = c * (teq + (b.e / c - teq) * (-g * dt / c).exp());
        // The ground pays the difference; if it cannot, the body ends colder.
        let give = e1 - b.e - heat;
        let got = crate::draw(&mut e_ground[i], ledger, cap, give);
        b.e = if got == give { e1 } else { e1 - (give - got) };
    }

    let chem = &w.chem;
    if b.lacking(chem) {
        return Some(LACK);
    }
    let t = b.temp(chem);
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
    b.prepare(&w.chem);
    let depth = bond::depth(w, b, w.life.env.get(b.cell as usize).map_or(0.0, |e| e[1]));
    let World { chem, grid, air, .. } = w;
    let (ns, na, i) = (chem.ns(), grid.na(), b.cell as usize);
    let j = grid.air_of(i);
    let tot: f64 = (0..ns).map(|k| air[k * na + j]).sum();
    let share = (b.k.gas * b.mass(chem) * days * (-depth / GAS_DEPTH).exp() / tot).min(0.5);
    let t = b.temp(chem);
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
    if left > 1e-9 * cost {
        return false;
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
    true
}
