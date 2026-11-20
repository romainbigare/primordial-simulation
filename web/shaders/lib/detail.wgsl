// The ground's detail up close (web/src/detail.ts): photographs of ground seen from above and tiles of ground cover
// rendered from 3D models as the map sees it (tex.wgsl `CARPETS_FROM`), each a layer of `detail_tex` with its
// mips down to its mean, laid on the map at its real size and read as a change around the colour the world gives the
// ground: the image over its mean. From afar the mips average the image to its mean, so the ground's colour there is
// the world's alone; near, the same colour shows its leaves, stones, cracks, grass and sticks. No repeat shows: a
// photograph is laid in hexagons, each turned and shifted by its own draw (`detail`), a tile of cover (whose things
// stand up the screen) in patches each shifted by its own draw (`carpet`).

// Of each photograph laid as detail (`detail`): its side in the world, m (web/textures/SOURCES.md), and its own stream
// of noise.
fn detail_m(t: i32) -> f32 { var m = array<f32, PHOTOS>(2.0, 50.0, 1.3, 3.0, 2.0, 15.0, 1.3, 2.2, 2.9, 1.3, 2.3, 2.07, 2.0, 2.35, 1.2, 1.5, 2.0, 2.17); return m[t]; }
fn detail_noise(t: i32) -> i32 { var k = array<i32, PHOTOS>(100, 102, 103, 101, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 185, 186, 187, 188); return k[t]; }
// The streams of noise of the tiles of cover: their patches' shifts and their seams' waves (`carpet`), each tile its own.
const CARPET_SHIFTS = 108; const CARPET_WAVES = 110;
// A photograph's hexagons are `DETAIL_HEX` times its side across, and meet over `DETAIL_EDGE` of their weights.
const DETAIL_HEX = 1.0; const DETAIL_EDGE = 0.12;
const CARPET_PATCH = 1.7; const CARPET_SEAM = 0.02; const CARPET_WAVE = 0.06;
// An image's own hue shows this much (the rest of its change is its lightness alone): the world gives the colour; a
// tile of cover's more (`CARPET_HUE`), its green plants on brown ground.
const DETAIL_HUE = 0.55; const CARPET_HUE = 0.95;

// The change image `t` of the array gives a colour: `a` and `b` its two layings, blended by noise `w`.
fn change_of(t: i32, a: vec3f, b: vec3f, w: f32, hue: f32) -> vec3f {
  let ratio = mix(a, b, w) / max(photo_mean(t), vec3f(1e-3));
  return mix(vec3f(dot(ratio, LUMA)), ratio, hue);
}

// Layer t of a tile of cover in patch `l` (of a lattice `CARPET_PATCH` tiles wide), shifted by the patch's draw x and,
// where nothing on the tile stands up (`flat`), turned by it: a half turn or none, and up to `FLAT_TILT` radians either
// way, so its wind ripples keep their line. `g` tiles a pixel.
const FLAT_TILT = 0.3;
fn carpet_patch(t: i32, l: Lat, x: u32, flat: bool, g: f32) -> vec3f {
  let turn = select(0.0, PI * f32((x >> 20u) & 1u) + FLAT_TILT * (f32(x >> 21u) / 1024.0 - 1.0), flat);
  let r = mat2x2f(cos(turn), sin(turn), -sin(turn), cos(turn));
  let shift = vec2f(f32(x & 1023u), f32((x >> 10u) & 1023u)) / 1024.0;
  return photo(t, r * (l.f * CARPET_PATCH) + shift, r * vec2f(g, 0.0), r * vec2f(0.0, g));
}

// The change tile of cover c gives the ground's colour at the pixel, at e m a pixel. The tile is laid in patches
// `CARPET_PATCH` tiles wide, each shifted (and a flat tile turned) by its own draw (`carpet_patch`), in two lattices of
// patches half a patch apart: the pixel shows the patch of the lattice whose middle it is nearer, the two meeting by
// their heights (ragged.wgsl `height_mix`) over a seam `CARPET_SEAM` wide (of a patch), wavy by `CARPET_WAVE`, so no
// repeat shows and a pixel shows one patch's grass and stones, not the mean of two.
fn carpet(c: i32, e: f32) -> vec3f {
  let s = carpet_m(c);
  let t = CARPETS_FROM + c;
  let pa = lat_axes(CARPET_PATCH * s, vec2f(0.0));
  let pb = lat_axes(CARPET_PATCH * s, vec2f(0.5));
  // How far the pixel lies inside its patch of each lattice (0 at its edge, 0.5 at its middle); away from the seam a
  // pixel reads one patch alone.
  let inside_a = min(min(pa.f.x, 1.0 - pa.f.x), min(pa.f.y, 1.0 - pa.f.y));
  let inside_b = min(min(pb.f.x, 1.0 - pb.f.x), min(pb.f.y, 1.0 - pb.f.y));
  let wave = CARPET_WAVE * noise_l(lat_turned(0.5 * s, vec2f(0.3)), CARPET_WAVES + c).x;
  let d = (inside_b - inside_a + wave) / CARPET_SEAM;
  if (d <= -CLAIMS_WITHIN) { return change_of(t, carpet_patch(t, pa, lowbias(pa.i, CARPET_SHIFTS + c), c == SNOW_CARPET, e / s), vec3f(0.0), 0.0, CARPET_HUE); }
  let b = carpet_patch(t, pb, lowbias(pb.i, CARPET_SHIFTS + c), c == SNOW_CARPET, e / s);
  if (d >= CLAIMS_WITHIN) { return change_of(t, b, b, 1.0, CARPET_HUE); }
  let a = carpet_patch(t, pa, lowbias(pa.i, CARPET_SHIFTS + c), c == SNOW_CARPET, e / s);
  let mean = max(dot(photo_mean(t), LUMA), 1e-3);
  return change_of(t, a, b, height_mix(d, dot(a, LUMA) / mean, dot(b, LUMA) / mean), CARPET_HUE);
}

// The change photograph t gives the ground's colour at the pixel, at e m a pixel: 1 where it averages to its mean. It
// is laid in hexagons (`hexes`, Mikkelsen 2022), each turned by any angle and shifted by its own draw (a photograph
// from above has no up), the hexagons about the pixel meeting by their heights (`hex_mix`): a stochastic tiling
// (Heitz and Neyret 2018), so no repeat shows and the mean stays the photograph's.
fn detail(t: i32, e: f32) -> vec3f {
  let s = detail_m(t);
  var h = hexes(DETAIL_HEX * s, DETAIL_EDGE);
  let least = hex_least(h);
  let mean = max(dot(photo_mean(t), LUMA), 1e-3);
  var c = array<vec3f, 3>(vec3f(1.0), vec3f(1.0), vec3f(1.0));
  for (var k = 0; k < 3; k++) {
    if (h.w[k] < least) { continue; }
    let x = lowbias(h.at[k], detail_noise(t));
    let turn = 2.0 * PI * f32(x & 1023u) / 1024.0;
    let r = mat2x2f(cos(turn), sin(turn), -sin(turn), cos(turn));
    let shift = vec2f(f32((x >> 10u) & 1023u), f32((x >> 20u) & 1023u)) / 1024.0;
    c[k] = photo(t, lat(r * (1.0 / s), shift).f, r * vec2f(e / s, 0.0), r * vec2f(0.0, e / s)) / mean;
  }
  let ratio = hex_mix(h, c[0], c[1], c[2]) * mean / max(photo_mean(t), vec3f(1e-3));
  return mix(vec3f(dot(ratio, LUMA)), ratio, DETAIL_HUE);
}

// The hexagons about the pixel (Mikkelsen 2022, hex-tiling): over a lattice of equal-sided triangles whose hexagons
// are `m` m across, the three corners of the pixel's triangle (`at`) and the pixel's weight in each over `edge`
// (`w`: the corner whose weight is largest by `CLAIMS_WITHIN` alone claims it).
struct Hexes { at: array<vec2i, 3>, w: vec3f }
fn hexes(m: f32, edge: f32) -> Hexes {
  // The triangle lattice: the plane skewed so its triangles are equal-sided, 2 sqrt 3 cells a hexagon's side.
  let skew = mat2x2f(1.0, 0.0, -0.57735027, 1.15470054) * (2.0 * 1.7320508 / m);
  let l = lat(skew, vec2f(0.0));
  let z = 1.0 - l.f.x - l.f.y;
  let up = select(0, 1, z < 0.0);
  let sg = 2.0 * f32(up) - 1.0;
  let w = vec3f(-z * sg, f32(up) - l.f.y * sg, f32(up) - l.f.x * sg) / edge;
  return Hexes(array<vec2i, 3>(l.i + vec2i(up, up), l.i + vec2i(up, 1 - up), l.i + vec2i(1 - up, up)), w);
}
// The least weight of a corner of `h` that claims the pixel.
fn hex_least(h: Hexes) -> f32 { return max(h.w.x, max(h.w.y, h.w.z)) - CLAIMS_WITHIN; }
// The changes `a`, `b` and `c` of the corners of `h` met by their heights (ragged.wgsl `claim`), so a pixel shows one
// image's grains, not the mean of three.
fn hex_mix(h: Hexes, a: vec3f, b: vec3f, c: vec3f) -> vec3f {
  let least = hex_least(h);
  let claims = vec3f(select(-1e6, claim(h.w.x, height_of(a)), h.w.x >= least), select(-1e6, claim(h.w.y, height_of(b)), h.w.y >= least),
    select(-1e6, claim(h.w.z, height_of(c)), h.w.z >= least));
  let top = max(claims.x, max(claims.y, claims.z));
  let k = vec3f(claim_weight(claims.x, top), claim_weight(claims.y, top), claim_weight(claims.z, top));
  return (k.x * a + k.y * b + k.z * c) / (k.x + k.y + k.z);
}

// Soil's images lie in patches about `SOIL_IMAGE_M` m across, each on a noise of its own (streams from `SOIL_NOISE`), and
// meet the images under them over `SOIL_EDGE` of that noise (made even, 0..1) by their heights (ragged.wgsl).
const SOIL_IMAGE_M = 2.5; const SOIL_EDGE = 0.06; const SOIL_NOISE = 180;
// An image that takes this much of what is left is the last (the rest is rounding).
const TAKES_ALL = 0.999;

// The lightness of a change, as a height (ragged.wgsl `claim`).
fn height_of(c: vec3f) -> f32 { return dot(c, LUMA); }

// The change soil's image i gives (`soil_detail`) to spot p: the tiles of cover of leaf litter, of a shaded floor of
// ferns, of dry stony soil and of cracked dry mud, the photograph of wet mud; a shore's: its bare sand
// (`beach_sand`), the tile of sand with its dry tufts and drift, the photographs of rounded pebbles and of dark wet mud
// (peat's too); the photographs of bare loam, of cracked clay, of dark soil rich in humus and of damp sand; and a
// deposit's: a flat of rippled sand, rippled silt, mud under algae and wet gravelly sand.
const SOIL_MUD = 4; const SOIL_BEACH = 5; const SOIL_PEBBLES = 7; const SOIL_DARK_MUD = 8; const SOIL_CLAY = 10;
const SOIL_WET_SAND = 12; const SOIL_FLAT = 13; const SOIL_SILT = 14; const SOIL_ALGAE = 15; const SOIL_WET_GRAVEL = 16;
const SOIL_IMAGES = 17;
fn soil_image(i: i32, p: Spot, wet: f32) -> vec3f {
  if (i == SOIL_MUD) { return detail(MUD_TEX, p.e); }
  if (i == SOIL_BEACH) { return beach_sand(p, wet); }
  if (i >= SOIL_PEBBLES) {
    var t = array<i32, 10>(ROUND_PEBBLE_TEX, DARK_MUD_TEX, LOAM_TEX, CLAY_TEX, HUMUS_TEX, WET_SAND_TEX, RIPPLE_FLAT_TEX,
      RIPPLE_SILT_TEX, ALGAL_MUD_TEX, WET_GRAVEL_TEX);
    return detail(t[i - SOIL_PEBBLES], p.e);
  }
  var c = array<i32, 7>(LITTER_CARPET, FERN_CARPET, SCRUB_CARPET, CRACKED_CARPET, -1, -1, SAND_CARPET);
  return carpet(c[i], p.e);
}

// A beach's bare sand, soaked to `wet` (0..1): dry sand, and damp sand streaked by the swash where it is soaked past
// half, the two meeting by their heights over an edge ragged by `SAND_WET_RAG` of noise over `SAND_WET_RAG_M`.
const SAND_WET_RAG = 0.15; const SAND_WET_RAG_M = 1.5;
fn beach_sand(p: Spot, wet: f32) -> vec3f {
  let d = (wet - 0.5 + SAND_WET_RAG * noise_l(lat_turned(SAND_WET_RAG_M, vec2f(0.45)), 189).x) / SOIL_EDGE;
  if (d <= -CLAIMS_WITHIN) { return detail(DRY_SAND_TEX, p.e); }
  if (d >= CLAIMS_WITHIN) { return detail(WET_SAND_TEX, p.e); }
  let a = detail(DRY_SAND_TEX, p.e);
  let b = detail(WET_SAND_TEX, p.e);
  return mix(a, b, height_mix(d, height_of(a), height_of(b)));
}

// Open sand (colour.wgsl `sandy_of`) is, where dry, drifted with its tufts (the tile of a shore's sand above the swash;
// wind moves bare sand, not a loam that holds together, Bagnold 1941), else damp. Ground soaked from `MUD_WET` (all of
// it from `MUD_WET_FULL`) is mud, but sand.
const MUD_WET = 0.8; const MUD_WET_FULL = 0.95;
// Cracked ground is the photograph of cracked clay as far as its fine grain's share passes `CLAY_PLATES` (all of it
// from `CLAY_PLATES_FULL`), else the tile of cracked mud with its stones.
const CLAY_PLATES = 0.25; const CLAY_PLATES_FULL = 0.5;
// A deposit's ground is soaked from `LAID_SOAKED.x`, all of it from `.y` (`soaked`): wet sand and silt keep their
// ripples, dry ones crumble and crack.
const LAID_SOAKED = vec2f(0.6, 0.85);
// The change the images give soil of spot p: under crowns its litter (a floor of ferns where wet); in the open peat
// (colour.wgsl `peat_of`) dark wet mud, wet mud where soaked, sand where its grain is coarse, dark humus as far as its
// organic matter darkens it (style.wgsl `humus`), cracked where driest as far as its grain shrinks (crag.wgsl
// `shrinks`), the rest stony soil and loam by their coarse grain; a deposit in the open (`laid_images`); on a shore
// its sand, pebbles or mud (colour.wgsl `shore_grain`), the mud of a fresh deposit rippled silt, the sand bare up to
// the storms' swash and with its tufts and drift above; and channels' pebbles where they lie.
fn soil_detail(p: Spot) -> vec3f {
  let canopy = min(p.canopy, 1.0);
  let dry = ease(0.45, 0.1, p.wet);
  let sandy = sandy_of(p.grain);
  let cracked = ease(0.25, 0.05, p.wet) * shrinks(p.grain) * (1.0 - sandy);
  let peat = peat_of(p);
  let open = 1.0 - canopy;
  let mud = open * (1.0 - peat) * ease(MUD_WET, MUD_WET_FULL, p.wet) * (1.0 - sandy);
  let soil = open * (1.0 - peat) - mud;
  let loam = soil * (1.0 - sandy) * (1.0 - humus(p.grain.z));
  let clay = ease(CLAY_PLATES, CLAY_PLATES_FULL, p.grain.y);
  var w = array<f32, SOIL_IMAGES>(canopy * (1.0 - p.wet), canopy * p.wet, loam * (1.0 - cracked) * p.grain.x,
    loam * cracked * (1.0 - clay), mud, 0.0, soil * sandy * dry, 0.0, open * peat, loam * (1.0 - cracked) * (1.0 - p.grain.x),
    loam * cracked * clay, soil * (1.0 - sandy) * humus(p.grain.z), soil * sandy * (1.0 - dry), 0.0, 0.0, 0.0, 0.0);
  laid_images(&w, p, open * (1.0 - peat), sandy);
  let sand = max(p.sand, 0.0);
  var wet = 0.0;
  if (sand > 0.0) {
    for (var i = 0; i < SOIL_IMAGES; i++) { w[i] *= 1.0 - sand; }
    let grain = shore_grain(p);
    let fine = sand * (1.0 - grain.x - grain.y);
    let bare = 1.0 - ease(0.0, STORM_RUN_UP * run_up(waves_of(p.sea, p.fetch), p.slope / earth.look.x), p.over);
    let fresh_mud = laid_share(p) * laid_fresh(p);
    w[SOIL_BEACH] += fine * bare;
    w[6] += fine * (1.0 - bare);
    w[SOIL_PEBBLES] += sand * grain.x;
    w[SOIL_DARK_MUD] += sand * grain.y * (1.0 - fresh_mud);
    w[SOIL_SILT] += sand * grain.y * fresh_mud;
    wet = grain.z;
  }
  var left = 0.0;
  for (var i = 0; i < SOIL_IMAGES; i++) { left += w[i]; }
  if (left <= 0.0) { return vec3f(1.0); }
  // The images lie in a stack, each over those after it: image i covers, of the ground the ones before leave, the share
  // its weight is of what is left, where its own noise falls under that share. So each holds its share of the ground
  // on the mean, and a small share is a few small patches of its own, not a thin band between two others. `d`: how far
  // inside its part each lies, in half widths of the edge; from `last` on, one covers all.
  var d: array<f32, SOIL_IMAGES>;
  for (var i = 0; i < SOIL_IMAGES; i++) { d[i] = -CLAIMS_WITHIN; }
  var last = SOIL_IMAGES - 1;
  for (var i = 0; i < SOIL_IMAGES; i++) {
    if (w[i] <= 0.0) { continue; }
    let take = w[i] / max(left, 1e-6);
    left -= w[i];
    if (take >= TAKES_ALL) { last = i; break; }
    let u = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(SOIL_IMAGE_M, vec2f(0.2 + 0.31 * f32(i))), SOIL_NOISE + i).x);
    d[i] = (take - u) / SOIL_EDGE;
    if (d[i] >= CLAIMS_WITHIN) { last = i; break; }
  }
  var out = soil_image(last, p, wet);
  for (var i = last - 1; i >= 0; i--) {
    if (d[i] <= -CLAIMS_WITHIN) { continue; }
    let c = soil_image(i, p, wet);
    out = mix(out, c, height_mix(d[i], height_of(out), height_of(c)));
  }
  // A channel's gravel and cobbles where the bed's stones lie (things.wgsl).
  out = bed_photos(out, p);
  return out;
}

// The images of a deposit (colour.wgsl `laid_share`) on the share `open` of spot p's ground in the open, of coarse grain
// as far as `sandy`, added to the soil's weights `w`, the rest of the soil's giving way to them. Fresh (`laid_fresh`)
// and soaked (`LAID_SOAKED`): a flat of rippled sand, or rippled silt, or wet gravelly sand where its channel moves
// stones (`Spot.moved`, from `BAR_SAND_STONE`); fresh and dry: bare sand (`beach_sand`), its fine grain cracked into
// plates, its gravel bare pebbles; old and soaked: its sand damp, its fine grain mud under a film of algae (algae coat
// mud that stays wet and still); old and dry, the soil as it is.
fn laid_images(w: ptr<function, array<f32, SOIL_IMAGES>>, p: Spot, open: f32, sandy: f32) {
  let laid = laid_share(p) * open;
  if (laid <= 0.0) { return; }
  let fresh = laid_fresh(p);
  let soak = ease(LAID_SOAKED.x, LAID_SOAKED.y, soaked(p));
  let gravel = ease(0.0, BAR_SAND_STONE, p.moved);
  let sand = laid * (1.0 - gravel) * sandy;
  let fine = laid * (1.0 - gravel) * (1.0 - sandy);
  var add = array<f32, SOIL_IMAGES>();
  add[SOIL_FLAT] = sand * fresh * soak;
  add[SOIL_SILT] = fine * fresh * soak;
  add[SOIL_WET_GRAVEL] = laid * gravel * soak;
  add[SOIL_BEACH] = sand * fresh * (1.0 - soak);
  add[SOIL_CLAY] = fine * fresh * (1.0 - soak);
  add[SOIL_PEBBLES] = laid * gravel * fresh * (1.0 - soak);
  add[SOIL_WET_SAND] = sand * (1.0 - fresh) * soak;
  add[SOIL_ALGAE] = fine * (1.0 - fresh) * soak;
  var taken = 0.0;
  for (var i = 0; i < SOIL_IMAGES; i++) { taken += add[i]; }
  for (var i = 0; i < SOIL_IMAGES; i++) { (*w)[i] = (*w)[i] * (1.0 - taken) + add[i]; }
}

// The change the photograph of mossy rock gives bare bed, at e m a pixel.
fn rock_detail(e: f32) -> vec3f { return detail(ROCK_TEX, e); }

// The change the grass gives the sward of cover s at e m a pixel: its tiles laid in hexagons (`grass_hexes`),
// green, dry as far as the sward is straw (`straw`, 0..1), with tall reeds as far as they stand (`reeds`, 0..1); its fine
// change `GRASS_FAR` of itself from `GRASS_FAR_E` m a pixel on, where it would read as noise; lighter and darker,
// greener and yellower over drifts `DRIFT_GRASS_M` m across (`DRIFT_TONE`, `DRIFT_HUE`), all about 1 so its mean stays
// the world's colour.
const GRASS_PATCH_M = 6.0; const GRASS_THIN = 0.5;
const DRIFT_GRASS_M = 6.0; const DRIFT_TONE = 0.06;
const DRIFT_HUE = vec3f(-0.04, 0.025, -0.05); const GRASS_FAR = 0.5; const GRASS_FAR_E = vec2f(0.01, 0.04);
fn grass_detail(e: f32, straw: f32, reeds: f32, s: f32) -> vec3f {
  // Which kinds the hexagons hold: short turf, a meadow, tussocks, as a noise over `GRASS_PATCH_M` m falls, a thinner
  // sward leaning to tussocks (`GRASS_THIN`).
  let n = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(GRASS_PATCH_M, vec2f(0.4)), 102).x
    + 0.35 * noise_l(lat_turned(0.31 * GRASS_PATCH_M, vec2f(0.9)), 103).x) + GRASS_THIN * (0.6 - s);
  var out = vec3f(1.0);
  if (straw < 1.0) { out = grass_hexes(e, n, false, reeds); }
  if (straw > 0.0) { out = mix(out, grass_hexes(e, n, true, reeds), straw); }
  out = 1.0 + mix(1.0, GRASS_FAR, ease(GRASS_FAR_E.x, GRASS_FAR_E.y, e)) * (out - 1.0);
  let d = noise_l(lat_turned(DRIFT_GRASS_M, vec2f(0.2)), 104).x + 0.5 * noise_l(lat_turned(0.4 * DRIFT_GRASS_M, vec2f(0.6)), 105).x;
  let h = noise_l(lat_turned(2.3 * DRIFT_GRASS_M, vec2f(0.8)), 106).x;
  return out * (1.0 + DRIFT_TONE * d) * (1.0 + DRIFT_HUE * h);
}

// The grass's tiles laid in hexagons (`hexes`) `HEX_M` m across, each
// corner a hexagon holding one of the three tiles (by its own draw against `n`, 0..1 from turf to tussocks: kinds mix
// at their edges hexagon by hexagon), or reeds in `reeds` of them, shifted and mirrored by its own draw; the three
// corners of a pixel's triangle meet by their heights (ragged.wgsl `claim`) over `HEX_EDGE` of its weights either way
// (so a pixel shows one tile's blades, not the mean of three), each tile's detail finer than `GRASS_DETAIL_M` alone.
// Dry tiles where `dry`. The change it gives a colour, about 1.
const HEX_M = 1.6; const HEX_EDGE = 0.08; const HEX_MIX = 0.25; const GRASS_DETAIL_M = 0.25;
fn grass_hexes(e: f32, n: f32, dry: bool, reeds: f32) -> vec3f {
  var h = hexes(HEX_M, HEX_EDGE);
  // Each corner near enough the nearest to claim the pixel (the others none): its tile's change.
  let least = hex_least(h);
  var c = array<vec3f, 3>(vec3f(1.0), vec3f(1.0), vec3f(1.0));
  for (var k = 0; k < 3; k++) {
    if (h.w[k] >= least) { c[k] = hex_tile(h.at[k], e, n, dry, reeds); }
  }
  let out = hex_mix(h, c[0], c[1], c[2]);
  return mix(vec3f(dot(out, LUMA)), out, CARPET_HUE);
}

// The change the tile of the hexagon at `corner` gives (`grass_hexes`), at e m a pixel.
fn hex_tile(corner: vec2i, e: f32, n: f32, dry: bool, reeds: f32) -> vec3f {
  var green = array<i32, 3>(TURF_CARPET, GRASS_CARPET, TUSSOCK_CARPET);
  var dried = array<i32, 3>(DRY_TURF_CARPET, STRAW_CARPET, DRY_TUSSOCK_CARPET);
  let x = lowbias(corner, 107);
  // Its kind: the noise, with `HEX_MIX` of the hexagon's own draw.
  let pick = clamp(mix(n, f32(x & 1023u) / 1024.0, HEX_MIX), 0.0, 0.999);
  // Reeds in as many hexagons as they stand (by a draw of the hexagon's own).
  let reed = f32(lowbias(corner, 109) & 1023u) / 1024.0 < reeds;
  let c = select(select(green[i32(pick * 3.0)], dried[i32(pick * 3.0)], dry), REED_CARPET, reed);
  let t = CARPETS_FROM + c;
  let side = carpet_m(c);
  let shift = vec2f(f32((x >> 10u) & 1023u), f32((x >> 20u) & 1023u)) / 1024.0;
  var uv = lat_axes(side, shift).f;
  var gx = vec2f(e / side, 0.0);
  if ((x >> 30u) == 1u) { uv.x = 1.0 - uv.x; gx = -gx; }
  // Its fine detail alone: the tile over itself blurred to `GRASS_DETAIL_M` (a mip as coarse), so its blades show and
  // no clump of lighter grass larger than that repeats as a spot.
  let fine = photo(t, uv, gx, vec2f(0.0, e / side));
  let g_blur = max(e, GRASS_DETAIL_M) / side;
  let blur = photo(t, uv, sign(gx) * g_blur, vec2f(0.0, g_blur));
  return fine / max(blur, vec3f(1e-3));
}

// Blades of grass drawn one by one, near (cover_field.wgsl `sward`): over the pixel, from the cells of a lattice `BLADE_CELL_M` m wide,
// two blades a cell at random feet, each `BLADE_LONG` m long by its own draw standing up `STAND` of that,
// leaning and bending its own way, `BLADE_W` m wide at its foot narrowing to its tip, dark at its foot and light at its
// tip (`BLADE_TONE`); the nearer over the farther. Its tone over the sward's colour and its cover (they show near:
// cover_field.wgsl `BLADE_SEEN_E`).
const BLADE_CELL_M = 0.035; const BLADE_LONG = vec2f(0.06, 0.2); const BLADE_W = 0.006; const BLADE_TONE = vec2f(0.45, 1.35);
fn blades(e: f32) -> vec2f {
  let l = lat_axes(BLADE_CELL_M, vec2f(0.0));
  var tone = 0.0;
  var cover = 0.0;
  var nearest = -99.0;
  for (var j = 3; j >= 0; j--) {
    for (var i = -1; i <= 1; i++) {
      let cell = l.i + vec2i(i, j);
      for (var b = 0u; b < 2u; b++) {
        let x = lowbias(cell, 108 + i32(b));
        let foot = vec2f(f32(i), f32(j)) + vec2f(f32(x & 1023u), f32((x >> 10u) & 1023u)) / 1024.0;
        let long = mix(BLADE_LONG.x, BLADE_LONG.y, f32((x >> 20u) & 255u) / 255.0) * 0.5 / BLADE_CELL_M;
        let q = l.f - foot;
        let up = -q.y / long;
        if (up < 0.0 || up > 1.0) { continue; }
        let lean = (f32((x >> 28u) & 15u) / 7.5 - 1.0) * 0.6;
        let mid = long * (lean * up + 0.5 * lean * up * up);
        let half = 0.5 * BLADE_W / BLADE_CELL_M * (1.0 - 0.85 * up);
        let off = abs(q.x - mid);
        let a = 1.0 - smoothstep(half, half + e / BLADE_CELL_M, off);
        if (a <= 0.0 || foot.y < nearest) { continue; }
        nearest = foot.y;
        let own = 0.85 + 0.3 * f32((x >> 4u) & 63u) / 63.0;
        tone = mix(tone, mix(BLADE_TONE.x, BLADE_TONE.y, up) * own * (1.0 - 0.25 * sign(q.x - mid) * lean), a);
        cover = max(cover, a);
      }
    }
  }
  return vec2f(tone, cover);
}


// Plants of colour c, covering s of the ground, seen at e m a pixel, as the tufts `t` (cover_field.wgsl `Tufts`:
// rendered models, tex.wgsl) standing on their foot, one in each cell of its lattice that holds one (as many as make
// the cover, its share of a tuft's box being blades), of its set or by its own draw of its second (`Tufts.other`),
// `TUFT_WIDE` cells wide by its own draw, the nearer over the farther, each over its shadow (`Tufts.shadow`: as deep,
// its middle that many of its widths down the sun and as wide); a blade keeps its lightness about the tufts' mean and
// takes the plants' colour, keeping `TUFT_HUE` of its own.
// They show as things (draw.wgsl `as_thing`): smaller, the plants are their veil (colour `veil` over their share).
// Each stands its stem's foot on its foot (tex.wgsl `foot_rise`); standing in liquid `depth` m deep (0: on land), each
// of its texels under the surface by its own height (tex.wgsl `model_height_m`) is seen through it (style.wgsl
// `seen_in_liquid`). Their colour (linear) and cover.
const TUFT_WIDE = vec2f(1.0, 1.8);
const TUFT_HUE = 0.3;
fn sward_tufts(c: vec3f, veil: vec3f, e: f32, s: f32, t: Tufts, depth: f32) -> vec4f {
  let wide = 0.5 * (TUFT_WIDE.x + TUFT_WIDE.y);
  let seen = as_thing(wide * t.cell_m, e);
  if (!models_ready() || seen <= 0.0) { return vec4f(veil, s); }
  let l = lat_axes(t.cell_m, vec2f(0.0));
  let held = s / (t.covers * t.aspect * wide * wide);
  let sun = normalize(LIGHT.xy);
  var dst = vec4f(0.0);
  // From the nearest (south) to the farthest, each under those already drawn: a tuft no wider than two cells reaches
  // the pixel from the cells beside its own, from up to `t.south` cells south (its height), and its shadow from the cell
  // north.
  for (var j = t.south; j >= -1; j--) {
    for (var i = -1; i <= 1; i++) {
      let cell = l.i + vec2i(i, j);
      let x = lowbias(cell, t.k);
      if (f32(x & 1023u) / 1024.0 >= held) { continue; }
      let foot = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0;
      let other = f32((lowbias(cell, t.k + 1) >> 8u) & 1023u) / 1024.0 < t.others;
      let k = select(t.first + i32((x >> 26u) % u32(t.n)), t.other.x + i32((x >> 26u) % u32(t.other.y)), other);
      let w = mix(TUFT_WIDE.x, TUFT_WIDE.y, f32(lowbias(cell, t.k + 1) & 255u) / 255.0);
      let h = w * model_h(k) / model_w(k);
      let v = l.f - foot;
      let uv = clamp(vec2f(v.x / w + 0.5, 1.0 + v.y / h - foot_rise(k)), vec2f(-1.0), vec2f(2.0));
      let inside = all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0));
      let tex = select(vec4f(0.0), model_texel(k, uv, e / (w * t.cell_m)), inside);
      let lit = dot(tex.rgb, LUMA) / TUFT_MEAN;
      var rgb = mix(c * lit, tex.rgb * dot(c, LUMA) / TUFT_MEAN, TUFT_HUE);
      if (depth > 0.0 && tex.a > 0.0) { rgb = seen_in_liquid(rgb, depth, model_height_m(k, uv, e / (w * t.cell_m), -v.y * t.cell_m, h * t.cell_m), e / MAP_VIEW.y); }
      dst += vec4f(rgb * tex.a, tex.a) * (1.0 - dst.a);
      // Its shadow away from the sun, on the ground at its foot or on the surface of the liquid it stands in (where its
      // stem meets it, the cosine of the depth north of its foot: water.wgsl `MAP_VIEW`), as much as it stands out of it.
      let rise = min(MAP_VIEW.y * depth / t.cell_m, h);
      let sv = (v + vec2f(0.0, rise) + sun * t.shadow.y * w) / (t.shadow.z * vec2f(0.4 * w, 0.2 * w));
      dst += vec4f(0.0, 0.0, 0.0, t.shadow.x * (1.0 - rise / h) * (1.0 - smoothstep(0.5, 1.0, length(sv)))) * (1.0 - dst.a);
    }
  }
  return mix(vec4f(veil, s), vec4f(dst.rgb / max(dst.a, 1e-4), dst.a), seen);
}

// The flowers' heads up close (cover_field.wgsl `flowers_at`): the share of the pixel their petals cover and
// their lightness, from the tile of petals, a petal's texel `PETAL_LIT` light (linear) where it is lit as a whole.
const PETAL_LIT = 0.25;
fn flower_heads(e: f32) -> vec2f {
  let s = carpet_m(FLOWER_CARPET);
  let g = vec2f(e / s, 0.0);
  let v = dot(photo(CARPETS_FROM + FLOWER_CARPET, lat_axes(s, vec2f(0.0)).f, g, g.yx), LUMA) / PETAL_LIT;
  return vec2f(min(v, 1.0), clamp(v, 0.5, 1.6));
}

// Snow's own texture over its colour (1 on the mean): wind drifts `DRIFT_M` m long and a quarter of that across, lying
// along the wind (`WIND_TURN`), lit by their slope (the sun's side warm white, the lee side in the sky's blue,
// `SNOW_LEE`), sastrugi `SASTRUGI_M` m long carved along them, the crust's grain (`SNOW_CRUST`), and a glint here and there where a
// crystal turns to the sun (`GLINT`, one in `GLINT_RARE` of the cells `GLINT_M` across); near, its tile of cover:
// wind ripples, stones and dry stalks.
const DRIFT_M = 18.0; const SASTRUGI_M = 2.5; const WIND_TURN = 0.35; const SNOW_RELIEF = 0.28; const SASTRUGI = 0.12;
const SNOW_LEE = vec3f(0.86, 0.91, 1.02); const SNOW_SUN = vec3f(1.03, 1.02, 0.98); const SNOW_CRUST = 0.05; const SNOW_CRUST_M = 0.06;
const GLINT = 0.5; const GLINT_RARE = 0.012; const GLINT_M = 0.04;
fn snow_texture(e: f32) -> vec3f {
  let r = mat2x2f(cos(WIND_TURN), sin(WIND_TURN), -sin(WIND_TURN), cos(WIND_TURN));
  let sun = normalize(LIGHT.xy);
  var slope = vec2f(0.0);
  if (!unseen(0.25 * DRIFT_M)) {
    let n = noise_l(lat(mat2x2f(1.0 / DRIFT_M, 0.0, 0.0, 2.5 / DRIFT_M) * r, vec2f(0.0)), 130);
    slope += SNOW_RELIEF * n.yz;
  }
  if (!unseen(0.25 * SASTRUGI_M)) {
    let n = noise_l(lat(mat2x2f(1.0 / SASTRUGI_M, 0.0, 0.0, 3.0 / SASTRUGI_M) * r, vec2f(3.0)), 131);
    slope += SASTRUGI * n.yz * shows_at(0.25 * SASTRUGI_M, e);
  }
  let lit = clamp(dot(slope, sun), -1.0, 1.0);
  var out = mix(vec3f(1.0), select(SNOW_SUN, SNOW_LEE, lit < 0.0), abs(lit));
  if (!unseen(SNOW_CRUST_M)) { out *= 1.0 + SNOW_CRUST * shows_at(SNOW_CRUST_M, e) * noise_l(lat_turned(SNOW_CRUST_M, vec2f(1.0)), 132).x; }
  if (!unseen(GLINT_M)) {
    let l = lat_axes(GLINT_M, vec2f(0.0));
    let x = lowbias(l.i, 133);
    if (f32(x & 65535u) / 65536.0 < GLINT_RARE) { out *= 1.0 + GLINT * shows_at(GLINT_M, e) * (1.0 - smoothstep(0.1, 0.35, length(l.f - 0.5))); }
  }
  out *= carpet(SNOW_CARPET, e);
  return out;
}

// Bare ground's filler: where it is cold or wet, mats of lichen and moss in patches `MAT_M` m across over up to
// `MATS` of it, grey-green and ochre (`LICHEN`, `MOSS_MAT`), in the mossy floor's photograph. (Its dry tufts and
// stones are in its images.)
const MAT_M = 0.7; const MATS = 0.3;
const LICHEN = vec3f(0.36, 0.37, 0.31); const MOSS_MAT = vec3f(0.20, 0.24, 0.12); const LICHEN_OCHRE = vec3f(0.48, 0.38, 0.20);
fn dry_filler(g0: vec3f, p: Spot, bare: f32) -> vec3f {
  let open = bare * (1.0 - min(p.canopy, 1.0)) * (1.0 - p.snow) * (1.0 - p.sand) * (1.0 - p.rock);
  if (open <= 0.02) { return g0; }
  var g = g0;
  let cold = max(ease(280.0, 272.0, p.t), ease(0.4, 0.8, p.wet));
  if (cold > 0.0 && !unseen(0.25 * MAT_M)) {
    let n = noise_l(lat_turned(MAT_M, vec2f(2.0)), 140);
    let frayed = n.x + 0.45 * noise_l(lat_turned(0.18 * MAT_M, vec2f(7.0)), 142).x;
    let m = 0.85 * smoothstep(0.15, 0.3, frayed + MATS * cold - 0.55) * shows_at(0.25 * MAT_M, p.e) * open;
    let kind = smoothstep(-0.3, 0.3, noise_l(lat_turned(3.0 * MAT_M, vec2f(5.0)), 141).x);
    let mat = mix(mix(LICHEN, MOSS_MAT, p.wet), LICHEN_OCHRE, 0.25 * kind) * photo_light(MOSSY_TEX, lat_axes(3.0, vec2f(0.0)).f, p.e / 3.0);
    g = mix(g, mat, m);
  }
  return g;
}
