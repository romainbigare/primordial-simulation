// A draw of the map's renderer (web/src/gpu/): a rectangle of the target filled by a program's fragment stage. Every
// program reads it at binding 0 and has this vertex stage: four corners as a triangle strip. With the helpers every
// program may use.

struct Draw {
  // The target's size, px; the sign of its rows (-1 where the back end keeps a target's rows bottom up, WebGL2 drawing
  // into a texture, so every target's first row is its top); how strongly the draw covers what is under it, 0..1.
  size: vec2f, flip: f32, alpha: f32,
  // The rectangle drawn, px from the target's top-left corner: x, y, w, h.
  rect: vec4f,
  // What the rectangle shows of its source, in the source's uv: from (x, y) at its top-left corner to (z, w).
  src: vec4f,
}
@group(0) @binding(0) var<uniform> draw: Draw;

struct Varying {
  @builtin(position) pos: vec4f,
  // The point of the source, uv, and of the target, px from its top-left corner.
  @location(0) uv: vec2f,
  @location(1) px: vec2f,
}

@vertex fn vs(@builtin(vertex_index) i: u32) -> Varying {
  let corner = vec2f(f32(i & 1u), f32(i >> 1u));
  let px = draw.rect.xy + corner * draw.rect.zw;
  var v: Varying;
  v.pos = vec4f(2.0 * px.x / draw.size.x - 1.0, (1.0 - 2.0 * px.y / draw.size.y) * draw.flip, 0.0, 1.0);
  v.uv = mix(draw.src.xy, draw.src.zw, corner);
  v.px = px;
  return v;
}

const PI = 3.14159265;
// The light from the north-west (a unit vector to the sun: east, south, up).
const LIGHT = vec3f(-0.588, -0.588, 0.647);

// 0 at a, 1 at b, smooth between (a may be above b).
fn ease(a: f32, b: f32, x: f32) -> f32 {
  let t = clamp((x - a) / (b - a), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}

// A random word of the grid cell c and the stream k, fixed in the world (lowbias32, Wellons 2018).
fn lowbias(c: vec2i, k: i32) -> u32 {
  var x = (u32(c.x) * 0x9E3779B1u) ^ (u32(c.y) * 0x85EBCA77u) ^ (u32(k) * 0xC2B2AE3Du);
  x ^= x >> 16u; x *= 0x7feb352du; x ^= x >> 15u; x *= 0x846ca68bu; x ^= x >> 16u;
  return x;
}

// Value noise at p (in its grid's cells) of stream k, -1..1, and its slope along x and y (Quilez 2017).
fn noise(p: vec2f, k: i32) -> vec3f {
  let i = vec2i(floor(p));
  let f = p - floor(p);
  let u = f * f * (3.0 - 2.0 * f);
  let du = 6.0 * f * (1.0 - f);
  let v = vec4f(f32(lowbias(i, k)), f32(lowbias(i + vec2i(1, 0), k)), f32(lowbias(i + vec2i(0, 1), k)), f32(lowbias(i + vec2i(1, 1), k))) * (2.0 / 4294967296.0) - 1.0;
  let a = v.y - v.x;
  let b = v.z - v.x;
  let c = v.x - v.y - v.z + v.w;
  return vec3f(v.x + a * u.x + b * u.y + c * u.x * u.y, du * vec2f(a + c * u.y, b + c * u.x));
}

// A cubic B-spline through the centres of the texels of a texture `size` texels wide, at `p` (texels from its corner,
// less half a texel), from four blended reads (Sigg and Hadwiger 2005): the places of the reads along each axis (uv) and
// their weights. Smooth across the texels' edges, so no texel's square shows, and never past their values.
struct Bspline { h0: vec2f, h1: vec2f, g0: vec2f, g1: vec2f }
fn bspline(p: vec2f, size: vec2f) -> Bspline {
  let i = floor(p);
  let f = p - i;
  let w0 = (1.0 - f) * (1.0 - f) * (1.0 - f) / 6.0;
  let w1 = (4.0 - 6.0 * f * f + 3.0 * f * f * f) / 6.0;
  let w3 = f * f * f / 6.0;
  let g0 = w0 + w1;
  let g1 = 1.0 - g0;
  return Bspline((i - 1.0 + w1 / g0 + 0.5) / size, (i + 1.0 + w3 / g1 + 0.5) / size, g0, g1);
}

// A frozen cover of this mean depth, m of ice, covers e^-1 less than all of the ground: about 3 cm of snow, the depth at
// which snow hides the ground's roughness (Yang and others 1997: cover = tanh(depth / 2.5 z0), z0 = 1 cm).
const COVER_ICE_M = 0.008;

// How much snow and ice lies on ground of `ice_m` m of it on average at `t` K, 0..1: as much as that depth covers
// (`COVER_ICE_M`), less where the point is warmer than the melting point of the frozen cover `melt` (fading over a
// kelvin either side; 0: none known, the ice alone): the cover melts there first.
fn snow_of(ice_m: f32, t: f32, melt: f32) -> f32 {
  // (tanh of a large number overflows in some shaders: past 10 it is 1 to the last bit of a 32-bit float.)
  let cover = tanh(clamp(ice_m / COVER_ICE_M, 0.0, 10.0));
  return select(cover, cover * (1.0 - ease(melt - 1.0, melt + 1.0, t)), melt > 0.0);
}

// A land plant's roots need air in the soil: liquid standing over its foot as deep as this, m, drowns them (half of the
// roots of most kinds of cover lie in the top 0.3 m, Jackson and others 1996).
const ROOTS_M = 0.3;
// The share of a plant that stands where liquid stands `depth` m deep and snow covers `snow` of the ground.
fn stands_in(depth: f32, snow: f32) -> f32 { return (1.0 - ease(0.0, ROOTS_M, depth)) * (1.0 - snow); }

// Light leaves the map as a screen shows it: every colour is worked in linear light (a reflectance times the light on
// the surface), then encoded once by the sRGB transfer curve (IEC 61966-2-1), at an exposure that shows sunlit flat
// ground of reflectance `1 / EXPOSURE` as full white.
const EXPOSURE = 1.0;
// Linear light `c` as the screen's share of full scale, 0..1.
fn srgb(c: vec3f) -> vec3f {
  let x = clamp(EXPOSURE * c, vec3f(0.0), vec3f(1.0));
  return select(1.055 * pow(x, vec3f(1.0 / 2.4)) - 0.055, 12.92 * x, x <= vec3f(0.0031308));
}
