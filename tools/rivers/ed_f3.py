def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/land.wgsl', [
 ("""// step from the sample that holds the texel's centre (0: none), one more than the pond it lies in (0: none), and of the
// line's nearest point how far along the channel it lies, m (modulo `land.reach.x`), and the line's curvature, 1/m.""",
  """// step from the sample that holds the texel's centre (0: none), one more than the pond it lies in (0: none), of the
// line's nearest point how far along the channel it lies, m (modulo `land.reach.x`), and the signed distance to the
// nearest channel's line without its bends, m."""),
 ("""// the side the point lies on, m; and of the line's nearest point, how far along the channel it lies, m (modulo
// `land.reach.x`, blended across the texels of its stretch), and the line's curvature, 1/m, positive where it turns
// toward the side the signed distance is positive on.
struct Tiled { h: f32, slope: vec2f, twi: f32, d: f32, d_slope: vec2f, channels: vec4i, ends: vec2f, water: vec2f, sd: f32, along: f32, curve: f32 }""",
  """// the side the point lies on, m; of the line's nearest point, how far along the channel it lies, m (modulo
// `land.reach.x`, blended across the texels of its stretch); the distance to the channel's line without its bends, m
// (`calm`: its valley's line with its wander, tape/src/ground/bends.rs; that of the line itself where none is held);
// and how far the line has swung from that one in its bend there, m, signed as the side its outer bank lies on (`lean`).
struct Tiled { h: f32, slope: vec2f, twi: f32, d: f32, d_slope: vec2f, channels: vec4i, ends: vec2f, water: vec2f, sd: f32, along: f32, calm: f32, lean: f32 }"""),
 ("""  // The place along the channel and its curvature: the nearest texel's, blended with those of the others that hold
  // the same stretch of the line (their places taken the short way round the length they are kept modulo).
  let own = a4[corner].zw;
  var line = vec3f(0.0);
  for (var n = 0; n < 4; n++) {
    let step = a4[n].z - own.x - land.reach.x * round((a4[n].z - own.x) / land.reach.x);
    if (abs(step) <= SAME_STRETCH_TEXELS * texel) { line += bilinear[n] * vec3f(step, a4[n].w, 1.0); }
  }
  let along = own.x + line.x / line.z;
  let curve = line.y / line.z;
  if (!spline) { return Tiled(o.x + o.y, vec2f(0.0), o.z, abs(signed), d_slope, channels, ends, water, signed, along, curve); }""",
  """  // The place along the channel: the nearest texel's, blended with those of the others that hold the same stretch of
  // the line (their places taken the short way round the length they are kept modulo).
  let own = a4[corner].z;
  var line = vec2f(0.0);
  for (var n = 0; n < 4; n++) {
    let step = a4[n].z - own - land.reach.x * round((a4[n].z - own) / land.reach.x);
    if (abs(step) <= SAME_STRETCH_TEXELS * texel) { line += bilinear[n] * vec2f(step, 1.0); }
  }
  let along = own + line.x / line.y;
  // The distance to the line without its bends, blended as the line's own; the side its slope and the line's agree
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
  if (!spline) { return Tiled(o.x + o.y, vec2f(0.0), o.z, abs(signed), d_slope, channels, ends, water, signed, along, calm, lean); }"""),
 ("""  return Tiled(h.x, h.yz / texel, o.z, abs(signed), d_slope, channels, ends, water, signed, along, curve);""", """  return Tiled(h.x, h.yz / texel, o.z, abs(signed), d_slope, channels, ends, water, signed, along, calm, lean);"""),
 ("""vec2f(-NO_LIQUID_M, 0.0), NO_LIQUID_M, 0.0, 0.0);""", """vec2f(-NO_LIQUID_M, 0.0), NO_LIQUID_M, 0.0, NO_LIQUID_M, 0.0);"""),
 ("""mix(under.sd, shown.sd, w), shown.along, shown.curve);""", """mix(under.sd, shown.sd, w), shown.along, mix(under.calm, shown.calm, w), shown.lean);"""),
 ("""// Half a texel's diagonal, in texels.
const HALF_DIAGONAL = 0.71;""", """// Half a texel's diagonal, in texels.
const HALF_DIAGONAL = 0.71;
// A texel no channel reaches holds a distance to a line past this, m (tape/src/ground/channels.rs `NO_CHANNEL_M`).
const NO_LINE_M = 3.0e4;"""),
])
edit('web/shaders/lib/rock.wgsl', [
 ("""// On the inside of a bend a bed reaches no farther from its line than this share of the radius the line turns on (its
// banks would meet past the bend's centre).
const INNER_MOST = 0.8;
// A bend is full where the line's curvature times the bed's width passes `BEND_FULL` (a bed turning on a radius of 2.5
// of its widths). There the water is a pool against the outer bank, `POOL_NARROW` narrower, over a bar on the inside
// (Leopold and Wolman 1960); between bends it is a riffle, `RIFFLE_WIDE` wider and shallower, across the whole bed.
const BEND_FULL = 0.4; const POOL_NARROW = 0.2; const RIFFLE_WIDE = 0.3;""", """// A bend shows from where the line has swung `BEND_OFF.x` of the bed's half width from its line without bends
// (`Tiled.lean`), and is full from `BEND_OFF.y`: at its apex. There the water is a pool against the outer bank,
// `POOL_NARROW` narrower, over a bar on the inside (Leopold and Wolman 1960); where the line crosses over between two
// bends it is a riffle, `RIFFLE_WIDE` wider and shallower, across the whole bed.
const BEND_OFF = vec2f(0.5, 3.0); const POOL_NARROW = 0.2; const RIFFLE_WIDE = 0.3;"""),
 ("""// side of its line, m (`banks_of`, varied along it, held on the inside of a bend); the half width of this month's
// water, m, and where its middle lies across the line (as `Tiled.sd`), m: against the outer bank in a bend; its mean""",
  """// side of its line, m (`banks_of`, varied along it); the half width of this month's
// water, m, and where its middle lies across the line (as `Tiled.sd`), m: against the outer bank in a bend; its mean"""),
 ("""  let turn = abs(g.curve);
  let bend = min(turn * 2.0 * law / BEND_FULL, 1.0);
  let outer = select(1.0, -1.0, g.curve > 0.0);
  var bed = law * vary;
  if (g.sd * g.curve > 0.0) { bed = min(bed, INNER_MOST / max(turn, 1e-6)); }
  let wide""", """  let bend = ease(BEND_OFF.x * law, BEND_OFF.y * law, abs(g.lean));
  let outer = select(1.0, -1.0, g.lean < 0.0);
  let bed = law * vary;
  let wide"""),
 ("""// flow through a narrower or a wider water); how much of a bend the place is, 0..1; and the side its outer bank lies
// on (the sign of `Tiled.sd` there; 1 where the line is straight).""", """// flow through a narrower or a wider water); how much of a bend the place is, 0..1; and the side its outer bank lies
// on (the sign of `Tiled.sd` there)."""),
])
edit('web/shaders/lib/river.wgsl', [
 ("""  let inside = g.sd * g.curve > 0.0;""", """  let inside = g.sd * g.lean < 0.0;"""),
 ("""// The reach at `at`, m, seen at `e` m a pixel, on ground `g` of the channel `c` (rock.wgsl `channel_at`), drawn no
// thinner than `line` m half wide from afar, `open` of its bed drawn apart from its water (land.wgsl), whose flow moves
// stones of `stone` m.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, line: f32, open: f32, stone: f32) -> Reach {""", """// The reach at `at`, m, seen at `e` m a pixel, on ground `g` of the channel `c` (rock.wgsl `channel_at`), drawn no
// thinner than `line` m half wide from afar, `open` of its bed drawn apart from its water (land.wgsl), whose flow moves
// stones of `stone` m. A line `far` wider than its bed (0..1, land.wgsl `LINE_BENDS`) is drawn about the channel's
// line without its bends (`Tiled.calm`): a line never shows bends it would swallow, and they come in as the view
// nears.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, line: f32, open: f32, far: f32, stone: f32) -> Reach {"""),
 ("""  let reach = max(banks, line);
  // Past a line's end its texels' signed distances change sign across its extension and blend to a false 0 there: the
  // nearest texel lies far from the line itself (`Tiled.ends`), and nothing is drawn (it drew small square brackets).
  let real = 1.0 - ease(reach + e, reach + 2.0 * e, g.ends.x);
  var in_bed = (tent((reach - g.d) / e) - tent((-reach - g.d) / e)) * real;""", """  let reach = max(banks, line);
  // Past a line's end its texels' signed distances change sign across its extension and blend to a false 0 there: the
  // nearest texel lies far from the line itself (`Tiled.ends`), and nothing is drawn (it drew small square brackets).
  let real = 1.0 - ease(reach + e, reach + 2.0 * e, g.ends.x);
  let from_line = mix(g.d, g.calm, far);
  var in_bed = (tent((reach - from_line) / e) - tent((-reach - from_line) / e)) * real;"""),
])
edit('web/shaders/land.wgsl', [
 ("""// than the channel law widens a channel: a map's choice, so the network's order reads), up to `LINE_MOST` times, so a
// stream is a hair and the main river a few times wider: no wider than its bends are drawn from as near (a tile draws
// them once the bed takes a texel or two, tape/src/ground/channels.rs `BEND_FROM`); it fades in from `LINE_FADE` times
// less, so no channel springs up as the view zooms.
const LINE_PX = 1.2;
const LINE_MOST = 4.0;
const LINE_GROW = 0.75;
const LINE_FADE = 4.0;""", """// than the channel law widens a channel: a map's choice, so the network's order reads), up to `LINE_MOST` times, so a
// stream is a hair and the main river many times wider, and a river widens down its way and below each tributary; it
// fades in from `LINE_FADE` times less, so no channel springs up as the view zooms. A line wider than its bed is drawn
// about the channel's line without its bends (lib/river.wgsl `reach_at`), all of it from `LINE_BENDS` times the bed's
// width: the bends come in as the view nears and the line narrows to the bed.
const LINE_PX = 1.6;
const LINE_MOST = 10.0;
const LINE_GROW = 0.6;
const LINE_FADE = 4.0;
const LINE_BENDS = 2.0;"""),
 ("""  var open = 0.0;
  // (No bed takes `BED_PX` pixels in a program made for draws from this far: rock.wgsl `BED_MOST_M`.)
  if (BED_PX * least_e() < BED_MOST_M) { open = ease(BED_PX * e, 2.0 * BED_PX * e, banks_of(q)) * (1.0 - ease(banks_of(q), 2.0 * banks_of(q), line)); }
  let r = reach_at(at, e, g, ch, line, open, stone);""", """  let far = ease(banks_of(q), LINE_BENDS * banks_of(q), line);
  var open = 0.0;
  // (No bed takes `BED_PX` pixels in a program made for draws from this far: rock.wgsl `BED_MOST_M`.)
  if (BED_PX * least_e() < BED_MOST_M) { open = ease(BED_PX * e, 2.0 * BED_PX * e, banks_of(q)) * (1.0 - far); }
  let r = reach_at(at, e, g, ch, line, open, far, stone);"""),
])
