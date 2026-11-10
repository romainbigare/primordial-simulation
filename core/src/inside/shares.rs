//! The liquid in a cell's soil at each point of its ground, by how much liquid gathers there (TOPMODEL, Beven and Kirkby
//! 1979).

use crate::Physics;

/// Rounds of halving that find the level of the soil's liquid (`soil_level`): enough to bring a span of the wetness index to
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
