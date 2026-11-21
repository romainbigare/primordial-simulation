//! The working memory of Flow (`Scratch`), its tables, and the recheck of what is present (`refresh`).

use super::{air, land::NONE, range};
use crate::{chem::Chemistry, store::Class, AIR, World};
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
    /// The temperature over which its solid is soft cover lying on the loose ground (`Physics::soft` of its melting
    /// point), K
    pub(super) soft: f64,
    /// Is it a mineral of growth (`Chemistry::is_mineral`), which the exchange sites of loose ground hold?
    pub(super) mineral: bool,
}

impl crate::chem::Heat for Sp {
    fn cp(&self) -> f64 {
        self.cp
    }
    fn lf(&self) -> f64 {
        self.lf
    }
    fn lv(&self) -> f64 {
        self.lv
    }
}

impl Sp {
    /// Its liquid share in ground at `t` K where the share `molten` of it is molten and the minerals of growth stay
    /// dissolved by the share `free` (`State::dissolved`): none at or over its boiling point; of a mineral only what
    /// stays dissolved, the rest held on the exchange sites, which neither moves, nor fills, nor takes room (laws.md 4).
    #[inline]
    pub(super) fn liquid(&self, t: f64, molten: f64, free: f64) -> f64 {
        if t >= self.boil {
            0.0
        } else if self.mineral {
            molten * free
        } else {
            molten
        }
    }
    /// The volume a mol of it takes in ground at `t` K (as `liquid`), m³: its liquid share at its liquid's molar volume,
    /// what is not molten at its solid's; none as a gas, nor what the exchange sites hold.
    #[inline]
    pub(super) fn volume(&self, t: f64, molten: f64, free: f64) -> f64 {
        let l = self.liquid(t, molten, free);
        if t >= self.boil { 0.0 } else { (1.0 - molten) * self.v[0] + l * self.v[1] }
    }
}

/// The fusion enthalpy a ground node holds off a plateau, and the temperatures between which it holds it.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Latent {
    /// Fusion enthalpy held by what is molten, J
    pub(super) held: f64,
    /// Off a plateau, the node keeps `held` while its temperature is in [`span[0]`, `span[1]`), K: the melting points
    /// below and above as its last solve found them (none: infinite; empty before its first); and the fusion
    /// enthalpy of the plateaus there, J
    pub(super) span: [f64; 2],
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
    /// Per ground node and per air node: the heads the lifts go between (`Scratch::lift_at`), m
    pub lift_at: [&'a [f64]; 2],
    /// Per air node: its temperature, K
    pub ta: &'a [f64],
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
    /// The share of the pores of ground node `i` its liquid fills (past 1 where liquid stands over them); 0 where it has
    /// none.
    pub(crate) fn filled(&self, i: usize) -> f64 {
        if self.pd[i] > 0.0 { self.liquid_depth(i) / self.pd[i] } else { 0.0 }
    }
    /// The depth of the liquid of ground node `i`, in its pores and standing, m.
    pub(crate) fn liquid_depth(&self, i: usize) -> f64 {
        (self.head[i] - self.z[i]).max(0.0)
    }
    /// How moist ground node `i` is for what lives in it, 0 to 1: the depth of its liquid, in its pores and standing,
    /// as living things feel it (`Physics::wetness`, half at `wet_depth`).
    pub(crate) fn moist(&self, i: usize, p: &crate::Physics) -> f64 {
        p.wetness(self.liquid_depth(i))
    }
    /// Energy to lift a kg from the head of ground node `i` into its air node `j`, J/kg: to the air node's mean ground,
    /// both as the potential energy found them at Flow's last booking (`lift_at`).
    #[inline]
    pub(crate) fn lift(&self, i: usize, j: usize, g: f64) -> f64 {
        g * (self.lift_at[1][j] - self.lift_at[0][i])
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
    /// Of the clouds of each air node: the share of its sky they cover, and their optical depth to sunlight there
    /// (`exchange::cloud`).
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
    /// Of each ground node, the share of the minerals of growth in it that stays dissolved in its liquid, the rest
    /// held on the exchange sites of its loose ground, as the last build of its state found it (`land::dissolved`).
    pub(super) dissolved: Vec<f64>,
    /// Of each ground node whose own ground lies partly under its liquid, how its matter and heat lie between its land
    /// and its liquid (`parts::Parts`); none for a node that is one.
    pub(super) parts: Vec<Option<super::parts::Parts>>,
    /// Of each cell, what its part of each body of standing liquid holds of what stands over its pores, m³, in the
    /// order of its parts (`curve::Curve::ponds`), as the last runoff's pass left it (`runoff::stand`): the share of a
    /// cell's liquid each of its bodies has.
    pub(super) held: Vec<Vec<f64>>,
}

/// Working memory of the law: its saved state (`State`) and caches, rebuilt at each liquid step and when missing
/// (`flow::ensure`). Snapshots are taken at a month's start, where a liquid step comes, so a resumed run matches.
#[derive(Clone, Debug, Default)]
pub struct Scratch {
    pub(super) state: State,
    /// The parts of Flow under way, whose lifts and falls are booked at their end (`potential::Lifts`); whether the
    /// parts follow each other with nothing between (`flow::chain`), booked as one; and the world's time and potential
    /// energy as the last booking left them.
    pub(super) lifts: Option<super::potential::Lifts>,
    pub(super) chained: bool,
    /// The head of each ground cell and the mean head of each air node by the potential energy at Flow's last booking
    /// (`potential`), m: where the gas the laws after Flow lift between a cell and its air node lies (empty before it).
    pub(super) lift_at: [Vec<f64>; 2],
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
    /// How far the solid surface and the head of each ground node moved since its surface was last found, m
    /// (`land::lift`).
    pub(super) shift: Vec<f64>,
    /// Pore space of the loose ground, as a depth of liquid over the cell, m: the first liquid fills it.
    pub(super) pd: Vec<f64>,
    /// Reflectance of the ground's surface.
    pub(super) alb: Vec<f64>,
    /// Of the soft solid cover: heat resistance (depth / conductivity), m² K / W; melting point (infinite with none), K;
    /// depth, m.
    pub(super) cover: Vec<[f64; 3]>,
    /// The gases that dissolve in the ground's liquid this month (`air::henry`), each with its index in `State::gas`.
    pub(super) henry: Vec<(usize, usize)>,
    /// The bodies of standing liquid of the cells and the ways to them (`bodies::Bodies`).
    pub(super) bodies: super::bodies::Bodies,
    /// The bodies as the last full build of the state of ground nodes found them, their levels as the last runoff's pass
    /// left them (`basin::find`).
    pub(super) basin: super::basin::Basin,
    /// The table of the saturation pressure of each substance, as far up as the exchange last asked (`air::Sat`).
    pub(super) sat: Vec<air::Sat>,
    /// The air nodes each ground cell reads, and their weights (`air::lerp`; a slot of no weight reads nothing).
    pub(super) near: Vec<[u32; air::SEES]>,
    pub(super) wt: Vec<[f64; air::SEES]>,
    /// Which cells see each air node and which nodes each node's cells see (`air::Corners`).
    pub(super) corners: air::Corners,
    /// The area of the air each ground cell sees (`air::lerp`), m²; and how much colder than that air at its mean
    /// height the air touching the cell is, K (`air::touching`, at each air state).
    pub(super) seen: Vec<f64>,
    pub(super) cool: Vec<f64>,
    /// What the air of each node does to heat radiation, clear and under its cloud (`air::Skies`).
    pub(super) heat: Vec<air::Skies>,
    /// Mean height of the ground under each air node, m; the air's volume, m³.
    pub(super) za: Vec<f64>,
    pub(super) vol: Vec<f64>,
    /// Amount, mol; heat capacity, J/K; and mass, kg, of each air node (moves keep them).
    pub(super) nt: Vec<f64>,
    pub(super) capa: Vec<f64>,
    pub(super) mass: Vec<f64>,
    /// Of each air node: temperature drop, K/m; mean height, m; temperature, K.
    pub(super) gam: Vec<f64>,
    pub(super) zair: Vec<f64>,
    pub(super) ta: Vec<f64>,
    /// Per gas [g * na + j]: may it condense; the height its amount fills near the ground, m; its share in the upper half.
    pub(super) cond: Vec<bool>,
    pub(super) hk: Vec<f64>,
    pub(super) split: Vec<f64>,
    /// Room the passes keep from step to step (`spare::Spare`).
    pub(super) spare: super::spare::Spare,
    /// The ground of each cell as liquid stands on it, its parts of the bodies (`curve::Curve`, `curve::build`).
    pub(super) curves: Vec<super::curve::Curve>,
    /// Of each mixed ground node, its parts as its last solve found them (`parts::Mixed`).
    pub(super) mixed: Vec<Option<Box<super::parts::Mixed>>>,
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
    /// The height of the solid surface of ground node `i` as Flow's state has it, m (`land::layers`); None before Flow
    /// first built its state.
    pub(crate) fn surface(&self, i: usize) -> Option<f64> {
        self.z.get(i).copied().filter(|_| self.state.tg.len() == self.z.len())
    }
    /// Temperature of ground node `i` at the last flow step, K, and the substance on its plateau with its molten share.
    pub(crate) fn plateau(&self, i: usize) -> Option<(f64, Option<(usize, f64)>)> {
        let t = *self.state.tg.get(i)?;
        Some((t, (self.state.pk[i] != NONE).then(|| (self.state.pk[i], self.state.phi[i]))))
    }
    /// For Law 2, from the last flow step (`flow::ensure` builds it where Flow has not run).
    pub(crate) fn view(&self) -> FlowView<'_> {
        FlowView { tg: &self.state.tg, skin: &self.skin, head: &self.head, lift_at: [&self.lift_at[0], &self.lift_at[1]], ta: &self.ta, latent: &self.latent, pk: &self.state.pk, z: &self.z, pd: &self.pd }
    }
    /// Heat capacity of a ground node or of a part of one with its share `skin` of its bed's skin and the loose amounts
    /// `amt`, mol, J/K (`ground::ground_cap` over `loose`).
    pub(super) fn part_cap(&self, skin: f64, amt: impl Fn(usize) -> f64) -> f64 {
        crate::ground::ground_cap(skin, self.state.loose.iter().copied(), amt, |k| self.sp[k].cp)
    }
    /// Loose substances that may be liquid between `tmin` and `tmax`, K, or (with `dissolved`) gases dissolved in it.
    pub(super) fn liquids(&self, chem: &Chemistry, (tmin, tmax): (f64, f64), dissolved: bool) -> Vec<usize> {
        self.state.loose.iter().copied().filter(|&k| self.sp[k].melt <= tmax && (self.sp[k].boil > tmin || dissolved && chem.subs[k].solubility > 0.0)).collect()
    }
    /// The solids of the loose ground: loose substances solid in some node (melting above the coldest, `tmin`, K) that
    /// are none of the liquids `liq` (`liquids`): a liquid's frozen solid, snow or ice, is no ground.
    pub(super) fn solids(&self, tmin: f64, liq: &[usize]) -> Vec<usize> {
        self.state.loose.iter().copied().filter(|&k| self.sp[k].melt > tmin && !liq.contains(&k)).collect()
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
    s.state.fixed = s.state.loose.iter().copied().filter(|&k| s.sp[k].soft > tmax + T_MARGIN && !up(k)).collect();
    s.state.vary = s.state.loose.iter().copied().filter(|k| !s.state.fixed.contains(k)).collect();
    s.state.vary.sort_by(|&a, &b| s.sp[a].melt.total_cmp(&s.sp[b].melt));
    s.state.bedk = (0..ns).filter(|&k| any(w.store.amounts(Class::Bed), n, k)).collect();
    // A substance the air holds less of than the rounding of its sums is no gas of it: the air's step leaves it be.
    let (air, all) = (w.store.amounts(Class::Air), w.store.amounts(Class::Air).iter().sum::<f64>());
    let in_air = |k: usize| air[k * na..(k + 1) * na].iter().sum::<f64>() > f64::EPSILON * all;
    s.state.gas = (0..ns).filter(|&k| in_air(k) || (s.state.loose.contains(&k) && up(k))).collect();
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
    s.latent = vec![Latent { held: 0.0, span: [f64::INFINITY, f64::NEG_INFINITY], lat: [0.0; 2], molten: 0 }; n];
    s.shift = vec![0.0; n];
    for v in [&mut s.skin, &mut s.cap, &mut s.z, &mut s.head, &mut s.pd, &mut s.alb] {
        *v = vec![0.0; n];
    }
    for v in [&mut s.za, &mut s.vol, &mut s.nt, &mut s.capa, &mut s.mass, &mut s.gam, &mut s.zair, &mut s.ta] {
        *v = vec![0.0; na];
    }
    s.cover = vec![[0.0; 3]; n];
    s.mixed = vec![None; n];
}

/// The numbers of each substance Flow reads often (`Sp`).
pub(super) fn substances(chem: &Chemistry, phys: &crate::Physics) -> Vec<Sp> {
    let mean = |v: [f32; 3]| v.iter().map(|&x| x as f64).sum::<f64>() / 3.0;
    (chem.subs.iter().enumerate())
        .map(|(k, b)| Sp {
            cp: b.cp,
            mm: b.molar_mass,
            lf: b.lf,
            lv: b.lv,
            cond: b.conductivity.max(COND_MIN),
            v: [b.molar_mass / b.density[0], b.molar_mass / b.density[1]],
            melt: b.melt,
            boil: b.boil,
            refl: [mean(b.refl[0]), mean(b.refl[1])],
            soft: phys.soft * b.melt,
            mineral: chem.is_mineral(k),
        })
        .collect()
}

/// Tables of the map and the chemistry alone (not saved): substance constants, and the four air nodes each cell reads.
pub(super) fn tables(w: &World, s: &mut Scratch) {
    s.sp = substances(&w.chem, &w.cfg.phys);
    // A sample of the map (`Square::samples`: a square, or a cell finer than one) reads its air node and the three next
    // to it on its side of the node's centre, bilinearly on the map (round a globe's wrap); along an axis with no node
    // on its side, its own alone, paired with the node on the other side. A cell reads what its samples read, by their
    // areas, so each node's air is read once in all.
    let (mesh, air, samples) = (&w.mesh, w.mesh.level(AIR), w.map.samples());
    let squares = World::mesh_of(&samples, &w.cfg);
    let reads: Vec<([u32; 4], [f64; 4])> = (0..squares.cells().n())
        .map(|i| {
            let p = squares.up(i, 0, AIR);
            let o = squares.on_map((AIR, p), (0, i));
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
        .collect();
    (s.near, s.wt) = (0..mesh.cells().n()).map(|i| ([mesh.up(i, 0, AIR) as u32; air::SEES], [0.0; air::SEES])).unzip();
    let mut used = vec![0; mesh.cells().n()];
    for (sq, (nodes, wts)) in reads.iter().enumerate() {
        let i = mesh.tree().cell_at(samples.leaves()[sq].corner());
        let (near, wt, share) = (&mut s.near[i], &mut s.wt[i], squares.cells().area()[sq] / mesh.cells().area()[i]);
        for (&q, &x) in nodes.iter().zip(wts).filter(|r| *r.1 > 0.0) {
            let c = (0..used[i]).find(|&c| near[c] == q).unwrap_or(used[i]);
            assert!(c < air::SEES, "a cell reads at most `SEES` air nodes");
            (near[c], wt[c], used[i]) = (q, wt[c] + share * x, used[i].max(c + 1));
        }
    }
    s.seen = (0..mesh.cells().n()).map(|i| air::lerp(s, air.area(), i)).collect();
    s.corners = air::Corners::of(air, &s.near, &s.wt);
}
