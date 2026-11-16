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
  // As wide as its crown, no taller than it stands (angled, `STAND` of its height), within its lattice's reach.
  let stands = select(BUSH_DEEP * q.d, STAND * q.h, q.design == TREE);
  let w = min(min(q.d / MODEL_CROWN, stands / aspect), 2.0 * q.reach);
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
