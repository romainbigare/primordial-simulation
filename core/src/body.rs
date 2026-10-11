//! Bodies: nodes of the sparse graph that hold substances, heat and genes, with stable IDs
//! (slot and generation). Here: their storage, and Flow and Change on each body since its
//! last update: light, gas and liquid exchange with its cell, its catalysed reactions, heat
//! to the surface, upkeep, and death. Choose (`choose`) and Copy (`copy`) act on them.
//! A body updates every `interval` days; everything in between is integrated at its update.

use crate::brain::Thought;
use crate::change::Reactor;
use crate::chem::{Chemistry, Phase, P_REF};
use crate::event::{Event, EventKind};
use crate::{choose, World};
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
pub const NSCALAR: usize = 9;

/// Death causes: a store below its safe minimum; a key liquid store froze or boiled; the
/// body could not pay its upkeep; taken apart by another body.
pub const LACK: u8 = 0;
pub const PHASE: u8 = 1;
pub const SPENT: u8 = 2;
pub const TAKEN: u8 = 3;

/// A body whose light reactions are sped up at least this much (log10) is a light-eater.
pub const LIGHT_EATER: f32 = 8.0;
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
/// Cost of one decision, J per kg.
const C_BRAIN: f64 = 1e5;

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
    /// Key stores (safe minimum > 0) that were liquid at birth, one bit per substance
    pub wet: u64,
    /// Visible traits: mass kg, light-capture area m², strength, colour (3 bands)
    pub vis: [f32; 6],
    /// Mass taken from other bodies in its life, kg: from light-eaters, from the others
    pub took: [f32; 2],
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
        self.dec(INTERVAL).round().clamp(MIN_INTERVAL, MAX_INTERVAL) as u64
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
    /// Key stores that are liquid now.
    pub fn key_liquids(&self, chem: &Chemistry) -> u64 {
        let (t, lay) = (self.temp(chem), Layout::of(chem));
        (0..chem.ns()).filter(|&k| self.genes[lay.min + k] > 0.0 && chem.phase(k, t) == Phase::Liquid).fold(0, |m, k| m | 1 << k)
    }
    /// Most of substance `k` the body holds, mol: its keep gene is the largest share of the
    /// body's mass it may make up (below 0.95). Liquid caught, taken or put stops there.
    pub fn level(&self, chem: &Chemistry, k: usize) -> f64 {
        let f = self.gene(Layout::of(chem).keep + k).clamp(0.0, 0.95);
        let mm = chem.subs[k].molar_mass;
        (f / (1.0 - f) * (self.mass(chem) - self.store[k] * mm) / mm).max(0.0)
    }
    /// A store below its safe minimum, or nothing left.
    pub fn lacking(&self, chem: &Chemistry) -> bool {
        let (m, lay) = (self.mass(chem), Layout::of(chem));
        m < 1e-3 || (0..chem.ns()).any(|k| self.store[k] * chem.subs[k].molar_mass < self.gene(lay.min + k) * m)
    }
}

pub fn light_eater(chem: &Chemistry, genes: &[f32]) -> bool {
    chem.reactions.iter().enumerate().any(|(r, rx)| rx.light && genes[NSCALAR + r] >= LIGHT_EATER)
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
    /// Updates due, by day modulo RING: (slot, generation)
    due: Vec<Vec<(u32, u32)>>,
    /// Brain weights of each lineage (None once it died out), and its living bodies
    pub brains: Vec<Option<Vec<f32>>>,
    pub members: Vec<u32>,
    /// Body updates since the start
    pub updates: u64,
}

impl Life {
    pub fn alive(&self) -> impl Iterator<Item = &Body> {
        self.bodies.iter().filter(|b| b.alive)
    }
    pub fn lineages(&self) -> usize {
        self.members.iter().filter(|&&m| m > 0).count()
    }
    pub fn new_lineage(&mut self, brain: Vec<f32>) -> u32 {
        self.brains.push(Some(brain));
        self.members.push(0);
        (self.brains.len() - 1) as u32
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
        self.due[((day + delay.clamp(1, MAX_INTERVAL as u64)) % RING) as usize].push((slot as u32, b.gen));
        self.bodies[slot] = b;
        slot
    }
    fn remove(&mut self, slot: usize) {
        let b = &mut self.bodies[slot];
        let (cell, lin) = (b.cell as usize, b.lineage as usize);
        (b.alive, b.store, b.genes) = (false, Vec::new(), Vec::new());
        b.gen += 1;
        self.at[cell].retain(|&s| s as usize != slot);
        self.members[lin] -= 1;
        if self.members[lin] == 0 {
            self.brains[lin] = None;
        }
        self.free.push(slot as u32);
    }
    pub fn relocate(&mut self, slot: usize, from: usize, to: usize) {
        self.at[from].retain(|&s| s as usize != slot);
        self.at[to].push(slot as u32);
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
        (w.life.at, w.life.due) = (vec![Vec::new(); n], vec![Vec::new(); RING as usize]);
    }
    if w.canopy.len() != n {
        (w.canopy, w.light) = (vec![0.0; n], vec![0.0; n]);
    }
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
    if w.day % w.cfg.days_per_month as u64 == 0 || w.life.env.len() != n || w.life.seen.len() != n * w.chem.ns() {
        w.life.env = (0..n).map(|i| env_of(w, i)).collect();
        let (ns, area) = (w.chem.ns(), w.grid.area());
        w.life.seen = (0..n * ns).map(|x| 0.2 * crate::brain::ln(1.0 + (w.ground[(x % ns) * n + x / ns] / area) as f32)).collect();
    }
}

/// Working memory for body updates.
#[derive(Default)]
pub struct Tmp {
    cat: Vec<f64>,
    pool: Vec<f64>,
    pool0: Vec<f64>,
    x: Vec<f64>,
    /// Light caught at the last update per kg per day, J
    pub lit: f64,
    pub input: Vec<f32>,
    pub thought: Thought,
}

/// Update the bodies due today.
pub fn day(w: &mut World) {
    let due = std::mem::take(&mut w.life.due[(w.day % RING) as usize]);
    if due.is_empty() {
        return;
    }
    // Cell by cell (a fixed order that keeps memory reads close together).
    let bodies = &w.life.bodies;
    let mut list: Vec<(u32, u32, u32)> = due.iter().filter(|&&(s, g)| bodies[s as usize].alive && bodies[s as usize].gen == g).map(|&(s, g)| (bodies[s as usize].cell, s, g)).collect();
    list.sort_unstable();
    let mut rc = Reactor::new(&w.chem);
    let mut tmp = Tmp::default();
    for (_, slot, gen) in list {
        let s = slot as usize;
        if !w.life.bodies[s].alive || w.life.bodies[s].gen != gen {
            continue;
        }
        let mut b = std::mem::take(&mut w.life.bodies[s]);
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
pub fn die(w: &mut World, slot: usize, b: Body, cause: u8) {
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
    let lay = Layout::of(&w.chem);
    let days = w.day.saturating_sub(b.last).max(1) as f64;
    let mass = b.mass(&w.chem);
    b.last = w.day;

    // Upkeep of catalysts, strength and capture area by the day, and of this decision; its
    // heat is spread over the time like the rest.
    let cats: f64 = (0..w.chem.reactions.len()).map(|r| b.gene(lay.cat + r).max(0.0)).sum();
    let upkeep = days * (mass * (C_CAT * cats + C_STRENGTH * b.gene(STRENGTH)) + C_LEAF * b.vis[1] as f64) + C_BRAIN * mass;
    let e0 = b.e;
    if !pay(w, b, upkeep, days) {
        return Some(SPENT);
    }
    let upkeep = b.e - e0;
    b.e = e0;

    let World { chem, grid, ground, e_ground, air, e_air, ledger, light, diag, .. } = w;
    let (ns, n, na, j) = (chem.ns(), grid.n(), grid.na(), grid.air_of(i));
    let dt = days * DAY;

    // Light caught since the last update, less what the body reflects; liquid over the body
    // absorbs its share first.
    let refl = (b.vis[3] + b.vis[4] + b.vis[5]) as f64 / 3.0;
    let got = mass * b.dec(LEAF) * (light[i] - b.mark) * (1.0 - refl);
    b.mark = light[i];
    let reach = (-depth / LIGHT_DEPTH).exp();
    ledger.sun_in += got;
    e_ground[i] += got * (1.0 - reach);
    let lit = got * reach;
    tmp.lit = lit / mass / days;

    // The air the body exchanges gas with in this time (mol), from its air node; liquid over
    // the body slows it.
    let tot: f64 = (0..ns).map(|k| air[k * na + j]).sum();
    let reach_mol = b.dec(GAS) * mass * days * (-depth / GAS_DEPTH).exp();
    tmp.x.clear();
    tmp.x.extend((0..ns).map(|k| air[k * na + j] / tot));
    tmp.pool.clear();
    tmp.pool.extend((0..ns).map(|k| (tmp.x[k] * reach_mol).min(0.5 * air[k * na + j])));
    tmp.pool0.clone_from(&tmp.pool);

    // Catalysed reactions, at the body's temperature held fixed; their heat is spread over the
    // time below.
    tmp.cat.clear();
    tmp.cat.extend((0..chem.reactions.len()).map(|r| if b.genes[lay.cat + r] > 0.0 { b.dec(lay.cat + r) } else { 1.0 }));
    let c0 = b.heat_cap(chem);
    let t0 = b.e / c0;
    let fixed = 1e6 * c0;
    let e_start = t0 * (fixed + c0);
    let mut e = e_start;
    let used = rc.react(chem, &mut b.store, Some(&mut tmp.pool), &mut e, fixed, Some(&tmp.cat), lit, dt);
    let mut heat = e - e_start + lit - used + upkeep;
    for k in 0..ns {
        let d = tmp.pool[k] - tmp.pool0[k];
        air[k * na + j] += d;
        heat -= d * held(chem, k);
    }

    // Liquids the body holds evaporate toward their vapour pressure at its temperature; what
    // falls as liquid on its capture area is caught, from the vapour it falls from. The net
    // goes in or out, up to the level the body keeps, with its latent heat.
    let fall = diag.rain.get(i).map_or(0.0, |&r| r as f64) * b.vis[1] as f64 * dt;
    tmp.pool.clear();
    tmp.pool.extend((0..ns).map(|k| if chem.phase(k, tg) == Phase::Liquid { air[k * na + j] } else { 0.0 }));
    let vap_tot: f64 = tmp.pool.iter().sum();
    for k in 0..ns {
        let (out, vap) = (if b.store[k] > 0.0 && chem.phase(k, t0) == Phase::Liquid { (chem.p_sat(k, t0) / P_REF - tmp.x[k]).max(0.0) * reach_mol } else { 0.0 }, tmp.pool[k]);
        let caught = if fall > 0.0 && vap > 0.0 { (fall * vap / vap_tot).min(0.5 * vap) } else { 0.0 };
        let net = caught - out;
        let (m, t) = if net > 0.0 { (net.min(b.level(chem, k) - b.store[k]).max(0.0), tg) } else { (net.max(-b.store[k]), t0) };
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
    let rate = -(-b.dec(LIQ) * days).exp_m1() * wet;
    for k in (0..ns).filter(|&k| liq(k) && in_ground > 0.0) {
        let x = ground[k * n + i] / in_ground;
        let d = ((b.level(chem, k) - b.store[k]) * x * rate * chem.subs[k].solubility).min(0.5 * ground[k * n + i]).max(0.0);
        let q = d * chem.subs[k].cp * tg;
        b.store[k] += d;
        ground[k * n + i] -= d;
        b.e += q;
        e_ground[i] -= q + d * fused(chem, k, tg);
        heat += d * fused(chem, k, tg);
    }

    // Heat to the surface the body stands on (which radiates it and warms the air): T relaxes
    // toward the ground's temperature plus the warming of the heat released, over the time.
    let c = b.heat_cap(chem);
    if c > 0.0 {
        let g = b.dec(HEAT) * mass;
        let teq = tg + heat / dt / g;
        let e1 = c * (teq + (b.e / c - teq) * (-g * dt / c).exp());
        e_ground[i] += b.e + heat - e1;
        b.e = e1;
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
    let World { chem, grid, air, life, .. } = w;
    let (ns, na, i, cat) = (chem.ns().min(64), grid.na(), b.cell as usize, Layout::of(chem).cat);
    let j = grid.air_of(i);
    let tot: f64 = (0..ns).map(|k| air[k * na + j]).sum();
    let depth = life.env.get(i).map_or(0.0, |e| e[1]);
    let reach_mol = b.dec(GAS) * b.mass(chem) * days * (-depth / GAS_DEPTH).exp() / tot;
    let share = reach_mol.min(0.5);
    let t = b.temp(chem);
    let gas = |k: usize| chem.phase(k, t) == Phase::Gas;
    let mut have = [0.0f64; 64];
    for (k, h) in have.iter_mut().enumerate().take(ns) {
        *h = if gas(k) { air[k * na + j] * share } else { b.store[k] };
    }
    let (mut ext, mut left) = ([0.0f64; 64], cost);
    for (r, rx) in chem.reactions.iter().enumerate().take(64) {
        if left <= 0.0 || b.genes[cat + r] <= 0.0 {
            continue;
        }
        let dh = chem.dh(r);
        if dh < 0.0 {
            let lim = rx.inputs.iter().map(|&(k, nu)| have[k as usize] / nu as f64).fold(f64::MAX, f64::min);
            ext[r] = lim.min(left / -dh);
            left += dh * ext[r];
            for &(k, nu) in &rx.inputs {
                have[k as usize] -= nu as f64 * ext[r];
            }
        }
    }
    if left > 1e-9 * cost {
        return false;
    }
    let mut latent = 0.0;
    for (r, rx) in chem.reactions.iter().enumerate().take(64).filter(|p| ext[p.0] > 0.0) {
        for &(k, nu) in &rx.inputs {
            let k = k as usize;
            let a = if gas(k) { &mut air[k * na + j] } else { &mut b.store[k] };
            let d = (nu as f64 * ext[r]).min(*a);
            *a -= d;
            if gas(k) {
                latent -= d * held(chem, k);
            }
        }
        for &(k, nu) in &rx.outputs {
            let (k, d) = (k as usize, nu as f64 * ext[r]);
            *if gas(k) { &mut air[k * na + j] } else { &mut b.store[k] } += d;
            if gas(k) {
                latent += d * held(chem, k);
            }
        }
    }
    b.e += cost - left - latent;
    true
}
