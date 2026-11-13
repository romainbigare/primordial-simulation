//! Radiation: sunlight, the heat radiation of the ground and the layers of the air, and contact heat between the
//! ground and the air touching it, in the balance of a ground node's surface over a step (`Surface`).

use super::super::land::NONE;
use super::surf::Surf;
use crate::chem::{Chemistry, HEAT_BANDS};
use crate::{math::one_minus_exp, units::{R_GAS, SIGMA}};
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
/// gases, scattered by its molecules, passed]. Of what a pass scatters, half goes on up and half down (single scattering
/// with a phase function that sends as much forward as back, as Rayleigh's does).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Column {
    down: [f64; 3],
    up: [f64; 3],
}

impl Column {
    /// The column whose gases absorb and scatter with the optical depths `depths(slant)` = [absorbed, scattered] on a
    /// path `slant` times its height (`Chemistry::sun_depth` of the amount on the path, and `Substance::scatter` times
    /// it), under the sun's mean height `mu` (`sun_height`).
    pub(crate) fn of(depths: impl Fn(f64) -> [f64; 2], mu: f64) -> Column {
        let pass = |slant: f64| {
            let [absorbs, scatters] = depths(slant);
            let (all, gone) = (absorbs + scatters, one_minus_exp(absorbs + scatters));
            if all > 0.0 { [gone * absorbs / all, gone * scatters / all, 1.0 - gone] } else { [0.0, 0.0, 1.0] }
        };
        Column { down: pass(1.0 / mu), up: pass(DIFFUSE) }
    }

    /// Per unit of the sunlight that enters it at the top, over a surface that reflects the share `alb`: [what reaches
    /// the surface, in all its passes; what the air absorbs; what goes back to space]. The surface gets what comes down,
    /// D, and again the share b scattered back down of what it sends up, so in all D / (1 − `alb` b); what it absorbs,
    /// (1 − `alb`) of that, and the other two add up to 1.
    pub(crate) fn shares(&self, alb: f64) -> [f64; 3] {
        self.through(self.down, alb)
    }

    /// `shares` of light that enters at the top with the pass `down`: the sun's beam (`Column::down`), or diffuse light,
    /// whose pass down is its pass up.
    fn through(&self, down: [f64; 3], alb: f64) -> [f64; 3] {
        let ([a1, s1, t1], [a2, s2, t2]) = (down, self.up);
        let ground = (t1 + 0.5 * s1) / (1.0 - alb * 0.5 * s2);
        [ground, a1 + a2 * alb * ground, 0.5 * s1 + (t2 + 0.5 * s2) * alb * ground]
    }

    /// `shares` under a cloud that lies over the column, per unit of the sunlight that enters the cloud; what the
    /// cloud absorbs counts with the air's. What the cloud passes straight goes on as the beam, what it passes
    /// scattered as diffuse light; of what the column and the surface send back up, U, the cloud's base reflects the
    /// share r down again as diffuse light and passes the share t to space. So the diffuse light that enters the column
    /// is in all D = (scattered + r straight u_b) / (1 − r u_d), u_b and u_d the shares the column and the surface
    /// send back of a beam and of diffuse light, and U = straight u_b + D u_d; the three still add up to 1.
    pub(crate) fn under(&self, cloud: &Cloud, alb: f64) -> [f64; 3] {
        let ([ground_b, air_b, up_b], [ground_d, air_d, up_d]) = (self.through(self.down, alb), self.through(self.up, alb));
        let ([back, scattered, straight], [back_d, passed_d]) = (cloud.beam, cloud.diffuse);
        let diffuse = (scattered + back_d * straight * up_b) / (1.0 - back_d * up_d);
        let up = straight * up_b + diffuse * up_d;
        let in_cloud = (1.0 - back - scattered - straight) + up * (1.0 - back_d - passed_d);
        [straight * ground_b + diffuse * ground_d, straight * air_b + diffuse * air_d + in_cloud, back + up * passed_d]
    }
}

/// How near to 1 a layer's fall of diffuse light along the beam, k μ, may come before the beam's height is moved by
/// twice this share: at 1 the two-stream solution divides nothing by nothing.
const BEAM_APART: f64 = 1e-6;

/// A layer of scatterers of optical depth `tau` that scatter the share `w0` of the light they meet, forward by the mean
/// cosine `g`, under a beam from the height `mu` (the sine of its elevation): [reflected, passed in all, passed
/// straight], per unit of the beam. Two streams in Eddington's form (Meador and Weaver 1980), the forward peak of the
/// scattering taken as light that passes (Joseph, Wiscombe and Weinman 1976), so a thin layer under a high sun
/// reflects no less than nothing.
fn two_stream(tau: f64, w0: f64, g: f64, mu: f64) -> [f64; 3] {
    let peak = g * g;
    let (tau, w0, g) = ((1.0 - w0 * peak) * tau, (1.0 - peak) * w0 / (1.0 - w0 * peak), g / (1.0 + g));
    let (g1, g2) = ((7.0 - w0 * (4.0 + 3.0 * g)) / 4.0, -(1.0 - w0 * (4.0 - 3.0 * g)) / 4.0);
    // How fast diffuse light falls with depth in the layer; none where nothing is absorbed.
    let k = (g1 * g1 - g2 * g2).max(0.0).sqrt();
    let mu = if (1.0 - k * mu).abs() < BEAM_APART { mu * (1.0 + 2.0 * BEAM_APART) } else { mu };
    let (g3, direct) = ((2.0 - 3.0 * g * mu) / 4.0, (-tau / mu).exp());
    let g4 = 1.0 - g3;
    let [back, passed] = if w0 >= 1.0 {
        let back = (g1 * tau + (g3 - g1 * mu) * (1.0 - direct)) / (1.0 + g1 * tau);
        [back, 1.0 - back]
    } else {
        let (a1, a2) = (g1 * g4 + g2 * g3, g1 * g3 + g2 * g4);
        // Every term over e^(k τ), so a deep layer overflows nothing.
        let (e1, e2) = ((-k * tau).exp(), (-2.0 * k * tau).exp());
        let per = w0 / ((1.0 - k * k * mu * mu) * (k + g1 + (k - g1) * e2));
        let back = per * ((1.0 - k * mu) * (a2 + k * g3) - (1.0 + k * mu) * (a2 - k * g3) * e2 - 2.0 * k * (g3 - a2 * mu) * direct * e1);
        let through = ((1.0 + k * mu) * (a1 + k * g4) - (1.0 - k * mu) * (a1 - k * g4) * e2) * direct - 2.0 * k * (g4 + a1 * mu) * e1;
        [back, direct - per * through]
    };
    let back = back.clamp(0.0, 1.0 - direct);
    [back, passed.clamp(direct, 1.0 - back), direct]
}

/// What a cloud, a layer of droplets, does to sunlight (laws.md 1.4), per unit of the light that enters it: of the
/// sun's beam [reflected, passed scattered, passed straight], and of diffuse light [reflected, passed]; the rest it
/// absorbs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cloud {
    beam: [f64; 3],
    diffuse: [f64; 2],
}

impl Cloud {
    /// The cloud of optical depth `tau` under the sun's mean height `mu` (`sun_height`), its droplets scattering forward
    /// by the mean cosine `g` (`two_stream`); in the share `dark[0]` of the sunlight (the wavelengths their liquid
    /// absorbs) a droplet absorbs the share `dark[1]` of what it meets, in the rest none. Diffuse light crosses it as a
    /// beam on the slant `DIFFUSE`.
    pub(crate) fn of(tau: f64, mu: f64, g: f64, dark: [f64; 2]) -> Cloud {
        let lit = |mu: f64| {
            let (clear, dim) = (two_stream(tau, 1.0, g, mu), two_stream(tau, 1.0 - dark[1], g, mu));
            [0, 1, 2].map(|q| (1.0 - dark[0]) * clear[q] + dark[0] * dim[q])
        };
        let ([back, passed, straight], [back_d, passed_d, _]) = (lit(mu), lit(1.0 / DIFFUSE));
        Cloud { beam: [back, passed - straight, straight], diffuse: [back_d, passed_d] }
    }
}

/// What the air of a node does to sunlight (laws.md 1.4): its clear column, and where a cloud covers a share of its
/// sky, that share and the cloud, which lies over the same column.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Light {
    clear: Column,
    cloud: Option<(f64, Cloud)>,
}

impl Light {
    pub(crate) fn of(clear: Column, cloud: Option<(f64, Cloud)>) -> Light {
        Light { clear, cloud }
    }

    /// `Column::shares` of the node's whole sky: the clear part's and the part's under the cloud (`Column::under`),
    /// each by its share.
    pub(crate) fn shares(&self, alb: f64) -> [f64; 3] {
        let clear = self.clear.shares(alb);
        match &self.cloud {
            None => clear,
            Some((share, cloud)) => {
                let under = self.clear.under(cloud, alb);
                [0, 1, 2].map(|q| (1.0 - share) * clear[q] + share * under[q])
            }
        }
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

/// Per band of heat radiation, the optical depth and the height its absorbers fill, each by its depth: of `gases`
/// (substance, mol/m², the height its amount fills, m) under air at `t` K whose own height is `big` m, a gas's lines
/// (`Chemistry::depth`) where its amount times the pressure is, as the pressure broadens them (height hk H / (hk + H),
/// hk its own, H the air's); its continuum, `continuum` times the column of its amount times its own pressure,
/// c p₀ / 2 = c² R T / (2 hk) (p₀ its partial pressure at the ground; both fall by e over hk, so their product over
/// hk / 2, the height it fills), in every band.
pub fn band_depths(chem: &Chemistry, gases: &[(usize, f64, f64)], (t, big): (f64, f64)) -> [(f64, f64); HEAT_BANDS] {
    let mut sum = [[0.0; 2]; HEAT_BANDS];
    let mut add = |b: usize, d: f64, h: f64| (sum[b][0], sum[b][1]) = (sum[b][0] + d, sum[b][1] + d * h);
    for &(k, c, hk) in gases {
        (0..HEAT_BANDS).for_each(|b| add(b, chem.depth(k, b, c), hk * big / (hk + big)));
        let near = chem.subs[k].continuum * c * c * R_GAS * t / (2.0 * hk);
        (0..HEAT_BANDS).for_each(|b| add(b, near, 0.5 * hk));
    }
    sum.map(|[d, dh]| (d, if d > 0.0 { dh / d } else { big }))
}

/// The parts of each band of heat radiation (`Heat`): its core and its wings.
const PARTS: usize = 2;
const SUBS: usize = HEAT_BANDS * PARTS;
/// The step of temperature over which the slope of a band's share is taken, K.
const SHARE_STEP_K: f64 = 0.5;

/// What the air over a place does to heat radiation (laws.md 1.4): per band of it (`Chemistry::heat_spans`), its
/// absorbers' optical depth and the height they fill near the ground; each band in two parts, its core `spread` times
/// its depth and its wings 1 / `spread` times, each half its light, as a band's lines are some strong and many weak;
/// a cloud's drops, grey, add the same depth to every part; each part a grey column (`column`) whose layers send in the band the share of their own temperature's light
/// (`Chemistry::heat_shares`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Heat {
    f: [f64; SUBS],
    lay: [[[f64; LAYERS]; 2]; SUBS],
}

/// The side a column's layers send to: down to the ground, up to space.
pub const DOWN: usize = 0;
pub const UP: usize = 1;

impl Heat {
    /// The air whose gases have in each band the optical depth and height `bands` (depth, m, `band_depths`) under a
    /// cloud of `cloud` (depth, m), its air falling by `gam` K/m over its column of `l` m, a band's core and wings
    /// `spread` apart.
    pub fn of(bands: [(f64, f64); HEAT_BANDS], (c, hc): (f64, f64), (gam, l): (f64, f64), spread: f64) -> Heat {
        let mut heat = Heat::default();
        for (b, &(d, hd)) in bands.iter().enumerate() {
            for (p, part) in [spread, 1.0 / spread].into_iter().enumerate() {
                let (q, x) = (b * PARTS + p, d * part + c);
                let h = if x > 0.0 { (d * part * hd + c * hc) / x } else { hc };
                (heat.f[q], heat.lay[q]) = (one_minus_exp(x), column(x, gam, l, h));
            }
        }
        heat
    }

    /// The share of the heat radiation of a surface at `t` K that the air absorbs.
    pub fn absorbs(&self, chem: &Chemistry, t: f64) -> f64 {
        let shares = chem.heat_shares(t);
        (0..SUBS).map(|q| shares[q / PARTS] / PARTS as f64 * self.f[q]).sum()
    }

    /// What its layers send to side `side` (`DOWN`, `UP`) over air at `tl` K, W/m², and its slope with `tl`, W/(m² K).
    pub fn glow(&self, chem: &Chemistry, tl: f64, side: usize) -> [f64; 2] {
        let mut out = [0.0; 2];
        for q in (0..SUBS).filter(|&q| self.f[q] != 0.0) {
            let (b, k) = (q / PARTS, SIGMA * self.f[q] / (LAYERS * PARTS) as f64);
            for &d in &self.lay[q][side] {
                let t = (tl + d).max(1.0);
                let s = chem.heat_share(t, b);
                let ds = (chem.heat_share(t + SHARE_STEP_K, b) - chem.heat_share(t - SHARE_STEP_K, b)) / (2.0 * SHARE_STEP_K);
                out[0] += k * s * t.powi(4);
                out[1] += k * (4.0 * s + t * ds) * t.powi(3);
            }
        }
        out
    }

    /// What its layers send to side `side` over air at `tl` K, W/m²: `glow` without its slope.
    pub fn sends(&self, chem: &Chemistry, tl: f64, side: usize) -> f64 {
        let mut out = 0.0;
        for q in (0..SUBS).filter(|&q| self.f[q] != 0.0) {
            let (b, k) = (q / PARTS, SIGMA * self.f[q] / (LAYERS * PARTS) as f64);
            for &d in &self.lay[q][side] {
                let t = (tl + d).max(1.0);
                out += k * chem.heat_share(t, b) * t.powi(4);
            }
        }
        out
    }
}

/// A cloud's optical depth to heat radiation, from its depth `tau` to sunlight: a droplet wider than the wave absorbs
/// the heat radiation that falls on its cross-section, half of what it takes out of a beam of sunlight (the other
/// half it bends past its edge), and the radiation crosses the cloud on the slant of diffuse light (`DIFFUSE`). For
/// droplets of 10 µm that is 125 m² a kg of liquid; measured, 130 upward and 158 downward (Stephens 1978).
pub(super) fn drops_heat(tau: f64) -> f64 {
    0.5 * DIFFUSE * tau
}

/// What the air over a place does to heat radiation under a sky a cloud covers a share of (laws.md 1.4): the clear
/// part's column and, where there is a cloud, its share and the column under it (`Heat`), each absorbing and sending
/// by its share.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Skies {
    clear: Heat,
    cloud: Option<(f64, Heat)>,
}

impl Skies {
    pub(crate) fn of(clear: Heat, cloud: Option<(f64, Heat)>) -> Skies {
        Skies { clear, cloud }
    }

    /// `Heat::absorbs` of the whole sky.
    pub(crate) fn absorbs(&self, chem: &Chemistry, t: f64) -> f64 {
        let clear = self.clear.absorbs(chem, t);
        self.cloud.as_ref().map_or(clear, |(share, under)| (1.0 - share) * clear + share * under.absorbs(chem, t))
    }

    /// `Heat::sends` of the whole sky.
    pub(crate) fn sends(&self, chem: &Chemistry, tl: f64, side: usize) -> f64 {
        let clear = self.clear.sends(chem, tl, side);
        self.cloud.as_ref().map_or(clear, |(share, under)| (1.0 - share) * clear + share * under.sends(chem, tl, side))
    }

    /// `Heat::glow` of the whole sky.
    pub(crate) fn glow(&self, chem: &Chemistry, tl: f64, side: usize) -> [f64; 2] {
        let clear = self.clear.glow(chem, tl, side);
        match &self.cloud {
            None => clear,
            Some((share, under)) => {
                let under = under.glow(chem, tl, side);
                [0, 1].map(|q| (1.0 - share) * clear[q] + share * under[q])
            }
        }
    }
}

/// The air touching a ground node over a step: its temperature, K; the share of the surface's heat radiation it absorbs
/// (`Skies::absorbs`); what its layers send down (`Skies::glow`), W/m²; and the slope of that with its temperature,
/// W/(m² K).
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
    use super::{Air, Cloud, Column, Light, Surf, Surface, NONE};
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
            let c = Column::of(|slant| [depth * slant.sqrt(), 0.5 * depth * slant], mu);
            let mut was = f64::NEG_INFINITY;
            for alb in [0.0, 0.1, 0.5, 0.9] {
                let [ground, air, space] = c.shares(alb);
                let all = ground * (1.0 - alb) + air + space;
                assert!((all - 1.0).abs() < 1e-12, "depth {depth}, mu {mu}, reflectance {alb}: {all}");
                assert!(air >= was, "the air absorbed less over a brighter surface: {air} after {was}");
                was = air;
                // Under a cloud over half its sky too.
                for tau in [0.5, 8.0, 80.0] {
                    let [ground, air, space] = Light::of(c, Some((0.5, Cloud::of(tau, mu, 0.85, [0.2, 0.04])))).shares(alb);
                    let all = ground * (1.0 - alb) + air + space;
                    assert!((all - 1.0).abs() < 1e-12, "depth {depth}, mu {mu}, reflectance {alb}, cloud {tau}: {all}");
                }
            }
        }
    }

    /// A thicker cloud reflects more of the sun's beam and passes less, and reflects less the higher the sun stands;
    /// one of no depth does nothing, and none reflects or passes less than nothing.
    #[test]
    fn a_thicker_cloud_reflects_more_and_a_higher_sun_less() {
        let (g, dark) = (0.85, [0.2, 0.04]);
        assert_eq!((Cloud::of(0.0, 0.5, g, dark).beam, Cloud::of(0.0, 0.5, g, dark).diffuse), ([0.0, 0.0, 1.0], [0.0, 1.0]));
        for mu in [0.05, 0.3, 0.6, 1.0] {
            let mut was = Cloud::of(0.0, mu, g, dark);
            for tau in [0.3, 1.0, 3.0, 10.0, 30.0, 100.0, 1000.0] {
                let c = Cloud::of(tau, mu, g, dark);
                let (passed, absorbed) = (c.beam[1] + c.beam[2], 1.0 - c.beam.iter().sum::<f64>());
                assert!(c.beam[0] > was.beam[0] && passed < was.beam[1] + was.beam[2], "depth {tau}, sun {mu}: {c:?} after {was:?}");
                assert!(c.beam.iter().chain(&c.diffuse).all(|&x| (0.0..=1.0).contains(&x)) && absorbed > 0.0 && absorbed < 0.2, "depth {tau}, sun {mu}: {c:?}");
                was = c;
            }
        }
        for tau in [1.0, 10.0, 100.0] {
            let back = |mu: f64| Cloud::of(tau, mu, g, dark).beam[0];
            assert!(back(0.2) > back(0.5) && back(0.5) > back(0.9), "depth {tau}: {} {} {}", back(0.2), back(0.5), back(0.9));
        }
    }

    /// Clear air scatters more of the sunlight back to space the more of it there is and the lower the sun stands; with
    /// nothing in it, all the light reaches the ground.
    #[test]
    fn the_clear_air_scatters_more_with_its_column() {
        assert_eq!(Column::of(|_| [0.0; 2], 0.5).shares(0.0), [1.0, 0.0, 0.0]);
        let back = |depth: f64, mu: f64| Column::of(|slant| [0.0, depth * slant], mu).shares(0.0)[2];
        let mut was = 0.0;
        for depth in [0.02, 0.05, 0.1, 0.2] {
            let b = back(depth, 0.5);
            assert!(b > was && back(depth, 0.3) > b && b < 0.5, "depth {depth}: {b} back to space after {was}");
            was = b;
        }
    }
}
