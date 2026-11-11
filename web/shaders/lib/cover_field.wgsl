// The plant cover as a field (web/src/canopy.ts `CoverField`), drawn in the live map's ground (land.wgsl) where its
// plants take too few pixels to be drawn as marks (cover.wgsl): of each lattice of marks the colour its plants cover
// the ground with and the share they cover, blended between the squares' centres, with the scatter a pixel of them
// would have; and the herbs as a sward with their flowers, at every scale. So the far and the near pictures come from
// the same numbers. Nothing stands where liquid stands over the roots or under snow.

struct CoverLook {
  // Of each lattice: its cell's side, m (0: no square holds it), and how strongly its marks show (the rest is the field);
  // and how much more than its share the sward is seen to cover (`sides.w`, canopy.ts `SWARD_SEEN`).
  sides: vec4f, shown: vec4f,
  // 1 where the cover is drawn (0: none); how strongly its marks cover the map; the loose ground a kind's roots need per
  // m of its height, and the least any kind needs, m (roots.ts).
  look: vec4f,
}
@group(0) @binding(14) var<uniform> cover_look: CoverLook;
@group(0) @binding(15) var cover_field_tex: texture_2d_array<f32>;
@group(0) @binding(16) var cover_field_sampler: sampler;

const COVER_LATTICES = 3;
// The layers of the field after the lattices' (canopy.ts `FIELD`): the herbs' sward and their flowers, and the part of
// the sward and of the lattice of stands (`STANDS`, the trees') that is its first kind.
const SWARD_LAYER = 3; const FLOWER_LAYER = 4; const SWARD_FIRST = 5; const STANDS_FIRST = 6;
// Of each lattice and the sward (red, green, blue, alpha): its plants' mean height, m, and mean rooted share in the square.
const TALL_LAYER = 7; const ROOTED_LAYER = 8;
// The sward and the low lattices as one layer (canopy.ts `lowOf`), drawn in their place where a pixel takes
// `LOW_FAR_E` m and more (where their detail no longer shows: no swathes, blades, tufts or flowers past a metre a
// pixel), fully from `LOW_FAR_FULL_E`; thicker and thinner over the patches as they are.
const LOW_LAYER = 9; const LOW_FAR_E = 0.75; const LOW_FAR_FULL_E = 1.0;
const STANDS = 2;
// A grain of the field (`grain`) where its cell takes more than a pixel: present in a cell as often as covers at most
// `GRAIN_FILL` of it, as wide as keeps the mean (at most half a cell), anywhere in its cell, so no grains stand in rows
// and none is cut at its cell's edge. Each is a crown lit from the north-west, its light departing from the flat
// ground's by up to `GRAIN_LIGHT`, and its tone its own, up to `GRAIN_TONE` either way.
const GRAIN_FILL = 0.35; const GRAIN_LIGHT = 0.2; const GRAIN_TONE = 0.1;
// A uniform draw on -1..1 spreads sqrt(1/3): a draw times this spreads 1.
const UNIFORM_SPREAD = 1.732;
// A grain that spreads less than this (a display level of 255) does not show, and is not drawn.
const SPREAD_SHOWS = 0.004;
// Soft patches: the plants stand thicker and thinner over patches about `PATCH_M` m across, by up to `PATCH` of their
// mean (fixed noise in the world), alike near (cover.wgsl) and far.
const PATCH_M = 8.0; const PATCH = 0.45;

// Kinds grow in stands of their own: a plant is of its square's first kind (canopy.ts `Paint`) where a smooth draw fixed
// in the world, mixed with `STAND_MIX` of the plant's own draw, falls within the first kind's share; so a kind's stands
// are patches about as large as the noise's cells (`STAND_M` the trees', `SWARD_STAND_M` the herbs') with mixed edges,
// and each kind keeps its share (draw.wgsl `noise_turned`, made even over 0..1).
const STAND_M = 60.0; const SWARD_STAND_M = 5.0; const STAND_MIX = 0.4;
// Seen from above the trees' crowns with their shade cover more than their share, as the sward's: the ground they leave
// is (1 - share)^`CANOPY_SEEN`; their canopy is lighter and darker by up to `CANOPY_TONE` over groups of crowns
// `CANOPY_M` across.
const CANOPY_SEEN = 2.0; const CANOPY_TONE = 0.2; const CANOPY_M = 35.0;
// Snow lies on the trees' crowns as on the ground under them, up to this share of their colour seen from afar (the
// evergreen ones hold it; the bare ones are thin and dark).
const CANOPY_SNOW = 0.35;

// What lies under and over the cover at a point (land.wgsl `land_spots`): how soaked its ground is, 0..1; its height
// over the nearest water's level, m, and its slope (rise over run); the depth of its loose ground, m; the share snow
// covers and the snow's depth, m.
struct Beneath { soaked: f32, over: f32, slope: f32, loose: f32, snow: f32, snow_m: f32 }

// The cover's field at a point, premultiplied, and the share the trees' crowns close over the ground there.
struct CoverAt { cover: vec4f, canopy: f32 }

// A square whose kinds' roots hold everywhere has its mean rooted share this near 1: no point of it limits them.
const ROOTED_ALL = 0.999;

// Of each lattice and the sward (red, green, blue, alpha), the share of its plants that stand at a point over `b`:
// where their roots hold (land.wgsl `rooted`, over their square's mean, so the square keeps its cover) and out of the
// snow (draw.wgsl `above_snow`). Each layer is read only where it may limit them.
fn held_at(at: vec2f, b: Beneath) -> vec4f {
  let mean = field_at(at, ROOTED_LAYER, 0.0);
  let roots = any(mean < vec4f(ROOTED_ALL));
  if (!roots && b.snow_m <= 0.0) { return vec4f(1.0); }
  let tall = field_at(at, TALL_LAYER, 0.0);
  let need = max(tall * cover_look.look.z, vec4f(cover_look.look.w));
  var out = vec4f(1.0);
  for (var c = 0; c < 4; c++) {
    if (roots) { out[c] = rooted(b.loose, need[c]) / max(mean[c], 1e-3); }
    out[c] *= above_snow(b.snow_m, tall[c]);
  }
  return out;
}

// The herbs' sward: their plants thicker and thinner by up to `SWARD` of their mean over tufts of `SWARD_M`; seen from
// above, their leaves leaning and their shade cover more than their share: the ground they leave is (1 - share)^
// `cover_look.sides.w` (canopy.ts `SWARD_SEEN`, an Earth view's style). Its tone varies by up to `TUFT_TONE` over the tufts, by up to `SWATH` over
// swathes of `SWATH_M` and by up to `BLADES` over blades `BLADE_W` wide and `BLADE_L` long, turned by tufts.
const SWARD = 0.45; const SWARD_M = 0.35;
// Under the trees' crowns the sward is in their shade: darker by up to `SWARD_SHADE` as they close over it.
const SWARD_SHADE = 0.35;
const TUFT_TONE = 0.15; const SWATH = 0.18; const SWATH_M = 3.0; const BLADES = 0.3; const BLADE_W = 0.05; const BLADE_L = 0.25;
// Swathes and blades show as detail this size, m (`shows_at`): none where a pixel takes 1 m or more, where they would
// read only as noise.
const SWATH_DETAIL_M = 1.33;
// Reeds: herbs on soaked ground (from `REED_WET`) and on the shore of standing or running water, out to `REED_BAND_M`
// m from its edge (its height over the water's level over the slope), stand dense (`REED_FILL` of the ground), their colour times `REED_TINT`, in streaks of tall blades
// `REED_W` wide, `REED_L` long, `REED_TONE` deep.
const REED_WET = 0.85; const REED_BAND_M = 5.0; const REED_FILL = 0.95; const REED_TINT = vec3f(0.75, 0.8, 0.45);
const REED_W = 0.07; const REED_L = 0.9; const REED_TONE = 0.5;
// Flowers: in clusters as large as the noise's cells, `FLOWER_M`, over as much of the sward as is in flower; in a
// cluster a dot of radius `DOT_R` lies in each cell of `DOT_GAP` at a place of its own (`DOT_PRESENT` of the cells),
// lighter at its heart by `DOT_HEART`; where a dot takes less than a pixel, the cluster's mean; they show as detail
// `FLOWERS_GONE_M` across (`shows_at`): none where a pixel takes a metre or more (their faint tint gone from afar).
const FLOWERS_GONE_M = 1.33; const FLOWER_M = 1.2; const DOT_R = 0.1; const DOT_GAP = 0.38; const DOT_PRESENT = 0.8; const DOT_HEART = 0.25;

// The field of lattice l at `at`, m from the map's north-west corner, at mip level lod: the colour its marks cover the
// ground with (a reflectance, premultiplied) and the share they cover.
fn field_at(at: vec2f, l: i32, lod: f32) -> vec4f {
  let uv = at / (land.map.x * land.map.yz);
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  return textureSampleLevel(cover_field_tex, cover_field_sampler, uv, l, lod);
}

// The field of lattice l at uv, blended between its texels.
fn field_tap(uv: vec2f, l: i32) -> vec4f { return textureSampleLevel(cover_field_tex, cover_field_sampler, uv, l, 0.0); }

// The field of lattice l at `at` through its texels' centres as a cubic B-spline (draw.wgsl `bspline`): no square
// shows in it.
fn field_smooth(at: vec2f, l: i32) -> vec4f {
  let b = bspline(at / land.map.x - 0.5, vec2f(textureDimensions(cover_field_tex).xy));
  let top = b.g0.x * field_tap(b.h0, l) + b.g1.x * field_tap(vec2f(b.h1.x, b.h0.y), l);
  let bottom = b.g0.x * field_tap(vec2f(b.h0.x, b.h1.y), l) + b.g1.x * field_tap(b.h1, l);
  return b.g0.y * top + b.g1.y * bottom;
}

// How much thicker than their mean the plants stand at `at` (`PATCH`).
fn patchy(at: vec2f) -> f32 { return 1.0 + PATCH * noise(at / PATCH_M, 40).x; }

// The stands' draw at `at` over patches of `m` m, even over 0..1.
fn stand_draw(at: vec2f, m: f32) -> f32 { return smoothstep(-EVEN_ONE, EVEN_ONE, noise_turned(at / m, 70)); }

// The stands' draw `u` mixed with a plant's own draw `r` (both even over 0..1), made even again: the distribution
// function of (1 - STAND_MIX) u + STAND_MIX r. A plant is of the first kind where this falls under the kind's share.
fn stand_pick(u: f32, r: f32) -> f32 {
  let a = 1.0 - STAND_MIX; let b = STAND_MIX;
  let v = a * u + b * r;
  if (v < b) { return v * v / (2.0 * a * b); }
  if (v < a) { return (v - 0.5 * b) / a; }
  return 1.0 - (1.0 - v) * (1.0 - v) / (2.0 * a * b);
}

// The share of the plants at stands' draw u that are of the first kind, whose share is s: the chance over a plant's
// own draw that `stand_pick` falls under s.
fn stand_share(u: f32, s: f32) -> f32 {
  let a = 1.0 - STAND_MIX; let b = STAND_MIX;
  var g = a * s + 0.5 * b;
  if (s < 0.5 * b / a) { g = sqrt(2.0 * a * b * s); } else if (s > 1.0 - 0.5 * b / a) { g = 1.0 - sqrt(2.0 * a * b * (1.0 - s)); }
  return clamp((g - a * u) / b, 0.0, 1.0);
}

// The colour at `at`, seen at `e` m a pixel, of layer l whose field is `f` (premultiplied), its kinds in stands of
// about `m` m: its first kind's (layer `first`, its share read between the texels as plainly blended: it only moves the
// stands' edges) where its stands are, the rest's elsewhere; their mean where a stand takes less than a pixel.
fn in_stands(at: vec2f, e: f32, f: vec4f, first: i32, m: f32) -> vec3f {
  let mean = f.rgb / f.a;
  let shows = shows_at(m, e);
  if (shows <= 0.0) { return mean; }
  let one = field_at(at, first, 0.0);
  let s = clamp(one.a / f.a, 0.0, 1.0);
  let own = one.rgb / max(one.a, 1e-6);
  let rest = max(f.rgb - one.rgb, vec3f(0.0)) / max(f.a - one.a, 1e-6);
  return mix(mean, mix(rest, own, stand_share(stand_draw(at, m), s)), shows);
}

// The share of the ground at `at` that lattice l's plants cover, where they cover `s` of it, seen at `e` m a pixel,
// and the tone there: grains at the plants' own size (`GRAIN_FILL`), as fine as its cells of side `side`, each lit and
// toned. A grain reaches at most half a cell from its cell, so only the point's cell and the three beside it towards the
// point can cover it.
fn grains(at: vec2f, l: i32, s: f32, e: f32, side: f32) -> vec2f {
  let q = at / side;
  let toward = select(vec2f(-1.0), vec2f(1.0), fract(q) > vec2f(0.5));
  let present = min(s / GRAIN_FILL, 1.0);
  let r = min(side * sqrt(s / (present * PI)), 0.5 * side);
  var out = vec2f(0.0, 1.0);
  for (var k = 0; k < 4; k++) {
    let c = floor(q) + vec2f(f32(k & 1), f32(k >> 1u)) * toward;
    let x = lowbias(vec2i(c), l + 7);
    if (f32(x & 1023u) / 1024.0 >= present) { continue; }
    let centre = (c + vec2f(f32((x >> 10u) & 1023u), f32((x >> 20u) & 1023u)) / 1024.0) * side;
    let d = at - centre;
    let a = 1.0 - smoothstep(r - 0.5 * e, r + 0.5 * e, length(d));
    if (a > out.x) {
      let lit = 1.0 - GRAIN_LIGHT * dot(d / max(r, 1e-4), normalize(LIGHT.xy));
      out = vec2f(a, lit * (1.0 + GRAIN_TONE * (f32(x >> 30u) / 1.5 - 1.0)));
    }
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

// The share of the ground at `at` that lattice l's plants cover, where they cover `share` of it on the mean, seen at
// `e` m a pixel, and their tone: grains where a cell takes more than a pixel, their scatter where it takes less,
// blended over the pixel's size from half a cell to a cell.
fn grain(at: vec2f, l: i32, share: f32, e: f32) -> vec2f {
  let side = cover_look.sides[l];
  let s = min(share, 1.0);
  let w = smoothstep(0.5 * side, side, e);
  var out = vec2f(0.0);
  if (w < 1.0) { out += (1.0 - w) * grains(at, l, s, e, side); }
  if (w > 0.0) { out += w * vec2f(scatter(at, l, s, e, side), 1.0); }
  return out;
}

// Streaks of blades `w` m wide and `len` m long at `at`, turned by `turn` radians: -1..1 (value noise stretched).
fn streaks(at: vec2f, w: f32, len: f32, turn: f32, k: i32) -> f32 {
  let p = mat2x2f(cos(turn), -sin(turn), sin(turn), cos(turn)) * at;
  return noise(p / vec2f(w, len), k).x;
}

// The flowers over the sward at `at`, seen at `e` m a pixel, of the herbs whose share `bloom` is in flower: their
// colour and the share of the ground they cover (the sward's own share times it): clusters of dots (`FLOWER_M`, `DOT_R`).
fn flowers_at(at: vec2f, e: f32, bloom: f32, petal: vec3f) -> vec4f {
  // The clusters: where the noise, made even, falls in the share in flower; softened over a pixel.
  let n = smoothstep(-EVEN_ONE, EVEN_ONE, noise_turned(at / FLOWER_M, 71));
  let edge = max(e / FLOWER_M, 0.05);
  let cluster = mix(bloom, ease(1.0 - bloom - edge, 1.0 - bloom + edge, n), shows_at(FLOWER_M, e));
  let mean = DOT_PRESENT * PI * DOT_R * DOT_R / (DOT_GAP * DOT_GAP);
  let dots = shows_at(2.0 * DOT_R, e);
  var cover = mean;
  var c = petal;
  if (dots > 0.0 && cluster > 0.0) {
    let q = at / DOT_GAP;
    let x = lowbias(vec2i(floor(q)), 72);
    let margin = DOT_R / DOT_GAP;
    let centre = (floor(q) + margin + (1.0 - 2.0 * margin) * vec2f(f32(x & 1023u), f32((x >> 10u) & 1023u)) / 1024.0) * DOT_GAP;
    let d = length(at - centre) / DOT_R;
    let present = select(0.0, 1.0, f32(x >> 22u) / 1024.0 < DOT_PRESENT);
    cover = mix(mean, present * (1.0 - smoothstep(1.0 - e / DOT_R, 1.0, d)), dots);
    c = petal * (1.0 + DOT_HEART * (1.0 - d) * dots);
  }
  return vec4f(c, cover * cluster);
}

// The herbs' sward at `at`, seen at `e` m a pixel, over `b`, premultiplied: their colour in stands of their kinds, the
// share they cover (thicker and thinner by `thick`, `patchy`, as far as they hold there, `hold`) seen from above
// (canopy.ts `SWARD_SEEN`) with its tufts, swathes and blades where each shows, in the shade of `canopy` of the trees' crowns,
// reeds on soaked ground and at the water's edge, and flowers over it.
fn sward(at: vec2f, e: f32, b: Beneath, canopy: f32, thick: f32, hold: f32) -> vec4f {
  let f = field_smooth(at, SWARD_LAYER);
  if (f.a <= 0.0 || hold <= 0.0) { return vec4f(0.0); }
  var c = in_stands(at, e, f, SWARD_FIRST, SWARD_STAND_M);
  var s = min(f.a * thick * hold, 1.0);
  let tufts = shows_at(SWARD_M, e);
  var tone = 1.0;
  if (tufts > 0.0) {
    let n = tufts * noise_turned(at / SWARD_M, 54);
    s = clamp(s * (1.0 + SWARD * n), 0.0, 1.0);
    tone += TUFT_TONE * n;
  }
  let swaths = shows_at(SWATH_DETAIL_M, e);
  if (swaths > 0.0) { tone += SWATH * swaths * noise_turned(at / SWATH_M, 73); }
  let blades = shows_at(BLADE_W, e) * swaths;
  if (blades > 0.0) { tone += BLADES * blades * streaks(at, BLADE_W, BLADE_L, PI * noise(at / SWARD_M, 74).x, 75); }
  let reeds = max(ease(REED_WET, 1.0, b.soaked), ease(REED_BAND_M, 0.0, b.over / max(b.slope, 1e-3)));
  if (reeds > 0.0) {
    s = mix(s, max(s, REED_FILL), reeds);
    c = mix(c, c * REED_TINT, reeds);
    tone += REED_TONE * reeds * shows_at(REED_W, e) * streaks(at, REED_W, REED_L, 0.3 * noise(at / REED_L, 76).x, 77);
  }
  let seen = 1.0 - pow(1.0 - s, cover_look.sides.w);
  c *= max(tone, 0.0) * (1.0 - SWARD_SHADE * min(canopy, 1.0));
  let near = shows_at(FLOWERS_GONE_M, e);
  if (near > 0.0) {
    let fl = field_at(at, FLOWER_LAYER, 0.0);
    let bloom = clamp(fl.a / f.a, 0.0, 1.0) * (1.0 - reeds);
    if (bloom > 0.0) {
      let dots = flowers_at(at, e, bloom, fl.rgb / fl.a);
      c = mix(c, dots.rgb, dots.a * near);
    }
  }
  return vec4f(c * seen, seen);
}

// The cover's field at `at` seen at `e` m a pixel over `b`, premultiplied, and the trees' crowns' share: the herbs'
// sward, then each lattice (`lattice_field`), the lower under the taller, thicker and thinner over the patches
// (`patchy`), as far as it holds there (`held_at`); far away, where the sward's and the low lattices' detail no longer
// shows, those three as their one layer (canopy.ts `FIELD.low`).
fn cover_field(at: vec2f, e: f32, b: Beneath) -> CoverAt {
  // The patches as far as they show: where they take less than a pixel the field is their mean (and no noise is read).
  let patches = shows_at(PATCH_M, e);
  let thick = select(1.0, 1.0 + (patchy(at) - 1.0) * patches, patches > 0.0);
  let hold = held_at(at, b);
  // The trees' field, read once: their crowns' share, and their layer below (plainly blended where their marks show in
  // its place: only the shade under them reads it).
  var trees = vec4f(0.0);
  if (cover_look.sides[STANDS] > 0.0) {
    trees = select(field_smooth(at, STANDS), field_at(at, STANDS, 0.0), cover_look.shown[STANDS] >= 1.0);
  }
  // Under the trees: the sward and the low lattices one by one near, as one layer far (`LOW_FAR_E`).
  let far = ease(LOW_FAR_E, LOW_FAR_FULL_E, e);
  var dst = vec4f(0.0);
  if (far < 1.0) {
    dst = sward(at, e, b, trees.a, thick, hold.w);
    for (var l = 0; l < STANDS; l++) { dst = lattice_field(at, e, l, field_smooth(at, l), b, thick, hold[l], dst); }
  }
  if (far > 0.0) { dst = mix(dst, field_smooth(at, LOW_LAYER) * min(hold.w * thick, 1.0), far); }
  dst = lattice_field(at, e, STANDS, trees, b, thick, hold[STANDS], dst);
  return CoverAt(dst, trees.a);
}

// Lattice l's field `f` at `at`, seen at `e` m a pixel over `b`, as far as its marks do not show, thicker and thinner by
// `thick`, holding `hold` of its plants there, with its grain, over `dst` (premultiplied); the trees' in their stands,
// their canopy seen as closer (`CANOPY_SEEN`) and snowy as the ground under it. A lattice no square holds (its side 0,
// textures.ts) adds nothing.
fn lattice_field(at: vec2f, e: f32, l: i32, f: vec4f, b: Beneath, thick: f32, hold: f32, dst: vec4f) -> vec4f {
  let unshown = 1.0 - cover_look.shown[l];
  if (unshown <= 0.0 || cover_look.sides[l] <= 0.0 || f.a <= 0.0) { return dst; }
  var c = f.rgb / f.a;
  var share = min(f.a * hold, 1.0);
  if (share <= 0.0) { return dst; }
  if (l == STANDS) {
    c = in_stands(at, e, f, STANDS_FIRST, STAND_M);
    let groups = shows_at(CANOPY_M, e);
    if (groups > 0.0) { c *= 1.0 + CANOPY_TONE * noise_turned(at / CANOPY_M, 79) * groups; }
    c = mix(c, vec3f(1.0), CANOPY_SNOW * b.snow);
    share = 1.0 - pow(1.0 - share, CANOPY_SEEN);
  }
  let g = grain(at, l, share * thick, e);
  let a = g.x * unshown;
  return vec4f(c * g.y * a, a) + dst * (1.0 - a);
}
