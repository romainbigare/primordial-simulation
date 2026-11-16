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
// snow rippled by the wind with stones and dry stalks in it, tall wet grass and reeds.
const CARPETS_FROM = 12;
const GRASS_CARPET = 0; const STRAW_CARPET = 1; const LITTER_CARPET = 2; const FERN_CARPET = 3; const SCRUB_CARPET = 4;
const CRACKED_CARPET = 5; const SAND_CARPET = 6; const SNOW_CARPET = 7; const REED_CARPET = 8;
// Each tile's side in the world, m.
fn carpet_m(c: i32) -> f32 { var m = array<f32, 9>(1.5, 1.5, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 2.0); return m[c]; }

// The rendered models (web/textures/models.png), four to a layer from `MODELS_FROM` on (models.wgsl draws the plants
// from them, detail.wgsl the boulders).
const MODELS_FROM = 21;
// The sprites of each kind of plant: the first and how many.
const ROUND_FROM = 0; const ROUNDS = 9; const FIR_FROM = 9; const FIRS = 9; const SHRUB_FROM = 18; const SHRUBS = 4;
const BOULDER_FROM = 22; const BOULDERS = 3; const LOG_FROM = 25; const LOGS = 3;
// Each sprite's content, as a share of its quarter: its width and its height.
fn model_w(s: i32) -> f32 {
  var w = array<f32, 28>(0.879, 0.984, 0.984, 0.984, 0.941, 0.984, 0.984, 0.984, 0.984, 0.387, 0.402, 0.410, 0.453, 0.492,
    0.488, 0.496, 0.441, 0.469, 0.836, 0.863, 0.984, 0.977, 0.984, 0.984, 0.984, 0.984, 0.984, 0.918);
  return w[s];
}
fn model_h(s: i32) -> f32 {
  var h = array<f32, 28>(0.984, 0.941, 0.945, 0.797, 0.984, 0.820, 0.746, 0.930, 0.949, 0.984, 0.984, 0.984, 0.984, 0.984,
    0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.609, 0.984, 0.969, 0.883, 0.746, 0.094, 0.953, 0.984);
  return h[s];
}
// The texel of model sprite s at `uv` in its content (0..1, y down), at `g` of its quarter a pixel: linear, with cover.
fn model_texel(s: i32, uv: vec2f, g: f32) -> vec4f {
  let size = vec2f(model_w(s), model_h(s));
  let q = vec2f(f32(s % 2), f32((s % 4) / 2));
  let at = (q + vec2f(0.5 - 0.5 * size.x, 1.0 - size.y - 2.0 / 256.0) + uv * size) * 0.5;
  let t = textureSampleGrad(detail_tex, detail_sampler, at, MODELS_FROM + s / 4, vec2f(0.5 * g * size.x, 0.0), vec2f(0.0, 0.5 * g * size.y));
  return vec4f(unsrgb(t.rgb), t.a);
}

// Whether the models are loaded (the detail texture is more than its one blank texel).
fn models_ready() -> bool { return textureDimensions(detail_tex).x > 1u; }

