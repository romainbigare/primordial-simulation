// The plant cover as a field (web/src/canopy.ts `CoverField`), drawn in the live map's ground (land.wgsl) where its
// plants take too few pixels to be drawn as marks (cover.wgsl): of each lattice of marks the colour its plants cover
// the ground with and the share they cover, blended between the squares' centres, as crowns no smaller than the eye
// reads, and far away as their mean; and the herbs as a sward with their flowers, at every scale. So the far and the near pictures come from
// the same numbers. Nothing stands where liquid stands over the roots or under snow.

struct CoverLook {
  // Of each lattice: its cell's side, m (0: no square holds it), and how strongly its marks show (the rest is the field);
  // w is not read.
  sides: vec4f, shown: vec4f,
  // 1 where the cover is drawn (0: none); how strongly its marks cover the map.
  look: vec4f,
}
@group(0) @binding(14) var<uniform> cover_look: CoverLook;
@group(0) @binding(15) var cover_field_tex: texture_2d_array<f32>;
@group(0) @binding(16) var cover_field_sampler: sampler;

const COVER_LATTICES = 3;
// The layers of the field after the lattices' (canopy.ts `FIELD`): the herbs' sward and their flowers, and the part of
// the sward and of the lattice of stands (`STANDS`, the trees') that is its first kind.
const SWARD_LAYER = 3; const FLOWER_LAYER = 4; const SWARD_FIRST = 5; const STANDS_FIRST = 6;
// Of each lattice and the sward (red, green, blue, alpha): its plants' mean height, m, the mean share of the square that
// holds them and the mean depth of loose ground their roots need, m (the core's: `Cover::rooted`, `Cover::need`).
const TALL_LAYER = 7; const ROOTED_LAYER = 8; const NEED_LAYER = 9;
// Of each lattice: its marks' mean width, m (canopy.ts `FIELD.wide`).
const WIDE_LAYER = 10;
// The sward and the low lattices as one layer (canopy.ts `lowOf`), drawn in their place where a pixel takes
// `LOW_FAR_E` m and more (where their detail no longer shows: no swathes, blades, tufts or flowers past a metre a
// pixel), fully from `LOW_FAR_FULL_E`; thicker and thinner over the patches as they are.
const LOW_LAYER = 11; const LOW_FAR_E = 0.75; const LOW_FAR_FULL_E = 1.0;
// Of each square, 1 where its cell holds land (canopy.ts `FIELD.land`); a square with no land holds 0 in every layer. A
// blend between the squares' centres is over the squares with land alone, their weights made whole (`on_land`): the
// cover is drawn on land only, and a square with no land is no bare ground beside it. A blend whose land weighs less
// than `LAND_LEAST` is none (it lies out in the liquid, where no cover is drawn).
const LAND_LAYER = 12; const LAND_LEAST = 1e-3;
// Of the sward and of the lattice of stands, what the kinds but the first make (canopy.ts `FIELD`).
const SWARD_REST = 13; const STANDS_REST = 14;
const STANDS = 2;
// A crown of the field (`crowns`), where its plants take too few pixels to be drawn as marks: a disc in a cell, present
// in a cell as often as covers at most `GRAIN_FILL` of it, as wide as keeps the mean (at most half a cell), anywhere in
// its cell, so no crowns stand in rows and none is cut at its cell's edge. Each is lit from the north-west, its light
// departing from the flat ground's by up to `GRAIN_LIGHT`, and its tone its own, up to `GRAIN_TONE` either way.
const GRAIN_FILL = 0.35; const GRAIN_LIGHT = 0.3; const GRAIN_TONE = 0.1;
// The pixel's place in the cells is moved by up to `GRAIN_WARP` cells by smooth noise `WARP_CELLS` cells across, so rows
// and columns of cells show no rows and columns in the crowns (a crown stays within half a cell of the place it is
// drawn at, which the warp moves as a whole).
const GRAIN_WARP = 0.35; const WARP_CELLS = 3.0;
// A crown of the field is as wide as its plants' marks, and is drawn only where those span `CLUMP_PX` css pixels: where
// a plant takes less, the field is its plants' mean, as seen from afar, and no grain coarser than the plants stands
// for them. Its cells are a lattice's cells doubled as often as the width takes; between two doublings a wide cell
// holds one crown where the width, as a power of two, passes a draw of the cell between `WIDE_FROM` and `WIDE_TO`, else
// its four cells hold theirs, and no line runs between two sizes. A crown wider than `CLUMP_MOST_M` m fades into its
// plants' mean, a calm tone, by twice as wide.
const CLUMP_PX = 1.5; const CLUMP_MOST_M = 10.0; const WIDE_FROM = 0.25; const WIDE_TO = 0.75;
// Discs at random places leave gaps however closed their stand: as a layer's plants cover from `CLOSES.x` to `CLOSES.y`
// of the ground, the gaps between its crowns fill with its canopy, `GAP_TONE` as light as a crown (their shade).
const CLOSES = vec2f(0.6, 0.9); const GAP_TONE = 0.65;
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
// Seen from above the trees' crowns with their shade cover more than their share, as the sward's: the ground they leave
// is (1 - share)^`CANOPY_SEEN`; their canopy is lighter and darker by up to `CANOPY_TONE` over groups of crowns
// `CANOPY_M` across.
const CANOPY_SEEN = 2.0; const CANOPY_TONE = 0.2; const CANOPY_M = 35.0;
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

// The cover's field at a point, premultiplied, the share the trees' crowns close over the ground there, and their
// leaves' colour.
struct CoverAt { cover: vec4f, canopy: f32, leaf: vec3f }

// A square whose kinds' roots hold everywhere has its mean rooted share this near 1: no point of it limits them.
const ROOTED_ALL = 0.999;

// Of each lattice and the sward (red, green, blue, alpha), the share of its plants that stand at a point over `b`:
// where their roots hold (land.wgsl `rooted`, over the share of their square that holds them, so the square keeps its
// cover) and out of the snow (draw.wgsl `above_snow`). Each layer is read only where it may limit them.
fn held_at(at: vec2f, b: Beneath) -> vec4f {
  let mean = field_at(at, ROOTED_LAYER, 0.0);
  let roots = any(mean < vec4f(ROOTED_ALL));
  if (!roots && b.snow_m <= 0.0) { return vec4f(1.0); }
  var out = vec4f(1.0);
  if (roots) {
    let need = field_at(at, NEED_LAYER, 0.0);
    for (var c = 0; c < 4; c++) { out[c] = rooted(b.loose, need[c], b.loose_px) / max(mean[c], 1e-3); }
  }
  if (b.snow_m > 0.0) {
    let tall = field_at(at, TALL_LAYER, 0.0);
    for (var c = 0; c < 4; c++) { out[c] *= above_snow(b.snow_m, tall[c]); }
  }
  return out;
}

// The herbs' sward: their plants thicker and thinner by up to `SWARD` of their mean over tufts of `SWARD_M`; they cover
// the share of the ground their leaves close by the cover's light law (canopy.ts `Paint`, tape/src/stands.rs `plants`),
// so a thin sward lets the ground show through. Its tone varies by up to `TUFT_TONE` over the tufts, by up to `SWATH` over
// swathes of `SWATH_M` and and by tufts of fanned blades.
const SWARD = 0.45; const SWARD_M = 0.35;
// Under the trees' crowns the sward is in their shade: darker by up to `SWARD_SHADE` as they close over it.
const SWARD_SHADE = 0.35;
const TUFT_TONE = 0.15; const SWATH = 0.18; const SWATH_M = 3.0;
// Tufts (floor.wgsl `fan`): cells of `TUFT_FAN_M` and `TUFT_BIG_M` m, the big ones this much (`TUFT_BIG_SHARE`) over the small
// where they meet; of the ground they cover `TUFT_COVER` and `TUFT_TONE_MEAN` is a tuft's mean tone; between them the
// ground lies in the shade of the blades, `GAP_SHADE` of their tone.
const TUFT_FAN_M = 0.45; const TUFT_BIG_M = 2.6; const TUFT_BIG_SHARE = 1.0; const TUFT_COVER = 0.35; const TUFT_TONE_MEAN = 0.75; const GAP_SHADE = 0.95;
// Swathes and blades show as detail this size, m (`shows_at`): none where a pixel takes 1 m or more, where they would
// read only as noise.
const SWATH_DETAIL_M = 1.33;
// Reeds: herbs on soaked ground (from `REED_WET`) and on the shore of standing or running water, out to `REED_BAND_M`
// m from its edge (its height over the water's level over the slope), stand dense (`REED_FILL` of the ground), their colour times `REED_TINT`, in tufts like the rest of the sward.
const REED_WET = 0.85; const REED_BAND_M = 5.0; const REED_FILL = 0.95; const REED_TINT = vec3f(0.75, 0.8, 0.45);
// How much reeds stand at a place: on soaked ground and in the water's edge band (`REED_WET`, `REED_BAND_M`).
fn reeds_of(b: Beneath) -> f32 {
  return max(ease(REED_WET, 1.0, b.soaked), ease(REED_BAND_M, 0.0, b.over / max(b.slope, 1e-3)));
}
// Flowers: in clusters as large as the noise's cells, `FLOWER_M`, over as much of the sward as is in flower; in a
// cluster the dots, of radius `DOT_R`, lie on two lattices of cells `DOT_GAP` across, each turned its own way off the
// map's axes (`DOT_TURN`, radians) and holding a dot at a place of its own in half of `DOT_PRESENT` of its cells, so
// the dots stand in no rows and no square grid; a dot is lighter at its heart by `DOT_HEART`; where a dot takes less than a pixel, the cluster's mean; they show as detail
// `FLOWERS_GONE_M` across (`shows_at`): none where a pixel takes a metre or more (their faint tint gone from afar).
const FLOWERS_GONE_M = 1.33; const FLOWER_M = 1.2; const DOT_R = 0.1; const DOT_GAP = 0.38; const DOT_PRESENT = 0.8; const DOT_HEART = 0.25;
const DOT_TURN = vec2f(0.55, 2.1);

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
  let b = bspline(at / land.map.x - 0.5, vec2f(textureDimensions(cover_field_tex).xy));
  let top = b.g0.x * field_tap(b.h0, LAND_LAYER).x + b.g1.x * field_tap(vec2f(b.h1.x, b.h0.y), LAND_LAYER).x;
  let bottom = b.g0.x * field_tap(vec2f(b.h0.x, b.h1.y), LAND_LAYER).x + b.g1.x * field_tap(b.h1, LAND_LAYER).x;
  land_smooth = b.g0.y * top + b.g1.y * bottom;
}

// The field of lattice l at `at`, m from the map's north-west corner, at mip level lod: the colour its marks cover the
// ground with (a reflectance, premultiplied) and the share they cover; at level 0 blended over the squares with land
// (`on_land`, `land_plain`; a level below holds the most of the texels above it, which no square without land lessens).
fn field_at(at: vec2f, l: i32, lod: f32) -> vec4f {
  let uv = at / (land.map.x * land.map.yz);
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  let v = textureSampleLevel(cover_field_tex, cover_field_sampler, uv, l, lod);
  if (lod > 0.0) { return v; }
  return on_land(v, land_plain);
}

// The field of lattice l at uv, blended between its texels.
fn field_tap(uv: vec2f, l: i32) -> vec4f { return textureSampleLevel(cover_field_tex, cover_field_sampler, uv, l, 0.0); }

// The field of lattice l at `at` through its texels' centres as a cubic B-spline (draw.wgsl `bspline`) over the squares
// with land (`on_land`, `land_smooth`): no square shows in it.
fn field_smooth(at: vec2f, l: i32) -> vec4f {
  let b = bspline(at / land.map.x - 0.5, vec2f(textureDimensions(cover_field_tex).xy));
  let top = b.g0.x * field_tap(b.h0, l) + b.g1.x * field_tap(vec2f(b.h1.x, b.h0.y), l);
  let bottom = b.g0.x * field_tap(vec2f(b.h0.x, b.h1.y), l) + b.g1.x * field_tap(b.h1, l);
  return on_land(b.g0.y * top + b.g1.y * bottom, land_smooth);
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
  let whole = field_at(at, l, 0.0);
  let one = field_at(at, first, 0.0);
  let others = field_at(at, rest, 0.0);
  let told = ease(REST_LEAST, 2.0 * REST_LEAST, others.a / max(whole.a, 1e-6));
  if (told <= 0.0) { return mean; }
  let own = one.rgb / max(one.a, 1e-6);
  let other = others.rgb / max(others.a, 1e-6);
  return mix(mean, mix(other, own, stand_share(stand_draw(at, m), clamp(one.a / whole.a, 0.0, 1.0))), shows * told);
}

// The share of the ground at the pixel that lattice l's plants cover, where they cover `s` of it, seen at `e` m a pixel,
// and the tone there: its crowns in the cells of side `side`, which are its cells doubled `level` times, each lit and
// toned. Of two sizes of cells drawn together, these are the wide ones (`wide`) or the fine ones: a wide cell holds a
// crown where `f`, the share of the way from the fine size to the wide one, passes its draw (`WIDE_FROM`), else its four
// fine cells hold theirs. A crown reaches at most half a cell from its cell, so only the pixel's cell and the three
// beside it towards the pixel can cover it. Cell and distance come from the pixel's lattice point (draw.wgsl `lat`), so
// a crown's place is exact at any size of map.
fn crowns(l: i32, s: f32, e: f32, side: f32, level: i32, wide: bool, f: f32) -> vec2f {
  let warp = GRAIN_WARP * vec2f(noise_l(lat_turned(WARP_CELLS * side, vec2f(0.0)), l + 31).x, noise_l(lat_turned(WARP_CELLS * side, vec2f(17.0)), l + 32).x);
  let at = lat_axes(side, warp);
  let toward = vec2i(select(vec2f(-1.0), vec2f(1.0), at.f > vec2f(0.5)));
  let present = min(s / GRAIN_FILL, 1.0);
  let r = min(side * sqrt(s / (present * PI)), 0.5 * side);
  var out = vec2f(0.0, 1.0);
  for (var j = 0; j < 4; j++) {
    let step = vec2i(j & 1, j >> 1u) * toward;
    let c = at.i + step;
    // The draw of the wide cell: the cell itself, or the one that holds this fine cell.
    var holder = c;
    if (!wide) { holder = vec2i(floor(0.5 * vec2f(c))); }
    let draw = lowbias(holder, l + 9 + 5 * (level + select(1, 0, wide)));
    if ((f >= mix(WIDE_FROM, WIDE_TO, f32(draw >> 8u) / 16777216.0)) != wide) { continue; }
    let x = lowbias(c, l + 7 + 5 * level);
    if (f32(x & 1023u) / 1024.0 >= present) { continue; }
    let d = (at.f - vec2f(step) - vec2f(f32((x >> 10u) & 1023u), f32((x >> 20u) & 1023u)) / 1024.0) * side;
    let a = 1.0 - smoothstep(r - 0.5 * e, r + 0.5 * e, length(d));
    if (a > out.x) {
      let lit = 1.0 - GRAIN_LIGHT * dot(d / max(r, 1e-4), normalize(LIGHT.xy));
      out = vec2f(a, lit * (1.0 + GRAIN_TONE * (f32(x >> 30u) / 1.5 - 1.0)));
    }
  }
  return out;
}

// The share of the ground at `at` that lattice l's plants cover, where they cover `share` of it on the mean, seen at
// `e` m a pixel, and their tone: their crowns, as wide as their marks there, the wide ones over the fine ones, where
// those span `CLUMP_PX` pixels; their mean where they span less, or where a crown would be too wide (`CLUMP_MOST_M`).
fn grain(at: vec2f, l: i32, share: f32, e: f32) -> vec2f {
  let side = cover_look.sides[l];
  let mean = vec2f(min(share, 1.0), 1.0);
  let least = CLUMP_PX * e * land.samples.w;
  if (least >= 2.0 * CLUMP_MOST_M) { return mean; }
  let own = field_at(at, WIDE_LAYER, 0.0)[l];
  let across = max(own, side);
  let shows = (1.0 - ease(CLUMP_MOST_M, 2.0 * CLUMP_MOST_M, across)) * ease(0.5 * least, least, own);
  if (shows <= 0.0) { return mean; }
  let k = log2(across / side);
  let level = floor(k);
  let cell = side * exp2(level);
  // Under `WIDE_FROM` no wide cell holds a crown, from `WIDE_TO` every one does: the other size is not read.
  let f = k - level;
  var fine = vec2f(0.0, 1.0);
  var wide = vec2f(0.0, 1.0);
  if (f < WIDE_TO) { fine = crowns(l, mean.x, e, cell, i32(level), false, f); }
  if (f >= WIDE_FROM) { wide = crowns(l, mean.x, e, 2.0 * cell, i32(level) + 1, true, f); }
  let drawn = vec2f(wide.x + fine.x * (1.0 - wide.x), mix(fine.y, wide.y, wide.x));
  // A stand that closes its ground leaves none between its crowns: the canopy fills the gaps of the field's crowns, in
  // their shade (`CLOSES`).
  let a = max(drawn.x, ease(CLOSES.x, CLOSES.y, mean.x));
  return mix(mean, vec2f(a, mix(GAP_TONE, drawn.y, drawn.x / max(a, 1e-4))), shows);
}

// The flowers over the sward at `at`, seen at `e` m a pixel, of the herbs whose share `bloom` is in flower: their
// colour and the share of the ground they cover (the sward's own share times it): clusters of dots (`FLOWER_M`, `DOT_R`).
fn flowers_at(e: f32, bloom: f32, petal: vec3f) -> vec4f {
  // The clusters: where the noise, made even, falls in the share in flower; softened over a pixel.
  let n = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(FLOWER_M, vec2f(0.0)), 71).x);
  let edge = max(e / FLOWER_M, 0.05);
  let cluster = mix(bloom, ease(1.0 - bloom - edge, 1.0 - bloom + edge, n), shows_at(FLOWER_M, e));
  let mean = DOT_PRESENT * PI * DOT_R * DOT_R / (DOT_GAP * DOT_GAP);
  let dots = shows_at(2.0 * DOT_R, e);
  var cover = mean;
  var c = petal;
  if (dots > 0.0 && cluster > 0.0) {
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
    c = petal * (1.0 + DOT_HEART * (1.0 - d) * dots);
  }
  return vec4f(c, cover * cluster);
}

// The herbs' sward at `at`, seen at `e` m a pixel, over `b`, premultiplied: their colour in stands of their kinds, the
// share they cover (thicker and thinner by `thick`, `patchy`, as far as they hold there, `hold`) with its tufts, swathes and blades where each shows, in the shade of `canopy` of the trees' crowns,
// reeds on soaked ground and at the water's edge, and flowers over it.
fn sward(at: vec2f, e: f32, b: Beneath, canopy: f32, thick: f32, hold: f32) -> vec4f {
  let f = field_smooth(at, SWARD_LAYER);
  if (f.a <= 0.0 || hold <= 0.0) { return vec4f(0.0); }
  var c = in_stands(at, e, f, SWARD_LAYER, SWARD_FIRST, SWARD_REST, SWARD_STAND_M);
  var s = min(f.a * thick * hold, 1.0);
  let tufts = shows_at(SWARD_M, e);
  var tone = 1.0;
  if (tufts > 0.0) {
    let n = tufts * noise_l(lat_turned(SWARD_M, vec2f(0.0)), 54).x;
    s = clamp(s * (1.0 + SWARD * n), 0.0, 1.0);
    tone += TUFT_TONE * n;
  }
  let swaths = shows_at(SWATH_DETAIL_M, e);
  if (swaths > 0.0) { tone += SWATH * swaths * noise_l(lat_turned(SWATH_M, vec2f(0.0)), 73).x; }
  var gap = 1.0;
  var tint = vec3f(1.0);
  let fans = shows_at(TUFT_FAN_M * 0.3, e);
  if (fans > 0.0) {
    // Tufts of two sizes (the small on two lattices turned from each other, so no rows show), each a fan of blades in its own hue: the ground shows between them where the cover is thin.
    let a = fan(lat_axes(TUFT_FAN_M, vec2f(0.0)), 110, 0.55, vec2f(0.22, 0.35), 9.0);
    let b = fan(lat_turned(TUFT_FAN_M * 1.35, vec2f(0.3)), 112, 0.55, vec2f(0.22, 0.35), 10.0);
    let small = mix(a, b, step(a.x, b.x));
    let large = fan(lat_turned(TUFT_BIG_M, vec2f(0.6)), 111, 0.5, vec2f(0.2, 0.35), 13.0);
    let own = mix(small, large, step(small.x, large.x * TUFT_BIG_SHARE));
    let m = max(small.x, large.x);
    s = mix(s, 1.0 - clamp((1.0 - s) * (1.0 - m) / (1.0 - TUFT_COVER), 0.0, 1.0), fans);
    gap = mix(1.0, mix(GAP_SHADE, own.y / TUFT_TONE_MEAN, m), fans);
    tint = mix(vec3f(1.0), vec3f(1.0 + 0.22 * (own.z - 0.5), 1.0 + 0.1 * (own.z - 0.5), 1.0 - 0.35 * (own.z - 0.5)), fans * m);
  }
  tone *= gap;
  let reeds = reeds_of(b);
  if (reeds > 0.0) {
    s = mix(s, max(s, REED_FILL), reeds);
    c = mix(c, c * REED_TINT, reeds);
  }
  let seen = s;
  c *= max(tone, 0.0) * tint * (1.0 - SWARD_SHADE * min(canopy, 1.0));
  let near = shows_at(FLOWERS_GONE_M, e);
  if (near > 0.0) {
    let fl = field_at(at, FLOWER_LAYER, 0.0);
    let bloom = clamp(fl.a / f.a, 0.0, 1.0) * (1.0 - reeds);
    if (bloom > 0.0) {
      let dots = flowers_at(e, bloom, fl.rgb / fl.a);
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
  land_weights(at);
  // The plants' roots find the loose ground's depth as the ground's samples know it: their share of the ground fades
  // over the change of that depth across a sample, never over less (`rooted`), so no edge sharper than the samples
  // runs across a sample's square.
  var bs = b;
  bs.loose_px = b.loose_px * max(1.0, land.map.x * land.map.y / land.samples.x / max(e, 1e-6));
  // The patches as far as they show: where they take less than a pixel the field is their mean (and no noise is read).
  let patches = shows_at(PATCH_M, e);
  let thick = select(1.0, 1.0 + (patchy(at) - 1.0) * patches, patches > 0.0);
  var hold = held_at(at, bs);
  hold.w *= mix(THIN_COVER, 1.0, ease(THIN_FROM_M - 0.5 * bs.loose_px, THIN_TO_M + 0.5 * bs.loose_px, b.loose));
  // The trees' field, read once: their crowns' share, and their layer below (plainly blended where their marks show in
  // its place: only the shade under them reads it).
  var trees = vec4f(0.0);
  if (cover_look.sides[STANDS] > 0.0) {
    trees = select(field_smooth(at, STANDS), field_at(at, STANDS, 0.0), cover_look.shown[STANDS] >= 1.0);
  }
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
    // A lattice no square holds, or whose marks show in full, adds nothing: its field is not read.
    for (var l = 0; l < STANDS; l++) {
      if (cover_look.sides[l] > 0.0 && cover_look.shown[l] < 1.0) { dst = lattice_field(at, e, l, field_smooth(at, l), b, thick, hold[l], dst); }
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
  return CoverAt(dst, min(closed * hold[STANDS], 1.0), trees.rgb / max(trees.a, 1e-4));
}

// Lattice l's field `f` at `at`, seen at `e` m a pixel over `b`, as far as its marks do not show, thicker and thinner by
// `thick`, holding `hold` of its plants there, with its grain, over `dst` (premultiplied); the trees' in their stands,
// their canopy seen as closer (`CANOPY_SEEN`) and snowy as the ground under it. A lattice no square holds (its side 0,
// textures.ts) adds nothing.
fn lattice_field(at: vec2f, e: f32, l: i32, f: vec4f, b: Beneath, thick: f32, hold: f32, dst: vec4f) -> vec4f {
  let unshown = 1.0 - cover_look.shown[l];
  if ((unshown <= 0.0 && l != STANDS) || cover_look.sides[l] <= 0.0 || f.a <= 0.0) { return dst; }
  var c = f.rgb / f.a;
  var share = min(f.a * hold, 1.0);
  if (share <= 0.0) { return dst; }
  if (l == STANDS) {
    c = in_stands(at, e, f, STANDS, STANDS_FIRST, STANDS_REST, STAND_M);
    let groups = shows_at(CANOPY_M, e);
    if (groups > 0.0) { c *= 1.0 + CANOPY_TONE * noise_turned(at / CANOPY_M, 79) * groups; }
    c = mix(c, vec3f(1.0), CANOPY_SNOW * b.snow);
    share = 1.0 - pow(1.0 - share, CANOPY_SEEN);
  }
  var g = vec2f(0.0, 1.0);
  if (unshown > 0.0) { g = grain(at, l, share * thick, e); }
  // Under their marks too a stand of trees that closes its ground leaves none between its crowns: its canopy fills
  // what the marks leave, in their shade (`CLOSES`, `GAP_TONE`), as between the field's own crowns.
  var under = 0.0;
  if (l == STANDS) { under = ease(CLOSES.x, CLOSES.y, min(share * thick, 1.0)) * (1.0 - unshown); }
  let drawn = g.x * unshown;
  let a = drawn + under;
  if (a <= 0.0) { return dst; }
  return vec4f(c * (g.y * drawn + GAP_TONE * under), a) + dst * (1.0 - a);
}
