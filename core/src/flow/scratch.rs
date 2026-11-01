//! The working memory of Flow (`Scratch`), its tables, and the recheck of what is present (`refresh`).

use super::{air, land::NONE, range};
use crate::{chem::Chemistry, solve::downhill::Order, store::Class, AIR, World};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Numbers of one substance Flow reads often (from `chem::Substance`).
#[derive(Clone, Copy, Debug, Default)]
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

/// The fusion enthalpy a ground node holds off a plateau, and the energies between which it holds it.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Latent {
    /// Fusion enthalpy held by what is molten, J
    pub(super) held: f64,
    /// The node keeps `held` while its energy is in [`lo`, `hi`), J (`hi` is `f64::MIN` on a plateau or when stale), and
    /// the fusion enthalpy of the plateaus there, J
    pub(super) lo: f64,
    pub(super) hi: f64,
    pub(super) lat: [f64; 2],
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
    /// Per ground node: its temperature, K; the heat capacity of the skin of its bed, J/K; the head of its liquid, m
    pub tg: &'a [f64],
    pub skin: &'a [f64],
    pub head: &'a [f64],
    /// Per air node: its temperature, K; the mean height of its ground, m
    pub ta: &'a [f64],
    pub za: &'a [f64],
    /// Per ground node: the fusion enthalpy held off a plateau, and the substance on its plateau (or `NONE`)
    latent: &'a [Latent],
    pk: &'a [usize],
    /// Per ground node: the height of its solid surface, m, and its pore space as a depth of liquid, m
    z: &'a [f64],
    pd: &'a [f64],
}

impl FlowView<'_> {
    /// Depth of the liquid standing on ground node `i` over what its pores hold, m (`land::stand`), and the height of
    /// its solid surface, m.
    #[inline]
    pub(crate) fn standing(&self, i: usize) -> (f64, f64) {
        ((self.head[i] - self.pd[i] - self.z[i]).max(0.0), self.z[i])
    }
    /// How moist ground node `i` is for what lives in it, 0 to 1: the depth of its liquid, in its pores and standing,
    /// as living things feel it (`Physics::wetness`, half at `wet_depth`).
    pub(crate) fn moist(&self, i: usize, p: &crate::Physics) -> f64 {
        p.wetness((self.head[i] - self.z[i]).max(0.0))
    }
    /// Energy to lift a kg from the head of ground node `i` into its air node `j`, J/kg: to the air node's mean ground
    /// (`potential`).
    #[inline]
    pub(crate) fn lift(&self, i: usize, j: usize, g: f64) -> f64 {
        g * (self.za[j] - self.head[i])
    }
    /// Fusion enthalpy ground node `i` holds at energy `e`, J, by Flow's last solve of its plateaus: off a plateau what is
    /// molten; on one, also what the energy above the plateau at heat capacity `c` melts of `lat(k)`.
    #[inline]
    pub(crate) fn fused(&self, i: usize, e: f64, c: f64, lat: impl Fn(usize) -> f64) -> f64 {
        let held = self.latent[i].held;
        match self.pk[i] {
            NONE => held,
            k => held + (e - c * self.tg[i] - held).clamp(0.0, lat(k)),
        }
    }
}

/// What Flow keeps from one step to a later one, saved with the world: the plateaus of its ground nodes (the one ground
/// temperature, `World::t_ground_last`), the clouds, and what its last refresh found present.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct State {
    /// Temperature of each ground node, K.
    pub(super) tg: Vec<f64>,
    /// Substance held at its melting point in each ground node (or `NONE`), and its molten share.
    pub(super) pk: Vec<usize>,
    pub(super) phi: Vec<f64>,
    /// Of the clouds of each air node: their reflectance, and their optical depth to heat radiation.
    pub(super) cloud: Vec<[f64; 2]>,
    /// Substances present in loose ground, in order (`refresh`; exchange and liquid add new ones).
    pub(super) loose: Vec<usize>,
    /// Loose substances that stay solid and cannot evaporate, and the others in order of melting point.
    pub(super) fixed: Vec<usize>,
    pub(super) vary: Vec<usize>,
    /// Volume of the `fixed` solids of each ground node, and volume × reflectance, m³.
    pub(super) fix: Vec<[f64; 2]>,
    /// Substances present in bonded ground, and in the air (more than the rounding of its sums, or able to evaporate into it).
    pub(super) bedk: Vec<usize>,
    pub(super) gas: Vec<usize>,
    /// Bonded ground of each ground node.
    pub(super) bed: Vec<Bed>,
}

/// Working memory of the law: its saved state (`State`) and caches, rebuilt at each liquid step and when missing
/// (`flow::ensure`). Snapshots are taken at a month's start, where a liquid step comes, so a resumed run matches.
#[derive(Clone, Debug, Default)]
pub struct Scratch {
    pub(super) state: State,
    /// The substances present changed: rebuild the state of ground and air at the next step.
    pub(super) stale: bool,
    /// The part of Flow under way, whose lifts and falls are booked at its end (`potential::Lifts`); whether the parts
    /// follow each other with nothing between (`flow::chain`); and the world's time and potential energy as the last
    /// part left them.
    pub(super) lifts: Option<super::potential::Lifts>,
    pub(super) chained: bool,
    pub(super) known: Option<(u64, super::potential::Potential)>,
    /// Room for the moves of a part of Flow (`potential::Lifts`), kept from part to part.
    pub(super) moved: Vec<super::potential::Moved>,
    /// The time of the last full build of the state of ground nodes, s.
    pub(super) full_at: Option<u64>,
    /// The time the state of ground nodes was last brought up to date, s (`flow::ground_now`).
    pub(super) built_at: Option<u64>,
    /// The numbers of each substance.
    pub(super) sp: Vec<Sp>,
    /// Heat capacity of the skin of each ground node's bed, J/K (at every step, `land::ground_state`).
    pub(super) skin: Vec<f64>,
    /// Heat capacity of each ground node, J/K (from the amounts at every step; moves keep it).
    pub(super) cap: Vec<f64>,
    /// Fusion enthalpy held off a plateau, per ground node.
    pub(super) latent: Vec<Latent>,
    /// Height of the solid surface, and of the surface of all the liquid (pores filled first), m.
    pub(super) z: Vec<f64>,
    pub(super) head: Vec<f64>,
    /// Pore space of the loose ground, as a depth of liquid over the cell, m: the first liquid fills it.
    pub(super) pd: Vec<f64>,
    /// Reflectance of the ground's surface.
    pub(super) alb: Vec<f64>,
    /// Of the soft solid cover: heat resistance (depth / conductivity), m² K / W; melting point (infinite with none), K;
    /// depth, m.
    pub(super) cover: Vec<[f64; 3]>,
    /// The order of the liquid's pass, by a priority flood over the ground (`liquid`).
    pub(super) order: Order,
    /// The basin, as the last full build of the state of ground nodes found it (`basin::find`).
    pub(super) basin: super::basin::Basin,
    /// The table of the saturation pressure of each substance, as far up as the exchange has asked (`air::Sat`).
    pub(super) sat: Vec<air::Sat>,
    /// The four nearest air nodes of each ground cell on the square map, and their weights (`air::lerp`).
    pub(super) near: Vec<[u32; 4]>,
    pub(super) wt: Vec<[f64; 4]>,
    /// How much colder than the air at its mean ground each layer of an air node is, [down, up], K (`air::column`), and
    /// the share of heat radiation its absorber takes.
    pub(super) lay: Vec<[[f64; air::LAYERS]; 2]>,
    pub(super) absorbs: Vec<f64>,
    /// Mean height of the ground under each air node, m; the air's volume, m³.
    pub(super) za: Vec<f64>,
    pub(super) vol: Vec<f64>,
    /// Amount, mol; heat capacity, J/K; and mass, kg, of each air node (moves keep them).
    pub(super) nt: Vec<f64>,
    pub(super) capa: Vec<f64>,
    pub(super) mass: Vec<f64>,
    /// Of each air node: optical depth of its gases for heat radiation; temperature drop, K/m; mean height, m; temperature, K.
    pub(super) fabs: Vec<f64>,
    pub(super) gam: Vec<f64>,
    pub(super) zair: Vec<f64>,
    pub(super) ta: Vec<f64>,
    /// Per gas [g * na + j]: may it condense; the height its amount fills near the ground, m; its share in the upper half.
    pub(super) cond: Vec<bool>,
    pub(super) hk: Vec<f64>,
    pub(super) split: Vec<f64>,
}

/// Only the state is saved (`Scratch`).
impl Serialize for Scratch {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        self.state.serialize(to)
    }
}

/// A loaded world has the state and no cache.
impl<'de> Deserialize<'de> for Scratch {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        Ok(Scratch { state: State::deserialize(from)?, ..Default::default() })
    }
}

impl Scratch {
    /// Temperature of ground node `i` at the last flow step, K, and the substance on its plateau with its molten share.
    pub(crate) fn plateau(&self, i: usize) -> Option<(f64, Option<(usize, f64)>)> {
        let t = *self.state.tg.get(i)?;
        Some((t, (self.state.pk[i] != NONE).then(|| (self.state.pk[i], self.state.phi[i]))))
    }
    /// For Law 2, from the last flow step (`flow::ensure` builds it where Flow has not run).
    pub(crate) fn view(&self) -> FlowView<'_> {
        FlowView { tg: &self.state.tg, skin: &self.skin, head: &self.head, ta: &self.ta, za: &self.za, latent: &self.latent, pk: &self.state.pk, z: &self.z, pd: &self.pd }
    }
    /// Loose substances that may be liquid between `tmin` and `tmax`, K, or (with `dissolved`) gases dissolved in it.
    pub(super) fn liquids(&self, chem: &Chemistry, (tmin, tmax): (f64, f64), dissolved: bool) -> Vec<usize> {
        self.state.loose.iter().copied().filter(|&k| self.sp[k].melt <= tmax && (self.sp[k].boil > tmin || dissolved && chem.subs[k].solubility > 0.0)).collect()
    }
    /// Loose substances solid in some node: melting above the coldest, `tmin`, K.
    pub(super) fn solids(&self, tmin: f64) -> Vec<usize> {
        self.state.loose.iter().copied().filter(|&k| self.sp[k].melt > tmin).collect()
    }
    /// `k` has come into the loose ground: add it to `loose` and to `vary` (at its end, which keeps the order of sums,
    /// or `in_order` of melting point).
    pub(super) fn add_loose(&mut self, k: usize, in_order: bool) {
        if self.state.loose.contains(&k) {
            return;
        }
        let at = self.state.loose.partition_point(|&x| x < k);
        self.state.loose.insert(at, k);
        if in_order {
            let at = self.state.vary.partition_point(|&x| self.sp[x].melt < self.sp[k].melt);
            self.state.vary.insert(at, k);
        } else {
            self.state.vary.push(k);
        }
    }
}

/// What may melt or evaporate until the next refresh is judged at the warmest ground node plus this, K.
const T_MARGIN: f64 = 20.0;
/// A loose substance whose saturation pressure there is over this can evaporate, Pa.
const P_UP: f64 = 1.0;

/// May loose substance `k` evaporate before the next refresh, the warmest ground node at `tmax` K?
pub(super) fn may_evaporate(w: &World, k: usize, tmax: f64) -> bool {
    w.chem.p_sat(k, tmax + T_MARGIN) > P_UP
}
/// Least bed volume a mean over the bed is taken over, m³ (a node with no bed).
const V_MIN: f64 = 1e-9;
/// Least heat conductivity of a substance, W/(m K) (the soft cover's resistance divides by it).
const COND_MIN: f64 = 1e-3;

/// Every few months: which substances are present, the slow bonded ground and the fixed solids.
pub(super) fn refresh(w: &World, s: &mut Scratch) {
    let (n, na, ns) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.chem.ns());
    let any = |v: &[f64], len: usize, k: usize| v[k * len..(k + 1) * len].iter().any(|&x| x > 0.0);
    let tmax = if s.state.tg.len() == n { range(&s.state.tg).1 } else { (0..n).map(|i| w.t_ground(i)).fold(0.0, f64::max) };
    let up = |k: usize| may_evaporate(w, k, tmax);
    s.state.loose = (0..ns).filter(|&k| any(w.store.amounts(Class::Ground), n, k)).collect();
    s.state.fixed = s.state.loose.iter().copied().filter(|&k| w.cfg.phys.soft * s.sp[k].melt > tmax + T_MARGIN && !up(k)).collect();
    s.state.vary = s.state.loose.iter().copied().filter(|k| !s.state.fixed.contains(k)).collect();
    s.state.vary.sort_by(|&a, &b| s.sp[a].melt.total_cmp(&s.sp[b].melt));
    s.state.bedk = (0..ns).filter(|&k| any(w.store.amounts(Class::Bed), n, k)).collect();
    // A substance the air holds less of than the rounding of its sums is no gas of it: the air's step leaves it be.
    let (air, all) = (w.store.amounts(Class::Air), w.store.amounts(Class::Air).iter().sum::<f64>());
    let in_air = |k: usize| air[k * na..(k + 1) * na].iter().sum::<f64>() > f64::EPSILON * all;
    let gas = (0..ns).filter(|&k| in_air(k) || (s.state.loose.contains(&k) && up(k))).collect();
    s.stale |= gas != s.state.gas;
    s.state.gas = gas;
    s.state.fix = vec![[0.0; 2]; n];
    for &k in &s.state.fixed {
        let (v, r) = (s.sp[k].v[0], s.sp[k].refl[0]);
        for (f, &a) in s.state.fix.iter_mut().zip(&w.store.amounts(Class::Ground)[k * n..(k + 1) * n]) {
            f[0] += a * v;
            f[1] += a * v * r;
        }
    }
    s.state.bed = (0..n)
        .map(|i| {
            let (mut v, mut r, mut c) = (0.0, 0.0, 0.0);
            for &k in &s.state.bedk {
                let (b, p) = (w.store.amounts(Class::Bed)[k * n + i], &s.sp[k]);
                (v, r, c) = (v + b * p.v[0], r + b * p.v[0] * p.refl[0], c + b * p.v[0] * p.cond);
            }
            Bed { vol: v, refl: r / v.max(V_MIN), cond: c / v.max(V_MIN) }
        })
        .collect();
}

/// Room for the caches of every ground and air node (`ensure` then builds them).
pub(super) fn room(w: &World, s: &mut Scratch) {
    let (n, na) = (w.mesh.cells().n(), w.mesh.level(AIR).n());
    s.latent = vec![Latent { held: 0.0, lo: 0.0, hi: f64::MIN, lat: [0.0; 2], molten: 0 }; n];
    for v in [&mut s.skin, &mut s.cap, &mut s.z, &mut s.head, &mut s.pd, &mut s.alb] {
        *v = vec![0.0; n];
    }
    for v in [&mut s.za, &mut s.vol, &mut s.nt, &mut s.capa, &mut s.mass, &mut s.fabs, &mut s.gam, &mut s.zair, &mut s.ta] {
        *v = vec![0.0; na];
    }
    s.cover = vec![[0.0; 3]; n];
}

/// Tables of the map and the chemistry alone (not saved): substance constants, and the four air nodes each cell reads.
pub(super) fn tables(w: &World, s: &mut Scratch) {
    let mean = |v: [f32; 3]| v.iter().map(|&x| x as f64).sum::<f64>() / 3.0;
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
    // A cell reads its air node and the three next to it on its side of the node's centre, bilinearly on the map (round
    // a globe's wrap); along an axis with no node on its side, its own alone, paired with the node on the other side.
    let (mesh, air) = (&w.mesh, w.mesh.level(AIR));
    (s.near, s.wt) = (0..mesh.cells().n())
        .map(|i| {
            let p = mesh.up(i, 0, AIR);
            let o = mesh.on_map((AIR, p), (0, i));
            // The node next to `p` in compass direction `d` (none: `p`).
            let node = |d: Option<usize>| d.and_then(|d| air.way(p, d)).map_or(p, |e| air.adj_cell()[e] as usize);
            // Along axis `x` (0 east, 1 south): the directions from `p` of the nodes before and after the cell (none:
            // `p`), and the share of the way from the first to the second that the cell lies at.
            let axis = |x: usize| {
                let (ahead, back) = (2 * x, 2 * x + 4);
                let part = |d: usize| o[x].abs() / mesh.on_map((AIR, p), (AIR, node(Some(d))))[x].abs();
                match (o[x] >= 0.0, air.way(p, ahead).is_some(), air.way(p, back).is_some()) {
                    (true, true, _) => ([None, Some(ahead)], part(ahead)),
                    (true, false, true) => ([Some(back), None], 1.0),
                    (false, _, true) => ([Some(back), None], 1.0 - part(back)),
                    (false, true, false) => ([None, Some(ahead)], 0.0),
                    _ => ([None, None], 0.0),
                }
            };
            let ((xs, tx), (ys, ty)) = (axis(0), axis(1));
            let at = |ew: Option<usize>, ns: Option<usize>| node(match (ew, ns) { (Some(0), Some(6)) => Some(7), (Some(a), Some(b)) => Some((a + b) / 2), (a, b) => a.or(b) }) as u32;
            ([at(xs[0], ys[0]), at(xs[1], ys[0]), at(xs[0], ys[1]), at(xs[1], ys[1])], [(1.0 - tx) * (1.0 - ty), tx * (1.0 - ty), (1.0 - tx) * ty, tx * ty])
        })
        .unzip();
}
