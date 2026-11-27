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
// A soil takes `SOIL_OF_BED` of its bed's colour where the bed is bare, less as its loose ground deepens, none from
// `SOIL_OF_BED_M` m on (Earth view only).
const SOIL_OF_BED = 0.9; const SOIL_OF_BED_M = 3.0;

// What is at a point: the ground without the channel's water (`dry`: on the bars of its bed washed gravel, bare, as
// far as they cover the pixel, river.wgsl `river_spots`), the channel's water (`wet`) and the share of the
// pixel that water covers; and the share
// of the pixel standing liquid covers (`pool`): the pixel's footprint across the line where the depth crosses `FILM_M`,
// so a shore is a smooth line at any zoom, not the pixels' steps;
// and of the channel's water: the signs of its flow (lib/flow.wgsl `flow_signs`), and (`water`) its depth over the
// depth the channel law gives its middle and how near its edge the point lies, 0..1; and the way it runs (`down`, a
// unit vector east and south).
struct Spots { dry: Spot, wet: Spot, share: f32, pool: f32, flow: vec3f, water: vec2f, down: vec2f }

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

// The byte of the layer drawn at `at` (`dry` its spot over the ground), 0..1: the cells' through their centres as a
// B-spline (`cells_smooth`, so no square shows where the cells' values jump), or of a layer of the soil the live map
// finds at each point (colour.wgsl `Earth.view`), its share as a byte of a layer from 0 to 1: the share of the pores
// the soil's liquid fills by the laws inside a cell (`State`), the grain's shares of the samples (rock.wgsl
// `soil_samples_at`, without the patches the Earth's colours draw), the liquid the samples' loose ground holds for roots as
// its layer shows it, or the share of that the roots read at the point: on the saturated strips all of it, on the drier
// rest the share the core's cover read there in its cell's last month (the layer `roots`, `cover::CellRan`), blended
// over the edge of the wet ground; or how saturated the ground is there (land.wgsl `saturated_at`).
fn layer_at(at: vec2f, dry: Spot) -> f32 {
  let k = earth.view.w;
  if (k == 2.0) { return dry.filled; }
  if (k == 3.0) { return soil_samples_at(at, GRAIN_PLANE).x; }
  if (k == 4.0) { return soil_samples_at(at, GRAIN_PLANE).y; }
  if (k == 5.0) { return soil_samples_at(at, GRAIN_PLANE).w; }
  if (k == 6.0) { return mix(cells_smooth(VIEW, at / land.map.x).x, 1.0, saturated_at(at)); }
  if (k == 7.0) { return saturated_at(at); }
  return cells_smooth(VIEW, at / land.map.x).x;
}

// A plume's load is drawn in streaks in the Earth view (drawing only: the core moves no currents in a body of liquid):
// `PLUME_STREAK` times an even noise over `PLUME_STREAK_M` m along the e-fold of the load, so the cloud's edge breaks
// into tongues and filaments.
const PLUME_STREAK = 1.2; const PLUME_STREAK_M = 300.0;
// The load `load` g/m³ of standing liquid at the pixel, as drawn.
fn plume_streaks(load: f32) -> f32 {
  if (load <= 0.0 || !styled()) { return load; }
  return load * exp(PLUME_STREAK * noise_l(lat_turned(PLUME_STREAK_M, vec2f(0.3)), 216).x);
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
  // (Its change across a pixel taken the short way round the length the place along is kept modulo.)
  let grows = vec2f(dpdx(g.along), dpdy(g.along));
  let grows_short = grows - land.reach.x * round(grows / land.reach.x);
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
  let st = state_on(at, g, cs, depth_in(g, ch), fine.h);
  let surface = blended(SURFACE, cs);
  var p: Spot;
  // A film of liquid over the drawn ground is soaked ground, not water (`FILM_M`).
  p.depth = select(st.depth, 0.0, st.depth < FILM_M);
  // What standing liquid carries: its plumes' load (web/src/land.ts), in the Earth view in streaks (`PLUME_STREAK`).
  if (p.depth > 0.0) { p.load = plume_streaks(plume_at(at)); }
  p.over = st.over;
  p.sea = surface.w;
  p.wet = st.wet;
  p.filled = st.filled;
  p.t = st.t;
  // The cells' snow placed where the ground holds it (`snow_at`); on standing liquid, the cells' share.
  let snow_x = snow_past(st.snow, st.bares);
  p.snow = select(snow_at(at, e, st.snow, snow_x, fwidth(snow_x)), st.snow, st.depth > 0.0);
  p.ground = surface.xyz;
  // In the Earth view a thin soil shows the colour of the bed it is broken from (a residual soil, worldgen::terrain):
  // the cells' soil is a mean over kilometres, the bed's make is the samples'.
  // (The bed's make is read once, where something below asks for it.)
  var make = vec3f(0.0);
  var made = false;
  if (styled() && loose < SOIL_OF_BED_M) {
    make = bed_make_at(at); made = true;
    p.ground = mix(p.ground, make, SOIL_OF_BED * (1.0 - loose / SOIL_OF_BED_M));
  }
  p.z = g.h;
  p.slope = length(g.slope) * look.x;
  p.shade = hill_shade(g.slope, EXAGGERATE * look.x);
  p.loose = loose;
  let soil = soil_at(at, e, blended(VIEW, cs).y, blended(GRAIN_SHIFT, cs).xy);
  p.grain = soil.grain;
  p.deposit = soil.deposit;
  // The height over the nearest water, standing or running: a channel's banks rise from its edge.
  let fringe = ease(FRINGE_PX.x * e, FRINGE_PX.y * e, 2.0 * ch.water);
  let shore_over = over_water(g, ch, p.over, fringe);
  let cf = cover_field(at, e, Beneath(max(soaked(p), marsh_of(p)), shore_over, length(g.slope), p.depth, loose, loose_mean, fwidth(loose), p.snow, st.snow_m));
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
  if (p.rock > 0.0 || p.stony > 0.0 || loose < SCREE_LOOSE_M) {
    if (!made) { make = bed_make_at(at); made = true; }
    p.bed = make;
  }
  p.fetch = g.water.z;
  let waves = waves_of(p.sea, p.fetch);
  // On a deposit (colour.wgsl `laid_share`) the waves sort sand up their storms' swash alone: past it lies what the
  // floods laid, a delta's top or a flood plain, not a beach.
  p.sand = beach(p.sea, g.h - earth.view.z, p.over, length(g.slope), p.loose, look.z * (1.0 - laid_share(p)), waves);
  p.swash = swash_of(over_ponds(tiled, 0.0), waves, length(tiled.slope), at, e, p.snow);
  // A beach of pebbles takes their colour from the bed they are worn from (colour.wgsl `shore_grain`).
  if (p.sand > 0.0 && p.rock <= 0.0 && p.stony <= 0.0 && loose >= SCREE_LOOSE_M) {
    if (!made) { make = bed_make_at(at); }
    p.bed = make;
  }
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
  // The marks of fire on dry ground: its burn scars and the plants they took (burn.wgsl).
  p = burnt_spot(p);
  var out = Spots(p, p, 0.0, pool, vec3f(0.0), vec2f(0.0), vec2f(0.0));
  // A channel is drawn on the land and through the band of shallows of the liquid it runs into, no farther.
  if (ch.water <= 0.0 || mouth >= 1.0) { return out; }
  if (p.depth <= 0.0) { out.dry.over = shore_over; }
  if (!near) { return out; }
  var r = river_spots(out, g, ch, q, moved, fall, grows_short, line_of(q, e), mouth);
  // A channel's water carries its sample's load, grown along it as its discharge is.
  r.wet.load = mix(load_of(reach), load_of(next.s), next.share);
  return r;
}

// The colours a pixel mixes, each over the last by the share of the pixel it covers: its ground (`LAYER_GROUND`); where
// the edge of standing liquid crosses the pixel, the other side of it (`LAYER_EDGE`); then a channel's water
// (`LAYER_RIVER`). One loop draws all three, so the ground's colour (`ground_for`, the largest part of the program) is
// compiled once. A graphics card that runs both sides of every branch runs each layer's code at every pixel: so the
// layers are few (a channel's bars are part of the ground's).
const LAYER_GROUND = 0; const LAYER_EDGE = 1; const LAYER_RIVER = 2; const LAYERS = 3;

fn live_map(v: Varying) -> vec4f {
  // The pixel's place from its own pixel (not from the map's whole rectangle, whose steps a float shows at a
  // centimetre): its distance from the draw's origin, which fine detail is hashed from (draw.wgsl `lat`), and its place
  // in the world.
  pix_origin = earth.frame.xy;
  pix_local = earth.corner.xy + v.px * earth.frame.z;
  bends_e = earth.frame.w;
  let at = pix_origin + pix_local;
  // Detail is chosen by the screen's pixel, so a picture drawn at fewer pixels is the same picture.
  let spots = land_spots(at, earth.frame.w);
  var c = Shaded(vec3f(0.0), 1.0, BARE);
  for (var k = LAYER_GROUND; k < rolled(LAYERS); k++) {
    var p = spots.dry;
    var share = 1.0;
    if (k == LAYER_EDGE) {
      share = select(0.0, select(1.0 - spots.pool, spots.pool, spots.dry.depth <= 0.0), spots.pool > 0.0 && spots.pool < 1.0);
      p.depth = select(FILM_M, 0.0, spots.dry.depth > 0.0);
    }
    if (k == LAYER_RIVER) { p = spots.wet; share = spots.share; }
    if (share <= 0.0) { continue; }
    let g = ground_for(p, k == LAYER_RIVER);
    var o: Shaded;
    if (k == LAYER_RIVER) { o = river_on(p, g, spots.flow, spots.water, spots.down); } else { o = colour_on(p, g); }
    c = Shaded(mix(c.rgb, o.rgb, share), mix(c.m, o.m, share), select(c.kind, o.kind, share > 0.5));
  }
  var shown = 0.0;
  if (earth.view.x == 1.0) { shown = layer_at(at, spots.dry); }
  var rgb = in_view_of(c, shown);
  // In the Earth view the dry ground takes the shade of its steepness and of the heights toward the light.
  if (styled() && spots.dry.depth <= 0.0) { rgb *= relief_shade(spots.dry.slope, horizon_at(at)); }
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
