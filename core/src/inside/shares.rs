//! The shares of a cell's means over its samples (`Inside`): the liquid in its soil by how much liquid gathers at each
//! point (TOPMODEL, Beven and Kirkby 1979), its snow by the sun on each point, and each kind of cover by how wet a place
//! its genes like, the liquid that stands on it lying by its level.

use super::Ground;
use crate::cover::{MOIST, RANGE};
use crate::Physics;

/// Rounds of halving that find the level of the soil's liquid (`soil`): enough to bring a span of the wetness index to
/// the last bit of a 64-bit number.
const HALVINGS: usize = 128;

/// What a cell holds as the shares read it, as depths over the cell, m (`flow::means`): the liquid in its pores, the
/// liquid its pores hold when full, the liquid that stands over them, and its soft cover (its snow).
#[derive(Clone, Copy, Debug, Default)]
pub struct Means {
    pub pore: f64,
    pub pores: f64,
    pub stands: f64,
    pub snow: f64,
}

/// The shares of a cell's means over its samples (`Ground`), each a mean of 1 by area: of the liquid in its soil and of
/// its snow; and how wet each sample is, 0..1 (`Physics::wetness` of the liquid in its pores and of what stands there at
/// the cell's level, `Ground::standing`).
#[derive(Clone, Debug)]
pub struct Inside {
    pub soil: Vec<f64>,
    pub snow: Vec<f64>,
    pub wet: Vec<f64>,
}

impl Inside {
    /// The shares of the means `m` over ground `g`, which `inflow` m² drain into from upstream (`Ground::wetness_index`).
    pub fn of(g: &Ground, ph: &Physics, inflow: f64, m: Means) -> Inside {
        let soil = soil(g, ph, &g.wetness_index(inflow), m);
        let snow = g.share(g.sun().iter().map(|s| (-ph.snow_sun * s).exp()).collect());
        let deep = g.standing(m.stands);
        let wet = (0..soil.len()).map(|p| ph.wetness(m.pore * soil[p] + deep[p])).collect();
        Inside { soil, snow, wet }
    }

    /// The share of a kind of cover of moisture gene `moist` (`cover::MOIST`) at each sample: a Gaussian response along
    /// the wetness gradient (ter Braak and Looman 1986), highest where the log of the sample's wetness is the gene, of
    /// width `moist_width` decades. A kind that needs a wet place to be active and works fastest there gathers where the
    /// ground is wet, and one that works on dry ground where it is dry (the wet kinds outgrow it on wet ground).
    pub fn kind(&self, g: &Ground, ph: &Physics, moist: f32) -> Vec<f64> {
        let (lo, hi) = (RANGE[MOIST].1 as f64, RANGE[MOIST].2 as f64);
        let off: Vec<f64> = self.wet.iter().map(|&w| w.max(1e-300).log10().clamp(lo, hi) - moist as f64).collect();
        let near = off.iter().fold(f64::INFINITY, |m, d| m.min(d * d));
        g.share(off.iter().map(|d| (-(d * d - near) / (2.0 * ph.moist_width * ph.moist_width)).exp()).collect())
    }
}

/// The share of the liquid in the soil at each sample of `g`, of wetness index `twi` (`Ground::wetness_index`): as in
/// TOPMODEL (Beven and Kirkby 1979), the liquid a point holds departs from the cell's mean by `soil_m` per unit of the
/// index over the index's level, within none and its pores full, the level such that the mean is the cell's (found by
/// halving). Ground with no liquid in its pores, or with its pores full, shares it evenly.
fn soil(g: &Ground, ph: &Physics, twi: &[f64], m: Means) -> Vec<f64> {
    if m.pore <= 0.0 || m.pore >= m.pores {
        return vec![1.0; twi.len()];
    }
    let held = |c: f64| twi.iter().map(|t| (m.pore + ph.soil_m * (t - c)).clamp(0.0, m.pores)).collect::<Vec<f64>>();
    let (lo, hi) = twi.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &t| (a.min(t), b.max(t)));
    let (mut lo, mut hi) = (lo - m.pores / ph.soil_m, hi + m.pore / ph.soil_m);
    for _ in 0..HALVINGS {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if g.mean(&held(mid)) > m.pore { lo = mid } else { hi = mid }
    }
    g.share(held(0.5 * (lo + hi)))
}
