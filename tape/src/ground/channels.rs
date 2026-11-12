//! The terrain's channels as a shell draws them (laws.md 1.2): the liquid each sample passes on runs along its way
//! (`Samples::to`), drawn as a curve from the middle of the way into it (from its neighbour upstream that the most drains
//! through, `Samples::main`) to the middle of its own, bent at its centre (a quadratic Bézier), and on to the line of
//! the next sample where that one's main way comes from elsewhere (a tributary joins); a sink of the open low ground
//! that liquid runs into ends the curve at its centre, under the low ground's liquid. The line swings about that curve
//! in its bends (`bends`, `belt`). A channel is as wide as the channel law makes its discharge (`Physics::channel`:
//! width = w0 × discharge^b), and what a m² drained passes on never passes `MOST_RUNOFF`, so a channel of drained area
//! a covers no point farther than w0 × (`MOST_RUNOFF` a)^b / 2 from its line (its reach). Of the channels within their
//! reach, a texel keeps the one whose distance over the larger of its reach and the texel's size is least: the one that
//! covers it first as the flux grows, or, where every channel is narrower than a texel, the nearest; it keeps that
//! channel's sample, its distance to its line, signed by the side the texel lies on, and of the line's nearest point
//! how far along the channel it lies and how the line curves there. A channel a shell may draw as a line wider than
//! itself (its drained area at least `SHOW_M` × the texel's size over `TEXEL_PX_MOST` × `WET_TIMES`) reaches
//! `LINE_REACH` texels too. Each number is a function of the seed, the map and the place alone.

use super::belt::STEPS;
use super::bends::{ease, Bends, Mark, Reaches, Start, BEND_MOST, CURVE_STEPS};
use super::stems::SMOOTH_MOST;
use super::valley::{Carve, VALLEY_FROM, VALLEY_M};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;

/// A stretch of a line is drawn in straight pieces no shorter than this many texels (and a stretch without bends in
/// at most `bends::CURVE_STEPS`): a curve departs from shorter chords by far less than a texel.
const STEP_TEXELS: f64 = 0.5;
/// A tile draws a channel's bends fully where their wavelength is at least `BEND_TEXELS` texels, and not at all under
/// `BEND_FROM`: its bed is then a texel or two wide, and from farther a shell draws it as a line wider than its bends
/// (`SHOW_M`). Between, they grow, the same at both ends of a joint.
const BEND_FROM: f64 = 12.0;
const BEND_TEXELS: f64 = 24.0;
/// The most liquid a m² drained passes on, as a depth, m/s: about the wettest month known on Earth (9.3 m of rain at
/// Cherrapunji, July 1861). No channel is wider than this makes it.
const MOST_RUNOFF: f64 = 3.6e-6;
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
/// The texels a line reaches from its centre: past the widest line a shell draws.
const LINE_REACH: f64 = 2.5;
/// The farthest the middle of a sample's line lies from its centre, samples: its curve's middle lies within a quarter
/// of a diagonal of it, and its bends move it by at most `bends::BEND_MOST`.
const NEXT_OFF: f64 = 0.36 + BEND_MOST + SMOOTH_MOST;
/// The samples a line lies within of its own sample's centre: it runs from the middle of the way in to the middle of
/// the way out, and on to the line of the next sample (a diagonal away, itself up to `NEXT_OFF` from its centre).
const SPAN: i64 = 4;
/// The farthest a texel's channel's sample lies from the sample that holds the texel's centre, samples along each
/// axis: a tile names it by that step (`code`), which a 16-bit float holds exactly.
pub const STEP_MOST: i64 = 21;
/// The distance a texel holds where no channel reaches it, m (a 16-bit float holds it exactly).
const NO_CHANNEL_M: f64 = 6.0e4;
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
/// for none), the distance to that channel's line, m, signed by the side (`NO_CHANNEL_M` for none), and of the line's
/// nearest point how far along the channel it lies, m, and its curvature, 1/m (positive where the line turns toward
/// the side the distance is positive on); and the valleys about the lines (`valley`): the sum of their weights at the
/// texel and of their floors' heights so weighted, m.
pub(super) struct Field {
    pub dist: Vec<f64>,
    pub sample: Vec<u32>,
    pub along: Vec<f64>,
    pub curve: Vec<f64>,
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

/// A point of a channel's line as drawn: its place, samples; the share of its piece's curve it lies along (0 to 1);
/// how far along the channel's line from its head, m; and the line's curvature there, 1/m.
#[derive(Clone, Copy)]
pub(super) struct Vertex {
    pub p: [f64; 2],
    pub t: f64,
    pub s: f64,
    pub curve: f64,
}

/// Where a tributary's line joins the next sample's (the middle of that one's line, samples), the direction of that
/// line there, that channel's bank: half the width of its bed (`bends::Reaches::bed_of`), samples, and the length of
/// the tributary's last piece, which runs into it, samples. The tributary takes no texel within that bank of the line
/// it joins (taken straight through the join) and nearer the join than its last piece is long, so in the bed of the
/// channel it joins every texel is that channel's.
#[derive(Clone, Copy)]
pub(super) struct Join {
    at: [f64; 2],
    along: [f64; 2],
    bank: f64,
    last: f64,
}

impl Join {
    /// Does the point `p` (samples) lie in the bed of the channel joined, where the tributary takes no texel?
    fn holds(&self, p: [f64; 2]) -> bool {
        let (dx, dy) = (p[0] - self.at[0], p[1] - self.at[1]);
        (dx * self.along[1] - dy * self.along[0]).abs() < self.bank && dx.hypot(dy) < self.last
    }
}

/// The channels of a map (`bends::Reaches`) and how each one's piece starts (`bends::Bends`); the channel's reach per
/// m² drained to the power b, and the largest drained area.
pub(super) struct Channels<'a> {
    r: Reaches<'a>,
    bends: &'a Bends,
    cap: f64,
    most: f64,
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

/// The steps of a half bend (`belt::STEPS`) a tile keeps one of, where each is `stride` m long on texels of `texel` m:
/// the fewest that make the pieces drawn at least `STEP_TEXELS` texels long, and a count that divides the half bend.
fn kept(stride: f64, texel: f64) -> u32 {
    (1..=STEPS).find(|n| STEPS.is_multiple_of(*n) && stride * *n as f64 >= STEP_TEXELS * texel).unwrap_or(STEPS)
}

impl<'a> Channels<'a> {
    /// The channels `r`, their pieces starting as `bends` holds.
    pub(super) fn new(r: Reaches<'a>, bends: &'a Bends) -> Channels<'a> {
        let most = r.s.up.iter().fold(0.0f64, |m, &a| m.max(a));
        Channels { r, bends, cap: r.law.width * MOST_RUNOFF.powf(r.law.width_exp) / 2.0, most }
    }

    /// The points of sample `k`'s own piece on texels of `texel` m, from its start to its end (none for a sink none
    /// runs into): its walked bends (`bends::Reaches::walk`) as far across its curve as the tile draws them
    /// (`BEND_FROM`: at each end as the joint there is drawn, so two pieces meet; none where the line ends, at a join
    /// or a sink), or its curve where it has none to draw.
    pub(super) fn piece(&self, k: usize, texel: f64) -> Option<Vec<Vertex>> {
        let (r, s) = (&self.r, self.r.s);
        let frame = r.frame(k)?;
        let (len, wave, start) = (frame.len(), r.wavelength(k), self.bends.start(k));
        let joint = |j: u32| ease(BEND_FROM, BEND_TEXELS, if j == NONE { wave } else { wave.min(r.wavelength(j as usize)) } / texel);
        let runs_on = s.to[k] != NONE && s.main[s.to[k] as usize] == k as u32;
        let ends = [joint(s.main[k]), if runs_on { joint(s.to[k]) } else { 0.0 }];
        if ends == [0.0; 2] || (matches!(start, Start::Straight(_)) && r.room(k, &frame).belt == 0.0) {
            // Its curve, the line's own length spread evenly along it (the bends' too, where they are too small to draw).
            let (from, to) = (start.along(), r.walk(k, &frame, start, &mut |_| {}).along());
            let n = ((len / (STEP_TEXELS * texel)).ceil() as usize).clamp(1, CURVE_STEPS);
            return Some((0..=n).map(|i| i as f64 / n as f64).map(|x| Vertex { p: frame.at(x * len, 0.0), t: frame.share(x * len), s: from + (to - from) * x, curve: 0.0 }).collect());
        }
        let mut marks = Vec::new();
        r.walk(k, &frame, start, &mut |m: Mark| {
            if m.step.is_multiple_of(kept(m.stride, texel)) {
                marks.push(m);
            }
        });
        // A bend drawn less than fully is drawn that much from the curve: across it, and along it from where the
        // line's own length puts the point (a loop's limb that runs back up the valley would else fold the line over
        // itself on the curve).
        let (from, run) = (marks[0].s, (marks[marks.len() - 1].s - marks[0].s).max(1e-9));
        let point = |m: &Mark| {
            let even = len * (m.s - from) / run;
            let drawn = ends[0] + (ends[1] - ends[0]) * even / len;
            let u = even + (m.u - even) * drawn;
            Vertex { p: frame.at(u, m.v * drawn), t: frame.share(u), s: m.s, curve: m.curve * drawn }
        };
        Some(marks.iter().map(point).collect())
    }

    /// The points of the line of sample `k` on texels of `texel` m: its own piece, and on to the middle of the next
    /// sample's line where a tributary joins; and there, that point and the direction of the line it joins (`Join`).
    pub(super) fn line(&self, k: usize, texel: f64) -> Option<(Vec<Vertex>, Option<Join>)> {
        let (r, s) = (&self.r, self.r.s);
        let mut pts = self.piece(k, texel)?;
        let j = s.to[k];
        if j == NONE || s.main[j as usize] == k as u32 {
            return Some((pts, None));
        }
        let join = self.piece(j as usize, texel).map(|next| {
            let i = (0..next.len()).min_by(|&a, &b| (next[a].t - 0.5).abs().total_cmp(&(next[b].t - 0.5).abs())).expect("a piece has points");
            let (a, c, at, from) = (next[i.saturating_sub(1)].p, next[(i + 1).min(next.len() - 1)].p, next[i].p, pts[pts.len() - 1]);
            let len = (c[0] - a[0]).hypot(c[1] - a[1]).max(f64::MIN_POSITIVE);
            let last = (at[0] - from.p[0]).hypot(at[1] - from.p[1]);
            pts.push(Vertex { p: at, t: 1.0, s: from.s + last * r.metres, curve: 0.0 });
            Join { at, along: [(c[0] - a[0]) / len, (c[1] - a[1]) / len], bank: r.bed_of(j as usize).0 / (2.0 * r.metres), last }
        });
        Some((pts, join))
    }

    /// The field over the window `win`: each line near enough drawn into the texels within its reach, and the valleys
    /// about the lines on texels finer than the samples.
    pub(super) fn field(&self, win: Window) -> Field {
        let (s, metres, k) = (self.r.s, self.r.metres, win.k);
        let (texel, n) = (k * metres, win.wide * win.high);
        let mut f = Field { dist: vec![NO_CHANNEL_M; n], sample: vec![NONE; n], along: vec![0.0; n], curve: vec![0.0; n], valley: vec![0.0; n], weight: vec![0.0; n] };
        let mut carve = Carve::new(n);
        let reach_valley = if k < 1.0 { VALLEY_M / metres } else { 0.0 };
        let mut best = vec![f64::INFINITY; n];
        // The reach of a line in samples, no farther than a texel may name its sample by its step from its own.
        let most_reach = (STEP_MOST - SPAN - 1) as f64;
        // A line shows where a pixel takes at least a texel over `TEXEL_PX_MOST`, its discharge at most `WET_TIMES`
        // × `TYPICAL_RUNOFF` per m² drained.
        let line_from = SHOW_M * texel / (TEXEL_PX_MOST * WET_TIMES);
        let reach_of = |a: f64| {
            let own = self.cap * a.powf(self.r.law.width_exp);
            let reach = if a >= line_from { own.max(LINE_REACH * texel) } else { own };
            (own, (reach / metres).min(most_reach))
        };
        // A channel that is held by no line and whose widest is under `LEAST_TEXELS` of a texel is not held.
        let least = (LEAST_TEXELS * texel / self.cap).powf(1.0 / self.r.law.width_exp).min(line_from);
        let window = SPAN + reach_of(self.most).1.max(reach_valley).ceil() as i64 + 1;
        let lo = |t0: i64| (t0 as f64 * k).floor() as i64 - window;
        let hi = |t0: i64, n: usize| ((t0 + n as i64) as f64 * k).floor() as i64 + window;
        for r in lo(win.y0).max(0)..=hi(win.y0, win.high).min(s.down as i64 - 1) {
            for c in lo(win.x0).max(0)..=hi(win.x0, win.wide).min(s.across as i64 - 1) {
                let q = r as usize * s.across + c as usize;
                if s.up[q] < least {
                    continue;
                }
                let Some(b) = base(s, self.r.at, q) else { continue };
                let (own, line_reach) = reach_of(s.up[q]);
                let carves = s.up[q] >= VALLEY_FROM && reach_valley > 0.0 && s.pond[q] == NONE;
                let reach = if carves { line_reach.max(reach_valley) } else { line_reach };
                // The box the line lies in: its curve's points and, where a tributary joins, the next sample's centre,
                // widened by its bends, by how far the next sample's line lies from its centre and by its reach: none
                // of the window's texels in it, and the line is not drawn.
                let next = if s.to[q] != NONE { self.r.at[s.to[q] as usize] } else { b[2] };
                let edge = |a: usize, low: bool| {
                    let v = b.iter().map(|p| p[a]).fold(next[a], if low { f64::min } else { f64::max });
                    v + if low { -1.0 } else { 1.0 } * (reach + NEXT_OFF)
                };
                if win.texels(edge(0, true), edge(0, false), 0).is_empty() || win.texels(edge(1, true), edge(1, false), 1).is_empty() {
                    continue;
                }
                let Some((pts, join)) = self.line(q, texel) else { continue };
                let scale = own.max(texel) / metres;
                // Each piece into the texels within its reach: a texel takes the line where it beats what it holds and
                // lies out of the bed of the channel it joins.
                for w in pts.windows(2) {
                    let (a, b) = (w[0].p, w[1].p);
                    for y in win.texels(a[1].min(b[1]) - line_reach, a[1].max(b[1]) + line_reach, 1) {
                        for x in win.texels(a[0].min(b[0]) - line_reach, a[0].max(b[0]) + line_reach, 0) {
                            let (at, i) = win.texel(x, y);
                            let (side, d2, t) = to_segment(at, a, b);
                            if d2 <= line_reach * line_reach && d2.sqrt() / scale < best[i] && !join.is_some_and(|j| j.holds(at)) {
                                (best[i], f.dist[i], f.sample[i]) = (d2.sqrt() / scale, side * d2.sqrt() * metres, q as u32);
                                (f.along[i], f.curve[i]) = (w[0].s + (w[1].s - w[0].s) * t, w[0].curve + (w[1].curve - w[0].curve) * t);
                            }
                        }
                    }
                }
                if carves {
                    carve.line(&mut f, &win, s, q, &pts, reach_valley);
                }
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
