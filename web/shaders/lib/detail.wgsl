// The ground's detail up close (web/src/detail.ts): photographs of ground seen from above, each a layer of
// `detail_tex` with its mips down to its mean, laid on the map at its real size (`detail_m`) and read as a change around
// the colour the world gives the ground: the photograph over its mean. From afar the mips average the photograph to its
// mean, so the ground's colour there is the world's alone; near, the same colour shows its leaves, stones and cracks.
// Each photograph is laid twice, along the world's axes and turned at `DETAIL_AGAIN` times its size, blended by noise,
// so no repeat shows.

// Each photograph's side in the world, m (web/textures/SOURCES.md).
fn detail_m(t: i32) -> f32 { var m = array<f32, 9>(1.26, 3.0, 2.0, 2.0, 1.5, 2.0, 15.0, 50.0, 1.3); return m[t]; }
// The turned laying is this many times as large; the two blend over noise `DETAIL_BLEND` times the photograph's size.
const DETAIL_AGAIN = 2.7; const DETAIL_BLEND = 3.0;
// A photograph's own hue shows this much (the rest of its change is its lightness alone): the world gives the colour.
const DETAIL_HUE = 0.55;

// The change photograph t gives the ground's colour at the pixel, at e m a pixel: 1 where it averages to its mean.
fn detail(t: i32, e: f32) -> vec3f {
  let s = detail_m(t);
  let a = photo(t, lat_axes(s, vec2f(0.0)).f, vec2f(e / s, 0.0), vec2f(0.0, e / s));
  let k = DETAIL_AGAIN * s;
  let r = mat2x2f(cos(TURN_A), sin(TURN_A), -sin(TURN_A), cos(TURN_A));
  let b = photo(t, lat_turned(k, vec2f(0.37)).f, r * vec2f(e / k, 0.0), r * vec2f(0.0, e / k));
  let w = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(DETAIL_BLEND * s, vec2f(0.61)), 95 + t).x);
  let mean = photo_mean(t);
  let ratio = mix(a, b, w) / max(mean, vec3f(1e-3));
  return mix(vec3f(dot(ratio, LUMA)), ratio, DETAIL_HUE);
}

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
  var sum = vec3f(0.0);
  var total = 0.0;
  for (var i = 0; i < 6; i++) {
    if (w[i] < 0.02) { continue; }
    sum += w[i] * detail(t[i], p.e);
    total += w[i];
  }
  var out = select(vec3f(1.0), sum / max(total, 1e-4), total > 0.0);
  if (p.stony > 0.02) { out = mix(out, detail(PEBBLE_TEX, p.e), min(p.stony, 1.0)); }
  return out;
}

// The change the photograph of mossy rock gives bare bed, at e m a pixel.
fn rock_detail(e: f32) -> vec3f { return detail(ROCK_TEX, e); }

// The change the photograph of grass gives the sward, at e m a pixel.
fn grass_detail(e: f32) -> vec3f { return detail(GRASS_TEX, e); }

// The sward's clumps standing up (angled), over `c`, the sward's colour, at e m a pixel: one in `CLUMP_FILL` of the cells
// of a lattice `CLUMP_M` m wide, at a random place in its cell, `CLUMP_WIDE` cells wide and as tall as the plants stand
// (`CLUMP_STAND`); a grass clump, dry where the sward is straw (`straw`, 0..1) and a fern frond where it is shaded and
// wet (`fern`), each of its own draw. A clump keeps its photograph's lightness and some of its hue (`DETAIL_HUE`)
// about its mean (`CLUMP_LIGHT`), and takes the sward's colour; the nearer clumps over the farther; between them the
// sward's ground in their shade (`CLUMP_GAP`).
const CLUMP_M = 0.25; const CLUMP_FILL = 0.8; const CLUMP_WIDE = 1.9; const CLUMP_STAND = 0.75; const CLUMP_GAP = 0.6;
const CLUMP_LIGHT = 0.06;
fn grass_clumps(c: vec3f, e: f32, straw: f32, fern: f32) -> vec3f { return clumps(c, e, straw, fern, CLUMP_FILL, CLUMP_GAP); }

// The clumps of `grass_clumps`, one in `fill` of the cells, the ground between them `gap` of its colour.
fn clumps(c: vec3f, e: f32, straw: f32, fern: f32, fill: f32, gap: f32) -> vec3f {
  let l = lat_axes(CLUMP_M, vec2f(0.0));
  let g = e / (CLUMP_WIDE * CLUMP_M);
  var out = c * gap;
  // From the farthest (north) to the nearest, so the nearer covers the farther.
  for (var j = 0; j <= 2; j++) {
    for (var i = -1; i <= 1; i++) {
      let cell = l.i + vec2i(i, j);
      let x = lowbias(cell, 93);
      if (f32(x & 1023u) / 1024.0 >= fill) { continue; }
      let foot = vec2f(f32((x >> 10u) & 1023u), f32((x >> 20u) & 1023u)) / 1024.0;
      // From the clump's foot to the pixel, in its widths.
      let q = (l.f - vec2f(f32(i), f32(j)) - foot) / CLUMP_WIDE;
      let uv = vec2f(q.x + 0.5, 1.0 + q.y / CLUMP_STAND);
      if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { continue; }
      let pick = f32((x >> 3u) & 127u) / 128.0;
      var s = GRASS + i32(pick * f32(GRASSES));
      if (fract(pick * 7.3) < straw) { s += DRY_GRASS; }
      if (fract(pick * 3.7) < fern) { s = FERNS + i32(fract(pick * 5.1) * f32(FERN_KINDS)); }
      let t = sprite(s, uv, g / CLUMP_STAND);
      if (t.a <= 0.0) { continue; }
      let light = dot(t.rgb, LUMA) / CLUMP_LIGHT;
      let hue = t.rgb / max(dot(t.rgb, LUMA), 1e-4);
      let col = c * light * mix(vec3f(1.0), hue * dot(c, LUMA) / max(c, vec3f(1e-4)), DETAIL_HUE);
      out = mix(out, col, t.a);
    }
  }
  return out;
}

// Wood lying on the ground: fallen trunks under crowns and, fewer, bleached on dry open ground, one in `LOG_FILL` of the
// cells of a lattice `LOG_CELL_M` m wide where it may lie (as the canopy closes, or as the ground is dry and bare),
// `LOG_LONG` m long and `LOG_R` m thick by its own draws, at its own angle; a trunk is a lying cylinder of the bark's
// photograph, lit across it from the sun, moss on its upper side where the floor is wet (`LOG_MOSS`), its shadow beside
// it away from the sun; and branches, one in `TWIG_FILL` of the cells of a lattice `TWIG_CELL_M` m wide, each a crooked
// stick of two pieces `TWIG_R` m thick with a side twig, dark on the floor.
const LOG_CELL_M = 9.0; const LOG_FILL = 0.55; const LOG_LONG = vec2f(2.5, 7.0); const LOG_R = vec2f(0.1, 0.32);
const LOG_MOSS = 0.8; const LOG_SHADOW = 0.5; const LOG_DRY = 0.35;
const TWIG_CELL_M = 1.4; const TWIG_FILL = 0.6; const TWIG_R = vec2f(0.008, 0.025); const TWIG_LONG = vec2f(0.4, 1.1);
const TWIG_DARK = 0.45;

// The distance from point v to the segment from a to b, and how far along it (0..1).
fn segment(v: vec2f, a: vec2f, b: vec2f) -> vec2f {
  let ab = b - a;
  let k = clamp(dot(v - a, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0);
  return vec2f(length(v - a - ab * k), k);
}

// The wood lying on ground of colour g at spot p, where the plants leave `bare` of it bare: g with its trunks and
// branches over it.
fn wood_on(g: vec3f, p: Spot, bare: f32) -> vec3f {
  let canopy = min(p.canopy, 1.0);
  let dry = ease(0.4, 0.1, p.wet) * (1.0 - canopy) * bare;
  let lies = max(canopy, LOG_DRY * dry) * (1.0 - p.sand) * (1.0 - p.snow);
  if (lies <= 0.01) { return g; }
  var out = g;
  let sun = normalize(LIGHT.xy);
  if (!unseen(2.0 * LOG_R.x)) {
    let l = lat_axes(LOG_CELL_M, vec2f(0.0));
    for (var j = -1; j <= 1; j++) {
      for (var i = -1; i <= 1; i++) {
        let c = l.i + vec2i(i, j);
        let x = lowbias(c, 120);
        if (f32(x & 1023u) / 1024.0 >= LOG_FILL * lies) { continue; }
        let mid = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0;
        let turn = f32(x >> 26u) / 64.0 * PI;
        let y = lowbias(c, 121);
        let half = 0.5 * mix(LOG_LONG.x, LOG_LONG.y, f32(y & 255u) / 255.0) / LOG_CELL_M;
        let r = mix(LOG_R.x, LOG_R.y, f32((y >> 8u) & 255u) / 255.0);
        let dir = vec2f(cos(turn), sin(turn));
        let v = l.f - mid;
        // Its shadow, then the trunk over it.
        let sh = segment(v + sun * (0.8 * r / LOG_CELL_M), -dir * half, dir * half);
        out *= 1.0 - LOG_SHADOW * (1.0 - smoothstep(0.7 * r, 1.4 * r, sh.x * LOG_CELL_M));
        let s = segment(v, -dir * half, dir * half);
        let d = s.x * LOG_CELL_M;
        if (d >= r) { continue; }
        let side = sign(dot(v - dir * dot(v, dir), vec2f(-dir.y, dir.x)));
        let across = side * d / r;
        let nrm = normalize(vec3f(vec2f(-dir.y, dir.x) * across, sqrt(max(1.0 - across * across, 0.0))));
        let lit = 1.0 + 0.6 * (dot(nrm, LIGHT) / LIGHT.z - 1.0);
        let along = s.y * 2.0 * half * LOG_CELL_M;
        var bark = photo(BARK_TEX, vec2f(across * 0.5 + f32(y >> 16u) / 65536.0, along / (6.0 * r)), vec2f(p.e / r, 0.0), vec2f(0.0, p.e / r));
        // Moss on its upper (sunward) side on a wet floor; its cut ends paler.
        let moss = LOG_MOSS * p.wet * canopy * smoothstep(0.0, 0.6, -dot(nrm.xy, sun));
        bark = mix(bark, photo(MOSSY_TEX, vec2f(along, across) / 0.8, vec2f(p.e, 0.0), vec2f(0.0, p.e)) * 1.2, moss);
        let edge = 1.0 - smoothstep(r - p.e, r, d);
        out = mix(out, bark * lit, edge);
      }
    }
  }
  if (!unseen(4.0 * TWIG_R.y)) {
    let l = lat_turned(TWIG_CELL_M, vec2f(0.0));
    for (var j = -1; j <= 1; j++) {
      for (var i = -1; i <= 1; i++) {
        let c = l.i + vec2i(i, j);
        let x = lowbias(c, 122);
        if (f32(x & 1023u) / 1024.0 >= TWIG_FILL * lies) { continue; }
        let a = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0;
        let t1 = f32(x >> 26u) / 64.0 * 2.0 * PI;
        let y = lowbias(c, 123);
        let len = mix(TWIG_LONG.x, TWIG_LONG.y, f32(y & 255u) / 255.0) / TWIG_CELL_M;
        let r = mix(TWIG_R.x, TWIG_R.y, f32((y >> 8u) & 255u) / 255.0) / TWIG_CELL_M;
        let t2 = t1 + 0.8 * (f32((y >> 16u) & 255u) / 255.0 - 0.5);
        let b = a + 0.55 * len * vec2f(cos(t1), sin(t1));
        let e2 = b + 0.45 * len * vec2f(cos(t2), sin(t2));
        let side = b + 0.25 * len * vec2f(cos(t1 + 0.9), sin(t1 + 0.9));
        let v = l.f;
        let d = min(min(segment(v, a, b).x, segment(v, b, e2).x / 0.8), segment(v, b, side).x / 0.6);
        let cover = 1.0 - smoothstep(r, r + p.e / TWIG_CELL_M, d);
        out *= 1.0 - TWIG_DARK * cover;
      }
    }
  }
  return out;
}

// Snow's own texture over its colour (1 on the mean): wind drifts `DRIFT_M` m long and a quarter of that across, lying
// along the wind (`WIND_TURN`), lit by their slope (the sun's side warm white, the lee side in the sky's blue,
// `SNOW_LEE`), sastrugi `SASTRUGI_M` m long carved along them, the crust's grain (`SNOW_CRUST`), and a glint here and there where a
// crystal turns to the sun (`GLINT`, one in `GLINT_RARE` of the cells `GLINT_M` across).
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
  return out;
}

// Bare ground's filler: where it is cold or wet, mats of lichen and moss in patches `MAT_M` m across over up to
// `MATS` of it, grey-green and ochre (`LICHEN`, `MOSS_MAT`), in the mossy floor's photograph; and, angled, where it is
// dry, dry grass tufts standing here and there (`FILLER_TUFTS` of the clumps' cells), as many as it is dry.
const FILLER_TUFTS = 0.08; const MAT_M = 0.7; const MATS = 0.3;
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
  let dry = ease(0.45, 0.1, p.wet) * open;
  let seen = shows_at(TUFT_SEEN_M, p.e);
  if (dry <= 0.02 || seen <= 0.0 || cover_look.look.z < 0.5) { return g; }
  let straw = straw_rgb() * dot(g, LUMA) / max(dot(straw_rgb(), LUMA), 1e-4);
  return mix(g, clumps(straw, p.e, 1.0, 0.0, FILLER_TUFTS * dry, 1.0), seen);
}

// The colour of standing straw against a ground's lightness (canopy.ts `STRAW_AREA`'s straw), linear.
fn straw_rgb() -> vec3f { return vec3f(0.62, 0.52, 0.30); }

// Boulders on bare ground (rendered models, tex.wgsl `BOULDER_FROM`), angled, standing on their foot with their shadow:
// one in `BOULDER_FILL` of the cells of a lattice `BOULDER_CELL_M` m wide, as many more as the ground is bare, rocky,
// steep or under thin snow (a snowcap on top where it lies), `BOULDER_M` m across by their own draws.
const BOULDER_CELL_M = 7.0; const BOULDER_FILL = 0.35; const BOULDER_M = vec2f(0.5, 2.4); const BOULDER_SHADOW = 0.45;
fn boulders(g: vec3f, p: Spot, bare: f32) -> vec3f {
  let stony = bare * (1.0 - p.sand) * (1.0 - min(p.canopy, 1.0)) * max(max(p.rock, ease(0.15, 0.6, p.slope)), 0.35 + 0.65 * ease(1.0, 0.1, p.loose));
  if (stony <= 0.02 || unseen(0.3 * BOULDER_M.x) || cover_look.look.z < 0.5 || !models_ready()) { return g; }
  let l = lat_axes(BOULDER_CELL_M, vec2f(0.0));
  var out = g;
  let sun = normalize(LIGHT.xy);
  // From the farthest (north) to the nearest; a boulder reaches up into the cell north of its foot.
  for (var j = 0; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let c = l.i + vec2i(i, j);
      let x = lowbias(c, 150);
      if (f32(x & 1023u) / 1024.0 >= BOULDER_FILL * stony) { continue; }
      let foot = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0 * 0.8 + 0.1;
      let s = BOULDER_FROM + i32((x >> 26u) % u32(BOULDERS));
      let w = mix(BOULDER_M.x, BOULDER_M.y, pow(f32(lowbias(c, 151) & 255u) / 255.0, 2.0)) / BOULDER_CELL_M;
      let h = w * model_h(s) / model_w(s);
      let v = l.f - foot;
      // Its shadow on the ground away from the sun.
      let sv = (v + sun * 0.35 * w) / vec2f(0.6 * w, 0.3 * w);
      out *= 1.0 - BOULDER_SHADOW * (1.0 - smoothstep(0.6, 1.0, length(sv)));
      let uv = vec2f(v.x / w + 0.5, 1.0 + v.y / h);
      if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { continue; }
      let t = model_texel(s, uv, p.e / (w * BOULDER_CELL_M));
      var rock = t.rgb;
      // Snow on its top where the ground holds some.
      rock = mix(rock, SNOW, p.snow * smoothstep(0.55, 0.25, uv.y));
      out = mix(out, rock, t.a);
    }
  }
  return out;
}
