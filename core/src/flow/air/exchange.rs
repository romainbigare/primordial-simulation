//! The exchange of ground nodes with the air: evaporation, condensation (rain and cloud), and gases dissolving in the
//! ground's liquid; and how what dissolves is booked (`Crossing`).

use super::super::{land::wet, Scratch};
use super::{lerp, over_cell};
use crate::{chem::Chemistry, math::one_minus_exp, store::{apply, Batch, Class}, units::R_GAS, AIR, World};

/// Entries of the table of a saturation pressure (`Sat`) per K, and the step between them, K.
const SAT_PER_K: f64 = 20.0;
const SAT_STEP: f64 = 0.05;
/// Every this many entries the table is exact.
const SAT_EXACT: usize = 64;
/// The table starts at `SAT_LOWEST`, K, and reaches this far above the warmest temperature it is asked for, K.
const SAT_MARGIN: f64 = 1.0;
const SAT_LOWEST: f64 = 1.0;

/// Saturation pressure of one substance, tabulated every `SAT_STEP` from `SAT_LOWEST`, Pa; each entry depends only on
/// its place, so a table rebuilt further up holds the same numbers.
#[derive(Clone, Debug, Default)]
pub(in crate::flow) struct Sat(Vec<f64>);

impl Sat {
    /// Reaches `hi` + `SAT_MARGIN` K at least for substance `k`: exact every `SAT_EXACT` entries, between each from the
    /// last by their ratio exp(b (1/t' - 1/t)), to its fifth term (ln p is linear in 1/t).
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

/// What evaporates over `dt` seconds from ground holding `a` mol that gets `r` mol of rain evenly: `e` mol/s while it
/// holds `wet` mol or more, in proportion below that (the bucket of Manabe 1969), integrated exactly over the step.
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

/// `bucket` and its slope with the open rate `e`: `dt` where the ground stays wet through the step, else over a nudge.
pub(super) fn bucket_slope(a: f64, r: f64, e: f64, wet: f64, dt: f64) -> (f64, f64) {
    if e > 0.0 && a > wet && a - wet >= (e - r / dt) * dt && e * dt <= a + r {
        return (e * dt, dt);
    }
    let m = bucket(a, r, e, wet, dt);
    (m, (bucket(a, r, e * (1.0 + NUDGE), wet, dt) - m) / (e * NUDGE))
}

/// The cloud where `m` mol condense over `area` m² in `dt` seconds, a layer of droplets whose optical depth grows with
/// the rate: [its reflectance, its optical depth to heat radiation]. A layer of scattering depth x reflects x / (1 + x)
/// (two-stream, Lacis and Hansen 1974), half at the rate `cloud`, times the condensate's mean reflectance `mr / m` as
/// it scatters like its powder; the same droplets absorb heat radiation, with the depth `cloud_ir` at that rate
/// (Stephens 1978).
pub(super) fn cloud((cloud, cloud_ir): (f64, f64), (m, mr): (f64, f64), area: f64, dt: f64) -> [f64; 2] {
    let x = m / (area * dt) / cloud;
    if m > 0.0 { [x / (1.0 + x) * mr / m, cloud_ir * x] } else { [0.0; 2] }
}

/// Gases that cannot condense on the ground (`henry`) dissolve in each ground node's liquid toward Henry's share and
/// come out as it warms, thins or freezes, through the air's film and the liquid's in series, solved exactly over the
/// step (laws.md 1.5). Dissolving frees the vaporisation enthalpy into the node; coming out takes it back. What crosses
/// is booked in one call: a link per ground node with the mol of each gas (+ up) and the heat that leaves the ground,
/// and the lift of the gas into the potential.
pub(super) fn dissolve(w: &mut World, s: &mut Scratch, dt: f64, henry: &[(usize, usize)], (tmin, tmax): (f64, f64)) {
    let (n, na) = (w.mesh.cells().n(), w.mesh.level(AIR).n());
    let solv = s.liquids(&w.chem, (tmin, tmax), false);
    let mut links = Batch::new([Class::Ground, Class::Air], &henry.iter().map(|h| h.1 as u16).collect::<Vec<_>>());
    // The lift paid by each air node, J; what each air node holds of each gas as the crossing leaves it, mol
    // (`[x * na + j]`).
    let mut lift = vec![0.0; na];
    let mut left: Vec<f64> = henry.iter().flat_map(|h| w.store.amounts(Class::Air)[h.1 * na..(h.1 + 1) * na].iter().copied()).collect();
    for i in 0..n {
        let (t, j, area) = (s.state.tg[i], w.mesh.up(i, 0, AIR), w.mesh.cells().area()[i]);
        let tl = (lerp(s, &s.ta, i) - s.seen[i][0] * (s.head[i] - s.seen[i][1])).max(1.0);
        let (l, v) = solv.iter().fold((0.0, 0.0), |(l, v), &k| (l + w.store.amounts(Class::Ground)[k * n + i] * wet(s, k, i), v + w.store.amounts(Class::Ground)[k * n + i] * wet(s, k, i) * s.sp[k].v[1]));
        for (x, &(gi, k)) in henry.iter().enumerate() {
            let (a, sp) = (w.store.amounts(Class::Ground)[k * n + i], s.sp[k]);
            if l <= 0.0 && a <= 0.0 {
                continue;
            }
            // The column over the cell, its height, mol dissolved per mol at balance, and the time to balance.
            let (col, hk) = (lerp(s, &w.store.amounts(Class::Air)[k * na..(k + 1) * na], i) * over_cell(w, s, i), lerp(s, &s.hk[gi * na..(gi + 1) * na], i));
            let b = l * w.chem.henry(k, t, R_GAS * tl / (area * hk));
            let tau = b * hk / w.cfg.evap + v / area / w.cfg.dissolve;
            let m = (b * (a + col) / (1.0 + b) - a) * one_minus_exp(dt * (1.0 + b) / tau);
            // Coming out cools the node, no further than the boiling point.
            let m = if m >= 0.0 { m.min(left[x * na + j]) } else { -(-m).min(s.cap[i].max(1.0) * (t - sp.boil) / (sp.cp * (t - sp.boil) + sp.lv)) };
            if m == 0.0 {
                continue;
            }
            // The gas brings its heat (the air's, or the ground's when it comes out) and frees its latent heat in the node;
            // `up` mol go up from the ground node into its air node (negative: down), and the ground node's state follows.
            let (tq, up) = (if m > 0.0 { s.ta[j] } else { t }, -m);
            links.add((i, j), k as u16, (up, -m * (sp.cp * tq + sp.lf + sp.lv)));
            (lift[j], left[x * na + j]) = (lift[j] + up * sp.mm * s.view().lift(i, j, w.cfg.gravity), left[x * na + j] + up);
            s.hold(k, i, -up);
        }
    }
    apply(w, &[links.flux(), Batch::fallen(Class::Air, lift.iter().map(|q| -q)).flux()], &[], &[]);
    for &(_, k) in henry {
        if (0..n).any(|i| w.store.amounts(Class::Ground)[k * n + i] > 0.0) {
            s.add_loose(k, true);
        }
    }
}
