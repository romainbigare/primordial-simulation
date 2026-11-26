// The live map at a point (web/src/land.ts): the ground from its tiles (web/src/ground.ts, tape/src/ground), which never
// change, and the state from the cells' table (tape/src/table.rs), blended between the squares' centres; then the laws
// inside a cell applied at the point, with the numbers the core sends: the ground's temperature along its height, the
// liquid that stands under its cells' level, the soil's liquid by the wetness index, a channel as wide as its own
// sample's discharge makes it (the samples' discharge, sent when it changed enough to show).
// Nothing depends on the view, a window or a tile: a place has the same colour after any pan or zoom.

struct Land {
  // Metres a square, squares across and down, the depth of a square.
  map: vec4f,
  // Texels along a tile's side inside its gutter, texels of the gutter, the depth the view is drawn at, and the most
  // steps along each axis by which a texel names its channel's sample (tape/src/ground/channels.rs `code`).
  tiles: vec4f,
  // The time since the tiles started, ms, the time a new tile fades in over, ms, the melting point of the frozen
  // cover, K (0: none), and the discharge per metre a css pixel takes past which a channel shows as a line, m³/s per m.
  time: vec4f,
  // The soil's liquid departs by this per unit of the wetness index, m; ground with this depth of liquid is half wet, m;
  // a living thing that holds the share x of the liquid it needs is x to this power active; a point whose loose ground
  // is under this share of the depth a kind's roots need holds none of it (`Physics::rooted_from`).
  soil: vec4f,
  // A channel's width and depth: each a factor, m, times its discharge in m³/s to a power.
  channel: vec4f,
  // The terrain's samples across and down, their depth, and the pixels drawn a css pixel.
  samples: vec4f,
  // The length a texel's place along its channel is kept modulo, m (tape/src/ground/channels.rs `ALONG_WRAP_M`), how
  // many times the mean flow the flood that shapes a channel's bed passes (tape/src/ground/bends.rs `FLOOD`), how
  // far the signs of the flow have slid down the slowest water, m (web/src/land.ts `slid`), and the share of its bed's
  // width a channel's water takes at its mean flow (`FLOOD` to the power minus `STATION_EXP`).
  reach: vec4f,
  // The line a channel is drawn as from afar (tape/src/ground/channels.rs `LINE_PX`): its css pixels where its
  // discharge is the one that shows, the power of its discharge over that it widens by, and the most times.
  line: vec4f,
}
@group(0) @binding(8) var<uniform> land: Land;
// The tiles kept, a layer each: of each texel the ground's height, m, as the sum of two 16-bit floats (to a millimetre),
// the wetness index, and the distance to its channel's line, m, signed by the side; and that channel's sample, as its
// step from the sample that holds the texel's centre (0: none), one more than the pond it lies in (0: none), of the
// line's nearest point how far along the channel it lies, m (modulo `land.reach.x`), and the signed distance to the
// nearest channel's line without its bends, m.
@group(0) @binding(9) var atlas: texture_2d_array<f32>;
@group(0) @binding(13) var areas: texture_2d_array<f32>;
// The page of the tiles (web/src/ground.ts `makePage`): in its first row its window at the view's depth (first column
// and row, tiles across and down) and the coarsest depth with its tiles across and down; then, of each tile of the
// window, row by row, and under them of each of the coarsest tiles: the layer drawn there and the layer it fades in
// over (-1: none).
@group(0) @binding(10) var pages: texture_2d<f32>;
// Of each layer: its tile's depth, column and row, and when it came, ms; in the second row, 1 where its tile is made of
// the samples edits have moved since (`moved_at`).
@group(0) @binding(11) var tile_of: texture_2d<f32>;
// The cells' table: `PLANES` planes of a texel a square, one under the other.
@group(0) @binding(12) var table: texture_2d<f32>;
// What runs through each of the terrain's samples, m³/s, and what its liquid carries, g/m³ (tape/src/table.rs
// `discharge` and `load`, with the plumes' load in the bodies, web/src/land.ts), and how far edits moved its ground
// since the samples the page's ground is made of, m (wasm/src/lib.rs `preview`), a texel a sample.
@group(0) @binding(17) var flows: texture_2d<f32>;

// The table's planes: the terrain's mean height over the cell, m, its ground's temperature, K, how fast its air cools
// with height, K/m, and its ice, m; its standing liquid's level, its lowest ground and the depth that stands, m, and the
// liquid in its pores, m; what its pores hold full, m, the level of the wetness index its soil's liquid departs
// about, the share of its area that is land and the wetness index from which its land is saturated (the core's
// `inside::wet_ground`); the ground's own colour and its share of the sea; the land's ice, m; the byte of the layer
// drawn and the share of organic matter of its loose ground, and in the last numbers of the layer's plane the land's
// ground temperature, K, and its mean height, m; and its cell's shares of coarse and fine grain over those of its samples as
// drawn (rock.wgsl `soil_at`). (The bed's colour comes with the ground: rock.wgsl `bed_make_at`.)
const GROUND = 0; const LIQUID = 1; const SOIL = 2; const SURFACE = 3; const LAND_ICE = 4; const VIEW = 5; const GRAIN_SHIFT = 6;
const PLANES = 7;
// Where no liquid stands near a point, its surface is taken this far below the ground, m.
const NO_LIQUID_M = 1000.0;
// Between texels of one channel its signed distance changes by at most the distance between them (√2 texels across a
// diagonal; a little more for the 16-bit floats): four texels whose signed distances differ by more and change sign
// hold two channels, or the two sides of a line's end, and are blended by their distances alone.
const SAME_LINE_TEXELS = 1.5;
// Half a texel's diagonal, in texels.
const HALF_DIAGONAL = 0.71;
// A texel no channel reaches holds a distance to a line past this, m (tape/src/ground/channels.rs `NO_CHANNEL_M`).
const NO_LINE_M = 3.0e4;
// Between texels of one stretch of a channel's line the place along it changes by a few texels at most (more on the
// inside of a bend, where the line's points lie closer than the texels): texels farther apart along it than this many
// hold another stretch, or another channel, and are not blended.
const SAME_STRETCH_TEXELS = 6.0;
// A channel's bed is a parabola across it, whose mean depth is two thirds of its depth at the middle.
const BED_MIDDLE = 1.5;

// What a point's ground holds: its height, m, its slope east and south, its wetness index, and the distance to its
// channel's line, m, its slope east and south; the samples of the channels of the texel nearest it and of the one
// across from that (-1: none) and the least distances of those texels to their lines, m (from their nearest
// points); and the highest level of the ponds of the four texels about it, m (`-NO_LIQUID_M`: none), the share of
// the point's weight on those of them in a pond, and the fetch of the highest, m (`pond_fetch`): its liquid stands at
// that level over the ground drawn, so a shore runs smooth between the texels where the ground crosses the level; the distance to the channel's line signed by
// the side the point lies on, m; of the line's nearest point, how far along the channel it lies, m (modulo
// `land.reach.x`, blended across the texels of its stretch); of the channel's line without its bends (its valley's
// line with its wander, tape/src/ground/bends.rs) and how
// far the line has swung from the line without bends in its bend there, m, signed as the side its outer bank lies on
// (`lean`; none where the texels do not hold both lines).
struct Tiled { h: f32, slope: vec2f, twi: f32, d: f32, d_slope: vec2f, channels: vec4i, ends: vec2f, water: vec3f, sd: f32, along: f32, lean: f32 }

// A channel's bends are drawn fully where their wavelength takes at least `BEND_PX.y` pixels of the screen, and not at
// all under `BEND_PX.x`: a line drawn from afar is several pixels wide, and a bend whose turns are tighter than it is
// wide reads as the teeth of a saw (a bend turns on about a fifth of its length). The tiles hold them whole where
// their texels show them (tape/src/ground/lines.rs `BEND_HELD`, under these at the depth a view is drawn at), so the
// bends drawn are the same from every depth of the tiles: a zoom that changes the depth moves none.
const BEND_PX = vec2f(24.0, 72.0);
// The metres a pixel of the screen takes in the draw, which the bends are drawn by (`bends_shown`): each program that
// reads the ground sets it first (0: the bends the tiles hold, whole).
var<private> bends_e: f32 = 0.0;
// The share of the bends of wavelength `length` m drawn (`BEND_PX`).
fn bends_shown(length: f32) -> f32 {
  if (bends_e <= 0.0) { return 1.0; }
  return ease(BEND_PX.x * bends_e, BEND_PX.y * bends_e, length);
}

// The sample texel `n` of the tile of depth `depth` at column and row `tile` names by `code` (tape/src/ground/channels.rs
// `code`): the sample that holds the texel's centre, stepped along each axis; none (-1) for 0.
fn sample_of(n: vec2i, depth: f32, tile: vec2f, code: i32) -> vec2i {
  if (code <= 0) { return vec2i(-1); }
  let most = i32(land.tiles.w);
  let side = 2 * most + 1;
  let texel = vec2i(tile) * i32(land.tiles.x) - i32(land.tiles.y) + n;
  let own = vec2i(floor((vec2f(texel) + 0.5) * exp2(land.samples.z - depth)));
  return own + vec2i((code - 1) % side, (code - 1) / side) - most;
}

// The ground of layer `layer` at `at`, m from the map's north-west corner, blended between the texels' centres; with
// `spline`, its height and slope through them as a Catmull-Rom spline (so light on a slope drawn over many pixels a
// texel shows no creases).
fn read_tile(layer: i32, at: vec2f, spline: bool) -> Tiled {
  let t = textureLoad(tile_of, vec2i(layer, 0), 0);
  let texel = land.map.x / exp2(t.x - land.map.w);
  let q = at / texel - (t.yz * land.tiles.x - land.tiles.y) - 0.5;
  let i = vec2i(floor(q));
  let f = q - floor(q);
  let last = i32(textureDimensions(atlas).x) - 1;
  let a = clamp(i, vec2i(0), vec2i(last));
  let b = clamp(i + 1, vec2i(0), vec2i(last));
  let c00 = textureLoad(atlas, a, layer, 0);
  let c10 = textureLoad(atlas, vec2i(b.x, a.y), layer, 0);
  let c01 = textureLoad(atlas, vec2i(a.x, b.y), layer, 0);
  let c11 = textureLoad(atlas, b, layer, 0);
  let top = mix(c00, c10, f.x);
  let bottom = mix(c01, c11, f.x);
  let o = mix(top, bottom, f.y);
  // The channel's signed distance blended across its line (exact for a straight one), unless the four texels hold more
  // than one channel; its slope, and the distance itself.
  let raw = vec4f(c00.w, c10.w, c01.w, c11.w);
  var sd = raw;
  let lo = min(min(sd.x, sd.y), min(sd.z, sd.w));
  let hi = max(max(sd.x, sd.y), max(sd.z, sd.w));
  if (lo < 0.0 && hi > 0.0 && hi - lo > SAME_LINE_TEXELS * texel) { sd = abs(sd); }
  let sx = mix(sd.x, sd.y, f.x);
  let sy = mix(sd.z, sd.w, f.x);
  let full = mix(sx, sy, f.y);
  let full_slope = vec2f(mix(sd.y - sd.x, sd.w - sd.z, f.y), sy - sx) / texel;
  // The channels' samples of the texel nearest the point and of the one across the four from it: where a tributary's
  // line runs into the water of the channel it joins, its texels lie between that channel's (`discharge_at`).
  let a4 = array<vec4f, 4>(textureLoad(areas, a, layer, 0), textureLoad(areas, vec2i(b.x, a.y), layer, 0), textureLoad(areas, vec2i(a.x, b.y), layer, 0), textureLoad(areas, b, layer, 0));
  let at4 = array<vec2i, 4>(a, vec2i(b.x, a.y), vec2i(a.x, b.y), b);
  let corner = select(0, 1, f.x > 0.5) + select(0, 2, f.y > 0.5);
  let channels = vec4i(sample_of(at4[corner], t.x, t.yz, i32(a4[corner].x + 0.5)), sample_of(at4[3 - corner], t.x, t.yz, i32(a4[3 - corner].x + 0.5)));
  let ends = max(abs(vec2f(raw[corner], raw[3 - corner])) - HALF_DIAGONAL * texel, vec2f(0.0));
  // The ponds the four texels lie in: their highest level, the share of the point's weight on them, and the highest's
  // fetch.
  let ponds = vec4i(i32(a4[0].y + 0.5), i32(a4[1].y + 0.5), i32(a4[2].y + 0.5), i32(a4[3].y + 0.5)) - 1;
  var levels = vec4f(-NO_LIQUID_M);
  if (all(ponds == vec4i(ponds.x))) { levels = vec4f(pond_level(ponds.x)); }
  else { levels = vec4f(pond_level(ponds.x), pond_level(ponds.y), pond_level(ponds.z), pond_level(ponds.w)); }
  let bilinear = vec4f((1.0 - f.x) * (1.0 - f.y), f.x * (1.0 - f.y), (1.0 - f.x) * f.y, f.x * f.y);
  let wet = select(vec4f(0.0), vec4f(1.0), ponds >= vec4i(0));
  let high = max(max(levels.x, levels.y), max(levels.z, levels.w));
  var highest = ponds.x;
  for (var n = 1; n < 4; n++) { if (levels[n] > levels[0] && levels[n] >= high) { highest = ponds[n]; } }
  let water = vec3f(high, dot(bilinear, wet), pond_fetch(highest));
  // The place along the channel: the nearest texel's, blended with those of the others that hold the same stretch of
  // the line (their places taken the short way round the length they are kept modulo).
  // (In a program made for draws from afar, where no bed's shape shows, the nearest texel's as it is, and no swing.)
  let own = a4[corner].z;
  var along = own;
  let shaped = !unseen(BED_VARY * BED_MOST_M);
  if (shaped) {
    var line = vec2f(0.0);
    for (var n = 0; n < 4; n++) {
      let step = a4[n].z - own - land.reach.x * round((a4[n].z - own) / land.reach.x);
      if (abs(step) <= SAME_STRETCH_TEXELS * texel) { line += bilinear[n] * vec2f(step, 1.0); }
    }
    along = own + line.x / line.y;
  }
  // The line without its bends, blended as the line's own; the line stands off it by the difference of the two
  // signed distances (exact where they run side by side): its swing in its bend.
  var cd = vec4f(a4[0].w, a4[1].w, a4[2].w, a4[3].w);
  let c_lo = min(min(cd.x, cd.y), min(cd.z, cd.w));
  let c_hi = max(max(cd.x, cd.y), max(cd.z, cd.w));
  if (c_lo < 0.0 && c_hi > 0.0 && c_hi - c_lo > SAME_LINE_TEXELS * texel) { cd = abs(cd); }
  let cx = mix(cd.x, cd.y, f.x);
  let cy = mix(cd.z, cd.w, f.x);
  let calm = mix(cx, cy, f.y);
  // The line as far as its bends show at the draw's pixels (`bends_shown`): between the line the tiles hold, with its
  // bends whole, and the line without them.
  let both = c_hi < NO_LINE_M && hi < NO_LINE_M;
  let bent = select(1.0, bends_shown(bend_length_of(channels.xy)), both);
  let signed = mix(calm, full, bent);
  let d_slope = sign(signed) * mix(vec2f(mix(cd.y - cd.x, cd.w - cd.z, f.y), cy - cx) / texel, full_slope, bent);
  let lean = select(0.0, calm - signed, shaped && both);
  if (!spline) { return Tiled(o.x + o.y, vec2f(0.0), o.z, abs(signed), d_slope, channels, ends, water, signed, along, lean); }
  let f2 = f * f;
  let f3 = f2 * f;
  // The weights of the texels from i - 1 to i + 2 along each axis, and their slopes.
  var w = array<vec2f, 4>(0.5 * (-f3 + 2.0 * f2 - f), 0.5 * (3.0 * f3 - 5.0 * f2 + 2.0), 0.5 * (-3.0 * f3 + 4.0 * f2 + f), 0.5 * (f3 - f2));
  var dw = array<vec2f, 4>(0.5 * (-3.0 * f2 + 4.0 * f - 1.0), 0.5 * (9.0 * f2 - 10.0 * f), 0.5 * (-9.0 * f2 + 8.0 * f + 1.0), 0.5 * (3.0 * f2 - 2.0 * f));
  var h = vec3f(0.0);
  for (var y = 0; y < 4; y++) {
    for (var x = 0; x < 4; x++) {
      let t = textureLoad(atlas, clamp(i + vec2i(x - 1, y - 1), vec2i(0), vec2i(last)), layer, 0);
      let v = t.x + t.y;
      h += v * vec3f(w[x].x * w[y].y, dw[x].x * w[y].y, w[x].x * dw[y].y);
    }
  }
  return Tiled(h.x, h.yz / texel, o.z, abs(signed), d_slope, channels, ends, water, signed, along, lean);
}

// The level of pond `p` (-1: none), m, and its fetch, m: from the rows of the table under its planes, two ponds a
// texel (its level, its fetch); the level far below any ground for none (no body holds it, or the texel lies in no
// pond).
fn pond_texel(p: i32) -> vec2f {
  let w = i32(land.map.y);
  let q = p / 2;
  let t = textureLoad(table, vec2i(q % w, PLANES * i32(land.map.z) + q / w), 0);
  return select(t.xy, t.zw, p % 2 == 1);
}
fn pond_level(p: i32) -> f32 { return select(pond_texel(p).x, -NO_LIQUID_M, p < 0); }
fn pond_fetch(p: i32) -> f32 { return select(pond_texel(p).y, 0.0, p < 0); }

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

// Cell value `k` at a point `s` squares from the map's corner, through the squares' centres as a B-spline (draw.wgsl
// `b_spline`): smooth across their edges, where `blended` creases.
fn cells_smooth(k: i32, s: vec2f) -> vec4f {
  let c = b_spline(s - 0.5);
  var sum = vec4f(0.0);
  for (var y = 0; y < 3; y++) {
    for (var x = 0; x < 3; x++) { sum += c.wx[x] * c.wy[y] * cell(k, c.i + vec2i(x - 1, y - 1)); }
  }
  return sum;
}

// The layers the page names at `at`, m from the map's north-west corner: of its tile at the view's depth inside its
// window, else of its coarsest tile.
fn page_at(at: vec2f) -> vec4f {
  let window = textureLoad(pages, vec2i(0, 0), 0);
  let coarse = textureLoad(pages, vec2i(1, 0), 0);
  let c = vec2i(floor(at / (land.map.x * land.tiles.x / exp2(land.tiles.z - land.map.w)))) - vec2i(window.xy);
  if (all(c >= vec2i(0)) && all(c < vec2i(window.zw))) { return textureLoad(pages, c + vec2i(0, 1), 0); }
  let k = clamp(vec2i(floor(at / (land.map.x * land.tiles.x / exp2(coarse.x - land.map.w)))), vec2i(0), vec2i(coarse.yz) - 1);
  return textureLoad(pages, k + vec2i(0, 1 + i32(window.w)), 0);
}

// The ground at `at`, m from the map's north-west corner (`read_tile`, with `spline`): from the tile the page names
// there, fading in over the one before it; before any tile came, the cells' mean heights, flat.
fn ground_at(at: vec2f, spline: bool) -> Tiled {
  let page = page_at(at);
  if (page.x < 0.0) {
    return Tiled(blended(GROUND, corners(at / land.map.x)).x, vec2f(0.0), 0.0, NO_LIQUID_M, vec2f(0.0), vec4i(-1), vec2f(NO_LIQUID_M), vec3f(-NO_LIQUID_M, 0.0, 0.0), NO_LIQUID_M, 0.0, 0.0);
  }
  let shown = with_moved(read_tile(i32(page.x), at, spline), i32(page.x), at);
  let came = textureLoad(tile_of, vec2i(i32(page.x), 0), 0).w;
  let w = clamp((land.time.x - came) / land.time.y, 0.0, 1.0);
  if (w >= 1.0 || page.y < 0.0) { return shown; }
  let under = with_moved(read_tile(i32(page.y), at, spline), i32(page.y), at);
  return Tiled(mix(under.h, shown.h, w), mix(under.slope, shown.slope, w), mix(under.twi, shown.twi, w), mix(under.d, shown.d, w), mix(under.d_slope, shown.d_slope, w), shown.channels, shown.ends, mix(under.water, shown.water, w), mix(under.sd, shown.sd, w), shown.along, shown.lean);
}

// How far edits moved the ground at `at` since the samples the page's ground is made of, m, through the samples'
// centres as a B-spline (draw.wgsl `b_spline`: no sample's square shows in the shading), and its slope east and south.
fn moved_at(at: vec2f) -> vec3f {
  let side = land.map.x * land.map.y / land.samples.x;
  let c = b_spline(at / side - 0.5);
  let t = at / side - 0.5 - vec2f(c.i);
  let dx = vec3f(t.x - 0.5, -2.0 * t.x, t.x + 0.5);
  let dy = vec3f(t.y - 0.5, -2.0 * t.y, t.y + 0.5);
  let last = vec2i(land.samples.xy) - 1;
  var m = vec3f(0.0);
  for (var y = 0; y < 3; y++) {
    for (var x = 0; x < 3; x++) {
      let v = textureLoad(flows, clamp(c.i + vec2i(x - 1, y - 1), vec2i(0), last), 0).z;
      m += v * vec3f(c.wx[x] * c.wy[y], dx[x] * c.wy[y], c.wx[x] * dy[y]);
    }
  }
  return vec3f(m.x, m.yz / side);
}

// The ground `t` of layer `layer` at `at`, with what edits moved since, where its tile is made of the samples they
// moved (a tile made since holds it).
fn with_moved(t: Tiled, layer: i32, at: vec2f) -> Tiled {
  if (textureLoad(tile_of, vec2i(layer, 1), 0).x <= 0.0) { return t; }
  var g = t;
  let m = moved_at(at);
  g.h += m.x;
  g.slope += m.yz;
  return g;
}

// The liquid in the soil at a point of wetness index `twi` of a cell whose pores hold `pore` m and `pores` m full, its
// index's level `c` (inside/shares.rs `soil_held`): departed from the mean by `soil_m` per unit of the index over the
// level, within none and full; even where the pores hold none or are full.
fn soil_held(pore: f32, pores: f32, c: f32, twi: f32) -> f32 {
  if (pore <= 0.0 || pore >= pores) { return pore; }
  return clamp(pore + land.soil.x * (twi - c), 0.0, pores);
}

// How saturated the ground at `at` is, 0..1 (TOPMODEL, Beven and Kirkby 1979): all of it in the water of a flowing
// channel, as wide as the channel law makes it on its discharge this month (the core's `inside::channel_share`: the
// flowing liquid is its table at the surface; rock.wgsl `discharge_at`, the drawn channel's own), about its line as
// drawn; elsewhere where its wetness index (`Tiled.twi`) reaches the index of saturation of its cells (the core's
// `inside::wet_ground`), blended over the cells about it that hold land (a cell of liquid alone has none), so the wet
// ground's edge is a line of the index, along the valleys, not a ring about a wet cell; the edge is spread over
// `SATURATED_EDGE` of the index under it. The blend is the B-spline through the cells' centres (draw.wgsl `b_spline`),
// each cell's weight kept where it holds land and the sum divided by the weights kept: where all hold land, the
// B-spline of the levels. Its value and its change are smooth across the centres, as the edge is narrow in the index
// and would draw any crease of the level as a straight line along the cells' rows and columns. (The page draws that
// water about any channel line its tiles hold, the law only about those that drain a channel's source,
// `inside::drains_source`: below it the law's channels are under a metre wide.)
const SATURATED_EDGE = 0.5;
fn saturated_at(at: vec2f) -> f32 {
  return saturated_of(ground_at(at, false), at);
}
// The same of the ground `g` at `at` (`ground_at`), where it is read already.
fn saturated_of(g: Tiled, at: vec2f) -> f32 {
  let q = discharge_at(g, at);
  if (q > 0.0 && g.d <= 0.5 * land.channel.x * pow(q, land.channel.y)) { return 1.0; }
  let c = b_spline(at / land.map.x - 0.5);
  var level = 0.0;
  var weight = 0.0;
  for (var y = 0; y < 3; y++) {
    for (var x = 0; x < 3; x++) {
      let s = cell(SOIL, c.i + vec2i(x - 1, y - 1));
      let w = select(0.0, c.wx[x] * c.wy[y], s.z > 0.0);
      level += w * s.w;
      weight += w;
    }
  }
  if (weight <= 0.0) { return 0.0; }
  return smoothstep(-SATURATED_EDGE, 0.0, g.twi - level / weight);
}

// The share of a tent of half-width 1 that lies below t (a pixel's footprint across a line).
fn tent(x: f32) -> f32 {
  let t = clamp(x, -1.0, 1.0);
  return select(1.0 - (1.0 - t) * (1.0 - t) / 2.0, (1.0 + t) * (1.0 + t) / 2.0, t <= 0.0);
}

// The share of a kind whose roots need `need` m of loose ground that holds where `loose` m lies (the cover's own rule,
// `cover::stand::rooted`, with its number `land.soil.z`): none from that share of its need down, all from its need up.
// Bare bed holds no liquid in pores and nothing to anchor a tall plant; the need of each kind is its slot's
// (`Cover::need`, tape/src/stands.rs). Where the loose depth changes by `spread` m across a pixel and that is more than
// the rule's own span, the edge is spread over it, as the share of water at a shore is (no speckle where the loose
// ground has fine patches).
fn rooted(loose: f32, need: f32, spread: f32) -> f32 {
  let none = land.soil.z;
  let middle = 0.5 * (1.0 + none) * need;
  let half = 0.5 * max((1.0 - none) * need, spread);
  return ease(middle - half, middle + half, loose);
}

// The state of the ground at a point: the depth of the liquid standing over it and its height over that liquid's
// surface, m; how wet it is, 0..1; its temperature, K; the snow and ice on it, 0..1, as its cells' share left unmelted
// there (`snow_at` places it), and the snow's depth, m; how readily it sheds its snow (`snow_bares`; `snow_at` reads
// it); and the share of its pores the soil's liquid fills there, 0..1.
struct State { depth: f32, over: f32, wet: f32, t: f32, snow: f32, snow_m: f32, bares: f32, filled: f32 }

// Inside a cell its snow lies first where the ground holds it (`snow_bares`): high ground before low (colder: the air's
// fall with height; over `SNOW_RISE_M` a unit), slopes turned from the sun before those facing it (they melt first;
// `SNOW_SUNNY` a unit of slope toward the light), hollows before knobs (drift fills them; `SNOW_HOLLOW_M` of the
// relief's height a unit). The cell's share `s` covers where this index lies under `SNOW_SPREAD` ln(s / (1 - s)): of
// an index spread about 1 either way (a logistic one), the share s, so as much snow is drawn as the cell holds.
// The index is held softly within `SNOW_BARES_MOST` either way, so a cell's last few hundredths of snow lie nowhere,
// and no snow ends on the straight line where the blended share reaches none (a line through the squares' centres).
const SNOW_RISE_M = 40.0; const SNOW_SUNNY = 2.0; const SNOW_HOLLOW_M = 0.25; const SNOW_SPREAD = 1.0;
const SNOW_BARES_MOST = 3.0;

// How readily the ground of slope `slope` (east, south), `rise` m over its cells' mean and `knob` m over its relief's
// mean (relief.wgsl), sheds its snow: the lower, the longer it keeps it.
fn snow_bares(slope: vec2f, rise: f32, knob: f32) -> f32 {
  let sunward = clamp(dot(-slope, LIGHT.xy) / length(LIGHT.xy), -1.0, 1.0);
  return SNOW_BARES_MOST * tanh((SNOW_SUNNY * sunward + knob / SNOW_HOLLOW_M - rise / SNOW_RISE_M) / SNOW_BARES_MOST);
}

// A snow line is ragged (ragged.wgsl `meet`): at its margin the snow breaks into drifts in hollows behind knobs, bare
// knobs first, at every scale from `SNOW_PATCH_M` down to the pixel, `SNOW_PATCHY` of the index either way (noise
// stream `SNOW_NOISE`), and lies in its own patches, thinning to none over `SNOW_THIN` of that swing at a patch's
// edge (a drift tapers, the ground showing through the thin snow). The melt moves the cells' share there, so the line
// where the snow melts with height is as ragged.
const SNOW_PATCH_M = 320.0; const SNOW_PATCHY = 2.5; const SNOW_NOISE = 80; const SNOW_THIN = 0.25;
// How far the index of the line the cell's share draws is held from its middle either way: past what the patches and
// the ground's index (`SNOW_BARES_MOST`) reach, so a share near none or all lies as none or all, and the change of the
// line across a pixel stays finite.
const SNOW_LINE_MOST = SNOW_BARES_MOST + 2.0 * SNOW_PATCHY;

// How far a point whose cells hold `share` of their ground under snow and whose ground sheds it as `bares`
// (`snow_bares`) lies past the line its snow ends at, in the index's units (into the snow).
fn snow_past(share: f32, bares: f32) -> f32 {
  let s = clamp(share, 1e-6, 1.0 - 1e-6);
  return clamp(SNOW_SPREAD * log(s / (1.0 - s)), -SNOW_LINE_MOST, SNOW_LINE_MOST) - bares;
}

// The snow at `at`, seen at `e` m a pixel, whose cells hold `share` of their ground under snow, `past` the line its snow
// ends at (`snow_past`), that changing by `px` across a pixel: 1 where it lies, 0 where not, the pixel's share at its
// edge.
fn snow_at(at: vec2f, e: f32, share: f32, past: f32, px: f32) -> f32 {
  if (share <= 0.0 || share >= 1.0) { return share; }
  return meet(at, e, past, SNOW_PATCHY, SNOW_THIN, px, SNOW_PATCH_M, SNOW_NOISE);
}

// The height of ground `g`, cut `cut` m deep, over the level of the ponds about it, m (`NO_LIQUID_M`: none about it):
// it counts the more the less of the point is theirs.
fn over_ponds(g: Tiled, cut: f32) -> f32 {
  if (g.water.x <= -NO_LIQUID_M || g.water.y <= 0.0) { return NO_LIQUID_M; }
  return min(max(g.h - cut - g.water.x, 0.0) / g.water.y, NO_LIQUID_M);
}

// How far through the band of shallows round standing liquid a point of ground `g` lies, seen at `e` m a pixel: 0 where
// none stands over the ground drawn, 1 from the open water past the band on (style.wgsl `shore_rim`).
fn through_band(g: Tiled, e: f32) -> f32 {
  if (g.water.y <= 0.0) { return 0.0; }
  return shore_rim(standing(g), length(g.slope), e, LAKE_RIM_M);
}
// The depth of the liquid of the ponds about a point that stands over its ground `g` as drawn, m.
fn standing(g: Tiled) -> f32 { return max(g.water.x - g.h, 0.0) * g.water.y; }

// The state at `at`, m from the map's north-west corner, on ground `g` whose bed a channel cuts
// `cut` m deep there (`depth_in`) and whose relief stands `knob` m over its mean (relief.wgsl): the cells' values
// blended between the squares' centres (`cs`), and the laws inside a cell applied at the point.
fn state_on(at: vec2f, g: Tiled, cs: Corners, cut: f32, knob: f32) -> State {
  // The cells' ground, and the share of it each one's frozen cover covers, blended: the shares, not the depths, so a
  // snowy cell beside a bare one draws its share falling across the space between their centres, not at once along a
  // line where the blended depth reaches what covers all (tanh saturates within a few centimetres).
  // Over land, the land's mean height, temperature and ice (the table's `LAND_ICE` and `VIEW` planes carry them, the open low ground's
  // coastal squares taking the land's nearest: web/src/land.ts `fromLand`); over standing liquid, the cells'.
  let dry = g.water.y <= 0.0 || g.h - cut > g.water.x;
  var ground = vec4f(0.0);
  var covered = 0.0;
  for (var k = 0; k < 4; k++) {
    var c = cell(GROUND, cs.at[k]);
    // A sea's square (its air's fall marked as -(it + 1), web/src/land.ts) gives a dry point the land's numbers.
    if (c.z < 0.0) {
      c.z = -c.z - 1.0;
      if (dry) { let v = cell(VIEW, cs.at[k]); c = vec4f(v.w, v.z, c.z, cell(LAND_ICE, cs.at[k]).x); }
    }
    ground += cs.w[k] * c;
    covered += cs.w[k] * snow_of(c.w, 0.0, 0.0);
  }
  // The liquid standing over the point: its ponds', flat at their levels over the ground under it (laws.md 1.2).
  // The soil's liquid of each cell about it at the point's wetness index, blended through the cells' centres as a
  // B-spline (draw.wgsl `b_spline`): its value and its change are smooth there, as the wet soil's colours switch over a
  // narrow edge of it and would draw any crease of the blend as a straight line along the cells' rows and columns.
  var held = 0.0;
  var pores = 0.0;
  let bs = b_spline(at / land.map.x - 0.5);
  for (var y = 0; y < 3; y++) {
    for (var x = 0; x < 3; x++) {
      let ci = bs.i + vec2i(x - 1, y - 1);
      let w = bs.wx[x] * bs.wy[y];
      let l = cell(LIQUID, ci);
      let s = cell(SOIL, ci);
      held += w * soil_held(l.w, s.x, s.y, g.twi);
      pores += w * s.x;
    }
  }
  var p: State;
  if (g.water.x > -NO_LIQUID_M && g.water.y > 0.0) {
    // Over the ponds' level, the height counts the more the less of the point is theirs, so a shore's fringe fades out
    // within a texel of the pond's last texel; under it, the depth shallows to none across that texel, so where the
    // ground drawn lies under the level past the pond's last texel the shore is a soft edge, not the texels' steps.
    p.depth = max(g.water.x - (g.h - cut), 0.0) * g.water.y;
  } else {
    p.depth = 0.0;
  }
  p.over = over_ponds(g, cut);
  let half = land.soil.y;
  p.wet = (held + p.depth) / (held + p.depth + half);
  p.filled = select(0.0, held / pores, pores > 0.0);
  // The ground's temperature is its cell's at the cell's mean height, less the air's fall with height above it.
  p.t = ground.y - ground.z * (g.h - ground.x);
  p.snow = covered * unmelted(p.t, land.time.z);
  p.snow_m = snow_depth(ground.w, p.t, land.time.z);
  p.bares = snow_bares(g.slope, g.h - ground.x, knob);
  return p;
}

// The state at `at` on ground `g` as a plant's foot reads it (cover.wgsl), on the tiles' ground (no
// relief finer than them): the water of the point's channel stands over its bed too, and in the whole of the bed its
// floods cut (`channel_at`, widened at its mouth) the liquid is taken as deeper than any plant's roots reach: the core
// stands plants on its cells' land and on their ponds' ground, never in a channel's running water.
fn state_at(at: vec2f, g: Tiled, cs: Corners) -> State {
  let c = channel_at(g, discharge_at(g, at));
  let cut = depth_in(g, c);
  var p = state_on(at, g, cs, cut, 0.0);
  p.depth = max(max(p.depth, cut), select(0.0, NO_LIQUID_M, g.d < c.bed));
  return p;
}
