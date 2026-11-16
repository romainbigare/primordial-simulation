// The ground's detail up close (web/src/detail.ts): photographs of ground seen from above, each a layer of
// `detail_tex` with its mips down to its mean, laid on the map at its real size (`detail_m`) and read as a change around
// the colour the world gives the ground: the photograph over its mean. From afar the mips average the photograph to its
// mean, so the ground's colour there is the world's alone; near, the same colour shows its leaves, stones and cracks.
// Each photograph is laid twice, along the world's axes and turned at `DETAIL_AGAIN` times its size, blended by noise,
// so no repeat shows.

// Each photograph's side in the world, m (web/textures/SOURCES.md).
fn detail_m(t: i32) -> f32 { var m = array<f32, 9>(1.26, 3.0, 2.0, 4.0, 1.5, 2.0, 15.0, 50.0, 1.3); return m[t]; }
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
fn grass_clumps(c: vec3f, e: f32, straw: f32, fern: f32) -> vec3f {
  let l = lat_axes(CLUMP_M, vec2f(0.0));
  let g = e / (CLUMP_WIDE * CLUMP_M);
  var out = c * CLUMP_GAP;
  // From the farthest (north) to the nearest, so the nearer covers the farther.
  for (var j = 0; j <= 2; j++) {
    for (var i = -1; i <= 1; i++) {
      let cell = l.i + vec2i(i, j);
      let x = lowbias(cell, 93);
      if (f32(x & 1023u) / 1024.0 >= CLUMP_FILL) { continue; }
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
