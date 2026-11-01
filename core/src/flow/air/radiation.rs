//! Radiation: sunlight, the heat radiation of the ground and the layers of the air, and contact heat between the
//! ground and the air touching it, in the balance of a ground node's surface over a step (`Surface`).

use super::super::{land::NONE, Scratch};
use crate::{math::one_minus_exp, units::SIGMA};
use std::f64::consts::{PI, TAU};

/// The sun's declination at `season` (0..1 from the solstice of the north's winter) on a circular orbit with axial tilt
/// `tilt`, degrees: sin δ = sin ε sin λ, λ the sun's longitude, radians.
pub fn declination(tilt: f64, season: f64) -> f64 {
    (tilt.to_radians().sin() * -(TAU * season).cos()).asin()
}

/// Daily-mean sunlight at the top of the air at latitude `lat` and declination `decl`, radians, W/m² (Berger 1978).
pub fn insolation(s0: f64, lat: f64, decl: f64) -> f64 {
    let h0 = (-lat.tan() * decl.tan()).clamp(-1.0, 1.0).acos();
    (s0 / PI * (h0 * lat.sin() * decl.sin() + lat.cos() * decl.cos() * h0.sin())).max(0.0)
}

/// Layers of the air in `column`.
pub(crate) const LAYERS: usize = 2;

/// The layers of the air over the ground that send heat radiation [down, up], as how much colder than the air at the
/// ground each is, K: a grey absorber of optical depth `x` spread like the air's mass (scale height `h`, column `l`),
/// cooling by `gam` per metre, summed at `LAYERS` points of Schwarzschild's integral. More gas lifts the layers that
/// reach space into colder air, so the ground warms with no ceiling (a single slab would have one).
pub fn column(x: f64, gam: f64, l: f64, h: f64) -> [[f64; LAYERS]; 2] {
    let (f, e, mut out) = (one_minus_exp(x), (-l / h).exp(), [[0.0; LAYERS]; 2]);
    // Drop to the height with a share `above` of the absorber above it.
    let drop = |above: f64| gam * h * (e + (1.0 - e) * above).ln();
    for m in (0..LAYERS).filter(|_| x > 0.0) {
        // Share of the absorber between the layer and the side it sends to.
        let near = -(-f * (m as f64 + 0.5) / LAYERS as f64).ln_1p() / x;
        (out[0][m], out[1][m]) = (drop(1.0 - near), drop(near));
    }
    out
}

/// What layers (`column`) over ground with air at `tl` that absorbs a share `f` send, W/m².
pub fn glow(f: f64, tl: f64, lay: &[f64; LAYERS]) -> f64 {
    let k = SIGMA * f / LAYERS as f64;
    lay.iter().map(|&d| k * (tl + d).max(1.0).powi(4)).sum()
}

/// The air touching a ground node over a step: its temperature, K; the share of heat radiation it absorbs; and what its
/// layers send down (`glow`), W/m².
pub(super) struct Air {
    pub(super) tl: f64,
    pub(super) f: f64,
    pub(super) down: f64,
}

/// What a ground node's surface balance reads, per m²: shortwave absorbed, W/m²; the air touching it; the heat of the
/// gas it exchanges and what its bodies draw, linearised about `t0` (`gain`, `slope`); and rain at `t_rain` through
/// `g_rain`.
pub(super) struct Surface<'a> {
    pub(super) sw: f64,
    pub(super) air: &'a Air,
    pub(super) t0: f64,
    pub(super) gain: f64,
    pub(super) slope: f64,
    pub(super) t_rain: f64,
    pub(super) g_rain: f64,
}

/// What the balance gives: the surface's end temperature, K; the heat the node gives its air node, J (sensible heat and
/// the heat radiation the air absorbs, less what the air sends down); and the heat it sends to space, J.
pub(super) struct Balance {
    pub(super) ts: f64,
    pub(super) to_air: f64,
    pub(super) to_space: f64,
}

impl Surface<'_> {
    /// The balance of ground node `i` over `dt` seconds: σT⁴ out, the air's layers back (`column`), contact heat
    /// (`sensible`) and the gas's heat; solved implicitly about the temperature before the step, the node's heat capacity
    /// in series with any soft cover, whose top melts. A melting point the node is at or reaches in the step holds it
    /// while the fusion enthalpy there lasts, `fusion` on its plateau (`plateau`), so a long step freezes and thaws as
    /// short ones do.
    pub(super) fn balance(&self, s: &Scratch, (i, area, dt): (usize, f64, f64), sensible: f64, fusion: f64) -> Balance {
        let (adt, t, tl, f, down) = (area * dt, s.state.tg[i], self.air.tl, self.air.f, self.air.down);
        let t3 = SIGMA * t * t * t;
        let num = self.sw + down + 3.0 * t3 * t + sensible * tl + self.gain - self.slope * self.t0 + self.g_rain * self.t_rain;
        let den = 4.0 * t3 + sensible + self.g_rain - self.slope;
        // The surface's end temperature with the node from `tn` behind a resistance `r`; held at `tn` where `r` is too
        // small to change it (`r den` below the last digit, where `1 / r` may overflow). The surface of a soft cover
        // warms no further than the cover's melting point `cap`: past it the top melts and the melt soaks in, so the
        // node takes all the heat that reaches the surface.
        let [rc, tc, _] = s.cover[i];
        let surface = |tn: f64, r: f64, cap: f64| (if r * den > f64::EPSILON { (tn / r + num) / (1.0 / r + den) } else { tn }).min(cap);
        let flux = |ts: f64| (t3 * (4.0 * ts - 3.0 * t), sensible * (ts - tl));
        // The heat the node gets over the step with the surface at `ts`, J.
        let net = |ts: f64| {
            let (up, hs) = flux(ts);
            adt * (self.sw + down - up - hs + self.gain + self.slope * (ts - self.t0) + self.g_rain * (self.t_rain - ts))
        };
        let (c, free) = (s.cap[i].max(1.0), adt / s.cap[i] + rc);
        // On the plateau at `tm` the node melts up to `melt` J or freezes up to `freeze` J. Where its heat over the step
        // melts or freezes all of it, it stays at `tm` only for the share `part` of the step that takes (with the heat
        // that brings it from `t` to `tm`), then goes on from `tm` for the rest; the surface is the mean of the two parts
        // by time. So it ends where its energy says, as short steps do. A cover that held the surface at its melting
        // point has then melted: neither its cap nor its resistance stays.
        let plateau = |tm: f64, (melt, freeze): (f64, f64)| {
            let at = surface(tm, rc, tc);
            let q = net(at) - c * (tm - t);
            let through = if q > melt { melt } else if q < -freeze { -freeze } else { return at };
            let part = ((c * (tm - t) + through) / net(at)).min(1.0);
            let (r, cap) = if q > melt && at >= tc { (0.0, f64::INFINITY) } else { (rc, tc) };
            part * at + (1.0 - part) * surface(tm, (1.0 - part) * adt / c + r, cap)
        };
        let (l, phi) = (s.latent[i], s.state.phi[i]);
        let mut ts = if s.state.pk[i] != NONE { plateau(t, ((1.0 - phi) * fusion, phi * fusion)) } else { surface(t, free, tc) };
        // Off a plateau, the node's end temperature past the melting point below or above it.
        let end = t + net(ts) / c;
        if s.state.pk[i] == NONE && l.hi != f64::MIN {
            if l.lo > f64::MIN && end < (l.lo - l.held) / c {
                ts = plateau((l.lo - l.held) / c, (f64::INFINITY, l.lat[0]));
            } else if l.hi < f64::MAX && end > (l.hi - l.held) / c {
                ts = plateau((l.hi - l.held) / c, (l.lat[1], f64::INFINITY));
            }
        }
        let (up, hs) = flux(ts);
        Balance { ts, to_air: adt * (f * up + hs - down), to_space: adt * (1.0 - f) * up }
    }
}
