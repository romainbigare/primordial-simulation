//! Radiation: sunlight, the heat radiation of the ground and the layers of the air, and contact heat between the
//! ground and the air touching it, in the balance of a ground node's surface over a step (`Surface`).

use super::super::land::NONE;
use super::super::Scratch;
use crate::math::one_minus_exp;
use crate::units::SIGMA;
use std::f64::consts::PI;

/// Daily-mean sunlight at the top of the air, W/m².
pub(super) fn insolation(s0: f64, lat: f64, decl: f64) -> f64 {
    let h0 = (-lat.tan() * decl.tan()).clamp(-1.0, 1.0).acos();
    (s0 / PI * (h0 * lat.sin() * decl.sin() + lat.cos() * decl.cos() * h0.sin())).max(0.0)
}

/// Layers of the air over a cell in `column`.
pub(crate) const LAYERS: usize = 4;

/// The layers of the air over a cell that send heat radiation [down to the ground, up to space],
/// as how much colder than the air touching the ground each is, K: the grey absorber of optical
/// depth `x` lies like the air's mass (scale height `h`, in a column `l` deep), and the air's
/// temperature falls by `gam` per metre. Each layer sends that side the same share of what the
/// column sends it, from its height (Schwarzschild's integral, summed at `LAYERS` points: see
/// `glow`). More gas lifts the layers that reach space into colder air, so less leaves and the
/// ground warms, with no ceiling (one slab at the mean temperature has one: 2^(1/4) times the bare ground's).
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

/// What a ground node's surface balance reads over a step, per m² of its cell: the shortwave it absorbs, W/m²; the
/// temperature of the air touching it, K; the heat it gains from the gas it exchanges with the air, linearised about
/// its temperature `t0` (`gain` W/m² there and `slope` W/(m² K) more per K warmer), and from rain at the air's
/// temperature `t_rain` through `g_rain`, W/(m² K).
pub(super) struct Surface {
    pub(super) sw: f64,
    pub(super) tl: f64,
    pub(super) t0: f64,
    pub(super) gain: f64,
    pub(super) slope: f64,
    pub(super) t_rain: f64,
    pub(super) g_rain: f64,
}

/// What the balance gives: the temperature of the node at the end of the step, K; the heat it gives its air node, J,
/// and the slope of that with the air's temperature, J/K, and with the node's temperature, J/K; how far the node's
/// temperature moves per W/m² more that reaches it, K m²/W; and the heat that leaves to space through the air, J.
pub(super) struct Balance {
    pub(super) ts: f64,
    pub(super) to_air: f64,
    pub(super) slope: f64,
    pub(super) warm: f64,
    pub(super) give: f64,
    pub(super) to_space: f64,
}

impl Surface {
    /// The balance of ground node `i`, of `area` m², over `dt` seconds: the ground emits σT⁴, of which the air absorbs
    /// its share and emits by its layers (`column`); contact heat with the air touching the ground (`sensible`); the
    /// heat of the gas it exchanges. The node's temperature is solved implicitly, the emission linearised about the
    /// node's temperature before the step (backward Euler): heat reaches the surface from the node through its heat
    /// capacity over the step (none on a melting plateau) in series with any soft solid cover. A plateau holds the node
    /// only while its fusion enthalpy lasts: a step that would freeze (or melt) more than the node holds takes it off,
    /// and its heat capacity holds it back.
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
            to_space: adt * ((1.0 - f) * up + emit),
        }
    }
}
