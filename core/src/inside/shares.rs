//! The liquid table of a cell at each point of its ground, by how much liquid gathers there (TOPMODEL, Beven and Kirkby
//! 1979), and the liquid its soil holds there.

use crate::Physics;

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
/// cell's: its liquid lies where it lies, only shared out. None where it is saturated all over or its room is the most
/// there is (it lies evenly), or every point is a channel. A channel's point, of infinite index (`flowing`), has no
/// room. The mean room grows with the level piece by piece in a line: a point's room is none up to its index less the
/// cell's room over `soil_m`, then grows by `soil_m` per unit of the level until it is the most. So the level is found
/// exactly, walking the points' ends in order (the points sorted by their index, quickly where they come sorted); where
/// the mean room reaches the cell's along no level (the channels hold too much of the area), the highest level the
/// points' rooms reach.
pub fn soil_level(ph: &Physics, twi: &[f64], area: &[f64], m: Means) -> Option<f64> {
    let [d, most] = m.deficit(ph);
    let mut points: Vec<(f64, f64)> = twi.iter().zip(area).filter(|p| p.0.is_finite()).map(|(&t, &a)| (t, a)).collect();
    if d <= 0.0 || d >= most || points.is_empty() {
        return None;
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    // A point's room starts to grow at its index less `rise` and stops at its index plus `rest`.
    let (rise, rest, n) = (d / ph.soil_m, (most - d) / ph.soil_m, points.len());
    // The mean room at the level `x` and how fast it grows there, per unit of the level; the next start and stop.
    let (mut room, mut grows, mut x, [mut start, mut stop]) = (0.0, 0.0, f64::NEG_INFINITY, [0, 0]);
    while stop < n {
        let (starts, stops) = (if start < n { points[start].0 - rise } else { f64::INFINITY }, points[stop].0 + rest);
        let next = starts.min(stops);
        if grows > 0.0 {
            let there = room + grows * (next - x);
            if there >= d {
                return Some((x + (d - room) / grows).clamp(x, next));
            }
            room = there;
        }
        x = next;
        if starts <= stops {
            grows += ph.soil_m * points[start].1;
            start += 1;
        } else {
            grows = (grows - ph.soil_m * points[stop].1).max(0.0);
            stop += 1;
        }
    }
    Some(x)
}
