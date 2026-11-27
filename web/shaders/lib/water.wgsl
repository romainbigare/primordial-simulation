// Water seen from above: a flat surface over its bed. Light that reaches the bed and comes back is absorbed on the
// way down and up (Beer–Lambert), red the most and blue the least, so the bed shows through the shallows and fades with
// depth into the colour of deep water; the surface reflects a little of the sky by the angle it is seen at (Fresnel,
// in Schlick's form, Schlick 1994) and the sun where a ripple faces it. A sea, a lake and a river are the same water;
// water is turbid with the load it carries (`loaded_k`, `loaded_deep`).

// The diffuse attenuation of clear water for red, green and blue light, 1/m (Jerlov's oceanic water; Pope and Fry 1997
// for pure water), and the light that comes back from water too deep to show its bed (linear, a reflectance).
const ATTENUATION = vec3f(0.35, 0.07, 0.03);
const DEEP_WATER = vec3f(0.0044, 0.027, 0.107);
// The share of light a liquid's surface reflects straight up (Fresnel at normal incidence, refractive index 1.33); the
// sky's light seen at its zenith and at its horizon, which a surface reflects as it is turned (a clear sky is bluest
// and darkest overhead and paler towards the horizon: Rayleigh scattering over a longer path), and the sun's (linear,
// over the light of sunlit flat ground).
const REFLECT_UP = 0.02;
const SKY = vec3f(0.61, 0.73, 0.87);
const SKY_ZENITH = vec3f(0.42, 0.58, 0.86); const SKY_HORIZON = vec3f(0.78, 0.84, 0.90);
const SUN = vec3f(1.0, 0.96, 0.83);
// The sun shows in a wave as sharply as this power of the cosine between its slope and the half-way light, at most
// this share of its light (the sun's disc is small: a glint is a sparkle, not a sheet).
const GLINT_POWER = 150.0;
const GLINT_MOST = 0.4;
// Waves too small for the pixel to show still reflect more of the sky and the sun on the mean than a flat surface: a
// surface of mean square slope s² seen from the map's view reflects `ROUGH_SKY` s² more of the sky's light and
// `ROUGH_GLINT` s² of the sun's (the mean of `surface_seen` over the water's waves, fitted for slopes of 0.05 to 0.3).
const ROUGH_SKY = 0.35; const ROUGH_GLINT = 0.04;

// The way to the eye of the map's angled view: from the south, 35 degrees above the horizon (the sprites' camera,
// web/textures/SOURCES.md), east, south and up.
const MAP_VIEW = vec3f(0.0, 0.8192, 0.5736);

// What a liquid's surface of slope g (east, south) adds of the sky and the sun lit from the north-west (`LIGHT`), seen
// from straight above.
fn surface_light(g: vec2f) -> vec3f { return surface_seen(g, vec3f(0.0, 0.0, 1.0), 0.0); }

// The same seen from the way `view` (a unit vector, east, south and up), with waves of mean square slope `hidden` too
// small to show (`ROUGH_SKY`): the sky it mirrors by the way it reflects the eye's ray (`SKY_ZENITH`).
fn surface_seen(g: vec2f, view: vec3f, hidden: f32) -> vec3f {
  let n = normalize(vec3f(-g, 1.0));
  let cosine = dot(n, view);
  let fresnel = REFLECT_UP + (1.0 - REFLECT_UP) * pow(1.0 - max(cosine, 0.0), 5.0);
  let ray = 2.0 * cosine * n - view;
  let sky = mix(SKY_HORIZON, SKY_ZENITH, clamp(ray.z, 0.0, 1.0));
  let half_way = normalize(LIGHT + view);
  let glint = pow(max(dot(n, half_way), 0.0), GLINT_POWER);
  return (fresnel + ROUGH_SKY * hidden) * sky + (GLINT_MOST * glint + ROUGH_GLINT * hidden) * SUN;
}

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
  return bed * t + deep * (1.0 - t) + surface_seen(g, view, 0.0);
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
// The light a liquid's waves focus on its bed (tex.wgsl `waves_read`) shows by up to `CAUSTICS` in the shallows, fading
// out to none where it is `CAUSTIC_DEEP_M` deep (the waves' focus lies within a few of their lengths).
const CAUSTICS = 0.6; const CAUSTIC_DEEP_M = 0.8;
// The mean square slope of the sea's surface in a wind of `WIND_M_S` (Cox and Munk 1954: 0.003 + 0.00512 U), and of
// standing liquid `sea` of it the sea's, of fetch `fetch` m (`waves_of`): a lake's as much less as its waves are lower
// than the sea's, no less than `LAKE_MSS_LEAST` of it (a pond's surface still ripples).
const SEA_MSS = 0.003 + 0.00512 * WIND_M_S; const LAKE_MSS_LEAST = 0.15;
fn standing_mss(sea: f32, fetch: f32) -> f32 { return SEA_MSS * max(waves_of(sea, fetch).x / SEA_WAVE.x, LAKE_MSS_LEAST); }
// The height over the still level the swash of waves `w` (`waves_of`) reaches up a shore of slope `slope` (rise over
// run), m: the run-up that 2% of them pass, R = 1.1 (0.35 β √(H L) + √(H L (0.563 β² + 0.004)) / 2) (Stockdon and
// others 2006).
fn run_up(w: vec2f, slope: f32) -> f32 {
  let hl = sqrt(w.x * w.y);
  return 1.1 * (0.35 * slope * hl + 0.5 * hl * sqrt(0.563 * slope * slope + 0.004));
}
