use super::scratch::{Scratch, Sp};
use crate::units::MAX_SUBSTANCES;
use crate::World;

pub(super) const NONE: usize = usize::MAX;

/// Molten share of substance `k` in ground node `i` (liquid, or gas above boiling).
#[inline]
pub(super) fn molten(s: &Scratch, k: usize, i: usize) -> f64 {
    if s.pk[i] == k { s.phi[i] } else { melted(s, k, i) as u8 as f64 }
}

/// Is substance `k` at or over its melting point in ground node `i` (`World::melt`, as of the last `ground_state`)?
#[inline]
pub(super) fn melted(s: &Scratch, k: usize, i: usize) -> bool {
    (s.hold[i][3] as u64) >> k & 1 == 1
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
    s.hold[i][2] = f64::MIN;
    s.z[i] += dz;
    s.head[i] += dh;
}

/// Raise (or with negative `m`, lower) the surface of ground node `i` for `m` mol of `k`.
#[inline]
pub(super) fn settle(s: &mut Scratch, k: usize, i: usize, m: f64) {
    let d = rise(&s.sp[k], s.tg[i], melted(s, k, i), m, s.area);
    lift(s, i, d);
}

/// Heat capacity, temperature, surface, head and reflectance of every ground node. Between full
/// steps only nodes whose phases change (on or off a melting plateau, past a melting point) get a new surface.
pub(super) fn ground_state(w: &World, s: &mut Scratch, full: bool) {
    // Heat capacity from the amounts now, at every step: bodies and Law 2 move loose matter
    // (with its heat) between liquid steps, and a stale capacity would read a node far too hot or cold.
    let n = w.grid.n();
    let mut cap: Vec<f64> = s.bed.iter().map(|b| b[0]).collect();
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
        let same = (ci - s.cap[i]).abs() <= 1e-9 * ci;
        s.cap[i] = ci;
        let (c, e, [held, lo, hi, was]) = (s.cap[i].max(1.0), w.e_ground[i], s.hold[i]);
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
        let bits = th.bits;
        s.hold[i] = [th.held, th.lo, th.hi, bits as f64];
        s.tg[i] = th.t;
        if full || pk != NONE || s.pk[i] != NONE || bits as f64 != was {
            surface(w, s, i);
        }
    }
}

/// Solid surface, head, heat resistance of the soft cover and reflectance of node `i`, from
/// the volume and volume × reflectance of its layers [soft solid, liquid, loose solid].
pub(super) fn surface(w: &World, s: &mut Scratch, i: usize) {
    let (n, area, t) = (w.grid.n(), s.area, s.tg[i]);
    let mut l = [0.0, 0.0, 0.0, 0.0, s.fix[i][0], s.fix[i][1]];
    let (mut z, mut rc) = (w.base_z + (s.bed[i][1] + s.fix[i][0]) / area, 0.0);
    for &k in &s.vary {
        let (p, a) = (&s.sp[k], w.ground[k * n + i]);
        if a <= 0.0 || t >= p.boil {
            continue;
        }
        let liq = molten(s, k, i);
        let solid = a * (1.0 - liq) * p.v[0];
        let o = if t > w.cfg.phys.soft * p.melt { 0 } else { 4 };
        (l[o], l[o + 1]) = (l[o] + solid, l[o + 1] + solid * p.refl[0]);
        (l[2], l[3]) = (l[2] + a * liq * p.v[1], l[3] + a * liq * p.v[1] * p.refl[1]);
        z += solid / area;
        if o == 0 {
            rc += solid / area / p.cond;
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
    (s.z[i], s.head[i], s.rc[i], s.alb[i]) = (z, z + l[2] / area, rc, a + seen * s.bed[i][2]);
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
