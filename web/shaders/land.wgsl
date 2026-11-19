// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl crag.wgsl stone.wgsl ice.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl river.wgsl flow.wgsl cover_field.wgsl sprites.wgsl heights.wgsl tex.wgsl detail.wgsl things.wgsl afloat.wgsl
// least metres a pixel: 0.5 1 3 30
// The live map in view: at each pixel of the screen the ground from its tiles and the state from the cells' table
// (land.wgsl), in the view's colours. Made once more for draws whose pixels take at least 0.5 m, 1 m, 3 m and 30 m
// (web/shade, draw.wgsl `LEAST_E`): seen from afar, the detail of the ground and the plants that cannot show is
// compiled out (from 30 m, the whole island on a phone, the sea ice's floes and the pack's texture: ice.wgsl).

// What grows along a channel is read within this many of its bed's half widths of its line: its bed, its banks and the
// stones beside a steep reach lie within them.
const CHANNEL_NEAR = 3.0;
// A channel's stones lie on its bed and its bars, in `IN_CHANNEL` of the places they could, fewer where it moves
// only sand (`BAR_SAND_STONE`); beside a steep reach they lie out to `STONES_OUT` of its bed's half widths from its
// line, thinning, and there the bed is bare (`WALL` of it: a steep reach runs between walls of its bed), and blocks
// fallen from those walls lie in and beside it, angular, in `BLOCKS` of the places a scree's could (`Spot.blocks`),
// its own stones `STEEP_SHARP` less worn. A channel wears its stones round as it carries them: angular where it carries `WORN_Q.x` m³/s (a
// head's rill), round from `WORN_Q.y` (the travel grows with the area drained, and so does the discharge). Up a bar
// they are finer: `BAR_FINES` smaller at its top. Between two samples a stone is drawn from `STONE_SMALL.x` to
// `STONE_SMALL.y` of its sample's size.
const IN_CHANNEL = 0.6; const STONES_OUT = 1.8; const WORN_Q = vec2f(0.005, 0.3); const BAR_FINES = 0.5; const WALL = 0.85; const BLOCKS = 0.5; const STEEP_SHARP = 0.5;
const STONE_SMALL = vec2f(0.3, 1.25);
// The edge of standing liquid is found from how fast its depth changes across the ground, the ground's slope: no slower
// than this (rise over run), so on flat ground the edge still lies within a pixel's footprint where the depth crosses.
const SHORE_RISE_LEAST = 1e-3;
// The relief moves the ground's height fully on ground drawn this steep (rise over run), less on gentler.
const RELIEF_EDGE_SLOPE = 0.1;
// Cells nearly all under snow keep their true colour as it is: the ground under it is not found from so little.
const SNOW_CELLS_ALL = 0.95;

// What is at a point: the ground without the channel's water (`dry`: on the bars of its bed washed gravel, bare, as
// far as they cover the pixel, river.wgsl `river_spots`), the channel's water (`wet`) and the share of the
// pixel that water covers; and the share
// of the pixel standing liquid covers (`pool`): the pixel's footprint across the line where the depth crosses `FILM_M`,
// so a shore is a smooth line at any zoom, not the pixels' steps;
// and of the channel's water: the signs of its flow (lib/flow.wgsl `flow_signs`), and (`water`) its depth over the
// depth the channel law gives its middle and how near its edge the point lies, 0..1; and the way it runs (`down`, a
// unit vector east and south).
struct Spots { dry: Spot, wet: Spot, share: f32, pool: f32, snow_cells: f32, flow: vec3f, water: vec2f, down: vec2f }

// The spot `s` with its channel's stones: on the bed of channel `ch` (of discharge `q`, on ground `g`), and beside it
// where the reach is steep, with the walls and the fallen blocks of a steep reach. The largest drawn takes its size
// from the point's own sample (`cobble` m, in a step where two samples meet: the size sets where the stones lie); what
// each is of it follows the size the channel moves there (`moved` m), which grows evenly along it.
fn stony(s: Spot, g: Tiled, ch: Channel, q: f32, moved: f32, cobble: f32) -> Spot {
  var p = s;
  p.stone = STONE_STEP * cobble;
  p.small = clamp(max(STONE_SEEN * moved, COBBLE_M) / cobble, STONE_SMALL.x, STONE_SMALL.y);
  let steep_reach = ease(STEEP_STONE.x, STEEP_STONE.y, moved);
  let beside = steep_reach * (1.0 - ease(ch.bed, STONES_OUT * ch.bed, g.d));
  p.stony = IN_CHANNEL * ease(0.0, BAR_SAND_STONE, moved) * select(beside, 1.0, g.d < ch.bed);
  p.blocks = BLOCKS * steep_reach * select(beside, 1.0, g.d < ch.bed);
  p.rock = max(p.rock, WALL * beside);
  p.cover *= 1.0 - WALL * beside;
  if (p.stony > 0.0) { p.worn = ease(log(WORN_Q.x), log(WORN_Q.y), log(max(q, 1e-9))) * (1.0 - STEEP_SHARP * steep_reach); }
  return p;
}

// Herbs float their leaves on liquid from their height's depth on, all of them from `FLOAT_TALL` times it.
const FLOAT_TALL = 1.5;

// What is at `at`, m from the map's north-west corner, seen at `e` m a pixel: its ground and its state (`state_on`), the
// ground's own colour and the sea blended between the squares' centres, the plants' field (lib/cover_field.wgsl); and
// its channel (rock.wgsl `channel_at`), from its own sample's discharge. The channel's bed is cut into the ground, and
// where it runs into standing liquid it runs on through that liquid's band of shallows (`river_spots`). Beside it the ground is
// wet by the capillary fringe over its water, which rises from the water's edge as the valley's side does, at least as
// steeply as the channel's own banks. Its water (`river_on`) is as deep as its bed lies there; a pixel wider than the
// channel sees its mean depth.
fn land_spots(at: vec2f, e: f32) -> Spots {
  let look = earth.look;
  let cs = corners(at / land.map.x);
  var g = ground_at(at, true);
  // The loose ground with its patches, the bed it bares, and the relief finer than the tiles (relief.wgsl), which
  // moves the ground's height and slope: the shading's normal and the fine line of a shore.
  let loose_mean = loose_at(at);
  let loose = loose_seen(at, loose_mean, e);
  let bare = bed_shows(loose);
  let fine = relief(e, length(g.slope), bare, loose);
  // The waves' swash follows the shore as the tiles draw it: the relief's bumps would break its line into specks.
  let tiled = g;
  // The relief moves the ground's height as much as the ground drawn is steep (`RELIEF_EDGE_SLOPE`): on a flat shore a
  // bump of a few decimetres would move the water's edge by tens of metres into a fringe of puddles.
  g.h += fine.h * ease(0.0, RELIEF_EDGE_SLOPE, length(g.slope));
  g.slope += fine.slope;
  // Its channel: what runs through it and the largest stone it moves, each grown evenly along it (rock.wgsl
  // `next_along`).
  let grows = vec2f(dpdx(g.along), dpdy(g.along));
  let reach = sample_at(g);
  var next = Next(reach, 0.0);
  var q = discharge_of(reach);
  // (Most points that hold a channel lie far from its line, in its reach alone: there its own sample's discharge says
  // enough, and what grows along it, its stones and its neighbour's are read within `CHANNEL_NEAR` of its line.)
  var banks = banks_of(q);
  let near = g.d < max(CHANNEL_NEAR * banks, line_most(e)) + 2.0 * e;
  if (near) {
    next = next_along(reach, at);
    q = mix(q, discharge_of(next.s), next.share);
    banks = banks_of(q);
  }
  let mouth = through_band(g, e);
  let ch = channel_of(g, q, banks, mouth);
  let st = state_on(at, g, cs, depth_in(g, ch), e, fine.h);
  let surface = blended(SURFACE, cs);
  var p: Spot;
  // A film of liquid over the drawn ground is soaked ground, not water (`FILM_M`).
  p.depth = select(st.depth, 0.0, st.depth < FILM_M);
  p.over = st.over;
  p.sea = surface.w;
  p.wet = st.wet;
  p.t = st.t;
  // The cells' snow placed where the ground holds it (`snow_at`); on standing liquid, the cells' share.
  p.snow = select(snow_at(st.snow, st.bares, fwidth(st.bares)), st.snow, st.depth > 0.0);
  p.ground = surface.xyz;
  p.z = g.h;
  p.slope = length(g.slope) * look.x;
  p.shade = hill_shade(g.slope, EXAGGERATE * look.x);
  p.loose = loose;
  // The height over the nearest water, standing or running: a channel's banks rise from its edge.
  let fringe = ease(FRINGE_PX.x * e, FRINGE_PX.y * e, 2.0 * ch.water);
  let shore_over = select(p.over, mix(p.over, min(p.over, max(abs(g.sd - ch.middle) - ch.water, 0.0) * max(length(g.slope), ch.mean / max(ch.water, 1e-6))), fringe), ch.water > 0.0);
  let cf = cover_field(at, e, Beneath(soaked(p), shore_over, length(g.slope), p.depth, loose, loose_mean, fwidth(loose), p.snow, st.snow_m));
  p.cover = cf.cover;
  p.canopy = cf.canopy;
  p.shed = cf.shed;
  p.wading = cf.wet.x;
  p.reach = cf.wet.y;
  // Herbs that stand in liquid deeper than they are tall float their leaves on it (afloat.wgsl), where their roots
  // reach (draw.wgsl `stands_in`): over the share of the ground they cover.
  p.pads = cf.herbs * ease(cf.wet.w, FLOAT_TALL * cf.wet.w, p.depth) * stands_in(p.depth, cf.wet.z);
  p.rock = bare;
  p.bed = vec3f(0.0);
  // Its channel's stones (`stony`), and the scree's place below the bed that sheds it: where the loose ground deepens
  // down the slope. The largest stone the channel moves and its reach's slope grow evenly along it.
  // (No bed takes `BED_PX` pixels and no stone shows in a program made for draws from far: neither is read there.)
  var moved = 0.0;
  var fall = 0.0;
  var cobble = COBBLE_M;
  if (near && ch.bed > 0.0 && BED_PX * least_e() < BED_MOST_M) {
    let own = stone_and_fall(reach);
    let grown = mix(own, stone_and_fall(next.s), next.share);
    moved = grown.x;
    fall = grown.y;
    p.moved = moved;
    cobble = max(STONE_SEEN * own.x, COBBLE_M);
  }
  let deepens = vec2f(dpdx(loose), dpdy(loose));
  if (!unseen(STONE_MOST_M / THING_PX.x)) {
    p = stony(p, g, ch, q, moved, cobble);
    p.below = mix(BELOW_LEAST, 1.0, 0.5 - 0.5 * dot(deepens, g.slope) / max(length(deepens) * length(g.slope), 1e-9));
  }
  p.snow_m = st.snow_m;
  // The bed's colour where it shows or stones of it lie: a channel's, and the scree's on thin loose ground.
  if (p.rock > 0.0 || p.stony > 0.0 || loose < SCREE_LOOSE_M) { p.bed = bed_make_at(at); }
  p.fetch = g.water.z;
  let waves = waves_of(p.sea, p.fetch);
  p.sand = beach(p.sea, g.h - earth.view.z, p.over, length(g.slope), p.loose, look.z, waves);
  p.swash = swash_of(over_ponds(tiled, 0.0), waves, length(tiled.slope), at, e, p.snow);
  // A beach of pebbles takes their colour from the bed they are worn from (colour.wgsl `shore_grain`).
  if (p.sand > 0.0 && p.rock <= 0.0 && p.stony <= 0.0 && loose >= SCREE_LOOSE_M) { p.bed = bed_make_at(at); }
  p.river = 0.0;
  p.river_depth = 0.0;
  p.mouth = 0.0;
  p.at = at;
  p.e = e;
  p.hollow = fine.hollow;
  p.hollow_d = vec2f(dpdx(fine.hollow), dpdy(fine.hollow));
  p.basin = fine.basin;
  p.low = cf.low;
  // The depth less the film, signed (under the ground drawn: minus its height over the level, as the share of the point
  // that is the ponds' weighs it), over the depth's change across a pixel.
  let signed = st.depth - min(st.over, NO_LIQUID_M) * g.water.y - FILM_M;
  let pool = select(0.0, tent(signed / (max(length(g.slope) * g.water.y, SHORE_RISE_LEAST) * e)), g.water.y > 0.0);
  var out = Spots(p, p, 0.0, pool, st.snow, vec3f(0.0), vec2f(0.0), vec2f(0.0));
  // A channel is drawn on the land and through the band of shallows of the liquid it runs into, no farther.
  if (ch.water <= 0.0 || mouth >= 1.0) { return out; }
  if (p.depth <= 0.0) { out.dry.over = shore_over; }
  if (!near) { return out; }
  return river_spots(out, g, ch, q, moved, fall, grows, line_of(q, e), mouth);
}

// The colours a pixel mixes, each over the last by the share of the pixel it covers: its ground (`LAYER_GROUND`); where
// the edge of standing liquid crosses the pixel, the other side of it (`LAYER_EDGE`); then a channel's water. One loop
// draws the first two, so their colour's code is compiled once. A graphics card that runs both sides of every branch
// runs each layer's code at every pixel: so the layers are few (a channel's bars are part of the ground's), and the
// water's own colour (`river_on`) stands after the loop, run once a pixel.
const LAYER_GROUND = 0; const LAYER_EDGE = 1; const LAYERS = 2;

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  // The pixel's place from its own pixel (not from the map's whole rectangle, whose steps a float shows at a
  // centimetre): its distance from the draw's origin, which fine detail is hashed from (draw.wgsl `lat`), and its place
  // in the world.
  pix_origin = earth.frame.xy;
  pix_local = earth.corner.xy + v.px * earth.frame.z;
  let at = pix_origin + pix_local;
  let cs = corners(at / land.map.x);
  // Detail is chosen by the screen's pixel, so a picture drawn at fewer pixels is the same picture.
  let spots = land_spots(at, earth.frame.w);
  var c = Shaded(vec3f(0.0), 1.0, BARE);
  for (var k = LAYER_GROUND; k < LAYERS; k++) {
    var p = spots.dry;
    var share = 1.0;
    if (k == LAYER_EDGE) {
      share = select(0.0, select(1.0 - spots.pool, spots.pool, spots.dry.depth <= 0.0), spots.pool > 0.0 && spots.pool < 1.0);
      p.depth = select(FILM_M, 0.0, spots.dry.depth > 0.0);
    }
    if (share <= 0.0) { continue; }
    let o = colour_on(p, ground_for(p, false));
    c = Shaded(mix(c.rgb, o.rgb, share), mix(c.m, o.m, share), select(c.kind, o.kind, share > 0.5));
  }
  if (spots.share > 0.0) {
    let o = river_on(spots.wet, ground_for(spots.wet, true), spots.flow, spots.water, spots.down);
    c = Shaded(mix(c.rgb, o.rgb, spots.share), mix(c.m, o.m, spots.share), select(c.kind, o.kind, spots.share > 0.5));
  }
  // The true colour is the cells', and the bed's where it shows; their snow placed as the Earth view places it: the
  // cells' colour is taken as their ground's under their share of snow, and that ground drawn under the point's snow.
  // On standing liquid its ice is placed so too (ice.wgsl `ice_cover`). Each is read only in the view that draws it
  // (colour.wgsl `in_view_of`): the true colour in the true view, the layer's value in a layer's view.
  var truth = vec3f(0.0);
  if (earth.view.x == 1.0) {
    truth = blended(TRUTH, cs).xyz;
    let s = spots.snow_cells;
    if (s > 0.0 && s < SNOW_CELLS_ALL) {
      let under = clamp((truth - s * SNOW) / (1.0 - s), vec3f(0.0), vec3f(1.0));
      var placed = spots.dry.snow;
      if (spots.dry.depth > 0.0) { placed = ice_cover(s, spots.dry.depth, at, spots.dry.e); }
      truth = mix(under, SNOW, placed);
    }
    truth = mix(truth, spots.dry.bed, spots.dry.rock);
  }
  var shown = 0.0;
  if (earth.view.x == 2.0) { shown = blended(VIEW, cs).x; }
  let rgb = in_view_of(c, shown, truth);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
