// The plant cover as a field (web/src/canopy.ts `CoverField`), drawn in the live map's ground (land.wgsl) where its
// plants take too few pixels to be drawn as marks (cover.wgsl): of each lattice of marks the colour its plants cover
// the ground with and the share they cover, blended between the squares' centres, with the scatter a pixel of them
// would have. So the far and the near pictures come from the same numbers. Nothing stands where liquid stands over the
// roots or under snow.

struct CoverLook {
  // Of each lattice: its cell's side, m, and how strongly its marks show (the rest is the field).
  sides: vec4f, shown: vec4f,
  // 1 where the cover is drawn (0: none); how strongly its marks cover the map.
  look: vec4f,
}
@group(0) @binding(14) var<uniform> cover_look: CoverLook;
@group(0) @binding(15) var cover_field_tex: texture_2d_array<f32>;
@group(0) @binding(16) var cover_field_sampler: sampler;

const COVER_LATTICES = 3;
// A crown of the field covers at most this share of its cell (a disc in a square, pi / 4) before it grows past it.
const DISC = 0.785398;
// A uniform draw on -1..1 spreads sqrt(1/3): a draw times this spreads 1.
const UNIFORM_SPREAD = 1.732;
// A grain that spreads less than this (a display level of 255) does not show, and is not drawn.
const SPREAD_SHOWS = 0.004;

// The field of lattice l at `at`, m from the map's north-west corner, at mip level lod: the colour its marks cover the
// ground with (a reflectance, premultiplied) and the share they cover.
fn field_at(at: vec2f, l: i32, lod: f32) -> vec4f {
  let uv = at / (land.map.x * land.map.yz);
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  return textureSampleLevel(cover_field_tex, cover_field_sampler, uv, l, lod);
}

// The field of lattice l at uv, blended between its texels.
fn field_tap(uv: vec2f, l: i32) -> vec4f { return textureSampleLevel(cover_field_tex, cover_field_sampler, uv, l, 0.0); }

// The field of lattice l at `at` through its texels' centres as a cubic B-spline, from four blended reads (Sigg and
// Hadwiger 2005): smooth across the squares' edges, so no square shows in it, and never past its texels' values.
fn field_smooth(at: vec2f, l: i32) -> vec4f {
  let size = vec2f(textureDimensions(cover_field_tex).xy);
  let p = at / land.map.x - 0.5;
  let i = floor(p);
  let f = p - i;
  // The weights of the texels from i - 1 to i + 2, paired into two blended reads along each axis.
  let w0 = (1.0 - f) * (1.0 - f) * (1.0 - f) / 6.0;
  let w1 = (4.0 - 6.0 * f * f + 3.0 * f * f * f) / 6.0;
  let w3 = f * f * f / 6.0;
  let w2 = 1.0 - w0 - w1 - w3;
  let g0 = w0 + w1;
  let g1 = w2 + w3;
  let h0 = (i - 1.0 + w1 / g0 + 0.5) / size;
  let h1 = (i + 1.0 + w3 / g1 + 0.5) / size;
  let top = g0.x * field_tap(h0, l) + g1.x * field_tap(vec2f(h1.x, h0.y), l);
  let bottom = g0.x * field_tap(vec2f(h0.x, h1.y), l) + g1.x * field_tap(h1, l);
  return g0.y * top + g1.y * bottom;
}

// The share of the ground at `at` that lattice l's plants cover, where they cover `share` of it on the mean, seen at
// `e` m a pixel. Where a cell of the lattice takes more than a pixel, the crowns themselves: each cell holds one disc,
// as wide as keeps the mean, by its own random draw; where it takes less, the mean with the scatter of the count of
// crowns over a block of cells about a pixel wide (a power of two of them across), each block its own draw. So the
// grain is canopy and gaps at the plants' own size.
fn grain(at: vec2f, l: i32, share: f32, e: f32) -> f32 {
  let side = cover_look.sides[l];
  if (e < side) {
    let c = floor(at / side);
    let h = f32(lowbias(vec2i(c), l + 7) & 1023u) / 1023.0;
    let r = 0.5 * side * sqrt(max(share / DISC, 1.0));
    if (h >= min(share / DISC, 1.0)) { return 0.0; }
    return 1.0 - smoothstep(r - 0.5 * e, r + 0.5 * e, length(at - (c + 0.5) * side));
  }
  let block = side * exp2(round(log2(e / side)));
  let spread = sqrt(share * (1.0 - share)) * side / block;
  if (spread <= SPREAD_SHOWS) { return share; }
  let h = f32(lowbias(vec2i(floor(at / block)), l + 7) & 1023u) / 1023.0;
  return clamp(share + spread * UNIFORM_SPREAD * (2.0 * h - 1.0), 0.0, 1.0);
}

// The cover's field at `at` seen at `e` m a pixel, premultiplied: each lattice as far as its marks do not show, the
// lower under the taller, with its grain (`grain`).
fn cover_field(at: vec2f, e: f32) -> vec4f {
  var dst = vec4f(0.0);
  for (var l = 0; l < COVER_LATTICES; l++) {
    let unshown = 1.0 - cover_look.shown[l];
    if (unshown <= 0.0) { continue; }
    let f = field_smooth(at, l);
    if (f.a <= 0.0) { continue; }
    let a = grain(at, l, f.a, e) * unshown;
    dst = vec4f(f.rgb / f.a * a, a) + dst * (1.0 - a);
  }
  return dst;
}
