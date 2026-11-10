// The live map at a point (web/src/land.ts): the ground from its tiles (web/src/ground.ts, tape/src/ground), which never
// change, and the state from the cells' table (tape/src/table.rs), blended between the squares' centres; then the laws
// inside a cell applied at the point, with the numbers the core sends: the ground's temperature along its height, the
// liquid that stands under its cells' level, the soil's liquid by the wetness index, a channel as wide as its discharge.
// Nothing depends on the view, a window or a tile: a place has the same colour after any pan or zoom.

struct Land {
  // Metres a square, squares across and down, the depth of a square.
  map: vec4f,
  // Texels along a tile's side inside its gutter, texels of the gutter, the depth the view is drawn at.
  tiles: vec4f,
  // The time since the tiles started, ms, and the time a new tile fades in over, ms.
  time: vec4f,
  // The soil's liquid departs by this per unit of the wetness index, m; ground with this depth of liquid is half wet, m.
  soil: vec4f,
  // A channel's width and depth: each a factor, m, times its discharge in m³/s to a power.
  channel: vec4f,
}
@group(0) @binding(8) var<uniform> land: Land;
// The tiles kept, a layer each: of each texel the ground's height, m, the wetness index, and of its nearest channel
// the distance to its centre line over its drained area to the power of its width, and that area, m².
@group(0) @binding(9) var atlas: texture_2d_array<f32>;
// Of each tile at the view's depth: the layer drawn there and the layer it fades in over (-1: none).
@group(0) @binding(10) var pages: texture_2d<f32>;
// Of each layer: its tile's depth, column and row, and when it came, ms.
@group(0) @binding(11) var tile_of: texture_2d<f32>;
// The cells' table: `PLANES` planes of a texel a square, one under the other.
@group(0) @binding(12) var table: texture_2d<f32>;

// The table's planes: the terrain's mean height over the cell, m, its ground's temperature, K, how fast its air cools
// with height, K/m, and its ice, m; its standing liquid's level, its lowest ground and the depth that stands, m, and the
// liquid in its pores, m; what its pores hold full, m, the level of the wetness index its soil's liquid departs about,
// the volume it passes on per m² drained, m/s, and how wet its heaviest cover needs its ground; the byte of its cover's
// mass, the share of its ground its cover leaves bare and its share of the sea; its true colour; the bytes of the layer
// drawn and of the worn paths.
const GROUND = 0; const LIQUID = 1; const SOIL = 2; const COVER = 3; const TRUTH = 4; const VIEW = 5;
// Where no liquid stands near a point, its surface is taken this far below the ground, m.
const NO_LIQUID_M = 1000.0;

// What a point's ground holds: its height, m, its slope east and south, its wetness index, and its nearest channel's
// distance over its drained area to the power of its width and that area, m².
struct Tiled { h: f32, slope: vec2f, twi: f32, ratio: f32, area: f32 }

// The ground of layer `layer` at `at`, m from the map's north-west corner: its height and slope through the texels'
// centres as a Catmull-Rom spline (so light on a slope drawn over many pixels a texel shows no creases), the rest blended
// between them.
fn read_tile(layer: i32, at: vec2f) -> Tiled {
  let t = textureLoad(tile_of, vec2i(layer, 0), 0);
  let texel = land.map.x / exp2(t.x - land.map.w);
  let q = at / texel - (t.yz * land.tiles.x - land.tiles.y) - 0.5;
  let i = vec2i(floor(q));
  let f = q - floor(q);
  let f2 = f * f;
  let f3 = f2 * f;
  let last = i32(textureDimensions(atlas).x) - 1;
  // The weights of the texels from i - 1 to i + 2 along each axis, and their slopes.
  var w = array<vec2f, 4>(0.5 * (-f3 + 2.0 * f2 - f), 0.5 * (3.0 * f3 - 5.0 * f2 + 2.0), 0.5 * (-3.0 * f3 + 4.0 * f2 + f), 0.5 * (f3 - f2));
  var dw = array<vec2f, 4>(0.5 * (-3.0 * f2 + 4.0 * f - 1.0), 0.5 * (9.0 * f2 - 10.0 * f), 0.5 * (-9.0 * f2 + 8.0 * f + 1.0), 0.5 * (3.0 * f2 - 2.0 * f));
  var h = vec3f(0.0);
  for (var y = 0; y < 4; y++) {
    for (var x = 0; x < 4; x++) {
      let v = textureLoad(atlas, clamp(i + vec2i(x - 1, y - 1), vec2i(0), vec2i(last)), layer, 0).x;
      h += v * vec3f(w[x].x * w[y].y, dw[x].x * w[y].y, w[x].x * dw[y].y);
    }
  }
  let a = clamp(i, vec2i(0), vec2i(last));
  let b = clamp(i + 1, vec2i(0), vec2i(last));
  let top = mix(textureLoad(atlas, a, layer, 0), textureLoad(atlas, vec2i(b.x, a.y), layer, 0), f.x);
  let bottom = mix(textureLoad(atlas, vec2i(a.x, b.y), layer, 0), textureLoad(atlas, b, layer, 0), f.x);
  let o = mix(top, bottom, f.y);
  return Tiled(h.x, h.yz / texel, o.y, o.z, o.w);
}

// Cell value `k` (a plane of the table) of the square at column and row c, the nearest on the map.
fn cell(k: i32, c: vec2i) -> vec4f {
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let q = clamp(c, vec2i(0), n - 1);
  return textureLoad(table, vec2i(q.x, k * n.y + q.y), 0);
}

// The four squares about a point `s` squares from the map's corner, between whose centres it is blended, and its weight
// in each.
struct Corners { at: array<vec2i, 4>, w: vec4f }
fn corners(s: vec2f) -> Corners {
  let p = s - 0.5;
  let i = vec2i(floor(p));
  let f = p - floor(p);
  return Corners(array<vec2i, 4>(i, i + vec2i(1, 0), i + vec2i(0, 1), i + vec2i(1, 1)), vec4f((1.0 - f.x) * (1.0 - f.y), f.x * (1.0 - f.y), (1.0 - f.x) * f.y, f.x * f.y));
}
fn blended(k: i32, c: Corners) -> vec4f {
  return c.w.x * cell(k, c.at[0]) + c.w.y * cell(k, c.at[1]) + c.w.z * cell(k, c.at[2]) + c.w.w * cell(k, c.at[3]);
}

// The ground at `at`, m from the map's north-west corner: from the tile the page names there, fading in over the one
// before it; before any tile came, the cells' mean heights, flat.
fn ground_at(at: vec2f) -> Tiled {
  let span = land.map.x * land.tiles.x / exp2(land.tiles.z - land.map.w);
  let size = vec2i(textureDimensions(pages));
  let page = textureLoad(pages, clamp(vec2i(floor(at / span)), vec2i(0), size - 1), 0);
  if (page.x < 0.0) {
    return Tiled(blended(GROUND, corners(at / land.map.x)).x, vec2f(0.0), 0.0, 3.0e38, 0.0);
  }
  let shown = read_tile(i32(page.x), at);
  let came = textureLoad(tile_of, vec2i(i32(page.x), 0), 0).w;
  let w = clamp((land.time.x - came) / land.time.y, 0.0, 1.0);
  if (w >= 1.0 || page.y < 0.0) { return shown; }
  let under = read_tile(i32(page.y), at);
  return Tiled(mix(under.h, shown.h, w), mix(under.slope, shown.slope, w), mix(under.twi, shown.twi, w), mix(under.ratio, shown.ratio, w), mix(under.area, shown.area, w));
}

// The liquid in the soil at a point of wetness index `twi` of a cell whose pores hold `pore` m and `pores` m full, its
// index's level `c` (inside/shares.rs `soil_held`): departed from the mean by `soil_m` per unit of the index over the
// level, within none and full; even where the pores hold none or are full.
fn soil_held(pore: f32, pores: f32, c: f32, twi: f32) -> f32 {
  if (pore <= 0.0 || pore >= pores) { return pore; }
  return clamp(pore + land.soil.x * (twi - c), 0.0, pores);
}

// The share of a tent of half-width 1 that lies below t (a pixel's footprint across a line).
fn tent(x: f32) -> f32 {
  let t = clamp(x, -1.0, 1.0);
  return select(1.0 - (1.0 - t) * (1.0 - t) / 2.0, (1.0 + t) * (1.0 + t) / 2.0, t <= 0.0);
}

// What is at `at`, m from the map's north-west corner, on ground `g`, seen at `e` m a pixel: the cells' values blended
// between the squares' centres, and the laws inside a cell applied at the point.
fn land_spot(at: vec2f, g: Tiled, e: f32) -> Spot {
  let look = earth.look;
  let cs = corners(at / land.map.x);
  let ground = blended(GROUND, cs);
  let soil = blended(SOIL, cs);
  let cover = blended(COVER, cs);
  // The level of the liquid standing over the point: of the cells about it whose lowest ground lies under the surface
  // of its own cell (its level, or its lowest ground where none stands), blended; liquid over ground that lies wholly
  // above a cell's surface runs down into it and does not stand over it. The soil's liquid of each cell about it at
  // the point's wetness index, blended.
  let own = cell(LIQUID, vec2i(floor(at / land.map.x)));
  let surface = select(own.y, own.x, own.z > 0.0);
  var level = 0.0;
  var weight = 0.0;
  var held = 0.0;
  for (var k = 0; k < 4; k++) {
    let l = cell(LIQUID, cs.at[k]);
    if (l.z > 0.0 && l.y < surface) { level += cs.w[k] * l.x; weight += cs.w[k]; }
    let s = cell(SOIL, cs.at[k]);
    held += cs.w[k] * soil_held(l.w, s.x, s.y, g.twi);
  }
  var p: Spot;
  if (weight > 0.0) {
    level /= weight;
    p.depth = max(level - g.h, 0.0);
    p.over = max(g.h - level, 0.0);
  } else {
    p.depth = 0.0;
    p.over = NO_LIQUID_M;
  }
  p.sea = cover.z;
  p.wet = (held + p.depth) / (held + p.depth + land.soil.y);
  p.bare = cover.y;
  // The ground's temperature is its cell's at the cell's mean height, less the air's fall with height above it.
  p.t = ground.y - ground.z * (g.h - ground.x);
  p.snow = snow_of(ground.w, p.t);
  p.plants = 255.0 * cover.x;
  p.z = g.h;
  p.slope = length(g.slope) * look.x;
  p.shade = hill_shade(g.slope, EXAGGERATE * look.x);
  p.sand = SAND_SHARE * p.sea * (1.0 - ease(0.0, look.z, p.over));
  // A channel where the point lies nearer its centre line than half its width: its discharge is its drained area times
  // the volume the cells pass on per m² drained (flow/channel.rs), its share of a pixel what the pixel's footprint covers.
  p.river = 0.0;
  p.river_depth = 0.0;
  if (soil.z > 0.0 && g.area > 0.0) {
    let k = pow(g.area, land.channel.y);
    let half = 0.5 * land.channel.x * pow(soil.z, land.channel.y) * k;
    let d = g.ratio * k;
    p.river = tent((half - d) / e) - tent((-half - d) / e);
    p.river_depth = land.channel.z * pow(soil.z * g.area, land.channel.w);
  }
  p.at = at;
  p.e = e;
  return p;
}
