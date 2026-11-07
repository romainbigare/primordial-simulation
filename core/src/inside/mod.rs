//! Inside a cell (handover 5.2, laws.md 0): a cell holds means (its cover, the liquid in its soil, its snow, the liquid
//! that stands on it). One rule shares each mean over the cell's own ground, from the ground's shape alone: the terrain
//! at a grid of samples inside the cell, `DEPTHS` below it (`Ground`), the liquid that comes from upstream, and the cell's
//! own means (`Inside`). Each share keeps its mean: its mean over the cell by area is 1. A split gives each quarter the
//! share of its own ground (`World::split`), and a shell draws inside a cell with the same shares.

mod shares;

pub use shares::{Inside, Means};

use crate::space::terrain::{real_slope, SAMPLE_DEPTH};
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

/// The ground inside a cell as the terrain stands there (`World::ground_in`): per sample, row by row from the north-west,
/// its height, m, and its share of the cell's area; the cell's leaf, the samples along its side and its area, m²; a
/// sample's length north to south, m, and the width of each row's samples, m; the latitude of the cell's centre, degrees.
#[derive(Clone, Debug)]
pub struct Ground {
    pub h: Vec<f64>,
    pub area: Vec<f64>,
    leaf: Leaf,
    side: usize,
    whole: f64,
    dy: f64,
    dx: Vec<f64>,
    lat: f64,
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

    /// The ground inside leaf `l` (`Ground`): the terrain's height over each sample, a leaf `DEPTHS` below `l`
    /// (`Terrain::inside`, its mean there; the height at its centre where it is coarser than the terrain's samples).
    pub fn ground_in(&self, l: Leaf) -> Ground {
        let lat = (self.cfg.lat_south, self.cfg.lat_north);
        let depth = (l.depth + DEPTHS).min(DEPTH_MOST);
        let side = 1usize << (depth - l.depth);
        let (mut h, mut area) = (Vec::with_capacity(side * side), Vec::with_capacity(side * side));
        for s in l.split(depth) {
            let (a, at) = self.map.place_of(&s, lat.0, lat.1);
            h.push(if depth >= SAMPLE_DEPTH { self.terrain.inside(self.seed, &self.map, s, lat).0 } else { self.terrain.height(self.seed, &self.map, at) });
            area.push(a);
        }
        let dy = self.map.length(depth);
        let dx = (0..side).map(|r| area[r * side] / dy).collect();
        let whole: f64 = area.iter().sum();
        area.iter_mut().for_each(|a| *a /= whole);
        Ground { h, area, leaf: l, side, whole, dy, dx, lat: self.map.place_of(&l, lat.0, lat.1).1.lat }
    }
}

impl Ground {
    /// The same ground with its heights' departures from their mean scaled by `s` (a split that cannot pay for the whole
    /// terrain inside its cell scales it so, `World::uneven`; 0 is flat).
    pub fn scaled(mut self, s: f64) -> Ground {
        let mean: f64 = self.h.iter().zip(&self.area).map(|(h, a)| h * a).sum();
        self.h.iter_mut().for_each(|h| *h = mean + s * (*h - mean));
        self
    }

    /// The samples next to sample `p` (across its sides and corners, on the grid), each with whether it lies across a
    /// corner and its distance, m.
    fn around(&self, p: usize) -> impl Iterator<Item = (usize, bool, f64)> + '_ {
        let n = self.side as i64;
        let (r, c) = (p as i64 / n, p as i64 % n);
        (-1..=1i64).flat_map(move |dr| (-1..=1i64).map(move |dc| (dr, dc))).filter_map(move |(dr, dc)| {
            let (r2, c2) = (r + dr, c + dc);
            ((dr, dc) != (0, 0) && (0..n).contains(&r2) && (0..n).contains(&c2)).then(|| {
                let dx = (self.dx[r as usize] + self.dx[r2 as usize]) / 2.0;
                ((r2 * n + c2) as usize, dr != 0 && dc != 0, (dx * dc as f64).hypot(self.dy * dr as f64))
            })
        })
    }

    /// The slope of the ground at each sample, rise over run, east and south: central differences, one-sided at the
    /// grid's edges.
    fn gradient(&self) -> Vec<[f64; 2]> {
        let n = self.side;
        let at = |r: usize, c: usize| self.h[r * n + c];
        // The slope from `lo` to `hi` along a line of samples `len` apart (none on a grid of one sample).
        let along = |lo: usize, hi: usize, f: &dyn Fn(usize) -> f64, len: f64| if hi > lo { (f(hi) - f(lo)) / (len * (hi - lo) as f64) } else { 0.0 };
        (0..n * n)
            .map(|p| {
                let (r, c) = (p / n, p % n);
                let (c0, c1, r0, r1) = (c.saturating_sub(1), (c + 1).min(n - 1), r.saturating_sub(1), (r + 1).min(n - 1));
                [along(c0, c1, &|x| at(r, x), self.dx[r]), along(r0, r1, &|y| at(y, c), self.dy)]
            })
            .collect()
    }

    /// How much liquid gathers at each sample: the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979), a
    /// the area that drains through the sample per metre of contour (its own and all upslope of it, over the width of a
    /// sample) and β its slope, no less than real ground's over a sample (`real_slope`). The area drains by multiple
    /// flow directions (Quinn et al. 1991): each sample, highest first, passes what it gathered to every lower sample
    /// next to it by drop over distance times contour length (`CONTOUR`), so liquid spreads over a convex ridge and
    /// gathers in a concave hollow (the curve of the ground lies in a); it spreads the soil's slow liquid as that liquid
    /// moves, where one steepest way (D-infinity) suits the channels of the liquid law between cells. What drains to the
    /// cell from upstream, `inflow` m², enters at the lowest sample of the cell's highest side, where a channel comes in,
    /// and runs down the low ground from there (on flat ground it spreads evenly). A sample with no lower one keeps what
    /// it gathered (a hollow, or the edge liquid leaves by).
    pub fn wetness_index(&self, inflow: f64) -> Vec<f64> {
        let n = self.h.len();
        let flat = self.h.iter().all(|&x| x == self.h[0]);
        let mut gathered: Vec<f64> = self.area.iter().map(|a| (self.whole + if flat { inflow } else { 0.0 }) * a).collect();
        if !flat {
            gathered[self.entry()] += inflow;
        }
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&p, &q| self.h[q].total_cmp(&self.h[p]).then(p.cmp(&q)));
        for &p in &order {
            let ways: Vec<(usize, f64)> = self.around(p).filter(|&(q, _, _)| self.h[q] < self.h[p]).map(|(q, corner, d)| (q, (self.h[p] - self.h[q]) / d * CONTOUR[corner as usize])).collect();
            let sum: f64 = ways.iter().map(|w| w.1).sum();
            if sum > 0.0 {
                let a = gathered[p];
                ways.iter().for_each(|&(q, w)| gathered[q] += a * w / sum);
            }
        }
        let floor = real_slope(self.dy);
        let grad = self.gradient();
        (0..n).map(|p| (gathered[p] / (self.area[p] * self.whole).sqrt() / grad[p][0].hypot(grad[p][1]).max(floor)).ln()).collect()
    }

    /// The lowest sample of the highest of the grid's four sides (north, east, south, west; ties the first).
    fn entry(&self) -> usize {
        let (n, last) = (self.side, self.side - 1);
        let sides: [Vec<usize>; 4] = [(0..n).collect(), (0..n).map(|r| r * n + last).collect(), (0..n).map(|c| last * n + c).collect(), (0..n).map(|r| r * n).collect()];
        let mean = |s: &[usize]| s.iter().map(|&p| self.h[p]).sum::<f64>() / s.len() as f64;
        let high = sides.iter().fold(&sides[0], |best, s| if mean(s) > mean(best) { s } else { best });
        high.iter().copied().fold(high[0], |low, p| if self.h[p] < self.h[low] { p } else { low })
    }

    /// The direct sun on each sample, over its mean over the cell: the cosine of the angle between the sample's ground
    /// and the noon sun at the cell's latitude when the sun stands over the equator (its mean height through the year),
    /// none on a face turned from it. A face toward the sun takes more, one turned away less.
    pub(crate) fn sun(&self) -> Vec<f64> {
        let (zenith, toward) = (self.lat.abs().to_radians(), self.lat.signum());
        let lit: Vec<f64> = (self.gradient().iter())
            .map(|g| ((zenith.cos() - g[1] * toward * zenith.sin()) / (1.0 + g[0] * g[0] + g[1] * g[1]).sqrt()).max(0.0))
            .collect();
        let mean = self.mean(&lit);
        lit.iter().map(|&x| if mean > 0.0 { x / mean } else { 1.0 }).collect()
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
        let (n, half) = (self.side, self.side / 2);
        let mut q = [0.0; 4];
        for p in 0..n * n {
            q[(p / n >= half) as usize * 2 + (p % n >= half) as usize] += self.area[p] * share[p];
        }
        let sum = crate::remap::sum4(q);
        q.map(|x| x / sum)
    }

    /// The mean by area of `field` (a number per sample) over the samples that the span from `x0` up to `x1` and from
    /// `y0` up to `y1` covers (in leaves of the tree's most depth, `x1` and `y1` outside it), as a shell reads it for a
    /// pixel: the one sample that holds a finer span, the mean of those it covers. A span beside the cell reads its
    /// nearest samples.
    pub fn over(&self, [x0, y0, x1, y1]: [u64; 4], field: &[f64]) -> f64 {
        let (l, n) = (&self.leaf, self.side as u64);
        let step = l.side() / n;
        let at = |from: u64, a: u64| (a.saturating_sub(from) / step).min(n - 1);
        let (c0, c1, r0, r1) = (at(l.x, x0), at(l.x, x1.saturating_sub(1).max(x0)), at(l.y, y0), at(l.y, y1.saturating_sub(1).max(y0)));
        let (mut sum, mut of) = (0.0, 0.0);
        for p in (r0..=r1).flat_map(|r| (c0..=c1).map(move |c| (r * n + c) as usize)) {
            (sum, of) = (sum + self.area[p] * field[p], of + self.area[p]);
        }
        sum / of
    }
}
