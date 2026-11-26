// The textures of the map's detail (web/src/detail.ts): one array of photographs of ground seen from above, of tiles of
// ground cover, and of rendered plants four to a layer, each a layer with its mips down to its mean; read by the ground
// (detail.wgsl) and by the plants (models.wgsl).

@group(0) @binding(20) var detail_tex: texture_2d_array<f32>;
@group(0) @binding(21) var detail_sampler: sampler;

// The photographs' layers: river pebbles, mossy rock, wet mud (laid by detail.wgsl `detail`), a mossy floor with
// twigs; damp beach sand streaked by the swash, dry beach sand with shells and grit, dark wet mud with stones and
// sticks, rounded river pebbles of many rocks, angular gravel of small stones, mud with moss and seedlings, a dense mat
// of fallen leaves; bare dry loam with grit, dry clay cracked into plates, and dark soil rich in humus with fallen
// leaves and seedlings; a flat of rippled sand with water left in its troughs, rippled silt, mud under a green film of
// algae, and wet gravelly sand with small stones.
const PEBBLE_TEX = 0; const ROCK_TEX = 1; const MUD_TEX = 2; const MOSSY_TEX = 3;
const WET_SAND_TEX = 4; const DRY_SAND_TEX = 5; const DARK_MUD_TEX = 6; const ROUND_PEBBLE_TEX = 7; const GRAVEL_TEX = 8;
const MOSS_MUD_TEX = 9; const LEAF_MAT_TEX = 10; const LOAM_TEX = 11; const CLAY_TEX = 12; const HUMUS_TEX = 13;
const RIPPLE_FLAT_TEX = 14; const RIPPLE_SILT_TEX = 15; const ALGAL_MUD_TEX = 16; const WET_GRAVEL_TEX = 17;
// The photographs' layers, the carpets' first.
const PHOTOS = 18;
// A mip level past the last of any layer's side (512 texels at most): the sampler reads the last, of one texel.
const DETAIL_TOP = 16.0;

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
const CARPETS_FROM = PHOTOS;
// The carpets' layers.
const CARPETS = 14;
const GRASS_CARPET = 0; const STRAW_CARPET = 1; const LITTER_CARPET = 2; const FERN_CARPET = 3; const SCRUB_CARPET = 4;
const CRACKED_CARPET = 5; const SAND_CARPET = 6; const SNOW_CARPET = 7; const REED_CARPET = 8; const FLOWER_CARPET = 9;
const TURF_CARPET = 10; const TUSSOCK_CARPET = 11; const DRY_TURF_CARPET = 12; const DRY_TUSSOCK_CARPET = 13;
// Each tile's side in the world, m.
const CARPET_M = array<f32, CARPETS>(1.5, 1.5, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 2.0, 0.8, 2.0, 2.0, 2.0, 2.0);
fn carpet_m(c: i32) -> f32 { return CARPET_M[c]; }

// The fire's strip (web/textures/fire.jpg, after the carpets): its first layer four sprites a quarter each, their
// density alone (two flame tongues, two puffs of smoke: flames.wgsl), its second a photograph of burnt ground (burn.wgsl).
const FIRE_FROM = CARPETS_FROM + CARPETS;
// The snags' strip (web/textures/snags.png, after the fire's): `SNAG_LAYERS` layers of sprites with their cover, a
// charred trunk a whole layer, a charred stump a quarter of one (snag_sprites.wgsl, snag.wgsl).
const SNAGS_FROM = FIRE_FROM + 2;

// The rendered models (web/textures/trees.png and models.png; their table, sets and counts in sprites.wgsl, which the
// bake tool writes): the trees one to a layer from `TREES_FROM` on, the rest four to a layer from `MODELS_FROM` on
// (models.wgsl draws the plants from them, detail.wgsl the things on the ground); the heights of those that stand in
// water four to a layer from `HEIGHTS_FROM` on (heights.png, heights.wgsl).
const MODELS_FROM = SNAGS_FROM + SNAG_LAYERS; const HEIGHTS_FROM = MODELS_FROM + MODEL_LAYERS; const TREES_FROM = HEIGHTS_FROM + HEIGHT_LAYERS;
// The mean lightness of the grass tufts' blades (linear).
const TUFT_MEAN = 0.048;
// A sprite read at a coarse level blurs its texels past its box, where nothing is read: its cover fades to none over
// half a pixel (`g` of its content, across and down) inside its box's sides and top, so the box's edge never shows as a
// straight line, in the sprite or in its shadow. (Its bottom is its foot, cut by the ground.)
fn box_fade(uv: vec2f, g: vec2f) -> f32 {
  let inside = vec3f(uv.x, 1.0 - uv.x, uv.y) / max(0.5 * vec3f(g.x, g.x, g.y), vec3f(1e-6));
  return smoothstep(0.0, 1.0, min(min(inside.x, inside.y), inside.z));
}

// The texel of model sprite s at `uv` in its content (0..1, y down), at `g` of its content a pixel: linear, with cover.
// Each stands on the bottom middle of its layer or quarter, two texels over its edge.
fn model_texel(s: i32, uv: vec2f, g: f32) -> vec4f {
  let size = vec2f(model_w(s), model_h(s));
  var t: vec4f;
  if (s < TREES) {
    let at = vec2f(0.5 - 0.5 * size.x, 1.0 - size.y - 2.0 / f32(textureDimensions(detail_tex).x)) + uv * size;
    t = textureSampleGrad(detail_tex, detail_sampler, at, TREES_FROM + s, vec2f(g * size.x, 0.0), vec2f(0.0, g * size.y));
  } else {
    let k = s - TREES;
    let q = vec2f(f32(k % 2), f32((k % 4) / 2));
    let at = (q + vec2f(0.5 - 0.5 * size.x, 1.0 - size.y - 2.0 / 256.0) + uv * size) * 0.5;
    t = textureSampleGrad(detail_tex, detail_sampler, at, MODELS_FROM + k / 4, vec2f(0.5 * g * size.x, 0.0), vec2f(0.0, 0.5 * g * size.y));
  }
  return vec4f(unsrgb(t.rgb), t.a * box_fade(uv, vec2f(g)));
}

// The height over the ground of the point of sprite s at `uv` in its content (`g` of it a pixel), drawn `h` m tall, the
// point `up` m north of its foot on the map, m: from its heights (heights.wgsl: each texel's as a share of its height,
// held to 1, in the cells of their quarter two texels over its edge, as the models'); else as a point of a stem
// standing up from its foot, which the map's view (water.wgsl `MAP_VIEW`) draws the cosine of its height north of it.
const HEIGHT_FOOT = 2.0 / 256.0;
fn model_height_m(s: i32, uv: vec2f, g: f32, up: f32, h: f32) -> f32 {
  let k = heights_of(s);
  if (k < 0) { return up / MAP_VIEW.y; }
  let size = vec2f(model_w(s), model_h(s));
  let q = vec2f(f32(k % 2), f32((k % 4) / 2));
  let at = (q + vec2f(0.5 - 0.5 * size.x, 1.0 - size.y - HEIGHT_FOOT) + uv * size) * 0.5;
  // The heights are each texel's own (not light), its texels between them taken by their cover.
  let t = textureSampleGrad(detail_tex, detail_sampler, at, HEIGHTS_FROM + k / 4, vec2f(0.5 * g * size.x, 0.0), vec2f(0.0, 0.5 * g * size.y));
  return min(t.r / max(t.a, 1e-3), 1.0) * h;
}

// The share of sprite s's height its stem's foot (the middle of its model's box on the ground) stands over its lowest
// point, which the bake stands on the sprite's foot: for a plant on a wide foot (a flare, a fan of prop roots, a clump
// of stems) the near edge of it, lower on the map's angled view than its stem. The bake's (heights.wgsl `foot_of`) for
// the sprites with heights, the ones that stand in water; the rest stand on narrow feet.
fn foot_rise(s: i32) -> f32 {
  let k = heights_of(s);
  return select(0.0, foot_of(max(k, 0)), k >= 0);
}

// Whether the models are loaded (the detail texture is more than its one blank texel).
fn models_ready() -> bool { return textureDimensions(detail_tex).x > 1u; }

