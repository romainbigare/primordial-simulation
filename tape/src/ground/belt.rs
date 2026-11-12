//! The bends of a channel on gentle ground, walked down its way: a meander belt. A channel's line swings from side to
//! side of its valley's line, from one apex to the next (a half bend). Along a half bend its curvature runs smoothly
//! from the first apex's to the next one's, turned the other way (a cubic of its heading: where both are alike, a
//! curvature that falls evenly along the line, as measured bends do: Langbein and Leopold 1966), so the line has no
//! kink and no jump of its curvature. Each apex has its own place across the valley and its own radius, drawn from the
//! place's random numbers within what its reach of the channel gives (`Room`): the belt's middle wanders about the
//! valley's line, a bend reaches a share of the belt's half width from it, which itself swells and narrows along the
//! way, and turns on a radius of a few channel widths (2 to 3 in most grown bends: Hickin and Nanson 1975; Williams
//! 1986). The length of a half bend is what carries the line from its apex to the next one. So bends differ in size
//! and reach, some stretches run nearly straight, and where a wide reach meets a tight radius the line turns back on
//! itself in a loop, never across itself. Each number is a function of the seed, the channel and the place along it
//! alone.

use pcore::rng::{Purpose, Rng};

/// The straight steps a half bend is walked in.
pub(super) const STEPS: u32 = 24;
/// A bend's heading turns from the valley's line by no more than this, rad: past a right angle the line runs back up
/// the valley; held under this, two limbs of a loop come no nearer than about a channel's width (40 walks of 140 half
/// bends at the numbers below: none within 1.2 widths, 17 within 2).
const HEADING_MOST: f64 = 1.85;
/// An apex turns on a radius between these, in channel widths: the tighter ones the more often (the draw squared).
const RADIUS_WIDTHS: [f64; 2] = [1.5, 3.5];
/// An apex reaches between these shares of the belt's half width from its middle where the belt is at its widest, and
/// lies at least `FAR_LEAST` of its reach across from the apex before it.
const REACH_SHARE: [f64; 2] = [0.5, 1.0];
const FAR_LEAST: f64 = 0.3;
/// The belt's width swells and narrows along the way over this many half bends, down to `SWELL_LEAST` of its widest
/// (the draw's complement squared: most reaches bend fully, some run nearly straight).
const SWELL_BENDS: u32 = 5;
const SWELL_LEAST: f64 = 0.1;
/// The belt's middle wanders from the valley's line by up to this share of the belt's half width, over this many half
/// bends.
const WANDER: f64 = 0.5;
const WANDER_BENDS: u32 = 7;
/// A half bend's length lies between these, in channel widths, and it carries the line at least `ADVANCE_WIDTHS` down
/// the valley; a straight stretch is walked in pieces of `STRAIGHT_WIDTHS`.
const ARC_WIDTHS: [f64; 2] = [3.0, 24.0];
const ADVANCE_WIDTHS: f64 = 2.5;
const STRAIGHT_WIDTHS: f64 = 6.0;
/// The halvings that find a half bend's length.
const ARC_HALVINGS: usize = 14;
/// The lattices of the walk's smooth noises along the way (`Walk::noise`): the belt's swell and its middle's wander.
const SWELL_LATTICE: u64 = 1;
const WANDER_LATTICE: u64 = 2;

/// What a reach of a channel gives its bends: the width of its bed, m, and the half width of its belt, m (none where
/// it runs straight).
#[derive(Clone, Copy)]
pub(super) struct Room {
    pub width: f64,
    pub belt: f64,
}

/// The walk down a channel's bends: the sample its bends began at and the half bends since; the steps done of the
/// half bend in hand, its length, m, the curvature at its two apexes, 1/m, the side it swings to (+1 or -1), the
/// apex it ends on (its place across the valley's line, m) and what its sideways steps are scaled by so it ends there
/// exactly; and where the walk stands: across the valley's line and along the channel's own, m.
#[derive(Clone, Copy, Debug)]
pub(super) struct Walk {
    pub stem: u32,
    pub half: u32,
    pub step: u32,
    arc: f64,
    curve: [f64; 2],
    swing: f64,
    apex: f64,
    gain: f64,
    pub v: f64,
    pub s: f64,
}

/// The heading of a half bend `arc` m long with curvatures `curve` at its apexes, at the share `x` of its length, rad
/// from the valley's line toward the side it swings to: 0 at both apexes, its slope there their curvatures.
fn heading(arc: f64, curve: [f64; 2], x: f64) -> f64 {
    arc * x * (1.0 - x) * (curve[0] * (1.0 - x) + curve[1] * x)
}

/// How far a half bend `arc` m long with curvatures `curve` carries the line across the valley and down it, m.
fn carried(arc: f64, curve: [f64; 2]) -> [f64; 2] {
    let ds = arc / STEPS as f64;
    (0..STEPS).map(|i| heading(arc, curve, (i as f64 + 0.5) / STEPS as f64)).fold([0.0; 2], |c, th| [c[0] + ds * th.sin(), c[1] + ds * th.cos()])
}

impl Walk {
    /// The walk at the sample `stem` where a channel's bends begin, `s` m along its line: on the valley's line, straight.
    pub(super) fn at(stem: u32, s: f64) -> Walk {
        Walk { stem, half: 0, step: STEPS, arc: 0.0, curve: [0.0; 2], swing: 1.0, apex: 0.0, gain: 1.0, v: 0.0, s }
    }

    /// Whether the walk stands on the valley's line at the end of a half bend with no curvature: it runs straight
    /// until a reach gives it room.
    pub(super) fn straight(&self) -> bool {
        self.step == STEPS && self.v == 0.0 && self.curve[1] == 0.0
    }

    /// A smooth noise along the way at the half bend in hand, 0 to 1: the draws of the corners of `lattice`, a corner
    /// every `every` half bends, eased between.
    fn noise(&self, seed: u64, lattice: u64, every: u32) -> f64 {
        let draw = |at: u32| Rng::new(seed, self.stem as u64, (lattice << 40) | at as u64, Purpose::Terrain).f64();
        let (i, f) = (self.half / every, (self.half % every) as f64 / every as f64);
        let t = f * f * (3.0 - 2.0 * f);
        draw(i) * (1.0 - t) + draw(i + 1) * t
    }

    /// Begin the next half bend at an apex, on a reach that gives `room`: the next apex's place and radius from the
    /// place's draws, on the other side of the belt's middle; on a reach with no room, back to the valley's line and
    /// straight.
    pub(super) fn begin(&mut self, seed: u64, room: Room) {
        let mut r = Rng::new(seed, self.stem as u64, self.half as u64, Purpose::Terrain);
        let (reach_draw, radius_draw, side_draw) = (r.f64(), r.f64(), r.f64());
        let unswollen = 1.0 - self.noise(seed, SWELL_LATTICE, SWELL_BENDS);
        let swell = SWELL_LEAST + (1.0 - SWELL_LEAST) * (1.0 - unswollen * unswollen);
        let wander = WANDER * swell * (2.0 * self.noise(seed, WANDER_LATTICE, WANDER_BENDS) - 1.0);
        let reach = room.belt * swell * (REACH_SHARE[0] + (REACH_SHARE[1] - REACH_SHARE[0]) * reach_draw) * (1.0 - wander.abs());
        // The first bend goes to either side; after it they alternate.
        let side = if self.half > 0 { -self.swing } else if side_draw < 0.5 { -1.0 } else { 1.0 };
        let radius = room.width * (RADIUS_WIDTHS[0] + (RADIUS_WIDTHS[1] - RADIUS_WIDTHS[0]) * radius_draw * radius_draw);
        // A bend that reaches little turns gently: a reach that runs nearly straight has no tight wiggles.
        let next = if reach > 0.0 { swell.sqrt() / radius } else { 0.0 };
        self.curve = [self.curve[1], next];
        let across = (side * (room.belt * wander + side * reach - self.v)).max(FAR_LEAST * reach);
        self.apex = (self.v + side * across).clamp(-room.belt, room.belt);
        let far = self.apex - self.v;
        let swing = if far < 0.0 { -1.0 } else { 1.0 };
        if self.half > 0 && swing == self.swing {
            // Back toward a belt that narrowed, on the side the line already turns from: it starts without curvature.
            self.curve[0] = 0.0;
        }
        self.swing = swing;
        if far != 0.0 && self.curve == [0.0; 2] {
            // Back to the line with no curvature at either end: the far end takes the gentlest.
            self.curve[1] = 1.0 / (RADIUS_WIDTHS[1] * room.width);
        }
        let sum = self.curve[0] + self.curve[1];
        if sum <= 0.0 {
            (self.arc, self.gain) = (STRAIGHT_WIDTHS * room.width, 0.0);
        } else {
            // The length that carries the line to the apex: the farther the longer, up to where its heading would pass
            // `HEADING_MOST` (the heading's largest is about the length times the mean curvature over 4) or it would
            // carry the line less than `ADVANCE_WIDTHS` down the valley.
            let (mut lo, mut hi) = (ARC_WIDTHS[0] * room.width, (8.0 * HEADING_MOST / sum).min(ARC_WIDTHS[1] * room.width));
            lo = lo.min(hi);
            for _ in 0..ARC_HALVINGS {
                let mid = 0.5 * (lo + hi);
                let [over, down] = carried(mid, self.curve);
                if over < far.abs() && down > ADVANCE_WIDTHS * room.width { lo = mid } else { hi = mid }
            }
            self.arc = 0.5 * (lo + hi);
            self.gain = far.abs() / carried(self.arc, self.curve)[0].max(1e-9);
        }
        (self.half, self.step) = (self.half + 1, 0);
    }

    /// Walk one step of the half bend in hand; how far it carries the line along the valley, m.
    pub(super) fn step(&mut self) -> f64 {
        let ds = self.arc / STEPS as f64;
        let th = heading(self.arc, self.curve, (self.step as f64 + 0.5) / STEPS as f64);
        self.step += 1;
        self.s += ds;
        // At its end the half bend stands on its apex exactly, whatever the steps' rounding left.
        self.v = if self.step == STEPS { self.apex } else { self.v + self.swing * self.gain * ds * th.sin() };
        ds * th.cos()
    }

    /// The length of a step of the half bend in hand, m.
    pub(super) fn stride(&self) -> f64 {
        self.arc / STEPS as f64
    }

    /// The line's curvature where the walk stands, 1/m: positive where it turns toward the side `v` grows to.
    pub(super) fn curvature(&self) -> f64 {
        let x = self.step as f64 / STEPS as f64;
        self.swing * (self.curve[0] * (1.0 - x) * (1.0 - 3.0 * x) + self.curve[1] * x * (2.0 - 3.0 * x))
    }
}
