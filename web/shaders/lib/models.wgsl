// Plants drawn from rendered models (web/textures/trees.png, models.png; CC0 and CC BY 4.0, web/textures/SOURCES.md):
// broad-leaved trees, conifers and bushes as the map sees them, lit from the north-west, four to a layer of the detail
// texture (tex.wgsl `MODELS_FROM`), each standing on the bottom middle of its quarter. A mark of a kind draws one of
// its form's sprites (its own draw picks which), covering as much as its crown and no taller than it stands; its leaves
// take its kind's colour in the season (the sprite's lightness about the leaves' mean, `LEAF_MEAN`), its wood keeps the
// model's bark; a thin crown is a smaller sprite (covering as much as its leaves close), and only the season takes
// leaves off it: its sprays fall and show its bare twin (sprites.wgsl `bare_of`: the same tree rendered without its
// leaves) under them.

// A sprite is drawn so its cover (tex.wgsl `model_fill` of its box) is its crown's ground, pi/4 of its width squared, as
// the field far away counts it (canopy.ts `marksOn`): its leaves have gaps, so it is wider than the crown, at most
// `MODEL_WIDER` times (canopy.ts `SPRITE_WIDER`).
const MODEL_WIDER = 1.6;
// The mean lightness of the models' leaves (linear), so a leaf of that lightness takes its kind's colour as it is.
const LEAF_MEAN = 0.05;
// A texel is leaf, and takes its kind's colour, where its green stands this far over its red and blue (`LEAF_GREEN`:
// share of its sum).
const LEAF_GREEN = vec2f(0.345, 0.39);
// An open crown loses its leaves in sprays, this many across its sprite.
const SPRAYS = 30.0;
// Snow lies on a crown's lit leaves up to this share of their colour, as closed as the crown is (a bare one holds
// little), in the colour of fresh snow (colour.wgsl `SNOW`): on none lit under `SNOW_LIT`.x of its sprite's leaves' mean,
// on all from .y.
const CROWN_SNOW = 0.8; const CROWN_SNOW_RGB = vec3f(0.90, 0.92, 0.95); const SNOW_LIT = vec2f(0.5, 1.2);
// A tree's shadow darkens the ground by `CROWN_SHADOW` (draw.wgsl); a bush's by this share of that.
const BUSH_SHADOW = 0.3;

// The sprites a plant of q's kind is drawn from, by its numbers (the first and how many). Its family comes from its
// kind's genes alone, each plant crossing a border between families by its own draw (`set_draw`, `under_edge`), so a
// kind whose genes drift changes plant by plant; its height picks only the stage of its family's sprites, as a young
// tree of a family looks. A tree by its crown's form (form.ts `formOf`, as surely as it leads: the rest round): a round
// crown a broad-leaved tree (island trees, oaks, horse chestnuts); a cone a spruce, hemlock or fir (young under
// `CONE_YOUNG_M` m); a column a poplar; tiers a pine (young under `PINE_YOUNG_M` m); an umbrella a flat-topped tree or a
// baobab, or, as dry as its kind is made for, a quiver or Joshua tree; lobes a willow, a black poplar or a fig. A bush by
// how dry its kind's ground (`BUSH_DRY`, over `BUSH_MIX`): leafy, middling, or a dry land's. A tree whose grown plants'
// walls are under `SOFT_WOOD` of their dry mass (style.ts `BUSH_WOOD`, `grownWalls`, over `SOFT_MIX`) is a soft one: of
// a kind as tender to frost as it is (form.ts `Form`, each plant by its own draw, `tender_draw`), a palm or a fern, as
// such plants live where it does not freeze hard (of them, the share as wet as its kind's ground is made for are ferns,
// each by its own draw: tree ferns stand on wet shaded ground, palms on drier); else drawn by its crown's form, as a
// woody tree. A woody tree whose foot stands in liquid (`Plant.wades`: the law stands its kind there as deep as its
// roots reach), as wet as its kind's ground is made for (each by its own draw), is a swamp tree whatever its crown's
// form: as tender to frost as its kind
// is (`tender_draw`), a mangrove on its prop roots, else a bald cypress on its flared feet (trees that stand in water
// hold themselves in its soft bed by buttresses, knees and prop roots: from memory).
const SOFT_WOOD = 0.6; const SOFT_MIX = 0.1;
const CONE_YOUNG_M = 9.0; const PINE_YOUNG_M = 12.0;
const BUSH_DRY = vec2f(0.33, 0.66); const BUSH_MIX = 0.2;
fn model_set(q: Plant) -> vec2i {
  let u = set_draw(q.r);
  if (q.design != TREE) {
    if (!under_edge(q.dry, BUSH_DRY.y, BUSH_MIX, u)) { return vec2i(OTHONNA_FROM, OTHONNA_N); }
    if (!under_edge(q.dry, BUSH_DRY.x, BUSH_MIX, u)) { return vec2i(SEARSIA_FROM, SEARSIA_N); }
    return vec2i(SHRUB_FROM, SHRUB_N);
  }
  if (under_edge(q.wood, SOFT_WOOD, SOFT_MIX, u) && q.tender > 1.0 - tender_draw(q.r)) {
    return select(vec2i(PALM_FROM, PALM_N), vec2i(FERN_FROM, FERN_N), q.wet > 1.0 - u);
  }
  if (q.wades > 0.0 && q.wet > 1.0 - u) {
    return select(vec2i(CYPRESS_FROM, CYPRESS_N), vec2i(MANGROVE_FROM, MANGROVE_N), q.tender > 1.0 - tender_draw(q.r));
  }
  // Of its trees the share `sure` stand in its form, each by its own draw, the rest round.
  let f = select(ROUND, i32(q.form + 0.5), q.sure > 1.0 - u);
  if (f == CONE) { return select(vec2i(CONIFER_FROM, CONIFER_N), vec2i(SPRUCE_YOUNG_FROM, SPRUCE_YOUNG_N), q.h < CONE_YOUNG_M); }
  if (f == COLUMN) { return vec2i(COLUMN_FROM, COLUMN_N); }
  if (f == TIERED) { return select(vec2i(PINE_FROM, PINE_N), vec2i(PINE_YOUNG_FROM, PINE_YOUNG_N), q.h < PINE_YOUNG_M); }
  // Of a broad crown's trees, the share as dry as its kind is made for are quiver trees, each by its own draw.
  if (f == UMBRELLA) { return select(vec2i(UMBRELLA_FROM, UMBRELLA_N), vec2i(QUIVER_FROM, QUIVER_N), q.dry > 1.0 - u); }
  if (f == LOBED) { return vec2i(LOBED_FROM, LOBED_N); }
  return vec2i(ROUND_FROM, ROUND_N);
}

// The sprite plant q is drawn from, its width and height on the map, m, how many times as dense as the sprite its
// cover is drawn (more than once where it is narrower than covers its crown's ground: `dense_cover`), and how many
// plants its mark draws (`k`, `clump_one`) with their feet how far about its own, m (`spread`).
struct ModelFit { s: i32, w: f32, h: f32, dense: f32, k: f32, spread: f32 }
fn model_fit(q: Plant) -> ModelFit {
  let pick = model_set(q);
  let s = pick.x + min(i32(fract(q.r.y * 7.13) * f32(pick.y)), pick.y - 1);
  let aspect = model_h(s) / model_w(s);
  // Covering its crown's ground (`MODEL_WIDER`), no taller than it stands (`STAND` of its height), within its
  // lattice's reach with its lean (the pixel looks no farther for it: cover.wgsl `lattice`).
  let stands = select(BUSH_DEEP * q.d, STAND * q.h, q.design == TREE);
  // A thin crown (how closed in leaf) is a smaller one, never a half-bare one.
  let covers = q.d * min(sqrt(0.25 * PI * q.closed / (model_fill(s) * aspect)), MODEL_WIDER);
  let held = min(covers, q.reach / (0.5 + abs(q.lean.x) * aspect));
  let w = min(held, stands / aspect);
  // A mark whose crown's ground is wider than one plant of its height covers (a clump of its plants, lattices.ts
  // `marksOf`) draws as many plants as one sprite's ground goes into it, at most `CLUMP_MOST`, so it covers the ground
  // the field far away counts for it (canopy.ts `Paint`); their sprites stand within its crown, `d` across.
  let sprites = (held / w) * (held / w);
  let k = select(1.0, min(round(sprites), CLUMP_MOST), sprites >= CLUMP_FROM);
  return ModelFit(s, w, w * aspect, (covers * covers) / (w * w), k, select(0.0, 0.5 * max(min(q.d, held) - w, 0.0), k > 1.0));
}

// A mark draws its clump (`model_fit`) where its crown's ground is at least this many sprites' ground; held by less, it
// is one sprite drawn dense. At most `CLUMP_MOST` plants: each costs a pixel in its box a look.
const CLUMP_FROM = 2.0; const CLUMP_MOST = 6.0;
// The plants of a clump differ in size by up to this share, by their own draw.
const CLUMP_SIZES = 0.3;

// Plant q drawn from `fit` (`model_fit`) and its sprite's fit: the i-th plant of its clump, from the north, each on
// its own foot at a place of its own draw inside the clump's crown (its draws north to south in as many bands as it
// has plants, so drawn in this order the nearer stand over the farther, as the marks do, cover.wgsl `lattice`),
// of its own size; drawn whole from its foot (`model_plant`, `model_shadow`).
struct ClumpOne { q: Plant, fit: ModelFit }
fn clump_one(q: Plant, fit: ModelFit, i: i32) -> ClumpOne {
  if (fit.k <= 1.0) { return ClumpOne(q, fit); }
  let x = lowbias(vec2i(i, i32(q.r.y * 65536.0)), i32(q.r.x * 65536.0));
  let u = vec3f(f32(x & 1023u), f32((x >> 10u) & 1023u), f32((x >> 20u) & 1023u)) / 1023.0;
  let y = fit.spread * (2.0 * (f32(i) + u.x) / fit.k - 1.0);
  var one = q;
  one.at += vec2f(sqrt(max(fit.spread * fit.spread - y * y, 0.0)) * (2.0 * u.y - 1.0), y);
  let size = 1.0 - CLUMP_SIZES * u.z;
  return ClumpOne(one, ModelFit(fit.s, fit.w * size, fit.h * size, fit.dense / fit.k, 1.0, 0.0));
}

// A texel's cover `a` of sprite s drawn `dense` times as dense: the share of `dense` layers of it that a ray meets, so a
// sprite held narrower than its crown (by its height or its reach, `model_fit`) still covers its crown's ground, as the
// field far away counts it (lattices.ts `marksOf`), and is not squeezed. A tree's then shows where that passes a half,
// over `TREE_EDGE` either way (its mips keep the share of its texels that show, detail.ts `keepShown`), so a crown of
// fine leaves far away keeps its cover and its edge and does not fade into a haze of half-covered texels.
const TREE_EDGE = 0.25;
fn dense_cover(s: i32, a: f32, dense: f32) -> f32 {
  let d = layered_cover(a, dense);
  return select(d, smoothstep(0.5 - TREE_EDGE, 0.5 + TREE_EDGE, d), s < TREES);
}
// The share of `dense` layers of a texel's cover `a` that a ray meets.
fn layered_cover(a: f32, dense: f32) -> f32 { return 1.0 - pow(1.0 - min(a, 1.0), dense); }

// A clump's sprites at a pixel gathered front to back are taken as covering it from this much cover on (the ones behind
// are not read).
const CLUMP_COVERED = 0.95;

// Plant q drawn from `fit` (`model_fit`) at p, as `model_plant`: one sprite, or its clump's (`clump_one`), the nearest
// first, each under those before it, as the marks lie over each other (cover.wgsl `lattice`), to `CLUMP_COVERED`.
fn clump_plant(q: Plant, fit: ModelFit, p: vec2f, e: f32, snow: f32) -> vec4f {
  if (fit.k <= 1.0) { return model_plant(q, fit, p, e, snow); }
  var got = vec4f(0.0);
  for (var i = i32(fit.k) - 1; i >= 0 && got.a <= CLUMP_COVERED; i--) {
    let one = clump_one(q, fit, i);
    let m = model_plant(one.q, one.fit, p, e, snow);
    got += vec4f(m.rgb * m.a, m.a) * (1.0 - got.a);
  }
  return vec4f(got.rgb / max(got.a, 1e-6), got.a);
}

// The bare twin of sprite s (sprites.wgsl `bare_of`) at `uv` of its content, at `g` of it a pixel; none where it has
// none.
fn bare_texel(s: i32, uv: vec2f, g: f32) -> vec4f {
  if (s >= TREES || bare_of(s) < 0) { return vec4f(0.0); }
  return model_texel(bare_of(s), uv, g);
}

// Plant q drawn from its model sprite `fit` (`model_fit`) at p, e m a pixel, where snow covers `snow` of the ground: its colour
// (linear) and cover; none where it does not reach. Its leafy sprite keeps the share of its leaves left this season in
// sprays (`SPRAYS` across the sprite) that fall whole, over its bare twin. Its stem's foot (tex.wgsl `foot_rise`)
// stands on its foot; standing in liquid, on the bed, and each of its texels under the surface by its own height
// (tex.wgsl `model_height_m`) is seen through the liquid (style.wgsl `seen_in_liquid`).
fn model_plant(q: Plant, fit: ModelFit, p: vec2f, e: f32, snow: f32) -> vec4f {
  let s = fit.s; let w = fit.w; let h = fit.h;
  let lean = q.lean.x * h;
  let up = q.at.y - p.y;
  let uv = vec2f((p.x - q.at.x) / w + 0.5 - lean * up / (h * w), 1.0 - up / h - foot_rise(s));
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  let kept = smoothstep(-0.12, 0.12, q.left - (0.5 + 0.5 * noise(uv * SPRAYS + 17.0 * q.r.xz, 150).x));
  var bare = vec4f(0.0);
  if (kept < 1.0) { bare = bare_texel(s, uv, e / w); }
  var t = model_texel(s, uv, e / w);
  let feet = wide_feet(q, s);
  if (feet > 0.0) { t = flared_foot(t, s, uv, e / w, feet); }
  let a = dense_cover(s, t.a, fit.dense) * kept;
  if (a + bare.a <= 0.0) { return vec4f(0.0); }
  let lum = dot(t.rgb, LUMA);
  let leafy = smoothstep(LEAF_GREEN.x, LEAF_GREEN.y, t.g / max(t.r + t.g + t.b, 1e-4));
  var leaf = q.rgb * lum / LEAF_MEAN;
  // Snow on its lit leaves: how lit, against its own leaves' mean (`model_leaf`), so a dark conifer holds it as a light
  // crown does.
  let lit = lum / max(model_leaf(s), 1e-4);
  leaf = mix(leaf, CROWN_SNOW_RGB * lit, CROWN_SNOW * snow * q.closed * q.left * smoothstep(SNOW_LIT.x, SNOW_LIT.y, lit));
  // The leafy sprite's sprays over the bare twin, its part under the liquid at its foot seen through it.
  let under = bare.a * (1.0 - a);
  let rgb = (mix(t.rgb, leaf, leafy) * a + bare.rgb * under) / (a + under);
  if (q.wades <= 0.0) { return vec4f(rgb, a + under); }
  return vec4f(seen_in_liquid(rgb, q.wades, model_height_m(s, uv, e / w, up, h), e / MAP_VIEW.y), a + under);
}

// A thing standing up from its foot (a plant, a dead tree) casts its sprite on the ground away from the sun
// (south-east): each of its texels over its foot, at a height on it, laid `MODEL_SHADOW_RUN` (draw.wgsl) of that height
// south and as far east, and across the light's way as far as it lies across the sprite. A crown and a stem are round,
// so the sun meets them as wide as the map sees them: the shadow is the sprite's whole width across the light, not the
// narrower shadow of a flat card facing south. A pixel looks for a mark no farther than its lattice's reach `side` m east
// and south of the mark's foot (cover.wgsl `lattice`, mark.wgsl `mark_box`): a shadow that would reach farther is drawn
// smaller, all of it by one factor, so it is never cut by a straight line. The cast of a thing `h` m tall and `w` m
// wide, its top leaning `top` m east: how long it lies south, how wide, and how far east its top lies, m.
struct Cast { long: f32, w: f32, top: f32 }
fn cast_of(h: f32, w: f32, top: f32, side: f32) -> Cast {
  let long = MODEL_SHADOW_RUN * h;
  let k = min(1.0, side / max(cast_reach(Cast(long, w, top)).x, 1e-6));
  return Cast(k * long, k * w, k * top);
}

// The light's way runs north-west to south-east: a ground point lies (east - south) times this across it from a cast's
// axis, and half a cast's width across it reaches half this of its width east and south (1 / sqrt 2).
const ACROSS_LIGHT = 0.70711;
// How far cast `c` reaches east and south of its foot, m.
fn cast_reach(c: Cast) -> vec2f {
  let side = c.long + 0.5 * ACROSS_LIGHT * c.w;
  return vec2f(side + max(c.top, 0.0), side);
}

// A stem stands round on the ground: its shadow starts across its whole base, and the base is read `BASE_ROW` of the
// sprite's height over its foot (the lowest rows of a sprite are the near rim of its stem's foot, narrower than the stem).
const BASE_ROW = 0.03;

// Where the ground point v (m from the foot, east and south) lies on the sprite cast `c` lays (uv: across, and down, 1
// at the sprite's bottom; its foot `foot` of its height over that), its base read as the stem just over it (`BASE_ROW`);
// out of 0..1 where the cast does not reach.
fn cast_uv(c: Cast, v: vec2f, foot: f32) -> vec2f {
  let up = v.y / max(c.long, 1e-6);
  if (up < 0.0 || up > 1.0) { return vec2f(-1.0); }
  return vec2f(ACROSS_LIGHT * (v.x - v.y - c.top * up) / c.w + 0.5, min((1.0 - foot) * (1.0 - up), 1.0 - foot - BASE_ROW));
}

// Where a stem meets the ground, the sky's light is shut out on every side but one: the ground under and about its foot
// is darker than its cast shadow (ambient occlusion): `CONTACT_SHADE` dark under the stem's round base, the ground point
// `v` m from its foot, fading out from half the stem's reach `most` m to it.
const CONTACT_SHADE = 0.6;
fn contact_shade(v: vec2f, most: f32) -> f32 { return CONTACT_SHADE * (1.0 - smoothstep(0.5 * most, most, length(v))); }
// A tree's stem reaches at most this share of its height from its foot (a stem is about 1/40 of its height across, and
// its root flare twice that: from memory).
const STEM_REACH = 0.03;

// The shadow of plant q drawn from its model sprite `fit` at p, e m a pixel: its sprite cast on the ground (`cast_of`)
// from where its stem meets what it falls on: the ground at its foot, or the surface of the liquid it stands in, where
// its stem shows out of it (`seen_in_liquid`): the map's view (water.wgsl `MAP_VIEW`) draws it the cosine of the depth
// north of the foot. Its leafy sprite's as many of its sprays as are left, over its bare twin's, a bush's lighter
// (`BUSH_SHADOW`). On land, its stem's contact shade (`contact_shade`) at its foot. How dark it makes the ground, 0..1.
// Its edge is soft, the softer the higher the texel it falls from: the sun is a disc half a degree across, so a texel `z`
// m up blurs its shadow over about 0.036 z under a 15 degree sun, and a crown's edge lets light through between its
// leaves. Blurred over `SHADOW_BLUR` of its sprite at its foot to `SHADOW_SOFT` at its top, at least a pixel, and laid
// as the share of its layers a ray meets (`layered_cover`), not edged as a crown is seen, so it fades out. A low sun's
// ray runs through a crown's whole depth, not down through its top as the map sees it: it meets `SHADOW_LAYERS` times
// as many leaves, so the blurred crown stays dark inside and its soft edge spreads out.
// A big crown is a cloud of leafy sprays, wider than the sprite's outline of it, and lit through from a low sun its
// shadow spreads: it is laid `SHADOW_WIDE` times its sprite's width (canopy.ts `shadowWide`). A small tree's shadow
// keeps its own shape. Each pair: for a small crown, `SMALL_CROWN_M` wide and narrower, and for a big one,
// `BIG_CROWN_M` and wider; between them as `crown_big`.
const SHADOW_BLUR = 0.03;
const SHADOW_SOFT = vec2f(0.08, 0.2); const SHADOW_LAYERS = vec2f(2.0, 3.0); const SHADOW_WIDE = vec2f(1.0, 1.5);
const SMALL_CROWN_M = 3.0; const BIG_CROWN_M = 8.0;
// How big a crown `w` m wide is, 0 (small) to 1 (big).
fn crown_big(w: f32) -> f32 { return smoothstep(SMALL_CROWN_M, BIG_CROWN_M, w); }
// One of the pairs above for a crown `big` big.
fn by_crown(pair: vec2f, big: f32) -> f32 { return mix(pair.x, pair.y, big); }
// The cast of plant q drawn from sprite `fit`.
fn model_cast(q: Plant, fit: ModelFit) -> Cast {
  return cast_of(fit.h, by_crown(SHADOW_WIDE, crown_big(fit.w)) * fit.w, 0.0, q.reach);
}
fn model_shadow(q: Plant, fit: ModelFit, p: vec2f, e: f32) -> f32 {
  let v = p - q.at + vec2f(0.0, MAP_VIEW.y * q.wades);
  let c = model_cast(q, fit);
  let foot = foot_rise(fit.s);
  var dark = 0.0;
  let uv = cast_uv(c, v, foot);
  if (all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0))) {
    // Its step over a pixel along the longer of its two axes (it is squeezed along the sun), so it shows no stripes.
    let big = crown_big(fit.w);
    let g = max(max(e / c.w, e / c.long), mix(SHADOW_BLUR, by_crown(SHADOW_SOFT, big), 1.0 - uv.y / (1.0 - foot)));
    let leafy = layered_cover(model_texel(fit.s, uv, g).a, by_crown(SHADOW_LAYERS, big) * fit.dense) * q.left;
    var kept = leafy;
    if (q.left < 1.0) { kept += bare_texel(fit.s, uv, g).a * (1.0 - leafy); }
    dark = CROWN_SHADOW * kept;
  }
  let most = STEM_REACH * fit.h;
  if (q.wades <= 0.0) {
    dark = max(dark, contact_shade(v, most));
  }
  return select(BUSH_SHADOW, 1.0, q.design == TREE) * dark;
}

// How wide the feet tree q drawn from sprite s stands on, 0..1: a woody tree's as wet as its kind's ground (form.ts
// `Form.wet`), drawn as a flare at the trunk's foot (`flared_foot`); a palm's or a fern's none (soft stems grow no
// buttresses), nor a swamp tree's (its sprite stands on its own feet).
fn wide_feet(q: Plant, s: i32) -> f32 {
  let own = in_set(s, PALM_FROM, PALM_N) || in_set(s, FERN_FROM, FERN_N) || in_set(s, CYPRESS_FROM, CYPRESS_N) || in_set(s, MANGROVE_FROM, MANGROVE_N);
  return select(0.0, q.wet, q.design == TREE && s < TREES && !own);
}

// Whether sprite s is of the set of `n` sprites from `first`.
fn in_set(s: i32, first: i32, n: i32) -> bool { return s >= first && s < first + n; }

// A tree of wet ground stands on a flared foot: buttresses and the tops of its roots spread its trunk at the ground, to
// `FOOT_FLARE` times its width at most (the kind of wettest ground), over the lowest `FOOT_RISE` of its trunk's
// height on the sprite, the more the nearer the ground (as a cube: a buttress swells at its foot); the trunk is
// `FOOT_TRUNK` of the sprite's width across where the flare starts (measured on the sprites), and the flare takes the
// colour of the trunk's foot in the sprite (`FOOT_BARK` where the sprite shows none there), its buttresses `FOOT_FINS`
// fins across, each `FIN_SHADE` darker on one side (trees of swamps and floodplains grow buttresses and knees that
// hold them in soft wet ground: from memory).
const FOOT_FLARE = 3.0; const FOOT_RISE = 0.1; const FOOT_TRUNK = 0.06; const FOOT_FINS = 5.0; const FIN_SHADE = 0.3;
const FOOT_BARK = vec3f(0.09, 0.07, 0.05);

// The texel `t` of sprite s at `uv` (`g` of it a pixel) with the flared foot of a tree whose kind is `wet` wet: the
// flare where the sprite shows nothing.
fn flared_foot(t: vec4f, s: i32, uv: vec2f, g: f32, wet: f32) -> vec4f {
  let up = (1.0 - uv.y) / FOOT_RISE;
  if (up >= 1.0 || t.a >= 1.0) { return t; }
  let low = (1.0 - up) * (1.0 - up) * (1.0 - up);
  let half = 0.5 * FOOT_TRUNK * (1.0 + (FOOT_FLARE - 1.0) * wet * low);
  let x = (uv.x - 0.5) / max(half, 1e-4);
  let inside = 1.0 - smoothstep(1.0 - g / half, 1.0, abs(x));
  if (inside <= 0.0) { return t; }
  let trunk = model_texel(s, vec2f(0.5, 1.0 - 0.5 * FOOT_RISE), g);
  let bark = mix(FOOT_BARK, trunk.rgb, trunk.a);
  let fins = 1.0 - FIN_SHADE * (0.5 + 0.5 * sin(PI * FOOT_FINS * x));
  let flare = vec4f(bark * fins, inside);
  return vec4f(mix(flare.rgb, t.rgb, t.a), t.a + flare.a * (1.0 - t.a));
}
