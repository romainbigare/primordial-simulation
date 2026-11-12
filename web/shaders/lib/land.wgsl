// The live map at a point (web/src/land.ts): the ground from its tiles (web/src/ground.ts, tape/src/ground), which never
// change, and the state from the cells' table (tape/src/table.rs), blended between the squares' centres; then the laws
// inside a cell applied at the point, with the numbers the core sends: the ground's temperature along its height, the
// liquid that stands under its cells' level, the soil's liquid by the wetness index, a channel as wide as its own
// sample's discharge makes it (the samples' discharge, sent when it changed enough to show).
// Nothing depends on the view, a window or a tile: a place has the same colour after any pan or zoom.

struct Land {
  // Metres a square, squares across and down, the depth of a square.
  map: vec4f,
  // Texels along a tile's side inside its gutter, texels of the gutter, the depth the view is drawn at, and the most
  // steps along each axis by which a texel names its channel's sample (tape/src/ground/channels.rs `code`).
  tiles: vec4f,
  // The time since the tiles started, ms, the time a new tile fades in over, ms, the melting point of the frozen
  // cover, K (0: none), and the discharge per metre a css pixel takes past which a channel shows as a line, m³/s per m.
  time: vec4f,
  // The soil's liquid departs by this per unit of the wetness index, m; ground with this depth of liquid is half wet, m;
  // a living thing that holds the share x of the liquid it needs is x to this power active.
  soil: vec4f,
  // A channel's width and depth: each a factor, m, times its discharge in m³/s to a power.
  channel: vec4f,
  // The terrain's samples across and down, their depth, and the screen's pixels a css pixel.
  samples: vec4f,
}
@group(0) @binding(8) var<uniform> land: Land;
// The tiles kept, a layer each: of each texel the ground's height, m, as the sum of two 16-bit floats (to a millimetre),
// the wetness index, and the distance to its channel's line, m, signed by the side; and that channel's sample, as its
// step from the sample that holds the texel's centre (0: none), and one more than the pond it lies in (0: none).
@group(0) @binding(9) var atlas: texture_2d_array<f32>;
@group(0) @binding(13) var areas: texture_2d_array<f32>;
// Of each tile at the view's depth: the layer drawn there and the layer it fades in over (-1: none).
@group(0) @binding(10) var pages: texture_2d<f32>;
// Of each layer: its tile's depth, column and row, and when it came, ms.
@group(0) @binding(11) var tile_of: texture_2d<f32>;
// The cells' table: `PLANES` planes of a texel a square, one under the other.
@group(0) @binding(12) var table: texture_2d<f32>;
// What runs through each of the terrain's samples, m³/s (tape/src/table.rs `discharge`), a texel a sample.
@group(0) @binding(17) var flows: texture_2d<f32>;

// The table's planes: the terrain's mean height over the cell, m, its ground's temperature, K, how fast its air cools
// with height, K/m, and its ice, m; its standing liquid's level, its lowest ground and the depth that stands, m, and the
// liquid in its pores, m; what its pores hold full, m, and the level of the wetness index its soil's liquid departs
// about; the ground's own colour and its share of the sea; its true colour; the bytes of the layer drawn and of the worn
// paths; and in the last numbers of the true colour's and the layer's planes, the land's ice, m, its ground
// temperature, K, and its mean height, m. (The bed's colour comes with the ground: rock.wgsl `bed_make_at`.)
const GROUND = 0; const LIQUID = 1; const SOIL = 2; const SURFACE = 3; const TRUTH = 4; const VIEW = 5; const PLANES = 6;
// Where no liquid stands near a point, its surface is taken this far below the ground, m.
const NO_LIQUID_M = 1000.0;
// Between texels of one channel its signed distance changes by at most the distance between them (√2 texels across a
// diagonal; a little more for the 16-bit floats): four texels whose signed distances differ by more and change sign
// hold two channels, or the two sides of a line's end, and are blended by their distances alone.
const SAME_LINE_TEXELS = 1.5;
// Half a texel's diagonal, in texels.
const HALF_DIAGONAL = 0.71;
// A channel's bed is a parabola across it, whose mean depth is two thirds of its depth at the middle.
const BED_MIDDLE = 1.5;

// What a point's ground holds: its height, m, its slope east and south, its wetness index, and the distance to its
// channel's line, m, its slope east and south; the samples of the channels of the texel nearest it and of the one
// across from that (-1: none) and the least distances of those texels to their lines, m (from their nearest
// points); and the highest level of the ponds of the four texels about it, m (`-NO_LIQUID_M`: none), and the share of
// the point's weight on those of them in a pond: its liquid stands at that level over the ground drawn, so a shore
// runs smooth between the texels where the ground crosses the level.
struct Tiled { h: f32, slope: vec2f, twi: f32, d: f32, d_slope: vec2f, channels: vec4i, ends: vec2f, water: vec2f }

// The sample texel `n` of the tile of depth `depth` at column and row `tile` names by `code` (tape/src/ground/channels.rs
// `code`): the sample that holds the texel's centre, stepped along each axis; none (-1) for 0.
fn sample_of(n: vec2i, depth: f32, tile: vec2f, code: i32) -> vec2i {
  if (code <= 0) { return vec2i(-1); }
  let most = i32(land.tiles.w);
  let side = 2 * most + 1;
  let texel = vec2i(tile) * i32(land.tiles.x) - i32(land.tiles.y) + n;
  let own = vec2i(floor((vec2f(texel) + 0.5) * exp2(land.samples.z - depth)));
  return own + vec2i((code - 1) % side, (code - 1) / side) - most;
}

// The ground of layer `layer` at `at`, m from the map's north-west corner, blended between the texels' centres; with
// `spline`, its height and slope through them as a Catmull-Rom spline (so light on a slope drawn over many pixels a
// texel shows no creases).
fn read_tile(layer: i32, at: vec2f, spline: bool) -> Tiled {
  let t = textureLoad(tile_of, vec2i(layer, 0), 0);
  let texel = land.map.x / exp2(t.x - land.map.w);
  let q = at / texel - (t.yz * land.tiles.x - land.tiles.y) - 0.5;
  let i = vec2i(floor(q));
  let f = q - floor(q);
  let last = i32(textureDimensions(atlas).x) - 1;
  let a = clamp(i, vec2i(0), vec2i(last));
  let b = clamp(i + 1, vec2i(0), vec2i(last));
  let c00 = textureLoad(atlas, a, layer, 0);
  let c10 = textureLoad(atlas, vec2i(b.x, a.y), layer, 0);
  let c01 = textureLoad(atlas, vec2i(a.x, b.y), layer, 0);
  let c11 = textureLoad(atlas, b, layer, 0);
  let top = mix(c00, c10, f.x);
  let bottom = mix(c01, c11, f.x);
  let o = mix(top, bottom, f.y);
  // The channel's signed distance blended across its line (exact for a straight one), unless the four texels hold more
  // than one channel; its slope, and the distance itself.
  let raw = vec4f(c00.w, c10.w, c01.w, c11.w);
  var sd = raw;
  let lo = min(min(sd.x, sd.y), min(sd.z, sd.w));
  let hi = max(max(sd.x, sd.y), max(sd.z, sd.w));
  if (lo < 0.0 && hi > 0.0 && hi - lo > SAME_LINE_TEXELS * texel) { sd = abs(sd); }
  let sx = mix(sd.x, sd.y, f.x);
  let sy = mix(sd.z, sd.w, f.x);
  let signed = mix(sx, sy, f.y);
  let d_slope = sign(signed) * vec2f(mix(sd.y - sd.x, sd.w - sd.z, f.y), sy - sx) / texel;
  // The channels' samples of the texel nearest the point and of the one across the four from it: where a tributary's
  // line runs into the water of the channel it joins, its texels lie between that channel's (`discharge_at`).
  let a4 = array<vec4f, 4>(textureLoad(areas, a, layer, 0), textureLoad(areas, vec2i(b.x, a.y), layer, 0), textureLoad(areas, vec2i(a.x, b.y), layer, 0), textureLoad(areas, b, layer, 0));
  let at4 = array<vec2i, 4>(a, vec2i(b.x, a.y), vec2i(a.x, b.y), b);
  let corner = select(0, 1, f.x > 0.5) + select(0, 2, f.y > 0.5);
  let channels = vec4i(sample_of(at4[corner], t.x, t.yz, i32(a4[corner].x + 0.5)), sample_of(at4[3 - corner], t.x, t.yz, i32(a4[3 - corner].x + 0.5)));
  let ends = max(abs(vec2f(raw[corner], raw[3 - corner])) - HALF_DIAGONAL * texel, vec2f(0.0));
  // The ponds the four texels lie in: their highest level, and the share of the point's weight on them.
  let ponds = vec4i(i32(a4[0].y + 0.5), i32(a4[1].y + 0.5), i32(a4[2].y + 0.5), i32(a4[3].y + 0.5)) - 1;
  var levels = vec4f(-NO_LIQUID_M);
  if (all(ponds == vec4i(ponds.x))) { levels = vec4f(pond_level(ponds.x)); }
  else { levels = vec4f(pond_level(ponds.x), pond_level(ponds.y), pond_level(ponds.z), pond_level(ponds.w)); }
  let bilinear = vec4f((1.0 - f.x) * (1.0 - f.y), f.x * (1.0 - f.y), (1.0 - f.x) * f.y, f.x * f.y);
  let wet = select(vec4f(0.0), vec4f(1.0), ponds >= vec4i(0));
  let water = vec2f(max(max(levels.x, levels.y), max(levels.z, levels.w)), dot(bilinear, wet));
  if (!spline) { return Tiled(o.x + o.y, vec2f(0.0), o.z, abs(signed), d_slope, channels, ends, water); }
  let f2 = f * f;
  let f3 = f2 * f;
  // The weights of the texels from i - 1 to i + 2 along each axis, and their slopes.
  var w = array<vec2f, 4>(0.5 * (-f3 + 2.0 * f2 - f), 0.5 * (3.0 * f3 - 5.0 * f2 + 2.0), 0.5 * (-3.0 * f3 + 4.0 * f2 + f), 0.5 * (f3 - f2));
  var dw = array<vec2f, 4>(0.5 * (-3.0 * f2 + 4.0 * f - 1.0), 0.5 * (9.0 * f2 - 10.0 * f), 0.5 * (-9.0 * f2 + 8.0 * f + 1.0), 0.5 * (3.0 * f2 - 2.0 * f));
  var h = vec3f(0.0);
  for (var y = 0; y < 4; y++) {
    for (var x = 0; x < 4; x++) {
      let t = textureLoad(atlas, clamp(i + vec2i(x - 1, y - 1), vec2i(0), vec2i(last)), layer, 0);
      let v = t.x + t.y;
      h += v * vec3f(w[x].x * w[y].y, dw[x].x * w[y].y, w[x].x * dw[y].y);
    }
  }
  return Tiled(h.x, h.yz / texel, o.z, abs(signed), d_slope, channels, ends, water);
}

// What runs through sample `s` (-1: none), m³/s.
fn discharge_of(s: vec2i) -> f32 {
  if (s.x < 0) { return 0.0; }
  return textureLoad(flows, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).x;
}

// What runs through the channel at a point of ground `g`, m³/s: its nearest texel's channel's, or the channel's of the
// texel across from that where it carries more and that texel lies in its water, so a pixel in the water of a channel
// a tributary joins is that channel's.
fn discharge_at(g: Tiled) -> f32 {
  let near = discharge_of(g.channels.xy);
  let across = discharge_of(g.channels.zw);
  if (across > near && g.ends.y < 0.5 * land.channel.x * pow(across, land.channel.y)) { return across; }
  return near;
}

// The channel at a point of ground `g`, by the channel law from its own sample's discharge `q`, m³/s (flow/channel.rs):
// its half width and mean depth, m, and the depth of its bed under its water's surface there, m: a parabola as wide as
// the channel and as deep at its middle as its mean depth makes it (`BED_MIDDLE`), so the water is deepest at its
// middle and its banks catch the light.
struct Bed { half: f32, mean: f32, depth: f32 }
fn bed_at(g: Tiled, q: f32) -> Bed {
  if (q <= 0.0) { return Bed(0.0, 0.0, 0.0); }
  let half = 0.5 * land.channel.x * pow(q, land.channel.y);
  let mean = land.channel.z * pow(q, land.channel.w);
  let x = min(g.d / half, 1.0);
  return Bed(half, mean, BED_MIDDLE * mean * (1.0 - x * x));
}

// A channel's bed is as wide as the flow that shapes it, its floods: the bankfull discharge, the flood of every year or
// two, about `FLOOD` times the mean flow in humid lands (Leopold and Maddock 1953; Wolman and Miller 1960; Dunne and
// Leopold 1978: 2 to 10 times).
const FLOOD = 5.0;
// The half width of the bed of a channel that carries `q` m³/s this month, m: the channel law's at `FLOOD` times it.
fn banks_of(q: f32) -> f32 { return 0.5 * land.channel.x * pow(FLOOD * q, land.channel.y); }

// The level of pond `p` (-1: none), m: from the rows of the table under its planes, four ponds a texel; far below
// any ground for none (no body holds it, or the texel lies in no pond).
fn pond_level(p: i32) -> f32 {
  if (p < 0) { return -NO_LIQUID_M; }
  let w = i32(land.map.y);
  let q = p / 4;
  return textureLoad(table, vec2i(q % w, PLANES * i32(land.map.z) + q / w), 0)[p % 4];
}

// Cell value `k` (a plane of the table) of the square at column and row c, the nearest on the map.
fn cell(k: i32, c: vec2i) -> vec4f {
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let q = clamp(c, vec2i(0), n - 1);
  return textureLoad(table, vec2i(q.x, k * n.y + q.y), 0);
}

// The four squares about a point `s` squares from the map's corner, between whose centres it is blended, and its weight
// in each.
struct Corners { at: array<vec2i, 4>, w: vec4f }
fn corners(s: vec2f) -> Corners {
  let p = s - 0.5;
  let i = vec2i(floor(p));
  let f = p - floor(p);
  return Corners(array<vec2i, 4>(i, i + vec2i(1, 0), i + vec2i(0, 1), i + vec2i(1, 1)), vec4f((1.0 - f.x) * (1.0 - f.y), f.x * (1.0 - f.y), (1.0 - f.x) * f.y, f.x * f.y));
}
fn blended(k: i32, c: Corners) -> vec4f {
  return c.w.x * cell(k, c.at[0]) + c.w.y * cell(k, c.at[1]) + c.w.z * cell(k, c.at[2]) + c.w.w * cell(k, c.at[3]);
}

// The ground at `at`, m from the map's north-west corner (`read_tile`, with `spline`): from the tile the page names
// there, fading in over the one before it; before any tile came, the cells' mean heights, flat.
fn ground_at(at: vec2f, spline: bool) -> Tiled {
  let span = land.map.x * land.tiles.x / exp2(land.tiles.z - land.map.w);
  let size = vec2i(textureDimensions(pages));
  let page = textureLoad(pages, clamp(vec2i(floor(at / span)), vec2i(0), size - 1), 0);
  if (page.x < 0.0) {
    return Tiled(blended(GROUND, corners(at / land.map.x)).x, vec2f(0.0), 0.0, NO_LIQUID_M, vec2f(0.0), vec4i(-1), vec2f(NO_LIQUID_M), vec2f(-NO_LIQUID_M, 0.0));
  }
  let shown = read_tile(i32(page.x), at, spline);
  let came = textureLoad(tile_of, vec2i(i32(page.x), 0), 0).w;
  let w = clamp((land.time.x - came) / land.time.y, 0.0, 1.0);
  if (w >= 1.0 || page.y < 0.0) { return shown; }
  let under = read_tile(i32(page.y), at, spline);
  return Tiled(mix(under.h, shown.h, w), mix(under.slope, shown.slope, w), mix(under.twi, shown.twi, w), mix(under.d, shown.d, w), mix(under.d_slope, shown.d_slope, w), shown.channels, shown.ends, mix(under.water, shown.water, w));
}

// The liquid in the soil at a point of wetness index `twi` of a cell whose pores hold `pore` m and `pores` m full, its
// index's level `c` (inside/shares.rs `soil_held`): departed from the mean by `soil_m` per unit of the index over the
// level, within none and full; even where the pores hold none or are full.
fn soil_held(pore: f32, pores: f32, c: f32, twi: f32) -> f32 {
  if (pore <= 0.0 || pore >= pores) { return pore; }
  return clamp(pore + land.soil.x * (twi - c), 0.0, pores);
}

// The share of a tent of half-width 1 that lies below t (a pixel's footprint across a line).
fn tent(x: f32) -> f32 {
  let t = clamp(x, -1.0, 1.0);
  return select(1.0 - (1.0 - t) * (1.0 - t) / 2.0, (1.0 + t) * (1.0 + t) / 2.0, t <= 0.0);
}

// The activity of a living thing on ground as wet as `wet` where it needs `need` (cover::activity, Physics::activity):
// full where it is as wet as it needs, less as it dries.
fn activity(wet: f32, need: f32) -> f32 {
  let x = wet / need;
  return select(pow(max(x, 0.0), land.soil.z), 1.0, x >= 1.0);
}

// The share of a kind whose roots need `need` m of loose ground that holds where `loose` m lies (roots.ts `rooted`, the
// same rule): none from `ROOTED_FROM` of its need down, all from its need up. Bare bed holds no liquid in pores and
// nothing to anchor a tall plant; a kind roots about `ROOT_PER_HEIGHT` of its height deep (roots.ts). Where the loose
// depth changes by `spread` m across a pixel and that is more than the rule's own span, the edge is spread over it, as
// the share of water at a shore is (no speckle where the loose ground has fine patches).
const ROOTED_FROM = 0.25;
fn rooted(loose: f32, need: f32, spread: f32) -> f32 {
  let middle = 0.5 * (1.0 + ROOTED_FROM) * need;
  let half = 0.5 * max((1.0 - ROOTED_FROM) * need, spread);
  return ease(middle - half, middle + half, loose);
}

// The state of the ground at a point: the depth of the liquid standing over it and its height over that liquid's
// surface, m; how wet it is, 0..1; its temperature, K; the snow and ice on it, 0..1, as its cells' share (`snow_at`
// places it), and the snow's depth, m; how wet its cells are on their mean, 0..1; and its height over its cells' mean,
// m, and the share of its cells' ground their frozen cover covers before it melts, 0..1.
struct State { depth: f32, over: f32, wet: f32, t: f32, snow: f32, snow_m: f32, mean: f32, rise: f32, covered: f32 }

// Inside a cell its snow lies first where the ground holds it (`snow_bares`): high ground before low (colder: the air's
// fall with height; over `SNOW_RISE_M` a unit), slopes turned from the sun before those facing it (they melt first;
// `SNOW_SUNNY` a unit of slope toward the light), hollows before knobs (drift fills them; `SNOW_HOLLOW_M` of the
// relief's height a unit). The cell's share `s` covers where this index lies under `SNOW_SPREAD` ln(s / (1 - s)): of
// an index spread about 1 either way (a logistic one), the share s, so as much snow is drawn as the cell holds. Its
// edge is a line of that field, no narrower than `SNOW_EDGE` of the index.
// The index is held softly within `SNOW_BARES_MOST` either way, so a cell's last few hundredths of snow lie nowhere,
// and no snow ends on the straight line where the blended share reaches none (a line through the squares' centres).
const SNOW_RISE_M = 40.0; const SNOW_SUNNY = 2.0; const SNOW_HOLLOW_M = 0.25; const SNOW_SPREAD = 1.0; const SNOW_EDGE = 0.05;
const SNOW_BARES_MOST = 3.0;

// How readily the ground of slope `slope` (east, south), `rise` m over its cells' mean and `knob` m over its relief's
// mean (relief.wgsl), sheds its snow: the lower, the longer it keeps it.
fn snow_bares(slope: vec2f, rise: f32, knob: f32) -> f32 {
  let sunward = clamp(dot(-slope, LIGHT.xy) / length(LIGHT.xy), -1.0, 1.0);
  return SNOW_BARES_MOST * tanh((SNOW_SUNNY * sunward + knob / SNOW_HOLLOW_M - rise / SNOW_RISE_M) / SNOW_BARES_MOST);
}

// The snow at a point whose cells hold `share` of their ground under snow and whose ground sheds it as `bares`
// (`snow_bares`), that index changing by `width` across a pixel: 1 where it lies, 0 where not, the pixel's share at
// its edge.
fn snow_at(share: f32, bares: f32, width: f32) -> f32 {
  if (share <= 0.0 || share >= 1.0) { return share; }
  let line = SNOW_SPREAD * log(share / (1.0 - share));
  return 1.0 - smoothstep(-1.0, 1.0, (bares - line) / max(width, SNOW_EDGE));
}

// The state at `at`, m from the map's north-west corner, on ground `g` whose bed a channel cuts `cut` m deep there
// (`bed_at`): the cells' values blended between the squares' centres (`cs`), and the laws inside a cell applied at the
// point.
fn state_on(at: vec2f, g: Tiled, cs: Corners, cut: f32) -> State {
  // The cells' ground, and the share of it each one's frozen cover covers, blended: the shares, not the depths, so a
  // snowy cell beside a bare one draws its share falling across the space between their centres, not at once along a
  // line where the blended depth reaches what covers all (tanh saturates within a few centimetres).
  // Over land, the land's mean height, temperature and ice (the table's `TRUTH` and `VIEW` planes carry them, the open low ground's
  // coastal squares taking the land's nearest: web/src/land.ts `fromLand`); over standing liquid, the cells'.
  let dry = g.water.y <= 0.0 || g.h - cut > g.water.x;
  var ground = vec4f(0.0);
  var covered = 0.0;
  for (var k = 0; k < 4; k++) {
    var c = cell(GROUND, cs.at[k]);
    // A sea's square (its air's fall marked as -(it + 1), web/src/land.ts) gives a dry point the land's numbers.
    if (c.z < 0.0) {
      c.z = -c.z - 1.0;
      if (dry) { let v = cell(VIEW, cs.at[k]); c = vec4f(v.w, v.z, c.z, cell(TRUTH, cs.at[k]).w); }
    }
    ground += cs.w[k] * c;
    covered += cs.w[k] * snow_of(c.w, 0.0, 0.0);
  }
  // The liquid standing over the point: its ponds', flat at their levels over the ground under it (laws.md 1.2).
  // The soil's liquid of each cell about it at the point's wetness index, and each one's mean liquid, blended.
  var held = 0.0;
  var mean = 0.0;
  for (var k = 0; k < 4; k++) {
    let l = cell(LIQUID, cs.at[k]);
    let s = cell(SOIL, cs.at[k]);
    held += cs.w[k] * soil_held(l.w, s.x, s.y, g.twi);
    mean += cs.w[k] * (l.w + l.z);
  }
  var p: State;
  if (g.water.x > -NO_LIQUID_M && g.water.y > 0.0) {
    // Over the ponds' level, the height counts the more the less of the point is theirs, so a shore's fringe fades out
    // within a texel of the pond's last texel; under it, the depth shallows to none across that texel, so where the
    // ground drawn lies under the level past the pond's last texel the shore is a soft edge, not the texels' steps.
    p.depth = max(g.water.x - (g.h - cut), 0.0) * g.water.y;
    p.over = min(max(g.h - cut - g.water.x, 0.0) / g.water.y, NO_LIQUID_M);
  } else {
    p.depth = 0.0;
    p.over = NO_LIQUID_M;
  }
  let half = land.soil.y;
  p.wet = (held + p.depth) / (held + p.depth + half);
  // The ground's temperature is its cell's at the cell's mean height, less the air's fall with height above it.
  p.t = ground.y - ground.z * (g.h - ground.x);
  p.snow = covered * unmelted(p.t, land.time.z);
  p.snow_m = snow_depth(ground.w, p.t, land.time.z);
  p.mean = mean / (mean + half);
  p.rise = g.h - ground.x;
  p.covered = covered;
  return p;
}

// The state at `at` on ground `g` as a plant's foot reads it (cover.wgsl): the water of the point's channel stands over
// its bed too, and its floods over its banks (`banks_of`): no plant roots in a channel's bed.
fn state_at(at: vec2f, g: Tiled, cs: Corners) -> State {
  let q = discharge_at(g);
  let bed = bed_at(g, q);
  var p = state_on(at, g, cs, bed.depth);
  p.depth = max(max(p.depth, bed.depth), select(0.0, ROOTS_M, g.d < banks_of(q)));
  return p;
}
