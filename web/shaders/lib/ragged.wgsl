// A limit on the ground is ragged, not a contour line of a smooth number. Where roots find the loose ground deep
// enough, where snow lies, the number departs from its smooth field over patches at
// every scale from the size of the things there down to the pixel: a mantle is thicker in hollows and thinner on
// knobs, drift fills the hollows. The limit is read on the number so moved, in the number's own units, so its zone
// is wide where the field changes slowly and narrow where it changes fast (never under `MEET_LEAST_M`, `meet`). The
// patches are octaves of noise fixed to the world, each `RAGGED_STEP` times finer, weighing `RAGGED_FALL` of the last.
// An octave under a pixel fades to its mean (draw.wgsl `shows_at`), and a rule read on the number is spread over the
// swing the pixel no longer shows: so a limit lies in the same place on the mean at every zoom.
// Two images of the ground meet by their heights (a photograph's lightness taken as its height): the tall parts of
// one show through the low parts of the other across the zone of their limit, not a fade and not a seam.
const RAGGED_STEP = 2.9; const RAGGED_FALL = 0.6;

// The patches at `at`, m from the map's corner, seen at `e` m a pixel: `octaves` octaves from `size` m down, noise
// streams from `k` on, -1..1 about 0. An octave too small to show reads no noise.
fn ragged(at: vec2f, e: f32, size: f32, octaves: i32, k: i32) -> f32 {
  var n = 0.0;
  var s = size;
  var weight = 1.0;
  var sum = 0.0;
  for (var o = 0; o < octaves; o++) {
    let shown = shows_at(s, e);
    if (shown > 0.0) { n += weight * shown * noise_turned(at / s + f32(o) * 7.3, k + o); }
    sum += weight;
    s /= RAGGED_STEP;
    weight *= RAGGED_FALL;
  }
  return clamp(2.0 * n / sum, -1.0, 1.0);
}

// How far a number swings over the octaves of patches from `size` m down that do not show at `e` m a pixel, as a share
// of the number times the patches' share: a rule read on the number is spread over that. No noise is read.
fn ragged_hidden(e: f32, size: f32, octaves: i32) -> f32 {
  var s = size;
  var weight = 1.0;
  var sum = 0.0;
  var hidden = 0.0;
  for (var o = 0; o < octaves; o++) {
    hidden += weight * (1.0 - shows_at(s, e));
    sum += weight;
    s /= RAGGED_STEP;
    weight *= RAGGED_FALL;
  }
  return 2.0 * hidden / sum;
}

// Where two looks of the ground meet (a patch of a cell's cover and the rest of it, snow and bare ground, a scar and
// the ground a fire spared) they meet over a zone, not along a line: the far look takes a point where its number past
// the limit, moved by the patches, passes none. The patches run from `size` m down to the pixel over up to
// `MEET_OCTAVES` octaves (the last a few centimetres across from a few hundred metres); those too small to show spread
// the limit over the swing they would make. So far away the zone is a fade, near it is ragged patches of both looks
// down to the pixel, and on the mean the limit lies in the same place at every zoom.
const MEET_OCTAVES = 10;

// The far look's share, 0..1, at a point `x` past the limit whose patches move it by up to `swing` (`ragged`, each to
// its own scale), both in the number's own units; `px` is how much `x` changes across a pixel (0 where not known). The
// zone is no narrower than `MEET_LEAST_M` either way, however fast the number changes: the things that make a limit on
// the ground (drifts, a fire's fingers, the stands thinning at a clearing's edge) reach tens of metres at least (from
// memory). Where the patches show, the share fades over `soft` of the zone's half width about their edge: 0 for a
// look that ends at an edge of its own (snow, char), 1 for one that thins out over the zone (the plants of a stand,
// the dead trees of a burn), the patches moving where it thins. The patches are read only at a point inside the zone.
const MEET_LEAST_M = 40.0;
fn meet(at: vec2f, e: f32, x: f32, swing: f32, soft: f32, px: f32, size: f32, k: i32) -> f32 {
  let half = max(swing, MEET_LEAST_M * px / max(e, 1e-9));
  let edge = max(max(half * max(0.5 * ragged_hidden(e, size, MEET_OCTAVES), soft), 0.5 * px), 1e-6 * half);
  if (x >= half + edge) { return 1.0; }
  if (x <= -half - edge) { return 0.0; }
  return smoothstep(-edge, edge, x + half * ragged(at, e, size, MEET_OCTAVES, k));
}

// The number `x` moved by the patches `n` (`ragged`), up to `share` of itself either way.
fn with_patches(x: f32, n: f32, share: f32) -> f32 { return x * (1.0 + share * n); }

// An image's claim at a point where images meet: how far inside its own part the point lies, in half widths of the
// zone where they meet, plus its height (its lightness over its mean, 1 on the mean), `HEIGHT_RANGE` of lightness
// counting one half width, at most `HEIGHT_MOST` half widths either way.
const HEIGHT_RANGE = 0.4; const HEIGHT_MOST = 1.0;
fn claim(inside: f32, height: f32) -> f32 { return inside + clamp((height - 1.0) / HEIGHT_RANGE, -HEIGHT_MOST, HEIGHT_MOST); }
// The weight of a claim against the highest at the point (to be shared out over the images): claims within
// `HEIGHT_SHARP` half widths of the highest blend, so an image's edge runs along the shapes in it, a texel or two wide.
const HEIGHT_SHARP = 0.15;
fn claim_weight(c: f32, top: f32) -> f32 { return 1.0 - smoothstep(0.0, 2.0 * HEIGHT_SHARP, top - c); }
// An image that lies past this many half widths outside its part claims no point (it need not be read).
const CLAIMS_WITHIN = 2.0 * (HEIGHT_MOST + HEIGHT_SHARP);

// The weight of image b over image a, the point lying `d` half widths into b's part from the limit between them,
// their heights `ha` and `hb`.
fn height_mix(d: f32, ha: f32, hb: f32) -> f32 {
  let ca = claim(0.0, ha);
  let cb = claim(d, hb);
  let top = max(ca, cb);
  let wb = claim_weight(cb, top);
  return wb / (wb + claim_weight(ca, top));
}
