// Plants drawn from rendered models (web/textures/models.png; CC0, web/textures/SOURCES.md): broad-leaved trees,
// conifers and bushes seen from the angled view, lit from the north-west, four to a layer of the detail texture (tex.wgsl
// `MODELS_FROM`), each standing on the bottom middle of its quarter. A mark of a kind draws one of its form's sprites
// (its own draw picks which), as wide as its crown and no taller than it stands; its leaves take its kind's colour in the
// season (the sprite's lightness about the leaves' mean, `LEAF_MEAN`), its wood keeps the model's bark, and an open
// crown keeps that share of its leaves.

// A sprite's crown is this share of its width (its leaves reach about that far): it is drawn so that share is the
// plant's crown.
const MODEL_CROWN = 0.85;
// The mean lightness of the models' leaves (linear), so a leaf of that lightness takes its kind's colour as it is.
const LEAF_MEAN = 0.05;
// A texel is leaf where its green stands this far over its red and blue (`LEAF_GREEN`: share of its sum).
const LEAF_GREEN = vec2f(0.345, 0.39);
// An open crown loses its leaves in sprays, this many across its sprite.
const SPRAYS = 30.0;

// The sprite plant q is drawn from and its width and height on the map, m.
struct ModelFit { s: i32, w: f32, h: f32 }
fn model_fit(q: Plant) -> ModelFit {
  var first = ROUND_FROM; var count = ROUNDS;
  if (q.design != TREE) { first = SHRUB_FROM; count = SHRUBS; } else if (i32(q.form + 0.5) == CONE || i32(q.form + 0.5) == COLUMN) { first = FIR_FROM; count = FIRS; }
  let s = first + min(i32(fract(q.r.y * 7.13) * f32(count)), count - 1);
  let aspect = model_h(s) / model_w(s);
  // As wide as its crown, no taller than it stands (angled, `STAND` of its height), within its lattice's reach with its
  // lean (the pixel looks no farther for it: cover.wgsl `lattice`).
  let stands = select(BUSH_DEEP * q.d, STAND * q.h, q.design == TREE);
  let w = min(min(q.d / MODEL_CROWN, stands / aspect), q.reach / (0.5 + abs(q.lean.x) * aspect));
  return ModelFit(s, w, w * aspect);
}

// Plant q drawn from a model sprite at p, angled, e m a pixel, where snow covers `snow` of the ground: its colour
// (linear) and cover; none where it does not reach.
fn model_plant(q: Plant, p: vec2f, e: f32, snow: f32) -> vec4f {
  let fit = model_fit(q);
  let s = fit.s; let w = fit.w; let h = fit.h;
  let lean = q.lean.x * h;
  let uv = vec2f((p.x - q.at.x) / w + 0.5 - lean * (q.at.y - p.y) / (h * w), 1.0 - (q.at.y - p.y) / h);
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  let t = model_texel(s, uv, e / w);
  if (t.a <= 0.0) { return vec4f(0.0); }
  let lum = dot(t.rgb, LUMA);
  let leafy = smoothstep(LEAF_GREEN.x, LEAF_GREEN.y, t.g / max(t.r + t.g + t.b, 1e-4));
  // An open crown keeps that share of its leaves, in sprays (`SPRAYS` across the sprite) that fall whole.
  let kept = smoothstep(-0.12, 0.12, q.closed - (0.5 + 0.5 * noise(uv * SPRAYS + 17.0 * q.r.xz, 150).x));
  var leaf = q.rgb * lum / LEAF_MEAN;
  leaf = mix(leaf, CROWN_SNOW_RGB * lum / LEAF_MEAN, CROWN_SNOW * snow * q.closed * smoothstep(0.6, 1.4, lum / LEAF_MEAN));
  let rgb = mix(t.rgb, leaf, leafy);
  return vec4f(rgb, t.a * mix(1.0, kept, leafy));
}

// The shadow of plant q drawn from a model sprite at p, angled, e m a pixel: its sprite cast on the ground away from the
// sun (south-east), each of its texels at a height on it laid `MODEL_SHADOW_RUN` of that height along the ground, blurred
// (`SHADOW_BLUR` of its sprite at least a pixel); its leaves as closed as its crown (a bare crown casts its branches'),
// a bush's lighter (`BUSH_SHADOW`). It fades out before the edge of the reach a pixel looks for it (cover.wgsl
// `lattice`). How dark it makes the ground, 0..1.
const MODEL_SHADOW_RUN = 0.9; const SHADOW_BLUR = 0.02;
fn model_shadow(q: Plant, p: vec2f, e: f32) -> f32 {
  let fit = model_fit(q);
  let v = p - q.at;
  // Its length along the ground southward, within its reach.
  let long = min(MODEL_SHADOW_RUN * fit.h, 0.9 * q.reach);
  let up = v.y / long;
  if (up < 0.0 || up > 1.0) { return 0.0; }
  let uv = vec2f((v.x - v.y) / fit.w + 0.5, 1.0 - up);
  if (uv.x < 0.0 || uv.x > 1.0) { return 0.0; }
  let t = model_texel(fit.s, uv, max(e / fit.w, SHADOW_BLUR));
  let leafy = smoothstep(LEAF_GREEN.x, LEAF_GREEN.y, t.g / max(t.r + t.g + t.b, 1e-4));
  let kept = t.a * mix(1.0, q.closed, leafy);
  let edge = (1.0 - smoothstep(0.8 * q.reach, q.reach, abs(v.x))) * (1.0 - smoothstep(0.8 * q.reach, q.reach, v.y));
  return select(BUSH_SHADOW, 1.0, q.design == TREE) * SHADOW * kept * edge;
}
