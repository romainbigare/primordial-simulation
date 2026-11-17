// Plants drawn from rendered models (web/textures/models.png; CC0, web/textures/SOURCES.md): broad-leaved trees,
// conifers and bushes seen from the angled view, lit from the north-west, four to a layer of the detail texture (tex.wgsl
// `MODELS_FROM`), each standing on the bottom middle of its quarter. A mark of a kind draws one of its form's sprites
// (its own draw picks which), covering as much as its crown and no taller than it stands; its leaves take its kind's colour in the
// season (the sprite's lightness about the leaves' mean, `LEAF_MEAN`), its wood keeps the model's bark; a thin crown is a
// smaller sprite (covering as much as its leaves close), and only the season takes leaves off it: its sprays fall and
// show its bare twin (sprites.wgsl `bare_of`: the same tree rendered without its leaves) under them.

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

// The sprites a plant of q's kind is drawn from, by its numbers (the first and how many): a tree by its crown's form
// (form.ts `formOf`, round where it is not sure) and its height, a dry kind's broad crown a quiver tree (from `QUIVER_DRY`
// of dryness); a cone young under `CONE_YOUNG_M` m and small under `SMALL_M` m (spruces, pines), a column a spruce
// (young under `CONE_YOUNG_M` m), tiers a pine (young under `PINE_YOUNG_M` m, small under `SMALL_M` m); a bush by how dry its kind's ground (its draw mixing the sets near their borders,
// `BUSH_DRY`): leafy, middling, or a dry land's.
const QUIVER_DRY = 0.5; const SMALL_M = 3.0; const CONE_YOUNG_M = 9.0; const PINE_YOUNG_M = 12.0;
const BUSH_DRY = vec2f(0.33, 0.66); const BUSH_MIX = 0.2;
fn model_set(q: Plant) -> vec2i {
  if (q.design != TREE) {
    let dry = q.dry + BUSH_MIX * (fract(q.r.x * 31.7) - 0.5);
    if (dry > BUSH_DRY.y) { return vec2i(OTHONNA_FROM, OTHONNA_N); }
    if (dry > BUSH_DRY.x) { return vec2i(SEARSIA_FROM, SEARSIA_N); }
    return vec2i(SHRUB_FROM, SHRUB_N);
  }
  let f = select(ROUND, i32(q.form + 0.5), q.sure > 0.5);
  if (f == CONE) {
    if (q.h < SMALL_M) { return vec2i(PINE_SMALL_FROM, PINE_SMALL_N); }
    if (q.h < CONE_YOUNG_M) { return vec2i(SPRUCE_YOUNG_FROM, SPRUCE_YOUNG_N); }
    return vec2i(FIR_FROM, FIR_N);
  }
  if (f == COLUMN) {
    if (q.h < CONE_YOUNG_M) { return vec2i(SPRUCE_YOUNG_FROM, SPRUCE_YOUNG_N); }
    return vec2i(FIR_FROM, FIR_N);
  }
  if (f == TIERED) {
    if (q.h < SMALL_M) { return vec2i(PINE_SMALL_FROM, PINE_SMALL_N); }
    if (q.h < PINE_YOUNG_M) { return vec2i(PINE_YOUNG_FROM, PINE_YOUNG_N); }
    return vec2i(PINE_FROM, PINE_N);
  }
  if (f == UMBRELLA) { return select(vec2i(JACARANDA_FROM, JACARANDA_N), vec2i(QUIVER_FROM, QUIVER_N), q.dry > QUIVER_DRY); }
  if (f == LOBED) { return vec2i(JACARANDA_FROM, JACARANDA_N); }
  return vec2i(ROUND_FROM, ROUND_N);
}

// The sprite plant q is drawn from, its width and height on the map, m, and how many times as dense as the sprite its
// cover is drawn (more than once where it is narrower than covers its crown's ground: `dense_cover`).
struct ModelFit { s: i32, w: f32, h: f32, dense: f32 }
fn model_fit(q: Plant) -> ModelFit {
  let pick = model_set(q);
  let s = pick.x + min(i32(fract(q.r.y * 7.13) * f32(pick.y)), pick.y - 1);
  let aspect = model_h(s) / model_w(s);
  // Covering its crown's ground (`MODEL_WIDER`), no taller than it stands (angled, `STAND` of its height), within its
  // lattice's reach with its lean (the pixel looks no farther for it: cover.wgsl `lattice`).
  let stands = select(BUSH_DEEP * q.d, STAND * q.h, q.design == TREE);
  // A thin crown (how closed over the leaves left) is a smaller one, never a half-bare one.
  let crown_closed = q.closed / max(q.left, 1e-3);
  let covers = q.d * min(sqrt(0.25 * PI * crown_closed / (model_fill(s) * aspect)), MODEL_WIDER);
  let w = min(min(covers, stands / aspect), q.reach / (0.5 + abs(q.lean.x) * aspect));
  return ModelFit(s, w, w * aspect, (covers * covers) / (w * w));
}

// A texel's cover `a` drawn `dense` times as dense: the share of `dense` layers of it that a ray meets, so a sprite
// held narrower than its crown (by its height or its reach, `model_fit`) still covers its crown's ground, as the field
// far away counts it (canopy.ts `marksOn`), and is not squeezed.
fn dense_cover(a: f32, dense: f32) -> f32 { return 1.0 - pow(1.0 - min(a, 1.0), dense); }

// The bare twin of sprite s (sprites.wgsl `bare_of`) at `uv` of its content, at `g` of it a pixel; none where it has
// none.
fn bare_texel(s: i32, uv: vec2f, g: f32) -> vec4f {
  if (s >= TREES || bare_of(s) < 0) { return vec4f(0.0); }
  return model_texel(bare_of(s), uv, g);
}

// Plant q drawn from a model sprite at p, angled, e m a pixel, where snow covers `snow` of the ground: its colour
// (linear) and cover; none where it does not reach. Its leafy sprite keeps the share of its leaves left this season in
// sprays (`SPRAYS` across the sprite) that fall whole, over its bare twin.
fn model_plant(q: Plant, p: vec2f, e: f32, snow: f32) -> vec4f {
  let fit = model_fit(q);
  let s = fit.s; let w = fit.w; let h = fit.h;
  let lean = q.lean.x * h;
  let uv = vec2f((p.x - q.at.x) / w + 0.5 - lean * (q.at.y - p.y) / (h * w), 1.0 - (q.at.y - p.y) / h);
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  let kept = smoothstep(-0.12, 0.12, q.left - (0.5 + 0.5 * noise(uv * SPRAYS + 17.0 * q.r.xz, 150).x));
  var bare = vec4f(0.0);
  if (kept < 1.0) { bare = bare_texel(s, uv, e / w); }
  let t = model_texel(s, uv, e / w);
  let a = dense_cover(t.a, fit.dense) * kept;
  if (a + bare.a <= 0.0) { return vec4f(0.0); }
  let lum = dot(t.rgb, LUMA);
  let leafy = smoothstep(LEAF_GREEN.x, LEAF_GREEN.y, t.g / max(t.r + t.g + t.b, 1e-4));
  var leaf = q.rgb * lum / LEAF_MEAN;
  leaf = mix(leaf, CROWN_SNOW_RGB * lum / LEAF_MEAN, CROWN_SNOW * snow * q.closed * smoothstep(0.6, 1.4, lum / LEAF_MEAN));
  // The leafy sprite's sprays over the bare twin.
  let under = bare.a * (1.0 - a);
  return vec4f((mix(t.rgb, leaf, leafy) * a + bare.rgb * under) / (a + under), a + under);
}

// The shadow of plant q drawn from a model sprite at p, angled, e m a pixel: its sprite cast on the ground away from the
// sun (south-east), each of its texels at a height on it laid `MODEL_SHADOW_RUN` of that height along the ground, blurred
// (`SHADOW_BLUR` of its sprite at least a pixel); its leafy sprite's as many of its sprays as are left, over its bare
// twin's, a bush's lighter (`BUSH_SHADOW`). It fades out before the edge of the reach a pixel looks for it (cover.wgsl
// `lattice`). How dark it makes the ground, 0..1.
const MODEL_SHADOW_RUN = 0.9; const SHADOW_BLUR = 0.05;
fn model_shadow(q: Plant, p: vec2f, e: f32) -> f32 {
  let fit = model_fit(q);
  let v = p - q.at;
  // Its length along the ground southward, within its reach.
  let long = min(MODEL_SHADOW_RUN * fit.h, 0.9 * q.reach);
  let up = v.y / long;
  if (up < 0.0 || up > 1.0) { return 0.0; }
  let uv = vec2f((v.x - v.y) / fit.w + 0.5, 1.0 - up);
  if (uv.x < 0.0 || uv.x > 1.0) { return 0.0; }
  // Its step over a pixel along the longer of its two axes (it is squeezed along the sun), so it shows no stripes.
  let g = max(max(e / fit.w, e / long), SHADOW_BLUR);
  let leafy = dense_cover(model_texel(fit.s, uv, g).a, fit.dense) * q.left;
  var kept = leafy;
  if (q.left < 1.0) { kept += bare_texel(fit.s, uv, g).a * (1.0 - leafy); }
  let edge = (1.0 - smoothstep(0.8 * q.reach, q.reach, abs(v.x))) * (1.0 - smoothstep(0.8 * q.reach, q.reach, v.y));
  return select(BUSH_SHADOW, 1.0, q.design == TREE) * SHADOW * kept * edge;
}
