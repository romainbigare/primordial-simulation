//! The liquid table of a cell at each point of its ground, by how much liquid gathers there (TOPMODEL, Beven and Kirkby
//! 1979), and the liquid its soil holds there.

use crate::Physics;

/// Rounds of halving that find the level of the soil's liquid (`soil_level`): enough to bring a span of the wetness index to
/// the last bit of a 64-bit number.
const HALVINGS: usize = 128;

/// What a cell holds as the shares read it, as depths over the cell, m (`flow::means`): the liquid in its pores, the
/// liquid its pores hold before liquid stands (when full; of a cell of parts less the room its land's pores have left,
/// laws.md 1.1), the liquid that stands over them, and the liquid in the bed's store under it (`Class::Deep`); and the
/// share of its pores it holds at field capacity (`Pores::field`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Means {
    pub pore: f64,
    pub pores: f64,
    pub stands: f64,
    pub store: f64,
    pub field: f64,
}

impl Means {
    /// The room between its surface and its liquid table, as a depth of liquid, m, and the most it may be (TOPMODEL's
    /// storage deficit, Beven and Kirkby 1979): its loose ground holds its liquid up to field capacity in its small pores
    /// all through it and what it holds over that fills its pores from the bottom up (`Pores::full`), so with liquid
    /// over field capacity the table lies in its loose ground and the room is what its pores lack; else under it, in
    /// the bed's store, and the room is its large pores' (over field capacity) and what the store lacks (`bed_depth` of
    /// ground of `bed_porosity`). None where liquid stands over its pores.
    pub fn deficit(&self, ph: &Physics) -> [f64; 2] {
        let (large, bed) = (self.pores * (1.0 - self.field), ph.bed_porosity * ph.bed_depth);
        let most = large + bed;
        if self.stands > 0.0 {
            [0.0, most]
        } else if self.pore > self.field * self.pores {
            [self.pores - self.pore, most]
        } else {
            [large + (bed - self.store).max(0.0), most]
        }
    }
}

/// The room between the surface and the liquid table at a point of wetness index `t` of a cell of means `m` whose
/// index's level is `c` (`soil_level`), as a depth of liquid, m: the cell's (`Means::deficit`), less `soil_m` per unit
/// of the index over the level, within none (saturated) and the most (TOPMODEL: the table's steady state as its liquid
/// runs downhill, its transmissivity falling by e every `soil_m`; Beven and Kirkby 1979, Beven 1997).
pub fn deficit_at(ph: &Physics, m: Means, t: f64, c: f64) -> f64 {
    let [d, most] = m.deficit(ph);
    (d - ph.soil_m * (t - c)).clamp(0.0, most)
}

/// The liquid a point of wetness index `t` holds in its loose ground, of a cell of means `m` whose index's level is `c`
/// (`soil_level`), as a depth, m: the cell's, departed as its table does (`deficit_at`), within none and its pores
/// full; full where it is saturated.
pub fn soil_held(ph: &Physics, m: Means, t: f64, c: f64) -> f64 {
    if t == f64::INFINITY {
        return m.pores;
    }
    if deficit_at(ph, m, t, c) <= 0.0 { m.pores } else { (m.pore + ph.soil_m * (t - c)).clamp(0.0, m.pores) }
}

/// The level of the wetness index about which the table of a cell of means `m` departs at points of index `twi` and
/// shares of the area `area` (summing to 1), such that the mean by area of their room over it (`deficit_at`) is the
/// cell's (found by halving): its liquid lies where it lies, only shared out. None where it is saturated all over or
/// its room is the most there is (it lies evenly), or every point is a channel. A channel's point, of infinite index
/// (`flowing`), has no room.
pub fn soil_level(ph: &Physics, twi: &[f64], area: &[f64], m: Means) -> Option<f64> {
    let [d, most] = m.deficit(ph);
    let (lo, hi) = twi.iter().filter(|t| t.is_finite()).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &t| (a.min(t), b.max(t)));
    if d <= 0.0 || d >= most || lo > hi {
        return None;
    }
    // The mean room over the level `c` (`deficit_at`, the cell's room found once).
    let mean = |c: f64| twi.iter().map(|&t| (d - ph.soil_m * (t - c)).clamp(0.0, most)).zip(area).map(|(v, a)| v * a).sum::<f64>();
    let (mut lo, mut hi) = (lo - most / ph.soil_m, hi + most / ph.soil_m);
    for _ in 0..HALVINGS {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if mean(mid) < d { lo = mid } else { hi = mid }
    }
    Some(0.5 * (lo + hi))
}
