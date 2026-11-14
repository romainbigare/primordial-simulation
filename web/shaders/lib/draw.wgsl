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

// Every program starts with `const LEAST_E`, the least metres a pixel takes in its draws (web/shade): 0, or for a
// program made once more for draws seen from farther, the metres it is made for (land.wgsl). Detail that cannot show
// at that size of a pixel is compiled out of it: the picture is the same, and the GPU runs and holds less code. It is
// read through `least_e`, so the translation to GLSL keeps it by name and the programs made for each size differ in
// that line alone (web/shade, web/src/gpu.ts `programs`).
fn least_e() -> f32 { return LEAST_E; }

// Whether detail `size` m across shows at no pixel of the draws of this program (`shows_at` is 0 from 0.75 of its size
// a pixel on).
fn unseen(size: f32) -> bool { return 0.75 * size <= least_e(); }

// The share of detail `size` m across that shows at `e` m a pixel: none below a pixel.
fn shows_at(size: f32, e: f32) -> f32 {
  if (unseen(size)) { return 0.0; }
  return 1.0 - ease(0.5 * size, 1.5 * size, 2.0 * e);
}

// Paint colour rgb with coverage a over dst (premultiplied).
fn over(dst: ptr<function, vec4f>, rgb: vec3f, a: f32) { *dst = vec4f(rgb * a, a) + *dst * (1.0 - a); }

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

// Value noise at p (in its grid's cells) of stream k turned off the grid's axes by `TURN_A` radians, so a contour of it
// shows no square cells: -1..1, spread about 0.43 (even over 0..1 through `smoothstep(-EVEN_ONE, EVEN_ONE, n)`).
const TURN_A = 0.55; const EVEN_ONE = 0.95;
fn noise_turned(p: vec2f, k: i32) -> f32 {
  return noise(mat2x2f(cos(TURN_A), sin(TURN_A), -sin(TURN_A), cos(TURN_A)) * p, k).x;
}

// Two octaves of turned value noise at p (in the first's cells) of stream k, the second `OCTAVE` times as fine, turned
// by `TURN_B` radians and weighing `SECOND`, so neither's cells show: -1..1, spread about 0.3 (even over 0..1 through
// `smoothstep(-EVEN_TWO, EVEN_TWO, n)`).
const OCTAVE = 1.93; const SECOND = 0.8; const TURN_B = 2.1; const EVEN_TWO = 0.655;
fn noise_two(p: vec2f, k: i32) -> f32 {
  let b = mat2x2f(cos(TURN_B), sin(TURN_B), -sin(TURN_B), cos(TURN_B)) * p;
  return (noise_turned(p, k) + SECOND * noise(OCTAVE * b + 17.0, k).x) / (1.0 + SECOND);
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
  return tanh(clamp(ice_m / COVER_ICE_M, 0.0, 10.0)) * unmelted(t, melt);
}

// The share of the frozen cover left at a point at `t` K where it melts at `melt` K (0: not known, all of it).
fn unmelted(t: f32, melt: f32) -> f32 { return select(1.0, 1.0 - ease(melt - 1.0, melt + 1.0, t), melt > 0.0); }

// Lying snow holds this many times the depth of the ice it is made of: settled seasonal snow of about 300 kg/m³ against
// ice of 917 (Paterson 1994, from memory).
const SNOW_PER_ICE = 3.0;
// The depth of the snow on ground of `ice_m` m of frozen cover on average at `t` K (`snow_of`), m.
fn snow_depth(ice_m: f32, t: f32, melt: f32) -> f32 { return SNOW_PER_ICE * ice_m * unmelted(t, melt); }

// Liquid standing shallower than this over the drawn ground is drawn as soaked ground, m: the drawn ground follows the
// samples within a few centimetres, so a body whose level lies that near its flat floor is a soaked basin, not water.
// From this depth on it is water, and no land plant stands in it (its roots need air in the soil): one rule for the
// water drawn and for the plants.
const FILM_M = 0.05;
// The share of a plant that stands where liquid stands `depth` m deep: all of it on soaked ground under a film, none
// in water.
fn above_water(depth: f32) -> f32 { return select(1.0, 0.0, depth >= FILM_M); }
// Snow buries a plant `height` m tall as it reaches from `BURIED_FROM` of its height to all of it: a herb is gone under
// a deep snow, a tree stands in it.
const BURIED_FROM = 0.6;
// The share of a plant `height` m tall that shows over snow `snow_m` m deep.
fn above_snow(snow_m: f32, height: f32) -> f32 { return 1.0 - ease(BURIED_FROM * height, height, snow_m); }

// Light leaves the map as a screen shows it: every colour is worked in linear light (a reflectance times the light on
// the surface), then encoded once by the sRGB transfer curve (IEC 61966-2-1), at an exposure that shows sunlit flat
// ground of reflectance `1 / EXPOSURE` as full white.
const EXPOSURE = 1.0;
// Linear light `c` as the screen's share of full scale, 0..1.
fn srgb(c: vec3f) -> vec3f {
  let x = clamp(EXPOSURE * c, vec3f(0.0), vec3f(1.0));
  return select(1.055 * pow(x, vec3f(1.0 / 2.4)) - 0.055, 12.92 * x, x <= vec3f(0.0031308));
}
