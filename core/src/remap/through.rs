//! What a split or a join makes of the liquid a cell passes on (`World::liquid_flux`, laws.md 0): a flow, not a stock.
//! It runs down the terrain's channels (`space::network`): what drains through a part of the map is what its
//! drained area carries, so a cell passes on its drained area that leaves it (`Samples::leaving`) times its flux per m²
//! of that area. A quarter passes on its own drained area that leaves it at its parent's flux per m²; a joined cell
//! what leaves it, each quarter's drained area that leaves the joined cell at that quarter's flux per m². So a channel
//! through two quarters passes on once, and a split joined again gives back its cell's flux.

use super::sum4;
use crate::space::Leaf;
use crate::World;

impl World {
    /// The liquid each quarter of cell `i` (its leaf `l`) passes on, mol/s, row by row, as the terrain's channels lead
    /// the cell's own through them.
    pub(super) fn split_flux(&self, i: usize, l: Leaf) -> [f64; 4] {
        let (s, lat) = (self.terrain_samples(), (self.cfg.lat_south, self.cfg.lat_north));
        let per = self.liquid_flux[i] as f64 / s.leaving(&self.map, lat, l, l);
        l.quarters().map(|q| per * s.leaving(&self.map, lat, q, q))
    }

    /// The liquid the join of cells `kids` (the quarters of leaf `l`, row by row) passes on, mol/s: what leaves `l` of
    /// what each passes on, as the terrain's channels lead it.
    pub(super) fn join_flux(&self, kids: [usize; 4], l: Leaf) -> f64 {
        let (s, lat, q) = (self.terrain_samples(), (self.cfg.lat_south, self.cfg.lat_north), l.quarters());
        sum4(std::array::from_fn(|k| self.liquid_flux[kids[k]] as f64 / s.leaving(&self.map, lat, q[k], q[k]) * s.leaving(&self.map, lat, q[k], l)))
    }
}
