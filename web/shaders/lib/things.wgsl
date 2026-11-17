// Things lying on the ground up close, angled (web/src/detail.ts): rocks, fallen wood and dead bushes, rendered models
// (tex.wgsl, sprites.wgsl) standing on their foot over the ground's photographs (detail.wgsl).

// Things lying on the ground (rendered models, tex.wgsl), angled, standing on their foot with their shadow away from the
// sun, each sort at its own lattice, one in `fill` of its cells, a size between its two by its own draw (more small than
// large), as many as the ground suits them: rocks where it is bare, rocky, steep or loose (a snowcap on top where snow
// lies), large ones in cells of `ROCK_CELL_M` m and small in cells of `PEBBLE_CELL_M` m; fallen trunks and branches
// under crowns and, fewer, on dry open ground (`WOOD_DRY`); dead bushes on dry bare ground.
const ROCK_CELL_M = 7.0; const ROCK_FILL = 0.35; const ROCK_M = vec2f(0.5, 2.4);
const PEBBLE_CELL_M = 1.6; const PEBBLE_FILL = 0.3; const PEBBLE_M = vec2f(0.12, 0.45);
// Snow shows at least `SNOW_ROCKS` as many rocks through it as the barest ground holds.
const SNOW_ROCKS = 0.6;
// Sand holds `1 - SAND_BARE` as many rocks and dead bushes as other bare ground.
const SAND_BARE = 0.6;
const WOOD_CELL_M = 6.0; const WOOD_FILL = 0.2; const WOOD_M = vec2f(1.5, 5.0); const WOOD_DRY = 0.35;
const DEAD_BUSH_CELL_M = 4.0; const DEAD_BUSH_FILL = 0.3; const DEAD_BUSH_M = vec2f(0.5, 1.3);
// A thing's shadow: a seam `CONTACT` of its width wide and deep (`CONTACT_UP` of it north of its foot, under it)
// `CONTACT_DARK` dark where it sits in the ground, and the shade it casts, `OBJECT_SHADOW` dark, `CAST` of its width
// across, its middle at `CAST_AT` of its width from its foot (east and a little north: mostly under it). Snow lies on its
// lit upper faces, `SNOW_CAP` of their colour.
const CONTACT = vec2f(0.5, 0.1); const CONTACT_UP = 0.04; const CONTACT_DARK = 0.55;
const OBJECT_SHADOW = 0.4; const CAST = vec2f(0.6, 0.22); const CAST_AT = vec2f(0.22, -0.06); const SNOW_CAP = 0.85;
// Each sort's sprites (sprites.wgsl): rocks, wood (a fallen trunk and branches), dead bushes.
const ROCK_SORT = 0; const WOOD_SORT = 1; const DEAD_BUSH_SORT = 2;
fn sort_sprite(sort: i32, k: u32) -> i32 {
  if (sort == ROCK_SORT) { return ROCK_FROM + i32(k % u32(ROCK_N)); }
  if (sort == WOOD_SORT) { return WOOD_FROM + i32(k % u32(WOOD_N)); }
  return DEAD_BUSH_FROM + i32(k % u32(DEAD_BUSH_N));
}

// Ground of colour g at spot p with one sort of things over it: `many` (0..1) as the ground suits them, at stream k.
// Each shows as a thing (draw.wgsl `as_thing`): its colour and its shadow depart from the ground's by its share.
fn scatter(g: vec3f, p: Spot, sort: i32, cell_m: f32, fill: f32, size: vec2f, many: f32, k: i32) -> vec3f {
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
      let rgb = mix(t.rgb, SNOW * clamp(lit, 0.75, 1.1), SNOW_CAP * cap);
      out = mix(out, rgb, t.a * seen);
    }
  }
  return out;
}

// The things lying on ground of colour g at spot p, where the plants leave `bare` of it bare.
fn objects(g: vec3f, p: Spot, bare: f32) -> vec3f {
  if (!angled() || !models_ready()) { return g; }
  let canopy = min(p.canopy, 1.0);
  let open = bare * (1.0 - SAND_BARE * p.sand) * (1.0 - canopy);
  let stony = max(open * max(max(p.rock, ease(0.15, 0.6, p.slope)), 0.35 + 0.65 * ease(1.0, 0.1, p.loose)), SNOW_ROCKS * p.snow * (1.0 - canopy));
  let dry = ease(0.4, 0.1, p.wet) * open * (1.0 - p.snow);
  let wood = max(canopy, WOOD_DRY * dry) * (1.0 - p.sand) * (1.0 - p.snow);
  var out = scatter(g, p, WOOD_SORT, WOOD_CELL_M, WOOD_FILL, WOOD_M, wood, 120);
  out = scatter(out, p, DEAD_BUSH_SORT, DEAD_BUSH_CELL_M, DEAD_BUSH_FILL, DEAD_BUSH_M, dry, 124);
  out = scatter(out, p, ROCK_SORT, PEBBLE_CELL_M, PEBBLE_FILL, PEBBLE_M, stony, 152);
  return scatter(out, p, ROCK_SORT, ROCK_CELL_M, ROCK_FILL, ROCK_M, stony, 150);
}
