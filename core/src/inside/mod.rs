//! Inside a cell (handover 5.2, laws.md 0): a cell holds means (its cover, the liquid in its soil, the liquid that stands
//! on it), and a shell draws them inside the cell by general laws at each point of its ground: the soil's liquid by how
//! much liquid gathers there (`soil_level`, `soil_held`, on a `Grid` of samples), and where that saturates its land
//! (`wet_ground`), which the laws read. The ground inside a cell is the
//! terrain's worn samples inside it, on which its liquid stands (`flow::curve`).

mod shares;
mod wet;

pub use shares::{deficit_at, soil_held, soil_level, Means};
pub use wet::{aired, airless, channel_share, drains_source, flowing, parts_now, rest, saturated, saturated_share, wet_ground, wet_land, WetGround};

use crate::math::central;
use crate::World;

/// Net rain below this, mol/(m² s), counts as this in `World::catchment` (a cell that sheds nothing of its own, its
/// flux all from upstream).
const SHED_MIN: f64 = 1e-6;

/// The least slope of the ground's liquid table, rise over run: the floors of real valleys fall no less than about this
/// (lowland floodplains 1e-4 to 1e-3, Leopold, Wolman and Miller 1964), so the wetness index of a flat sample stays finite.
const TAN_MIN: f64 = 1e-3;

/// A grid of samples of the ground: `across` x `down` of them, row by row from the north-west, a sample's length north
/// to south, m, and the width of each row's samples, m. The ground inside a cell stands on one (`Ground`), and so does
/// a window a shell draws.
#[derive(Clone, Debug)]
pub struct Grid {
    across: usize,
    down: usize,
    dy: f64,
    dx: Vec<f64>,
}

impl World {
    /// The upstream area cell `i` drains, in units of its own: its liquid flux over what its own area sheds (rain less
    /// evaporation, per m²), 1 for a cell with only its own runoff. It does not depend on the cell's size.
    pub fn catchment(&self, i: usize) -> f64 {
        let shed = (self.rain[i] as f64 - self.evap[i] as f64).max(SHED_MIN) * self.mesh.cells().area()[i];
        self.liquid_flux.get(i).copied().unwrap_or(0.0) as f64 / shed
    }

    /// The terrain's worn samples of this world (`Terrain::samples`), which the ground inside each cell reads.
    pub fn terrain_samples(&self) -> std::sync::Arc<crate::space::samples::Samples> {
        self.terrain.samples(self.terrain.seed, &self.map, (self.cfg.lat_south, self.cfg.lat_north))
    }
}

impl Grid {
    /// A grid `across` x `down` of samples `dy` m long north to south, those of row r `dx[r]` m wide.
    pub fn new(across: usize, down: usize, dy: f64, dx: Vec<f64>) -> Grid {
        Grid { across, down, dy, dx }
    }

    /// The slope of the ground at each sample, rise over run, east and south: central differences, one-sided at the
    /// grid's edges.
    fn gradient(&self, h: &[f64]) -> Vec<[f64; 2]> {
        let (w, d) = (self.across, self.down);
        let at = |r: usize, c: usize| h[r * w + c];
        (0..w * d)
            .map(|p| {
                let (r, c) = (p / w, p % w);
                [central(c, w, |x| at(r, x), self.dx[r]), central(r, d, |y| at(y, c), self.dy)]
            })
            .collect()
    }

    /// The slope of ground `h` at each sample, rise over run, no less than a valley floor's (`TAN_MIN`).
    pub fn slope(&self, h: &[f64]) -> Vec<f64> {
        self.gradient(h).iter().map(|g| g[0].hypot(g[1]).max(TAN_MIN)).collect()
    }

    /// How much liquid gathers at each sample, the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979), from
    /// the area that drains through each, m², over the width of a sample (the root of its area `own`, m²),
    /// and its slope (`slope`).
    pub fn wetness_index(drains: &[f64], own: &[f64], tan: &[f64]) -> Vec<f64> {
        (0..drains.len()).map(|p| (drains[p] / own[p].sqrt() / tan[p]).ln()).collect()
    }
}
