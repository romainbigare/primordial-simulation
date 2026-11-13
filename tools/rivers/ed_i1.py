def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/shore.rs', [
 ("""const KEEP_M: f64 = 3.0;
const KEPT: f64 = 3.0;""", """const KEEP_M: f64 = 3.0;
const KEPT: f64 = 3.0;
/// A channel's line that ends in standing liquid ends where the shore drawn puts that liquid: the samples' shore and
/// the smooth one lie up to a sample or two apart, so from the point where the samples end the line it runs on down
/// the offset's slope, in steps of `MOUTH_STEP` samples, to where the offset lies `MOUTH_UNDER_M` under the level, no
/// farther than `MOUTH_MOST` samples.
const MOUTH_UNDER_M: f64 = 1.0;
const MOUTH_MOST: f64 = 2.5;
const MOUTH_STEP: f64 = 0.1;"""),
 ("""    /// The height drawn at `p` (samples from the map's corner) whose own height is `h`, m, beside liquid at `level` (the""",
  """    /// Where a channel's line that the samples end at `from` (samples from the map's corner) ends as drawn (`MOUTH_MOST`):
    /// in the liquid of the shore drawn; `from` itself where it lies in it, or where none lies near.
    pub(super) fn mouth(&self, from: [f64; 2]) -> [f64; 2] {
        let mut p = from;
        for _ in 0..(MOUTH_MOST / MOUTH_STEP) as usize {
            if self.at(p).0 <= -MOUTH_UNDER_M {
                return p;
            }
            let slope = [self.at([p[0] + MOUTH_STEP, p[1]]).0 - self.at([p[0] - MOUTH_STEP, p[1]]).0, self.at([p[0], p[1] + MOUTH_STEP]).0 - self.at([p[0], p[1] - MOUTH_STEP]).0];
            let steep = slope[0].hypot(slope[1]);
            if steep < 1e-9 {
                break;
            }
            p = [p[0] - MOUTH_STEP * slope[0] / steep, p[1] - MOUTH_STEP * slope[1] / steep];
        }
        from
    }

    /// The height drawn at `p` (samples from the map's corner) whose own height is `h`, m, beside liquid at `level` (the"""),
])

edit('tape/src/ground/stems.rs', [
 ("""//! tributary joins keep their place, and a tributary ends on the smoothed point of the stem it joins. A point moves at
//! most `SMOOTH_MOST` from its sample's centre. A function of the samples alone.

use pcore::space::network::NONE;""", """//! tributary joins keep their place, and a tributary ends on the smoothed point of the stem it joins. A point moves at
//! most `SMOOTH_MOST` from its sample's centre. The point of a sample that liquid runs into and none out of (a line's
//! end in standing liquid) lies where the shore drawn puts that liquid (`shore::Shore::mouth`), so a river reaches
//! its lake or its sea. A function of the samples alone.

use super::shore::Shore;
use pcore::space::network::NONE;"""),
 ("""/// The point of each sample of `s` its line runs through, samples (the module's doc).""", """/// The point of each sample of `s`, whose shore is drawn as `shore`, its line runs through, samples (the module's doc)."""),
 ("""pub(super) fn points(s: &Samples) -> Vec<[f64; 2]> {""", """pub(super) fn points(s: &Samples, shore: &Shore) -> Vec<[f64; 2]> {"""),
 ("""        at[k] = [own[0] + dx * keep, own[1] + dy * keep];
    }
    at""", """        at[k] = [own[0] + dx * keep, own[1] + dy * keep];
    }
    for k in (0..s.h.len()).filter(|&k| s.to[k] == NONE && s.main[k] != NONE) {
        at[k] = shore.mouth(at[k]);
    }
    at"""),
])

edit('tape/src/ground/mod.rs', [
 ("""        let at = stems::points(&samples);
        let bends = bends::Bends::new(&bends::Reaches { seed, s: &samples, at: &at, law, metres: map.length(SAMPLE_DEPTH) });
        let shore = shore::Shore::new(&samples);""", """        let shore = shore::Shore::new(&samples);
        let at = stems::points(&samples, &shore);
        let bends = bends::Bends::new(&bends::Reaches { seed, s: &samples, at: &at, law, metres: map.length(SAMPLE_DEPTH) });"""),
])

edit('web/shaders/land.wgsl', [
 ("""// A channel wets the ground beside it (its fringe)""", """// A channel widens into the standing liquid it runs into: its bed and its water `MOUTH_FLARE` wider (and its water
// that much shallower) where the ground lies at that liquid's level, none of it from `MOUTH_OVER_M` over the level.
// Where it moves stones of `BAR_SAND_STONE` or more it lays a fan under the liquid: sand within `FAN_WIDTHS` of its
// bed's half widths of its line, under liquid shallower than `FAN_DEPTH_M`.
const MOUTH_FLARE = 1.5; const MOUTH_OVER_M = 1.5; const FAN_WIDTHS = 5.0; const FAN_DEPTH_M = 1.5;
// A channel wets the ground beside it (its fringe)"""),
 ("""  let ch = channel_at(g, q);
  let st = state_on(at, g, cs, depth_in(g, ch));""", """  var ch = channel_at(g, q);
  let st = state_on(at, g, cs, depth_in(g, ch));
  let flare = 1.0 + MOUTH_FLARE * (1.0 - ease(0.0, MOUTH_OVER_M, st.over));
  ch.bed *= flare;
  ch.water *= flare;
  ch.mean /= flare;"""),
 ("""  p.sand = beach(p.sea, g.h - earth.view.w, p.over, length(g.slope), p.loose, look.z);""", """  p.sand = beach(p.sea, g.h - earth.view.w, p.over, length(g.slope), p.loose, look.z);
  // A river's fan under the liquid it runs into.
  if (ch.bed > 0.0 && p.depth > 0.0) { p.sand = max(p.sand, ease(0.0, BAR_SAND_STONE, moved) * (1.0 - ease(ch.bed, FAN_WIDTHS * ch.bed, g.d)) * (1.0 - ease(0.0, FAN_DEPTH_M, p.depth))); }"""),
])
