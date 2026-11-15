// The Earth view's fine detail of the ground, by habitat and season (colour.wgsl `ground_of`): the snow's drifts, crust
// and thin cover, the bare soil's pebbles, cracks and ripples, and the forest floor's carpet. Each detail is a
// zero-mean change over the ground's far tone (a stone's lit side balances its shaded side, noise has no mean), and
// fades into that tone as a pixel grows past it (`shows_at`), so the mean of the near picture is the far picture.
// Fine places come from the lattice of the pixel (draw.wgsl `lat`), never from a float position.

// The wind that lays drifts and ripples blows along this angle (radians from the world's east) everywhere.
const WIND = 0.5;

// The lattice of cells `long` by `short` m, their long side along the wind.
fn elongated(long: f32, short: f32, off: vec2f) -> Lat {
  let c = cos(WIND); let s = sin(WIND);
  return lat(mat2x2f(c / long, -s / short, s / long, c / short), off);
}

// A pebble in cell `l` (its lattice turned by `turn` radians) of stream k with probability `fill`, its radius between `radius` (cell widths, 0.3 at most): its
// own tone and a lit side towards the light, as a multiplier of the ground's colour (1 where none lies).
fn pebbles(l: Lat, turn: f32, k: i32, fill: f32, radius: vec2f) -> f32 {
  let h = lowbias(l.i, k);
  let u = vec4f(f32(h & 255u), f32((h >> 8u) & 255u), f32((h >> 16u) & 255u), f32(h >> 24u)) / 255.0;
  if (u.w >= fill) { return 1.0; }
  let d = (l.f - (vec2f(0.5) + (u.xy - 0.5) * 0.4)) / mix(radius.x, radius.y, u.z * u.z);
  let body = 1.0 - smoothstep(0.75, 1.0, length(d));
  let own = fract(u.x * 7.3 + u.y * 3.1) - 0.5;
  return 1.0 + body * (PEBBLE_TONE * own + PEBBLE_LIT * dot(d, mat2x2f(cos(turn), sin(turn), -sin(turn), cos(turn)) * vec2f(-0.7, -0.7)));
}
// A pebble's tone differs by up to this share over the ground, and its lit side by this much more.
const PEBBLE_TONE = 1.0; const PEBBLE_LIT = 0.5;

// Snow: its cover breaks where it is thin (ground shows through, in patches `SNOW_BREAK_M` across, the less snow the
// more), and its tone has drifts laid along the wind (`SNOW_DRIFT_M` by `SNOW_DRIFT_W`, `SNOW_DRIFT` deep) and a crust
// of grains `SNOW_CRUST_M` across (`SNOW_CRUST` deep), the shaded part bluer. Snow deeper than `SNOW_THIN_M` is whole.
const SNOW_THIN_M = 0.25; const SNOW_BREAK_M = vec2f(0.45, 2.4); const SNOW_BREAKS = 0.85; const SNOW_BREAK_MEAN = 0.3;
const SNOW_DRIFT_M = 7.0; const SNOW_DRIFT_W = 1.5; const SNOW_DRIFT = 0.12;
// Sastrugi: crests `SNOW_RIDGE_M` long and `SNOW_RIDGE_W` apart, `SNOW_RIDGE` deep (a crest's mean height is
// `RIDGE_MEAN`), and swells `SNOW_SWELL_M` across, `SNOW_SWELL` deep.
const SNOW_RIDGE_M = 3.0; const SNOW_RIDGE_W = 0.5; const SNOW_RIDGE = 0.16; const RIDGE_MEAN = 0.64;
const SNOW_SWELL_M = 45.0; const SNOW_SWELL = 0.07;
const SNOW_CRUST_M = 0.16; const SNOW_CRUST = 0.06;

// The share of the ground that `snow` covers, with ground showing through where the snow of depth `snow_m` is thin.
fn snow_cover(snow: f32, snow_m: f32, e: f32) -> f32 {
  let thin = (1.0 - ease(0.02, SNOW_THIN_M, snow_m)) * SNOW_BREAKS * STYLE;
  if (snow <= 0.0 || thin <= 0.0) { return snow; }
  var open = SNOW_BREAK_MEAN;
  let near = shows_at(SNOW_BREAK_M.x, e);
  if (near > 0.0) {
    let n = noise_l(lat_turned(SNOW_BREAK_M.x, vec2f(0.0)), 81).x + 0.5 * noise_l(lat_axes(SNOW_BREAK_M.y, vec2f(0.0)), 82).x;
    open = mix(open, ease(0.1, 0.6, n), near);
  }
  return snow * (1.0 - thin * open);
}

// The colour of the snow at the pixel seen at `e` m a pixel: its drifts and crust.
fn snow_look(e: f32) -> vec3f {
  var n = 0.0;
  let drift = shows_at(SNOW_DRIFT_W, e);
  if (drift > 0.0) { n += SNOW_DRIFT * noise_l(elongated(SNOW_DRIFT_M, SNOW_DRIFT_W, vec2f(0.0)), 83).x * drift; }
  let ridges = shows_at(SNOW_RIDGE_W, e);
  if (ridges > 0.0) { n += SNOW_RIDGE * (1.0 - abs(noise_l(elongated(SNOW_RIDGE_M, SNOW_RIDGE_W, vec2f(0.0)), 100).x) - RIDGE_MEAN) * ridges; }
  let swell = shows_at(SNOW_SWELL_M * 0.1, e);
  if (swell > 0.0) { n += SNOW_SWELL * noise_l(lat_turned(SNOW_SWELL_M, vec2f(0.0)), 101).x * swell; }
  let crust = shows_at(SNOW_CRUST_M, e);
  if (crust > 0.0) { n += SNOW_CRUST * noise_l(lat_axes(SNOW_CRUST_M, vec2f(0.0)), 84).x * crust; }
  return SNOW * (1.0 + STYLE * n * vec3f(1.0, 1.0, 0.7));
}

// Bare soil: pebbles of three sizes (more where the loose ground is thin), cracks where it is dry and fine, ripples laid
// by the wind on loose dry ground and sand, and mats of lichen where it is not dry. Each `*_M` is a cell's width, m.
const PEBBLE_FINE_M = 0.22; const PEBBLE_M = 0.7; const ROCK_M = 2.4;
const DRY_CRACK_M = 1.1; const DRY_CRACK_W = 0.07; const DRY_CRACK_DARK = 0.45; const DRY_CRACK_MEAN = 0.05;
const RIPPLE_M = vec2f(1.3, 0.3); const RIPPLE = 0.07;
// Boulders `BOULDER_M` apart, soil's tone mottled by `MOTTLE_SOIL` over patches `MOTTLE_SOIL_M` across (the drier tint
// warm), and dunes laid along the wind `DUNE_M` long and wide, `DUNE` deep.
const BOULDER_M = 9.0; const MOTTLE_SOIL = 0.08; const MOTTLE_SOIL_M = vec2f(2.8, 11.0); const DUNE_M = vec2f(16.0, 3.5); const DUNE = 0.1;
const MAT_M = 0.9; const MAT_TINT = vec3f(0.92, 1.08, 0.82); const MAT = 0.35;

// The multiplier of soil's colour at the pixel: soaked `wet`, `loose` m of loose ground deep, `sand` of it beach.
fn soil_detail(e: f32, wet: f32, loose: f32, sand: f32) -> vec3f {
  let dry = 1.0 - wet;
  let thin = 1.0 - ease(0.0, 2.0, loose);
  var out = vec3f(1.0);
  let fine = shows_at(PEBBLE_FINE_M * 0.3, e);
  if (fine > 0.0) { out *= mix(1.0, pebbles(lat_axes(PEBBLE_FINE_M, vec2f(0.0)), 0.0, 85, mix(0.5, 0.8, thin), vec2f(0.12, 0.28)), fine); }
  let mid = shows_at(PEBBLE_M * 0.3, e);
  if (mid > 0.0) { out *= mix(1.0, pebbles(lat_turned(PEBBLE_M, vec2f(0.0)), TURN_A, 86, mix(0.25, 0.6, thin), vec2f(0.1, 0.3)), mid); }
  let big = shows_at(ROCK_M * 0.2, e);
  if (big > 0.0) { out *= mix(1.0, pebbles(lat_turned(ROCK_M, vec2f(0.3)), TURN_A, 87, mix(0.04, 0.3, thin), vec2f(0.12, 0.3)), big); }
  let boulders = shows_at(BOULDER_M * 0.15, e);
  if (boulders > 0.0) { out *= mix(1.0, pebbles(lat_axes(BOULDER_M, vec2f(0.4)), 0.0, 96, mix(0.1, 0.4, thin), vec2f(0.1, 0.3)), boulders); }
  let mottle = shows_at(MOTTLE_SOIL_M.x, e);
  if (mottle > 0.0) { out *= 1.0 + MOTTLE_SOIL * (noise_l(lat_turned(MOTTLE_SOIL_M.x, vec2f(0.0)), 97).x + noise_l(lat_axes(MOTTLE_SOIL_M.y, vec2f(0.0)), 98).x) * mottle * vec3f(1.0, 0.95, 0.85); }
  let dunes = shows_at(DUNE_M.y, e);
  if (dunes > 0.0) { out *= 1.0 + DUNE * ease(0.0, 1.0, 1.0 - thin) * dry * noise_l(elongated(DUNE_M.x, DUNE_M.y, vec2f(0.0)), 99).x * dunes; }
  let cracks = shows_at(DRY_CRACK_M * DRY_CRACK_W * 4.0, e);
  if (cracks > 0.0) {
    let c = crack(noise_l(lat_turned(DRY_CRACK_M, vec2f(0.7)), 88).x, DRY_CRACK_W);
    out *= 1.0 - DRY_CRACK_DARK * ease(0.5, 0.2, wet) * (1.0 - sand) * (c - DRY_CRACK_MEAN) * cracks;
  }
  let ripples = shows_at(RIPPLE_M.y, e);
  if (ripples > 0.0) { out *= 1.0 + RIPPLE * ease(0.0, 1.0, max(sand, 1.0 - thin)) * dry * noise_l(elongated(RIPPLE_M.x, RIPPLE_M.y, vec2f(0.0)), 89).x * ripples; }
  let mats = shows_at(MAT_M, e);
  if (mats > 0.0) { out *= mix(vec3f(1.0), MAT_TINT, MAT * thin * ease(0.2, 0.6, wet) * mats * ease(0.0, 0.5, noise_l(lat_axes(MAT_M, vec2f(0.0)), 90).x)); }
  return mix(vec3f(1.0), out, STYLE);
}

// The forest floor under `canopy` of closed crowns: a carpet over its litter, in the leaf's colour `leaf` (the crowns'
// as the season turns them, so the floor is green in leaf, and yellow, red and brown as the leaves fall): patches of
// undergrowth and fern `FERN_M` across (more where wet), leaves in grains of `LEAF_M`, fallen trunks `LOG_M` long, and
// flowers in the undergrowth.
const FERN_M = 2.6; const FERN_FINE_M = 0.45; const FERN_TINT = vec3f(0.6, 1.05, 0.5); const FERN = 0.55;
const LEAF_M = 0.14; const LEAF_TONE = 0.35; const LEAF_SHARE = 0.55; const LEAF_LIGHT = 0.85;
const LOG_M = vec2f(6.0, 0.7); const LOG_FROM = 0.55; const LOG_COLOUR = vec3f(0.55, 0.42, 0.32);
const LOG_MEAN = 0.02; const LOGS = 0.55;
const BLOOM_M = 0.3; const BLOOM_SHARE = 0.07; const BLOOMS = 0.8;

// The tint of the litter: the leaf's hue at the litter's lightness (`LEAF_LIGHT`), `LEAF_SHARE` of the way from the
// bare litter's.
fn litter_tint(leaf: vec3f) -> vec3f { return mix(LITTER_TINT, LEAF_LIGHT * leaf / max(dot(leaf, LUMA), 1e-3), LEAF_SHARE); }

// The forest floor's colour `out` (litter under `canopy`) with its carpet at the pixel.
fn carpet(out: vec3f, e: f32, canopy: f32, wet: f32, leaf: vec3f) -> vec3f {
  var c = out;
  let k = STYLE * canopy;
  let undergrowth = ease(0.0, 0.6, noise_l(lat_turned(FERN_M, vec2f(0.0)), 91).x) * (0.35 + 0.65 * wet);
  let grain = shows_at(LEAF_M, e);
  if (grain > 0.0) { c *= 1.0 + k * LEAF_TONE * noise_l(lat_axes(LEAF_M, vec2f(0.0)), 92).x * grain; }
  let fronds = shows_at(FERN_FINE_M, e);
  var green = FERN * undergrowth * k;
  if (fronds > 0.0) { green *= 1.0 + fronds * noise_l(lat_turned(FERN_FINE_M, vec2f(0.0)), 93).x; }
  c = mix(c, c * FERN_TINT * 1.3, clamp(green, 0.0, 1.0));
  let logs = shows_at(LOG_M.y, e);
  var trunk = LOG_MEAN;
  if (logs > 0.0) { trunk = mix(LOG_MEAN, ease(LOG_FROM, LOG_FROM + 0.2, noise_l(elongated(LOG_M.x, LOG_M.y, vec2f(0.0)), 94).x), logs); }
  c = mix(c, c * LOG_COLOUR * 1.6, LOGS * k * trunk);
  let bloom = shows_at(BLOOM_M, e);
  if (bloom > 0.0) {
    let l = lat_axes(BLOOM_M, vec2f(0.0));
    let h = lowbias(l.i, 95);
    let on = step(f32(h & 255u) / 255.0, BLOOM_SHARE * undergrowth) * (1.0 - smoothstep(0.1, 0.2, length(l.f - 0.5)));
    let petal = select(vec3f(1.0, 0.95, 0.5), vec3f(0.85, 0.7, 1.0), ((h >> 8u) & 1u) == 1u);
    c = mix(c, petal * dot(c, LUMA) * 3.0, BLOOMS * on * k * bloom);
  }
  return c;
}
