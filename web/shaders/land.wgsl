// uses: draw.wgsl water.wgsl style.wgsl crag.wgsl ice.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl cover_field.wgsl
// least metres a pixel: 0.5 1 3 30
// The live map in view: at each pixel of the screen the ground from its tiles and the state from the cells' table
// (land.wgsl), in the view's colours. Made once more for draws whose pixels take at least 0.5 m, 1 m, 3 m and 30 m
// (web/shade, draw.wgsl `LEAST_E`): seen from afar, the detail of the ground and the plants that cannot show is
// compiled out (from 30 m, the whole island on a phone, the sea ice's floes and the pack's texture: ice.wgsl).

// A channel shows as a line of at least `LINE_PX` css pixels once its discharge passes the discharge that shows in the
// view (`land.time.w` times the metres a css pixel takes: tape/src/ground/channels.rs `SHOW_M`), as a map draws its
// rivers; wider as its discharge grows past that, as its discharge over that to the power `LINE_GROW` (more steeply
// than the channel law widens a channel: a map's choice, so the network's order reads), up to `LINE_MOST` times, so a
// stream is a hair and the main river many times wider; it fades in from `LINE_FADE` times less, so no channel springs
// up as the view zooms.
const LINE_PX = 1.2;
const LINE_MOST = 8.0;
const LINE_GROW = 0.75;
const LINE_FADE = 4.0;
// A channel's bed is as wide as its floods make it (lib/land.wgsl `banks_of`). This month's water runs in it between
// bars of washed gravel, its thread wandering
// across the bed's free width over `WANDER` bed widths along it, splitting about a bar where it wanders farthest (bars
// alternate every few widths: Leopold and Wolman 1957). Where the bed takes under `BED_PX` pixels it is all water.
const WANDER = 5.0;
const BED_PX = 3.0;
// Liquid standing shallower than this over the drawn ground is drawn as soaked ground, m: the drawn ground follows the
// samples within a few centimetres, so a body whose level lies that near its flat floor is a soaked basin, not water.
const FILM_M = 0.05;
// A channel's stones lie on its bed and its bars, out to this many of its half widths from its line.
const STONES_OUT = 2.0;
// The edge of standing liquid is found from how fast its depth changes across the ground, the ground's slope: no slower
// than this (rise over run), so on flat ground the edge still lies within a pixel's footprint where the depth crosses.
const SHORE_RISE_LEAST = 1e-3;
// The relief moves the ground's height fully on ground drawn this steep (rise over run), less on gentler.
const RELIEF_EDGE_SLOPE = 0.1;
// Cells nearly all under snow keep their true colour as it is: the ground under it is not found from so little.
const SNOW_CELLS_ALL = 0.95;

// What is at a point: the ground without the channel's water (`dry`), the channel's water (`wet`) and the share of the
// pixel that water covers; the share the bars of its bed cover (`bars`, the dry spot as washed gravel); and the share
// of the pixel standing liquid covers (`pool`): the pixel's footprint across the line where the depth crosses `FILM_M`,
// so a shore is a smooth line at any zoom, not the pixels' steps.
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32, snow_cells: f32 }

// A snow line is ragged: at its margin the snow breaks into patches (drifts in hollows behind knobs, bare knobs first)
// at every scale from `SNOW_PATCH_M` down to the pixel, octaves each `SNOW_PATCH_STEP` times finer weighing
// `SNOW_PATCH_FALL` of the last, `SNOW_PATCHY` of the index (lib/land.wgsl `snow_bares`) at most.
const SNOW_PATCH_M = 320.0; const SNOW_PATCH_STEP = 2.9; const SNOW_PATCH_FALL = 0.65; const SNOW_PATCH_OCTAVES = 6;
const SNOW_PATCHY = 2.5;
// The patches warm or cool the ground by this many kelvin a unit of the index, where its snow melts.
const SNOW_PATCH_K = 0.6;

// The snow's patches at `at`, seen at `e` m a pixel: index units, each octave fading in as it grows past a few pixels.
fn snow_patches(at: vec2f, e: f32) -> f32 {
  var n = 0.0;
  var size = SNOW_PATCH_M;
  var weight = 1.0;
  let sum = (1.0 - pow(SNOW_PATCH_FALL, f32(SNOW_PATCH_OCTAVES))) / (1.0 - SNOW_PATCH_FALL);
  for (var o = 0; o < SNOW_PATCH_OCTAVES; o++) {
    // Each octave finer than the last: none shows once one does not (nor in a draw of pixels that large).
    if (unseen(size) || shows_at(size, e) <= 0.0) { break; }
    n += weight * shows_at(size, e) * noise_turned(at / size + f32(o) * 5.1, 80 + o);
    size /= SNOW_PATCH_STEP;
    weight *= SNOW_PATCH_FALL;
  }
  return SNOW_PATCHY * clamp(2.0 * n / sum, -1.0, 1.0);
}

// What is at `at`, m from the map's north-west corner, seen at `e` m a pixel: its ground and its state (`state_on`), the
// ground's own colour and the sea blended between the squares' centres, the plants' field (lib/cover_field.wgsl); and
// its channel (`bed_at`), from its own sample's discharge. The channel's bed is cut into the ground, so where it runs
// into standing liquid that liquid takes it over at its shore at the same depth. Beside it the ground is wet by the
// capillary fringe over its water, which rises from the water's edge as the valley's side does, at least as steeply as
// the channel's own banks. Its water (`river_colour`) is as deep as its bed lies there; a pixel wider than the channel
// sees its mean depth.
fn land_spots(at: vec2f, e: f32) -> Spots {
  let look = earth.look;
  let cs = corners(at / land.map.x);
  var g = ground_at(at, true);
  // The loose ground with its patches, the bed it bares, and the relief finer than the tiles (relief.wgsl), which
  // moves the ground's height and slope: the shading's normal and the fine line of a shore.
  let loose = loose_seen(at, e);
  let bare = bed_shows(loose);
  let fine = relief(at, e, length(g.slope), bare, loose);
  // The relief moves the ground's height as much as the ground drawn is steep (`RELIEF_EDGE_SLOPE`): on a flat shore a
  // bump of a few decimetres would move the water's edge by tens of metres into a fringe of puddles.
  g.h += fine.x * ease(0.0, RELIEF_EDGE_SLOPE, length(g.slope));
  g.slope += fine.yz;
  let q = discharge_at(g);
  let bed = bed_at(g, q);
  let st = state_on(at, g, cs, bed.depth);
  let surface = blended(SURFACE, cs);
  var p: Spot;
  // A film of liquid over the drawn ground is soaked ground, not water (`FILM_M`).
  p.depth = select(st.depth, 0.0, st.depth < FILM_M);
  p.over = st.over;
  p.sea = surface.w;
  p.wet = st.wet;
  p.t = st.t;
  // The cells' snow placed where the ground holds it (`snow_at`); on standing liquid, the cells' share.
  // Its patches move both where the share lies and where the ground is warm enough to melt it (`SNOW_PATCH_K`), so
  // the line where the snow melts with height is as ragged as the rest.
  var bares = snow_bares(g.slope, st.rise, fine.x);
  var share = st.snow;
  if (st.covered > 0.0) {
    let patches = snow_patches(at, e);
    bares += patches;
    share = st.covered * unmelted(st.t + SNOW_PATCH_K * patches, land.time.z);
  }
  let snow_w = fwidth(bares);
  p.snow = select(snow_at(share, bares, snow_w), st.snow, st.depth > 0.0);
  p.ground = surface.xyz;
  p.z = g.h;
  p.slope = length(g.slope) * look.x;
  p.shade = hill_shade(g.slope, EXAGGERATE * look.x);
  p.loose = loose;
  // The height over the nearest water, standing or running: a channel's banks rise from its edge.
  let shore_over = select(p.over, min(p.over, max(g.d - bed.half, 0.0) * max(length(g.slope), bed.mean / max(bed.half, 1e-6))), bed.half > 0.0);
  let cf = cover_field(at, e, Beneath(soaked(p), shore_over, length(g.slope), loose, fwidth(loose), p.snow, st.snow_m));
  p.cover = cf.cover;
  p.canopy = cf.canopy;
  p.rock = bare;
  p.bed = vec3f(0.0);
  if (p.rock > 0.0) { p.bed = bed_make_at(at); }
  p.stone = select(0.0, stone_of(g.channels.xy), bed.half > 0.0 && g.d < STONES_OUT * bed.half);
  p.sand = beach(p.sea, g.h - earth.view.w, p.over, length(g.slope), p.loose, look.z);
  p.river = 0.0;
  p.river_depth = 0.0;
  p.at = at;
  p.e = e;
  // The depth less the film, signed (under the ground drawn: minus its height over the level, as the share of the point
  // that is the ponds' weighs it), over the depth's change across a pixel.
  let signed = st.depth - min(st.over, NO_LIQUID_M) * g.water.y - FILM_M;
  let pool = select(0.0, tent(signed / (max(length(g.slope) * g.water.y, SHORE_RISE_LEAST) * e)), g.water.y > 0.0);
  var out = Spots(p, p, 0.0, 0.0, pool, st.snow);
  if (p.depth > 0.0 || bed.half <= 0.0) { return out; }
  out.dry.over = shore_over;
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * LINE_PX * css * min(pow(q / shows, LINE_GROW), LINE_MOST) * ease(shows / LINE_FADE, shows, q);
  // The bed (`FLOOD`) and the thread of this month's water in it, `off` m from the line.
  let banks = banks_of(q);
  let free = max(banks - bed.half, 0.0);
  let open = ease(BED_PX * e, 2.0 * BED_PX * e, banks) * (1.0 - ease(banks, 2.0 * banks, line));
  // Where the bed is drawn as all water (from afar), the thread's wander is not read.
  var off = 0.0;
  if (open > 0.0) { off = free * ease(-0.1, 0.5, noise_turned(at / (2.0 * WANDER * banks), 63)); }
  let thread = abs(g.d - off);
  let in_bed = tent((max(banks, line) - g.d) / e) - tent((-max(banks, line) - g.d) / e);
  let water = tent((bed.half - thread) / e) - tent((-bed.half - thread) / e);
  out.share = mix(in_bed, water, open);
  out.bars = max(in_bed - out.share, 0.0);
  let resolved = ease(e, 2.0 * e, bed.half);
  let x = min(thread / bed.half, 1.0);
  out.wet.depth = mix(bed.mean, bed.depth, resolved);
  out.wet.sand = 0.0;
  out.wet.cover = vec4f(0.0);
  // The bed's slope across the channel, d(depth)/d(distance) along the distance's own slope; where the channel is
  // narrower than a pixel its banks do not show.
  let bank = 2.0 * BED_MIDDLE * bed.mean * x / bed.half * g.d_slope * select(0.0, 1.0, x < 1.0) * resolved;
  out.wet.slope = length(g.slope + bank) * look.x;
  out.wet.shade = hill_shade(g.slope + bank, EXAGGERATE * look.x);
  return out;
}

// A stream's bed sends back this share of the light soaked bare ground does: a film of algae and fine dead matter
// coats the stones under running water, and a bed sends back about 5% of the light against about 20% for soaked sand
// (Legleiter and others 2004; from memory).
const BED_FILM = 0.3;

// The colour of a channel's water at spot `p` (`land_spots`) over its bed `g` (colour.wgsl `ground_for`), a reflectance
// in linear light: its bed, soaked bare ground (washed sand and gravel: the flow takes the plants) under its film
// (`BED_FILM`), seen through running water as deep as it is there (water.wgsl); snow lies on it as on standing liquid
// (colour.wgsl `colour_on`), and the true view shades its bed as far as it shows.
fn river_on(p: Spot, g: Ground) -> Shaded {
  let lit = light(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), p.slope);
  let bed = BED_FILM * soil(g, 1.0, lit);
  let ripple = ripples(p.at, p.e) * (1.0 - p.snow);
  // In the style a channel's water is coloured as deeper than it is (style.wgsl `river_seen`), lighter over its banks.
  var water = seen_through(bed, p.depth, ripple, RUNNING_ATTENUATION, DEEP_WATER);
  if (styled()) { water = water_painted(bed, river_seen(p.depth, p.e), ripple); }
  let rgb = mix(water, SNOW, p.snow * 0.85);
  return Shaded(rgb, 1.0 + (lit - 1.0) * exp(-2.0 * RUNNING_ATTENUATION.g * p.depth), RIVER);
}

// The colours a pixel mixes, each over the last by the share of the pixel it covers: its ground (`LAYER_GROUND`); the
// bars of a channel's bed, washed gravel, bare, as high over the water as it is deep (`LAYER_BARS`); where the edge of
// standing liquid crosses the pixel, the other side of it (`LAYER_EDGE`); a channel's water (`LAYER_RIVER`). One loop
// draws them all, so the colour's code (its ground, rock and stones) is compiled once.
const LAYER_GROUND = 0; const LAYER_BARS = 1; const LAYER_EDGE = 2; const LAYER_RIVER = 3; const LAYERS = 4;

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let at = earth.place.xy + v.uv * earth.place.zw;
  let cs = corners(at / land.map.x);
  let spots = land_spots(at, metres_a_pixel(v.uv));
  var c = Shaded(vec3f(0.0), 1.0, BARE);
  for (var k = LAYER_GROUND; k < LAYERS; k++) {
    var p = spots.dry;
    var share = 1.0;
    if (k == LAYER_BARS) {
      share = spots.bars;
      p.sand = 1.0;
      p.cover = vec4f(0.0);
      p.over = spots.wet.depth;
    } else if (k == LAYER_EDGE) {
      share = select(0.0, select(1.0 - spots.pool, spots.pool, spots.dry.depth <= 0.0), spots.pool > 0.0 && spots.pool < 1.0);
      p.depth = select(FILM_M, 0.0, spots.dry.depth > 0.0);
    } else if (k == LAYER_RIVER) {
      share = spots.share;
      p = spots.wet;
    }
    if (share <= 0.0) { continue; }
    let running = k == LAYER_RIVER;
    let g = ground_for(p, running);
    var o: Shaded;
    if (running) { o = river_on(p, g); } else { o = colour_on(p, g); }
    c = Shaded(mix(c.rgb, o.rgb, share), mix(c.m, o.m, share), select(c.kind, o.kind, k != LAYER_BARS && share > 0.5));
  }
  // The true colour is the cells', and the bed's where it shows; their snow placed as the Earth view places it: the
  // cells' colour is taken as their ground's under their share of snow, and that ground drawn under the point's snow.
  // On standing liquid its ice is placed so too (ice.wgsl `ice_cover`), in the true view only.
  var truth = blended(TRUTH, cs).xyz;
  let s = spots.snow_cells;
  if (s > 0.0 && s < SNOW_CELLS_ALL) {
    let under = clamp((truth - s * SNOW) / (1.0 - s), vec3f(0.0), vec3f(1.0));
    var placed = spots.dry.snow;
    if (earth.view.x == 1.0 && spots.dry.depth > 0.0) { placed = ice_cover(s, spots.dry.depth, at, spots.dry.e); }
    truth = mix(under, SNOW, placed);
  }
  truth = mix(truth, spots.dry.bed, spots.dry.rock);
  let rgb = in_view_of(c, blended(VIEW, cs).xy, truth);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
