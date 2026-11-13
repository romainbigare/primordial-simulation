def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/wander.rs', [
 ("""//! The wander of a channel's line about its valley's line: the terrain's samples give a valley's line smoothed over
//! several of them (`stems`), and a channel does not keep to the middle of its valley floor: it leans to one side for
//! a stretch, then to the other, in long swings of no fixed length. The wander is a smooth noise along the way down
//! the valley, counted in bed widths (a corner of its lattice every `WANDER_WIDTHS` of them, and a finer one
//! `WANDER_FINE` times as often, `WANDER_FINE_SHARE` as strong), whose slope runs on across the corners (gradient
//! noise, Perlin 2002), so the line it moves has no kink. A reach's bends swing about the line so moved (`belt`); from
//! afar, where a shell draws a channel as a line wider than its bends, the wander is all of the line's sway that
//! shows. A function of the seed, the channel and the place along it alone.""",
  """//! The wander of a channel's line about its valley's line: the terrain's samples place a valley's line no closer
//! than a share of a sample, smoothed over several of them (`stems`), and a channel does not keep to the middle of
//! its valley: it leans to one side for a stretch, then to the other, in long swings of no fixed length. The wander
//! is a smooth noise along the way down the valley, counted in samples (a corner of its lattice every `WANDER_EVERY`
//! of them, and a finer one `WANDER_FINE` times as often, `WANDER_FINE_SHARE` as strong), whose slope runs on across
//! the corners (gradient noise, Perlin 2002), so the line it moves has no kink. A reach's bends swing about the line
//! so moved (`belt`); from afar, where a shell draws a channel as a line wider than its bends, the wander is all of
//! the line's sway that shows. A function of the seed, the channel and the place along it alone."""),
 ("""/// The bed widths down the valley between two corners of the wander's lattice: a swing to one side and back takes
/// about twice that.
pub(super) const WANDER_WIDTHS: f64 = 30.0;""",
  """/// The samples down the valley between two corners of the wander's lattice: a swing to one side and back takes
/// about twice that.
pub(super) const WANDER_EVERY: f64 = 1.6;"""),
 ("""/// The wander `down` bed widths (over `WANDER_WIDTHS`) down the valley of the channel whose line began at `head`: the""",
  """/// The wander `down` samples (over `WANDER_EVERY`) down the valley of the channel whose line began at `head`: the"""),
])

edit('tape/src/ground/bends.rs', [
 ("""//! (`wander`), and swings about its wander in the bends of a meander belt (`belt`). A channel's bed is shaped by its
//! floods, so its sway is sized by the bed's width (`bed_of`): a tile is made once, and the width is a stand-in, the
//! channel law's (`Physics::channel`) at the drained area times `BED_RUNOFF`. The line sways `SWAY_WIDTHS` bed widths to
//! each side where the reach's slope is below `BENDS_BELOW`, and `HARD_SHARE` of that above `STRAIGHT_ABOVE`, where
//! the bed is hard (pool-riffle reaches meander, step-pool and cascade reaches do not: Schumm 1985, Montgomery and
//! Buffington 1997; between the two the change is gradual, and the slope is the reach's, not one sample's); no wider
//! than the valley's floor (where the ground across the valley rises from the channel by more than `WALL_DEPTHS` bed
//! depths, the valley's side holds it), than `TURN_SHARE` of the radius the valley's own line turns on, and than
//! `BEND_MOST`. `WANDER_SHARE` of the sway is the wander's, the rest the belt's. The walk of the bends runs on from
//! piece to piece down each main way (`Bends`), so the line has no break from sample to sample, and along it runs the
//! line's own length from its head, which a shell lays its signs of flow along. Each number is a function of the seed,
//! the map and the place alone.""",
  """//! (`wander`), and swings about its wander in the bends of a meander belt (`belt`). It wanders `WANDER_SAMPLES` to
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
//! function of the seed, the map and the place alone."""),
 ("""use super::channels::{base, Law, TYPICAL_RUNOFF};
use super::frame::{Frame, CURVE_STEPS};
use super::wander::{wander, WANDER_WIDTHS};""",
  """use super::channels::{base, Law, MOST_RUNOFF, TYPICAL_RUNOFF};
use super::frame::{Frame, CURVE_STEPS};
use super::wander::{wander, WANDER_EVERY};"""),
 ("""/// How far a line sways to each side of its valley's line, in bed widths, where nothing holds it (belts are 4 to 20
/// widths across, Williams 1986); the share of that its wander takes (the belt of its bends the rest); and the share
/// of it left on a hard bed.
const SWAY_WIDTHS: f64 = 11.0;
const WANDER_SHARE: f64 = 0.4;
const HARD_SHARE: f64 = 0.14;
/// A bed narrower than this, m, is drawn on its valley's line: its sway is smaller than the finest texels show.""",
  """/// How far a line's bends swing to each side of its wander, in bed widths, where nothing holds them (belts are 4 to
/// 20 widths across, Williams 1986), and the share of that left on a hard bed.
const BELT_WIDTHS: f64 = 3.5;
const HARD_SHARE: f64 = 0.35;
/// How far a line wanders to each side of its valley's line, samples (the samples place the valley's line no closer),
/// and the share of that left on a hard bed, whose walls hold the channel.
const WANDER_SAMPLES: f64 = 0.4;
const WANDER_HARD: f64 = 0.5;
/// A bed narrower than this, m, is drawn on its valley's line: a rill, which a tile holds for a texel or two."""),
 ("""    /// The length of a swing of sample `k`'s wander to one side and back, m.
    pub(super) fn wander_length(&self, k: usize) -> f64 {
        2.0 * WANDER_WIDTHS * self.bed_of(k).0
    }

    /// How far sample `k`'s line sways to each side of its curve `frame` (the module's doc), m, and how hard its bed
    /// is, 0 to 1; no sway under standing liquid, at a sink and for a bed under `BENDS_FROM_M`.
    fn sway(&self, k: usize, frame: &Frame) -> [f64; 2] {
        let s = self.s;
        let (width, depth) = self.bed_of(k);
        if s.to[k] == NONE || s.pond[k] != NONE || width < BENDS_FROM_M {
            return [0.0; 2];
        }
        let [slope, across] = self.fall(k);
        let hard = ease(BENDS_BELOW, STRAIGHT_ABOVE, slope);
        let floor = if across > 0.0 { WALL_DEPTHS * depth / across } else { f64::INFINITY };
        let free = SWAY_WIDTHS * width * (1.0 + (HARD_SHARE - 1.0) * hard);
        [free.min(floor).min(TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres), hard]
    }

    /// What sample `k`'s reach gives its bends: the belt's share of its sway.
    pub(super) fn room(&self, k: usize, frame: &Frame) -> Room {
        let [sway, hard] = self.sway(k, frame);
        Room { width: self.bed_of(k).0, belt: (1.0 - WANDER_SHARE) * sway, hard }
    }""",
  """    /// The length of a swing of a wander to one side and back, m.
    pub(super) fn wander_length(&self) -> f64 {
        2.0 * WANDER_EVERY * self.metres
    }

    /// Half the width of the widest a channel that drains `area` m² gets, m: its reach (`channels`).
    pub(super) fn widest(&self, area: f64) -> f64 {
        self.law.width * (MOST_RUNOFF * area).powf(self.law.width_exp) / 2.0
    }

    /// How far sample `k`'s line wanders to each side of its curve `frame`, m, how far its bends swing to each side
    /// of that, m (the module's doc), and how hard its bed is, 0 to 1; no sway under standing liquid, at a sink and
    /// for a bed under `BENDS_FROM_M`.
    fn sway(&self, k: usize, frame: &Frame) -> [f64; 3] {
        let s = self.s;
        let (width, depth) = self.bed_of(k);
        if s.to[k] == NONE || s.pond[k] != NONE || width < BENDS_FROM_M {
            return [0.0; 3];
        }
        let [slope, across] = self.fall(k);
        let hard = ease(BENDS_BELOW, STRAIGHT_ABOVE, slope);
        let most = (TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres);
        let wander = (WANDER_SAMPLES * self.metres * (1.0 + (WANDER_HARD - 1.0) * hard)).min(most);
        let floor = if across > 0.0 { WALL_DEPTHS * depth / across } else { f64::INFINITY };
        let belt = BELT_WIDTHS * width * (1.0 + (HARD_SHARE - 1.0) * hard);
        [wander, belt.min(floor).min(self.widest(s.up[k]) - width).min(most - wander).max(0.0), hard]
    }

    /// What sample `k`'s reach gives its bends.
    pub(super) fn room(&self, k: usize, frame: &Frame) -> Room {
        let [_, belt, hard] = self.sway(k, frame);
        Room { width: self.bed_of(k).0, belt, hard }
    }"""),
 ("""            b.lean[k] = WANDER_SHARE * r.sway(k, &frame)[0];""", """            b.lean[k] = r.sway(k, &frame)[0];"""),
 ("""(b.head[k], b.down[k] + frame.len() / (WANDER_WIDTHS * r.bed_of(k).0));""", """(b.head[k], b.down[k] + frame.len() / (WANDER_EVERY * r.metres));"""),
 ("""        most * wander(r.seed, self.head[k], self.down[k] + u / (WANDER_WIDTHS * r.bed_of(k).0))""", """        most * wander(r.seed, self.head[k], self.down[k] + u / (WANDER_EVERY * r.metres))"""),
])

edit('tape/src/ground/channels.rs', [
 ("""const MOST_RUNOFF: f64 = 3.6e-6;""", """pub(super) const MOST_RUNOFF: f64 = 3.6e-6;"""),
 ("""        Channels { r, bends, cap: r.law.width * MOST_RUNOFF.powf(r.law.width_exp) / 2.0, most }""", """        Channels { r, bends, cap: r.widest(1.0), most }"""),
 ("""        let sways = [joint(r.wander_length(k), s.main[k], &|j| r.wander_length(j)), joint(r.wander_length(k), s.to[k], &|j| r.wander_length(j))];
        let between = |ends: [f64; 2], u: f64| ends[0] + (ends[1] - ends[0]) * (u / len).clamp(0.0, 1.0);
        let lean = |u: f64| between(sways, u) * self.bends.leans(r, k, &frame, u.clamp(0.0, len));""",
  """        let wanders = ease(BEND_FROM, BEND_TEXELS, r.wander_length() / texel);
        let between = |ends: [f64; 2], u: f64| ends[0] + (ends[1] - ends[0]) * (u / len).clamp(0.0, 1.0);
        let lean = |u: f64| wanders * self.bends.leans(r, k, &frame, u.clamp(0.0, len));"""),
])
