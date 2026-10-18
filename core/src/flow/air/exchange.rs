//! The exchange of ground nodes with the air: evaporation, condensation (rain and cloud), and
//! gases dissolving in the ground's liquid.

use super::super::land::{melted, wet};
use super::super::scratch::may_evaporate;
use super::super::{follow, range, year_weight, Scratch};
use super::{air_temp, lerp, over_cell};
use crate::chem::Chemistry;
use crate::AIR;
use crate::math::one_minus_exp;
use crate::store::{apply, Batch, Class};
use crate::units::{P_REF, R_GAS};
use crate::World;

/// A gas making up less than this share of the air is a trace: it evaporates and condenses
/// only at the trace turns of the exchange (`flow::TRACE_TURN`), for the time to the next.
const TRACE_AIR: f64 = 1e-7;
/// Entries of the table of a saturation pressure (`Sat`) per K, and the step between them, K.
const SAT_PER_K: f64 = 20.0;
const SAT_STEP: f64 = 0.05;
/// Every this many entries the table is exact.
const SAT_EXACT: usize = 64;
/// The table starts at `SAT_LOWEST`, K, and reaches this far above the warmest temperature it is asked for, K.
const SAT_MARGIN: f64 = 1.0;
const SAT_LOWEST: f64 = 1.0;

/// Saturation pressure of one substance, tabulated every `SAT_STEP` from `SAT_LOWEST`, Pa. Each entry depends only on
/// its place, so a table built again further up holds the same numbers, and it is kept between calls (`Scratch::sat`).
#[derive(Clone, Debug, Default)]
pub(in crate::flow) struct Sat(Vec<f64>);

impl Sat {
    /// Reaches `hi` + `SAT_MARGIN` K at least, for substance `k` (built again further up when it does not): exact every
    /// `SAT_EXACT` entries; between, each from the last by their ratio, exp(b (1/t' - 1/t)), summed to its fifth term
    /// (ln p is linear in 1/t).
    fn fit(&mut self, chem: &Chemistry, k: usize, hi: f64) {
        let len = ((hi + SAT_MARGIN - SAT_LOWEST) * SAT_PER_K) as usize + 2;
        if self.0.len() >= len {
            return;
        }
        let (b, mut p) = (chem.subs[k].lv / R_GAS, 0.0);
        self.0 = (0..len)
            .map(|i| {
                let t = SAT_LOWEST + i as f64 / SAT_PER_K;
                if i % SAT_EXACT == 0 {
                    p = chem.p_sat(k, t);
                } else {
                    let x = b * SAT_STEP / (t * (t - SAT_STEP));
                    p *= 1.0 + x * (1.0 + x / 2.0 * (1.0 + x / 3.0 * (1.0 + x / 4.0)));
                }
                p
            })
            .collect();
    }
    fn at(&self, t: f64) -> f64 {
        let x = ((t - SAT_LOWEST) * SAT_PER_K).clamp(0.0, (self.0.len() - 2) as f64);
        let i = x as usize;
        let f = x - i as f64;
        self.0[i] + (self.0[i + 1] - self.0[i]) * f
    }
}

/// A gas the exchange moves in this call.
struct Active {
    /// Index in `Scratch::gas`, and the substance.
    gi: usize,
    k: usize,
    /// It may evaporate from the ground.
    up: bool,
    /// Evaporation per unit of deficit and of area over the time it moves for, mol K/(Pa m²).
    c_evap: f64,
}

/// Evaporation by vapour pressure (the ground's saturation pressure against the partial pressure
/// near the ground); then condensation where the air a cell sees holds more than it can at its
/// temperature (colder on higher ground): what it cannot hold rains out within the call; then gases
/// that cannot condense dissolve in the ground's liquid (`dissolve`, slow: with the trace gases). With `trace` (a
/// trace turn, `flow::TRACE_TURN`) the trace gases move, and gases dissolve, for that many seconds.
pub(crate) fn exchange(w: &mut World, s: &mut Scratch, dt: f64, trace: Option<f64>) {
    let (tmin, tmax) = range(&s.tg);
    let henry: Vec<(usize, usize)> = s.gas.iter().copied().enumerate().filter(|&(_, k)| s.sp[k].boil < tmin && w.chem.subs[k].solubility > 0.0).collect();
    let act = active(w, s, &henry, dt, trace);
    // Each pass sees the air as it was before it.
    evaporate(w, s, &act);
    let (rained, rain, cloud) = condense(w, s, &act);
    if let Some(span) = trace {
        dissolve(w, s, span, &henry, (tmin, tmax));
    }
    for (a, _) in act.iter().zip(&rained).filter(|(_, &r)| r) {
        s.add_loose(a.k, false);
    }
    clouds(w, s, dt, (&rain, &cloud));
}

/// The gases that leave the ground or may condense (not those of `henry`, which dissolve), with
/// their evaporation per unit of deficit, and their saturation pressures up to the warmest ground or air touching it
/// (`Scratch::sat`): a trace gas only at a `trace` turn, for its seconds, the others for `dt`.
fn active(w: &World, s: &mut Scratch, henry: &[(usize, usize)], dt: f64, trace: Option<f64>) -> Vec<Active> {
    let (na, tmax) = (w.mesh.level(AIR).n(), range(&s.tg).1);
    let (hi, all) = (tmax.max(range(&s.tl).1), s.nt.iter().sum::<f64>());
    s.sat.resize(w.chem.ns(), Sat::default());
    let mut act = Vec::new();
    for (gi, &k) in s.gas.iter().enumerate() {
        let up = s.loose.contains(&k) && may_evaporate(w, k, tmax) && !henry.iter().any(|h| h.1 == k);
        let span = if w.air[k * na..(k + 1) * na].iter().sum::<f64>() < TRACE_AIR * all { trace } else { Some(dt) };
        if let Some(span) = span.filter(|_| up || s.cond[gi]) {
            s.sat[k].fit(&w.chem, k, hi);
            act.push(Active { gi, k, up, c_evap: w.cfg.evap * span / R_GAS });
        }
    }
    act
}

/// What a pass of the exchange moves, booked at its end in one call (`Pass::book`): a link from each ground node that
/// exchanges to its air node, with the mol of each of its gases (+ up into the air) and the heat that leaves the ground
/// with them, J (the air receives it less the latent heat it holds of them, `store::held`); and the lift of the gas,
/// from each air node's heat into the world's potential energy, J (`Sky::cross` books it the same).
struct Pass {
    /// The gases, and the gravity that lifts them, m/s²
    ks: Vec<usize>,
    g: f64,
    links: Batch,
    lift: Vec<f64>,
    /// The air each pass sees, as it was before it: each air node's temperature, K, and what it holds of each gas, mol
    /// (`[x * na + j]`); and what it holds as the pass leaves it
    ta: Vec<f64>,
    seen: Vec<f64>,
    left: Vec<f64>,
}

impl Pass {
    /// A pass of the gases `ks`, from the air now.
    fn new(w: &World, s: &Scratch, ks: Vec<usize>) -> Pass {
        let na = w.mesh.level(AIR).n();
        let left: Vec<f64> = ks.iter().flat_map(|&k| w.air[k * na..(k + 1) * na].iter().copied()).collect();
        let ta = (0..na).map(|j| air_temp(w.e_air[j], s.capa[j])).collect();
        let links = Batch::new([Class::Ground, Class::Air], &ks.iter().map(|&k| k as u16).collect::<Vec<_>>());
        Pass { ks, g: w.cfg.gravity, links, lift: vec![0.0; na], ta, seen: left.clone(), left }
    }
    /// `m` mol of gas `x` of the pass go from ground node `i`, of `area` m², up into its air node `j` (negative `m`:
    /// down) with `heat` J that leaves the ground; the state of both nodes follows (`Scratch::note`, or for a gas that
    /// was `dissolved`, `Scratch::hold`; and the air's sums).
    fn cross(&mut self, s: &mut Scratch, x: usize, (i, j, area): (usize, usize, f64), (m, heat): (f64, f64), dissolved: bool) {
        let (na, k) = (self.lift.len(), self.ks[x]);
        let p = s.sp[k];
        self.links.add((i, j), k as u16, (m, heat));
        (self.lift[j], self.left[x * na + j]) = (self.lift[j] + m * p.mm * s.view().lift(i, j, self.g), self.left[x * na + j] + m);
        if dissolved { s.hold(k, i, -m) } else { s.note(k, i, -m, area) }
        (s.capa[j], s.nt[j], s.mass[j]) = (s.capa[j] + m * p.cp, s.nt[j] + m, s.mass[j] + m * p.mm);
    }
    /// Book the pass through the store.
    fn book(mut self, w: &mut World) {
        let mut lift = Batch::falls(Class::Air);
        self.lift.iter().enumerate().for_each(|(j, &q)| lift.fall(j, -q));
        apply(w, &[self.links.flux(), lift.flux()], &[], &[]);
    }
}

/// The amount of gas `x` of `act` in the column above ground cell `i` (of the air as `pass` sees
/// it), mol, and the height it fills near the ground, m.
fn above(w: &World, s: &Scratch, pass: &Pass, (a, x): (&Active, usize), i: usize) -> (f64, f64) {
    let na = w.mesh.level(AIR).n();
    (lerp(s, &pass.seen[x * na..(x + 1) * na], i) * over_cell(w, s, i), lerp(s, &s.hk[a.gi * na..(a.gi + 1) * na], i))
}

/// Evaporation from every ground node, booked as one pass (`Pass`).
fn evaporate(w: &mut World, s: &mut Scratch, act: &[Active]) {
    let n = w.mesh.cells().n();
    let mut pass = Pass::new(w, s, act.iter().map(|a| a.k).collect());
    for i in 0..n {
        let (t, j, tl, area) = (s.tg[i], w.mesh.up(i, 0, AIR), s.tl[i], w.mesh.cells().area()[i]);
        let c_cond = area / R_GAS;
        for (x, ac) in act.iter().enumerate() {
            let (k, a) = (ac.k, w.ground[ac.k * n + i]);
            if !ac.up || a <= 0.0 {
                continue;
            }
            let (share, hk) = above(w, s, &pass, (ac, x), i);
            let (sp, pa) = (s.sp[k], share * tl / (c_cond * hk));
            let ps = if t >= sp.boil { f64::INFINITY } else { s.sat[k].at(t) };
            if ps <= pa {
                continue;
            }
            let m = if t >= sp.boil {
                a
            } else {
                let d = a * sp.v[melted(s, k, i) as usize] / area;
                // A wet surface evaporates at the open rate: liquid stands on it, or its loose ground holds its field
                // capacity or more (stage 1); a drier one goes with its liquid over that (stage 2: the bucket of Manabe 1969).
                // No more than fills the column above the cell to saturation (the rest would rain back at once).
                (ac.c_evap * area * (ps - pa) / t * (d / (w.cfg.phys.field * s.pd[i])).min(1.0)).min(a).min((ps - pa) * c_cond * hk / tl)
            };
            // Evaporating cools the node, no further than where the vapour pressure falls to the
            // partial pressure (the boiling point, when it boils): only heat above that pays.
            let over = t - if t >= sp.boil { sp.boil } else { w.chem.t_sat(k, pa / P_REF).min(t) };
            let m = m.min(s.cap[i].max(1.0) * over / (sp.cp * over + sp.lv));
            if m <= 0.0 {
                continue;
            }
            // The ground pays the gas's heat and its fusion and vaporisation enthalpy; the air gets the gas's heat.
            pass.cross(s, x, (i, j, area), (m, m * (sp.cp * t + sp.lf + sp.lv)), false);
        }
    }
    pass.book(w);
}

/// Condensation onto every ground node of what the air it sees cannot hold at the temperature of
/// the air touching it, booked as one pass (`Pass`). Returns which gases of `act` rained, what
/// condensed on each ground node, mol, and in each air node, mol and mol × reflectance.
fn condense(w: &mut World, s: &mut Scratch, act: &[Active]) -> (Vec<bool>, Vec<f64>, Vec<[f64; 2]>) {
    let (n, na) = (w.mesh.cells().n(), w.mesh.level(AIR).n());
    let (mut rained, mut pass) = (vec![false; act.len()], Pass::new(w, s, act.iter().map(|a| a.k).collect()));
    let (mut rain, mut cloud) = (vec![0.0; n], vec![[0.0; 2]; na]);
    for (i, rain) in rain.iter_mut().enumerate() {
        let (j, tl, area) = (w.mesh.up(i, 0, AIR), s.tl[i], w.mesh.cells().area()[i]);
        let c_cond = area / R_GAS;
        for (x, ac) in act.iter().enumerate() {
            if !s.cond[ac.gi] {
                continue;
            }
            let k = ac.k;
            let (share, hk) = above(w, s, &pass, (ac, x), i);
            let m = (share - s.sat[k].at(tl) * c_cond * hk / tl).min(pass.left[x * na + j]);
            if m <= 0.0 {
                continue;
            }
            let (sp, tair) = (s.sp[k], pass.ta[j]);
            // Condensing frees the vaporisation enthalpy into the air; the liquid keeps its fusion.
            pass.cross(s, x, (i, j, area), (-m, -m * (sp.cp * tair + sp.lf)), false);
            *rain += m;
            cloud[j] = [cloud[j][0] + m, cloud[j][1] + m * sp.refl[0]];
            rained[x] = true;
        }
    }
    pass.book(w);
    (rained, rain, cloud)
}

/// Condensate in the air scatters sunlight like the powder of its solid; the faster a node
/// condenses, the more of its sky it covers (`cloud`: condensed in each air node, mol and mol ×
/// reflectance). Also the yearly average of the rain on each cell (`rain`, mol).
fn clouds(w: &mut World, s: &mut Scratch, dt: f64, (rain, cloud): (&[f64], &[[f64; 2]])) {
    let air = w.mesh.level(AIR).area();
    let refl: Vec<f64> = (cloud.iter().zip(air))
        .map(|(&[m, mr], a)| {
            let rate = m / (a * dt);
            if m > 0.0 { rate / (rate + w.cfg.cloud) * mr / m } else { 0.0 }
        })
        .collect();
    let ema = year_weight(w, dt);
    for (i, r) in rain.iter().enumerate() {
        s.seen[i][3] = lerp(s, &refl, i);
        let area = w.mesh.cells().area()[i];
        follow(&mut w.rain[i], (r / (area * dt)) as f32, ema);
    }
}

/// Gases that cannot condense on the ground (`henry`: index in `gas`, substance) dissolve in the
/// liquid standing on each ground node toward the share Henry's law gives (`Chemistry::henry`), and
/// come out as the liquid warms, thins or freezes or the gas thins. They cross the surface through
/// the air's film (`evap`, as evaporation) and the liquid's (`dissolve`, over its whole depth: one
/// mixed layer, a known limit) in series, solved exactly over the step toward the balance of node
/// and column. Dissolving frees the vaporisation enthalpy into the node (the heat of solution) and
/// the gas brings its heat; coming out takes them back. Dissolved gas moves with the liquid
/// (`liquid`), may feed ground reactions (Law 2), and goes back to the air only here.
fn dissolve(w: &mut World, s: &mut Scratch, dt: f64, henry: &[(usize, usize)], (tmin, tmax): (f64, f64)) {
    let (n, na) = (w.mesh.cells().n(), w.mesh.level(AIR).n());
    let solv = s.liquids(&w.chem, (tmin, tmax), false);
    let mut pass = Pass::new(w, s, henry.iter().map(|h| h.1).collect());
    for i in 0..n {
        let (t, j, tl, area) = (s.tg[i], w.mesh.up(i, 0, AIR), s.tl[i], w.mesh.cells().area()[i]);
        let (l, v) = solv.iter().fold((0.0, 0.0), |(l, v), &k| (l + w.ground[k * n + i] * wet(s, k, i), v + w.ground[k * n + i] * wet(s, k, i) * s.sp[k].v[1]));
        for (x, &(gi, k)) in henry.iter().enumerate() {
            let (a, sp) = (w.ground[k * n + i], s.sp[k]);
            if l <= 0.0 && a <= 0.0 {
                continue;
            }
            // The column over the cell, the height it fills, mol dissolved per mol at balance, and the
            // time to balance through the two films.
            let (col, hk) = (lerp(s, &w.air[k * na..(k + 1) * na], i) * over_cell(w, s, i), lerp(s, &s.hk[gi * na..(gi + 1) * na], i));
            let b = l * w.chem.henry(k, t, R_GAS * tl / (area * hk));
            let tau = b * hk / w.cfg.evap + v / area / w.cfg.dissolve;
            let m = (b * (a + col) / (1.0 + b) - a) * one_minus_exp(dt * (1.0 + b) / tau);
            // Coming out cools the node, no further than the boiling point.
            let m = if m >= 0.0 { m.min(pass.left[x * na + j]) } else { -(-m).min(s.cap[i].max(1.0) * (t - sp.boil) / (sp.cp * (t - sp.boil) + sp.lv)) };
            if m == 0.0 {
                continue;
            }
            // The gas brings its heat (at the air's temperature, or the ground's when it comes out), and dissolving frees
            // its fusion and vaporisation enthalpy into the node.
            let tq = if m > 0.0 { pass.ta[j] } else { t };
            pass.cross(s, x, (i, j, area), (-m, -m * (sp.cp * tq + sp.lf + sp.lv)), true);
        }
    }
    pass.book(w);
    for &(_, k) in henry {
        if (0..n).any(|i| w.ground[k * n + i] > 0.0) {
            s.add_loose(k, true);
        }
    }
}
