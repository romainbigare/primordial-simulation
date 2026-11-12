// uses: draw.wgsl water.wgsl style.wgsl crag.wgsl stone.wgsl ice.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl river.wgsl flow.wgsl cover_field.wgsl
// least metres a pixel: 0.5 1 3 30
// The live map in view: at each pixel of the screen the ground from its tiles and the state from the cells' table
// (land.wgsl), in the view's colours. Made once more for draws whose pixels take at least 0.5 m, 1 m, 3 m and 30 m
// (web/shade, draw.wgsl `LEAST_E`): seen from afar, the detail of the ground and the plants that cannot show is
// compiled out (from 30 m, the whole island on a phone, the sea ice's floes and the pack's texture: ice.wgsl).

// A channel shows as a line of at least `LINE_PX` css pixels once its discharge passes the discharge that shows in the
// view (`land.time.w` times the metres a css pixel takes: tape/src/ground/channels.rs `SHOW_M`), as a map draws its
// rivers; wider as its discharge grows past that, as its discharge over that to the power `LINE_GROW` (more steeply
// than the channel law widens a channel: a map's choice, so the network's order reads), up to `LINE_MOST` times, so a
// stream is a hair and the main river a few times wider: no wider than its bends are drawn from as near (a tile draws
// them once the bed takes a texel or two, tape/src/ground/channels.rs `BEND_FROM`); it fades in from `LINE_FADE` times
// less, so no channel springs up as the view zooms.
const LINE_PX = 1.2;
const LINE_MOST = 4.0;
const LINE_GROW = 0.75;
const LINE_FADE = 4.0;
// A channel's bed is as wide as its floods make it (lib/rock.wgsl `banks_of`). This month's water runs in it between
// bars of washed gravel (lib/river.wgsl `reach_at`). Where the bed takes under `BED_PX` pixels it is all water.
const BED_PX = 3.0;
// A channel's bars are its bed's make washed (`BAR_LIFT` times as light, `BAR_PALE` of the way to pale sand), and sand
// where its flow moves no stones bigger than `BAR_SAND_STONE` m: gravel on most reaches, sand and silt on the
// gentlest. A bar rises from the water's edge at `BAR_RISE` (rise over run), which is how far up it the water's fringe
// wets and darkens it; past that fringe its gravel drains dry.
const BAR_LIFT = 1.7; const BAR_PALE = 0.35; const BAR_SAND_STONE = 0.03; const BAR_RISE = 0.6;
// The water within `EDGE_M` of its edge is lighter by up to `EDGE_LIGHT`: the wash at the bank, where the bed's last
// centimetres show.
const EDGE_M = 0.35; const EDGE_LIGHT = 0.25;
// Liquid standing shallower than this over the drawn ground is drawn as soaked ground, m: the drawn ground follows the
// samples within a few centimetres, so a body whose level lies that near its flat floor is a soaked basin, not water.
const FILM_M = 0.05;
// A channel's stones lie on its bed and its bars, in `IN_CHANNEL` of the places they could, fewer where it moves
// only sand (`BAR_SAND_STONE`); beside a steep reach they lie out to `STONES_OUT` of its bed's half widths from its
// line, thinning. A channel wears its stones round as it carries them: angular where it carries `WORN_Q.x` m³/s (a
// head's rill), round from `WORN_Q.y` (the travel grows with the area drained, and so does the discharge). Up a bar
// they are finer: `BAR_FINES` smaller at its top. Between two samples a stone is drawn from `STONE_SMALL.x` to
// `STONE_SMALL.y` of its sample's size.
const IN_CHANNEL = 0.6; const STONES_OUT = 1.8; const WORN_Q = vec2f(0.005, 0.3); const BAR_FINES = 0.5;
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

// What is at `at`, m from the map's north-west corner, seen at `e` m a pixel: its ground and its state (`state_on`), the
// ground's own colour and the sea blended between the squares' centres, the plants' field (lib/cover_field.wgsl); and
// its channel (rock.wgsl `channel_at`), from its own sample's discharge. The channel's bed is cut into the ground, so
// where it runs into standing liquid that liquid takes it over at its shore at the same depth. Beside it the ground is
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
  let next = next_along(reach, at);
  let q = mix(discharge_of(reach), discharge_of(next.s), next.share);
  let ch = channel_at(g, q);
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
  let shore_over = select(p.over, min(p.over, max(abs(g.sd - ch.middle) - ch.water, 0.0) * max(length(g.slope), ch.mean / max(ch.water, 1e-6))), ch.water > 0.0);
  let cf = cover_field(at, e, Beneath(soaked(p), shore_over, length(g.slope), loose, fwidth(loose), p.snow, st.snow_m));
  p.cover = cf.cover;
  p.canopy = cf.canopy;
  p.rock = bare;
  p.bed = vec3f(0.0);
  // Its channel's stones: on its bed, and beside it where the reach is steep.
  // (The largest drawn takes its size from the point's own sample, in a step where two samples meet: the size sets
  // where the stones lie. What each is of it follows the size the channel moves there, which grows evenly along it.)
  let moved = select(0.0, mix(stone_of(reach), stone_of(next.s), next.share), ch.bed > 0.0);
  p.stone = max(STONE_SEEN * stone_of(reach), COBBLE_M);
  p.small = clamp(max(STONE_SEEN * moved, COBBLE_M) / p.stone, STONE_SMALL.x, STONE_SMALL.y);
  let beside = ease(STEEP_STONE.x, STEEP_STONE.y, moved) * (1.0 - ease(ch.bed, STONES_OUT * ch.bed, g.d));
  p.stony = IN_CHANNEL * ease(0.0, BAR_SAND_STONE, moved) * select(beside, 1.0, g.d < ch.bed);
  p.worn = ease(log(WORN_Q.x), log(WORN_Q.y), log(max(q, 1e-9)));
  // The bed's colour where it shows or stones of it lie: a channel's, and the scree's on thin loose ground.
  if (p.rock > 0.0 || p.stony > 0.0 || loose < SCREE_LOOSE_M) { p.bed = bed_make_at(at); }
  p.sand = beach(p.sea, g.h - earth.view.w, p.over, length(g.slope), p.loose, look.z);
  p.river = 0.0;
  p.river_depth = 0.0;
  p.at = at;
  p.e = e;
  // The depth less the film, signed (under the ground drawn: minus its height over the level, as the share of the point
  // that is the ponds' weighs it), over the depth's change across a pixel.
  let signed = st.depth - min(st.over, NO_LIQUID_M) * g.water.y - FILM_M;
  let pool = select(0.0, tent(signed / (max(length(g.slope) * g.water.y, SHORE_RISE_LEAST) * e)), g.water.y > 0.0);
  let out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), 0.0, vec3f(0.0), vec2f(0.0), vec2f(0.0));
  if (p.depth > 0.0 || ch.water <= 0.0) { return out; }
  return river_spots(out, shore_over, g, ch, q, moved, grows);
}

// The spots `s` of a point with its channel `ch` of discharge `q`, which moves stones of `stone` m, on ground `g` (the
// place along the channel growing by `grows` a pixel east and south), the ground `shore_over` m over its water: the bed (rock.wgsl `banks_of`) and this month's water in it (lib/river.wgsl `reach_at`), apart where the bed
// takes a few pixels and is wider than the line; the bars' look, the banks' light and the water's depth and signs of
// flow.
fn river_spots(s: Spots, shore_over: f32, g: Tiled, ch: Channel, q: f32, stone: f32, grows: vec2f) -> Spots {
  var out = s;
  let at = s.dry.at;
  let e = s.dry.e;
  out.dry.over = shore_over;
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * LINE_PX * css * min(pow(q / shows, LINE_GROW), LINE_MOST) * ease(shows / LINE_FADE, shows, q);
  let open = ease(BED_PX * e, 2.0 * BED_PX * e, banks_of(q)) * (1.0 - ease(banks_of(q), 2.0 * banks_of(q), line));
  let r = reach_at(at, e, g, ch, line, open, stone);
  out.share = r.share;
  out.bars = r.bars;
  // The bars' colour: the bed's make washed, sand on the gentlest reaches; the bed seen through the water is the same.
  let washed = mix(min(bed_make_at(at) * BAR_LIFT, vec3f(1.0)), BEACH_SAND, BAR_PALE);
  let gravel = mix(BEACH_SAND, washed, ease(0.0, BAR_SAND_STONE, stone));
  out.bar = gravel * bar_tone(at, e, g.along, r);
  out.bar_over = BAR_RISE * r.edge;
  out.dry.small *= 1.0 - BAR_FINES * r.up;
  out.wet.ground = gravel;
  // The banks' light falls on the ground beside the bed, on its bars and on the water under the bank.
  let banks = mix(1.0, bank_light(e, g, ch, r), open);
  out.dry.shade *= banks;
  // Where the bed is drawn apart from the water and the water takes a few pixels, its depth shows: its pools and
  // riffles and its bed's shape across it; else its mean depth.
  let resolved = ease(e, 2.0 * e, ch.water) * open;
  let x = r.across;
  out.wet.depth = mix(ch.mean, depth_across(ch, x), resolved);
  out.flow = resolved * flow_signs(e, g.along, x, ch, r.steep);
  // Down the flow: square to the distance's slope, the way the place along the channel grows.
  let square = vec2f(-g.d_slope.y, g.d_slope.x);
  out.down = square * sign(dot(square, grows));
  out.water = vec2f(out.wet.depth / (BED_MIDDLE * ch.mean), resolved * (1.0 - ease(0.0, EDGE_M, (1.0 - abs(x)) * ch.water)));
  out.wet.sand = 0.0;
  out.wet.cover = vec4f(0.0);
  // The bed's slope across the water, d(depth)/d(distance from its middle) along that distance's own slope; where the
  // water is narrower than a pixel its banks do not show.
  let from_middle = g.d_slope * sign(g.sd) * sign(g.sd - ch.middle);
  let bank = 2.0 * BED_MIDDLE * ch.mean * ch.deep * abs(x) / ch.water * from_middle * select(0.0, 1.0, abs(x) < 1.0) * resolved;
  out.wet.slope = length(g.slope + bank) * earth.look.x;
  out.wet.shade = hill_shade(g.slope + bank, EXAGGERATE * earth.look.x) * banks;
  return out;
}

// A stream's bed sends back this share of the light soaked bare ground does: a film of algae and fine dead matter
// coats the stones under running water, and a bed sends back about 5% of the light against about 20% for soaked sand
// (Legleiter and others 2004; from memory).
const BED_FILM = 0.3;
// A streak of the flow reflects this share of the sky's light more than the water about it (a line of small ripples
// where the current shears). Fast shallow water is `FAST` of the way to `FAST_WATER`, the green-blue of water full of
// air over a pale bed.
const REFLECT_STREAK = 0.25; const FAST = 0.25; const FAST_WATER = vec3f(0.06, 0.34, 0.36);
// A stone of the bed stands out of the water where it is `STONE_STANDS` of its size taller than the water is deep;
// fast water breaks white on it and trails `WAKE` of white water down the flow behind it (stone.wgsl `wake`).
const STONE_STANDS = 0.3; const WAKE = 0.7;
// In the style the bed under running water shows this share of its light: its cobbles read through the shallows.
const BED_SHOWN = 0.8;

// The colour of a channel's water at spot `p` (`land_spots`) over its bed `g` (colour.wgsl `ground_for`), a reflectance
// in linear light: its bed, soaked bare ground (washed sand and gravel: the flow takes the plants) under its film
// (`BED_FILM`), seen through running water as deep as it is there (water.wgsl); snow lies on it as on standing liquid
// (colour.wgsl `colour_on`), and the true view shades its bed as far as it shows.
fn river_on(p: Spot, g: Ground, flow: vec3f, w: vec2f, down: vec2f) -> Shaded {
  let lit = light(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), p.slope);
  let soaked_bed = soil(g, 1.0, lit);
  let ripple = ripples(p.at, p.e) * (1.0 - p.snow);
  var water = seen_through(BED_FILM * soaked_bed, p.depth, ripple, RUNNING_ATTENUATION, DEEP_WATER);
  // In the style a channel's water is coloured by its bed's shape (style.wgsl `river_seen`): light shallows over the
  // bed's stones, a dark deep thread, a light wash at its edge.
  if (styled()) { water = mix(water_painted(BED_SHOWN * soaked_bed, river_seen(p.depth, w.x), ripple) * (1.0 + EDGE_LIGHT * w.y), FAST_WATER, FAST * flow.z); }
  // The signs of its flow (lib/flow.wgsl `flow_signs`): streaks catch the sky's light, white water breaks over its
  // steps.
  water = mix(water + flow.x * REFLECT_STREAK * SKY, FOAM_RGB, flow.y);
  // The stones that stand out of it, with their wakes.
  if (styled() && p.stony > 0.0 && !unseen(STONE_MOST_M / STONE_PX)) {
    let s = Stony(p.stony, p.stone, p.small, p.worn, BED_SUNK, 0.0);
    let least = p.depth / STONE_STANDS;
    if (flow.z > 0.0) { water = mix(water, FOAM_RGB, WAKE * flow.z * wake(p.at, p.e, s, least, down, 61 + 8)); }
    let st = stones(p.at, p.e, s, least, p.bed, 61 + 8);
    water = mix(water, st.rgb / max(st.a, 1e-4), st.a);
  }
  let rgb = mix(water, SNOW, p.snow * 0.85);
  return Shaded(rgb, 1.0 + (lit - 1.0) * exp(-2.0 * RUNNING_ATTENUATION.g * p.depth), RIVER);
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
