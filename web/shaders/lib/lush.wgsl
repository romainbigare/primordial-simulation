// How lush the ground is inside a cell, for the plants that stand on it (cover_field.wgsl, mark.wgsl) and the grain
// that wash sorts on it (rock.wgsl `grain_at`).

// Inside a cell the plants gather where water gathers and the sun dries the ground least, as seen from a plane: along
// the drainage lines and in the hollows, on the slopes turned from the sun, and thin out on the ridges, the knobs and
// the sunny sides (drawing only: the core knows a cell's mean and its saturated strips). How lush a point is, -1..1
// (`lush_of`): the tanh of its wetness index over `LUSH_TWI_MID` times `LUSH_TWI` a unit, its rise toward the light
// times `LUSH_SUN` (as snow lies longest there, land.wgsl `snow_bares`), the noise of the samples' places (rock.wgsl
// `Place.n`, the patches of a mantle from a kilometre down) times `LUSH_PLACE`, how low it lies in its relief's noise
// (a hollow's depth over `LUSH_LOW_M`, relief.wgsl), and how low it lies in its gullies (gullies.wgsl `Gullies`, each
// 0 a floor .. 1 a crest): among its gullies of every size times `LUSH_SPUR`, in their broad hollows times `LUSH_FOLD`
// (a gully is tens of metres deep: it is read by its place across it, not by its depth); about none on the mean, so a
// cell keeps its cover. The share the plants cover moves by up to `LUSH` of its room either way by it (`lushed`), and
// the herbs' colour turns by `LUSH_HUE` of itself, greener where lush, straw where dry. (The wetness index of the
// tiles' texels lies within about 6 to 9 on half the land.)
const LUSH_TWI_MID = 7.5; const LUSH_TWI = 0.8; const LUSH_SUN = 3.0; const LUSH_PLACE = 0.6; const LUSH_LOW_M = 10.0;
const LUSH_SPUR = 0.8; const LUSH_FOLD = 1.2;
const LUSH = 1.0; const LUSH_HUE = vec3f(-0.25, -0.08, 0.06);

// How lush a point is (`LUSH_TWI`): of wetness index `twi` (land.wgsl `Tiled.twi`), slope `slope` (east, south; rise
// over run), the noise of its samples' place `n` (rock.wgsl `Place.n`), `low` m under its relief's mean (0 where not
// known) and its place in its gullies `cut` (`Gullies.spur`, `.fold`; 0.5 each where not known).
// (Its sum is held within `LUSH_SUM_MOST` either way before the tanh: a channel's texels hold an index past any
// float's exp, and a tanh worked out by exp gives no number there.)
const LUSH_SUM_MOST = 10.0;
fn lush_of(twi: f32, slope: vec2f, n: f32, low: f32, cut: vec2f) -> f32 {
  let gully = LUSH_SPUR * (1.0 - 2.0 * cut.x) + LUSH_FOLD * (1.0 - 2.0 * cut.y);
  let sum = LUSH_TWI * (twi - LUSH_TWI_MID) - LUSH_SUN * sunward(slope) + LUSH_PLACE * n + low / LUSH_LOW_M + gully;
  return tanh(clamp(sum, -LUSH_SUM_MOST, LUSH_SUM_MOST));
}
// The share of the ground plants that cover `s` of it on the mean (0..1) cover where it is as lush as `lush`
// (`lush_of`): moved by up to `LUSH` of s (1 - s) either way, so it stays within 0..1, a closed cover gains no more
// than it has room for, and the mean is kept.
fn lushed(s: f32, lush: f32) -> f32 { return s * lush_thick(s, lush); }
// How many times as dense as their mean plants that cover `s` of the ground on the mean stand there (`lushed`).
fn lush_thick(s: f32, lush: f32) -> f32 { return 1.0 + LUSH * lush * (1.0 - clamp(s, 0.0, 1.0)); }

// What the herbs' colour is multiplied by where they are as lush as `lush` (`LUSH_HUE`).
fn lush_hue(lush: f32) -> vec3f { return 1.0 + LUSH_HUE * lush; }

// A sward is mottled in patches tens of metres across (`HERB_MOTTLE_M`, and a second octave `HERB_MOTTLE_STEP` times
// finer at `HERB_MOTTLE_FALL` of its weight), where its plants grow a little thicker or thinner, greener or paler (seen from a plane a
// meadow or a heath is a mottle of patches, from memory): what their colour is multiplied by at `at`, m, seen at `e` m
// a pixel: their tone by up to `HERB_MOTTLE` against how lush they look (`lush_hue` of `HERB_MOTTLE_LUSH` a unit), as
// far as each octave shows (`shows_at`); about none on the mean.
const HERB_MOTTLE = 0.12; const HERB_MOTTLE_LUSH = 0.6; const HERB_MOTTLE_M = 45.0; const HERB_MOTTLE_STEP = 3.0;
const HERB_MOTTLE_FALL = 0.5; const HERB_MOTTLE_STREAM = 266;
fn herb_mottle(at: vec2f, e: f32) -> vec3f {
  var n = 0.0;
  var size = HERB_MOTTLE_M;
  var weight = 1.0;
  for (var o = 0; o < 2; o++) {
    let shows = shows_at(size, e);
    if (shows > 0.0) { n += weight * shows * noise_turned(at / size, HERB_MOTTLE_STREAM + o); }
    size /= HERB_MOTTLE_STEP;
    weight *= HERB_MOTTLE_FALL;
  }
  return (1.0 - HERB_MOTTLE * n) * lush_hue(HERB_MOTTLE_LUSH * n);
}

// Herbs on a slope stand in drifts down its fall line, wetter and greener in the paths the water takes, drier and
// paler between them (rills and runnels sort a sward on a hillside: from memory): what their colour is multiplied by at
// `at`, m, seen at `e` m a pixel, on ground of slope `slope` (rise over run, east and south): their tone moves by up to
// `HERB_DRIFT` against how lush they look (`lush_hue` of `HERB_DRIFT_LUSH` a unit) over noise `HERB_DRIFT_M` m across
// and `HERB_DRIFT_LONG` times that long down the fall line, as the slope rises to `HERB_DRIFT_STEEP` (none on flat
// ground), as far as it shows (`shows_at`). The noise is laid along four fixed ways (`HERB_DRIFT_WAYS`, over a half
// turn), the two nearest the fall line blended: a noise turned or stretched pixel by pixel would swirl, or shift by
// whole cells over a small turn so far from the map's corner.
const HERB_DRIFT = 0.1; const HERB_DRIFT_LUSH = 0.5; const HERB_DRIFT_M = 6.0; const HERB_DRIFT_LONG = 4.0;
const HERB_DRIFT_STEEP = 0.25; const HERB_DRIFT_WAYS = 4; const HERB_DRIFT_STREAM = 262;
fn herb_drift(at: vec2f, e: f32, slope: vec2f) -> vec3f {
  let shows = shows_at(HERB_DRIFT_M, e) * ease(0.0, HERB_DRIFT_STEEP, length(slope));
  if (shows <= 0.0) { return vec3f(1.0); }
  // The fall line's way in steps of the ways, over a half turn.
  let way = fract(atan2(slope.y, slope.x) / PI + 1.0) * f32(HERB_DRIFT_WAYS);
  let first = floor(way);
  var n = 0.0;
  for (var k = 0; k < 2; k++) {
    let w = (first + f32(k)) % f32(HERB_DRIFT_WAYS);
    let turn = PI * w / f32(HERB_DRIFT_WAYS);
    let along = vec2f(cos(turn), sin(turn));
    let q = vec2f(dot(at, along) / HERB_DRIFT_LONG, dot(at, vec2f(-along.y, along.x))) / HERB_DRIFT_M;
    n += select(way - first, 1.0 - (way - first), k == 0) * noise(q, HERB_DRIFT_STREAM + i32(w)).x;
  }
  return (1.0 - HERB_DRIFT * shows * n) * lush_hue(HERB_DRIFT_LUSH * shows * n);
}
