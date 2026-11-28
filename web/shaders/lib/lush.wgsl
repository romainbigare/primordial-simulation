// How lush the ground is inside a cell, for the plants that stand on it (cover_field.wgsl, mark.wgsl) and the grain
// that wash sorts on it (rock.wgsl `soil_at`).

// Inside a cell the plants gather where water gathers and the sun dries the ground least, as seen from a plane: along
// the drainage lines and in the hollows, on the slopes turned from the sun, and thin out on the ridges, the knobs and
// the sunny sides (drawing only: the core knows a cell's mean and its saturated strips). How lush a point is, -1..1
// (`lush_of`): the tanh of its wetness index over `LUSH_TWI_MID` times `LUSH_TWI` a unit, its rise toward the light
// times `LUSH_SUN` (as snow lies longest there, land.wgsl `snow_bares`), the noise of the samples' places (rock.wgsl
// `Place.n`, the patches of a mantle from a kilometre down) times `LUSH_PLACE`, and how low it lies in its relief (a
// hollow's depth over `LUSH_LOW_M`, relief.wgsl); about none on the mean, so a cell keeps its cover. The share the
// plants cover moves by up to `LUSH` of its room either way by it (`lushed`), and the herbs' colour turns by `LUSH_HUE` of itself, greener
// where lush, straw where dry. (The wetness index of the tiles' texels lies within about 6 to 9 on half the land.)
const LUSH_TWI_MID = 7.5; const LUSH_TWI = 0.8; const LUSH_SUN = 3.0; const LUSH_PLACE = 0.6; const LUSH_LOW_M = 10.0;
const LUSH = 1.0; const LUSH_HUE = vec3f(-0.25, -0.08, 0.06);

// How lush a point is (`LUSH_TWI`): of wetness index `twi` (land.wgsl `Tiled.twi`), slope `slope` (east, south; rise
// over run), the noise of its samples' place `n` (rock.wgsl `Place.n`) and `low` m under its relief's mean (0 where
// not known).
// (Its sum is held within `LUSH_SUM_MOST` either way before the tanh: a channel's texels hold an index past any
// float's exp, and a tanh worked out by exp gives no number there.)
const LUSH_SUM_MOST = 10.0;
fn lush_of(twi: f32, slope: vec2f, n: f32, low: f32) -> f32 {
  let sum = LUSH_TWI * (twi - LUSH_TWI_MID) - LUSH_SUN * sunward(slope) + LUSH_PLACE * n + low / LUSH_LOW_M;
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
