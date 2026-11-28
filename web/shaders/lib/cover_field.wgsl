// The plant cover as a field (web/src/canopy.ts `CoverField`), drawn in the live map's ground (land.wgsl) where its
// plants take too few pixels to be drawn as marks (cover.wgsl): of each lattice of marks the colour its plants cover
// the ground with and the share they cover, blended between the squares' centres, at their mean, the ground between
// darkened by their shadows; and the herbs as a sward with their flowers, at every scale. So the far and the near pictures come from
// the same numbers. Nothing stands where liquid stands over the roots or under snow.

struct CoverLook {
  // Of each lattice: its cell's side, m (0: no square holds it), and 1 where any of its marks takes enough pixels to show
  // (each by the width of its lattice's marks where it stands, the rest is the field, `lattice_field`); w is not read.
  sides: vec4f, shown: vec4f,
  // How strongly its marks cover the map; y, z and w are not read.
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
// (`FIELD.shade`), which they darken by `CROWN_SHADOW` (draw.wgsl); in its fourth number the share of
// the ground under crowns whose leaves fall this month (canopy.ts `leavesFalling`).
const WIDE_LAYER = 10; const SHADE_LAYER = 11;
// The sward and the low lattices as one layer (canopy.ts `lowOf`), drawn in their place where a pixel takes
// `LOW_FAR_E` m and more (where their detail no longer shows: no swathes, blades, tufts or flowers past a metre a
// pixel), fully from `LOW_FAR_FULL_E`; thicker and thinner over the patches as they are.
const LOW_LAYER = 12; const LOW_FAR_E = 0.75; const LOW_FAR_FULL_E = 1.0;
// Of each square, 1 where its cell holds land or a kind stands in it (canopy.ts `FIELD.land`); a square with neither
// holds 0 in every layer. A blend between the squares' centres is over the squares with cover alone, their weights
// made whole (`on_land`): a square with none is no bare ground beside them. A blend whose cover's squares weigh less
// than `LAND_LEAST` is none (it lies out in the liquid, where no cover is drawn).
const LAND_LAYER = 13; const LAND_LEAST = 1e-3;
// Of the sward and of the lattice of stands, what the kinds but the first make (canopy.ts `FIELD`).
const SWARD_REST = 14; const STANDS_REST = 15;
// Of the plants that stand in liquid (canopy.ts `FIELD.wet`): the share of the cover that is of kinds the core stands
// on the ground under liquid there, the depth they stand in (down to their line), m, and the herbs' depth and height, m.
const WET_LAYER = 16;
const STANDS = 2;
// The patches of a cell's cover (the core's `cover::PATCHES`, laws.md 3): a square's field above is its cell's first
// patch's, made of that patch's kinds; the field has `FIELD_LAYERS` layers a patch. Then two layers of where its
// cell's other patches lie (canopy.ts `PLACES`), three of the dead walls standing on each of its patches, their height
// and the years since they died (canopy.ts `DEAD`, `patch_dead`), and from `PATCH_FIELDS` the field of each other patch, made of its own
// kinds, `FIELD_LAYERS` layers each (canopy.ts `patchLayer`).
const FIELD_LAYERS = 17; const PLACES_LAYER = 17; const DEAD_LAYER = 19; const PATCH_FIELDS = 22;
// A patch is the ground of its cell nearest its point (the core's `inside::patch_ground`): a disc about the point, cut by
// its cell's edges. It is drawn on ground warped by up to `PATCH_WARP` of a square's side along the slope of noise
// `PATCH_LOBES` of a side across, and a share `PATCH_FINE.x` of that along noise `PATCH_FINE.y` times finer, each way
// no farther than `PATCH_WARP_MOST` of a side (under half: the cell a point is drawn in is one of the four about it):
// so a patch's edge and its cell's are winding lines, its size its own (the edge of a burn or of a gap follows the
// ground and the stands about it, not a circle or a square), and on the mean each patch keeps its share of the ground.
const PATCH_WARP = 0.16; const PATCH_LOBES = 0.55; const PATCH_FINE = vec2f(0.35, 4.0); const PATCH_WARP_MOST = 0.4;
// A patch meets the rest of its cell over a margin `PATCH_EDGE_M` either way of its edge, and a cell's ground its
// neighbour's over `CELL_EDGE_M` (ragged.wgsl `meet`, the patches of each from twice that across, noise streams from
// `PATCH_NOISE`): the stands of a gap or a burn thin into those about it over the whole margin, and a cell's numbers
// are its mean over its square, its edge no line on the ground. A mark's foot draws its patch and its cell by the
// share each takes there, with its own draw fixed at its place to `FOOT_DRAW_M`.
const PATCH_EDGE_M = 250.0; const CELL_EDGE_M = 500.0; const PATCH_NOISE = 213; const FOOT_DRAW_M = 0.01;
// Kinds grow in stands of their own: a plant is of its square's first kind (canopy.ts `Paint`) where a smooth draw fixed
// in the world, mixed with `STAND_MIX` of the plant's own draw, falls within the first kind's share; so a kind's stands
// are patches about as large as the noise's cells (`STAND_M` the trees', `SWARD_STAND_M` the herbs') with mixed edges,
// and each kind keeps its share (draw.wgsl `noise_turned`, made even over 0..1).
const STAND_M = 60.0; const SWARD_STAND_M = 5.0; const STAND_MIX = 0.5;
// Far away a canopy's tone varies gently: the stands fade into their mean where a pixel takes from `STAND_CALM_E` to
// twice as many m (by a 500 m scale bar), so only real differences (forest and meadow, leafy and bare, turned and
// green) show there.
const STAND_CALM_E = 1.5;
// Thin ground is poor: the herbs cover from `THIN_COVER` of their share where the loose ground is `THIN_FROM_M` m deep
// or less to all of it from `THIN_TO_M` m, so the ground's own colour shows through toward the bed's.
const THIN_COVER = 0.25; const THIN_FROM_M = 0.05; const THIN_TO_M = 0.5;
// The light the trees leave the kinds below on the mean of a square is counted no less than this, so a square its
// trees close does not heap its low kinds on its last open ground.
const LIT_LEAST = 0.05;
// Snow lies on the trees' crowns as on the ground under them, up to this share of their colour seen from afar (the
// evergreen ones hold it; the bare ones are thin and dark).
const CANOPY_SNOW = 0.35;

// What lies under and over the cover at a point (land.wgsl `land_spots`): how soaked its ground is, 0..1; its height
// over the nearest water's level, m, and its slope (rise over run, east and south); the depth of the liquid standing
// over it, m; the depth of its loose ground with its patches (rock.wgsl `loose_seen`), without them (`loose_at`) and
// that depth's change across a pixel, m; the share snow covers and the snow's depth, m; and how lush it is (`lush_of`).
struct Beneath { soaked: f32, over: f32, slope: vec2f, depth: f32, loose: f32, loose_mean: f32, loose_px: f32, snow: f32, snow_m: f32, lush: f32 }

// The cover's field at a point, premultiplied; the share the trees' crowns close over the ground there; the share of
// the ground there and about it (blended over the squares with land) under crowns whose leaves fall this month; and
// of the plants there in liquid (`WET_LAYER`): the share of the cover of the kinds that stand in it, the depth they
// stand in (down to their line), m, and the herbs' depth and height, m; the share of the ground the herbs cover; and the
// share the low plants (the sward and the low lattices, `LOW_LAYER`) cover at the point on their mean, as thick as they
// stand there.
struct CoverAt { cover: vec4f, canopy: f32, shed: f32, wet: vec4f, herbs: f32, low: f32 }

// A square whose kinds' roots hold everywhere has its mean rooted share this near 1: no point of it limits them.
const ROOTED_ALL = 0.999;

// Of each lattice and the sward (red, green, blue, alpha), the share of its plants that stand at a point over `b`:
// where their roots hold (land.wgsl `rooted`, over the share of their square that holds them, so the square keeps its
// cover) and out of the snow (draw.wgsl `above_snow`). Each layer is read only where it may limit them. Plants thin
// out over `ROOTS_SOFT` of their need across their rule's edge, on the loose ground's depth `ROOTS_REACH` of the way
// from its patches to its mean without them (`rooted_loose`), the marks as the field (mark.wgsl `holds`,
// `foot_loose`): roots reach metres about a plant, across the patches, and plants thin out toward bare ground over the
// thinning soil, the herbs with no outline, a stand's edge in single crowns (from memory).
const ROOTS_SOFT = 0.3; const ROOTS_REACH = 0.2;
// The depth of loose ground plants' roots hold by, m, where it lies `seen` m deep with its patches and `mean` m without
// them (rock.wgsl `loose_seen`, `loose_at`).
fn rooted_loose(seen: f32, mean: f32) -> f32 { return mix(seen, mean, ROOTS_REACH); }
fn held_at(at: vec2f, b: Beneath) -> vec4f {
  let mean = field_at(at, ROOTED_LAYER);
  let roots = any(mean < vec4f(ROOTED_ALL));
  // The snow over the point: its cells' depth where it lies, none where it does not (as the stones, stone.wgsl).
  let buried = b.snow * b.snow_m;
  if (!roots && buried <= 0.0) { return vec4f(1.0); }
  var out = vec4f(1.0);
  if (roots) {
    let need = field_at(at, NEED_LAYER);
    let loose = rooted_loose(b.loose, b.loose_mean);
    for (var c = 0; c < 4; c++) { out[c] = rooted(loose, need[c], max(b.loose_px, ROOTS_SOFT * need[c])) / max(mean[c], 1e-3); }
  }
  if (buried > 0.0) {
    let tall = field_at(at, TALL_LAYER);
    for (var c = 0; c < 4; c++) { out[c] *= above_snow(buried, tall[c]); }
  }
  return out;
}

// The share of the herbs' height that stands out of liquid `depth` m deep (0: on land), of their field's wet layer `wet`
// (`WET_LAYER`: their depth, z, and height, w, m): all on land, the less the deeper, none past their line (draw.wgsl
// `stands_in`). A herb under the liquid shows nothing from above; the core holds a herb's plants in the liquid down to
// its line.
fn herbs_out(depth: f32, wet: vec4f) -> f32 {
  if (depth <= 0.0) { return 1.0; }
  return stands_in(depth, wet.z) * (1.0 - ease(0.0, max(wet.w, 1e-3), depth));
}

// The herbs' sward: their plants thicker and thinner by up to `SWARD` of their mean over tufts of `SWARD_M`; they cover
// the share of the ground their leaves close by the cover's light law (canopy.ts `Paint`, tape/src/stands.rs `plants`),
// so a thin sward lets the ground show through. Its tone varies by up to `TUFT_TONE` over the tufts and by up to `SWATH`
// over swathes of `SWATH_M`, as far as its images do not show them.
const SWARD = 0.45; const SWARD_M = 0.35;
// Under the trees' crowns the sward is in their shade: darker by up to `SWARD_SHADE` as they close over it.
const SWARD_SHADE = 0.35;
const TUFT_TONE = 0.15; const SWATH = 0.18; const SWATH_M = 3.0; const SWARD_STANDS_NEAR = 0.5;
// A sward whose red over its green passes from `STRAW_HUE.x` to `.y` is straw: its grass is drawn dry.
const STRAW_HUE = vec2f(0.85, 1.1);
// A sward that covers up to `TUFTS_THICK.x` of the ground is drawn as tufts alone, from `.y` on as its tile.
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
// times `REED_TINT`, their blades standing up in their tile (detail.wgsl `grass_hexes`).
const REED_WET = 0.85; const REED_BAND_M = 5.0; const REED_FILL = 0.95; const REED_TINT = vec3f(0.75, 0.8, 0.45);
// Reeds are the herbs' own plants: they stand dense as far as the herbs are there to make them, fully where the herbs
// cover `REED_HERBS` of the ground; a trace of herbs makes a trace of reeds, not a belt.
const REED_HERBS = 0.1;
// A set of tufts drawn one by one (detail.wgsl `sward_tufts`): its first sprite and how many (sprites.wgsl), and a
// second set (its first and how many) that the share `other` of its tufts are drawn from, each by its own draw; its
// lattice's cell, m, the share of a tuft's box its blades cover and how many times as tall as wide the box is (measured
// on the sprites), the cells south of its own a tuft reaches up into (its height), its stream of the world's random
// numbers (and the next), and its shadow (how dark, its middle how many widths down the sun, how many times the tuft's
// own size). The sward's tufts in cells `TUFT_CELL_M` m wide. The tall herbs of the wet edge (`wet_edge`) stand near
// as clumps of their own sprites, their box's cover and shape the first sprite's (`sprite_tufts`): reeds and cattails
// in cells `REED_CELL_M` m wide, straw-olive (`REED_STEM` of the way to `REED_STRAW` from the sward's colour), over a
// longer shadow, the cattails as many as the liquid stands deep over their ground, to all of them from `CATTAIL_M`
// (cattails hold the flooded shallows, reeds the margin above them: from memory); rushes and sedges in cells
// `RUSH_CELL_M` m wide on soaked ground off the water's edge; both on `REED_STANDS` of a clump's cells (so their stems
// stand apart), in clumps with a sharp edge over noise `REED_CLUMP_M` across, about `REED_CLUMPS` of the ground they
// hold (tall marsh grasses grow 1.5 to 3 m tall in clonal clumps, rushes and sedges 0.5 to 1.5 m: from memory).
struct Tufts { first: i32, n: i32, other: vec2i, others: f32, cell_m: f32, covers: f32, aspect: f32, south: i32, k: i32, shadow: vec3f }
const TUFT_CELL_M = 0.08; const TUFT_COVERS = 0.14; const TUFT_ASPECT = 0.81; const TUFT_SOUTH = 2; const TUFT_SHADOW = vec3f(0.3, 0.25, 1.0);
const REED_CELL_M = 0.9; const REED_SOUTH = 6; const REED_SHADOW = vec3f(0.4, 0.5, 1.3); const CATTAIL_M = 0.3;
const REED_STRAW = vec3f(0.42, 0.38, 0.17); const REED_STEM = 0.8;
const RUSH_CELL_M = 0.2; const RUSH_SOUTH = 7; const RUSH_SHADOW = vec3f(0.35, 0.4, 1.2);
const REED_CLUMP_M = 2.5; const REED_CLUMPS = 0.4; const REED_CLUMP_EDGE = 0.1; const REED_STANDS = 0.5;
fn grass_tufts() -> Tufts { return Tufts(TUFT_FROM, TUFT_N, vec2i(TUFT_FROM, TUFT_N), 0.0, TUFT_CELL_M, TUFT_COVERS, TUFT_ASPECT, TUFT_SOUTH, 170, TUFT_SHADOW); }
// The tufts of the set of `n` sprites from `first`, the share `others` of them of the set `other`, measured on its first
// sprite (sprites.wgsl).
fn sprite_tufts(first: i32, n: i32, other: vec2i, others: f32, cell_m: f32, south: i32, k: i32, shadow: vec3f) -> Tufts {
  return Tufts(first, n, other, others, cell_m, model_fill(first), model_h(first) / model_w(first), south, k, shadow);
}
// Reeds, and cattails as deep as liquid `depth` m stands over their ground.
fn reed_tufts(depth: f32) -> Tufts {
  return sprite_tufts(REED_FROM, REED_N, vec2i(CATTAIL_FROM, CATTAIL_N), ease(0.0, CATTAIL_M, depth), REED_CELL_M, REED_SOUTH, 172, REED_SHADOW);
}
fn rush_tufts() -> Tufts { return sprite_tufts(RUSH_FROM, RUSH_N, vec2i(RUSH_FROM, RUSH_N), 0.0, RUSH_CELL_M, RUSH_SOUTH, 176, RUSH_SHADOW); }
// Ferns: the herbs on wet ground in the crowns' shade (as soaked as the ground and as closed as the crowns over it,
// where no reeds stand) stand near as ferns (tex.wgsl, sprites.wgsl `FERN_FROM`), as the soil's floor there is one of
// ferns (detail.wgsl `soil_detail`): `FERN_STANDS` of the herbs' cover, in cells `FERN_CELL_M` m wide (ferns of
// shaded wet woods stand 0.5 to 1.5 m tall in loose clumps: from memory), their box's cover and shape the sprite's.
const FERN_STANDS = 0.6; const FERN_CELL_M = 0.8; const FERN_SOUTH = 2; const FERN_SHADOW = vec3f(0.4, 0.35, 1.2);
fn fern_tufts() -> Tufts { return sprite_tufts(FERN_FROM, FERN_N, vec2i(FERN_FROM, FERN_N), 0.0, FERN_CELL_M, FERN_SOUTH, 174, FERN_SHADOW); }
// How much the herbs at a place over `b` under crowns closing `canopy` of it, where reeds stand as `reeds`, are ferns.
fn ferns_of(b: Beneath, canopy: f32, reeds: f32) -> f32 { return min(canopy, 1.0) * b.soaked * (1.0 - reeds); }
// How much a place over `b` holds the tall herbs of the wet edge, on soaked ground and in the water's edge band
// (`REED_WET`, `REED_BAND_M`): x the band's, y the soaked ground's, as far as it is flat (all under the slope
// `REED_FLAT.x`, rise over run, none from `.y`: a soaked slope drains, the rushes hold the seeps and the flats); their
// larger is the place's (`reed_place`).
const REED_FLAT = vec2f(0.05, 0.15);
fn wet_edge(b: Beneath) -> vec2f {
  let slope = length(b.slope);
  return vec2f(ease(REED_BAND_M, 0.0, b.over / max(slope, 1e-3)), ease(REED_WET, 1.0, b.soaked) * ease(REED_FLAT.y, REED_FLAT.x, slope));
}
fn reed_place(b: Beneath) -> f32 { let w = wet_edge(b); return max(w.x, w.y); }
// How much reeds stand at a place that holds them as `place`, where the herbs cover `herbs` of the ground (`REED_HERBS`):
// one rule far and near.
fn reeds_of(place: f32, herbs: f32) -> f32 { return place * ease(0.0, REED_HERBS, herbs); }
// Flowers: in clusters as large as the noise's cells, `FLOWER_M`, over as much of the sward as is in flower; in a
// cluster their heads, of radius `DOT_R`, `DOT_PRESENT` of them to a cell `DOT_GAP` across, cover their mean share of
// the ground where a head takes less than a pixel, and show in the tile of petals where it takes more (detail.wgsl
// `flower_heads`); they show as detail `FLOWERS_GONE_M` across (`shows_at`): none where a pixel takes a metre or more
// (their faint tint gone from afar).
const FLOWERS_GONE_M = 1.33; const FLOWER_M = 1.2; const DOT_R = 0.045; const DOT_GAP = 0.171; const DOT_PRESENT = 0.8;

// A blend `v` of a layer over the squares about a point, of which the squares with land weigh `held`, as a blend over
// those squares alone (`LAND_LAYER`).
fn on_land(v: vec4f, held: f32) -> vec4f { return select(vec4f(0.0), v / held, held > LAND_LEAST); }

// What the squares with land weigh at the point `cover_field` draws, plainly blended (`field_at`) and through the
// squares' centres (`field_smooth`): read once a pixel (`land_weights`), as every layer read there shares them. Where
// no cover field is drawn (the marks, cover.wgsl) they stay 1 and a layer is read as it is.
var<private> land_plain: f32 = 1.0;
var<private> land_smooth: f32 = 1.0;

// Of the point a pixel or a mark's foot reads the field at (`cell_patch_at`, set once with `land_weights`, as every
// layer read there shares them): the square of the cell it lies in (`patch_of`), the patch past the first of that
// cell's cover it lies nearest the inside of (0: none, the rest of its cell) and the share of the point that patch
// takes; the squares of the cells across the nearest of its cell's edges along x and y, and the share of the point
// each takes (`PatchAt`).
var<private> cell_patch: i32 = 0;
var<private> patch_cell: vec2i = vec2i(0);
var<private> patch_share: f32 = 0.0;
var<private> cell_across: vec2i = vec2i(0);
var<private> across_share: vec2f = vec2f(0.0);

// Of the point `at`, m from the map's north-west corner, seen at `e` m a pixel: the square of the cell its warped place
// lies in (`PATCH_WARP`; x -1 off the map), the patch past the first of that cell's cover (tape/src/stands.rs `places`,
// canopy.ts `PLACES`) it lies nearest the inside of (0: none) and the share of the point that patch takes, as it meets
// the rest of its cell over its margin (`PATCH_EDGE_M`, ragged.wgsl `meet`) and as far as the point is its cell's;
// the steps to the cells across the nearer of its cell's edges along x and along y, and the share of the point each
// takes, as the cells meet across their edges over theirs (`CELL_EDGE_M`: so the rest of a cell, and its dead walls,
// meet its neighbours' over a band, not along a line). Where no cell holds a patch (the field has no patch's layers) no noise is
// read: the square the point lies in, none of its neighbours'.
struct PatchAt { cell: vec2i, which: i32, share: f32, step: vec2i, across: vec2f }
fn patch_of(at: vec2f, e: f32) -> PatchAt {
  let side = land.map.x;
  var s = at / side;
  let patches = i32(textureNumLayers(cover_field_tex)) > PATCH_FIELDS;
  if (patches) {
    let lobes = noise(at / (PATCH_LOBES * side), 211).yz + PATCH_FINE.x * noise(PATCH_FINE.y * at / (PATCH_LOBES * side), 212).yz;
    s += clamp(PATCH_WARP * lobes, vec2f(-PATCH_WARP_MOST), vec2f(PATCH_WARP_MOST));
  }
  let q = vec2i(floor(s));
  var out = PatchAt(q, 0, 0.0, vec2i(0), vec2f(0.0));
  if (any(q < vec2i(0)) || any(q >= vec2i(land.map.yz))) { out.cell.x = -1; return out; }
  if (!patches) { return out; }
  // The cells across: how far past the edge between the lower and the higher cell along each axis the point lies, m,
  // and the higher's share.
  let o = s - vec2f(q);
  let up = o >= vec2f(0.5);
  out.step = vec2i(select(-1, 1, up.x), select(-1, 1, up.y));
  let past = (o - select(vec2f(0.0), vec2f(1.0), up)) * side;
  let higher = vec2f(meet(at, e, past.x, CELL_EDGE_M, 1.0, 0.0, 0.0, 2.0 * CELL_EDGE_M, PATCH_NOISE + 1), meet(at, e, past.y, CELL_EDGE_M, 1.0, 0.0, 0.0, 2.0 * CELL_EDGE_M, PATCH_NOISE + 2));
  out.across = select(1.0 - higher, higher, up);
  let a = textureLoad(cover_field_tex, q, PLACES_LAYER, 0);
  if (a.z <= 0.0) { return out; }
  let b = textureLoad(cover_field_tex, q, PLACES_LAYER + 1, 0);
  // How far inside each patch's reach the point lies, m (the second's where it is the deeper inside).
  var which = 1;
  var inside = a.z - length(o - a.xy);
  if (b.y > 0.0 && b.y - length(o - vec2f(a.w, b.x)) > inside) { which = 2; inside = b.y - length(o - vec2f(a.w, b.x)); }
  if (inside * side <= -2.0 * PATCH_EDGE_M) { return out; }
  out.which = which;
  out.share = meet(at, e, inside * side, PATCH_EDGE_M, 1.0, 0.0, 0.0, 2.0 * PATCH_EDGE_M, PATCH_NOISE) * (1.0 - out.across.x) * (1.0 - out.across.y);
  return out;
}

// Set the point's cells and patch (`cell_patch`...) at `at`, seen at `e` m a pixel (`patch_of`; off the map: the
// first patch of no cell).
fn cell_patch_at(at: vec2f, e: f32) {
  let p = patch_of(at, e);
  cell_patch = p.which;
  patch_cell = p.cell;
  patch_share = p.share;
  cell_across = p.step;
  across_share = p.across;
}

// The patch a mark's foot at `at` stands in, and its cell's square (x, y, z; -1 off the map): one look, not a share,
// so its plants are its patch's or the rest's alone, of one cell; each drawn by the share it takes there (`patch_of` at
// no size of a pixel, every scale of the margins' patches shown) against the foot's own draws, so the marks thin over a
// margin as the field does.
fn foot_patch(at: vec2f) -> vec3i {
  let p = patch_of(at, 0.0);
  if (p.cell.x < 0) { return vec3i(-1, 0, 0); }
  let place = vec2i(floor(at / FOOT_DRAW_M));
  let draw = vec3f(f32(lowbias(place, PATCH_NOISE) & 1023u), f32(lowbias(place, PATCH_NOISE + 1) & 1023u), f32(lowbias(place, PATCH_NOISE + 2) & 1023u)) / 1024.0;
  let cell = p.cell + vec2i(select(0, p.step.x, p.across.x > draw.x), select(0, p.step.y, p.across.y > draw.y));
  // (The patch is its own cell's: its share is of the point and that cell's alone.)
  let own = all(cell == p.cell) && p.share / max((1.0 - p.across.x) * (1.0 - p.across.y), 1e-6) > draw.z;
  return vec3i(select(0, p.which, own), cell);
}
// Set `cell_patch`, `patch_cell` and `patch_share` at a mark's foot at `at` (`foot_patch`, which it returns), and no
// cells across.
fn foot_patch_at(at: vec2f) -> vec3i {
  let p = foot_patch(at);
  cell_patch = max(p.x, 0);
  patch_cell = p.yz;
  patch_share = select(0.0, 1.0, p.x > 0);
  across_share = vec2f(0.0);
  return p;
}

// Of patch `p` of the cell of square `q` (`foot_patch`): the dead walls flames left standing on it, kg a m² of its
// land, the height they stood to, m, and the years since they died (tape/src/stands.rs `dead`); none off the map.
fn patch_dead(p: vec3i) -> vec3f {
  if (p.x < 0) { return vec3f(0.0); }
  let q = clamp(p.yz, vec2i(0), vec2i(land.map.yz) - 1);
  return vec3f(textureLoad(cover_field_tex, q, DEAD_LAYER, 0)[p.x], textureLoad(cover_field_tex, q, DEAD_LAYER + 1, 0)[p.x], textureLoad(cover_field_tex, q, DEAD_LAYER + 2, 0)[p.x]);
}

// The dead walls standing at the point the field is read at (`land_weights`), as `patch_dead` gives them: of the rest
// of its cell and of the cells across its edges by the share of the point each takes (`across_share`), and of its
// patch by its share; the heights and years weighed by the walls.
fn dead_here() -> vec3f {
  if (patch_cell.x < 0) { return vec3f(0.0); }
  var walls = vec3f(0.0);
  for (var k = 0; k < 4; k++) {
    let across = vec2<bool>(k % 2 == 1, k / 2 == 1);
    let w = select(1.0 - across_share.x, across_share.x, across.x) * select(1.0 - across_share.y, across_share.y, across.y);
    if (w <= 0.0) { continue; }
    let d = patch_dead(vec3i(0, patch_cell + vec2i(select(0, cell_across.x, across.x), select(0, cell_across.y, across.y))));
    walls += w * d.x * vec3f(1.0, d.y, d.z);
  }
  // The patch takes its share of the point from the rest of its own cell.
  if (patch_share > 0.0) {
    let own = patch_dead(vec3i(0, patch_cell));
    let d = patch_dead(vec3i(cell_patch, patch_cell));
    walls += patch_share * (d.x * vec3f(1.0, d.y, d.z) - own.x * vec3f(1.0, own.y, own.z));
  }
  return vec3f(walls.x, walls.yz / max(walls.x, 1e-9));
}

// Layer l of the field of the patch `cell_patch` of its cell `patch_cell` (one past the first: its own,
// `PATCH_FIELDS`): its square's alone, not blended with its neighbours', whose cells' patches are not its.
fn patch_field(l: i32) -> vec4f {
  return textureLoad(cover_field_tex, patch_cell, PATCH_FIELDS + (cell_patch - 1) * FIELD_LAYERS + l, 0);
}

// Layer l of the field read as `plain` without the patch, with the patch's (`patch_field`) over the share of the
// point it takes.
fn with_patch(plain: vec4f, l: i32) -> vec4f {
  if (patch_share <= 0.0) { return plain; }
  return mix(plain, patch_field(l), patch_share);
}

// Set `land_plain` and `land_smooth` at `at`, m from the map's north-west corner (the patch is found after,
// `cell_patch_at`).
fn land_weights(at: vec2f) {
  let uv = at / (land.map.x * land.map.yz);
  land_plain = select(field_tap(uv, LAND_LAYER).x, 0.0, any(uv < vec2f(0.0)) || any(uv > vec2f(1.0)));
  land_smooth = spline_field(at, LAND_LAYER).x;
}

// The field of layer l at `at` through its texels' centres as a B-spline read texel by texel (draw.wgsl `b_spline`).
fn spline_field(at: vec2f, l: i32) -> vec4f {
  let c = b_spline(at / land.map.x - 0.5);
  let last = vec2i(textureDimensions(cover_field_tex).xy) - 1;
  var sum = vec4f(0.0);
  for (var n = 0; n < B_TAPS; n++) { sum += b_weight(c, n) * textureLoad(cover_field_tex, clamp(b_texel(c, n), vec2i(0), last), l, 0); }
  return sum;
}

// The field of lattice l at `at`, m from the map's north-west corner: the colour its marks cover the ground with (a
// reflectance, premultiplied) and the share they cover, blended over the squares with land (`on_land`, `land_plain`);
// over the share of the point a patch past the first of its cell's cover takes (`patch_share`), that patch's
// (`with_patch`).
fn field_at(at: vec2f, l: i32) -> vec4f {
  let uv = at / (land.map.x * land.map.yz);
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  if (patch_share >= 1.0) { return patch_field(l); }
  return with_patch(on_land(field_tap(uv, l), land_plain), l);
}

// The field of lattice l at uv, blended between its texels.
fn field_tap(uv: vec2f, l: i32) -> vec4f { return textureSampleLevel(cover_field_tex, cover_field_sampler, uv, l, 0.0); }

// The field of lattice l at `at` through its texels' centres as a B-spline read texel by texel (draw.wgsl `b_spline`)
// over the squares with land (`on_land`, `land_smooth`): no square shows in it, and no staircase of the GPU's blend. Over
// the share of the point a patch past the first of its cell's cover takes, that patch's (`with_patch`).
fn field_smooth(at: vec2f, l: i32) -> vec4f {
  if (patch_share >= 1.0) { return patch_field(l); }
  return with_patch(on_land(spline_field(at, l), land_smooth), l);
}

// The stands' draw at `at` over patches of `m` m, even over 0..1.
fn stand_draw(at: vec2f, m: f32) -> f32 { return smoothstep(-EVEN_ONE, EVEN_ONE, noise_turned(at / m, 70)); }

// The stands' draw `u` mixed with a plant's own draw `r` (both even over 0..1), made even again (`even_mix`). A plant
// is of the first kind where this falls under the kind's share.
fn stand_pick(u: f32, r: f32) -> f32 { return even_mix(u, r, STAND_MIX); }
// A draw `u` mixed with a draw `r` (both even over 0..1, `b` of r, at most half), made even again: the distribution
// function of (1 - b) u + b r.
fn even_mix(u: f32, r: f32, b: f32) -> f32 {
  let a = 1.0 - b;
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

// The kinds at `at`, seen at `e` m a pixel, of layer l whose field is `f` (premultiplied), its kinds in stands of
// about `m` m: its mean colour, its first kind's (layer `first`) and the rest's (layer `rest`), the first's share of
// it, and how far its stands show (where a stand takes a pixel or more; the layer is its mean where they do not). The
// first kind's share and the rest's colour are read between the texels as plainly blended, both alike, so the rest's
// colour is its kinds' own; a rest under `REST_LEAST` of the layer is too small to tell its colour from the field's
// rounding, and the layer is its mean there (in full from twice that share).
const REST_LEAST = 0.01;
struct Stands { mean: vec3f, first: vec3f, rest: vec3f, share: f32, shows: f32 }
fn stands_of(at: vec2f, e: f32, f: vec4f, l: i32, first: i32, rest: i32, m: f32) -> Stands {
  let mean = f.rgb / f.a;
  var out = Stands(mean, mean, mean, 1.0, 0.0);
  if (least_e() >= 2.0 * STAND_CALM_E) { return out; }
  let shows = shows_at(m, e) * (1.0 - ease(STAND_CALM_E, 2.0 * STAND_CALM_E, e));
  if (shows <= 0.0) { return out; }
  let whole = field_at(at, l);
  let one = field_at(at, first);
  let others = field_at(at, rest);
  out.shows = shows * ease(REST_LEAST, 2.0 * REST_LEAST, others.a / max(whole.a, 1e-6));
  out.first = one.rgb / max(one.a, 1e-6);
  out.rest = others.rgb / max(others.a, 1e-6);
  out.share = clamp(one.a / whole.a, 0.0, 1.0);
  return out;
}
// The colour of the kinds `k` (`stands_of`) where the first kind's share of the plants is `first`.
fn stands_colour(k: Stands, first: f32) -> vec3f { return mix(k.mean, mix(k.rest, k.first, first), k.shows); }
// The colour at `at` of the kinds of layer l as `stands_of` gives them: its first kind's where its stands are, the
// rest's elsewhere (`stand_share`).
fn in_stands(at: vec2f, e: f32, f: vec4f, l: i32, first: i32, rest: i32, m: f32) -> vec3f {
  let k = stands_of(at, e, f, l, first, rest, m);
  if (k.shows <= 0.0) { return k.mean; }
  return stands_colour(k, stand_share(stand_draw(at, m), k.share));
}

// The flowers over the sward at `at`, seen at `e` m a pixel, of the herbs whose share `bloom` is in flower: their
// colour and the share of the ground they cover (the sward's own share times it): clusters (`FLOWER_M`) of flower heads
// (`DOT_R`).
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
    // Their heads in the tile of petals (detail.wgsl `flower_heads`).
    let h = flower_heads(e);
    cover = mix(mean, h.x, dots);
    c = petal * h.y;
  }
  return vec4f(c, cover * cluster);
}

// The herbs' sward at `at`, seen at `e` m a pixel, over `b`, its field there `f` (`field_smooth`), premultiplied: their
// colour in stands of their kinds, the share they cover (as lush as the ground is, lush.wgsl `lushed`, as far as they
// hold there, `hold`, in clumps `lc` in the gaps of the trees' crowns, which close `closed` of the ground on their mean,
// clumps.wgsl `clumped`) in their drifts (`drift`, lush.wgsl `herb_drift`) with its tufts, swathes and blades where
// each shows, in the shade of `canopy` of the trees' crowns, reeds on soaked ground and at the water's edge, ferns on
// wet shaded ground, and flowers over it.
fn sward(at: vec2f, e: f32, b: Beneath, f: vec4f, canopy: f32, hold: f32, lc: Clumps, closed: f32, drift: vec3f) -> vec4f {
  if (f.a <= 0.0 || hold <= 0.0) { return vec4f(0.0); }
  // Near, its images give its detail (detail.wgsl `grass_detail`): the drawn tufts' and swathes' tone fades out as they
  // show, and its kinds' stands keep `SWARD_STANDS_NEAR` of their difference from the mean, so the layers do not pile
  // into smudges.
  let near_images = shows_at(SWATH_DETAIL_M, e);
  let mean = f.rgb / f.a;
  var c = in_stands(at, e, f, SWARD_LAYER, SWARD_FIRST, SWARD_REST, SWARD_STAND_M);
  let even = lushed(min(f.a * hold, 1.0), b.lush);
  c = mix(c, mean + SWARD_STANDS_NEAR * (c - mean), near_images) * lush_hue(b.lush) * drift * clump_light(lc, even * (1.0 - closed));
  var s = clumped(lc, even, even * (1.0 - closed));
  let reeds = reeds_of(reed_place(b), s);
  let tufts = shows_at(SWARD_M, e);
  var tone = 1.0;
  if (tufts > 0.0) {
    let n = tufts * noise_l(lat_turned(SWARD_M, vec2f(0.0)), 54).x;
    s = clamp(s * (1.0 + SWARD * n), 0.0, 1.0);
    tone += TUFT_TONE * n * (1.0 - near_images);
  }
  let swaths = shows_at(SWATH_DETAIL_M, e);
  if (swaths > 0.0) { tone += SWATH * swaths * (1.0 - near_images) * noise_l(lat_turned(SWATH_M, vec2f(0.0)), 73).x; }
  if (reeds > 0.0) {
    s = mix(s, max(s, REED_FILL), reeds);
    c = mix(c, c * REED_TINT, reeds);
  }
  // Its grass, straw where its colour is straw's.
  let straw = smoothstep(STRAW_HUE.x, STRAW_HUE.y, c.r / max(c.g, 1e-4));
  let plain = c * max(tone, 0.0) * (1.0 - SWARD_SHADE * min(canopy, 1.0));
  c = plain * grass_detail(e, straw, reeds, s);
  // Very near, its blades drawn one by one (detail.wgsl `blades`) over its tiles in their shade (`BLADE_GROUND`), coming
  // in smoothly as a pixel passes from `BLADE_SEEN_E.y` to `.x` m.
  let near_blades = 1.0 - ease(BLADE_SEEN_E.x, BLADE_SEEN_E.y, e);
  if (near_blades > 0.0) {
    let b = blades(e);
    c = mix(c, mix(c * BLADE_GROUND, plain * b.x, b.y), near_blades);
  }
  // Where its tufts show as things, a thin sward is its tufts standing on bare ground (detail.wgsl `sward_tufts`), not a
  // veil: fully where it covers up to `TUFTS_THICK.x` of the ground, its tile of grass from `.y` on. In liquid it is
  // always its tufts, the liquid between them: herbs standing in water are no lawn over it.
  var seen = s;
  let thin = select(1.0 - smoothstep(TUFTS_THICK.x, TUFTS_THICK.y, s), 1.0, b.depth > 0.0);
  if (thin > 0.0) {
    let t = sward_tufts(plain, c, e, s, grass_tufts(), b.depth);
    c = mix(c, t.rgb, thin);
    seen = mix(s, t.a, thin);
  }
  // Near, its tall herbs of the wet edge stand in clumps over it as tall tufts: reeds and cattails in the edge's band
  // (`reed_tufts`), rushes on the soaked ground the band does not reach (`rush_tufts`); far, they are its tint alone.
  if (reeds > 0.0) {
    let clumps = REED_STANDS * reeds * ease(1.0 - 2.0 * REED_CLUMPS - REED_CLUMP_EDGE, 1.0 - 2.0 * REED_CLUMPS + REED_CLUMP_EDGE, noise_l(lat_turned(REED_CLUMP_M, vec2f(0.3)), 86).x);
    let w = wet_edge(b);
    let rushes = clamp(1.0 - w.x / max(w.y, 1e-6), 0.0, 1.0);
    if (rushes < 1.0) {
      let r = sward_tufts(mix(plain, REED_STRAW, REED_STEM), c, e, clumps * (1.0 - rushes), reed_tufts(b.depth), b.depth);
      c = mix(c, r.rgb, r.a);
    }
    if (rushes > 0.0) {
      let r = sward_tufts(plain, c, e, clumps * rushes, rush_tufts(), b.depth);
      c = mix(c, r.rgb, r.a);
    }
  }
  // Near, on wet shaded ground its herbs stand as ferns over it (`ferns_of`).
  let ferns = ferns_of(b, canopy, reeds);
  if (ferns > 0.0) {
    let fern = sward_tufts(plain, c, e, FERN_STANDS * ferns * s, fern_tufts(), b.depth);
    c = mix(c, fern.rgb, fern.a);
  }
  let near = shows_at(FLOWERS_GONE_M, e);
  if (near > 0.0) {
    let fl = field_at(at, FLOWER_LAYER);
    let bloom = clamp(fl.a / f.a, 0.0, 1.0) * (1.0 - reeds - ferns);
    if (bloom > 0.0) {
      let dots = flowers_at(e, bloom, fl.rgb / fl.a);
      c = mix(c, dots.rgb, dots.a * near);
    }
  }
  return vec4f(c * seen, seen);
}

// The cover's field at `at` seen at `e` m a pixel over `b`, premultiplied, and the trees' crowns' share: the herbs'
// sward, then each lattice (`lattice_field`), the lower under the taller, as the ground is lush (lush.wgsl
// `lushed`), in clumps (clumps.wgsl), as far as it holds there (`held_at`); far away, where the sward's and the low
// lattices' detail no longer shows, those three as their one layer (canopy.ts `FIELD.low`).
fn cover_field(at: vec2f, e: f32, b: Beneath) -> CoverAt {
  land_weights(at);
  // Under liquid where no square about the point holds cover, none stands: every layer reads none (`on_land`; the
  // squares of the cells about it hold none in any layer, its patch's too, as the patch's cell is one of the four
  // about it), and under liquid no burn reads its patch (burn.wgsl). Neither the patch nor a layer is read.
  if (b.depth > 0.0 && land_plain <= LAND_LEAST && land_smooth <= LAND_LEAST) { return CoverAt(vec4f(0.0), 0.0, 0.0, vec4f(0.0), 0.0, 0.0); }
  cell_patch_at(at, e);
  // The clumps the trees stand in, and the low plants in their gaps (clumps.wgsl).
  let tc = clumps_at(at, e);
  let lc = low_rank(tc);
  // The plants' roots hold where the marks' do (mark.wgsl `holds`, on every patch of the loose ground), over a pixel and
  // over the patches too small to show here (`loose_unseen`), so the field far away and the marks near stand on the
  // same ground.
  var bs = b;
  bs.loose_px = max(b.loose_px, loose_unseen(b.loose_mean, e));
  // The marks' mean widths, read once.
  let wide = field_at(at, WIDE_LAYER);
  var hold = held_at(at, bs);
  hold.w *= mix(THIN_COVER, 1.0, ease(THIN_FROM_M - 0.5 * bs.loose_px, THIN_TO_M + 0.5 * bs.loose_px, rooted_loose(b.loose, b.loose_mean)));
  // The trees' field, read once: their crowns' share on the mean, and at the point in their clumps.
  var trees = vec4f(0.0);
  if (cover_look.sides[STANDS] > 0.0) { trees = field_smooth(at, STANDS); }
  let crowns = lushed(min(trees.a * hold[STANDS], 1.0), b.lush);
  let crowns_here = clumped(crowned(tc, wide[STANDS]), crowns, crowns);
  // The low kinds take the light the trees leave (the cover's light law, laws.md 3: a kind below gets the light of
  // the ground the taller does not hold and what passes its layer): they thin under the crowns that stand at the point
  // and stand as much thicker on the ground that holds no tree, their square keeping its cover. So ground too thin for
  // the trees shows the sward and the bushes of its cell before it shows bare.
  let closed = min(trees.a, 1.0);
  let lit = (1.0 - min(closed * hold[STANDS], 1.0)) / max(1.0 - closed, LIT_LEAST);
  // The bushes thin in the shade but do not heap in the gaps: only the sward, which fills ground, thickens there. In
  // liquid the sward shows as far as its herbs stand out of it (`herbs_out`).
  let wet = field_at(at, WET_LAYER);
  hold = vec4f(hold.xy * min(lit, 1.0), hold.z, hold.w * lit * herbs_out(b.depth, wet));
  // Under the trees: the sward and the low lattices one by one near, as one layer far (`LOW_FAR_E`).
  let far = ease(LOW_FAR_E, LOW_FAR_FULL_E, e);
  let drift = herb_drift(at, e, b.slope);
  var dst = vec4f(0.0);
  // The sward's field, read once where the near or the far picture asks for it.
  var swarded = vec4f(0.0);
  var sward_read = false;
  if (least_e() < LOW_FAR_FULL_E && far < 1.0) {
    swarded = field_smooth(at, SWARD_LAYER);
    sward_read = true;
    dst = sward(at, e, b, swarded, crowns_here, hold.w, lc, closed, drift);
    // A lattice no square holds adds nothing: its field is not read.
    for (var l = 0; l < rolled(STANDS); l++) {
      if (cover_look.sides[l] > 0.0) { dst = lattice_field(at, e, l, field_smooth(at, l), wide, b, hold[l], dst, lc, closed, crowns_here); }
    }
  }
  if (far > 0.0) {
    // As they stand at the point (`hold`, `lushed`), in the gaps of the trees' clumps.
    // In the shade of the trees' crowns as the sward is near (`SWARD_SHADE`), so the far and the near tone agree.
    let low = field_smooth(at, LOW_LAYER);
    let mean = lushed(min(low.a * hold.w, 1.0), b.lush);
    var seen = low * (clumped(lc, mean, mean * (1.0 - closed)) / max(low.a, 1e-6));
    // Reeds on soaked ground and at the water's edge stand dense and in their own tint, as near (`sward`), as far as the
    // herbs stand there.
    let place = reed_place(b);
    var reeds = 0.0;
    if (place > 0.0) {
      if (!sward_read) { swarded = field_smooth(at, SWARD_LAYER); }
      reeds = reeds_of(place, lushed(min(swarded.a * hold.w, 1.0), b.lush));
    }
    let filled = mix(seen.a, max(seen.a, REED_FILL), reeds);
    seen = vec4f(seen.rgb / max(seen.a, 1e-6) * mix(vec3f(1.0), REED_TINT, reeds) * lush_hue(b.lush) * drift * clump_light(lc, mean * (1.0 - closed)) * filled, filled);
    dst = mix(dst, vec4f(seen.rgb * (1.0 - SWARD_SHADE * crowns_here), seen.a), far);
  }
  dst = lattice_field(at, e, STANDS, trees, wide, b, hold[STANDS], dst, tc, 0.0, 0.0);
  // The crowns that stand over the point shade its floor (crag.wgsl `floor_of`).
  return CoverAt(dst, crowns_here, field_at(at, SHADE_LAYER).w, wet, min(field_at(at, SWARD_LAYER).a, 1.0), lushed(min(field_at(at, LOW_LAYER).a * hold.w, 1.0), b.lush));
}

// Lattice l's field `f` at `at`, seen at `e` m a pixel over `b`, its marks' mean widths there `wide` (`WIDE_LAYER`),
// as far as its marks do not show (draw.wgsl `as_crown` by the width of its marks there, as each mark shows,
// mark.wgsl `decide`), as lush as the ground is (lush.wgsl `lushed`), holding `hold` of its plants there, in clumps `c`
// in the gaps of the trees, which close `closed` of the ground on their mean (0 for the trees: clumps.wgsl `clumped`),
// over `dst` (premultiplied): its shadows darken what lies below, its crowns cover their share; the trees' in their
// stands, snowy as the ground under them, their clumps' edges made of crowns (clumps.wgsl `crowned`), their canopy
// grained by their groups (crowns.wgsl `canopy_grain`), each crown a lit dome over its shadow (crowns.wgsl `specks`, as
// wide as the marks there, each of its stand's first kind or the rest by its own draw, `stand_share`) from where it
// takes a pixel or more; the low plants under `canopy` of the trees' crowns in their shade, as the sward
// (`SWARD_SHADE`). A lattice no square holds (its side 0, textures.ts) adds nothing. Its shadows fall on the ground
// among its plants: where the clumps show, none in a gap. A far crown casts its shadow `DOME_SHADOW` of its width
// south-east (the marks' shadows reach about a crown's width: models.wgsl `cast_of`). Where the trees' marks show, the
// floor of their stand lies in the gloom of its canopy beside their own shadows, darker by `FLOOR_GLOOM` of the share
// its crowns close (a closed canopy lets little light through its gaps to the floor: its shadows cross and the sky is
// mostly hidden; seen from a plane the gaps in a closed wood are dark: from memory).
const DOME_SHADOW = 0.6; const FLOOR_GLOOM = 0.6;
// The trees' canopy seen from above is a rough top of crowns in groups (clumps.wgsl `clump_light`), its clumps' rims
// lit by up to `CANOPY_RIM` within `CANOPY_RIM_CROWNS` crowns' widths of their edge as they face the sun or turn from
// it; and the ground beyond a clump's edge away from the sun lies in its cast shade, as long as the marks'
// (`MODEL_SHADOW_RUN` times the trees' height) and as dark as a crown's shadow (`CROWN_SHADOW`), its edge eased over
// half a crown (it is the edge crowns' shadows) or a pixel.
const CANOPY_RIM = 0.2; const CANOPY_RIM_CROWNS = 1.5;
fn lattice_field(at: vec2f, e: f32, l: i32, f: vec4f, wide: vec4f, b: Beneath, hold: f32, dst: vec4f, clumps: Clumps, closed: f32, canopy: f32) -> vec4f {
  if (cover_look.sides[l] <= 0.0 || f.a <= 0.0) { return dst; }
  let w = wide[l];
  var unshown = 1.0;
  if (cover_look.shown[l] > 0.0) { unshown = 1.0 - as_crown(w, e); }
  let plain = min(f.a * hold, 1.0);
  let mean = lushed(plain, b.lush);
  if (mean <= 0.0 || (unshown <= 0.0 && l != STANDS)) { return dst; }
  let near = mean * (1.0 - closed);
  var c = clumps;
  if (l == STANDS) { c = crowned(clumps, w); }
  let crowns = clumped(c, mean, near);
  var dark = CROWN_SHADOW * min(field_at(at, SHADE_LAYER)[l] * hold * lush_thick(plain, b.lush), 1.0) * mix(1.0, min(crowns / mean, 1.0), c.shows);
  let gloom = select(0.0, FLOOR_GLOOM * min(crowns, 1.0), l == STANDS);
  if (unshown <= 0.0) { return darkened(dst, gloom); }
  var col = f.rgb / f.a * clump_light(c, near) * (1.0 - SWARD_SHADE * min(canopy, 1.0));
  var share = crowns;
  if (l == STANDS) {
    let kinds = stands_of(at, e, f, STANDS, STANDS_FIRST, STANDS_REST, STAND_M);
    var u = 0.5;
    var first = kinds.share;
    if (kinds.shows > 0.0) { u = stand_draw(at, STAND_M); first = stand_share(u, kinds.share); }
    var tone = canopy_grain(at, w, e);
    if (c.shows > 0.0) {
      let edge = clump_edge(c, near);
      let rim = edge.y * (1.0 - ease(0.0, CANOPY_RIM_CROWNS * w, edge.x));
      tone *= 1.0 + CANOPY_RIM * c.shows * rim;
      let soft = max(e, 0.5 * w);
      let shaded = smoothstep(-soft, soft, edge.x - MODEL_SHADOW_RUN * field_at(at, TALL_LAYER)[l] * edge.y);
      dark = max(dark, CROWN_SHADOW * c.shows * shaded);
    }
    if (crowns > 0.0 && SPECK_PX.x * least_e() < SPECK_MOST_M && SPECK_PX.x * e < SPECK_MOST_M) {
      let dome = specks(at, w, crowns, e, 83);
      // The shadows: the crowns' own specks laid south-east, as dark on the mean as the field's shade.
      let laid = specks(at - DOME_SHADOW * w * vec2f(1.0), w, crowns, e, 83).on;
      dark *= laid / max(crowns, 1e-3);
      share = dome.on;
      tone *= dome.tone;
      first = mix(first, select(0.0, 1.0, stand_pick(u, dome.own) < kinds.share), dome.look);
    }
    col = stands_colour(kinds, first) * clump_light(c, near) * tone;
    col = mix(col, vec3f(1.0), CANOPY_SNOW * b.snow);
  }
  share *= unshown;
  dark = mix(gloom, min(dark, 1.0), unshown);
  return vec4f(col * share, share) + darkened(dst, dark) * (1.0 - share);
}
// The cover `dst` (premultiplied) under a shadow `dark` deep: what lies below darkened, the shadow's own cover added.
fn darkened(dst: vec4f, dark: f32) -> vec4f { return vec4f(dst.rgb * (1.0 - dark), dark + dst.a * (1.0 - dark)); }
