// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl sprites.wgsl snag_sprites.wgsl heights.wgsl tex.wgsl fire.wgsl smoke.wgsl
// The fires burning this month over the live map, over its plants (web/src/fire.ts; drawn only over the squares they
// burn and the smoke's reach): the glow of the burning ground, the flames along the front of each burning patch (the
// outer edge of the square's burnt share, fire.wgsl `under_share`, as tall as the core's flame length), the embers behind
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
// Behind the front the burnt share of this month glows with embers: `EMBERS` of the flames' red, in sparse specks of
// about `EMBER_M` that flicker (where the noise passes `EMBER_SPARSE`); seen from afar, their mean (`EMBER_MEAN`).
const EMBERS = 0.35; const EMBER_M = 0.6; const EMBER_SPARSE = 0.75; const EMBER_MEAN = 0.1; const EMBER_RGB = vec3f(0.95, 0.25, 0.04);
// A burning square glows over its ground and the squares about it, seen from afar: as bright as `GLOW` times its
// burning share over `GLOW_SHARE` (held to 1) times its flames' length over `GLOW_M` (held to 1), fading in as a pixel
// takes from `GLOW_E.x` to `GLOW_E.y` m, where its fronts are too thin to see. The map draws it stronger by
// `GLOW_EARTH`.
const GLOW = 0.3; const GLOW_SHARE = 0.05; const GLOW_M = 3.0; const GLOW_E = vec2f(10.0, 60.0); const GLOW_EARTH = 1.6;
const GLOW_RGB = vec3f(1.0, 0.36, 0.06);
// Inside a square's burning share its fire runs as fronts across it, each point swept once by a front as `SWEEP` runs
// down through a whole number: `SWEEP` is a ragged field of the place (ragged.wgsl, its patches from `SWEEP_M` across
// down to the pixel, `SWEEP_LEVELS` whole numbers deep, so fronts lie some hundred metres apart and run ragged, as a
// fire's front does along its fuel) less the time over `SWEEP_S` s, so the fronts move while the world plays. A front flames `FLAME_DEPTH` lengths deep (as the outer front, tongues on it as
// tall); behind it the embers fade over `EMBER_FADE` of the way to the next front, to `EMBER_BASE` of their glow: the
// burning share smoulders all month.
const SWEEP_M = 400.0; const SWEEP_OCTAVES = 6; const SWEEP_LEVELS = 3.0; const SWEEP_S = 6.0; const EMBER_FADE = 0.35; const EMBER_BASE = 0.4;
// A point's flames fade in over the first `FLAME_FADE` of a square's burning share, so they end smoothly where the
// share blended between the squares' centres ends; none stands on liquid (its ponds' as the ground's tiles draw them,
// land.wgsl `standing`, deeper than `FILM_M`).
const FLAME_FADE = 0.005; const FLAME_COVER = 0.8;
// The fronts inside a burning share are drawn as lines where their flaming depth takes from `INNER_PX.x` to
// `INNER_PX.y` pixels: thinner, a share's glow is its embers' mean.
const INNER_PX = vec2f(1.0, 3.0);

// Smoke: its column of particles, g/m², and how far it came, worked out once a world frame (smoke.wgsl, read here
// through lib/smoke.wgsl `smoke_at`). Light passes the column by e^-(`EXTINCTION` × g/m²): biomass smoke takes 3.5 to
// 5 m² a gram (Reid and others 2005). The map draws it `SMOKE_EARTH` times as thick, so a fire's plume shows,
// and covers no more than `SMOKE_EARTH_MOST` of the map under it, so a fire shows through its own smoke. It shows from
// `SMOKE_NEAR_E` m a pixel on.
const EXTINCTION = 4.5; const SMOKE_EARTH = 2.0; const SMOKE_EARTH_MOST = 0.45;
const SMOKE_NEAR_E = vec2f(0.15, 1.5);
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

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  pix_origin = fire.frame.xy;
  pix_local = fire.corner.xy + v.px * fire.frame.z;
  bends_e = fire.frame.w;
  let at = pix_origin + pix_local;
  let e = fire.frame.w;
  let time = fire.air.x;
  let f = fire_at(at);
  // The front: the outer edge of the burnt share, the flaming zone `FLAME_DEPTH` lengths deep behind it, and the embers
  // over the rest of this month's burnt share.
  // (The front and the band behind it are read without the patches finer than the flames, `FRONT_SMOOTH`.)
  var r = burn_rank(at, e);
  if (FRONT_SMOOTH * f.length > e) { r = burn_rank(at, FRONT_SMOOTH * f.length); }
  // (The zone behind the front: the rank within the depth under the burnt share's level.)
  let across = fwidth(r.x);
  let depth = max(FLAME_DEPTH * f.length, FRONT_PX * e) / e * across;
  let burnt = under_share(r, f.burnt, at);
  let zone = max(burnt - under_share(r + vec2f(depth, 0.0), f.burnt, at), 0.0);
  let band = max(burnt - under_share(r, f.burnt - f.flame, at), 0.0);
  // The tongues stand on the front below the point: the rank rises to the burnt share's level that far south (to first
  // order; a front that runs north and south is read as no less than half as steep).
  let slope = vec2f(dpdx(r.x), dpdy(r.x)) / e;
  let dy = sign(slope.y + 1e-12) * max(abs(slope.y), 0.5 * length(slope));
  let t = (burnt_level(f.burnt, at) - r.x) / dy;
  // The fronts inside the burning share (`SWEEP`): how far behind the last front the point lies, as a share of the way
  // between two, and in metres; the tongues on the front south of it, as on the outer front.
  let sweep = (0.5 + 0.5 * ragged(at, max(e, FRONT_SMOOTH * f.length), SWEEP_M, SWEEP_OCTAVES, 17)) * SWEEP_LEVELS - time / SWEEP_S;
  let sweep_slope = vec2f(dpdx(sweep), dpdy(sweep)) / e;
  let sweep_dy = sign(sweep_slope.y + 1e-12) * max(abs(sweep_slope.y), 0.5 * length(sweep_slope));
  let behind = 1.0 - fract(sweep);
  let behind_m = behind / max(length(sweep_slope), 1e-9);
  var light = vec3f(0.0);
  if (f.flame > 0.0) {
    let shown = ease(TONGUE_PX.x * e, TONGUE_PX.y * e, MAP_VIEW.y * f.length);
    // The fronts inside the share show where they are a few pixels deep; from afar the share glows with its mean.
    let resolved = ease(INNER_PX.x * e, INNER_PX.y * e, FLAME_DEPTH * f.length);
    let inner = max(band - zone, 0.0);
    let flaming = max(1.0 - ease(max(FLAME_DEPTH * f.length, FRONT_PX * e), max(FLAME_DEPTH * f.length, FRONT_PX * e) + e, behind_m), 0.0);
    light += tongues(at, t, f.length, f.wind, e, time) * shown;
    light += inner * resolved * tongues(at, (round(sweep) - sweep) / sweep_dy, f.length, f.wind, e, time) * shown;
    // The fronts flame where tongues do not show; the embers flicker behind them.
    light += (zone + inner * resolved * flaming) * FLAME_MID * (1.0 - shown);
    let speck = ease(EMBER_SPARSE, 1.0, noise(at / EMBER_M + vec2f(0.0, 3.0 * time), 13).x);
    let specks = mix(EMBER_MEAN, speck, shows_at(EMBER_M, e));
    light += EMBERS * inner * (1.0 - flaming) * mix(EMBER_BASE, 1.0, exp(-behind / EMBER_FADE)) * specks * EMBER_RGB;
    light *= ease(0.0, FLAME_FADE, f.flame) * select(1.0, 0.0, standing(ground_at(at, false)) > FILM_M);
  }
  // The glow of the burning squares, seen from afar.
  let glow = GLOW * GLOW_EARTH * ease(0.0, GLOW_SHARE, f.flame) * min(f.length / GLOW_M, 1.0) * ease(GLOW_E.x, GLOW_E.y, e);
  light += glow * GLOW_RGB;
  // The smoke over it all: its column's depth, drawn thicker (`SMOKE_EARTH`); its texture moves with the wind.
  let col = smoke_at(at);
  let drift = at - f.wind * time;
  let rough = 0.55 + 0.5 * noise(drift / SMOKE_NOISE_M, 21).x + 0.3 * noise(drift / (0.4 * SMOKE_NOISE_M), 22).x;
  var tau = EXTINCTION * col.x * SMOKE_EARTH * max(rough, 0.0);
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
      // (Each puff fades out before its quarter's edge, so no square shows.)
      if (all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0))) { d += fire_sprite(2 + i32(lowbias(l.i, 31 + k) & 1u), uv, e / (size * PUFF_M)) * (1.0 - smoothstep(0.35, 0.5, length(uv - 0.5))); }
    }
    tau *= mix(1.0, 0.4 + 0.9 * d, puffs);
  }
  let smoke = 1.0 - exp(-tau);
  let tone = mix(SMOKE_YOUNG_RGB, SMOKE_RGB, ease(0.0, SMOKE_YOUNG_M, col.y));
  // Light the flames give, covered by the smoke over them, over the map as far as it outshines it (`FLAME_COVER` of its
  // brightest channel: a flame hides what is behind it); the smoke over the map.
  // The smoke stands some hundreds of metres up: close to the ground the map draws what is under it, the smoke fading
  // out as a pixel takes from `SMOKE_NEAR_E.y` down to `SMOKE_NEAR_E.x` m.
  let a = clamp(smoke, 0.0, SMOKE_EARTH_MOST) * ease(SMOKE_NEAR_E.x, SMOKE_NEAR_E.y, e);
  let lit = light * (1.0 - 0.7 * a);
  let shine = min(FLAME_COVER * max(lit.r, max(lit.g, lit.b)), 1.0);
  return vec4f(tone * a + lit, a + shine * (1.0 - a)) * draw.alpha;
}
