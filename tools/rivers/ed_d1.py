import shutil
def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

S = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv/'
f = open(S + 'flow.wgsl').read()
f = f.replace("  let lane = LANE_M * c.water / (LANE_M * lanes);\n  let shown = shows_at(2.0 * lane, e);", "  let shown = shows_at(2.0 * c.water / lanes, e);")
f = f.replace("""// The signs of the flow at a point: the light its streaks add to the surface's, the share of white water, and how
// fast and shallow the water runs there, 0..1.
struct Flow { streak: f32, white: f32, fast: f32 }

""", "")
f = f.replace("""// its water from its middle to its outer edge (-1 at its inner edge), on the channel `c` (rock.wgsl `channel_at`) of a
// reach of steepness `steep`, 0..1.
fn flow_signs(e: f32, along: f32, x: f32, c: Channel, steep: f32) -> Flow {
  var f = Flow(0.0, 0.0, steep);""", """// its water from its middle to its outer edge (-1 at its inner edge), on the channel `c` (rock.wgsl `channel_at`) of a
// reach of steepness `steep`, 0..1: the light its streaks add to the surface's, the share of white water, and how
// fast and shallow the water runs there, 0..1.
fn flow_signs(e: f32, along: f32, x: f32, c: Channel, steep: f32) -> vec3f {
  var f = vec3f(0.0, 0.0, steep);""")
for a, b in [("  f.streak = ", "  f.x = "), ("  f.white = FLECK", "  f.y = FLECK"), ("    f.white = max(f.white, ", "    f.y = max(f.y, ")]:
    assert f.count(a) == 1, a
    f = f.replace(a, b)
open('web/shaders/lib/flow.wgsl', 'w').write(f)

R = 'web/shaders/lib/river.wgsl'
t = open(R).read()
i = t.index("// The streaks of the flow: lines along the channel `STREAK_W_M` apart")
open(R, 'w').write(t[:i].rstrip('\n') + '\n')

edit('web/shaders/land.wgsl', [
 ("relief.wgsl river.wgsl cover_field.wgsl", "relief.wgsl river.wgsl flow.wgsl cover_field.wgsl"),
 ("""// so a shore is a smooth line at any zoom, not the pixels' steps; the bars' colour and their height over the water, m;
// and of the channel's water (`water`): the signs of its flow (lib/river.wgsl `flow_signs`), its depth over the depth
// the channel law gives its middle, and how near its edge the point lies, 0..1.
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32, snow_cells: f32, bar: vec3f, bar_over: f32, water: vec4f }""",
  """// so a shore is a smooth line at any zoom, not the pixels' steps; the bars' colour and their height over the water, m;
// and of the channel's water: the signs of its flow (lib/flow.wgsl `flow_signs`), and (`water`) its depth over the
// depth the channel law gives its middle and how near its edge the point lies, 0..1.
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32, snow_cells: f32, bar: vec3f, bar_over: f32, flow: vec3f, water: vec2f }"""),
 ("""  let out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), 0.0, vec4f(0.0));""", """  let out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), 0.0, vec3f(0.0), vec2f(0.0));"""),
 ("""  out.water = vec4f(flow_signs(e, g.along, g.sd, r.steep, x), out.wet.depth / (BED_MIDDLE * ch.mean), resolved * (1.0 - ease(0.0, EDGE_M, (1.0 - abs(x)) * ch.water)));""",
  """  out.flow = resolved * flow_signs(e, g.along, x, ch, r.steep);
  out.water = vec2f(out.wet.depth / (BED_MIDDLE * ch.mean), resolved * (1.0 - ease(0.0, EDGE_M, (1.0 - abs(x)) * ch.water)));"""),
 ("""// A streak of the flow reflects this share of the sky's light more than the water about it (a line of small ripples
// where the current shears).
const REFLECT_STREAK = 0.25;""", """// A streak of the flow reflects this share of the sky's light more than the water about it (a line of small ripples
// where the current shears). Fast shallow water is `FAST` of the way to `FAST_WATER`, the green-blue of water full of
// air over a pale bed.
const REFLECT_STREAK = 0.25; const FAST = 0.4; const FAST_WATER = vec3f(0.10, 0.36, 0.34);"""),
 ("""fn river_on(p: Spot, g: Ground, w: vec4f) -> Shaded {""", """fn river_on(p: Spot, g: Ground, flow: vec3f, w: vec2f) -> Shaded {"""),
 ("""  if (styled()) { water = water_painted(BED_SHOWN * soaked_bed, river_seen(p.depth, w.z), ripple) * (1.0 + EDGE_LIGHT * w.w); }
  // The signs of its flow (lib/river.wgsl `flow_signs`): streaks catch the sky's light, white water over its stones.
  water = mix(water + w.x * REFLECT_STREAK * SKY, FOAM_RGB, w.y);""", """  if (styled()) { water = mix(water_painted(BED_SHOWN * soaked_bed, river_seen(p.depth, w.x), ripple) * (1.0 + EDGE_LIGHT * w.y), FAST_WATER, FAST * flow.z); }
  // The signs of its flow (lib/flow.wgsl `flow_signs`): streaks catch the sky's light, white water breaks over its
  // steps.
  water = mix(water + flow.x * REFLECT_STREAK * SKY, FOAM_RGB, flow.y);"""),
 ("""    if (running) { o = river_on(p, g, spots.water); } else { o = colour_on(p, g); }""", """    if (running) { o = river_on(p, g, spots.flow, spots.water); } else { o = colour_on(p, g); }"""),
])
edit('web/shaders/lib/land.wgsl', [
 ("""  // The length a texel's place along its channel is kept modulo, m (tape/src/ground/channels.rs `ALONG_WRAP_M`), and
  // how many times the mean flow the flood that shapes a channel's bed passes (tape/src/ground/bends.rs `FLOOD`).
  reach: vec4f,""", """  // The length a texel's place along its channel is kept modulo, m (tape/src/ground/channels.rs `ALONG_WRAP_M`), how
  // many times the mean flow the flood that shapes a channel's bed passes (tape/src/ground/bends.rs `FLOOD`), and how
  // far the signs of the flow have slid down the slowest water, m (web/src/land.ts `slid`).
  reach: vec4f,"""),
])
