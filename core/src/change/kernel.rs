//! The reactor of Law 2: the rate law and its helpers, and `Reactor`, which reacts the amounts of one node, body or part.

use crate::{chem::{Chemistry, Phase, Reaction}, math::{add_times, one_minus_exp}, units::P_REF, Config};

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
/// A gas or liquid input under this share of its phase is a trace: a reaction on a trace does not shorten a node's
/// sleep (`Reactor::rest`), as one long span of a first-order decay is exact.
const TRACE: f64 = 1e-6;

/// A substance's change whose terms add up to more than this many times it is summed exactly (`Reactor::made`).
const CANCEL: f64 = 1e3;

/// The phases by the number of the points a substance is past (melting, boiling).
const PHASES: [Phase; 3] = [Phase::Solid, Phase::Liquid, Phase::Gas];

/// Reusable reaction solver for one chemistry: per-reaction constants and scratch buffers.
#[derive(Default)]
pub struct Reactor {
    /// Temperature below which a reaction is frozen (catalyst 1), K (`frozen_below`).
    thr: Vec<f64>,
    /// Enthalpy change per reaction unit, J.
    dh: Vec<f64>,
    /// Per reaction: does it store absorbed light (light-driven and taking energy)? And its inputs, a bit per substance.
    lit: Vec<bool>,
    needs: Vec<u64>,
    /// Share of the light a light-driven reaction takes that it stores (`Physics::light_yield`).
    light_yield: f64,
    /// Per substance: heat capacity, melting and boiling point.
    cp: Vec<f64>,
    melt: Vec<f64>,
    boil: Vec<f64>,
    /// Phase of each substance at the current temperature (`Phase as usize` indexes the totals of the phases).
    ph: Vec<Phase>,
    /// Amount of each substance that can react now (for a gas, with the gas around), and a bit for each above 0.
    pub(super) av: Vec<f64>,
    held: u64,
    /// Effective rate of each reaction in this sub-step, 1/s (0 = idle), and the reactions with all their inputs.
    kap: Vec<f64>,
    on: Vec<usize>,
    /// Limiting input of each reaction at the sub-step's start, in reaction units, and the share of it that reacts.
    lim: Vec<f64>,
    far: Vec<f64>,
    /// The power of what is left of its scarcest input that its rate per unit goes with (`gone`).
    pw: Vec<f64>,
    /// Per reaction: is its scarcest input a trace of its phase (`TRACE`)?
    minor: Vec<bool>,
    /// At the start of the last `react`: the heat capacity, J/K, and the rate and limit of each reaction not driven by
    /// light nor on a trace, and those reactions.
    cap0: f64,
    kap0: Vec<f64>,
    lim0: Vec<f64>,
    on0: Vec<usize>,
    /// Time, s, the liquid standing over the node takes to renew its gas through its film; 0: the node touches the gas
    /// around it. Its gas is then held where film and reactions balance (laws.md 2.3).
    pub film: f64,
    /// How moist the node is, 0 to 1 (`FlowView::moist`): the decomposers' catalysts of the open world work so far
    /// (`Reaction::open`); 0 until its caller sets it.
    pub moist: f64,
    /// Per substance, used and made, mol/s (scratch for `film`).
    flux: Vec<[f64; 2]>,
    /// Over the last `react`: how far each reaction went and those that went; per substance, what they took from the
    /// gas around the node (below 0) and gave to it, mol, summed from what each asked, never as the pool's change: a
    /// small draw on a large pool rounds away in the pool but not here.
    done: Vec<f64>,
    ran: Vec<usize>,
    pub(crate) dpool: Vec<f64>,
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

/// Temperature below which reaction `rx` (catalyst 1) goes no further than `NEGLIGIBLE` over a year of `year` s, K.
fn frozen_below(rx: &Reaction, year: f64) -> f64 {
    let z = rx.k * year / NEGLIGIBLE;
    if z > 1.0 { rx.ea / z.ln() } else { f64::INFINITY }
}

/// The substances reaction `rx` changes, each with what a reaction unit makes of it (minus what it uses).
pub(crate) fn terms(rx: &Reaction) -> impl Iterator<Item = (usize, f64)> + '_ {
    rx.inputs.iter().map(|&(k, nu)| (k as usize, -(nu as f64))).chain(rx.outputs.iter().map(|&(k, nu)| (k as usize, nu as f64)))
}

/// The input that runs out first: its amount per `nu` (reaction units) and its place in `inputs`.
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

/// The rate of a reaction per unit of `lim` before catalyst · k · exp(−Ea/T): its inputs' shares of their phases
/// (`whole`) to their order, times contact over `lim` (laws.md 2.1).
#[inline]
pub(crate) fn pace(rx: &Reaction, lim: f64, a: impl Fn(usize) -> f64, whole: impl Fn(usize) -> f64) -> f64 {
    let (mut x, mut e) = (1.0, f64::MAX);
    for (q, &(k, nu)) in rx.inputs.iter().enumerate() {
        let (a, n) = (a(k as usize), whole(k as usize));
        (x, e) = (x * pow(a / n, rx.order(q)), e.min(per(n, nu)));
    }
    x * e / lim
}

/// Catalyst factor `f` at `t` of a catalyst that loses its shape above `td` K over a width `wk` K. A speed-up is a
/// lowered barrier; an unfolded share loses it, so f is kept to the power 1 / (1 + exp((t − td) / wk)): smooth, and
/// near 1 a few `wk` above `td` whatever its size.
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
        let mut rc = Reactor {
            thr: chem.reactions.iter().map(|rx| frozen_below(rx, year)).collect(),
            lit: chem.reactions.iter().zip(&dh).map(|(rx, &d)| rx.light && d > 0.0).collect(),
            needs: chem.reactions.iter().map(|rx| rx.inputs.iter().fold(0, |m, &(k, _)| m | 1 << k)).collect(),
            light_yield: cfg.phys.light_yield,
            dh,
            cp: chem.subs.iter().map(|s| s.cp).collect(),
            melt: chem.subs.iter().map(|s| s.melt).collect(),
            boil: chem.subs.iter().map(|s| s.boil).collect(),
            ph: vec![Phase::Solid; ns],
            minor: vec![false; nr],
            ..Default::default()
        };
        // The scratch of each reaction and of each substance, empty; the lists of reactions fill as they run.
        [&mut rc.kap, &mut rc.lim, &mut rc.far, &mut rc.pw, &mut rc.kap0, &mut rc.lim0, &mut rc.done].into_iter().for_each(|v| *v = vec![0.0; nr]);
        [&mut rc.av, &mut rc.dpool].into_iter().for_each(|v| *v = vec![0.0; ns]);
        (rc.flux, rc.sums) = (vec![[0.0; 2]; ns], vec![[0.0; 2]; ns]);
        rc
    }

    /// The longest the node of the last `react` may wait before it reacts again, s: until a reaction at its starting rate
    /// would use a share `far` of its scarcest input (more than `trace` mol), or their heat would move it `dt_k`.
    pub(super) fn rest(&self, far: f64, trace: f64, dt_k: f64) -> f64 {
        let (mut most, mut heat) = (f64::INFINITY, 0.0);
        for &r in &self.on0 {
            let k = self.kap0[r];
            most = most.min((far / k).max(trace / (self.lim0[r] * k)));
            heat += self.dh[r].abs() * self.lim0[r] * k;
        }
        most.min(dt_k * self.cap0 / heat)
    }

    /// What the reactions of the last `react` made less what they used of each substance, mol, into `made`. Terms that
    /// cancel (a fast pair that undo each other) are summed exactly (`math::add_times`), so every element is kept.
    pub(super) fn made(&mut self, chem: &Chemistry, made: &mut [f64]) {
        self.sums.fill([0.0; 2]);
        for &r in &self.ran {
            for (k, nu) in terms(&chem.reactions[r]) {
                let (x, s) = (nu * self.done[r], &mut self.sums[k]);
                *s = [s[0] + x, s[1] + x.abs()];
            }
        }
        for (k, (d, &[sum, all])) in made.iter_mut().zip(&self.sums).enumerate() {
            *d = sum;
            if all > CANCEL * sum.abs() {
                self.parts.clear();
                for &r in &self.ran {
                    terms(&chem.reactions[r]).filter(|t| t.0 == k).for_each(|(_, nu)| add_times(&mut self.parts, nu, self.done[r]));
                }
                *d = self.parts.iter().sum();
            }
        }
    }

    /// The rates of the last `phases` under a liquid whose film renews its gas in `film` s.
    fn through_film(&mut self, chem: &Chemistry, tot: &[f64; 3], t: f64) {
        self.flux.fill([0.0; 2]);
        for &r in &self.on {
            let rx = &chem.reactions[r];
            for (side, &(k, nu)) in rx.inputs.iter().map(|p| (0, p)).chain(rx.outputs.iter().map(|p| (1, p))) {
                self.flux[k as usize][side] += nu as f64 * (self.kap[r] * self.lim[r]) * (self.ph[k as usize] == Phase::Gas) as u8 as f64;
            }
        }
        for k in 0..self.flux.len() {
            let [used, made] = self.flux[k];
            let eq = if used > 0.0 { tot[1] * chem.henry(k, t, self.av[k] / tot[2] * P_REF) } else { 0.0 };
            self.flux[k][0] = if eq > 0.0 { (1.0 + made * self.film / eq) / (1.0 + used * self.film / eq) } else { 1.0 };
        }
        for &r in &self.on {
            let rx = &chem.reactions[r];
            rx.inputs.iter().enumerate().for_each(|(q, p)| self.kap[r] *= self.flux[p.0 as usize][0].powi(rx.order(q) as i32));
        }
    }

    /// `pace` of `rx` with the amounts and phases of the last `phases`.
    pub(super) fn pace(&self, rx: &Reaction, lim: f64, tot: &[f64; 3]) -> f64 {
        pace(rx, lim, |k| self.av[k], |k| if self.ph[k] != Phase::Solid { tot[self.ph[k] as usize] } else { self.av[k] })
    }

    /// Phase and available amount of each substance at `t`, and the total of each phase (a solid is a phase of its own).
    pub(super) fn phases(&mut self, chem: &Chemistry, n: &[f64], gas: Option<&[f64]>, t: f64) -> [f64; 3] {
        let (mut tot, mut held) = ([0.0; 3], 0);
        for k in 0..n.len() {
            let p = (t >= self.melt[k] || n[k] > 0.0 && t >= chem.melt_in(k, |j| n[j])) as usize + (t >= self.boil[k]) as usize;
            let a = if let (Some(g), 2) = (gas, p) { n[k] + g[k] } else { n[k] };
            (self.ph[k], self.av[k]) = (PHASES[p], a);
            held |= ((a > 0.0) as u64) << k;
            tot[p] += a;
        }
        self.held = held;
        tot
    }

    /// React the amounts `n` of one node for `dt` seconds; returns the light used, J. `gas`: the gas around the node
    /// (gases are drawn from it first, gas products go to it); `energy`: its heat, J, at heat capacity `heat_cap` plus
    /// its amounts'; `cat`: catalyst factors with their `denature` limits; `light_j`: light available, J. Elements are
    /// exact; heat plus Σ n·hf gains exactly the light used.
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
        let (mut light, mut left) = (light_j, dt);
        self.on0.clear();
        self.ran.iter().for_each(|&r| self.done[r] = 0.0);
        self.ran.clear();
        self.dpool.fill(0.0);
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
        // A later sub-step may run a reaction an earlier one did not.
        self.ran.sort_unstable();
        light_j - light
    }

    /// The rate `kap`, limit `lim` and power `pw` of each reaction at `t` that is not frozen and has all its inputs (with
    /// light, for one that stores it). Returns whether any can run.
    fn rates(&mut self, chem: &Chemistry, tot: &[f64; 3], t: f64, cat: Option<(&[f64], f64, f64)>, light: f64) -> bool {
        self.on.clear();
        for (r, rx) in chem.reactions.iter().enumerate() {
            self.kap[r] = 0.0;
            // One whose inputs are not all here has none of the scarcest.
            if self.needs[r] & !self.held != 0 {
                continue;
            }
            let cr = cat.map_or(crate::body::Body::catalyst(rx.open, self.moist), |(c, td, wk)| denature(c[r], t, td, wk));
            if cr <= 0.0 || (t < self.thr[r] && cr <= 1.0) || (self.lit[r] && light <= 0.0) {
                continue;
            }
            let (lim, q) = scarce(&rx.inputs, |k| self.av[k]);
            if lim > 0.0 {
                self.kap[r] = cr * rx.arrhenius(t) * self.pace(rx, lim, tot);
                self.lim[r] = lim;
                self.on.push(r);
                // In a liquid or a gas the power is its order less 1, as far as it is not most of its phase.
                let k = rx.inputs[q].0 as usize;
                let share = if self.ph[k] != Phase::Solid { self.av[k] / tot[self.ph[k] as usize] } else { 1.0 };
                (self.pw[r], self.minor[r]) = ((rx.order(q) as f64 - 1.0) * (1.0 - share), share < TRACE);
            }
        }
        !self.on.is_empty()
    }

    /// The length of sub-step `iter` of the `left` seconds still to go, at heat capacity `c`: halved while the reactions'
    /// heat would move the temperature more than `MAX_DT_K` (not the last). Sets `far`, and on the first `kap0`, `lim0`.
    fn sub_step(&mut self, left: f64, c: f64, iter: usize) -> f64 {
        // Sets `far`, the share of its scarcest input each running reaction uses in `sub` seconds, and returns the heat of
        // those not driven by light over that time, J.
        let mut reach = |sub: f64| self.on.iter().filter(|&&r| self.kap[r] > 0.0).fold(0.0, |heat, &r| {
            self.far[r] = gone(self.kap[r] * sub, self.pw[r]);
            heat + if self.lit[r] { 0.0 } else { self.dh[r].abs() * self.lim[r] * self.far[r] }
        });
        let mut sub = left;
        loop {
            let heat = reach(sub);
            if heat <= MAX_DT_K * c || iter >= MAX_SUBSTEPS || sub <= left * MIN_SUB {
                break;
            }
            // The heat is concave in the length, so it falls by less than half when the length halves: the first try
            // jumps to the fewest halvings that can be enough, ceil(log2(heat / bound)), not past the shortest sub-step.
            let most = (1.0 / MIN_SUB).log2().ceil();
            let halvings = if sub == left { (heat / (MAX_DT_K * c)).log2().ceil().clamp(1.0, most) } else { 1.0 };
            sub *= 0.5f64.powi(halvings as i32);
        }
        for &r in self.on.iter().filter(|&&r| iter == 0 && self.kap[r] > 0.0 && !self.lit[r] && !self.minor[r]) {
            (self.kap0[r], self.lim0[r]) = (self.kap[r], self.lim[r]);
            self.on0.push(r);
        }
        sub
    }

    /// Run each reaction its share `far` of its scarcest input, one after the other, each limited by what is left (a gas
    /// from the `pool` around first, gas products to it), its heat to or from `energy`, stored light from `light`.
    fn apply(&mut self, chem: &Chemistry, n: &mut [f64], pool: &mut [f64], energy: &mut f64, light: &mut f64) {
        let has_pool = !pool.is_empty();
        for &r in self.on.iter().filter(|&&r| self.kap[r] > 0.0) {
            let rx = &chem.reactions[r];
            let (dh, lim) = (self.dh[r], scarce(&rx.inputs, |k| self.av[k]).0);
            // No more than the light it stores, or than `MOST_HEAT` of the heat it takes.
            let most = if self.lit[r] { *light * self.light_yield / dh } else if dh > 0.0 { MOST_HEAT * *energy / dh } else { f64::INFINITY };
            let ext = (lim * self.far[r]).min(most);
            if ext <= 0.0 {
                continue;
            }
            for &(k, nu) in &rx.inputs {
                let (k, d) = (k as usize, nu as f64 * ext);
                let from_pool = if self.ph[k] == Phase::Gas && has_pool { pool[k].min(d) } else { 0.0 };
                if from_pool > 0.0 {
                    (pool[k], self.dpool[k]) = (pool[k] - from_pool, self.dpool[k] - from_pool);
                }
                n[k] = (n[k] - (d - from_pool)).max(0.0);
                self.av[k] = (self.av[k] - d).max(0.0);
            }
            for &(k, nu) in &rx.outputs {
                let (k, d) = (k as usize, nu as f64 * ext);
                if self.ph[k] == Phase::Gas && has_pool {
                    (pool[k], self.dpool[k]) = (pool[k] + d, self.dpool[k] + d);
                } else {
                    n[k] += d;
                }
                self.av[k] += d;
            }
            if self.done[r] == 0.0 {
                self.ran.push(r);
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
