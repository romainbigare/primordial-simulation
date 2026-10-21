//! Ground nodes: heat capacity and temperature on the melting plateaus (`ground_state`), phases, the surface and
//! heads, and heat conducted sideways (`conduct`).

use super::scratch::{Latent, Scratch, Sp};
use crate::{ground::{melting, Melting}, solve::spread::Net, store::{apply, Class, LinkFlux}, units::{MAX_SUBSTANCES, T_FLOOR}, World};

/// No substance on a melting plateau (`Scratch::pk`).
pub(super) const NONE: usize = usize::MAX;
/// A node whose heat capacity changed by less than this share keeps its state off a plateau.
const CAP_SAME: f64 = 1e-9;
/// Liquid between two nodes goes no more than this share of the way to level (so heads never cross).
pub(super) const HALF_WAY: f64 = 0.5;

/// Molten share of substance `k` in ground node `i` (liquid, or gas above boiling).
#[inline]
pub(super) fn molten(s: &Scratch, k: usize, i: usize) -> f64 {
    if s.state.pk[i] == k { s.state.phi[i] } else { melted(s, k, i) as u8 as f64 }
}

/// Is substance `k` at or over its melting point in ground node `i` (`World::melt`, as of the last `ground_state`)?
#[inline]
pub(super) fn melted(s: &Scratch, k: usize, i: usize) -> bool {
    s.latent[i].molten >> k & 1 == 1
}

/// Liquid share of substance `k` in ground node `i`.
#[inline]
pub(super) fn wet(s: &Scratch, k: usize, i: usize) -> f64 {
    if s.state.tg[i] < s.sp[k].boil { molten(s, k, i) } else { 0.0 }
}

/// Rise of the solid surface and of the head of a ground node for `m` mol of a substance (negative: fall).
#[inline]
pub(super) fn rise(p: &Sp, t: f64, liq: bool, m: f64, area: f64) -> [f64; 2] {
    if t >= p.boil {
        [0.0; 2]
    } else if !liq {
        let d = m * p.v[0] / area;
        [d, d]
    } else {
        [0.0, m * p.v[1] / area]
    }
}

/// Raise (or lower) the surface of ground node `i` by `[solid, head]` m, and mark it for a new surface.
#[inline]
pub(super) fn lift(s: &mut Scratch, i: usize, [dz, dh]: [f64; 2]) {
    s.latent[i].hi = f64::MIN;
    s.z[i] += dz;
    s.head[i] += dh;
}

impl Scratch {
    /// Ground node `i`, of `area` m², gained `m` mol of `k` (negative: lost): its heat capacity, fusion enthalpy and
    /// surface follow.
    #[inline]
    pub(super) fn note(&mut self, k: usize, i: usize, m: f64, area: f64) {
        self.hold(k, i, m);
        let d = rise(&self.sp[k], self.state.tg[i], melted(self, k, i), m, area);
        lift(self, i, d);
    }
    /// `note` but for the surface: for a gas dissolved in the liquid, which takes no room.
    #[inline]
    pub(super) fn hold(&mut self, k: usize, i: usize, m: f64) {
        self.cap[i] += m * self.sp[k].cp;
        self.latent[i].held += self.fusion(k, i, m);
    }
    /// Most mol of `k` ground node `i` at `t` K can boil with its heat over `k`'s boiling point, cooling to it: each
    /// takes its vaporisation enthalpy and its heat over that point.
    #[inline]
    pub(super) fn boil_off(&self, k: usize, i: usize, t: f64) -> f64 {
        let p = &self.sp[k];
        self.cap[i].max(1.0) * (t - p.boil) / (p.cp * (t - p.boil) + p.lv)
    }
    /// Fusion enthalpy `m` mol of `k` hold in ground node `i`, J: none where `k` is solid or on its plateau.
    #[inline]
    pub(super) fn fusion(&self, k: usize, i: usize, m: f64) -> f64 {
        if self.state.pk[i] != k && melted(self, k, i) { m * self.sp[k].lf } else { 0.0 }
    }
}

/// Solid amount of `k` in ground node `i`, mol.
#[inline]
pub(super) fn solid(w: &World, s: &Scratch, k: usize, i: usize) -> f64 {
    w.store.amounts(Class::Ground)[k * w.mesh.cells().n() + i] * (1.0 - molten(s, k, i))
}

/// Temperature of ground node `i` now, K, no warmer than at the last `ground_state`.
#[inline]
pub(super) fn t_now(w: &World, s: &Scratch, i: usize) -> f64 {
    s.state.tg[i].min((w.store.heat(Class::Ground)[i] - s.latent[i].held) / s.cap[i].max(1.0)).max(T_FLOOR)
}

/// Heat capacity, temperature, surface, head and reflectance of every ground node; between full steps only nodes whose
/// phases change get a new surface.
pub(super) fn ground_state(w: &World, s: &mut Scratch, full: bool) {
    // Heat capacity from the amounts now: other laws move matter between liquid steps, and a stale capacity would read a
    // node far too hot or cold.
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area());
    s.skin = (0..n).map(|i| crate::ground::skin_cap(&w.chem, &w.cfg, area[i], s.state.bedk.iter().copied(), |k| w.store.amounts(Class::Bed)[k * n + i])).collect();
    let mut cap = s.skin.clone();
    for &k in &s.state.loose {
        let cp = s.sp[k].cp;
        cap.iter_mut().zip(&w.store.amounts(Class::Ground)[k * n..(k + 1) * n]).for_each(|(c, &a)| *c += a * cp);
    }
    for (i, &ci) in cap.iter().enumerate().take(n) {
        let same = (ci - s.cap[i]).abs() <= CAP_SAME * ci;
        s.cap[i] = ci;
        let (c, e, Latent { held, lo, hi, molten: was, .. }) = (s.cap[i].max(1.0), w.store.heat(Class::Ground)[i], s.latent[i]);
        if !full && same && e >= lo && e < hi {
            s.state.tg[i] = (e - held) / c;
            continue;
        }
        let pk = s.state.pk[i];
        let th = thaw_at(w, s, c, e, |k| w.store.amounts(Class::Ground)[k * n + i]);
        (s.state.pk[i], s.state.phi[i]) = th.on.unwrap_or((NONE, s.state.phi[i]));
        s.latent[i] = Latent { held: th.held, lo: th.lo, hi: th.hi, lat: th.lat, molten: th.bits };
        s.state.tg[i] = th.t;
        if full || pk != NONE || s.state.pk[i] != NONE || th.bits != was {
            surface(w, s, i);
        }
    }
}

/// Where a ground node of heat capacity `c`, J/K, energy `e`, J, and amounts `amt(k)`, mol, stands on the melting
/// plateaus of the substances that vary (`ground::melting`), each at its point lowered by what is dissolved (laws.md 2.5).
pub(super) fn thaw_at(w: &World, s: &Scratch, c: f64, e: f64, amt: impl Fn(usize) -> f64) -> Melting {
    let (mut ord, m) = ([(0.0, 0); MAX_SUBSTANCES], s.state.vary.len());
    for (o, &k) in ord.iter_mut().zip(&s.state.vary) {
        *o = (w.chem.melt_in(k, &amt), k);
    }
    ord[..m].sort_by(|a, b| a.0.total_cmp(&b.0));
    melting(c, e, ord[..m].iter().map(|&(t, k)| (t, k, amt(k) * s.sp[k].lf)))
}

/// Heat conducts sideways through the ground's active layer over `dt` seconds: one backward-Euler step of
/// `solve::spread` on the cells' links, booked as heat-only link fluxes. A link passes the harmonic mean of its nodes'
/// conductivity × thickness (two half cells in series, Patankar 1980, 4.2) times width over length. A node on a plateau
/// takes its share as if it warmed; the next `ground_state` sets its temperature from its energy.
pub(super) fn conduct(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, cells) = (w.mesh.cells().n(), w.mesh.cells());
    // Conductivity × thickness of each node's active layer, W/K: its bed's skin and its loose ground side by side.
    let mut kt: Vec<f64> = s.state.bed.iter().map(|b| b.cond * w.cfg.skin_m).collect();
    for &k in &s.state.loose {
        let p = s.sp[k];
        for (i, x) in kt.iter_mut().enumerate() {
            let a = w.store.amounts(Class::Ground)[k * n + i];
            if a > 0.0 {
                *x += a * ((1.0 - molten(s, k, i)) * p.v[0] + wet(s, k, i) * p.v[1]) * p.cond / cells.area()[i];
            }
        }
    }
    let (mut a, mut b, mut g) = (Vec::new(), Vec::new(), Vec::new());
    for (l, (&i, &j)) in cells.a().iter().zip(cells.b()).enumerate() {
        let (ki, kj) = (kt[i as usize], kt[j as usize]);
        let gl = 2.0 * ki * kj / (ki + kj).max(f64::MIN_POSITIVE) * cells.width()[l] / cells.dist()[l];
        if gl > 0.0 {
            a.push(i);
            b.push(j);
            g.push(gl);
        }
    }
    let (cap, mut t, mut heat) = (s.cap.iter().map(|&c| c.max(1.0)).collect::<Vec<_>>(), s.state.tg.clone(), vec![0.0; a.len()]);
    // At most as many iterations as nodes: the solve stops once converged, in a few where links pass little.
    s.spread.step(&Net { a: &a, b: &b, g: &g, cap: &cap }, &mut t, dt, n, &mut heat);
    apply(w, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &[], mol: &[], heat: &heat }], &[], &[]);
}

/// Solid surface, head, soft cover and reflectance of node `i` from its layers (`layers`); the pores of the loose
/// ground hold the first liquid. A soft cover holds none: its pores close as it settles, as snow becomes ice (Cuffey
/// and Paterson 2010), so the liquid on it stands and runs off.
pub(super) fn surface(w: &World, s: &mut Scratch, i: usize) {
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[i]);
    let (l, z, [rc, tc]) = layers(w, s, i, s.state.tg[i], |k| w.store.amounts(Class::Ground)[k * n + i], |k| molten(s, k, i));
    (s.z[i], s.head[i], s.cover[i], s.alb[i]) = (z, z + l[2] / area, [rc, tc, l[0] / area], reflectance(w, s, i, &l));
    s.pd[i] = w.cfg.phys.porosity * l[4] / area;
}

/// The layers of ground node `i` at temperature `t` with amounts `amt(k)`, of which the share `molten(k)` is molten:
/// the volume and volume × reflectance of each of [soft solid, liquid, loose solid] top down, m³; its solid surface, m;
/// and of its soft cover the heat resistance, m² K/W, and the melting point, K: its solids' melting points (lowered by
/// what is dissolved, `Chemistry::melt_in`), a mean by volume; infinite with no cover.
pub(super) fn layers(w: &World, s: &Scratch, i: usize, t: f64, amt: impl Fn(usize) -> f64, molten: impl Fn(usize) -> f64) -> ([f64; 6], f64, [f64; 2]) {
    let area = w.mesh.cells().area()[i];
    let mut l = [0.0, 0.0, 0.0, 0.0, s.state.fix[i][0], s.state.fix[i][1]];
    let (mut z, mut cover) = (w.base_z + (s.state.bed[i].vol + s.state.fix[i][0]) / area, [0.0; 2]);
    for &k in &s.state.vary {
        let (p, a) = (&s.sp[k], amt(k));
        if a <= 0.0 || t >= p.boil {
            continue;
        }
        let liq = molten(k);
        let vs = a * (1.0 - liq) * p.v[0];
        let o = if t > w.cfg.phys.soft * p.melt { 0 } else { 4 };
        (l[o], l[o + 1]) = (l[o] + vs, l[o + 1] + vs * p.refl[0]);
        (l[2], l[3]) = (l[2] + a * liq * p.v[1], l[3] + a * liq * p.v[1] * p.refl[1]);
        z += vs / area;
        if o == 0 {
            (cover[0], cover[1]) = (cover[0] + vs / area / p.cond, cover[1] + vs * w.chem.melt_in(k, &amt));
        }
    }
    cover[1] = if l[0] > 0.0 { cover[1] / l[0] } else { f64::INFINITY };
    (l, z, cover)
}

/// Reflectance of ground node `i` with `layers` `l`: soft solid, liquid and loose solid, top down, each cover depth /
/// (depth + `hide`) of what lies below, and its bed the rest.
pub(super) fn reflectance(w: &World, s: &Scratch, i: usize, l: &[f64; 6]) -> f64 {
    let area = w.mesh.cells().area()[i];
    let (mut a, mut seen) = (0.0, 1.0);
    for c in 0..3 {
        if l[2 * c] > 0.0 {
            let d = l[2 * c] / area;
            let cover = d / (d + w.cfg.phys.hide[c]);
            a += seen * cover * l[2 * c + 1] / l[2 * c];
            seen *= 1.0 - cover;
        }
    }
    a + seen * s.state.bed[i].refl
}

/// Head of the liquid that stands on node `i` over what its pores hold (the solid surface when none does), m.
#[inline]
pub(super) fn stand(s: &Scratch, i: usize) -> f64 {
    (s.head[i] - s.pd[i]).max(s.z[i])
}

/// Whether the pores of node `i` (porosity `por`) are full: all its other liquid stands on it. Ground with no pores
/// always is (its head may round a hair under its surface).
#[inline]
pub(super) fn filled(s: &Scratch, i: usize, por: f64) -> bool {
    por <= 0.0 || s.head[i] - s.z[i] >= s.pd[i]
}

/// Height of the bottom of the loose ground of node `i` (porosity `por`), m: the depth its pores take as liquid under
/// its soft cover.
#[inline]
pub(super) fn bottom(s: &Scratch, i: usize, por: f64) -> f64 {
    s.z[i] - s.cover[i][2] - if por > 0.0 { s.pd[i] / por } else { 0.0 }
}

/// Head of the liquid table of node `i`, m: with pores not full, the bottom of its loose ground plus the depth they
/// fill; else the head that stands.
#[inline]
pub(super) fn table(s: &Scratch, i: usize, por: f64) -> f64 {
    if filled(s, i, por) { s.head[i] - s.pd[i] } else { bottom(s, i, por) + (s.head[i] - s.z[i]) / por }
}
