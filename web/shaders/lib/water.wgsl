// Water seen from above: a flat surface over its bed. Light that reaches the bed and comes back is absorbed on the
// way down and up (Beer–Lambert), red the most and blue the least, so the bed shows through the shallows and fades with
// depth into the colour of deep water; the surface reflects a little of the sky by the angle it is seen at (Fresnel,
// in Schlick's form, Schlick 1994) and the sun where a ripple faces it. A sea, a lake and a river are the same water;
// running water is turbid with what it carries (`RUNNING_ATTENUATION`).

// The diffuse attenuation of clear water for red, green and blue light, 1/m (Jerlov's oceanic water; Pope and Fry 1997
// for pure water), and the light that comes back from water too deep to show its bed (linear, a reflectance).
const ATTENUATION = vec3f(0.35, 0.07, 0.03);
const DEEP_WATER = vec3f(0.0044, 0.027, 0.107);
// The share of light a liquid's surface reflects straight up (Fresnel at normal incidence, refractive index 1.33), the
// sky's light and the sun's (linear, over the light of sunlit flat ground).
const REFLECT_UP = 0.02;
const SKY = vec3f(0.61, 0.73, 0.87);
const SUN = vec3f(1.0, 0.96, 0.83);
// The sun shows in a ripple as sharply as this power of the cosine between its slope and the half-way light, at most
// this share of its light (the sun's disc is small: a glint is a sparkle, not a sheet).
const GLINT_POWER = 150.0;
const GLINT_MOST = 0.4;
// Ripples: their longest wavelength, m, the octaves below it (each half as long), and the slope of each.
const RIPPLE_LONGEST_M = 2.0;
const RIPPLE_OCTAVES = 4;
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

// What a liquid's surface of slope g (east, south) adds of the sky and the sun lit from the north-west (`LIGHT`), seen
// from straight above.
fn surface_light(g: vec2f) -> vec3f {
  let n = normalize(vec3f(-g, 1.0));
  let fresnel = REFLECT_UP + (1.0 - REFLECT_UP) * pow(1.0 - n.z, 5.0);
  let half_way = normalize(LIGHT + vec3f(0.0, 0.0, 1.0));
  let glint = pow(max(dot(n, half_way), 0.0), GLINT_POWER);
  return fresnel * SKY + GLINT_MOST * glint * SUN;
}

// The share of the light that crosses `depth` m of standing water down to its bed and back, for red, green and blue.
fn through(depth: f32) -> vec3f { return exp(-2.0 * ATTENUATION * depth); }

// Bed `bed` (linear, lit) seen through `depth` m of standing water (0: a wet film) whose surface has slope g.
fn water_over(bed: vec3f, depth: f32, g: vec2f) -> vec3f { return seen_through(bed, depth, g, ATTENUATION, DEEP_WATER); }

// Running water carries in suspension what it wears from its bed and holds what it washed from the soil, so its light
// attenuates as in turbid inland water, 1 to 4 per m (Kirk 1994), the most in the blue that the dissolved matter
// absorbs: a stream a few centimetres deep shows its bed, a river a metre deep hardly.
const RUNNING_ATTENUATION = vec3f(1.6, 1.2, 1.8);

// Bed `bed` (linear, lit) seen through `depth` m of water of diffuse attenuation `k`, 1/m, whose surface has slope g,
// over the light `deep` that comes back from it where it is too deep to show its bed.
fn seen_through(bed: vec3f, depth: f32, g: vec2f, k: vec3f, deep: vec3f) -> vec3f {
  let t = exp(-2.0 * k * depth);
  return bed * t + deep * (1.0 - t) + surface_light(g);
}
