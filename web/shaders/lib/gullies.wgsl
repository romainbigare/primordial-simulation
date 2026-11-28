// The gullies of the ground finer than its tiles (relief.wgsl): the samples stand 125 m apart and the tiles draw the
// ground between them smooth, so a mountain would be a soft swell. Here steep ground is cut into a branching net of
// gullies with spurs between them, at every scale from `GULLY_LONGEST_M` down to about a hundred metres: the
// water that runs down a slope cuts it into gullies that join into larger ones (Horton 1945; Montgomery and Dietrich
// 1992), and leaves a plain, a wet flat and a valley's floor uncut. Below them the same net goes on as rills, down to a
// few metres, so a slope seen close has a grain down its fall line. Each octave is erosion noise (after Fewes 2023 and
// Johansen 2024): in each cell of a turned lattice a wave runs across the fall line from a point of its own, and the
// four cells about a point are blended as a pair (cos, sin), so a gully runs on where they agree and forks or ends
// where they do not. Each octave follows the fall line of the ground with the octaves before it, so small gullies run
// down the sides of big ones and join them. Fixed in the world; each octave fades out as it shrinks to a few pixels
// (draw.wgsl `shows_at`), so its cost does not grow with the zoom.

// The longest octave's wavelength, m; each next one `GULLY_STEP` times shorter; their number (2000 m to 62 m); and the
// rills' octaves after them (31 m to 2 m), which count in neither share of the gullies (`Gullies.spur`, `fold`: they
// place the snow, which would move as the rills show with the zoom).
const GULLY_LONGEST_M = 2000.0; const GULLY_STEP = 2.0; const GULLY_OCTAVES = 6; const RILL_OCTAVES = 5;
// A gully's sides slope `GULLY_K` times the ground's slope (counted up to `GULLY_STEEP_MOST`, rise over run), each
// octave `GULLY_DECAY` times less, a rill's `RILL_K` times less again (a rill is a groove in the gully's wall, not a
// gully of its own), softly held under `WALL_MOST`: under the low light not every wall turns into the shade.
const GULLY_K = 1.0; const RILL_K = 0.5; const GULLY_DECAY = 0.9; const GULLY_STEEP_MOST = 0.8; const WALL_MOST = 0.7;
// A cell is `GULLY_PER_CELL` wavelengths across; its point moves up to `GULLY_JITTER` of a cell; its wavelength is the
// octave's times 2^(+-`WAVE_VARY`/2) and its gullies bend by up to `BEND` of a cell across per cell along, so no two
// cells comb the slope alike.
const GULLY_PER_CELL = 1.5; const GULLY_JITTER = 0.6; const WAVE_VARY = 0.5; const BEND = 0.25;
// The blended wave is made whole again as c / sqrt(r^2 + `NORM_SOFT`^2) (r the length of its pair): full depth but at a
// fork, and no crease. At a fork, where the cells' waves cancel (r under `FORK_FADE`), the octave fades out: made whole
// there it would draw a closed loop of floor and crest about the fork, a lens. A V floor and a sharp crest are rounded
// over `SHARP_FLOOR` of the profile, and over no less than `CREASE_PX` pixels to each side (finer, a crease would draw
// the pixels' steps). Octaves from
// `ROUND_LONG_M.x` to `.y` long turn from V to round, so a long wall has no sharp crest and the edge of its shade no
// razor's.
const NORM_SOFT = 0.3; const FORK_FADE = 0.45; const SHARP_FLOOR = 0.15; const CREASE_PX = 1.0;
const ROUND_LONG_M = vec2f(1000.0, 4000.0);
// The fall line that steers an octave, and the slope drawn, still fall along the ground's own fall line by
// `KEEP_FALL` of its slope (no closed pit or knob), the hold rounded over `KEEP_SOFT` of it.
const KEEP_FALL = 0.35; const KEEP_SOFT = 0.05;
// An octave shows fully from 9 pixels a wave and none under 3: `shows_at` of this share of its wavelength.
const GULLY_SHOW = 0.444;
// Loose ground `SOIL_LOOSE_M` deep (less of it from slope `SOIL_STEEP.x` to `.y`: steep ground is cut whatever covers
// it) and a deposit `FAN_M` deep (a fan, an apron) are soil: there the octaves from `SOIL_SHORT_M.x` long to `.y` lose
// `SOIL_SHORT_CUT` of their depth and their V floors `SOIL_ROUND` of the way to round, as creep fills the small gullies of soil; bare bed keeps
// its crisp crests.
const SOIL_LOOSE_M = vec2f(0.4, 3.0); const SOIL_STEEP = vec2f(0.35, 0.75); const FAN_M = vec2f(1.0, 6.0);
const SOIL_SHORT_M = vec2f(500.0, 100.0); const SOIL_SHORT_CUT = 0.75; const SOIL_ROUND = 0.6;
// Over `VARY_M` the gullies' depth varies by up to `VARY_DEPTH` either way and their crests' sharpness by `VARY_SHARP`:
// arêtes in one range, soft shoulders in the next.
const VARY_M = 3500.0; const VARY_DEPTH = 0.7; const VARY_SHARP = 0.9;
// Calm ground, eased between the two numbers: a gentle slope (rise over run), wet ground (wetness index), a shore (m
// over the level of the sea or of the ponds about it: a gully cut under it would fill with their liquid); and a valley's floor, `CALM_CHANNEL_M` m from its channel's line where it carries `CALM_Q.y`
// m³/s (that of about 10 km² drained), `CALM_CHANNEL_GROW` times wider per tenfold more, none for channels under
// `CALM_Q.x` (about 1 km²: a small channel runs in its gully's floor).
const CALM_SLOPE = vec2f(0.06, 0.3); const CALM_TWI = vec2f(12.0, 16.0); const CALM_SHORE_M = vec2f(0.0, 15.0);
const CALM_CHANNEL_M = vec2f(40.0, 160.0); const CALM_CHANNEL_GROW = 0.8; const CALM_Q = vec2f(0.008, 0.08);
// The gullies' share of the point (`Gullies.spur`, `fold`) is all of it from walls that slope this much in sum, the
// rills' (`Gullies.rill`) from `RILL_SEEN`.
const GULLY_SEEN = 0.3; const RILL_SEEN = 0.1;
// A dry wash runs in the floor of every gully (not a rill) from `WASH_LONG_M` long down: a pale line of the fines and
// gravel its floods sort, `WASH_SHARE` of its wave to each side of its middle, all of it where its walls slope
// `WASH_WALL` (rise over run). A line finer than `WASH_PX` pixels is spread over them, as pale on the mean.
const WASH_LONG_M = 500.0; const WASH_SHARE = 0.03; const WASH_WALL = 0.15; const WASH_PX = 0.7;
// The noise streams of the octaves (from `GULLY_STREAM` on) and of the slow variation.
const GULLY_STREAM = 300; const VARY_STREAM = 340;

// The ground the gullies cut: its slope (east, south; rise over run), the share of its bed that shows (rock.wgsl
// `bed_shows`), its loose ground and the load laid on it, m, its wetness index, its height over the level of the
// standing liquid about it (the sea's or its ponds'), m, the distance to its channel's line, m, and that channel's
// discharge, m³/s.
struct GullyGround { slope: vec2f, bare: f32, loose: f32, laid: f32, twi: f32, over_level: f32, channel_d: f32, q: f32 }
// The gullies: their height, m, and slope east and south; how high the point stands in them, 0 a floor to 1 a spur
// (`spur`, of all octaves; `fold`, of the octaves weighted by their length, each as a round wave: the broad hollows),
// 0.5 where none is cut; how high it stands in its rills (`rill`, as `spur`); and the share of the point a wash's pale
// floor takes (`WASH_SHARE`), 0..1.
struct Gullies { h: f32, slope: vec2f, spur: f32, fold: f32, rill: f32, wash: f32 }

// The slope `d` added to ground of slope `g0`, held so the whole still falls along g0 by `keep` of its slope.
fn keep_falling(g0: vec2f, d: vec2f, keep: f32) -> vec2f {
  let s0 = length(g0);
  if (s0 <= 0.0) { return d; }
  let n = g0 / s0;
  let along = dot(d, n);
  let least = (keep - 1.0) * s0;
  let w = KEEP_SOFT * s0;
  let held = 0.5 * (along + least + sqrt((along - least) * (along - least) + w * w));
  return d + (held - along) * n;
}

// A gully's profile at the wave c (-1 a floor .. 1 a crest): its height 0..1 and its change with c. `vee` turns a
// round floor to a V, rounded over `round` of the profile (near the floor t = (1 + c) / 2 is about (phase / 2)^2, so
// the rounding reaches `round` wave / pi to each side); `sharp` turns the crest from its own to sharp (halfway to the
// round floor and sharp crest).
fn gully_profile(c: f32, vee: f32, sharp: f32, round: f32) -> vec2f {
  let t = clamp(0.5 * (1.0 + c), 0.0, 1.0);
  let q = round * round;
  let n = sqrt(1.0 + q) - round;
  let rv = sqrt(t + q);
  let rr = sqrt(1.0 - t + q);
  let v = vec2f((rv - round) / n, 0.25 / (rv * n));
  let r = vec2f(1.0 - (rr - round) / n, 0.25 / (rr * n));
  let f = mix(vec2f(t, 0.5), v, vee);
  return f + sharp * 0.5 * (r - f);
}

// One octave `wave` m long across the fall line `across` (a unit vector) in the lattice turned by `rot`: the blended
// pair (cos, sin) and the slopes of each, east and south, per metre.
struct GullyWave { cs: vec2f, dc: vec2f, ds: vec2f }
fn gully_wave(wave: f32, across: vec2f, rot: mat2x2f, stream: i32) -> GullyWave {
  let cell = wave * GULLY_PER_CELL;
  let l = lat(rot * (1.0 / cell), vec2f(0.0));
  let f = l.f;
  let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
  let along = vec2f(across.y, -across.x);
  var o = GullyWave(vec2f(0.0), vec2f(0.0), vec2f(0.0));
  for (var j = 0; j < 4; j++) {
    let corner = vec2f(f32(j % 2), f32(j / 2));
    // One word: the point's place in its cell (11 bits each way), and its wave's own length and bend (10 bits).
    let word = lowbias(l.i + vec2i(corner), stream);
    let r = vec3f(f32(word & 0x7ffu) / 2048.0, f32((word >> 11u) & 0x7ffu) / 2048.0, f32(word >> 22u) / 1024.0);
    // The point from the cell's own, m along the world's axes.
    let q = transpose(rot) * (f - corner - GULLY_JITTER * (r.xy - 0.5)) * cell;
    let k = (2.0 * PI / wave) * exp2(WAVE_VARY * (r.z - 0.5));
    let bend = BEND * (2.0 * fract(8.0 * r.z) - 1.0) / cell;
    let qn = dot(q, along);
    let phase = k * (dot(q, across) + bend * qn * qn);
    let dphase = k * (across + 2.0 * bend * qn * along);
    let cw = cos(phase);
    let sw = sin(phase);
    // The corner's weight and its slope along the world's axes.
    let wx = select(1.0 - u.x, u.x, corner.x > 0.0);
    let wy = select(1.0 - u.y, u.y, corner.y > 0.0);
    let w = wx * wy;
    let dw = transpose(rot) * vec2f(select(-du.x, du.x, corner.x > 0.0) * wy, wx * select(-du.y, du.y, corner.y > 0.0)) / cell;
    o.cs += w * vec2f(cw, sw);
    o.dc += -w * sw * dphase + dw * cw;
    o.ds += w * cw * dphase + dw * sw;
  }
  return o;
}

// How far ground `gg` is cut, 0 (calm: a plain, a wet flat, a channel's floor, a shore, under liquid) .. 1.
fn gully_calm(gg: GullyGround) -> f32 {
  let q = max(gg.q, 1e-12);
  let grow = 1.0 + CALM_CHANNEL_GROW * max(log(q / CALM_Q.y) / log(10.0), 0.0);
  let off_floor = ease(CALM_CHANNEL_M.x * grow, CALM_CHANNEL_M.y * grow, gg.channel_d);
  let channel = mix(1.0, off_floor, ease(log(CALM_Q.x), log(CALM_Q.y), log(q)));
  return ease(CALM_SLOPE.x, CALM_SLOPE.y, length(gg.slope)) * (1.0 - ease(CALM_TWI.x, CALM_TWI.y, gg.twi))
    * ease(CALM_SHORE_M.x, CALM_SHORE_M.y, gg.over_level) * channel;
}

// The gullies of ground `gg` seen at `e` m a pixel.
fn gullies(e: f32, gg: GullyGround) -> Gullies {
  var out = Gullies(0.0, vec2f(0.0), 0.5, 0.5, 0.5, 0.0);
  let s0 = length(gg.slope);
  var depth = min(s0, GULLY_STEEP_MOST) * gully_calm(gg);
  // (Calm ground, and a view too far for the longest octave, read nothing.)
  if (depth <= 0.0 || shows_at(GULLY_SHOW * GULLY_LONGEST_M, e) <= 0.0) { return out; }
  let vary = noise_l(lat_axes(VARY_M, vec2f(0.0)), VARY_STREAM).x;
  depth *= 1.0 + VARY_DEPTH * vary;
  let sharp = clamp(rough_of(gg.bare, gg.loose) + VARY_SHARP * vary, 0.0, 1.0);
  let soil = max(ease(SOIL_LOOSE_M.x, SOIL_LOOSE_M.y, gg.loose) * (1.0 - ease(SOIL_STEEP.x, SOIL_STEEP.y, s0)), ease(FAN_M.x, FAN_M.y, gg.laid));
  let q2 = NORM_SOFT * NORM_SOFT;
  let whole = sqrt(1.0 + q2);
  // The sums `spur` and `fold` are the means of: the profile weighted by each octave's wall slope, and the round wave
  // weighted by that times its length; and `rill`'s, as `spur`'s.
  var sums = vec4f(0.0);
  var rills = vec2f(0.0);
  var wave = GULLY_LONGEST_M;
  var k = GULLY_K;
  var rot = mat2x2f(cos(TURN_A), sin(TURN_A), -sin(TURN_A), cos(TURN_A));
  for (var o = 0; o < GULLY_OCTAVES + RILL_OCTAVES; o++) {
    let shown = shows_at(GULLY_SHOW * wave, e);
    if (shown <= 0.0) { break; }
    // The fall line so far, never back up the ground's; the unit vector across it.
    let g = gg.slope + keep_falling(gg.slope, out.slope, KEEP_FALL);
    let across = vec2f(-g.y, g.x) / max(length(g), 1e-9);
    let w = gully_wave(wave, across, rot, GULLY_STREAM + o);
    // Made whole: c' = whole c / sqrt(c^2 + s^2 + q^2), with its slope.
    let rr = sqrt(dot(w.cs, w.cs) + q2);
    let c = clamp(whole * w.cs.x / rr, -1.0, 1.0);
    // (r dr, the change of r^2 / 2.)
    let rdr = w.cs.x * w.dc + w.cs.y * w.ds;
    let dc = whole * (w.dc / rr - w.cs.x * rdr / (rr * rr * rr));
    // The fade at a fork, smooth in r (its slope from r's), and its slope.
    let r = length(w.cs);
    let t = clamp(r / FORK_FADE, 0.0, 1.0);
    let fork = t * t * (3.0 - 2.0 * t);
    let dfork = select(vec2f(0.0), 6.0 * t * (1.0 - t) / FORK_FADE * rdr / max(r, 1e-9), r < FORK_FADE);
    let rounded = ease(ROUND_LONG_M.x, ROUND_LONG_M.y, wave);
    let p = gully_profile(c, (1.0 - rounded) * (1.0 - SOIL_ROUND * soil), sharp * (1.0 - rounded), max(SHARP_FLOOR, PI * CREASE_PX * e / wave));
    var wall = k * depth * (1.0 - SOIL_SHORT_CUT * soil * ease(SOIL_SHORT_M.x, SOIL_SHORT_M.y, wave)) * select(1.0, RILL_K, o >= GULLY_OCTAVES);
    wall = wall / sqrt(1.0 + (wall / WALL_MOST) * (wall / WALL_MOST));
    let a = shown * wall * wave / (2.0 * PI);
    out.h += a * fork * (p.x - 0.5);
    out.slope += a * (fork * p.y * dc + (p.x - 0.5) * dfork);
    if (o < GULLY_OCTAVES) { sums += shown * wall * fork * vec4f(p.x, 1.0, wave * 0.5 * (1.0 + c), wave); }
    else { rills += shown * wall * fork * vec2f(p.x, 1.0); }
    // Its wash: how near the floor's middle the point lies, m (near it t = (1 + c) / 2 is about (phase / 2)^2), against
    // its half width spread over the pixel.
    if (o < GULLY_OCTAVES && wave <= WASH_LONG_M) {
      let half = WASH_SHARE * wave;
      let spread = max(half, WASH_PX * e);
      let off = sqrt(0.5 * (1.0 + c)) * wave / PI;
      let line = (1.0 - smoothstep(0.5 * spread, spread, off)) * half / spread;
      out.wash = max(out.wash, shown * fork * ease(0.0, WASH_WALL, wall) * line);
    }
    wave /= GULLY_STEP;
    k *= GULLY_DECAY;
    rot = OCTAVE_TURN * rot;
  }
  out.slope = keep_falling(gg.slope, out.slope, KEEP_FALL);
  let cut = ease(0.0, GULLY_SEEN, sums.y);
  out.spur = 0.5 + (sums.x / max(sums.y, 1e-9) - 0.5) * cut;
  out.fold = 0.5 + (sums.z / max(sums.w, 1e-9) - 0.5) * cut;
  out.rill = 0.5 + (rills.x / max(rills.y, 1e-9) - 0.5) * ease(0.0, RILL_SEEN, rills.y);
  return out;
}
