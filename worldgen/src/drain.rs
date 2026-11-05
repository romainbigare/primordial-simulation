//! The starting terrain drains: a priority flood fills each closed basin to its outlet.

use pcore::space::Level;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The least slope of ground raised to fill a closed basin, m per m: liquid still runs across the filled floor
/// (Planchon and Darboux 2001).
const DRAIN_SLOPE: f64 = 1e-3;

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

/// Give every cell of `cells` a way down to the low ground: a priority flood from it, the lowest cell reached first,
/// raises each new neighbour no higher than it to its height plus `DRAIN_SLOPE` times their distance (Barnes, Lehman
/// and Mulla 2014); a higher neighbour already drains through it. Noise has closed basins that no liquid ever cut; real wet land drains. On a globe (`wrap`) the low ground is
/// every cell below 0 m; on an island, the cells below 0 m joined to its lowest cell, so ground below 0 m inland is a
/// closed basin too.
pub(crate) fn drain(cells: &Level, z: &mut [f64], wrap: bool) {
    let mut reached: Vec<bool> = z.iter().map(|&h| h < 0.0 && wrap).collect();
    let lowest = (0..z.len()).min_by(|&a, &b| z[a].total_cmp(&z[b])).filter(|&i| !wrap && z[i] < 0.0);
    let mut low: Vec<usize> = lowest.into_iter().collect();
    low.iter().for_each(|&i| reached[i] = true);
    while let Some(i) = low.pop() {
        for &j in cells.around(i).1 {
            if z[j as usize] < 0.0 && !reached[j as usize] {
                reached[j as usize] = true;
                low.push(j as usize);
            }
        }
    }
    let mut flood: BinaryHeap<Low> = (0..z.len()).filter(|&i| reached[i]).map(|i| Low(z[i], i)).collect();
    while let Some(Low(h, i)) = flood.pop() {
        let (links, near) = cells.around(i);
        for (&l, &j) in links.iter().zip(near) {
            let j = j as usize;
            if !reached[j] {
                reached[j] = true;
                if z[j] <= h {
                    z[j] = h + DRAIN_SLOPE * cells.dist()[l as usize];
                }
                flood.push(Low(z[j], j));
            }
        }
    }
}
