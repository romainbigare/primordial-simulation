// Crowns seen from above, one rule far and near: the field's specks and grain (cover_field.wgsl `lattice_field`) and
// the marks' sprites (models.wgsl `model_plant`) light a crown alike, so a stand keeps its grain as its crowns pass from
// specks to single trees.

// A crown is a dome lit by the low sun (`LIGHT`): a point `d` of it (from its middle, in its radius, east and south)
// is lit by its normal's turn toward the light about its mean over the dome (Lambert), -1 (its side away from the sun)
// .. 1 (its side toward it), and a crown's colour moves by up to `DOME_LIT` of itself either way with it; its rim is
// darker by up to `DOME_RIM` (its leaves thin out there and show the shade inside it), about none on the mean.
const DOME_LIT = 0.45; const DOME_RIM = 0.25;
fn dome_lit(d: vec2f) -> f32 {
  let r2 = min(dot(d, d), 1.0);
  return dot(d, LIGHT.xy) + LIGHT.z * (sqrt(1.0 - r2) - 2.0 / 3.0);
}
// What a crown's colour is multiplied by at point `d` of it (`dome_lit`).
fn dome_tone(d: vec2f) -> f32 { return 1.0 + DOME_LIT * dome_lit(d) - DOME_RIM * (min(dot(d, d), 1.0) - 0.5); }

// Things too small to be drawn one by one but a pixel or more across (crowns and dead trees seen from afar) are drawn
// as round specks about `w` m across that cover the share `share` of the ground on the mean: on lattices fixed in the
// world whose cells are `SPECK_BASE_M` times a power of two (above or below it: a young plant's speck is as small as
// its crown), each turned its own way (`SPECK_TURN` radians an octave, so no rows line up), each cell as wide as its
// specks, one speck a cell at most at a place of its own draw (`SPECK_JITTER` of its cell either way), in as many cells
// as the share asks (past what specks in every cell cover, the rest lies evenly between them); the two lattices whose
// cells are nearest `w` blended by how near, so a speck's size follows the plants' with no seam. A point reads the four
// cells nearest it. They melt into their mean, the share evenly, as they shrink from `SPECK_PX`.y to `SPECK_PX`.x pixels
// across, and their light and their own colour from `SPECK_LOOK_PX`.y to `.x` (on a speck of a pixel or two they would
// read as noise). Of the point: the specks' cover; what their colour is multiplied by (`dome_tone`, 1 between them); and
// the speck's own draw, even over 0..1 (its kind's, `stand_share`; 0.5 between them), and how far its own colour shows.
// No speck is wider than `SPECK_MOST_M` (a mark is at most 10 m across: lattices.ts `marksOf`): in a program made for
// draws whose pixels are wider than a speck (draw.wgsl `LEAST_E`) they are compiled out.
const SPECK_BASE_M = 1.0; const SPECK_TURN = 0.9; const SPECK_JITTER = 0.4; const SPECK_PX = vec2f(1.0, 3.0);
const SPECK_LOOK_PX = vec2f(2.0, 4.0); const SPECK_MOST_M = 10.0;
struct Specks { on: f32, tone: f32, own: f32, look: f32 }
fn specks(at: vec2f, w: f32, share: f32, e: f32, k: i32) -> Specks {
  let none = Specks(min(share, 1.0), 1.0, 0.5, 0.0);
  if (SPECK_PX.x * least_e() >= SPECK_MOST_M) { return none; }
  let detail = specks_show(w, e);
  if (detail <= 0.0 || share <= 0.0) { return none; }
  let octave = log2(w / SPECK_BASE_M);
  let low = floor(octave);
  let fine = speck_lattice(at, SPECK_BASE_M * exp2(low), SPECK_TURN * low, share, e, k);
  let coarse = speck_lattice(at, SPECK_BASE_M * exp2(low + 1.0), SPECK_TURN * (low + 1.0), share, e, k + 3);
  let f = octave - low;
  let on = mix(fine.on, coarse.on, f);
  let tone = mix(fine.tone, coarse.tone, f);
  let look = ease(SPECK_LOOK_PX.x * e, SPECK_LOOK_PX.y * e, w);
  return Specks(mix(min(share, 1.0), on, detail), mix(1.0, tone, look), select(fine.own, coarse.own, f > 0.5), look);
}
// How fully specks `w` m across show at `e` m a pixel (`SPECK_PX`).
fn specks_show(w: f32, e: f32) -> f32 { return ease(SPECK_PX.x * e, SPECK_PX.y * e, w); }
// The specks of `specks` on the lattice of cells `side` m, as wide as its cells, turned `turn` radians.
fn speck_lattice(at_m: vec2f, side: f32, turn: f32, share: f32, e: f32, k: i32) -> Specks {
  let full = 0.25 * PI;
  let chance = share / full;
  let r = 0.5 * side;
  let turned = mat2x2f(cos(turn), sin(turn), -sin(turn), cos(turn));
  let at = turned * at_m;
  let near = floor(at / side - 0.5);
  var on = 0.0;
  var d_on = vec2f(1.0);
  var own = 0.5;
  for (var c = 0; c < 4; c++) {
    let cell = near + vec2f(f32(c % 2), f32(c / 2));
    let id = vec2i(cell);
    let draw = vec3f(f32(lowbias(id, k) & 1023u), f32(lowbias(id, k + 1) & 1023u), f32(lowbias(id, k + 2) & 1023u)) / 1023.0;
    if (draw.z >= chance) { continue; }
    let d = (at - (cell + 0.5 + SPECK_JITTER * (2.0 * draw.xy - 1.0)) * side) / r;
    let one = 1.0 - smoothstep(1.0 - 0.5 * e / r, 1.0 + 0.5 * e / r, length(d));
    if (one > on) { on = one; d_on = d; own = draw.z / max(chance, 1.0); }
  }
  let even = max(share - full, 0.0) / (1.0 - full);
  // (The dome's point turned back to the map's axes, so its light comes from the sun.)
  return Specks(on + (1.0 - on) * even, mix(1.0, dome_tone(d_on * turned), on), own, 1.0);
}

// Crowns stand in groups: a closed stand seen from above is a pebbly top of crowns and of their groups. Groups about
// `GRAIN_CROWNS` crowns across, and their own groups `GRAIN_STEP` times that, each a dome (`dome_tone`) over the
// specks' lattices (`speck_lattice`, every cell holding one; of each size the two lattices of `SPECK_BASE_M` times a
// power of two about it, blended by how near, as `specks`: a lattice whose cells followed the crowns' width would
// shift by whole cells over a small change of it, and show noise) with the shade between them `GRAIN_GAP` darker,
// light the canopy's colour by `GRAIN` of a dome's change, as far as each shows (from `GRAIN_PX`.x pixels across,
// fully from `.y`: a smaller dome reads as noise), the second weighing `GRAIN_FALL` of the first: about none on the
// mean (the domes cover about `GRAIN_ON` of their ground, measured). What the colour of a canopy of crowns `w` m
// across is multiplied by at `at`, m, seen at `e` m a pixel.
const GRAIN_CROWNS = 2.5; const GRAIN_STEP = 4.0; const GRAIN = 0.6; const GRAIN_FALL = 0.7; const GRAIN_GAP = 0.3;
const GRAIN_ON = 0.67; const GRAIN_PX = vec2f(2.0, 4.0); const GRAIN_STREAM = 250;
fn canopy_grain(at: vec2f, w: f32, e: f32) -> f32 {
  var tone = 1.0;
  var size = GRAIN_CROWNS * w;
  var weight = GRAIN;
  for (var o = 0; o < 2; o++) {
    let shows = ease(GRAIN_PX.x * e, GRAIN_PX.y * e, size);
    if (shows > 0.0) {
      let octave = log2(size / SPECK_BASE_M);
      let low = floor(octave);
      let f = octave - low;
      var g = 0.0;
      for (var n = 0; n < 2; n++) {
        let at_n = low + f32(n);
        let one = speck_lattice(at, SPECK_BASE_M * exp2(at_n), SPECK_TURN * at_n, 0.25 * PI, e, GRAIN_STREAM + 3 * n);
        g += select(1.0 - f, f, n == 1) * (one.tone - 1.0 + GRAIN_GAP * (one.on - GRAIN_ON));
      }
      tone += weight * shows * g;
    }
    size *= GRAIN_STEP;
    weight *= GRAIN_FALL;
  }
  return max(tone, 0.0);
}
