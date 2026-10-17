//! Which nodes react now: the reactions that may go far within a step somewhere, and the ground and air nodes where
//! they may (by their rate at the node's temperature and the most activity their inputs can have there, checked one
//! reaction at a time over all nodes).

use super::kernel::{frozen_below, pow, warm_at};
use super::{FAR, TRACE};
use crate::flow::FlowView;
use crate::chem::{Chemistry, Phase};
use crate::units::MAX_SUBSTANCES;
use crate::World;
use std::cell::Cell;

/// Gas a ground node holds itself (left by its reactions, or dissolved in its liquid) lets it
/// react every step only when it is more than this share of the air's gas over it: less changes
/// the warmth its reactions need by about a tenth of a kelvin, and it reacts at its turn anyway.
const OWN: f64 = 0.01;
/// Whether a substance is anywhere in the ground is asked of this many nodes at a time (it stops at the first that has some).
const SCAN: usize = 256;
/// The commonest substances of the ground (`COMMONEST` of them) are found from every `SAMPLE`-th node.
const SAMPLE: usize = 61;
const COMMONEST: usize = 3;
/// A reaction that is warm enough at no more than one ground node in `FEW` looks at those nodes only.
const FEW: usize = 8;

/// The bit of substance `k` in a set of substances.
#[inline]
pub(super) fn bit(k: usize) -> u64 {
    1u64 << k.min(MAX_SUBSTANCES - 1)
}

/// A reaction not driven by light: its inputs (a bit per substance), the warmth below which it is frozen
/// (`frozen_below`), and below which it is too slow to go far in a step even with all inputs at activity 1, K.
struct Gate {
    r: usize,
    need: u64,
    thr: f64,
    hot: f64,
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
    need: Vec<Cell<f64>>,
}

impl Cand {
    /// Temperature at which it goes far, for `z` times its rate constant over the step.
    fn warm(&self, z: f64) -> f64 {
        warm_at(self.ea, z)
    }
}

/// What the screening found: per ground node and per air node, whether it may go far this step; per air node, the
/// substances it holds (a bit each); and the reactions by which a node is frozen (`cold`).
pub(super) struct Plan {
    pub ground: Vec<bool>,
    pub air: Vec<bool>,
    pub in_air: Vec<u64>,
    gates: Vec<Gate>,
}

impl Plan {
    /// Is every reaction that has all its inputs in the set `has` frozen at `t`?
    pub fn cold(&self, t: f64, has: u64) -> bool {
        !self.gates.iter().any(|g| t >= g.thr && has & g.need == g.need)
    }
}

/// The most activity `a` mol of a gas can have among at least `d` mol of gas.
fn act(a: f64, d: f64) -> f64 {
    if d > 0.0 { (a / d).min(1.0) } else { 1.0 }
}

/// The most activity the rate per unit of the scarcest input can have from gases with shares
/// `f`, whichever input that is: all shares to `nu`, or one power of one left out (not a trace,
/// see `TRACE`). A bound: `pace` is never more, as `e / lim` is at most the scarcest input's
/// phase over its amount.
fn up(gas: &[(usize, u16)], f: &dyn Fn(usize) -> f64) -> f64 {
    (0..=gas.len()).filter(|&d| d == gas.len() || f(gas[d].0) >= TRACE).map(|d| gas.iter().enumerate().map(|(q, &(k, nu))| pow(f(k), nu.saturating_sub((q == d) as u16))).product::<f64>()).fold(0.0, f64::max)
}

/// The ground and air nodes that may go far within a step of `dt` seconds, at the temperatures of `nodes`.
pub(super) fn screen(w: &World, nodes: &FlowView, dt: f64) -> Plan {
    let (chem, ns, na) = (&w.chem, w.chem.ns(), w.grid.na());
    let gates: Vec<Gate> = (0..chem.reactions.len())
        .filter(|&r| !chem.reactions[r].light)
        .map(|r| {
            let rx = &chem.reactions[r];
            Gate { r, need: rx.inputs.iter().fold(0, |m, p| m | bit(p.0 as usize)), thr: frozen_below(rx), hot: warm_at(rx.ea, rx.k * dt / FAR) }
        })
        .collect();
    let mut in_air = vec![0u64; na];
    for k in 0..ns {
        for (m, &a) in in_air.iter_mut().zip(&w.air[k * na..(k + 1) * na]) {
            *m |= if a > 0.0 { bit(k) } else { 0 };
        }
    }
    let s = Screen::new(w, nodes, &gates, &in_air, dt);
    let air = (0..na).map(|j| s.cand.iter().any(|c| in_air[j] & c.need_bits == c.need_bits && nodes.ta[j] >= c.low && nodes.ta[j] >= s.need(c, j))).collect();
    Plan { ground: s.ground(), air, in_air, gates }
}

/// The numbers the screening of one step shares.
struct Screen<'a> {
    chem: &'a Chemistry,
    ground: &'a [f64],
    air: &'a [f64],
    tg: &'a [f64],
    air_of: &'a [u32],
    n: usize,
    na: usize,
    /// One ground node's share of its air node (`Grid::air_share`).
    share: f64,
    /// Per air node, the gas that is gas wherever a reaction may go far, mol.
    pool_lo: Vec<f64>,
    /// The commonest substances of the ground, most first.
    most: Vec<usize>,
    /// The substances that are liquid at some ground node.
    liquids: Vec<usize>,
    cand: Vec<Cand>,
}

impl<'a> Screen<'a> {
    fn new(w: &'a World, nodes: &FlowView<'a>, gates: &[Gate], in_air: &[u64], dt: f64) -> Self {
        let (chem, ns, n, na) = (&w.chem, w.chem.ns(), w.grid.n(), w.grid.na());
        let warmest = |v: &[f64]| v.iter().fold(f64::MIN, |a, &t| a.max(t));
        let (gmax, thi) = (warmest(nodes.tg), warmest(nodes.tg).max(warmest(nodes.ta)));
        let anywhere = in_air.iter().fold(0, |m, &x| m | x);
        let col = |k: usize| &w.ground[k * n..(k + 1) * n];
        let somewhere = |k: usize| anywhere & bit(k) != 0 || col(k).chunks(SCAN).any(|c| c.iter().fold(false, |m, &a| m | (a > 0.0)));
        // Reactions that may go far somewhere; a node must be at least as warm as `hot` for one, so
        // substances boiling below the least `hot` are gas wherever one can.
        let fast: Vec<&Gate> = gates.iter().filter(|g| g.hot <= thi && chem.reactions[g.r].inputs.iter().all(|p| somewhere(p.0 as usize))).collect();
        let tlo = fast.iter().fold(f64::INFINITY, |m, g| m.min(g.hot));
        let mut pool_lo = vec![0.0; na];
        for k in (0..ns).filter(|&k| chem.subs[k].boil <= tlo) {
            for (p, &a) in pool_lo.iter_mut().zip(&w.air[k * na..(k + 1) * na]) {
                *p += a;
            }
        }
        let most_act: Vec<f64> = (0..ns).map(|k| if chem.subs[k].boil <= tlo { (0..na).fold(0.0, |m, j| act(w.air[k * na + j], pool_lo[j]).max(m)) } else { 1.0 }).collect();
        let cand = fast.iter().map(|g| candidate(chem, g, tlo, dt, na, &most_act)).collect();
        let mut most: Vec<(f64, usize)> = (0..ns).map(|k| (col(k).iter().step_by(SAMPLE).sum::<f64>(), k)).collect();
        most.sort_by(|a, b| b.0.total_cmp(&a.0));
        let most = most.iter().take(COMMONEST).map(|m| m.1).collect();
        let gmin = nodes.tg.iter().fold(f64::MAX, |a, &t| a.min(t));
        let liquids = (0..ns).filter(|&k| chem.subs[k].melt <= gmax && chem.subs[k].boil > gmin).collect();
        Screen { chem, ground: &w.ground, air: &w.air, tg: nodes.tg, air_of: nodes.air_of, n, na, share: w.grid.air_share(), pool_lo, most, liquids, cand }
    }

    /// The amounts of substance `k` in the ground nodes.
    fn col(&self, k: usize) -> &'a [f64] {
        &self.ground[k * self.n..(k + 1) * self.n]
    }

    /// The warmth `c` needs at air node `j` for its gases at the most activity they can have there (`Cand::need`).
    fn need(&self, c: &Cand, j: usize) -> f64 {
        if c.need[j].get().is_nan() {
            c.need[j].set(c.warm(c.kdt * up(&c.gas, &|k| act(self.air[k * self.na + j], self.pool_lo[j]))));
        }
        c.need[j].get()
    }

    /// Does ground node `i` hold more of gas `k` than `OWN` of the air's over it?
    fn own_gas(&self, k: usize, i: usize) -> bool {
        self.col(k)[i] > OWN * self.share * self.air[k * self.na + self.air_of[i] as usize]
    }

    /// Ground nodes that may go far: warm enough; holding the inputs that are not gas beyond a
    /// trace (of the substances most of the ground is made of); and warm enough for the gases of
    /// the air above, thinned by those the node holds or gives off itself. Reactions that need
    /// warmth few nodes have look at those nodes only.
    fn ground(&self) -> Vec<bool> {
        let (n, tg) = (self.n, self.tg);
        let gmax = tg.iter().fold(f64::MIN, |a, &t| a.max(t));
        let (mut maybe, mut ok, mut gas) = (vec![false; n], vec![false; n], vec![false; n]);
        let mut warm: Option<Vec<u32>> = None;
        let mut cs: Vec<&Cand> = self.cand.iter().filter(|c| c.hot <= gmax).collect();
        cs.sort_by(|a, b| a.hot.total_cmp(&b.hot));
        for c in cs {
            if warm.is_none() && tg.iter().filter(|&&t| t >= c.hot).count() * FEW <= n {
                warm = Some((0..n as u32).filter(|&i| tg[i as usize] >= c.hot).collect());
            }
            if let Some(v) = &mut warm {
                v.retain(|&i| tg[i as usize] >= c.hot);
                for &i in v.iter() {
                    maybe[i as usize] = maybe[i as usize] || self.fits(c, i as usize, tg[i as usize]);
                }
                continue;
            }
            // Many warm nodes: a cheap test over all (holding the non-gas inputs beyond a trace of the
            // commonest substance, and warm enough for the air's gases or holding a gas input), then the full one.
            for (o, &t) in ok.iter_mut().zip(tg) {
                *o = t >= c.hot;
            }
            for &(k, boil, _, _) in &c.held {
                for ((o, &t), (&a, &b)) in ok.iter_mut().zip(tg).zip(self.col(k).iter().zip(self.col(self.most[0]))) {
                    *o &= t >= boil || a > TRACE * b;
                }
            }
            if !c.gas.is_empty() {
                for (g, &t) in gas.iter_mut().zip(tg) {
                    *g = t >= c.low;
                }
                for &(k, _) in &c.gas {
                    for (i, g) in gas.iter_mut().enumerate() {
                        *g |= self.own_gas(k, i);
                    }
                }
                for (o, &g) in ok.iter_mut().zip(&gas) {
                    *o &= g;
                }
            }
            for i in 0..n {
                if ok[i] && !maybe[i] {
                    maybe[i] = self.fits(c, i, tg[i]);
                }
            }
        }
        maybe
    }

    /// May `c` go far at ground node `i` at `t`?
    fn fits(&self, c: &Cand, i: usize, t: f64) -> bool {
        let col = |k: usize| self.col(k)[i];
        let held = |all: f64| c.held.iter().all(|&(k, boil, _, _)| t >= boil || col(k) > TRACE * all);
        if t < c.hot || !held(col(self.most[0])) {
            return false;
        }
        let liquid = |k: usize| self.chem.phase_in(k, t, col) == Phase::Liquid;
        let wet = || self.liquids.iter().filter(|&&k| liquid(k)).map(|&k| col(k)).sum::<f64>();
        // An input that counts more than once, in a liquid, goes by its share of the liquid to the
        // power `nu - 1`: a node holding a trace of it in its liquid does not react every step.
        if !c.shared.is_empty() {
            let all = wet();
            let x: f64 = (c.shared.iter().filter(|p| liquid(p.0))).map(|&(k, nu)| pow(col(k) / all, nu - 1)).product();
            if t < c.warm(c.kdt * x) {
                return false;
            }
        }
        let (j, own) = (self.air_of[i] as usize, c.gas.iter().any(|&(k, _)| self.own_gas(k, i)));
        if !own && (t < c.low || (!c.gas.is_empty() && t < self.need(c, j))) {
            return false;
        }
        // Vapour of what the node holds thins the gas too.
        let vapour = self.most.iter().filter(|&&k| t >= self.chem.subs[k].boil).map(|&k| col(k)).sum::<f64>();
        let d = self.pool_lo[j] + vapour / self.share;
        let x = |k: usize| act(col(k) / self.share + self.air[k * self.na + j], d);
        if (own || vapour > 0.0) && t < c.warm(c.kdt * up(&c.gas, &x)) {
            return false;
        }
        // The contact of the phases (`pace`): with a gas the scarce input, the reaction goes no faster
        // than the least the node holds of its other inputs (a liquid by all the liquid) over the
        // gas over it, times the gas's count; with another input scarce, all shares count.
        let airy: Vec<(usize, u16)> = c.gas.iter().copied().chain(c.held.iter().filter(|h| t >= h.1).map(|h| (h.0, h.3))).collect();
        if !airy.is_empty() && c.held.iter().any(|h| t < h.1) {
            let hold = (c.held.iter().filter(|h| t < h.1)).map(|&(k, _, nu, _)| (if liquid(k) { wet() } else { col(k) }) / nu as f64).fold(f64::MAX, f64::min);
            let gnu = c.held.iter().filter(|h| t >= h.1).fold(c.gnu, |m, h| m.max(h.2 as f64));
            let full: f64 = airy.iter().map(|&(k, o)| pow(x(k), o)).product();
            if t < c.warm(c.kdt * full.max((gnu * hold / (d * self.share)).min(1.0) * up(&airy, &x))) {
                return false;
            }
        }
        held(self.most.iter().map(|&k| col(k)).sum::<f64>() + c.held.iter().filter(|h| !self.most.contains(&h.0)).map(|h| col(h.0)).sum::<f64>())
    }
}

/// The candidate of gate `g`: its inputs that boil below `tlo` are gases (at most activity `most_act` in any air node).
fn candidate(chem: &Chemistry, g: &Gate, tlo: f64, dt: f64, na: usize, most_act: &[f64]) -> Cand {
    let rx = &chem.reactions[g.r];
    let (gas, held): (Vec<_>, Vec<_>) = rx.inputs.iter().enumerate().map(|(q, p)| (p.0 as usize, rx.order(q))).partition(|p| chem.subs[p.0].boil <= tlo);
    let shared = held.iter().copied().filter(|g| g.1 > 1).collect();
    let count = |k: usize| rx.inputs.iter().find(|p| p.0 as usize == k).map_or(1, |p| p.1);
    let gnu = gas.iter().map(|g| count(g.0)).max().unwrap_or(1) as f64;
    let held = held.iter().map(|g| (g.0, chem.subs[g.0].boil, count(g.0), g.1)).collect();
    let mut c = Cand { ea: rx.ea, kdt: rx.k * dt / FAR, hot: g.hot, low: 0.0, need_bits: g.need, gas, held, gnu, shared, need: vec![Cell::new(f64::NAN); na] };
    c.low = c.warm(c.kdt * up(&c.gas, &|k| most_act[k])).max(g.hot);
    c
}
