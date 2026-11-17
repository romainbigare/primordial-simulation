// The plant cover as a field (web/src/canopy.ts `CoverField`), drawn in the live map's ground (land.wgsl) where its
// plants take too few pixels to be drawn as marks (cover.wgsl): of each lattice of marks the colour its plants cover
// the ground with and the share they cover, blended between the squares' centres, at their mean, the ground between
// darkened by their shadows; and the herbs as a sward with their flowers, at every scale. So the far and the near pictures come from
// the same numbers. Nothing stands where liquid stands over the roots or under snow.

struct CoverLook {
  // Of each lattice: its cell's side, m (0: no square holds it), and 1 where any of its marks takes enough pixels to show
  // (each by the width of its lattice's marks where it stands, the rest is the field, `lattice_field`); w is not read.
  sides: vec4f, shown: vec4f,
  // 1 where the cover is drawn (0: none); how strongly its marks cover the map; 1 where the plants stand up (angled).
  look: vec4f,
}
@group(0) @binding(14) var<uniform> cover_look: CoverLook;
@group(0) @binding(15) var cover_field_tex: texture_2d_array<f32>;
@group(0) @binding(16) var cover_field_sampler: sampler;

// The layers of the field after the lattices' (canopy.ts `FIELD`): the herbs' sward and their flowers, and the part of
// the sward and of the lattice of stands (`STANDS`, the trees') that is its first kind.
const SWARD_LAYER = 3; const FLOWER_LAYER = 4; const SWARD_FIRST = 5; const STANDS_FIRST = 6;
// Of each lattice and the sward (red, green, blue, alpha): its plants' mean height, m, the mean share of the square that
// holds them and the mean depth of loose ground their roots need, m (the core's: `Cover::rooted`, `Cover::need`).
const TALL_LAYER = 7; const ROOTED_LAYER = 8; const NEED_LAYER = 9;
// Of each lattice: its marks' mean width, m (canopy.ts `FIELD.wide`), and the share of the ground its shadows fall on
// (`FIELD.shade`), which they darken by `FIELD_SHADOW` (canopy.ts `CROWN_SHADOW`).
const WIDE_LAYER = 10; const SHADE_LAYER = 11; const FIELD_SHADOW = 0.42;
// The sward and the low lattices as one layer (canopy.ts `lowOf`), drawn in their place where a pixel takes
// `LOW_FAR_E` m and more (where their detail no longer shows: no swathes, blades, tufts or flowers past a metre a
// pixel), fully from `LOW_FAR_FULL_E`; thicker and thinner over the patches as they are.
const LOW_LAYER = 12; const LOW_FAR_E = 0.75; const LOW_FAR_FULL_E = 1.0;
// Of each square, 1 where its cell holds land (canopy.ts `FIELD.land`); a square with no land holds 0 in every layer. A
// blend between the squares' centres is over the squares with land alone, their weights made whole (`on_land`): the
// cover is drawn on land only, and a square with no land is no bare ground beside it. A blend whose land weighs less
// than `LAND_LEAST` is none (it lies out in the liquid, where no cover is drawn).
const LAND_LAYER = 13; const LAND_LEAST = 1e-3;
// Of the sward and of the lattice of stands, what the kinds but the first make (canopy.ts `FIELD`).
const SWARD_REST = 14; const STANDS_REST = 15;
const STANDS = 2;
// Soft patches: the plants stand thicker and thinner over patches about `PATCH_M` m across, by up to `PATCH` of their
// mean (fixed noise in the world), alike near (cover.wgsl) and far.
const PATCH_M = 8.0; const PATCH = 0.45;

// Kinds grow in stands of their own: a plant is of its square's first kind (canopy.ts `Paint`) where a smooth draw fixed
// in the world, mixed with `STAND_MIX` of the plant's own draw, falls within the first kind's share; so a kind's stands
// are patches about as large as the noise's cells (`STAND_M` the trees', `SWARD_STAND_M` the herbs') with mixed edges,
// and each kind keeps its share (draw.wgsl `noise_turned`, made even over 0..1).
const STAND_M = 60.0; const SWARD_STAND_M = 5.0; const STAND_MIX = 0.4;
// Far away a canopy's tone varies gently: the stands fade into their mean where a pixel takes from `STAND_CALM_E` to
// twice as many m (by a 500 m scale bar), so only real differences (forest and meadow, leafy and bare, turned and
// green) show there.
const STAND_CALM_E = 1.5;
// Thin ground is poor: the herbs cover from `THIN_COVER` of their share where the loose ground is `THIN_FROM_M` m deep
// or less to all of it from `THIN_TO_M` m, so the ground's own colour shows through toward the bed's.
const THIN_COVER = 0.25; const THIN_FROM_M = 0.05; const THIN_TO_M = 0.5;
// The trees' canopy is lighter and darker by up to `CANOPY_TONE` over groups of crowns `CANOPY_M` across.
const CANOPY_TONE = 0.2; const CANOPY_M = 35.0;
// The light the trees leave the kinds below on the mean of a square is counted no less than this, so a square its
// trees close does not heap its low kinds on its last open ground.
const LIT_LEAST = 0.05;
// Snow lies on the trees' crowns as on the ground under them, up to this share of their colour seen from afar (the
// evergreen ones hold it; the bare ones are thin and dark).
const CANOPY_SNOW = 0.35;

// What lies under and over the cover at a point (land.wgsl `land_spots`): how soaked its ground is, 0..1; its height
// over the nearest water's level, m, and its slope (rise over run); the depth of its loose ground with its patches
// (rock.wgsl `loose_seen`) and that depth's change across a pixel, m; the share snow covers and the snow's depth, m.
struct Beneath { soaked: f32, over: f32, slope: f32, loose: f32, loose_px: f32, snow: f32, snow_m: f32 }

// The cover's field at a point, premultiplied, and the share the trees' crowns close over the ground there.
struct CoverAt { cover: vec4f, canopy: f32 }

// A square whose kinds' roots hold everywhere has its mean rooted share this near 1: no point of it limits them.
const ROOTED_ALL = 0.999;

// Of each lattice and the sward (red, green, blue, alpha), the share of its plants that stand at a point over `b`:
// where their roots hold (land.wgsl `rooted`, over the share of their square that holds them, so the square keeps its
// cover) and out of the snow (draw.wgsl `above_snow`). Each layer is read only where it may limit them.
fn held_at(at: vec2f, b: Beneath) -> vec4f {
  let mean = field_at(at, ROOTED_LAYER);
  let roots = any(mean < vec4f(ROOTED_ALL));
  // The snow over the point: its cells' depth where it lies, none where it does not (as the stones, stone.wgsl).
  let buried = b.snow * b.snow_m;
  if (!roots && buried <= 0.0) { return vec4f(1.0); }
  var out = vec4f(1.0);
  if (roots) {
    let need = field_at(at, NEED_LAYER);
    for (var c = 0; c < 4; c++) { out[c] = rooted(b.loose, need[c], b.loose_px) / max(mean[c], 1e-3); }
  }
  if (buried > 0.0) {
    let tall = field_at(at, TALL_LAYER);
    for (var c = 0; c < 4; c++) { out[c] *= above_snow(buried, tall[c]); }
  }
  return out;
}

// The herbs' sward: their plants thicker and thinner by up to `SWARD` of their mean over tufts of `SWARD_M`; they cover
// the share of the ground their leaves close by the cover's light law (canopy.ts `Paint`, tape/src/stands.rs `plants`),
// so a thin sward lets the ground show through. Its tone varies by up to `TUFT_TONE` over the tufts, by up to `SWATH` over
// swathes of `SWATH_M`, and near it is drawn as its tufts of blades (`tuft_tone`).
const SWARD = 0.45; const SWARD_M = 0.35;
// Under the trees' crowns the sward is in their shade: darker by up to `SWARD_SHADE` as they close over it.
const SWARD_SHADE = 0.35;
const TUFT_TONE = 0.15; const SWATH = 0.18; const SWATH_M = 3.0; const SWARD_STANDS_NEAR = 0.5;
// A sward whose red over its green passes from `STRAW_HUE.x` to `.y` is straw: angled, its grass is drawn dry.
const STRAW_HUE = vec2f(0.85, 1.1);
// Angled, a sward that covers up to `TUFTS_THICK.x` of the ground is drawn as tufts alone, from `.y` on as its tile.
const TUFTS_THICK = vec2f(0.4, 0.75);
// Under the blades drawn near, the sward's tiles show this light (the ground between the blades in their shade).
const BLADE_GROUND = 0.6;
// The blades show from `BLADE_SEEN_E.y` m a pixel, fully from `.x` in.
const BLADE_SEEN_E = vec2f(0.004, 0.012);
// Swathes and blades show as detail this size, m (`shows_at`): none where a pixel takes 1 m or more, where they would
// read only as noise.
const SWATH_DETAIL_M = 1.33;
// Reeds: herbs on soaked ground (from `REED_WET`) and on the shore of standing or running water, out to `REED_BAND_M`
// m from its edge (its height over the water's level over the slope), stand dense (`REED_FILL` of the ground), their colour
// times `REED_TINT`; seen from above in streaks of tall blades `REED_W` wide, `REED_L` long, `REED_TONE` deep.
const REED_WET = 0.85; const REED_BAND_M = 5.0; const REED_FILL = 0.95; const REED_TINT = vec3f(0.75, 0.8, 0.45);
const REED_W = 0.07; const REED_L = 0.9; const REED_TONE = 0.5;
// How much reeds stand at a place: on soaked ground and in the water's edge band (`REED_WET`, `REED_BAND_M`).
fn reeds_of(b: Beneath) -> f32 {
  return max(ease(REED_WET, 1.0, b.soaked), ease(REED_BAND_M, 0.0, b.over / max(b.slope, 1e-3)));
}
// Flowers: in clusters as large as the noise's cells, `FLOWER_M`, over as much of the sward as is in flower; in a
// cluster the dots, of radius `DOT_R`, lie on two lattices of cells `DOT_GAP` across, each turned its own way off the
// map's axes (`DOT_TURN`, radians) and holding a dot at a place of its own in half of `DOT_PRESENT` of its cells, so
// the dots stand in no rows and no square grid; a dot is lighter at its heart by `DOT_HEART`; where a dot takes less than a pixel, the cluster's mean; they show as detail
// `FLOWERS_GONE_M` across (`shows_at`): none where a pixel takes a metre or more (their faint tint gone from afar).
const FLOWERS_GONE_M = 1.33; const FLOWER_M = 1.2; const DOT_R = 0.045; const DOT_GAP = 0.171; const DOT_PRESENT = 0.8; const DOT_HEART = 0.25;
const DOT_TURN = vec2f(0.55, 2.1);
// A flower's eye, the inner third of its dot.
const FLOWER_EYE = vec3f(0.75, 0.55, 0.12);

// A blend `v` of a layer over the squares about a point, of which the squares with land weigh `held`, as a blend over
// those squares alone (`LAND_LAYER`).
fn on_land(v: vec4f, held: f32) -> vec4f { return select(vec4f(0.0), v / held, held > LAND_LEAST); }

// What the squares with land weigh at the point `cover_field` draws, plainly blended (`field_at`) and through the
// squares' centres (`field_smooth`): read once a pixel (`land_weights`), as every layer read there shares them. Where
// no cover field is drawn (the marks, cover.wgsl) they stay 1 and a layer is read as it is.
var<private> land_plain: f32 = 1.0;
var<private> land_smooth: f32 = 1.0;

// Set `land_plain` and `land_smooth` at `at`, m from the map's north-west corner.
fn land_weights(at: vec2f) {
  let uv = at / (land.map.x * land.map.yz);
  land_plain = select(field_tap(uv, LAND_LAYER).x, 0.0, any(uv < vec2f(0.0)) || any(uv > vec2f(1.0)));
  land_smooth = cubic_field(at, LAND_LAYER).x;
}

// The field of layer l at `at` through its texels' centres as a cubic B-spline read texel by texel (draw.wgsl `cubic`).
fn cubic_field(at: vec2f, l: i32) -> vec4f {
  let c = cubic(at / land.map.x - 0.5);
  let last = vec2i(textureDimensions(cover_field_tex).xy) - 1;
  var sum = vec4f(0.0);
  for (var y = 0; y < 4; y++) {
    for (var x = 0; x < 4; x++) {
      sum += c.wx[x] * c.wy[y] * textureLoad(cover_field_tex, clamp(c.i + vec2i(x - 1, y - 1), vec2i(0), last), l, 0);
    }
  }
  return sum;
}

// The field of lattice l at `at`, m from the map's north-west corner: the colour its marks cover the ground with (a
// reflectance, premultiplied) and the share they cover, blended over the squares with land (`on_land`, `land_plain`).
fn field_at(at: vec2f, l: i32) -> vec4f {
  let uv = at / (land.map.x * land.map.yz);
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  return on_land(field_tap(uv, l), land_plain);
}

// The field of lattice l at uv, blended between its texels.
fn field_tap(uv: vec2f, l: i32) -> vec4f { return textureSampleLevel(cover_field_tex, cover_field_sampler, uv, l, 0.0); }

// The field of lattice l at `at` through its texels' centres as a cubic B-spline read texel by texel (draw.wgsl
// `cubic`) over the squares with land (`on_land`, `land_smooth`): no square shows in it, and no staircase of the GPU's
// blend.
fn field_smooth(at: vec2f, l: i32) -> vec4f { return on_land(cubic_field(at, l), land_smooth); }

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
// about `m` m: its first kind's (layer `first`) where its stands are, the rest's (layer `rest`) elsewhere; their mean
// where a stand takes less than a pixel. The first kind's share and the rest's colour are read between the texels as
// plainly blended, both alike, so the rest's colour is its kinds' own; a rest under `REST_LEAST` of the layer is too
// small to tell its colour from the field's rounding, and the layer is its mean there (in full from twice that share).
const REST_LEAST = 0.01;
fn in_stands(at: vec2f, e: f32, f: vec4f, l: i32, first: i32, rest: i32, m: f32) -> vec3f {
  let mean = f.rgb / f.a;
  if (least_e() >= 2.0 * STAND_CALM_E) { return mean; }
  let shows = shows_at(m, e) * (1.0 - ease(STAND_CALM_E, 2.0 * STAND_CALM_E, e));
  if (shows <= 0.0) { return mean; }
  let whole = field_at(at, l);
  let one = field_at(at, first);
  let others = field_at(at, rest);
  let told = ease(REST_LEAST, 2.0 * REST_LEAST, others.a / max(whole.a, 1e-6));
  if (told <= 0.0) { return mean; }
  let own = one.rgb / max(one.a, 1e-6);
  let other = others.rgb / max(others.a, 1e-6);
  return mix(mean, mix(other, own, stand_share(stand_draw(at, m), clamp(one.a / whole.a, 0.0, 1.0))), shows * told);
}

// Streaks of blades `w` m wide and `len` m long at the pixel, turned by `turn` radians: -1..1 (noise stretched along
// the turn). A turn lies between the nearest two of `STREAK_TURNS` turns that stand fixed in the world, whose streaks
// blend (kept as strong as one), so no lattice turns about the world's origin as the blades turn with the tufts.
const STREAK_TURNS = 3.0;
fn streaks(w: f32, len: f32, turn: f32, k: i32) -> f32 {
  let t = turn / PI * STREAK_TURNS;
  let x = t - STREAK_TURNS * floor(t / STREAK_TURNS);
  let b = floor(x);
  let u = smoothstep(0.0, 1.0, x - b);
  var n = 0.0;
  for (var j = 0; j < 2; j++) {
    let a = PI * (b + f32(j)) / STREAK_TURNS;
    let m = mat2x2f(cos(a) / w, -sin(a) / len, sin(a) / w, cos(a) / len);
    n += select(1.0 - u, u, j == 1) * noise_l(lat(m, vec2f(0.0)), k + 100 * j).x;
  }
  return n / sqrt((1.0 - u) * (1.0 - u) + u * u);
}

// The flowers over the sward at `at`, seen at `e` m a pixel, of the herbs whose share `bloom` is in flower: their
// colour and the share of the ground they cover (the sward's own share times it): clusters (`FLOWER_M`) of dots
// (`DOT_R`) seen from above, of flower heads angled.
fn flowers_at(e: f32, bloom: f32, petal: vec3f) -> vec4f {
  // The clusters: where the noise, made even, falls in the share in flower; softened over a pixel.
  let n = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(FLOWER_M, vec2f(0.0)), 71).x);
  let edge = max(e / FLOWER_M, 0.05);
  let cluster = mix(bloom, ease(1.0 - bloom - edge, 1.0 - bloom + edge, n), shows_at(FLOWER_M, e));
  let mean = DOT_PRESENT * PI * DOT_R * DOT_R / (DOT_GAP * DOT_GAP);
  let dots = shows_at(2.0 * DOT_R, e);
  var cover = mean;
  var c = petal;
  if (dots > 0.0 && cluster > 0.0 && cover_look.look.z > 0.5) {
    // Angled, their heads in the tile of petals (detail.wgsl `flower_heads`).
    let h = flower_heads(e);
    cover = mix(mean, h.x, dots);
    c = petal * h.y;
  } else if (dots > 0.0 && cluster > 0.0) {
    let margin = DOT_R / DOT_GAP;
    // The nearest dot of the two lattices: how far from its centre the point lies, in its radii (past it: none).
    var d = 1.0;
    for (var l = 0; l < 2; l++) {
      let q = lat(mat2x2f(cos(DOT_TURN[l]), sin(DOT_TURN[l]), -sin(DOT_TURN[l]), cos(DOT_TURN[l])) * (1.0 / DOT_GAP), vec2f(13.0 * f32(l)));
      let x = lowbias(q.i, 72 + l);
      let centre = margin + (1.0 - 2.0 * margin) * vec2f(f32(x & 1023u), f32((x >> 10u) & 1023u)) / 1024.0;
      if (f32(x >> 22u) / 1024.0 < 0.5 * DOT_PRESENT) { d = min(d, length(q.f - centre) * DOT_GAP / DOT_R); }
    }
    cover = mix(mean, 1.0 - smoothstep(1.0 - e / DOT_R, 1.0, d), dots);
    c = mix(petal * (1.0 + DOT_HEART * (1.0 - d) * dots), FLOWER_EYE, smoothstep(0.4, 0.25, d) * dots);
  }
  return vec4f(c, cover * cluster);
}

// The herbs' sward at `at`, seen at `e` m a pixel, over `b`, premultiplied: their colour in stands of their kinds, the
// share they cover (thicker and thinner by `thick`, `patchy`, as far as they hold there, `hold`) with its tufts, swathes and blades where each shows, in the shade of `canopy` of the trees' crowns,
// reeds on soaked ground and at the water's edge, and flowers over it.
fn sward(at: vec2f, e: f32, b: Beneath, canopy: f32, thick: f32, hold: f32) -> vec4f {
  let f = field_smooth(at, SWARD_LAYER);
  if (f.a <= 0.0 || hold <= 0.0) { return vec4f(0.0); }
  // Angled, near, its images give its detail (detail.wgsl `grass_detail`): the drawn tufts' and swathes' tone fades out
  // as they show, and its kinds' stands keep `SWARD_STANDS_NEAR` of their difference from the mean, so the layers do not
  // pile into smudges.
  let near_images = select(0.0, shows_at(SWATH_DETAIL_M, e), cover_look.look.z > 0.5);
  let mean = f.rgb / f.a;
  var c = in_stands(at, e, f, SWARD_LAYER, SWARD_FIRST, SWARD_REST, SWARD_STAND_M);
  c = mix(c, mean + SWARD_STANDS_NEAR * (c - mean), near_images);
  var s = min(f.a * thick * hold, 1.0);
  let tufts = shows_at(SWARD_M, e);
  var tone = 1.0;
  if (tufts > 0.0) {
    let n = tufts * noise_l(lat_turned(SWARD_M, vec2f(0.0)), 54).x;
    s = clamp(s * (1.0 + SWARD * n), 0.0, 1.0);
    tone += TUFT_TONE * n * (1.0 - near_images);
  }
  let swaths = shows_at(SWATH_DETAIL_M, e);
  if (swaths > 0.0) { tone += SWATH * swaths * (1.0 - near_images) * noise_l(lat_turned(SWATH_M, vec2f(0.0)), 73).x; }
  // From above, its tufts; angled, its grass stands up in its images (detail.wgsl `grass_detail`).
  let blades_seen = blades_show(e);
  if (blades_seen > 0.0 && cover_look.look.z < 0.5) { tone *= mix(1.0, tuft_tone(), blades_seen); }
  let reeds = reeds_of(b);
  if (reeds > 0.0) {
    s = mix(s, max(s, REED_FILL), reeds);
    c = mix(c, c * REED_TINT, reeds);
    // From above, in streaks of blades; angled, its blades stand up in their image (detail.wgsl `grass_detail`).
    if (cover_look.look.z < 0.5) { tone += REED_TONE * reeds * shows_at(REED_W, e) * streaks(REED_W, REED_L, 0.3 * noise_l(lat_axes(REED_L, vec2f(0.0)), 76).x, 77); }
  }
  // Its grass, straw where its colour is straw's.
  let straw = smoothstep(STRAW_HUE.x, STRAW_HUE.y, c.r / max(c.g, 1e-4));
  let plain = c * max(tone, 0.0) * (1.0 - SWARD_SHADE * min(canopy, 1.0));
  c = plain * grass_detail(e, straw, reeds, s);
  // Angled and very near, its blades drawn one by one (detail.wgsl `blades`) over its tiles in their shade
  // (`BLADE_GROUND`), coming in smoothly as a pixel passes from `BLADE_SEEN_E.y` to `.x` m.
  if (cover_look.look.z > 0.5) {
    let near = 1.0 - ease(BLADE_SEEN_E.x, BLADE_SEEN_E.y, e);
    if (near > 0.0) {
      let b = blades(e);
      c = mix(c, mix(c * BLADE_GROUND, plain * b.x, b.y), near);
    }
  }
  // Angled, where its tufts show as things, a thin sward is its tufts standing on bare ground (detail.wgsl
  // `sward_tufts`), not a veil: fully where it covers up to `TUFTS_THICK.x` of the ground, its tile of grass from `.y` on.
  var seen = s;
  if (cover_look.look.z > 0.5) {
    let thin = 1.0 - smoothstep(TUFTS_THICK.x, TUFTS_THICK.y, s);
    if (thin > 0.0) {
      let t = sward_tufts(plain, c, e, s);
      c = mix(c, t.rgb, thin);
      seen = mix(s, t.a, thin);
    }
  }
  let near = shows_at(FLOWERS_GONE_M, e);
  if (near > 0.0) {
    let fl = field_at(at, FLOWER_LAYER);
    let bloom = clamp(fl.a / f.a, 0.0, 1.0) * (1.0 - reeds);
    if (bloom > 0.0) {
      let dots = flowers_at(e, bloom, fl.rgb / fl.a);
      c = mix(c, dots.rgb, dots.a * near);
    }
  }
  return vec4f(c * seen, seen);
}

// The sward's tufts seen from above: one in `TUFT_FILL` of the cells of a lattice `TUFT_M` m wide (at random places in
// them, so no rows), each a star of `TUFT_BLADES` blades up to `TUFT_TALL` times its cell long, each `TUFT_FAN` of a
// half turn wide. A blade is dark at its foot and light at its tip (`TUFT_FOOT` to `TUFT_TIP` of the sward's tone), the
// ground between the tufts in their shade (`TUFT_GAP`), so the sward's mean stays its colour. They show from
// `TUFT_SEEN_M` m a pixel.
const TUFT_M = 0.22; const TUFT_FILL = 0.75; const TUFT_BLADES = 9.0; const TUFT_TALL = 1.2; const TUFT_FAN = 0.85;
// Each tuft's tone departs from the sward's by up to `TUFT_OWN` either way.
const TUFT_OWN = 0.15;
const TUFT_FOOT = 0.62; const TUFT_TIP = 1.3; const TUFT_GAP = 0.8; const TUFT_SEEN_M = 0.1;
// How much the sward's tufts show at e m a pixel.
fn blades_show(e: f32) -> f32 { return shows_at(TUFT_SEEN_M, e); }

// The tone the tufts give the sward at the pixel.
fn tuft_tone() -> f32 {
  let l = lat_axes(TUFT_M, vec2f(0.0));
  var tone = TUFT_GAP;
  for (var j = -1; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let c = l.i + vec2i(i, j);
      let x = lowbias(c, 91);
      if (f32(x & 1023u) / 1024.0 >= TUFT_FILL) { continue; }
      let foot = vec2f(f32((x >> 10u) & 1023u), f32((x >> 20u) & 1023u)) / 1024.0;
      // From the tuft's foot to the pixel, in cells; its length, in cells, by its own draw.
      let q = l.f - vec2f(f32(i), f32(j)) - foot;
      let len = TUFT_TALL * (0.6 + 0.4 * f32(x >> 30u) / 3.0);
      let v = length(q) / (0.5 * len);
      let turn = atan2(q.y, q.x) / PI * TUFT_FAN;
      // Its blades, each of its own length.
      let k = TUFT_BLADES * turn / TUFT_FAN;
      let blade = floor(k);
      let reach = 0.55 + 0.45 * fract(f32(x >> 3u) * 0.0137 + 0.61 * blade);
      let w = 1.0 - smoothstep(0.18, 0.42, abs(fract(k) - 0.5));
      if (v > reach || w <= 0.4) { continue; }
      tone = mix(TUFT_FOOT, TUFT_TIP, v / reach) * (1.0 + TUFT_OWN * (f32((x >> 4u) & 63u) / 31.5 - 1.0));
    }
  }
  return tone;
}

// The cover's field at `at` seen at `e` m a pixel over `b`, premultiplied, and the trees' crowns' share: the herbs'
// sward, then each lattice (`lattice_field`), the lower under the taller, thicker and thinner over the patches
// (`patchy`), as far as it holds there (`held_at`); far away, where the sward's and the low lattices' detail no longer
// shows, those three as their one layer (canopy.ts `FIELD.low`).
fn cover_field(at: vec2f, e: f32, b: Beneath) -> CoverAt {
  land_weights(at);
  // The patches as far as they show: where they take less than a pixel the field is their mean (and no noise is read).
  let patches = shows_at(PATCH_M, e);
  let thick = select(1.0, 1.0 + (patchy(at) - 1.0) * patches, patches > 0.0);
  // The plants' roots hold where the marks' do (mark.wgsl `holds`, on every patch of the loose ground), over a pixel and
  // over the patches too small to show here (`loose_unseen`), so the field far away and the marks near stand on the
  // same ground.
  var bs = b;
  bs.loose_px = max(b.loose_px, loose_unseen(at, e));
  var hold = held_at(at, bs);
  hold.w *= mix(THIN_COVER, 1.0, ease(THIN_FROM_M - 0.5 * bs.loose_px, THIN_TO_M + 0.5 * bs.loose_px, b.loose));
  // The trees' field, read once: their crowns' share.
  var trees = vec4f(0.0);
  if (cover_look.sides[STANDS] > 0.0) { trees = field_smooth(at, STANDS); }
  // The low kinds take the light the trees leave (the cover's light law, laws.md 3: a kind below gets the light of
  // the ground the taller does not hold and what passes its layer): they thin under the crowns that stand at the point
  // and stand as much thicker on the ground that holds no tree, their square keeping its cover. So ground too thin for
  // the trees shows the sward and the bushes of its cell before it shows bare.
  let closed = min(trees.a, 1.0);
  let lit = (1.0 - min(closed * hold[STANDS], 1.0)) / max(1.0 - closed, LIT_LEAST);
  // The bushes thin in the shade but do not heap in the gaps: only the sward, which fills ground, thickens there.
  hold = vec4f(hold.xy * min(lit, 1.0), hold.z, hold.w * lit);
  // Under the trees: the sward and the low lattices one by one near, as one layer far (`LOW_FAR_E`).
  let far = ease(LOW_FAR_E, LOW_FAR_FULL_E, e);
  var dst = vec4f(0.0);
  if (least_e() < LOW_FAR_FULL_E && far < 1.0) {
    dst = sward(at, e, b, trees.a, thick, hold.w);
    // A lattice no square holds adds nothing: its field is not read.
    for (var l = 0; l < STANDS; l++) {
      if (cover_look.sides[l] > 0.0) { dst = lattice_field(at, e, l, field_smooth(at, l), b, thick, hold[l], dst); }
    }
  }
  if (far > 0.0) {
    // As thick as they stand at the point (`hold`, `thick`), up to all of the ground.
    // In the shade of the trees' crowns as the sward is near (`SWARD_SHADE`), so the far and the near tone agree.
    let low = field_smooth(at, LOW_LAYER);
    var seen = low * (min(low.a * hold.w * thick, 1.0) / max(low.a, 1e-6));
    // Reeds on soaked ground and at the water's edge stand dense and in their own tint, as near (`sward`).
    let reeds = reeds_of(b) * step(1e-4, seen.a);
    let filled = mix(seen.a, max(seen.a, REED_FILL), reeds);
    seen = vec4f(seen.rgb / max(seen.a, 1e-6) * mix(vec3f(1.0), REED_TINT, reeds) * filled, filled);
    dst = mix(dst, vec4f(seen.rgb * (1.0 - SWARD_SHADE * min(trees.a, 1.0)), seen.a), far);
  }
  dst = lattice_field(at, e, STANDS, trees, b, thick, hold[STANDS], dst);
  // The crowns that stand over the point shade its floor (crag.wgsl `floor_of`).
  return CoverAt(dst, min(closed * hold[STANDS], 1.0));
}

// Lattice l's field `f` at `at`, seen at `e` m a pixel over `b`, as far as its marks do not show (draw.wgsl `as_thing`
// by the width of its marks there, as each mark shows, mark.wgsl `decide`), thicker and thinner by `thick`, holding
// `hold` of its plants there, over `dst` (premultiplied): its shadows darken what lies below, its crowns cover their
// share at their mean; the trees' in their stands, in groups of lighter and darker crowns, and snowy as the ground under
// them. A lattice no square holds (its side 0, textures.ts) adds nothing.
fn lattice_field(at: vec2f, e: f32, l: i32, f: vec4f, b: Beneath, thick: f32, hold: f32, dst: vec4f) -> vec4f {
  if (cover_look.sides[l] <= 0.0 || f.a <= 0.0) { return dst; }
  var unshown = 1.0;
  if (cover_look.shown[l] > 0.0) { unshown = 1.0 - as_thing(field_at(at, WIDE_LAYER)[l], e); }
  let share = min(f.a * hold * thick, 1.0) * unshown;
  if (share <= 0.0) { return dst; }
  var c = f.rgb / f.a;
  if (l == STANDS) {
    c = in_stands(at, e, f, STANDS, STANDS_FIRST, STANDS_REST, STAND_M);
    let groups = shows_at(CANOPY_M, e);
    if (groups > 0.0) { c *= 1.0 + CANOPY_TONE * noise_turned(at / CANOPY_M, 79) * groups; }
    c = mix(c, vec3f(1.0), CANOPY_SNOW * b.snow);
  }
  let dark = FIELD_SHADOW * min(field_at(at, SHADE_LAYER)[l] * hold * thick, 1.0) * unshown;
  let below = vec4f(dst.rgb * (1.0 - dark), dark + dst.a * (1.0 - dark));
  return vec4f(c * share, share) + below * (1.0 - share);
}
