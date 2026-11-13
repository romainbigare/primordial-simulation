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
    sun_by_latitude(s0, lat, decl)[0]
}

/// `insolation` and its slope with latitude, W/m² per radian. The sun's height over flat ground at hour angle h is
/// sin μ = sin φ sin δ + cos φ cos δ cos h, and its slope with φ is the northward part of the unit vector to the sun;
/// the hours of daylight move with φ too, but the sun is on the horizon at their ends, so they add nothing (Leibniz).
pub fn sun_by_latitude(s0: f64, lat: f64, decl: f64) -> [f64; 2] {
    let h0 = (-lat.tan() * decl.tan()).clamp(-1.0, 1.0).acos();
    let flat = (s0 / PI * (h0 * lat.sin() * decl.sin() + lat.cos() * decl.cos() * h0.sin())).max(0.0);
    [flat, s0 / PI * (h0 * lat.cos() * decl.sin() - lat.sin() * decl.cos() * h0.sin())]
}

/// The daily-mean sunlight at the top of the air over ground that rises `rise` m per m toward the south (its mean over
/// the ground), W/m² of its plan, from `sun` (`sun_by_latitude`). By Lambert's cosine law the beam on a surface of
/// upward normal N = (−∂z/∂x, −∂z/∂y, 1) per m² of plan is S (s · N), s the unit vector to the sun: linear in the slope,
/// so the sun on a ground is that on its mean slope, and the sum over a ground's parts is S (s · ẑ) over its plan, the
/// beam that crosses it. Over the daylight hours the east-west part cancels (the morning's against the evening's) and
/// the north-south part is the slope with latitude: a slope is flat ground moved toward the sun by its rise (the
/// equivalent latitude, Lee 1962; Swift 1976), exactly in this plan form. An hour a face turns away from the sun counts
/// against it, as the shadow its ridge casts on the ground beyond (the ground beyond takes the beam it misses), so
/// the beam is kept whatever the slopes; only a ground whose day sums below zero takes none.
pub fn on_slope(sun: [f64; 2], rise: f64) -> f64 {
    (sun[0] + rise * sun[1]).max(0.0)
}

/// The sun's mean height over the hours it is up at latitude `lat`, radians, on the days of declinations `decl`: the
/// mean sine of its elevation over those hours (the daily sunlight, `sun_by_latitude`, over the top's `s0` times the
/// share of the day it is up), the slant its light takes through the air being 1 / that. 1 (overhead) where it never
/// rises.
pub fn sun_height(s0: f64, lat: f64, decl: &[f64]) -> f64 {
    let (mut q, mut up) = (0.0, 0.0);
    for &d in decl {
        let day = sun_by_latitude(s0, lat, d)[0];
        let h0 = (-lat.tan() * d.tan()).clamp(-1.0, 1.0).acos();
        (q, up) = (q + day, up + s0 * h0 / PI);
    }
    if q > 0.0 { (q / up).clamp(f64::MIN_POSITIVE, 1.0) } else { 1.0 }
}

/// The slant of light that leaves a surface diffuse, on its way through the air: the mean of 1 / cos over a hemisphere
/// weighted by cos, as the two-stream methods take it (Elsasser's diffusivity factor 1.66).
const DIFFUSE: f64 = 1.66;

/// What a column of air does to sunlight (laws.md 1.4), per unit of the light that enters it: on the way down along the
/// sun's slant, then on the way up as diffuse light of what the surface reflects (`DIFFUSE`), each pass [absorbed by its
/// gases, passed].
#[derive(Clone, Copy, Debug)]
pub(crate) struct Column {
    down: [f64; 2],
    up: [f64; 2],
}

impl Column {
    /// The column whose gases absorb with the optical depth `absorbs(slant)` on a path `slant` times its height
    /// (`Chemistry::sun_depth` of the amount on the path), under the sun's mean height `mu` (`sun_height`).
    pub(crate) fn of(absorbs: impl Fn(f64) -> f64, mu: f64) -> Column {
        let pass = |slant: f64| {
            let gone = one_minus_exp(absorbs(slant));
            [gone, 1.0 - gone]
        };
        Column { down: pass(1.0 / mu), up: pass(DIFFUSE) }
    }

    /// Per unit of the sunlight that enters it at the top, over a surface that reflects the share `alb`: [what reaches
    /// the surface; what the air absorbs, on the way down and of what the surface reflects; what goes back to space].
    /// What the surface absorbs, (1 − `alb`) of the first, and the other two add up to 1.
    pub(crate) fn shares(&self, alb: f64) -> [f64; 3] {
        let ([a1, t1], [a2, t2]) = (self.down, self.up);
        [t1, a1 + a2 * alb * t1, t2 * alb * t1]
    }
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
        if g.pk == NONE {
            if end < l.span[0] {
                (ts, give) = plateau(l.span[0], (f64::INFINITY, l.lat[0]));
            } else if end > l.span[1] {
                (ts, give) = plateau(l.span[1], (l.lat[1], f64::INFINITY));
            }
        }
        let (up, hs) = flux(ts);
        Balance { ts, give, to_air: adt * (f * up + hs - down), to_space: adt * (1.0 - f) * up }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::scratch::Latent;
    use super::{Air, Column, Surf, Surface, NONE};
    use crate::units::{DAY, MAX_SUBSTANCES, SIGMA};

    /// A ground node of 1 m² at the melting point `tm` that holds `fusion` J of the plateau's substance, `phi` of it
    /// molten, under a soft cover `cover` ([resistance, melting point, depth]).
    fn node(tm: f64, (fusion, phi): (f64, f64), cover: [f64; 3]) -> Surf {
        let pk = if fusion > 0.0 { 0 } else { NONE };
        let latent = Latent { held: phi * fusion, span: [f64::MIN, f64::MAX], lat: [0.0; 2], molten: 1 };
        Surf {
            area: 1.0, j: 0, seen_area: 1.0, sun: 0.0, gam: 0.0, zair: 0.0, head: 0.0, tg: tm, pk, phi, cap: CAP,
            latent, cover, alb: 0.0, pd: 0.0, wet: 0.0, heat: 0.0, amt: [0.0; MAX_SUBSTANCES], fusion, canopy: 0.0,
            absorb: 0.0, keep: 0.0, second: None, cell: 0, part: None, share: 1.0,
        }
    }

    /// Heat capacity of the node, J/K per m², and its melting point, K.
    const CAP: f64 = 5e6;
    const TM: f64 = 273.15;

    /// A node on its melting plateau whose step melts all of it (in the sun, under the cover of its own ice, whose
    /// melting point caps the surface) or freezes all of it (in the dark) stays on it for the share of the step its
    /// fusion enthalpy is of the heat the step brings at the melting point (what a node with no end to its plateau
    /// takes), then goes on from the melting point as the same balance with no plateau and no cover that melted, over
    /// the rest of the step: it ends with the heat of both, and its surface is their mean by time, to rounding.
    #[test]
    fn a_plateau_used_up_in_a_step_holds_the_node_for_its_share_then_lets_it_go() {
        let dt = 3.0 * DAY;
        for (sun, tl, phi, cover) in [(250.0, 285.0, 0.1, [1e-2, TM, 0.02]), (0.0, 250.0, 0.9, [1e-3, TM, 0.002])] {
            let air = Air { tl, f: 0.8, down: 0.8 * SIGMA * (tl - 20.0).powi(4), slope: 4.0 * 0.8 * SIGMA * (tl - 20.0).powi(3) };
            let surface = Surface { sw: sun, air: &air, t0: TM, gain: 0.0, slope: 0.0 };
            // The heat a balance gives its node over `dt` s, J.
            let heat = |g: &Surf, dt: f64| {
                let b = surface.balance(g, dt, 10.0);
                (b.ts, sun * dt - b.to_air - b.to_space)
            };
            let fusion = 0.02 * 1000.0 * 3.34e5;
            let used = if sun > 0.0 { (1.0 - phi) * fusion } else { phi * fusion };
            let (held, q) = heat(&node(TM, (1e6 * fusion, phi), cover), dt);
            let share = used / q.abs();
            assert!(share > 0.05 && share < 0.95, "the plateau takes {share} of the step");
            let rest = if sun > 0.0 { [0.0, f64::INFINITY, 0.0] } else { cover };
            let (after, q_rest) = heat(&node(TM, (0.0, 0.0), rest), (1.0 - share) * dt);
            let (ts, q_all) = heat(&node(TM, (fusion, phi), cover), dt);
            let want_ts = share * held + (1.0 - share) * after;
            assert!((ts - want_ts).abs() < 1e-9 * TM, "sun {sun}: the surface {ts} K, the plateau then the rest {want_ts} K");
            assert!((q_all - (q.signum() * used + q_rest)).abs() < 1e-9 * q.abs(), "sun {sun}: {q_all} J, the plateau's {used} J and the rest's {q_rest} J");
        }
    }

    /// What reaches the surface and is absorbed there, what the air absorbs and what goes back to space make up all the
    /// sunlight that enters a column, to rounding, whatever the column, the sun's height and the surface's reflectance;
    /// a surface that reflects more leaves more of the light to the air on its way back up.
    #[test]
    fn a_columns_sunlight_closes() {
        for (depth, mu) in [(0.0, 0.5), (0.05, 0.3), (0.2, 0.7), (3.0, 1.0)] {
            let c = Column::of(|slant| depth * slant.sqrt(), mu);
            let mut was = f64::NEG_INFINITY;
            for alb in [0.0, 0.1, 0.5, 0.9] {
                let [ground, air, space] = c.shares(alb);
                let all = ground * (1.0 - alb) + air + space;
                assert!((all - 1.0).abs() < 1e-12, "depth {depth}, mu {mu}, reflectance {alb}: {all}");
                assert!(air >= was, "the air absorbed less over a brighter surface: {air} after {was}");
                was = air;
            }
        }
    }
}
