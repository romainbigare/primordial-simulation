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
// A grain of the field (`grain`) where its cell takes more than a pixel: present in a cell as often as covers at most
// `GRAIN_FILL` of it, as wide as keeps the mean (at most half a cell), anywhere in its cell, so no grains stand in rows
// and none is cut at its cell's edge.
const GRAIN_FILL = 0.35;
// A uniform draw on -1..1 spreads sqrt(1/3): a draw times this spreads 1.
const UNIFORM_SPREAD = 1.732;
// A grain that spreads less than this (a display level of 255) does not show, and is not drawn.
const SPREAD_SHOWS = 0.004;
// Soft patches: the plants stand thicker and thinner over patches about `PATCH_M` m across, by up to `PATCH` of their
// mean (fixed noise in the world), alike near (cover.wgsl) and far.
const PATCH_M = 8.0; const PATCH = 0.45;

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

// How much thicker than their mean the plants stand at `at` (`PATCH`).
fn patchy(at: vec2f) -> f32 { return 1.0 + PATCH * noise(at / PATCH_M, 40).x; }

// The share of the ground at `at` that lattice l's plants cover, where they cover `s` of it, seen at `e` m a pixel:
// grains at the plants' own size (`GRAIN_FILL`), as fine as its cells of side `side`. A grain reaches at most half a
// cell from its cell, so only the point's cell and the three beside it towards the point can cover it.
fn grains(at: vec2f, l: i32, s: f32, e: f32, side: f32) -> f32 {
  let q = at / side;
  let toward = select(vec2f(-1.0), vec2f(1.0), fract(q) > vec2f(0.5));
  let present = min(s / GRAIN_FILL, 1.0);
  let r = min(side * sqrt(s / (present * PI)), 0.5 * side);
  var out = 0.0;
  for (var k = 0; k < 4; k++) {
    let c = floor(q) + vec2f(f32(k & 1), f32(k >> 1u)) * toward;
    let x = lowbias(vec2i(c), l + 7);
    if (f32(x & 1023u) / 1024.0 >= present) { continue; }
    let centre = (c + vec2f(f32((x >> 10u) & 1023u), f32((x >> 20u) & 1023u)) / 1024.0) * side;
    out = max(out, 1.0 - smoothstep(r - 0.5 * e, r + 0.5 * e, length(at - centre)));
  }
  return out;
}

// The mean share `s` with the scatter of the count of crowns of cells of side `side` over a block of them about `e`
// m wide (a power of two of them across), as a smooth noise of the block; the two blocks either side of the pixel's
// size blended by it, so no block shows and nothing jumps as the view zooms.
fn scatter(at: vec2f, l: i32, s: f32, e: f32, side: f32) -> f32 {
  let k = log2(max(e / side, 1.0));
  let fine = side * exp2(floor(k));
  let spread = sqrt(s * (1.0 - s)) * side / fine * UNIFORM_SPREAD;
  if (spread <= SPREAD_SHOWS) { return s; }
  let blend = mix(noise(at / fine, l + 7).x, 0.5 * noise(at / (2.0 * fine), l + 7).x, fract(k));
  return clamp(s + spread * blend, 0.0, 1.0);
}

// The share of the ground at `at` that lattice l's plants cover, where they cover `share` of it on the mean, seen at `e` m a pixel: grains where a cell takes more than a pixel, their scatter
// where it takes less, blended over the pixel's size from half a cell to a cell.
fn grain(at: vec2f, l: i32, share: f32, e: f32) -> f32 {
  let side = cover_look.sides[l];
  let s = min(share, 1.0);
  let w = smoothstep(0.5 * side, side, e);
  var out = 0.0;
  if (w < 1.0) { out += (1.0 - w) * grains(at, l, s, e, side); }
  if (w > 0.0) { out += w * scatter(at, l, s, e, side); }
  return out;
}

// The cover's field at `at` seen at `e` m a pixel, premultiplied: each lattice as far as its marks do not show, the
// lower under the taller, thicker and thinner over the patches (`patchy`), with its grain (`grain`).
fn cover_field(at: vec2f, e: f32) -> vec4f {
  var dst = vec4f(0.0);
  let thick = patchy(at);
  for (var l = 0; l < COVER_LATTICES; l++) {
    let unshown = 1.0 - cover_look.shown[l];
    if (unshown <= 0.0) { continue; }
    let f = field_smooth(at, l);
    if (f.a <= 0.0) { continue; }
    let a = grain(at, l, f.a * thick, e) * unshown;
    dst = vec4f(f.rgb / f.a * a, a) + dst * (1.0 - a);
  }
  return dst;
}
