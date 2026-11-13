def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/land.wgsl', [
 ("""  // far the signs of the flow have slid down the slowest water, m (web/src/land.ts `slid`).
  reach: vec4f,
}""", """  // far the signs of the flow have slid down the slowest water, m (web/src/land.ts `slid`).
  reach: vec4f,
  // The line a channel is drawn as from afar (tape/src/ground/channels.rs `LINE_PX`): its css pixels where its
  // discharge is the one that shows, the power of its discharge over that it widens by, and the most times.
  line: vec4f,
}"""),
 ("""// `land.reach.x`, blended across the texels of its stretch); the distance to the channel's line without its bends, m
// (`calm`: its valley's line with its wander, tape/src/ground/bends.rs; that of the line itself where none is held);
// and how far the line has swung from that one in its bend there, m, signed as the side its outer bank lies on (`lean`).
struct Tiled { h: f32, slope: vec2f, twi: f32, d: f32, d_slope: vec2f, channels: vec4i, ends: vec2f, water: vec2f, sd: f32, along: f32, calm: f32, lean: f32 }""",
  """// `land.reach.x`, blended across the texels of its stretch); of the channel's line without its bends (its valley's
// line with its wander, tape/src/ground/bends.rs; the line itself where the texels hold none) the distance signed by
// the side, m, its slope, and the nearest texel's least distance to it (`calm`, `calm_slope`, `calm_end`); and how far
// the line has swung from that one in its bend there, m, signed as the side its outer bank lies on (`lean`).
struct Tiled { h: f32, slope: vec2f, twi: f32, d: f32, d_slope: vec2f, channels: vec4i, ends: vec2f, water: vec2f, sd: f32, along: f32, calm: f32, calm_slope: vec2f, calm_end: f32, lean: f32 }"""),
 ("""  // The distance to the line without its bends, blended as the line's own; the side its slope and the line's agree
  // on is the bend's outer side, and what it is more than the line's distance on that side is the bend's swing.
  var cd = vec4f(a4[0].w, a4[1].w, a4[2].w, a4[3].w);
  let c_lo = min(min(cd.x, cd.y), min(cd.z, cd.w));
  let c_hi = max(max(cd.x, cd.y), max(cd.z, cd.w));
  if (c_lo < 0.0 && c_hi > 0.0 && c_hi - c_lo > SAME_LINE_TEXELS * texel) { cd = abs(cd); }
  let c_signed = mix(mix(cd.x, cd.y, f.x), mix(cd.z, cd.w, f.x), f.y);
  let c_slope = sign(c_signed) * vec2f(mix(cd.y - cd.x, cd.w - cd.z, f.y), mix(cd.z, cd.w, f.x) - mix(cd.x, cd.y, f.x));
  var calm = abs(signed);
  var lean = 0.0;
  if (c_hi < NO_LINE_M) {
    calm = abs(c_signed);
    let outer = select(-1.0, 1.0, dot(c_slope, d_slope) * signed >= 0.0);
    lean = outer * max(calm - outer * signed, 0.0);
  }
  if (!spline) { return Tiled(o.x + o.y, vec2f(0.0), o.z, abs(signed), d_slope, channels, ends, water, signed, along, calm, lean); }""",
  """  // The line without its bends, blended as the line's own; the line stands off it by the difference of the two
  // signed distances (exact where they run side by side): its swing in its bend.
  var cd = vec4f(a4[0].w, a4[1].w, a4[2].w, a4[3].w);
  let c_lo = min(min(cd.x, cd.y), min(cd.z, cd.w));
  let c_hi = max(max(cd.x, cd.y), max(cd.z, cd.w));
  var unbent = vec4f(signed, d_slope, ends.x);
  if (c_hi < NO_LINE_M && hi < NO_LINE_M) {
    if (c_lo < 0.0 && c_hi > 0.0 && c_hi - c_lo > SAME_LINE_TEXELS * texel) { cd = abs(cd); }
    let cx = mix(cd.x, cd.y, f.x);
    let cy = mix(cd.z, cd.w, f.x);
    let c = mix(cx, cy, f.y);
    unbent = vec4f(c, sign(c) * vec2f(mix(cd.y - cd.x, cd.w - cd.z, f.y), cy - cx) / texel, max(abs(a4[corner].w) - HALF_DIAGONAL * texel, 0.0));
  }
  let lean = unbent.x - signed;
  if (!spline) { return Tiled(o.x + o.y, vec2f(0.0), o.z, abs(signed), d_slope, channels, ends, water, signed, along, unbent.x, unbent.yz, unbent.w, lean); }"""),
 ("""  return Tiled(h.x, h.yz / texel, o.z, abs(signed), d_slope, channels, ends, water, signed, along, calm, lean);""",
  """  return Tiled(h.x, h.yz / texel, o.z, abs(signed), d_slope, channels, ends, water, signed, along, unbent.x, unbent.yz, unbent.w, lean);"""),
 ("""vec2f(-NO_LIQUID_M, 0.0), NO_LIQUID_M, 0.0, NO_LIQUID_M, 0.0);""", """vec2f(-NO_LIQUID_M, 0.0), NO_LIQUID_M, 0.0, NO_LIQUID_M, vec2f(0.0), NO_LIQUID_M, 0.0);"""),
 ("""mix(under.sd, shown.sd, w), shown.along, mix(under.calm, shown.calm, w), shown.lean);""", """mix(under.sd, shown.sd, w), shown.along, mix(under.calm, shown.calm, w), mix(under.calm_slope, shown.calm_slope, w), shown.calm_end, shown.lean);"""),
])

edit('web/shaders/lib/rock.wgsl', [
 ("""// A channel at a point of ground `g`, of this month's discharge `q`, m³/s: the half width of its bed on the point's""",
  """// The ground `g` with its channel's line drawn `far` of the way (0..1) from the line itself to the line without its
// bends (`Tiled.calm`): the two signed distances blended, over their blend's slope (the line so drawn runs between the
// two through each point where they cross, the same width all along), and the bend's swing that much less.
fn unbent(g: Tiled, far: f32) -> Tiled {
  var out = g;
  if (far <= 0.0) { return out; }
  let slope = mix(sign(g.sd) * g.d_slope, sign(g.calm) * g.calm_slope, far);
  let steep = max(length(slope), UNBENT_SLOPE_LEAST);
  out.sd = mix(g.sd, g.calm, far) / steep;
  out.d = abs(out.sd);
  out.d_slope = sign(out.sd) * slope / steep;
  out.ends.x = mix(g.ends.x, g.calm_end, far);
  out.lean = g.lean * (1.0 - far);
  return out;
}
// (Where the two lines run against one another their blend's slope is near none: no less than this.)
const UNBENT_SLOPE_LEAST = 0.3;

// A channel at a point of ground `g`, of this month's discharge `q`, m³/s: the half width of its bed on the point's"""),
])

edit('web/shaders/lib/river.wgsl', [
 ("""// stones of `stone` m. A line `far` wider than its bed (0..1, land.wgsl `LINE_BENDS`) is drawn about the channel's
// line without its bends (`Tiled.calm`): a line never shows bends it would swallow, and they come in as the view
// nears.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, line: f32, open: f32, far: f32, stone: f32) -> Reach {""",
  """// stones of `stone` m.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, line: f32, open: f32, stone: f32) -> Reach {"""),
 ("""  let from_line = mix(g.d, g.calm, far);
  var in_bed = (tent((reach - from_line) / e) - tent((-reach - from_line) / e)) * real;""",
  """  var in_bed = (tent((reach - g.d) / e) - tent((-reach - g.d) / e)) * real;"""),
])

edit('web/shaders/land.wgsl', [
 ("""// A channel shows as a line of at least `LINE_PX` css pixels once its discharge passes the discharge that shows in the""",
  """// A channel shows as a line of at least `land.line.x` css pixels once its discharge passes the discharge that shows in the"""),
 ("""// than the channel law widens a channel: a map's choice, so the network's order reads), up to `LINE_MOST` times, so a
// stream is a hair and the main river many times wider, and a river widens down its way and below each tributary; it
// fades in from `LINE_FADE` times less, so no channel springs up as the view zooms. A line wider than its bed is drawn
// about the channel's line without its bends (lib/river.wgsl `reach_at`), all of it from `LINE_BENDS` times the bed's
// width: the bends come in as the view nears and the line narrows to the bed.
const LINE_PX = 1.6;
const LINE_MOST = 10.0;
const LINE_GROW = 0.6;
const LINE_FADE = 4.0;
const LINE_BENDS = 2.0;""",
  """// than the channel law widens a channel: a map's choice, so the network's order reads), up to `land.line.z` times
// (tape/src/ground/channels.rs `LINE_PX`); it fades in from `LINE_FADE` times less, so no channel springs up as the view
// zooms. A line wider than its bed is drawn toward the channel's line without its bends (rock.wgsl `unbent`), all the
// way from `LINE_BENDS` times the bed's width: there the line is wider than about a sixth of a bend's length and would
// swallow its bends, and they come in as the view nears and the line narrows to the bed.
const LINE_FADE = 4.0;
const LINE_BENDS = 2.0;

// The line a channel of discharge `q` is drawn no thinner than where a pixel takes `e` m: its half width, m, and how
// far it is drawn toward the line without its bends, 0..1.
fn line_of(q: f32, e: f32) -> vec2f {
  if (q <= 0.0) { return vec2f(0.0); }
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * land.line.x * css * min(pow(q / shows, land.line.y), land.line.z) * ease(shows / LINE_FADE, shows, q);
  return vec2f(line, ease(banks_of(q), LINE_BENDS * banks_of(q), line));
}"""),
 ("""  let q = mix(discharge_of(reach), discharge_of(next.s), next.share);
  let ch = channel_at(g, q);""", """  let q = mix(discharge_of(reach), discharge_of(next.s), next.share);
  g = unbent(g, line_of(q, e).y);
  let ch = channel_at(g, q);"""),
 ("""  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * LINE_PX * css * min(pow(q / shows, LINE_GROW), LINE_MOST) * ease(shows / LINE_FADE, shows, q);
  let far = ease(banks_of(q), LINE_BENDS * banks_of(q), line);
  var open = 0.0;
  // (No bed takes `BED_PX` pixels in a program made for draws from this far: rock.wgsl `BED_MOST_M`.)
  if (BED_PX * least_e() < BED_MOST_M) { open = ease(BED_PX * e, 2.0 * BED_PX * e, banks_of(q)) * (1.0 - far); }
  let r = reach_at(at, e, g, ch, line, open, far, stone);""",
  """  let line = line_of(q, e);
  var open = 0.0;
  // (No bed takes `BED_PX` pixels in a program made for draws from this far: rock.wgsl `BED_MOST_M`.)
  if (BED_PX * least_e() < BED_MOST_M) { open = ease(BED_PX * e, 2.0 * BED_PX * e, banks_of(q)) * (1.0 - line.y); }
  let r = reach_at(at, e, g, ch, line.x, open, stone);"""),
])
