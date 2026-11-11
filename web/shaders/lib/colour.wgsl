// The map's colours (web/src/fields.ts, web/src/land.ts): what is at a point of the map (`Spot`) turned into the colour
// of the ground there, lit from the north-west, under the liquid that stands on it (water.wgsl), and into its map class;
// then into the view drawn. One rule for every way the map is drawn (earth.wgsl, land.wgsl); the viewer, not the core,
// decides what counts as sea, river, desert. Colours are reflectances and light is linear until the view drawn turns
// them into the screen's bytes, once (`in_view_of`, draw.wgsl `srgb`).

struct Earth {
  // How the fields are read: the slope is read this many times as steep (a texel's slope over a short run is steeper
  // than the map shows it, web/src/land.ts); the melting point of the frozen cover, K, by which a texel's own
  // temperature lays snow near it (0: the ice of the cells alone); a shore by the sea is sand up to this height over
  // the liquid, m; the bytes of a layer view over which an empty spot fades into a full one.
  look: vec4f,
  // What is drawn: the view (0 the Earth's colours, 1 the true colour, 2 a layer over them); the stops of the layer's
  // ramp; whether the worn paths are drawn (1); and the sea's level, m (the live map: the lowest body's).
  view: vec4f,
  // Where the map drawn lies in the world: its top-left corner and its size, m.
  place: vec4f,
  // The layer's ramp, display colours 0..255.
  ramp: array<vec4f, 10>,
}
@group(0) @binding(1) var<uniform> earth: Earth;

// The map classes (web/src/earth.ts CLASSES).
const DEEP = 0u; const SHALLOW = 1u; const LAKE = 2u; const RIVER = 3u; const ICE = 4u;
const ROCK = 5u; const BARE = 6u; const PLANTS = 7u;

// The reflectances (red, green, blue) of the hand rules' grounds: dry sand (0.3 to 0.5, quartz the most), the pale
// ground of the cold and of the heights, and fresh snow (about 0.9); from memory (Bowker and others 1985; Warren
// 1982). A worn path is drawn in display bytes over the view.
const SAND = vec3f(0.45, 0.40, 0.30); const TUNDRA = vec3f(0.30, 0.31, 0.26); const UPLAND = vec3f(0.30, 0.27, 0.21);
const SNOW = vec3f(0.90, 0.92, 0.95); const PATH = vec3f(232, 89, 12);
// How many times steeper than it is the ground is shaded.
const EXAGGERATE = 6.0;
// A beach (`beach`) lies on ground whose slope (rise over run) is under `BEACH_SLOPE`, none where it passes
// `ROCK_SLOPE` (a steep coast is a rock shore: waves carry the sand away), and on loose ground at least `BEACH_LOOSE_M`
// deep (sand is the load the wear laid at the shore); by a lake up to `LAKE_BEACH` of the sea's height over its level
// (smaller waves).
const BEACH_SLOPE = 0.15; const ROCK_SLOPE = 0.35; const BEACH_LOOSE_M = 0.5; const LAKE_BEACH = 0.3;
// Plants hold on bare bed this share of what they hold on loose ground (only in its cracks).
const ON_BED = 0.3;
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
// The share of beach sand on ground `sea_over` m over the sea's surface and `over` m over the nearest liquid's, `sea` of
// its shore the sea's, of slope `slope` (rise over run) on `loose` m of loose ground: sand by the sea up to `high` m
// over it and under it (the shallows' bed), by a lake up to `LAKE_BEACH` of that over its level, on gentle loose
// ground only. A body that holds no liquid has no level (tape/src/table.rs), so no dry basin is drawn as sand.
fn beach(sea: f32, sea_over: f32, over: f32, slope: f32, loose: f32, high: f32) -> f32 {
  let shore = max(sea * (1.0 - ease(0.0, high, sea_over)), (1.0 - sea) * (1.0 - ease(0.0, LAKE_BEACH * high, over)));
  return shore * (1.0 - ease(BEACH_SLOPE, ROCK_SLOPE, slope)) * ease(0.0, BEACH_LOOSE_M, loose);
}

// Whether the view drawn is the Earth's colours, which take the style step (style.wgsl).
fn styled() -> bool { return earth.view.x == 0.0; }

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
  // it); the share of the liquid that is the sea's; how wet the ground is, 0..1.
  depth: f32, over: f32, sea: f32, wet: f32,
  // Snow and ice on it, 0..1; the ground's temperature, K; the ground's own reflectance; the plants on it as far as
  // they are not drawn one by one: their reflectance times the share of the ground they cover, and
  // that share; the ground's height, m; its slope (rise over run); the light on it over the light on flat ground.
  snow: f32, t: f32, ground: vec3f, cover: vec4f, z: f32, slope: f32, shade: f32,
  // Beach sand on it and a river through it, 0..1, and the river's depth, m.
  sand: f32, river: f32, river_depth: f32,
  // The reflectance of its bed; the share of the bed that shows (rock.wgsl `bed_shows`); the depth of its loose
  // ground, m; and the largest stone its channel moves there, m (0: none).
  bed: vec3f, rock: f32, loose: f32, stone: f32,
  // The share of the ground the tallest plants' crowns close over it (their litter lies under them).
  canopy: f32,
  // Where it lies in the world, m, and the metres a pixel takes there (for the ripples of a liquid's surface).
  at: vec2f, e: f32,
}
// A spot's shaded colour (linear light), its shade's multiplier and its map class.
struct Shaded { rgb: vec3f, m: f32, kind: u32 }

// The colour of the ground of spot p, unlit: its own colour, tinted by the hand rules of cold, height and sand, and its bed
// where it shows; the plants on it, as far as liquid over their roots, snow and bare bed leave them (`stands_in`); and snow `snow`
// over all. The share of it that is bare, and its map class.
struct Ground { rgb: vec3f, bare: f32, kind: u32 }
fn ground_of(p: Spot, snow: f32) -> Ground {
  var g: Ground;
  g.rgb = p.ground;
  g.kind = BARE;
  g.rgb = mix(g.rgb, TUNDRA, ease(278.0, 266.0, p.t) * 0.7);
  g.rgb = mix(g.rgb, UPLAND, ease(1200.0, 2600.0, p.z) * 0.5);
  g.rgb = mix(g.rgb, select(SAND, BEACH_SAND, styled()), p.sand);
  // Where the loose ground thins its bed shows, and holds plants only in its cracks.
  g.rgb = mix(g.rgb, p.bed, p.rock);
  if (p.rock > 0.5) { g.kind = ROCK; }
  var plants = p.cover.a * stands_in(p.depth, snow) * (1.0 - (1.0 - ON_BED) * p.rock);
  // In the style: the floor's grain, and the bed's cliffs, which hold no plants (crag.wgsl).
  let steep = p.slope / earth.look.x;
  if (styled()) {
    g.rgb = floor_of(g.rgb, p.at, p.e, p.canopy, p.wet);
    let cliff = cliff_of(steep);
    if (cliff > 0.0) { g.rgb = mix(g.rgb, cliff_face(p.bed, p.z, p.at, steep, p.e), cliff); plants *= 1.0 - (1.0 - LEDGES) * cliff; }
  }
  g.bare = 1.0 - plants;
  if (plants > 0.0) { g.rgb = mix(g.rgb, p.cover.rgb / p.cover.a, plants); }
  // Stones over it: scree, on bare bed, and in a channel; none on a beach.
  if (styled()) {
    let thin = 1.0 - ease(0.0, SCREE_LOOSE_M, p.loose);
    let scree = max(SCREE * ease(SCREE_FROM, CLIFF_FROM, steep) * thin, STONES_ON_BED * p.rock) * (1.0 - p.sand);
    let rubble = stones(p.at, p.e, scree, SCREE_STONE_M, p.bed, 61);
    let pebbles = stones(p.at, p.e, select(0.0, IN_CHANNEL, p.stone > 0.0) * (1.0 - p.sand), max(p.stone, PEBBLE_M), p.bed, 62);
    g.rgb = mix(mix(g.rgb, rubble.rgb / max(rubble.a, 1e-4), rubble.a), pebbles.rgb / max(pebbles.a, 1e-4), pebbles.a);
  }
  if (plants >= 0.5) { g.kind = PLANTS; }
  g.rgb = mix(g.rgb, SNOW, snow);
  if (snow > 0.5) { g.kind = ICE; }
  return g;
}

// How soaked the ground of spot p is, 0..1: as wet as the field says, and wetter the nearer it lies over the liquid's
// surface (the capillary fringe); soaked under it.
fn soaked(p: Spot) -> f32 { return 1.0 - (1.0 - p.wet) * (1.0 - exp(-p.over / FRINGE_M)); }

// Ground g soaked to s, lit by `lit` (its shade's multiplier): its bare part the darker the more soaked (and in the
// style warmer, style.wgsl `bare_tint`).
fn soil(g: Ground, s: f32, lit: f32) -> vec3f {
  let tint = select(vec3f(1.0), bare_tint(g.bare), styled());
  return lit_by(g.rgb * (1.0 - WET_DARK * s * g.bare) * tint, lit);
}

// Colour `c` (linear) lit by `lit`; in the style, with its cool shade and warm light (style.wgsl `relief_tint`).
fn lit_by(c: vec3f, lit: f32) -> vec3f { return select(c * lit, relief_tint(c, lit), styled()); }

// The light on ground of slope `slope` (as drawn) that the sun lights `x` times as much as flat ground, over the light
// on flat ground: no more than `LIT_MOST`, and no less than the sky's light on it, `SKY_LIGHT` of the light on flat
// ground times the share of the sky the slope sees, (1 + cos slope) / 2 (an even sky, Liu and Jordan 1963). So a slope
// turned away from the sun is lit by the sky alone, the less the steeper, and still shows its shape.
const SKY_LIGHT = 0.5; const LIT_MOST = 1.45;
fn light(x: f32, slope: f32) -> f32 {
  let steep = EXAGGERATE * slope;
  return clamp(x, SKY_LIGHT * 0.5 * (1.0 + 1.0 / sqrt(1.0 + steep * steep)), LIT_MOST);
}

fn colour(p: Spot) -> Shaded {
  let x = 1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow);
  let lit = select(light(x, p.slope), relief_light(x), styled());
  // Under the liquid: its bed, soaked and without snow (frozen liquid lies on the surface, calm), seen through it,
  // flat: at no depth it is the soaked ground with its film of liquid.
  if (p.depth > 0.0) {
    // No bed shows through water deeper than shows any of it.
    var bed = vec3f(0.0);
    let ripple = ripples(p.at, p.e) * (1.0 - p.snow);
    var water: vec3f;
    if (styled()) {
      // The style: clear water over a pale bed, deepening to the open water, foam at its edge (style.wgsl).
      if (exp(-2.0 * CLEAR_ATTENUATION.b * p.depth) > SHOWS) { bed = soil(ground_of(p, 0.0), 1.0, lit); }
      let lake = min(LAKE_SHALLOWEST_M + p.depth * LAKE_SCALE, LAKE_DEEPEST_M) * ease(0.0, LAKE_RIM_M, p.depth);
      let seen = mix(lake, p.depth, p.sea);
      water = mix(water_painted(bed, seen, ripple), FOAM_RGB, foam(p.depth, p.slope / earth.look.x, p.e));
    } else {
      if (through(p.depth).b > SHOWS) { bed = soil(ground_of(p, 0.0), 1.0, lit); }
      water = water_over(bed, p.depth, ripple);
    }
    let rgb = mix(water, SNOW, p.snow * 0.85);
    var kind = select(LAKE, select(SHALLOW, DEEP, p.depth > DEEP_SEA_M), p.sea >= 0.5);
    if (p.snow > 0.5) { kind = ICE; }
    // The true view shades the bed's colours as far as the bed shows.
    return Shaded(rgb, 1.0 + (lit - 1.0) * through(p.depth).g, kind);
  }
  // Over it: the ground, wet by how soaked it is, a film of liquid shining on its bare part once it is near soaked
  // (mud).
  let g = ground_of(p, p.snow);
  let s = soaked(p);
  let film = ease(FILM_FROM, 1.0, s) * g.bare;
  var rgb = soil(g, s, lit);
  let river = p.river * (1.0 - 0.6 * p.snow);
  if (film <= 0.0 && river <= 0.0) { return Shaded(rgb, lit, g.kind); }
  let ripple = ripples(p.at, p.e);
  rgb += film * surface_light(ripple * film);
  var kind = g.kind;
  // A river: its water over its own bed. Running water carries the fine soil away: its bed is washed sand and gravel,
  // soaked.
  if (river > 0.0) { rgb = mix(rgb, water_over(SAND * (1.0 - WET_DARK) * lit, p.river_depth, ripple), river); }
  if (river > 0.5) { kind = RIVER; }
  return Shaded(rgb, lit, kind);
}
// Shaded colour `c` in the view drawn, as the screen's bytes (0..255), where the byte of the layer it draws and of the
// worn paths are `o` (0..1) and the true colour is `truth` (reflectance, 0..1): the Earth's colours; the true colour,
// shaded; or a layer's ramp over a little of the Earth's colours (where the layer is empty, the Earth's colours faded);
// and the worn paths over it.
fn in_view_of(c: Shaded, o: vec2f, truth: vec3f) -> vec3f {
  var rgb = 255.0 * srgb(c.rgb);
  if (styled()) {
    rgb = 255.0 * painted(srgb(c.rgb));
  } else if (earth.view.x == 1.0) {
    rgb = 255.0 * srgb(truth * clamp(1.0 + (c.m - 1.0) * 0.7, 0.55, 1.4));
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
