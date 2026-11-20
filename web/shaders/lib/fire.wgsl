// What fire leaves and does on each square of the live map (web/src/fire.ts), as the core's layers say it, and where it
// lies inside a square: read by the ground's burn scars (burn.wgsl) and by the flames and smoke over the map
// (flames.wgsl), and by the snags standing on them (snags.wgsl). Points are in metres from the map's north-west corner.

struct Fire {
  // A square's side, m; the map's squares across and down; the view drawn (0 the Earth's colours, 1 the true colour).
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
// air this month, kg/m²; the low wind, m/s east and south; the dead walls standing and the cured leaves standing, kg/m².
struct FireAt { burnt: f32, age: f32, flame: f32, length: f32, ash: f32, air: f32, wind: vec2f, snag: f32, cured: f32 }

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
  return FireAt(a.x, a.y / max(a.x, 1e-6), a.z, a.w / max(a.z, 1e-6), b.x, b.y, b.zw, c.x, c.y);
}

// A square's burnt share lies in patches with ragged edges (ragged.wgsl), from `BURN_PATCH_M` across down to the
// pixel: a fire runs where its fuel joins and stops at gaps, so its scar is a mosaic of burnt and spared ground (Turner
// and others 1994). The patches are a share of the ground read off the noise, even over 0..1: a point is burnt where
// that number lies under the burnt share, so the drawn share is the square's at every zoom.
const BURN_PATCH_M = 600.0; const BURN_OCTAVES = 7; const BURN_NOISE = 91;
// The edge of a patch is spread over this much of the number besides the swing of the patches too small to show.
const BURN_EDGE = 0.02;

// The place of point `at` among the patches, 0..1, even, seen at `e` m a pixel; and how far the number swings over the
// patches that do not show there (the spread a share read on it takes).
fn burn_rank(at: vec2f, e: f32) -> vec2f {
  let n = ragged(at, e, BURN_PATCH_M, BURN_OCTAVES, BURN_NOISE);
  return vec2f(smoothstep(-EVEN_ONE, EVEN_ONE, n), BURN_EDGE + 0.25 * ragged_hidden(e, BURN_PATCH_M, BURN_OCTAVES));
}

// The share of a point of rank `r` (`burn_rank`) in the first `share` of the ground.
fn under_share(r: vec2f, share: f32) -> f32 {
  if (share <= 0.0) { return 0.0; }
  return 1.0 - smoothstep(share - r.y, share + r.y, r.x);
}
