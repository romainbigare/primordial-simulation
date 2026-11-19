//! The terrain's channels as a shell draws them (laws.md 1.2): the liquid each sample passes on runs along its way
//! (`Samples::to`), drawn as a curve from the middle of the way into it (from its neighbour upstream that the most drains
//! through, `Samples::main`) to the middle of its own, bent at its centre (a quadratic Bézier), and on to the line of
//! the next sample where that one's main way comes from elsewhere (a tributary joins); a sink of the open low ground
//! that liquid runs into ends the curve at its centre, under the low ground's liquid. The line swings about that curve
//! in its bends (`bends`, `belt`). A channel is as wide as the channel law makes its discharge (`Physics::channel`:
//! width = w0 × discharge^b), and what a m² drained passes on never passes `MOST_RUNOFF`, so a channel of drained area
//! a covers no point farther than w0 × (`MOST_RUNOFF` a)^b / 2 from its line (its reach). Of the channels within their
//! reach, a texel keeps the one whose distance over the larger of its reach and the texel's size is least: the one that
//! covers it first as the flux grows, or, where every channel is narrower than a texel, the nearest; but a line that
//! passes within half a texel's diagonal of the texel's centre (`CLAIM_DIAGONAL`) is as near as can be, so a rill
//! keeps the texels it runs through however wide the river whose reach they lie in; it keeps that
//! channel's sample, its distance to its line, signed by the side the texel lies on, of the line's nearest point how
//! far along the channel it lies, and its distance to the line without its bends (its curve and its wander alone:
//! from afar a shell draws a channel as a line wider than its bends, about that one). A channel a shell may draw as
//! a line wider than itself (its drained area at least `SHOW_M` × the texel's size over `TEXEL_PX_MOST` ×
//! `WET_TIMES`) reaches as far as the widest line the shell draws it as on these texels too (`line_reach`). Each
//! number is a function of the seed, the map and the place alone.

use super::bends::{Bends, Reaches, BEND_MOST};
use super::lines::{Drawn, Join};
use super::mend::STRIP;
use super::stems::SMOOTH_MOST;
use super::valley::{carve, VALLEY_FROM, VALLEY_M};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;

/// The most liquid a m² drained passes on, as a depth, m/s: about the wettest month known on Earth (9.3 m of rain at
/// Cherrapunji, July 1861). No channel is wider than this makes it.
pub(super) const MOST_RUNOFF: f64 = 3.6e-6;
/// What a m² of land passes on to the rivers in a year on Earth's mean, as a depth, m/s: 0.27 m a year, about 40,000
/// km³ a year from 1.5e8 km² of land (Dai and Trenberth 2002). The stand-in for a channel's discharge where a number
/// that never changes needs one (its bed, `bends::BED_RUNOFF`; which channels a tile holds as lines).
pub const TYPICAL_RUNOFF: f64 = 8.5e-9;
/// A shell draws a channel no thinner than a line of a few pixels once its discharge passes `TYPICAL_RUNOFF` × this ×
/// the metres a css pixel takes, m: the drained area that shows grows with the metres a pixel takes, so a view shows
/// its main stems and their larger branches, more as it zooms in, and no mesh of rills. A map's choice, set by eye: at
/// `TYPICAL_RUNOFF` the whole of a 22 island (78 m a css pixel) shows the rivers that drain 60 km² or more, a view with
/// a scale bar of 2 km those of 16 km², one of 200 m those of 1.6 km²; a wetter island shows more.
pub const SHOW_M: f64 = 7.5e5;
/// A channel's discharge per m² drained is at most about this many times `TYPICAL_RUNOFF` on a year's mean: the wettest
/// lands pass on 2 to 3 m a year (the Amazon's basin passes on about 1 m).
const WET_TIMES: f64 = 10.0;
/// The most screen pixels a texel takes: a page draws a view at the depth whose texels take a pixel (`web/src/ground.ts`
/// `TEXEL_PX`), or coarser where that asks too many tiles: a texel takes at most about 4.
const TEXEL_PX_MOST: f64 = 4.0;
/// A tile holds no channel narrower than this share of a texel at `MOST_RUNOFF` that is not held as a line: it would
/// cover no more than this share of the texel's pixels.
const LEAST_TEXELS: f64 = 0.1;
/// The least width a shell draws a channel at: `LINE_PX` css pixels where its discharge is the one that shows
/// (`SHOW_M`), wider as its discharge over that to the power `LINE_GROW`, up to `LINE_MOST` times. A map's choice: a
/// channel narrower than this on screen stays in sight as a thin line, a stream a hair and a river a little more,
/// and no line is much wider than the bed it stands for (a channel is drawn as wide as it is wherever that is wider).
pub const LINE_PX: f64 = 1.0;
pub const LINE_GROW: f64 = 0.5;
pub const LINE_MOST: f64 = 2.5;
/// A css pixel takes at most this many texels: a texel takes a screen's pixel or more (`TEXEL_PX_MOST`), and a
/// phone's screen has three pixels to a css pixel.
const CSS_TEXELS_MOST: f64 = 3.0;
/// A line reaches this many texels past its edge: a shell blends the distance between texels and spreads the edge
/// over a pixel.
const LINE_MARGIN: f64 = 1.5;
/// A line claims the texels whose centre lies within this many texels of it (half a texel's diagonal: the texels it
/// runs through) before any line farther away; among such lines, and lines as near over their scale, the nearest wins
/// (by `NEAREST_TIE` of its distance, samples, too small to change any other choice).
const CLAIM_DIAGONAL: f64 = 0.71;
const NEAREST_TIE: f64 = 1e-6;
/// The farthest the middle of a sample's line lies from its centre, samples: its curve's middle lies within a quarter
/// of a diagonal of it, and its bends move it by at most `bends::BEND_MOST`.
const NEXT_OFF: f64 = 0.36 + BEND_MOST + SMOOTH_MOST;
/// The samples a line lies within of its own sample's centre: it runs from the middle of the way in to the middle of
/// the way out, and on to the line of the next sample (a diagonal away, itself up to `NEXT_OFF` from its centre); a
/// line's end in standing liquid lies up to `shore::MOUTH_MOST` from its sample's centre.
const SPAN: i64 = 6;
/// The farthest a texel's channel's sample lies from the sample that holds the texel's centre, samples along each
/// axis: a tile names it by that step (`code`), which a 16-bit float holds exactly.
pub const STEP_MOST: i64 = 21;
/// The distance a texel holds where no channel reaches it, m (a 16-bit float holds it exactly).
pub(super) const NO_CHANNEL_M: f64 = 6.0e4;
/// A texel holds how far along its channel's line its nearest point lies modulo this length, m: a 16-bit float holds
/// it to a few centimetres, and a shell lays along it only signs that repeat over it.
pub const ALONG_WRAP_M: f64 = 128.0;

/// The channel law a shell sizes a channel by (`Physics::channel`): the width and the depth of a channel that carries
/// 1 m³/s, m, and the powers of its discharge they grow as.
#[derive(Clone, Copy, Debug)]
pub struct Law {
    pub width: f64,
    pub width_exp: f64,
    pub depth: f64,
    pub depth_exp: f64,
}

impl Law {
    /// The channel law of `ph`.
    pub fn of(ph: &pcore::Physics) -> Law {
        Law { width: ph.channel_width, width_exp: ph.channel_width_exp, depth: ph.channel_depth, depth_exp: ph.channel_depth_exp }
    }
}

/// A window of texels: from column `x0` and row `y0`, `wide` x `high` of them, each `k` samples a side.
#[derive(Clone, Copy)]
pub(super) struct Window {
    pub x0: i64,
    pub y0: i64,
    pub wide: usize,
    pub high: usize,
    pub k: f64,
}

impl Window {
    /// The window's columns (`axis` 0) or rows (1) whose texels' centres lie between `from` and `to`, samples.
    pub(super) fn texels(&self, from: f64, to: f64, axis: usize) -> std::ops::RangeInclusive<i64> {
        let (t0, n) = if axis == 0 { (self.x0, self.wide) } else { (self.y0, self.high) };
        ((from / self.k - 0.5).ceil() as i64).max(t0)..=((to / self.k - 0.5).floor() as i64).min(t0 + n as i64 - 1)
    }

    /// The centre of the texel at column `x` and row `y`, samples, and its place in the window.
    pub(super) fn texel(&self, x: i64, y: i64) -> ([f64; 2], usize) {
        ([(x as f64 + 0.5) * self.k, (y as f64 + 0.5) * self.k], (y - self.y0) as usize * self.wide + (x - self.x0) as usize)
    }
}

/// The field of the channels over a window of texels (`Channels::field`): of each texel its channel's sample (`NONE`
/// for none), the distance to that channel's line, m, signed by the side (`NO_CHANNEL_M` for none), of the line's
/// nearest point how far along the channel it lies, m, and the signed distance to the nearest channel's line without
/// its bends, m (`NO_CHANNEL_M` for none); and the valleys about the lines (`valley`): the sum of their weights at the
/// texel and of their floors' heights so weighted, m.
pub(super) struct Field {
    pub dist: Vec<f64>,
    pub sample: Vec<u32>,
    pub along: Vec<f64>,
    pub calm: Vec<f64>,
    pub valley: Vec<f64>,
    pub weight: Vec<f64>,
}

impl Field {
    /// The height `h` of texel `i` drawn with the valleys about it: the ground and the floors of the lines within
    /// `VALLEY_M`, by weight, the ground's 1 (`valley::weight`).
    pub(super) fn drawn(&self, i: usize, h: f64) -> f64 {
        (h + self.valley[i]) / (1.0 + self.weight[i])
    }
}

/// The channels of a map (`bends::Reaches`) and how each one's piece starts (`bends::Bends`); the channel's reach per
/// m² drained to the power b, and the largest drained area.
pub(super) struct Channels<'a> {
    pub r: Reaches<'a>,
    pub bends: &'a Bends,
    cap: f64,
    most: f64,
}

/// A line near a window of texels: its sample, the line as drawn, where it joins the next sample's, what its
/// distances are told apart by (half its channel's widest width, no less than a texel, samples), how far it reaches,
/// samples, and whether a valley is drawn about it.
pub(super) struct Near {
    pub q: usize,
    pub drawn: Drawn,
    join: Option<Join>,
    scale: f64,
    reach: f64,
    carves: bool,
}

fn mid(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
}

/// The three points of the curve through sample `k` of `s` whose lines run through the points `at` (`stems::points`),
/// in samples: from the middle of its main way in (its point at a head) through its point to the middle of its way out;
/// a sink that liquid runs into ends at its point; none for a sink none runs into.
pub(super) fn base(s: &Samples, at: &[[f64; 2]], k: usize) -> Option<[[f64; 2]; 3]> {
    let c = at[k];
    let start = (s.main[k] != NONE).then(|| mid(at[s.main[k] as usize], c));
    match (s.to[k], start) {
        (NONE, None) => None,
        (NONE, Some(a)) => Some([a, mid(a, c), c]),
        (to, a) => Some([a.unwrap_or(c), c, mid(c, at[to as usize])]),
    }
}

/// The side of the segment from `a` to `b` the point `p` lies on (1 on its left, else -1), the square of its distance
/// to it, and how far along it its nearest point lies, 0 to 1.
pub(super) fn to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> (f64, f64, f64) {
    let (d, q) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let len = d[0] * d[0] + d[1] * d[1];
    let t = if len > 0.0 { ((q[0] * d[0] + q[1] * d[1]) / len).clamp(0.0, 1.0) } else { 0.0 };
    let (x, y) = (q[0] - t * d[0], q[1] - t * d[1]);
    let side = if d[0] * q[1] - d[1] * q[0] >= 0.0 { 1.0 } else { -1.0 };
    (side, x * x + y * y, t)
}

impl<'a> Channels<'a> {
    /// The channels `r`, their pieces starting as `bends` holds.
    pub(super) fn new(r: Reaches<'a>, bends: &'a Bends) -> Channels<'a> {
        let most = r.s.up.iter().fold(0.0f64, |m, &a| m.max(a));
        Channels { r, bends, cap: r.widest(1.0), most }
    }

    /// The lines that reach the window `win` of texels of `texel` m, and how far a valley reaches, samples (none on
    /// texels no finer than the samples).
    fn near(&self, win: &Window, texel: f64) -> (Vec<Near>, f64) {
        let (s, metres, k) = (self.r.s, self.r.metres, win.k);
        let reach_valley = if k < 1.0 { VALLEY_M / metres } else { 0.0 };
        // The reach of a line in samples, no farther than a texel may name its sample by its step from its own.
        let most_reach = (STEP_MOST - SPAN - 1) as f64;
        // A line shows where a pixel takes at least a texel over `TEXEL_PX_MOST`, its discharge at most `WET_TIMES`
        // × `TYPICAL_RUNOFF` per m² drained.
        let line_from = SHOW_M * texel / (TEXEL_PX_MOST * WET_TIMES);
        // Half the widest line a shell draws a channel that drains `a` m² as on these texels, with its margin, m:
        // the line law where a css pixel takes the most texels (the line's width in metres grows with the pixel).
        let line_reach = |a: f64| {
            let css = CSS_TEXELS_MOST * texel;
            0.5 * LINE_PX * css * (WET_TIMES * a / (SHOW_M * css)).powf(LINE_GROW).min(LINE_MOST) + LINE_MARGIN * texel
        };
        let reach_of = |a: f64| {
            let own = self.cap * a.powf(self.r.law.width_exp);
            let reach = if a >= line_from { own.max(line_reach(a)) } else { own };
            (own, (reach / metres).min(most_reach))
        };
        // A channel that is held by no line and whose widest is under `LEAST_TEXELS` of a texel is not held.
        let least = (LEAST_TEXELS * texel / self.cap).powf(1.0 / self.r.law.width_exp).min(line_from);
        let window = SPAN + reach_of(self.most).1.max(reach_valley).ceil() as i64 + 1;
        let lo = |t0: i64| (t0 as f64 * k).floor() as i64 - window;
        let hi = |t0: i64, n: usize| ((t0 + n as i64) as f64 * k).floor() as i64 + window;
        let mut near = Vec::new();
        for r in lo(win.y0).max(0)..=hi(win.y0, win.high).min(s.down as i64 - 1) {
            for c in lo(win.x0).max(0)..=hi(win.x0, win.wide).min(s.across as i64 - 1) {
                let q = r as usize * s.across + c as usize;
                if s.up[q] < least {
                    continue;
                }
                let Some(b) = base(s, self.r.at, q) else { continue };
                let (own, reach) = reach_of(s.up[q]);
                let carves = s.up[q] >= VALLEY_FROM && reach_valley > 0.0 && s.pond[q] == NONE;
                let widest = if carves { reach.max(reach_valley) } else { reach };
                // The box the line lies in: its curve's points and, where a tributary joins, the next sample's centre,
                // widened by its sway, by how far the next sample's line lies from its centre and by its reach: none
                // of the window's texels in it, and the line is not drawn.
                let next = if s.to[q] != NONE { self.r.at[s.to[q] as usize] } else { b[2] };
                let edge = |a: usize, low: bool| {
                    let v = b.iter().map(|p| p[a]).fold(next[a], if low { f64::min } else { f64::max });
                    v + if low { -1.0 } else { 1.0 } * (widest + NEXT_OFF)
                };
                if win.texels(edge(0, true), edge(0, false), 0).is_empty() || win.texels(edge(1, true), edge(1, false), 1).is_empty() {
                    continue;
                }
                if let Some((drawn, join)) = self.line(q, texel) {
                    near.push(Near { q, drawn, join, scale: own.max(texel) / metres, reach, carves });
                }
            }
        }
        (near, reach_valley)
    }

    /// Lay the piece from `a` to `b` of the line `l` into the texels of `win` within its reach that it lies nearer
    /// than `best` holds (over its scale), that `takes` it and that lie out of the bed of the channel it joins: `put`
    /// gets the texel, its distance signed by the side, m, and the share of the piece its nearest point lies along.
    fn lay(&self, win: &Window, l: &Near, [a, b]: [[f64; 2]; 2], best: &mut [f64], takes: &dyn Fn(usize) -> bool, put: &mut dyn FnMut(usize, f64, f64)) {
        let claims = CLAIM_DIAGONAL * win.k;
        for y in win.texels(a[1].min(b[1]) - l.reach, a[1].max(b[1]) + l.reach, 1) {
            for x in win.texels(a[0].min(b[0]) - l.reach, a[0].max(b[0]) + l.reach, 0) {
                let (at, i) = win.texel(x, y);
                let (side, d2, t) = to_segment(at, a, b);
                let near = (d2.sqrt() - claims).max(0.0) / l.scale + NEAREST_TIE * d2.sqrt();
                if d2 <= l.reach * l.reach && near < best[i] && takes(i) && !l.join.is_some_and(|j| j.holds(at)) {
                    best[i] = near;
                    put(i, side * d2.sqrt() * self.r.metres, t);
                }
            }
        }
    }

    /// The field over the window `win`: the lines laid into its texels (`laid`) and its strips mended (`mend`), over
    /// a window `mend::STRIP` texels larger all round, so each texel has the neighbours its mending reads.
    pub(super) fn field(&self, win: Window) -> Field {
        let (m, wide) = (STRIP, win.wide + 2 * STRIP);
        let all = Window { x0: win.x0 - m as i64, y0: win.y0 - m as i64, wide, high: win.high + 2 * m, k: win.k };
        let (near, reach_valley) = self.near(&all, all.k * self.r.metres);
        let mut f = self.laid(&all, &near, reach_valley);
        self.mend(&mut f, &all, &near);
        let cut = |v: &[f64]| (0..win.high).flat_map(|y| v[(y + m) * wide + m..(y + m + 1) * wide - m].iter().copied()).collect::<Vec<f64>>();
        let sample = (0..win.high).flat_map(|y| f.sample[(y + m) * wide + m..(y + m + 1) * wide - m].iter().copied()).collect();
        Field { dist: cut(&f.dist), sample, along: cut(&f.along), calm: cut(&f.calm), valley: cut(&f.valley), weight: cut(&f.weight) }
    }

    /// The lines `near` laid into the texels of the window `win`: each line drawn into the texels within its reach,
    /// and the valleys about the lines (as far as `reach_valley` samples); then the lines without their bends, each
    /// into the texels that hold its own channel's line (the line from the same head, `Bends::head`) or none.
    fn laid(&self, win: &Window, near: &[Near], reach_valley: f64) -> Field {
        let n = win.wide * win.high;
        let mut f = Field { dist: vec![NO_CHANNEL_M; n], sample: vec![NONE; n], along: vec![0.0; n], calm: vec![NO_CHANNEL_M; n], valley: vec![0.0; n], weight: vec![0.0; n] };
        // Of each texel, how near the line it holds lies, over the line's scale.
        let mut best = vec![f64::INFINITY; n];
        for l in near {
            for w in l.drawn.line.windows(2) {
                self.lay(win, l, [w[0].p, w[1].p], &mut best, &|_| true, &mut |i, d, t| (f.dist[i], f.sample[i], f.along[i]) = (d, l.q as u32, w[0].s + (w[1].s - w[0].s) * t));
            }
            if l.carves {
                carve(&mut f, win, self.r.s, l.q, &l.drawn.line, reach_valley);
            }
        }
        let (held, head) = (f.sample.clone(), |q: u32| self.bends.head(q as usize));
        best.fill(f64::INFINITY);
        for l in near {
            for w in l.drawn.calm.windows(2) {
                self.lay(win, l, [w[0], w[1]], &mut best, &|i| held[i] == NONE || head(held[i]) == head(l.q as u32), &mut |i, d, _| {
                    f.calm[i] = d;
                    if held[i] == NONE {
                        f.sample[i] = l.q as u32;
                    }
                });
            }
        }
        f
    }
}

/// The step from the sample that holds the centre of the texel at column `x` and row `y` of texels `k` samples a side
/// to sample `q` of `s`, as one whole number: 1 + (dx + `STEP_MOST`) + (2 `STEP_MOST` + 1)(dy + `STEP_MOST`); 0 for
/// none. A shell finds the sample again from the texel's centre: the texel's own, floor((x + 1/2) k), stepped by it.
pub(super) fn code(s: &Samples, [x, y]: [i64; 2], k: f64, q: u32) -> u32 {
    if q == NONE {
        return 0;
    }
    let own = |t: i64| ((t as f64 + 0.5) * k).floor() as i64;
    let (dx, dy) = ((q as usize % s.across) as i64 - own(x), (q as usize / s.across) as i64 - own(y));
    debug_assert!(dx.abs() <= STEP_MOST && dy.abs() <= STEP_MOST, "a channel's sample within the steps a tile names");
    (1 + (dx + STEP_MOST) + (2 * STEP_MOST + 1) * (dy + STEP_MOST)) as u32
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
