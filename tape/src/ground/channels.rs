//! The terrain's channels as a shell draws them (laws.md 1.2): the liquid each sample passes on runs along its way
//! (`Samples::to`), drawn as a curve from the middle of the way into it (from its neighbour upstream that the most drains
//! through, `Samples::main`) to the middle of its own, bent at its centre (a quadratic Bézier), and on to the curve
//! through the next sample where that one's main way comes from elsewhere (a tributary joins). A channel is as wide as
//! the channel law makes its discharge (`Physics::channel`: width = w0 × discharge^b), and its discharge is its drained
//! area a (`Samples::up`) times what its cell passes on per m² drained, which changes but never passes `MOST_RUNOFF`.
//! So a channel covers a point at a distance d from its centre line only where d / a^b < w0 × `MOST_RUNOFF`^b / 2 (the
//! cap): a texel keeps, of the channels about it, the least d / a^b over the cap, at most 1 (none covers it), and that
//! channel's area. The least is continuous across the texels (a least of continuous numbers, capped), so a shader may
//! blend it; it and the area are a function of the seed and the map alone. A curve is drawn into the texels within its
//! reach only (the cap times a^b): most are far narrower than a texel.

use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::square::Square;
use pcore::space::terrain::SAMPLE_DEPTH;

/// The straight pieces each way's curve is drawn as.
const CURVE_STEPS: usize = 8;
/// The most liquid a m² drained passes on, as a depth, m/s: about the wettest month known on Earth (9.3 m of rain at
/// Cherrapunji, July 1861). A shader draws no channel wider than this makes it.
pub const MOST_RUNOFF: f64 = 3.6e-6;
/// A curve lies within this many samples of its own sample's centre (it runs from the middle of the way in to the
/// middle of the way out, and on to the middle of the next sample's curve).
const SPAN: i64 = 2;

/// The field of the channels over a window of texels (`Channels::field`): of each texel the least distance to a curve
/// over its area to the power b, over the cap (1: none within reach), and that curve's area, m².
pub(super) struct Field {
    pub ratio: Vec<f64>,
    pub area: Vec<f64>,
}

/// The channels of the samples `s` of a map, `metres` a sample, a channel's width going as its discharge to the power
/// `b`, and the cap of d / a^b, m per m² to that power.
pub(super) struct Channels<'a> {
    s: &'a Samples,
    metres: f64,
    b: f64,
    cap: f64,
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

/// The square of the distance from `p` to the segment from `a` to `b`.
fn to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (d, q) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let len = d[0] * d[0] + d[1] * d[1];
    let t = if len > 0.0 { ((q[0] * d[0] + q[1] * d[1]) / len).clamp(0.0, 1.0) } else { 0.0 };
    let (x, y) = (q[0] - t * d[0], q[1] - t * d[1]);
    x * x + y * y
}

impl<'a> Channels<'a> {
    /// The channels of `s` on `map`, a channel `width` m wide at a discharge of 1 m³/s, its width going as the discharge
    /// to the power `b`.
    pub(super) fn new(map: &Square, s: &'a Samples, width: f64, b: f64) -> Channels<'a> {
        Channels { s, metres: map.length(SAMPLE_DEPTH), b, cap: width * MOST_RUNOFF.powf(b) / 2.0 }
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

    /// The points of the curve of sample `k` whose three points are `bend`.
    fn curve(&self, k: usize, bend: [[f64; 2]; 3]) -> Vec<[f64; 2]> {
        let s = self.s;
        let mut pts: Vec<[f64; 2]> = (0..=CURVE_STEPS).map(|t| bezier(bend, t as f64 / CURVE_STEPS as f64)).collect();
        let j = s.to[k] as usize;
        if s.main[j] != k as u32 {
            pts.extend(self.bend(j).map(|b| bezier(b, 0.5)));
        }
        pts
    }

    /// The field over the texels from column `x0` and row `y0`, `wide` x `high` of them, each `k` samples a side (their
    /// centres at (x + 1/2) k samples): each curve near enough drawn into the texels within its reach.
    pub(super) fn field(&self, [x0, y0]: [i64; 2], [wide, high]: [usize; 2], k: f64) -> Field {
        let s = self.s;
        let mut f = Field { ratio: vec![1.0; wide * high], area: vec![0.0; wide * high] };
        // The samples whose curves may reach the window, and the texels a span of samples holds.
        let lo = |t0: i64| (t0 as f64 * k).floor() as i64 - SPAN;
        let hi = |t0: i64, n: usize| ((t0 + n as i64) as f64 * k).floor() as i64 + SPAN;
        let texels = |from: f64, to: f64, t0: i64, n: usize| ((from / k - 0.5).ceil() as i64).max(t0)..=((to / k - 0.5).floor() as i64).min(t0 + n as i64 - 1);
        for r in lo(y0).max(0)..=hi(y0, high).min(s.down as i64 - 1) {
            for c in lo(x0).max(0)..=hi(x0, wide).min(s.across as i64 - 1) {
                let q = r as usize * s.across + c as usize;
                let Some(bend) = self.bend(q) else { continue };
                let scale = s.up[q].powf(self.b);
                // Its reach in samples, and the texels within it of the control points' box (the curve lies inside): none,
                // and the curve is not drawn.
                let reach = self.cap * scale / self.metres;
                // A tributary's curve runs on into the next sample, within half a sample of its centre.
                let next = centre(s, s.to[q] as usize);
                let edge = |a: usize, low: bool| {
                    let v = bend.iter().map(|p| p[a]).fold(next[a] + if low { -0.5 } else { 0.5 }, if low { f64::min } else { f64::max });
                    if low { v - reach } else { v + reach }
                };
                let (bx0, bx1, by0, by1) = (edge(0, true), edge(0, false), edge(1, true), edge(1, false));
                let (xs, ys) = (texels(bx0, bx1, x0, wide), texels(by0, by1, y0, high));
                if xs.is_empty() || ys.is_empty() {
                    continue;
                }
                let pts = self.curve(q, bend);
                let to_ratio = self.metres / (scale * self.cap);
                // Each piece into the texels within its reach: a texel takes the curve where it beats what it holds.
                for w in pts.windows(2) {
                    let (lo, hi) = ([w[0][0].min(w[1][0]), w[0][1].min(w[1][1])], [w[0][0].max(w[1][0]), w[0][1].max(w[1][1])]);
                    for y in texels(lo[1] - reach, hi[1] + reach, y0, high) {
                        for x in texels(lo[0] - reach, hi[0] + reach, x0, wide) {
                            let i = (y - y0) as usize * wide + (x - x0) as usize;
                            let ratio = to_segment([(x as f64 + 0.5) * k, (y as f64 + 0.5) * k], w[0], w[1]).sqrt() * to_ratio;
                            if ratio < f.ratio[i] {
                                (f.ratio[i], f.area[i]) = (ratio, s.up[q]);
                            }
                        }
                    }
                }
            }
        }
        f
    }
}
