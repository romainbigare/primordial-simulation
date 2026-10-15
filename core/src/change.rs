//! Law 2: Change. Substances react inside nodes, faster when hot.
//!
//! Mass action: a reaction runs at catalyst · k · exp(−Ea/T) · `e` · the product, over its inputs,
//! of their share of their phase to their order (`Reaction::order`; a solid is a phase of its own,
//! share 1), in reaction units a second. `e` is the contact of its phases: what `lim` would be
//! were each input all of its phase (the least, over the inputs, of its phase's amount per `nu`).
//! So a reaction within one phase goes with that phase, and one between a solid and the air with
//! the solid, as a surface would. In a step it goes as far as `lim × (1 − exp(−rate / lim ×
//! time))`, `lim` the amount of its scarcest input per `nu`: never past it, exact for a first-order
//! decay. The rate has no jump when another input becomes the scarcest, and is first order in a
//! trace input (a trace gas in plenty of the other goes at the same rate per unit whatever its
//! amount). A body's catalysts work only below the temperature where they lose their shape
//! (`denature`). A ground node with liquid standing over it reacts with the gas dissolved in that
//! liquid, which the liquid's film renews (`Reactor::film`).

use crate::World;
use crate::chem::{Chemistry, Phase, Reaction};

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
/// A substance making up less than this share of a ground node's loose amounts (or of its phase,
/// for a gas) is a trace: reactions on traces alone never make a node react every step, and
/// lumping a reaction that runs out of a trace over `LAZY` steps is exact for a first-order decay.
const TRACE: f64 = 1e-6;
/// Gas a ground node holds itself (left by its reactions, or dissolved in its liquid) lets it
/// react every step only when it is more than this share of the air's gas over it: less changes
/// the warmth its reactions need by about a tenth of a kelvin, and it reacts at its turn anyway.
const OWN: f64 = 0.01;
const BLOCK: usize = 64;
/// Share of the light a light-driven reaction takes that it stores; the rest becomes heat.
pub(crate) const LIGHT_YIELD: f64 = 0.05;

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
fn pow(f: f64, nu: u16) -> f64 {
    match nu {
        1 => f,
        2 => f * f,
        _ => f.powi(nu as i32),
    }
}

/// Pairs of reactions that undo each other, as (first, back, m, substance): the first makes one
/// substance, the back takes `m` times as much of it and gives `m` times what the first took.
pub fn reversible(chem: &Chemistry) -> Vec<(usize, usize, u16, usize)> {
    let mut out = Vec::new();
    for (f, rf) in chem.reactions.iter().enumerate().filter(|(_, r)| !r.light && r.outputs.len() == 1) {
        let (u, made) = rf.outputs[0];
        for (b, rb) in chem.reactions.iter().enumerate().filter(|(b, r)| *b != f && !r.light && r.inputs.len() == 1 && r.inputs[0].0 == u) {
            let m = rb.inputs[0].1 / made;
            let scaled = |v: &[(u16, u16)]| {
                let mut v: Vec<_> = v.iter().map(|&(k, c)| (k, c * m)).collect();
                v.sort_unstable();
                v
            };
            let mut back = rb.outputs.clone();
            back.sort_unstable();
            if m >= 1 && m * made == rb.inputs[0].1 && back == scaled(&rf.inputs) {
                out.push((f, b, m, u as usize));
            }
        }
    }
    out
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
pub fn denature(f: f64, t: f64, td: f64, wk: f64) -> f64 {
    if f <= 1.0 { f } else { f.powf(1.0 / (1.0 + ((t - td) / wk).exp())) }
}

/// 1 - exp(-z), cheap when small.
#[inline]
pub(crate) fn frac(z: f64) -> f64 {
    if z < 1e-3 { z * (1.0 - 0.5 * z) } else { -(-z).exp_m1() }
}

/// The share of its scarcest input a reaction uses where it would use `z` at its starting rate, its rate per unit going
/// with the power `p` of what is left: 1 - (1 + p z)^(-1/p), the integral of dx/dt = -k x^(1 + p) (15 Oct 2026; it was
/// `frac`, the rate held over the step, so a salt far past its balance fell out to its last mol, exp(-250), not to 4.5%).
#[inline]
fn gone(z: f64, p: f64) -> f64 {
    if p <= 0.0 || z < 1e-3 { frac(z) } else { -(-(p * z).ln_1p() / p).exp_m1() }
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
            pw: vec![0.0; nr],
            cap0: 0.0,
            kap0: vec![0.0; nr],
            film: 0.0,
            flux: vec![[0.0; 2]; ns],
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
            let k = rx.inputs[who].0 as usize;
            if lim > 0.0 && (self.ph[k] == 0 || self.av[k] >= TRACE * tot[self.ph[k] as usize]) {
                best = best.max(rx.k * (-rx.ea / t).exp() * self.pace(rx, lim, &tot));
            }
        }
        best
    }

    /// The amounts at which pairs of reactions that undo each other (`reversible`) balance in one
    /// node at `t`, from the chemistry's own rate constants and barriers, with the gas around the
    /// node (`gas`) held as it is. Of each pair the substance the first makes, if the node holds
    /// none of it, is set to the amount at which the two take it and give it at the same rate,
    /// with the other inputs of the first (those that are not gas) lessened by what it holds.
    /// Returns the substances set and their amounts. A node without an input of the first (no
    /// liquid, say) holds none: its rate is nothing at every amount.
    pub fn balance(&mut self, chem: &Chemistry, n: &mut [f64], gas: &[f64], t: f64) -> Vec<(usize, f64)> {
        let mut out = Vec::new();
        for (f, b, m, u) in reversible(chem) {
            if n[u] > 0.0 || gas[u] > 0.0 {
                continue;
            }
            let (made, took) = (chem.reactions[f].outputs[0].1 as f64, m as f64 * chem.reactions[f].outputs[0].1 as f64);
            let used: Vec<(usize, f64, f64)> = (chem.reactions[f].inputs.iter()).filter(|p| chem.phase(p.0 as usize, t) != Phase::Gas).map(|&(k, c)| (k as usize, n[k as usize], c as f64 / made)).collect();
            // The rate of change of `u`, mol/s, with `x` of it in the node: positive below the balance.
            let net = |rc: &mut Self, n: &mut [f64], x: f64| {
                n[u] = x;
                for &(k, a, c) in &used {
                    n[k] = (a - c * x).max(0.0);
                }
                let tot = rc.phases(n, Some(gas), t);
                let eps = |rc: &Self, r: usize| {
                    let rx = &chem.reactions[r];
                    let lim = scarce(&rx.inputs, |k| rc.av[k]).0;
                    if lim > 0.0 { lim * rx.k * (-rx.ea / t).exp() * rc.pace(rx, lim, &tot) } else { 0.0 }
                };
                made * eps(rc, f) - took * eps(rc, b)
            };
            if net(self, n, 0.0) <= 0.0 {
                continue;
            }
            let (mut lo, mut hi) = (0.0, 1.0);
            while net(self, n, hi) > 0.0 && hi < 1e30 {
                (lo, hi) = (hi, hi * 2.0);
            }
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if net(self, n, mid) > 0.0 { lo = mid } else { hi = mid }
            }
            let x = 0.5 * (lo + hi);
            net(self, n, x);
            out.push((u, x));
        }
        out
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
            let eq = if used > 0.0 { tot[1] * chem.henry(k, t, self.av[k] / tot[2] * crate::chem::P_REF) } else { 0.0 };
            self.flux[k][0] = if eq > 0.0 { (1.0 + made * self.film / eq) / (1.0 + used * self.film / eq) } else { 1.0 };
        }
        for (r, rx) in chem.reactions.iter().enumerate() {
            for (q, p) in rx.inputs.iter().enumerate() {
                self.kap[r] *= self.flux[p.0 as usize][0].powi(rx.order(q) as i32);
            }
        }
    }

    /// `pace` of `rx` with the amounts and phases of the last `phases`.
    fn pace(&self, rx: &Reaction, lim: f64, tot: &[f64; 3]) -> f64 {
        pace(rx, lim, |k| self.av[k], |k| if self.ph[k] > 0 { tot[self.ph[k] as usize] } else { self.av[k] })
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
                let cr = cat.map_or(1.0, |(c, td, wk)| denature(c[r], t, td, wk));
                let uses_light = rx.light && self.dh[r] > 0.0;
                if cr <= 0.0 || (t < self.thr[r] && cr <= 1.0) || (uses_light && light <= 0.0) {
                    continue;
                }
                let (lim, q) = scarce(&rx.inputs, |k| self.av[k]);
                if lim > 0.0 {
                    self.kap[r] = cr * rx.k * (-rx.ea / t).exp() * self.pace(rx, lim, &tot);
                    self.lim[r] = lim;
                    any = true;
                    // (In a liquid or a gas that power is its order less 1, as far as it is not most of its phase.)
                    let k = rx.inputs[q].0 as usize;
                    self.pw[r] = if self.ph[k] > 0 { (rx.order(q) as f64 - 1.0) * (1.0 - self.av[k] / tot[self.ph[k] as usize]) } else { 0.0 };
                }
            }
            if !any {
                break;
            }
            if self.film > 0.0 {
                self.through_film(chem, &tot, t);
            }
            for (r, rx) in chem.reactions.iter().enumerate().filter(|(r, _)| self.kap[*r] > 0.0) {
                self.far[r] = gone(self.kap[r] * left, self.pw[r]);
                if !(rx.light && self.dh[r] > 0.0) {
                    heat += self.dh[r].abs() * self.lim[r] * self.far[r];
                    if iter == 0 {
                        self.kap0[r] = self.kap[r];
                    }
                }
            }
            // Sub-step length: the heat of one sub-step must stay within MAX_DT_K.
            let mut sub = left;
            if iter < MAX_SUBSTEPS {
                while heat > MAX_DT_K * c && sub > left * 1e-4 {
                    sub *= 0.5;
                    heat = (0..chem.reactions.len())
                        .filter(|&r| self.kap[r] > 0.0 && !(chem.reactions[r].light && self.dh[r] > 0.0))
                        .map(|r| self.dh[r].abs() * self.lim[r] * gone(self.kap[r] * sub, self.pw[r]))
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
                let mut ext = lim * if sub == left { self.far[r] } else { gone(self.kap[r] * sub, self.pw[r]) };
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
/// (`held`, with boiling points, counts and orders), and its rate constant over the step relative
/// to going far. It needs at least `hot` (all activities 1), and `low` for the gases of any air
/// node; per air node, `need` for those gases at the most activity they can have there, once
/// asked for (NaN before). `shared`: the inputs that are not gas and count more than once, which
/// in a liquid go by their share of it (a trace of one cannot go far however fast the rate is);
/// `gnu` the largest count of a gas input.
struct Cand {
    ea: f64,
    kdt: f64,
    hot: f64,
    low: f64,
    need_bits: u64,
    gas: Vec<(usize, u16)>,
    held: Vec<(usize, f64, u16, u16)>,
    gnu: f64,
    shared: Vec<(usize, u16)>,
    need: Vec<std::cell::Cell<f64>>,
}

impl Cand {
    /// Temperature at which it goes far, for `z` times its rate constant over the step.
    fn warm(&self, z: f64) -> f64 {
        if z > 1.0 { self.ea / z.ln() } else { f64::INFINITY }
    }
}

/// Run Law 2 on ground nodes (loose amounts, gas from the air above) and on air nodes, at the
/// temperatures of the last flow step. A ground node whose reactions may go far within one step
/// (by their rate at its temperature and the most activity their inputs can have, checked one
/// reaction at a time over all nodes) reacts every step; at its turn, every `LAZY` (or `SLOW`)
/// steps by block, it reacts for all the time since it last did, unless all its reactions are
/// frozen. Air nodes go far by their exact rates, else react at their turn.
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
            let (c, v) = (0..ns).fold((0.0, 0.0), |(c, v), k| (c + w.b(k, i) * chem.subs[k].cp, v + w.b(k, i) * chem.molar_volume(k, Phase::Solid)));
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
    // Per reaction not driven by light: its inputs (a bit per substance), frozen below `thr`,
    // too slow to go far in a step below `hot` even with all inputs at activity 1.
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
    // `f`, whichever input that is: all shares to `nu`, or one power of one left out (not a trace,
    // see `TRACE`). A bound: `pace` is never more, as `e / lim` is at most the scarcest input's
    // phase over its amount.
    let up = |gas: &[(usize, u16)], f: &dyn Fn(usize) -> f64| {
        (0..=gas.len()).filter(|&d| d == gas.len() || f(gas[d].0) >= TRACE).map(|d| gas.iter().enumerate().map(|(q, &(k, nu))| pow(f(k), nu.saturating_sub((q == d) as u16))).product::<f64>()).fold(0.0, f64::max)
    };
    let most_act: Vec<f64> = (0..ns).map(|k| if chem.subs[k].boil <= tlo { (0..na).fold(0.0, |m, j| act(w.air[k * na + j], pool_lo[j]).max(m)) } else { 1.0 }).collect();
    let cand: Vec<Cand> = (fast.iter())
        .map(|&&(r, need_bits, _, hot)| {
            let rx = &chem.reactions[r];
            let (gas, held): (Vec<_>, Vec<_>) = rx.inputs.iter().enumerate().map(|(q, p)| (p.0 as usize, rx.order(q))).partition(|p| chem.subs[p.0].boil <= tlo);
            let shared = held.iter().copied().filter(|g| g.1 > 1).collect();
            let count = |k: usize| rx.inputs.iter().find(|p| p.0 as usize == k).map_or(1, |p| p.1);
            let gnu = gas.iter().map(|g| count(g.0)).max().unwrap_or(1) as f64;
            let held = held.iter().map(|g| (g.0, chem.subs[g.0].boil, count(g.0), g.1)).collect();
            let mut c = Cand { ea: rx.ea, kdt: rx.k * dt / FAR, hot, low: 0.0, need_bits, gas, held, gnu, shared, need: vec![std::cell::Cell::new(f64::NAN); na] };
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
    // Ground nodes that may go far: warm enough; holding the inputs that are not gas beyond a
    // trace (of the substances most of the ground is made of); and warm enough for the gases of
    // the air above, thinned by those the node holds or gives off itself. Reactions that need
    // warmth few nodes have look at those nodes only.
    let mut most: Vec<(f64, usize)> = (0..ns).map(|k| (col(k).iter().step_by(61).sum::<f64>(), k)).collect();
    most.sort_by(|a, b| b.0.total_cmp(&a.0));
    let most: Vec<usize> = most.iter().take(3).map(|m| m.1).collect();
    // The substances that are liquid at some ground node.
    let gmin = tg.iter().fold(f64::MAX, |a, &t| a.min(t));
    let liquids: Vec<usize> = (0..ns).filter(|&k| chem.subs[k].melt <= gmax && chem.subs[k].boil > gmin).collect();
    let own_gas = |k: usize, i: usize| col(k)[i] > OWN * share * w.air[k * na + air_of[i] as usize];
    let fits = |c: &Cand, i: usize, t: f64| {
        let held = |all: f64| c.held.iter().all(|&(k, boil, _, _)| t >= boil || col(k)[i] > TRACE * all);
        if t < c.hot || !held(col(most[0])[i]) {
            return false;
        }
        // An input that counts more than once, in a liquid, goes by its share of the liquid to the
        // power `nu - 1`: a node holding a trace of it in its liquid does not react every step.
        if !c.shared.is_empty() {
            let liquid = |k: usize| chem.subs[k].melt <= t && t < chem.subs[k].boil;
            let all: f64 = liquids.iter().filter(|&&k| liquid(k)).map(|&k| col(k)[i]).sum();
            let x: f64 = (c.shared.iter().filter(|p| liquid(p.0))).map(|&(k, nu)| pow(col(k)[i] / all, nu - 1)).product();
            if t < c.warm(c.kdt * x) {
                return false;
            }
        }
        let (j, own) = (air_of[i] as usize, c.gas.iter().any(|&(k, _)| own_gas(k, i)));
        if !own && (t < c.low || (!c.gas.is_empty() && t < need(c, j))) {
            return false;
        }
        // Vapour of what the node holds thins the gas too.
        let vapour = most.iter().filter(|&&k| t >= chem.subs[k].boil).map(|&k| col(k)[i]).sum::<f64>();
        let d = pool_lo[j] + vapour / share;
        if (own || vapour > 0.0) && t < c.warm(c.kdt * up(&c.gas, &|k| act(col(k)[i] / share + w.air[k * na + j], d))) {
            return false;
        }
        // The contact of the phases (`pace`): with a gas the scarce input, the reaction goes no faster
        // than the least the node holds of its other inputs (a liquid by all the liquid) over the
        // gas over it, times the gas's count; with another input scarce, all shares count.
        let airy: Vec<(usize, u16)> = c.gas.iter().copied().chain(c.held.iter().filter(|h| t >= h.1).map(|h| (h.0, h.3))).collect();
        if !airy.is_empty() && c.held.iter().any(|h| t < h.1) {
            let liquid = |k: usize| chem.subs[k].melt <= t && t < chem.subs[k].boil;
            let wet = || liquids.iter().filter(|&&k| liquid(k)).map(|&k| col(k)[i]).sum::<f64>();
            let hold = (c.held.iter().filter(|h| t < h.1)).map(|&(k, _, nu, _)| (if liquid(k) { wet() } else { col(k)[i] }) / nu as f64).fold(f64::MAX, f64::min);
            let gnu = c.held.iter().filter(|h| t >= h.1).fold(c.gnu, |m, h| m.max(h.2 as f64));
            let x = |k: usize| act(col(k)[i] / share + w.air[k * na + j], d);
            let full: f64 = airy.iter().map(|&(k, o)| pow(x(k), o)).product();
            if t < c.warm(c.kdt * full.max((gnu * hold / (d * share)).min(1.0) * up(&airy, &x))) {
                return false;
            }
        }
        held(most.iter().map(|&k| col(k)[i]).sum::<f64>() + c.held.iter().filter(|h| !most.contains(&h.0)).map(|h| col(h.0)[i]).sum::<f64>())
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
        // Many warm nodes: a cheap test over all (holding the non-gas inputs beyond a trace of the
        // commonest substance, and warm enough for the air's gases or holding a gas input), then the full one.
        for (o, &t) in ok.iter_mut().zip(tg) {
            *o = t >= c.hot;
        }
        for &(k, boil, _, _) in &c.held {
            for ((o, &t), (&a, &b)) in ok.iter_mut().zip(tg).zip(col(k).iter().zip(col(most[0]))) {
                *o &= t >= boil || a > TRACE * b;
            }
        }
        if !c.gas.is_empty() {
            for (g, &t) in gas.iter_mut().zip(tg) {
                *g = t >= c.low;
            }
            for &(k, _) in &c.gas {
                for (i, g) in gas.iter_mut().enumerate() {
                    *g |= own_gas(k, i);
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
        // A node that may go far reacts for this step (at least; a first turn has no earlier turn to
        // count from); at its turn, for all the time since its last turn it has not reacted for.
        let [last, _, fasts] = slow[i];
        let ticks = if due && !cold(t, has()) { (tick - last).saturating_sub(fasts).max(maybe[i] as u32) } else { maybe[i] as u32 };
        let mut next = LAZY;
        if ticks > 0 {
            for k in 0..ns {
                pool0[k] = w.air[k * na + j] * share;
            }
            pool.copy_from_slice(&pool0);
            // Liquid stands over it (more than the pores of its loose ground hold): it reacts with
            // the gas dissolved in that liquid, renewed through the liquid's film over its whole
            // depth (`Reactor::film`; Flow's law, `air::dissolve`: depth / `dissolve`; the air's
            // side of the film is faster by 100 times or more for every gas and left out).
            let vol = |p: Phase| (0..ns).filter(|&k| chem.phase(k, t) == p).map(|k| amt[k] * chem.molar_volume(k, p)).sum::<f64>();
            let wet = vol(Phase::Liquid);
            rc.film = if wet > w.cfg.phys.porosity * vol(Phase::Solid) { wet / (w.grid.area() * w.cfg.dissolve) } else { 0.0 };
            let all = amt.iter().sum::<f64>();
            // The reactor sees sensible heat: molten amounts hold their fusion enthalpy apart.
            let fused: f64 = (0..ns).filter(|&k| chem.subs[k].melt <= t).map(|k| amt[k] * chem.subs[k].lf).sum();
            let mut e = w.e_ground[i] - fused;
            rc.react(chem, &mut amt, Some(&mut pool), &mut e, bed[i][0], None, 0.0, ticks as f64 * dt);
            // Gas sent to the air takes its fusion and vaporisation enthalpy from the ground (gas
            // taken back returns it); gas the ground cannot pay for stays behind. Gas taken from the
            // air brings its heat at the air's temperature; gas sent leaves with its share of the
            // node's heat by heat capacity.
            let (mut pay, mut back, mut c_out, mut q_in) = (0.0, 0.0, 0.0, 0.0);
            for k in 0..ns {
                let (d, cp) = (pool[k] - pool0[k], chem.subs[k].cp);
                (pay, back) = (pay + d.max(0.0) * held[k], back - d.min(0.0) * held[k]);
                (c_out, q_in) = (c_out + d.max(0.0) * cp, q_in - d.min(0.0) * cp * ta[j]);
            }
            let c_r = bed[i][0] + (0..ns).map(|k| amt[k] * chem.subs[k].cp).sum::<f64>();
            let q_out = e.max(0.0) * c_out / (c_r + c_out).max(1e-9);
            w.e_ground[i] = e + fused + back + q_in - q_out;
            w.e_air[j] += q_out - q_in;
            let cap = w.heat_cap_ground(i);
            let paid = if pay > 0.0 { crate::share(crate::draw(&mut w.e_ground[i], &mut w.ledger, cap, pay), pay) } else { 1.0 };
            // The heat of the gas that stays behind stays with it.
            (w.e_ground[i], w.e_air[j]) = (w.e_ground[i] + q_out * (1.0 - paid), w.e_air[j] - q_out * (1.0 - paid));
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
    // Air: each node on its own, so from the amounts before the loop (in parallel with the
    // `threads` feature) into a row [reacted, heat, amounts], then applied in order.
    let mut rows = vec![0.0; na * (ns + 2)];
    let (air_now, e_now) = (&w.air, &w.e_air);
    crate::par::rows(&mut rows, ns + 2, || None, |st: &mut Option<(Reactor, Vec<f64>)>, j, row| {
        let t = ta[j];
        let lazy = j % LAZY as usize == turn && !cold(t, in_air[j]);
        if !air_maybe[j] && !lazy {
            return;
        }
        let (rc, amt) = st.get_or_insert_with(|| (Reactor::new(chem), vec![0.0; ns]));
        for k in 0..ns {
            amt[k] = air_now[k * na + j];
        }
        let time = if air_maybe[j] && rc.fastest(chem, amt, t) * dt >= FAR {
            dt
        } else if lazy {
            dt * LAZY as f64
        } else {
            return;
        };
        let mut e = e_now[j];
        rc.react(chem, amt, None, &mut e, 0.0, None, 0.0, time);
        for k in 0..ns {
            e -= (amt[k] - air_now[k * na + j]) * held[k];
        }
        row[0] = 1.0;
        row[1] = e;
        row[2..].copy_from_slice(amt);
    });
    for (j, row) in rows.chunks(ns + 2).enumerate().filter(|(_, r)| r[0] != 0.0) {
        w.e_air[j] = row[1];
        for k in 0..ns {
            w.air[k * na + j] = row[2 + k];
        }
    }
    s.slow = slow;
    w.scratch = s;
}
