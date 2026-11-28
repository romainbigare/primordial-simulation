// Water seen from above: a flat surface over its bed. Light that reaches the bed and comes back is absorbed on the
// way down and up (Beer–Lambert), red the most and blue the least, so the bed shows through the shallows and fades with
// depth into the colour of deep water; the surface reflects a little of the sky by the angle it is seen at (Fresnel,
// in Schlick's form, Schlick 1994) and the sun where a ripple faces it. A sea, a lake and a river are the same water;
// water is turbid with the load it carries (`loaded_k`, `loaded_deep`).

// The diffuse attenuation of clear water for red, green and blue light, 1/m (Jerlov's oceanic water; Pope and Fry 1997
// for pure water), and the light that comes back from water too deep to show its bed (linear, a reflectance).
const ATTENUATION = vec3f(0.35, 0.07, 0.03);
const DEEP_WATER = vec3f(0.0044, 0.027, 0.107);
// The share of light a liquid's surface reflects straight up (Fresnel at normal incidence, refractive index 1.33), the
// sky's light and the sun's (linear, over the light of sunlit flat ground).
const REFLECT_UP = 0.02;
const SKY = vec3f(0.61, 0.73, 0.87);
const SUN = vec3f(1.0, 0.96, 0.83);
// The sun shows in a ripple that faces it: the low sun (`LIGHT`) shows in no ripple seen from straight above, so the
// map draws its glints where a ripple's slope toward the sun passes `GLINT_SLOPE.x`, fully from `.y`, at most `GLINT_MOST`
// of its light (the sun's disc is small: a glint is a sparkle, not a sheet).
const GLINT_SLOPE = vec2f(0.55, 0.75);
const GLINT_MOST = 0.4;
// The sky is brightest about the sun: a ripple sends back `SKY_TILT` more of the sky's light per unit of its slope
// toward the sun (less turned away), so wind waves show as a fine sheen of light and dark (drawing only).
const SKY_TILT = 0.08;
// Ripples: their longest wavelength, m, the octaves below it (each half as long), and the slope of each.
const RIPPLE_LONGEST_M = 8.0;
const RIPPLE_OCTAVES = 6;
const RIPPLE_SLOPE = 0.09;

// The slope (east, south) of a liquid's surface at the pixel, seen at `e` m a pixel: octaves of ripples fixed
// in the world, each turned from the last so they stand in no grid, each fading out as it shrinks below a few pixels
// (it would only flicker).
fn ripples(e: f32) -> vec2f {
  var g = vec2f(0.0);
  var wave = RIPPLE_LONGEST_M;
  // The octave's turn from the world's axes; a turn of about 37 degrees between octaves.
  var rot = mat2x2f(1.0, 0.0, 0.0, 1.0);
  let turn = mat2x2f(0.8, 0.6, -0.6, 0.8);
  for (var k = 0; k < RIPPLE_OCTAVES; k++) {
    // An octave fades out by half its wavelength a pixel: none shows in a draw of pixels that large (draw.wgsl
    // `LEAST_E`).
    if (least_e() >= 0.5 * wave) { break; }
    let shown = 1.0 - ease(wave / 8.0, wave / 2.0, e);
    if (shown > 0.0) { g += shown * RIPPLE_SLOPE * (transpose(rot) * noise_l(lat(rot * (1.0 / wave), vec2f(0.0)), k).yz); }
    wave *= 0.5;
    rot = turn * rot;
  }
  return g;
}

// The way to the eye of the map's angled view: from the south, 35 degrees above the horizon (the sprites' camera,
// web/textures/SOURCES.md), east, south and up.
const MAP_VIEW = vec3f(0.0, 0.8192, 0.5736);

// What a liquid's surface of slope g (east, south) adds of the sky and the sun lit from the north-west (`LIGHT`), seen
// from straight above.
fn surface_light(g: vec2f) -> vec3f { return surface_seen(g, vec3f(0.0, 0.0, 1.0)); }

// The same seen from the way `view` (a unit vector, east, south and up).
fn surface_seen(g: vec2f, view: vec3f) -> vec3f {
  let n = normalize(vec3f(-g, 1.0));
  let fresnel = REFLECT_UP + (1.0 - REFLECT_UP) * pow(1.0 - dot(n, view), 5.0);
  let sunward = dot(-g, LIGHT.xy) / length(LIGHT.xy);
  let glint = smoothstep(GLINT_SLOPE.x, GLINT_SLOPE.y, sunward);
  return (fresnel + SKY_TILT * sunward) * SKY + GLINT_MOST * glint * SUN;
}

// The wind roughens standing liquid unevenly, and a rougher patch sends back more of the bright sky near the horizon,
// a calmer one less (seen from the air a lake is streaked and blotched by its wind; from memory): gusts run in lanes
// along the wind (`WIND_WAY`, a unit vector east and south) `GUST_LANE_M.x` long and `.y` wide, and cat's paws,
// patches `PAW_M` across, `PAW` as strong, roughen it between them; close, the wind's streaks lie along it in lines
// `WINDROW_M.y` apart and some `WINDROW_M.x` long (windrows, Langmuir 1938), `WINDROW_W` of the noise wide (no
// narrower than a pixel and a half), `WINDROW` as rough. Where the wind roughens it by r (`gusts`, about -1..1, 0 on
// the mean) the surface sends back `GUST_SHEEN` r more of the sky's light, and its ripples are `GUST_RIPPLE` r steeper.
const WIND_WAY = vec2f(0.91, 0.415); const GUST_LANE_M = vec2f(2400.0, 450.0); const PAW_M = 150.0; const PAW = 0.7;
const WINDROW_M = vec2f(400.0, 30.0); const WINDROW_W = 0.1; const WINDROW = 1.0;
const GUST_SHEEN = 0.008; const GUST_RIPPLE = 0.6;
const GUST_STREAM = 220; const PAW_STREAM = 221; const WINDROW_STREAM = 222;
// A patch of the wind's roughness shows from `SWAY_PX.x` pixels across, fully from `.y`: smaller, it reads as a stain,
// not as water.
const SWAY_PX = vec2f(6.0, 16.0);
fn sway_shows(size: f32, e: f32) -> f32 { return select(ease(SWAY_PX.x * e, SWAY_PX.y * e, size), 0.0, least_e() >= size / SWAY_PX.x); }
// The lattice of cells `size.x` m long along the wind and `size.y` m across it.
fn wind_lat(size: vec2f) -> Lat {
  return lat(mat2x2f(WIND_WAY.x / size.x, -WIND_WAY.y / size.y, WIND_WAY.y / size.x, WIND_WAY.x / size.y), vec2f(0.0));
}
fn gusts(e: f32) -> f32 {
  var r = 0.0;
  let lanes = sway_shows(GUST_LANE_M.y, e);
  if (lanes > 0.0) { r += lanes * noise_l(wind_lat(GUST_LANE_M), GUST_STREAM).x; }
  let paws = sway_shows(PAW_M, e);
  if (paws > 0.0) { r += PAW * paws * noise_l(lat_turned(PAW_M, vec2f(0.0)), PAW_STREAM).x; }
  let rows = sway_shows(WINDROW_M.y, e);
  if (rows > 0.0) {
    let n = noise_l(wind_lat(WINDROW_M), WINDROW_STREAM);
    // (The noise's change across a pixel, from its slope along and across the wind.)
    let px = length(n.yz / WINDROW_M) * e;
    r += WINDROW * rows * (1.0 - smoothstep(0.0, max(WINDROW_W, 1.5 * px), abs(n.x)));
  }
  return r;
}

// The open sea's swell runs with the wind (its waves `SEA_WAVE` long), its crests bent by `SWELL_BEND` of a wavelength
// over noise `SWELL_BEND_M` across and stronger and weaker along them by `SWELL_GROUPS` of that noise (a swell comes
// in groups); where a crest's face is turned to the sun it sends back `SWELL_SHEEN` more of the sky's light, less on
// its back. It shows as its wavelength takes the pixels a patch of the wind's does (`SWAY_PX`). The share of that
// sheen at the pixel at `at`, seen at `e` m a pixel, -1..1.
const SWELL_BEND = 0.8; const SWELL_BEND_M = 350.0; const SWELL_GROUPS = 0.5; const SWELL_SHEEN = 0.009; const SWELL_STREAM = 223;
fn swell(at: vec2f, e: f32) -> f32 {
  let shown = sway_shows(SEA_WAVE.y, e);
  if (shown <= 0.0) { return 0.0; }
  let n = noise_l(lat_turned(SWELL_BEND_M, vec2f(0.0)), SWELL_STREAM).x;
  let phase = 2.0 * PI * (dot(at, WIND_WAY) / SEA_WAVE.y + SWELL_BEND * n);
  return shown * (1.0 - SWELL_GROUPS + SWELL_GROUPS * n) * sin(phase);
}

// The bed under standing liquid is drawn as uneven as a real one, which the samples are too coarse to hold: the depth
// its colour reads moves by up to `BED_SWAY` of itself either way (as its logarithm) by the noise of its samples' place
// (rock.wgsl `Place.n`, from a kilometre down), so the open water shows banks and deeps and the shallows break into
// irregular shoals; its waterline stays where it is. The share of its depth the colour reads where that noise is `n`.
const BED_SWAY = 0.35;
fn bed_sway(n: f32) -> f32 { return exp(BED_SWAY * n); }

// The share of the light that crosses `depth` m of standing water down to its bed and back, for red, green and blue.
fn through(depth: f32) -> vec3f { return exp(-2.0 * ATTENUATION * depth); }

// Bed `bed` (linear, lit) seen through `depth` m of standing water (0: a wet film) whose surface has slope g, carrying
// `load` g/m³.
fn water_over(bed: vec3f, depth: f32, g: vec2f, load: f32) -> vec3f {
  return seen_through(bed, depth, g, loaded_k(ATTENUATION, load), loaded_deep(DEEP_WATER, load));
}

// Running water holds what it washed from the soil, the dissolved organic matter that takes the blue most: a stream
// that carries no load attenuates as the clearer inland waters do, a few tenths per m (Kirk 1994, from memory).
const RUNNING_CLEAR = vec3f(0.5, 0.25, 0.5);
// The load a liquid carries in suspension attenuates its light by `LOAD_ATTENUATION` per m for each g/m³ (the specific
// attenuation of suspended mineral grains, some 0.03 to 0.1 m²/g; Kirk 1994, from memory), the blue a little more:
// water carrying 10 g/m³ still shows a bed a metre down, at 100 g/m³ none past a few decimetres.
const LOAD_ATTENUATION = vec3f(0.045, 0.05, 0.06);
// What comes back from water too deep to show its bed moves to what its grains send back, `LOADED_WATER` (a muddy
// river's water-leaving reflectance, about 0.1 in the red; from memory), as its load over its load and `LOAD_HALF`
// g/m³ (the reflectance of turbid water saturates with its load: Nechad and others 2010).
const LOADED_WATER = vec3f(0.14, 0.11, 0.05); const LOAD_HALF = 50.0;
// The attenuation `k` of water, 1/m, carrying `load` g/m³.
fn loaded_k(k: vec3f, load: f32) -> vec3f { return k + LOAD_ATTENUATION * load; }
// The light `deep` that comes back from water too deep to show its bed, carrying `load` g/m³.
fn loaded_deep(deep: vec3f, load: f32) -> vec3f { return mix(deep, LOADED_WATER, load / (load + LOAD_HALF)); }

// Bed `bed` (linear, lit) seen through `depth` m of water of diffuse attenuation `k`, 1/m, whose surface has slope g,
// over the light `deep` that comes back from it where it is too deep to show its bed.
fn seen_through(bed: vec3f, depth: f32, g: vec2f, k: vec3f, deep: vec3f) -> vec3f { return seen_from(bed, depth, g, k, deep, vec3f(0.0, 0.0, 1.0)); }
// The same seen from the way `view` (`surface_seen`).
fn seen_from(bed: vec3f, depth: f32, g: vec2f, k: vec3f, deep: vec3f, view: vec3f) -> vec3f {
  let t = exp(-2.0 * k * depth);
  return bed * t + deep * (1.0 - t) + surface_seen(g, view);
}

// Waves at a shore, as their height and their length in deep water, m (L = g T² / 2π): the open sea's, a metre and six
// seconds (Sverdrup and Munk 1947); a lake's as a fresh wind of `WIND_M_S` raises them over its fetch F, the length of
// open water it blows over (the pond's, tape/src/table.rs), fetch-limited (Shore Protection Manual 1984): H = 0.0016
// √(g F / U²) U² / g and T = 0.2857 (g F / U²)^(1/3) U / g, no higher than the sea's.
const SEA_WAVE = vec2f(1.0, 56.0); const WIND_M_S = 10.0; const GRAVITY = 9.81;
// The waves of standing liquid `sea` of it the sea's (the rest a lake's of fetch `fetch` m).
fn waves_of(sea: f32, fetch: f32) -> vec2f {
  let u2g = WIND_M_S * WIND_M_S / GRAVITY;
  let x = fetch / u2g;
  let t = 0.2857 * pow(x, 1.0 / 3.0) * WIND_M_S / GRAVITY;
  let lake = min(vec2f(0.0016 * sqrt(x) * u2g, GRAVITY * t * t / (2.0 * PI)), SEA_WAVE);
  return mix(lake, SEA_WAVE, sea);
}
// The height over the still level the swash of waves `w` (`waves_of`) reaches up a shore of slope `slope` (rise over
// run), m: the run-up that 2% of them pass, R = 1.1 (0.35 β √(H L) + √(H L (0.563 β² + 0.004)) / 2) (Stockdon and
// others 2006).
fn run_up(w: vec2f, slope: f32) -> f32 {
  let hl = sqrt(w.x * w.y);
  return 1.1 * (0.35 * slope * hl + 0.5 * hl * sqrt(0.563 * slope * slope + 0.004));
}
