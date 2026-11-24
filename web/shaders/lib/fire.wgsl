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
}
@group(0) @binding(25) var fire_tex: texture_2d<f32>;
@group(0) @binding(26) var fire_sampler: sampler;
@group(0) @binding(27) var<uniform> fire: Fire;

// e^x - 1, kept exact for small x.
fn expm1(x: f32) -> f32 { return select(exp(x) - 1.0, x + 0.5 * x * x, abs(x) < 1e-3); }

// What fire did at a point, blended between the squares' centres: the share of the land under a scar (burnt within the
// months the core tells apart, fire.ts), the months since its last burn (the scar's mean), the share burning this
// month, its flames' mean length, m (the burning share's mean), the ash on the ground and the gas the flames sent to the
// air this month, kg/m²; the low wind, m/s east and south; the cured leaves standing, kg/m². (Its third plane's second
// number is the share of its land saturated, `under_share`.)
struct FireAt { burnt: f32, age: f32, flame: f32, length: f32, ash: f32, air: f32, wind: vec2f, cured: f32 }

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
// Ash on the ground covers all but e^-1 of it at this much, kg/m² (about a millimetre of loose ash at 0.2 to 0.4 t/m³:
// Bodí and others 2014).
const ASH_HIDES_KG = 0.25;
// The Earth view darkens the plants standing on a fresh scar towards the char by this much (scorched leaves and
// stems, black with soot; they fade with the char as the months pass).
const SCORCHED_COVER = 0.75;

// The planes of the fire's texture (fire.ts), one under the other.
const FIRE_PLANES = 3.0;

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
  return FireAt(a.x, a.y / max(a.x, 1e-6), a.z, a.w / max(a.z, 1e-6), b.x, b.y, b.zw, c.x);
}

// The months since the burn of the scar at `at`, a point of rank `rank` among the patches (`burn_rank`'s number made
// even, 0..1): of one of the four squares whose centres lie about it, drawn by the rank with the chance of each
// square's blend weight times its scar. So where scars of different ages meet, the point takes one of them, and their meeting is as ragged
// as the patches; a blend of their ages would lay a straight line where the younger one's char fades.
fn scar_age(at: vec2f, rank: f32) -> f32 {
  let s = at / fire.map.x - 0.5;
  let c = vec2i(floor(s));
  let f = fract(s);
  var t: array<vec4f, 4>;
  var w = vec4f((1.0 - f.x) * (1.0 - f.y), f.x * (1.0 - f.y), (1.0 - f.x) * f.y, f.x * f.y);
  for (var k = 0; k < 4; k++) {
    t[k] = fire_texel(0, c + vec2i(k % 2, k / 2));
    w[k] *= t[k].x;
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

// The scar at `at` of what fire did there `f` (`fire_at`), seen at `e` m a pixel: the share of the point under it
// (`under_share`), how much of it is char, ash and scorched soil (0 where none lies there, burn.wgsl `scar_over`).
fn scar_of(f: FireAt, at: vec2f, e: f32) -> vec4f {
  if (f.burnt <= 0.0) { return vec4f(0.0); }
  let rank = burn_rank(at, e);
  let scar = under_share(rank, f.burnt, at);
  if (scar <= 0.0) { return vec4f(0.0); }
  let age = scar_age(at, smoothstep(-EVEN_ONE, EVEN_ONE, rank.x));
  return vec4f(scar, exp(-age / CHAR_MONTHS), -expm1(-f.ash / ASH_HIDES_KG), exp(-age / SCORCHED_MONTHS));
}

// The share of a point under the fresh char of scar `s` (`scar_of`), which scorches the plants standing there.
fn fresh_char(s: vec4f) -> f32 { return s.x * s.y; }

// Plants of colour `rgb` (premultiplied by `a`) standing where `fresh` of the ground is under fresh char
// (`fresh_char`): scorched, as dark as the char under them (`SCORCHED_COVER`), in the field of the cover (burn.wgsl)
// and in its marks (cover.wgsl, decided once for each by mark.wgsl).
fn scorched(rgb: vec3f, a: f32, fresh: f32) -> vec3f { return mix(rgb, CHAR * a, SCORCHED_COVER * fresh); }
