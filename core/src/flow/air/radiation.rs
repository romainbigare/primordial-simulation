//! Radiation: sunlight, the heat radiation of the ground and the layers of the air, and contact heat between the
//! ground and the air touching it, in the balance of a ground node's surface over a step (`Surface`).

use super::super::land::NONE;
use super::super::Scratch;
use crate::math::one_minus_exp;
use crate::units::SIGMA;
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

/// Layers of the air over a cell in `column`.
pub(crate) const LAYERS: usize = 4;

/// The layers of the air over a cell that send heat radiation [down, up], as how much colder than the air at the
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
    lay.iter().map(|&d| (tl + d).max(1.0).powi(4)).sum::<f64>() * SIGMA * f / LAYERS as f64
}

/// `glow` and its slope with `tl`, W/(m² K).
fn glow_slope(f: f64, tl: f64, lay: &[f64; LAYERS]) -> (f64, f64) {
    let k = SIGMA * f / LAYERS as f64;
    lay.iter().fold((0.0, 0.0), |(g, s), &d| {
        let t = (tl + d).max(1.0);
        (g + k * t.powi(4), s + 4.0 * k * t.powi(3))
    })
}

/// What a ground node's surface balance reads, per m²: shortwave absorbed, W/m²; the air's temperature at the ground,
/// K; the heat of the gas it exchanges, linearised about `t0` (`gain`, `slope`); and rain at `t_rain` through `g_rain`.
pub(super) struct Surface {
    pub(super) sw: f64,
    pub(super) tl: f64,
    pub(super) t0: f64,
    pub(super) gain: f64,
    pub(super) slope: f64,
    pub(super) t_rain: f64,
    pub(super) g_rain: f64,
}

/// What the balance gives: the node's end temperature, K; the heat it gives its air node, J, with its slopes with the
/// air's and the node's temperature, J/K; the node's response per W/m², K m²/W, and per K of the air touching it; the
/// heat to space through the air, J.
pub(super) struct Balance {
    pub(super) ts: f64,
    pub(super) to_air: f64,
    pub(super) slope: f64,
    pub(super) warm: f64,
    pub(super) give: f64,
    pub(super) follow: f64,
    pub(super) to_space: f64,
}

impl Surface {
    /// The balance of ground node `i` over `dt` seconds: σT⁴ out, the air's layers back (`column`), contact heat
    /// (`sensible`) and the gas's heat; solved implicitly about the temperature before the step, the node's heat capacity
    /// in series with any soft cover. A plateau holds the node only while its fusion enthalpy lasts.
    pub(super) fn balance(&self, s: &Scratch, (i, area, dt): (usize, f64, f64), sensible: f64, fusion: f64) -> Balance {
        let (adt, f, t) = (area * dt, s.seen[i][2], s.state.tg[i]);
        let ((down, d_down), (emit, d_emit)) = (glow_slope(f, self.tl, &s.lay[i][0]), glow_slope(f, self.tl, &s.lay[i][1]));
        let t3 = SIGMA * t * t * t;
        let num = self.sw + down + 3.0 * t3 * t + sensible * self.tl + self.gain - self.slope * self.t0 + self.g_rain * self.t_rain;
        let den = 4.0 * t3 + sensible + self.g_rain - self.slope;
        let surface = |r: f64| if r > 0.0 { ((t / r + num) / (1.0 / r + den), 1.0 / (1.0 / r + den)) } else { (t, 0.0) };
        let flux = |ts: f64| (t3 * (4.0 * ts - 3.0 * t), sensible * (ts - self.tl));
        let (plateau, free) = (s.state.pk[i] != NONE, adt / s.cap[i] + s.rc[i]);
        let (mut ts, mut give) = surface(if plateau { s.rc[i] } else { free });
        if plateau {
            let (up, hs) = flux(ts);
            let net = adt * (self.sw + down - up - hs + self.gain + self.slope * (ts - self.t0) + self.g_rain * (self.t_rain - ts));
            if net < -s.state.phi[i] * fusion || net > (1.0 - s.state.phi[i]) * fusion {
                (ts, give) = surface(free);
            }
        }
        // How far the node follows a change of the air touching it, per K.
        let follow = give * (d_down + sensible + self.g_rain);
        let (up, hs) = flux(ts);
        Balance {
            ts,
            to_air: adt * (f * up + hs - down - emit),
            slope: adt * (f * 4.0 * t3 * follow + sensible * (follow - 1.0) - d_down - d_emit),
            warm: adt * (f * 4.0 * t3 + sensible),
            give,
            follow,
            to_space: adt * ((1.0 - f) * up + emit),
        }
    }
}
