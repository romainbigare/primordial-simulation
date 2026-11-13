def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

C = 'tape/src/ground/channels.rs'
t = open(C).read()
i = t.index("    /// The points of sample `k`'s own piece on texels of `texel` m, from its start to its end")
j = t.index("/// The step from the sample that holds the centre of the texel at column `x` and row `y`")
new = '''    /// Sample `k`'s own piece as drawn on texels of `texel` m, from its start to its end (none for a sink none runs
    /// into): its curve moved by its wander (`bends::Bends::leans`), and that with its walked bends
    /// (`bends::Reaches::walk`), each as far across as the tile draws it (`BEND_FROM`: at each end as the joint there is
    /// drawn, so two pieces meet; no bend where the line ends, at a join or a sink).
    pub(super) fn piece(&self, k: usize, texel: f64) -> Option<Drawn> {
        let (r, s) = (&self.r, self.r.s);
        let frame = r.frame(k)?;
        let (len, start) = (frame.len(), self.bends.start(k));
        // How far a sway of length `own` (its neighbour `j`'s by `of`) is drawn where the two pieces meet.
        let joint = |own: f64, j: u32, of: &dyn Fn(usize) -> f64| ease(BEND_FROM, BEND_TEXELS, if j == NONE { own } else { own.min(of(j as usize)) } / texel);
        let runs_on = s.to[k] != NONE && s.main[s.to[k] as usize] == k as u32;
        let bends = [joint(r.wavelength(k), s.main[k], &|j| r.wavelength(j)), if runs_on { joint(r.wavelength(k), s.to[k], &|j| r.wavelength(j)) } else { 0.0 }];
        let sways = [joint(r.wander_length(k), s.main[k], &|j| r.wander_length(j)), joint(r.wander_length(k), s.to[k], &|j| r.wander_length(j))];
        let between = |ends: [f64; 2], u: f64| ends[0] + (ends[1] - ends[0]) * (u / len).clamp(0.0, 1.0);
        let lean = |u: f64| between(sways, u) * self.bends.leans(r, k, &frame, u.clamp(0.0, len));
        let n = ((len / (STEP_TEXELS * texel)).ceil() as usize).clamp(1, CALM_STEPS);
        let calm: Vec<[f64; 2]> = (0..=n).map(|i| i as f64 * len / n as f64).map(|u| frame.at(u, lean(u))).collect();
        if bends == [0.0; 2] || (matches!(start, Start::Straight(_)) && r.room(k, &frame).belt == 0.0) {
            // No bends to draw: its wander alone, the line's own length spread evenly along it (the bends' too, where
            // they are too small to draw).
            let (from, to) = (start.along(), r.walk(k, &frame, start, &mut |_| {}).along());
            let line = calm.iter().enumerate().map(|(i, &p)| Vertex { p, t: frame.share(i as f64 * len / n as f64), s: from + (to - from) * i as f64 / n as f64 }).collect();
            return Some(Drawn { line, calm });
        }
        let mut marks = Vec::new();
        r.walk(k, &frame, start, &mut |m: Mark| {
            if m.step.is_multiple_of(kept(m.stride, texel)) {
                marks.push(m);
            }
        });
        // A bend drawn less than fully is drawn that much from the wander: across it, and along it from where the
        // line's own length puts the point (a loop's limb that runs back up the valley would else fold the line over
        // itself).
        let (from, run) = (marks[0].s, (marks[marks.len() - 1].s - marks[0].s).max(1e-9));
        let point = |m: &Mark| {
            let even = len * (m.s - from) / run;
            let drawn = between(bends, even);
            let u = even + (m.u - even) * drawn;
            Vertex { p: frame.at(u, lean(u) + m.v * drawn), t: frame.share(u), s: m.s }
        };
        Some(Drawn { line: marks.iter().map(point).collect(), calm })
    }

    /// The line of sample `k` on texels of `texel` m: its own piece, and on to the middle of the next sample's line
    /// where a tributary joins (with its bends and without); and there, that point and the direction of the line it
    /// joins (`Join`).
    pub(super) fn line(&self, k: usize, texel: f64) -> Option<(Drawn, Option<Join>)> {
        let (r, s) = (&self.r, self.r.s);
        let mut own = self.piece(k, texel)?;
        let j = s.to[k];
        if j == NONE || s.main[j as usize] == k as u32 {
            return Some((own, None));
        }
        let join = self.piece(j as usize, texel).map(|next| {
            let i = (0..next.line.len()).min_by(|&a, &b| (next.line[a].t - 0.5).abs().total_cmp(&(next.line[b].t - 0.5).abs())).expect("a piece has points");
            let (a, c, at, from) = (next.line[i.saturating_sub(1)].p, next.line[(i + 1).min(next.line.len() - 1)].p, next.line[i].p, own.line[own.line.len() - 1]);
            let len = (c[0] - a[0]).hypot(c[1] - a[1]).max(f64::MIN_POSITIVE);
            let last = (at[0] - from.p[0]).hypot(at[1] - from.p[1]);
            own.line.push(Vertex { p: at, t: 1.0, s: from.s + last * r.metres });
            own.calm.push(next.calm[next.calm.len() / 2]);
            Join { at, along: [(c[0] - a[0]) / len, (c[1] - a[1]) / len], bank: r.bed_of(j as usize).0 / (2.0 * r.metres), last }
        });
        Some((own, join))
    }

    /// The field over the window `win`: each line near enough drawn into the texels within its reach, and the valleys
    /// about the lines on texels finer than the samples.
    pub(super) fn field(&self, win: Window) -> Field {
        let (s, metres, k) = (self.r.s, self.r.metres, win.k);
        let (texel, n) = (k * metres, win.wide * win.high);
        let mut f = Field { dist: vec![NO_CHANNEL_M; n], sample: vec![NONE; n], along: vec![0.0; n], calm: vec![NO_CHANNEL_M; n], valley: vec![0.0; n], weight: vec![0.0; n] };
        let mut carve = Carve::new(n);
        let reach_valley = if k < 1.0 { VALLEY_M / metres } else { 0.0 };
        // Of each texel, how near the line it holds and the line without bends it holds lie, over their reach.
        let mut best = vec![[f64::INFINITY; 2]; n];
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
                // widened by its sway, by how far the next sample's line lies from its centre and by its reach: none
                // of the window's texels in it, and the line is not drawn.
                let next = if s.to[q] != NONE { self.r.at[s.to[q] as usize] } else { b[2] };
                let edge = |a: usize, low: bool| {
                    let v = b.iter().map(|p| p[a]).fold(next[a], if low { f64::min } else { f64::max });
                    v + if low { -1.0 } else { 1.0 } * (reach + NEXT_OFF)
                };
                if win.texels(edge(0, true), edge(0, false), 0).is_empty() || win.texels(edge(1, true), edge(1, false), 1).is_empty() {
                    continue;
                }
                let Some((drawn, join)) = self.line(q, texel) else { continue };
                let scale = own.max(texel) / metres;
                // Each piece of a line into the texels within its reach; `take` says whether a texel takes it (it
                // beats what the texel holds and the texel lies out of the bed of the channel it joins).
                let mut lay = |a: [f64; 2], b: [f64; 2], which: usize, take: &mut dyn FnMut(usize, f64, f64)| {
                    for y in win.texels(a[1].min(b[1]) - line_reach, a[1].max(b[1]) + line_reach, 1) {
                        for x in win.texels(a[0].min(b[0]) - line_reach, a[0].max(b[0]) + line_reach, 0) {
                            let (at, i) = win.texel(x, y);
                            let (side, d2, t) = to_segment(at, a, b);
                            if d2 <= line_reach * line_reach && d2.sqrt() / scale < best[i][which] && !join.is_some_and(|j| j.holds(at)) {
                                best[i][which] = d2.sqrt() / scale;
                                take(i, side * d2.sqrt() * metres, t);
                            }
                        }
                    }
                };
                for w in drawn.line.windows(2) {
                    lay(w[0].p, w[1].p, 0, &mut |i, d, t| (f.dist[i], f.sample[i], f.along[i]) = (d, q as u32, w[0].s + (w[1].s - w[0].s) * t));
                }
                for w in drawn.calm.windows(2) {
                    lay(w[0], w[1], 1, &mut |i, d, _| f.calm[i] = d);
                }
                if carves {
                    carve.line(&mut f, &win, s, q, &drawn.line, reach_valley);
                }
            }
        }
        f
    }
}

'''
t = t[:i] + new + t[j:]
open(C, 'w').write(t)
edit(C, [
 ("""//! channel's sample, its distance to its line, signed by the side the texel lies on, and of the line's nearest point
//! how far along the channel it lies and how the line curves there. A channel a shell may draw as a line wider than""",
  """//! channel's sample, its distance to its line, signed by the side the texel lies on, of the line's nearest point how
//! far along the channel it lies, and its distance to the line without its bends (its curve and its wander alone:
//! from afar a shell draws a channel as a line wider than its bends, about that one). A channel a shell may draw as a line wider than"""),
 ("""use super::belt::STEPS;
use super::bends::{ease, Bends, Mark, Reaches, Start, BEND_MOST, CURVE_STEPS};""", """use super::belt::STEPS;
use super::bends::{ease, Bends, Mark, Reaches, Start, BEND_MOST};"""),
 ("""/// A stretch of a line is drawn in straight pieces no shorter than this many texels (and a stretch without bends in
/// at most `bends::CURVE_STEPS`): a curve departs from shorter chords by far less than a texel.
const STEP_TEXELS: f64 = 0.5;
/// A tile draws a channel's bends fully where their wavelength is at least `BEND_TEXELS` texels, and not at all under
/// `BEND_FROM`: its bed is then a texel or two wide, and from farther a shell draws it as a line wider than its bends
/// (`SHOW_M`). Between, they grow, the same at both ends of a joint.
const BEND_FROM: f64 = 12.0;
const BEND_TEXELS: f64 = 24.0;""", """/// A stretch of a line is drawn in straight pieces no shorter than this many texels (and a stretch without bends in
/// at most `CALM_STEPS`): a curve departs from shorter chords by far less than a texel.
const STEP_TEXELS: f64 = 0.5;
const CALM_STEPS: usize = 16;
/// A tile draws a channel's sway (its bends, and its wander) fully where its length is at least `BEND_TEXELS` texels,
/// and not at all under `BEND_FROM`: shorter, it moves the line by less than a texel. Between, it grows, the same at
/// both ends of a joint.
const BEND_FROM: f64 = 4.0;
const BEND_TEXELS: f64 = 8.0;"""),
 ("""/// for none), the distance to that channel's line, m, signed by the side (`NO_CHANNEL_M` for none), and of the line's
/// nearest point how far along the channel it lies, m, and its curvature, 1/m (positive where the line turns toward
/// the side the distance is positive on); and the valleys about the lines (`valley`): the sum of their weights at the
/// texel and of their floors' heights so weighted, m.
pub(super) struct Field {
    pub dist: Vec<f64>,
    pub sample: Vec<u32>,
    pub along: Vec<f64>,
    pub curve: Vec<f64>,""", """/// for none), the distance to that channel's line, m, signed by the side (`NO_CHANNEL_M` for none), of the line's
/// nearest point how far along the channel it lies, m, and the signed distance to the nearest channel's line without
/// its bends, m (`NO_CHANNEL_M` for none); and the valleys about the lines (`valley`): the sum of their weights at the
/// texel and of their floors' heights so weighted, m.
pub(super) struct Field {
    pub dist: Vec<f64>,
    pub sample: Vec<u32>,
    pub along: Vec<f64>,
    pub calm: Vec<f64>,"""),
 ("""/// A point of a channel's line as drawn: its place, samples; the share of its piece's curve it lies along (0 to 1);
/// how far along the channel's line from its head, m; and the line's curvature there, 1/m.
#[derive(Clone, Copy)]
pub(super) struct Vertex {
    pub p: [f64; 2],
    pub t: f64,
    pub s: f64,
    pub curve: f64,
}""", """/// A point of a channel's line as drawn: its place, samples; the share of its piece's curve it lies along (0 to 1);
/// and how far along the channel's line from its head, m.
#[derive(Clone, Copy)]
pub(super) struct Vertex {
    pub p: [f64; 2],
    pub t: f64,
    pub s: f64,
}

/// A piece of a channel's line as a tile draws it: its points, and the points of the same piece without its bends.
pub(super) struct Drawn {
    pub line: Vec<Vertex>,
    pub calm: Vec<[f64; 2]>,
}"""),
])
edit('tape/src/ground/mod.rs', [
 ("""mod belt;
mod bends;
mod channels;
mod half;""", """mod belt;
mod bends;
mod channels;
mod frame;
mod half;"""),
 ("""mod valley;
mod wetness;""", """mod valley;
mod wander;
mod wetness;"""),
 ("""    /// the nearest point of that channel's line, how far along the channel it lies, m, modulo `ALONG_WRAP_M`, and the
    /// line's curvature there, 1/m, positive where it turns toward the side the distance is positive on (0 for none).
    /// Texels off the map are those of its edge.""", """    /// the nearest point of that channel's line, how far along the channel it lies, m, modulo `ALONG_WRAP_M`, and the
    /// distance to the nearest channel's line without its bends, m, signed by the side (60,000 where none reaches it).
    /// Texels off the map are those of its edge."""),
 ("""half(along as f32), half(near.curve[i] as f32)]);""", """half(along as f32), half(near.calm[i] as f32)]);"""),
 ("""//! the channel it is nearest by the channel law (`channels`), with how far along that channel's line it lies and how
//! the line curves there; and apart""", """//! the channel it is nearest by the channel law (`channels`), with how far along that channel's line it lies and how
//! far from that line without its bends; and apart"""),
])
T = 'tape/src/ground/tests.rs'
edit(T, [
 ("""        let pts = ch.line(q as usize, k * metres).expect("a channel").0;
        let own = pts.windows(2)""", """        let pts = ch.line(q as usize, k * metres).expect("a channel").0.line;
        let own = pts.windows(2)"""),
 ("""        let (own, next) = (ch.piece(k, texel).expect("a channel"), ch.piece(s.to[k] as usize, texel).expect("a channel"));""",
  """        let (own, next) = (ch.piece(k, texel).expect("a channel").line, ch.piece(s.to[k] as usize, texel).expect("a channel").line);"""),
 ("""/// and the length along the channel runs on: a river's line has no break. Some of the island's channels bend, and no
/// bend leaves its belt (`BEND_MOST`).""", """/// and the length along the channel runs on: a river's line has no break. Some of the island's channels sway, and none
/// farther from its curve than `BEND_MOST`."""),
])
