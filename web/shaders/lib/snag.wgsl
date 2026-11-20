// The dead trees fire left standing (the core's dead walls standing, `FireAt.snag`, kg/m²), drawn by the plant cover's
// marks (cover.wgsl `lattice`) on the trees' lattice in the cells where no living plant stands, in the same order as
// the living ones, so a living crown nearer the eye hides a dead trunk behind it. Each is a charred trunk broken off, a
// stand of dead stems or a charred stump (web/textures/snags.png), or a whole tree of the map's own forms with its
// leaves gone (its bare twin, models.png) weathered grey: a fresh burn's are mostly charred, older ones more often grey,
// bare or broken to stumps, and leaning. Each is mirrored, sized, leaned and tinted by its own draw. Colours linear.

// One dead tree holds `SNAG_KG` of walls, from memory (a stem of about 20 cm and 10 m). A square's dead trees stand in
// its empty cells, each with the chance its walls make over the empty share of the cells about it (`snag_chance`), so
// their number is the walls' however full the living stand is (none more than one a cell).
const SNAG_KG = 150.0;
// Each sprite stands on its quarter's bottom middle `SNAG_FOOT` of the quarter over the edge (bake/snags.py).
const SNAG_FOOT = 2.0 / 256.0;
// The kinds: charred trunks, stands of dead stems, charred stumps (snags.png, snag_sprites.wgsl) and bare trees; each
// one's height drawn, m (its tallest part), `SNAG_VARY` either way, no taller than its lattice lets a plant stand
// (`snag_tallest`).
const KIND_TRUNK = 0; const KIND_STEMS = 1; const KIND_STUMP = 2; const KIND_BARE = 3;
var<private> KIND_FROM: array<i32, 3> = array<i32, 3>(TRUNK_FROM, STEMS_FROM, STUMP_FROM);
var<private> KIND_N: array<i32, 3> = array<i32, 3>(TRUNK_N, STEMS_N, STUMP_N);
var<private> KIND_TALL: array<f32, 4> = array<f32, 4>(5.0, 9.0, 1.4, 10.0);
const SNAG_VARY = 0.3;
// The longer since the burn (`old`, its share of the most months the core tells apart), the more dead trees are bare
// grey trees (`BARE_FRESH` of them at once, `BARE_OLD` more when old) and stumps (`STUMP_OLD` when old) and the more
// they lean, by up to `LEAN_MOST` radians, more on the burnt than on the bare; of the charred ones `STEMS_SHARE` are
// stands of stems.
const BARE_FRESH = 0.15; const BARE_OLD = 0.5; const STUMP_OLD = 0.35; const STEMS_SHARE = 0.4; const LEAN_MOST = 0.6;
// Dead wood weathers grey: a bare tree's bark is drawn `GREYED` of the way to its own grey, lit to `GREY_LIGHT` times as
// light. Each dead tree's tone is moved by up to `TINT` either way, warmer or cooler, by its own draw.
const GREYED = 0.65; const GREY_LIGHT = 1.35; const TINT = 0.12;
// The charred sprites' scans read a little blue: burnt bark in the sun is charcoal to grey-brown, so they are drawn
// warmer by `CHAR_TONE`.
const CHAR_TONE = vec3f(1.1, 1.0, 0.85);
// The trees with bare twins (sprites.wgsl `bare_of`): sprites 0 to `BARE_TREES_A - 1` and from `BARE_TREES_A + 2` to
// `BARE_TREES_N + 1` (the quiver trees' second pair has none).
const BARE_TREES_A = 19; const BARE_TREES_N = 27;
// The empty share of a cell's neighbourhood is never taken under `EMPTY_LEAST`, so a full stand's few gaps do not take
// all its dead trees.
const EMPTY_LEAST = 0.1;

fn snag_unit(c: vec2i, k: i32) -> f32 { return f32(lowbias(c, k)) * (1.0 / 4294967296.0); }

// A dead tree: its kind (-1: none) and sprite (of snags.png, or the tree whose bare twin it is), its height, m, its
// lean, radians, whether it is mirrored (-1) or not (1), and its tint.
struct Snag { kind: i32, s: i32, h: f32, lean: f32, mirror: f32, tint: vec3f }

// The dead tree of the cell `id` (fixed in the world) where the burn's age is `old` and a plant may stand at most
// `tallest` m.
fn snag_of(id: vec2i, old: f32, tallest: f32) -> Snag {
  var q: Snag;
  let pick = snag_unit(id, 42);
  let bare = BARE_FRESH + BARE_OLD * old;
  let r = snag_unit(id, 43);
  if (pick < bare) {
    q.kind = KIND_BARE;
    let k = min(i32(r * f32(BARE_TREES_N)), BARE_TREES_N - 1);
    q.s = select(k + 2, k, k < BARE_TREES_A);
  } else {
    q.kind = select(select(KIND_TRUNK, KIND_STEMS, snag_unit(id, 48) < STEMS_SHARE), KIND_STUMP, pick < bare + STUMP_OLD * old);
    q.s = KIND_FROM[q.kind] + min(i32(r * f32(KIND_N[q.kind])), KIND_N[q.kind] - 1);
  }
  q.h = min(KIND_TALL[q.kind] * mix(1.0 - SNAG_VARY, 1.0 + SNAG_VARY, snag_unit(id, 44)), tallest);
  q.lean = LEAN_MOST * old * (2.0 * snag_unit(id, 47) - 1.0) * select(1.0, 0.5, q.kind == KIND_BARE);
  q.mirror = select(1.0, -1.0, snag_unit(id, 49) < 0.5);
  q.tint = 1.0 + TINT * (2.0 * snag_unit(id, 50) - 1.0) * vec3f(1.0, 0.4, -0.6);
  return q;
}

// Snag q's content box: its width and height as shares of its sprite's side.
fn snag_size(q: Snag) -> vec2f {
  if (q.kind == KIND_BARE) { return vec2f(model_w(q.s), model_h(q.s)); }
  return vec2f(SNAG_W[q.s], SNAG_H[q.s]);
}

// Snag q at `uv` in its content (0..1, y down; outside: none), at `g` of its content a pixel: linear, and cover.
fn snag_texel(q: Snag, uv: vec2f, g: f32) -> vec4f {
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  if (q.kind == KIND_BARE) {
    let t = model_texel(bare_of(q.s), uv, g);
    let grey = vec3f(dot(t.rgb, LUMA));
    return vec4f(mix(t.rgb, grey, GREYED) * GREY_LIGHT * q.tint, t.a);
  }
  let size = snag_size(q);
  let k = q.s % 4;
  let quarter = vec2f(f32(k % 2), f32(k / 2));
  let at = (quarter + vec2f(0.5 - 0.5 * size.x, 1.0 - size.y - SNAG_FOOT) + uv * size) * 0.5;
  let t = textureSampleGrad(detail_tex, detail_sampler, at, SNAGS_FROM + q.s / 4, vec2f(0.5 * g * size.x, 0.0), vec2f(0.0, 0.5 * g * size.y));
  return vec4f(unsrgb(t.rgb) * CHAR_TONE * q.tint, t.a);
}

// Snag q seen at `d` m from its foot, `e` m a pixel: its shadow's darkness (as a tree's, models.wgsl `model_shadow`:
// its sprite laid `MODEL_SHADOW_RUN` of its height south and as far east on the ground, `CROWN_SHADOW` dark; no lean
// in it), and itself (linear, cover) standing up from its foot as the map's angled view draws it, turned by its lean.
struct SnagSeen { shadow: f32, rgba: vec4f }
fn snag_seen(q: Snag, d: vec2f, e: f32) -> SnagSeen {
  let size = snag_size(q);
  let side = q.h / size.y;
  let w = size.x * side;
  var out = SnagSeen(0.0, vec4f(0.0));
  let up = d.y / (MODEL_SHADOW_RUN * q.h);
  if (up >= 0.0 && up <= 1.0) { out.shadow = CROWN_SHADOW * snag_texel(q, vec2f(q.mirror * (d.x - d.y) / w + 0.5, 1.0 - up), e / side).a; }
  let r = vec2f(cos(q.lean) * d.x + sin(q.lean) * d.y, -sin(q.lean) * d.x + cos(q.lean) * d.y);
  out.rgba = snag_texel(q, vec2f(0.5 + q.mirror * r.x / w, 1.0 + r.y / (MAP_VIEW.y * q.h)), e / side);
  return out;
}
