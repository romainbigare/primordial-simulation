// The plant cover as a field (web/src/canopy.ts `CoverField`), drawn in the live map's ground (land.wgsl) where its
// plants take too few pixels to be drawn as marks (cover.wgsl): of each lattice of marks the colour its plants cover
// the ground with and the share they cover, blended between the squares' centres, with the scatter a pixel of them
// would have. So the far and the near pictures come from the same numbers. Nothing stands where liquid stands over the
// roots or under snow.

struct CoverLook {
  // Of each lattice: its cell's side, m, and how strongly its marks show (the rest is the field).
  sides: vec4f, shown: vec4f,
  // 1 where the cover is drawn (0: none); how strongly it covers the map.
  look: vec4f,
}
@group(0) @binding(14) var<uniform> cover_look: CoverLook;
@group(0) @binding(15) var cover_field_tex: texture_2d_array<f32>;
@group(0) @binding(16) var cover_field_sampler: sampler;

const COVER_LATTICES = 3;
// A land plant's roots need air in the soil: liquid standing over its foot as deep as this, m, drowns them (half of the
// roots of most kinds of cover lie in the top 0.3 m, Jackson and others 1996).
const ROOTS_M = 0.3;
// The field's grain: the scatter of the marks' cover over a pixel, of a binomial count of the cells under it; value
// noise's draws are brought to unit spread by this (a smooth blend of four uniforms on -1..1 spreads about 0.43).
const GRAIN = 1.0; const NOISE_SPREAD = 2.3;
// A grain that spreads less than this (a display level of 255) does not show, and is not drawn.
const SPREAD_SHOWS = 0.004;

// The share of a plant that stands where liquid stands `depth` m deep and snow covers `snow` of the ground.
fn stands_in(depth: f32, snow: f32) -> f32 { return (1.0 - ease(0.0, ROOTS_M, depth)) * (1.0 - snow); }

// The field of lattice l at `at`, m from the map's north-west corner, at mip level lod: the colour its marks cover the
// ground with (premultiplied, display 0..1) and the share they cover.
fn field_at(at: vec2f, l: i32, lod: f32) -> vec4f {
  let uv = at / (land.map.x * land.map.yz);
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  return textureSampleLevel(cover_field_tex, cover_field_sampler, uv, l, lod);
}

// Value noise of unit spread on a grid of unit cells, blended smoothly between their corners' hashes, stream k.
fn value_noise(x: vec2f, k: i32) -> f32 {
  let i = vec2i(floor(x));
  let f = x - floor(x);
  let u = f * f * (3.0 - 2.0 * f);
  let h = vec4f(vec4u(lowbias(i, k), lowbias(i + vec2i(1, 0), k), lowbias(i + vec2i(0, 1), k), lowbias(i + vec2i(1, 1), k)) & vec4u(1023u)) / 1023.0;
  return NOISE_SPREAD * (2.0 * mix(mix(h.x, h.y, u.x), mix(h.z, h.w, u.x), u.y) - 1.0);
}

// The cover's field at `at` seen at `e` m a pixel, premultiplied: each lattice as far as its marks do not show, the
// lower under the taller, its share with the scatter of the cells under a block of its grain (a power of two of its
// cells across, at least a pixel).
fn cover_field(at: vec2f, e: f32) -> vec4f {
  var dst = vec4f(0.0);
  for (var l = 0; l < COVER_LATTICES; l++) {
    let unshown = 1.0 - cover_look.shown[l];
    if (unshown <= 0.0) { continue; }
    let f = field_at(at, l, 0.0);
    if (f.a <= 0.0) { continue; }
    let side = cover_look.sides[l];
    let block = side * exp2(max(ceil(log2(e / side)), 0.0));
    let spread = GRAIN * sqrt(f.a * (1.0 - f.a)) * side / block;
    var a = f.a;
    if (spread > SPREAD_SHOWS) { a = clamp(a + spread * value_noise(at / block, l + 7), 0.0, 1.0); }
    a *= unshown;
    dst = vec4f(f.rgb / f.a * a, a) + dst * (1.0 - a);
  }
  return dst;
}
