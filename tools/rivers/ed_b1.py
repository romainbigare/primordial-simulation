def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/rock.wgsl', [
 ("""// of that on the inside
// (Leopold and Wolman 1960); between bends it is a riffle, `RIFFLE_WIDE` wider and shallower, across the whole bed.
const BEND_FULL = 0.4; const POOL_NARROW = 0.2; const RIFFLE_WIDE = 0.3;""", "") if False else ("""const BEND_FULL = 0.4; const POOL_NARROW = 0.2; const RIFFLE_WIDE = 0.3;""", """const BEND_FULL = 0.4; const POOL_NARROW = 0.2; const RIFFLE_WIDE = 0.3;
// In a full bend the pool's deepest lies toward the outer bank: across the water the bed's parabola leans by this much
// (its depth times 1 + `POOL_SKEW` × the share of the way to the outer edge).
const POOL_SKEW = 0.8;"""),
 ("""// flow through a narrower or a wider water); and how much of a bend the place is, 0..1.
struct Channel { bed: f32, water: f32, middle: f32, mean: f32, deep: f32, bend: f32 }
fn channel_at(g: Tiled, q: f32) -> Channel {
  if (q <= 0.0) { return Channel(0.0, 0.0, 0.0, 0.0, 1.0, 0.0); }""", """// flow through a narrower or a wider water); how much of a bend the place is, 0..1; and the side its outer bank lies
// on (the sign of `Tiled.sd` there; 1 where the line is straight).
struct Channel { bed: f32, water: f32, middle: f32, mean: f32, deep: f32, bend: f32, outer: f32 }
fn channel_at(g: Tiled, q: f32) -> Channel {
  if (q <= 0.0) { return Channel(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0); }"""),
 ("""  let outer = -sign(g.curve);
  var bed = law * vary;""", """  let outer = select(1.0, -1.0, g.curve > 0.0);
  var bed = law * vary;"""),
 ("""  return Channel(bed, water, middle, land.channel.z * pow(q, land.channel.w), 1.0 / wide, bend);
}

// How far the point of ground `g` lies across the water of its channel `c`: 0 at its middle, 1 at its edge and past.
fn across_water(g: Tiled, c: Channel) -> f32 { return min(abs(g.sd - c.middle) / max(c.water, 1e-6), 1.0); }

// The depth of the bed of channel `c` under its water's surface at the point of ground `g`, m: a parabola across the
// water, as deep at its middle as its mean depth makes it (`BED_MIDDLE`), so the water is deepest at its middle and
// its banks catch the light.
fn depth_in(g: Tiled, c: Channel) -> f32 {
  let x = across_water(g, c);
  return BED_MIDDLE * c.mean * c.deep * (1.0 - x * x);
}""", """  return Channel(bed, water, middle, land.channel.z * pow(q, land.channel.w), 1.0 / wide, bend, outer);
}

// The depth of the bed of channel `c` under its water's surface, m, the share `x` of the way across the water from its
// middle to its outer edge (-1 at its inner edge): a parabola across the water, as deep at its middle as its mean
// depth makes it (`BED_MIDDLE`), leaning to the outer bank in a bend (`POOL_SKEW`), so the water is deepest in its
// thread and its banks catch the light.
fn depth_across(c: Channel, x: f32) -> f32 {
  return BED_MIDDLE * c.mean * c.deep * (1.0 - x * x) * (1.0 + POOL_SKEW * c.bend * x);
}

// The depth of the bed of channel `c` under its water's surface at the point of ground `g`, m (`depth_across`).
fn depth_in(g: Tiled, c: Channel) -> f32 {
  return depth_across(c, clamp((g.sd - c.middle) * c.outer / max(c.water, 1e-6), -1.0, 1.0));
}"""),
])

P = 'web/shaders/land.wgsl'
t = open(P).read()
i = t.index("  var out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), vec2f(0.0));")
j = t.index("// A stream's bed sends back this share of the light soaked bare ground does")
new = """  let out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), 0.0, vec4f(0.0));
  if (p.depth > 0.0 || ch.water <= 0.0) { return out; }
  return river_spots(out, shore_over, g, ch, q);
}

// The spots `s` of a point with its channel `ch` of discharge `q` on ground `g`, the ground `shore_over` m over its
// water: the bed (rock.wgsl `banks_of`) and this month's water in it (lib/river.wgsl `reach_at`), apart where the bed
// takes a few pixels and is wider than the line; the bars' look, the banks' light and the water's depth and signs of
// flow.
fn river_spots(s: Spots, shore_over: f32, g: Tiled, ch: Channel, q: f32) -> Spots {
  var out = s;
  let at = s.dry.at;
  let e = s.dry.e;
  out.dry.over = shore_over;
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * LINE_PX * css * min(pow(q / shows, LINE_GROW), LINE_MOST) * ease(shows / LINE_FADE, shows, q);
  let open = ease(BED_PX * e, 2.0 * BED_PX * e, banks_of(q)) * (1.0 - ease(banks_of(q), 2.0 * banks_of(q), line));
  let stone = stone_of(g.channels.xy);
  let r = reach_at(at, e, g, ch, line, open, stone);
  out.share = r.share;
  out.bars = r.bars;
  // The bars' colour: the bed's make washed, sand on the gentlest reaches; the bed seen through the water is the same.
  let washed = min(bed_make_at(at) * BAR_LIFT, vec3f(1.0));
  let gravel = mix(BEACH_SAND, washed, ease(0.0, BAR_SAND_STONE, stone));
  out.bar = gravel * bar_tone(at, e, g.along, r);
  out.bar_over = BAR_RISE * r.edge;
  out.wet.ground = gravel;
  // The banks' light falls on the ground beside the bed, on its bars and on the water under the bank.
  let banks = mix(1.0, bank_light(e, g, ch, r), open);
  out.dry.shade *= banks;
  // Where the bed is drawn apart from the water and the water takes a few pixels, its depth shows: its pools and
  // riffles and its bed's shape across it; else its mean depth.
  let resolved = ease(e, 2.0 * e, ch.water) * open;
  let x = r.across;
  out.wet.depth = mix(ch.mean, depth_across(ch, x), resolved);
  out.water = vec4f(flow_signs(e, g.along, g.sd, r.steep, x), out.wet.depth / (BED_MIDDLE * ch.mean), resolved * (1.0 - ease(0.0, EDGE_M, (1.0 - abs(x)) * ch.water)));
  out.wet.sand = 0.0;
  out.wet.cover = vec4f(0.0);
  // The bed's slope across the water, d(depth)/d(distance from its middle) along that distance's own slope; where the
  // water is narrower than a pixel its banks do not show.
  let from_middle = g.d_slope * sign(g.sd) * sign(g.sd - ch.middle);
  let bank = 2.0 * BED_MIDDLE * ch.mean * ch.deep * abs(x) / ch.water * from_middle * select(0.0, 1.0, abs(x) < 1.0) * resolved;
  out.wet.slope = length(g.slope + bank) * earth.look.x;
  out.wet.shade = hill_shade(g.slope + bank, EXAGGERATE * earth.look.x) * banks;
  return out;
}

"""
t = t[:i] + new + t[j:]
open(P, 'w').write(t)
edit(P, [
 ("""// of the pixel standing liquid covers (`pool`): the pixel's footprint across the line where the depth crosses `FILM_M`,
// so a shore is a smooth line at any zoom, not the pixels' steps.
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32, snow_cells: f32, bar: vec3f, flow: vec2f }""",
  """// of the pixel standing liquid covers (`pool`): the pixel's footprint across the line where the depth crosses `FILM_M`,
// so a shore is a smooth line at any zoom, not the pixels' steps; the bars' colour and their height over the water, m;
// and of the channel's water (`water`): the signs of its flow (lib/river.wgsl `flow_signs`), its depth over the depth
// the channel law gives its middle, and how near its edge the point lies, 0..1.
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32, snow_cells: f32, bar: vec3f, bar_over: f32, water: vec4f }"""),
 ("""// A channel's bars are its bed's make washed (`BAR_LIFT` times as light), and sand where its flow moves no stones
// bigger than `BAR_SAND_STONE` m: gravel on most reaches, sand and silt on the gentlest.
const BAR_LIFT = 1.5; const BAR_SAND_STONE = 0.03;""", """// A channel's bars are its bed's make washed (`BAR_LIFT` times as light), and sand where its flow moves no stones
// bigger than `BAR_SAND_STONE` m: gravel on most reaches, sand and silt on the gentlest. A bar rises from the water's
// edge at `BAR_RISE` (rise over run), which is how far up it the water's fringe wets it.
const BAR_LIFT = 1.5; const BAR_SAND_STONE = 0.03; const BAR_RISE = 0.5;
// The water within `EDGE_M` of its edge is lighter by up to `EDGE_LIGHT`: the wash at the bank, where the bed's last
// centimetres show.
const EDGE_M = 0.35; const EDGE_LIGHT = 0.25;"""),
 ("""// A streak of the flow reflects this share of the sky's light more than the water about it (a line of small ripples
// where the current shears).
const REFLECT_STREAK = 0.25;""", """// A streak of the flow reflects this share of the sky's light more than the water about it (a line of small ripples
// where the current shears).
const REFLECT_STREAK = 0.25;
// In the style the bed under running water shows this share of its light: its cobbles read through the shallows.
const BED_SHOWN = 0.8;"""),
 ("""fn river_on(p: Spot, g: Ground, flow: vec2f) -> Shaded {
  let lit = light(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), p.slope);
  let bed = BED_FILM * soil(g, 1.0, lit);
  let ripple = ripples(p.at, p.e) * (1.0 - p.snow);
  // In the style a channel's water is coloured as deeper than it is (style.wgsl `river_seen`), lighter over its banks.
  var water = seen_through(bed, p.depth, ripple, RUNNING_ATTENUATION, DEEP_WATER);
  if (styled()) { water = water_painted(bed, river_seen(p.depth, p.e), ripple); }
  // The signs of its flow (lib/river.wgsl `flow_signs`): streaks catch the sky's light, white water over its stones.
  water = mix(water + flow.x * REFLECT_STREAK * SKY, FOAM_RGB, flow.y);""", """fn river_on(p: Spot, g: Ground, w: vec4f) -> Shaded {
  let lit = light(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), p.slope);
  let soaked_bed = soil(g, 1.0, lit);
  let ripple = ripples(p.at, p.e) * (1.0 - p.snow);
  var water = seen_through(BED_FILM * soaked_bed, p.depth, ripple, RUNNING_ATTENUATION, DEEP_WATER);
  // In the style a channel's water is coloured by its bed's shape (style.wgsl `river_seen`): light shallows over the
  // bed's stones, a dark deep thread, a light wash at its edge.
  if (styled()) { water = water_painted(BED_SHOWN * soaked_bed, river_seen(p.depth, w.z), ripple) * (1.0 + EDGE_LIGHT * w.w); }
  // The signs of its flow (lib/river.wgsl `flow_signs`): streaks catch the sky's light, white water over its stones.
  water = mix(water + w.x * REFLECT_STREAK * SKY, FOAM_RGB, w.y);"""),
 ("""      p.cover = vec4f(0.0);
      p.over = spots.wet.depth;""", """      p.cover = vec4f(0.0);
      p.over = spots.bar_over;"""),
 ("""    if (running) { o = river_on(p, g, spots.flow); } else { o = colour_on(p, g); }""", """    if (running) { o = river_on(p, g, spots.water); } else { o = colour_on(p, g); }"""),
 ("""// bars of a channel's bed, washed gravel, bare, as high over the water as it is deep (`LAYER_BARS`);""", """// bars of a channel's bed, washed gravel, bare, rising from the water's edge (`LAYER_BARS`);"""),
])

edit('web/shaders/lib/style.wgsl', [
 ("""// shore: as if shallower where it is less than a rim's depth deep (a lake's `LAKE_RIM_M`, a channel's `RIVER_RIM_M`:
// its banks), the shallower the nearer the shore (its square): a band as wide as the bed's slope makes it, wide where
// the bed is gentle and no narrower than `LAKE_RIM_PX` pixels where it is steep, so every shore has its light shallows.
const LAKE_SCALE = 1.5; const LAKE_SHALLOWEST_M = 5.0; const LAKE_DEEPEST_M = 45.0;
const LAKE_RIM_M = 0.3; const RIVER_RIM_M = 0.1; const LAKE_RIM_PX = 2.5;""", """// shore: as if shallower where it is less than a rim's depth deep (`LAKE_RIM_M`), the shallower the nearer the shore
// (its square): a band as wide as the bed's slope makes it, wide where the bed is gentle and no narrower than
// `LAKE_RIM_PX` pixels where it is steep, so every shore has its light shallows.
const LAKE_SCALE = 1.5; const LAKE_SHALLOWEST_M = 5.0; const LAKE_DEEPEST_M = 45.0;
const LAKE_RIM_M = 0.3; const LAKE_RIM_PX = 2.5;"""),
 ("""// A channel is a thin line of water decimetres deep: coloured by that depth it shows its bed and reads as a grey line
// of gravel from afar. A map draws a river as water, so it is coloured as a lake of its depth and `RIVER_DEEPER_M` more,
// lighter over its banks (`RIVER_RIM_M`). Close, where a pixel takes under `RIVER_TRUE_E` m (the stones of its bed show),
// it is coloured by its own depth, so its bed shows through its shallows and its deep thread is darker.
const RIVER_DEEPER_M = 5.0; const RIVER_TRUE_E = vec2f(0.05, 0.4);
// The depth a channel's water `depth` m deep is coloured as, seen at `e` m a pixel.
fn river_seen(depth: f32, e: f32) -> f32 {
  let rim = ease(0.0, RIVER_RIM_M, depth);
  return lake_seen(depth, 0.0, e, RIVER_RIM_M) + RIVER_DEEPER_M * rim * rim * ease(RIVER_TRUE_E.x, RIVER_TRUE_E.y, e);
}""", """// A channel is a thin line of water decimetres deep: coloured by that depth alone it shows only its bed, a grey line
// of gravel. A map draws a river as water, and its bed's shape is what reads: it is coloured as its own depth as a
// lake's (`LAKE_SCALE`) and `RIVER_SEEN_M` more where it is as deep as the channel law makes its middle, that share
// (`rel`) to the power `RIVER_SEEN_POW`: the shallows stay light over their bed, the deep thread is dark.
const RIVER_SEEN_M = 14.0; const RIVER_SEEN_POW = 1.6;
// The depth a channel's water `depth` m deep is coloured as, `rel` of the depth of its middle by the channel law.
fn river_seen(depth: f32, rel: f32) -> f32 { return LAKE_SCALE * depth + RIVER_SEEN_M * pow(max(rel, 0.0), RIVER_SEEN_POW); }"""),
])
