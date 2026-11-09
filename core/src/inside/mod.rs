//! Inside a cell (handover 5.2, laws.md 0): a cell holds means (its cover, the liquid in its soil, the liquid that stands
//! on it). Each is shared over the cell's own ground by a general law evaluated at a grid of samples inside the cell,
//! `DEPTHS` below it (`Ground`): the soil's liquid by its steady flow downhill, what stands by its level, each kind of
//! cover by its own growth law there (`Inside`). Each share keeps its mean: its mean over the cell by area is 1. A split
//! gives each quarter the share of its own ground (`World::split`), and a shell draws inside a cell with the same
//! shares. What the terrain alone gives (the slopes, where the liquid runs) is found once, when the ground is made.

mod shares;

pub use shares::{Inside, Means};

use crate::math::central;
use crate::space::terrain::real_slope;
use crate::space::tree::DEPTH_MOST;
use crate::space::Leaf;
use crate::World;

/// The samples inside a cell are the leaves this many depths below it (no deeper than the tree's most): a grid of 8 x 8,
/// on which a hollow, a ridge and the slope between them each take a few samples.
pub const DEPTHS: u8 = 3;

/// Net rain below this, mol/(m² s), counts as this in `World::catchment` (a cell that sheds nothing of its own, its
/// flux all from upstream).
const SHED_MIN: f64 = 1e-6;

/// Quinn's contour lengths, in sides of a sample, of the ways to the samples across a side and across a corner: the
/// share of a sample's liquid that goes each way down weighs its drop by them (Quinn et al. 1991).
const CONTOUR: [f64; 2] = [0.5, std::f64::consts::SQRT_2 / 4.0];

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
/// its height, m, and its share of the cell's area; its grid of samples (as many along a side as down it) and the
/// cell's area, m². And what the terrain alone gives, found when it is made (`Ground::found`): per sample the area
/// of the cell that drains through it, m², the share of what comes from upstream that drains through it, and its
/// slope, rise over run.
#[derive(Clone, Debug)]
pub struct Ground {
    pub h: Vec<f64>,
    pub area: Vec<f64>,
    grid: Grid,
    whole: f64,
    drains: Vec<f64>,
    through: Vec<f64>,
    tan: Vec<f64>,
}

impl World {
    /// The upstream area cell `i` drains, in units of its own: its liquid flux over what its own area sheds (rain less
    /// evaporation, per m²), 1 for a cell with only its own runoff. It does not depend on the cell's size.
    pub fn catchment(&self, i: usize) -> f64 {
        let shed = (self.rain[i] as f64 - self.evap[i] as f64).max(SHED_MIN) * self.mesh.cells().area()[i];
        self.liquid_flux.get(i).copied().unwrap_or(0.0) as f64 / shed
    }

    /// The area that drains to cell `i` from upstream, m²: that of its `catchment` less its own.
    pub fn inflow(&self, i: usize) -> f64 {
        (self.catchment(i) - 1.0).max(0.0) * self.mesh.cells().area()[i]
    }

    /// The shares inside cell `i` (`Inside`) of the means `m`, on its ground (`ground_in`) with the departures scaled
    /// by `s` (`Ground::scaled`), with what drains to it from upstream (`inflow`).
    pub fn inside(&self, i: usize, s: f64, m: Means) -> (Ground, Inside) {
        let g = self.ground_in(self.map.leaves()[i]).scaled(s);
        let shares = Inside::of(&g, &self.cfg.phys, self.inflow(i), m);
        (g, shares)
    }

    /// The terrain's drained samples of this world (`Terrain::samples`), which the ground inside each cell reads.
    pub fn drained(&self) -> std::sync::Arc<crate::space::samples::Samples> {
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
        Ground { h, area, grid: Grid::new(side, side, dy, dx), whole, drains: Vec::new(), through: Vec::new(), tan: Vec::new() }.found()
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

    /// The samples next to sample `p` (across its sides and corners, on the grid), each with whether it lies across a
    /// corner and its distance, m.
    fn around(&self, p: usize) -> impl Iterator<Item = (usize, bool, f64)> + '_ {
        let (w, h) = (self.across as i64, self.down as i64);
        let (r, c) = (p as i64 / w, p as i64 % w);
        (-1..=1i64).flat_map(move |dr| (-1..=1i64).map(move |dc| (dr, dc))).filter_map(move |(dr, dc)| {
            let (r2, c2) = (r + dr, c + dc);
            ((dr, dc) != (0, 0) && (0..h).contains(&r2) && (0..w).contains(&c2)).then(|| {
                let dx = (self.dx[r as usize] + self.dx[r2 as usize]) / 2.0;
                ((r2 * w + c2) as usize, dr != 0 && dc != 0, (dx * dc as f64).hypot(self.dy * dr as f64))
            })
        })
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

    /// What gathers at each sample of ground `h` when `gathered` falls on the samples and runs down by multiple flow directions
    /// (Quinn et al. 1991): each sample, highest first, passes what it gathered to every lower sample next to it by drop
    /// over distance times contour length (`CONTOUR`), so it spreads over a convex ridge and gathers in a concave hollow;
    /// a sample with no lower one keeps it (a hollow, or the edge it leaves by). It is the steady flow of the soil's
    /// liquid downhill by Darcy's law, which follows the ground's slope, spread as slow flow through the soil spreads;
    /// the liquid law's one steepest way (D-infinity) suits channels.
    pub fn drain(&self, h: &[f64], mut gathered: Vec<f64>) -> Vec<f64> {
        let mut order: Vec<usize> = (0..h.len()).collect();
        order.sort_by(|&p, &q| h[q].total_cmp(&h[p]).then(p.cmp(&q)));
        for &p in &order {
            let ways: Vec<(usize, f64)> = self.around(p).filter(|&(q, _, _)| h[q] < h[p]).map(|(q, corner, d)| (q, (h[p] - h[q]) / d * CONTOUR[corner as usize])).collect();
            let sum: f64 = ways.iter().map(|w| w.1).sum();
            if sum > 0.0 {
                let a = gathered[p];
                ways.iter().for_each(|&(q, w)| gathered[q] += a * w / sum);
            }
        }
        gathered
    }

    /// The slope of ground `h` at each sample, rise over run, no less than real ground's over a sample (`real_slope`).
    pub fn slope(&self, h: &[f64]) -> Vec<f64> {
        let floor = real_slope(self.dy);
        self.gradient(h).iter().map(|g| g[0].hypot(g[1]).max(floor)).collect()
    }

    /// How much liquid gathers at each sample, the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979), from
    /// the area that drains through each, m² (`drain`), over the width of a sample (the root of its area `own`, m²),
    /// and its slope (`slope`).
    pub fn wetness_index(drains: &[f64], own: &[f64], tan: &[f64]) -> Vec<f64> {
        (0..drains.len()).map(|p| (drains[p] / own[p].sqrt() / tan[p]).ln()).collect()
    }
}

impl Ground {
    /// The same ground with its heights' departures from their mean scaled by `s` (a split that cannot pay for the whole
    /// terrain inside its cell scales it so, `World::uneven`; 0 is flat).
    pub fn scaled(mut self, s: f64) -> Ground {
        if s == 1.0 {
            return self;
        }
        let mean: f64 = self.h.iter().zip(&self.area).map(|(h, a)| h * a).sum();
        self.h.iter_mut().for_each(|h| *h = mean + s * (*h - mean));
        self.found()
    }

    /// The ground with what its terrain alone gives found (`Ground`): the area that drains through each sample from the
    /// cell's own area and the share of what comes from upstream that does (`drain`; from upstream it enters at the
    /// lowest sample of the grid's highest side, where a channel comes in, and spreads evenly on flat ground), and each
    /// sample's slope, no less than real ground's over a sample (`real_slope`).
    fn found(mut self) -> Ground {
        let flat = self.h.iter().all(|&x| x == self.h[0]);
        let mut entry = vec![0.0; self.h.len()];
        if flat {
            entry.clone_from(&self.area);
        } else {
            entry[self.entry()] = 1.0;
        }
        self.drains = self.grid.drain(&self.h, self.area.iter().map(|a| a * self.whole).collect());
        self.through = self.grid.drain(&self.h, entry);
        self.tan = self.grid.slope(&self.h);
        self
    }

    /// The pairs of samples beside each other across a side, on the grid.
    pub(crate) fn sides(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.grid.sides()
    }

    /// How much liquid gathers at each sample: the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979), a
    /// the area that drains through the sample per metre of contour (its own and all upslope of it, and its share of
    /// the `inflow` m² that drains to the cell from upstream, over the width of a sample) and β its slope (`found`).
    pub fn wetness_index(&self, inflow: f64) -> Vec<f64> {
        let drains: Vec<f64> = (0..self.h.len()).map(|p| self.drains[p] + inflow * self.through[p]).collect();
        let own: Vec<f64> = self.area.iter().map(|a| a * self.whole).collect();
        Grid::wetness_index(&drains, &own, &self.tan)
    }

    /// The lowest sample of the highest of the grid's four sides (north, east, south, west; ties the first).
    fn entry(&self) -> usize {
        let (n, last) = (self.grid.across, self.grid.across - 1);
        let sides: [Vec<usize>; 4] = [(0..n).collect(), (0..n).map(|r| r * n + last).collect(), (0..n).map(|c| last * n + c).collect(), (0..n).map(|r| r * n).collect()];
        let mean = |s: &[usize]| s.iter().map(|&p| self.h[p]).sum::<f64>() / s.len() as f64;
        let high = sides.iter().fold(&sides[0], |best, s| if mean(s) > mean(best) { s } else { best });
        high.iter().copied().fold(high[0], |low, p| if self.h[p] < self.h[low] { p } else { low })
    }

    /// The depth over each sample of liquid that stands `depth` m deep over the cell at one level, m: it fills the
    /// lowest samples first (`flow::level_of`), each as deep as it lies under the level.
    pub(crate) fn standing(&self, depth: f64) -> Vec<f64> {
        if depth <= 0.0 {
            return vec![0.0; self.h.len()];
        }
        let mut floors: Vec<(f64, f64)> = self.h.iter().copied().zip(self.area.iter().copied()).collect();
        let top = crate::flow::level_of(&mut floors, depth);
        self.h.iter().map(|h| (top - h).max(0.0)).collect()
    }

    /// The mean of `x` over the cell by area.
    pub fn mean(&self, x: &[f64]) -> f64 {
        x.iter().zip(&self.area).map(|(v, a)| v * a).sum()
    }

    /// `x` as shares: over its mean by area, so their mean is 1 (all 1 where it is nothing).
    pub fn share(&self, x: Vec<f64>) -> Vec<f64> {
        let mean = self.mean(&x);
        if mean > 0.0 { x.into_iter().map(|v| v / mean).collect() } else { vec![1.0; x.len()] }
    }

    /// The part of the cell's whole that each of its quarters (row by row) holds where the whole lies by the shares
    /// `share` (`Inside`: none below 0, and not all 0): the sum by area of the shares of its samples over the sum of all,
    /// so the four parts sum to 1.
    pub fn quarters(&self, share: &[f64]) -> [f64; 4] {
        let (n, half) = (self.grid.across, self.grid.across / 2);
        let mut q = [0.0; 4];
        for p in 0..n * n {
            q[(p / n >= half) as usize * 2 + (p % n >= half) as usize] += self.area[p] * share[p];
        }
        let sum = crate::remap::sum4(q);
        q.map(|x| x / sum)
    }
}
