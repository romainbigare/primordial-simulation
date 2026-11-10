//! Inside a cell (handover 5.2, laws.md 0): a cell holds means (its cover, the liquid in its soil, the liquid that stands
//! on it), and a shell draws them inside the cell by general laws at each point of its ground: the soil's liquid by how
//! much liquid gathers there (`soil_level`, `soil_held`, on a `Grid` of samples). The ground inside a cell is the
//! terrain at a grid of samples `DEPTHS` below it (`Ground`), on which its liquid stands (`flow::curve`).

mod shares;

pub use shares::{soil_held, soil_level, Means};

use crate::math::central;
use crate::space::tree::DEPTH_MOST;
use crate::space::Leaf;
use crate::World;

/// The samples inside a cell are the leaves this many depths below it (no deeper than the tree's most): a grid of 8 x 8,
/// on which a hollow, a ridge and the slope between them each take a few samples.
pub const DEPTHS: u8 = 3;

/// Net rain below this, mol/(m² s), counts as this in `World::catchment` (a cell that sheds nothing of its own, its
/// flux all from upstream).
const SHED_MIN: f64 = 1e-6;

/// The rms slope of real ground (rise over run) measured over `REAL_LENGTH_M`, and the Hurst exponent of its roughness:
/// the slope over a length ℓ goes as ℓ^(`HURST` − 1) (`real_slope`).
const REAL_SLOPE: f64 = 0.1;
const REAL_LENGTH_M: f64 = 30.0;
const HURST: f64 = 0.8;

/// The rms slope of real ground over a length `len`, m: self-affine, so it falls slowly as the length grows. The
/// ground inside a cell is no smoother than it at the size of its samples.
fn real_slope(len: f64) -> f64 {
    REAL_SLOPE * (len / REAL_LENGTH_M).powf(HURST - 1.0)
}

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

/// The ground inside a cell as the terrain stands there (`World::ground_in`): per sample, row by row from the north-west,
/// its height, m, and its share of the cell's area; and its grid of samples (as many along a side as down it).
#[derive(Clone, Debug)]
pub struct Ground {
    pub h: Vec<f64>,
    pub area: Vec<f64>,
    grid: Grid,
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

    /// The ground inside leaf `l` (`Ground`): the terrain's mean height over each sample, a leaf `DEPTHS` below `l`
    /// (`Terrain::heights`, the ground a shell draws too).
    pub fn ground_in(&self, l: Leaf) -> Ground {
        let lat = (self.cfg.lat_south, self.cfg.lat_north);
        let depth = (l.depth + DEPTHS).min(DEPTH_MOST);
        let side = 1usize << (depth - l.depth);
        let (x, y) = ((l.x / l.side()) as i64 * side as i64, (l.y / l.side()) as i64 * side as i64);
        let h = self.terrain.heights(self.terrain.seed, &self.map, lat, [x, y, x + side as i64 - 1, y + side as i64 - 1], depth);
        let mut area: Vec<f64> = l.split(depth).map(|s| self.map.place_of(&s, lat.0, lat.1).0).collect();
        let dy = self.map.length(depth);
        let dx = (0..side).map(|r| area[r * side] / dy).collect();
        let whole: f64 = area.iter().sum();
        area.iter_mut().for_each(|a| *a /= whole);
        Ground { h, area, grid: Grid::new(side, side, dy, dx) }
    }
}

impl Grid {
    /// A grid `across` x `down` of samples `dy` m long north to south, those of row r `dx[r]` m wide.
    pub fn new(across: usize, down: usize, dy: f64, dx: Vec<f64>) -> Grid {
        Grid { across, down, dy, dx }
    }

    /// The pairs of samples beside each other across a side.
    fn sides(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let (w, n) = (self.across, self.across * self.down);
        (0..n).flat_map(move |p| [(p % w + 1 < w).then(|| (p, p + 1)), (p + w < n).then(|| (p, p + w))]).flatten()
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

    /// The slope of ground `h` at each sample, rise over run, no less than real ground's over a sample (`real_slope`).
    pub fn slope(&self, h: &[f64]) -> Vec<f64> {
        let floor = real_slope(self.dy);
        self.gradient(h).iter().map(|g| g[0].hypot(g[1]).max(floor)).collect()
    }

    /// How much liquid gathers at each sample, the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979), from
    /// the area that drains through each, m², over the width of a sample (the root of its area `own`, m²),
    /// and its slope (`slope`).
    pub fn wetness_index(drains: &[f64], own: &[f64], tan: &[f64]) -> Vec<f64> {
        (0..drains.len()).map(|p| (drains[p] / own[p].sqrt() / tan[p]).ln()).collect()
    }
}

impl Ground {
    /// The pairs of samples beside each other across a side, on the grid.
    pub(crate) fn sides(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.grid.sides()
    }

    /// The mean of `x` over the cell by area.
    pub fn mean(&self, x: &[f64]) -> f64 {
        x.iter().zip(&self.area).map(|(v, a)| v * a).sum()
    }
}
