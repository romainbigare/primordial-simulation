//! Ground nodes: their heat capacity and temperature on the melting plateaus (`ground_state`),
//! the phase of each substance, the surface and heads, what matter brings into a node, and heat
//! conducted sideways between nodes (`conduct`).

use super::scratch::{Latent, Scratch, Sp};
use crate::solve::spread::{Net, Spread};
use crate::store::{apply, Class, LinkFlux};
use crate::units::{MAX_SUBSTANCES, T_FLOOR};
use crate::World;

/// No substance on a melting plateau (`Scratch::pk`).
pub(super) const NONE: usize = usize::MAX;
/// A node whose heat capacity changed by less than this share keeps its state off a plateau.
const CAP_SAME: f64 = 1e-9;
/// Liquid between two nodes goes no more than this share of the way to level (so heads never cross).
pub(super) const HALF_WAY: f64 = 0.5;

/// Molten share of substance `k` in ground node `i` (liquid, or gas above boiling).
#[inline]
pub(super) fn molten(s: &Scratch, k: usize, i: usize) -> f64 {
    if s.pk[i] == k { s.phi[i] } else { melted(s, k, i) as u8 as f64 }
}

/// Is substance `k` at or over its melting point in ground node `i` (`World::melt`, as of the last `ground_state`)?
#[inline]
pub(super) fn melted(s: &Scratch, k: usize, i: usize) -> bool {
    s.latent[i].molten >> k & 1 == 1
}

/// Liquid share of substance `k` in ground node `i`.
#[inline]
pub(super) fn wet(s: &Scratch, k: usize, i: usize) -> f64 {
    if s.tg[i] < s.sp[k].boil { molten(s, k, i) } else { 0.0 }
}

/// Rise of the solid surface and of the head of a ground node at `t` for `m` mol of a
/// substance, molten there or not (negative `m`: fall).
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

/// Raise (or with negative `m`, lower) the surface of ground node `i`, of `area` m², for `m` mol of `k`.
#[inline]
pub(super) fn settle(s: &mut Scratch, k: usize, i: usize, m: f64, area: f64) {
    let d = rise(&s.sp[k], s.tg[i], melted(s, k, i), m, area);
    lift(s, i, d);
}

impl Scratch {
    /// Ground node `i`, of `area` m², gained `m` mol of `k` (negative `m`: lost): its heat capacity, the fusion enthalpy
    /// it holds there (`fusion`), and the rise of the surface.
    #[inline]
    pub(super) fn note(&mut self, k: usize, i: usize, m: f64, area: f64) {
        self.hold(k, i, m);
        settle(self, k, i, m, area);
    }
    /// `note` but for the surface: for a gas dissolved in the liquid, which takes no room.
    #[inline]
    pub(super) fn hold(&mut self, k: usize, i: usize, m: f64) {
        self.cap[i] += m * self.sp[k].cp;
        self.latent[i].held += self.fusion(k, i, m);
    }
    /// Fusion enthalpy `m` mol of `k` hold in ground node `i`, J: theirs where `k` is molten there,
    /// none where it is solid or on its plateau (whose molten share `phi` holds its own).
    #[inline]
    pub(super) fn fusion(&self, k: usize, i: usize, m: f64) -> f64 {
        if self.pk[i] != k && melted(self, k, i) { m * self.sp[k].lf } else { 0.0 }
    }
}

/// Solid amount of `k` in ground node `i`, mol.
#[inline]
pub(super) fn solid(w: &World, s: &Scratch, k: usize, i: usize) -> f64 {
    w.ground[k * w.mesh.cells().n() + i] * (1.0 - molten(s, k, i))
}

/// Temperature of ground node `i` now, K, no warmer than at the last `ground_state` (`tg`):
/// earlier radiation and evaporation in this call may have cooled it, and liquid from above
/// mixes in at its own temperature.
#[inline]
pub(super) fn t_now(w: &World, s: &Scratch, i: usize) -> f64 {
    s.tg[i].min((w.e_ground[i] - s.latent[i].held) / s.cap[i].max(1.0)).max(T_FLOOR)
}

/// Heat capacity, temperature, surface, head and reflectance of every ground node. Between full
/// steps only nodes whose phases change (on or off a melting plateau, past a melting point) get a new surface.
pub(super) fn ground_state(w: &World, s: &mut Scratch, full: bool) {
    // Heat capacity from the amounts now, at every step, as `World::heat_cap_ground` sums it: bodies and Law 2 move loose
    // matter (with its heat) between liquid steps, liquid wears the bed and bodies dig it, and a stale capacity would read a
    // node far too hot or cold.
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area());
    for (i, skin) in s.skin.iter_mut().enumerate() {
        *skin = crate::ground::skin_cap(&w.chem, &w.cfg, area[i], s.bedk.iter().copied(), |k| w.bed[k * n + i]);
    }
    let mut cap = s.skin.clone();
    for &k in &s.loose {
        let cp = s.sp[k].cp;
        for (c, &a) in cap.iter_mut().zip(&w.ground[k * n..(k + 1) * n]) {
            *c += a * cp;
        }
    }
    // Temperature on the melting plateaus (`ground::melting`): each substance melts at its melting point in the node,
    // lowered by what is dissolved in its liquid (`World::melt`), in the order of those.
    let mut ord = [(0.0, 0); MAX_SUBSTANCES];
    let mix: Vec<Option<Vec<_>>> = s.vary.iter().map(|&k| w.chem.mixes(k).map(|js| js.collect())).collect();
    for (i, &ci) in cap.iter().enumerate().take(n) {
        let same = (ci - s.cap[i]).abs() <= CAP_SAME * ci;
        s.cap[i] = ci;
        let (c, e, Latent { held, lo, hi, molten: was }) = (s.cap[i].max(1.0), w.e_ground[i], s.latent[i]);
        if !full && same && e >= lo && e < hi {
            s.tg[i] = (e - held) / c;
            continue;
        }
        let (pk, m) = (s.pk[i], s.vary.len());
        for ((o, &k), js) in ord.iter_mut().zip(&s.vary).zip(&mix) {
            *o = (js.as_ref().map_or(s.sp[k].melt, |js| w.chem.melt_with(k, |j| w.ground[j * n + i], js.iter().copied())), k);
        }
        ord[..m].sort_by(|a, b| a.0.total_cmp(&b.0));
        let th = crate::ground::melting(c, e, ord[..m].iter().map(|&(t, k)| (t, k, w.ground[k * n + i] * s.sp[k].lf)));
        (s.pk[i], s.phi[i]) = th.on.unwrap_or((NONE, s.phi[i]));
        s.latent[i] = Latent { held: th.held, lo: th.lo, hi: th.hi, molten: th.bits };
        s.tg[i] = th.t;
        if full || pk != NONE || s.pk[i] != NONE || th.bits != was {
            surface(w, s, i);
        }
    }
}

/// Heat conducts sideways through the active layer of the ground over `dt` seconds: one backward-Euler step on the cell
/// level's links (`solve::spread`), booked as heat-only link fluxes (`store::apply`). The potential is each node's
/// temperature of the last `ground_state` and its capacity the heat capacity `cap`. A node on a melting plateau holds
/// its temperature in truth, so within the step it is given its share as if it warmed; the books are the fluxes, and
/// the next `ground_state` sets each node's temperature from its energy, plateaus included. A link passes the harmonic
/// mean of its nodes' conductivity times thickness (two half cells in series, Patankar 1980, 4.2) times its width
/// over its length: a corner-only diagonal (width 0) passes nothing and is left out.
pub(super) fn conduct(w: &mut World, s: &Scratch, dt: f64) {
    let (n, cells) = (w.mesh.cells().n(), w.mesh.cells());
    // Conductivity times thickness of each node's active layer, W/K: the skin of its bed and the solid and liquid
    // of its loose ground, side by side.
    let mut kt: Vec<f64> = s.bed.iter().map(|b| b.cond * w.cfg.skin_m).collect();
    for &k in &s.loose {
        let p = s.sp[k];
        for (i, x) in kt.iter_mut().enumerate() {
            let a = w.ground[k * n + i];
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
    let (cap, mut t, mut heat) = (s.cap.iter().map(|&c| c.max(1.0)).collect::<Vec<_>>(), s.tg.clone(), vec![0.0; a.len()]);
    // At most as many iterations as nodes: the solve stops once converged, in a few where links pass little.
    Spread::default().step(&Net { a: &a, b: &b, g: &g, cap: &cap }, &mut t, dt, n, &mut heat);
    apply(w, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &[], mol: &[], heat: &heat }], &[], &[]);
}

/// Solid surface, head, heat resistance of the soft cover and reflectance of node `i`, from
/// the volume and volume × reflectance of its layers [soft solid, liquid, loose solid], top
/// down: each covers a share depth / (depth + `hide`) of what lies below. A solid is soft over
/// `soft` times its melting point. The pores of the solids hold the first liquid (`pd`).
pub(super) fn surface(w: &World, s: &mut Scratch, i: usize) {
    let (n, area, t) = (w.mesh.cells().n(), w.mesh.cells().area()[i], s.tg[i]);
    let mut l = [0.0, 0.0, 0.0, 0.0, s.fix[i][0], s.fix[i][1]];
    let (mut z, mut rc) = (w.base_z + (s.bed[i].vol + s.fix[i][0]) / area, 0.0);
    for &k in &s.vary {
        let (p, a) = (&s.sp[k], w.ground[k * n + i]);
        if a <= 0.0 || t >= p.boil {
            continue;
        }
        let liq = molten(s, k, i);
        let vs = a * (1.0 - liq) * p.v[0];
        let o = if t > w.cfg.phys.soft * p.melt { 0 } else { 4 };
        (l[o], l[o + 1]) = (l[o] + vs, l[o + 1] + vs * p.refl[0]);
        (l[2], l[3]) = (l[2] + a * liq * p.v[1], l[3] + a * liq * p.v[1] * p.refl[1]);
        z += vs / area;
        if o == 0 {
            rc += vs / area / p.cond;
        }
    }
    let (mut a, mut seen) = (0.0, 1.0);
    for c in 0..3 {
        if l[2 * c] > 0.0 {
            let d = l[2 * c] / area;
            let cover = d / (d + w.cfg.phys.hide[c]);
            a += seen * cover * l[2 * c + 1] / l[2 * c];
            seen *= 1.0 - cover;
        }
    }
    (s.z[i], s.head[i], s.rc[i], s.alb[i]) = (z, z + l[2] / area, rc, a + seen * s.bed[i].refl);
    s.pd[i] = w.cfg.phys.porosity * (l[0] + l[4]) / area;
}

/// Head of the liquid that stands on node `i` (the solid surface when none does), m: the liquid
/// over what the pores hold.
#[inline]
pub(super) fn stand(s: &Scratch, i: usize) -> f64 {
    (s.head[i] - s.pd[i]).max(s.z[i])
}

/// Head of the liquid table of node `i`, m (Darcy's law reads it): in a node whose pores are not
/// full, the bottom of its loose ground plus the depth they fill; else the head that stands.
#[inline]
pub(super) fn table(s: &Scratch, i: usize, por: f64) -> f64 {
    let full = s.head[i] - s.z[i] >= s.pd[i];
    if full { s.head[i] - s.pd[i] } else { s.z[i] - s.pd[i] / por + (s.head[i] - s.z[i]) / por }
}
