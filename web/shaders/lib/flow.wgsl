// The signs of the flow on a channel's water (land.wgsl `river_spots`): what shows which way and how fast it runs,
// laid along the channel's own line (`Tiled.along`) and across its water, fixed in the world and coming again after
// the length a place along a channel is kept modulo (rock.wgsl `noise_reach`). Streaks: short bright lines along the
// flow, where the current's shear ripples the surface, in lanes `LANE_M` wide (`LANES` of them at least and at most
// across half the water) and dashes `STREAK_M` long; many over a riffle, few on a pool, whose water lies smooth and
// dark. A riffle's water breaks in small white flecks `FLECK_M` long. A steep reach falls in steps every `STEP_M` or
// so (step-pool reaches: Montgomery and Buffington 1997): white water starts at a step, on a line across the channel that
// wavers by up to `STEP_WAVE` of a step along it and bows over `STEP_BOW` of the waver's cells from the water's middle
// to its edge (a line that swung more ran along the flow, and the white water's edge with it, a straight edge down the
// channel), and fades down the flow over `STEP_FADE` of the way to the next, in streaks; a step is white over part
// of the channel's width only, in patches `STEP_PATCH` steps long; past the white water it lies in a small pool
// (`STEP_POOL` deeper, `POOL_CALM` slower) up to the lip of the next step, where it runs fast and shallow. The signs slide down the channel (`land.reach.z` m for the slowest water): a pool's at that pace,
// a riffle's twice, a steep reach's up to four times as fast (each a whole number of times, blended, so the slide has
// no jump where it comes round); the steps stay where they are.
const LANE_M = 0.45; const LANES = vec2f(3.0, 12.0); const STREAK_M = 8.0; const STREAK = 0.55;
const FLECK_M = 2.0; const FLECK = 0.3;
const STEP_M = 8.0; const STEP_WAVE = 0.5; const STEP_BOW = 0.4; const STEP_FADE = 0.3; const STEP_PATCH = 2.0; const FOAM_STEEP = 0.9;
const STEP_POOL = 0.6; const POOL_CALM = 0.8;
// The white water begins over this length below a step's line, m: the place along a channel is known to a few
// centimetres (the tiles keep it as a 16-bit float), and an edge sharper than that would draw the texels' lattice.
const STEP_EDGE_M = 0.4;
// The streams of the world's random numbers of the streaks, the flecks, the steps' lines, their strengths and the
// white water's streaks.
const FLOW_STREAK = 74; const FLOW_FLECK = 75; const FLOW_STEP = 76; const FLOW_STRENGTH = 77; const FLOW_WHITE = 78;

// A pattern of the flow's signs slid down the channel: `pattern` at `along` less `pace` times the slide, for the two
// whole paces about `pace`, blended.
fn slid(along: f32, pace: f32, cell: f32, across: f32, k: i32) -> f32 {
  let slide = land.reach.z;
  let lower = noise_reach(along - floor(pace) * slide, cell, across, k);
  if (slide == 0.0) { return lower; }
  return mix(lower, noise_reach(along - (floor(pace) + 1.0) * slide, cell, across, k), fract(pace));
}

// The signs of the flow seen at `e` m a pixel, `along` m along the channel's line, the share `x` of the way across
// its water from its middle to its outer edge (-1 at its inner edge), on the channel `c` (rock.wgsl `channel_at`) of a
// reach of steepness `steep`, 0..1: the light its streaks add to the surface's, the share of white water, and how
// fast and shallow the water runs there, 0..1.
fn flow_signs(e: f32, along: f32, x: f32, c: Channel, steep: f32) -> vec3f {
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
  if (steep > 0.0) {
    // The steps: each a line across the channel that wavers, of its own strength, more of them white the steeper.
    let p = along / STEP_M + STEP_WAVE * noise_reach(along, 2.0 * STEP_M, STEP_BOW * x, FLOW_STEP);
    let cells = i32(round(land.reach.x / STEP_M));
    let own = f32(lowbias(vec2i(((i32(floor(p)) % cells) + cells) % cells, 0), FLOW_STRENGTH) & 1023u) / 1023.0;
    let broken = ease(-0.3, 0.2, noise_reach(along, STEP_PATCH * STEP_M, 1.3 * x + 7.0 * own, FLOW_STRENGTH) + steep - 0.7 + 0.4 * own);
    let fall = exp(-fract(p) / STEP_FADE) * smoothstep(0.0, max(2.0 * e, STEP_EDGE_M) / STEP_M, fract(p));
    let torn = 0.3 + 0.7 * ease(-0.4, 0.5, slid(along, pace, FLECK_M, 2.0 * x * lanes, FLOW_WHITE));
    f.y = max(f.y, FOAM_STEEP * shown * steep * broken * fall * torn * sqrt(mid));
    f.z = steep * (1.0 - POOL_CALM * smoothstep(STEP_FADE, 2.0 * STEP_FADE, fract(p)) * (1.0 - smoothstep(0.75, 0.95, fract(p))));
  }
  return f;
}
