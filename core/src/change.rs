//! Law 2: Change. Substances react inside nodes, faster when hot.
//!
//! A reaction goes as far as `lim × (1 − exp(−rate × time))`, where `lim` is the amount of its
//! scarcest input per `nu` (the first of the least) and `rate` is per unit of that input:
//! catalyst · k · exp(−Ea/T) · the product, over the other inputs in a gas or a liquid, of their
//! share of the phase to their `nu` (the scarcest one to `nu − 1`: `lim` already counts it
//! once, so it counts `nu` times in all, as mass action says). So the reaction is first order in
//! its scarce input (a trace gas in plenty of the other goes at the same rate per unit whatever
//! its amount), not second order. A solid is a phase of its own (share 1).

use crate::World;
use crate::chem::{Chemistry, Reaction};

const YEAR_S: f64 = 3.156e7;
/// A reaction whose fraction per year stays below this is frozen and skipped.
const NEGLIGIBLE: f64 = 1e-12;
/// Largest temperature change in one sub-step, K.
const MAX_DT_K: f64 = 30.0;
const MAX_SUBSTEPS: usize = 48;
/// A node that is neither frozen nor hot reacts every `LAZY` ticks with `LAZY` times the time:
/// reactions at ordinary temperatures are slow, so one long step is as good as many short ones.
/// Nodes take turns in blocks of `BLOCK` (contiguous memory). A ground node waits `SLOW` ticks
/// if over that time its reactions would go no further than a share `CALM` of their inputs
/// (but for reactions on a `SPECK` of what it holds) and warm it by no more than `CALM_K` kelvin.
const LAZY: u64 = 40;
const SLOW: u64 = 8 * LAZY;
const CALM: f64 = 0.05;
const CALM_K: f64 = 0.5;
const SPECK: f64 = 1e-12;
/// A reaction goes far within a step when its rate times the step reaches this.
const FAR: f64 = 0.05;
/// A substance making up less than this share of a ground node's loose amounts is a trace:
/// reactions on traces alone never make a node react every step.
const TRACE: f64 = 1e-6;
const BLOCK: usize = 64;
/// Share of the light a light-driven reaction takes that it stores; the rest becomes heat.
const LIGHT_YIELD: f64 = 0.05;

/// Reusable reaction solver for one chemistry: per-reaction constants and scratch buffers.
pub struct Reactor {
    /// Temperature below which a reaction is frozen (catalyst 1), K.
    thr: Vec<f64>,
    /// Enthalpy change per reaction unit, J.
    dh: Vec<f64>,
    /// Per substance: heat capacity, melting and boiling point.
    cp: Vec<f64>,
    melt: Vec<f64>,
    boil: Vec<f64>,
    /// Phase of each substance at the current temperature: 0 solid, 1 liquid, 2 gas.
    ph: Vec<u8>,
    /// Amount of each substance that can react now (the node's plus, for gas, the surrounding gas).
    av: Vec<f64>,
    /// Effective rate of each reaction in this sub-step, 1/s (0 = idle).
    kap: Vec<f64>,
    /// Limiting input amount of each reaction at the start of the sub-step, in reaction units,
    /// and the share of it that reacts over the whole time left.
    lim: Vec<f64>,
    far: Vec<f64>,
    /// At the start of the last `react`: the heat capacity, J/K, and the rate of each reaction
    /// not driven by light, 1/s.
    cap0: f64,
    kap0: Vec<f64>,
}

/// `f` to a small whole power.
#[inline]
fn pow(f: f64, nu: u16) -> f64 {
    match nu {
        1 => f,
        2 => f * f,
        _ => f.powi(nu as i32),
    }
}

/// The input that runs out first: its amount per `nu` (reaction units) and its place in `inputs`
/// (the first of the least). `a` gives the amount of a substance.
#[inline]
pub(crate) fn scarce(inputs: &[(u16, u16)], a: impl Fn(usize) -> f64) -> (f64, usize) {
    let (mut lim, mut who) = (f64::MAX, 0);
    for (q, &(k, nu)) in inputs.iter().enumerate() {
        let l = per(a(k as usize), nu);
        if l < lim {
            (lim, who) = (l, q);
        }
    }
    (lim, who)
}

/// `a` per `nu` (exact).
#[inline]
fn per(a: f64, nu: u16) -> f64 {
    match nu {
        1 => a,
        2 => a * 0.5,
        _ => a / nu as f64,
    }
}

/// 1 - exp(-z), cheap when small.
#[inline]
pub(crate) fn frac(z: f64) -> f64 {
    if z < 1e-3 { z * (1.0 - 0.5 * z) } else { -(-z).exp_m1() }
}

impl Reactor {
    pub fn new(chem: &Chemistry) -> Self {
        let (nr, ns) = (chem.reactions.len(), chem.ns());
        let thr = chem
            .reactions
            .iter()
            .map(|r| if r.k <= 0.0 { f64::INFINITY } else { r.ea / (r.k * YEAR_S / NEGLIGIBLE).ln().max(1e-9) })
            .collect();
        Reactor {
            thr,
            dh: (0..nr).map(|r| chem.dh(r)).collect(),
            cp: chem.subs.iter().map(|s| s.cp).collect(),
            melt: chem.subs.iter().map(|s| s.melt).collect(),
            boil: chem.subs.iter().map(|s| s.boil).collect(),
            ph: vec![0; ns],
            av: vec![0.0; ns],
            kap: vec![0.0; nr],
            lim: vec![0.0; nr],
            far: vec![0.0; nr],
            cap0: 0.0,
            kap0: vec![0.0; nr],
        }
    }

    /// Would the last `react` have gone no further over `span` seconds than a share `far` of
    /// each reaction's inputs, nor than a `trace` of the amounts, nor heated by more than `dt_k`?
    /// (Rates and limits as at its start.)
    fn calm(&self, span: f64, far: f64, trace: f64, dt_k: f64) -> bool {
        let mut heat = 0.0;
        for (r, &k) in self.kap0.iter().enumerate() {
            let go = self.lim[r] * k * span;
            if k * span > far && go > trace {
                return false;
            }
            heat += self.dh[r].abs() * go;
        }
        heat <= dt_k * self.cap0
    }

    /// Fastest rate, 1/s, among the reactions not driven by light that the amounts `n` of one
    /// node can run at `t`.
    pub fn fastest(&mut self, chem: &Chemistry, n: &[f64], t: f64) -> f64 {
        let tot = self.phases(n, None, t);
        let mut best = 0.0f64;
        for (r, rx) in chem.reactions.iter().enumerate() {
            if rx.light || t < self.thr[r] {
                continue;
            }
            let (lim, who) = scarce(&rx.inputs, |k| self.av[k]);
            if lim > 0.0 {
                best = best.max(rx.k * (-rx.ea / t).exp() * self.share(rx, who, &tot));
            }
        }
        best
    }

    /// The product, over the inputs of `rx` in a gas or a liquid, of their share of the phase to
    /// their `nu`; the scarcest input (place `who`) to `nu - 1`.
    fn share(&self, rx: &Reaction, who: usize, tot: &[f64; 3]) -> f64 {
        let mut x = 1.0;
        for (q, &(k, nu)) in rx.inputs.iter().enumerate() {
            let (k, p) = (k as usize, self.ph[k as usize] as usize);
            if p > 0 {
                x *= pow(self.av[k] / tot[p], nu - (q == who) as u16);
            }
        }
        x
    }

    /// Phase and available amount of each substance at `t` (the node's, plus for a gas the gas
    /// around it), and the total of each phase: in a gas or a liquid the mole fraction is a
    /// substance's activity; a solid is a phase of its own (activity 1).
    fn phases(&mut self, n: &[f64], gas: Option<&[f64]>, t: f64) -> [f64; 3] {
        let mut tot = [0.0; 3];
        for k in 0..n.len() {
            let p = (t >= self.melt[k]) as usize + (t >= self.boil[k]) as usize;
            let a = match gas {
                Some(g) if p == 2 => n[k] + g[k],
                _ => n[k],
            };
            self.ph[k] = p as u8;
            self.av[k] = a;
            tot[p] += a;
        }
        tot
    }

    /// React the amounts `n` of one node for `dt` seconds. Returns the light energy used, J.
    /// - `gas`: extra amounts of the gas around the node (e.g. its share of the air). Substances that are
    ///   gas at the node's temperature are drawn from it first, and gas products are put into it.
    /// - `energy`: thermal energy of the node, J. Its temperature is `energy / (heat_cap + Σ n·cp)`,
    ///   where `heat_cap` is the fixed part (e.g. bonded ground).
    /// - `cat`: catalyst factor per reaction (`None` = 1 for all).
    /// - `light_j`: absorbed light energy available to light reactions, J.
    ///
    /// Elements are exact; thermal energy plus Σ n·hf gains exactly the light used (a share
    /// `LIGHT_YIELD` of it stored, the rest as heat).
    #[allow(clippy::too_many_arguments)]
    pub fn react(
        &mut self,
        chem: &Chemistry,
        n: &mut [f64],
        gas: Option<&mut [f64]>,
        energy: &mut f64,
        heat_cap: f64,
        cat: Option<&[f64]>,
        light_j: f64,
        dt: f64,
    ) -> f64 {
        let pool: &mut [f64] = gas.unwrap_or(&mut []);
        let has_pool = !pool.is_empty();
        let mut light = light_j;
        let mut left = dt;
        self.kap0.fill(0.0);
        for iter in 0..=MAX_SUBSTEPS {
            if left <= 0.0 {
                break;
            }
            let c = heat_cap + n.iter().zip(&self.cp).map(|(a, c)| a * c).sum::<f64>();
            let t = *energy / c.max(1.0);
            if iter == 0 {
                self.cap0 = c;
            }
            let tot = self.phases(n, has_pool.then_some(&*pool), t);
            // Rates of the reactions that are not frozen and have all their inputs.
            let (mut any, mut heat) = (false, 0.0);
            for (r, rx) in chem.reactions.iter().enumerate() {
                self.kap[r] = 0.0;
                let cr = cat.map_or(1.0, |c| c[r]);
                let uses_light = rx.light && self.dh[r] > 0.0;
                if cr <= 0.0 || (t < self.thr[r] && cr <= 1.0) || (uses_light && light <= 0.0) {
                    continue;
                }
                let (lim, who) = scarce(&rx.inputs, |k| self.av[k]);
                if lim > 0.0 {
                    let kap = cr * rx.k * (-rx.ea / t).exp() * self.share(rx, who, &tot);
                    self.kap[r] = kap;
                    self.lim[r] = lim;
                    any = true;
                    self.far[r] = frac(kap * left);
                    if !uses_light {
                        heat += self.dh[r].abs() * lim * self.far[r];
                        if iter == 0 {
                            self.kap0[r] = kap;
                        }
                    }
                }
            }
            if !any {
                break;
            }
            // Sub-step length: the heat of one sub-step must stay within MAX_DT_K.
            let mut sub = left;
            if iter < MAX_SUBSTEPS {
                while heat > MAX_DT_K * c && sub > left * 1e-4 {
                    sub *= 0.5;
                    heat = (0..chem.reactions.len())
                        .filter(|&r| self.kap[r] > 0.0 && !(chem.reactions[r].light && self.dh[r] > 0.0))
                        .map(|r| self.dh[r].abs() * self.lim[r] * frac(self.kap[r] * sub))
                        .sum();
                }
            }
            // Apply one after the other, each limited by what is left, so nothing goes negative.
            for (r, rx) in chem.reactions.iter().enumerate() {
                if self.kap[r] <= 0.0 {
                    continue;
                }
                let dh = self.dh[r];
                let uses_light = rx.light && dh > 0.0;
                let lim = scarce(&rx.inputs, |k| self.av[k]).0;
                let mut ext = lim * if sub == left { self.far[r] } else { frac(self.kap[r] * sub) };
                if uses_light {
                    ext = ext.min(light * LIGHT_YIELD / dh);
                } else if dh > 0.0 {
                    ext = ext.min(0.5 * *energy / dh);
                }
                if ext <= 0.0 {
                    continue;
                }
                for &(k, nu) in &rx.inputs {
                    let (k, d) = (k as usize, nu as f64 * ext);
                    let from_pool = if self.ph[k] == 2 && has_pool { pool[k].min(d) } else { 0.0 };
                    if from_pool > 0.0 {
                        pool[k] -= from_pool;
                    }
                    n[k] = (n[k] - (d - from_pool)).max(0.0);
                    self.av[k] = (self.av[k] - d).max(0.0);
                }
                for &(k, nu) in &rx.outputs {
                    let (k, d) = (k as usize, nu as f64 * ext);
                    if self.ph[k] == 2 && has_pool { pool[k] += d } else { n[k] += d }
                    self.av[k] += d;
                }
                if uses_light {
                    light -= dh * ext / LIGHT_YIELD;
                    *energy += dh * ext * (1.0 / LIGHT_YIELD - 1.0);
                } else {
                    *energy -= dh * ext
                }
            }
            left -= sub;
        }
        light_j - light
    }
}

/// A reaction that may go far within a step somewhere: its barrier, its inputs (one bit per
/// substance), those that are gas at every temperature (drawn from the air) and the others
/// with their boiling points, and its rate constant over the step relative to going far. It
/// needs at least `hot` (all activities 1), and `low` for the gases of any air node; per air
/// node, `need` for those gases at the most activity they can have there (their share of the
/// air's gas), once asked for (NaN before).
struct Cand {
    ea: f64,
    kdt: f64,
    hot: f64,
    low: f64,
    need_bits: u64,
    gas: Vec<(usize, u16)>,
    held: Vec<(usize, f64)>,
    need: Vec<std::cell::Cell<f64>>,
}

impl Cand {
    /// Temperature at which it goes far, for `z` times its rate constant over the step.
    fn warm(&self, z: f64) -> f64 {
        if z > 1.0 { self.ea / z.ln() } else { f64::INFINITY }
    }
}

/// Run Law 2 on ground nodes (loose amounts, gas from the air above) and on air nodes.
/// Node temperatures come from the last flow step. A ground node whose reactions may go far
/// within one step (by their rate at its temperature and the most activity their inputs can
/// have, a bound checked one reaction at a time over all nodes) reacts every step; at its turn,
/// every `LAZY` (or `SLOW`) steps by block, a node reacts for all the time since its last turn
/// it has not reacted for, unless all its reactions are frozen. Air nodes go far by their exact
/// rates, else react at their turn.
pub fn step(w: &mut World, dt: f64) {
    let mut s = std::mem::take(&mut w.scratch);
    let chem = &w.chem;
    let (ns, n, na, sc) = (chem.ns(), w.grid.n(), w.grid.na(), w.grid.air_scale);
    let tick = (w.day / w.cfg.phys_days as u64) as u32;
    if s.slow.len() != n {
        // Blocks take turns: block `b` first reacts when `(b + tick) % LAZY == 0`.
        s.slow = (0..n)
            .map(|i| tick + ((2 * LAZY - (i / BLOCK) as u64 % LAZY - tick as u64 % LAZY) % LAZY) as u32)
            .map(|due| [due.saturating_sub(LAZY as u32), due, 0])
            .collect();
    }
    let mut slow = std::mem::take(&mut s.slow);
    let fallback: (Vec<f64>, Vec<[f64; 4]>, Vec<u32>, Vec<f64>);
    let (mut tg, mut bed, mut air_of, mut ta) = s.view();
    if tg.len() != n || bed.len() != n || air_of.len() != n || ta.len() != na {
        let skin = |i: usize| {
            let (c, v) = (0..ns).fold((0.0, 0.0), |(c, v), k| (c + w.b(k, i) * chem.subs[k].cp, v + w.b(k, i) * chem.molar_volume(k, crate::chem::Phase::Solid)));
            if v > 0.0 { c / v * w.cfg.skin_m * w.grid.area() } else { 0.0 }
        };
        fallback = ((0..n).map(|i| w.t_ground(i)).collect(), (0..n).map(|i| [skin(i), 0.0, 0.0, 0.0]).collect(), (0..n).map(|i| w.grid.air_of(i) as u32).collect(), (0..na).map(|j| w.t_air(j)).collect());
        (tg, bed, air_of, ta) = (&fallback.0, &fallback.1, &fallback.2, &fallback.3);
    }
    let turn = ((LAZY - tick as u64 % LAZY) % LAZY) as usize;
    let share = 1.0 / (sc * sc) as f64;
    // Gas held by the air carries its fusion and vaporisation enthalpy (see `check::totals`).
    let held: Vec<f64> = chem.subs.iter().map(|b| b.lf + b.lv).collect();
    let mut rc = Reactor::new(chem);
    let bit = |k: usize| 1u64 << k.min(63);
    // Per reaction not driven by light: its inputs (one bit per substance), frozen below `thr`,
    // too slow to go far within a step below `hot` even with all its inputs at activity 1.
    let rxs: Vec<(usize, u64, f64, f64)> = (0..chem.reactions.len())
        .filter(|&r| !chem.reactions[r].light)
        .map(|r| {
            let rx = &chem.reactions[r];
            let hot = if rx.k > 0.0 { rx.ea / (rx.k * dt / FAR).ln().max(1e-9) } else { f64::INFINITY };
            (r, rx.inputs.iter().fold(0, |m, p| m | bit(p.0 as usize)), rc.thr[r], hot)
        })
        .collect();
    let cold = |t: f64, has: u64| !rxs.iter().any(|&(_, need, thr, _)| t >= thr && has & need == need);
    let mut in_air = vec![0u64; na];
    for k in 0..ns {
        for (m, &a) in in_air.iter_mut().zip(&w.air[k * na..(k + 1) * na]) {
            *m |= if a > 0.0 { bit(k) } else { 0 };
        }
    }
    let warmest = |v: &[f64]| v.iter().fold(f64::MIN, |a, &t| a.max(t));
    let (gmax, thi) = (warmest(tg), warmest(tg).max(warmest(ta)));
    let anywhere = in_air.iter().fold(0, |m, &x| m | x);
    let col = |k: usize| &w.ground[k * n..(k + 1) * n];
    let somewhere = |k: usize| anywhere & bit(k) != 0 || col(k).chunks(256).any(|c| c.iter().fold(false, |m, &a| m | (a > 0.0)));
    // Reactions that may go far somewhere; a node must be at least as warm as `hot` for one, so
    // substances boiling below the least `hot` are gas wherever one can.
    let fast: Vec<_> = rxs.iter().filter(|&&(r, _, _, hot)| hot <= thi && chem.reactions[r].inputs.iter().all(|p| somewhere(p.0 as usize))).collect();
    let tlo = fast.iter().fold(f64::INFINITY, |m, x| m.min(x.3));
    let mut pool_lo = vec![0.0; na];
    for k in (0..ns).filter(|&k| chem.subs[k].boil <= tlo) {
        for (p, &a) in pool_lo.iter_mut().zip(&w.air[k * na..(k + 1) * na]) {
            *p += a;
        }
    }
    // The most activity `a` mol of a gas can have among at least `d` mol of gas.
    let act = |a: f64, d: f64| if d > 0.0 { (a / d).min(1.0) } else { 1.0 };
    // The most activity the rate per unit of the scarcest input can have from gases with shares
    // `f`, whichever input that is: all shares to `nu`, or one power of one of them left out.
    let up = |gas: &[(usize, u16)], f: &dyn Fn(usize) -> f64| {
        (0..=gas.len()).map(|d| gas.iter().enumerate().map(|(q, &(k, nu))| pow(f(k), nu - (q == d) as u16)).product::<f64>()).fold(0.0, f64::max)
    };
    let most_act: Vec<f64> = (0..ns).map(|k| if chem.subs[k].boil <= tlo { (0..na).fold(0.0, |m, j| act(w.air[k * na + j], pool_lo[j]).max(m)) } else { 1.0 }).collect();
    let cand: Vec<Cand> = (fast.iter())
        .map(|&&(r, need_bits, _, hot)| {
            let rx = &chem.reactions[r];
            let (gas, held): (Vec<_>, Vec<_>) = rx.inputs.iter().map(|p| (p.0 as usize, p.1)).partition(|p| chem.subs[p.0].boil <= tlo);
            let held = held.iter().map(|g| (g.0, chem.subs[g.0].boil)).collect();
            let mut c = Cand { ea: rx.ea, kdt: rx.k * dt / FAR, hot, low: 0.0, need_bits, gas, held, need: vec![std::cell::Cell::new(f64::NAN); na] };
            c.low = c.warm(c.kdt * up(&c.gas, &|k| most_act[k])).max(hot);
            c
        })
        .collect();
    let need = |c: &Cand, j: usize| {
        if c.need[j].get().is_nan() {
            c.need[j].set(c.warm(c.kdt * up(&c.gas, &|k| act(w.air[k * na + j], pool_lo[j]))));
        }
        c.need[j].get()
    };
    // Ground nodes that may go far: warm enough; holding the inputs that are not gas there
    // beyond a trace (of at least what the node holds of them and of the substances most of the
    // ground is made of); and warm enough for the gases of the air above, thinned by those the
    // node holds or gives off itself. Reactions that need warmth few nodes have look at those
    // nodes only.
    let mut most: Vec<(f64, usize)> = (0..ns).map(|k| (col(k).iter().step_by(61).sum::<f64>(), k)).collect();
    most.sort_by(|a, b| b.0.total_cmp(&a.0));
    let most: Vec<usize> = most.iter().take(3).map(|m| m.1).collect();
    let fits = |c: &Cand, i: usize, t: f64| {
        let held = |all: f64| c.held.iter().all(|&(k, boil)| t >= boil || col(k)[i] > TRACE * all);
        if t < c.hot || !held(col(most[0])[i]) {
            return false;
        }
        let (j, own) = (air_of[i] as usize, c.gas.iter().any(|&(k, _)| col(k)[i] > 0.0));
        if !own && (t < c.low || (!c.gas.is_empty() && t < need(c, j))) {
            return false;
        }
        // Vapour of what the node holds thins the gas too.
        let vapour = most.iter().filter(|&&k| t >= chem.subs[k].boil).map(|&k| col(k)[i]).sum::<f64>();
        let d = pool_lo[j] + vapour / share;
        if (own || vapour > 0.0) && t < c.warm(c.kdt * up(&c.gas, &|k| act(col(k)[i] / share + w.air[k * na + j], d))) {
            return false;
        }
        held(most.iter().map(|&k| col(k)[i]).sum::<f64>() + c.held.iter().filter(|h| !most.contains(&h.0)).map(|&(k, _)| col(k)[i]).sum::<f64>())
    };
    let (mut maybe, mut ok, mut gas) = (vec![false; n], vec![false; n], vec![false; n]);
    let mut warm: Option<Vec<u32>> = None;
    let mut cs: Vec<&Cand> = cand.iter().filter(|c| c.hot <= gmax).collect();
    cs.sort_by(|a, b| a.hot.total_cmp(&b.hot));
    for c in cs {
        if warm.is_none() && tg.iter().filter(|&&t| t >= c.hot).count() * 8 <= n {
            warm = Some((0..n as u32).filter(|&i| tg[i as usize] >= c.hot).collect());
        }
        if let Some(v) = &mut warm {
            v.retain(|&i| tg[i as usize] >= c.hot);
            for &i in v.iter() {
                maybe[i as usize] = maybe[i as usize] || fits(c, i as usize, tg[i as usize]);
            }
            continue;
        }
        // Many warm nodes: first a cheap test over all nodes (holding the inputs not gas there
        // beyond a trace of the commonest substance, and warm enough for the air's gases or
        // holding a gas input), then the full one.
        for (o, &t) in ok.iter_mut().zip(tg) {
            *o = t >= c.hot;
        }
        for &(k, boil) in &c.held {
            for ((o, &t), (&a, &b)) in ok.iter_mut().zip(tg).zip(col(k).iter().zip(col(most[0]))) {
                *o &= t >= boil || a > TRACE * b;
            }
        }
        if !c.gas.is_empty() {
            for (g, &t) in gas.iter_mut().zip(tg) {
                *g = t >= c.low;
            }
            for &(k, _) in &c.gas {
                for (g, &a) in gas.iter_mut().zip(col(k)) {
                    *g |= a > 0.0;
                }
            }
            for (o, &g) in ok.iter_mut().zip(&gas) {
                *o &= g;
            }
        }
        for i in 0..n {
            if ok[i] && !maybe[i] {
                maybe[i] = fits(c, i, tg[i]);
            }
        }
    }
    let (mut amt, mut pool, mut pool0) = (vec![0.0; ns], vec![0.0; ns], vec![0.0; ns]);

    // Air nodes that may go far.
    let air_maybe: Vec<bool> = (0..na).map(|j| cand.iter().any(|c| in_air[j] & c.need_bits == c.need_bits && ta[j] >= c.low && ta[j] >= need(c, j))).collect();

    // Ground.
    for i in 0..n {
        let due = slow[i][1] <= tick;
        if !due && !maybe[i] {
            continue;
        }
        let (t, j) = (tg[i], air_of[i] as usize);
        for k in 0..ns {
            amt[k] = w.ground[k * n + i];
        }
        let has = || (0..ns).filter(|&k| amt[k] > 0.0).fold(in_air[j], |m, k| m | bit(k));
        // A node that may go far reacts for this step; at its turn, for all the time since its
        // last turn it has not reacted for.
        let [last, _, fasts] = slow[i];
        // (At least this step if it may go far: a first turn has no earlier turn to count from.)
        let ticks = if due && !cold(t, has()) { (tick - last).saturating_sub(fasts).max(maybe[i] as u32) } else { maybe[i] as u32 };
        let mut next = LAZY;
        if ticks > 0 {
            for k in 0..ns {
                pool0[k] = w.air[k * na + j] * share;
            }
            pool.copy_from_slice(&pool0);
            let all = amt.iter().sum::<f64>();
            // The reactor sees sensible heat: molten amounts hold their fusion enthalpy apart.
            let fused: f64 = (0..ns).filter(|&k| chem.subs[k].melt <= t).map(|k| amt[k] * chem.subs[k].lf).sum();
            let mut e = w.e_ground[i] - fused;
            rc.react(chem, &mut amt, Some(&mut pool), &mut e, bed[i][0], None, 0.0, ticks as f64 * dt);
            w.e_ground[i] = e + fused;
            // Gas sent to the air takes its fusion and vaporisation enthalpy from the ground (gas
            // taken back from the air returns it); gas the ground cannot pay for stays behind.
            let (mut pay, mut back) = (0.0, 0.0);
            for k in 0..ns {
                let d = pool[k] - pool0[k];
                (pay, back) = (pay + d.max(0.0) * held[k], back - d.min(0.0) * held[k]);
            }
            w.e_ground[i] += back;
            let cap = w.heat_cap_ground(i);
            let paid = if pay > 0.0 { crate::share(crate::draw(&mut w.e_ground[i], &mut w.ledger, cap, pay), pay) } else { 1.0 };
            for k in 0..ns {
                let d = pool[k] - pool0[k];
                let sent = if d > 0.0 { d * paid } else { d };
                w.ground[k * n + i] = amt[k] + (d - sent);
                w.air[k * na + j] += sent;
            }
            if rc.calm(SLOW as f64 * dt, CALM, SPECK * all, CALM_K) {
                next = SLOW;
            }
        }
        slow[i] = if due { [tick, tick + next as u32, 0] } else { [last, slow[i][1], fasts + 1] };
    }
    // Air.
    for j in 0..na {
        let t = ta[j];
        for k in 0..ns {
            amt[k] = w.air[k * na + j];
        }
        let time = if air_maybe[j] && rc.fastest(chem, &amt, t) * dt >= FAR {
            dt
        } else if j % LAZY as usize == turn && !cold(t, in_air[j]) {
            dt * LAZY as f64
        } else {
            continue;
        };
        rc.react(chem, &mut amt, None, &mut w.e_air[j], 0.0, None, 0.0, time);
        for k in 0..ns {
            w.e_air[j] -= (amt[k] - w.air[k * na + j]) * held[k];
            w.air[k * na + j] = amt[k];
        }
    }
    s.slow = slow;
    w.scratch = s;
}
