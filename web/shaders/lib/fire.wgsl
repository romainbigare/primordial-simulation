// What fire leaves and does on each square of the live map (web/src/fire.ts), as the core's layers say it, and where it
// lies inside a square: read by the ground's burn scars (burn.wgsl) and by the flames and smoke over the map
// (flames.wgsl), and by the dead trees standing on them among the plants (snag.wgsl, cover.wgsl). Points are in metres from the map's north-west corner.

struct Fire {
  // A square's side, m; the map's squares across and down; w is not read.
  map: vec4f,
  // Where a pixel of the target lies in the world (field-window.ts `frameOf`): the origin of the draw, m, the metres a
  // pixel of the target and of the screen take; the world's point under the target's first pixel, m from the origin.
  frame: vec4f, corner: vec4f,
  // The time, s (wrapped); the months since a burn that mean none (the core tells no more apart).
  air: vec4f,
  // The burns of god mode (web/src/burns.ts `Burns.uniforms`), `BURN_VEC4S` members each: the way downwind (east,
  // south, a unit), how far its head runs ahead of its mean growth and its breadth over its length; how far its mean
  // growth has reached, m (past every square's once it burnt out), the months since it burnt, the share of the soft
  // tissue it killed and its flames' length, m; the count of the places it grew from; those places, m, two a member.
  burns: array<vec4f, 28>,
}
// The burns the uniforms hold, the members each takes and the most places each grows from (burns.ts `BURNS`,
// `BURN_VEC4S`, `SEEDS`).
const BURNS = 4; const BURN_VEC4S = 7; const SEEDS = 8;
@group(0) @binding(25) var fire_tex: texture_2d<f32>;
@group(0) @binding(26) var fire_sampler: sampler;
@group(0) @binding(27) var<uniform> fire: Fire;

// e^x - 1, kept exact for small x.
fn expm1(x: f32) -> f32 { return select(exp(x) - 1.0, x + 0.5 * x * x, abs(x) < 1e-3); }

// What fire did at a point, blended between the squares' centres: the share of the land under a scar (its last burn's,
// within the months the core tells apart, fire.ts), the months since its last burn (the scar's mean), the share burning
// this month in patches, its flames' mean length, m (the burning share's mean), the ash on the ground and the gas the
// flames sent to the air this month, kg/m²; the low wind, m/s east and south; the cured leaves standing, kg/m²; the
// share of the scar laid in patches (the rest is a burn of god mode's, `burn_at`); and the share of the soft tissue
// standing on the scar its burn killed (the scar's mean). (Its third plane's second number is the share of its land
// saturated, `under_share`.)
struct FireAt { burnt: f32, age: f32, flame: f32, length: f32, ash: f32, air: f32, wind: vec2f, cured: f32, rest: f32, crown: f32 }

// A dead stem as tall as the plants that died (`Patch::snag_h`) holds `snag_kg` of walls (snag.wgsl, burn.wgsl):
// `SNAG_KG` at `SNAG_KG_TALL_M` (a stem of about 20 cm and 10 m, from memory), its mass growing with the fourth power
// of its height (a stem's width grows with its height to the 3/2, McMahon 1973, so its mass with height times width
// squared).
const SNAG_KG = 150.0; const SNAG_KG_TALL_M = 10.0;
fn snag_kg(h: f32) -> f32 { return SNAG_KG * pow(max(h, 0.5) / SNAG_KG_TALL_M, 4.0); }

// A dead tree `h` m tall and its shadow cover the map as a disc `DEAD_SPECK` of its height across (a 10 m trunk and
// its shadow cover about 4 m²): the specks of the dead trees far away (burn.wgsl `dead_far`), and the width by which
// they are drawn one by one near (mark.wgsl `dead_decided`), so the one gives way to the other at one scale.
const DEAD_SPECK = 0.22;
fn dead_seen_w(h: f32) -> f32 { return DEAD_SPECK * h; }

// Char, the black of a fresh burn, reflectance, linear (charred litter and soil reflect 3 to 6 % of visible light:
// Smith and Hudak 2005, Roy and others 2010, read).
const CHAR = vec3f(0.022, 0.020, 0.019);
// The char darkens a scar by e^-(months / `CHAR_MONTHS`): wind and rain take a burn's char and ash away within months
// (burn scars of savannas fade in 1 to 3 months, of forests over a year or more: Trigg and Flasse 2000); its bared,
// scorched soil fades by e^-(months / `SCORCHED_MONTHS`) as litter falls on it again.
const CHAR_MONTHS = 3.0; const SCORCHED_MONTHS = 12.0;
// Ash on the ground covers all but e^-1 of it at this much, kg/m²: a layer one grain deep of fine ash, grains of some
// 50 µm at about 2 t/m³ (fine ash is mostly silt-sized: Bodí and others 2014, read; the layer's mass worked out from
// them).
const ASH_HIDES_KG = 0.08;
// The Earth view darkens the plants standing on a fresh scar towards the char by this much (scorched leaves and
// stems, black with soot; they fade with the char as the months pass).
const SCORCHED_COVER = 0.75;

// The planes of the fire's texture (fire.ts), one under the other.
const FIRE_PLANES = 4.0;

// Plane k of the fire's texture at `at`, blended between the squares' centres and held inside its plane.
fn fire_plane(k: i32, at: vec2f) -> vec4f {
  let n = fire.map.yz;
  let s = clamp(at / fire.map.x, vec2f(0.5), n - 0.5);
  return textureSampleLevel(fire_tex, fire_sampler, (s + vec2f(0.0, f32(k) * n.y)) / vec2f(n.x, FIRE_PLANES * n.y), 0.0);
}

// Plane k of the fire's texture at square `c` (held inside the map), its own numbers.
fn fire_texel(k: i32, c: vec2i) -> vec4f {
  let n = vec2i(fire.map.yz);
  return textureLoad(fire_tex, clamp(c, vec2i(0), n - 1) + vec2i(0, k * n.y), 0);
}

fn fire_at(at: vec2f) -> FireAt {
  let a = fire_plane(0, at);
  let b = fire_plane(1, at);
  let c = fire_plane(2, at);
  return FireAt(a.x, a.y / max(a.x, 1e-6), a.z, a.w / max(a.z, 1e-6), b.x, b.y, b.zw, c.x, c.w, c.z / max(a.x, 1e-6));
}

// The months since the burn of the scar in patches at `at` (`FireAt.rest`), a point of rank `rank` among the patches
// (`burn_rank`'s number made even, 0..1): of one of the four squares whose centres lie about it, drawn by the rank with
// the chance of each square's blend weight times its scar in patches. So where scars of different ages meet, the point
// takes one of them, and their meeting is as ragged as the patches; a blend of their ages would lay a straight line where
// the younger one's char fades.
fn scar_age(at: vec2f, rank: f32) -> f32 {
  let s = at / fire.map.x - 0.5;
  let c = vec2i(floor(s));
  let f = fract(s);
  var t: array<vec4f, 4>;
  var w = vec4f((1.0 - f.x) * (1.0 - f.y), f.x * (1.0 - f.y), (1.0 - f.x) * f.y, f.x * f.y);
  for (var k = 0; k < 4; k++) {
    t[k] = fire_texel(0, c + vec2i(k % 2, k / 2));
    w[k] *= fire_texel(2, c + vec2i(k % 2, k / 2)).w;
  }
  var left = rank * (w.x + w.y + w.z + w.w);
  for (var k = 0; k < 3; k++) {
    if (left < w[k]) { return t[k].y / max(t[k].x, 1e-6); }
    left -= w[k];
  }
  return t[3].y / max(t[3].x, 1e-6);
}

// A square's burnt share lies in patches with ragged edges, from `BURN_PATCH_M` across down to the pixel (ragged.wgsl
// `meet_noise`, noise stream `BURN_NOISE`): a fire runs where its fuel joins and stops at gaps, so its scar is a
// mosaic of burnt and spared ground (Turner and others 1994). A point is burnt where its patches' number lies under
// the level of the burnt share of the dry ground (ragged.wgsl `share_level`), so the drawn share is the square's at
// every zoom and, where it falls to none, thins to none.
const BURN_PATCH_M = 600.0; const BURN_NOISE = 91;

// The patches' number at point `at` seen at `e` m a pixel, and the spread a share read on it takes (`meet_noise`).
fn burn_rank(at: vec2f, e: f32) -> vec2f { return meet_noise(at, e, BURN_PATCH_M, BURN_NOISE); }

// How far burn k's mean growth has reached when its edge reaches `at`, m (burns.ts `reachAt`): the least over the
// places it grew from of the ellipse of semi-axes t along the wind and its breadth times t across it, its centre its
// lead times t downwind of the place.
fn burn_time(k: i32, at: vec2f) -> f32 {
  let g = fire.burns[k * BURN_VEC4S];
  let n = i32(fire.burns[k * BURN_VEC4S + 2].x);
  let c = g.z;
  let q = 1.0 - c * c;
  var least = 1e9;
  for (var j = 0; j < SEEDS; j++) {
    if (j >= n) { break; }
    let pair = fire.burns[k * BURN_VEC4S + 3 + j / 2];
    let d = at - select(pair.xy, pair.zw, j % 2 == 1);
    let u = dot(d, g.xy);
    let v = g.x * d.y - g.y * d.x;
    least = min(least, (-u * c + sqrt(u * u * c * c + q * (u * u + v * v / (g.w * g.w)))) / q);
  }
  return least;
}

// A burn of god mode's edge is ragged by `BURN_RAGGED` of how far it reached (no less than `BURN_RAGGED_LEAST_M`), in
// patches from `BURN_EDGE_M` across down to the pixel (noise stream `BURN_EDGE_NOISE`): a fire's edge runs out along
// its fuel and stops at gaps. A point takes the burns of the square `BURN_WARP` of a side about it, moved along noise
// `BURN_WARP_M` across (streams `BURN_WARP_NOISE` and the next), so a burn that ends at a square's edge ends on a winding
// line.
const BURN_RAGGED = 0.3; const BURN_RAGGED_LEAST_M = 20.0; const BURN_EDGE_M = 120.0; const BURN_EDGE_NOISE = 95;
const BURN_WARP = 0.06; const BURN_WARP_M = 600.0; const BURN_WARP_NOISE = 96;

// The burn of god mode at `at` (of those on its square, the one it lies deepest inside), seen at `e` m a pixel: how far
// inside its burnt land the point lies, m (less than 0: outside; -1e9 where none lies), the spread of that over the
// pixel's patches, m, whether its front still moves there (1) or the square's share has been taken (0), its flames'
// length, m, the months since it burnt, and the share of the soft tissue on its land it killed (the burn's own).
struct BurnAt { inside: f32, soft: f32, live: f32, length: f32, age: f32, crown: f32 }
fn burn_at(at: vec2f, e: f32) -> BurnAt {
  var out = BurnAt(-1e9, 1.0, 0.0, 0.0, 0.0, 0.0);
  // (No burn lies on the four squares about the point: none is read.)
  if (fire.map.x <= 0.0 || dot(fire_plane(3, at), vec4f(1.0)) <= 0.0) { return out; }
  let w = vec2f(ragged(at, e, BURN_WARP_M, 3, BURN_WARP_NOISE), ragged(at, e, BURN_WARP_M, 3, BURN_WARP_NOISE + 1));
  let limits = fire_texel(3, vec2i(floor(at / fire.map.x + BURN_WARP * w)));
  let n = meet_noise(at, e, BURN_EDGE_M, BURN_EDGE_NOISE);
  for (var k = 0; k < BURNS; k++) {
    let t = limits[k];
    if (t <= 0.0) { continue; }
    let play = fire.burns[k * BURN_VEC4S + 1];
    let limit = min(play.x, t);
    let scale = BURN_RAGGED * max(limit, BURN_RAGGED_LEAST_M);
    let inside = limit - burn_time(k, at) - scale * n.x;
    if (inside > out.inside) { out = BurnAt(inside, scale * n.y + 0.5 * e, select(0.0, 1.0, play.x < t), play.w, play.y, play.z); }
  }
  return out;
}

// The share of a point in the burnt land of burn `b` (`burn_at`).
fn burnt_by(b: BurnAt) -> f32 { return smoothstep(-b.soft, b.soft, b.inside); }

// The level of the burnt share `share` of the land at `at` among the patches (ragged.wgsl `share_level`): saturated
// ground does not burn (its fuel is wet; the core's burnt share leaves its wet share out), so the share fills the dry
// ground (`fire_plane`'s wet share) alone, as much of it as the core burnt.
fn burnt_level(share: f32, at: vec2f) -> f32 { return share_level(share / max(1.0 - fire_plane(2, at).y, 1e-6)); }

// The share of a point of rank `r` (`burn_rank`) in the burnt share `share` of the land at `at` (`burnt_level`), none
// of it on the saturated ground there (land.wgsl `saturated_at`).
fn under_share(r: vec2f, share: f32, at: vec2f) -> f32 {
  if (share <= 0.0) { return 0.0; }
  return smoothstep(-r.y, r.y, burnt_level(share, at) - r.x) * (1.0 - saturated_at(at));
}

// The scar at `at` of what fire did there `f` (`fire_at`) and the burn of god mode there `b` (`burn_at`), seen at `e`
// m a pixel: the share of the point under it (its share in patches, `under_share`, or the burn's land), how much of it
// is char, ash and scorched soil (0 where none lies there, burn.wgsl `scar_over`).
fn scar_of(f: FireAt, b: BurnAt, at: vec2f, e: f32) -> vec4f {
  if (f.burnt <= 0.0) { return vec4f(0.0); }
  let rank = burn_rank(at, e);
  let patches = under_share(rank, f.rest, at);
  let burn = burnt_by(b) * (1.0 - saturated_at(at));
  let scar = max(patches, burn);
  if (scar <= 0.0) { return vec4f(0.0); }
  // (Each part of the scar is as old as its own burn.)
  let age = select(scar_age(at, smoothstep(-EVEN_ONE, EVEN_ONE, rank.x)), b.age, burn > patches);
  // (The core's ash is per m² of the square's land: it lies on the land burnt, its share of it.)
  return vec4f(scar, exp(-age / CHAR_MONTHS), -expm1(-f.ash / max(f.burnt, 1e-6) / ASH_HIDES_KG), exp(-age / SCORCHED_MONTHS));
}

// The share of a point under the fresh char of scar `s` (`scar_of`), which scorches the plants standing there.
fn fresh_char(s: vec4f) -> f32 { return s.x * s.y; }

// A plant a front of god mode is passing burns: lit by the flames under it by `PLANT_GLOW_LEAST`, and by up to 1 as its
// crown burns (the share of the soft tissue its burn killed), in the flames' colour (`PLANT_GLOW`, linear, added); by
// e^-(m / `BURNING_M`) of the front's run behind it (a crown flames for under a minute, some metres of a front's run:
// from memory).
const PLANT_GLOW = vec3f(0.6, 0.19, 0.03); const PLANT_GLOW_LEAST = 0.35; const BURNING_M = 10.0;

// The share of the plants burning where the front of a burn of god mode `b` passes (`burn_at`).
fn burning_of(b: BurnAt) -> f32 {
  if (b.live <= 0.0) { return 0.0; }
  return burnt_by(b) * exp(-max(b.inside, 0.0) / BURNING_M);
}

// What fire does to the plants where fire did `f` (`fire_at`), its scar is `s` (`scar_of`) and a burn of god mode is
// `b` (`burn_at`): how far soot darkens the living towards the char (`SCORCHED_COVER` of the share under fresh char,
// `fresh_char`, by the share of the soft tissue the burn killed: soot stands as high as the flames and their heat
// reached), the share of the woody plants it killed (`crown_kill`: they stand dead, bare and charred, mark.wgsl
// `living_decided`), the months since it burnt, and how much they burn now (`burning_of`, by how much of their soft
// tissue the burn kills).
fn plant_fire(f: FireAt, s: vec4f, b: BurnAt) -> vec4f {
  // (On a burn of god mode's land, its own share killed.)
  let crown = mix(f.crown, b.crown, burnt_by(b));
  return vec4f(SCORCHED_COVER * fresh_char(s) * crown, crown * s.x, -SCORCHED_MONTHS * log(max(s.w, 1e-30)), burning_of(b) * mix(PLANT_GLOW_LEAST, 1.0, crown));
}

// Living plants of colour `rgb` (premultiplied by `a`) with soot `soot` and burning `burning` on them (`plant_fire`):
// darkened by their soot and lit by their burning, in the field of the cover (burn.wgsl `burnt_field`) and in its marks
// (cover.wgsl, decided once for each by mark.wgsl).
fn burnt_plant(rgb: vec3f, a: f32, soot: f32, burning: f32) -> vec3f { return mix(rgb, CHAR * a, soot) + PLANT_GLOW * a * burning; }

// Dead walls that died within `SAME_BURN_MONTHS` of a square's last burn are that burn's. The core keeps a burn of
// under `EVENT_LEAST` of a patch spread over it, its dead walls too (laws.md 3c); the map stands them on the land the
// burn took: their density at a point under `scar` of a scar (`scar_of`) of the square's share is their patch's mean
// times this.
const SAME_BURN_MONTHS = 1.5; const YEAR_MONTHS = 12.0;
fn dead_on_scar(f: FireAt, years: f32, scar: f32) -> f32 {
  if (f.burnt <= 0.0 || abs(years * YEAR_MONTHS - f.age) > SAME_BURN_MONTHS) { return 1.0; }
  return scar / f.burnt;
}
