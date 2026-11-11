// The Earth view's style of the ground between the plants and of its rock (colour.wgsl `colour`; the rest of the style
// is style.wgsl). Each look means what the model holds: soil has clods, the forest floor its litter, a face of the bed
// too steep to hold loose ground is a cliff in the bed's own colour, and stones lie where the loose ground is thin on a
// steep slope (scree) or bare, and in a channel as big as its power moves (rock.wgsl `stone_of`). Fixed in the world:
// nothing moves as the view pans or zooms, and what is smaller than a pixel fades into its mean.

// Bare soil's tone varies by up to this share over clods `CLOD_M` across.
const CLOD = 0.14; const CLOD_M = 0.5;
// Under a closed canopy the bare ground is litter: its colour times `LITTER_TINT`, up to `LITTER` of it; moss grows on
// it in patches `MOSS_M` across over this share of it, the more the wetter the ground.
const LITTER = 0.8; const LITTER_TINT = vec3f(0.95, 0.72, 0.5);
const MOSS = 0.45; const MOSS_M = 1.5; const MOSS_TINT = vec3f(0.6, 0.95, 0.45);
// A face of the drawn ground steeper than `CLIFF_FROM` (rise over run) is a cliff by `CLIFF_FULL`: its bed's colour
// times `CLIFF_TONE`, with its strata along the contours every `STRATA_M` of height, their foot darker (`STRATA_DARK`),
// wobbling by up to `STRATA_WOBBLE` m over `STRATA_WAVE_M`; plants hold on its ledges, `LEDGES` of what they would.
const CLIFF_FROM = 0.9; const CLIFF_FULL = 1.4; const CLIFF_TONE = 0.85; const LEDGES = 0.3;
const STRATA_M = 8.0; const STRATA_DARK = 0.45; const STRATA_WOBBLE = 2.0; const STRATA_WAVE_M = 40.0;
// The strata come at `STRATA_M` and at `STRATA_FINE` of it; a bed's tone differs from the face's by up to
// `STRATA_TONES` either way, and the line at its foot is `STRATA_LINE` of its thickness.
const STRATA_FINE = 0.25; const STRATA_TONES = 0.35; const STRATA_LINE = 0.12;
// Stones: on a slope from `SCREE_FROM` to `CLIFF_FROM` where the loose ground is thinner than `SCREE_LOOSE_M` (the
// scree below a face), `SCREE_STONE_M` across, at most `SCREE` of the places they could lie; on bare bed, `ON_BED` of
// them; in a channel, `IN_CHANNEL` of them, as big as it moves (no smaller than `PEBBLE_M`). None on a beach.
const SCREE_FROM = 0.6; const SCREE_LOOSE_M = 1.0; const SCREE_STONE_M = 0.6; const SCREE = 0.7;
const STONES_ON_BED = 0.25; const IN_CHANNEL = 0.6; const PEBBLE_M = 0.05;
// A stone lies one to a cell `STONE_GAP` stones wide, its size differing by up to `STONE_SIZES` either way; it is
// its bed's colour times `STONE_TONE`, lit as a dome; it is drawn as itself from `STONE_PX` pixels across, below as
// the tone its stones give the ground.
const STONE_GAP = 2.5; const STONE_SIZES = 0.4; const STONE_TONE = 1.15; const STONE_PX = 2.0;

// The share of detail `size` m across that shows at `e` m a pixel: none below a pixel.
fn shows_at(size: f32, e: f32) -> f32 { return 1.0 - ease(0.5 * size, 1.5 * size, 2.0 * e); }

// The bare ground's colour `c` (linear, unlit) at `at`, seen at `e` m a pixel, under `canopy` of closed crowns, `wet`:
// its clods, and under the crowns its litter and moss.
fn floor_of(c: vec3f, at: vec2f, e: f32, canopy: f32, wet: f32) -> vec3f {
  var out = c * (1.0 + CLOD * STYLE * noise(at / CLOD_M, 51).x * shows_at(CLOD_M, e));
  let litter = LITTER * STYLE * canopy;
  out *= mix(vec3f(1.0), LITTER_TINT, litter);
  let moss = MOSS * litter * wet * ease(0.0, 0.4, noise(at / MOSS_M, 52).x) * shows_at(MOSS_M, e);
  return out * mix(vec3f(1.0), MOSS_TINT, moss);
}

// How much of a point of drawn slope `steep` (rise over run) is a cliff.
fn cliff_of(steep: f32) -> f32 { return ease(CLIFF_FROM, CLIFF_FULL, steep); }

// A cliff's face of bed `bed` (linear, unlit) at height `z`, m, and `at`, of slope `steep`, seen at `e` m a pixel: its
// strata along the contours, each bed of rock its own tone (`STRATA_TONES`) with a dark line at its foot, at two
// spacings; each fading where a pixel spans more height than a quarter of its spacing.
fn cliff_face(bed: vec3f, z: f32, at: vec2f, steep: f32, e: f32) -> vec3f {
  let h = z + STRATA_WOBBLE * noise(at / STRATA_WAVE_M, 53).x;
  let span = steep * e;
  var tone = 1.0;
  for (var k = 0; k < 2; k++) {
    let spacing = STRATA_M * select(1.0, STRATA_FINE, k == 1);
    let layer = h / spacing;
    let shown = 1.0 - ease(0.15 * spacing, 0.3 * spacing, span);
    let own = f32(lowbias(vec2i(i32(floor(layer)), k), 56) & 255u) / 255.0 - 0.5;
    let line = 1.0 - smoothstep(STRATA_LINE * spacing, STRATA_LINE * spacing + span, fract(layer) * spacing);
    tone *= 1.0 + shown * (STRATA_TONES * own - (1.0 - STRATA_DARK) * line);
  }
  return bed * CLIFF_TONE * tone;
}

// The stones of `density` (the share of cells that hold one) and `size` m at `at`, seen at `e` m a pixel, of bed `bed`
// (linear, unlit), stream `k` of the world's random numbers: their colour (lit as domes from the north-west) and
// coverage. Each lies anywhere in its cell, so the four cells about the point are all that may cover it.
fn stones(at: vec2f, e: f32, density: f32, size: f32, bed: vec3f, k: i32) -> vec4f {
  if (density <= 0.0 || size <= 0.0) { return vec4f(0.0); }
  let side = STONE_GAP * size;
  let tone = bed * STONE_TONE;
  // Smaller than a few pixels: the share of the ground they cover, as a tone.
  let near = ease(STONE_PX * e, 2.0 * STONE_PX * e, size);
  let mean = density * PI * 0.25 / (STONE_GAP * STONE_GAP);
  var out = vec4f(tone * mean, mean) * (1.0 - near);
  if (near <= 0.0) { return out; }
  let q = at / side;
  let toward = select(vec2f(-1.0), vec2f(1.0), fract(q) > vec2f(0.5));
  for (var i = 0; i < 4; i++) {
    let c = floor(q) + vec2f(f32(i & 1), f32(i >> 1u)) * toward;
    let x = lowbias(vec2i(c), k);
    if (f32(x & 1023u) / 1024.0 >= density) { continue; }
    let r = 0.5 * size * (1.0 + STONE_SIZES * (f32((x >> 20u) & 1023u) / 512.0 - 1.0));
    let centre = (c + vec2f(f32((x >> 10u) & 31u), f32((x >> 15u) & 31u)) / 32.0) * side;
    let v = (at - centre) / max(r, 1e-4);
    let a = 1.0 - smoothstep(1.0 - e / r, 1.0, length(v));
    if (a <= 0.0) { continue; }
    let n = normalize(vec3f(v, sqrt(max(1.0 - dot(v, v), 0.0)) + 0.2));
    let lit = 0.55 + 0.6 * max(dot(n, LIGHT), 0.0);
    out = mix(out, vec4f(tone * lit, 1.0), a * near);
  }
  return out;
}
