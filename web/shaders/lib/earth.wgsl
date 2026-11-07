// The map's colours: the fields of a window of the map (web/src/fields.ts), a texel each, read at a point between the
// texels' centres and turned into the colour of the ground there, lit from the north-west, and into its map class. One
// rule for the map of the squares and for the window finer than its cells; the viewer, not the core, decides what
// counts as sea, river, desert.

struct Earth {
  // How the fields are read: the slope is read this many times as steep (a pixel's slope over a short run is steeper
  // than the map shows it, web/src/fine.ts); the melting point of the frozen cover, K, by which a texel's own
  // temperature lays snow near it (0: the ice of the cells alone); a shore by the sea is sand up to this height over
  // the liquid, m; the bytes of a layer view over which an empty spot fades into a full one.
  look: vec4f,
  // What is drawn: the view (0 the Earth's colours, 1 the true colour, 2 a layer over them); the stops of the layer's
  // ramp; whether the worn paths are drawn (1).
  view: vec4f,
  // The share of the window along its west, north, east and south edges over which it fades out (0: none).
  edge: vec4f,
  // The layer's ramp, display colours 0..255.
  ramp: array<vec4f, 10>,
}
@group(0) @binding(1) var<uniform> earth: Earth;
// The ground's height, m; the liquid's surface over it, m (below it: negative); the ground's slope east and south.
@group(0) @binding(2) var height_field: texture_2d<f32>;
// The ice, m; the ground's temperature, K; the share that is the sea's; the share that is a river.
@group(0) @binding(3) var matter_field: texture_2d<f32>;
// The byte of the plant cover's mass, and the true colour (red, green, blue), 0..1.
@group(0) @binding(4) var byte_field: texture_2d<f32>;
// The byte of the layer a view draws, and of the worn paths, 0..1.
@group(0) @binding(5) var view_field: texture_2d<f32>;
@group(0) @binding(6) var smooth_sampler: sampler;

// The map classes (web/src/earth.ts CLASSES).
const DEEP = 0u; const SHALLOW = 1u; const LAKE = 2u; const RIVER = 3u; const ICE = 4u;
const ROCK = 5u; const DESERT = 6u; const GRASS = 7u; const FOREST = 8u;

const SAND = vec3f(226, 211, 164); const TUNDRA = vec3f(185, 191, 168); const UPLAND = vec3f(168, 156, 124);
const SNOW = vec3f(247, 250, 252); const RIVER_RGB = vec3f(42, 130, 234); const PATH = vec3f(232, 89, 12);
const ROCK_LO = vec3f(141, 138, 132); const ROCK_HI = vec3f(205, 202, 196);
// The light from the north-west.
const LIGHT = vec3f(-0.588, -0.588, 0.647);
// How many times steeper than it is the ground is shaded.
const EXAGGERATE = 6.0;
// The least rise of the liquid over the ground across a pixel a shore is smoothed over, m.
const SHORE_RISE_M = 0.05;
// The most of a shore by the sea that is sand.
const SAND_SHARE = 0.55;
// Snow lies by a texel's own temperature where that is within these of the cover's melting point, K (fading between),
// and by the cells' ice elsewhere.
const SNOW_NEAR_K = vec2f(2.0, 6.0);

// Field `t` (32-bit floats, which a sampler may not blend) at uv, blended between its texels' centres, its edge texels
// held beyond them.
fn blend(t: texture_2d<f32>, uv: vec2f) -> vec4f {
  let size = vec2i(textureDimensions(t));
  let p = uv * vec2f(size) - 0.5;
  let i = vec2i(floor(p));
  let f = p - floor(p);
  let a = clamp(i, vec2i(0), size - 1);
  let b = clamp(i + 1, vec2i(0), size - 1);
  let top = mix(textureLoad(t, a, 0), textureLoad(t, vec2i(b.x, a.y), 0), f.x);
  let bottom = mix(textureLoad(t, vec2i(a.x, b.y), 0), textureLoad(t, b, 0), f.x);
  return mix(top, bottom, f.y);
}

// Field `t` at uv through its texels' centres as a Catmull-Rom spline (it keeps each texel's value, so a lake of one
// texel stays, and its slope runs on across the texels' edges, so the light on a slope drawn over many pixels a texel
// shows no creases there), its edge texels held beyond them.
fn spline(t: texture_2d<f32>, uv: vec2f) -> vec4f {
  let size = vec2i(textureDimensions(t));
  let p = uv * vec2f(size) - 0.5;
  let i = vec2i(floor(p));
  let f = p - floor(p);
  let f2 = f * f;
  let f3 = f2 * f;
  // The weights of the texels from i - 1 to i + 2 along each axis.
  var w = array<vec2f, 4>(0.5 * (-f3 + 2.0 * f2 - f), 0.5 * (3.0 * f3 - 5.0 * f2 + 2.0), 0.5 * (-3.0 * f3 + 4.0 * f2 + f), 0.5 * (f3 - f2));
  var sum = vec4f(0.0);
  for (var y = 0; y < 4; y++) {
    var row = vec4f(0.0);
    for (var x = 0; x < 4; x++) { row += w[x].x * textureLoad(t, clamp(i + vec2i(x - 1, y - 1), vec2i(0), size - 1), 0); }
    sum += w[y].y * row;
  }
  return sum;
}

// The ramps of the sea and the lakes by depth and of the plant cover.
fn ramp_sea(t: f32) -> vec3f {
  var r = array<vec3f, 4>(vec3f(143, 214, 220), vec3f(79, 168, 210), vec3f(43, 108, 171), vec3f(20, 61, 115));
  let x = clamp(t, 0.0, 1.0) * 3.0; let i = min(u32(x), 2u);
  return mix(r[i], r[i + 1u], x - f32(i));
}
fn ramp_lake(t: f32) -> vec3f {
  var r = array<vec3f, 3>(vec3f(163, 216, 232), vec3f(90, 159, 210), vec3f(58, 115, 181));
  let x = clamp(t, 0.0, 1.0) * 2.0; let i = min(u32(x), 1u);
  return mix(r[i], r[i + 1u], x - f32(i));
}
fn ramp_plants(t: f32) -> vec3f {
  var r = array<vec3f, 6>(vec3f(230, 210, 156), vec3f(217, 202, 138), vec3f(188, 203, 124), vec3f(146, 189, 106), vec3f(92, 155, 87), vec3f(50, 113, 74));
  let x = clamp(t, 0.0, 1.0) * 5.0; let i = min(u32(x), 4u);
  return mix(r[i], r[i + 1u], x - f32(i));
}
// The layer's ramp (`earth.ramp`, `earth.view.y` stops).
fn ramp_layer(t: f32) -> vec3f {
  let n = u32(earth.view.y);
  let x = clamp(t, 0.0, 1.0) * f32(n - 1u); let i = min(u32(x), n - 2u);
  return mix(earth.ramp[i].rgb, earth.ramp[i + 1u].rgb, x - f32(i));
}

// The light from the north-west on ground of slope (gx, gy) (east, south), made `k` times steeper, over the light on
// flat ground.
fn hill_shade(g: vec2f, k: f32) -> f32 {
  let n = -k * g;
  return ((n.x * LIGHT.x + n.y * LIGHT.y + LIGHT.z) / sqrt(dot(n, n) + 1.0)) / LIGHT.z;
}

// What the colour of a point of the map reads.
struct Spot {
  // The depth of the liquid standing there, m; the share of it that is the sea's; the share of the point it covers.
  depth: f32, sea: f32, cover: f32,
  // Snow and ice on it, 0..1; the ground's temperature, K; the byte of the plant cover's mass (0..255); the ground's
  // height, m; its slope (rise over run); the light on it over the light on flat ground.
  snow: f32, t: f32, plants: f32, z: f32, slope: f32, shade: f32,
  // Beach sand on it and a river through it, 0..1.
  sand: f32, river: f32,
}

// The spot at uv of the fields, whose height texel `h` is (`blend` of `height_field`) and whose liquid over the ground
// rises `rise` m across a pixel.
fn spot_at(uv: vec2f, h: vec4f, rise: f32) -> Spot {
  let m = blend(matter_field, uv);
  let look = earth.look;
  var s: Spot;
  let water = h.y;
  s.depth = max(water, 0.0);
  s.sea = m.z;
  // The shore lies where the liquid's surface meets the ground, soft over a pixel or `SHORE_RISE_M` of rise.
  s.cover = clamp(0.5 + water / max(rise, SHORE_RISE_M), 0.0, 1.0);
  s.t = m.y;
  let ice = ease(0.02, 0.5, m.x);
  let near = select(0.0, 1.0 - ease(SNOW_NEAR_K.x, SNOW_NEAR_K.y, abs(s.t - look.y)), look.y > 0.0);
  s.snow = ice + (ease(look.y + 1.0, look.y - 1.0, s.t) - ice) * near;
  s.plants = 255.0 * textureSampleLevel(byte_field, smooth_sampler, uv, 0.0).x;
  s.z = h.x;
  s.slope = length(h.zw) * look.x;
  s.shade = hill_shade(h.zw, EXAGGERATE * look.x);
  s.sand = select(0.0, SAND_SHARE * s.sea * (1.0 - ease(0.0, look.z, -water)), water < 0.0);
  s.river = m.w;
  return s;
}

// A spot's shaded colour (display, 0..255), its shade's multiplier and its map class (the water's where the liquid
// covers half of it or more).
struct Shaded { rgb: vec3f, m: f32, kind: u32 }

fn colour(p: Spot) -> Shaded {
  let k = p.cover;
  var wet_class = LAKE; var dry_class = DESERT;
  var wet_m = 1.0; var dry_m = 1.0;
  var water = vec3f(0.0); var land = vec3f(0.0);
  if (k > 0.0) {
    water = mix(ramp_lake(1.0 - exp(-p.depth / 60.0)), ramp_sea(1.0 - exp(-p.depth / 350.0)), p.sea);
    if (p.sea >= 0.5) { wet_class = select(SHALLOW, DEEP, p.depth > 200.0); }
    water = mix(water, SNOW, p.snow * 0.85);
    if (p.snow > 0.5) { wet_class = ICE; }
    wet_m = clamp(1.0 + (p.shade - 1.0) * (0.25 + 0.5 * p.snow), 0.5, 1.45);
    water *= wet_m;
  }
  if (k < 1.0) {
    let v = sqrt(p.plants / 255.0);
    land = ramp_plants(v);
    dry_class = select(select(FOREST, GRASS, v < 0.62), DESERT, v < 0.3);
    land = mix(land, TUNDRA, ease(278.0, 266.0, p.t) * 0.7);
    land = mix(land, UPLAND, ease(1200.0, 2600.0, p.z) * 0.5);
    land = mix(land, SAND, p.sand);
    let rock = max(ease(0.14, 0.42, p.slope) * ease(150.0, 700.0, p.z), ease(2000.0, 3200.0, p.z) * 0.85) * (1.0 - 0.5 * v);
    land = mix(land, mix(ROCK_LO, ROCK_HI, ease(1500.0, 4000.0, p.z)), rock);
    if (rock > 0.5) { dry_class = ROCK; }
    land = mix(land, SNOW, p.snow);
    if (p.snow > 0.5) { dry_class = ICE; }
    dry_m = clamp(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), 0.5, 1.45);
    land *= dry_m;
    let river = p.river * (1.0 - 0.6 * p.snow);
    land = mix(land, RIVER_RGB * (0.7 + 0.3 * dry_m), river * 0.9);
    if (river > 0.5) { dry_class = RIVER; }
  }
  return Shaded(mix(land, water, k), select(dry_m, wet_m, k >= 0.5), select(dry_class, wet_class, k >= 0.5));
}

// Shaded colour `c` at uv in the view drawn: the Earth's colours; the true colour, shaded; or a layer's ramp over a
// little of the Earth's colours (where the layer is empty, the Earth's colours faded); and the worn paths over it.
fn in_view(c: Shaded, uv: vec2f) -> vec3f {
  var rgb = c.rgb;
  let o = textureSampleLevel(view_field, smooth_sampler, uv, 0.0);
  if (earth.view.x == 1.0) {
    rgb = 255.0 * textureSampleLevel(byte_field, smooth_sampler, uv, 0.0).yzw * clamp(1.0 + (c.m - 1.0) * 0.7, 0.55, 1.4);
  } else if (earth.view.x == 2.0) {
    let full = clamp(255.0 * o.x / earth.look.w, 0.0, 1.0);
    let grey = dot(rgb, vec3f(0.3, 0.59, 0.11));
    let mo = 0.72 * (0.75 + 0.25 * c.m);
    rgb += ((rgb * 0.5 + grey * 0.5) * 0.6 + 100.0 - rgb) * (1.0 - full) + (ramp_layer(o.x) * mo - rgb * 0.72) * full;
  }
  // A path (byte 1 is the uncleared ground): stronger the more cleared.
  let worn = 255.0 * o.y;
  if (earth.view.z > 0.5 && worn > 1.0) { rgb = mix(rgb, PATH, 0.4 + 0.5 * ((worn - 1.0) / 254.0)); }
  return rgb;
}

// How much of the window shows at uv: it fades out along the edges `earth.edge` names.
fn edge_fade(uv: vec2f) -> f32 {
  let e = earth.edge;
  var a = 1.0;
  if (e.x > 0.0) { a *= ease(0.0, e.x, uv.x); }
  if (e.y > 0.0) { a *= ease(0.0, e.y, uv.y); }
  if (e.z > 0.0) { a *= ease(0.0, e.z, 1.0 - uv.x); }
  if (e.w > 0.0) { a *= ease(0.0, e.w, 1.0 - uv.y); }
  return a;
}
