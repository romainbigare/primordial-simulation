//! Terrain that drains (laws.md 0 and 8): a priority flood raises each closed basin of the terrain to its outlet, so
//! liquid runs off every place it reaches (Planchon and Darboux 2001; Barnes, Lehman and Mulla 2014). The terrain's
//! samples drain by it (`space::samples`), and every cell stands on them.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The least slope of ground raised to fill a closed basin, m per m: liquid still runs across the filled floor
/// (Planchon and Darboux 2001).
pub const DRAIN_SLOPE: f64 = 1e-3;

/// A cell waiting in the flood of `drain`, by its height: the lowest first, then the lowest index.
struct Low(f64, usize);

impl Ord for Low {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
    }
}
impl PartialOrd for Low {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Low {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Low {}

/// Give every cell of heights `z` (m) a way down to the cells `reached` already: a priority flood from them, the lowest
/// cell reached first, raises each new neighbour no higher than it to its height plus `DRAIN_SLOPE` times their
/// distance; a higher neighbour already drains through it. `around(i)` gives the neighbours of cell `i` and the
/// distance to each, m, in order.
pub fn drain<I: Iterator<Item = (usize, f64)>>(z: &mut [f64], mut reached: Vec<bool>, around: impl Fn(usize) -> I) {
    let mut flood: BinaryHeap<Low> = (0..z.len()).filter(|&i| reached[i]).map(|i| Low(z[i], i)).collect();
    while let Some(Low(h, i)) = flood.pop() {
        for (j, dist) in around(i) {
            if !reached[j] {
                reached[j] = true;
                if z[j] <= h {
                    z[j] = h + DRAIN_SLOPE * dist;
                }
                flood.push(Low(z[j], j));
            }
        }
    }
}
