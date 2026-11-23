//! Heat added under a brush (handover.md 11: "Set on fire: add a lot of heat to nodes in an area; burning comes from
//! the chemistry"). The heat comes from the outside node into each cell's ground node; the flames take it to dry and
//! light the remains lying there (`flame::heated`) when the edits settle or the month runs.

use super::Brush;
use crate::store::{Boundary, Node};
use crate::World;
use std::collections::BTreeMap;

impl World {
    /// `q` J a m² of heat added under the brush `b` at its centre, by (1 − (d/r)²)² of that at a distance d from it (r
    /// its radius; as `shape`), over the samples it covers (`covered`): each land cell takes the heat its samples take,
    /// from the outside node into its ground node, kept for its flames with the share of it the brush covers
    /// (`World::heated`; a cell heated again adds to it, over the larger share). Whether any heat was added.
    pub(crate) fn heat(&mut self, b: Brush, q: f64) -> bool {
        let s = self.terrain_samples();
        let mut cells: BTreeMap<usize, f64> = BTreeMap::new();
        for c in self.covered(b, s.across) {
            *cells.entry(c.cell).or_default() += q * (1.0 - c.d * c.d).powi(2) * s.area(c.k);
        }
        let over: BTreeMap<usize, f64> = self.under(b).into_iter().map(|u| (u.cell, u.share)).collect();
        let mut added = false;
        let land: Vec<(usize, f64)> = cells.into_iter().filter(|&(i, e)| e > 0.0 && crate::contract::land(self, i) > 0.0).collect();
        for (i, e) in land {
            self.cross((Boundary::Outside, Node::ground(i)), [], e);
            let share = over.get(&i).copied().unwrap_or(0.0);
            match self.heated.iter_mut().find(|h| h.0 == i as u32) {
                Some(h) => h.1 = [h.1[0] + e, h.1[1].max(share)],
                None => self.heated.push((i as u32, [e, share])),
            }
            added = true;
        }
        added
    }
}
