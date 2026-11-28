// Plants stand in clumps with gaps between them, not as an even wash: groves and thickets in a wood, single crowns at
// its edge, tussocks and bare gaps in a dry sward (plants of dry ground gather where they trap the water and the
// fines; seen from a plane a stand's edge breaks into clumps and then single plants: from memory). A share `s` of the
// ground at a point lies in clumps that cover `fill` of their ground (or `s`, where more), over the points whose rank,
// a number even over 0..1 fixed in the world, lies under the clumps' line (`clumped`), eased over the line's width
// either way: the line is set so the eased share keeps `s / fill` on the mean whatever the width (`clump_line`), so on
// the mean every share is kept, a share near all is a closed stand with a few gaps, a share near none a scatter of
// single clumps, and a width past the share's room melts the clumps into their mean. The trees take the low ranks,
// their crowns closing `CANOPY_FILL` of a grove, their edge eased over `CANOPY_EDGE` crowns (of `CANOPY_EDGE_M` m at
// least) either way (`crowned`: an edge is made of whole crowns, which the specks and the marks draw one by one, in front of it and
// back from it), the low plants the high ones (`low_rank`), closing `LOW_FILL` of a clump (their small plants' gaps
// show between them), so the sward and the bushes fill the gaps between the groves; a clump of low plants thins over
// at least `LOW_SOFT` of the rank either way (a sward's tussocks and a heath's bushes thin out into the bare ground
// about them, with no outline).
// Each clump's top is lit by its slope toward the light (`Clumps.bumps`), by `lit` of its colour a unit of it: the
// trees' by `CANOPY_BUMP`, the low plants' by `LOW_BUMP`; and its plants stand darker toward its heart and lighter
// toward its edge, by up to `CLUMP_TONE` either way (taller and denser where the clump is old, from memory). The marks
// near (mark.wgsl `marks_thick`) and the field far (cover_field.wgsl) read one rank, so a clump drawn far is the
// plants drawn near.
// The rank is made of `CLUMP_OCTAVES` octaves of gradient noise from `CLUMP_M` m down to about a crown, each
// `CLUMP_STEP` times finer, weighing `CLUMP_FALL` of the last and turned from it by `CLUMP_TURN` radians (so no grid
// shows), each as far as it shows at the size of a pixel (draw.wgsl `shows_at`), made even again over the octaves
// shown (relief.wgsl `rank_of`): an octave too small to show reads no noise. The share is eased over the rank's
// change across `CLUMP_PX` pixels, and no less than `CLUMP_HIDDEN` of the swing of the octaves that do not show, so a
// clump's edge shows no pixel and far away the clumps melt into their mean; they show fully once the longest octave
// does.
const CLUMP_M = 90.0; const CLUMP_STEP = 2.9; const CLUMP_FALL = 0.6; const CLUMP_OCTAVES = 4; const CLUMP_TURN = 0.7;
const CANOPY_FILL = 0.85; const LOW_FILL = 0.75; const CANOPY_BUMP = 0.5; const LOW_BUMP = 0.3; const LOW_SOFT = 0.3;
const CANOPY_EDGE = 1.5; const CANOPY_EDGE_M = 2.0; const CLUMP_TONE = 0.12;
const CLUMP_PX = 0.75; const CLUMP_HIDDEN = 0.35; const CLUMP_STREAM = 230;

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

// The trees' clumps `c` where their crowns are `w` m across: their edge eased over `CANOPY_EDGE` crowns either way at
// least, of no less than `CANOPY_EDGE_M` (in the rank's units, by its change across a metre).
fn crowned(c: Clumps, w: f32) -> Clumps {
  var out = c;
  out.soft = max(c.soft, CANOPY_EDGE * max(w, CANOPY_EDGE_M) * length(c.slope));
  return out;
}

// The line of clumps `c` whose plants cover `near` of the ground: the rank at the middle of their edge and the half
// width of its ease (`Clumps.edge`, and `.soft` no wider than the clumps' ground: a few small clumps are their plants,
// not a haze), set so the share eased linearly over it (`clump_ramp`) keeps their ground (`clump_ground`) on the mean
// over the rank, even over 0..1: the ramp's mean is the integral of the rank held to 0..1 over the ramp, which is
// quadratic where the ramp passes an end of the rank.
fn clump_line(c: Clumps, near: f32) -> vec2f {
  let g = clump_ground(c, near);
  let w = max(max(c.edge, min(c.soft, g)), 1e-4);
  let end = min(w, 0.25 / w);
  if (g <= end) { return vec2f(sqrt(4.0 * w * g) - w, w); }
  if (g >= 1.0 - end) { return vec2f(1.0 + w - sqrt(4.0 * w * max(1.0 - g, 0.0)), w); }
  return vec2f(select(g, 2.0 * w * g + 0.5 - w, w > 0.5), w);
}
// The share of its clumps' ground held at rank `rank` by line `l` (`clump_line`).
fn clump_ramp(rank: f32, l: vec2f) -> f32 { return clamp(0.5 + (l.x - rank) / (2.0 * l.y), 0.0, 1.0); }

// The share of the ground covered at a point of clumps `c` where the plants cover `far` of it as a field (their mean,
// where the clumps do not show) and `near` of it in clumps: `Clumps.fill` of their ground, eased about their line
// (`clump_line`).
fn clumped(c: Clumps, far: f32, near: f32) -> f32 {
  if (c.shows <= 0.0) { return far; }
  return mix(far, max(c.fill, near) * clump_ramp(c.rank, clump_line(c, near)), c.shows);
}
// The share of the ground clumps `c` take where their plants cover `near` of it.
fn clump_ground(c: Clumps, near: f32) -> f32 { return near / max(c.fill, near); }

// What the colour of plants in clumps `c` that cover `near` of the ground is multiplied by: their tops lit by their
// slope toward the light, darker toward a clump's heart (`CLUMP_TONE`, even over a clump: none on the mean).
fn clump_light(c: Clumps, near: f32) -> f32 {
  let line = clump_line(c, near).x;
  let heart = clamp((line - c.rank) / max(line, 1e-6), 0.0, 1.0);
  return max(1.0 - c.lit * dot(c.bumps, normalize(LIGHT.xy)) + CLUMP_TONE * c.shows * (1.0 - 2.0 * heart), 0.0);
}

// How far inside the clumps `c` whose plants cover `near` of the ground a point lies, m (under 0: outside; from their
// line, `clump_line`), and how much the clumps' edge there faces the sun, -1 (away) .. 1 (`LIGHT`).
fn clump_edge(c: Clumps, near: f32) -> vec2f {
  let line = clump_line(c, near).x;
  let d = length(c.slope);
  if (d <= 0.0) { return vec2f(select(-1e6, 1e6, line > c.rank), 0.0); }
  // (The rank rises out of the clumps: its slope points away from them.)
  return vec2f((line - c.rank) / d, dot(c.slope / d, normalize(LIGHT.xy)));
}
