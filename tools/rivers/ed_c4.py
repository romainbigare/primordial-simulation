def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

R = 'web/shaders/lib/rock.wgsl'
t = open(R).read()
i = t.index("// What runs through the channel of sample `s` beside the point `at` (m from the map's north-west corner), m³/s: a")
j = t.index("// A channel's bed is as wide as the flow that shapes it, its floods")
new = """// The sample next along the channel of sample `s` from the point `at` (m from the map's north-west corner), and the
// share of the way from `s`'s centre to that one's the point lies (0: none): a channel's numbers grow evenly from one
// sample's centre to the next one's along it (`links_of`), so its width has no step where two samples meet; a
// tributary keeps its own to its join.
struct Next { s: vec2i, share: f32 }
fn next_along(s: vec2i, at: vec2f) -> Next {
  var next = Next(s, 0.0);
  if (s.x < 0) { return next; }
  let links = links_of(s);
  let p = at / (land.map.x * exp2(land.map.w - land.samples.z)) - (vec2f(s) + 0.5);
  for (var k = 0; k < 2; k++) {
    // Down the channel (not into the channel a tributary joins), then up its main way.
    let code = select(links / LINK_UP, links % LINK_UP, k == 0);
    if (code <= 0 || code > 8) { continue; }
    let n = select(code, code - 1, code <= 4);
    let step = vec2i(n % 3, n / 3) - 1;
    let a = min(dot(p, vec2f(step)) / f32(dot(step, step)), 1.0);
    if (a > next.share) { next = Next(s + step, a); }
  }
  return next;
}

// The sample of the channel at a point of ground `g`: its nearest texel's channel's, or the channel's of the texel
// across from that where it carries more and that texel lies in its water, so a pixel in the water of a channel a
// tributary joins is that channel's.
fn sample_at(g: Tiled) -> vec2i {
  let across = discharge_of(g.channels.zw);
  if (across > discharge_of(g.channels.xy) && g.ends.y < 0.5 * land.channel.x * pow(across, land.channel.y)) { return g.channels.zw; }
  return g.channels.xy;
}

// What runs through the channel at a point `at` of ground `g`, m³/s (`sample_at`, `next_along`).
fn discharge_at(g: Tiled, at: vec2f) -> f32 {
  let s = sample_at(g);
  let n = next_along(s, at);
  return mix(discharge_of(s), discharge_of(n.s), n.share);
}

"""
t = t[:i] + new + t[j:]
open(R, 'w').write(t)

edit('web/shaders/land.wgsl', [
 ("""  let q = discharge_at(g, at);
  let ch = channel_at(g, q);""", """  // Its channel: what runs through it and the largest stone it moves, each grown evenly along it (rock.wgsl
  // `next_along`).
  let reach = sample_at(g);
  let next = next_along(reach, at);
  let q = mix(discharge_of(reach), discharge_of(next.s), next.share);
  let ch = channel_at(g, q);"""),
 ("""  p.stone = select(0.0, stone_of(g.channels.xy), ch.bed > 0.0);""", """  p.stone = select(0.0, mix(stone_of(reach), stone_of(next.s), next.share), ch.bed > 0.0);"""),
 ("""  return river_spots(out, shore_over, g, ch, q);
}""", """  return river_spots(out, shore_over, g, ch, q, p.stone);
}"""),
 ("""// The spots `s` of a point with its channel `ch` of discharge `q` on ground `g`, the ground `shore_over` m over its
// water:""", """// The spots `s` of a point with its channel `ch` of discharge `q`, which moves stones of `stone` m, on ground `g`, the
// ground `shore_over` m over its water:"""),
 ("""fn river_spots(s: Spots, shore_over: f32, g: Tiled, ch: Channel, q: f32) -> Spots {""", """fn river_spots(s: Spots, shore_over: f32, g: Tiled, ch: Channel, q: f32, stone: f32) -> Spots {"""),
 ("""  let stone = stone_of(g.channels.xy);
  let r = reach_at(at, e, g, ch, line, open, stone);""", """  let r = reach_at(at, e, g, ch, line, open, stone);"""),
 ("""  out.flow = resolved * flow_signs(e, g.along, x, ch, r.steep);""", """  out.flow = resolved * flow_signs(e, g.along, x, ch, r.steep);
  // Down the flow: square to the distance's slope, the way the place along the channel grows.
  let square = vec2f(-g.d_slope.y, g.d_slope.x);
  out.down = square * sign(dot(square, vec2f(dpdx(g.along), dpdy(g.along))));"""),
 ("""// depth the channel law gives its middle and how near its edge the point lies, 0..1.
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32, snow_cells: f32, bar: vec3f, bar_over: f32, flow: vec3f, water: vec2f }""",
  """// depth the channel law gives its middle and how near its edge the point lies, 0..1; and the way it runs (`down`, a
// unit vector east and south).
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32, snow_cells: f32, bar: vec3f, bar_over: f32, flow: vec3f, water: vec2f, down: vec2f }"""),
 ("""  let out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), 0.0, vec3f(0.0), vec2f(0.0));""", """  let out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), 0.0, vec3f(0.0), vec2f(0.0), vec2f(0.0));"""),
 ("""const REFLECT_STREAK = 0.25; const FAST = 0.4; const FAST_WATER = vec3f(0.10, 0.36, 0.34);""", """const REFLECT_STREAK = 0.25; const FAST = 0.25; const FAST_WATER = vec3f(0.06, 0.34, 0.36);
// A stone of the bed stands out of the water where it is `STONE_STANDS` of its size taller than the water is deep; the
// water breaks white on it and trails `WAKE` of white water down the flow behind it (stone.wgsl `wake`).
const STONE_STANDS = 0.3; const WAKE = 0.7;"""),
 ("""fn river_on(p: Spot, g: Ground, flow: vec3f, w: vec2f) -> Shaded {""", """fn river_on(p: Spot, g: Ground, flow: vec3f, w: vec2f, down: vec2f) -> Shaded {"""),
 ("""  water = mix(water + flow.x * REFLECT_STREAK * SKY, FOAM_RGB, flow.y);""", """  water = mix(water + flow.x * REFLECT_STREAK * SKY, FOAM_RGB, flow.y);
  // The stones that stand out of it, with their wakes.
  if (styled() && p.stony > 0.0 && !unseen(STONE_MOST_M / STONE_PX)) {
    let s = Stony(p.stony, max(STONE_SEEN * p.stone, COBBLE_M), p.worn, BED_SUNK, 0.0);
    let least = p.depth / STONE_STANDS;
    water = mix(water, FOAM_RGB, WAKE * wake(p.at, p.e, s, least, down, 61 + 8));
    let st = stones(p.at, p.e, s, least, p.bed, 61 + 8);
    water = mix(water, st.rgb / max(st.a, 1e-4), st.a);
  }"""),
 ("""    if (running) { o = river_on(p, g, spots.flow, spots.water); } else { o = colour_on(p, g); }""", """    if (running) { o = river_on(p, g, spots.flow, spots.water, spots.down); } else { o = colour_on(p, g); }"""),
])

edit('web/shaders/lib/colour.wgsl', [
 ("""      let st = stones(p.at, p.e, s, p.bed, 61 + 8 * k);""", """      let st = stones(p.at, p.e, s, 0.0, p.bed, 61 + 8 * k);"""),
])

S = 'web/shaders/lib/stone.wgsl'
t = open(S).read()
i = t.index("// The stones `s` at `at`, seen at `e` m a pixel, of bed `bed` (linear, unlit), stream `k`")
new = """// A stone of a lattice: whether its cell holds one, its random word, its radius, m, and the point from its centre, m.
struct Placed { held: bool, x: u32, r: f32, v: vec2f }

// The stone of size `size` m (those of its octave `o`: `density` of the cells hold one) of lattice `l` whose cell
// holds the point `at`, stream `k` of the world's random numbers. The second lattice lies turned and half a cell off
// the first, so no rows of stones show; a stone lies anywhere its cell holds it whole, with its shadow.
fn placed(at: vec2f, size: f32, density: f32, o: i32, l: i32, k: i32) -> Placed {
  let side = STONE_GAP * size;
  let q = select(at, vec2f(0.8 * at.x + 0.6 * at.y, 0.8 * at.y - 0.6 * at.x), l == 1) / side + f32(l) * 0.5;
  let c = floor(q);
  let x = lowbias(vec2i(c), k + 2 * o + l);
  let r = 0.5 * size * (1.0 + STONE_SIZES * (f32((x >> 20u) & 1023u) / 512.0 - 1.0));
  let room = 0.5 - (1.0 + STONE_SHADOW_REACH) * r / side;
  let centre = c + 0.5 + room * (vec2f(f32((x >> 10u) & 31u), f32((x >> 15u) & 31u)) / 15.5 - 1.0);
  let v = (q - centre) * side;
  // The point from the centre back in the map's axes.
  return Placed(f32(x & 1023u) / 1024.0 < density, x, r, select(v, vec2f(0.8 * v.x - 0.6 * v.y, 0.6 * v.x + 0.8 * v.y), l == 1));
}

// The stones `s` at least `least` m large at `at`, seen at `e` m a pixel, of bed `bed` (linear, unlit), stream `k` of
// the world's random numbers: their colour times their coverage (with the shadow they throw as black), and that
// coverage.
fn stones(at: vec2f, e: f32, s: Stony, least: f32, bed: vec3f, k: i32) -> vec4f {
  var out = vec4f(0.0);
  if (s.density <= 0.0 || s.size <= 0.0) { return out; }
  let tone = bed * STONE_TONE;
  var size = s.size / pow(STONE_STEP, f32(STONE_OCTAVES - 1));
  // The smallest first, the larger over them.
  for (var o = STONE_OCTAVES - 1; o >= 0; o--) {
    let density = s.density * select(1.0, STONE_FEW, o == 0);
    // Under the snow that covers its ground, a small stone does not show.
    let stands = (1.0 - s.snow * (1.0 - ease(SNOW_BURIES_M, 2.0 * SNOW_BURIES_M, size))) * ease(least, 1.5 * least, size);
    // Smaller than a few pixels: the share of the ground they cover, as a tone.
    var near = 0.0;
    if (!unseen(STONE_MOST_M / STONE_PX)) { near = ease(STONE_PX * e, 2.0 * STONE_PX * e, size); }
    let mean = 2.0 * density * stands * PI * 0.25 * 0.7 / (STONE_GAP * STONE_GAP) * (1.0 - near);
    out = mix(out, vec4f(tone, 1.0), mean);
    if (near > 0.0 && stands > 0.0) {
      for (var l = 0; l < 2; l++) {
        let p = placed(at, size, density, o, l, k);
        if (!p.held) { continue; }
        let v = p.v / p.r;
        let w = p.x * 2654435761u;
        let st = stone_at(v, e / p.r, w, s.worn, s.sunk);
        // Its shadow on the ground, away from the sun.
        let shade = STONE_SHADOW * (1.0 - smoothstep(0.6, 1.1, length(v + STONE_SHADOW_REACH * LIGHT.xy / length(LIGHT.xy))));
        out = mix(out, vec4f(0.0, 0.0, 0.0, 1.0), shade * near * stands * (1.0 - st.a));
        if (st.a <= 0.0) { continue; }
        // Its own tone, and the bands of its rock across it.
        let own = 1.0 + STONE_TONES * (f32((w >> 8u) & 255u) / 127.5 - 1.0);
        let dir = vec2f(cos(f32(w >> 24u)), sin(f32(w >> 24u)));
        let bands = 1.0 + STONE_BANDS * sin(dot(v, dir) * p.r / STONE_BAND_M * 2.0 * PI + f32((w >> 16u) & 255u));
        let cap = s.snow * smoothstep(SNOW_TOP.x, SNOW_TOP.y, st.flat);
        let rgb = mix(lit_by(tone * own * bands, 0.35 + 0.65 * st.lit), SNOW, cap);
        out = mix(out, vec4f(rgb, 1.0), st.a * near * stands);
      }
    }
    size *= STONE_STEP;
  }
  return out;
}

// A stone that stands in a flow breaks it: white water piles against its face up the flow and trails down the flow
// behind it, `WAKE_LONG` of its size long, spreading by `WAKE_SPREAD` of the way, with calm water right behind it
// (its eddy, `WAKE_EDDY` of its radius).
const WAKE_LONG = 2.2; const WAKE_SPREAD = 0.35; const WAKE_EDDY = 0.8;

// The white water at `at`, seen at `e` m a pixel, about the stones `s` at least `least` m large (`stones`, stream `k`)
// in a flow that runs `down` (a unit vector), 0..1: each stone's the same stone `stones` draws, found from a point up
// the flow.
fn wake(at: vec2f, e: f32, s: Stony, least: f32, down: vec2f, k: i32) -> f32 {
  var white = 0.0;
  var size = s.size;
  for (var o = 0; o < STONE_OCTAVES; o++) {
    let stands = ease(least, 1.5 * least, size) * ease(STONE_PX * e, 2.0 * STONE_PX * e, size);
    if (stands > 0.0) {
      let density = s.density * select(1.0, STONE_FEW, o == 0);
      for (var l = 0; l < 2; l++) {
        // The stone whose cell holds the point half a wake up the flow.
        let back = 0.5 * WAKE_LONG * size * down;
        let p = placed(at - back, size, density, o, l, k);
        if (!p.held) { continue; }
        let v = p.v + back;
        let along = dot(v, down) / p.r;
        let across = abs(dot(v, vec2f(-down.y, down.x))) / p.r;
        // Behind it: two trails at its sides that spread and fade, calm between them at first; before it: a rim.
        let reach = 2.0 * WAKE_LONG;
        let wide = 1.0 + WAKE_SPREAD * max(along, 0.0);
        let trail = (1.0 - smoothstep(0.5 * wide, wide, across)) * smoothstep(WAKE_EDDY * (1.0 - along / reach), wide, across + 0.6 * along) * (1.0 - smoothstep(0.0, reach, along)) * step(0.0, along);
        let rim = (1.0 - smoothstep(1.0, 1.35, length(v) / p.r)) * smoothstep(-0.2, -0.9, along);
        white = max(white, stands * max(trail, rim));
      }
    }
    size /= STONE_STEP;
  }
  return white;
}
"""
open(S, 'w').write(t[:i] + new)

edit('web/shaders/lib/flow.wgsl', [
 ("""// so (step-pool reaches: Montgomery and Buffington 1997): white water starts at a step, on a line that wavers across
// the channel, and fades down the flow over `STEP_FADE` of the way to the next, in streaks; the water between runs
// fast and shallow.""", """// so (step-pool reaches: Montgomery and Buffington 1997): white water starts at a step, on a line that wavers across
// the channel, and fades down the flow over `STEP_FADE` of the way to the next, in streaks; a step is white over part
// of the channel's width only, in patches `STEP_PATCH` steps long; the water between runs fast and shallow."""),
 ("""const STEP_M = 8.0; const STEP_WAVE = 0.45; const STEP_FADE = 0.35; const FOAM_STEEP = 0.9;""", """const STEP_M = 8.0; const STEP_WAVE = 0.7; const STEP_FADE = 0.3; const STEP_PATCH = 2.0; const FOAM_STEEP = 0.9;"""),
 ("""    let own = f32(lowbias(vec2i(((i32(floor(p)) % cells) + cells) % cells, 0), FLOW_STRENGTH) & 1023u) / 1023.0;
    let fall = exp(-fract(p) / STEP_FADE) * smoothstep(0.0, 2.0 * e / STEP_M, fract(p));
    let torn = 0.45 + 0.55 * ease(-0.4, 0.5, slid(along, pace, FLECK_M, 2.0 * x * lanes, FLOW_WHITE));
    f.y = max(f.y, FOAM_STEEP * shown * steep * ease(0.9 - steep, 1.3 - steep, own + 0.3) * fall * torn * sqrt(mid));""",
  """    let own = f32(lowbias(vec2i(((i32(floor(p)) % cells) + cells) % cells, 0), FLOW_STRENGTH) & 1023u) / 1023.0;
    let patch = ease(-0.3, 0.2, noise_reach(along, STEP_PATCH * STEP_M, 1.3 * x + 7.0 * own, FLOW_STRENGTH) + steep - 0.7 + 0.4 * own);
    let fall = exp(-fract(p) / STEP_FADE) * smoothstep(0.0, 2.0 * e / STEP_M, fract(p));
    let torn = 0.3 + 0.7 * ease(-0.4, 0.5, slid(along, pace, FLECK_M, 2.0 * x * lanes, FLOW_WHITE));
    f.y = max(f.y, FOAM_STEEP * shown * steep * patch * fall * torn * sqrt(mid));"""),
])
