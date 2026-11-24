// The dead trees fire left standing (the core's dead walls standing, `FireAt.snag`, kg/m²): marks of the plant cover's
// trees' lattice decided where no living plant stands, by the living marks' rules of the ground (mark.wgsl
// `dead_decided`), and drawn with them in one order (cover.wgsl `lattice`), so a living crown nearer the eye hides a
// dead trunk behind it. Each is a charred trunk broken off or
// a charred stump (web/textures/snags.png), or a whole tree of the map's own forms with its
// leaves gone (its bare twin, models.png) weathered grey: a fresh burn's are mostly charred, older ones more often grey,
// bare or broken to stumps, and leaning. Each is mirrored, sized, leaned and tinted by its own draw. Colours linear.

// A dead stem breaks down as the years pass: it stands whole, its twigs and branches gone (a bare tree of the map's own
// forms, models.png), then broken off to a trunk (snags.png: `TRUNK_SHARE` of its height), then to a stump
// (`STUMP_SHARE`, no taller than `STUMP_MOST_M`). Each does so at its own time: its stage is the years since it died over
// `BREAK_YEARS` (dead trees lose their crowns within years and break within one or two decades: from memory), moved by
// up to `BREAK_DRAW` by its own draw; whole below `BROKEN_FROM`, a stump from `STUMP_FROM_STAGE`. So a dead tree only
// ever breaks further, never stands again. A dead stem under `SHRUB_M` (a shrub's) stays whole till it falls.
const BREAK_YEARS = 8.0; const BREAK_DRAW = 0.6; const BROKEN_FROM = 0.6; const STUMP_FROM_STAGE = 1.3;
const TRUNK_SHARE = 0.5; const STUMP_SHARE = 0.15; const STUMP_MOST_M = 1.4; const SHRUB_M = 4.0;
// The kinds: bare whole trees and shrubs, charred trunks and charred stumps (snags.png, snag_sprites.wgsl: each stands
// on its foot, the row where its trunk meets the ground); each one's height `SNAG_VARY` either way of its share of the
// stem's, no taller and no wider than its lattice lets a mark reach (`snag_of`).
const KIND_TRUNK = 0; const KIND_STUMP = 1; const KIND_BARE = 2;
var<private> KIND_FROM: array<i32, 2> = array<i32, 2>(TRUNK_FROM, STUMP_FROM);
var<private> KIND_N: array<i32, 2> = array<i32, 2>(TRUNK_N, STUMP_N);
const SNAG_VARY = 0.2;
// The older (`old`, its stage over `STUMP_FROM_STAGE`, at most 1), the more a dead tree leans, by up to `LEAN_MOST`
// radians, more on the broken than on the whole.
const LEAN_MOST = 0.6;
// A fresh whole dead tree is charred: its bark drawn `CHARRED` of the way to char (fire.wgsl `CHAR`) at once, weathering
// grey over `GREY_YEARS` (charcoal flakes off within a few years: from memory), drawn `GREYED` of the way to its own
// grey, lit to `GREY_LIGHT` times as light. Each dead tree's tone is moved by up to `TINT` either way, warmer or cooler,
// by its own draw.
const CHARRED = 0.8; const SNAG_CHAR = vec3f(0.04, 0.035, 0.03); const GREY_YEARS = 4.0; const GREYED = 0.65; const GREY_LIGHT = 1.35; const TINT = 0.12;
// A charred trunk's or stump's sprite is all stem, its base about `SNAG_STEM` of its width from its foot (its contact
// shade's reach, models.wgsl `contact_shade`).
const SNAG_STEM = 0.4;
// The charred sprites' scans read a little blue: burnt bark in the sun is charcoal to grey-brown, so they are drawn
// warmer by `CHAR_TONE`.
const CHAR_TONE = vec3f(1.1, 1.0, 0.85);
// The trees whose bare twins (sprites.wgsl `bare_of`) stand as dead trees: all with one but the baobabs (a swollen
// trunk reads as a heap, not a tree), a side of an island tree and of an island tree's second model whose branches are cut by
// their frame, and a horse chestnut whose fine twigs blur into a haze.
const BARE_TREES_N = 21;
var<private> BARE_TREES: array<i32, 21> = array<i32, 21>(0, 1, 2, 4, 6, 7, 8, 10, 11, 12, 13, 17, 18, 21, 22, 23, 24, 25, 26, 27, 28);

fn snag_unit(c: vec2i, k: i32) -> f32 { return f32(lowbias(c, k)) * (1.0 / 4294967296.0); }

// A dead tree: its kind (-1: none) and sprite (of snags.png, or the tree whose bare twin it is), its height, m, its
// lean, radians, whether it is mirrored (-1) or not (1), its tint, and how charred it is (0..1, a whole tree's).
struct Snag { kind: i32, s: i32, h: f32, lean: f32, mirror: f32, tint: vec3f, char: f32 }

// The dead tree of the cell `id` (fixed in the world), dead for `years`, of plants `tall` m tall, on a
// lattice whose marks reach `side` m east, west and south of their foot and `north` m north: its kind by how far it
// has broken down (`BREAK_YEARS`), its sprite, its height and its lean within that box.
fn snag_of(id: vec2i, years: f32, tall: f32, side: f32, north: f32) -> Snag {
  var q: Snag;
  let stage = select(years / BREAK_YEARS + BREAK_DRAW * snag_unit(id, 42), 0.0, tall < SHRUB_M);
  let old = min(stage / STUMP_FROM_STAGE, 1.0);
  let r = snag_unit(id, 43);
  var share = 1.0;
  if (stage < BROKEN_FROM) {
    q.kind = KIND_BARE;
    q.s = BARE_TREES[min(i32(r * f32(BARE_TREES_N)), BARE_TREES_N - 1)];
  } else {
    q.kind = select(KIND_TRUNK, KIND_STUMP, stage >= STUMP_FROM_STAGE);
    q.s = KIND_FROM[q.kind] + min(i32(r * f32(KIND_N[q.kind])), KIND_N[q.kind] - 1);
    share = select(TRUNK_SHARE, STUMP_SHARE, q.kind == KIND_STUMP);
  }
  q.lean = LEAN_MOST * old * (2.0 * snag_unit(id, 47) - 1.0) * select(1.0, 0.5, q.kind == KIND_BARE);
  let size = snag_size(q);
  let wide = side / (0.5 * size.x / size.y + abs(sin(q.lean)));
  var h = tall * share * mix(1.0 - SNAG_VARY, 1.0 + SNAG_VARY, snag_unit(id, 44));
  if (q.kind == KIND_STUMP) { h = min(h, STUMP_MOST_M); }
  q.h = min(h, min(north / MAP_VIEW.y, wide));
  q.mirror = select(1.0, -1.0, snag_unit(id, 49) < 0.5);
  q.tint = 1.0 + TINT * (2.0 * snag_unit(id, 50) - 1.0) * vec3f(1.0, 0.4, -0.6);
  q.char = CHARRED * exp(-years / GREY_YEARS);
  return q;
}

// Snag q's content box: its width and its height over its foot as shares of its sprite's side.
fn snag_size(q: Snag) -> vec2f {
  if (q.kind == KIND_BARE) { return vec2f(model_w(bare_of(q.s)), model_h(bare_of(q.s))); }
  return vec2f(SNAG_W[q.s], SNAG_H[q.s]);
}

// Snag q at `uv` in its content (0..1, y down; outside: none), `g` of its content a pixel along each of its axes:
// linear, and cover. (Its mip is chosen by its size on the screen.)
fn snag_texel(q: Snag, uv: vec2f, g: vec2f) -> vec4f {
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  if (q.kind == KIND_BARE) {
    let t = model_texel(bare_of(q.s), uv, max(g.x, g.y));
    let grey = mix(t.rgb, vec3f(dot(t.rgb, LUMA)), GREYED) * GREY_LIGHT * q.tint;
    return vec4f(mix(grey, SNAG_CHAR * q.tint, q.char), t.a);
  }
  let size = snag_size(q);
  // Its cell: a whole layer, or a quarter of one.
  let k = SNAG_QUARTER[q.s];
  let scale = select(0.5, 1.0, k < 0);
  let origin = select(vec2f(f32(k % 2), f32(k / 2)) * 0.5, vec2f(0.0), k < 0);
  let at = origin + scale * (vec2f(0.5 - 0.5 * size.x, 1.0 - SNAG_F[q.s] - size.y) + uv * size);
  let t = textureSampleGrad(detail_tex, detail_sampler, at, SNAGS_FROM + SNAG_LAYER[q.s], vec2f(scale * g.x * size.x, 0.0), vec2f(0.0, scale * g.y * size.y));
  return vec4f(unsrgb(t.rgb) * CHAR_TONE * q.tint, t.a * box_fade(uv, g));
}

// Snag q's width, m.
fn snag_w(q: Snag) -> f32 { let size = snag_size(q); return size.x * q.h / size.y; }
// The cast of snag q on a lattice whose marks reach `side` m east and south of their foot (models.wgsl `cast_of`): its
// lean lays the shadow's top as far east as the trunk's top stands.
fn snag_cast(q: Snag, side: f32) -> Cast { return cast_of(q.h, snag_w(q), tan(q.lean) * q.h, side); }

// Snag q seen at `d` m from its foot, `e` m a pixel, on a lattice whose marks reach `side` m east and south of their
// foot: its shadow's darkness (as a tree's, models.wgsl `model_shadow`: its sprite cast on the ground, `CROWN_SHADOW`
// dark, and the contact shade of its stem at its foot: a trunk's or a stump's sprite is all stem), and itself (linear,
// cover) standing up from the same foot as the map's angled view draws it, turned by its lean about it.
struct SnagSeen { shadow: f32, rgba: vec4f }
fn snag_seen(q: Snag, d: vec2f, e: f32, side: f32) -> SnagSeen {
  let w = snag_w(q);
  var out = SnagSeen(0.0, vec4f(0.0));
  let c = snag_cast(q, side);
  let uv = cast_uv(c, d, 0.0);
  if (all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0))) {
    out.shadow = CROWN_SHADOW * snag_texel(q, vec2f(0.5 + q.mirror * (uv.x - 0.5), uv.y), vec2f(e / c.w, e / c.long)).a;
  }
  out.shadow = max(out.shadow, contact_shade(d, select(SNAG_STEM * w, STEM_REACH * q.h, q.kind == KIND_BARE)));
  let r = vec2f(cos(q.lean) * d.x + sin(q.lean) * d.y, -sin(q.lean) * d.x + cos(q.lean) * d.y);
  out.rgba = snag_texel(q, vec2f(0.5 + q.mirror * r.x / w, 1.0 + r.y / (MAP_VIEW.y * q.h)), vec2f(e / w, e / (MAP_VIEW.y * q.h)));
  return out;
}
