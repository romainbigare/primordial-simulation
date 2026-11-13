def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

def cut(p, start, end, new=''):
    t = open(p).read()
    i = t.index(start); j = t.index(end, i)
    open(p, 'w').write(t[:i] + new + t[j:])

R = 'web/shaders/lib/river.wgsl'
cut(R, "// The share of a pixel `e` m across that a channel's line from afar covers on ground `g`:", "// The reach at `at`, m, seen at `e` m a pixel,")
edit(R, [
 ("""// The reach at `at`, m, seen at `e` m a pixel, on ground `g` of the channel `c` (rock.wgsl `channel_at`), `open` of
// its bed drawn apart from its water (land.wgsl), whose flow moves stones of `stone` m.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, open: f32, stone: f32) -> Reach {""",
  """// The reach at `at`, m, seen at `e` m a pixel, on ground `g` of the channel `c` (rock.wgsl `channel_at`), drawn no
// thinner than `line` m half wide (`line_of`), `open` of its bed drawn apart from its water, whose flow moves stones
// of `stone` m.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, line: f32, open: f32, stone: f32) -> Reach {"""),
 ("""  let reach = banks;
""", """  let reach = max(banks, line);
"""),
 ("""(tape/src/ground/channels.rs `LINE_PX`); it fades in from `LINE_FADE` times less, so no channel springs up as the view
// zooms. The line runs along the channel's line without its bends (lib/river.wgsl `line_over`): wider than about a
// sixth of a bend's length it would swallow its bends. As the view nears and the line narrows toward its bed, the
// bed itself with its bends fades in under it and the line fades out, from `LINE_BENDS.y` bed widths to
// `LINE_BENDS.x`: two pictures blended over that range of zoom, neither moving. A line `LINE_LOOK_PX` pixels wide or
// more has the water's look across it: its middle the deep thread's tone, `LINE_SHALLOW` shallower toward its
// edges, and a light wash from `LINE_EDGE` of the way to the edge.
const LINE_FADE = 4.0;
const LINE_BENDS = vec2f(1.0, 2.5);
const LINE_LOOK_PX = vec2f(1.5, 4.0); const LINE_SHALLOW = 0.6; const LINE_EDGE = 0.65;""",
  """(tape/src/ground/channels.rs `LINE_PX`); it fades in from `LINE_FADE` times less, so no channel springs up as the view
// zooms. The line is the least width of the channel's own bed on screen, no more: a channel is drawn as wide as it
// is wherever that is wider (width in pixels = the larger of the two), so its width changes evenly with the zoom and
// with nothing else, and a bed narrower than its line is plain water, the same water as a bed's.
const LINE_FADE = 4.0;"""),
 ("""// The line a channel of discharge `q`, whose bed the law makes `banks` m half wide, is drawn as from afar where a
// pixel takes `e` m: its half width, m, and how much of it shows over the bed itself, 0..1.
fn line_of(q: f32, e: f32, banks: f32) -> vec2f {
  if (q <= 0.0) { return vec2f(0.0); }
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * land.line.x * css * min(pow(q / shows, land.line.y), land.line.z) * ease(shows / LINE_FADE, shows, q);
  return vec2f(line, ease(LINE_BENDS.x * banks, LINE_BENDS.y * banks, line));
}""", """// The least half width a channel of discharge `q` is drawn at where a pixel takes `e` m, m.
fn line_of(q: f32, e: f32) -> f32 {
  if (q <= 0.0) { return 0.0; }
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  return 0.5 * land.line.x * css * min(pow(q / shows, land.line.y), land.line.z) * ease(shows / LINE_FADE, shows, q);
}"""),
 ("""// place along the channel growing by `grows` a pixel east and south), drawn from afar as the line `line`
// (`line_of`): the bed""", """// place along the channel growing by `grows` a pixel east and south), drawn no thinner than `line` m half wide
// (`line_of`): the bed"""),
 ("""fn river_spots(s: Spots, g: Tiled, ch: Channel, q: f32, stone: f32, grows: vec2f, line: vec2f) -> Spots {""", """fn river_spots(s: Spots, g: Tiled, ch: Channel, q: f32, stone: f32, grows: vec2f, line: f32) -> Spots {"""),
 ("""  if (g.d > BED_OUT.x * ch.bed + BED_OUT.y + 2.0 * e && abs(g.calm) > line.x + 2.0 * e) { return out; }
  var open = 0.0;
  // (No bed takes `BED_PX` pixels in a program made for draws from this far: rock.wgsl `BED_MOST_M`.)
  if (BED_PX * least_e() < BED_MOST_M) { open = ease(BED_PX * e, 2.0 * BED_PX * e, ch.law) * (1.0 - line.y); }
  let r = reach_at(at, e, g, ch, open, stone);
  // The line from afar over the bed itself, and the share of the pixel's water that is the bed's own.
  let open_sky = above_snow(s.dry.snow * s.dry.snow_m, 2.0 * max(ch.water, line.x));
  out.share = mix(r.share, line_over(g, line.x, e), line.y) * open_sky;
  out.bars = r.bars * (1.0 - line.y);
  let own = (1.0 - line.y) * r.share * open_sky / max(out.share, 1e-6);""",
  """  if (g.d > BED_OUT.x * ch.bed + BED_OUT.y + 2.0 * e && g.d > line + 2.0 * e) { return out; }
  var open = 0.0;
  // (No bed takes `BED_PX` pixels in a program made for draws from this far: rock.wgsl `BED_MOST_M`.)
  if (BED_PX * least_e() < BED_MOST_M) { open = ease(BED_PX * e, 2.0 * BED_PX * e, ch.law) * (1.0 - ease(ch.law, 2.0 * ch.law, line)); }
  let r = reach_at(at, e, g, ch, line, open, stone);
  out.share = r.share * above_snow(s.dry.snow * s.dry.snow_m, 2.0 * max(ch.water, line));
  out.bars = r.bars;"""),
 ("""  let resolved = ease(e, 2.0 * e, ch.water) * open * own;""", """  let resolved = ease(e, 2.0 * e, ch.water) * open;"""),
 ("""  // (A line from afar is coloured as the river's deep water, across it as `LINE_SHALLOW` says.)
  let off = clamp(abs(g.calm) / max(line.x, 1e-6), 0.0, 1.0);
  let look = ease(LINE_LOOK_PX.x * e, LINE_LOOK_PX.y * e, 2.0 * line.x);
  let afar = vec2f(1.0 - LINE_SHALLOW * look * off * off, look * ease(LINE_EDGE, 1.0, off));
  out.water = mix(afar, vec2f(out.wet.depth / (BED_MIDDLE * ch.mean), 1.0 - ease(0.0, EDGE_M, (1.0 - abs(x)) * ch.water)), resolved);""",
  """  // (A channel too narrow for its bed's shape to show is coloured as the river's deep water.)
  out.water = vec2f(mix(1.0, out.wet.depth / (BED_MIDDLE * ch.mean), resolved), resolved * (1.0 - ease(0.0, EDGE_M, (1.0 - abs(x)) * ch.water)));"""),
])

L = 'web/shaders/lib/land.wgsl'
edit(L, [
 ("""// line with its wander, tape/src/ground/bends.rs) the distance signed by the side, m (`calm`), and the nearest
// texel's least distance to it, m (`calm_end`); and how
""", """// line with its wander, tape/src/ground/bends.rs) and how
"""),
 ("""along: f32, calm: f32, calm_end: f32, lean: f32 }""", """along: f32, lean: f32 }"""),
 ("""  let calm_end = max(abs(a4[corner].w) - HALF_DIAGONAL * texel, 0.0);
""", ""),
 ("""signed, along, calm, calm_end, lean); }""", """signed, along, lean); }"""),
 ("""signed, along, calm, calm_end, lean);
}""", """signed, along, lean);
}"""),
 ("""  p.depth = max(max(p.depth, cut), ROOTS_M * max(select(0.0, 1.0, g.d < c.bed), ease(0.0, FAN_BARE, fan_at(g, c, p.depth))));""",
  """  p.depth = max(max(p.depth, cut), select(0.0, ROOTS_M, g.d < c.bed));"""),
 ("""its bed too, and its floods over the whole of it (`channel_at`): no plant roots in a channel's bed, widened at its
// mouth, nor on the fan it lays under standing liquid (rock.wgsl `fan_at`).""", """its bed too, and its floods over the whole of it (`channel_at`): no plant roots in a channel's bed, widened at its
// mouth."""),
 ("""  // The terrain's samples across and down, their depth, and the screen's pixels a css pixel.""", """  // The terrain's samples across and down, their depth, and the pixels drawn a css pixel."""),
])
t = open(L).read()
import re
n = t.count("NO_LIQUID_M, NO_LIQUID_M, 0.0);")
assert n == 1, n
t = t.replace("NO_LIQUID_M, NO_LIQUID_M, 0.0);", "0.0);")
i = t.index("mix(under.sd, shown.sd, w), shown.along,")
j = t.index(");", i)
t = t[:i] + "mix(under.sd, shown.sd, w), shown.along, shown.lean" + t[j:]
open(L, 'w').write(t)

K = 'web/shaders/lib/rock.wgsl'
cut(K, "// The share of a channel's fan at a point of ground `g` under standing liquid", "// The depth of the bed of channel `c` under its water's surface, m, the share `x`")
edit(K, [
 ("""// (land.wgsl `over_ponds`). Where it moves stones of `BAR_SAND_STONE` m or more (a channel that moves less carries
// sand and silt alone) it lays a fan under the liquid: sand within `FAN_WIDTHS` of its bed's half widths of its line,
// under liquid shallower than `FAN_DEPTH_M` (`fan_at`).
// No plant roots where `FAN_BARE` of a fan or more lies (land.wgsl `state_at`).
const MOUTH_FLARE = 1.5; const MOUTH_OVER_M = 1.5; const FAN_WIDTHS = 5.0; const FAN_DEPTH_M = 1.5; const FAN_BARE = 0.2;
const BAR_SAND_STONE = 0.03;""", """// (land.wgsl `over_ponds`). A channel that moves no stones of `BAR_SAND_STONE` m carries sand and silt alone.
const MOUTH_FLARE = 1.5; const MOUTH_OVER_M = 1.5;
const BAR_SAND_STONE = 0.03;"""),
 ("""// The depth of the bed of channel `c` under its water's surface at the point of ground `g`, m (`depth_across`).
fn depth_in(g: Tiled, c: Channel) -> f32 {
  return depth_across(c, clamp((g.sd - c.middle) * c.outer / max(c.water, 1e-6), -1.0, 1.0));
}""", """// The depth of the bed of channel `c` under its water's surface at the point of ground `g`, m (`depth_across`): none
// where the point is standing liquid's (the share of it that is: a body's floor is its own, and no channel is cut
// into it).
fn depth_in(g: Tiled, c: Channel) -> f32 {
  return depth_across(c, clamp((g.sd - c.middle) * c.outer / max(c.water, 1e-6), -1.0, 1.0)) * (1.0 - g.water.y);
}"""),
])
t = open(K).read()
assert "return Channel(bed, water, middle, land.channel.z * pow(q, land.channel.w) / flare, 1.0 / wide, bend, outer, banks);\n}\n\n}" not in t
open(K, 'w').write(t)

edit('web/shaders/land.wgsl', [
 ("""  let near = g.d < CHANNEL_NEAR * banks + 2.0 * e || abs(g.calm) < line_most(e) + 2.0 * e;""", """  let near = g.d < max(CHANNEL_NEAR * banks, line_most(e)) + 2.0 * e;"""),
 ("""  // A river's fan under the liquid it runs into.
  p.sand = max(p.sand, fan_at(g, ch, p.depth));
""", ""),
 ("""  return river_spots(out, g, ch, q, moved, grows, line_of(q, e, banks));""", """  return river_spots(out, g, ch, q, moved, grows, line_of(q, e));"""),
])

S = 'web/shaders/lib/style.wgsl'
edit(S, [
 ("""// The depth a lake or a river `depth` m deep is coloured as, where its bed rises at `slope` (rise over run) to a rim
// `rim_m` deep, seen at `e` m a pixel (`LAKE_SCALE`, `LAKE_RIM_PX`).
fn lake_seen(depth: f32, slope: f32, e: f32, rim_m: f32) -> f32 {
  let rim = ease(0.0, max(rim_m, slope * LAKE_RIM_PX * e), depth);
  return min(LAKE_SHALLOWEST_M + depth * LAKE_SCALE, LAKE_DEEPEST_M) * rim * rim;
}""", """// How far from its shore standing liquid `depth` m deep lies, 0 at its edge to 1 where it is as deep as its rim: its
// bed rises at `slope` (rise over run) to a rim `rim_m` deep, seen at `e` m a pixel (`LAKE_RIM_PX`). The share of
// the deep water's colour it takes (`river_seen`'s `rel`: a lake's shallows are a river's).
fn shore_rim(depth: f32, slope: f32, e: f32, rim_m: f32) -> f32 {
  return ease(0.0, max(rim_m, slope * LAKE_RIM_PX * e), depth);
}"""),
])

C = 'web/shaders/lib/colour.wgsl'
edit(C, [
 ("""      let seen = mix(lake_seen(p.depth, p.slope / earth.look.x, p.e, LAKE_RIM_M), p.depth, p.sea);
      water = mix(water_painted(bed, seen, ripple), FOAM_RGB, p.sea * foam(p.depth, p.slope / earth.look.x, p.e));""",
  """      // One water: standing liquid is coloured by the channel's law (style.wgsl `river_seen`), its shore the
      // channel's shallows and its open water the channel's deep thread, so a river runs into a lake or the sea with
      // no step of colour; the sea by its own depth, a lake as `LAKE_SCALE` times as deep.
      let seen = river_seen(p.depth * mix(1.0, 1.0 / LAKE_SCALE, p.sea), shore_rim(p.depth, p.slope / earth.look.x, p.e, LAKE_RIM_M));
      water = mix(water_painted(BED_SHOWN * bed, seen, ripple), FOAM_RGB, p.sea * foam(p.depth, p.slope / earth.look.x, p.e));"""),
])
