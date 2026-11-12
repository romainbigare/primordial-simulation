//! A channel's line as a shell draws it, piece by piece: between the centres of the terrain's samples it follows the
//! curve through them (`channels::base`, the valley's line, `frame`), wanders about that curve in long swings
//! (`wander`), and swings about its wander in the bends of a meander belt (`belt`). It wanders `WANDER_SAMPLES` to
//! each side. A channel's bed is shaped by its floods, so its bends are sized by the bed's width (`bed_of`): a tile is
//! made once, and the width is a stand-in, the channel law's (`Physics::channel`) at the drained area times
//! `BED_RUNOFF`. The bends swing `BELT_WIDTHS` bed widths to each side of the wander where the reach's slope is below
//! `BENDS_BELOW`; above `STRAIGHT_ABOVE` the bed is hard, and `HARD_SHARE` of the bends and `WANDER_HARD` of the wander
//! are left (pool-riffle reaches meander, step-pool and cascade reaches do not: Schumm 1985, Montgomery and Buffington
//! 1997; between the two the change is gradual, and the slope is the reach's, not one sample's). The bends swing no
//! wider than the valley's floor (where the ground across the valley rises from the channel by more than `WALL_DEPTHS`
//! bed depths, the valley's side holds them) and than the channel's reach less its bed (a tile holds a line within
//! its reach, `channels`, and a shell that draws the line without its bends reads both lines between them); the whole
//! sway no wider than `TURN_SHARE` of the radius the valley's own line turns on and than `BEND_MOST`. The walk of the
//! bends runs on from piece to piece down each main way (`Bends`), so the line has no break from sample to sample, and
//! along it runs the line's own length from its head, which a shell lays its signs of flow along. Each number is a
//! function of the seed, the map and the place alone.

use super::belt::{Room, Walk, STEPS};
use super::channels::{base, Law, MOST_RUNOFF, TYPICAL_RUNOFF};
use super::frame::{Frame, CURVE_STEPS};
use super::wander::{wander, WANDER_EVERY};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use std::collections::HashMap;

/// The flood that shapes a channel's bed passes this many times the year's mean flow: the bankfull discharge, the
/// flood of every year or two (Leopold and Maddock 1953; Wolman and Miller 1960; Dunne and Leopold 1978: 2 to 10 times
/// in humid lands).
pub const FLOOD: f64 = 5.0;
/// At one place a channel's water grows wider with its discharge as its power this (at-a-station hydraulic geometry,
/// Leopold and Maddock 1953), far less than from channel to channel: at its mean flow the water takes `FLOOD` to the
/// power minus this of the width of the bed its floods cut.
pub const STATION_EXP: f64 = 0.26;
/// What a m² drained passes on in the flood that shapes a channel's bed, as a depth, m/s: the stand-in for a bed's
/// width where a number that never changes needs one.
pub const BED_RUNOFF: f64 = FLOOD * TYPICAL_RUNOFF;
/// A bend's wavelength, in bed widths: 7 to 15 in rivers of every size, about 11 on their mean (Leopold and Wolman
/// 1960). A tile draws the bends whose wavelength its texels show (`lines::BEND_FROM`).
const BEND_WIDTHS: f64 = 11.0;
/// How far a line's bends swing to each side of its wander, in bed widths, where nothing holds them (belts are 4 to
/// 20 widths across, Williams 1986), and the share of that left on a hard bed.
const BELT_WIDTHS: f64 = 3.5;
const HARD_SHARE: f64 = 0.35;
/// How far a line wanders to each side of its valley's line, samples (the samples place the valley's line no closer),
/// and the share of that left on a hard bed, whose walls hold the channel.
const WANDER_SAMPLES: f64 = 0.4;
const WANDER_HARD: f64 = 0.5;
/// A bed narrower than this, m, has no bends walked (a rill, whose bends are finer than the wander's finest turns):
/// it wanders like any channel.
const BENDS_FROM_M: f64 = 1.0;
/// Below this slope of the reach a channel sways fully, above `STRAIGHT_ABOVE` its bed is hard: pool-riffle channels
/// lie below about 0.015, step-pool and cascade ones above about 0.03 (Montgomery and Buffington 1997).
const BENDS_BELOW: f64 = 0.01;
const STRAIGHT_ABOVE: f64 = 0.03;
/// The sway is no wider to each side than this share of the radius the valley's line turns on: past it the bends
/// would fold over one another on the inside of the valley's turn.
const TURN_SHARE: f64 = 0.4;
/// A reach's slope and its valley's floor are read over this many samples up and down the channel from its own: the
/// ground from one sample to the next steps, a reach's bends follow the valley over a few of them.
const REACH_SAMPLES: usize = 2;
/// The valley's side holds the sway where the ground stands this many bed depths over the channel: a bank a few times
/// as high as the bed is deep is cut back by the bends, a higher one is the valley's wall.
const WALL_DEPTHS: f64 = 4.0;
/// The most a line sways from the curve through the samples, in samples: it stays inside the window of samples whose
/// curves a tile reads (`channels::SPAN`).
pub(super) const BEND_MOST: f64 = 0.8;

/// The smooth step from 0 at `a` to 1 at `b`.
pub(super) fn ease(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A point of a walked piece: how far along its curve, m, and across it from its wander, m; how far along the
/// channel's own line from its head, m; and the step of its half bend it ends (`belt::STEPS` at an apex, 0 at the
/// piece's ends and on a straight stretch) with the step's length, m.
#[derive(Clone, Copy)]
pub(super) struct Mark {
    pub u: f64,
    pub v: f64,
    pub s: f64,
    pub step: u32,
    pub stride: f64,
}

/// Where a walk of bends enters a piece: its place across the curve and along the line at the piece's start; and the
/// walk after the step that crossed into the piece, which ended `u` m along the piece's curve.
#[derive(Clone, Copy)]
pub(super) struct Entry {
    edge: [f64; 2],
    walk: Walk,
    u: f64,
}

/// How a piece starts: with no bend in hand, so far along the channel's line, m; or in a walk.
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

    /// The length of a swing of a wander to one side and back, m.
    pub(super) fn wander_length(&self) -> f64 {
        2.0 * WANDER_EVERY * self.metres
    }

    /// Half the width of the widest a channel that drains `area` m² gets, m: its reach (`channels`).
    pub(super) fn widest(&self, area: f64) -> f64 {
        self.law.width * (MOST_RUNOFF * area).powf(self.law.width_exp) / 2.0
    }

    /// How far sample `k`'s line wanders to each side of its curve `frame`, m, how far its bends swing to each side
    /// of that, m (the module's doc), and how hard its bed is, 0 to 1; no sway at a sink, and no bends (the wander
    /// alone) under standing liquid and for a bed under `BENDS_FROM_M`.
    fn sway(&self, k: usize, frame: &Frame) -> [f64; 3] {
        let s = self.s;
        let (width, depth) = self.bed_of(k);
        if s.to[k] == NONE {
            return [0.0; 3];
        }
        let [slope, across] = self.fall(k);
        let hard = ease(BENDS_BELOW, STRAIGHT_ABOVE, slope);
        let most = (TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres);
        let wander = (WANDER_SAMPLES * self.metres * (1.0 + (WANDER_HARD - 1.0) * hard)).min(most);
        if s.pond[k] != NONE || width < BENDS_FROM_M {
            return [wander, 0.0, hard];
        }
        let floor = if across > 0.0 { WALL_DEPTHS * depth / across } else { f64::INFINITY };
        let belt = BELT_WIDTHS * width * (1.0 + (HARD_SHARE - 1.0) * hard);
        [wander, belt.min(floor).min(self.widest(s.up[k]) - width).min(most - wander).max(0.0), hard]
    }

    /// What sample `k`'s reach gives its bends.
    pub(super) fn room(&self, k: usize, frame: &Frame) -> Room {
        let [_, belt, hard] = self.sway(k, frame);
        Room { width: self.bed_of(k).0, belt, hard }
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
    /// walked.
    pub(super) fn walk(&self, k: usize, frame: &Frame, start: Start, emit: &mut dyn FnMut(Mark)) -> Start {
        let (len, room) = (frame.len(), self.room(k, frame));
        let (mut w, mut u) = match start {
            Start::Straight(s) if room.belt == 0.0 => return Start::Straight(s + len),
            Start::Straight(s) => {
                emit(Mark { u: 0.0, v: 0.0, s, step: 0, stride: 0.0 });
                (Walk::at(k as u32, s), 0.0)
            }
            Start::Bent(e) => {
                emit(Mark { u: 0.0, v: e.edge[0], s: e.edge[1], step: 0, stride: 0.0 });
                emit(Mark { u: e.u, v: e.walk.v, s: e.walk.s, step: e.walk.step, stride: e.walk.stride() });
                (e.walk, e.u)
            }
        };
        loop {
            if w.straight() && room.belt == 0.0 {
                // Back on its wander with no room to bend: the rest of the piece runs straight along it.
                let rest = len - u;
                for i in 1..=CURVE_STEPS {
                    let d = rest * i as f64 / CURVE_STEPS as f64;
                    emit(Mark { u: u + d, v: 0.0, s: w.s + d, step: 0, stride: 0.0 });
                }
                return Start::Straight(w.s + rest);
            }
            if w.step == STEPS {
                w.begin(self.seed, room);
            }
            let before = [u, w.v, w.s];
            let du = w.step();
            u += du;
            if u >= len {
                // The step that crosses the piece's end is cut there: the next piece starts on the same point.
                let f = (len - before[0]) / du;
                let edge = [before[1] + (w.v - before[1]) * f, before[2] + (w.s - before[2]) * f];
                emit(Mark { u: len, v: edge[0], s: edge[1], step: 0, stride: 0.0 });
                return Start::Bent(Entry { edge, walk: w, u: u - len });
            }
            emit(Mark { u, v: w.v, s: w.s, step: w.step, stride: w.stride() });
        }
    }

    /// The frame of sample `k`'s piece; none for a sink none runs into.
    pub(super) fn frame(&self, k: usize) -> Option<Frame> {
        base(self.s, self.at, k).map(|b| Frame::new(b, self.metres))
    }
}

/// How each sample's piece starts (`Start`): the lengths along the lines of those that start straight, m, and the
/// walks of the others; and of each sample's wander: the most it leans from its curve, m, the sample its line began
/// at, and how far down the valley its piece starts, in the wander's lattice.
pub(super) struct Bends {
    straight: Vec<f64>,
    bent: HashMap<u32, Entry>,
    lean: Vec<f64>,
    head: Vec<u32>,
    down: Vec<f64>,
}

impl Bends {
    /// The starts of the pieces of `r`'s channels: a head's line starts straight at 0; down a main way (where a sample
    /// is the main one upstream of the next, `Samples::main`) the next piece starts as the last one ends. In the
    /// flood's order from the last sample reached, so a sample comes before the one its liquid runs to.
    pub(super) fn new(r: &Reaches) -> Bends {
        let s = r.s;
        let n = s.h.len();
        let mut b = Bends { straight: vec![0.0; n], bent: HashMap::new(), lean: vec![0.0; n], head: (0..n as u32).collect(), down: vec![0.0; n] };
        for &k in s.order.iter().rev() {
            let (k, to) = (k as usize, s.to[k as usize]);
            let Some(frame) = r.frame(k) else { continue };
            b.lean[k] = r.sway(k, &frame)[0];
            if to == NONE || s.main[to as usize] != k as u32 {
                continue;
            }
            (b.head[to as usize], b.down[to as usize]) = (b.head[k], b.down[k] + frame.len() / (WANDER_EVERY * r.metres));
            match r.walk(k, &frame, b.start(k), &mut |_| {}) {
                Start::Straight(s) => b.straight[to as usize] = s,
                Start::Bent(e) => {
                    b.bent.insert(to, e);
                }
            }
        }
        b
    }

    /// The sample the line of sample `k` began at: the same down a main way, so it names the channel.
    pub(super) fn head(&self, k: usize) -> u32 {
        self.head[k]
    }

    /// How the piece of sample `k` starts.
    pub(super) fn start(&self, k: usize) -> Start {
        self.bent.get(&(k as u32)).map_or(Start::Straight(self.straight[k]), |e| Start::Bent(*e))
    }

    /// How far the line of sample `k` of `r` leans from its curve `frame` by its wander, `u` m along the curve, m: the
    /// wander there (`wander::wander`) times the most it leans, which runs smoothly from the mean of its own and its
    /// main neighbour's upstream at the piece's start to the mean of its own and the next one's at its end, and is
    /// none where the line starts or ends (a head; a join, a sink). The wander's finer turns are those of a bed as
    /// wide as the channel's there (`wander::wander`), in samples, which runs between the same means.
    pub(super) fn leans(&self, r: &Reaches, k: usize, frame: &Frame, u: f64) -> f64 {
        let s = r.s;
        let (up, on) = (s.main[k] != NONE, s.to[k] != NONE && s.main[s.to[k] as usize] == k as u32);
        let t = ease(0.0, 1.0, u / frame.len());
        let joint = |j: u32, runs: bool| if runs { 0.5 * (self.lean[k] + self.lean[j as usize]) } else { 0.0 };
        let most = joint(s.main[k], up) + (joint(s.to[k], on) - joint(s.main[k], up)) * t;
        let wide = |j: u32, runs: bool| if runs { 0.5 * (r.bed_of(k).0 + r.bed_of(j as usize).0) } else { r.bed_of(k).0 };
        let width = wide(s.main[k], up) + (wide(s.to[k], on) - wide(s.main[k], up)) * t;
        most * wander(r.seed, self.head[k], self.down[k] + u / (WANDER_EVERY * r.metres), width / (WANDER_EVERY * r.metres))
    }
}
