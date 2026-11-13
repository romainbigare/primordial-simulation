def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/river.wgsl', [
 ("""// A channel widens into the standing liquid it runs into: its bed and its water `MOUTH_FLARE` wider (and its water
// that much shallower) where the ground lies at that liquid's level, none of it from `MOUTH_OVER_M` over the level.
// Where it moves stones of `BAR_SAND_STONE` or more it lays a fan under the liquid: sand within `FAN_WIDTHS` of its
// bed's half widths of its line, under liquid shallower than `FAN_DEPTH_M`.
const MOUTH_FLARE = 1.5; const MOUTH_OVER_M = 1.5; const FAN_WIDTHS = 5.0; const FAN_DEPTH_M = 1.5;
""", ""),
 ("""const BAR_LIFT = 1.7; const BAR_PALE = 0.35; const BAR_SAND_STONE = 0.03; const BAR_RISE = 0.6;""", """const BAR_LIFT = 1.7; const BAR_PALE = 0.35; const BAR_RISE = 0.6;
// Snow bridges a channel's water narrower than the snow is deep, as it buries a plant lower than that (draw.wgsl
// `above_snow`): a rill under a snowfield does not show."""),
 ("""  out.share = mix(r.share, line_over(g, line.x, e), line.y);
  out.bars = r.bars * (1.0 - line.y);
  let own = (1.0 - line.y) * r.share / max(out.share, 1e-6);""",
  """  let open_sky = above_snow(s.dry.snow * s.dry.snow_m, 2.0 * max(ch.water, line.x));
  out.share = mix(r.share, line_over(g, line.x, e), line.y) * open_sky;
  out.bars = r.bars * (1.0 - line.y);
  let own = (1.0 - line.y) * r.share * open_sky / max(out.share, 1e-6);"""),
])

edit('web/shaders/lib/rock.wgsl', [
 ("""fn channel_at(g: Tiled, q: f32) -> Channel {
  if (q <= 0.0) { return Channel(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0); }
  let law = banks_of(q);""", """fn channel_at(g: Tiled, q: f32) -> Channel {
  if (q <= 0.0) { return Channel(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0); }
  let flare = 1.0 + MOUTH_FLARE * (1.0 - ease(0.0, MOUTH_OVER_M, over_ponds(g, 0.0)));
  let law = banks_of(q) * flare;"""),
 ("""  return Channel(bed, water, middle, land.channel.z * pow(q, land.channel.w), 1.0 / wide, bend, outer);""",
  """  return Channel(bed, water, middle, land.channel.z * pow(q, land.channel.w) / flare, 1.0 / wide, bend, outer);
}

// The share of a channel's fan at a point of ground `g` under standing liquid `depth` m deep, 0..1 (`FAN_WIDTHS`): the
// channel is `c`.
fn fan_at(g: Tiled, c: Channel, depth: f32) -> f32 {
  if (c.bed <= 0.0 || depth <= 0.0 || depth >= FAN_DEPTH_M || g.d >= FAN_WIDTHS * c.bed) { return 0.0; }
  return ease(0.0, BAR_SAND_STONE, stone_of(sample_at(g))) * (1.0 - ease(c.bed, FAN_WIDTHS * c.bed, g.d)) * (1.0 - ease(0.0, FAN_DEPTH_M, depth));"""),
 ("""// A channel at a point of ground `g`, of this month's discharge `q`, m³/s: the half width of its bed on the point's""",
  """// A channel widens into the standing liquid it runs into: its bed and its water `MOUTH_FLARE` wider (and its water
// that much shallower) where the ground lies at that liquid's level, none of it from `MOUTH_OVER_M` over the level
// (land.wgsl `over_ponds`). Where it moves stones of `BAR_SAND_STONE` m or more (a channel that moves less carries
// sand and silt alone) it lays a fan under the liquid: sand within `FAN_WIDTHS` of its bed's half widths of its line,
// under liquid shallower than `FAN_DEPTH_M` (`fan_at`).
const MOUTH_FLARE = 1.5; const MOUTH_OVER_M = 1.5; const FAN_WIDTHS = 5.0; const FAN_DEPTH_M = 1.5;
const BAR_SAND_STONE = 0.03;

// A channel at a point of ground `g`, of this month's discharge `q`, m³/s: the half width of its bed on the point's"""),
])

edit('web/shaders/lib/land.wgsl', [
 ("""    p.depth = max(g.water.x - (g.h - cut), 0.0) * g.water.y;
    p.over = min(max(g.h - cut - g.water.x, 0.0) / g.water.y, NO_LIQUID_M);
  } else {
    p.depth = 0.0;
    p.over = NO_LIQUID_M;
  }""", """    p.depth = max(g.water.x - (g.h - cut), 0.0) * g.water.y;
  } else {
    p.depth = 0.0;
  }
  p.over = over_ponds(g, cut);"""),
 ("""// The state at `at`, m from the map's north-west corner, on ground `g` whose bed a channel cuts `cut` m deep there""",
  """// The height of ground `g`, cut `cut` m deep, over the level of the ponds about it, m (`NO_LIQUID_M`: none about it):
// it counts the more the less of the point is theirs.
fn over_ponds(g: Tiled, cut: f32) -> f32 {
  if (g.water.x <= -NO_LIQUID_M || g.water.y <= 0.0) { return NO_LIQUID_M; }
  return min(max(g.h - cut - g.water.x, 0.0) / g.water.y, NO_LIQUID_M);
}

// The state at `at`, m from the map's north-west corner, on ground `g` whose bed a channel cuts `cut` m deep there"""),
 ("""// its bed too, and its floods over the whole of it (`channel_at`): no plant roots in a channel's bed.""",
  """// its bed too, and its floods over the whole of it (`channel_at`): no plant roots in a channel's bed, widened at its
// mouth, nor on the fan it lays under standing liquid (rock.wgsl `fan_at`)."""),
 ("""  p.depth = max(max(p.depth, cut), select(0.0, ROOTS_M, g.d < c.bed));""", """  p.depth = max(max(p.depth, cut), ROOTS_M * max(select(0.0, 1.0, g.d < c.bed), fan_at(g, c, p.depth)));"""),
])

edit('web/shaders/land.wgsl', [
 ("""  var ch = channel_at(g, q);
  let st = state_on(at, g, cs, depth_in(g, ch));
  let flare = 1.0 + MOUTH_FLARE * (1.0 - ease(0.0, MOUTH_OVER_M, st.over));
  ch.bed *= flare;
  ch.water *= flare;
  ch.mean /= flare;
""", """  let ch = channel_at(g, q);
  let st = state_on(at, g, cs, depth_in(g, ch));
"""),
])
t = open('web/shaders/land.wgsl').read()
i = t.index("  if (ch.bed > 0.0 && p.depth > 0.0) { p.sand = max(p.sand, ease(0.0, BAR_SAND_STONE, stone_of(reach))")
j = t.index("\n", i)
t = t[:i] + "  p.sand = max(p.sand, fan_at(g, ch, p.depth));" + t[j:]
open('web/shaders/land.wgsl', 'w').write(t)
