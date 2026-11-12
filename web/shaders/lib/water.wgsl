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

// The slope (east, south) of a liquid's surface at `at`, m in the world, seen at `e` m a pixel: octaves of ripples fixed
// in the world, each turned from the last so they stand in no grid, each fading out as it shrinks below a few pixels
// (it would only flicker).
fn ripples(at: vec2f, e: f32) -> vec2f {
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
    if (shown > 0.0) { g += shown * RIPPLE_SLOPE * (transpose(rot) * noise(rot * at / wave, k).yz); }
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

// Ice on standing liquid. The cells' share of their liquid under ice is true only cell by cell (the open low ground's
// cells are 8 km, and freeze as one surface), so where in a cell it lies is drawn: first over shallow liquid, last over
// deep (a shallow bay loses its heat first: up to `ICE_DEEP` of the index either way, by the e-folds of depth over
// `ICE_DEPTH_M` past the first, so the open sea's ice and a bay's both lie where their cells say), its edge
// broken by octaves of noise from `ICE_PATCH_M` down to the pixel (`ICE_PATCHY` of the index), so no edge follows a
// cell's border. The share s covers where the index lies under ln(s / (1 - s)) (an index spread about 1 either way,
// so as much ice is drawn as the cells hold), over a margin `ICE_BAND` of the index wide where the sheet breaks into
// floes: cells `FLOE_M` across, fixed in the world, each whole or open by its own draw against the cover there, with dark leads `LEAD` of a cell wide
// between them, drawn from `FLOE_PX` pixels a floe (smaller, the cover's mean); thin new ice is grey (`NEW_ICE`), old
// thick ice white with snow. The noise stops at the floes' scale (`ICE_PATCH_OCTAVES`): finer, the floes break it.
const ICE_DEEP = 1.0; const ICE_DEPTH_M = 10.0; const ICE_PATCH_M = 6000.0; const ICE_PATCH_STEP = 2.9;
const ICE_PATCH_FALL = 0.62; const ICE_PATCH_OCTAVES = 5; const ICE_PATCHY = 2.0; const ICE_BAND = 1.2;
const FLOE_M = 150.0; const FLOE_PX = vec2f(4.0, 10.0); const LEAD = 0.06; const NEW_ICE = vec3f(0.32, 0.40, 0.46);
// The snow that lies on old ice: a reflectance like fresh snow's.
const ICE_SNOW = vec3f(0.86, 0.89, 0.93);

// The share of liquid `depth` m deep at `at`, seen at `e` m a pixel, under ice where its cells hold `share` of theirs
// under ice: 0..1, smooth across the margin (the floes' cover there).
fn ice_cover(share: f32, depth: f32, at: vec2f, e: f32) -> f32 {
  if (share <= 0.0 || share >= 1.0) { return share; }
  var n = 0.0;
  var size = ICE_PATCH_M;
  var weight = 1.0;
  var sum = 0.0;
  for (var o = 0; o < ICE_PATCH_OCTAVES; o++) {
    let shown = shows_at(size, e);
    if (shown > 0.0) { n += weight * shown * noise_turned(at / size + f32(o) * 3.7, 90 + o); }
    sum += weight;
    size /= ICE_PATCH_STEP;
    weight *= ICE_PATCH_FALL;
  }
  let index = ICE_DEEP * tanh(log(1.0 + depth / ICE_DEPTH_M) - 1.0) + ICE_PATCHY * clamp(2.0 * n / sum, -1.0, 1.0);
  return 1.0 - smoothstep(-ICE_BAND, ICE_BAND, index - log(share / (1.0 - share)));
}

// Water of colour `water` at `at`, seen at `e` m a pixel, under ice that covers `cover` of it (`ice_cover`): the floes
// where their cells take a few pixels or more, else the cover's mean; grey where the ice is new, white where old.
fn ice_on(water: vec3f, cover: f32, at: vec2f, e: f32) -> vec3f {
  if (cover <= 0.0) { return water; }
  let ice = mix(NEW_ICE, ICE_SNOW, ease(0.2, 0.9, cover));
  let size = FLOE_M;
  let near = ease(FLOE_PX.x * e, FLOE_PX.y * e, size);
  var floes = cover;
  if (near > 0.0 && cover < 1.0) {
    // The two nearest of the floes' centres, one jittered in each cell (a Voronoi diagram: Worley 1996).
    let q = at / size;
    let c = floor(q);
    var d1 = 1e9;
    var d2 = 1e9;
    var id = 0u;
    for (var j = -1; j <= 1; j++) {
      for (var i = -1; i <= 1; i++) {
        let cell = c + vec2f(f32(i), f32(j));
        let x = lowbias(vec2i(cell), 97);
        let centre = cell + vec2f(f32(x & 1023u), f32((x >> 10u) & 1023u)) / 1024.0;
        let d = length(q - centre);
        if (d < d1) { d2 = d1; d1 = d; id = x; } else if (d < d2) { d2 = d; }
      }
    }
    let whole = select(0.0, 1.0, f32(id >> 22u) / 1024.0 < cover);
    let lead = smoothstep(LEAD, LEAD + e / size, 0.5 * (d2 - d1));
    floes = mix(cover, whole * lead, near);
  }
  return mix(water, ice, floes);
}
