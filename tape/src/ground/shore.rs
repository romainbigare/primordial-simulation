//! The shore as a shell draws it: where the ground crosses the level of the liquid beside it. The terrain's samples
//! stand 125 m apart, and along a coast they lie over and under the level by turns, so the line where the ground drawn
//! between them crosses the level is a chain of scallops a sample wide, and the shallows' depth jumps from sample to
//! sample. The shore is drawn instead where a smooth field crosses 0: each sample's height over the level of the
//! nearest pond (`level`), held within `SHORE_M` either side of it, blurred over the samples by a Gaussian of
//! `SMOOTH_SAMPLES` (`offset`). Its zero line is a long smooth line with capes and bays as large as the samples' coast
//! has over a few samples, nothing a sample wide (a contour of a smoothed field, the same idea as cutting a polygon's
//! corners, Chaikin 1974); held evenly either side, it keeps the land's area of a straight coast. The ground drawn
//! keeps its own height where it lies on the offset's side of the level, and is lifted or lowered to that side by
//! `SIDE` of the offset where it does not; a closed pond's deep samples stay under it (`KEEP_M`). Over the level the
//! ground leans to the level plus the offset at the shore and takes its own height from `LEAN_M` of offset on, so ground
//! that stands high by the water rises from the shore as a slope, not a wall at the zero line. Under the level the
//! bed leans to a smooth depth, the offset at the shore and farther out the samples' depth held within `WATER_M` and
//! blurred alike (`depth`), all of it in the shallows and less the deeper, so the shallows' colour follows the coast as
//! one band. Liquid may stand on a texel beside a pond where the offset drawn lies under `EDGE_M`, and the ground drawn
//! there lies over the level wherever the offset does (`drawn`): the edge of where it may stand is dry ground, a smooth
//! line of the place, so no tile at any depth draws an edge of liquid along its texels or the samples. Each number is a
//! function of the samples alone.

use super::bends::ease;
use pcore::space::network::NONE;
use pcore::space::samples::Samples;

/// The level of the open low ground (the sea), m: the shelf is laid about it (`space::shelf`).
const SEA_M: f64 = 0.0;
/// A sample's offset is held within this height either side of the level, m: the samples nearer the level than this
/// decide where the shore runs by their height, those farther by their side alone.
const SHORE_M: f64 = 4.0;
/// A sample's depth is held within this depth under the level, m: the shallows down to it are drawn smooth, so their
/// colour follows the coast as one band: the bed of the clearest water shows to tens of metres (`style.wgsl`).
const WATER_M: f64 = 40.0;
/// The Gaussian's standard deviation, samples, and the samples it reaches each way: a wave along the coast one sample
/// long keeps 1e-7 of itself, one of two samples 2%, one of five 50%.
const SMOOTH_SAMPLES: f64 = 0.9;
const SMOOTH_REACH: i64 = 3;
/// The samples about a sample in which its nearest pond is found, each way.
const NEAR_SAMPLES: i64 = 2;
/// The ground drawn lies on the offset's side of the level by at least this share of the offset, all of it to
/// `SIDE_FROM` of `SHORE_M` and none at `SHORE_M`, where the ground lies on that side by itself.
const SIDE: f64 = 0.5;
const SIDE_FROM: f64 = 0.5;
/// Over the level the ground drawn leans to the level plus the offset at the shore, all of it at the zero line and none
/// from this offset on, m: the bed leans to the offset there too, so the ground is one smooth surface across the shore.
const LEAN_M: f64 = 1.0;
/// A sample of a closed pond deeper than this under its level keeps its liquid however small the pond, m: its offset is
/// no more than minus `KEPT` times `SHORE_M`, so the spline through it lies under 0 at its centre even among land held
/// at `SHORE_M` (its own weight there is 4/9), and no lake is blurred away (a sea's islet of a sample may be).
const KEEP_M: f64 = 3.0;
const KEPT: f64 = 3.0;
/// A channel's line that ends in standing liquid ends where the shore drawn puts that liquid: the samples' shore and
/// the smooth one lie up to a sample or two apart, so from the point where the samples end the line it runs on down
/// the offset's slope, in steps of `MOUTH_STEP` samples, to where the offset lies `MOUTH_UNDER_M` under the level, no
/// farther than `MOUTH_MOST` samples.
const MOUTH_UNDER_M: f64 = 0.5;
const MOUTH_MOST: f64 = 4.5;
const MOUTH_STEP: f64 = 0.1;
/// Standing liquid drowns the mouth of a channel that runs into it: a channel's bed lies under its banks, so at the
/// shore the offset along a channel's line lies lower by `MOUTH_DROWN` bed depths (its bars and its banks' feet go
/// under: shallows, not a deep cut), all of it within the bed and none from `MOUTH_WIDE` of its half widths from the
/// line, and less with the way from the shore, none from `MOUTH_LONG` bed widths up the channel or out into the
/// liquid. The shore so turns a little up the channel in a short funnel, and its band of shallows with it.
const MOUTH_DROWN: f64 = 0.25;
const MOUTH_WIDE: f64 = 5.0;
const MOUTH_LONG: f64 = 4.0;
/// Liquid may stand on a point beside a pond where the offset drawn lies under this, m; the ground drawn lies over the
/// level by `SIDE` of the offset at least (by all of it where no sample about it has a pond near, `Shore::beside`), all
/// the way to here and none from `SHORE_M` on: the edge of where liquid may stand lies on dry ground.
const EDGE_M: f64 = 2.0;

/// A channel's mouth at a point `off` m from the line of a channel whose bed is `width` m wide and `depth` m deep:
/// how far the offset lies lower at the shore, m, and the way from the shore over which that fades out, m
/// (`MOUTH_DROWN`).
pub(super) fn drowned(width: f64, depth: f64, off: f64) -> [f64; 2] {
    [MOUTH_DROWN * depth * (1.0 - ease(0.5 * width, MOUTH_WIDE * 0.5 * width, off)), MOUTH_LONG * width]
}

/// Of each sample of the terrain: the pond it lies in or the nearest within `NEAR_SAMPLES` (`NONE`: none), and its
/// offset and its depth under that pond's level, blurred (the module's doc), m; and the level of each pond, m.
pub(super) struct Shore {
    across: usize,
    down: usize,
    pond: Vec<u32>,
    levels: Vec<f64>,
    offset: Vec<f64>,
    depth: Vec<f64>,
    far: Vec<bool>,
}

/// The level of pond `q` of `s`, m: the sea's for the open low ground.
fn level_of(s: &Samples, q: u32) -> f64 {
    let p = &s.ponds[q as usize];
    if p.pass == NONE { SEA_M } else { p.level }
}

/// The pond beside a point (`Shore::beside`; `NONE`: none) and its level, m (NaN: none), and the share of the point's
/// weight on the samples about it that have a pond near.
pub(super) struct Beside {
    pub(super) pond: u32,
    pub(super) level: f64,
    pub(super) share: f64,
}

impl Shore {
    /// The shore of the samples `s`.
    pub(super) fn new(s: &Samples) -> Shore {
        let (across, down) = (s.across as i64, s.down as i64);
        let at = |c: i64, r: i64| r.clamp(0, down - 1) as usize * s.across + c.clamp(0, across - 1) as usize;
        let pond: Vec<u32> = (0..s.h.len())
            .map(|k| {
                let (c, r) = ((k % s.across) as i64, (k / s.across) as i64);
                let near = (-NEAR_SAMPLES..=NEAR_SAMPLES)
                    .flat_map(|dr| (-NEAR_SAMPLES..=NEAR_SAMPLES).map(move |dc| (dc, dr)))
                    .filter(|&(dc, dr)| s.pond[at(c + dc, r + dr)] != NONE)
                    .min_by_key(|&(dc, dr)| dc * dc + dr * dr);
                let q = near.map_or(NONE, |(dc, dr)| s.pond[at(c + dc, r + dr)]);
                // Out of a pond and under its level is the ground its liquid spills down below its pass: it holds none,
                // and its offset is held over the level, as land's.
                if q != NONE && s.pond[k] == NONE && s.h[k] < level_of(s, q) { NONE } else { q }
            })
            .collect();
        let levels: Vec<f64> = (0..s.ponds.len() as u32).map(|q| level_of(s, q)).collect();
        let level: Vec<f64> = pond.iter().map(|&q| if q == NONE { f64::NAN } else { levels[q as usize] }).collect();
        // Each sample's height over its level, held; a sample with no pond near stands over any.
        let held = |lo: f64, hi: f64| -> Vec<f64> { (0..s.h.len()).map(|k| if level[k].is_nan() { hi } else { (s.h[k] - level[k]).clamp(lo, hi) }).collect() };
        let kernel: Vec<f64> = (-SMOOTH_REACH..=SMOOTH_REACH).map(|d| (-0.5 * (d as f64 / SMOOTH_SAMPLES).powi(2)).exp()).collect();
        let sum: f64 = kernel.iter().sum();
        let blur = |v: &[f64], step: [i64; 2]| -> Vec<f64> {
            (0..v.len())
                .map(|k| {
                    let (c, r) = ((k % s.across) as i64, (k / s.across) as i64);
                    kernel.iter().zip(-SMOOTH_REACH..).map(|(w, d)| w * v[at(c + d * step[0], r + d * step[1])]).sum::<f64>() / sum
                })
                .collect()
        };
        let smooth = |v: Vec<f64>| blur(&blur(&v, [1, 0]), [0, 1]);
        let mut offset = smooth(held(-SHORE_M, SHORE_M));
        for (k, o) in offset.iter_mut().enumerate() {
            let q = s.pond[k];
            if q != NONE && s.ponds[q as usize].pass != NONE && s.h[k] < level[k] - KEEP_M {
                *o = o.min(-KEPT * SHORE_M);
            }
        }
        let depth = smooth(held(-WATER_M, 0.0));
        // A sample whose texels' splines read (two samples each way) only offsets held at `SHORE_M`, or only offsets held
        // at minus it and depths held at `WATER_M`, draws its own ground.
        let far = (0..s.h.len())
            .map(|k| {
                let (c, r) = ((k % s.across) as i64, (k / s.across) as i64);
                let all = |f: &dyn Fn(usize) -> bool| (-2..=2i64).all(|dr| (-2..=2i64).all(|dc| f(at(c + dc, r + dr))));
                all(&|j| offset[j] >= SHORE_M) || all(&|j| offset[j] <= -SHORE_M && depth[j] <= -WATER_M)
            })
            .collect();
        Shore { across: s.across, down: s.down, pond, levels, offset, depth, far }
    }

    /// The four samples between whose centres the point `p` (samples from the map's corner) lies, each with its weight
    /// there (bilinear; off the map, the edge's).
    fn about(&self, p: [f64; 2]) -> [(usize, f64); 4] {
        let (u, w) = (p[0] - 0.5, p[1] - 0.5);
        let (fx, fy) = (u - u.floor(), w - w.floor());
        let at = |dc: i64, dr: i64| (w.floor() as i64 + dr).clamp(0, self.down as i64 - 1) as usize * self.across + (u.floor() as i64 + dc).clamp(0, self.across as i64 - 1) as usize;
        [(at(0, 0), (1.0 - fx) * (1.0 - fy)), (at(1, 0), fx * (1.0 - fy)), (at(0, 1), (1.0 - fx) * fy), (at(1, 1), fx * fy)]
    }

    /// The share of the weight of the point `p` (samples from the map's corner) that lies on the samples about it
    /// that `on` holds for: 1 at such a sample's centre, 0 at another's, even between.
    pub(super) fn share(&self, p: [f64; 2], on: impl Fn(usize) -> bool) -> f64 {
        self.about(p).iter().filter(|(k, _)| on(*k)).map(|(_, w)| w).sum()
    }

    /// The pond beside the point `p` (samples from the map's corner): its own sample's (the nearest within
    /// `NEAR_SAMPLES`), or where that has none, the heaviest's of the samples about it that have one, or of the samples
    /// its offset is drawn from (`at`), so every point whose offset lies under `SHORE_M` has one; and the share of the
    /// point's weight on the samples about it that have one (`share`).
    pub(super) fn beside(&self, p: [f64; 2]) -> Beside {
        let own = (p[1] as usize).min(self.down - 1) * self.across + (p[0] as usize).min(self.across - 1);
        let heaviest = |about: &[(usize, f64)]| about.iter().filter(|(k, _)| self.pond[*k] != NONE).max_by(|a, b| a.1.total_cmp(&b.1)).map(|(k, _)| self.pond[*k]);
        let pond = if self.pond[own] != NONE { self.pond[own] } else { heaviest(&self.about(p)).or_else(|| heaviest(&self.spline(p))).unwrap_or(NONE) };
        let level = if pond == NONE { f64::NAN } else { self.levels[pond as usize] };
        Beside { pond, level, share: self.share(p, |k| self.pond[k] != NONE) }
    }

    /// The sixteen samples whose centres a cubic B-spline at `p` (samples from the map's corner) runs through, each with
    /// its weight there, row by row (off the map, the edge's).
    fn spline(&self, p: [f64; 2]) -> [(usize, f64); 16] {
        let (u, w) = (p[0] - 0.5, p[1] - 0.5);
        let (c, r) = (u.floor() as i64, w.floor() as i64);
        let (wx, wy) = (bspline(u - u.floor()), bspline(w - w.floor()));
        std::array::from_fn(|n| {
            let (i, j) = (n % 4, n / 4);
            let k = (r + j as i64 - 1).clamp(0, self.down as i64 - 1) as usize * self.across + (c + i as i64 - 1).clamp(0, self.across as i64 - 1) as usize;
            (k, wy[j] * wx[i])
        })
    }

    /// The offset and the depth at `p` (samples from the map's corner) as cubic B-splines through the samples' centres:
    /// smooth with their slopes, so the shore is a smooth line.
    fn at(&self, p: [f64; 2]) -> (f64, f64) {
        self.spline(p).iter().fold((0.0, 0.0), |(offset, depth), &(k, w)| (offset + w * self.offset[k], depth + w * self.depth[k]))
    }

    /// Whether the shore drawn leaves each sample out of its pond's liquid (the offset drawn at its centre over minus
    /// `MOUTH_UNDER_M`): a narrow arm of a body is drawn as land, and a river runs on through it.
    pub(super) fn dry(&self) -> Vec<bool> {
        (0..self.offset.len()).map(|k| self.at([(k % self.across) as f64 + 0.5, (k / self.across) as f64 + 0.5]).0 > -MOUTH_UNDER_M).collect()
    }

    /// Where a channel's line that the samples end at `from` (samples from the map's corner) ends as drawn (`MOUTH_MOST`):
    /// in the liquid of the shore drawn; `from` itself where it lies in it, or where none lies near.
    pub(super) fn mouth(&self, from: [f64; 2]) -> [f64; 2] {
        let mut p = from;
        for _ in 0..(MOUTH_MOST / MOUTH_STEP) as usize {
            if self.at(p).0 <= -MOUTH_UNDER_M {
                return p;
            }
            let slope = [self.at([p[0] + MOUTH_STEP, p[1]]).0 - self.at([p[0] - MOUTH_STEP, p[1]]).0, self.at([p[0], p[1] + MOUTH_STEP]).0 - self.at([p[0], p[1] - MOUTH_STEP]).0];
            let steep = slope[0].hypot(slope[1]);
            if steep < 1e-9 {
                break;
            }
            p = [p[0] - MOUTH_STEP * slope[0] / steep, p[1] - MOUTH_STEP * slope[1] / steep];
        }
        from
    }

    /// The height drawn at `p` (samples from the map's corner) whose own height is `h`, m, beside the pond `by`
    /// (`beside`): the module's doc, as far as the share of the point's weight on samples beside a pond, so the ground
    /// has no step where a sample beside a pond meets one beside none, and over the level where the offset lies under
    /// `EDGE_M` whatever that share; its offset lower by its channel's mouth (`drowned`: `mouth[0]` m at the shore, none
    /// `mouth[1]` samples from it, the way from the shore taken as the offset over its slope). And whether `by`'s
    /// liquid may stand on the point: where the offset lies under `EDGE_M`.
    pub(super) fn drawn(&self, p: [f64; 2], h: f64, by: &Beside, mouth: [f64; 2]) -> (f64, bool) {
        let own = (p[1] as usize).min(self.down - 1) * self.across + (p[0] as usize).min(self.across - 1);
        let stands = |d: f64| d < EDGE_M;
        if by.pond == NONE {
            return (h, false);
        }
        if self.far[own] {
            return (h, stands(self.offset[own]));
        }
        let (level, share) = (by.level, by.share);
        let (mut d, depth) = self.at(p);
        if mouth[0] > 0.0 {
            let slope = [self.at([p[0] + MOUTH_STEP, p[1]]).0 - self.at([p[0] - MOUTH_STEP, p[1]]).0, self.at([p[0], p[1] + MOUTH_STEP]).0 - self.at([p[0], p[1] - MOUTH_STEP]).0];
            let from_shore = d.abs() * 2.0 * MOUTH_STEP / slope[0].hypot(slope[1]).max(1e-9);
            d -= mouth[0] * (1.0 - ease(0.0, mouth[1], from_shore));
        }
        let side = if d < 0.0 { -1.0 } else { 1.0 };
        let push = SIDE * d.abs() * (1.0 - ease(SIDE_FROM * SHORE_M, SHORE_M, d.abs()));
        let sided = level + side * (side * (h - level)).max(push);
        if d >= 0.0 {
            let leaned = level + d + (sided - level - d) * ease(0.0, LEAN_M, d);
            // Where a sample about the point has no pond near (the ground a pond spills down below its pass), the
            // share blends in ground under the level: it is lifted over the level by the push, and by as much as the
            // offset where none of the samples about it has a pond near, all of it to `EDGE_M`, so the edge of where
            // liquid may stand lies as high over the level as the offset says.
            let blended = h + (leaned - h) * share;
            let least = level + push + (d - push) * (1.0 - share);
            return (blended + (blended.max(least) - blended) * (1.0 - ease(EDGE_M, SHORE_M, d)), stands(d));
        }
        // Under the level: the bed leans to a smooth depth, the offset's at the shore and the blurred depth's farther
        // out, the more the shallower (all of it at the shore, so the shallows have no bumps a sample wide).
        let deep = d + (depth.min(d) - d) * ease(0.0, SHORE_M, -d);
        let f = 1.0 - ease(0.0, WATER_M, -deep);
        let w = f / (1.0 - f).max(1e-12);
        (h + ((sided + w * (level + deep)) / (1.0 + w) - h) * share, stands(d))
    }
}

/// The weights of a uniform cubic B-spline at `t` (0..1) on the four points about it.
fn bspline(t: f64) -> [f64; 4] {
    let s = 1.0 - t;
    [s * s * s / 6.0, (4.0 - 6.0 * t * t + 3.0 * t * t * t) / 6.0, (1.0 + 3.0 * t + 3.0 * t * t - 3.0 * t * t * t) / 6.0, t * t * t / 6.0]
}
