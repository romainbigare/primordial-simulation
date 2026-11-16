// The ground's detail up close (web/src/detail.ts): photographs of ground seen from above and, angled, tiles of ground
// cover rendered from 3D models as that view sees it (tex.wgsl `CARPETS_FROM`), each a layer of `detail_tex` with its
// mips down to its mean, laid on the map at its real size and read as a change around the colour the world gives the
// ground: the image over its mean. From afar the mips average the image to its mean, so the ground's colour there is
// the world's alone; near, the same colour shows its leaves, stones, cracks, grass and sticks. Each image is laid
// twice, blended by noise, so no repeat shows: a photograph the second time turned and `DETAIL_AGAIN` times as large,
// a tile of cover (whose things stand up the screen) in patches each shifted by its own draw (`carpet`).

// Each photograph's side in the world, m (web/textures/SOURCES.md).
fn detail_m(t: i32) -> f32 { var m = array<f32, 9>(1.26, 3.0, 2.0, 2.0, 1.5, 2.0, 15.0, 50.0, 1.3); return m[t]; }
// The turned laying is this many times as large; the two blend over noise `DETAIL_BLEND` times the image's size.
const DETAIL_AGAIN = 2.7; const DETAIL_BLEND = 3.0;
const CARPET_PATCH = 1.7; const CARPET_SEAM = 0.02; const CARPET_WAVE = 0.06;
// An image's own hue shows this much (the rest of its change is its lightness alone): the world gives the colour; a
// tile of cover's more (`CARPET_HUE`), its green plants on brown ground.
const DETAIL_HUE = 0.55; const CARPET_HUE = 0.7;

// The change image `t` of the array gives a colour: `a` and `b` its two layings, blended by noise `w`.
fn change_of(t: i32, a: vec3f, b: vec3f, w: f32, hue: f32) -> vec3f {
  let ratio = mix(a, b, w) / max(photo_mean(t), vec3f(1e-3));
  return mix(vec3f(dot(ratio, LUMA)), ratio, hue);
}

// Whether the map is drawn angled (the ground's cover then stands up).
fn angled() -> bool { return cover_look.look.z > 0.5; }

// The tile of cover drawn, angled, in place of photograph t: none (-1) for rock, pebbles and mud.
fn carpet_of(t: i32) -> i32 {
  var c = array<i32, 9>(LITTER_CARPET, FERN_CARPET, GRASS_CARPET, SCRUB_CARPET, CRACKED_CARPET, -1, SAND_CARPET, -1, -1);
  return c[t];
}

// The change tile of cover c gives the ground's colour at the pixel, at e m a pixel. The tile is laid in patches
// `CARPET_PATCH` tiles wide, each shifted by its own draw, in two lattices of patches half a patch apart: the pixel
// shows the patch of the lattice whose middle it is nearer, the two meeting in a seam `CARPET_SEAM` wide (of a patch),
// wavy by `CARPET_WAVE`, so no repeat shows and a pixel shows one patch's grass and stones, not the mean of two.
fn carpet(c: i32, e: f32) -> vec3f {
  let s = carpet_m(c);
  let g = vec2f(e / s, 0.0);
  let t = CARPETS_FROM + c;
  let pa = lat_axes(CARPET_PATCH * s, vec2f(0.0));
  let pb = lat_axes(CARPET_PATCH * s, vec2f(0.5));
  let xa = lowbias(pa.i, 96 + t);
  let xb = lowbias(pb.i, 96 + t);
  let a = photo(t, lat_axes(s, vec2f(f32(xa & 1023u), f32((xa >> 10u) & 1023u)) / 1024.0).f, g, g.yx);
  let b = photo(t, lat_axes(s, vec2f(f32(xb & 1023u), f32((xb >> 10u) & 1023u)) / 1024.0).f, g, g.yx);
  // How far the pixel lies inside its patch of each lattice (0 at its edge, 0.5 at its middle).
  let inside_a = min(min(pa.f.x, 1.0 - pa.f.x), min(pa.f.y, 1.0 - pa.f.y));
  let inside_b = min(min(pb.f.x, 1.0 - pb.f.x), min(pb.f.y, 1.0 - pb.f.y));
  let wave = CARPET_WAVE * noise_l(lat_turned(0.5 * s, vec2f(0.3)), 98 + t).x;
  let w = smoothstep(-CARPET_SEAM, CARPET_SEAM, inside_b - inside_a + wave);
  return change_of(t, a, b, w, CARPET_HUE);
}

// The change photograph t gives the ground's colour at the pixel, at e m a pixel: 1 where it averages to its mean;
// angled, its tile of cover's where it has one.
fn detail(t: i32, e: f32) -> vec3f {
  let c = carpet_of(t);
  if (c >= 0 && angled()) { return carpet(c, e); }
  let s = detail_m(t);
  let a = photo(t, lat_axes(s, vec2f(0.0)).f, vec2f(e / s, 0.0), vec2f(0.0, e / s));
  let k = DETAIL_AGAIN * s;
  let r = mat2x2f(cos(TURN_A), sin(TURN_A), -sin(TURN_A), cos(TURN_A));
  let b = photo(t, lat_turned(k, vec2f(0.37)).f, r * vec2f(e / k, 0.0), r * vec2f(0.0, e / k));
  let w = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(DETAIL_BLEND * s, vec2f(0.61)), 95 + t).x);
  return change_of(t, a, b, w, DETAIL_HUE);
}

// Soil's images meet in patches this wide, m, over this much of their noise (`soil_detail`).
const SOIL_IMAGE_M = 2.5; const SOIL_SEAM = 0.04;

// The change the photographs give soil of spot p: under crowns its litter (a mossy floor where wet), in the open dry
// stony soil, cracked where driest, wet mud where wet, beach sand and channels' pebbles where they lie.
fn soil_detail(p: Spot) -> vec3f {
  let canopy = min(p.canopy, 1.0);
  let dry = ease(0.45, 0.1, p.wet);
  let cracked = ease(0.25, 0.05, p.wet);
  var w = array<f32, 6>(canopy * (1.0 - p.wet), canopy * p.wet, (1.0 - canopy) * dry * (1.0 - cracked), (1.0 - canopy) * cracked, (1.0 - canopy) * (1.0 - dry), 0.0);
  var t = array<i32, 6>(LITTER_TEX, MOSSY_TEX, DRY_TEX, CRACKED_TEX, MUD_TEX, SAND_TEX);
  let sand = max(p.sand, 0.0);
  for (var i = 0; i < 5; i++) { w[i] *= 1.0 - sand; }
  w[5] = sand;
  var total = 0.0;
  for (var i = 0; i < 6; i++) { total += w[i]; }
  if (total <= 0.0) { return vec3f(1.0); }
  // Each image holds its share of the ground in patches (`SOIL_IMAGE_M`) where a noise, made even, falls in its part of
  // the shares, meeting the next over `SOIL_SEAM` of the noise: a pixel shows one image, not the mean of several.
  let n = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(SOIL_IMAGE_M, vec2f(0.2)), 97).x);
  var sum = vec3f(0.0);
  var kept = 0.0;
  var lo = 0.0;
  for (var i = 0; i < 6; i++) {
    let hi = lo + w[i] / total;
    let k = smoothstep(lo - SOIL_SEAM, lo + SOIL_SEAM, n) * (1.0 - smoothstep(hi - SOIL_SEAM, hi + SOIL_SEAM, n));
    lo = hi;
    if (k < 0.01) { continue; }
    sum += k * detail(t[i], p.e);
    kept += k;
  }
  var out = sum / max(kept, 1e-4);
  // River pebbles where the bed's stones lie, in patches of their own as many as it holds.
  if (p.stony > 0.02) {
    let m = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(SOIL_IMAGE_M, vec2f(0.7)), 99).x);
    out = mix(out, detail(PEBBLE_TEX, p.e), smoothstep(-SOIL_SEAM, SOIL_SEAM, min(p.stony, 1.0) - m));
  }
  return out;
}

// The change the photograph of mossy rock gives bare bed, at e m a pixel.
fn rock_detail(e: f32) -> vec3f { return detail(ROCK_TEX, e); }

// The change the grass gives the sward, at e m a pixel; angled, its tile of grass, of straw as far as the sward is
// straw (`straw`, 0..1), of reeds as far as reeds stand (`reeds`, 0..1).
fn grass_detail(e: f32, straw: f32, reeds: f32) -> vec3f {
  if (!angled()) { return detail(GRASS_TEX, e); }
  var out = carpet(GRASS_CARPET, e);
  if (straw > 0.0) { out = mix(out, carpet(STRAW_CARPET, e), straw); }
  if (reeds > 0.0) { out = mix(out, carpet(REED_CARPET, e), reeds); }
  return out;
}

// A thin sward of colour c, covering s of the ground, seen at e m a pixel: its tufts (rendered models, tex.wgsl
// `TUFT_FROM`) standing on their foot, one in each cell of a lattice `TUFT_CELL_M` m wide that holds one (as many as
// make the sward's cover, `TUFT_COVERS` of a tuft's box being blades), `TUFT_WIDE` cells wide by its own draw, the
// nearer over the farther, each over its shadow (`TUFT_SHADOW` deep); a blade keeps its lightness about the tufts' mean
// and takes the sward's colour, keeping `TUFT_HUE` of its own. Their colour (linear) and cover.
const TUFT_CELL_M = 0.12; const TUFT_WIDE = vec2f(1.5, 3.0); const TUFT_SHADOW = 0.35;
// A tuft's blades cover `TUFT_COVERS` of its box, which is `TUFT_ASPECT` as tall as wide (measured on the sprites).
const TUFT_COVERS = 0.14; const TUFT_ASPECT = 0.81;
const TUFT_HUE = 0.3;
fn sward_tufts(c: vec3f, e: f32, s: f32) -> vec4f {
  if (!models_ready()) { return vec4f(c, s); }
  let l = lat_axes(TUFT_CELL_M, vec2f(0.0));
  let wide = 0.5 * (TUFT_WIDE.x + TUFT_WIDE.y);
  let held = s / (TUFT_COVERS * TUFT_ASPECT * wide * wide);
  let sun = normalize(LIGHT.xy);
  var dst = vec4f(0.0);
  // From the nearest (south) to the farthest, each under those already drawn.
  for (var j = 3; j >= 0; j--) {
    for (var i = -1; i <= 1; i++) {
      let cell = l.i + vec2i(i, j);
      let x = lowbias(cell, 170);
      if (f32(x & 1023u) / 1024.0 >= held) { continue; }
      let foot = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0;
      let k = TUFT_FROM + i32((x >> 26u) % u32(TUFTS));
      let w = mix(TUFT_WIDE.x, TUFT_WIDE.y, f32(lowbias(cell, 171) & 255u) / 255.0);
      let h = w * model_h(k) / model_w(k);
      let v = l.f - foot;
      let uv = clamp(vec2f(v.x / w + 0.5, 1.0 + v.y / h), vec2f(-1.0), vec2f(2.0));
      let inside = all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0));
      let t = select(vec4f(0.0), model_texel(k, uv, e / (w * TUFT_CELL_M)), inside);
      let lit = dot(t.rgb, LUMA) / TUFT_MEAN;
      let rgb = mix(c * lit, t.rgb * dot(c, LUMA) / TUFT_MEAN, TUFT_HUE);
      dst += vec4f(rgb * t.a, t.a) * (1.0 - dst.a);
      // Its shadow on the ground at its foot, away from the sun.
      let sv = (v + sun * 0.25 * w) / vec2f(0.45 * w, 0.2 * w);
      dst += vec4f(0.0, 0.0, 0.0, TUFT_SHADOW * (1.0 - smoothstep(0.5, 1.0, length(sv)))) * (1.0 - dst.a);
    }
  }
  return vec4f(dst.rgb / max(dst.a, 1e-4), dst.a);
}

// The flowers' heads up close, angled (cover_field.wgsl `flowers_at`): the share of the pixel their petals cover and
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
// crystal turns to the sun (`GLINT`, one in `GLINT_RARE` of the cells `GLINT_M` across); angled, near, its tile of
// cover: wind ripples, stones and dry stalks.
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
  if (angled()) { out *= carpet(SNOW_CARPET, e); }
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

// Things lying on the ground (rendered models, tex.wgsl), angled, standing on their foot with their shadow away from the
// sun, each sort at its own lattice, one in `fill` of its cells, a size between its two by its own draw (more small than
// large), as many as the ground suits them: rocks where it is bare, rocky, steep or loose (a snowcap on top where snow
// lies), large ones in cells of `ROCK_CELL_M` m and small in cells of `PEBBLE_CELL_M` m; fallen trunks and branches
// under crowns and, fewer, on dry open ground (`WOOD_DRY`); dead bushes on dry bare ground.
const ROCK_CELL_M = 7.0; const ROCK_FILL = 0.35; const ROCK_M = vec2f(0.5, 2.4);
const PEBBLE_CELL_M = 1.6; const PEBBLE_FILL = 0.3; const PEBBLE_M = vec2f(0.12, 0.45);
// Sand holds `1 - SAND_BARE` as many rocks and dead bushes as other bare ground.
const SAND_BARE = 0.6;
const WOOD_CELL_M = 6.0; const WOOD_FILL = 0.2; const WOOD_M = vec2f(1.5, 5.0); const WOOD_DRY = 0.35;
const DEAD_BUSH_CELL_M = 4.0; const DEAD_BUSH_FILL = 0.3; const DEAD_BUSH_M = vec2f(0.5, 1.3);
// A thing's shadow: this dark at its middle, as long across as the thing and `SHADOW_DEEP` of that along the sun.
const OBJECT_SHADOW = 0.45; const SHADOW_DEEP = 0.5;
// Each sort's sprites: rocks, wood, dead bushes (tex.wgsl).
const ROCK_SORT = 0; const WOOD_SORT = 1; const DEAD_BUSH_SORT = 2;
fn sort_sprite(sort: i32, k: u32) -> i32 {
  if (sort == ROCK_SORT) { let r = i32(k % u32(ROCKS)); return select(ROCKS_MORE_FROM + r - BOULDERS, BOULDER_FROM + r, r < BOULDERS); }
  // Wood: the fallen trunk lying across the view (its first side) and the branches.
  if (sort == WOOD_SORT) { let r = i32(k % u32(WOODS)); return select(BRANCH_FROM + r - 1, LOG_FROM, r == 0); }
  return DEAD_BUSH_FROM + i32(k % u32(DEAD_BUSHES));
}

// Ground of colour g at spot p with one sort of things over it: `many` (0..1) as the ground suits them, at stream k.
fn scatter(g: vec3f, p: Spot, sort: i32, cell_m: f32, fill: f32, size: vec2f, many: f32, k: i32) -> vec3f {
  if (many <= 0.02 || unseen(0.3 * size.x)) { return g; }
  let l = lat_axes(cell_m, vec2f(0.0));
  let sun = normalize(LIGHT.xy);
  var out = g;
  // From the farthest (north) to the nearest; a thing reaches up into the cell north of its foot.
  for (var j = 0; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let c = l.i + vec2i(i, j);
      let x = lowbias(c, k);
      if (f32(x & 1023u) / 1024.0 >= fill * many) { continue; }
      let foot = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0 * 0.8 + 0.1;
      let s = sort_sprite(sort, x >> 26u);
      let w = mix(size.x, size.y, pow(f32(lowbias(c, k + 1) & 255u) / 255.0, 2.0)) / cell_m;
      let h = w * model_h(s) / model_w(s);
      let v = l.f - foot;
      let sv = (v + sun * SHADOW_DEEP * 0.7 * w) / vec2f(0.6 * w, SHADOW_DEEP * 0.6 * w);
      out *= 1.0 - OBJECT_SHADOW * (1.0 - smoothstep(0.6, 1.0, length(sv)));
      let uv = vec2f(v.x / w + 0.5, 1.0 + v.y / h);
      if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { continue; }
      let t = model_texel(s, uv, p.e / (w * cell_m));
      // Snow on its top where the ground holds some.
      let rgb = mix(t.rgb, SNOW, p.snow * smoothstep(0.55, 0.25, uv.y) * f32(sort != DEAD_BUSH_SORT));
      out = mix(out, rgb, t.a);
    }
  }
  return out;
}

// The things lying on ground of colour g at spot p, where the plants leave `bare` of it bare.
fn objects(g: vec3f, p: Spot, bare: f32) -> vec3f {
  if (!angled() || !models_ready()) { return g; }
  let canopy = min(p.canopy, 1.0);
  let open = bare * (1.0 - SAND_BARE * p.sand) * (1.0 - canopy);
  let stony = open * max(max(p.rock, ease(0.15, 0.6, p.slope)), 0.35 + 0.65 * ease(1.0, 0.1, p.loose));
  let dry = ease(0.4, 0.1, p.wet) * open * (1.0 - p.snow);
  let wood = max(canopy, WOOD_DRY * dry) * (1.0 - p.sand) * (1.0 - p.snow);
  var out = scatter(g, p, WOOD_SORT, WOOD_CELL_M, WOOD_FILL, WOOD_M, wood, 120);
  out = scatter(out, p, DEAD_BUSH_SORT, DEAD_BUSH_CELL_M, DEAD_BUSH_FILL, DEAD_BUSH_M, dry, 124);
  out = scatter(out, p, ROCK_SORT, PEBBLE_CELL_M, PEBBLE_FILL, PEBBLE_M, stony, 152);
  return scatter(out, p, ROCK_SORT, ROCK_CELL_M, ROCK_FILL, ROCK_M, stony, 150);
}
