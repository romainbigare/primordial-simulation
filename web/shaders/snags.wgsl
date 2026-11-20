// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl sprites.wgsl heights.wgsl tex.wgsl fire.wgsl snag_sprites.wgsl
// The dead trees fire left standing over the live map, over its plants (web/src/fire.ts; drawn only over the squares
// that hold them, in the Earth view): as many as the core's dead walls standing (`FireAt.snag`, kg/m²) make, one at most
// in each cell of a lattice fixed in the world. Each is a charred trunk broken off, a stand of dead stems or a charred
// stump (web/textures/snags.png), or a whole tree of the map's own forms with its leaves gone (its bare twin, models.png)
// weathered grey: a fresh burn's are mostly charred, older ones more often grey, bare or broken to stumps, and leaning.
// Each is mirrored, sized, leaned and tinted by its own draw. Seen from afar, their mean dark over the ground. Colours
// are the screen's shares, 0..1, premultiplied.

// One dead tree holds `SNAG_KG` of walls, from memory (a stem of about 20 cm and 10 m). A cell of the lattice is
// `SNAG_CELL` m across and holds one with the chance its share of the walls makes.
const SNAG_KG = 150.0; const SNAG_CELL = 8.0;
// Each sprite stands on its quarter's bottom middle `SNAG_FOOT` of the quarter over the edge (bake/snags.py).
const SNAG_FOOT = 2.0 / 256.0;
// The kinds: charred trunks, stands of dead stems, charred stumps (snags.png, snag_sprites.wgsl) and bare trees; each
// one's height drawn, m (its tallest part), `SNAG_VARY` either way.
const KIND_TRUNK = 0; const KIND_STEMS = 1; const KIND_STUMP = 2; const KIND_BARE = 3;
var<private> KIND_FROM: array<i32, 3> = array<i32, 3>(TRUNK_FROM, STEMS_FROM, STUMP_FROM);
var<private> KIND_N: array<i32, 3> = array<i32, 3>(TRUNK_N, STEMS_N, STUMP_N);
var<private> KIND_TALL: array<f32, 4> = array<f32, 4>(5.0, 9.0, 1.4, 12.0);
const SNAG_VARY = 0.3;
// The longer since the burn (`old`, its share of the most months the core tells apart), the more dead trees are bare
// grey trees (`BARE_FRESH` of them at once, `BARE_OLD` more when old) and stumps (`STUMP_OLD` when old) and the more
// they lean, by up to `LEAN_MOST` radians, more on the burnt than on the bare; of the charred ones `STEMS_SHARE` are
// stands of stems.
const BARE_FRESH = 0.15; const BARE_OLD = 0.5; const STUMP_OLD = 0.35; const STEMS_SHARE = 0.4; const LEAN_MOST = 0.6;
// Dead wood weathers grey: a bare tree's bark is drawn `GREYED` of the way to its own grey, lit to `GREY_LIGHT` times as
// light. Each dead tree's tone is moved by up to `TINT` either way, warmer or cooler, by its own draw.
const GREYED = 0.65; const GREY_LIGHT = 1.35; const TINT = 0.12;
// The trees with bare twins (sprites.wgsl `bare_of`): sprites 0 to `BARE_TREES_A - 1` and from `BARE_TREES_A + 2` to
// `BARE_TREES_N + 1` (the quiver trees' second pair has none).
const BARE_TREES_A = 19; const BARE_TREES_N = 27;
// From afar, a snag darkens the ground it stands over by `SNAG_DARK` of its cover. Up close each casts its shadow as a
// tree does (models.wgsl `model_shadow`: its sprite laid `MODEL_SHADOW_RUN` of its height south and as far east on the
// ground, `CROWN_SHADOW` dark), no lean drawn in it.
const SNAG_DARK = 0.5;

fn unit(c: vec2i, k: i32) -> f32 { return f32(lowbias(c, k)) * (1.0 / 4294967296.0); }

// The dead tree of lattice cell c where its chance is `chance` and the burn's age `old`: its kind (-1: none) and sprite
// (of snags.png, or the tree whose bare twin it is), its height, m, its foot, m, its lean, radians, whether it is
// mirrored (-1) or not (1), and its tint.
struct Snag { kind: i32, s: i32, h: f32, foot: vec2f, lean: f32, mirror: f32, tint: vec3f }
fn snag_of(c: vec2i, chance: f32, old: f32) -> Snag {
  var q: Snag;
  q.kind = -1;
  if (unit(c, 41) >= chance) { return q; }
  let pick = unit(c, 42);
  let bare = BARE_FRESH + BARE_OLD * old;
  let r = unit(c, 43);
  if (pick < bare) {
    q.kind = KIND_BARE;
    let k = min(i32(r * f32(BARE_TREES_N)), BARE_TREES_N - 1);
    q.s = select(k + 2, k, k < BARE_TREES_A);
  } else {
    q.kind = select(select(KIND_TRUNK, KIND_STEMS, unit(c, 48) < STEMS_SHARE), KIND_STUMP, pick < bare + STUMP_OLD * old);
    q.s = KIND_FROM[q.kind] + min(i32(r * f32(KIND_N[q.kind])), KIND_N[q.kind] - 1);
  }
  q.h = KIND_TALL[q.kind] * mix(1.0 - SNAG_VARY, 1.0 + SNAG_VARY, unit(c, 44));
  q.foot = (vec2f(c) + vec2f(unit(c, 45), unit(c, 46))) * SNAG_CELL;
  q.lean = LEAN_MOST * old * (2.0 * unit(c, 47) - 1.0) * select(1.0, 0.5, q.kind == KIND_BARE);
  q.mirror = select(1.0, -1.0, unit(c, 49) < 0.5);
  q.tint = 1.0 + TINT * (2.0 * unit(c, 50) - 1.0) * vec3f(1.0, 0.4, -0.6);
  return q;
}

// Snag q's content box: its width and height as shares of its sprite's side.
fn snag_size(q: Snag) -> vec2f {
  if (q.kind == KIND_BARE) { return vec2f(model_w(q.s), model_h(q.s)); }
  return vec2f(SNAG_W[q.s], SNAG_H[q.s]);
}

// Snag q at `uv` in its content (0..1, y down; outside: none), at `g` of its content a pixel: the screen's shares, and
// cover.
fn snag_texel(q: Snag, uv: vec2f, g: f32) -> vec4f {
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  if (q.kind == KIND_BARE) {
    let t = model_texel(bare_of(q.s), uv, g);
    let grey = vec3f(dot(t.rgb, LUMA));
    return vec4f(srgb(mix(t.rgb, grey, GREYED) * GREY_LIGHT * q.tint), t.a);
  }
  let size = snag_size(q);
  let k = q.s % 4;
  let quarter = vec2f(f32(k % 2), f32(k / 2));
  let at = (quarter + vec2f(0.5 - 0.5 * size.x, 1.0 - size.y - SNAG_FOOT) + uv * size) * 0.5;
  let t = textureSampleGrad(detail_tex, detail_sampler, at, SNAGS_FROM + q.s / 4, vec2f(0.5 * g * size.x, 0.0), vec2f(0.0, 0.5 * g * size.y));
  return vec4f(srgb(unsrgb(t.rgb) * q.tint), t.a);
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  pix_origin = fire.frame.xy;
  pix_local = fire.corner.xy + v.px * fire.frame.z;
  let at = pix_origin + pix_local;
  let e = fire.frame.w;
  let f = fire_at(at);
  if (f.snag <= 0.0 || standing(ground_at(at, false)) > FILM_M) { return vec4f(0.0); }
  let chance = min(f.snag * SNAG_CELL * SNAG_CELL / SNAG_KG, 1.0);
  let old = clamp(f.age / max(fire.air.y, 1.0), 0.0, 1.0);
  // Up close each dead tree in the cells whose trees reach the point; from afar their mean.
  let shows = as_thing(MAP_VIEW.y * KIND_TALL[KIND_TRUNK], e);
  var out = vec4f(0.0);
  if (shows > 0.0) {
    let cell = vec2i(floor(at / SNAG_CELL));
    // The cells whose trees reach the point: south of it within the tallest's height as drawn, either side within half
    // its height (no sprite is wider than tall), or north and west of it within its shadow's length.
    let most = KIND_TALL[KIND_BARE] * (1.0 + SNAG_VARY);
    let down = i32(ceil(MAP_VIEW.y * most / SNAG_CELL));
    let across = i32(ceil(0.5 * most / SNAG_CELL));
    let back = i32(ceil(MODEL_SHADOW_RUN * most / SNAG_CELL));
    var shade = 0.0;
    var sprites = vec4f(0.0);
    for (var dy = -back; dy <= down; dy++) {
      for (var dx = -back - across; dx <= across; dx++) {
        let q = snag_of(cell + vec2i(dx, dy), chance, old);
        if (q.kind < 0) { continue; }
        let d = at - q.foot;
        let size = snag_size(q);
        // Its side (its height the sprite's share of it) and width, m.
        let side = q.h / size.y;
        let w = size.x * side;
        // Its shadow, laid from its foot (models.wgsl `model_shadow`).
        let up = d.y / (MODEL_SHADOW_RUN * q.h);
        if (up >= 0.0 && up <= 1.0) { shade = max(shade, CROWN_SHADOW * snag_texel(q, vec2f(q.mirror * (d.x - d.y) / w + 0.5, 1.0 - up), e / side).a); }
        // Itself, standing up from its foot as the map's angled view draws it, turned by its lean about its foot.
        let r = vec2f(cos(q.lean) * d.x + sin(q.lean) * d.y, -sin(q.lean) * d.x + cos(q.lean) * d.y);
        let t = snag_texel(q, vec2f(0.5 + q.mirror * r.x / w, 1.0 + r.y / (MAP_VIEW.y * q.h)), e / side);
        if (t.a > 0.0) { sprites = vec4f(t.rgb * t.a, t.a) + sprites * (1.0 - t.a); }
      }
    }
    out = sprites + vec4f(0.0, 0.0, 0.0, shade) * (1.0 - sprites.a);
  }
  let tall = KIND_TALL[KIND_TRUNK];
  let mean = SNAG_DARK * chance * 0.5 * SNAG_W[0] * MAP_VIEW.y * tall * tall / (SNAG_CELL * SNAG_CELL);
  return mix(vec4f(0.0, 0.0, 0.0, min(mean, 1.0)), out, shows) * draw.alpha;
}
