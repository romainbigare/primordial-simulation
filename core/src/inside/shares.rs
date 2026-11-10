//! The shares of a cell's means over its samples (`Inside`): the liquid in its soil by how much liquid gathers at each
//! point (TOPMODEL, Beven and Kirkby 1979), the liquid that stands on it by its level, and each kind of cover by its own
//! growth law at each point.

use super::Ground;
use crate::cover::NG;
use crate::Physics;

/// Rounds of halving that find the level of the soil's liquid (`soil`): enough to bring a span of the wetness index to
/// the last bit of a 64-bit number.
const HALVINGS: usize = 128;

/// What a cell holds as the shares read it, as depths over the cell, m (`flow::means`): the liquid in its pores, the
/// liquid its pores hold before liquid stands (when full; of a cell of parts less the room its land's pores have left,
/// laws.md 1.1), and the liquid that stands over them.
#[derive(Clone, Copy, Debug, Default)]
pub struct Means {
    pub pore: f64,
    pub pores: f64,
    pub stands: f64,
}

/// The shares of a cell's means over its samples (`Ground`), each a mean of 1 by area: of the liquid in its soil; and
/// how wet each sample is, 0..1 (`Physics::wetness` of the liquid in its pores and of what stands there at the cell's
/// level, `Ground::standing`).
#[derive(Clone, Debug)]
pub struct Inside {
    pub soil: Vec<f64>,
    pub wet: Vec<f64>,
}

impl Inside {
    /// The shares of the means `m` over ground `g`, which `inflow` m² drain into from upstream (`Ground::wetness_index`).
    pub fn of(g: &Ground, ph: &Physics, inflow: f64, m: Means) -> Inside {
        let soil = soil(g, ph, &g.wetness_index(inflow), m);
        let deep = g.standing(m.stands);
        let wet = (0..soil.len()).map(|p| ph.wetness(m.pore * soil[p] + deep[p])).collect();
        Inside { soil, wet }
    }

    /// The share of a kind of cover of genes `genes` at each sample: as the cover's growth law runs there, its light
    /// reaction's capacity times its activity on ground that wet (`cover::activity`); the light and the capacity are
    /// the cell's at every sample, so the share goes as the activity. Where every sample is as wet as the kind needs,
    /// or none is wet at all, it is even.
    pub fn kind(&self, g: &Ground, ph: &Physics, genes: &[f32; NG]) -> Vec<f64> {
        g.share(self.wet.iter().map(|&w| crate::cover::activity(ph, genes, w)).collect())
    }
}

/// The share of the liquid in the soil at each sample of `g`, of wetness index `twi` (`Ground::wetness_index`): the
/// steady state of the soil's liquid flowing downhill by Darcy's law; as in TOPMODEL (Beven and Kirkby 1979), the liquid
/// a point holds departs from the cell's mean by `soil_m` per unit of the
/// index over the index's level, within none and its pores full, the level such that the mean is the cell's (found by
/// halving). Ground with no liquid in its pores, or with its pores full, shares it evenly.
fn soil(g: &Ground, ph: &Physics, twi: &[f64], m: Means) -> Vec<f64> {
    match soil_level(ph, twi, &g.area, m) {
        Some(c) => g.share(twi.iter().map(|&t| soil_held(ph, m, t, c)).collect()),
        None => vec![1.0; twi.len()],
    }
}

/// The liquid a point of wetness index `t` holds in a soil of means `m` whose index's level is `c` (`soil_level`), as a
/// depth, m: the mean's, departed by `soil_m` per unit of the index over the level, within none and the pores full.
pub fn soil_held(ph: &Physics, m: Means, t: f64, c: f64) -> f64 {
    (m.pore + ph.soil_m * (t - c)).clamp(0.0, m.pores)
}

/// The level of the wetness index about which the liquid in a soil of means `m` departs at points of index `twi` and
/// shares of the area `area` (summing to 1), such that the mean by area of what they hold (`soil_held`) is the mean's
/// (found by halving); none where the pores hold no liquid or are full (it lies evenly).
pub fn soil_level(ph: &Physics, twi: &[f64], area: &[f64], m: Means) -> Option<f64> {
    if m.pore <= 0.0 || m.pore >= m.pores {
        return None;
    }
    let mean = |c: f64| twi.iter().map(|&t| soil_held(ph, m, t, c)).zip(area).map(|(v, a)| v * a).sum::<f64>();
    let (lo, hi) = twi.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &t| (a.min(t), b.max(t)));
    let (mut lo, mut hi) = (lo - m.pores / ph.soil_m, hi + m.pore / ph.soil_m);
    for _ in 0..HALVINGS {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if mean(mid) > m.pore { lo = mid } else { hi = mid }
    }
    Some(0.5 * (lo + hi))
}
