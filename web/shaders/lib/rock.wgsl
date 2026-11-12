// The rock of the live map (web/src/land.ts, tape/src/ground/rock.rs): of each of the terrain's samples the depth of
// its loose ground and the largest stone its channel moves; and how much of the bed shows where the loose ground thins.

// Of each sample (a texel each): the depth of its loose ground, m, and the largest stone its channel moves, m; and in a
// second plane under the first, the reflectance of its bed's make (tape/src/ground/rock.rs).
@group(0) @binding(18) var rock_field: texture_2d<f32>;
@group(0) @binding(19) var rock_sampler: sampler;

// Loose ground hides the bed from this depth on, m, and shows all of it at none: soils thinner than a few decimetres
// show their rock in outcrops (lithic soils are under 0.1 m deep; from memory).
const HIDES_BED_M = 0.1;
// The loose ground's depth departs from the samples' by up to this share over patches `PATCHY_M` across and in
// `PATCHY_OCTAVES` octaves each `PATCHY_STEP` times finer, each weighing `PATCHY_FALL` of the last and fading out as it
// shrinks to a few pixels (draw.wgsl `shows_at`): a mantle is thicker in hollows and thinner on knobs at every
// scale, so the bed shows in outcrops with ragged edges, not along a smooth line.
const PATCHY = 0.6; const PATCHY_M = 40.0; const PATCHY_OCTAVES = 4; const PATCHY_STEP = 2.9; const PATCHY_FALL = 0.6;
// Loose ground deeper than this, m, creeps into smooth swells; thinner, the bed's roughness shows through it
// (relief.wgsl).
const SMOOTH_LOOSE_M = 0.5;

// The depth of the loose ground at `at`, m from the map's north-west corner: through the samples' centres as a cubic
// B-spline (draw.wgsl `bspline`), so no sample's square shows in the bed it bares.
fn loose_at(at: vec2f) -> f32 {
  let size = land.samples.xy;
  let b = bspline(at / (land.map.x * land.map.yz) * size - 0.5, size);
  let tap = vec4f(
    textureSampleLevel(rock_field, rock_sampler, rock_plane(b.h0, 0.0), 0.0).x, textureSampleLevel(rock_field, rock_sampler, rock_plane(vec2f(b.h1.x, b.h0.y), 0.0), 0.0).x,
    textureSampleLevel(rock_field, rock_sampler, rock_plane(vec2f(b.h0.x, b.h1.y), 0.0), 0.0).x, textureSampleLevel(rock_field, rock_sampler, rock_plane(b.h1, 0.0), 0.0).x);
  return b.g0.y * (b.g0.x * tap.x + b.g1.x * tap.y) + b.g1.y * (b.g0.x * tap.z + b.g1.x * tap.w);
}

// The place in the rock's texture of the point `uv` (0..1 over the samples) of its plane `plane` (0 the loose ground's,
// 1 the bed's make), held within the plane's own rows so no blend reads the other plane.
fn rock_plane(uv: vec2f, plane: f32) -> vec2f {
  let down = land.samples.y;
  return vec2f(uv.x, (clamp(uv.y * down, 0.5, down - 0.5) + plane * down) / (2.0 * down));
}

// The reflectance of the bed's make at `at`, m from the map's north-west corner, blended between the samples' centres:
// so bare rock shows the bands the folded layers make (core/src/space/strata.rs).
fn bed_make_at(at: vec2f) -> vec3f {
  return textureSampleLevel(rock_field, rock_sampler, rock_plane(at / (land.map.x * land.map.yz), 1.0), 0.0).xyz;
}

// The largest stone the channel of sample `s` moves, m (0 for none, `s.x < 0`).
fn stone_of(s: vec2i) -> f32 {
  if (s.x < 0) { return 0.0; }
  return textureLoad(rock_field, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).y;
}

// The depth of the loose ground at `at`, m, seen at `e` m a pixel, with its patches (`PATCHY`): the field every edge of
// bare bed is drawn from, the ground's and the plants' alike.
fn loose_seen(at: vec2f, e: f32) -> f32 {
  let mean = loose_at(at);
  // Where the thinnest the patches make it is deeper than shows the bed or its roughness, they change nothing drawn
  // (and no noise is read).
  if (mean * (1.0 - PATCHY) >= max(HIDES_BED_M, SMOOTH_LOOSE_M)) { return mean; }
  var n = 0.0;
  var size = PATCHY_M;
  var weight = 1.0;
  var sum = 0.0;
  for (var o = 0; o < PATCHY_OCTAVES; o++) {
    let shown = shows_at(size, e);
    // An octave too small to show reads no noise.
    if (shown > 0.0) { n += weight * shown * noise_turned(at / size + f32(o) * 7.3, 55 + o); }
    sum += weight;
    size /= PATCHY_STEP;
    weight *= PATCHY_FALL;
  }
  return mean * (1.0 + PATCHY * clamp(2.0 * n / sum, -1.0, 1.0));
}

// The share of the bed that shows through `loose` m of loose ground (`loose_seen`).
fn bed_shows(loose: f32) -> f32 { return 1.0 - ease(0.0, HIDES_BED_M, loose); }
