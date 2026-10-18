//! The working memory of Flow (`Scratch`), its tables, and the every-few-months recheck of what
//! is present (`refresh`).

use super::air;
use super::land::NONE;
use super::range;
use crate::chem::Chemistry;
use crate::solve::downhill::Order;
use crate::World;
use serde::{Deserialize, Serialize};

/// Numbers of one substance Flow reads often (from `chem::Substance`).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub(super) struct Sp {
    /// Heat capacity, J/(mol K)
    pub(super) cp: f64,
    /// Molar mass, kg/mol
    pub(super) mm: f64,
    /// Fusion enthalpy, J/mol
    pub(super) lf: f64,
    /// Vaporisation enthalpy, J/mol
    pub(super) lv: f64,
    /// Heat conductivity, W/(m K), no less than `COND_MIN`
    pub(super) cond: f64,
    /// Molar volume [solid, liquid], m³/mol
    pub(super) v: [f64; 2],
    /// Melting point, K
    pub(super) melt: f64,
    /// Boiling point, K
    pub(super) boil: f64,
    /// Mean reflectance [solid, liquid]
    pub(super) refl: [f64; 2],
}

/// The fusion enthalpy a ground node holds off a plateau, and the energies between which it
/// holds it (until the amounts change).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub(super) struct Latent {
    /// Fusion enthalpy held by what is molten, J
    pub(super) held: f64,
    /// The node keeps this `held` while its energy is at least `lo` and below `hi`, J (`hi` is
    /// `f64::MIN` on a plateau, or when `lift` marks the node for a new state)
    pub(super) lo: f64,
    pub(super) hi: f64,
    /// What is at or over its melting point, one bit per substance (`melted`)
    pub(super) molten: u64,
}

/// The bonded ground of a ground node as of the last refresh (liquid wears `vol`).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub(super) struct Bed {
    /// Volume, m³
    pub(super) vol: f64,
    /// Mean reflectance
    pub(super) refl: f64,
    /// Mean heat conductivity by volume, W/(m K)
    pub(super) cond: f64,
}

/// What Law 2 reads of the last flow step (`Scratch::view`).
pub(crate) struct FlowView<'a> {
    /// Per ground node: its temperature, K; the heat capacity of the skin of its bed, J/K; its air node; the head of
    /// its liquid, m
    pub tg: &'a [f64],
    pub skin: &'a [f64],
    pub air_of: &'a [u32],
    pub head: &'a [f64],
    /// Per air node: its temperature, K; the mean height of its air, m
    pub ta: &'a [f64],
    pub zair: &'a [f64],
    /// Per ground node: the fusion enthalpy held off a plateau, and the substance on its plateau (or `NONE`)
    latent: &'a [Latent],
    pk: &'a [usize],
}

impl FlowView<'_> {
    /// Energy to lift a kg from the head of ground node `i` to the mean height of its air node, J/kg, under a gravity
    /// of `g` m/s² (`Sky::cross`).
    #[inline]
    pub(crate) fn lift(&self, i: usize, g: f64) -> f64 {
        g * (self.zair[self.air_of[i] as usize] - self.head[i])
    }
    /// Fusion enthalpy ground node `i` holds when its energy is `e`, J, as Flow's last solve of its plateaus has it
    /// (`ground::melting`): that of what is molten off a plateau and, on one, as much of the fusion enthalpy of the
    /// substance there (`lat(k)` J for all of it) as the energy above the plateau's temperature at heat capacity `c` melts.
    #[inline]
    pub(crate) fn fused(&self, i: usize, e: f64, c: f64, lat: impl Fn(usize) -> f64) -> f64 {
        let held = self.latent[i].held;
        match self.pk[i] {
            NONE => held,
            k => held + (e - c * self.tg[i] - held).clamp(0.0, lat(k)),
        }
    }
}

/// Working memory of the law, rebuilt from the world when missing. It is saved with the world
/// (but for the fields marked `serde(skip)`: tables `tables` rebuilds and space used within a
/// call): the slower parts of the law run from what it held at their last turn, so a resumed run
/// matches an unbroken one. Ground fields are per ground node, air fields per air node.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scratch {
    /// The month of the last step, and the creep period (`CREEP_YEARS`) of the last liquid step.
    pub(super) month: Option<u64>,
    pub(super) year: Option<u64>,
    /// Seconds since the wind last moved gas between air nodes, since liquid (and creep) last
    /// moved, and since the ground last exchanged with the air.
    pub(super) pending: f64,
    pub(super) pending_liq: f64,
    pub(super) pending_exc: f64,
    /// The substances present changed: rebuild the state of ground and air at the next step.
    #[serde(skip)]
    pub(super) stale: bool,
    /// Area of a ground cell, m².
    #[serde(skip)]
    pub(super) area: f64,
    /// The numbers of each substance.
    #[serde(skip)]
    pub(super) sp: Vec<Sp>,
    /// Substances present in loose ground, in order (`refresh`; exchange and liquid add new ones).
    pub(super) loose: Vec<usize>,
    /// Loose substances that stay solid and cannot evaporate, and the others in order of melting point.
    pub(super) fixed: Vec<usize>,
    pub(super) vary: Vec<usize>,
    /// Volume of the `fixed` solids of each ground node, and volume × reflectance, m³.
    pub(super) fix: Vec<[f64; 2]>,
    /// Substances present in bonded ground, and in the air (or able to evaporate into it).
    pub(super) bedk: Vec<usize>,
    pub(super) gas: Vec<usize>,
    /// Heat capacity of the skin of each ground node's bed, J/K (at every step, `land::ground_state`).
    pub(super) skin: Vec<f64>,
    /// Bonded ground of each ground node.
    pub(super) bed: Vec<Bed>,
    /// Heat capacity of each ground node, J/K (from the amounts at every step; moves keep it).
    pub(super) cap: Vec<f64>,
    /// Temperature of each ground node, K.
    pub(super) tg: Vec<f64>,
    /// Substance held at its melting point in each ground node (or `NONE`), and its molten share.
    pub(super) pk: Vec<usize>,
    pub(super) phi: Vec<f64>,
    /// Fusion enthalpy held off a plateau, per ground node.
    pub(super) latent: Vec<Latent>,
    /// Height of the solid surface, and of the surface of all the liquid (pores filled first), m.
    pub(super) z: Vec<f64>,
    pub(super) head: Vec<f64>,
    /// Pore space of the loose ground, as a depth of liquid over the cell, m: the first liquid fills it.
    pub(super) pd: Vec<f64>,
    /// Reflectance of the ground's surface.
    pub(super) alb: Vec<f64>,
    /// Heat resistance of the soft solid cover (depth / conductivity), m² K / W.
    pub(super) rc: Vec<f64>,
    /// Cells by the head of the liquid that may run off, highest first: the order of the liquid's pass (`liquid`).
    #[serde(skip)]
    pub(super) order: Order,
    /// Air node over each ground cell (radiation and Law 2 read it); the four nearest air nodes and their weights
    /// (`air::lerp`).
    #[serde(skip)]
    pub(super) air_of: Vec<u32>,
    #[serde(skip)]
    pub(super) near: Vec<[u32; 4]>,
    #[serde(skip)]
    pub(super) wt: Vec<[f64; 4]>,
    /// Temperature of the air touching the ground, K (radiation; exchange reads it).
    pub(super) tl: Vec<f64>,
    /// The air each ground cell sees (`air::state`): temperature drop, K/m; mean height of the air,
    /// m; share of heat radiation it absorbs; cloud reflectance (exchange sets it); temperature, K.
    pub(super) seen: Vec<[f64; 5]>,
    /// How much colder than the air touching the ground each layer of the air over a cell is,
    /// [down, up], K (`air::column`).
    pub(super) lay: Vec<[[f64; air::LAYERS]; 2]>,
    /// Temperature of each air node at the last `air::state`, K.
    pub(super) ta0: Vec<f64>,
    /// Mean height of the ground under each air node, m; the air's volume, m³.
    pub(super) za: Vec<f64>,
    pub(super) vol: Vec<f64>,
    /// Amount, mol; heat capacity, J/K; and mass, kg, of each air node (moves keep them).
    pub(super) nt: Vec<f64>,
    pub(super) capa: Vec<f64>,
    pub(super) mass: Vec<f64>,
    /// Optical depth for heat radiation; temperature drop with height, K/m; mean height of the
    /// air, m; temperature, K; of each air node.
    pub(super) fabs: Vec<f64>,
    pub(super) gam: Vec<f64>,
    pub(super) zair: Vec<f64>,
    pub(super) ta: Vec<f64>,
    /// Per gas [g * na + j]: may it condense in the column; height its amount fills near the
    /// ground, m; share of it in the upper half of the column.
    pub(super) cond: Vec<bool>,
    pub(super) hk: Vec<f64>,
    pub(super) split: Vec<f64>,
    /// Work space of radiation (a row per ground cell).
    #[serde(skip)]
    pub(super) rad: Vec<f64>,
    /// Time since trace gases last evaporated and condensed, s, and the calls since.
    pub(super) trace: [f64; 2],
}

impl Scratch {
    /// Temperature of each ground node and of each air node at the last flow step, K (empty before it).
    pub fn temps(&self) -> (&[f64], &[f64]) {
        (&self.tg, &self.ta)
    }
    /// Temperature of ground node `i` at the last flow step, K, and the substance on its melting
    /// plateau with its molten share (none before the first step).
    pub(crate) fn plateau(&self, i: usize) -> Option<(f64, Option<(usize, f64)>)> {
        let t = *self.tg.get(i)?;
        Some((t, (self.pk[i] != NONE).then(|| (self.pk[i], self.phi[i]))))
    }
    /// For Law 2, from the last flow step (`flow::ensure` builds it where Flow has not run).
    pub(crate) fn view(&self) -> FlowView<'_> {
        FlowView { tg: &self.tg, skin: &self.skin, air_of: &self.air_of, head: &self.head, ta: &self.ta, zair: &self.zair, latent: &self.latent, pk: &self.pk }
    }
    /// Loose substances that may be liquid somewhere between `tmin` and `tmax`, K: melting
    /// below the warmest node and boiling above the coldest, or (with `dissolved`) gases that
    /// dissolve in the liquid (`air::dissolve`), which move with it.
    pub(super) fn liquids(&self, chem: &Chemistry, (tmin, tmax): (f64, f64), dissolved: bool) -> Vec<usize> {
        self.loose.iter().copied().filter(|&k| self.sp[k].melt <= tmax && (self.sp[k].boil > tmin || dissolved && chem.subs[k].solubility > 0.0)).collect()
    }
    /// Loose substances solid in some node: melting above the coldest, `tmin`, K.
    pub(super) fn solids(&self, tmin: f64) -> Vec<usize> {
        self.loose.iter().copied().filter(|&k| self.sp[k].melt > tmin).collect()
    }
    /// `k` has come into the loose ground: add it to `loose` and `vary` (at the end of `vary`,
    /// whose order sets the order of sums over it, or `in_order` of melting point).
    pub(super) fn add_loose(&mut self, k: usize, in_order: bool) {
        if self.loose.contains(&k) {
            return;
        }
        let at = self.loose.partition_point(|&x| x < k);
        self.loose.insert(at, k);
        if in_order {
            let at = self.vary.partition_point(|&x| self.sp[x].melt < self.sp[k].melt);
            self.vary.insert(at, k);
        } else {
            self.vary.push(k);
        }
    }
}

/// What may melt or evaporate until the next refresh is judged at the warmest ground node now plus
/// this, K.
const T_MARGIN: f64 = 20.0;
/// A loose substance whose saturation pressure there is over this can evaporate, Pa.
const P_UP: f64 = 1.0;

/// May loose substance `k` evaporate before the next refresh, the warmest ground node being at
/// `tmax` K now (`refresh` and `air::exchange` judge it alike)?
pub(super) fn may_evaporate(w: &World, k: usize, tmax: f64) -> bool {
    w.chem.p_sat(k, tmax + T_MARGIN) > P_UP
}
/// Least bed volume a mean over the bed is taken over, m³ (a node with no bed).
const V_MIN: f64 = 1e-9;
/// Least heat conductivity of a substance, W/(m K) (the soft cover's resistance divides by it).
const COND_MIN: f64 = 1e-3;

/// Every few months: which substances are present, the slow-changing bonded ground and fixed
/// solids, and (once) the state vectors.
pub(super) fn refresh(w: &World, s: &mut Scratch) {
    let (n, na, ns) = (w.grid.n(), w.grid.na(), w.chem.ns());
    let any = |v: &[f64], len: usize, k: usize| v[k * len..(k + 1) * len].iter().any(|&x| x > 0.0);
    let tmax = if s.tg.len() == n { range(&s.tg).1 } else { (0..n).map(|i| w.t_ground(i)).fold(0.0, f64::max) };
    let up = |k: usize| may_evaporate(w, k, tmax);
    s.loose = (0..ns).filter(|&k| any(&w.ground, n, k)).collect();
    s.fixed = s.loose.iter().copied().filter(|&k| w.cfg.phys.soft * s.sp[k].melt > tmax + T_MARGIN && !up(k)).collect();
    s.vary = s.loose.iter().copied().filter(|k| !s.fixed.contains(k)).collect();
    s.vary.sort_by(|&a, &b| s.sp[a].melt.total_cmp(&s.sp[b].melt));
    s.bedk = (0..ns).filter(|&k| any(&w.bed, n, k)).collect();
    let gas = (0..ns).filter(|&k| any(&w.air, na, k) || (s.loose.contains(&k) && up(k))).collect();
    s.stale |= gas != s.gas;
    s.gas = gas;
    s.fix = vec![[0.0; 2]; n];
    for &k in &s.fixed {
        let (v, r) = (s.sp[k].v[0], s.sp[k].refl[0]);
        for (f, &a) in s.fix.iter_mut().zip(&w.ground[k * n..(k + 1) * n]) {
            f[0] += a * v;
            f[1] += a * v * r;
        }
    }
    s.bed = (0..n)
        .map(|i| {
            let (mut v, mut r, mut c) = (0.0, 0.0, 0.0);
            for &k in &s.bedk {
                let (b, p) = (w.bed[k * n + i], &s.sp[k]);
                v += b * p.v[0];
                r += b * p.v[0] * p.refl[0];
                c += b * p.v[0] * p.cond;
            }
            Bed { vol: v, refl: r / v.max(V_MIN), cond: c / v.max(V_MIN) }
        })
        .collect();
    if s.cap.len() == n {
        return;
    }
    (s.pk, s.phi) = (vec![NONE; n], vec![0.0; n]);
    s.latent = vec![Latent { held: 0.0, lo: 0.0, hi: f64::MIN, molten: 0 }; n];
    for v in [&mut s.skin, &mut s.cap, &mut s.tg, &mut s.z, &mut s.head, &mut s.pd, &mut s.alb, &mut s.rc, &mut s.tl] {
        *v = vec![0.0; n];
    }
    for v in [&mut s.za, &mut s.vol, &mut s.nt, &mut s.capa, &mut s.mass, &mut s.fabs, &mut s.gam, &mut s.zair, &mut s.ta, &mut s.ta0] {
        *v = vec![0.0; na];
    }
    s.seen = vec![[0.0; 5]; n];
}

/// Tables that depend on the grid and the chemistry alone: ground cell area, substance constants,
/// and the map from ground cells to air nodes. Rebuilt when missing (they are not saved).
pub(super) fn tables(w: &World, s: &mut Scratch) {
    let (n, g) = (w.grid.n(), &w.grid);
    let mean = |v: [f32; 3]| v.iter().map(|&x| x as f64).sum::<f64>() / 3.0;
    s.area = g.area();
    s.sp = (w.chem.subs.iter())
        .map(|b| Sp {
            cp: b.cp,
            mm: b.molar_mass,
            lf: b.lf,
            lv: b.lv,
            cond: b.conductivity.max(COND_MIN),
            v: [b.molar_mass / b.density[0], b.molar_mass / b.density[1]],
            melt: b.melt,
            boil: b.boil,
            refl: [mean(b.refl[0]), mean(b.refl[1])],
        })
        .collect();
    s.air_of = (0..n).map(|i| g.air_of(i) as u32).collect();
    let axis = |c: usize, len: usize| {
        let f = ((c as f64 + 0.5) / g.air_scale as f64 - 0.5).clamp(0.0, (len - 1) as f64);
        let a = (f as usize).min(len.saturating_sub(2));
        (a, (a + 1).min(len - 1), f - a as f64)
    };
    (s.near, s.wt) = (0..n)
        .map(|i| {
            let (x, y) = g.xy(i);
            let ((x0, x1, tx), (y0, y1, ty)) = (axis(x, g.aw()), axis(y, g.ah()));
            let j = |x: usize, y: usize| (y * g.aw() + x) as u32;
            ([j(x0, y0), j(x1, y0), j(x0, y1), j(x1, y1)], [(1.0 - tx) * (1.0 - ty), tx * (1.0 - ty), (1.0 - tx) * ty, tx * ty])
        })
        .unzip();
}
