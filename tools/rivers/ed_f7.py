import re
P = 'tape/src/ground/channels.rs'
t = open(P).read()

def cut(t, start, end):
    """Remove and return the text from `start` up to (not including) `end`."""
    i = t.index(start); j = t.index(end, i)
    return t[:i] + t[j:], t[i:j]

# The drawn pieces move to lines.rs.
t, vertex = cut(t, "/// A point of a channel's line as drawn:", "/// The channels of a map (`bends::Reaches`)")
t, kept = cut(t, "/// The steps of a half bend (`belt::STEPS`) a tile keeps one of", "impl<'a> Channels<'a> {")
t, pieces = cut(t, "    /// Sample `k`'s own piece as drawn on texels of `texel` m", "    /// The field over the window `win`:")
t, field = cut(t, "    /// The field over the window `win`:", "/// The step from the sample that holds the centre of the texel")

lines = '''//! The pieces of the channels' lines as a tile draws them (`channels`): each sample's own piece, its curve moved by
//! its wander and swung in its bends as far as the tile's texels show them, and the same piece without its bends; and
//! where a tributary joins, the piece on to the line it joins.

use super::belt::STEPS;
use super::bends::{ease, Mark, Start};
use super::channels::Channels;
use pcore::space::network::NONE;

/// A stretch of a line is drawn in straight pieces no shorter than this many texels (and a stretch without bends in
/// at most `CALM_STEPS`): a curve departs from shorter chords by far less than a texel.
const STEP_TEXELS: f64 = 0.5;
const CALM_STEPS: usize = 16;
/// A tile draws a channel's sway (its bends, and its wander) fully where its length is at least `BEND_TEXELS` texels,
/// and not at all under `BEND_FROM`: shorter, it moves the line by less than a texel. Between, it grows, the same at
/// both ends of a joint.
const BEND_FROM: f64 = 4.0;
const BEND_TEXELS: f64 = 8.0;

''' + vertex.replace("    at: [f64; 2],\n    along: [f64; 2],\n    bank: f64,\n    last: f64,", "    at: [f64; 2],\n    along: [f64; 2],\n    bank: f64,\n    last: f64,").replace("    fn holds(", "    pub(super) fn holds(") + kept + "impl Channels<'_> {\n" + pieces.rstrip() + "\n}\n"
open('tape/src/ground/lines.rs', 'w').write(lines)

# The consts that moved.
t, _ = cut(t, "/// A stretch of a line is drawn in straight pieces no shorter than this many texels", "/// The most liquid a m² drained passes on")
t = t.replace("use super::belt::STEPS;\nuse super::bends::{ease, Bends, Mark, Reaches, Start, BEND_MOST};", "use super::bends::{Bends, Reaches, BEND_MOST};\nuse super::lines::{Drawn, Join};")
t = t.replace("""pub(super) struct Channels<'a> {
    r: Reaches<'a>,
    bends: &'a Bends,
    cap: f64,
    most: f64,
}""", """pub(super) struct Channels<'a> {
    pub r: Reaches<'a>,
    pub bends: &'a Bends,
    cap: f64,
    most: f64,
}

/// A line near a window of texels: its sample, the line as drawn, where it joins the next sample's, what its
/// distances are told apart by (half its channel's widest width, no less than a texel, samples), how far it reaches,
/// samples, and whether a valley is drawn about it.
struct Near {
    q: usize,
    drawn: Drawn,
    join: Option<Join>,
    scale: f64,
    reach: f64,
    carves: bool,
}""")

new_field = '''    /// The lines that reach the window `win` of texels of `texel` m, and how far a valley reaches, samples (none on
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
        for y in win.texels(a[1].min(b[1]) - l.reach, a[1].max(b[1]) + l.reach, 1) {
            for x in win.texels(a[0].min(b[0]) - l.reach, a[0].max(b[0]) + l.reach, 0) {
                let (at, i) = win.texel(x, y);
                let (side, d2, t) = to_segment(at, a, b);
                if d2 <= l.reach * l.reach && d2.sqrt() / l.scale < best[i] && takes(i) && !l.join.is_some_and(|j| j.holds(at)) {
                    best[i] = d2.sqrt() / l.scale;
                    put(i, side * d2.sqrt() * self.r.metres, t);
                }
            }
        }
    }

    /// The field over the window `win`: each line near enough drawn into the texels within its reach, and the valleys
    /// about the lines on texels finer than the samples; then the lines without their bends, each into the texels
    /// that hold its own channel's line (the line from the same head, `Bends::head`) or none.
    pub(super) fn field(&self, win: Window) -> Field {
        let n = win.wide * win.high;
        let (near, reach_valley) = self.near(&win, win.k * self.r.metres);
        let mut f = Field { dist: vec![NO_CHANNEL_M; n], sample: vec![NONE; n], along: vec![0.0; n], calm: vec![NO_CHANNEL_M; n], valley: vec![0.0; n], weight: vec![0.0; n] };
        let mut carve = Carve::new(n);
        // Of each texel, how near the line it holds lies, over the line's scale.
        let mut best = vec![f64::INFINITY; n];
        for l in &near {
            for w in l.drawn.line.windows(2) {
                self.lay(&win, l, [w[0].p, w[1].p], &mut best, &|_| true, &mut |i, d, t| (f.dist[i], f.sample[i], f.along[i]) = (d, l.q as u32, w[0].s + (w[1].s - w[0].s) * t));
            }
            if l.carves {
                carve.line(&mut f, &win, self.r.s, l.q, &l.drawn.line, reach_valley);
            }
        }
        let (held, head) = (f.sample.clone(), |q: u32| self.bends.head(q as usize));
        best.fill(f64::INFINITY);
        for l in &near {
            for w in l.drawn.calm.windows(2) {
                self.lay(&win, l, [w[0], w[1]], &mut best, &|i| held[i] == NONE || head(held[i]) == head(l.q as u32), &mut |i, d, _| {
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

'''
t = t.replace("impl<'a> Channels<'a> {\n", "impl<'a> Channels<'a> {\n", 1)
i = t.index("/// The step from the sample that holds the centre of the texel")
# The impl block now ends right after `new`: put the new methods before its closing brace.
head_part = t[:i].rstrip()
assert head_part.endswith('}'), head_part[-200:]
head_part = head_part[:-1].rstrip() + "\n\n" + new_field
t = head_part + t[i:]
t = t.replace("use super::valley::{Carve, VALLEY_FROM, VALLEY_M};", "use super::valley::{Carve, VALLEY_FROM, VALLEY_M};")
open(P, 'w').write(t)

def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/bends.rs', [
 ("""    /// How the piece of sample `k` starts.
    pub(super) fn start(&self, k: usize) -> Start {""", """    /// The sample the line of sample `k` began at: the same down a main way, so it names the channel.
    pub(super) fn head(&self, k: usize) -> u32 {
        self.head[k]
    }

    /// How the piece of sample `k` starts.
    pub(super) fn start(&self, k: usize) -> Start {"""),
 ("""/// 1960). A tile draws the bends whose wavelength its texels show (`channels::BEND_FROM`).""", """/// 1960). A tile draws the bends whose wavelength its texels show (`lines::BEND_FROM`)."""),
])
edit('tape/src/ground/valley.rs', [("use super::channels::{to_segment, Field, Vertex, Window};", "use super::channels::{to_segment, Field, Window};\nuse super::lines::Vertex;")])
edit('tape/src/ground/mod.rs', [("mod frame;", "mod frame;\nmod lines;")])
