//! Law 2: Change. Substances react inside nodes, faster when hot.

use crate::World;
use crate::chem::Chemistry;

const YEAR_S: f64 = 3.156e7;
/// A reaction whose fraction per year stays below this is frozen and skipped.
const NEGLIGIBLE: f64 = 1e-12;
/// Largest temperature change in one sub-step, K.
const MAX_DT_K: f64 = 30.0;
const MAX_SUBSTEPS: usize = 48;
/// A node that is neither frozen nor hot reacts every `LAZY` ticks with `LAZY` times the time:
/// reactions at ordinary temperatures are slow, so one long step is as good as many short ones.
/// Nodes take turns in blocks of `BLOCK` (contiguous memory).
const LAZY: u64 = 40;
/// A substance making up less than this share of a ground node's loose amounts is a trace:
/// reactions on traces alone never make a node react every step.
const TRACE: f64 = 1e-6;
const BLOCK: usize = 64;

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
    /// Limiting input amount of each reaction at the start of the sub-step, in reaction units.
    lim: Vec<f64>,
}

/// 1 - exp(-z), cheap when small.
#[inline]
fn frac(z: f64) -> f64 {
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
        }
    }

    /// React the amounts `n` of one node for `dt` seconds. Returns the light energy used, J.
    /// - `gas`: extra amounts of the gas around the node (e.g. its share of the air). Substances that are
    ///   gas at the node's temperature are drawn from it first, and gas products are put into it.
    /// - `energy`: thermal energy of the node, J. Its temperature is `energy / (heat_cap + Σ n·cp)`,
    ///   where `heat_cap` is the fixed part (e.g. bonded ground).
    /// - `cat`: catalyst factor per reaction (`None` = 1 for all).
    /// - `light_j`: absorbed light energy available to light reactions, J.
    ///
    /// Elements are exact; thermal energy plus Σ n·hf is exact (light used leaves the node).
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
        let ns = n.len();
        let mut light = light_j;
        let mut left = dt;
        for iter in 0..=MAX_SUBSTEPS {
            if left <= 0.0 {
                break;
            }
            let c = heat_cap + n.iter().zip(&self.cp).map(|(a, c)| a * c).sum::<f64>();
            let t = *energy / c.max(1.0);
            // Phases, and the total moles of each phase: in a gas or a liquid the mole fraction is
            // a substance's activity; a solid is a phase of its own (activity 1).
            let mut tot = [0.0; 3];
            for k in 0..ns {
                let p = (t >= self.melt[k]) as usize + (t >= self.boil[k]) as usize;
                let a = if p == 2 && has_pool { n[k] + pool[k] } else { n[k] };
                self.ph[k] = p as u8;
                self.av[k] = a;
                tot[p] += a;
            }
            // Rates of the reactions that are not frozen and have all their inputs.
            let (mut any, mut heat) = (false, 0.0);
            for (r, rx) in chem.reactions.iter().enumerate() {
                self.kap[r] = 0.0;
                let cr = cat.map_or(1.0, |c| c[r]);
                let uses_light = rx.light && self.dh[r] > 0.0;
                if cr <= 0.0 || (t < self.thr[r] && cr <= 1.0) || (uses_light && light <= 0.0) {
                    continue;
                }
                let (mut lim, mut x) = (f64::MAX, 1.0);
                for &(k, nu) in &rx.inputs {
                    let (k, a) = (k as usize, self.av[k as usize]);
                    lim = lim.min(a / nu as f64);
                    if a <= 0.0 {
                        break;
                    }
                    if self.ph[k] > 0 {
                        let f = a / tot[self.ph[k] as usize];
                        x *= if nu == 1 { f } else { f.powi(nu as i32) };
                    }
                }
                if lim > 0.0 {
                    let kap = cr * rx.k * (-rx.ea / t).exp() * x;
                    self.kap[r] = kap;
                    self.lim[r] = lim;
                    any = true;
                    if !uses_light {
                        heat += self.dh[r].abs() * lim * frac(kap * left);
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
                let lim = rx.inputs.iter().map(|&(k, nu)| self.av[k as usize] / nu as f64).fold(f64::MAX, f64::min);
                let mut ext = lim * frac(self.kap[r] * sub);
                if uses_light {
                    ext = ext.min(light / dh);
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
                if uses_light { light -= dh * ext } else { *energy -= dh * ext }
            }
            left -= sub;
        }
        light_j - light
    }
}

/// Run Law 2 on ground nodes (loose amounts, gas from the air above) and on air nodes.
/// Node temperatures come from the last flow step. Each node looks at the reactions whose
/// inputs it holds: if one of them goes far within one step it reacts every step; if they are
/// only slow it reacts every `LAZY` steps (in turns, by block) for that time; if all are frozen
/// it is skipped.
pub fn step(w: &mut World, dt: f64) {
    let s = std::mem::take(&mut w.scratch);
    let (tg, ta) = s.temps();
    let turn = ((LAZY - (w.day / w.cfg.phys_days as u64) % LAZY) % LAZY) as usize;
    let chem = &w.chem;
    let (ns, n, na) = (chem.ns(), w.grid.n(), w.grid.na());
    let share = 1.0 / (w.grid.air_scale * w.grid.air_scale) as f64;
    let skin_area = w.cfg.skin_m * w.grid.area();
    let vol: Vec<f64> = (0..ns).map(|k| chem.molar_volume(k, crate::chem::Phase::Solid)).collect();
    // Gas held by the air carries its fusion and vaporisation enthalpy (see `check::totals`).
    let held: Vec<f64> = chem.subs.iter().map(|b| b.lf + b.lv).collect();
    let mut rc = Reactor::new(chem);
    // Per reaction (not driven by light): its inputs (one bit per substance), frozen below
    // `thr`, goes far within a step above `hot`.
    let bit = |k: usize| 1u64 << k.min(63);
    let rxs: Vec<(u64, f64, f64)> = (0..chem.reactions.len())
        .filter(|&r| !chem.reactions[r].light)
        .map(|r| {
            let rx = &chem.reactions[r];
            (rx.inputs.iter().fold(0, |m, p| m | bit(p.0 as usize)), rc.thr[r], rx.ea / (rx.k * dt / 0.05).ln().max(1e-9))
        })
        .collect();
    // How long a node at `t` holding the substances `has` (`more`: beyond traces) reacts in turn
    // `b` now: 0 if not at all.
    let span = |t: f64, b: usize, has: u64, more: u64| {
        let (mut cold, mut fast) = (true, false);
        for &(need, thr, hot) in &rxs {
            if t >= thr && has & need == need {
                cold = false;
                fast |= t >= hot && more & need == need;
            }
        }
        if fast { dt } else if !cold && b % LAZY as usize == turn { dt * LAZY as f64 } else { 0.0 }
    };
    // Can a reaction anywhere go far within a step? If not, blocks off their turn are skipped
    // unread.
    let tmax = tg.iter().cloned().fold(f64::MIN, f64::max);
    let present = (0..ns).filter(|&k| tg.is_empty() || s.present(k)).fold(0, |m, k| m | bit(k));
    let fast_any = tg.is_empty() || rxs.iter().any(|&(need, _, hot)| hot <= tmax && present & need == need);
    let in_air: Vec<u64> = (0..na).map(|j| (0..ns).filter(|&k| w.air[k * na + j] > 0.0).fold(0, |m, k| m | bit(k))).collect();
    let t_of = |v: &[f64], i: usize| v.get(i).copied().unwrap_or(f64::INFINITY);
    let (mut amt, mut pool, mut pool0) = (vec![0.0; ns], vec![0.0; ns], vec![0.0; ns]);
    let mut blk = vec![0.0; ns * BLOCK];

    // Ground: blocks of neighbouring nodes, so memory is read in runs.
    for b in 0..n.div_ceil(BLOCK) {
        let (i0, i1) = (b * BLOCK, ((b + 1) * BLOCK).min(n));
        if !fast_any && b % LAZY as usize != turn {
            continue;
        }
        let m = i1 - i0;
        let (mut bc, mut bv) = ([0.0; BLOCK], [0.0; BLOCK]);
        for k in 0..ns {
            blk[k * BLOCK..k * BLOCK + m].copy_from_slice(&w.ground[k * n + i0..k * n + i1]);
            for (x, &v) in w.bed[k * n + i0..k * n + i1].iter().enumerate() {
                bc[x] += v * chem.subs[k].cp;
                bv[x] += v * vol[k];
            }
        }
        for x in 0..m {
            let (i, t) = (i0 + x, t_of(tg, i0 + x));
            let j = w.grid.air_of(i);
            let trace = TRACE * (0..ns).map(|k| blk[k * BLOCK + x]).sum::<f64>();
            let has = (0..ns).filter(|&k| blk[k * BLOCK + x] > 0.0).fold(in_air[j], |m, k| m | bit(k));
            let more = (0..ns).filter(|&k| blk[k * BLOCK + x] > trace).fold(in_air[j], |m, k| m | bit(k));
            let time = span(t, b, has, more);
            if time == 0.0 {
                continue;
            }
            for k in 0..ns {
                amt[k] = blk[k * BLOCK + x];
                pool0[k] = w.air[k * na + j] * share;
            }
            pool.copy_from_slice(&pool0);
            let c_bed = if bv[x] > 0.0 { bc[x] / bv[x] * skin_area } else { 0.0 };
            // The reactor sees sensible heat: molten amounts hold their fusion enthalpy apart.
            let fused: f64 = (0..ns).filter(|&k| chem.subs[k].melt <= t).map(|k| amt[k] * chem.subs[k].lf).sum();
            let mut e = w.e_ground[i] - fused;
            rc.react(chem, &mut amt, Some(&mut pool), &mut e, c_bed, None, 0.0, time);
            w.e_ground[i] = e + fused;
            for k in 0..ns {
                blk[k * BLOCK + x] = amt[k];
                if pool[k] != pool0[k] {
                    w.air[k * na + j] += pool[k] - pool0[k];
                    w.e_ground[i] -= (pool[k] - pool0[k]) * held[k];
                }
            }
        }
        for k in 0..ns {
            w.ground[k * n + i0..k * n + i1].copy_from_slice(&blk[k * BLOCK..k * BLOCK + m]);
        }
    }
    // Air.
    for j in 0..na {
        let time = span(t_of(ta, j), j, in_air[j], in_air[j]);
        if time == 0.0 {
            continue;
        }
        for k in 0..ns {
            amt[k] = w.air[k * na + j];
        }
        rc.react(chem, &mut amt, None, &mut w.e_air[j], 0.0, None, 0.0, time);
        for k in 0..ns {
            w.e_air[j] -= (amt[k] - w.air[k * na + j]) * held[k];
            w.air[k * na + j] = amt[k];
        }
    }
    w.scratch = s;
}
