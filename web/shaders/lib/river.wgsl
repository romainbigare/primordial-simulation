// A channel's reach as the live map draws it (land.wgsl `land_spots`): its water and its bed by the channel law, varied
// along the reach and broken at its banks by numbers fixed in the world, so no bank is a straight line nor the width
// one width. Pools and riffles alternate along a reach every five to seven channel widths (Leopold and Wolman 1957,
// Keller and Melhorn 1978): the water is narrower and deeper in a pool, wider and shallower over a riffle, as much as
// `WIDTH_VARY` either way, over a noise of wavelength `POOL_M` (the first for a brook, the second for a river, blended
// by its width). Its banks are ragged by `RAG_SHARE` of its half width, at most `RAG_MOST_M`, over noise `RAG_M` long
// (each octave fading in as it grows past a few pixels). The flood's bed (`banks_of`) varies by `BANK_VARY` too. A
// steep reach (one whose flow moves stones from `STEEP_STONE` m: rock.wgsl `stone_of`) is a narrow cut: its bed no
// wider than `CUT` of its water. A wide bed on a gentle reach braids: where it is more than `BRAID_FREE` times the
// water wide, a second thread runs on the other side of the bed where a noise is high.
const POOL_M = vec2f(60.0, 300.0); const WIDTH_VARY = 0.35; const BANK_VARY = 0.3;
const RAG_M = vec2f(9.0, 2.6); const RAG_SHARE = 0.35; const RAG_MOST_M = 3.0;
const STEEP_STONE = vec2f(0.12, 0.5); const CUT = 1.4; const BRAID_FREE = 1.5; const BRAID_HALF = 0.6;
// A channel's thread of this month's water wanders across its bed over `WANDER` bed widths along it.
const WANDER = 5.0;
// A bar's gravel and sand: its tone varies by up to `BAR_MOTTLE` over patches `BAR_PATCH_M` across and finer, and it is
// darker where wet within `BAR_WET_M` of the water.
const BAR_MOTTLE = 0.3; const BAR_PATCH_M = vec2f(2.5, 0.6); const BAR_WET_M = 1.0;

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
// `STREAK`; white water over a steep reach's stones, in patches `WHITE_M` across, as much as `FOAM_STEEP` where the
// reach is steepest.
const STREAK_W_M = 1.2; const STREAK_M = 9.0; const STREAK = 0.6; const WHITE_M = 3.0; const FOAM_STEEP = 0.55;

// A reach at a point: the share of the pixel its water covers, and its bars (the bed out of the water); its water's
// depth there over the depth the channel law gives (`depth`), how far across the water the point lies, 0 at its
// thread and 1 at its edge (`across`), and how steep the reach reads, 0..1 (`steep`).
struct Reach { share: f32, bars: f32, depth: f32, across: f32, steep: f32 }

// The reach at `at`, m, seen at `e` m a pixel, on ground `g` (its signed distance to the channel's line), of the channel
// `bed` that carries `q` m³/s, drawn no thinner than `line` m half wide from afar, `open` of its bed drawn apart from its
// water (land.wgsl), whose flow moves stones of `stone` m.
fn reach_at(at: vec2f, e: f32, g: Tiled, bed: Bed, q: f32, line: f32, open: f32, stone: f32) -> Reach {
  var r: Reach;
  r.steep = ease(STEEP_STONE.x, STEEP_STONE.y, stone);
  var half = bed.half;
  var banks = banks_of(q);
  var pool = 0.0;
  // The variation along the reach shows where the channel takes a few pixels.
  if (bed.half > e) {
    pool = mix(noise_turned(at / POOL_M.x, 64), noise_turned(at / POOL_M.y, 65), ease(5.0, 30.0, bed.half));
    half *= 1.0 + WIDTH_VARY * pool;
    banks *= 1.0 + BANK_VARY * noise_turned(at / POOL_M.y + 5.0, 68);
    let rag = min(RAG_SHARE * half, RAG_MOST_M);
    var edge = 0.0;
    var bank_edge = 0.0;
    // Two octaves, the finer half as strong, each read only where it shows.
    let coarse = shows_at(RAG_M.x, e);
    if (coarse > 0.0) { edge += coarse * noise_turned(at / RAG_M.x, 66); bank_edge += coarse * noise_turned(at / RAG_M.x + 11.0, 70); }
    let fine = 0.5 * shows_at(RAG_M.y, e);
    if (fine > 0.0) { edge += fine * noise_turned(at / RAG_M.y, 67); bank_edge += fine * noise_turned(at / RAG_M.y + 11.0, 71); }
    half = max(half + rag * edge, 0.25 * bed.half);
    banks += min(RAG_SHARE * banks, RAG_MOST_M) * bank_edge;
  }
  banks = max(mix(banks, CUT * half, r.steep), half);
  r.depth = 1.0 / (1.0 + WIDTH_VARY * pool);
  // The thread of this month's water, `off` m from the line, on either side; and a second one where the bed braids.
  let free = max(banks - half, 0.0);
  var off = 0.0;
  // (Its noise's scale is the flood bed's by the channel law alone: one length along the reach.)
  if (open > 0.0) { off = free * clamp(1.4 * noise_turned(at / (2.0 * WANDER * banks_of(q)), 63), -1.0, 1.0); }
  let thread = abs(g.sd - off);
  var water = tent((half - thread) / e) - tent((-half - thread) / e);
  r.across = min(thread / half, 1.0);
  let braids = ease(BRAID_FREE, 2.0 * BRAID_FREE, free / half) * (1.0 - r.steep) * open;
  if (braids > 0.0) {
    let other = abs(g.sd + sign(off) * 0.8 * free);
    let side = BRAID_HALF * half * ease(-0.1, 0.3, noise_turned(at / POOL_M.y + 23.0, 72)) * braids;
    let second = tent((side - other) / e) - tent((-side - other) / e);
    if (second > water) { r.across = min(other / max(side, 1e-3), 1.0); }
    water = max(water, second);
  }
  let reach = max(banks, line);
  let in_bed = tent((reach - g.d) / e) - tent((-reach - g.d) / e);
  r.share = mix(in_bed, water, open);
  r.bars = max(in_bed - r.share, 0.0);
  return r;
}

// The signs of the flow on a channel's water at `at`, seen at `e` m a pixel, `sd` m across from its line, on a reach of
// steepness `steep` (`Reach`), `across` its water from its thread: streaks along the flow (lines along the channel's
// line, broken into dashes by a noise fixed in the world) and white water where the reach is steep, as a share of the
// surface's light to add, and the share of white water.
fn flow_signs(at: vec2f, e: f32, sd: f32, d_slope: vec2f, steep: f32, across: f32) -> vec2f {
  // The point's place along the channel: across its line's direction (the signed distance's slope), so the signs are
  // drawn out along the flow.
  let n = normalize(d_slope * sign(sd) + vec2f(1e-6, 0.0));
  let along = dot(at, vec2f(-n.y, n.x));
  let flow = vec2f(along, sd);
  var streak = 0.0;
  let shown = shows_at(STREAK_W_M, e);
  if (shown > 0.0) {
    let lines = noise(vec2f(sd / STREAK_W_M, 0.5), 73).x;
    let dashes = ease(0.0, 0.5, noise(flow / vec2f(STREAK_M, 2.0 * STREAK_W_M), 74).x);
    streak = STREAK * shown * ease(0.3, 0.9, lines) * dashes * (1.0 - across * across);
  }
  var foam = 0.0;
  if (steep > 0.0) {
    // In patches over the stones, more of them the steeper: a third of the surface at the steepest; none from afar,
    // where a pixel's mean of them would only grey the water.
    let w = noise(flow / vec2f(WHITE_M, 0.35 * WHITE_M), 75).x + 0.5 * noise(flow / vec2f(0.4 * WHITE_M, 0.15 * WHITE_M), 78).x;
    foam = FOAM_STEEP * steep * ease(0.45 - 0.2 * steep, 0.7 - 0.2 * steep, w) * shows_at(WHITE_M, e);
  }
  return vec2f(streak, foam);
}
