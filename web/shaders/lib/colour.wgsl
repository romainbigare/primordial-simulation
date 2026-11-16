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
  // ramp; and the sea's level, m (the live map: the lowest body's); w is not read.
  view: vec4f,
  // Where the map drawn lies in the world: its top-left corner and its size, m.
  place: vec4f,
  // Where a pixel of the target lies in the world (land.wgsl): the origin of the draw, m (a whole multiple of 1024 m,
  // field-window.ts `frameOf`) and the metres a pixel takes (w is not read); and the world's point under the target's
  // first pixel, m from the origin.
  frame: vec4f, corner: vec4f,
  // The layer's ramp, display colours 0..255.
  ramp: array<vec4f, 10>,
}
@group(0) @binding(1) var<uniform> earth: Earth;

// The map classes (web/src/earth.ts CLASSES).
const DEEP = 0u; const SHALLOW = 1u; const LAKE = 2u; const RIVER = 3u; const ICE = 4u;
const ROCK = 5u; const BARE = 6u; const PLANTS = 7u;

// The reflectances (red, green, blue) of the hand rules' grounds: dry sand (0.3 to 0.5, quartz the most), the pale
// ground of the cold and of the heights, and fresh snow (about 0.9); from memory (Bowker and others 1985; Warren
// 1982).
const SAND = vec3f(0.45, 0.40, 0.30); const TUNDRA = vec3f(0.30, 0.31, 0.26); const UPLAND = vec3f(0.30, 0.27, 0.21);
const SNOW = vec3f(0.90, 0.92, 0.95);
// How many times steeper than it is the ground is shaded.
const EXAGGERATE = 6.0;
// A beach (`beach`) lies on ground whose slope (rise over run) is under `BEACH_SLOPE`, none where it passes
// `ROCK_SLOPE` (a steep coast is a rock shore: waves carry the sand away), and on loose ground at least `BEACH_LOOSE_M`
// deep (sand is the load the wear laid at the shore); by a lake up to `LAKE_BEACH` of the sea's height over its level
// (smaller waves).
// It reaches up the shore as far as its height times the share of `BEACH_DEEP_M` the loose ground is deep, so it is
// wide where the shore's load lies deep and thin where little lies.
const BEACH_SLOPE = 0.15; const ROCK_SLOPE = 0.35; const BEACH_LOOSE_M = 0.5; const LAKE_BEACH = 0.3; const BEACH_DEEP_M = 4.0;
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
// Dry open ground (wetter than `DRY_STONES_WET` none) holds stones up to `DRY_STONE_M` across, half worn, on up to
// `DRY_STONES` of its places, gathered in fields about `DRY_FIELD_M` across.
const DRY_STONES_WET = 0.4; const DRY_STONES = 0.55; const DRY_STONE_M = 0.7; const DRY_WORN = 0.5; const DRY_FIELD_M = 9.0;
// The share of beach sand on ground `sea_over` m over the sea's surface and `over` m over the nearest liquid's, `sea` of
// its shore the sea's, of slope `slope` (rise over run) on `loose` m of loose ground: sand by the sea up to `high` m
// over it and under it (the shallows' bed), by a lake up to `LAKE_BEACH` of that over its level, on gentle loose
// ground only. A body that holds no liquid has no level (tape/src/table.rs), so no dry basin is drawn as sand.
fn beach(sea: f32, sea_over: f32, over: f32, slope: f32, loose: f32, high: f32) -> f32 {
  let reach = high * max(ease(0.0, BEACH_DEEP_M, loose), 1e-3);
  let shore = max(sea * (1.0 - ease(0.0, reach, sea_over)), (1.0 - sea) * (1.0 - ease(0.0, LAKE_BEACH * reach, over)));
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
  // Beach sand on it and a river through it, 0..1, and the river's depth, m; how much of a river's mouth it is, 0..1
  // (river.wgsl `MOUTH_CALM`).
  sand: f32, river: f32, river_depth: f32, mouth: f32,
  // The reflectance of its bed; the share of the bed that shows (rock.wgsl `bed_shows`); the depth of its loose
  // ground, m; and of its channel's stones there (stone.wgsl `Stony`): the largest's size as drawn, m, what each is
  // of that size, the share of the places that hold one (0: none), how round the channel has worn them, 0..1, and
  // the share of a scree's places that hold a block fallen into a steep reach; the depth of the snow where it lies, m;
  // and how far the point lies below the bed that sheds its scree, `BELOW_LEAST`..1 (crag.wgsl).
  bed: vec3f, rock: f32, loose: f32, stone: f32, small: f32, stony: f32, worn: f32, blocks: f32, snow_m: f32, below: f32,
  // The share of the ground the tallest plants' crowns close over it (their litter lies under them).
  canopy: f32,
  // Where it lies in the world, m, and the metres a pixel takes there (for the ripples of a liquid's surface).
  at: vec2f, e: f32,
}
// A spot's shaded colour (linear light), its shade's multiplier and its map class.
struct Shaded { rgb: vec3f, m: f32, kind: u32 }

// The colour of the ground of spot p, unlit: its own colour, tinted by the hand rules of cold, height and sand, and its bed
// where it shows; snow `snow` over it; its stones, as far as they stand out of the snow; and over all the plants that
// stand out of it (its cover, as far as
// liquid over their roots leaves them, `above_water`; the cover's field holds how the snow buries them and how the bed
// holds them). The share of it that is bare, and its map class.
struct Ground { rgb: vec3f, bare: f32, kind: u32 }
fn ground_of(p: Spot, snow: f32) -> Ground {
  var g: Ground;
  g.rgb = p.ground;
  g.kind = BARE;
  g.rgb = mix(g.rgb, TUNDRA, ease(278.0, 266.0, p.t) * 0.7);
  g.rgb = mix(g.rgb, UPLAND, ease(1200.0, 2600.0, p.z) * 0.5);
  // In the style a beach is the loose ground's own colour lightened toward washed sand (`BEACH_OWN`).
  g.rgb = mix(g.rgb, select(SAND, mix(BEACH_SAND, min(p.ground * BEACH_LIFT, vec3f(1.0)), BEACH_OWN), styled()), p.sand);
  // In the style the soil has its grain (crag.wgsl `floor_of`). Where the loose ground thins its bed shows, in the
  // style as slabs or, where steep, a cliff's face (`rock_face`); it holds plants only in its cracks. Slope alone
  // never bares the ground.
  let steep = p.slope / earth.look.x;
  var bed = p.bed;
  if (styled()) {
    g.rgb = floor_of(g.rgb, p.e, p.canopy, p.wet) * soil_detail(p);
    if (p.rock > 0.0) { bed = rock_face(p.bed, p.z, p.at, steep, p.e, p.shade) * rock_detail(p.e); }
  }
  g.rgb = mix(g.rgb, bed, p.rock);
  if (p.rock > 0.5) { g.kind = ROCK; }
  let plants = p.cover.a * above_water(p.depth);
  g.bare = 1.0 - plants;
  g.rgb = mix(g.rgb, SNOW, snow);
  // Snow is not bare soil: it takes neither the soil's warmth nor its darkening when wet (`soil`).
  g.bare *= 1.0 - snow;
  if (snow > 0.5) { g.kind = ICE; }
  // Stones over it (stone.wgsl): scree, on bare bed, and in a channel; none on a beach.
  if (styled()) {
    let thin = 1.0 - ease(0.0, SCREE_LOOSE_M, p.loose);
    let scree = max(SCREE * ease(SCREE_FROM, SCREE_FULL, steep) * thin * thin * p.below, max(STONES_ON_BED * p.rock, p.blocks)) * (1.0 - p.sand);
    let sunk = SUNK_MOST * ease(0.0, SUNK_LOOSE_M, p.loose);
    // Dry open ground is strewn with stones in fields and strings (a desert's pavement), the drier and the thinner its
    // loose ground the more, as far as the plants leave it bare.
    let dry = ease(DRY_STONES_WET, 0.0, p.wet) * (1.0 - p.sand) * (1.0 - plants) * (1.0 - min(p.canopy, 1.0));
    var field = 0.0;
    if (dry > 0.0 && !unseen(DRY_STONE_M)) { field = smoothstep(-0.2, 0.6, noise(p.at / DRY_FIELD_M, 62).x + 0.6 * thin); }
    // The scree's angular blocks, the channel's worn stones, then the dry ground's, from one place in the code.
    for (var k = 0; k < 3; k++) {
      var s = Stony(scree * (1.0 - plants), SCREE_STONE_M, 1.0, 0.0, sunk, snow, p.snow_m, STONE_RARE);
      if (k == 1) { s = Stony(p.stony * (1.0 - p.sand), p.stone, p.small, p.worn, BED_SUNK, snow, p.snow_m, 0.0); }
      if (k == 2) { s = Stony(DRY_STONES * dry * field, DRY_STONE_M, 1.0, DRY_WORN, sunk, snow, p.snow_m, STONE_RARE); }
      let st = stones(p.e, s, 0.0, p.bed, 61 + 8 * k);
      g.rgb = mix(g.rgb, st.rgb / max(st.a, 1e-4), st.a);
    }
  }
  if (plants > 0.0) { g.rgb = mix(g.rgb, p.cover.rgb / p.cover.a, plants); }
  if (plants >= 0.5) { g.kind = PLANTS; }
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

// Whether the colour of spot p reads its ground: always over the liquid; under it, where any of its bed shows through
// (no bed shows through water deeper than shows any of it).
fn reads_ground(p: Spot) -> bool {
  if (p.depth <= 0.0) { return true; }
  if (styled()) { return exp(-2.0 * CLEAR_ATTENUATION.b * p.depth) > SHOWS; }
  return through(p.depth).b > SHOWS;
}

// The ground spot p's colour reads (`ground_of`), the bed of running water where `running` (always read): with its
// snow over the liquid, without it under the liquid (frozen liquid lies on the surface); black where it is not read.
// Each pixel's colours read it at one place in the code (land.wgsl `fs`), so its rock and stones are compiled once.
fn ground_for(p: Spot, running: bool) -> Ground {
  if (!running && !reads_ground(p)) { return Ground(vec3f(0.0), 1.0, BARE); }
  return ground_of(p, select(p.snow, 0.0, p.depth > 0.0 || running));
}

// The light on spot p over the light on flat ground, in the view drawn (`light`, in the style `relief_light`).
fn lit_at(p: Spot) -> f32 {
  let x = 1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow);
  return select(light(x, p.slope), relief_light(x), styled());
}

fn colour(p: Spot) -> Shaded { return colour_on(p, ground_for(p, false)); }

// The colour of spot p over its ground g (`ground_for`), lit.
fn colour_on(p: Spot, g: Ground) -> Shaded {
  let lit = lit_at(p);
  // Under the liquid: its bed, soaked, seen through it, flat: at no depth it is the soaked ground with its film of
  // liquid.
  if (p.depth > 0.0) {
    let bed = soil(g, 1.0, lit);
    let ripple = ripples(p.e) * (1.0 - p.snow);
    var water: vec3f;
    if (styled()) {
      // The style: clear water over a pale bed, deepening to the open water, the sea's foam at its edge (style.wgsl;
      // a lake's small waves make none, and on its fine shore it would only fleck).
      // One water: standing liquid is coloured by the channel's law (style.wgsl `river_seen`), its shore the
      // channel's shallows and its open water the channel's deep thread, so a river runs into a lake or the sea with
      // no step of colour; the sea by its own depth, a lake as `LAKE_SCALE` times as deep.
      let seen = river_seen(p.depth * mix(1.0, 1.0 / LAKE_SCALE, p.sea), shore_rim(p.depth, p.slope / earth.look.x, p.e, LAKE_RIM_M));
      water = mix(water_painted(BED_SHOWN * bed, seen, ripple), FOAM_RGB, p.sea * (1.0 - p.mouth) * foam(p.depth, p.slope / earth.look.x, p.e, p.at));
    } else {
      water = water_over(bed, p.depth, ripple);
    }
    // Ice where the cells' share of it lies (ice.wgsl `ice_cover`), as floes at its margin.
    let ice = ice_cover(p.snow, p.depth, p.at, p.e);
    let rgb = ice_on(water, ice, p.depth, p.at, p.e);
    var kind = select(LAKE, select(SHALLOW, DEEP, p.depth > DEEP_SEA_M), p.sea >= 0.5);
    if (ice > 0.5) { kind = ICE; }
    // The true view shades the bed's colours as far as the bed shows.
    return Shaded(rgb, 1.0 + (lit - 1.0) * through(p.depth).g, kind);
  }
  // Over it: the ground, wet by how soaked it is, a film of liquid shining on its bare part once it is near soaked
  // (mud).
  let s = soaked(p);
  let film = ease(FILM_FROM, 1.0, s) * g.bare;
  var rgb = soil(g, s, lit);
  let river = p.river * (1.0 - 0.6 * p.snow);
  if (film <= 0.0 && river <= 0.0) { return Shaded(rgb, lit, g.kind); }
  let ripple = ripples(p.e);
  rgb += film * surface_light(ripple * film);
  var kind = g.kind;
  // A river: its water over its own bed. Running water carries the fine soil away: its bed is washed sand and gravel,
  // soaked.
  if (river > 0.0) { rgb = mix(rgb, water_over(SAND * (1.0 - WET_DARK) * lit, p.river_depth, ripple), river); }
  if (river > 0.5) { kind = RIVER; }
  return Shaded(rgb, lit, kind);
}
// Shaded colour `c` in the view drawn, as the screen's bytes (0..255), where the byte of the layer it draws is `o`
// (0..1) and the true colour is `truth` (reflectance, 0..1): the Earth's colours; the true colour, shaded; or a layer's
// ramp over a little of the Earth's colours (where the layer is empty, the Earth's colours faded).
fn in_view_of(c: Shaded, o: f32, truth: vec3f) -> vec3f {
  var rgb = 255.0 * srgb(c.rgb);
  if (styled()) {
    rgb = 255.0 * painted(srgb(c.rgb));
  } else if (earth.view.x == 1.0) {
    rgb = 255.0 * srgb(truth * clamp(1.0 + (c.m - 1.0) * 0.7, 0.55, 1.4));
  } else if (earth.view.x == 2.0) {
    let full = clamp(255.0 * o / earth.look.w, 0.0, 1.0);
    let grey = dot(rgb, vec3f(0.3, 0.59, 0.11));
    let mo = 0.72 * (0.75 + 0.25 * c.m);
    rgb += ((rgb * 0.5 + grey * 0.5) * 0.6 + 100.0 - rgb) * (1.0 - full) + (ramp_layer(o) * mo - rgb * 0.72) * full;
  }
  return rgb;
}

// The metres of the world a pixel of the target takes where it shows uv of the window.
fn metres_a_pixel(uv: vec2f) -> f32 {
  return max(length(dpdx(uv) * earth.place.zw), length(dpdy(uv) * earth.place.zw));
}
