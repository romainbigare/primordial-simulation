// The textures of the map's detail (web/src/detail.ts): one array of photographs of ground seen from above, of leaves,
// needles and bark, each a layer with its mips down to its mean, and of cut-out plants four to a layer after them; read
// by the ground (detail.wgsl) and by the plants (crown.wgsl).

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

// The cut-out plants (detail.ts), four to a layer from layer `SPRITES_FROM` on, each standing on the bottom middle of
// its quarter: grass clumps, the same dry (`DRY_GRASS`), fern fronds, moss, flower heads.
const SPRITES_FROM = 12; const GRASS = 0; const DRY_GRASS = 5; const GRASSES = 5; const FERNS = 10; const FERN_KINDS = 3;
// A plant's quarter is read this far inside its edge, so its mips draw nothing of its neighbours.
const SPRITE_INSIDE = 0.004;

// Cut-out plant `s` at `uv` in its quarter (0..1, y down), its quarter's step over a pixel `g`: its colour (linear) and
// how much of the pixel it covers.
fn sprite(s: i32, uv: vec2f, g: f32) -> vec4f {
  let q = vec2f(f32(s % 2), f32((s % 4) / 2));
  let at = (q + clamp(uv, vec2f(SPRITE_INSIDE), vec2f(1.0 - SPRITE_INSIDE))) * 0.5;
  let t = textureSampleGrad(detail_tex, detail_sampler, at, SPRITES_FROM + s / 4, vec2f(0.5 * g, 0.0), vec2f(0.0, 0.5 * g));
  return vec4f(unsrgb(t.rgb), t.a);
}

