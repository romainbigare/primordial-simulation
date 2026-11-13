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

edit('web/shaders/lib/land.wgsl', [
 ("""// line with its wander, tape/src/ground/bends.rs) the distance signed by the side, m, and its slope (`calm`,
// `calm_slope`); the nearest texel's own signed distances to the two lines and half its diagonal, m (`own`); and how
""", """// line with its wander, tape/src/ground/bends.rs) the distance signed by the side, m (`calm`), and the nearest
// texel's least distance to it, m (`calm_end`); and how
"""),
 ("""along: f32, calm: f32, calm_slope: vec2f, own: vec3f, lean: f32 }""", """along: f32, calm: f32, calm_end: f32, lean: f32 }"""),
 ("""  let calm_slope = sign(calm) * vec2f(mix(cd.y - cd.x, cd.w - cd.z, f.y), cy - cx) / texel;
  let nearest = vec3f(raw[corner], a4[corner].w, HALF_DIAGONAL * texel);
""", """  let calm_end = max(abs(a4[corner].w) - HALF_DIAGONAL * texel, 0.0);
"""),
 ("""signed, along, calm, calm_slope, nearest, lean); }""", """signed, along, calm, calm_end, lean); }"""),
 ("""signed, along, calm, calm_slope, nearest, lean);
}""", """signed, along, calm, calm_end, lean);
}"""),
 ("""NO_LIQUID_M, vec2f(0.0), vec3f(NO_LIQUID_M, NO_LIQUID_M, 0.0), 0.0);""", """NO_LIQUID_M, NO_LIQUID_M, 0.0);"""),
 ("""mix(under.calm_slope, shown.calm_slope, w), shown.own, shown.lean);""", """shown.calm_end, shown.lean);"""),
])

cut('web/shaders/lib/rock.wgsl', "// The ground `g` with its channel's line drawn `far` of the way", "// A channel at a point of ground `g`, of this month's discharge `q`")

edit('web/shaders/lib/river.wgsl', [
 ("""// The reach at `at`, m, seen at `e` m a pixel, on ground `g` of the channel `c` (rock.wgsl `channel_at`), drawn no
// thinner than `line` m half wide from afar, `open` of its bed drawn apart from its water (land.wgsl), whose flow moves
// stones of `stone` m.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, line: f32, open: f32, stone: f32) -> Reach {""",
  """// The share of a pixel `e` m across that a channel's line from afar covers on ground `g`: a band `line` m to each
// side of the channel's line without its bends (`Tiled.calm`), none past that line's end (as `reach_at`'s).
fn line_over(g: Tiled, line: f32, e: f32) -> f32 {
  let d = abs(g.calm);
  return (tent((line - d) / e) - tent((-line - d) / e)) * (1.0 - ease(line + e, line + 2.0 * e, g.calm_end));
}

// The reach at `at`, m, seen at `e` m a pixel, on ground `g` of the channel `c` (rock.wgsl `channel_at`), `open` of
// its bed drawn apart from its water (land.wgsl), whose flow moves stones of `stone` m.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, open: f32, stone: f32) -> Reach {"""),
 ("""  let reach = max(banks, line);
  // Past a line's end""", """  let reach = banks;
  // Past a line's end"""),
])

edit('web/shaders/land.wgsl', [
 ("""// zooms. A line wider than its bed is drawn toward the channel's line without its bends (rock.wgsl `unbent`), all the
// way from `LINE_BENDS` times the bed's width: there the line is wider than about a sixth of a bend's length and would
// swallow its bends, and they come in as the view nears and the line narrows to the bed.
const LINE_FADE = 4.0;
const LINE_BENDS = 2.0;""",
  """// zooms. The line runs along the channel's line without its bends (lib/river.wgsl `line_over`): wider than about a
// sixth of a bend's length it would swallow its bends. As the view nears and the line narrows toward its bed, the
// bed itself with its bends fades in under it and the line fades out, from `LINE_BENDS.y` bed widths to
// `LINE_BENDS.x`: two pictures blended over that range of zoom, neither moving. A line `LINE_LOOK_PX` pixels wide or
// more has the water's look across it: its middle the deep thread's tone, `LINE_SHALLOW` shallower toward its
// edges, and a light wash from `LINE_EDGE` of the way to the edge.
const LINE_FADE = 4.0;
const LINE_BENDS = vec2f(1.0, 2.5);
const LINE_LOOK_PX = vec2f(1.5, 4.0); const LINE_SHALLOW = 0.6; const LINE_EDGE = 0.65;
// A channel wets the ground beside it (its fringe) as far as a pixel shows its water: none where its water is under
// `FRINGE_PX.x` pixels wide, all from `FRINGE_PX.y` (a rill's fringe drew a dark stroke straight down the slope).
const FRINGE_PX = vec2f(0.5, 2.0);"""),
 ("""// The line a channel of discharge `q` is drawn no thinner than where a pixel takes `e` m: its half width, m, and how
// far it is drawn toward the line without its bends, 0..1.""", """// The line a channel of discharge `q` is drawn as from afar where a pixel takes `e` m: its half width, m, and how
// much of it shows over the bed itself, 0..1."""),
 ("""  return vec2f(line, ease(banks_of(q), LINE_BENDS * banks_of(q), line));""", """  return vec2f(line, ease(LINE_BENDS.x * banks_of(q), LINE_BENDS.y * banks_of(q), line));"""),
 ("""  g = unbent(g, line_of(q, e).y);
""", ""),
 ("""  let shore_over = select(p.over, min(p.over, max(abs(g.sd - ch.middle) - ch.water, 0.0) * max(length(g.slope), ch.mean / max(ch.water, 1e-6))), ch.water > 0.0);""",
  """  let fringe = ease(FRINGE_PX.x * e, FRINGE_PX.y * e, 2.0 * ch.water);
  let shore_over = select(p.over, mix(p.over, min(p.over, max(abs(g.sd - ch.middle) - ch.water, 0.0) * max(length(g.slope), ch.mean / max(ch.water, 1e-6))), fringe), ch.water > 0.0);"""),
 ("""  let r = reach_at(at, e, g, ch, line.x, open, stone);
  out.share = r.share;
  out.bars = r.bars;""", """  let r = reach_at(at, e, g, ch, open, stone);
  // The line from afar over the bed itself, and the share of the pixel's water that is the bed's own.
  out.share = mix(r.share, line_over(g, line.x, e), line.y);
  out.bars = r.bars * (1.0 - line.y);
  let own = (1.0 - line.y) * r.share / max(out.share, 1e-6);"""),
 ("""  let resolved = ease(e, 2.0 * e, ch.water) * open;""", """  let resolved = ease(e, 2.0 * e, ch.water) * open * own;"""),
 ("""  // (A line from afar is coloured as the river's deep water.)
  out.water = vec2f(mix(1.0, out.wet.depth / (BED_MIDDLE * ch.mean), resolved), resolved * (1.0 - ease(0.0, EDGE_M, (1.0 - abs(x)) * ch.water)));""",
  """  // (A line from afar is coloured as the river's deep water, across it as `LINE_SHALLOW` says.)
  let off = clamp(abs(g.calm) / max(line.x, 1e-6), 0.0, 1.0);
  let look = ease(LINE_LOOK_PX.x * e, LINE_LOOK_PX.y * e, 2.0 * line.x);
  let afar = vec2f(1.0 - LINE_SHALLOW * look * off * off, look * ease(LINE_EDGE, 1.0, off));
  out.water = mix(afar, vec2f(out.wet.depth / (BED_MIDDLE * ch.mean), 1.0 - ease(0.0, EDGE_M, (1.0 - abs(x)) * ch.water)), resolved);"""),
])

edit('web/shaders/lib/stone.wgsl', [
 ("""const STONE_TONE = 1.15; const STONE_TONES = 0.18; const STONE_BANDS = 0.12; const STONE_BAND_M = 0.35;""",
  """const STONE_TONE = 1.15; const STONE_TONES = 0.18; const STONE_BANDS = 0.12; const STONE_BAND_M = 0.35; const STONE_BAND_LEAST = 0.6;"""),
 ("""`STONE_TONES`) with the bands of the layers it broke from (`STONE_BANDS`).""", """`STONE_TONES`) with the bands of the layers it broke from (`STONE_BANDS`, every `STONE_BAND_M`, no closer than
// `STONE_BAND_LEAST` of its radius: a large block shows a few beds, not a hatching)."""),
 ("""sin(dot(v, dir) * r / STONE_BAND_M * 2.0 * PI""", """sin(dot(v, dir) * r / max(STONE_BAND_M, STONE_BAND_LEAST * r) * 2.0 * PI"""),
])
edit('web/shaders/lib/crag.wgsl', [("const SCREE_STONE_M = 3.6;", "const SCREE_STONE_M = 3.0;")])
