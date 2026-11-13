// uses: draw.wgsl water.wgsl style.wgsl crag.wgsl stone.wgsl ice.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl river.wgsl flow.wgsl cover_field.wgsl
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

// What is at a point: the ground without the channel's water (`dry`), the channel's water (`wet`) and the share of the
// pixel that water covers; the share the bars of its bed cover (`bars`, the dry spot as washed gravel); and the share
// of the pixel standing liquid covers (`pool`): the pixel's footprint across the line where the depth crosses `FILM_M`,
// so a shore is a smooth line at any zoom, not the pixels' steps; the bars' colour and their height over the water, m;
// and of the channel's water: the signs of its flow (lib/flow.wgsl `flow_signs`), and (`water`) its depth over the
// depth the channel law gives its middle and how near its edge the point lies, 0..1; and the way it runs (`down`, a
// unit vector east and south).
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32, snow_cells: f32, bar: vec3f, bar_over: f32, flow: vec3f, water: vec2f, down: vec2f }

// Inside a cell its snow lies first where the ground holds it (`snow_bares`): high ground before low (colder: the air's
// fall with height; over `SNOW_RISE_M` a unit), slopes turned from the sun before those facing it (they melt first;
// `SNOW_SUNNY` a unit of slope toward the light), hollows before knobs (drift fills them; `SNOW_HOLLOW_M` of the
// relief's height a unit). The cell's share `s` covers where this index lies under `SNOW_SPREAD` ln(s / (1 - s)): of
// an index spread about 1 either way (a logistic one), the share s, so as much snow is drawn as the cell holds. Its
// edge is a line of that field, no narrower than `SNOW_EDGE` of the index.
// The index is held softly within `SNOW_BARES_MOST` either way, so a cell's last few hundredths of snow lie nowhere,
// and no snow ends on the straight line where the blended share reaches none (a line through the squares' centres).
const SNOW_RISE_M = 40.0; const SNOW_SUNNY = 2.0; const SNOW_HOLLOW_M = 0.25; const SNOW_SPREAD = 1.0; const SNOW_EDGE = 0.05;
const SNOW_BARES_MOST = 3.0;

// How readily the ground of slope `slope` (east, south), `rise` m over its cells' mean and `knob` m over its relief's
// mean (relief.wgsl), sheds its snow: the lower, the longer it keeps it.
fn snow_bares(slope: vec2f, rise: f32, knob: f32) -> f32 {
  let sunward = clamp(dot(-slope, LIGHT.xy) / length(LIGHT.xy), -1.0, 1.0);
  return SNOW_BARES_MOST * tanh((SNOW_SUNNY * sunward + knob / SNOW_HOLLOW_M - rise / SNOW_RISE_M) / SNOW_BARES_MOST);
}

// The snow at a point whose cells hold `share` of their ground under snow and whose ground sheds it as `bares`
// (`snow_bares`), that index changing by `width` across a pixel: 1 where it lies, 0 where not, the pixel's share at
// its edge.
fn snow_at(share: f32, bares: f32, width: f32) -> f32 {
  if (share <= 0.0 || share >= 1.0) { return share; }
  let line = SNOW_SPREAD * log(share / (1.0 - share));
  return 1.0 - smoothstep(-1.0, 1.0, (bares - line) / max(width, SNOW_EDGE));
}

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
  let loose = loose_seen(at, e);
  let bare = bed_shows(loose);
  let fine = relief(at, e, length(g.slope), bare, loose);
  // The relief moves the ground's height as much as the ground drawn is steep (`RELIEF_EDGE_SLOPE`): on a flat shore a
  // bump of a few decimetres would move the water's edge by tens of metres into a fringe of puddles.
  g.h += fine.x * ease(0.0, RELIEF_EDGE_SLOPE, length(g.slope));
  g.slope += fine.yz;
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
  let st = state_on(at, g, cs, depth_in(g, ch));
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
  let fringe = ease(FRINGE_PX.x * e, FRINGE_PX.y * e, 2.0 * ch.water);
  let shore_over = select(p.over, mix(p.over, min(p.over, max(abs(g.sd - ch.middle) - ch.water, 0.0) * max(length(g.slope), ch.mean / max(ch.water, 1e-6))), fringe), ch.water > 0.0);
  let cf = cover_field(at, e, Beneath(soaked(p), shore_over, length(g.slope), loose, fwidth(loose), p.snow, st.snow_m));
  p.cover = cf.cover;
  p.canopy = cf.canopy;
  p.rock = bare;
  p.bed = vec3f(0.0);
  // Its channel's stones (`stony`), and the scree's place below the bed that sheds it: where the loose ground deepens
  // down the slope.
  // (No bed takes `BED_PX` pixels and no stone shows in a program made for draws from far: neither is read there.)
  var moved = 0.0;
  var cobble = COBBLE_M;
  if (near && ch.bed > 0.0 && BED_PX * least_e() < BED_MOST_M) {
    moved = mix(stone_of(reach), stone_of(next.s), next.share);
    cobble = max(STONE_SEEN * stone_of(reach), COBBLE_M);
  }
  let deepens = vec2f(dpdx(loose), dpdy(loose));
  if (!unseen(STONE_MOST_M / STONE_PX)) {
    p = stony(p, g, ch, q, moved, cobble);
    p.below = mix(BELOW_LEAST, 1.0, 0.5 - 0.5 * dot(deepens, g.slope) / max(length(deepens) * length(g.slope), 1e-9));
  }
  p.snow_m = st.snow_m;
  // The bed's colour where it shows or stones of it lie: a channel's, and the scree's on thin loose ground.
  if (p.rock > 0.0 || p.stony > 0.0 || loose < SCREE_LOOSE_M) { p.bed = bed_make_at(at); }
  p.sand = beach(p.sea, g.h - earth.view.w, p.over, length(g.slope), p.loose, look.z);
  p.river = 0.0;
  p.river_depth = 0.0;
  p.mouth = 0.0;
  p.at = at;
  p.e = e;
  // The depth less the film, signed (under the ground drawn: minus its height over the level, as the share of the point
  // that is the ponds' weighs it), over the depth's change across a pixel.
  let signed = st.depth - min(st.over, NO_LIQUID_M) * g.water.y - FILM_M;
  let pool = select(0.0, tent(signed / (max(length(g.slope) * g.water.y, SHORE_RISE_LEAST) * e)), g.water.y > 0.0);
  var out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), 0.0, vec3f(0.0), vec2f(0.0), vec2f(0.0));
  // A channel is drawn on the land and through the band of shallows of the liquid it runs into, no farther.
  if (ch.water <= 0.0 || mouth >= 1.0) { return out; }
  if (p.depth <= 0.0) { out.dry.over = shore_over; }
  if (!near) { return out; }
  return river_spots(out, g, ch, q, moved, grows, line_of(q, e), mouth);
}

// The colours a pixel mixes, each over the last by the share of the pixel it covers: its ground (`LAYER_GROUND`); the
// bars of a channel's bed, washed gravel, bare, rising from the water's edge (`LAYER_BARS`); where the edge of
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
      p.sand = 0.0;
      p.ground = spots.bar;
      p.rock = 0.0;
      p.cover = vec4f(0.0);
      p.over = spots.bar_over;
      p.wet = 0.0;
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
    if (running) { o = river_on(p, g, spots.flow, spots.water, spots.down); } else { o = colour_on(p, g); }
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
