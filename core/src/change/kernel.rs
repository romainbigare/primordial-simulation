//! The reactor of Law 2: the rate law and its helpers, and `Reactor`, which reacts the amounts of one node, body or part.

use super::TRACE;
use crate::chem::{Chemistry, Reaction};
use crate::math::one_minus_exp;
use crate::units::P_REF;

/// The time over which a frozen reaction goes no further than `NEGLIGIBLE`, s: a calendar year of 365.25 days (not the
/// world's year, `World::days_per_year`; to be fixed with the bugs).
const FROZEN_SPAN: f64 = 3.156e7;
/// A reaction whose fraction over `FROZEN_SPAN` stays below this is frozen and skipped.
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
/// Share of the light a light-driven reaction takes that it stores; the rest becomes heat.
pub(crate) const LIGHT_YIELD: f64 = 0.05;

/// Reusable reaction solver for one chemistry: per-reaction constants and scratch buffers.
pub struct Reactor {
    /// Temperature below which a reaction is frozen (catalyst 1), K (`frozen_below`).
    thr: Vec<f64>,
    /// Enthalpy change per reaction unit, J.
    dh: Vec<f64>,
    /// Per reaction: does it store absorbed light (light-driven and taking energy)?
    lit: Vec<bool>,
    /// Per substance: heat capacity, melting and boiling point.
    cp: Vec<f64>,
    melt: Vec<f64>,
    boil: Vec<f64>,
    /// Phase of each substance at the current temperature: 0 solid, 1 liquid, 2 gas.
    ph: Vec<u8>,
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
    /// At the start of the last `react`: the heat capacity, J/K, and the rate of each reaction
    /// not driven by light, 1/s.
    cap0: f64,
    kap0: Vec<f64>,
    /// Time, s, the liquid standing over the node takes to renew its gas through its film (Flow's
    /// law, `air::dissolve`); 0: the node touches the gas around it. Its gas is then the gas
    /// dissolved in the liquid, held where the film and its reactions balance: Henry's share `eq`
    /// of the gas around it (`Chemistry::henry`) times (1 + made film / eq) / (1 + used film / eq),
    /// with what its reactions use and make, mol/s, at that share. What they take and give still
    /// comes from and goes to the gas around it: through the film.
    pub film: f64,
    /// Per substance, used and made, mol/s (scratch for `film`).
    flux: Vec<[f64; 2]>,
}

/// `f` to a small whole power.
#[inline]
pub(super) fn pow(f: f64, nu: u16) -> f64 {
    match nu {
        1 => f,
        2 => f * f,
        _ => f.powi(nu as i32),
    }
}

/// The temperature at which a reaction of barrier `ea` K reaches `z` times its rate constant times a time: ea / ln z,
/// K; never where `z` is at most 1.
#[inline]
pub(super) fn warm_at(ea: f64, z: f64) -> f64 {
    if z > 1.0 { ea / z.ln() } else { f64::INFINITY }
}

/// Temperature below which reaction `rx` (catalyst 1) is frozen: it would go no further than `NEGLIGIBLE` over
/// `FROZEN_SPAN`, K.
pub(super) fn frozen_below(rx: &Reaction) -> f64 {
    warm_at(rx.ea, rx.k * FROZEN_SPAN / NEGLIGIBLE)
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
/// whatever its size, and smooth, so a body warming past `td` slows without a jump.
#[inline]
pub(crate) fn denature(f: f64, t: f64, td: f64, wk: f64) -> f64 {
    if f <= 1.0 { f } else { f.powf(1.0 / (1.0 + ((t - td) / wk).exp())) }
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
    pub fn new(chem: &Chemistry) -> Self {
        let (nr, ns) = (chem.reactions.len(), chem.ns());
        let dh: Vec<f64> = (0..nr).map(|r| chem.dh(r)).collect();
        Reactor {
            thr: chem.reactions.iter().map(frozen_below).collect(),
            lit: chem.reactions.iter().zip(&dh).map(|(rx, &d)| rx.light && d > 0.0).collect(),
            dh,
            cp: chem.subs.iter().map(|s| s.cp).collect(),
            melt: chem.subs.iter().map(|s| s.melt).collect(),
            boil: chem.subs.iter().map(|s| s.boil).collect(),
            ph: vec![0; ns],
            av: vec![0.0; ns],
            kap: vec![0.0; nr],
            lim: vec![0.0; nr],
            far: vec![0.0; nr],
            pw: vec![0.0; nr],
            cap0: 0.0,
            kap0: vec![0.0; nr],
            film: 0.0,
            flux: vec![[0.0; 2]; ns],
        }
    }

    /// Would the last `react` have gone no further over `span` seconds than a share `far` of
    /// each reaction's inputs, nor than a `trace` of the amounts, nor heated by more than `dt_k`?
    /// (The rates of its first sub-step, the limits of its last.)
    pub(super) fn calm(&self, span: f64, far: f64, trace: f64, dt_k: f64) -> bool {
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
    pub(crate) fn fastest(&mut self, chem: &Chemistry, n: &[f64], t: f64) -> f64 {
        let tot = self.phases(chem, n, None, t);
        let mut best = 0.0f64;
        for (r, rx) in chem.reactions.iter().enumerate() {
            if rx.light || t < self.thr[r] {
                continue;
            }
            let (lim, who) = scarce(&rx.inputs, |k| self.av[k]);
            let k = rx.inputs[who].0 as usize;
            if lim > 0.0 && (self.ph[k] == 0 || self.av[k] >= TRACE * tot[self.ph[k] as usize]) {
                best = best.max(rx.k * (-rx.ea / t).exp() * self.pace(rx, lim, &tot));
            }
        }
        best
    }

    /// The rates of the last `phases` under a liquid whose film renews its gas in `film` s: each
    /// reaction goes with the liquid's share of Henry's of each gas it takes, to its order.
    fn through_film(&mut self, chem: &Chemistry, tot: &[f64; 3], t: f64) {
        self.flux.fill([0.0; 2]);
        for (r, rx) in chem.reactions.iter().enumerate() {
            for (side, &(k, nu)) in rx.inputs.iter().map(|p| (0, p)).chain(rx.outputs.iter().map(|p| (1, p))) {
                self.flux[k as usize][side] += nu as f64 * (self.kap[r] * self.lim[r]) * (self.ph[k as usize] == 2) as u8 as f64;
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
        pace(rx, lim, |k| self.av[k], |k| if self.ph[k] > 0 { tot[self.ph[k] as usize] } else { self.av[k] })
    }

    /// Phase and available amount of each substance at `t` (the node's, plus for a gas the gas
    /// around it), and the total of each phase: in a gas or a liquid the mole fraction is a
    /// substance's activity; a solid is a phase of its own (activity 1).
    pub(super) fn phases(&mut self, chem: &Chemistry, n: &[f64], gas: Option<&[f64]>, t: f64) -> [f64; 3] {
        let mut tot = [0.0; 3];
        for k in 0..n.len() {
            let p = (t >= self.melt[k] || t >= chem.melt_in(k, |j| n[j])) as usize + (t >= self.boil[k]) as usize;
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
    /// - `cat`: catalyst factor per reaction, the temperature above which they lose their shape and
    ///   the width of that, K (`denature`; `None` = 1 for all).
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
        cat: Option<(&[f64], f64, f64)>,
        light_j: f64,
        dt: f64,
    ) -> f64 {
        let pool: &mut [f64] = gas.unwrap_or(&mut []);
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
                self.kap[r] = cr * rx.k * (-rx.ea / t).exp() * self.pace(rx, lim, tot);
                self.lim[r] = lim;
                any = true;
                // (In a liquid or a gas that power is its order less 1, as far as it is not most of its phase.)
                let k = rx.inputs[q].0 as usize;
                self.pw[r] = if self.ph[k] > 0 { (rx.order(q) as f64 - 1.0) * (1.0 - self.av[k] / tot[self.ph[k] as usize]) } else { 0.0 };
            }
        }
        any
    }

    /// The length of sub-step `iter` of the `left` seconds still to go, at heat capacity `c`: all of them, halved while the
    /// heat of the reactions not driven by light would change the temperature by more than `MAX_DT_K` (but for the last
    /// sub-step). Sets the share `far` each reaction goes in it, and on the first the rates `kap0`.
    fn sub_step(&mut self, left: f64, c: f64, iter: usize) -> f64 {
        let mut heat = 0.0;
        for r in (0..self.kap.len()).filter(|&r| self.kap[r] > 0.0) {
            self.far[r] = gone(self.kap[r] * left, self.pw[r]);
            if !self.lit[r] {
                heat += self.dh[r].abs() * self.lim[r] * self.far[r];
                if iter == 0 {
                    self.kap0[r] = self.kap[r];
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
    /// pool); its heat to or from `energy`, and the light a light-driven one stores from `light`.
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
                ext = ext.min(*light * LIGHT_YIELD / dh);
            } else if dh > 0.0 {
                ext = ext.min(MOST_HEAT * *energy / dh);
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
            if self.lit[r] {
                *light -= dh * ext / LIGHT_YIELD;
                *energy += dh * ext * (1.0 / LIGHT_YIELD - 1.0);
            } else {
                *energy -= dh * ext
            }
        }
    }
}
