// The rock of the live map (web/src/land.ts, tape/src/ground/rock.rs): of each of the terrain's samples the depth of
// its loose ground and the largest stone its channel moves; and how much of the bed shows where the loose ground thins.

// Of each sample (a texel each): the depth of its loose ground, m, and the largest stone its channel moves, m.
@group(0) @binding(18) var rock_field: texture_2d<f32>;
@group(0) @binding(19) var rock_sampler: sampler;

// Loose ground hides the bed from this depth on, m, and shows all of it at none: soils thinner than a few decimetres
// show their rock in outcrops (lithic soils are under 0.1 m deep; from memory).
const HIDES_BED_M = 0.1;
// The loose ground's depth departs from the samples' by up to this share over patches `PATCHY_M` across: a mantle is
// thicker in hollows and thinner on knobs, so the bed shows in outcrops, not along a smooth line.
const PATCHY = 0.5; const PATCHY_M = 40.0;

// The depth of the loose ground at `at`, m from the map's north-west corner: through the samples' centres as a cubic
// B-spline (draw.wgsl `bspline`), so no sample's square shows in the bed it bares.
fn loose_at(at: vec2f) -> f32 {
  let size = vec2f(textureDimensions(rock_field));
  let b = bspline(at / (land.map.x * land.map.yz) * size - 0.5, size);
  let tap = vec4f(
    textureSampleLevel(rock_field, rock_sampler, b.h0, 0.0).x, textureSampleLevel(rock_field, rock_sampler, vec2f(b.h1.x, b.h0.y), 0.0).x,
    textureSampleLevel(rock_field, rock_sampler, vec2f(b.h0.x, b.h1.y), 0.0).x, textureSampleLevel(rock_field, rock_sampler, b.h1, 0.0).x);
  return b.g0.y * (b.g0.x * tap.x + b.g1.x * tap.y) + b.g1.y * (b.g0.x * tap.z + b.g1.x * tap.w);
}

// The largest stone the channel of sample `s` moves, m (0 for none, `s.x < 0`).
fn stone_of(s: vec2i) -> f32 {
  if (s.x < 0) { return 0.0; }
  return textureLoad(rock_field, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).y;
}

// The share of the bed that shows at `at` through `loose` m of loose ground on the mean (`PATCHY`).
fn bed_shows(loose: f32, at: vec2f) -> f32 {
  // Deeper than the thinnest a patch makes it reach the bed, none shows (and the noise need not be read).
  if (loose * (1.0 - PATCHY) >= HIDES_BED_M) { return 0.0; }
  return 1.0 - ease(0.0, HIDES_BED_M, loose * (1.0 + PATCHY * noise(at / PATCHY_M, 55).x));
}
