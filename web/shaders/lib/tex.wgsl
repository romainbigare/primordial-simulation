// The textures of the map's detail (web/src/detail.ts): one array of photographs of ground seen from above, of leaves,
// needles and bark, of tiles of ground cover, and of rendered plants four to a layer, each a layer with its mips down
// to its mean; read by the ground (detail.wgsl) and by the plants (crown.wgsl, models.wgsl).

@group(0) @binding(20) var detail_tex: texture_2d_array<f32>;
@group(0) @binding(21) var detail_sampler: sampler;

// The photographs' layers: leaf litter, a mossy floor with twigs, grass, dry stony soil, cracked dry mud, river pebbles,
// beach sand, mossy rock, wet mud; a crown's leaves and a conifer's needles, close together; bark.
const LITTER_TEX = 0; const MOSSY_TEX = 1; const GRASS_TEX = 2; const DRY_TEX = 3; const CRACKED_TEX = 4;
const PEBBLE_TEX = 5; const SAND_TEX = 6; const ROCK_TEX = 7; const MUD_TEX = 8;
const FOLIAGE_TEX = 9; const NEEDLES_TEX = 10; const BARK_TEX = 11;
// The mip level of one texel (512 texels a side).
const DETAIL_TOP = 9.0;

// Photograph t at `uv` (repeating), its step over a pixel along the screen's axes `gx` and `gy` (for its mip): linear.
fn photo(t: i32, uv: vec2f, gx: vec2f, gy: vec2f) -> vec3f {
  return unsrgb(textureSampleGrad(detail_tex, detail_sampler, uv, t, gx, gy).rgb);
}

// Photograph t's mean (its last mip), linear.
fn photo_mean(t: i32) -> vec3f { return unsrgb(textureSampleLevel(detail_tex, detail_sampler, vec2f(0.5), t, DETAIL_TOP).rgb); }

// Photograph t at `uv` over its mean: the change it gives a colour, in lightness alone, its step `g` over a pixel.
fn photo_light(t: i32, uv: vec2f, g: f32) -> f32 {
  return dot(photo(t, uv, vec2f(g, 0.0), vec2f(0.0, g)), LUMA) / max(dot(photo_mean(t), LUMA), 1e-3);
}

// The ground's cover (web/textures/carpets.jpg), each tile rendered from 3D models as the angled view sees it (the
// ground from above, things standing up `STAND` of their height), repeating: green grass, straw, leaf litter with
// sticks and bark, a shaded floor of ferns and herbs on leaves, dry stony soil with dry tufts, cracked dry mud, sand,
// snow rippled by the wind with stones and dry stalks in it, tall wet grass and reeds; and the petals of a meadow's
// flowers alone, grey on black (their lightness: the world gives their colour); short turf and tussocks with bare
// ground between them, green and dry.
const CARPETS_FROM = 12;
const GRASS_CARPET = 0; const STRAW_CARPET = 1; const LITTER_CARPET = 2; const FERN_CARPET = 3; const SCRUB_CARPET = 4;
const CRACKED_CARPET = 5; const SAND_CARPET = 6; const SNOW_CARPET = 7; const REED_CARPET = 8; const FLOWER_CARPET = 9;
const TURF_CARPET = 10; const TUSSOCK_CARPET = 11; const DRY_TURF_CARPET = 12; const DRY_TUSSOCK_CARPET = 13;
// Each tile's side in the world, m.
fn carpet_m(c: i32) -> f32 { var m = array<f32, 14>(1.5, 1.5, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 2.0, 0.8, 2.0, 2.0, 2.0, 2.0); return m[c]; }

// The rendered models: the trees (web/textures/trees.png, sprites 0 to `TREES - 1`) one to a layer from `TREES_FROM`
// on, the rest (web/textures/models.png) four to a layer from `MODELS_FROM` on (models.wgsl draws the plants from them,
// detail.wgsl the things on the ground).
const MODELS_FROM = 26; const MODEL_LAYERS = 11; const TREES_FROM = MODELS_FROM + MODEL_LAYERS; const TREES = 33;
// The sprites of each kind of plant, the first and how many: broad crowns, firs, pines grown, young and small, young
// spruces, quiver trees of dry land (trees, one to a layer); bushes: leafy, of middling ground, and of dry ground
// (`OTHONNA_FROM` and the one after its five).
const ROUND_FROM = 0; const ROUNDS = 6; const JACARANDA_FROM = 6; const JACARANDAS = 3; const FIR_FROM = 9; const FIRS = 9;
const PINE_FROM = 18; const PINES = 3; const PINE_YOUNG_FROM = 21; const PINES_YOUNG = 2; const PINE_SMALL_FROM = 23;
const PINES_SMALL = 3; const SPRUCE_YOUNG_FROM = 26; const SPRUCES_YOUNG = 3; const QUIVER_FROM = 29; const QUIVERS = 4;
const SHRUB_FROM = 33; const SHRUBS = 4; const SEARSIA_FROM = 63; const SEARSIAS = 5; const OTHONNA_FROM = 68;
const OTHONNAS = 5;
// Things on the ground: boulders, a fallen trunk, more rocks, fallen branches, dead bushes; how many of each sort there
// are with the above.
const BOULDER_FROM = 37; const BOULDERS = 3; const LOG_FROM = 40;
const ROCKS_MORE_FROM = 43; const ROCKS = 9; const BRANCH_FROM = 49; const WOODS = 3; const DEAD_BUSH_FROM = 51;
const DEAD_BUSHES = 4;
// Grass tufts, and the mean lightness of their blades (linear).
const TUFT_FROM = 55; const TUFTS = 8; const TUFT_MEAN = 0.048;
// Each sprite's content, as a share of its layer (a tree) or its quarter: its width and its height.
fn model_w(s: i32) -> f32 {
  var w = array<f32, 74>(0.939, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.387, 0.400, 0.410, 0.451, 0.490,
    0.492, 0.496, 0.441, 0.471, 0.430, 0.578, 0.502, 0.525, 0.635, 0.580, 0.729, 0.654, 0.611, 0.658,
    0.766, 0.486, 0.527, 0.629, 0.627, 0.836, 0.863, 0.984, 0.977, 0.984, 0.984, 0.984, 0.984, 0.984,
    0.918, 0.984, 0.984, 0.898, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.754, 0.984,
    0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.980, 0.871, 0.984, 0.867, 0.984, 0.941, 0.633, 0.672,
    0.605, 0.699, 0.680, 0.535);
  return w[s];
}
fn model_h(s: i32) -> f32 {
  var h = array<f32, 74>(0.984, 0.932, 0.949, 0.793, 0.979, 0.779, 0.697, 0.932, 0.953, 0.984, 0.984, 0.984, 0.984, 0.984,
    0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984,
    0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.609, 0.984, 0.969, 0.883, 0.746, 0.094, 0.953,
    0.984, 0.449, 0.426, 0.984, 0.922, 0.465, 0.488, 0.219, 0.266, 0.859, 0.957, 0.836, 0.984, 0.914,
    0.625, 0.746, 0.746, 0.844, 0.672, 0.832, 0.984, 0.984, 0.930, 0.984, 0.867, 0.984, 0.984, 0.984,
    0.984, 0.984, 0.984, 0.984);
  return h[s];
}
// Each sprite's mean cover over its content's box: its alpha's mean there in the strips.
fn model_fill(s: i32) -> f32 {
  var f = array<f32, 74>(0.315, 0.355, 0.344, 0.321, 0.298, 0.285, 0.500, 0.481, 0.437, 0.313, 0.319, 0.316, 0.291, 0.266,
    0.271, 0.112, 0.118, 0.120, 0.255, 0.252, 0.239, 0.221, 0.228, 0.225, 0.203, 0.244, 0.220, 0.208,
    0.238, 0.188, 0.202, 0.372, 0.372, 0.125, 0.129, 0.126, 0.105, 0.643, 0.608, 0.649, 0.647, 0.172,
    0.163, 0.709, 0.716, 0.694, 0.717, 0.693, 0.707, 0.422, 0.316, 0.456, 0.406, 0.422, 0.401, 0.093,
    0.224, 0.188, 0.118, 0.130, 0.247, 0.056, 0.075, 0.338, 0.340, 0.346, 0.302, 0.235, 0.504, 0.448,
    0.430, 0.315, 0.391, 0.364);
  return f[s];
}
// The texel of model sprite s at `uv` in its content (0..1, y down), at `g` of its content a pixel: linear, with cover.
// Each stands on the bottom middle of its layer or quarter, two texels over its edge.
fn model_texel(s: i32, uv: vec2f, g: f32) -> vec4f {
  let size = vec2f(model_w(s), model_h(s));
  var t: vec4f;
  if (s < TREES) {
    let at = vec2f(0.5 - 0.5 * size.x, 1.0 - size.y - 2.0 / 512.0) + uv * size;
    t = textureSampleGrad(detail_tex, detail_sampler, at, TREES_FROM + s, vec2f(g * size.x, 0.0), vec2f(0.0, g * size.y));
  } else {
    let k = s - TREES;
    let q = vec2f(f32(k % 2), f32((k % 4) / 2));
    let at = (q + vec2f(0.5 - 0.5 * size.x, 1.0 - size.y - 2.0 / 256.0) + uv * size) * 0.5;
    t = textureSampleGrad(detail_tex, detail_sampler, at, MODELS_FROM + k / 4, vec2f(0.5 * g * size.x, 0.0), vec2f(0.0, 0.5 * g * size.y));
  }
  return vec4f(unsrgb(t.rgb), t.a);
}

// Whether the models are loaded (the detail texture is more than its one blank texel).
fn models_ready() -> bool { return textureDimensions(detail_tex).x > 1u; }

