// The map's colours: the fields of a window of the map (web/src/fields.ts), a texel each, read at a point between the
// texels' centres and turned into the colour of the ground there, lit from the north-west, under the liquid that
// stands on it (water.wgsl), and into its map class. One rule for the map of the squares and for the window finer than
// its cells; the viewer, not the core, decides what counts as sea, river, desert.

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
  // Where the window lies in the world: its top-left corner and its size, m.
  place: vec4f,
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
// How wet the ground is, 0..1; the depth of the river through it, m; the share of the ground the plant cover leaves
// bare.
@group(0) @binding(7) var inside_field: texture_2d<f32>;

// The map classes (web/src/earth.ts CLASSES).
const DEEP = 0u; const SHALLOW = 1u; const LAKE = 2u; const RIVER = 3u; const ICE = 4u;
const ROCK = 5u; const DESERT = 6u; const GRASS = 7u; const FOREST = 8u;

const SAND = vec3f(226, 211, 164); const TUNDRA = vec3f(185, 191, 168); const UPLAND = vec3f(168, 156, 124);
const SNOW = vec3f(247, 250, 252); const PATH = vec3f(232, 89, 12);
const ROCK_LO = vec3f(141, 138, 132); const ROCK_HI = vec3f(205, 202, 196);
// The light from the north-west.
const LIGHT = vec3f(-0.588, -0.588, 0.647);
// How many times steeper than it is the ground is shaded.
const EXAGGERATE = 6.0;
// The most of a shore by the sea that is sand.
const SAND_SHARE = 0.55;
// Snow lies by a texel's own temperature where that is within these of the cover's melting point, K (fading between),
// and by the cells' ice elsewhere.
const SNOW_NEAR_K = vec2f(2.0, 6.0);
// Water standing deeper than this, m, is the deep sea's.
const DEEP_SEA_M = 200.0;
// Over the liquid's surface the ground draws water up through its pores (the capillary fringe), wet to e^-1 at this
// height, m (about half a metre in a loam; Bear 1972).
const FRINGE_M = 0.5;
// Soaked bare ground reflects this much less than dry (Lobell and Asner 2002: about half); a film of liquid starts to
// shine on it from this share of soaking on.
const WET_DARK = 0.5;
const FILM_FROM = 0.8;
// A bed that sends back less than this share of its light (half a display level of 255) does not show.
const SHOWS = 0.002;

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

// The ramp of the plant cover.
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
  // The depth of the liquid standing there, m (0: none); the ground's height over the liquid's surface, m (0 under
  // it); the share of the liquid that is the sea's; how wet the ground is, 0..1, and the share of it that is bare.
  depth: f32, over: f32, sea: f32, wet: f32, bare: f32,
  // Snow and ice on it, 0..1; the ground's temperature, K; the byte of the plant cover's mass (0..255); the ground's
  // height, m; its slope (rise over run); the light on it over the light on flat ground.
  snow: f32, t: f32, plants: f32, z: f32, slope: f32, shade: f32,
  // Beach sand on it and a river through it, 0..1, and the river's depth, m.
  sand: f32, river: f32, river_depth: f32,
  // Where it lies in the world, m, and the metres a pixel takes there (for the ripples of a liquid's surface).
  at: vec2f, e: f32,
}

// The spot at uv of the fields, whose ground's height and slope are those of `h` (`spline` of `height_field`), seen
// at `e` m a pixel.
fn spot_at(uv: vec2f, h: vec4f, e: f32) -> Spot {
  let m = blend(matter_field, uv);
  let inside = blend(inside_field, uv);
  let look = earth.look;
  var s: Spot;
  // The liquid over the ground blended between the texels, not as a spline: a spline would overshoot where a cell
  // that holds liquid meets one that holds none.
  let water = blend(height_field, uv).y;
  s.depth = max(water, 0.0);
  s.over = max(-water, 0.0);
  s.sea = m.z;
  s.wet = inside.x;
  s.bare = inside.z;
  s.t = m.y;
  let ice = ease(0.02, 0.5, m.x);
  let near = select(0.0, 1.0 - ease(SNOW_NEAR_K.x, SNOW_NEAR_K.y, abs(s.t - look.y)), look.y > 0.0);
  s.snow = ice + (ease(look.y + 1.0, look.y - 1.0, s.t) - ice) * near;
  s.plants = 255.0 * textureSampleLevel(byte_field, smooth_sampler, uv, 0.0).x;
  s.z = h.x;
  s.slope = length(h.zw) * look.x;
  s.shade = hill_shade(h.zw, EXAGGERATE * look.x);
  s.sand = SAND_SHARE * s.sea * (1.0 - ease(0.0, look.z, s.over));
  s.river = m.w;
  s.river_depth = inside.y;
  s.at = earth.place.xy + uv * earth.place.zw;
  s.e = e;
  return s;
}

// A spot's shaded colour (display, 0..255), its shade's multiplier and its map class.
struct Shaded { rgb: vec3f, m: f32, kind: u32 }

// The colour of the ground of spot p, unlit, with plants `plants` (their byte) and snow `snow` on it, the share of it
// that is bare (all where it has no plants) and its map class.
struct Ground { rgb: vec3f, bare: f32, kind: u32 }
fn ground_of(p: Spot, plants: f32, snow: f32) -> Ground {
  let v = sqrt(plants / 255.0);
  var g: Ground;
  g.rgb = ramp_plants(v);
  g.bare = select(p.bare, 1.0, plants <= 0.0);
  g.kind = select(select(FOREST, GRASS, v < 0.62), DESERT, v < 0.3);
  g.rgb = mix(g.rgb, TUNDRA, ease(278.0, 266.0, p.t) * 0.7);
  g.rgb = mix(g.rgb, UPLAND, ease(1200.0, 2600.0, p.z) * 0.5);
  g.rgb = mix(g.rgb, SAND, p.sand);
  let rock = max(ease(0.14, 0.42, p.slope) * ease(150.0, 700.0, p.z), ease(2000.0, 3200.0, p.z) * 0.85) * (1.0 - 0.5 * v);
  g.rgb = mix(g.rgb, mix(ROCK_LO, ROCK_HI, ease(1500.0, 4000.0, p.z)), rock);
  if (rock > 0.5) { g.kind = ROCK; }
  g.rgb = mix(g.rgb, SNOW, snow);
  if (snow > 0.5) { g.kind = ICE; }
  return g;
}

// How soaked the ground of spot p is, 0..1: as wet as the field says, and wetter the nearer it lies over the liquid's
// surface (the capillary fringe); soaked under it.
fn soaked(p: Spot) -> f32 { return 1.0 - (1.0 - p.wet) * (1.0 - exp(-p.over / FRINGE_M)); }

// Ground g soaked to s, lit by `lit` (its shade's multiplier): its bare part the darker the more soaked.
fn soil(g: Ground, s: f32, lit: f32) -> vec3f { return g.rgb * (1.0 - WET_DARK * s * g.bare) * lit; }

fn colour(p: Spot) -> Shaded {
  let lit = clamp(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), 0.5, 1.45);
  // Under the liquid: its bed, soaked and without snow (frozen liquid lies on the surface, calm), seen through it,
  // flat: at no depth it is the soaked ground with its film of liquid.
  if (p.depth > 0.0) {
    // No bed shows through water deeper than shows any of it.
    var bed = vec3f(0.0);
    if (through(p.depth).b > SHOWS) { bed = soil(ground_of(p, p.plants, 0.0), 1.0, lit); }
    let rgb = mix(water_over(bed, p.depth, ripples(p.at, p.e) * (1.0 - p.snow)), SNOW, p.snow * 0.85);
    var kind = select(LAKE, select(SHALLOW, DEEP, p.depth > DEEP_SEA_M), p.sea >= 0.5);
    if (p.snow > 0.5) { kind = ICE; }
    // The true view shades the bed's colours as far as the bed shows.
    return Shaded(rgb, 1.0 + (lit - 1.0) * through(p.depth).g, kind);
  }
  // Over it: the ground, wet by how soaked it is, a film of liquid shining on its bare part once it is near soaked
  // (mud).
  let g = ground_of(p, p.plants, p.snow);
  let s = soaked(p);
  let film = ease(FILM_FROM, 1.0, s) * g.bare;
  var rgb = soil(g, s, lit);
  let river = p.river * (1.0 - 0.6 * p.snow);
  if (film <= 0.0 && river <= 0.0) { return Shaded(rgb, lit, g.kind); }
  let ripple = ripples(p.at, p.e);
  rgb += film * surface_light(ripple * film);
  var kind = g.kind;
  // A river: its water over its own bare bed.
  if (river > 0.0) { rgb = mix(rgb, water_over(soil(ground_of(p, 0.0, 0.0), 1.0, lit), p.river_depth, ripple), river); }
  if (river > 0.5) { kind = RIVER; }
  return Shaded(rgb, lit, kind);
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

// The metres of the world a pixel of the target takes where it shows uv of the window.
fn metres_a_pixel(uv: vec2f) -> f32 {
  return max(length(dpdx(uv) * earth.place.zw), length(dpdy(uv) * earth.place.zw));
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
