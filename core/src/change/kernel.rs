//! The reactor of Law 2: the rate law and its helpers, and `Reactor`, which reacts the amounts of one node, body or part.

use crate::chem::{Chemistry, Phase, Reaction};
use crate::math::{add_times, one_minus_exp};
use crate::units::P_REF;
use crate::Config;

/// A reaction whose fraction over the world's year stays below this is frozen and skipped.
const NEGLIGIBLE: f64 = 1e-12;
/// Largest temperature change in one sub-step, K.
const MAX_DT_K: f64 = 30.0;
/// Sub-steps of one `react` that are shortened to keep within `MAX_DT_K`; one more takes all the time left.
const MAX_SUBSTEPS: usize = 48;
/// Shortest sub-step, as a share of the time left.
const MIN_SUB: f64 = 1e-4;
/// A reaction that takes heat takes at most this share of the node's heat in one sub-step.
const MOST_HEAT: f64 = 0.5;
/// Below this, 1 - exp(-z) is taken as its series to second order (exact to 2e-7 of its value).
const SERIES_BELOW: f64 = 1e-3;
/// A gas or liquid input making up less than this share of its phase is a trace: a reaction whose scarcest input is a
/// trace does not shorten a node's sleep (`Reactor::rest`), as one long span of a first-order decay is exact.
const TRACE: f64 = 1e-6;

/// A substance's change whose terms add up to more than this many times it is summed exactly (`Reactor::made`): below,
/// the rounding of a plain sum of one term for each reaction is within 1e-12 of it.
const CANCEL: f64 = 1e3;

/// The phases by the number of the points a substance is past (melting, boiling).
const PHASES: [Phase; 3] = [Phase::Solid, Phase::Liquid, Phase::Gas];

/// Reusable reaction solver for one chemistry: per-reaction constants and scratch buffers.
pub struct Reactor {
    /// Temperature below which a reaction is frozen (catalyst 1), K (`frozen_below`).
    thr: Vec<f64>,
    /// Enthalpy change per reaction unit, J.
    dh: Vec<f64>,
    /// Per reaction: does it store absorbed light (light-driven and taking energy)?
    lit: Vec<bool>,
    /// Share of the light a light-driven reaction takes that it stores (`Physics::light_yield`).
    light_yield: f64,
    /// Per substance: heat capacity, melting and boiling point.
    cp: Vec<f64>,
    melt: Vec<f64>,
    boil: Vec<f64>,
    /// Phase of each substance at the current temperature (`Phase as usize` indexes the totals of the phases).
    ph: Vec<Phase>,
    /// Amount of each substance that can react now (the node's plus, for gas, the surrounding gas).
    pub(super) av: Vec<f64>,
    /// Effective rate of each reaction in this sub-step, 1/s (0 = idle).
    kap: Vec<f64>,
    /// Limiting input amount of each reaction at the start of the sub-step, in reaction units,
    /// and the share of it that reacts over the sub-step.
    lim: Vec<f64>,
    far: Vec<f64>,
    /// The power of what is left of its scarcest input that its rate per unit goes with (`gone`).
    pw: Vec<f64>,
    /// Per reaction: is its scarcest input a trace of its phase (`TRACE`)?
    minor: Vec<bool>,
    /// At the start of the last `react`: the heat capacity, J/K, and the rate of each reaction
    /// not driven by light nor on a trace (`minor`), 1/s, with its limiting input in reaction units.
    cap0: f64,
    kap0: Vec<f64>,
    lim0: Vec<f64>,
    /// Time, s, the liquid standing over the node takes to renew its gas through its film (Flow's
    /// law, `air::dissolve`); 0: the node touches the gas around it. Its gas is then the gas
    /// dissolved in the liquid, held where the film and its reactions balance: Henry's share `eq`
    /// of the gas around it (`Chemistry::henry`) times (1 + made film / eq) / (1 + used film / eq),
    /// with what its reactions use and make, mol/s, at that share. What they take and give still
    /// comes from and goes to the gas around it: through the film.
    pub film: f64,
    /// Per substance, used and made, mol/s (scratch for `film`).
    flux: Vec<[f64; 2]>,
    /// Over the last `react`: per reaction, how far it went, in reaction units; per substance, what they made less what
    /// they used of the node's own amounts, mol, and whether they took or gave any of it in the gas around it.
    done: Vec<f64>,
    pub(super) own: Vec<f64>,
    pub(super) pooled: Vec<bool>,
    /// Per substance, the sum of the terms of its change and of their sizes, mol, and room for an exact sum (`made`).
    sums: Vec<[f64; 2]>,
    parts: Vec<f64>,
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

/// Temperature below which reaction `rx` (catalyst 1) is frozen: it would go no further than `NEGLIGIBLE` over a
/// year of `year` s, K: Ea / ln z, z its rate constant times the year over `NEGLIGIBLE` (frozen at every temperature
/// where z is at most 1).
fn frozen_below(rx: &Reaction, year: f64) -> f64 {
    let z = rx.k * year / NEGLIGIBLE;
    if z > 1.0 { rx.ea / z.ln() } else { f64::INFINITY }
}

/// The substances reaction `rx` changes, each with what a reaction unit makes of it (minus what it uses).
fn terms(rx: &Reaction) -> impl Iterator<Item = (usize, f64)> + '_ {
    rx.inputs.iter().map(|&(k, nu)| (k as usize, -(nu as f64))).chain(rx.outputs.iter().map(|&(k, nu)| (k as usize, nu as f64)))
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

/// The rate of a reaction per unit of `lim` (its scarcest input per `nu`, above 0) before
/// catalyst · k · exp(−Ea/T): the product of its inputs' shares to their order, times `e / lim`.
/// `a(k)` is the amount of an input and `whole(k)` that of its phase (its own for a solid).
#[inline]
pub(crate) fn pace(rx: &Reaction, lim: f64, a: impl Fn(usize) -> f64, whole: impl Fn(usize) -> f64) -> f64 {
    let (mut x, mut e) = (1.0, f64::MAX);
    for (q, &(k, nu)) in rx.inputs.iter().enumerate() {
        let (a, n) = (a(k as usize), whole(k as usize));
        (x, e) = (x * pow(a / n, rx.order(q)), e.min(per(n, nu)));
    }
    x * e / lim
}

/// Catalyst factor `f` at `t` of a catalyst that loses its shape above `td` K, over a width of
/// `wk` K. A catalyst lowers the reaction's barrier (a speed-up `f` is a lowering of ln f × T);
/// losing its shape loses the lowering, so the share it keeps, 1 / (1 + exp((t − td) / wk)), is
/// the share of its decades kept: f to that power. Whole below `td`, near 1 a few `wk` above it
/// whatever its size, and smooth, so a body warming past `td` slows without a jump (with no
/// width: whole up to `td`, gone above it).
#[inline]
pub(crate) fn denature(f: f64, t: f64, td: f64, wk: f64) -> f64 {
    if f <= 1.0 {
        return f;
    }
    let z = if wk > 0.0 { (t - td) / wk } else if t > td { f64::INFINITY } else { f64::NEG_INFINITY };
    f.powf(1.0 / (1.0 + z.exp()))
}

/// 1 - exp(-z), cheap when small.
#[inline]
pub(crate) fn frac(z: f64) -> f64 {
    if z < SERIES_BELOW { z * (1.0 - 0.5 * z) } else { one_minus_exp(z) }
}

/// Share of the scarcest input used when the rate per unit goes with x^p: 1 - (1 + p z)^(-1/p), exact for
/// dx/dt = -k x^(1 + p); `frac` where p is 0.
#[inline]
fn gone(z: f64, p: f64) -> f64 {
    if p <= 0.0 || z < SERIES_BELOW { frac(z) } else { one_minus_exp((p * z).ln_1p() / p) }
}

impl Reactor {
    /// The reactor of `chem` in a world of settings `cfg` (a reaction too slow to matter over its year is frozen).
    pub fn new(chem: &Chemistry, cfg: &Config) -> Self {
        let (nr, ns, year) = (chem.reactions.len(), chem.ns(), cfg.calendar().year as f64);
        let dh: Vec<f64> = (0..nr).map(|r| chem.dh(r)).collect();
        Reactor {
            thr: chem.reactions.iter().map(|rx| frozen_below(rx, year)).collect(),
            lit: chem.reactions.iter().zip(&dh).map(|(rx, &d)| rx.light && d > 0.0).collect(),
            light_yield: cfg.phys.light_yield,
            dh,
            cp: chem.subs.iter().map(|s| s.cp).collect(),
            melt: chem.subs.iter().map(|s| s.melt).collect(),
            boil: chem.subs.iter().map(|s| s.boil).collect(),
            ph: vec![Phase::Solid; ns],
            av: vec![0.0; ns],
            kap: vec![0.0; nr],
            lim: vec![0.0; nr],
            far: vec![0.0; nr],
            pw: vec![0.0; nr],
            minor: vec![false; nr],
            cap0: 0.0,
            kap0: vec![0.0; nr],
            lim0: vec![0.0; nr],
            film: 0.0,
            flux: vec![[0.0; 2]; ns],
            done: vec![0.0; nr],
            own: vec![0.0; ns],
            pooled: vec![false; ns],
            sums: vec![[0.0; 2]; ns],
            parts: Vec::new(),
        }
    }

    /// The longest the node of the last `react` may wait before it reacts again, s: until a reaction, at its rate at the
    /// start of that `react`, would go further than a share `far` of its scarcest input (but for one that would use no
    /// more than `trace` mol), or the heat of them all would change its temperature by more than `dt_k`. Infinite where
    /// nothing reacts.
    pub(super) fn rest(&self, far: f64, trace: f64, dt_k: f64) -> f64 {
        let (mut most, mut heat) = (f64::INFINITY, 0.0);
        for (r, &k) in self.kap0.iter().enumerate().filter(|e| *e.1 > 0.0) {
            most = most.min((far / k).max(trace / (self.lim0[r] * k)));
            heat += self.dh[r].abs() * self.lim0[r] * k;
        }
        most.min(dt_k * self.cap0 / heat)
    }

    /// What the reactions of the last `react` made less what they used of each substance, mol, into `made`, from how far
    /// each went. Where the terms of one cancel (a pair of reactions that undo each other, fast, beside a slow one) they
    /// are summed exactly and rounded once (`math::add_times`), so the changes keep every element to rounding.
    pub(super) fn made(&mut self, chem: &Chemistry, made: &mut [f64]) {
        let ran = |r: &(usize, &Reaction)| self.done[r.0] > 0.0;
        self.sums.fill([0.0; 2]);
        for (r, rx) in chem.reactions.iter().enumerate().filter(ran) {
            for (k, nu) in terms(rx) {
                let (x, s) = (nu * self.done[r], &mut self.sums[k]);
                *s = [s[0] + x, s[1] + x.abs()];
            }
        }
        for (k, (d, &[sum, all])) in made.iter_mut().zip(&self.sums).enumerate() {
            *d = sum;
            if all > CANCEL * sum.abs() {
                self.parts.clear();
                for (r, rx) in chem.reactions.iter().enumerate().filter(ran) {
                    terms(rx).filter(|t| t.0 == k).for_each(|(_, nu)| add_times(&mut self.parts, nu, self.done[r]));
                }
                *d = self.parts.iter().sum();
            }
        }
    }

    /// The rates of the last `phases` under a liquid whose film renews its gas in `film` s: each
    /// reaction goes with the liquid's share of Henry's of each gas it takes, to its order.
    fn through_film(&mut self, chem: &Chemistry, tot: &[f64; 3], t: f64) {
        self.flux.fill([0.0; 2]);
        for (r, rx) in chem.reactions.iter().enumerate() {
            for (side, &(k, nu)) in rx.inputs.iter().map(|p| (0, p)).chain(rx.outputs.iter().map(|p| (1, p))) {
                self.flux[k as usize][side] += nu as f64 * (self.kap[r] * self.lim[r]) * (self.ph[k as usize] == Phase::Gas) as u8 as f64;
            }
        }
        for k in 0..self.flux.len() {
            let [used, made] = self.flux[k];
            let eq = if used > 0.0 { tot[1] * chem.henry(k, t, self.av[k] / tot[2] * P_REF) } else { 0.0 };
            self.flux[k][0] = if eq > 0.0 { (1.0 + made * self.film / eq) / (1.0 + used * self.film / eq) } else { 1.0 };
        }
        for (r, rx) in chem.reactions.iter().enumerate() {
            for (q, p) in rx.inputs.iter().enumerate() {
                self.kap[r] *= self.flux[p.0 as usize][0].powi(rx.order(q) as i32);
            }
        }
    }

    /// `pace` of `rx` with the amounts and phases of the last `phases`.
    pub(super) fn pace(&self, rx: &Reaction, lim: f64, tot: &[f64; 3]) -> f64 {
        pace(rx, lim, |k| self.av[k], |k| if self.ph[k] != Phase::Solid { tot[self.ph[k] as usize] } else { self.av[k] })
    }

    /// Phase and available amount of each substance at `t` (the node's, plus for a gas the gas
    /// around it), and the total of each phase: in a gas or a liquid the mole fraction is a
    /// substance's activity; a solid is a phase of its own (activity 1).
    pub(super) fn phases(&mut self, chem: &Chemistry, n: &[f64], gas: Option<&[f64]>, t: f64) -> [f64; 3] {
        let mut tot = [0.0; 3];
        for k in 0..n.len() {
            let p = (t >= self.melt[k] || n[k] > 0.0 && t >= chem.melt_in(k, |j| n[j])) as usize + (t >= self.boil[k]) as usize;
            let a = match gas {
                Some(g) if p == 2 => n[k] + g[k],
                _ => n[k],
            };
            self.ph[k] = PHASES[p];
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
    /// - `cat`: catalyst factor per reaction, the temperature above which they lose their shape and
    ///   the width of that, K (`denature`; `None` = 1 for all).
    /// - `light_j`: absorbed light energy available to light reactions, J.
    ///
    /// Elements are exact; thermal energy plus Σ n·hf gains exactly the light used (a share
    /// `Physics::light_yield` of it stored, the rest as heat).
    #[allow(clippy::too_many_arguments)]
    pub fn react(
        &mut self,
        chem: &Chemistry,
        n: &mut [f64],
        gas: Option<&mut [f64]>,
        energy: &mut f64,
        heat_cap: f64,
        cat: Option<(&[f64], f64, f64)>,
        light_j: f64,
        dt: f64,
    ) -> f64 {
        let pool: &mut [f64] = gas.unwrap_or(&mut []);
        let mut light = light_j;
        let mut left = dt;
        self.kap0.fill(0.0);
        self.done.fill(0.0);
        self.own.fill(0.0);
        self.pooled.fill(false);
        for iter in 0..=MAX_SUBSTEPS {
            if left <= 0.0 {
                break;
            }
            let c = heat_cap + n.iter().zip(&self.cp).map(|(a, c)| a * c).sum::<f64>();
            let t = *energy / c.max(1.0);
            if iter == 0 {
                self.cap0 = c;
            }
            let tot = self.phases(chem, n, (!pool.is_empty()).then_some(&*pool), t);
            if !self.rates(chem, &tot, t, cat, light) {
                break;
            }
            if self.film > 0.0 {
                self.through_film(chem, &tot, t);
            }
            let sub = self.sub_step(left, c, iter);
            self.apply(chem, n, pool, energy, &mut light);
            left -= sub;
        }
        light_j - light
    }

    /// The rate `kap`, limit `lim` and power `pw` of each reaction at `t` that is not frozen and has all its inputs (with
    /// light, for one that stores it). Returns whether any can run.
    fn rates(&mut self, chem: &Chemistry, tot: &[f64; 3], t: f64, cat: Option<(&[f64], f64, f64)>, light: f64) -> bool {
        let mut any = false;
        for (r, rx) in chem.reactions.iter().enumerate() {
            self.kap[r] = 0.0;
            let cr = cat.map_or(1.0, |(c, td, wk)| denature(c[r], t, td, wk));
            if cr <= 0.0 || (t < self.thr[r] && cr <= 1.0) || (self.lit[r] && light <= 0.0) {
                continue;
            }
            let (lim, q) = scarce(&rx.inputs, |k| self.av[k]);
            if lim > 0.0 {
                self.kap[r] = cr * rx.arrhenius(t) * self.pace(rx, lim, tot);
                self.lim[r] = lim;
                any = true;
                // (In a liquid or a gas that power is its order less 1, as far as it is not most of its phase.)
                let k = rx.inputs[q].0 as usize;
                let share = if self.ph[k] != Phase::Solid { self.av[k] / tot[self.ph[k] as usize] } else { 1.0 };
                (self.pw[r], self.minor[r]) = ((rx.order(q) as f64 - 1.0) * (1.0 - share), share < TRACE);
            }
        }
        any
    }

    /// The length of sub-step `iter` of the `left` seconds still to go, at heat capacity `c`: all of them, halved while the
    /// heat of the reactions not driven by light would change the temperature by more than `MAX_DT_K` (but for the last
    /// sub-step). Sets the share `far` each reaction goes in it, and on the first the rates `kap0` and limits `lim0`.
    fn sub_step(&mut self, left: f64, c: f64, iter: usize) -> f64 {
        let mut heat = 0.0;
        for r in (0..self.kap.len()).filter(|&r| self.kap[r] > 0.0) {
            self.far[r] = gone(self.kap[r] * left, self.pw[r]);
            if !self.lit[r] {
                heat += self.dh[r].abs() * self.lim[r] * self.far[r];
                if iter == 0 && !self.minor[r] {
                    (self.kap0[r], self.lim0[r]) = (self.kap[r], self.lim[r]);
                }
            }
        }
        let mut sub = left;
        if iter < MAX_SUBSTEPS {
            while heat > MAX_DT_K * c && sub > left * MIN_SUB {
                sub *= 0.5;
                heat = (0..self.kap.len())
                    .filter(|&r| self.kap[r] > 0.0 && !self.lit[r])
                    .map(|r| self.dh[r].abs() * self.lim[r] * gone(self.kap[r] * sub, self.pw[r]))
                    .sum();
            }
        }
        if sub != left {
            for r in (0..self.kap.len()).filter(|&r| self.kap[r] > 0.0) {
                self.far[r] = gone(self.kap[r] * sub, self.pw[r]);
            }
        }
        sub
    }

    /// Run each reaction its share `far` of its scarcest input, one after the other, each limited by what is left, so
    /// nothing goes negative: inputs from the node (a gas from the `pool` around it first), outputs to it (a gas to the
    /// pool), noted in `done`, `own` and `pooled`; its heat to or from `energy`, and the light a light-driven one stores
    /// from `light`.
    fn apply(&mut self, chem: &Chemistry, n: &mut [f64], pool: &mut [f64], energy: &mut f64, light: &mut f64) {
        let has_pool = !pool.is_empty();
        for (r, rx) in chem.reactions.iter().enumerate() {
            if self.kap[r] <= 0.0 {
                continue;
            }
            let dh = self.dh[r];
            let lim = scarce(&rx.inputs, |k| self.av[k]).0;
            let mut ext = lim * self.far[r];
            if self.lit[r] {
                ext = ext.min(*light * self.light_yield / dh);
            } else if dh > 0.0 {
                ext = ext.min(MOST_HEAT * *energy / dh);
            }
            if ext <= 0.0 {
                continue;
            }
            for &(k, nu) in &rx.inputs {
                let (k, d) = (k as usize, nu as f64 * ext);
                let from_pool = if self.ph[k] == Phase::Gas && has_pool { pool[k].min(d) } else { 0.0 };
                if from_pool > 0.0 {
                    pool[k] -= from_pool;
                    self.pooled[k] = true;
                }
                n[k] = (n[k] - (d - from_pool)).max(0.0);
                self.own[k] -= d - from_pool;
                self.av[k] = (self.av[k] - d).max(0.0);
            }
            for &(k, nu) in &rx.outputs {
                let (k, d) = (k as usize, nu as f64 * ext);
                if self.ph[k] == Phase::Gas && has_pool {
                    (pool[k], self.pooled[k]) = (pool[k] + d, true);
                } else {
                    (n[k], self.own[k]) = (n[k] + d, self.own[k] + d);
                }
                self.av[k] += d;
            }
            self.done[r] += ext;
            if self.lit[r] {
                *light -= dh * ext / self.light_yield;
                *energy += dh * ext * (1.0 / self.light_yield - 1.0);
            } else {
                *energy -= dh * ext
            }
        }
    }
}
