//! The terrain's channels as a shell draws them (laws.md 1.2): the liquid each sample passes on runs along its way
//! (`Samples::to`), drawn as a curve from the middle of the way into it (from its neighbour upstream that the most drains
//! through, `Samples::main`) to the middle of its own, bent at its centre (a quadratic Bézier), and on to the curve
//! through the next sample where that one's main way comes from elsewhere (a tributary joins); a sink of the open low
//! ground that liquid runs into ends the curve at its centre, under the low ground's liquid. The line winds about that
//! curve by its bends (`bends`). A channel is as wide as the channel law makes its discharge (`Physics::channel`: width
//! = w0 × discharge^b), and what a m² drained passes on never passes `MOST_RUNOFF`, so a channel of drained area a
//! covers no point farther than w0 × (`MOST_RUNOFF` a)^b / 2 from its line (its reach). Of the channels within their
//! reach, a texel keeps the one whose distance over the larger of its reach and the texel's size is least: the one that
//! covers it first as the flux grows, or, where every channel is narrower than a texel, the nearest; it keeps that
//! channel's sample and its distance to its line, signed by the side the texel lies on. A channel a shell may draw as
//! a line wider than itself (its drained area at least `SHOW_M` × the texel's size over `TEXEL_PX_MOST` × `WET_TIMES`)
//! reaches `LINE_REACH` texels too. Each number is a function of the seed, the map and the place alone.

use super::bends::{amplitude, ease, length, stand_in, wavelength, Piece, BEND_MOST};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::square::Square;
use pcore::space::terrain::SAMPLE_DEPTH;

/// The most straight pieces a curve is drawn as where its bends are not drawn, and the texels a piece spans at most:
/// fewer pieces where the texels are as large as the samples, as the curve departs from its chords by far less than
/// a texel.
const CURVE_STEPS: usize = 8;
const STEP_TEXELS: f64 = 0.5;
/// The straight pieces each bend is drawn as.
const STEPS_PER_BEND: f64 = 12.0;
/// A tile draws a channel's bends fully where their wavelength is at least `BEND_TEXELS` texels, and not at all under
/// `BEND_FROM`: shorter, they move the line by less than a texel. Between, they grow, the same at both ends of a joint.
const BEND_FROM: f64 = 4.0;
const BEND_TEXELS: f64 = 8.0;
/// The most liquid a m² drained passes on, as a depth, m/s: about the wettest month known on Earth (9.3 m of rain at
/// Cherrapunji, July 1861). No channel is wider than this makes it.
const MOST_RUNOFF: f64 = 3.6e-6;
/// What a m² of land passes on to the rivers in a year on Earth's mean, as a depth, m/s: 0.27 m a year, about 40,000
/// km³ a year from 1.5e8 km² of land (Dai and Trenberth 2002). The stand-in for a channel's discharge where a number
/// that never changes needs one (its bends, `bends`; which channels a tile holds as lines).
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
/// The most screen pixels a texel takes: two css pixels (`web/src/ground.ts` `TEXEL_CSS_PX`) of a screen of two pixels a
/// css pixel.
const TEXEL_PX_MOST: f64 = 4.0;
/// A tile holds no channel narrower than this share of a texel at `MOST_RUNOFF` that is not held as a line: it would
/// cover no more than this share of the texel's pixels.
const LEAST_TEXELS: f64 = 0.1;
/// The texels a line reaches from its centre: past the widest line a shell draws.
const LINE_REACH: f64 = 2.5;
/// The farthest the middle of a sample's line lies from its centre, samples: its curve's middle lies within a quarter
/// of a diagonal of it, and its bends move it by at most `bends::BEND_MOST`.
const NEXT_OFF: f64 = 0.36 + BEND_MOST;
/// The samples a curve lies within of its own sample's centre (it runs from the middle of the way in to the middle of
/// the way out, and on to the middle of the next sample's curve; its bends move it by at most `bends::BEND_MOST`).
const SPAN: i64 = 2;
/// The farthest a texel's channel's sample lies from the sample that holds the texel's centre, samples along each
/// axis: a tile names it by that step (`code`), which a 16-bit float holds exactly.
pub const STEP_MOST: i64 = 21;
/// The distance a texel holds where no channel reaches it, m (a 16-bit float holds it exactly).
const NO_CHANNEL_M: f64 = 6.0e4;

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

/// The field of the channels over a window of texels (`Channels::field`): of each texel its channel's sample (`NONE`
/// for none) and the distance to that channel's line, m, signed by the side (`NO_CHANNEL_M` for none).
pub(super) struct Field {
    pub dist: Vec<f64>,
    pub sample: Vec<u32>,
}

/// Where a tributary's line joins the next sample's (the middle of that one's curve, samples), the direction of that
/// curve there, that channel's bank: half the width it has at `WET_TIMES` × `TYPICAL_RUNOFF` (`bends::stand_in`),
/// samples, and the length of the tributary's last piece, which runs into it, samples. The tributary takes no texel
/// within that bank of the line it joins (taken straight through the join) and nearer the join than its last piece is
/// long, so in the water of the channel it joins every texel is that channel's.
#[derive(Clone, Copy)]
struct Join {
    at: [f64; 2],
    along: [f64; 2],
    bank: f64,
    last: f64,
}

impl Join {
    /// Does the point `p` (samples) lie in the water of the channel joined, where the tributary takes no texel?
    fn holds(&self, p: [f64; 2]) -> bool {
        let (dx, dy) = (p[0] - self.at[0], p[1] - self.at[1]);
        (dx * self.along[1] - dy * self.along[0]).abs() < self.bank && dx.hypot(dy) < self.last
    }
}

/// The channels of the samples `s` of a map, `metres` a sample, sized by `law`; the phase of each sample's bends
/// (`bends::phases`); the channel's reach per m² drained to the power b, and the largest drained area.
pub(super) struct Channels<'a> {
    s: &'a Samples,
    law: Law,
    phase: &'a [f32],
    metres: f64,
    cap: f64,
    most: f64,
}

/// The centre of sample `k` of `s`, in samples.
fn centre(s: &Samples, k: usize) -> [f64; 2] {
    [(k % s.across) as f64 + 0.5, (k / s.across) as f64 + 0.5]
}

fn mid(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
}

/// The three points of the curve through sample `k` of `s`, in samples: from the middle of its main way in (its centre
/// at a head) through its centre to the middle of its way out; a sink that liquid runs into ends at its centre; none
/// for a sink none runs into.
pub(super) fn base(s: &Samples, k: usize) -> Option<[[f64; 2]; 3]> {
    let c = centre(s, k);
    let start = (s.main[k] != NONE).then(|| mid(centre(s, s.main[k] as usize), c));
    match (s.to[k], start) {
        (NONE, None) => None,
        (NONE, Some(a)) => Some([a, mid(a, c), c]),
        (to, a) => Some([a.unwrap_or(c), c, mid(c, centre(s, to as usize))]),
    }
}

/// The signed distance from `p` to the segment from `a` to `b` (positive on its left), and that distance's square.
fn to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> (f64, f64) {
    let (d, q) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let len = d[0] * d[0] + d[1] * d[1];
    let t = if len > 0.0 { ((q[0] * d[0] + q[1] * d[1]) / len).clamp(0.0, 1.0) } else { 0.0 };
    let (x, y) = (q[0] - t * d[0], q[1] - t * d[1]);
    let side = if d[0] * q[1] - d[1] * q[0] >= 0.0 { 1.0 } else { -1.0 };
    (side, x * x + y * y)
}

impl<'a> Channels<'a> {
    /// The channels of `s` on `map`, sized by `law`, with the phase of their bends `phase` (`bends::phases`).
    pub(super) fn new(map: &Square, s: &'a Samples, law: Law, phase: &'a [f32]) -> Channels<'a> {
        let most = s.up.iter().fold(0.0f64, |m, &a| m.max(a));
        Channels { s, law, phase, metres: map.length(SAMPLE_DEPTH), cap: law.width * MOST_RUNOFF.powf(law.width_exp) / 2.0, most }
    }

    /// The piece of sample `k` (none for a sink none runs into) on texels of `texel` m: the half amplitude of its bends
    /// at each end the least of the two samples' that meet there, times how far their bends are drawn (`BEND_FROM`;
    /// none at a head and at a sink), so the two pieces of a joint meet.
    fn piece(&self, k: usize, texel: f64) -> Option<Piece> {
        let s = self.s;
        let b = base(s, k)?;
        let wave = wavelength(&self.law, s, k);
        let (to, main) = (s.to[k], s.main[k]);
        if wave < BEND_FROM * texel || (to == NONE && main == NONE) {
            return Some(Piece { base: b, amp: [0.0; 2], phase: 0.0, cycles: 0.0 });
        }
        let amp = |k: usize| amplitude(&self.law, s, self.metres, k);
        let own = amp(k);
        let joint = |j: u32| {
            if j == NONE {
                return 0.0;
            }
            let drawn = ease(BEND_FROM, BEND_TEXELS, wave.min(wavelength(&self.law, s, j as usize)) / texel);
            if drawn == 0.0 { 0.0 } else { own.min(amp(j as usize)) * drawn }
        };
        let ends = [joint(main), joint(to)];
        Some(Piece { base: b, amp: ends, phase: self.phase[k] as f64, cycles: length(b) * self.metres / wave })
    }

    /// The points of the line of sample `k` whose piece is `p`, on texels of `texel` m: the piece in straight pieces
    /// enough for its bends and its texels, and on to the middle of the next sample's line where a tributary joins;
    /// and there, that point and the direction of the line it joins (`Join`).
    fn line(&self, k: usize, p: &Piece, texel: f64) -> (Vec<[f64; 2]>, Option<Join>) {
        let s = self.s;
        let plain = ((length(p.base) * self.metres / (STEP_TEXELS * texel)).ceil() as usize).clamp(1, CURVE_STEPS);
        let steps = if p.amp != [0.0; 2] { CURVE_STEPS.max((p.cycles * STEPS_PER_BEND).ceil() as usize) } else { plain };
        let mut pts: Vec<[f64; 2]> = (0..=steps).map(|t| p.at(t as f64 / steps as f64)).collect();
        let j = s.to[k];
        if j == NONE || s.main[j as usize] == k as u32 {
            return (pts, None);
        }
        let join = self.piece(j as usize, texel).map(|next| {
            let ([a, _, c], at, from) = (next.base, next.at(0.5), pts[pts.len() - 1]);
            let len = (c[0] - a[0]).hypot(c[1] - a[1]).max(f64::MIN_POSITIVE);
            let bank = stand_in(&self.law, s, j as usize).0 * WET_TIMES.powf(self.law.width_exp) / (2.0 * self.metres);
            Join { at, along: [(c[0] - a[0]) / len, (c[1] - a[1]) / len], bank, last: (at[0] - from[0]).hypot(at[1] - from[1]) }
        });
        pts.extend(join.map(|j| j.at));
        (pts, join)
    }

    /// The field over the texels from column `x0` and row `y0`, `wide` x `high` of them, each `k` samples a side (their
    /// centres at (x + 1/2) k samples): each line near enough drawn into the texels within its reach.
    pub(super) fn field(&self, [x0, y0]: [i64; 2], [wide, high]: [usize; 2], k: f64) -> Field {
        let s = self.s;
        let texel = k * self.metres;
        let mut f = Field { dist: vec![NO_CHANNEL_M; wide * high], sample: vec![NONE; wide * high] };
        let mut best = vec![f64::INFINITY; wide * high];
        // The reach of a line in samples, no farther than a texel may name its sample by its step from its own.
        let most_reach = (STEP_MOST - SPAN - 1) as f64;
        // A line shows where a pixel takes at least a texel over `TEXEL_PX_MOST`, its discharge at most `WET_TIMES`
        // × `TYPICAL_RUNOFF` per m² drained.
        let line_from = SHOW_M * texel / (TEXEL_PX_MOST * WET_TIMES);
        let reach_of = |a: f64| {
            let own = self.cap * a.powf(self.law.width_exp);
            let reach = if a >= line_from { own.max(LINE_REACH * texel) } else { own };
            (own, (reach / self.metres).min(most_reach))
        };
        // A channel that is held by no line and whose widest is under `LEAST_TEXELS` of a texel is not held.
        let least = (LEAST_TEXELS * texel / self.cap).powf(1.0 / self.law.width_exp).min(line_from);
        let window = SPAN + reach_of(self.most).1.ceil() as i64 + 1;
        let lo = |t0: i64| (t0 as f64 * k).floor() as i64 - window;
        let hi = |t0: i64, n: usize| ((t0 + n as i64) as f64 * k).floor() as i64 + window;
        let texels = |from: f64, to: f64, t0: i64, n: usize| ((from / k - 0.5).ceil() as i64).max(t0)..=((to / k - 0.5).floor() as i64).min(t0 + n as i64 - 1);
        for r in lo(y0).max(0)..=hi(y0, high).min(s.down as i64 - 1) {
            for c in lo(x0).max(0)..=hi(x0, wide).min(s.across as i64 - 1) {
                let q = r as usize * s.across + c as usize;
                if s.up[q] < least {
                    continue;
                }
                let Some(b) = base(s, q) else { continue };
                let (own, reach) = reach_of(s.up[q]);
                // The box the line lies in: its curve's points and, where a tributary joins, the next sample's centre,
                // widened by its bends, by how far the next sample's line lies from its centre and by its reach: none
                // of the window's texels in it, and the line is not drawn.
                let next = if s.to[q] != NONE { centre(s, s.to[q] as usize) } else { b[2] };
                let edge = |a: usize, low: bool| {
                    let v = b.iter().map(|p| p[a]).fold(next[a], if low { f64::min } else { f64::max });
                    v + if low { -1.0 } else { 1.0 } * (reach + NEXT_OFF)
                };
                if texels(edge(0, true), edge(0, false), x0, wide).is_empty() || texels(edge(1, true), edge(1, false), y0, high).is_empty() {
                    continue;
                }
                let Some(p) = self.piece(q, texel) else { continue };
                let (pts, join) = self.line(q, &p, texel);
                let scale = own.max(texel) / self.metres;
                // Each piece into the texels within its reach: a texel takes the line where it beats what it holds and
                // lies out of the water of the channel it joins.
                for w in pts.windows(2) {
                    let (lo, hi) = ([w[0][0].min(w[1][0]), w[0][1].min(w[1][1])], [w[0][0].max(w[1][0]), w[0][1].max(w[1][1])]);
                    for y in texels(lo[1] - reach, hi[1] + reach, y0, high) {
                        for x in texels(lo[0] - reach, hi[0] + reach, x0, wide) {
                            let at = [(x as f64 + 0.5) * k, (y as f64 + 0.5) * k];
                            let (side, d2) = to_segment(at, w[0], w[1]);
                            let i = (y - y0) as usize * wide + (x - x0) as usize;
                            if d2 <= reach * reach && d2.sqrt() / scale < best[i] && !join.is_some_and(|j| j.holds(at)) {
                                (best[i], f.dist[i], f.sample[i]) = (d2.sqrt() / scale, side * d2.sqrt() * self.metres, q as u32);
                            }
                        }
                    }
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
