// uses: draw.wgsl water.wgsl style.wgsl crag.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl cover_field.wgsl
// The live map in view: at each pixel of the screen the ground from its tiles and the state from the cells' table
// (land.wgsl), in the view's colours.

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

// What is at a point: the ground without the channel's water (`dry`), the channel's water (`wet`) and the share of the
// pixel that water covers; the share the bars of its bed cover (`bars`, the dry spot as washed gravel); and the share
// of the pixel standing liquid covers (`pool`): the pixel's footprint across the line where the depth crosses `FILM_M`,
// so a shore is a smooth line at any zoom, not the pixels' steps.
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32 }

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
  g.h += fine.x;
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
  p.snow = st.snow;
  p.ground = surface.xyz;
  p.canopy = field_at(at, STANDS, 0.0).a;
  p.z = g.h;
  p.slope = length(g.slope) * look.x;
  p.shade = hill_shade(g.slope, EXAGGERATE * look.x);
  p.loose = loose;
  p.cover = cover_field(at, e, Beneath(soaked(p), p.canopy, loose_at(at), st.snow, st.snow_m));
  p.rock = bare;
  p.bed = vec3f(0.0);
  if (p.rock > 0.0) {
    let soil = blended(SOIL, cs);
    p.bed = vec3f(soil.zw, blended(BED, cs).x);
  }
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
  var out = Spots(p, p, 0.0, 0.0, pool);
  if (p.depth > 0.0 || bed.half <= 0.0) { return out; }
  out.dry.over = min(p.over, max(g.d - bed.half, 0.0) * max(length(g.slope), bed.mean / bed.half));
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * LINE_PX * css * min(pow(q / shows, LINE_GROW), LINE_MOST) * ease(shows / LINE_FADE, shows, q);
  // The bed (`FLOOD`) and the thread of this month's water in it, `off` m from the line.
  let banks = banks_of(q);
  let free = max(banks - bed.half, 0.0);
  let off = free * ease(-0.1, 0.5, noise_turned(at / (2.0 * WANDER * banks), 63));
  let thread = abs(g.d - off);
  let in_bed = tent((max(banks, line) - g.d) / e) - tent((-max(banks, line) - g.d) / e);
  let water = tent((bed.half - thread) / e) - tent((-bed.half - thread) / e);
  let open = ease(BED_PX * e, 2.0 * BED_PX * e, banks) * (1.0 - ease(banks, 2.0 * banks, line));
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

// The colour of a channel's water at spot `p` (`land_spots`), a reflectance in linear light: its bed, soaked bare
// ground (washed sand and gravel: the flow takes the plants) under its film (`BED_FILM`), seen through running water as
// deep as it is there (water.wgsl); snow lies on it as on standing liquid (colour.wgsl `colour`), and the true view
// shades its bed as far as it shows.
fn river_colour(p: Spot) -> Shaded {
  let lit = light(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), p.slope);
  let bed = BED_FILM * soil(ground_of(p, 0.0), 1.0, lit);
  let ripple = ripples(p.at, p.e) * (1.0 - p.snow);
  // In the style a channel's water is a lake's (style.wgsl `lake_seen`), lighter over its shallow banks.
  var water = seen_through(bed, p.depth, ripple, RUNNING_ATTENUATION, DEEP_WATER);
  if (styled()) { water = water_painted(bed, lake_seen(p.depth, 0.0, p.e, RIVER_RIM_M), ripple); }
  let rgb = mix(water, SNOW, p.snow * 0.85);
  return Shaded(rgb, 1.0 + (lit - 1.0) * exp(-2.0 * RUNNING_ATTENUATION.g * p.depth), RIVER);
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let at = earth.place.xy + v.uv * earth.place.zw;
  let cs = corners(at / land.map.x);
  let spots = land_spots(at, metres_a_pixel(v.uv));
  var c = colour(spots.dry);
  // The bars of a channel's bed: washed gravel, bare, as high over the water as it is deep.
  if (spots.bars > 0.0) {
    var bar = spots.dry;
    bar.sand = 1.0;
    bar.cover = vec4f(0.0);
    bar.over = spots.wet.depth;
    let b = colour(bar);
    c = Shaded(mix(c.rgb, b.rgb, spots.bars), mix(c.m, b.m, spots.bars), c.kind);
  }
  // Where the edge of standing liquid crosses the pixel: the ground and the liquid by the share each covers.
  if (spots.pool > 0.0 && spots.pool < 1.0) {
    var other = spots.dry;
    other.depth = select(FILM_M, 0.0, spots.dry.depth > 0.0);
    let o = colour(other);
    let k = select(1.0 - spots.pool, spots.pool, spots.dry.depth <= 0.0);
    c = Shaded(mix(c.rgb, o.rgb, k), mix(c.m, o.m, k), select(c.kind, o.kind, k > 0.5));
  }
  if (spots.share > 0.0) {
    let w = river_colour(spots.wet);
    c = Shaded(mix(c.rgb, w.rgb, spots.share), mix(c.m, w.m, spots.share), select(c.kind, RIVER, spots.share > 0.5));
  }
  // The true colour is the cells', and the bed's where it shows.
  let truth = mix(blended(TRUTH, cs).xyz, spots.dry.bed, spots.dry.rock);
  let rgb = in_view_of(c, blended(VIEW, cs).xy, truth);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
