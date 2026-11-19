// Things lying on the ground up close (web/src/detail.ts): rocks, fallen wood and dead bushes, a channel's boulders and
// driftwood, rendered models (tex.wgsl, sprites.wgsl) standing on their foot over the ground's photographs
// (detail.wgsl); and the photographs of a channel's bed (`bed_photos`).

// Things lying on the ground (rendered models, tex.wgsl), standing on their foot with their shadow away from the
// sun, each sort at its own lattice, one in `fill` of its cells, a size between its two by its own draw (more small than
// large), as many as the ground suits them: rocks where it is bare, rocky, steep or loose (a snowcap on top where snow
// lies), large ones in cells of `ROCK_CELL_M` m and small in cells of `PEBBLE_CELL_M` m; fallen trunks and branches
// under crowns and, fewer, on dry open ground (`WOOD_DRY`); dead bushes on dry bare ground; driftwood on a channel's
// bars; its boulders where it moves large stones (`bed_rocks`); cobbles on a shore of pebbles (`COBBLE_CELL_M`).
const ROCK_CELL_M = 7.0; const ROCK_FILL = 0.35; const ROCK_M = vec2f(0.5, 2.4);
const PEBBLE_CELL_M = 1.6; const PEBBLE_FILL = 0.3; const PEBBLE_M = vec2f(0.12, 0.45);
// Snow shows at least `SNOW_ROCKS` as many rocks through it as the barest ground holds.
const SNOW_ROCKS = 0.6;
// Sand holds `1 - SAND_BARE` as many rocks and dead bushes as other bare ground.
const SAND_BARE = 0.6;
const WOOD_CELL_M = 6.0; const WOOD_FILL = 0.2; const WOOD_M = vec2f(1.5, 5.0); const WOOD_DRY = 0.35;
// Driftwood stranded on a channel's bars (river.wgsl `DRIFTWOOD_UP`): logs and branches in cells of `DRIFTWOOD_CELL_M` m, one in
// `DRIFTWOOD_FILL` of them, `DRIFTWOOD_M` m long.
const DRIFTWOOD_CELL_M = 6.0; const DRIFTWOOD_FILL = 0.5; const DRIFTWOOD_M = vec2f(1.2, 4.5);
const DEAD_BUSH_CELL_M = 4.0; const DEAD_BUSH_FILL = 0.3; const DEAD_BUSH_M = vec2f(0.5, 1.3);
// Cobbles worn round lie over a shore of pebbles (colour.wgsl `shore_grain`), larger than its pebbles: in cells of
// `COBBLE_CELL_M` m, one in `COBBLE_FILL` of them, `COBBLE_SIZE_M` m across, as wet as the shore is soaked (the waves and the
// swash leave the coarsest stones on top of a beach of pebbles: from memory).
const COBBLE_CELL_M = 1.5; const COBBLE_FILL = 0.35; const COBBLE_SIZE_M = vec2f(0.12, 0.35);
// A thing's shadow: a seam `CONTACT` of its width wide and deep (`CONTACT_UP` of it north of its foot, under it)
// `CONTACT_DARK` dark where it sits in the ground, and the shade it casts, `OBJECT_SHADOW` dark, `CAST` of its width
// across, its middle at `CAST_AT` of its width from its foot (east and a little north: mostly under it). Snow lies on its
// lit upper faces, `SNOW_CAP` of their colour.
const CONTACT = vec2f(0.5, 0.1); const CONTACT_UP = 0.04; const CONTACT_DARK = 0.55;
const OBJECT_SHADOW = 0.4; const CAST = vec2f(0.6, 0.22); const CAST_AT = vec2f(0.22, -0.06); const SNOW_CAP = 0.85;
// Each sort's sprites (sprites.wgsl): rocks, wood (a fallen trunk and branches), dead bushes, worn rocks under moss,
// driftwood, stones worn round.
const ROCK_SORT = 0; const WOOD_SORT = 1; const DEAD_BUSH_SORT = 2; const MOSSY_ROCK_SORT = 3; const DRIFTWOOD_SORT = 4;
const COBBLE_SORT = 5;
fn sort_sprite(sort: i32, k: u32) -> i32 {
  if (sort == ROCK_SORT) { return ROCK_FROM + i32(k % u32(ROCK_N)); }
  if (sort == COBBLE_SORT) { return COBBLE_FROM + i32(k % u32(COBBLE_N)); }
  if (sort == MOSSY_ROCK_SORT) { return SHORE_ROCK_FROM + i32(k % u32(SHORE_ROCK_N)); }
  if (sort == DRIFTWOOD_SORT) { return DRIFTWOOD_FROM + i32(k % u32(DRIFTWOOD_N)); }
  if (sort == WOOD_SORT) { return WOOD_FROM + i32(k % u32(WOOD_N)); }
  return DEAD_BUSH_FROM + i32(k % u32(DEAD_BUSH_N));
}

// One sort of things strewn over the ground: its sort, its lattice's cell, m, the share `fill` of the cells that may
// hold one, its size, m (the smallest and the largest), `many` (0..1) as the ground suits them, a rock `wet` wet
// (stone.wgsl `wet_stone`), and its stream of the world's random numbers.
struct Strewn { sort: i32, cell_m: f32, fill: f32, size: vec2f, many: f32, wet: f32, k: i32 }

// Ground of colour g at spot p with the things `t` over it. Each shows as a thing (draw.wgsl `as_thing`): its colour
// and its shadow depart from the ground's by its share.
fn scatter(g: vec3f, p: Spot, t: Strewn) -> vec3f {
  let sort = t.sort; let cell_m = t.cell_m; let fill = t.fill; let size = t.size; let many = t.many; let wet = t.wet; let k = t.k;
  if (many <= 0.02 || unseen(size.y / THING_PX.x) || as_thing(size.y, p.e) <= 0.0) { return g; }
  let l = lat_axes(cell_m, vec2f(0.0));
  var out = g;
  // From the farthest (north) to the nearest; a thing reaches up into the cell north of its foot, its shadow into the
  // cell south of it.
  for (var j = -1; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let c = l.i + vec2i(i, j);
      let x = lowbias(c, k);
      if (f32(x & 1023u) / 1024.0 >= fill * many) { continue; }
      let foot = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0 * 0.8 + 0.1;
      let s = sort_sprite(sort, x >> 26u);
      let w = mix(size.x, size.y, pow(f32(lowbias(c, k + 1) & 255u) / 255.0, 2.0)) / cell_m;
      let h = w * model_h(s) / model_w(s);
      let seen = as_thing(w * cell_m, p.e);
      if (seen <= 0.0) { continue; }
      let v = l.f - foot;
      // Its shadow: a dark seam where it sits in the ground, and the shade it casts away from the sun from under it,
      // most of it hidden by the thing itself.
      let contact = length((v - vec2f(0.0, -CONTACT_UP * w)) / vec2f(CONTACT.x * w, CONTACT.y * w));
      let shade = length((v - CAST_AT * w) / (CAST * w));
      out *= 1.0 - seen * (1.0 - (1.0 - CONTACT_DARK * (1.0 - smoothstep(0.5, 1.0, contact))) * (1.0 - OBJECT_SHADOW * (1.0 - smoothstep(0.4, 1.0, shade))));
      let uv = vec2f(v.x / w + 0.5, 1.0 + v.y / h);
      if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { continue; }
      let t = model_texel(s, uv, p.e / (w * cell_m));
      // Snow on its faces that look up, its upper part's lit texels, keeping their light.
      let lit = dot(t.rgb, LUMA) / max(dot(model_texel(s, uv, 1.0).rgb, LUMA), 1e-4);
      let cap = p.snow * f32(sort != DEAD_BUSH_SORT) * smoothstep(0.7, 0.35, uv.y) * smoothstep(0.8, 1.3, lit);
      let rgb = mix(wet_stone(t.rgb, lit, wet, 0.0), SNOW * clamp(lit, 0.75, 1.1), SNOW_CAP * cap);
      out = mix(out, rgb, t.a * seen);
    }
  }
  return out;
}

// The things lying on ground of colour g at spot p, where the plants leave `bare` of it bare.
fn objects(g: vec3f, p: Spot, bare: f32) -> vec3f {
  if (!models_ready()) { return g; }
  let canopy = min(p.canopy, 1.0);
  let open = bare * (1.0 - SAND_BARE * p.sand) * (1.0 - canopy);
  let stony = max(open * max(max(p.rock, ease(0.15, 0.6, p.slope)), 0.35 + 0.65 * ease(1.0, 0.1, p.loose)), SNOW_ROCKS * p.snow * (1.0 - canopy));
  let dry = ease(0.4, 0.1, p.wet) * open * (1.0 - p.snow);
  let wood = max(canopy, WOOD_DRY * dry) * (1.0 - p.sand) * (1.0 - p.snow);
  let free = bare * (1.0 - p.snow);
  // One loop over the sorts, so the program holds the code that draws a thing once.
  let grain = shore_grain(p);
  var sorts = array<Strewn, 7>(
    Strewn(WOOD_SORT, WOOD_CELL_M, WOOD_FILL, WOOD_M, wood, 0.0, 120),
    Strewn(DEAD_BUSH_SORT, DEAD_BUSH_CELL_M, DEAD_BUSH_FILL, DEAD_BUSH_M, dry, 0.0, 124),
    Strewn(ROCK_SORT, PEBBLE_CELL_M, PEBBLE_FILL, PEBBLE_M, stony, 0.0, 152),
    Strewn(ROCK_SORT, ROCK_CELL_M, ROCK_FILL, ROCK_M, stony, 0.0, 150),
    Strewn(DRIFTWOOD_SORT, DRIFTWOOD_CELL_M, DRIFTWOOD_FILL, DRIFTWOOD_M, p.driftwood * free, 0.0, 160),
    bed_rocks(p, free, soaked(p)),
    Strewn(COBBLE_SORT, COBBLE_CELL_M, COBBLE_FILL, COBBLE_SIZE_M, max(p.sand, 0.0) * grain.x * free, grain.z, 162));
  var out = g;
  for (var i = 0; i < 7; i++) { out = scatter(out, p, sorts[i]); }
  return out;
}

// The bed's loose stones as photographs, laid where its stones lie (soil's images, detail.wgsl `soil_detail`), in
// patches of their own as many as it holds: by the largest stone its channel moves there (`Spot.moved`), gravel under
// `BED_COARSE.x` m and cobbles from `BED_COARSE.y` m, the two meeting in patches between; the gravel angular
// where the channel has worn its stones less than `BED_ROUND` and round past it (a stream rounds its stones within a
// few kilometres of travel: Krumbein 1941, from memory).
const BED_COARSE = vec2f(0.06, 0.25); const BED_ROUND = 0.5;
// The noise streams of the stones' patches and of the two sizes' patches.
const BED_PATCHES = 99; const BED_SIZES = 197;

// The change the bed's stones of spot p give the soil's colour change `under`.
fn bed_photos(under: vec3f, p: Spot) -> vec3f {
  if (p.stony <= 0.02) { return under; }
  let m = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(SOIL_IMAGE_M, vec2f(0.7)), BED_PATCHES).x);
  let near = (min(p.stony, 1.0) - m) / SOIL_EDGE;
  if (near <= -CLAIMS_WITHIN) { return under; }
  let coarse = ease(BED_COARSE.x, BED_COARSE.y, p.moved);
  var c = detail(select(GRAVEL_TEX, ROUND_PEBBLE_TEX, p.worn > BED_ROUND), p.e);
  if (coarse > 0.0) {
    let u = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(SOIL_IMAGE_M, vec2f(0.13)), BED_SIZES).x);
    let d = (coarse - u) / SOIL_EDGE;
    if (d >= CLAIMS_WITHIN) { c = detail(PEBBLE_TEX, p.e); }
    else if (d > -CLAIMS_WITHIN) { let b = detail(PEBBLE_TEX, p.e); c = mix(c, b, height_mix(d, height_of(c), height_of(b))); }
  }
  return mix(under, c, height_mix(near, height_of(under), height_of(c)));
}

// A channel's boulders as rocks (rendered models) where the largest stone it moves (`Spot.moved`) passes
// `BED_ROCK_STONE.x` m, all from `.y`: in cells `BED_ROCK_CELL` of that size across, one in `BED_ROCK_FILL` of them,
// each `BED_ROCK_SIZE` of it: worn rocks with moss on them (moss grows on rock a stream keeps wet), or where the
// channel has worn its stones round (`BED_ROUND`, as its gravel is drawn), cobbles.
const BED_ROCK_STONE = vec2f(0.15, 0.4); const BED_ROCK_CELL = 6.0; const BED_ROCK_FILL = 0.6;
const BED_ROCK_SIZE = vec2f(1.0, 2.5);

// The boulders of spot p's channel, as far as `open` of its ground is open to them, `wet` wet.
fn bed_rocks(p: Spot, open: f32, wet: f32) -> Strewn {
  let many = open * min(p.stony, 1.0) * ease(BED_ROCK_STONE.x, BED_ROCK_STONE.y, p.moved);
  return Strewn(select(MOSSY_ROCK_SORT, COBBLE_SORT, p.worn > BED_ROUND), BED_ROCK_CELL * p.moved, BED_ROCK_FILL, BED_ROCK_SIZE * p.moved, many, wet, 156);
}
