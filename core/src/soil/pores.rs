//! The pores of the loose ground from its texture (laws.md 4): how much of it is pore space, what it holds against
//! gravity and against roots, and how fast liquid passes through it when full. Its mineral part's by the regressions
//! of Saxton and Rawls (2006) on the shares of coarse and fine particles, fitted to 2,000 soils of the USDA's data; its
//! organic part's as decomposed organic ground holds and passes liquid (`ORGANIC`); the two mixed by their volumes.

use super::Texture;
use serde::{Deserialize, Serialize};

/// The regressions' terms with no organic matter, each a weight of: the coarse share S, the fine share C, S C, and one
/// (Saxton and Rawls 2006, their table 1): of the liquid held at the wilting point (−1500 kPa), at field capacity
/// (−33 kPa), and between field capacity and full.
const WILT_FIT: [f64; 4] = [-0.024, 0.487, 0.068, 0.031];
const FIELD_FIT: [f64; 4] = [-0.251, 0.195, 0.452, 0.299];
const OVER_FIT: [f64; 4] = [0.278, 0.034, -0.584, 0.078];
/// Each regression's second step on its first estimate θ: θ + a θ² + b θ + c (their table 1).
const WILT_STEP: [f64; 3] = [0.0, 0.14, -0.02];
const FIELD_STEP: [f64; 3] = [1.283, -0.374, -0.015];
const OVER_STEP: [f64; 3] = [0.0, 0.636, -0.107];
/// The full pores: field capacity, the share between it and full, and this weight of S and this one (their eq. 5).
const FULL_FIT: [f64; 2] = [-0.097, 0.043];
/// The conductivity when full, mm an hour: this times (full − field capacity)^(3 − λ) (their eq. 16), λ the slope of
/// the logarithm of the liquid held against that of the suction from −1500 to −33 kPa (their eq. 15).
const SEEP_FIT: f64 = 1930.0;
const SUCTIONS_KPA: [f64; 2] = [1500.0, 33.0];
/// The range of the soils the regressions are fitted to: the fine share and the coarse share.
const FINE_FITTED: [f64; 2] = [0.05, 0.6];
const COARSE_FITTED: [f64; 2] = [0.05, 0.95];
/// Millimetres an hour in a metre a second.
const MM_AN_HOUR: f64 = 3.6e6;
/// Decomposed organic ground: the share of its volume that is pores, the shares of its volume it holds at field
/// capacity and at the wilting point, and its conductivity when full, m/s.
const ORGANIC: [f64; 4] = [0.85, 0.6, 0.3, 1e-6];

/// The pores of loose ground: the share of its volume that is pore space; the shares of its pores it holds against
/// gravity (its field capacity) and at the wilting point, which no root draws; and its conductivity when full, m/s
/// (Darcy's law).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pores {
    pub porosity: f64,
    pub field: f64,
    pub wilt: f64,
    pub seep: f64,
}

impl Pores {
    /// The pores of loose ground of texture `t`: of its mineral part (`mineral`) and its organic part (`ORGANIC`), each
    /// liquid held and pore space by their shares of its volume, and the conductivity their mean by volume of its
    /// logarithm (a mixture's, neither in series nor beside each other).
    pub fn of(t: &Texture) -> Pores {
        let (m, o) = (mineral(t), t.organic.clamp(0.0, 1.0));
        let mix = |a: f64, b: f64| (1.0 - o) * a + o * b;
        let porosity = mix(m[0], ORGANIC[0]);
        let seep = (mix(m[3].ln(), ORGANIC[3].ln())).exp();
        Pores { porosity, field: mix(m[1], ORGANIC[1]) / porosity, wilt: mix(m[2], ORGANIC[2]) / porosity, seep }
    }

    /// The share of the liquid ground holds for roots that ground whose pores are the share `filled` full holds: none
    /// at the wilting point, all at field capacity and over it, in proportion between (the available liquid of soil
    /// physics; Saxton and Rawls 2006). Roots draw it (`cover::wet`) and the air takes it up (`air::surf::Vapour`, the
    /// bucket of Manabe 1969).
    pub fn available(&self, filled: f64) -> f64 {
        ((filled - self.wilt) / (self.field - self.wilt)).clamp(0.0, 1.0)
    }

    /// The share of loose ground whose pores are full of liquid when the share `filled` of its pores is: up to field
    /// capacity its liquid lies in its small pores all through it and its large ones hold gas; what it holds over that
    /// fills its pores from the bottom up (Flow's one store over the depth of its loose ground), so the gas reaches
    /// only the share above that table (roots, `cover::working`; the decomposers, `change`).
    pub fn full(&self, filled: f64) -> f64 {
        ((filled - self.field) / (1.0 - self.field).max(1e-9)).clamp(0.0, 1.0)
    }

    /// The liquid loose ground `depth` m deep holds for roots, between the wilting point and field capacity, m (its
    /// available water capacity).
    pub fn for_roots(&self, depth: f64) -> f64 {
        (self.field - self.wilt) * self.porosity * depth
    }
}

/// The mineral part of loose ground of texture `t` (Saxton and Rawls 2006, its shares held inside the range their
/// regressions are fitted to): the shares of its volume that are pores, that it holds at field capacity and at the
/// wilting point, and its conductivity when full, m/s.
fn mineral(t: &Texture) -> [f64; 4] {
    let c = t.fine.clamp(FINE_FITTED[0], FINE_FITTED[1]);
    let s = t.coarse.clamp(COARSE_FITTED[0], COARSE_FITTED[1].min(1.0 - c));
    let fit = |w: &[f64; 4], [a, b, k]: [f64; 3]| {
        let first = w[0] * s + w[1] * c + w[2] * s * c + w[3];
        first + a * first * first + b * first + k
    };
    let (wilt, field) = (fit(&WILT_FIT, WILT_STEP), fit(&FIELD_FIT, FIELD_STEP));
    let full = field + fit(&OVER_FIT, OVER_STEP) + FULL_FIT[0] * s + FULL_FIT[1];
    let lambda = (field / wilt).ln() / (SUCTIONS_KPA[0] / SUCTIONS_KPA[1]).ln();
    [full, field, wilt, SEEP_FIT * (full - field).powf(3.0 - lambda) / MM_AN_HOUR]
}
