// A channel's reach as the live map draws it (land.wgsl `land_spots`): its bed and this month's water in it, as its
// shape at the point gives them (rock.wgsl `channel_at`: the bed's width along the reach, the pool against the outer
// bank of a bend and the bar on its inside, the riffle between bends), broken at their edges by numbers fixed in the
// world, so no bank is a smooth line: ragged by `RAG_SHARE` of the water's half width, at most `RAG_MOST_M`, over noise
// `RAG_M` long (each octave fading in as it grows past a few pixels). A steep reach is one whose flow moves stones
// from `STEEP_STONE` m (rock.wgsl `stone_of`).
const RAG_M = vec2f(9.0, 2.6); const RAG_SHARE = 0.3; const RAG_MOST_M = 3.0;
const STEEP_STONE = vec2f(0.12, 0.5);
// A bar's gravel and sand: its tone varies by up to `BAR_MOTTLE` over patches `BAR_PATCH_M` across and finer.
const BAR_MOTTLE = 0.3; const BAR_PATCH_M = vec2f(2.5, 0.6);

// The tone of a bar at `at`, seen at `e` m a pixel: its mottle (each octave where it shows).
fn bar_tone(at: vec2f, e: f32) -> f32 {
  var t = 1.0;
  let coarse = shows_at(BAR_PATCH_M.x, e);
  if (coarse > 0.0) { t += BAR_MOTTLE * coarse * noise_turned(at / BAR_PATCH_M.x, 76); }
  let fine = shows_at(BAR_PATCH_M.y, e);
  if (fine > 0.0) { t += 0.6 * BAR_MOTTLE * fine * noise_turned(at / BAR_PATCH_M.y, 77); }
  return t;
}

// The streaks of the flow: lines along the channel `STREAK_W_M` apart, broken into dashes `STREAK_M` long, as strong as
// `STREAK`; white water over a steep reach's stones, in patches `WHITE_M` long, as much as `FOAM_STEEP` where the
// reach is steepest.
const STREAK_W_M = 1.2; const STREAK_M = 8.0; const STREAK = 0.6; const WHITE_M = 4.0; const FOAM_STEEP = 0.55;

// A reach at a point: the share of the pixel its water covers, and its bars (the bed out of the water); how far across
// the water the point lies, 0 at its middle and 1 at its edge (`across`), and how steep the reach reads, 0..1
// (`steep`).
struct Reach { share: f32, bars: f32, across: f32, steep: f32 }

// The reach at `at`, m, seen at `e` m a pixel, on ground `g` of the channel `c` (rock.wgsl `channel_at`), drawn no
// thinner than `line` m half wide from afar, `open` of its bed drawn apart from its water (land.wgsl), whose flow moves
// stones of `stone` m.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, line: f32, open: f32, stone: f32) -> Reach {
  var r: Reach;
  r.steep = ease(STEEP_STONE.x, STEEP_STONE.y, stone);
  var half = c.water;
  var banks = c.bed;
  // The edges' raggedness shows where the water takes a few pixels.
  if (c.water > e) {
    var edge = 0.0;
    var bank_edge = 0.0;
    // Two octaves, the finer half as strong, each read only where it shows.
    let coarse = shows_at(RAG_M.x, e);
    if (coarse > 0.0) { edge += coarse * noise_turned(at / RAG_M.x, 66); bank_edge += coarse * noise_turned(at / RAG_M.x + 11.0, 70); }
    let fine = 0.5 * shows_at(RAG_M.y, e);
    if (fine > 0.0) { edge += fine * noise_turned(at / RAG_M.y, 67); bank_edge += fine * noise_turned(at / RAG_M.y + 11.0, 71); }
    half += min(RAG_SHARE * half, RAG_MOST_M) * edge;
    banks += min(RAG_SHARE * banks, RAG_MOST_M) * bank_edge;
  }
  let thread = abs(g.sd - c.middle);
  r.across = min(thread / max(half, 1e-6), 1.0);
  let reach = max(banks, line);
  // Past a line's end its texels' signed distances change sign across its extension and blend to a false 0 there: the
  // nearest texel lies far from the line itself (`Tiled.ends`), and nothing is drawn (it drew small square brackets).
  let real = 1.0 - ease(reach + e, reach + 2.0 * e, g.ends.x);
  let in_bed = (tent((reach - g.d) / e) - tent((-reach - g.d) / e)) * real;
  // The water lies in its bed: where the two edges' raggedness would put it past the bank, the bank holds it.
  let water = min(tent((half - thread) / e) - tent((-half - thread) / e), in_bed);
  r.share = mix(in_bed, water, open);
  r.bars = max(in_bed - r.share, 0.0);
  return r;
}

// The signs of the flow on a channel's water seen at `e` m a pixel, `along` m along its line and `sd` m across it, on
// a reach of steepness `steep` (`Reach`), `across` its water from its middle: streaks along the flow (lines along the
// channel's line, broken into dashes by a noise fixed in the world) and white water where the reach is steep, as a
// share of the surface's light to add, and the share of white water. Each repeats over the length the place along a
// channel is kept modulo (rock.wgsl `noise_reach`).
fn flow_signs(e: f32, along: f32, sd: f32, steep: f32, across: f32) -> vec2f {
  var streak = 0.0;
  let shown = shows_at(STREAK_W_M, e);
  if (shown > 0.0) {
    let dashes = ease(0.35, 0.75, noise_reach(along, STREAK_M, sd / STREAK_W_M, 74));
    streak = STREAK * shown * dashes * (1.0 - across * across);
  }
  var foam = 0.0;
  if (steep > 0.0) {
    // In patches along the flow, more of them the steeper; none from afar, where a pixel's mean of them would only
    // grey the water.
    let w = noise_reach(along, WHITE_M, sd / (0.35 * WHITE_M), 75) + 0.5 * noise_reach(along, 0.5 * WHITE_M, sd / (0.15 * WHITE_M), 78);
    foam = FOAM_STEEP * steep * ease(0.45 - 0.2 * steep, 0.7 - 0.2 * steep, w) * shows_at(WHITE_M, e);
  }
  return vec2f(streak, foam);
}
