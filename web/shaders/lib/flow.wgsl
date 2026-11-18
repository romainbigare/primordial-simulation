// The signs of the flow on a channel's water (land.wgsl `river_spots`): what shows which way and how fast it runs,
// laid along the channel's own line (`Tiled.along`) and across its water, fixed in the world and coming again after
// the length a place along a channel is kept modulo (rock.wgsl `noise_reach`). Streaks: short bright lines along the
// flow, where the current's shear ripples the surface, in lanes `LANE_M` wide (`LANES` of them at least and at most
// across half the water) and dashes `STREAK_M` long; many over a riffle, few on a pool, whose water lies smooth and
// dark. A riffle's water breaks in small white flecks `FLECK_M` long. The signs slide down the channel (`land.reach.z`
// m for the slowest water): a pool's at that pace, a riffle's twice, a steep reach's up to four times as fast (each a
// whole number of times, blended, so the slide has no jump where it comes round); the steps stay where they are.
const LANE_M = 0.45; const LANES = vec2f(3.0, 12.0); const STREAK_M = 8.0; const STREAK = 0.55;
const FLECK_M = 2.0; const FLECK = 0.3;
// A reach falls in steps from a slope of `STEP_SLOPE.x`, all of it stepped from `STEP_SLOPE.y` (step-pool reaches lie
// above a slope of about 0.03, pool-riffle and plane-bed ones below: Montgomery and Buffington 1997). Its steps lie
// `STEP_JUDD.x` S^-`STEP_JUDD.y` m apart on the mean (S its slope: Judd 1964, from memory), no closer than
// `STEP_WIDTHS.x` and no farther than `STEP_WIDTHS.y` of its bed's widths (one to four widths: Chin 1989). They lie at
// random along it, not in rows: in cells of `STEP_CELL` of that spacing (a power of two metres, so they come again with
// the place along; the two powers about it blended), each holding one step by the chance that keeps the mean spacing,
// anywhere within `STEP_JITTER` of the cell from its middle; and runs of `STEP_RUN_CELLS` cells, `STEP_RUNS` of them,
// hold none: smooth runs between stepped stretches.
const STEP_SLOPE = vec2f(0.025, 0.045); const STEP_JUDD = vec2f(0.31, 1.19); const STEP_WIDTHS = vec2f(1.0, 4.0);
const STEP_CELL = 0.5; const STEP_JITTER = 0.4; const STEP_RUN_CELLS = 8; const STEP_RUNS = 0.25;
// A step's line crosses the channel tilted by up to `STEP_TILT` of the water's half width from its middle to its edge
// and bowed by up to `STEP_BOW`, and wavers by `STEP_WAVE` of the spacing over noise `STEP_WAVE_M` long; it is white
// over a part of the water's width only, from `STEP_SPAN.x` to `STEP_SPAN.y` of its half width either side of a middle
// within `STEP_OFF` of it, fading out from `STEP_SPAN_SOFT` of that. Its white water begins over `STEP_EDGE_M`
// below the line (the place along a channel is known to a few centimetres: the tiles keep it as a 16-bit float, and
// an edge sharper than that would draw the texels' lattice) and fades down the flow over `STEP_FADE` of the spacing, each step's from half to one and a half times
// that, gone at `STEP_TAIL` times; past it the water lies in a small pool (`STEP_POOL` deeper, `POOL_CALM` slower)
// over the next `STEP_CALM` of those lengths. Steps come `STEP_BACK` cells up the flow at most (and one down it, which a
// tilted line may reach over).
const STEP_TILT = 0.5; const STEP_BOW = 0.4; const STEP_WAVE = 0.12; const STEP_WAVE_M = 3.0;
const STEP_SPAN = vec2f(0.6, 1.6); const STEP_OFF = 0.6; const STEP_SPAN_SOFT = 0.3;
const STEP_EDGE_M = 0.4; const STEP_FADE = 0.2; const STEP_TAIL = 2.0; const STEP_CALM = vec2f(1.0, 3.0);
const STEP_BACK = 4;
const STEP_POOL = 0.6; const POOL_CALM = 0.8;
// A step breaks white as far as its stones stand up into the flow: from stones `STEP_ROUGH.x` of the water's depth at
// its middle, all from `STEP_ROUGH.y` (a step is as high as its largest stones; from memory), by up to `FOAM_STEEP`.
// Steeper than `CASCADE_SLOPE.x` the reach is a cascade, white between its steps too, by up to `CASCADE_FOAM` from
// `CASCADE_SLOPE.y` (cascades lie above a slope of about 0.1: Montgomery and Buffington 1997).
const STEP_ROUGH = vec2f(0.15, 0.6); const FOAM_STEEP = 0.9; const CASCADE_SLOPE = vec2f(0.08, 0.15); const CASCADE_FOAM = 0.45;
// The streams of the world's random numbers of the streaks, the flecks, the steps' lines, the steps, the runs without
// steps and the white water's streaks.
const FLOW_STREAK = 74; const FLOW_FLECK = 75; const FLOW_WAVE = 76; const FLOW_STEP = 77; const FLOW_RUN = 82; const FLOW_WHITE = 78;

// A pattern of the flow's signs slid down the channel: `pattern` at `along` less `pace` times the slide, for the two
// whole paces about `pace`, blended.
fn slid(along: f32, pace: f32, cell: f32, across: f32, k: i32) -> f32 {
  let slide = land.reach.z;
  let lower = noise_reach(along - floor(pace) * slide, cell, across, k);
  if (slide == 0.0) { return lower; }
  return mix(lower, noise_reach(along - (floor(pace) + 1.0) * slide, cell, across, k), fract(pace));
}

// The steps of cells `cell` m long (a power of two) at `a` m along the channel (its line's waver added), the share `x`
// of the way across its water of half width `half` m, seen at `e` m a pixel, on a reach whose steps lie `spacing` m
// apart on the mean: the white water of the steps up the flow, 0..1, and how calm the pool below them lies, 0..1.
fn steps_of(a: f32, x: f32, half: f32, cell: f32, spacing: f32, e: f32) -> vec2f {
  let n = max(i32(round(land.reach.x / cell)), 1);
  let i = i32(floor(a / cell));
  let held = cell / spacing;
  let edge = max(2.0 * e, STEP_EDGE_M);
  var out = vec2f(0.0);
  for (var b = -1; b < STEP_BACK; b++) {
    let j = ((i - b) % n + n) % n;
    if (f32(lowbias(vec2i(j / STEP_RUN_CELLS, n), FLOW_RUN) & 1023u) / 1024.0 < STEP_RUNS) { continue; }
    let w = lowbias(vec2i(j, n), FLOW_STEP);
    if (f32(w & 1023u) / 1024.0 >= held) { continue; }
    let v = w * 747796405u + 2891336453u;
    // Its line: where in its cell, tilted and bowed across the water.
    let jitter = STEP_JITTER * (f32((w >> 10u) & 127u) / 63.5 - 1.0);
    let tilt = STEP_TILT * (f32((w >> 17u) & 31u) / 15.5 - 1.0);
    let bow = STEP_BOW * (f32((w >> 22u) & 31u) / 15.5 - 1.0);
    let d = a - (f32(i - b) + 0.5 + jitter) * cell - (tilt * x + bow * x * x) * half;
    if (d <= 0.0) { continue; }
    // How long its white water runs, and the part of the width it spans.
    let own = f32(w >> 27u) / 31.0;
    let long = STEP_FADE * spacing * (0.5 + own);
    let middle = STEP_OFF * (f32(v & 255u) / 127.5 - 1.0);
    let span = mix(STEP_SPAN.x, STEP_SPAN.y, f32((v >> 8u) & 255u) / 255.0);
    let across = 1.0 - smoothstep(STEP_SPAN_SOFT * span, span, abs(x - middle));
    let r = d / long;
    out.x = max(out.x, (0.6 + 0.4 * own) * across * exp(-r) * smoothstep(0.0, edge, d) * (1.0 - smoothstep(0.5 * STEP_TAIL, STEP_TAIL, r)));
    out.y = max(out.y, smoothstep(STEP_CALM.x - 0.5, STEP_CALM.x, r) * (1.0 - smoothstep(STEP_CALM.y - 1.0, STEP_CALM.y, r)));
  }
  return out;
}

// The signs of the flow seen at `e` m a pixel, `along` m along the channel's line, the share `x` of the way across
// its water from its middle to its outer edge (-1 at its inner edge), on the channel `c` (rock.wgsl `channel_at`) of a
// reach of steepness `steep`, 0..1, and slope `fall` (rise over run), whose flow moves stones of `stone` m: the light its
// streaks add to the surface's, the share of white water, and how fast and shallow the water runs there, 0..1.
fn flow_signs(e: f32, along: f32, x: f32, c: Channel, steep: f32, fall: f32, stone: f32) -> vec3f {
  var f = vec3f(0.0, 0.0, steep);
  // (The widest lanes of the widest bed do not show in a program made for draws from far: rock.wgsl `BED_MOST_M`.)
  if (unseen(2.0 * BED_MOST_M / LANES.x)) { return f; }
  let lanes = clamp(c.water / LANE_M, LANES.x, LANES.y);
  let shown = shows_at(2.0 * c.water / lanes, e);
  if (shown <= 0.0) { return f; }
  let riffle = 1.0 - c.bend;
  let pace = 1.0 + riffle + 2.0 * steep;
  let mid = 1.0 - x * x;
  f.x = STREAK * shown * mid * (0.25 + 0.75 * riffle) * ease(0.3, 0.7, slid(along, pace, STREAK_M, x * lanes, FLOW_STREAK));
  f.y = FLECK * shown * mid * riffle * riffle * (1.0 - steep) * ease(0.5, 0.8, slid(along, pace, FLECK_M, 2.0 * x * lanes, FLOW_FLECK));
  let stepped = ease(STEP_SLOPE.x, STEP_SLOPE.y, fall);
  let rough = ease(STEP_ROUGH.x, STEP_ROUGH.y, stone / max(BED_MIDDLE * c.mean, 1e-3));
  if (stepped > 0.0 && rough > 0.0) {
    // The steps of the two cell sizes about the reach's own, blended.
    let spacing = clamp(STEP_JUDD.x * pow(fall, -STEP_JUDD.y), 2.0 * STEP_WIDTHS.x * c.bed, 2.0 * STEP_WIDTHS.y * c.bed);
    let size = log2(STEP_CELL * spacing);
    let a = along + STEP_WAVE * spacing * noise_reach(along, STEP_WAVE_M, 1.5 * x, FLOW_WAVE);
    let s = mix(steps_of(a, x, c.water, exp2(floor(size)), spacing, e), steps_of(a, x, c.water, exp2(floor(size) + 1.0), spacing, e), fract(size));
    let torn = 0.5 + 0.5 * ease(-0.4, 0.5, slid(along, pace, FLECK_M, 2.0 * x * lanes, FLOW_WHITE));
    let cascade = CASCADE_FOAM * ease(CASCADE_SLOPE.x, CASCADE_SLOPE.y, fall) * mid;
    f.y = max(f.y, shown * stepped * rough * torn * max(FOAM_STEEP * s.x * sqrt(mid), cascade));
    f.z = steep * (1.0 - POOL_CALM * stepped * s.y);
  }
  return f;
}
