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
  // ramp; the sea's level, m (the live map: the lowest body's); and where the layer is of the soil (drawn on dry
  // ground alone, `in_view_of`) 1, or from 2 on which of the layers the live map finds at each point (live_map.wgsl
  // `layer_at`, earth.ts `GROUND_LAYERS`).
  view: vec4f,
  // Where the map drawn lies in the world: its top-left corner and its size, m.
  place: vec4f,
  // Where a pixel of the target lies in the world (land.wgsl): the origin of the draw, m (a whole multiple of 1024 m,
  // field-window.ts `frameOf`), the metres a pixel of the target takes and the metres a pixel of the screen takes,
  // which the detail drawn is chosen by; and the world's point under the target's first pixel, m from the origin.
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
// wide where the shore's load lies deep and thin where little lies, and at least as far as the storms' swash
// (`STORM_RUN_UP`), which works it.
const BEACH_SLOPE = 0.15; const ROCK_SLOPE = 0.35; const BEACH_LOOSE_M = 0.5; const LAKE_BEACH = 0.3; const BEACH_DEEP_M = 4.0;
// Water standing deeper than this, m, is the deep sea's.
const DEEP_SEA_M = 200.0;
// Soaked bare ground reflects this much less than dry (Lobell and Asner 2002: about half); a film of liquid starts to
// shine on it from this share of soaking on.
const WET_DARK = 0.5;
const FILM_FROM = 0.8;
// A bed that sends back less than this share of its light (half a display level of 255) does not show.
const SHOWS = 0.002;
// The share of beach sand on ground `sea_over` m over the sea's surface and `over` m over the nearest liquid's, `sea` of
// its shore the sea's, of slope `slope` (rise over run) on `loose` m of loose ground: sand by the sea up to `high` m
// over it and under it (the shallows' bed), by a lake up to `LAKE_BEACH` of that over its level, on gentle loose
// ground only, and at least up to the storms' swash of its waves `waves` (water.wgsl `waves_of`). A body that holds
// no liquid has no level (tape/src/table.rs), so no dry basin is drawn as sand.
fn beach(sea: f32, sea_over: f32, over: f32, slope: f32, loose: f32, high: f32, waves: vec2f) -> f32 {
  let reach = high * max(ease(0.0, BEACH_DEEP_M, loose), 1e-3);
  let storm = STORM_RUN_UP * run_up(waves, slope);
  let shore = max(sea * (1.0 - ease(0.0, max(reach, storm), sea_over)), (1.0 - sea) * (1.0 - ease(0.0, max(LAKE_BEACH * reach, storm), over)));
  return shore * (1.0 - ease(BEACH_SLOPE, ROCK_SLOPE, slope)) * ease(0.0, BEACH_LOOSE_M, loose);
}

// What a shore is made of, read from what the page knows of it. A beach's slope is set by its grain: fine sand rests
// near 0.02, coarse sand near 0.1, pebbles from about 0.15 (Bascom 1951), so a shore steeper than `GRAVEL_SLOPE.x` is of
// pebbles by `GRAVEL_SLOPE.y`, and a channel that carries stones (`Spot.stony`, `STONY_PEBBLES` of its places holding
// one) lays them at its mouth; a lake's flattest shores (from `MUD_SLOPE.x` down to `.y`), where its small waves do not
// winnow the fines out, are of mud. The share of pebbles, of mud, and how soaked the shore is (by the liquid's fringe
// and the swash; under the liquid, all).
// A shore of a fine deposit (`laid_share`) is mud too, as far as its waves are lower than `WINNOW_M`: a wave stirs the
// fines off its bed and the backwash carries them out, so mud flats lie where the waves are small (from memory).
const GRAVEL_SLOPE = vec2f(0.07, 0.14); const STONY_PEBBLES = 0.3; const MUD_SLOPE = vec2f(0.03, 0.01); const WINNOW_M = 0.5;
fn shore_grain(p: Spot) -> vec3f {
  let slope = p.slope / earth.look.x;
  let pebbles = max(ease(GRAVEL_SLOPE.x, GRAVEL_SLOPE.y, slope), ease(0.0, STONY_PEBBLES, p.stony));
  let fine_laid = laid_share(p) * (1.0 - sandy_of(p.grain)) * (1.0 - ease(0.0, WINNOW_M, waves_of(p.sea, p.fetch).x));
  let mud = (1.0 - pebbles) * max((1.0 - p.sea) * ease(MUD_SLOPE.x, MUD_SLOPE.y, slope), fine_laid);
  return vec3f(pebbles, mud, select(max(soaked(p), p.swash.x), 1.0, p.depth > 0.0));
}
// Loose ground of coarse grain is sand: the grain's share from `DRIFT_COARSE`, all of it from `DRIFT_COARSE_FULL`. How
// far loose ground of grain `grain` (`Spot.grain`) is sand, 0..1.
const DRIFT_COARSE = 0.45; const DRIFT_COARSE_FULL = 0.8;
fn sandy_of(grain: vec4f) -> f32 { return ease(DRIFT_COARSE, DRIFT_COARSE_FULL, grain.x); }

// The load the liquid laid (`Spot.deposit`) shows as a deposit from `LAID_M.x` deep, all of it from `.y` (thinner, it
// is the creep that fills the foot of a slope, and the ground's own soil is worked into it), as far as the water
// reaches it: all of it to `FLOOD_FULL` of `FLOOD_M` over the nearest water, standing or running, none from `FLOOD_M`
// (`over_level`; a river's floods rise a metre or three over its banks and a storm's surge as much over the sea, from
// memory: a delta's top is its floods' surface). Higher, the load laid long ago is soil.
const LAID_M = vec2f(0.5, 3.0); const FLOOD_M = 3.0; const FLOOD_FULL = 0.5;
// How high spot p lies over the nearest water, m: over the liquid about it (`Spot.over`, known by the ponds' shores
// and a channel's banks), and no higher than over the sea's level, which a low coast stands over however far from its
// shore.
fn over_level(p: Spot) -> f32 { return min(p.over, max(p.z - earth.view.z, 0.0)); }
// The share of spot p's ground that is a deposit, 0..1.
fn laid_share(p: Spot) -> f32 {
  return ease(LAID_M.x, LAID_M.y, p.deposit.x) * ease(FLOOD_M, FLOOD_FULL * FLOOD_M, over_level(p));
}
// A deposit is fresh (bare, its ripples sharp) where the water still works it, within `REWORK_M` of its surface, and
// where its loose ground has lain no more than 10^`FRESH_LOG_YEARS.x` years, old from 10^`.y` (plants and a soil take
// a bare deposit in decades to centuries; from memory). How fresh spot p's deposit is, 0..1.
const REWORK_M = 0.3; const FRESH_LOG_YEARS = vec2f(1.5, 3.5);
fn laid_fresh(p: Spot) -> f32 {
  return max(ease(REWORK_M, 0.0, over_level(p)), ease(FRESH_LOG_YEARS.y, FRESH_LOG_YEARS.x, p.deposit.y));
}
// The share of spot p's ground that is a fresh deposit (`laid_share`, `laid_fresh`), 0..1.
fn laid_fresh_share(p: Spot) -> f32 { return laid_share(p) * laid_fresh(p); }
// A deposit's liquid stands at its surface where it lies no higher over the water than the liquid rises up its pores
// over the water table, the capillary fringe: about `MARSH_RISE_M.x` in fine grain (silt rises 1 to 3 m), `.y` in sand
// (Bear 1972, from memory), all of it soaked to its surface from `MARSH_FULL` of that down. So a low delta, a flood
// plain and a lagoon's shore are a marsh: soaked among their plants as peat is (`peat_of`), its pools between them
// (`puddles`), and the plants that stand in wet ground stand there (cover_field.wgsl `Beneath.soaked`). How far spot p
// is a marsh, 0..1.
const MARSH_RISE_M = vec2f(2.5, 0.3); const MARSH_FULL = 0.6;
fn marsh_of(p: Spot) -> f32 {
  let rise = mix(MARSH_RISE_M.x, MARSH_RISE_M.y, sandy_of(p.grain));
  return laid_share(p) * ease(rise, MARSH_FULL * rise, over_level(p));
}
// How far spot p's liquid stands at its surface among its plants: peat, or a deposit's marsh.
fn wet_through(p: Spot) -> f32 { return max(peat_of(p), marsh_of(p)); }
// The grain of spot p's soil as it is seen (`Spot.grain`): a fresh deposit holds next to no organic matter yet, so
// it is not darkened by it (style.wgsl `humus`).
fn grain_seen(p: Spot) -> vec4f { return vec4f(p.grain.xy, p.grain.z * (1.0 - laid_fresh_share(p)), p.grain.w); }

// Bare sand reaches up a beach to `STORM_RUN_UP` times the waves' run-up (water.wgsl `run_up`): a storm's swash keeps
// plants and their drift off it (its waves some three times as high).
const STORM_RUN_UP = 3.0;

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
  // The share of the ground the tallest plants' crowns close over it (their litter lies under them), the share of it
  // and the ground about it under woody crowns whose leaves fall this month (cover_field.wgsl `CoverAt.shed`), and the
  // share of its plants' cover of kinds that stand in liquid there and the depth they stand in, m (`CoverAt.wet`), and
  // the share of the liquid's surface the herbs that stand in it deeper than they are tall cover with their floating
  // leaves (land.wgsl).
  canopy: f32, shed: f32, wading: f32, reach: f32, pads: f32,
  // The fetch of the lake beside it, m (water.wgsl `waves_of`; 0 where none is known: no waves).
  fetch: f32,
  // How soaked the waves' swash leaves it, and the share of it the foam of the last bore covers, 0..1 (`swash_of`).
  swash: vec2f,
  // The largest stone the point's channel moves, m (0: none), and the share of woody crowns about a channel's bar,
  // whose dead wood the floods leave on it, 0..1.
  moved: f32, driftwood: f32,
  // Where it lies in the world, m, and the metres a pixel takes there (for the ripples of a liquid's surface).
  at: vec2f, e: f32,
  // Its loose ground's grain: the shares of coarse and of fine grain, of organic matter, and how slowly its pores pass
  // liquid (the decimal logarithm of 1 m/s over their conductivity; tape/src/ground/rock.rs); and the share of its
  // pores the soil's liquid fills, 0..1 (land.wgsl `State`); how deep the load the liquid laid lies on it, m, and the
  // decimal logarithm of the years its loose ground has lain (rock.wgsl `soil_at`).
  grain: vec4f, filled: f32, deposit: vec2f,
  // How low it lies among the hollows of its ground, 0..1, even (relief.wgsl `Relief`), and its change across a pixel
  // along the screen's axes; how low it lies among the ground's basins, 0..1, even; the share of the ground the low
  // plants cover there on their mean (cover_field.wgsl `CoverAt.low`).
  hollow: f32, hollow_d: vec2f, basin: f32, low: f32,
  // The share of it under a burn scar, and how much of the scar is char, ash and scorched soil, 0..1 (burn.wgsl
  // `burnt_spot`; none where no program draws the marks of fire).
  scar: vec4f,
}
// A spot's shaded colour (linear light), its shade's multiplier and its map class.
struct Shaded { rgb: vec3f, m: f32, kind: u32 }

// The colour of the ground of spot p, unlit: its own colour, tinted by the hand rules of cold, height and sand, and its bed
// where it shows; snow `snow` over it; its stones, as far as they stand out of the snow; and over all the plants that
// stand out of it (its cover, as far as
// liquid over their roots leaves them on the bed, draw.wgsl `stands_in`; the cover's field holds how the snow buries them and how the bed
// holds them). The share of it that is bare, its map class, the share of its bare part that takes the soil's tint
// (`soil`: a beach's sand keeps its own colour), its loose ground's grain (`Spot.grain`), and the share of it whose
// soil is seen from above: its bare part, and `SWARD_GAP` of its low plants, the gaps between their leaves; and how far
// its liquid stands at its surface among its plants (`wet_through`: peat, a marsh).
struct Ground { rgb: vec3f, bare: f32, kind: u32, warm: f32, grain: vec4f, seen: f32, peat: f32 }
// The share of the ground under low plants seen from above through the gaps between their leaves: exp(-0.5 LAI) for a
// sward of leaf area index 2 (Monsi and Saeki 1953). Crowns of trees hide their floor (`Spot.canopy`).
const SWARD_GAP = 0.35;
// The share of the ground of spot p, whose share `plants` is plants, seen from above (`Ground.seen`).
fn seen_of(p: Spot, plants: f32) -> f32 { return 1.0 - plants + SWARD_GAP * max(plants - min(p.canopy, 1.0), 0.0); }
// Ground whose pores stay full and whose soil is rich in organic matter is peat: its liquid stands at its surface, among
// its low plants as in its open ground, so its soil seen is soaked and its film and pools lie between the plants
// (`puddles`). How far spot p is peat, 0..1: as its pores are full from `PEAT_FILLED` to all, times how dark its organic
// matter makes its soil (style.wgsl `humus`).
const PEAT_FILLED = 0.9;
fn peat_of(p: Spot) -> f32 { return ease(PEAT_FILLED, 1.0, p.filled) * humus(p.grain.z); }
// The share of ground g whose soil shows soaked: its bare part, on peat its soil seen.
fn wets(g: Ground) -> f32 { return mix(g.bare, g.seen, g.peat); }
// The share of the plants of spot p that the waves strip where their swash soaks the ground `soak` (`swash_of`; all
// of it under the liquid): as much as its waves are high against the sea's.
fn waves_strip(p: Spot, soak: f32) -> f32 { return min(soak / SWASH_DRY, 1.0) * waves_of(p.sea, p.fetch).x / SEA_WAVE.x; }

fn ground_of(p: Spot, snow: f32) -> Ground {
  var g: Ground;
  g.rgb = p.ground;
  g.kind = BARE;
  g.warm = 1.0;
  g.grain = grain_seen(p);
  // Ground the plants cover whole shows none of its soil, bed, snow, stones or things: none of them is drawn. The
  // waves' swash (`swash_of`) strips the plants where it runs as far as its waves are high against the sea's: the sea's
  // swash bares its beach, a lake's small waves leave its plants standing to the water.
  let stripped = waves_strip(p, p.swash.x);
  let plants = p.cover.a * stands_in(p.depth, 0.0) * (1.0 - stripped);
  if (plants >= 1.0) { return Ground(p.cover.rgb / p.cover.a, 0.0, PLANTS, 1.0, g.grain, seen_of(p, 1.0), wet_through(p)); }
  g.rgb = mix(g.rgb, TUNDRA, ease(278.0, 266.0, p.t) * 0.7);
  g.rgb = mix(g.rgb, UPLAND, ease(1200.0, 2600.0, p.z) * 0.5);
  // In the style a beach of sand is the loose ground's own colour lightened toward washed sand (`BEACH_OWN`); of pebbles,
  // the colour of the bed they are worn from; of mud, the loose ground's own (`shore_grain`).
  if (p.sand > 0.0) {
    let grain = shore_grain(p);
    let sand = select(SAND, mix(BEACH_SAND, min(p.ground * BEACH_LIFT, vec3f(1.0)), BEACH_OWN), styled());
    g.rgb = mix(g.rgb, mix(sand, p.bed, grain.x), p.sand * (1.0 - grain.y));
    g.warm = 1.0 - p.sand * (1.0 - grain.y);
  }
  // In the style the soil has its grain (crag.wgsl `floor_of`). Where the loose ground thins its bed shows, in the
  // style as slabs or, where steep, a cliff's face (`rock_face`); it holds plants only in its cracks. Slope alone
  // never bares the ground.
  let steep = p.slope / earth.look.x;
  var bed = p.bed;
  if (styled()) {
    // A burn scar takes the floor (burn.wgsl `burnt_floor`) and as much of the soil's own tint.
    let burnt = burnt_floor(floor_of(g.rgb, p.e, p.canopy, p.wet, g.grain) * soil_detail(p), p);
    g.rgb = burnt.rgb;
    g.warm *= 1.0 - burnt.a;
    // Bed the waves' swash wets takes the one wet look of rock (stone.wgsl `wet_stone`) as far as it is soaked.
    if (p.rock > 0.0) {
      bed = rock_face(p.bed, p.z, p.at, steep, p.e, p.shade) * rock_detail(p.e);
      if (p.swash.x > 0.0) { bed = mix(bed, wet_stone(bed, p.shade, 1.0, 0.0), p.swash.x); }
    }
  }
  g.rgb = mix(g.rgb, bed, p.rock);
  if (p.rock > 0.5) { g.kind = ROCK; }
  g.bare = 1.0 - plants;
  g.rgb = mix(g.rgb, SNOW * select(vec3f(1.0), snow_texture(p.e), styled()), snow);
  // Snow is not bare soil: it takes neither the soil's warmth nor its darkening when wet (`soil`).
  g.bare *= 1.0 - snow;
  g.seen = seen_of(p, plants) * (1.0 - snow);
  g.peat = wet_through(p) * (1.0 - snow);
  if (snow > 0.5) { g.kind = ICE; }
  // Stones over it (stone.wgsl): scree, on bare bed, and in a channel; none on a beach.
  if (styled()) {
    let thin = 1.0 - ease(0.0, SCREE_LOOSE_M, p.loose);
    let scree = max(SCREE * ease(SCREE_FROM, SCREE_FULL, steep) * thin * thin * p.below, max(STONES_ON_BED * p.rock, p.blocks)) * (1.0 - p.sand);
    let sunk = SUNK_MOST * ease(0.0, SUNK_LOOSE_M, p.loose);
    // The scree's angular blocks, then the channel's worn stones, from one place in the code, as wet as the swash leaves them
    // (and a channel's, as its ground is soaked). (Dry open ground's
    // stones are the things on the ground, detail.wgsl `objects`.)
    for (var k = 0; k < 2; k++) {
      var s = Stony(scree * (1.0 - plants), SCREE_STONE_M, 1.0, 0.0, sunk, snow, p.snow_m, STONE_RARE, p.swash.x);
      if (k == 1) { s = Stony(p.stony * (1.0 - p.sand), p.stone, p.small, p.worn, BED_SUNK, snow, p.snow_m, 0.0, max(soaked(p), p.swash.x)); }
      let st = stones(p.e, s, 0.0, p.bed, 61 + 8 * k);
      g.rgb = mix(g.rgb, st.rgb / max(st.a, 1e-4), st.a);
    }
  }
  // Mats of lichen and moss, and things lying on the ground (detail.wgsl `objects`), under the low plants.
  if (styled()) { g.rgb = objects(dry_filler(g.rgb, p, 1.0 - plants), p, 1.0 - plants); }
  if (plants > 0.0) { g.rgb = mix(g.rgb, p.cover.rgb / p.cover.a, plants); }
  if (plants >= 0.5) { g.kind = PLANTS; }
  return g;
}

// How soaked the ground of spot p is, 0..1: as wet as the field says, and wetter the nearer it lies over the liquid's
// surface (the capillary fringe); soaked under it.
fn soaked(p: Spot) -> f32 { return 1.0 - (1.0 - p.wet) * (1.0 - exp(-p.over / FRINGE_M)); }

// Waves run up a shore past the still level and soak it to their run-up (water.wgsl `run_up`), the run-up ragged along
// the shore by `SWASH_VARY` of itself over `SWASH_VARY_M` (the waves' groups) and a third of that over a fifth of it:
// the sand is soaked at the water's edge and drier up the swash, `SWASH_DRY` soaked at its top, where it dries out over
// the last `SWASH_EDGE` of it; the film on it lies `SWASH_CALM` calmer than standing liquid. The last bore leaves foam
// up `BORE_REACH` of the run-up: a soft band at the water's edge `BORE_EDGE_M` wide, broken where the clumps' noise
// falls (`BORE_EDGE` of it white), and small clumps where two octaves of noise `BORE_CLUMP_M` across pass a level that
// rises from `BORE_DENSE` at the water's edge to `BORE_SPARSE` at the band's top, over `BORE_SOFT` of it, `BORE_FOAM`
// of them white.
const SWASH_VARY = 0.25; const SWASH_VARY_M = 12.0; const SWASH_DRY = 0.4; const SWASH_EDGE = 0.25;
const SWASH_CALM = 0.8;
const BORE_REACH = 0.15; const BORE_CLUMP_M = 0.7; const BORE_DENSE = 0.75; const BORE_SPARSE = 1.2;
const BORE_SOFT = 0.3; const BORE_FOAM = 0.5; const BORE_EDGE = 0.3; const BORE_EDGE_M = 0.8;
// The swash on ground `over` m over standing liquid (`NO_LIQUID_M` far from any), `sea` of it the sea's, of slope
// `slope` (rise over run), at `at` seen at `e` m a pixel, under snow `snow`: how soaked it leaves the ground, and the
// share of it the last bore's foam covers (from afar, where its clumps do not show, their mean).
fn swash_of(over: f32, waves: vec2f, slope: f32, at: vec2f, e: f32, snow: f32) -> vec2f {
  let reach = run_up(waves, slope);
  if (over >= (1.0 + 1.4 * SWASH_VARY) * reach) { return vec2f(0.0); }
  let ragged = noise_turned(at / SWASH_VARY_M, 111) + 0.35 * noise_turned(at / (0.2 * SWASH_VARY_M), 113);
  let up = over / (reach * (1.0 + SWASH_VARY * ragged));
  let wet = mix(1.0, SWASH_DRY, min(up, 1.0)) * (1.0 - ease(1.0 - SWASH_EDGE, 1.0, up));
  let band = 1.0 - ease(0.0, BORE_REACH, up);
  let edge = BORE_EDGE / BORE_FOAM * (1.0 - ease(0.0, max(BORE_EDGE_M, e), over / max(slope, 1e-3)));
  var foam = 0.3 * edge;
  if (band > 0.0 && !unseen(BORE_CLUMP_M)) {
    let level = mix(BORE_SPARSE, BORE_DENSE, band);
    let n = noise_l(lat_turned(BORE_CLUMP_M, vec2f(0.0)), 112).x + 0.4 * noise_l(lat_turned(0.35 * BORE_CLUMP_M, vec2f(0.5)), 114).x;
    let clumps = smoothstep(level, level + BORE_SOFT, n);
    let shows = shows_at(BORE_CLUMP_M, e);
    foam = max(band * mix(0.2 * (1.0 - level), clumps, shows), edge * mix(0.3, ease(0.1, 0.6, n), shows));
  }
  return vec2f(wet, foam) * (1.0 - snow);
}

// The true colour `truth` of what covers spot p, its ground darker as far as it is soaked (`WET_DARK`), where no
// liquid stands over it and as far as no snow lies on it: the liquid in its pores is part of the ground, not a surface.
fn true_soaked(truth: vec3f, p: Spot) -> vec3f {
  if (p.depth > 0.0) { return truth; }
  return truth * (1.0 - WET_DARK * soaked(p) * (1.0 - p.snow));
}

// Ground g soaked to s, lit by `lit` (its shade's multiplier): its bare part (on peat its soil seen, `wets`) the darker the more soaked (and in the
// style tinted by its grain as far as it takes the tint, and darkened by its organic matter as far as its soil is seen,
// style.wgsl `bare_tint`).
fn soil(g: Ground, s: f32, lit: f32) -> vec3f {
  let tint = select(vec3f(1.0), bare_tint(g.bare * g.warm, g.seen * g.warm, g.grain), styled());
  return lit_by(g.rgb * (1.0 - WET_DARK * s * wets(g)) * tint, lit);
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
  if (!running && !reads_ground(p)) { return Ground(vec3f(0.0), 1.0, BARE, 1.0, p.grain, 1.0, 0.0); }
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
      // The style: clear water over its bed, deepening to the open water (a lake's greener, `LAKE_CLEAR`), foam at its edge where the waves break
      // (style.wgsl `foam`): a lake's, of smaller waves, breaking shallower, as much less as its waves are lower.
      // Standing liquid is coloured as a channel is (style.wgsl `standing_seen`), its shore the channel's shallows and
      // its open water past the band deeper; the sea by its own depth, a lake as `LAKE_SCALE` times as deep.
      let seen = standing_seen(p.depth * mix(1.0, 1.0 / LAKE_SCALE, p.sea), shore_rim(p.depth, p.slope / earth.look.x, p.e, LAKE_RIM_M));
      let waves = max(waves_of(p.sea, p.fetch).x / SEA_WAVE.x, 1e-3);
      water = mix(standing_painted(BED_SHOWN * bed, seen, ripple, p.sea), FOAM_RGB, waves * (1.0 - p.mouth) * foam(p.depth / waves, p.slope / earth.look.x, p.e, p.at));
    } else {
      water = water_over(bed, p.depth, ripple);
    }
    // What floats on still water (afloat.wgsl), and the plants that stand in it down to their line
    // (draw.wgsl `stands_in`), over it as on land, as far as its waves leave them (`waves_strip`).
    water = afloat(water, p);
    // Under the crowns of the trees that stand in it as the field draws them the water lies in their shadow, as the
    // ground does (draw.wgsl `CROWN_SHADOW`); near, the trees drawn one by one cast their own (models.wgsl).
    let standing = min(p.cover.a, 1.0) * p.wading * stands_in(p.depth, p.reach) * (1.0 - waves_strip(p, SWASH_DRY));
    water *= 1.0 - CROWN_SHADOW * min(p.canopy, 1.0) * standing;
    if (standing > 0.0) { water = mix(water, lit_by(p.cover.rgb / p.cover.a, lit), standing); }
    // Ice where the cells' share of it lies (ice.wgsl `ice_cover`), as floes at its margin.
    let ice = ice_cover(p.snow, p.depth, p.at, p.e);
    let rgb = ice_on(water, ice, p.depth, p.at, p.e);
    var kind = select(LAKE, select(SHALLOW, DEEP, p.depth > DEEP_SEA_M), p.sea >= 0.5);
    if (ice > 0.5) { kind = ICE; }
    // The true view shades the bed's colours as far as the bed shows.
    return Shaded(rgb, 1.0 + (lit - 1.0) * through(p.depth).g, kind);
  }
  // Over it: the ground, wet by how soaked it is and by the swash (peat soaked through as far as it is peat,
  // `peat_of`), a film of liquid on its bare part once it is near soaked (mud, the swash by the water; on peat among
  // its plants too), puddles in its hollows (`puddles`), and the foam of the swash's last bore.
  let s = mix(max(soaked(p), p.swash.x), 1.0, g.peat);
  let film = ease(FILM_FROM, 1.0, s) * mix(g.bare, 1.0, g.peat);
  var rgb = mix(soil(g, s, lit), FOAM_RGB * lit, BORE_FOAM * p.swash.y * g.bare);
  let river = p.river * (1.0 - 0.6 * p.snow);
  if (film <= 0.0 && river <= 0.0) { return Shaded(rgb, lit, g.kind); }
  // A film on soaked ground lies still and mirrors the sky; the swash's sheet runs, rippled `SWASH_CALM` less than
  // standing liquid. A river's water is rippled.
  let ripple = select(vec2f(0.0), ripples(p.e), p.swash.x > 0.0 || river > 0.0);
  if (film > 0.0) {
    rgb += film * surface_light(ripple * p.swash.x * (1.0 - SWASH_CALM));
    let pud = puddles(p, film, g.peat);
    if (pud.wet > 0.0) {
      // About a puddle the mud is soaked through, darker, and its film mirrors the sky as the map's view sees it.
      let mud = soil(g, 1.0, lit) * (1.0 - PUDDLE_WET_DARK);
      rgb = mix(rgb, mud + surface_seen(vec2f(0.0), MAP_VIEW), pud.wet);
      if (pud.water > 0.0) {
        let deep = lit_by(p.ground * (1.0 - WET_DARK) * MUDDY_WATER * mix(vec3f(1.0), BOG_WATER, peat_of(p)), lit);
        rgb = mix(rgb, seen_from(mud, pud.depth, pud.tilt, PUDDLE_ATTENUATION, deep, MAP_VIEW), pud.water);
      }
    }
  }
  var kind = g.kind;
  // A river: its water over its own bed. Running water carries the fine soil away: its bed is washed sand and gravel,
  // soaked.
  if (river > 0.0) { rgb = mix(rgb, water_over(SAND * (1.0 - WET_DARK) * lit, p.river_depth, ripple), river); }
  if (river > 0.5) { kind = RIVER; }
  return Shaded(rgb, lit, kind);
}
// Puddles: on bare ground soaked past `FILM_FROM` (`film` 0..1 from there to soaked through) where the low plants leave
// it open (a sward's puddles lie under its blades), liquid stands over up to `PUDDLES` of the ground when soaked
// through, gathered where the ground lies low: in the lowest `BASIN_LOW` of its basins (the relief's octaves `BASIN_M`
// long, relief.wgsl `Relief`), the more the lower, and in each basin in the lowest of its hollows (its octaves
// `HOLLOW_M` long) up to the basin's level. So most of the ground holds none, and a basin's lowest part a few large
// puddles of joined hollows among smaller ones. A puddle is `PUDDLE_DEPTH_M` deep for each unit of the hollows' rank
// its point lies under its level; the mud about it is soaked through out to `PUDDLE_HALO` times its level above it in
// the rank, softly; at its waterline its surface curves up the rim (the meniscus and the wet film over it) to a slope
// of `MENISCUS` over the last `PUDDLE_SHORE` of its level. A puddle shows one by one where it takes `PUDDLE_PX.x`
// pixels across, fully from `.y`; its size is about the hollows' length times the root of its level. Where it does not
// show, it and its halo are their mean soaked ground, no water: from afar the puddled ground is a darker wet tone.
// Liquid stands on ground whose pores pass it more slowly than rain falls (infiltration excess, Horton 1933): the
// puddles are as many as said where the ground passes liquid as slowly as `PUDDLE_SLOW` (its `Spot.grain.w`, 6.5:
// 0.3 µm/s, about a mm an hour), none where it passes it as fast as `PUDDLE_FAST` (4.5: 30 µm/s, about 100 mm an hour,
// faster than nearly any rain). On peat (`peat`, `peat_of`) the pools stand among the low plants whatever its pores
// pass, over up to `BOG_POOLS` of it: the liquid stands at its surface, and the hollows between a bog's hummocks hold
// its pools (Belyea and Clymo 2001, from memory: hollows take a third to a half of a bog's surface).
const BOG_POOLS = 0.3;
// A bog's pools are dark with the dissolved organic matter of the peat, which takes most of the light, the blue most:
// their light is `BOG_WATER` of a muddy puddle's (humic waters; Kirk 1994, from memory).
const BOG_WATER = vec3f(0.35, 0.25, 0.15);
const HOLLOW_M = vec2f(0.5, 2.0); const BASIN_M = vec2f(4.0, 20.0); const BASIN_LOW = 0.35;
const PUDDLES = 0.06; const PUDDLE_DEPTH_M = 0.3; const PUDDLE_HALO = 1.0; const PUDDLE_SHORE = 0.5; const MENISCUS = 0.3;
const PUDDLE_PX = vec2f(3.0, 8.0); const PUDDLE_FAST = 4.5; const PUDDLE_SLOW = 6.5;
// The mud about a puddle, soaked through, is `PUDDLE_WET_DARK` darker than the soaked ground. A puddle's water is
// turbid with the mud it stands on: its light attenuates by `PUDDLE_ATTENUATION`, 1/m (turbid ponds pass 10 to 30:
// Kirk 1994, from memory), so its bed shows at its edge and fades into the light the muddy water sends back, the
// soaked ground's own colour times `MUDDY_WATER` (the suspended mud absorbs the blue most): darker and browner.
const PUDDLE_WET_DARK = 0.4; const PUDDLE_ATTENUATION = vec3f(30.0, 26.0, 22.0); const MUDDY_WATER = vec3f(0.6, 0.5, 0.38);
// What of spot p is a puddle's: the share its water covers, its depth, m, and its surface's slope (east, south); the
// share of the soaked mud about it (and under it).
struct Puddle { water: f32, depth: f32, tilt: vec2f, wet: f32 }
fn puddles(p: Spot, film: f32, peat: f32) -> Puddle {
  let stands = mix(ease(PUDDLE_FAST, PUDDLE_SLOW, p.grain.w) * (1.0 - p.low), 1.0, peat);
  let full = mix(PUDDLES, BOG_POOLS, peat) * film * stands * (1.0 - p.sand * (1.0 - shore_grain(p).y));
  // Its basin's level, from the mean where the basins do not show; the mean share it covers is `full`.
  let level = mix(full, full / (0.5 * BASIN_LOW) * ease(BASIN_LOW, 0.0, p.basin), shows_at(BASIN_M.x, p.e));
  let edge = max(0.5 * (abs(p.hollow_d.x) + abs(p.hollow_d.y)), 1e-4);
  // (No puddle where none stands: a pixel's edge never darkens the lowest hollows' points.)
  let any = ease(0.0, edge, level);
  let pool = (1.0 - smoothstep(level - edge, level + edge, p.hollow)) * any;
  let halo = (1.0 - smoothstep(level, (1.0 + PUDDLE_HALO) * level + edge, p.hollow)) * any;
  let shown = shows_at(HOLLOW_M.y, p.e) * ease(PUDDLE_PX.x, PUDDLE_PX.y, HOLLOW_M.y * sqrt(level) / p.e);
  let under = max(level - p.hollow, 0.0);
  let up = normalize(p.hollow_d + vec2f(1e-9, 0.0));
  let tilt = MENISCUS * (1.0 - ease(0.0, PUDDLE_SHORE * level, under)) * up;
  return Puddle(pool * shown, PUDDLE_DEPTH_M * under, tilt, mix(min((1.0 + 0.5 * PUDDLE_HALO) * level, 1.0), halo, shown));
}

// Shaded colour `c` in the view drawn, as the screen's bytes (0..255), where the byte of the layer it draws is `o`
// (0..1) and the true colour is `truth` (reflectance, 0..1): the Earth's colours; the true colour, shaded; or a layer's
// ramp over a little of the Earth's colours (where the layer is empty, the Earth's colours faded; a layer of the soil,
// `earth.view.w` from 1, is empty over standing liquid).
fn in_view_of(c: Shaded, o: f32, truth: vec3f) -> vec3f {
  var rgb = 255.0 * srgb(c.rgb);
  if (styled()) {
    rgb = 255.0 * painted(srgb(c.rgb));
  } else if (earth.view.x == 1.0) {
    rgb = 255.0 * srgb(truth * clamp(1.0 + (c.m - 1.0) * 0.7, 0.55, 1.4));
  } else if (earth.view.x == 2.0) {
    // A layer found at each point is a share known everywhere on dry ground: none of it is a value, not an empty point.
    var full = select(clamp(255.0 * o / earth.look.w, 0.0, 1.0), 1.0, earth.view.w >= 2.0);
    if (earth.view.w >= 1.0 && c.kind <= LAKE) { full = 0.0; }
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
