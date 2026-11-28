// The ground's relief finer than its tiles (land.wgsl): the terrain's samples stand 125 m apart and the tiles draw the
// ground between them smooth, so a sample's worth of ground would be one soft swell. Here the drawing adds detail at
// every scale from the gullies' longest octave (gullies.wgsl) down to the pixel, fixed in the world, each scale fading
// out as it shrinks to a few pixels (draw.wgsl `shows_at`; it would only shimmer), so its cost does not grow with the
// zoom. From kilometres down to a few metres steep ground is cut into branching gullies and the rills down their sides
// (`gullies`). From `RELIEF_LONGEST_M` down, octaves of gradient noise each `RELIEF_STEP` times finer (draw.wgsl
// `gradient_noise`, whose slope runs on smoothly across its cells, so the light shows no grid), each turned from the
// last by `OCTAVE_TURN` so they stand in no grid. Their strength follows the world's numbers: deep loose ground creeps
// into smooth swells (rise per run `SWELL`, more on a slope), bare bed is rough (`CRAG`), its octaves ridged into
// crests and ledges, the more the steeper (a fractal surface whose slope is alike at every scale, as measured ground is
// over metres to kilometres: Mark and Aronson 1984). It moves the shading's normal and the fine line of every edge it
// meets (a shore wiggles at the metre scale), not the physics. Deterministic by world position: nothing moves as the
// view pans or zooms.

// Each octave's lattice is turned from the last by this (a turn of about 37 degrees).
const OCTAVE_TURN = mat2x2f(0.8, 0.6, -0.6, 0.8);

// The longest octave's wavelength, m, the ratio between octaves, and their number.
const RELIEF_LONGEST_M = 48.0; const RELIEF_STEP = 3.0; const RELIEF_OCTAVES = 6;
// The slope each octave adds (its height over its wavelength): on deep loose ground, and more per unit of its slope
// (creep heaps a steep soil into lobes and steps), on bare bed, and more on bare bed per unit of its slope (a face is
// rougher than a slab).
const SWELL = 0.025; const SWELL_STEEP = 0.08; const CRAG = 0.09; const CRAG_STEEP = 0.12;

// The relief at the pixel's place (`relief`): its height, m, and its slope east and south; the height of its noise
// alone, m, and the length of that height's slope (`knob`: what a snow drift and the edge of a cast shade follow at the
// metre scale); how low the point lies
// among its hollows (its octaves `HOLLOW_M` long) and among its basins (its octaves `BASIN_M` long, colour.wgsl
// `puddles`), each as far as its octaves show, 0 the lowest to 1 the highest, even over 0..1 (draw.wgsl `EVEN_ONE`;
// 0.5 where none shows); how high it stands in its gullies and rills (`Gullies.spur`, `fold`, `rill`), and the share of
// it their washes take (`Gullies.wash`); and the forms the snow lies by (`drift`): the height of the noise's octaves from
// `DRIFT_FORM_M` long up, m, the slope of those and of the gullies, east and south, and the length of those octaves'
// slope. Snow drifts over smaller forms and does not trace them.
struct Relief { h: f32, slope: vec2f, knob: vec2f, hollow: f32, basin: f32, spur: f32, fold: f32, rill: f32, wash: f32, drift: vec4f }
const DRIFT_FORM_M = 4.0;

// How rough ground whose bed shows by `bare` under `loose` m of loose ground is, 0 (deep loose ground: smooth swells)
// .. 1 (bare bed: crests and ledges).
fn rough_of(bare: f32, loose: f32) -> f32 { return max(bare, 1.0 - ease(0.0, SMOOTH_LOOSE_M, loose)); }

// How low the octaves' sum `n` of weights whose squares sum to `w2` lies, even over 0..1 (0.5 for none).
fn rank_of(n: f32, w2: f32) -> f32 { return smoothstep(-EVEN_ONE, EVEN_ONE, NOISE_GAIN * n / sqrt(max(w2, 1e-12))); }

// The relief at the pixel's place, seen at `e` m a pixel, on ground `gg` (gullies.wgsl `GullyGround`).
fn relief(e: f32, gg: GullyGround) -> Relief {
  let cut = gullies(e, gg);
  // No octave of the noise shows: none of it is read (the fitted view pays nothing).
  if (shows_at(RELIEF_LONGEST_M, e) <= 0.0) { return Relief(cut.h, cut.slope, vec2f(0.0), 0.5, 0.5, cut.spur, cut.fold, cut.rill, cut.wash, vec4f(0.0, cut.slope, 0.0)); }
  // The noise's crags are as steep as the ground the gullies leave.
  let steep = length(gg.slope + cut.slope);
  let rough = rough_of(gg.bare, gg.loose);
  let k = mix(SWELL + SWELL_STEEP * min(steep, 1.0), CRAG + CRAG_STEEP * min(steep, 2.0), rough);
  var out = vec3f(0.0);
  var drift = vec3f(0.0);
  var wave = RELIEF_LONGEST_M;
  var rot = mat2x2f(1.0, 0.0, 0.0, 1.0);
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
    let added = shown * k * wave * vec3f(v.x, transpose(rot) * v.yz / wave);
    out += added;
    if (wave >= DRIFT_FORM_M) { drift += added; }
    ranks = vec4f(ranks.xy + shown * v.x * lengths, ranks.zw);
    wave /= RELIEF_STEP;
    rot = OCTAVE_TURN * rot;
  }
  return Relief(cut.h + out.x, cut.slope + out.yz, vec2f(out.x, length(out.yz)), rank_of(ranks.x, ranks.z), rank_of(ranks.y, ranks.w), cut.spur, cut.fold, cut.rill, cut.wash, vec4f(drift.x, cut.slope + drift.yz, length(drift.yz)));
}
