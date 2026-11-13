def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/belt.rs', [
 ("""/// A half bend's length lies between these, in channel widths, and it carries the line at least `ADVANCE_WIDTHS` down
/// the valley (on a hard bed from `HARD_ARC` on, at least `HARD_ADVANCE` down it); its least length is up to
/// `ARC_VARY` times longer by its own draw (squared), so the turns that reach little come at no even pace; a straight
/// stretch is walked in pieces of `STRAIGHT_WIDTHS`.""",
  """/// A half bend's length lies between these, in channel widths, and it carries the line at least `ADVANCE_WIDTHS` down
/// the valley (on a hard bed from `HARD_ARC` on, at least `HARD_ADVANCE` down it); its least length is up to
/// `ARC_VARY` times longer by its own draw (squared; `HARD_VARY` on a hard bed, where a short sharp turn follows a
/// long stretch that runs nearly straight), so the turns that reach little come at no even pace; a straight stretch
/// is walked in pieces of `STRAIGHT_WIDTHS`."""),
 ("""const ARC_VARY: f64 = 3.0;""", """const ARC_VARY: f64 = 3.0;
const HARD_VARY: f64 = 12.0;"""),
 ("""            let least = (ARC_WIDTHS[0] + (HARD_ARC - ARC_WIDTHS[0]) * room.hard) * (1.0 + (ARC_VARY - 1.0) * arc_draw * arc_draw) * room.width;""",
  """            let vary = ARC_VARY + (HARD_VARY - ARC_VARY) * room.hard;
            let least = (ARC_WIDTHS[0] + (HARD_ARC - ARC_WIDTHS[0]) * room.hard) * (1.0 + (vary - 1.0) * arc_draw * arc_draw) * room.width;"""),
])

edit('web/shaders/lib/land.wgsl', [
 ("""// line with its wander, tape/src/ground/bends.rs; the line itself where the texels hold none) the distance signed by
// the side, m, its slope, and the nearest texel's least distance to it (`calm`, `calm_slope`, `calm_end`); and how far
// the line has swung from that one in its bend there, m, signed as the side its outer bank lies on (`lean`).
struct Tiled { h: f32, slope: vec2f, twi: f32, d: f32, d_slope: vec2f, channels: vec4i, ends: vec2f, water: vec2f, sd: f32, along: f32, calm: f32, calm_slope: vec2f, calm_end: f32, lean: f32 }""",
  """// line with its wander, tape/src/ground/bends.rs) the distance signed by the side, m, and its slope (`calm`,
// `calm_slope`); the nearest texel's own signed distances to the two lines and half its diagonal, m (`own`); and how
// far the line has swung from the line without bends in its bend there, m, signed as the side its outer bank lies on
// (`lean`; none where the texels do not hold both lines).
struct Tiled { h: f32, slope: vec2f, twi: f32, d: f32, d_slope: vec2f, channels: vec4i, ends: vec2f, water: vec2f, sd: f32, along: f32, calm: f32, calm_slope: vec2f, own: vec3f, lean: f32 }"""),
 ("""  var unbent = vec4f(signed, d_slope, ends.x);
  if (c_hi < NO_LINE_M && hi < NO_LINE_M) {
    if (c_lo < 0.0 && c_hi > 0.0 && c_hi - c_lo > SAME_LINE_TEXELS * texel) { cd = abs(cd); }
    let cx = mix(cd.x, cd.y, f.x);
    let cy = mix(cd.z, cd.w, f.x);
    let c = mix(cx, cy, f.y);
    unbent = vec4f(c, sign(c) * vec2f(mix(cd.y - cd.x, cd.w - cd.z, f.y), cy - cx) / texel, max(abs(a4[corner].w) - HALF_DIAGONAL * texel, 0.0));
  }
  let lean = unbent.x - signed;
  if (!spline) { return Tiled(o.x + o.y, vec2f(0.0), o.z, abs(signed), d_slope, channels, ends, water, signed, along, unbent.x, unbent.yz, unbent.w, lean); }""",
  """  if (c_lo < 0.0 && c_hi > 0.0 && c_hi - c_lo > SAME_LINE_TEXELS * texel) { cd = abs(cd); }
  let cx = mix(cd.x, cd.y, f.x);
  let cy = mix(cd.z, cd.w, f.x);
  let calm = mix(cx, cy, f.y);
  let calm_slope = sign(calm) * vec2f(mix(cd.y - cd.x, cd.w - cd.z, f.y), cy - cx) / texel;
  let own = vec3f(raw[corner], a4[corner].w, HALF_DIAGONAL * texel);
  let lean = select(0.0, calm - signed, c_hi < NO_LINE_M && hi < NO_LINE_M);
  if (!spline) { return Tiled(o.x + o.y, vec2f(0.0), o.z, abs(signed), d_slope, channels, ends, water, signed, along, calm, calm_slope, own, lean); }"""),
 ("""ends, water, signed, along, unbent.x, unbent.yz, unbent.w, lean);
}""", """ends, water, signed, along, calm, calm_slope, own, lean);
}"""),
 ("""vec2f(-NO_LIQUID_M, 0.0), NO_LIQUID_M, 0.0, NO_LIQUID_M, vec2f(0.0), NO_LIQUID_M, 0.0);""", """vec2f(-NO_LIQUID_M, 0.0), NO_LIQUID_M, 0.0, NO_LIQUID_M, vec2f(0.0), vec3f(NO_LIQUID_M, NO_LIQUID_M, 0.0), 0.0);"""),
 ("""mix(under.calm_slope, shown.calm_slope, w), shown.calm_end, shown.lean);""", """mix(under.calm_slope, shown.calm_slope, w), shown.own, shown.lean);"""),
])

edit('web/shaders/lib/rock.wgsl', [
 ("""// two through each point where they cross, the same width all along), and the bend's swing that much less.
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
}""", """// two through each point where they cross, the same width all along), and the bend's swing that much less. A line
// the texels do not hold lies far away (tape/src/ground/channels.rs `NO_CHANNEL_M`), and so does a blend with it.
fn unbent(g: Tiled, far: f32) -> Tiled {
  var out = g;
  if (far <= 0.0) { return out; }
  var slope = sign(g.calm) * g.calm_slope;
  if (far < 1.0) { slope = mix(sign(g.sd) * g.d_slope, slope, far); }
  let steep = clamp(length(slope), UNBENT_SLOPE.x, UNBENT_SLOPE.y);
  out.sd = select(mix(g.sd, g.calm, far), g.calm, far >= 1.0) / steep;
  out.d = abs(out.sd);
  out.d_slope = sign(out.sd) * slope / max(length(slope), 1e-6);
  out.ends.x = max(abs(select(mix(g.own.x, g.own.y, far), g.own.y, far >= 1.0)) - g.own.z, 0.0);
  out.lean = g.lean * (1.0 - far);
  return out;
}"""),
 ("""// (Where the two lines run against one another their blend's slope is near none: no less than this.)
const UNBENT_SLOPE_LEAST = 0.3;""", """// (Where the two lines run against one another their blend's slope is near none, and between the texels of two
// channels it is steep: taken within these.)
const UNBENT_SLOPE = vec2f(0.3, 1.2);"""),
])
