//! Radiation: sunlight, the heat radiation of the ground and the layers of the air, and contact heat between the
//! ground and the air touching it, in the balance of a ground node's surface over a step (`Surface`).

use super::super::land::NONE;
use super::surf::Surf;
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

/// The slope of `glow` with `tl`, W/(m² K).
pub(super) fn glow_slope(f: f64, tl: f64, lay: &[f64; LAYERS]) -> f64 {
    let k = 4.0 * SIGMA * f / LAYERS as f64;
    lay.iter().map(|&d| k * (tl + d).max(1.0).powi(3)).sum()
}

/// The air touching a ground node over a step: its temperature, K; the share of heat radiation it absorbs; what its
/// layers send down (`glow`), W/m²; and the slope of that with its temperature (`glow_slope`), W/(m² K).
pub(super) struct Air {
    pub(super) tl: f64,
    pub(super) f: f64,
    pub(super) down: f64,
    pub(super) slope: f64,
}

impl Air {
    /// How the balance `bal` of the ground `g` over `dt` seconds moves, to first order on the branch it ended on, with
    /// a change `dtl` of the air's temperature, K, and `dgain` of the gas's heat at its end temperature, W/m²: its end
    /// temperature, K, and the heat it gives its air node and sends to space, J (the implicit function theorem on
    /// `balance`, whose fluxes are linear in the end temperature).
    pub(super) fn moves(&self, g: &Surf, dt: f64, sensible: f64, bal: &Balance, (dtl, dgain): (f64, f64)) -> [f64; 3] {
        let (adt, t, f) = (g.area * dt, g.tg, self.f);
        let (t4, by_air) = (4.0 * SIGMA * t * t * t, (sensible + self.slope) * dtl);
        let dts = bal.give * (by_air + dgain);
        [dts, adt * ((f * t4 + sensible) * dts - by_air), adt * (1.0 - f) * t4 * dts]
    }
}

/// What a ground node's surface balance reads, per m²: shortwave absorbed, W/m²; the air touching it; and the heat of
/// the gas it exchanges, linearised about `t0` (`gain`, `slope`).
pub(super) struct Surface<'a> {
    pub(super) sw: f64,
    pub(super) air: &'a Air,
    pub(super) t0: f64,
    pub(super) gain: f64,
    pub(super) slope: f64,
}

/// What the balance gives: the surface's end temperature, K; its response to a change of what reaches it, K m²/W (on the
/// branch it ended on: none at the cover's cap or on a bare plateau, by the shares of the step on and past a plateau);
/// the heat the node gives its air node, J (sensible heat and the heat radiation the air absorbs, less what
/// the air sends down); and the heat it sends to space, J.
pub(super) struct Balance {
    pub(super) ts: f64,
    pub(super) give: f64,
    pub(super) to_air: f64,
    pub(super) to_space: f64,
}

impl Surface<'_> {
    /// The balance of the ground `g` over `dt` seconds: σT⁴ out, the air's layers back (`column`), contact heat
    /// (`sensible`) and the gas's heat; solved implicitly about the temperature before the step, the node's heat capacity
    /// in series with any soft cover, whose top melts. A melting point the node is at or reaches in the step holds it
    /// while the fusion enthalpy there lasts, `Surf::fusion` on its plateau (`plateau`), so a long step freezes and
    /// thaws as short ones do.
    pub(super) fn balance(&self, g: &Surf, dt: f64, sensible: f64) -> Balance {
        let (adt, t, tl, f, down, fusion) = (g.area * dt, g.tg, self.air.tl, self.air.f, self.air.down, g.fusion);
        let t3 = SIGMA * t * t * t;
        let num = self.sw + down + 3.0 * t3 * t + sensible * tl + self.gain - self.slope * self.t0;
        let den = 4.0 * t3 + sensible - self.slope;
        // The surface's end temperature with the node from `tn` behind a resistance `r`; held at `tn` where `r` is too
        // small to change it (`r den` below the last digit, where `1 / r` may overflow). The surface of a soft cover
        // warms no further than the cover's melting point `cap`: past it the top melts and the melt soaks in, so the
        // node takes all the heat that reaches the surface.
        let [rc, tc, _] = g.cover;
        // With it, its slopes with `num` and with `r`.
        let surface = |tn: f64, r: f64, cap: f64| {
            let by_r = (num - den * tn) / (1.0 + r * den).powi(2);
            let at = if r * den > f64::EPSILON { [(tn / r + num) / (1.0 / r + den), 1.0 / (1.0 / r + den), by_r] } else { [tn, 0.0, by_r] };
            if at[0] > cap { [cap, 0.0, 0.0] } else { at }
        };
        let flux = |ts: f64| (t3 * (4.0 * ts - 3.0 * t), sensible * (ts - tl));
        // The heat the node gets over the step with the surface at `ts`, J.
        let net = |ts: f64| {
            let (up, hs) = flux(ts);
            adt * (self.sw + down - up - hs + self.gain + self.slope * (ts - self.t0))
        };
        let (c, free) = (g.cap.max(1.0), adt / g.cap + rc);
        // On the plateau at `tm` the node melts up to `melt` J or freezes up to `freeze` J. Where its heat over the step
        // melts or freezes all of it, it stays at `tm` only for the share `part` of the step that takes (with the heat
        // that brings it from `t` to `tm`), then goes on from `tm` for the rest; the surface is the mean of the two parts
        // by time. So it ends where its energy says, as short steps do. A cover that held the surface at its melting
        // point has then melted: neither its cap nor its resistance stays. Its slope with `num`: more heat ends the
        // plateau sooner (`dpart`), and the rest is longer.
        let plateau = |tm: f64, (melt, freeze): (f64, f64)| {
            let at = surface(tm, rc, tc);
            let q = net(at[0]) - c * (tm - t);
            let through = if q > melt { melt } else if q < -freeze { -freeze } else { return (at[0], at[1]) };
            let part = ((c * (tm - t) + through) / net(at[0])).min(1.0);
            let dpart = if part < 1.0 { -part / net(at[0]) * adt * (1.0 - den * at[1]) } else { 0.0 };
            let (r, cap) = if q > melt && at[0] >= tc { (0.0, f64::INFINITY) } else { (rc, tc) };
            let rest = surface(tm, (1.0 - part) * adt / c + r, cap);
            let d_rest = rest[1] - rest[2] * adt / c * dpart;
            (part * at[0] + (1.0 - part) * rest[0], part * at[1] + (1.0 - part) * d_rest + (at[0] - rest[0]) * dpart)
        };
        let (l, phi) = (g.latent, g.phi);
        let (mut ts, mut give) = if g.pk != NONE { plateau(t, ((1.0 - phi) * fusion, phi * fusion)) } else { (surface(t, free, tc)[0], surface(t, free, tc)[1]) };
        // Off a plateau, the node's end temperature past the melting point below or above it.
        let end = t + net(ts) / c;
        if g.pk == NONE && l.hi != f64::MIN {
            if l.lo > f64::MIN && end < (l.lo - l.held) / c {
                (ts, give) = plateau((l.lo - l.held) / c, (f64::INFINITY, l.lat[0]));
            } else if l.hi < f64::MAX && end > (l.hi - l.held) / c {
                (ts, give) = plateau((l.hi - l.held) / c, (l.lat[1], f64::INFINITY));
            }
        }
        let (up, hs) = flux(ts);
        Balance { ts, give, to_air: adt * (f * up + hs - down), to_space: adt * (1.0 - f) * up }
    }
}
