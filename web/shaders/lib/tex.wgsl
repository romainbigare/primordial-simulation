// The textures of the map's detail (web/src/detail.ts): one array of photographs of ground seen from above, of tiles of
// ground cover, and of rendered plants four to a layer, each a layer with its mips down to its mean; read by the ground
// (detail.wgsl) and by the plants (models.wgsl).

@group(0) @binding(20) var detail_tex: texture_2d_array<f32>;
@group(0) @binding(21) var detail_sampler: sampler;

// The photographs' layers: river pebbles, mossy rock, wet mud (laid by detail.wgsl `detail`), and a mossy floor with
// twigs.
const PEBBLE_TEX = 0; const ROCK_TEX = 1; const MUD_TEX = 2; const MOSSY_TEX = 3;
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

// The ground's cover (web/textures/carpets.jpg), each tile rendered from 3D models as the map sees it (the
// ground from above, things standing up `STAND` of their height), repeating: green grass, straw, leaf litter with
// sticks and bark, a shaded floor of ferns and herbs on leaves, dry stony soil with dry tufts, cracked dry mud, sand,
// snow rippled by the wind with stones and dry stalks in it, tall wet grass and reeds; and the petals of a meadow's
// flowers alone, grey on black (their lightness: the world gives their colour); short turf and tussocks with bare
// ground between them, green and dry.
const CARPETS_FROM = 4;
const GRASS_CARPET = 0; const STRAW_CARPET = 1; const LITTER_CARPET = 2; const FERN_CARPET = 3; const SCRUB_CARPET = 4;
const CRACKED_CARPET = 5; const SAND_CARPET = 6; const SNOW_CARPET = 7; const REED_CARPET = 8; const FLOWER_CARPET = 9;
const TURF_CARPET = 10; const TUSSOCK_CARPET = 11; const DRY_TURF_CARPET = 12; const DRY_TUSSOCK_CARPET = 13;
// Each tile's side in the world, m.
fn carpet_m(c: i32) -> f32 { var m = array<f32, 14>(1.5, 1.5, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 2.0, 0.8, 2.0, 2.0, 2.0, 2.0); return m[c]; }

// The rendered models (web/textures/trees.png and models.png; their table, sets and counts in sprites.wgsl, which the
// bake tool writes): the trees one to a layer from `TREES_FROM` on, the rest four to a layer from `MODELS_FROM` on
// (models.wgsl draws the plants from them, detail.wgsl the things on the ground).
const MODELS_FROM = 18; const TREES_FROM = MODELS_FROM + MODEL_LAYERS;
// The mean lightness of the grass tufts' blades (linear).
const TUFT_MEAN = 0.048;
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

