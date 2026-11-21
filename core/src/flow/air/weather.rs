//! The weather (laws.md 1.6): the synoptic eddies, which the air's step of 15 days cannot hold, as one random field of
//! the north-south displacement of the air they bring to each place, keyed by the seed, the day and the place, so it
//! holds no state and is the same on any count of threads. It acts in two places with no amplitude of its own: across
//! the map's open edges the planet's air is read where the eddies bring it from (`Edges::weather`), and the air an
//! eddy moves slides along the surfaces of equal warmth, so it rises or sinks (`Weather::lift`), drawing the low air of
//! the nodes about it in, or spreading its own out (`draw`).

use super::super::Scratch;
use super::cell::top_layer;
use super::rise::felt;
use super::stretch::Stretch;
use super::wind::End;
use crate::rng::{Purpose, Rng};
use crate::units::{DAY, DAY_S};
use crate::{World, AIR};

/// The e-folds of its time over which each part of the field sums the past days' draws (`series`): the weights it drops
/// are below e^-5 of the newest.
const FOLDS: f64 = 5.0;

/// The weather of one air step (`Weather::of`): on a lattice of points `eddy_size` apart on the planet, per point and
/// day of the step the field (`series`), and what a unit of it is: the displacement `reach`, m of the planet; and per
/// air node what the weather lifts its air by (`lift`).
pub(super) struct Weather {
    /// The lattice's first point [east, south] and its count of points each way
    origin: [i64; 2],
    size: [usize; 2],
    days: usize,
    /// Per point and day (`[(y * size[0] + x) * days + d]`)
    field: Vec<f64>,
    /// Lattice units per m of the map
    per_m: f64,
    reach: f64,
    lift: Vec<f64>,
}

impl Weather {
    /// The weather of the step of `days` days that starts now in `w`, its air nodes' ends `ends` (`wind::ends`), over
    /// the map and one node beyond its edges. Its unit is the eddies' mixing length sqrt(`mix` × `eddy_time`) (a
    /// diffusivity is a speed times the length an eddy moves air in its time, Taylor 1922; Prandtl 1925). None on a
    /// globe: the planet's strip is one (`worldgen::planet`), and the edges read its last year as the climate
    /// (`Planet`), which a year of its weather would stand for; and none where it acts on nothing, the map's edges closed
    /// and no node feeling a gradient (`lift`).
    pub(super) fn of(w: &World, s: &Scratch, ends: &[End], days: usize) -> Option<Weather> {
        let lift = lift(w, s, ends);
        if w.map.wrap || (!w.edges_open() && lift.iter().all(|&l| l == 0.0)) {
            return None;
        }
        let (air, phys) = (w.mesh.level(AIR), &w.cfg.phys);
        let per_m = Stretch::of(w).length(1.0) / phys.eddy_size;
        let beyond = air.area().iter().fold(0.0f64, |m, &a| m.max(a.sqrt()));
        let (wide, high) = (w.map.w as f64 * w.map.side, w.map.h as f64 * w.map.side);
        let origin = [(-beyond * per_m).floor() as i64, (-beyond * per_m).floor() as i64];
        let end = [((wide + beyond) * per_m).floor() as i64 + 1, ((high + beyond) * per_m).floor() as i64 + 1];
        let size = [(end[0] - origin[0] + 1) as usize, (end[1] - origin[1] + 1) as usize];
        let parts = Parts::of(w);
        let first = (w.time / DAY_S) as i64;
        let mut field = Vec::with_capacity(size[0] * size[1] * days);
        for y in 0..size[1] as i64 {
            for x in 0..size[0] as i64 {
                field.extend(parts.series(w.seed, [origin[0] + x, origin[1] + y], first, days));
            }
        }
        Some(Weather { origin, size, days, field, per_m, reach: (w.cfg.mix * phys.eddy_time).sqrt(), lift })
    }

    /// What the weather lifts each air node's air by on day `d` of the step, mol/s (sinking below 0): what it lifts it by
    /// per m (`lift`) times the air's displacement at its centre (`shift`).
    pub(super) fn lift(&self, w: &World, d: usize) -> Vec<f64> {
        let air = w.mesh.level(AIR);
        self.lift.iter().enumerate().map(|(j, l)| l * self.shift(air.centre()[j], d)).collect()
    }

    /// The displacement of the air at a point of the map, m [east, south] from its north-west corner, on day `d` of the
    /// step, m of the planet to the north it came from the south: the field of the four points of the lattice about it,
    /// bilinearly, its weights over their root sum of squares, so its spread is the same everywhere.
    pub(super) fn shift(&self, xy: [f64; 2], d: usize) -> f64 {
        let (u, v) = (xy[0] * self.per_m - self.origin[0] as f64, xy[1] * self.per_m - self.origin[1] as f64);
        let (ix, iy) = (u.floor() as usize, v.floor() as usize);
        let (fx, fy) = (u - ix as f64, v - iy as f64);
        let weights = [((0, 0), (1.0 - fx) * (1.0 - fy)), ((1, 0), fx * (1.0 - fy)), ((0, 1), (1.0 - fx) * fy), ((1, 1), fx * fy)];
        let norm = weights.iter().map(|e| e.1 * e.1).sum::<f64>().sqrt();
        let at = |(dx, dy): (usize, usize)| self.field[((iy + dy) * self.size[0] + ix + dx) * self.days + d];
        self.reach * weights.iter().map(|&(p, wt)| wt * at(p)).sum::<f64>() / norm
    }
}

/// What the weather lifts each air node's air by, mol/s per m of its displacement (`Weather::shift`): the air an eddy
/// moves in its time `eddy_time` slides along the surfaces of equal warmth, which rise toward the cold side by the slope
/// of the warmth's gradient the node feels (`rise::felt`, its part toward the south) over the warmth's rise with height
/// (`End::rise` over a mol's heat capacity: the dry air's fall with height less the node's), so air brought from the
/// warm side rises (the baroclinic eddy, Holton 2004, 8.4), at the ground's density (the column's mol over its scale
/// height), as the ground's rise lifts it (`rise::rise`). None where the warmth does not rise with height.
fn lift(w: &World, s: &Scratch, ends: &[End]) -> Vec<f64> {
    let felt = felt(w, ends);
    (0..ends.len())
        .map(|j| {
            let up = ends[j].rise * s.nt[j] / s.capa[j].max(f64::MIN_POSITIVE);
            if up > 0.0 { s.nt[j] / super::node_scale_height(s, j, w.cfg.gravity) * felt[j][1] / up / w.cfg.phys.eddy_time } else { 0.0 }
        })
        .collect()
}

/// The parts of the field (`Parts::of`): a fast one of the eddies' time and a slow one that a liquid's top layer sums.
struct Parts {
    fast: f64,
    slow: Option<(f64, f64)>,
}

impl Parts {
    /// The fast part keeps by e^(-1 / `eddy_time`) a day; the slow part is what the top layer of the key liquid sums of
    /// the same weather and lets go in τ = its heat per K (`cell::top_layer`) over `top_damp` (Hasselmann 1976;
    /// Frankignoul and Hasselmann 1977): forced by the fast part, its spread over the fast part's is sqrt(`eddy_time`
    /// / τ). None where the chemistry names no key liquid.
    fn of(w: &World) -> Parts {
        let phys = &w.cfg.phys;
        let slow = top_layer(w).map(|heat| heat / phys.top_damp).map(|tau| ((-DAY / tau).exp(), (phys.eddy_time / tau).sqrt()));
        Parts { fast: (-DAY / phys.eddy_time).exp(), slow }
    }

    /// The field at lattice point `p` on each of the `days` days from day `first`: per part the sum of the point's keyed
    /// daily draws (unit normals) over its last K days, weighted by its keep to the age of each, K for `FOLDS` e-folds,
    /// over its spread, so the part's spread is one and each day's value reads only that day's draws; the fast part plus
    /// the slow part times its gain.
    fn series(&self, seed: u64, p: [i64; 2], first: i64, days: usize) -> Vec<f64> {
        let reach = |keep: f64| (FOLDS / -keep.ln()).ceil() as usize;
        let longest = self.slow.map_or(reach(self.fast), |(keep, _)| reach(keep).max(reach(self.fast)));
        let node = ((p[0] as i32 as u32 as u64) << 32) | p[1] as i32 as u32 as u64;
        let draws: Vec<f64> = (0..longest + days - 1).map(|k| Rng::new(seed, (first - longest as i64 + 1 + k as i64) as u64, node, Purpose::Weather).normal()).collect();
        let mut out = summed(&draws, self.fast, reach(self.fast), days);
        if let Some((keep, gain)) = self.slow {
            out.iter_mut().zip(summed(&draws, keep, reach(keep), days)).for_each(|(v, s)| *v += gain * s);
        }
        out
    }
}

/// For each of the last `days` of `draws`, the sum of it and the `k - 1` before it, each weighted by `keep` to the
/// power of its age, over the spread of such a sum of unit normals.
fn summed(draws: &[f64], keep: f64, k: usize, days: usize) -> Vec<f64> {
    let weights: Vec<f64> = (0..k as i32).map(|age| keep.powi(age)).collect();
    let norm = weights.iter().map(|v| v * v).sum::<f64>().sqrt();
    (draws.len() - days..draws.len()).map(|t| weights.iter().enumerate().map(|(age, v)| v * draws[t - age]).sum::<f64>() / norm).collect()
}

#[cfg(test)]
mod tests {
    use super::Parts;

    /// The field holds no state: a day's value is the same whichever step reads it, and its fast part is a red noise of
    /// mean zero and spread one.
    #[test]
    fn the_weather_of_a_day_is_the_same_in_any_step() {
        let parts = Parts { fast: (-1.0f64 / 3.0).exp(), slow: Some(((-1.0f64 / 120.0).exp(), (3.0f64 / 120.0).sqrt())) };
        assert!(parts.series(7, [2, -1], 100, 30)[15..] == parts.series(7, [2, -1], 115, 15)[..], "a day reads only its own draws");
        let x = Parts { slow: None, ..parts }.series(7, [0, 0], 0, 100_000);
        let mean = x.iter().sum::<f64>() / x.len() as f64;
        let spread = (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt();
        assert!(mean.abs() < 0.03 && (spread - 1.0).abs() < 0.02, "mean {mean}, spread {spread}");
    }
}
