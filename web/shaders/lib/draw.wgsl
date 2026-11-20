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

// The one rule for a thing drawn one by one (a plant's crown in the field, a tuft, a stone, a rock or a log): it shows
// as itself, against the mean of its ground, by this share of its contrast, from its size in pixels: none under
// `THING_PX.x` (it would be a speck: its colour is its ground's mean), all from `THING_PX.y`.
const THING_PX = vec2f(4.0, 8.0);
fn as_thing(size: f32, e: f32) -> f32 { return ease(THING_PX.x * e, THING_PX.y * e, size); }

// A random word of the grid cell c and the stream k, fixed in the world (lowbias32, Wellons 2018).
fn lowbias(c: vec2i, k: i32) -> u32 {
  var x = (u32(c.x) * 0x9E3779B1u) ^ (u32(c.y) * 0x85EBCA77u) ^ (u32(k) * 0xC2B2AE3Du);
  x ^= x >> 16u; x *= 0x7feb352du; x ^= x >> 15u; x *= 0x846ca68bu; x ^= x >> 16u;
  return x;
}

// The place of the pixel being drawn, for detail finer than a float holds at a map's size (land.wgsl sets them): the
// world's origin of the draw, a whole multiple of 1024 m, and the point's distance from it, m. A position held as one
// float over tens of kilometres has steps of millimetres, which centimetre detail shows as rows and columns.
var<private> pix_origin: vec2f = vec2f(0.0);
var<private> pix_local: vec2f = vec2f(0.0);

// A point in a lattice: its cell and its place in it (0..1).
struct Lat { i: vec2i, f: vec2f }

// `x` with its mantissa cut to 12 bits.
fn bits12(x: f32) -> f32 { return bitcast<f32>(bitcast<u32>(x) & 0xfffff000u); }

// The lattice point of the pixel's place under the map `m` (cells a metre, along the world's axes, turned or not),
// shifted by `off` cells. The map's entries are cut to 12 bits, so `m` times the origin (a whole multiple of 1024 m, up
// to 12 bits too) is exact in a float: the cell comes from it whole and the rest from the small distance, to the
// float's last bit at any size of map and at any lattice, turned or not.
fn lat(m: mat2x2f, off: vec2f) -> Lat {
  let q = mat2x2f(vec2f(bits12(m[0].x), bits12(m[0].y)), vec2f(bits12(m[1].x), bits12(m[1].y)));
  let c = q * pix_origin;
  let whole = floor(c);
  let f = (c - whole) + q * pix_local + off;
  let k = floor(f);
  return Lat(vec2i(whole + k), f - k);
}

// The lattice of cells `size` m across along the world's axes, and turned by `TURN_A` radians (so a contour of its noise
// shows no square cells).
fn lat_axes(size: f32, off: vec2f) -> Lat { return lat(mat2x2f(1.0 / size, 0.0, 0.0, 1.0 / size), off); }
const TURN_A = 0.55; const EVEN_ONE = 0.95;
fn lat_turned(size: f32, off: vec2f) -> Lat {
  return lat(mat2x2f(cos(TURN_A), sin(TURN_A), -sin(TURN_A), cos(TURN_A)) * (1.0 / size), off);
}

// The gradient at lattice point c of stream k: two 16-bit halves of its random word, each -1..1 (no trigonometry: the
// gradients need not be of one length for the noise to show no grid).
fn gradient(c: vec2i, k: i32) -> vec2f {
  let x = lowbias(c, k);
  return vec2f(f32(x & 0xffffu), f32(x >> 16u)) * (2.0 / 65535.0) - 1.0;
}

// Gradient noise at the lattice point `l` of stream k (Perlin 2002; quintic easing; the slope as Quilez 2017): about
// -0.7..0.7, and its slope along the lattice's x and y. It is smooth with its slope, and its contours follow no lattice
// (value noise's do: thresholded, it draws blocks and mazes).
fn gradient_noise(l: Lat, k: i32) -> vec3f {
  let f = l.f;
  let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
  let ga = gradient(l.i, k);
  let gb = gradient(l.i + vec2i(1, 0), k);
  let gc = gradient(l.i + vec2i(0, 1), k);
  let gd = gradient(l.i + vec2i(1, 1), k);
  let va = dot(ga, f);
  let vb = dot(gb, f - vec2f(1.0, 0.0));
  let vc = dot(gc, f - vec2f(0.0, 1.0));
  let vd = dot(gd, f - vec2f(1.0, 1.0));
  let m = va - vb - vc + vd;
  return vec3f(va + u.x * (vb - va) + u.y * (vc - va) + u.x * u.y * m, ga + u.x * (gb - ga) + u.y * (gc - ga) + u.x * u.y * (ga - gb - gc + gd) + du * (u.yx * m + vec2f(vb, vc) - va));
}

// Noise of stream k at the lattice point `l`: gradient noise made wide (a spread of about 0.45, so
// `smoothstep(-EVEN_ONE, EVEN_ONE, n)` is even over 0..1 to within 3%), held within -1..1, and its slope.
const NOISE_GAIN = 2.55;
fn noise_l(l: Lat, k: i32) -> vec3f {
  let n = NOISE_GAIN * gradient_noise(l, k);
  return vec3f(clamp(n.x, -1.0, 1.0), n.yz);
}

// The same at p (in its grid's cells), for places seen from afar or coarse enough for a float.
fn noise(p: vec2f, k: i32) -> vec3f {
  let i = floor(p);
  return noise_l(Lat(vec2i(i), p - i), k);
}

// Noise at p (in its grid's cells) of stream k turned off the grid's axes by `TURN_A` radians (`lat_turned`): -1..1,
// spread about 0.43 (even over 0..1 through `smoothstep(-EVEN_ONE, EVEN_ONE, n)`).
fn noise_turned(p: vec2f, k: i32) -> f32 {
  return noise(mat2x2f(cos(TURN_A), sin(TURN_A), -sin(TURN_A), cos(TURN_A)) * p, k).x;
}

// A quadratic B-spline through the centres of a texture's texels at `p` (texels from its corner, less half a texel),
// read texel by texel (`textureLoad`), nine texels a point: smooth across the texels' edges, and so is its change, so no
// texel's square shows, and never past their values. Not through the GPU's own blend between texels (Sigg and Hadwiger
// 2005): that takes its weights in 1/256 steps, so the field it gives is a fine staircase, and its change across a
// pixel, which eases edges, a grid of spikes. The texel nearest the point, and the weights along each axis of the three
// texels from one before it.
struct BSpline { i: vec2i, wx: vec3f, wy: vec3f }
fn b_spline(p: vec2f) -> BSpline {
  let i = round(p);
  let t = p - i;
  let w0 = 0.5 * (0.5 - t) * (0.5 - t);
  let w2 = 0.5 * (0.5 + t) * (0.5 + t);
  let w1 = 1.0 - w0 - w2;
  return BSpline(vec2i(i), vec3f(w0.x, w1.x, w2.x), vec3f(w0.y, w1.y, w2.y));
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

// A crown's shadow darkens what lies under it by this share: it lets through the sky's light and what crowns scatter,
// about half of the sun's (canopy.ts `CROWN_SHADOW`).
const CROWN_SHADOW = 0.42;
// A thing standing up casts its shadow on the ground away from the sun (south-east, `LIGHT`), this many times its
// height long (models.wgsl `model_shadow`, snags.wgsl).
const MODEL_SHADOW_RUN = 0.9;

// Liquid standing shallower than this over the drawn ground is drawn as soaked ground, m: the drawn ground follows the
// samples within a few centimetres, so a body whose level lies that near its flat floor is a soaked basin, not water.
// From this depth on it is water, and a plant stands in it only down to its kind's line (the core's `Cover::line`, as
// deep under the level as it lies): one rule for the water drawn and for the plants.
const FILM_M = 0.05;
// The share of a plant whose kind stands in liquid `reach` m deep (down to its line) that stands where liquid stands
// `depth` m deep: all of it on soaked ground under a film and in liquid shallower than that, none deeper.
fn stands_in(depth: f32, reach: f32) -> f32 { return select(1.0, 0.0, depth >= max(FILM_M, reach)); }
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

// The light that the screen's share `x` of full scale shows (`srgb` backwards).
fn unsrgb(x: vec3f) -> vec3f {
  return select(pow((x + 0.055) / 1.055, vec3f(2.4)), x / 12.92, x <= vec3f(0.04045)) / EXPOSURE;
}
