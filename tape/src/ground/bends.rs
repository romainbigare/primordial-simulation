//! A channel's line as a shell draws it, piece by piece: between the centres of the terrain's samples it follows the
//! curve through them (`channels::base`, the valley's line), and on gentle ground it swings about that curve in a
//! meander belt (`belt`). A channel's bed is shaped by its floods, so its bends are sized by the bed's width
//! (`bed_of`): a tile is made once, and the width is a stand-in, the channel law's (`Physics::channel`) at the drained
//! area times `BED_RUNOFF`. The belt is `BELT_WIDTHS` bed widths to each side where the valley's slope is below
//! `BENDS_BELOW`, none above `STRAIGHT_ABOVE` (pool-riffle reaches meander, step-pool and cascade reaches do not:
//! Schumm 1985, Montgomery and Buffington 1997), no wider than the valley's floor (where the ground across the valley
//! rises from the channel by more than `WALL_DEPTHS` bed depths, the valley's side holds it), than `TURN_SHARE` of the radius
//! the valley's own line turns on, and than `BEND_MOST`. The walk of the bends runs on from piece to piece down each
//! main way (`Bends`), so the line has no break from sample to sample, and along it runs the line's own length from
//! its head, which a shell lays its signs of flow along. Each number is a function of the seed, the map and the place
//! alone.

use super::belt::{Room, Walk, STEPS};
use super::channels::{base, Law, TYPICAL_RUNOFF};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use std::collections::HashMap;

/// The flood that shapes a channel's bed passes this many times the year's mean flow: the bankfull discharge, the
/// flood of every year or two (Leopold and Maddock 1953; Wolman and Miller 1960; Dunne and Leopold 1978: 2 to 10 times
/// in humid lands).
pub const FLOOD: f64 = 5.0;
/// What a m² drained passes on in the flood that shapes a channel's bed, as a depth, m/s: the stand-in for a bed's
/// width where a number that never changes needs one.
pub const BED_RUNOFF: f64 = FLOOD * TYPICAL_RUNOFF;
/// A bend's wavelength, in bed widths: 7 to 15 in rivers of every size, about 11 on their mean (Leopold and Wolman
/// 1960). A tile draws the bends whose wavelength its texels show (`channels::BEND_FROM`).
const BEND_WIDTHS: f64 = 11.0;
/// The half width of the belt the bends swing in, in bed widths, where nothing holds it: belts are 4 to 20 widths
/// across (Williams 1986).
const BELT_WIDTHS: f64 = 8.0;
/// A bed narrower than this, m, is drawn without bends: its bends are smaller than the finest texels show.
const BENDS_FROM_M: f64 = 1.0;
/// Below this slope of the valley a channel bends fully, above `STRAIGHT_ABOVE` not at all: pool-riffle channels lie
/// below about 0.015, step-pool and cascade ones above about 0.03 (Montgomery and Buffington 1997).
const BENDS_BELOW: f64 = 0.01;
const STRAIGHT_ABOVE: f64 = 0.03;
/// The belt is no wider to each side than this share of the radius the valley's line turns on: past it the bends
/// would fold over one another on the inside of the valley's turn.
const TURN_SHARE: f64 = 0.4;
/// A reach's slope and its valley's floor are read over this many samples up and down the channel from its own: the
/// ground from one sample to the next steps, a reach's bends follow the valley over a few of them.
const REACH_SAMPLES: usize = 2;
/// The valley's side holds the belt where the ground stands this many bed depths over the channel: a bank a few times
/// as high as the bed is deep is cut back by the bends, a higher one is the valley's wall.
const WALL_DEPTHS: f64 = 4.0;
/// The most a line bends from the curve through the samples, in samples: it stays inside the window of samples whose
/// curves a tile reads (`channels::SPAN`).
pub(super) const BEND_MOST: f64 = 0.8;
/// The straight pieces a curve's length is summed over, and that a stretch without bends is drawn as at most.
pub(super) const CURVE_STEPS: usize = 8;

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

/// The curve of a sample's piece (`channels::base`, samples) with its length: the length to each of `CURVE_STEPS`
/// even shares of it, m.
pub(super) struct Frame {
    base: [[f64; 2]; 3],
    metres: f64,
    to: [f64; CURVE_STEPS + 1],
}

impl Frame {
    pub(super) fn new(base: [[f64; 2]; 3], metres: f64) -> Frame {
        let mut to = [0.0; CURVE_STEPS + 1];
        for i in 0..CURVE_STEPS {
            let (p, q) = (bezier(base, i as f64 / CURVE_STEPS as f64), bezier(base, (i + 1) as f64 / CURVE_STEPS as f64));
            to[i + 1] = to[i] + (q[0] - p[0]).hypot(q[1] - p[1]) * metres;
        }
        Frame { base, metres, to }
    }

    /// The curve's length, m.
    pub(super) fn len(&self) -> f64 {
        self.to[CURVE_STEPS]
    }

    /// The share of the curve that lies `u` m along it, 0 to 1.
    pub(super) fn share(&self, u: f64) -> f64 {
        let i = self.to.partition_point(|&l| l <= u).clamp(1, CURVE_STEPS);
        let span = (self.to[i] - self.to[i - 1]).max(1e-12);
        ((i - 1) as f64 + ((u - self.to[i - 1]) / span).clamp(0.0, 1.0)) / CURVE_STEPS as f64
    }

    /// The curve's direction at its share `t`, a unit vector (its chord's where the curve starts on its bend point).
    fn along(&self, t: f64) -> [f64; 2] {
        let [a, b, c] = self.base;
        let mut d: [f64; 2] = std::array::from_fn(|k| (1.0 - t) * (b[k] - a[k]) + t * (c[k] - b[k]));
        if d[0].hypot(d[1]) < 1e-9 {
            d = [c[0] - a[0], c[1] - a[1]];
        }
        let len = d[0].hypot(d[1]).max(1e-12);
        [d[0] / len, d[1] / len]
    }

    /// The point `u` m along the curve and `off` m across it (past its ends, along its direction there), samples.
    pub(super) fn at(&self, u: f64, off: f64) -> [f64; 2] {
        let t = self.share(u);
        let (p, d) = (bezier(self.base, t), self.along(t));
        let (past, off) = ((u - u.clamp(0.0, self.len())) / self.metres, off / self.metres);
        [p[0] + d[0] * past - d[1] * off, p[1] + d[1] * past + d[0] * off]
    }

    /// The radius the curve turns on, m: its length over the angle between its two ends' directions.
    fn radius(&self) -> f64 {
        let (d0, d1) = (self.along(0.0), self.along(1.0));
        let turn = (d0[0] * d1[1] - d0[1] * d1[0]).atan2(d0[0] * d1[0] + d0[1] * d1[1]).abs();
        self.len() / turn.max(1e-9)
    }
}

/// A point of a walked piece: how far along its curve and across it, m; how far along the channel's own line from its
/// head, m; the line's curvature there, 1/m (positive where it turns toward the side `v` grows to); and the step of
/// its half bend it ends (`belt::STEPS` at an apex, 0 at the piece's ends and on a straight stretch) with the step's
/// length, m.
#[derive(Clone, Copy)]
pub(super) struct Mark {
    pub u: f64,
    pub v: f64,
    pub s: f64,
    pub curve: f64,
    pub step: u32,
    pub stride: f64,
}

/// Where a walk of bends enters a piece: its place across the curve, along the line, and its curvature at the piece's
/// start; and the walk after the step that crossed into the piece, which ended `u` m along the piece's curve.
#[derive(Clone, Copy)]
pub(super) struct Entry {
    edge: [f64; 3],
    walk: Walk,
    u: f64,
}

/// How a piece starts: on the valley's line with no bend in hand, so far along the channel's line, m; or in a walk.
#[derive(Clone, Copy)]
pub(super) enum Start {
    Straight(f64),
    Bent(Entry),
}

impl Start {
    /// How far along the channel's line the piece starts, m.
    pub(super) fn along(&self) -> f64 {
        match self {
            Start::Straight(s) => *s,
            Start::Bent(e) => e.edge[1],
        }
    }
}

/// The channels of the samples `s` whose lines run through the points `at` (`stems::points`), `metres` a sample,
/// sized by `law`, for the seed of the place's draws.
#[derive(Clone, Copy)]
pub(super) struct Reaches<'a> {
    pub seed: u64,
    pub s: &'a Samples,
    pub at: &'a [[f64; 2]],
    pub law: Law,
    pub metres: f64,
}

impl Reaches<'_> {
    /// The width and the depth of the bed of sample `k`'s channel, m: the channel law's at its drained area times
    /// `BED_RUNOFF`.
    pub(super) fn bed_of(&self, k: usize) -> (f64, f64) {
        let q = self.s.up[k] * BED_RUNOFF;
        (self.law.width * q.powf(self.law.width_exp), self.law.depth * q.powf(self.law.depth_exp))
    }

    /// The wavelength of the bends of sample `k`'s channel, m.
    pub(super) fn wavelength(&self, k: usize) -> f64 {
        BEND_WIDTHS * self.bed_of(k).0
    }

    /// What sample `k`'s reach gives its bends (the module's doc); no belt under standing liquid, at a sink and for a
    /// bed under `BENDS_FROM_M`.
    pub(super) fn room(&self, k: usize, frame: &Frame) -> Room {
        let s = self.s;
        let (width, depth) = self.bed_of(k);
        let to = s.to[k];
        if to == NONE || s.pond[k] != NONE || width < BENDS_FROM_M {
            return Room { width, belt: 0.0 };
        }
        let [slope, across] = self.fall(k);
        let floor = if across > 0.0 { WALL_DEPTHS * depth / across } else { f64::INFINITY };
        let free = BELT_WIDTHS * width * (1.0 - ease(BENDS_BELOW, STRAIGHT_ABOVE, slope));
        Room { width, belt: free.min(floor).min(TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres) }
    }

    /// The fall of the reach about sample `k`, which runs on (`REACH_SAMPLES` up its main way and down its own, out
    /// of the ponds): its slope along the channel, and how fast the ground rises across the valley on its mean, each
    /// rise over run.
    pub(super) fn fall(&self, k: usize) -> [f64; 2] {
        let s = self.s;
        let (mut top, mut bottom, mut run, mut across, mut n) = (k, k, 0.0, self.rise_across(k, s.to[k] as usize), 1.0);
        for _ in 0..REACH_SAMPLES {
            if s.main[top] != NONE {
                (run, top) = (run + self.apart(top, s.main[top] as usize), s.main[top] as usize);
                (across, n) = (across + self.rise_across(top, s.to[top] as usize), n + 1.0);
            }
            let j = s.to[bottom];
            if j != NONE && s.pond[j as usize] == NONE {
                (run, bottom) = (run + self.apart(bottom, j as usize), j as usize);
                if s.to[bottom] != NONE {
                    (across, n) = (across + self.rise_across(bottom, s.to[bottom] as usize), n + 1.0);
                }
            }
        }
        // A reach of its own sample alone (a head that runs into a pond): its fall to the sample it runs to.
        if run == 0.0 {
            (run, bottom) = (self.apart(k, s.to[k] as usize), s.to[k] as usize);
        }
        [(s.h[top] - s.h[bottom]).max(0.0) / run, across / n]
    }

    /// The distance between the centres of the neighbours `a` and `b`, m.
    fn apart(&self, a: usize, b: usize) -> f64 {
        let n = self.s.across;
        self.metres * ((a % n) as f64 - (b % n) as f64).hypot((a / n) as f64 - (b / n) as f64)
    }

    /// How fast the ground rises across the valley at sample `k`, which runs to `to`, rise over run: the mean of its
    /// two neighbours square to its way, as far as they stand over it.
    fn rise_across(&self, k: usize, to: usize) -> f64 {
        let s = self.s;
        let (c, r) = ((k % s.across) as i64, (k / s.across) as i64);
        let (dx, dy) = ((to % s.across) as i64 - c, (to / s.across) as i64 - r);
        let at = |c: i64, r: i64| s.h[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
        ((at(c - dy, r + dx) - s.h[k]).max(0.0) + (at(c + dy, r - dx) - s.h[k]).max(0.0)) / (2.0 * self.apart(k, to))
    }

    /// Walk the piece of sample `k` along `frame` from `start`, giving each point walked to `emit` (from the piece's
    /// start to its end); how the next piece starts. A piece that starts straight on a reach with no belt is not
    /// walked: it follows its curve.
    pub(super) fn walk(&self, k: usize, frame: &Frame, start: Start, emit: &mut dyn FnMut(Mark)) -> Start {
        let (len, room) = (frame.len(), self.room(k, frame));
        let (mut w, mut u) = match start {
            Start::Straight(s) if room.belt == 0.0 => return Start::Straight(s + len),
            Start::Straight(s) => {
                emit(Mark { u: 0.0, v: 0.0, s, curve: 0.0, step: 0, stride: 0.0 });
                (Walk::at(k as u32, s), 0.0)
            }
            Start::Bent(e) => {
                emit(Mark { u: 0.0, v: e.edge[0], s: e.edge[1], curve: e.edge[2], step: 0, stride: 0.0 });
                emit(Mark { u: e.u, v: e.walk.v, s: e.walk.s, curve: e.walk.curvature(), step: e.walk.step, stride: e.walk.stride() });
                (e.walk, e.u)
            }
        };
        loop {
            if w.straight() && room.belt == 0.0 {
                // Back on the valley's line with no room to bend: the rest of the piece follows its curve.
                let rest = len - u;
                for i in 1..=CURVE_STEPS {
                    let d = rest * i as f64 / CURVE_STEPS as f64;
                    emit(Mark { u: u + d, v: 0.0, s: w.s + d, curve: 0.0, step: 0, stride: 0.0 });
                }
                return Start::Straight(w.s + rest);
            }
            if w.step == STEPS {
                w.begin(self.seed, room);
            }
            let before = [u, w.v, w.s, w.curvature()];
            let du = w.step();
            u += du;
            if u >= len {
                // The step that crosses the piece's end is cut there: the next piece starts on the same point.
                let f = (len - before[0]) / du;
                let edge = [before[1] + (w.v - before[1]) * f, before[2] + (w.s - before[2]) * f, before[3] + (w.curvature() - before[3]) * f];
                emit(Mark { u: len, v: edge[0], s: edge[1], curve: edge[2], step: 0, stride: 0.0 });
                return Start::Bent(Entry { edge, walk: w, u: u - len });
            }
            emit(Mark { u, v: w.v, s: w.s, curve: w.curvature(), step: w.step, stride: w.stride() });
        }
    }

    /// The frame of sample `k`'s piece; none for a sink none runs into.
    pub(super) fn frame(&self, k: usize) -> Option<Frame> {
        base(self.s, self.at, k).map(|b| Frame::new(b, self.metres))
    }
}

/// How each sample's piece starts (`Start`): the lengths along the lines of those that start straight, m, and the
/// walks of the others.
pub(super) struct Bends {
    straight: Vec<f64>,
    bent: HashMap<u32, Entry>,
}

impl Bends {
    /// The starts of the pieces of `r`'s channels: a head's line starts straight at 0; down a main way (where a sample
    /// is the main one upstream of the next, `Samples::main`) the next piece starts as the last one ends. In the
    /// flood's order from the last sample reached, so a sample comes before the one its liquid runs to.
    pub(super) fn new(r: &Reaches) -> Bends {
        let s = r.s;
        let mut b = Bends { straight: vec![0.0; s.h.len()], bent: HashMap::new() };
        for &k in s.order.iter().rev() {
            let (k, to) = (k as usize, s.to[k as usize]);
            if to == NONE || s.main[to as usize] != k as u32 {
                continue;
            }
            let Some(frame) = r.frame(k) else { continue };
            match r.walk(k, &frame, b.start(k), &mut |_| {}) {
                Start::Straight(s) => b.straight[to as usize] = s,
                Start::Bent(e) => {
                    b.bent.insert(to, e);
                }
            }
        }
        b
    }

    /// How the piece of sample `k` starts.
    pub(super) fn start(&self, k: usize) -> Start {
        self.bent.get(&(k as u32)).map_or(Start::Straight(self.straight[k]), |e| Start::Bent(*e))
    }
}
