//! Ground nodes: heat capacity and temperature on the melting plateaus (`ground_state`), phases, the surface and
//! heads, and heat conducted sideways (`conduct`).

use super::{on_land, scratch::{Latent, Scratch, Sp}};
use crate::{ground::{heat_above_floor, melting, Melting}, solve::sweep::{sweep, Net}, store::{apply, Class, LinkFlux}, units::{MAX_SUBSTANCES, T_FLOOR}, World};

/// No substance on a melting plateau (`Scratch::pk`).
pub(super) const NONE: usize = usize::MAX;
/// A node whose heat capacity changed by less than this share keeps its state off a plateau.
const CAP_SAME: f64 = 1e-9;

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
    /// Fusion enthalpy `m` mol of `k` hold in ground node `i`, J: none where `k` is solid or on its plateau.
    #[inline]
    pub(super) fn fusion(&self, k: usize, i: usize, m: f64) -> f64 {
        if self.state.pk[i] != k && melted(self, k, i) { m * self.sp[k].lf } else { 0.0 }
    }
}

/// Most mol of a substance `p` a ground node of heat capacity `cap`, J/K, at `t` K can boil with its heat over the
/// substance's boiling point, cooling to it: each takes its vaporisation enthalpy and its heat over that point.
#[inline]
pub(super) fn boil_off(p: &Sp, cap: f64, t: f64) -> f64 {
    cap.max(1.0) * (t - p.boil) / (p.cp * (t - p.boil) + p.lv)
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

/// Heat capacity, temperature, surface, head and reflectance of every ground node, each in parallel with `threads`
/// (`node_state`); between full steps only nodes whose phases change get a new surface. A full build finds the basin
/// (`basin::find`).
pub(super) fn ground_state(w: &World, s: &mut Scratch, full: bool) {
    let n = w.mesh.cells().n();
    let mut rows = vec![0.0; n * STATE_ROW];
    let sr = &*s;
    crate::par::rows(&mut rows, STATE_ROW, || (), |_, i, row| node_state(w, sr, i, full, row));
    for (i, r) in rows.chunks_exact(STATE_ROW).enumerate() {
        (s.skin[i], s.cap[i], s.state.tg[i]) = (r[0], r[1], r[2]);
        if r[3] != 0.0 {
            (s.state.pk[i], s.state.phi[i]) = (r[4] as usize, r[5]);
            s.latent[i] = Latent { held: r[6], lo: r[7], hi: r[8], lat: [r[9], r[10]], molten: r[11].to_bits() };
        }
        if r[12] != 0.0 {
            (s.z[i], s.head[i], s.cover[i], s.alb[i], s.pd[i]) = (r[13], r[14], [r[15], r[16], r[17]], r[18], r[19]);
        }
    }
    if full {
        s.basin = super::basin::find(w, s);
    }
}

/// The numbers of a node's row in `ground_state`: the heat capacity of its skin and its own, J/K; its temperature, K;
/// whether it was found anew, then its plateau's substance (`NONE` as is) and molten share, and its `Latent` (the bits
/// of what is molten as an f64's bits); whether it has a new surface, then its solid surface and head, m, its soft
/// cover (`Scratch::cover`), its reflectance and its pore space as a depth, m.
const STATE_ROW: usize = 20;

/// The row of ground node `i` (`STATE_ROW`): its heat capacity from the amounts now (other laws move matter between
/// liquid steps, and a stale capacity would read a node far too hot or cold); off its plateaus, where its capacity is
/// the same and its energy in the span that keeps it there, its temperature by its energy alone; else where it stands
/// on its plateaus (`thaw_at`), with a new surface where its phases changed (`surface_of`).
fn node_state(w: &World, s: &Scratch, i: usize, full: bool, row: &mut [f64]) {
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[i]);
    let amt = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
    let skin = crate::ground::skin_cap(&w.chem, &w.cfg, area, s.state.bedk.iter().copied(), |k| w.store.amounts(Class::Bed)[k * n + i]);
    let ci = s.state.loose.iter().fold(skin, |c, &k| c + amt(k) * s.sp[k].cp);
    let same = s.cap.get(i).is_some_and(|&c| (ci - c).abs() <= CAP_SAME * ci);
    let (c, e, Latent { held, lo, hi, molten: was, .. }) = (ci.max(1.0), w.store.heat(Class::Ground)[i], s.latent[i]);
    row[..3].copy_from_slice(&[skin, ci, (e - held) / c]);
    if !full && same && e >= lo && e < hi {
        return;
    }
    let melt = melts(w, s, amt);
    let th = thaw_at(s, c, e, amt, &melt);
    let (pk, phi) = th.on.unwrap_or((NONE, s.state.phi[i]));
    row[2..12].copy_from_slice(&[th.t, 1.0, pk as f64, phi, th.held, th.lo, th.hi, th.lat[0], th.lat[1], f64::from_bits(th.bits)]);
    if full || s.state.pk[i] != NONE || pk != NONE || th.bits != was {
        row[12] = 1.0;
        row[13..].copy_from_slice(&surface_of(w, s, i, th.t, |k| th.molten(k)));
    }
}

/// The melting point of each substance that varies in a ground node of amounts `amt(k)`, mol, lowered by what is
/// dissolved (`Chemistry::melt_in`, laws.md 2.5), K, by substance.
pub(super) fn melts(w: &World, s: &Scratch, amt: impl Fn(usize) -> f64) -> [f64; MAX_SUBSTANCES] {
    let mut melt = [0.0; MAX_SUBSTANCES];
    for &k in &s.state.vary {
        melt[k] = w.chem.melt_in(k, &amt);
    }
    melt
}

/// Where a ground node of heat capacity `c`, J/K, energy `e`, J, and amounts `amt(k)`, mol, stands on the melting
/// plateaus of the substances that vary (`ground::melting`), each at its point `melt[k]`, K (`melts`).
pub(super) fn thaw_at(s: &Scratch, c: f64, e: f64, amt: impl Fn(usize) -> f64, melt: &[f64; MAX_SUBSTANCES]) -> Melting {
    // The substances by their melting points here; a substance's number fits a byte (`MAX_SUBSTANCES`).
    let (mut ord, m) = ([0u8; MAX_SUBSTANCES], s.state.vary.len());
    for (o, &k) in ord.iter_mut().zip(&s.state.vary) {
        *o = k as u8;
    }
    ord[..m].sort_by(|&a, &b| melt[a as usize].total_cmp(&melt[b as usize]));
    melting(c, e, ord[..m].iter().map(|&k| (melt[k as usize], k as usize, amt(k as usize) * s.sp[k as usize].lf)))
}

/// Heat conducts sideways through the ground's active layer over `dt` seconds: one explicit `solve::sweep` on the
/// cells' links (rock passes a cell's heat to its neighbours in about 70,000 years, so a year is far below its limit),
/// booked as heat-only link fluxes. A link passes the harmonic mean of its nodes'
/// conductivity × thickness (two half cells in series, Patankar 1980, 4.2) times width over length. A node on a plateau
/// takes its share as if it warmed; the next `ground_state` sets its temperature from its energy.
pub(super) fn conduct(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, cells) = (w.mesh.cells().n(), w.mesh.cells());
    // Conductivity × thickness of each node's active layer, W/K: its bed's skin and its loose ground side by side; none
    // below 0 m (`on_land`), so heat crosses only links between two land cells.
    let on: Vec<bool> = (0..n).map(|i| on_land(s, i)).collect();
    let mut kt: Vec<f64> = s.state.bed.iter().zip(&on).map(|(b, &on)| if on { b.cond * w.cfg.skin_m } else { 0.0 }).collect();
    for &k in &s.state.loose {
        let p = s.sp[k];
        for (i, x) in kt.iter_mut().enumerate() {
            let a = w.store.amounts(Class::Ground)[k * n + i];
            if a > 0.0 && on[i] {
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
    let cap: Vec<f64> = s.cap.iter().map(|&c| c.max(1.0)).collect();
    let have: Vec<f64> = (0..n).map(|i| heat_above_floor(w.store.heat(Class::Ground)[i], cap[i])).collect();
    let heat = sweep(&Net { a: &a, b: &b, g: &g, cap: &cap }, &s.state.tg, dt, &have);
    apply(w, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &[], mol: &[], heat: &heat }], &[]);
}

/// Solid surface, head, soft cover and reflectance of node `i` at temperature `t` with the share `molten(k)` of each
/// substance molten, from its layers (`layers`): its solid surface and head, m, its soft cover (`Scratch::cover`), its
/// reflectance and its pore space as a depth, m. The pores of the loose ground hold the first liquid. A soft cover holds
/// none: its pores close as it settles, as snow becomes ice (Cuffey and Paterson 2010), so the liquid on it stands and
/// runs off.
fn surface_of(w: &World, s: &Scratch, i: usize, t: f64, molten: impl Fn(usize) -> f64) -> [f64; 7] {
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[i]);
    let amt = |k: usize| w.store.amounts(Class::Ground)[k * n + i];
    let (l, z, [rc, tc]) = layers(w, s, i, t, amt, molten, |k| w.chem.melt_in(k, amt));
    [z, z + l[2] / area, rc, tc, l[0] / area, reflectance(w, s, i, &l), w.cfg.phys.porosity * l[4] / area]
}

/// The layers of ground node `i` at temperature `t` with amounts `amt(k)`, of which the share `molten(k)` is molten:
/// the volume and volume × reflectance of each of [soft solid, liquid, loose solid] top down, m³; its solid surface, m;
/// and of its soft cover the heat resistance, m² K/W, and the melting point, K: its solids' melting points `melt(k)`
/// (lowered by what is dissolved, `Chemistry::melt_in`), a mean by volume; infinite with no cover.
pub(super) fn layers(w: &World, s: &Scratch, i: usize, t: f64, amt: impl Fn(usize) -> f64, molten: impl Fn(usize) -> f64, melt: impl Fn(usize) -> f64) -> ([f64; 6], f64, [f64; 2]) {
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
            (cover[0], cover[1]) = (cover[0] + vs / area / p.cond, cover[1] + vs * melt(k));
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
