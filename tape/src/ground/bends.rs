//! The bends of a channel as a shell draws it: between the centres of the terrain's samples its line winds about the
//! curve through them (`channels::base`) as a sine whose wavelength is `BEND_WIDTHS` channel widths and whose half
//! amplitude is `BEND_HALF` widths (Leopold and Wolman 1960). It bends fully where the valley's slope is below
//! `BENDS_BELOW` and not at all above `STRAIGHT_ABOVE` (pool-riffle reaches meander, step-pool and cascade reaches do
//! not: Schumm 1985, Montgomery and Buffington 1997), and no wider than the valley's floor: where the ground across the
//! valley rises from the channel by more than the channel's depth, the valley's side holds it. A tile is made once,
//! so the width and the depth are stand-ins: the channel law's (`Physics::channel`) at the drained area times
//! `TYPICAL_RUNOFF`. The phase of the bends runs on along each main way from its head (`phases`), so the line is
//! continuous from sample to sample; it is a function of the seed, the map and the place alone.

use super::channels::{base, Law, TYPICAL_RUNOFF};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use std::f64::consts::TAU;

/// A bend's wavelength, in channel widths: 7 to 15 in rivers of every size, about 11 on their mean (Leopold and Wolman
/// 1960).
const BEND_WIDTHS: f64 = 11.0;
/// The half amplitude of the bends, in channel widths: their amplitude, from one side to the other, is about 2.7 widths
/// (Leopold and Wolman 1960).
const BEND_HALF: f64 = 1.35;
/// Below this slope of the valley a channel bends fully, above `STRAIGHT_ABOVE` not at all: pool-riffle channels lie
/// below about 0.015, step-pool and cascade ones above about 0.03 (Montgomery and Buffington 1997).
const BENDS_BELOW: f64 = 0.01;
const STRAIGHT_ABOVE: f64 = 0.03;
/// The most a line bends from the curve through the samples, in samples: it stays inside the window of samples whose
/// curves a tile reads (`channels::SPAN`).
pub(super) const BEND_MOST: f64 = 0.25;

/// One sample's piece of the channel: the curve through it (`channels::base`), in samples; the half amplitude of its
/// bends at its two ends, samples; their phase at its start and the bends along it, cycles.
#[derive(Clone, Copy)]
pub(super) struct Piece {
    pub base: [[f64; 2]; 3],
    pub amp: [f64; 2],
    pub phase: f64,
    pub cycles: f64,
}

/// The point at `t` along the quadratic Bézier from `a` to `c` bent toward `b`.
fn bezier([a, b, c]: [[f64; 2]; 3], t: f64) -> [f64; 2] {
    let u = 1.0 - t;
    std::array::from_fn(|k| u * u * a[k] + 2.0 * u * t * b[k] + t * t * c[k])
}

/// The smooth step from 0 at `a` to 1 at `b`.
pub(super) fn ease(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Piece {
    /// The point at `t` along the piece, samples: the curve's, moved across it by the bend there.
    pub(super) fn at(&self, t: f64) -> [f64; 2] {
        let p = bezier(self.base, t);
        let a = self.amp[0] * (1.0 - t) + self.amp[1] * t;
        let [a0, b0, c0] = self.base;
        // The curve's direction at t (its derivative over 2), and the bend's offset along its normal.
        let d: [f64; 2] = std::array::from_fn(|k| (1.0 - t) * (b0[k] - a0[k]) + t * (c0[k] - b0[k]));
        let len = d[0].hypot(d[1]);
        if a == 0.0 || len == 0.0 {
            return p;
        }
        let off = a * (TAU * (self.phase + self.cycles * t)).sin() / len;
        [p[0] - d[1] * off, p[1] + d[0] * off]
    }
}

/// What a shell takes of the channel of sample `k` of `s`: the width and the depth the law gives its drained area at
/// `TYPICAL_RUNOFF`, m.
pub(super) fn stand_in(law: &Law, s: &Samples, k: usize) -> (f64, f64) {
    let q = s.up[k] * TYPICAL_RUNOFF;
    (law.width * q.powf(law.width_exp), law.depth * q.powf(law.depth_exp))
}

/// The wavelength of the bends of sample `k`'s channel, m.
pub(super) fn wavelength(law: &Law, s: &Samples, k: usize) -> f64 {
    BEND_WIDTHS * law.width * (s.up[k] * TYPICAL_RUNOFF).powf(law.width_exp)
}

/// The half amplitude of the bends of sample `k`'s channel, samples of `metres`: `BEND_HALF` widths, less as the
/// valley's slope to the sample it runs to rises from `BENDS_BELOW` to `STRAIGHT_ABOVE`, and no more than the floor's
/// half width (the channel's depth over the rise of the ground across the valley, the mean of the two neighbours
/// square to its way); none under standing liquid and at a sink.
pub(super) fn amplitude(law: &Law, s: &Samples, metres: f64, k: usize) -> f64 {
    let to = s.to[k];
    if to == NONE || s.pond[k] != NONE {
        return 0.0;
    }
    let (c, r) = ((k % s.across) as i64, (k / s.across) as i64);
    let (dx, dy) = ((to as usize % s.across) as i64 - c, (to as usize / s.across) as i64 - r);
    let run = metres * ((dx * dx + dy * dy) as f64).sqrt();
    let slope = (s.h[k] - s.h[to as usize]).max(0.0) / run;
    let at = |c: i64, r: i64| s.h[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
    let across = ((at(c - dy, r + dx) - s.h[k]).max(0.0) + (at(c + dy, r - dx) - s.h[k]).max(0.0)) / (2.0 * run);
    let (width, depth) = stand_in(law, s, k);
    let floor = if across > 0.0 { depth / across } else { f64::INFINITY };
    (BEND_HALF * width * (1.0 - ease(BENDS_BELOW, STRAIGHT_ABOVE, slope))).min(floor).min(BEND_MOST * metres) / metres
}

/// The phase of the bends at the start of each sample's piece, cycles (0 to 1): 0 at a head; along a main way (where a
/// sample is the main one upstream of the next, `Samples::main`) the next piece starts where the last one ends, its
/// length (`length`, m) over its wavelength on. In the flood's order from the last sample reached, so a sample comes
/// before the one its liquid runs to.
pub(super) fn phases(law: &Law, s: &Samples, metres: f64) -> Vec<f32> {
    let mut phase = vec![0.0f64; s.h.len()];
    for &k in s.order.iter().rev() {
        let (k, to) = (k as usize, s.to[k as usize]);
        if to != NONE && s.main[to as usize] == k as u32 {
            let cycles = base(s, k).map_or(0.0, |b| length(b) * metres / wavelength(law, s, k));
            phase[to as usize] = (phase[k] + cycles).fract();
        }
    }
    phase.into_iter().map(|p| p as f32).collect()
}

/// The pieces the length of a curve is summed over.
const LENGTH_STEPS: usize = 8;

/// The length of the curve `b`, in its units: over `LENGTH_STEPS` straight pieces.
pub(super) fn length(b: [[f64; 2]; 3]) -> f64 {
    (0..LENGTH_STEPS)
        .map(|i| {
            let (p, q) = (bezier(b, i as f64 / LENGTH_STEPS as f64), bezier(b, (i + 1) as f64 / LENGTH_STEPS as f64));
            (q[0] - p[0]).hypot(q[1] - p[1])
        })
        .sum()
}
