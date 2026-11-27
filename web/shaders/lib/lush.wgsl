// How lush the ground is inside a cell, for the plants that stand on it (cover_field.wgsl, mark.wgsl), its loose
// ground (rock.wgsl `loose_at`) and the grain that wash sorts on it (rock.wgsl `grain_at`).

// Inside a cell the plants gather where water gathers and the sun dries the ground least, as seen from a plane: along
// the drainage lines and in the hollows, on the slopes turned from the sun, and thin out on the ridges, the knobs and
// the sunny sides (drawing only: the core knows a cell's mean and its saturated strips). How lush a point is, -1..1
// (`lush_of`): the tanh of its wetness index over `LUSH_TWI_MID` times `LUSH_TWI` a unit, its rise toward the light
// times `LUSH_SUN` (as snow lies longest there, land.wgsl `snow_bares`), the noise of the samples' places (rock.wgsl
// `Place.n`, the patches of a mantle from a kilometre down) times `LUSH_PLACE`, and how low it lies in its relief (a
// hollow's depth over `LUSH_LOW_M`, relief.wgsl); about none on the mean, so a cell keeps its cover. The plants stand
// up to `LUSH` thicker and thinner by it (`lush_thick`), and the herbs' colour turns by `LUSH_HUE` of itself, greener
// where lush, straw where dry. (The wetness index of the tiles' texels lies within about 6 to 9 on half the land.)
const LUSH_TWI_MID = 7.5; const LUSH_TWI = 0.8; const LUSH_SUN = 3.0; const LUSH_PLACE = 0.6; const LUSH_LOW_M = 2.0;
const LUSH = 0.9; const LUSH_HUE = vec3f(-0.25, -0.08, 0.06);

// How lush a point is (`LUSH_TWI`): of wetness index `twi` (land.wgsl `Tiled.twi`), slope `slope` (east, south; rise
// over run), the noise of its samples' place `n` (rock.wgsl `Place.n`) and `low` m under its relief's mean (0 where
// not known).
fn lush_of(twi: f32, slope: vec2f, n: f32, low: f32) -> f32 {
  return tanh(LUSH_TWI * (twi - LUSH_TWI_MID) - LUSH_SUN * sunward(slope) + LUSH_PLACE * n + low / LUSH_LOW_M);
}
// How much thicker than their mean the plants stand where they are as lush as `lush` (`lush_of`).
fn lush_thick(lush: f32) -> f32 { return 1.0 + LUSH * lush; }

// What the herbs' colour is multiplied by where they are as lush as `lush` (`LUSH_HUE`).
fn lush_hue(lush: f32) -> vec3f { return 1.0 + LUSH_HUE * lush; }
