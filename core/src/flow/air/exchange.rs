//! The exchange of ground nodes with the air: evaporation, condensation (rain and cloud), and gases dissolving in the
//! ground's liquid; and how what crosses is booked (`Crossing`).

use super::super::land::wet;
use super::super::Scratch;
use super::{lerp, over_cell};
use crate::chem::Chemistry;
use crate::math::one_minus_exp;
use crate::store::{apply, Batch, Class};
use crate::units::R_GAS;
use crate::{World, AIR};

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
    pub(super) fn fit(&mut self, chem: &Chemistry, k: usize, hi: f64) {
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
    pub(super) fn at(&self, t: f64) -> f64 {
        let x = ((t - SAT_LOWEST) * SAT_PER_K).clamp(0.0, (self.0.len() - 2) as f64);
        let i = x as usize;
        let f = x - i as f64;
        self.0[i] + (self.0[i + 1] - self.0[i]) * f
    }
}

/// What evaporates over `dt` seconds from ground that holds `a` mol of a substance and gets `r` mol more as rain, evenly
/// over the step: `e` mol/s while the ground is wet, holding `wet` mol or more (liquid stands on it, or its loose ground
/// holds its field capacity or more: stage 1), and in proportion to what it holds below that (stage 2: the bucket of
/// Manabe 1969), integrated exactly over the step. Never more than it holds and gets.
fn bucket(a: f64, r: f64, e: f64, wet: f64, dt: f64) -> f64 {
    let rate = r / dt;
    if e <= 0.0 {
        return 0.0;
    }
    let (mut m, mut left, mut now) = (0.0, dt, a);
    if now > wet {
        let t1 = if e > rate { (now - wet) / (e - rate) } else { dt };
        if t1 >= dt {
            return (e * dt).min(a + r);
        }
        (m, left, now) = (e * t1, dt - t1, wet);
    }
    if wet <= 0.0 {
        return (m + rate.min(e) * left).min(a + r);
    }
    // Below `wet`: d(held)/dt = rate - k held, toward `rate / k`; when that is over `wet` the ground wets again.
    let (k, eq) = (e / wet, rate * wet / e);
    let t2 = if eq > wet { ((eq - now) / (eq - wet)).ln() / k } else { f64::INFINITY };
    let m = if t2 >= left { m + rate * left + (now - eq) * one_minus_exp(k * left) } else { m + rate * t2 - (wet - now) + e * (left - t2) };
    m.min(a + r).max(0.0)
}

/// The slope of what evaporates with the open rate is taken over this share of it, where the ground dries in the step.
const NUDGE: f64 = 1e-6;

/// `bucket` and its slope with the open rate `e`, mol per mol/s: `dt` where the ground stays wet through the step (and
/// gives no more than it holds and gets), else over a nudge of `e`.
pub(super) fn bucket_slope(a: f64, r: f64, e: f64, wet: f64, dt: f64) -> (f64, f64) {
    if e > 0.0 && a > wet && a - wet >= (e - r / dt) * dt && e * dt <= a + r {
        return (e * dt, dt);
    }
    let m = bucket(a, r, e, wet, dt);
    (m, (bucket(a, r, e * (1.0 + NUDGE), wet, dt) - m) / (e * NUDGE))
}

/// The share of the sky the droplets of the air cover where `m` mol condense over `area` m² in `dt` seconds (the faster
/// a node condenses, the more of its sky it covers: half at the rate `cloud`), times their mean reflectance, `mr / m`
/// (`mr`: mol × reflectance): condensate in the air scatters sunlight like the powder of its solid.
pub(super) fn cloud(cloud: f64, (m, mr): (f64, f64), area: f64, dt: f64) -> f64 {
    let rate = m / (area * dt);
    if m > 0.0 { rate / (rate + cloud) * mr / m } else { 0.0 }
}

/// What crosses between ground nodes and their air nodes, booked at its end in one call (`Crossing::book`): a link from
/// each ground node that exchanges to its air node, with the mol of each of its gases (+ up into the air) and the heat
/// that leaves the ground with them, J (the air receives it less the latent heat it holds of them, `store::held`); and
/// the lift of the gas, from each air node's heat into the world's potential energy, J (`Sky::cross` books it the same).
pub(super) struct Crossing {
    /// The gases, and the gravity that lifts them, m/s²
    ks: Vec<usize>,
    g: f64,
    links: Batch,
    lift: Vec<f64>,
    /// What each air node holds of each gas as the crossing leaves it, mol (`[x * na + j]`)
    pub(super) left: Vec<f64>,
}

impl Crossing {
    /// A crossing of the gases `ks`, from the air now.
    pub(super) fn new(w: &World, ks: Vec<usize>) -> Crossing {
        let na = w.mesh.level(AIR).n();
        let left: Vec<f64> = ks.iter().flat_map(|&k| w.air[k * na..(k + 1) * na].iter().copied()).collect();
        let links = Batch::new([Class::Ground, Class::Air], &ks.iter().map(|&k| k as u16).collect::<Vec<_>>());
        Crossing { ks, g: w.cfg.gravity, links, lift: vec![0.0; na], left }
    }
    /// `m` mol of gas `x` of the crossing go from ground node `i`, of `area` m², up into its air node `j` (negative `m`:
    /// down) with `heat` J that leaves the ground; the state of both nodes follows (`Scratch::note`, or for a gas that
    /// was `dissolved`, `Scratch::hold`; and the air's sums).
    pub(super) fn cross(&mut self, s: &mut Scratch, x: usize, (i, j, area): (usize, usize, f64), (m, heat): (f64, f64), dissolved: bool) {
        let (na, k) = (self.lift.len(), self.ks[x]);
        let p = s.sp[k];
        self.links.add((i, j), k as u16, (m, heat));
        (self.lift[j], self.left[x * na + j]) = (self.lift[j] + m * p.mm * s.view().lift(i, j, self.g), self.left[x * na + j] + m);
        if dissolved { s.hold(k, i, -m) } else { s.note(k, i, -m, area) }
        (s.capa[j], s.nt[j], s.mass[j]) = (s.capa[j] + m * p.cp, s.nt[j] + m, s.mass[j] + m * p.mm);
    }
    /// The links of the crossing and the lifts, for the store.
    pub(super) fn batches(self) -> (Batch, Batch) {
        let mut lift = Batch::falls(Class::Air);
        self.lift.iter().enumerate().for_each(|(j, &q)| lift.fall(j, -q));
        (self.links, lift)
    }
    /// Book the crossing through the store.
    pub(super) fn book(self, w: &mut World) {
        let (mut links, mut lift) = self.batches();
        apply(w, &[links.flux(), lift.flux()], &[], &[]);
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
pub(super) fn dissolve(w: &mut World, s: &mut Scratch, dt: f64, henry: &[(usize, usize)], (tmin, tmax): (f64, f64)) {
    let (n, na) = (w.mesh.cells().n(), w.mesh.level(AIR).n());
    let solv = s.liquids(&w.chem, (tmin, tmax), false);
    let mut pass = Crossing::new(w, henry.iter().map(|h| h.1).collect());
    for i in 0..n {
        let (t, j, area) = (s.state.tg[i], w.mesh.up(i, 0, AIR), w.mesh.cells().area()[i]);
        let tl = (lerp(s, &s.ta, i) - s.seen[i][0] * (s.head[i] - s.seen[i][1])).max(1.0);
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
            let tq = if m > 0.0 { s.ta[j] } else { t };
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
