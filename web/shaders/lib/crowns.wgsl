// Crowns seen from above, one rule far and near: the field's specks and grain (cover_field.wgsl `lattice_field`) and
// the marks' sprites (models.wgsl `model_plant`) light a crown alike, so a stand keeps its grain as its crowns pass from
// specks to single trees.

// A crown is a dome lit by the low sun (`LIGHT`): a point `d` of it (from its middle, in its radius, east and south)
// is lit by its normal's turn toward the light about its mean over the dome (Lambert), -1 (its side away from the sun)
// .. 1 (its side toward it), and a crown's colour moves by up to `DOME_LIT` of itself either way with it; its rim is
// darker by up to `DOME_RIM` (its leaves thin out there and show the shade inside it), about none on the mean.
const DOME_LIT = 0.45; const DOME_RIM = 0.25;
fn dome_lit(d: vec2f) -> f32 { return dome_lambert(d) - 2.0 / 3.0 * LIGHT.z; }
// What a crown's colour is multiplied by at point `d` of it (`dome_lit`).
fn dome_tone(d: vec2f) -> f32 { return 1.0 + DOME_LIT * dome_lit(d) - DOME_RIM * (min(dot(d, d), 1.0) - 0.5); }

// Things too small to be drawn one by one but a pixel or more across (crowns and dead trees seen from afar) are drawn
// as round specks about `w` m across that cover the share `share` of the ground on the mean: on lattices fixed in the
// world whose cells are `SPECK_BASE_M` times a power of two (above or below it: a young plant's speck is as small as
// its crown), each turned its own way (`SPECK_TURN` radians an octave, so no rows line up), each cell as wide as its
// specks, one speck a cell at most at a place of its own draw (`SPECK_JITTER` of its cell either way), in as many cells
// as the share asks (past what specks in every cell cover, the rest lies evenly between them); the two lattices whose
// cells are nearest `w` blended by how near, so a speck's size follows the plants' with no seam. A point reads the four
// cells nearest it. They melt into their mean, the share evenly, as they shrink from `SPECK_PX`.y to `.x` pixels
// across (draw.wgsl), and their light and their own colour from `SPECK_LOOK_PX`.y to `.x` (on a speck of a pixel or two they would
// read as noise). Of the point: the specks' cover; what their colour is multiplied by (`dome_tone`, 1 between them); and
// the speck's own draw, even over 0..1 (its kind's, `stand_share`; 0.5 between them), and how far its own colour shows.
// No speck is wider than `SPECK_MOST_M` (a mark is at most 10 m across: lattices.ts `marksOf`): in a program made for
// draws whose pixels are wider than a speck (draw.wgsl `LEAST_E`) they are compiled out.
const SPECK_BASE_M = 1.0; const SPECK_TURN = 0.9; const SPECK_JITTER = 0.4;
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

// Crowns stand in groups: a closed stand seen from above is a top of round, bumpy blobs of foliage, each a dome lit on
// its side toward the sun and dark on the other, with thin dark gaps between them, of clearly unequal sizes (a few big
// emergent crowns, many middle ones, small ones in the gaps), each its own lightness and hue (from memory: an aerial
// photograph of a wood shows its crowns 10 to 15 % apart in tone and hue). Such blobs about `GRAIN_CROWNS` times the
// stand's mean crown across, and some `GRAIN_EMERGENT` of them twice as wide over them (`grain_domes`), and their own
// groups `GRAIN_STEP` times wider, light the canopy's colour by `GRAIN` of a dome's change (`dome_tone`; the second
// size by `GRAIN_FALL` of that), the shade between them `GRAIN_GAP` darker; each dome lighter or darker by up to
// `GRAIN_TREE_TONE` and yellower or bluer by up to `GRAIN_TREE_HUE` (`GRAIN_HUE` a unit) by its own draw. They show as
// far as a dome takes from `GRAIN_PX`.x pixels across, fully from `.y` (a smaller dome reads as noise). A size lies on
// the specks' lattices of `SPECK_BASE_M` times a power of two (`speck_lattice`, every cell holding one), the two about
// it blended over the middle `GRAIN_BLEND` of the octave between them (a lattice whose cells followed the width would
// shift by whole cells over a small change of it and show noise; a wider blend lays two sizes of domes over each other
// and blurs both). About none on the mean (the domes cover about `GRAIN_ON` of their ground, measured). Of a canopy of
// crowns `w` m across at `at`, m, seen at `e` m a pixel: what its colour is multiplied by, and how much of the point
// its blobs of the first size cover (`CanopyGrain`). The field far away (cover_field.wgsl `lattice_field`) and the
// marks near (cover.wgsl `marks_over`) lay it alike, so a stand keeps its blobs from the field to the single trees.
const GRAIN_CROWNS = 1.0; const GRAIN_STEP = 4.0; const GRAIN = 0.8; const GRAIN_FALL = 0.7; const GRAIN_GAP = 0.3;
const GRAIN_ON = 0.67; const GRAIN_PX = vec2f(2.0, 4.0); const GRAIN_STREAM = 250; const GRAIN_BLEND = 0.3;
const GRAIN_EMERGENT = 0.25;
const GRAIN_TREE_TONE = 0.12; const GRAIN_TREE_HUE = 0.12; const GRAIN_HUE = vec3f(0.6, 0.15, -0.9);
struct CanopyGrain { tone: vec3f, on: f32 }
fn canopy_grain(at: vec2f, w: f32, e: f32) -> CanopyGrain {
  var out = CanopyGrain(vec3f(1.0), 0.0);
  var size = GRAIN_CROWNS * w;
  var weight = GRAIN;
  for (var o = 0; o < 2; o++) {
    let shows = ease(GRAIN_PX.x * e, GRAIN_PX.y * e, size);
    if (shows > 0.0) {
      let octave = log2(size / SPECK_BASE_M);
      let low = floor(octave);
      let f = smoothstep(0.5 - 0.5 * GRAIN_BLEND, 0.5 + 0.5 * GRAIN_BLEND, octave - low);
      var g = vec4f(0.0);
      for (var n = 0; n < 2; n++) {
        let wn = select(1.0 - f, f, n == 1);
        if (wn > 0.0) { g += wn * grain_domes(at, low + f32(n), e, GRAIN_STREAM + 6 * n + 12 * o); }
      }
      out.tone += weight * shows * g.rgb;
      if (o == 0) { out.on = shows * g.a; }
    }
    size *= GRAIN_STEP;
    weight *= GRAIN_FALL;
  }
  out.tone = max(out.tone, vec3f(0.0));
  return out;
}
// The domes of `canopy_grain` on the lattice of octave `at_n` (`SPECK_BASE_M` times 2 to its power), with
// `GRAIN_EMERGENT` of the cells of the next lattice holding a dome twice as wide over them, seen at `e` m a pixel,
// streams from `k`: the change they make to a canopy's colour, and their cover.
fn grain_domes(at: vec2f, at_n: f32, e: f32, k: i32) -> vec4f {
  let one = speck_lattice(at, SPECK_BASE_M * exp2(at_n), SPECK_TURN * at_n, 0.25 * PI, e, k);
  let big = speck_lattice(at, SPECK_BASE_M * exp2(at_n + 1.0), SPECK_TURN * (at_n + 1.0), 0.25 * PI * GRAIN_EMERGENT, e, k + 3);
  // (The big dome's draw is under `GRAIN_EMERGENT`: made even over 0..1 again.)
  let is_big = big.on > 0.5;
  let draw = select(one.own, big.own / GRAIN_EMERGENT, is_big);
  let own = vec2f(draw, fract(draw * 61.7)) * 2.0 - 1.0;
  let tree = GRAIN_TREE_TONE * own.x + GRAIN_TREE_HUE * own.y * GRAIN_HUE;
  let on = max(one.on, big.on);
  return vec4f(select(one.tone, big.tone, is_big) - 1.0 + GRAIN_GAP * (on - GRAIN_ON) + on * tree, on);
}
