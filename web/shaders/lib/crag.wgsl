// The Earth view's style of the ground between the plants and of its rock (colour.wgsl `colour`; the rest of the style
// is style.wgsl). Each look means what the model holds: soil has clods, the forest floor its litter; where the loose
// ground is thin the bed shows (rock.wgsl), drawn as slabs with cracks where it is gentle and as a cliff's face where
// it is steep (slope alone never bares the ground); stones lie where the loose ground is thin on a steep slope (scree)
// or bare, and in a channel as big as its power moves (rock.wgsl `stone_of`). Fixed in the world: nothing moves as the
// view pans or zooms, and what is smaller than a pixel fades into its mean.

// Bare soil's tone varies by up to this share over clods `CLOD_M` across.
const CLOD = 0.06; const CLOD_M = 0.5;
// Under a closed canopy the bare ground is litter: its colour times `LITTER_TINT`, up to `LITTER` of it, and in the
// crowns' shade, darker by up to `UNDER_DARK`; moss grows on it in patches `MOSS_M` across over this share of it, the
// more the wetter the ground.
const LITTER = 0.8; const LITTER_TINT = vec3f(0.95, 0.72, 0.5); const UNDER_DARK = 0.35;
const MOSS = 0.45; const MOSS_M = 1.5; const MOSS_TINT = vec3f(0.6, 0.95, 0.45);

// Bare bed: its own colour greyed by `ROCK_GREY` and times `ROCK_TONE`, mottled by up to `MOTTLE` over `MOTTLE_M`
// (so it reads as rock from far, never as a smooth blot), its light's departure from flat ground's `ROCK_RELIEF` times
// as strong (a face shows how it is turned).
const ROCK_GREY = 0.55; const ROCK_TONE = 0.95; const MOTTLE = 0.25; const MOTTLE_M = 60.0; const ROCK_RELIEF = 1.8;
// Gentle bare bed is slabs: a network of cracks `CRACK_W` of their spacing wide, every `SLAB_M`, darker by
// `CRACK_DARK`; each slab's tone differs by up to `SLAB_TONES`.
const SLAB_M = 4.0; const CRACK_W = 0.06; const CRACK_DARK = 0.5; const SLAB_TONES = 0.15;
// Bare bed steeper than `CLIFF_FROM` (rise over run) is a cliff's face by `CLIFF_FULL`: its strata along the contours
// every `STRATA_M` of height and `STRATA_FINE` of that, each bed of rock its own tone (up to `STRATA_TONES` either way)
// with a dark ledge at its foot (`STRATA_LINE` of its thickness, `STRATA_DARK`), wobbling by up to `STRATA_WOBBLE` m
// over `STRATA_WAVE_M` and broken where the noise over `BREAK_M` is under `BREAKS`; cracked down its face every
// `FACE_CRACK_M`.
const CLIFF_FROM = 0.9; const CLIFF_FULL = 1.4;
const STRATA_M = 8.0; const STRATA_FINE = 0.25; const STRATA_TONES = 0.35; const STRATA_LINE = 0.15; const STRATA_DARK = 0.4;
const STRATA_WOBBLE = 3.0; const STRATA_WAVE_M = 30.0; const BREAK_M = 20.0; const BREAKS = -0.25; const FACE_CRACK_M = 6.0;
// Stones: on a slope from `SCREE_FROM` to `CLIFF_FROM` where the loose ground is thinner than `SCREE_LOOSE_M` (the
// scree below a face), `SCREE_STONE_M` across, at most `SCREE` of the places they could lie; on bare bed, `STONES_ON_BED` of
// them; in a channel, `IN_CHANNEL` of them, as big as it moves (no smaller than `PEBBLE_M`). None on a beach.
const SCREE_FROM = 0.6; const SCREE_LOOSE_M = 1.0; const SCREE_STONE_M = 0.6; const SCREE = 0.7;
const STONES_ON_BED = 0.25; const IN_CHANNEL = 0.6; const PEBBLE_M = 0.05;
// A stone lies one to a cell `STONE_GAP` stones wide, its size differing by up to `STONE_SIZES` either way; it is
// its bed's colour times `STONE_TONE`, lit as a dome; it is drawn as itself from `STONE_PX` pixels across, below as
// the tone its stones give the ground.
const STONE_GAP = 2.5; const STONE_SIZES = 0.4; const STONE_TONE = 1.15; const STONE_PX = 2.0;

// The bare ground's colour `c` (linear, unlit) at `at`, seen at `e` m a pixel, under `canopy` of closed crowns, `wet`:
// its clods, and under the crowns their shade, litter and moss.
fn floor_of(c: vec3f, at: vec2f, e: f32, canopy: f32, wet: f32) -> vec3f {
  let litter = LITTER * STYLE * canopy;
  var out = c * mix(vec3f(1.0), LITTER_TINT, litter) * (1.0 - UNDER_DARK * STYLE * canopy);
  // Each grain only where it shows: its noise costs a few reads of the hash.
  let clods = shows_at(CLOD_M, e);
  if (clods > 0.0) { out *= 1.0 + CLOD * STYLE * noise(at / CLOD_M, 51).x * clods; }
  let moss = MOSS * litter * wet * shows_at(MOSS_M, e);
  if (moss > 0.0) { out *= mix(vec3f(1.0), MOSS_TINT, moss * ease(0.0, 0.4, noise(at / MOSS_M, 52).x)); }
  return out;
}

// The ground of a point of drawn slope `steep` (rise over run) that is a cliff.
fn cliff_of(steep: f32) -> f32 { return ease(CLIFF_FROM, CLIFF_FULL, steep); }

// A dark line where noise `n` (-1..1) crosses 0, `width` of its spacing wide: a network of cracks.
fn crack(n: f32, width: f32) -> f32 { return 1.0 - smoothstep(0.0, width, abs(n)); }

// Bare bed `bed` (linear, unlit) at height `z`, m, and `at`, of slope `steep`, lit `shade` times flat ground, seen at
// `e` m a pixel, in the style: slabs where gentle, a cliff's face where steep (the constants above). Each detail fades
// where a pixel spans more than a quarter of it.
fn rock_face(bed: vec3f, z: f32, at: vec2f, steep: f32, e: f32, shade: f32) -> vec3f {
  let grey = mix(bed, vec3f(dot(bed, LUMA)), ROCK_GREY) * ROCK_TONE;
  var tone = (1.0 + MOTTLE * noise(at / MOTTLE_M, 57).x) * (1.0 + (ROCK_RELIEF - 1.0) * (shade - 1.0));
  // The slabs and their cracks, read only where they show.
  let slab_shows = shows_at(SLAB_M * CRACK_W * 4.0, e);
  var slabs = 1.0;
  if (slab_shows > 0.0) { slabs = 1.0 + SLAB_TONES * noise(at / SLAB_M + 17.0, 59).x - CRACK_DARK * crack(noise(at / SLAB_M, 58).x, CRACK_W); }
  let cliff = cliff_of(steep);
  var face = 1.0;
  // A face is no less steep than `CLIFF_FROM`, so its pixels span at least that much height: its strata are gone where
  // that is 0.3 of their spacing, and its cracks where they are under a pixel (draw.wgsl `LEAST_E`).
  if (cliff > 0.0 && CLIFF_FROM * least_e() < 0.3 * STRATA_M) {
    let h = z + STRATA_WOBBLE * noise(at / STRATA_WAVE_M, 53).x;
    let whole = ease(BREAKS - 0.1, BREAKS + 0.1, noise(at / BREAK_M, 60).x);
    let span = steep * e;
    for (var k = 0; k < 2; k++) {
      let spacing = STRATA_M * select(1.0, STRATA_FINE, k == 1);
      let layer = h / spacing;
      let shown = 1.0 - ease(0.15 * spacing, 0.3 * spacing, span);
      let own = f32(lowbias(vec2i(i32(floor(layer)), k), 56) & 255u) / 255.0 - 0.5;
      let ledge = (1.0 - smoothstep(STRATA_LINE * spacing, STRATA_LINE * spacing + span, fract(layer) * spacing)) * whole;
      face *= 1.0 + shown * (STRATA_TONES * own - (1.0 - STRATA_DARK) * ledge);
    }
    face *= 1.0 - CRACK_DARK * crack(noise(at / FACE_CRACK_M, 61).x, CRACK_W) * shows_at(FACE_CRACK_M * CRACK_W * 4.0, e);
  }
  tone *= mix(mix(1.0, slabs, slab_shows), face, cliff);
  return grey * max(tone, 0.0);
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
