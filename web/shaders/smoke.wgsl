// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl sprites.wgsl snag_sprites.wgsl heights.wgsl tex.wgsl fire.wgsl smoke.wgsl
// The smoke of the fires burning this month, worked out once a world frame (web/src/fire.ts `prepare`): a texel every
// `1 / SMOKE_PER_SQUARE` of a square, its column and how far it came, packed in four bytes (lib/smoke.wgsl), which
// flames.wgsl reads between texels. The smoke depends on the place and the wind alone, not on the time or the pixel.

// Smoke: of the gas a burning square sends to the air, `SMOKE_SHARE` is its particles (biomass burning gives about
// 10 g of fine particles to 1600 g of carbon dioxide: Andreae and Merlet 2001), sent over the hours its fronts burn,
// `FLAMING_S` (ten days of its fronts' burning, eight hours a day: a square's fire lives days to weeks, Andela and others
// 2019). The wind carries them at its
// speed; a point's column is what the burning ground upwind sends over the time the wind takes to cross it, spread
// sideways `SPREAD` of the way gone and lost from the column over `PLUME_M` (rising out of the layer the map shows,
// settling). Smoke rises over the flames before the wind lays it over the ground the map shows: within about
// `SMOKE_RISE_M` of them the column is thin (a plume's rise, Briggs 1975, read).
const SMOKE_SHARE = 0.006; const FLAMING_S = 2.9e5; const SPREAD = 0.5;
const SMOKE_RISE_M = 800.0; const CALM_M_S = 0.5;
// The column is summed over `PLUME_STEPS` steps upwind, each read at five points across the spread, weighted as a
// Gaussian of the spread (`PLUME_ACROSS`).
const PLUME_STEPS = 8;
const PLUME_ACROSS: array<f32, 5> = array<f32, 5>(0.054, 0.244, 0.403, 0.244, 0.054);

// The column of smoke particles at `at`, in wind `wind` (m/s east and south) g/m²: what the burning ground upwind sends
// (`SMOKE_SHARE` of its gas over `FLAMING_S`) over the time the wind takes to cross each step, spread across, fading
// out over `PLUME_M`; and the mean distance from where it was sent, m. A calm air (under `CALM_M_S`) lays it about its
// fire as a wind of that speed would along its last way.
// The gas a burning square sends to the air at `at`, kg/m²: its square's, laid round its centre as a Gaussian of
// `SOURCE_SPREAD` of a square's side (of the same sum), so a plume starts as a soft round cloud, not as the square.
const SOURCE_SPREAD = 0.4;
fn round_air(at: vec2f) -> f32 {
  // (Of the four squares whose centres lie about the point: the rest add next to nothing.)
  let s = at / fire.map.x;
  let c = floor(s - 0.5);
  var air = 0.0;
  for (var k = 0; k < 4; k++) {
    let sq = c + vec2f(f32(k % 2), f32(k / 2));
    let d = s - sq - 0.5;
    air += fire_texel(1, vec2i(sq)).y * exp(-0.5 * dot(d, d) / (SOURCE_SPREAD * SOURCE_SPREAD));
  }
  return air / (2.0 * PI * SOURCE_SPREAD * SOURCE_SPREAD);
}

fn smoke_column(at: vec2f, wind_at: vec2f) -> vec2f {
  let speed = max(length(wind_at), CALM_M_S);
  let wind = select(vec2f(1.0, 0.0), wind_at / length(wind_at), length(wind_at) > 1e-3);
  let side = vec2f(-wind.y, wind.x);
  var column = 0.0;
  var far = 0.0;
  for (var k = 0; k < PLUME_STEPS; k++) {
    let a = f32(k) / f32(PLUME_STEPS);
    let b = f32(k + 1) / f32(PLUME_STEPS);
    // Steps grow along the plume: its spread is wide far from the fire and its detail is fine near it.
    let s0 = PLUME_M * a * a;
    let s1 = PLUME_M * b * b;
    let s = 0.5 * (s0 + s1);
    // (Its spread starts at the source's own, so a plume's edge is soft from its fire on.)
    let spread = SPREAD * s + SOURCE_SPREAD * fire.map.x;
    let src = at - wind * s;
    var air = 0.0;
    for (var j = -2; j <= 2; j++) { air += PLUME_ACROSS[j + 2] * round_air(src + side * spread * f32(j) * 0.5); }
    let c = air * (s1 - s0) * exp(-s / PLUME_M) * -expm1(-s / SMOKE_RISE_M);
    column += c;
    far += c * s;
  }
  let g = 1000.0 * SMOKE_SHARE / FLAMING_S / speed * column;
  return vec2f(g, far / max(column, 1e-12));
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let at = (floor(v.px) + 0.5) * fire.map.x / SMOKE_PER_SQUARE;
  let col = smoke_column(at, fire_at(at).wind);
  return smoke_packed(col.x, col.y / PLUME_M);
}
