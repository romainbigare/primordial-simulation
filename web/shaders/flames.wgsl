// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl sprites.wgsl heights.wgsl tex.wgsl fire.wgsl
// The fires burning this month over the live map, over its plants (web/src/fire.ts; drawn only over the squares they
// burn and the smoke's reach): the glow of the burning ground, the flames along the front of each burning patch (the
// outer edge of the square's burnt share, fire.wgsl `burn_rank`, as tall as the core's flame length), the embers behind
// it, and the smoke drifting down the low wind (the core's, fire.wgsl `FireAt.wind`). Light the flames give is added to the map's colour; smoke covers it. Colours
// are the screen's shares, 0..1, premultiplied.

// The flaming zone behind a front is `FLAME_DEPTH` flame lengths deep, and drawn no thinner than `FRONT_PX` pixels: a
// front's depth is its speed times the time a fuel flames, about a flame's length in grass and litter (Byram 1959,
// Anderson 1969, read).
const FLAME_DEPTH = 1.0; const FRONT_PX = 1.5;
// A flame of length L leans with the wind by tan θ = `LEAN` u / √(g L) (u the wind, g `GRAVITY`, water.wgsl: a flame's lean goes
// with its Froude number, Albini 1981, read), held to `LEAN_MOST`; the map draws its lean along the east and its height
// as the map's angled view shows it, `MAP_VIEW.y` as long (water.wgsl), less as it leans. Its tongues (fire.jpg's first two sprites, `TONGUE_FILL` of their quarter tall) stand `TONGUE_STEP`
// lengths apart along the front in two rows, each `TONGUE_LOW`..1 of the length tall, swaying `TONGUE_SWAY` of their
// height at about `TONGUE_HZ` times a second. They show as tongues from `TONGUE_PX` pixels tall. The front they stand
// on is read without the patches finer than `FRONT_SMOOTH` of their length: a flame's foot spans those.
const TONGUE_FILL = 0.969; const TONGUE_STEP = 0.35; const TONGUE_LOW = 0.6; const TONGUE_SWAY = 0.18;
const TONGUE_HZ = 2.5; const TONGUE_PX = vec2f(4.0, 10.0); const FRONT_SMOOTH = 0.5;
const LEAN = 1.4; const LEAN_MOST = 1.7;
// The flame's light (the screen's shares, added): white-yellow at its foot, orange, red at its tip, as a flame of
// burning wood glows at about 1400 K at its foot and cools to 1000 K in its tip (Drysdale 2011, read).
const FLAME_FOOT = vec3f(1.0, 0.86, 0.48); const FLAME_MID = vec3f(1.0, 0.47, 0.10); const FLAME_TIP = vec3f(0.62, 0.10, 0.02);
// Behind the front the burnt share of this month glows with embers: `EMBERS` of the flames' red, in specks of about
// `EMBER_M` that flicker; seen from afar, their mean.
const EMBERS = 0.35; const EMBER_M = 0.6; const EMBER_RGB = vec3f(0.95, 0.25, 0.04);
// A burning square glows over its ground and the squares about it, seen from afar: as bright as `GLOW` times its
// burning share over `GLOW_SHARE` (held to 1) times its flames' length over `GLOW_M` (held to 1), fading in as a pixel
// takes from `GLOW_E.x` to `GLOW_E.y` m, where its fronts are too thin to see. In the Earth view stronger by
// `GLOW_EARTH`.
const GLOW = 0.3; const GLOW_SHARE = 0.05; const GLOW_M = 3.0; const GLOW_E = vec2f(10.0, 60.0); const GLOW_EARTH = 1.6;
const GLOW_RGB = vec3f(1.0, 0.36, 0.06);
// Inside a square's burning share its fire runs as fronts across it, each point swept once by a front as `SWEEP` runs
// down through a whole number: `SWEEP` is a smooth field of the place (noise `SWEEP_M` across, `SWEEP_LEVELS` whole
// numbers deep, so fronts lie a few hundred metres apart and run along no line) less the time over `SWEEP_S` s, so the
// fronts move while the world plays. A front flames `FLAME_DEPTH` lengths deep (as the outer front, tongues on it as
// tall); behind it the embers fade over `EMBER_FADE` of the way to the next front.
const SWEEP_M = 600.0; const SWEEP_LEVELS = 4.0; const SWEEP_S = 6.0; const EMBER_FADE = 0.35;
// A point's flames fade in over the first `FLAME_FADE` of a square's burning share, so they end smoothly where the
// share blended between the squares' centres ends; none stands on liquid (its ponds' as the ground's tiles draw them,
// land.wgsl `standing`, deeper than `FILM_M`).
const FLAME_FADE = 0.005;

// Smoke: of the gas a burning square sends to the air, `SMOKE_SHARE` is its particles (biomass burning gives about
// 10 g of fine particles to 1600 g of carbon dioxide: Andreae and Merlet 2001), sent over the hours its fronts burn,
// `FLAMING_S` (four days of a front's life, eight hours a day: Andela and others 2019). The wind carries them at its
// speed; a point's column is what the burning ground upwind sends over the time the wind takes to cross it, spread
// sideways `SPREAD` of the way gone and lost from the column over `PLUME_M` (rising out of the layer the map shows,
// settling). Light passes the column by e^-(`EXTINCTION` × g/m²): biomass smoke takes 3.5 to 5 m² a gram (Reid and
// others 2005). Smoke rises over the flames before the wind lays it over the ground the map shows: within about
// `SMOKE_RISE_M` of them the column is thin (a plume's rise, Briggs 1975, read). The Earth view draws it `SMOKE_EARTH`
// times as thick, so a fire's plume shows, and covers no more than `SMOKE_EARTH_MOST` of the map under it, so a fire
// shows through its own smoke.
const SMOKE_SHARE = 0.006; const FLAMING_S = 1.15e5; const SPREAD = 0.25; const PLUME_M = 8000.0; const EXTINCTION = 4.5;
const SMOKE_RISE_M = 800.0; const SMOKE_EARTH = 2.0; const SMOKE_EARTH_MOST = 0.8; const CALM_M_S = 0.5;
// The column is summed over `PLUME_STEPS` steps upwind, each read at three points across the spread.
const PLUME_STEPS = 8;
// Smoke scatters light about evenly over colours: grey to white (`SMOKE_RGB`), browner within `SMOKE_YOUNG_M` of the
// flames (`SMOKE_YOUNG_RGB`), where unburnt tar and soot darken it.
const SMOKE_RGB = vec3f(0.78, 0.78, 0.80); const SMOKE_YOUNG_RGB = vec3f(0.42, 0.38, 0.35); const SMOKE_YOUNG_M = 1500.0;
// The smoke's texture: noise `SMOKE_NOISE_M` across and half that, moving with the wind; up close the puffs of
// fire.jpg's last two sprites, one in each cell `PUFF_M` across of two lattices (along the axes and turned), each
// placed and sized (`PUFF_SIZE`) by its own draw, from `PUFF_PX` pixels.
const SMOKE_NOISE_M = 900.0; const PUFF_M = 30.0; const PUFF_PX = vec2f(6.0, 16.0); const PUFF_SIZE = vec2f(0.7, 1.3);

// A random number 0..1 of the grid cell c and the stream k.
fn unit(c: vec2i, k: i32) -> f32 { return f32(lowbias(c, k)) * (1.0 / 4294967296.0); }

// The density of fire.jpg's sprite s (0, 1 tongues; 2, 3 puffs) at `uv` in its quarter (0..1, y down), `g` of it a pixel.
fn fire_sprite(s: i32, uv: vec2f, g: f32) -> f32 {
  let q = vec2f(f32(s % 2), f32(s / 2));
  let at = (q + clamp(uv, vec2f(0.0), vec2f(1.0))) * 0.5;
  return textureSampleGrad(detail_tex, detail_sampler, at, FIRE_FROM, vec2f(0.5 * g, 0.0), vec2f(0.0, 0.5 * g)).r;
}

// The flames' colour along a flame, `h` from its foot (0) to its tip (1).
fn flame_rgb(h: f32) -> vec3f { return mix(mix(FLAME_FOOT, FLAME_MID, ease(0.0, 0.45, h)), FLAME_TIP, ease(0.45, 1.0, h)); }

// The tongues of flame at `at`, seen at `e` m a pixel, `t` m north of the front at the foot below it (as the map draws
// a flame standing on it), flames `len` m long in wind `wind` (m/s east and south), at time `time`: their light.
fn tongues(at: vec2f, t: f32, len: f32, wind: vec2f, e: f32, time: f32) -> vec3f {
  let lean = clamp(LEAN * wind / sqrt(GRAVITY * max(len, 0.01)), vec2f(-LEAN_MOST), vec2f(LEAN_MOST));
  let tall = MAP_VIEW.y * len / sqrt(1.0 + dot(lean, lean));
  if (t < 0.0 || t > tall) { return vec3f(0.0); }
  let step = TONGUE_STEP * len;
  var light = vec3f(0.0);
  for (var row = 0; row < 2; row++) {
    let x = (at.x - t * lean.x) / step + 0.5 * f32(row);
    let c = vec2i(i32(floor(x)), row);
    let r = unit(c, 7);
    let sway = 1.0 + TONGUE_SWAY * sin(6.2832 * (TONGUE_HZ * time + r));
    let h = tall * mix(TONGUE_LOW, 1.0, unit(c, 8)) * sway;
    let s = select(0, 1, r > 0.5);
    let side = h / TONGUE_FILL;
    let u = vec2f(0.5 + (fract(x) - 0.5) * step / side + 0.15 * (unit(c, 9) - 0.5), 1.0 - t / side);
    if (abs(u.x - 0.5) > 0.5 || u.y < 0.0) { continue; }
    let d = fire_sprite(s, u, e / side);
    light += d * flame_rgb(t / h);
  }
  return light;
}

// The column of smoke particles at `at`, in wind `wind` (m/s east and south) g/m²: what the burning ground upwind sends
// (`SMOKE_SHARE` of its gas over `FLAMING_S`) over the time the wind takes to cross each step, spread across, fading
// out over `PLUME_M`; and the mean distance from where it was sent, m. A calm air (under `CALM_M_S`) lays it about its
// fire as a wind of that speed would along its last way.
// The gas a burning square sends to the air at `at`, kg/m²: its square's, laid round its centre as a Gaussian of
// `SOURCE_SPREAD` of a square's side (of the same sum), so a plume starts as a soft round cloud, not as the square.
const SOURCE_SPREAD = 0.4;
fn round_air(at: vec2f) -> f32 {
  let s = at / fire.map.x;
  let c = floor(s);
  let d = s - c - 0.5;
  return fire_texel(1, vec2i(c)).y * exp(-0.5 * dot(d, d) / (SOURCE_SPREAD * SOURCE_SPREAD)) / (2.0 * PI * SOURCE_SPREAD * SOURCE_SPREAD);
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
    let spread = SPREAD * s;
    let src = at - wind * s;
    let air = (round_air(src) + round_air(src + side * spread) + round_air(src - side * spread)) / 3.0;
    let c = air * (s1 - s0) * exp(-s / PLUME_M) * -expm1(-s / SMOKE_RISE_M);
    column += c;
    far += c * s;
  }
  let g = 1000.0 * SMOKE_SHARE / FLAMING_S / speed * column;
  return vec2f(g, far / max(column, 1e-12));
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  pix_origin = fire.frame.xy;
  pix_local = fire.corner.xy + v.px * fire.frame.z;
  let at = pix_origin + pix_local;
  let e = fire.frame.w;
  let time = fire.air.x;
  let earth_view = fire.map.w == 0.0;
  let f = fire_at(at);
  // The front: the outer edge of the burnt share, the flaming zone `FLAME_DEPTH` lengths deep behind it, and the embers
  // over the rest of this month's burnt share.
  // (The front and the band behind it are read without the patches finer than the flames, `FRONT_SMOOTH`.)
  var r = burn_rank(at, e);
  if (FRONT_SMOOTH * f.length > e) { r = burn_rank(at, FRONT_SMOOTH * f.length); }
  let across = fwidth(r.x);
  let depth = max(FLAME_DEPTH * f.length, FRONT_PX * e) / e * across;
  let zone = max(under_share(r, f.burnt) - under_share(r, f.burnt - depth), 0.0);
  let band = max(under_share(r, f.burnt) - under_share(r, f.burnt - f.flame), 0.0);
  // The tongues stand on the front below the point: the rank rises to the burnt share that far south (to first order;
  // a front that runs north and south is read as no less than half as steep).
  let slope = vec2f(dpdx(r.x), dpdy(r.x)) / e;
  let dy = sign(slope.y + 1e-12) * max(abs(slope.y), 0.5 * length(slope));
  let t = (f.burnt - r.x) / dy;
  // The fronts inside the burning share (`SWEEP`): how far behind the last front the point lies, as a share of the way
  // between two, and in metres; the tongues on the front south of it, as on the outer front.
  let sweep = (0.5 + 0.5 * noise(at / SWEEP_M, 17).x) * SWEEP_LEVELS - time / SWEEP_S;
  let sweep_slope = vec2f(dpdx(sweep), dpdy(sweep)) / e;
  let sweep_dy = sign(sweep_slope.y + 1e-12) * max(abs(sweep_slope.y), 0.5 * length(sweep_slope));
  let behind = 1.0 - fract(sweep);
  let behind_m = behind / max(length(sweep_slope), 1e-9);
  var light = vec3f(0.0);
  if (f.flame > 0.0) {
    let shown = ease(TONGUE_PX.x * e, TONGUE_PX.y * e, MAP_VIEW.y * f.length);
    let inner = max(band - zone, 0.0);
    let flaming = max(1.0 - ease(max(FLAME_DEPTH * f.length, FRONT_PX * e), max(FLAME_DEPTH * f.length, FRONT_PX * e) + e, behind_m), 0.0);
    light += tongues(at, t, f.length, f.wind, e, time) * shown;
    light += inner * tongues(at, (round(sweep) - sweep) / sweep_dy, f.length, f.wind, e, time) * shown;
    // The fronts flame where tongues do not show (from afar, their mean light); the embers flicker behind them.
    light += (zone + inner * flaming) * FLAME_MID * (1.0 - shown);
    let speck = ease(0.55, 0.9, noise(at / EMBER_M + vec2f(0.0, 3.0 * time), 13).x);
    let specks = mix(0.5, speck * 2.0, shows_at(EMBER_M, e));
    light += EMBERS * inner * (1.0 - flaming) * exp(-behind / EMBER_FADE) * specks * EMBER_RGB;
    light *= ease(0.0, FLAME_FADE, f.flame) * select(1.0, 0.0, standing(ground_at(at, false)) > FILM_M);
  }
  // The glow of the burning squares, seen from afar.
  let glow = GLOW * select(1.0, GLOW_EARTH, earth_view) * ease(0.0, GLOW_SHARE, f.flame) * min(f.length / GLOW_M, 1.0) * ease(GLOW_E.x, GLOW_E.y, e);
  light += glow * GLOW_RGB;
  // The smoke over it all: its column's depth, thicker in the Earth view; its texture moves with the wind.
  let col = smoke_column(at, f.wind);
  let drift = at - f.wind * time;
  let rough = 0.55 + 0.5 * noise(drift / SMOKE_NOISE_M, 21).x + 0.3 * noise(drift / (0.4 * SMOKE_NOISE_M), 22).x;
  var tau = EXTINCTION * col.x * select(1.0, SMOKE_EARTH, earth_view) * max(rough, 0.0);
  let puffs = shows_at(PUFF_M, e) * ease(PUFF_PX.x * e, PUFF_PX.y * e, PUFF_M);
  if (puffs > 0.0 && tau > 0.0) {
    let moved = -f.wind * time / PUFF_M;
    var d = 0.0;
    for (var k = 0; k < 2; k++) {
      var l = lat_axes(PUFF_M, moved);
      if (k == 1) { l = lat_turned(PUFF_M, moved); }
      let size = mix(PUFF_SIZE.x, PUFF_SIZE.y, unit(l.i, 32 + k));
      let off = vec2f(unit(l.i, 34 + k), unit(l.i, 36 + k)) - 0.5;
      let uv = (l.f - 0.5 - (1.0 - size) * off) / size + 0.5;
      if (all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0))) { d += fire_sprite(2 + i32(lowbias(l.i, 31 + k) & 1u), uv, e / (size * PUFF_M)); }
    }
    tau *= mix(1.0, 0.4 + 0.9 * d, puffs);
  }
  let smoke = 1.0 - exp(-tau);
  let tone = mix(SMOKE_YOUNG_RGB, SMOKE_RGB, ease(0.0, SMOKE_YOUNG_M, col.y));
  // Light the flames give, covered by the smoke over them; the smoke over the map.
  let a = clamp(smoke, 0.0, select(1.0, SMOKE_EARTH_MOST, earth_view));
  return vec4f(tone * a + light * (1.0 - 0.7 * a), a) * draw.alpha;
}
