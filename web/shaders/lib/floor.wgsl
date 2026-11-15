// The Earth view's detail of the ground, by habitat and season (colour.wgsl `ground_of`, cover_field.wgsl `sward`):
// the snow's drifts, lee shade, sastrugi, rocks and crust; the bare soil's tones, stones, washes, cracks and ripples;
// the forest floor's carpet; the sward's tufts. Detail is strong but keeps the ground's far tone as its mean: what
// darkens (a stone's shadow, a crack) is balanced by a lift of the rest, and each detail fades into that mean as a
// pixel grows past it (`shows_at`). Light comes from the north-west, as the relief's (draw.wgsl `LIGHT`): a stone has
// a lit side, a shaded side, a cast shadow away from the sun and a dark line at its foot. Fine places come from the
// lattice of the pixel (draw.wgsl `lat`), never from a float position.

// The wind that lays drifts, ripples and dunes blows along this angle (radians from the world's east) everywhere.
const WIND = 0.5;

// The lattice of cells `long` by `short` m, their long side along the wind.
fn elongated(long: f32, short: f32, off: vec2f) -> Lat {
  let c = cos(WIND); let s = sin(WIND);
  return lat(mat2x2f(c / long, -s / short, s / long, c / short), off);
}

// The direction away from the sun (east, south) in the lattice turned by `turn` radians.
fn away_in(turn: f32) -> vec2f { return mat2x2f(cos(turn), sin(turn), -sin(turn), cos(turn)) * vec2f(0.7071); }

// A stone in cell `l` (its lattice turned by `turn`) of stream k, present with probability `fill`, its radius from
// `radius.x` to `radius.y` cell widths (at most 0.2: its shadow stays in the cell): x the multiplier of the ground's
// colour (its own tone, its lit side, the cast shadow and the foot's dark line, the rest lifted to keep the mean), y how
// much of the point is the stone (0..1).
const BODY_TONE = 0.8; const BODY_LIT = 0.55; const SHADOW_DARK = 0.5; const RING_DARK = 0.45;
fn stone(l: Lat, turn: f32, k: i32, fill: f32, radius: vec2f) -> vec2f {
  let span = radius.y - radius.x;
  let mean_r2 = radius.x * radius.x + 2.0 * radius.x * span / 3.0 + span * span / 5.0;
  let lift = 1.0 / (1.0 - fill * PI * mean_r2 * 0.5 * (SHADOW_DARK + RING_DARK));
  let h = lowbias(l.i, k);
  let u = vec4f(f32(h & 255u), f32((h >> 8u) & 255u), f32((h >> 16u) & 255u), f32(h >> 24u)) / 255.0;
  if (u.w >= fill) { return vec2f(lift, 0.0); }
  let r = mix(radius.x, radius.y, u.z * u.z);
  let c = vec2f(0.25) + u.xy * 0.25;
  let away = away_in(turn);
  let d = (l.f - c) / r;
  let far = length(d);
  let body = 1.0 - smoothstep(0.8, 1.0, far);
  let own = fract(u.x * 7.3 + u.y * 3.1) - 0.5;
  let tone = (1.0 + BODY_TONE * own) * (1.0 + BODY_LIT * dot(d, -away)) * (1.0 + 0.2 * (1.0 - far * far));
  let shadow = (1.0 - smoothstep(0.75, 1.0, length((l.f - c - away * r) / (0.95 * r)))) * (1.0 - body);
  let foot = smoothstep(0.9, 1.0, far) * (1.0 - smoothstep(1.05, 1.25, far));
  let around = (1.0 - SHADOW_DARK * shadow) * (1.0 - RING_DARK * foot);
  return vec2f(mix(around, tone, body) * lift, body);
}

// Snow: its cover breaks where it is thin (ground shows through, in patches `SNOW_BREAK_M` across, the less snow the
// more); its tone has swells of `SNOW_SWELL_M`, drifts laid along the wind (`SNOW_DRIFT_M` by `SNOW_DRIFT_W`) whose lee
// side is shaded blue (`LEE_TINT`), sastrugi crests `SNOW_RIDGE_M` long and `SNOW_RIDGE_W` apart, a crust of grains
// `SNOW_CRUST_M` across with sparkles; rocks `SNOW_ROCK_M` apart stand through it, capped by snow on their sunward
// side, a cast shadow and a dark foot (the thinner the snow, the more of them). Each mean is held by a lift.
const SNOW_THIN_M = 0.25; const SNOW_BREAK_M = vec2f(0.45, 2.4); const SNOW_BREAKS = 0.85; const SNOW_BREAK_MEAN = 0.3;
const SNOW_DRIFT_M = 22.0; const SNOW_DRIFT_W = 5.0; const LEE_TINT = vec3f(0.62, 0.74, 1.0); const LEE = 0.6; const LEE_MEAN = 0.1;
const SNOW_RIDGE_M = 3.0; const SNOW_RIDGE_W = 0.5; const SNOW_RIDGE = 0.3; const RIDGE_MEAN = 0.64;
const SNOW_SWELL_M = 45.0; const SNOW_SWELL = 0.1; const SNOW_CRUST_M = 0.16; const SNOW_CRUST = 0.07;
const SPARKLE_M = 0.05; const SPARKLE = 0.02; const SNOW_ROCK_M = 7.0; const SNOW_ROCK_DEEP_M = 1.2;
const ROCK_COLOUR = vec3f(0.3, 0.27, 0.25);

// The share of the ground that `snow` covers, with ground showing through where the snow of depth `snow_m` is thin.
fn snow_cover(snow: f32, snow_m: f32, e: f32) -> f32 {
  let thin = (1.0 - ease(0.02, SNOW_THIN_M, snow_m)) * SNOW_BREAKS;
  if (snow <= 0.0 || thin <= 0.0) { return snow; }
  var open = SNOW_BREAK_MEAN;
  let near = shows_at(SNOW_BREAK_M.x, e);
  if (near > 0.0) {
    let n = noise_l(lat_turned(SNOW_BREAK_M.x, vec2f(0.0)), 81).x + 0.5 * noise_l(lat_axes(SNOW_BREAK_M.y, vec2f(0.0)), 82).x;
    open = mix(open, ease(0.1, 0.6, n), near);
  }
  return snow * (1.0 - thin * open);
}

// The colour of the snow of depth `snow_m` at the pixel seen at `e` m a pixel.
fn snow_look(e: f32, snow_m: f32) -> vec3f {
  var tone = 1.0;
  var rgb = SNOW;
  let drift = shows_at(SNOW_DRIFT_W, e);
  if (drift > 0.0) {
    let n = noise_l(elongated(SNOW_DRIFT_M, SNOW_DRIFT_W, vec2f(0.0)), 83).x;
    rgb = mix(rgb, rgb * LEE_TINT, LEE * ease(0.0, -0.7, n) * drift);
    tone += 0.12 * n * drift + LEE_MEAN * LEE * (1.0 - LEE_TINT.r) * drift;
  }
  let ridges = shows_at(SNOW_RIDGE_W, e);
  if (ridges > 0.0) { tone += SNOW_RIDGE * (1.0 - abs(noise_l(elongated(SNOW_RIDGE_M, SNOW_RIDGE_W, vec2f(0.0)), 100).x) - RIDGE_MEAN) * ridges; }
  let swell = shows_at(SNOW_SWELL_M * 0.1, e);
  if (swell > 0.0) { tone += SNOW_SWELL * noise_l(lat_turned(SNOW_SWELL_M, vec2f(0.0)), 101).x * swell; }
  let crust = shows_at(SNOW_CRUST_M, e);
  if (crust > 0.0) { tone += SNOW_CRUST * noise_l(lat_axes(SNOW_CRUST_M, vec2f(0.0)), 84).x * crust; }
  let sparkle = shows_at(SPARKLE_M, e);
  if (sparkle > 0.0) {
    let l = lat_axes(SPARKLE_M, vec2f(0.0));
    tone += 0.5 * step(f32(lowbias(l.i, 102) & 1023u) / 1023.0, SPARKLE) * (1.0 - smoothstep(0.2, 0.5, length(l.f - 0.5))) * sparkle;
  }
  rgb *= tone;
  let rocks = shows_at(SNOW_ROCK_M * 0.12, e) * (1.0 - ease(0.3, SNOW_ROCK_DEEP_M, snow_m));
  if (rocks > 0.0) {
    let s = stone(lat_turned(SNOW_ROCK_M, vec2f(0.2)), TURN_A, 103, 0.35, vec2f(0.08, 0.2));
    let cap = ease(0.1, 0.9, s.x - 0.8);
    rgb = mix(rgb, mix(rgb * s.x, mix(ROCK_COLOUR * s.x, SNOW * 1.05, cap), s.y), rocks);
  }
  return rgb;
}

// Bare soil: tones from pale sand through ochre to rust over patches of 60, 11 and 2.8 m, stones of four sizes (more
// where the loose ground is thin), dry washes with pale beds and dark banks, cracks where it is dry and fine, wind
// ripples and dunes on loose dry ground, bleached wood, and mats of lichen where it is not dry. `*_M` is a cell's width.
const SOIL_PALE = vec3f(1.25, 1.15, 1.0); const SOIL_RUST = vec3f(0.72, 0.64, 0.56);
const TONES_M = vec3f(60.0, 11.0, 2.8); const TONES = vec3f(0.9, 0.7, 0.5);
const PEBBLE_FINE_M = 0.3; const PEBBLE_M = 1.0; const ROCK_M = 3.5; const BOULDER_M = 11.0;
const WASH_M = vec2f(14.0, 3.2); const WASH_W = 0.035; const WASH_PALE = 0.35; const WASH_BANK = 0.4; const WASH_MEAN = 0.04;
const DRY_CRACK_M = 1.1; const DRY_CRACK_W = 0.07; const DRY_CRACK_DARK = 0.45; const DRY_CRACK_MEAN = 0.05;
const RIPPLE_M = vec2f(1.3, 0.3); const RIPPLE = 0.12; const DUNE_M = vec2f(16.0, 3.5); const DUNE = 0.14;
const BLEACH_M = vec2f(3.2, 0.4); const BLEACH = 0.8;
const MAT_M = 0.9; const MAT_TINT = vec3f(0.92, 1.08, 0.82); const MAT = 0.45;

// Soil's tone at `n` (-1..1) of a patch: pale sand above, rust below.
fn soil_tone(n: f32) -> vec3f { return mix(mix(vec3f(1.0), SOIL_RUST, ease(0.0, -0.7, n)), SOIL_PALE, ease(0.0, 0.7, n)); }

// A dry wash or rill of cells `m` across: x its pale bed, y its dark banks (the zero crossings of a meandering noise).
fn wash(m: f32, k: i32) -> vec2f {
  let warp = vec2f(noise_l(lat_axes(4.0 * m, vec2f(0.0)), k + 1).x, noise_l(lat_axes(4.0 * m, vec2f(5.0)), k + 2).x) * 0.8;
  let a = abs(noise_l(lat_turned(m, warp), k).x);
  return vec2f(1.0 - smoothstep(0.0, WASH_W, a), smoothstep(WASH_W, 2.2 * WASH_W, a) * (1.0 - smoothstep(2.2 * WASH_W, 3.5 * WASH_W, a)));
}

// Shores and beds: within `SHORE_M` m of the liquid's level, and in a channel's bed (`stony`), the ground is gravel:
// stones thicker (`GRAVEL_FILL`), paler (`GRAVEL_TINT`), with driftwood on the shore.
const SHORE_M = 1.5; const GRAVEL_FILL = vec4f(0.75, 0.6, 0.35, 0.12); const GRAVEL_TINT = vec3f(1.12, 1.08, 1.0);

// The multiplier of soil's colour at the pixel: soaked `wet`, `loose` m of loose ground deep, `sand` of it beach, `over`
// m over the nearest liquid's level, `stony` of it a channel's stony bed.
fn soil_detail(e: f32, wet: f32, loose: f32, sand: f32, over: f32, stony: f32) -> vec3f {
  let gravel = max(ease(SHORE_M, 0.0, over) * ease(0.0, 0.5, loose), stony);
  let dry = 1.0 - wet;
  let thin = 1.0 - ease(0.0, 2.0, loose);
  var out = vec3f(1.0);
  let a = shows_at(TONES_M.x * 0.1, e);
  if (a > 0.0) { out *= mix(vec3f(1.0), soil_tone(noise_l(lat_turned(TONES_M.x, vec2f(0.0)), 97).x), TONES.x * a); }
  let b = shows_at(TONES_M.y * 0.1, e);
  if (b > 0.0) { out *= mix(vec3f(1.0), soil_tone(noise_l(lat_axes(TONES_M.y, vec2f(0.0)), 98).x), TONES.y * b); }
  let c = shows_at(TONES_M.z * 0.1, e);
  if (c > 0.0) { out *= mix(vec3f(1.0), soil_tone(noise_l(lat_turned(TONES_M.z, vec2f(0.3)), 99).x), TONES.z * c); }
  let sizes = vec4f(PEBBLE_FINE_M, PEBBLE_M, ROCK_M, BOULDER_M);
  let fills = mix(mix(vec4f(0.5, 0.3, 0.12, 0.08), vec4f(0.8, 0.6, 0.35, 0.3), thin), GRAVEL_FILL, gravel);
  out *= mix(vec3f(1.0), GRAVEL_TINT, gravel);
  // Each size lies on a lattice, the small ones on two turned from each other (a second, thinner one), so no rows show.
  for (var k = 0; k < 6; k++) {
    let size = sizes[k % 4];
    let shown = shows_at(size * 0.16, e);
    if (shown > 0.0) {
      let turned = ((k & 1) == 1) != (k > 3);
      var l = lat_axes(size, vec2f(0.17 * f32(k)));
      var turn = 0.0;
      if (turned) { turn = TURN_A; l = lat_turned(size, vec2f(0.17 * f32(k))); }
      out *= mix(1.0, stone(l, turn, 85 + k, fills[k % 4] * select(1.0, 0.6, k > 3), vec2f(0.06, 0.2)).x, shown);
    }
  }
  let wide = shows_at(WASH_M.y * 0.5, e);
  if (wide > 0.0) {
    let w = wash(WASH_M.x, 110) + 0.7 * wash(WASH_M.y, 113);
    out *= 1.0 + dry * wide * (WASH_PALE * (w.x - WASH_MEAN) - WASH_BANK * (w.y - WASH_MEAN));
  }
  let cracks = shows_at(DRY_CRACK_M * DRY_CRACK_W * 4.0, e);
  if (cracks > 0.0) {
    let cr = crack(noise_l(lat_turned(DRY_CRACK_M, vec2f(0.7)), 88).x, DRY_CRACK_W);
    out *= 1.0 - DRY_CRACK_DARK * ease(0.5, 0.2, wet) * (1.0 - sand) * (cr - DRY_CRACK_MEAN) * cracks;
  }
  let ripples = shows_at(RIPPLE_M.y, e);
  if (ripples > 0.0) { out *= 1.0 + RIPPLE * (1.0 - thin) * dry * noise_l(elongated(RIPPLE_M.x, RIPPLE_M.y, vec2f(0.0)), 89).x * ripples; }
  let dunes = shows_at(DUNE_M.y, e);
  if (dunes > 0.0) { out *= 1.0 + DUNE * (1.0 - thin) * dry * noise_l(elongated(DUNE_M.x, DUNE_M.y, vec2f(0.0)), 105).x * dunes; }
  let wood = shows_at(BLEACH_M.y, e);
  if (wood > 0.0) { out *= 1.0 + BLEACH * max(thin, ease(SHORE_M, 0.0, over)) * dry * ease(0.62, 0.8, 1.0 - abs(noise_l(elongated(BLEACH_M.x, BLEACH_M.y, vec2f(0.5)), 106).x)) * wood; }
  let mats = shows_at(MAT_M, e);
  if (mats > 0.0) { out *= mix(vec3f(1.0), MAT_TINT, MAT * thin * ease(0.2, 0.6, wet) * mats * ease(0.0, 0.5, noise_l(lat_axes(MAT_M, vec2f(0.0)), 90).x)); }
  return out;
}

// The forest floor under `canopy` of closed crowns: a carpet over its litter, in the leaf's colour `leaf` (the crowns'
// as the season turns them: green in leaf, yellow, red and brown as the leaves fall): patches of undergrowth `FERN_M`
// across (more where wet) with fern rosettes and moss cushions, leaf grains of `LEAF_M` each its own shade, fallen
// trunks `LOG_M` long, mossy, and drifts of flowers.
const FERN_M = 4.0; const FERN_FINE_M = 0.7; const FERN_TINT = vec3f(0.55, 1.1, 0.45); const FERN = 0.8;
const CUSHION_M = 1.3; const CUSHION_TINT = vec3f(0.55, 1.2, 0.5);
const LEAF_M = 0.14; const LEAF_TONE = 0.55; const LEAF_SHARE = 0.55; const LEAF_LIGHT = 0.85;
const LOG_M = vec2f(6.0, 0.7); const LOG_FROM = 0.5; const LOG_COLOUR = vec3f(0.5, 0.38, 0.28); const LOG_MEAN = 0.03; const LOGS = 0.75;
const BLOOM_M = 0.22; const BLOOM_DRIFT_M = 7.0; const BLOOMS = 0.9;

// The tint of the litter: the leaf's hue at the litter's lightness, `LEAF_SHARE` of the way from the bare litter's.
fn litter_tint(leaf: vec3f) -> vec3f { return mix(LITTER_TINT, LEAF_LIGHT * leaf / max(dot(leaf, LUMA), 1e-3), LEAF_SHARE); }

// The forest floor's colour `out` (litter under `canopy`) with its carpet at the pixel.
fn carpet(out: vec3f, e: f32, canopy: f32, wet: f32, leaf: vec3f) -> vec3f {
  var c = out;
  let k = canopy;
  let grown = ease(0.0, 0.6, noise_l(lat_turned(FERN_M, vec2f(0.0)), 91).x) * (0.35 + 0.65 * wet);
  let grain = shows_at(LEAF_M, e);
  if (grain > 0.0) {
    let l = lat_axes(LEAF_M, vec2f(0.0));
    let shade = f32(lowbias(l.i, 92) & 255u) / 255.0 - 0.5;
    c *= 1.0 + k * grain * LEAF_TONE * (noise_l(l, 92).x + shade) * vec3f(1.0, 0.9, 0.6);
  }
  var green = FERN * grown * k;
  let rosette = shows_at(FERN_FINE_M * 0.3, e);
  if (rosette > 0.0) {
    let f = fan(lat_turned(FERN_FINE_M, vec2f(0.0)), 93, 0.7, vec2f(0.2, 0.35), 8.0);
    green *= mix(1.0, 0.35 + f.x * 1.3, rosette);
    c *= mix(vec3f(1.0), vec3f(0.8 + 0.5 * f.y), rosette * f.x * grown * k);
  }
  c = mix(c, c * FERN_TINT * 1.35, clamp(green, 0.0, 1.0));
  let cushion = shows_at(CUSHION_M * 0.16, e);
  if (cushion > 0.0) {
    let s = stone(lat_axes(CUSHION_M, vec2f(0.4)), 0.0, 94, 0.5 * (0.4 + wet), vec2f(0.1, 0.2));
    c = mix(c, c * CUSHION_TINT * 1.3, s.y * k * cushion) * mix(1.0, s.x, cushion);
  }
  let logs = shows_at(LOG_M.y, e);
  var trunk = LOG_MEAN;
  if (logs > 0.0) { trunk = mix(LOG_MEAN, ease(LOG_FROM, LOG_FROM + 0.2, noise_l(elongated(LOG_M.x, LOG_M.y, vec2f(0.0)), 95).x), logs); }
  c = mix(c, c * mix(LOG_COLOUR, CUSHION_TINT * 0.8, 0.5 * wet) * 1.7, LOGS * k * trunk);
  let bloom = shows_at(BLOOM_M * 0.5, e);
  if (bloom > 0.0) {
    let l = lat_axes(BLOOM_M, vec2f(0.0));
    let h = lowbias(l.i, 96);
    let drift = ease(0.15, 0.6, noise_l(lat_turned(BLOOM_DRIFT_M, vec2f(0.0)), 107).x);
    let on = step(f32(h & 255u) / 255.0, 0.55 * drift) * (1.0 - smoothstep(0.2, 0.4, length(l.f - 0.5)));
    let petal = mix(vec3f(1.0, 0.95, 0.5), vec3f(0.85, 0.7, 1.0), f32((h >> 8u) & 1u));
    c = mix(c, petal * dot(c, LUMA) * 3.5, BLOOMS * on * mix(0.4, 1.0, grown) * bloom);
  }
  return c;
}
