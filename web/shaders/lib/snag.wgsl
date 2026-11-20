// The dead trees fire left standing (the core's dead walls standing, `FireAt.snag`, kg/m²): marks of the plant cover's
// trees' lattice decided where no living plant stands, by the living marks' rules of the ground (mark.wgsl
// `dead_decided`), and drawn with them in one order (cover.wgsl `lattice`), so a living crown nearer the eye hides a
// dead trunk behind it. Each is a charred trunk broken off or
// a charred stump (web/textures/snags.png), or a whole tree of the map's own forms with its
// leaves gone (its bare twin, models.png) weathered grey: a fresh burn's are mostly charred, older ones more often grey,
// bare or broken to stumps, and leaning. Each is mirrored, sized, leaned and tinted by its own draw. Colours linear.

// One dead tree holds `SNAG_KG` of walls, from memory (a stem of about 20 cm and 10 m). A square's dead trees stand in
// its empty cells, each with the chance its walls make over the share of cells its living kinds leave empty, so their
// number is the walls' however full the living stand is (none more than one a cell).
const SNAG_KG = 150.0;
// The kinds: charred trunks, charred stumps (snags.png, snag_sprites.wgsl: each stands on its foot, the row where its
// trunk meets the ground) and bare trees; each one's height drawn, m (its tallest part), `SNAG_VARY` either way, no
// taller than its lattice lets a plant stand (cover.wgsl `dead_tree`).
const KIND_TRUNK = 0; const KIND_STUMP = 1; const KIND_BARE = 2;
var<private> KIND_FROM: array<i32, 2> = array<i32, 2>(TRUNK_FROM, STUMP_FROM);
var<private> KIND_N: array<i32, 2> = array<i32, 2>(TRUNK_N, STUMP_N);
var<private> KIND_TALL: array<f32, 3> = array<f32, 3>(5.0, 1.4, 10.0);
const SNAG_VARY = 0.3;
// The longer since the burn (`old`, its share of the most months the core tells apart), the more dead trees are bare
// grey trees (`BARE_FRESH` of them at once, `BARE_OLD` more when old) and stumps (`STUMP_OLD` when old) and the more
// they lean, by up to `LEAN_MOST` radians, more on the burnt than on the bare.
const BARE_FRESH = 0.15; const BARE_OLD = 0.5; const STUMP_OLD = 0.35; const LEAN_MOST = 0.6;
// Dead wood weathers grey: a bare tree's bark is drawn `GREYED` of the way to its own grey, lit to `GREY_LIGHT` times as
// light. Each dead tree's tone is moved by up to `TINT` either way, warmer or cooler, by its own draw.
const GREYED = 0.65; const GREY_LIGHT = 1.35; const TINT = 0.12;
// The charred sprites' scans read a little blue: burnt bark in the sun is charcoal to grey-brown, so they are drawn
// warmer by `CHAR_TONE`.
const CHAR_TONE = vec3f(1.1, 1.0, 0.85);
// The trees with bare twins (sprites.wgsl `bare_of`): sprites 0 to `BARE_TREES_A - 1` and from `BARE_TREES_A + 2` to
// `BARE_TREES_N + 1` (the quiver trees' second pair has none).
const BARE_TREES_A = 19; const BARE_TREES_N = 27;
// The empty share of the cells is never taken under `EMPTY_LEAST`, so a full stand's few gaps do not take all its dead
// trees.
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
    q.kind = select(KIND_TRUNK, KIND_STUMP, pick < bare + STUMP_OLD * old);
    q.s = KIND_FROM[q.kind] + min(i32(r * f32(KIND_N[q.kind])), KIND_N[q.kind] - 1);
  }
  q.h = min(KIND_TALL[q.kind] * mix(1.0 - SNAG_VARY, 1.0 + SNAG_VARY, snag_unit(id, 44)), tallest);
  q.lean = LEAN_MOST * old * (2.0 * snag_unit(id, 47) - 1.0) * select(1.0, 0.5, q.kind == KIND_BARE);
  q.mirror = select(1.0, -1.0, snag_unit(id, 49) < 0.5);
  q.tint = 1.0 + TINT * (2.0 * snag_unit(id, 50) - 1.0) * vec3f(1.0, 0.4, -0.6);
  return q;
}

// Snag q's content box: its width and its height over its foot as shares of its sprite's side.
fn snag_size(q: Snag) -> vec2f {
  if (q.kind == KIND_BARE) { return vec2f(model_w(q.s), model_h(q.s)); }
  return vec2f(SNAG_W[q.s], SNAG_H[q.s]);
}

// Snag q at `uv` in its content (0..1, y down; outside: none), `g` of its content a pixel along each of its axes:
// linear, and cover. (Its mip is chosen by its size on the screen.)
fn snag_texel(q: Snag, uv: vec2f, g: vec2f) -> vec4f {
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  if (q.kind == KIND_BARE) {
    let t = model_texel(bare_of(q.s), uv, max(g.x, g.y));
    let grey = vec3f(dot(t.rgb, LUMA));
    return vec4f(mix(t.rgb, grey, GREYED) * GREY_LIGHT * q.tint, t.a);
  }
  let size = snag_size(q);
  // Its cell: a whole layer, or a quarter of one.
  let k = SNAG_QUARTER[q.s];
  let scale = select(0.5, 1.0, k < 0);
  let origin = select(vec2f(f32(k % 2), f32(k / 2)) * 0.5, vec2f(0.0), k < 0);
  let at = origin + scale * (vec2f(0.5 - 0.5 * size.x, 1.0 - SNAG_F[q.s] - size.y) + uv * size);
  let t = textureSampleGrad(detail_tex, detail_sampler, at, SNAGS_FROM + SNAG_LAYER[q.s], vec2f(scale * g.x * size.x, 0.0), vec2f(0.0, scale * g.y * size.y));
  return vec4f(unsrgb(t.rgb) * CHAR_TONE * q.tint, t.a);
}

// Snag q seen at `d` m from its foot, `e` m a pixel, on a lattice whose plants reach `reach` m: its shadow's darkness
// (as a tree's, models.wgsl `model_shadow`: from its foot, its sprite laid `MODEL_SHADOW_RUN` of its height south, no
// longer than 0.9 of the reach, and as far east on the ground, fading out before the reach's edge, `CROWN_SHADOW` dark,
// leaning as it leans), and itself (linear, cover) standing up from the same foot as the map's angled view draws it, turned
// by its lean about it.
struct SnagSeen { shadow: f32, rgba: vec4f }
fn snag_seen(q: Snag, d: vec2f, e: f32, reach: f32) -> SnagSeen {
  let size = snag_size(q);
  let side = q.h / size.y;
  let w = size.x * side;
  var out = SnagSeen(0.0, vec4f(0.0));
  let up = d.y / min(MODEL_SHADOW_RUN * q.h, 0.9 * reach);
  if (up >= 0.0 && up <= 1.0) {
    // (Its lean lays the shadow's top as far east as the trunk's top stands.)
    let edge = (1.0 - smoothstep(0.8 * reach, reach, abs(d.x))) * (1.0 - smoothstep(0.8 * reach, reach, d.y));
    let across = d.x - d.y - tan(q.lean) * up * q.h;
    out.shadow = CROWN_SHADOW * edge * snag_texel(q, vec2f(q.mirror * across / w + 0.5, 1.0 - up), vec2f(e / w, e / (MODEL_SHADOW_RUN * q.h))).a;
  }
  let r = vec2f(cos(q.lean) * d.x + sin(q.lean) * d.y, -sin(q.lean) * d.x + cos(q.lean) * d.y);
  out.rgba = snag_texel(q, vec2f(0.5 + q.mirror * r.x / w, 1.0 + r.y / (MAP_VIEW.y * q.h)), vec2f(e / w, e / (MAP_VIEW.y * q.h)));
  return out;
}
