// Plants stand in clumps with gaps between them, not as an even wash: groves and thickets in a wood, single crowns at
// its edge, tussocks and bare gaps in a dry sward (plants of dry ground gather where they trap the water and the
// fines; seen from a plane a stand's edge breaks into clumps and then single plants: from memory). A share `s` of the
// ground at a point lies in clumps that cover `fill` of their ground (or `s`, where more), over the points where the
// clumps' rank, a number even over 0..1 fixed in the world, lies under `s / fill` (`clumped`): so on the mean every
// share is kept, a share near all is a closed stand with a few gaps, and a share near none a scatter of single
// clumps. The trees take the low ranks, their crowns closing `CANOPY_FILL` of a grove, the low plants the high ones
// (`low_rank`), closing `LOW_FILL` of a clump (their small plants' gaps show between them), so the sward and the
// bushes fill the gaps between the groves; a clump of low plants thins over at least `LOW_SOFT` of the rank at its
// edge (a sward's tussocks and a heath's bushes thin out into the bare ground about them), a grove's edge is sharp.
// Each clump's top is lit by its slope toward the light (`Clumps.bumps`), by `lit` of its colour a unit of it: the
// trees' by `CANOPY_BUMP`, the low plants' by `LOW_BUMP`; and its plants stand darker toward its heart and lighter
// toward its edge, by up to `CLUMP_TONE` either way (taller and denser where the clump is old, from memory). The marks near (mark.wgsl `marks_thick`) and the field far
// (cover_field.wgsl) read one rank, so a clump drawn far is the plants drawn near.
// The rank is made of `CLUMP_OCTAVES` octaves of gradient noise from `CLUMP_M` m down to about a crown, each
// `CLUMP_STEP` times finer, weighing `CLUMP_FALL` of the last and turned from it by `CLUMP_TURN` radians (so no grid
// shows), each as far as it shows at the size of a pixel (draw.wgsl `shows_at`), made even again over the octaves
// shown (relief.wgsl `rank_of`): an octave too small to show reads no noise. The share is eased over the rank's
// change across `CLUMP_PX` pixels, and no less than `CLUMP_HIDDEN` of the swing of the octaves that do not show, so a
// clump's edge shows no pixel and far away the clumps melt into their mean; they show fully once the longest octave
// does. The ease is no wider than the share's room to none and to all (so on the mean it keeps the share), and the
// clumps melt into their mean as it would be from one to `CLUMP_BLUR` times wider than that room: a pixel that holds
// many small clumps of a thin share sees their mean, not a speck.
const CLUMP_M = 90.0; const CLUMP_STEP = 2.9; const CLUMP_FALL = 0.6; const CLUMP_OCTAVES = 4; const CLUMP_TURN = 0.7;
const CANOPY_FILL = 0.85; const LOW_FILL = 0.75; const CANOPY_BUMP = 0.5; const LOW_BUMP = 0.3; const LOW_SOFT = 0.3;
const CLUMP_TONE = 0.12;
const CLUMP_PX = 0.75; const CLUMP_HIDDEN = 0.35; const CLUMP_BLUR = 2.0; const CLUMP_STREAM = 230;

// The clumps at a point: its rank, even over 0..1, and its change across a metre east and south; how far a share
// read on it is eased either way, in its units; how fully the clumps show, 0 (their mean) .. 1; and the slope of the
// noise's relief across the octaves shown (each octave's slope across its own cell, weighed as the octaves), east and
// south: the bumps a canopy's top makes, in groups at every scale; the share of a clump's ground its plants cover, and
// how much its bumps light its colour (`clump_light`); the least ease at its edge, in the rank's units.
struct Clumps { rank: f32, slope: vec2f, edge: f32, shows: f32, bumps: vec2f, fill: f32, lit: f32, soft: f32 }

// The clumps at `at`, m from the map's north-west corner, seen at `e` m a pixel.
fn clumps_at(at: vec2f, e: f32) -> Clumps {
  var n = 0.0;
  var dn = vec2f(0.0);
  var bumps = vec2f(0.0);
  var shown2 = 0.0;
  var all2 = 0.0;
  var size = CLUMP_M;
  var w = 1.0;
  for (var o = 0; o < CLUMP_OCTAVES; o++) {
    all2 += w * w;
    let shown = w * shows_at(size, e);
    if (shown > 0.0) {
      let turn = CLUMP_TURN * f32(o);
      let rot = mat2x2f(cos(turn), sin(turn), -sin(turn), cos(turn));
      let q = rot * at / size;
      let i = floor(q);
      let g = gradient_noise(Lat(vec2i(i), q - i), CLUMP_STREAM + o);
      let slope = transpose(rot) * g.yz;
      n += shown * g.x;
      dn += shown * slope / size;
      bumps += shown * slope;
      shown2 += shown * shown;
    }
    size /= CLUMP_STEP;
    w *= CLUMP_FALL;
  }
  if (shown2 <= 0.0) { return Clumps(0.5, vec2f(0.0), 0.5, 0.0, vec2f(0.0), CANOPY_FILL, CANOPY_BUMP, 0.0); }
  let rank = rank_of(n, shown2);
  // The rank's change with the noise's sum: smoothstep's over the span `rank_of` reads.
  let x = NOISE_GAIN * n / sqrt(shown2);
  let t = clamp((x + EVEN_ONE) / (2.0 * EVEN_ONE), 0.0, 1.0);
  let slope = 6.0 * t * (1.0 - t) / (2.0 * EVEN_ONE) * NOISE_GAIN / sqrt(shown2) * dn;
  let hidden = sqrt(max(1.0 - shown2 / all2, 0.0));
  let edge = max(CLUMP_PX * e * length(slope), CLUMP_HIDDEN * hidden);
  return Clumps(rank, slope, max(edge, 1e-4), shows_at(CLUMP_M, e), bumps / sqrt(all2), CANOPY_FILL, CANOPY_BUMP, 0.0);
}

// The clumps of the low plants: the rank turned over (`Clumps`), so they take the gaps the trees leave.
fn low_rank(c: Clumps) -> Clumps { return Clumps(1.0 - c.rank, -c.slope, c.edge, c.shows, c.bumps, LOW_FILL, LOW_BUMP, LOW_SOFT); }

// The share of the ground covered at a point of clumps `c` where the plants cover `far` of it as a field (their mean,
// where the clumps do not show) and `near` of it in clumps: `Clumps.fill` of the points whose rank lies under its
// share of `near` (`clump_ground`).
fn clumped(c: Clumps, far: f32, near: f32) -> f32 {
  if (c.shows <= 0.0) { return far; }
  let full = max(c.fill, near);
  let ground = clump_ground(c, near);
  let room = min(ground, 1.0 - ground);
  if (room <= 0.0) { return mix(far, select(0.0, full, ground >= 1.0), c.shows); }
  let w = min(max(c.edge, c.soft), room);
  return mix(far, full * smoothstep(-w, w, ground - c.rank), c.shows * (1.0 - ease(1.0, CLUMP_BLUR, c.edge / room)));
}
// The share of the ground clumps `c` take where their plants cover `near` of it.
fn clump_ground(c: Clumps, near: f32) -> f32 { return near / max(c.fill, near); }

// What the colour of plants in clumps `c` that cover `near` of the ground is multiplied by: their tops lit by their
// slope toward the light, darker toward a clump's heart (`CLUMP_TONE`, even over a clump: none on the mean).
fn clump_light(c: Clumps, near: f32) -> f32 {
  let ground = clump_ground(c, near);
  let heart = clamp((ground - c.rank) / max(ground, 1e-6), 0.0, 1.0);
  return max(1.0 - c.lit * dot(c.bumps, normalize(LIGHT.xy)) + CLUMP_TONE * c.shows * (1.0 - 2.0 * heart), 0.0);
}

// How far inside the clumps of share `s` of the ground (`clump_ground`) a point of clumps `c` lies, m (under 0:
// outside), and how much the clumps' edge there faces the sun, -1 (away) .. 1 (`LIGHT`).
fn clump_edge(c: Clumps, s: f32) -> vec2f {
  let d = length(c.slope);
  if (d <= 0.0) { return vec2f(select(-1e6, 1e6, s > c.rank), 0.0); }
  // (The rank rises out of the clumps: its slope points away from them.)
  return vec2f((s - c.rank) / d, dot(c.slope / d, normalize(LIGHT.xy)));
}
