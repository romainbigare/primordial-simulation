def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/mend.rs', [
 ("""//! The strips of a field of channels mended (`channels::Channels::field`): a small channel that crosses the reach of a
//! larger one lies nearer over its scale in a strip a texel wide, which would notch the line a shell draws the larger
//! one as from afar. A texel that holds a channel between two texels (left and right, or above and below) that both
//! hold one other channel, which drains more, is given to that channel.""",
  """//! The strips of a field of channels mended (`channels::Channels::field`): a small channel that crosses the reach of a
//! larger one lies nearer over its scale in a strip a texel or two wide, which would notch the line a shell draws the
//! larger one as from afar. A texel that holds a channel and lies within `STRIP` texels, to its left and to its right
//! or above and below, of texels that hold one other channel, which drains more (with none but its own channel's
//! between), is given to that channel."""),
 ("""use std::collections::HashMap;
""", """use std::collections::HashMap;

/// The widest strip mended, texels.
pub(super) const STRIP: usize = 2;
"""),
 ("""            for (a, b) in [(i - 1, i + 1), (i - wide, i + wide)] {
                let (qa, qb) = (sample[a], sample[b]);
                if qa == NONE || qb == NONE || head(qa) != head(qb) || head(qa) == head(sample[i]) || s.up[qa as usize].min(s.up[qb as usize]) <= s.up[sample[i] as usize] {
                    continue;
                }""",
  """            // The first texel of another channel within `STRIP` of this one along `step` (none past the window's row,
            // or past a texel that holds no channel).
            let beside = |step: isize| {
                (1..=STRIP as isize).map(|n| i as isize + n * step).take_while(|&j| j >= 0 && (j as usize) < sample.len() && (step.unsigned_abs() > 1 || j as usize / wide == i / wide) && sample[j as usize] != NONE).map(|j| j as usize).find(|&j| head(sample[j]) != head(sample[i]))
            };
            for step in [1, wide as isize] {
                let (Some(a), Some(b)) = (beside(-step), beside(step)) else { continue };
                let (qa, qb) = (sample[a], sample[b]);
                if head(qa) != head(qb) || s.up[qa as usize].min(s.up[qb as usize]) <= s.up[sample[i] as usize] {
                    continue;
                }"""),
 ("""        for i in wide..sample.len() - wide {
            if i % wide == 0 || i % wide == wide - 1 || sample[i] == NONE {
                continue;
            }""", """        for i in (0..sample.len()).filter(|&i| sample[i] != NONE) {"""),
])

edit('tape/src/ground/channels.rs', [
 ("""    /// The field over the window `win`: the lines laid into its texels (`laid`) and its strips mended (`mend`), over
    /// a window a texel larger all round, so each texel has its neighbours.
    pub(super) fn field(&self, win: Window) -> Field {
        let wide = win.wide + 2;
        let all = Window { x0: win.x0 - 1, y0: win.y0 - 1, wide, high: win.high + 2, k: win.k };""",
  """    /// The field over the window `win`: the lines laid into its texels (`laid`) and its strips mended (`mend`), over
    /// a window `mend::STRIP` texels larger all round, so each texel has the neighbours its mending reads.
    pub(super) fn field(&self, win: Window) -> Field {
        let (m, wide) = (STRIP, win.wide + 2 * STRIP);
        let all = Window { x0: win.x0 - m as i64, y0: win.y0 - m as i64, wide, high: win.high + 2 * m, k: win.k };"""),
 ("""        let cut = |v: &[f64]| (0..win.high).flat_map(|y| v[(y + 1) * wide + 1..(y + 2) * wide - 1].iter().copied()).collect::<Vec<f64>>();
        let sample = (0..win.high).flat_map(|y| f.sample[(y + 1) * wide + 1..(y + 2) * wide - 1].iter().copied()).collect();""",
  """        let cut = |v: &[f64]| (0..win.high).flat_map(|y| v[(y + m) * wide + m..(y + m + 1) * wide - m].iter().copied()).collect::<Vec<f64>>();
        let sample = (0..win.high).flat_map(|y| f.sample[(y + m) * wide + m..(y + m + 1) * wide - m].iter().copied()).collect();"""),
 ("""use super::lines::{Drawn, Join};""", """use super::lines::{Drawn, Join};
use super::mend::STRIP;"""),
])

edit('tape/src/ground/wander.rs', [
 ("""//! is a smooth noise along the way down the valley, counted in samples (a corner of its lattice every `WANDER_EVERY`
//! of them, and a finer one `WANDER_FINE` times as often, `WANDER_FINE_SHARE` as strong), whose slope runs on across
//! the corners (gradient noise, Perlin 2002), so the line it moves has no kink.""",
  """//! is a smooth noise along the way down the valley, counted in samples (a corner of its lattice every `WANDER_EVERY`
//! of them, and `WANDER_OCTAVES` in all, each `WANDER_FINE` times as often as the last and `WANDER_FINE_SHARE` as
//! strong: the finest turns the line every few metres, so no channel, however small, runs straight), whose slope
//! runs on across the corners (gradient noise, Perlin 2002), so the line it moves has no kink."""),
 ("""/// The finer wander: its corners this many times as often, and it this share as strong.
const WANDER_FINE: f64 = 2.7;
const WANDER_FINE_SHARE: f64 = 0.4;""", """/// The wander's octaves: each one's corners `WANDER_FINE` times as often as the last's, and it `WANDER_FINE_SHARE`
/// as strong.
const WANDER_OCTAVES: u64 = 5;
const WANDER_FINE: f64 = 2.7;
const WANDER_FINE_SHARE: f64 = 0.4;"""),
 ("""    (noise(seed, head, 0, down) + WANDER_FINE_SHARE * noise(seed, head, 1, WANDER_FINE * down)) / (1.0 + WANDER_FINE_SHARE)""",
  """    let (mut sum, mut most, mut often, mut strong) = (0.0, 0.0, 1.0, 1.0);
    for octave in 0..WANDER_OCTAVES {
        (sum, most) = (sum + strong * noise(seed, head, octave, often * down), most + strong);
        (often, strong) = (often * WANDER_FINE, strong * WANDER_FINE_SHARE);
    }
    sum / most"""),
])

edit('tape/src/ground/bends.rs', [
 ("""        if s.to[k] == NONE || s.pond[k] != NONE || width < BENDS_FROM_M {
            return [0.0; 3];
        }
        let [slope, across] = self.fall(k);
        let hard = ease(BENDS_BELOW, STRAIGHT_ABOVE, slope);
        let most = (TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres);
        let wander = (WANDER_SAMPLES * self.metres * (1.0 + (WANDER_HARD - 1.0) * hard)).min(most);""",
  """        if s.to[k] == NONE {
            return [0.0; 3];
        }
        let [slope, across] = self.fall(k);
        let hard = ease(BENDS_BELOW, STRAIGHT_ABOVE, slope);
        let most = (TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres);
        let wander = (WANDER_SAMPLES * self.metres * (1.0 + (WANDER_HARD - 1.0) * hard)).min(most);
        if s.pond[k] != NONE || width < BENDS_FROM_M {
            return [wander, 0.0, hard];
        }"""),
 ("""    /// of that, m (the module's doc), and how hard its bed is, 0 to 1; no sway under standing liquid, at a sink and
    /// for a bed under `BENDS_FROM_M`.""", """    /// of that, m (the module's doc), and how hard its bed is, 0 to 1; no sway at a sink, and no bends (the wander
    /// alone) under standing liquid and for a bed under `BENDS_FROM_M`."""),
 ("""/// A bed narrower than this, m, is drawn on its valley's line: a rill, which a tile holds for a texel or two.""",
  """/// A bed narrower than this, m, has no bends walked (a rill, whose bends are finer than the wander's finest turns):
/// it wanders like any channel."""),
])

edit('tape/src/ground/stems.rs', [
 ("""        while down.len() < SMOOTH_SAMPLES && s.to[q] != NONE && s.pond[s.to[q] as usize] == NONE {""", """        while down.len() < SMOOTH_SAMPLES && s.to[q] != NONE {"""),
])
edit('tape/src/ground/lines.rs', [("const CALM_STEPS: usize = 16;", "const CALM_STEPS: usize = 48;")])

edit('web/shaders/lib/flow.wgsl', [
 ("""    let fall = exp(-fract(p) / STEP_FADE) * smoothstep(0.0, 2.0 * e / STEP_M, fract(p));""",
  """    let fall = exp(-fract(p) / STEP_FADE) * smoothstep(0.0, max(2.0 * e, STEP_EDGE_M) / STEP_M, fract(p));"""),
 ("""const STEP_POOL = 0.6; const POOL_CALM = 0.8;""", """const STEP_POOL = 0.6; const POOL_CALM = 0.8;
// The white water begins over this length below a step's line, m: the place along a channel is known to a few
// centimetres (the tiles keep it as a 16-bit float), and an edge sharper than that would draw the texels' lattice.
const STEP_EDGE_M = 0.4;"""),
])
