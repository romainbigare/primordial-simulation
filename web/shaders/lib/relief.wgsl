// The ground's relief finer than its tiles (land.wgsl): the terrain's samples stand 125 m apart and the tiles draw the
// ground between them smooth, so a sample's worth of ground would be one soft swell. Here the drawing adds detail at
// every scale from `RIB_LONGEST_M` down to the pixel, fixed in the world, each scale fading out as it shrinks to a few
// pixels (draw.wgsl `shows_at`; it would only shimmer), so its cost does not grow with the zoom. From the samples'
// scale down to the noise's longest octave (`RELIEF_LONGEST_M`) the steep ground is cut into ribs and gullies that run
// down its fall line (`ribs`): the water that runs down a slope cuts it into parallel gullies spaced tens to hundreds
// of metres, with spurs between them (Horton 1945; Montgomery and Dietrich 1992), and a gentle plain is not cut. Below
// that, octaves of gradient noise each `RELIEF_STEP` times finer (draw.wgsl `gradient_noise`, whose slope runs on
// smoothly across its cells, so the light shows no grid), each turned from the last so they stand in no grid. Their
// strength follows the world's numbers: deep loose ground creeps into smooth swells (rise per run `SWELL`), bare bed is
// rough (`CRAG`), its octaves ridged into crests and ledges, the more the steeper (a fractal surface whose slope is
// alike at every scale, as measured ground is over metres to kilometres: Mark and Aronson 1984). It moves the shading's
// normal and the fine line of every edge it meets (a shore wiggles at the metre scale), not the physics. Deterministic
// by world position: nothing moves as the view pans or zooms.

// The gullies' cells: the longest, m, and their number, each `RELIEF_STEP` times shorter. In a cell `RIB_ACROSS`
// gullies stand side by side across the fall line, so a gully runs on longer than the gullies are apart.
const RIB_LONGEST_M = 432.0; const RIB_OCTAVES = 2; const RIB_ACROSS = 1.5;
// The slope the gullies' sides take, as a share of the ground's own slope, that slope counted up to `RIB_STEEP_MOST`
// (rise over run); none on ground gentler than `RIB_SLOPES.x`, all from `RIB_SLOPES.y`: a plain is not cut. Each
// cell's gullies turn up to `RIB_TILT` radians either way from the fall line (they wander and branch) and are cut to a
// depth of its own, from none to all, most of them shallow (a slope is cut unevenly); noise stream of the first
// octave. A gully's V floor is a crease: its wave shows fully from `RIB_PX` pixels a wave, none under a third of that
// (`shows_at`).
const RIB = 0.45; const RIB_STEEP_MOST = 1.0; const RIB_SLOPES = vec2f(0.15, 0.6); const RIB_TILT = 0.5;
const RIB_STREAM = 230; const RIB_PX = 8.0;

// The gullies on ground of slope `slope` (east, south; rise over run) seen at `e` m a pixel: their height, m, and their
// slope east and south. In each cell of a turned lattice a wave runs across the fall line, of its own phase and turn
// (hashed from the cell: arithmetic, no texture read), and the four about a point are blended with the quintic weights
// of gradient noise, so a gully runs on for about a cell and fades into its neighbours'. The wave's absolute value is
// taken: its zeros are V-shaped gully floors, its crests the round spurs between them.
fn ribs(e: f32, slope: vec2f) -> vec3f {
  let steep = length(slope);
  let k = RIB * min(steep, RIB_STEEP_MOST) * ease(RIB_SLOPES.x, RIB_SLOPES.y, steep);
  var out = vec3f(0.0);
  if (k <= 0.0) { return out; }
  let across = vec2f(-slope.y, slope.x) / steep;
  var cell = RIB_LONGEST_M;
  var rot = mat2x2f(cos(TURN_A), sin(TURN_A), -sin(TURN_A), cos(TURN_A));
  let turn = mat2x2f(0.8, 0.6, -0.6, 0.8);
  for (var o = 0; o < RIB_OCTAVES; o++) {
    let wave = cell / RIB_ACROSS;
    let shown = shows_at(0.25 * RIB_PX * wave, e);
    if (shown <= 0.0) { break; }
    let l = lat(rot * (1.0 / cell), vec2f(0.0));
    let f = l.f;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
    // The wave's value, and its slope in the world's axes, per metre.
    var h = 0.0;
    var dh = vec2f(0.0);
    for (var j = 0; j < 4; j++) {
      let corner = vec2f(f32(j % 2), f32(j / 2));
      let word = lowbias(l.i + vec2i(corner), RIB_STREAM + o);
      let phase = 2.0 * PI * f32(word & 0x3ffu) / 1024.0;
      let tilt = RIB_TILT * (f32((word >> 10u) & 0x3ffu) / 512.0 - 1.0);
      let cut = pow(f32(word >> 20u) / 4096.0, 2.0);
      let dir = vec2f(across.x * cos(tilt) - across.y * sin(tilt), across.x * sin(tilt) + across.y * cos(tilt));
      let theta = 2.0 * PI * dot(transpose(rot) * (f - corner) * cell, dir) / wave + phase;
      // The corner's weight and its slope along the lattice's axes.
      let wx = select(1.0 - u.x, u.x, corner.x > 0.0);
      let wy = select(1.0 - u.y, u.y, corner.y > 0.0);
      let dw = vec2f(select(-du.x, du.x, corner.x > 0.0) * wy, wx * select(-du.y, du.y, corner.y > 0.0));
      h += cut * wx * wy * cos(theta);
      dh += cut * (transpose(rot) * dw * (cos(theta) / cell) - wx * wy * sin(theta) * (2.0 * PI / wave) * dir);
    }
    // A wave of height `wave / 2 pi` per unit has slopes up to one: its sides slope at `k`.
    let a = shown * k * wave / (2.0 * PI);
    out += a * vec3f(abs(h) - 0.25, sign(h) * dh);
    cell /= RELIEF_STEP;
    rot = turn * rot;
  }
  return out;
}

// The longest octave's wavelength, m, the ratio between octaves, and their number.
const RELIEF_LONGEST_M = 48.0; const RELIEF_STEP = 3.0; const RELIEF_OCTAVES = 6;
// The slope each octave adds (its height over its wavelength): on deep loose ground, on bare bed, and more on bare bed
// per unit of its slope (a face is rougher than a slab).
const SWELL = 0.025; const CRAG = 0.09; const CRAG_STEEP = 0.12;

// The relief at the pixel's place (`relief`): its height, m, and its slope east and south; and how low the point lies
// among its hollows (its octaves `HOLLOW_M` long) and among its basins (its octaves `BASIN_M` long, colour.wgsl
// `puddles`), each as far as its octaves show, 0 the lowest to 1 the highest, even over 0..1 (draw.wgsl `EVEN_ONE`;
// 0.5 where none shows).
struct Relief { h: f32, slope: vec2f, hollow: f32, basin: f32 }

// How low the octaves' sum `n` of weights whose squares sum to `w2` lies, even over 0..1 (0.5 for none).
fn rank_of(n: f32, w2: f32) -> f32 { return smoothstep(-EVEN_ONE, EVEN_ONE, NOISE_GAIN * n / sqrt(max(w2, 1e-12))); }

// The relief at the pixel's place, seen at `e` m a pixel, on ground of slope `slope` (east, south; rise over run) whose
// bed shows by `bare` (0..1, rock.wgsl `bed_shows`) under `loose` m of loose ground.
fn relief(e: f32, slope: vec2f, bare: f32, loose: f32) -> Relief {
  var out = ribs(e, slope);
  // No octave of the noise shows: none of it is read (the fitted view pays nothing).
  if (shows_at(RELIEF_LONGEST_M, e) <= 0.0) { return Relief(out.x, out.yz, 0.5, 0.5); }
  let steep = length(slope);
  let rough = max(bare, 1.0 - ease(0.0, SMOOTH_LOOSE_M, loose));
  let k = mix(SWELL, CRAG + CRAG_STEEP * min(steep, 2.0), rough);
  var wave = RELIEF_LONGEST_M;
  var rot = mat2x2f(1.0, 0.0, 0.0, 1.0);
  let turn = mat2x2f(0.8, 0.6, -0.6, 0.8);
  // Of the hollows' and the basins' octaves: the sum of each one's noise times its length, and of the lengths' squares.
  var ranks = vec4f(0.0);
  for (var o = 0; o < RELIEF_OCTAVES; o++) {
    let lengths = vec2f(select(0.0, wave, wave >= HOLLOW_M.x && wave <= HOLLOW_M.y), select(0.0, wave, wave >= BASIN_M.x && wave <= BASIN_M.y));
    ranks = vec4f(ranks.xy, ranks.zw + lengths * lengths);
    let shown = shows_at(wave, e);
    if (shown <= 0.0) { break; }
    let n = gradient_noise(lat(rot * (1.0 / wave), vec2f(0.0)), 70 + o);
    // Ridged on bare bed (1 - |n|: crests and ledges), smooth swells on loose ground; slope back in the world's axes.
    let ridge = vec3f(0.35 - abs(n.x), -sign(n.x) * n.yz);
    let v = mix(n, ridge, rough);
    out += shown * k * wave * vec3f(v.x, transpose(rot) * v.yz / wave);
    ranks = vec4f(ranks.xy + shown * v.x * lengths, ranks.zw);
    wave /= RELIEF_STEP;
    rot = turn * rot;
  }
  return Relief(out.x, out.yz, rank_of(ranks.x, ranks.z), rank_of(ranks.y, ranks.w));
}
