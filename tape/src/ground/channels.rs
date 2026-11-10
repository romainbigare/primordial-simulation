//! The terrain's channels as a shell draws them (laws.md 1.2): the liquid each sample passes on runs along its way
//! (`Samples::to`), drawn as a curve from the middle of the way into it (from its neighbour upstream that the most drains
//! through, `Samples::main`) to the middle of its own, bent at its centre (a quadratic Bézier), and on to the curve
//! through the next sample where that one's main way comes from elsewhere (a tributary joins). A channel is as wide as
//! the channel law makes its discharge (`Physics::channel`: width ∝ discharge^b), and its discharge is its drained area
//! (`Samples::up`) times its cell's flux per drained area, which changes; so a texel keeps, of the channels about it,
//! the one whose distance d to its centre line over its area a to the power b is least: that channel covers the texel
//! first as the flux grows, whatever it is. That least ratio is continuous across the texels (a least of continuous
//! numbers), so a shader may blend it; it and the area are a function of the seed and the map alone.

use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::square::Square;
use pcore::space::terrain::SAMPLE_DEPTH;

/// The straight pieces each way's curve is drawn as.
const CURVE_STEPS: usize = 8;
/// The channels of the samples this many samples each way of a texel's own are looked at: a channel that covers a texel
/// runs through a sample next to it (no channel is a sample wide), and farther ones cover none of it.
const REACH: i64 = 1;
/// The ratio of a texel with no channel about it: as far as an f32 goes, so a shader can blend it.
const NO_CHANNEL: f64 = f32::MAX as f64;

/// A way's curve: its points and its bounding box, in samples from the map's north-west corner, its drained area, m²,
/// and that area to the power of a channel's width.
struct Curve {
    pts: Vec<[f64; 2]>,
    lo: [f64; 2],
    hi: [f64; 2],
    area: f64,
    scale: f64,
}

/// The channels of the samples `s` of a map, `metres` a sample, a channel's width going as its discharge to the power `b`:
/// the curves of the samples from column `c0` and row `r0`, `wide` a row (none for a sink or off the map).
pub(super) struct Near<'a> {
    s: &'a Samples,
    metres: f64,
    b: f64,
    c0: i64,
    r0: i64,
    wide: i64,
    curves: Vec<Option<Curve>>,
}

/// The centre of sample `k` of `s`, in samples.
fn centre(s: &Samples, k: usize) -> [f64; 2] {
    [(k % s.across) as f64 + 0.5, (k / s.across) as f64 + 0.5]
}

fn mid(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
}

/// The point at `t` along the quadratic Bézier from `a` to `c` bent toward `b`.
fn bezier([a, b, c]: [[f64; 2]; 3], t: f64) -> [f64; 2] {
    let u = 1.0 - t;
    std::array::from_fn(|k| u * u * a[k] + 2.0 * u * t * b[k] + t * t * c[k])
}

/// The square of the distance from `p` to the box from `lo` to `hi` (none inside it).
fn box_distance(p: [f64; 2], lo: [f64; 2], hi: [f64; 2]) -> f64 {
    let (x, y) = ((lo[0] - p[0]).max(p[0] - hi[0]).max(0.0), (lo[1] - p[1]).max(p[1] - hi[1]).max(0.0));
    x * x + y * y
}

/// The square of the distance from `p` to the segment from `a` to `b`.
fn to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (d, q) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let len = d[0] * d[0] + d[1] * d[1];
    let t = if len > 0.0 { ((q[0] * d[0] + q[1] * d[1]) / len).clamp(0.0, 1.0) } else { 0.0 };
    let (x, y) = (q[0] - t * d[0], q[1] - t * d[1]);
    x * x + y * y
}

impl<'a> Near<'a> {
    /// The channels a texel between the points `lo` and `hi` (in samples) is nearest.
    pub(super) fn new(map: &Square, s: &'a Samples, b: f64, [lo, hi]: [[f64; 2]; 2]) -> Near<'a> {
        let (c0, r0) = (lo[0].floor() as i64 - REACH, lo[1].floor() as i64 - REACH);
        let (wide, high) = (hi[0].floor() as i64 + REACH - c0 + 1, hi[1].floor() as i64 + REACH - r0 + 1);
        let mut near = Near { s, metres: map.length(SAMPLE_DEPTH), b, c0, r0, wide, curves: Vec::new() };
        let on = |c: i64, r: i64| (0..s.across as i64).contains(&c) && (0..s.down as i64).contains(&r);
        near.curves = (0..wide * high).map(|q| (c0 + q % wide, r0 + q / wide)).map(|(c, r)| if on(c, r) { near.curve(r as usize * s.across + c as usize) } else { None }).collect();
        near
    }

    /// The three points of the curve through sample `k` (none for a sink).
    fn bend(&self, k: usize) -> Option<[[f64; 2]; 3]> {
        let s = self.s;
        let to = s.to[k];
        (to != NONE).then(|| {
            let c = centre(s, k);
            let start = if s.main[k] == NONE { c } else { mid(centre(s, s.main[k] as usize), c) };
            [start, c, mid(c, centre(s, to as usize))]
        })
    }

    /// The curve of the way of sample `k` (none for a sink).
    fn curve(&self, k: usize) -> Option<Curve> {
        let s = self.s;
        let bend = self.bend(k)?;
        let mut pts: Vec<[f64; 2]> = (0..=CURVE_STEPS).map(|t| bezier(bend, t as f64 / CURVE_STEPS as f64)).collect();
        let j = s.to[k] as usize;
        if s.main[j] != k as u32 {
            pts.extend(self.bend(j).map(|b| bezier(b, 0.5)));
        }
        let (lo, hi) = pts.iter().fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])]));
        Some(Curve { pts, lo, hi, area: s.up[k], scale: s.up[k].powf(self.b) })
    }

    /// Of the channels of the samples within `REACH` of point `p` (in samples), the least distance to a centre line over
    /// the drained area to the power `b` (m per m² to that power) and that channel's drained area, m².
    pub(super) fn nearest(&self, p: [f64; 2]) -> (f64, f64) {
        let s = self.s;
        let (pc, pr) = (p[0].floor() as i64, p[1].floor() as i64);
        let mut best = (NO_CHANNEL, 0.0);
        // The texel's own sample first, so the bound it sets passes over most of the others.
        let around = (-REACH..=REACH).flat_map(|dr| (-REACH..=REACH).map(move |dc| (dc, dr))).filter(|&d| d != (0, 0));
        for (c, r) in std::iter::once((0, 0)).chain(around).map(|(dc, dr)| (pc + dc, pr + dr)) {
            if !(0..s.across as i64).contains(&c) || !(0..s.down as i64).contains(&r) {
                continue;
            }
            let Some(curve) = &self.curves[((r - self.r0) * self.wide + c - self.c0) as usize] else { continue };
            // The square of the distance, in samples, under which this channel's ratio beats the best so far.
            let mut limit = (best.0 * curve.scale / self.metres).powi(2);
            if box_distance(p, curve.lo, curve.hi) >= limit {
                continue;
            }
            let mut near = f64::MAX;
            for w in curve.pts.windows(2) {
                let (lo, hi) = ([w[0][0].min(w[1][0]), w[0][1].min(w[1][1])], [w[0][0].max(w[1][0]), w[0][1].max(w[1][1])]);
                if box_distance(p, lo, hi) < limit {
                    near = near.min(to_segment(p, w[0], w[1]));
                    limit = limit.min(near);
                }
            }
            if near < f64::MAX && near.sqrt() * self.metres / curve.scale < best.0 {
                best = (near.sqrt() * self.metres / curve.scale, curve.area);
            }
        }
        best
    }
}
