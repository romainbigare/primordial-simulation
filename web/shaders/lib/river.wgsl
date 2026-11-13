// A channel's reach as the live map draws it (land.wgsl `river_spots`): its bed and this month's water in it, as its
// shape at the point gives them (rock.wgsl `channel_at`: the bed's width along the reach, the pool against the outer
// bank of a bend and the bar on its inside, the riffle between bends), broken at their edges by numbers fixed in the
// world, so no bank is a smooth line: ragged by `RAG_SHARE` of the water's half width, at most `RAG_MOST_M`, over noise
// `RAG_M` long (each octave fading in as it grows past a few pixels; the bed's edge, where the plants end, by the
// finest too). A steep reach is one whose flow moves stones from `STEEP_STONE` m (rock.wgsl `stone_of`).
const RAG_M = vec3f(9.0, 2.6, 0.8); const RAG_SHARE = 0.3; const RAG_MOST_M = 3.0;
const STEEP_STONE = vec2f(0.12, 0.5);
// A bar is washed gravel and sand: its tone varies by up to `BAR_MOTTLE` over patches `BAR_PATCH_M` across and finer;
// the falling water leaves drift lines along the flow every `STRAND_M` up it, `STRAND_W` of that wide and `STRAND`
// darker, wavering along the reach over `STRAND_WAVE_M`; and plants take its top from `BAR_GROWN` of the way up to the
// bank, in tufts `BAR_TUFT_M` across.
const BAR_MOTTLE = 0.2; const BAR_PATCH_M = vec2f(2.5, 0.6);
const STRAND_M = 0.9; const STRAND_W = 0.2; const STRAND = 0.28; const STRAND_WAVE_M = 8.0;
const BAR_GROWN = 0.55; const BAR_TUFT_M = 1.1;
// A bank is a low scarp at the bed's edge, `BANK_M` wide plus `BANK_SHARE` of the bed's half width: its face is lit
// or shaded as it is turned to the sun, by up to `BANK_SHADE` where the bend cuts it (the outer bank) and `BANK_LOW`
// of that on the low slope inside a bend.
const BANK_M = 0.4; const BANK_SHARE = 0.12; const BANK_SHADE = 0.45; const BANK_LOW = 0.25;

// A reach at a point: the share of the pixel its water covers, and its bars (the bed out of the water); how far across
// the water the point lies, -1 at its inner edge, 0 at its middle and 1 at its outer edge (`across`); how steep the
// reach reads, 0..1 (`steep`); how far out of the water the point lies on the bed, m (`edge`), and that as a share of
// the way to the bed's edge (`up`); and how far outside the bed's edge it lies, m, negative inside (`bank`).
struct Reach { share: f32, bars: f32, across: f32, steep: f32, edge: f32, up: f32, bank: f32 }

// The reach at `at`, m, seen at `e` m a pixel, on ground `g` of the channel `c` (rock.wgsl `channel_at`), drawn no
// thinner than `line` m half wide (`line_of`), `open` of its bed drawn apart from its water, whose flow moves stones
// of `stone` m.
fn reach_at(at: vec2f, e: f32, g: Tiled, c: Channel, line: f32, open: f32, stone: f32) -> Reach {
  var r: Reach;
  r.steep = ease(STEEP_STONE.x, STEEP_STONE.y, stone);
  var half = c.water;
  var banks = c.bed;
  // The edges' raggedness shows where the water takes a few pixels.
  if (c.water > e) {
    var edge = 0.0;
    var bank_edge = 0.0;
    // Octaves each weaker than the last, each read only where it shows.
    let coarse = shows_at(RAG_M.x, e);
    if (coarse > 0.0) { edge += coarse * noise_turned(at / RAG_M.x, 66); bank_edge += coarse * noise_turned(at / RAG_M.x + 11.0, 70); }
    let fine = 0.5 * shows_at(RAG_M.y, e);
    if (fine > 0.0) { edge += fine * noise_turned(at / RAG_M.y, 67); bank_edge += fine * noise_turned(at / RAG_M.y + 11.0, 71); }
    let finest = 0.3 * shows_at(RAG_M.z, e);
    if (finest > 0.0) { bank_edge += finest * noise_turned(at / RAG_M.z, 72); }
    half += min(RAG_SHARE * half, RAG_MOST_M) * edge;
    banks += min(RAG_SHARE * banks, RAG_MOST_M) * bank_edge;
  }
  let thread = abs(g.sd - c.middle);
  r.across = clamp((g.sd - c.middle) * c.outer / max(half, 1e-6), -1.0, 1.0);
  r.edge = max(thread - half, 0.0);
  r.bank = g.d - banks;
  r.up = clamp(r.edge / max(r.edge - r.bank, 1e-3), 0.0, 1.0);
  let reach = max(banks, line);
  // Past a line's end its texels' signed distances change sign across its extension and blend to a false 0 there: the
  // nearest texel lies far from the line itself (`Tiled.ends`), and nothing is drawn (it drew small square brackets).
  let real = 1.0 - ease(reach + e, reach + 2.0 * e, g.ends.x);
  var in_bed = (tent((reach - g.d) / e) - tent((-reach - g.d) / e)) * real;
  // The water lies in its bed: where the two edges' raggedness would put it past the bank, the bank holds it.
  let water = min(tent((half - thread) / e) - tent((-half - thread) / e), in_bed);
  // The top of a bar is the plants' again, in tufts with an edge of their own: more of them the nearer the bank.
  let tufts = shows_at(BAR_TUFT_M, e);
  if (tufts > 0.0 && r.up > BAR_GROWN) {
    let grown = noise_turned(at / BAR_TUFT_M, 79) + 1.2 * ease(BAR_GROWN, 1.0, r.up) - 0.6;
    in_bed = max(in_bed * (1.0 - open * tufts * tent(grown * BAR_TUFT_M / e)), water);
  }
  r.share = mix(in_bed, water, open);
  r.bars = max(in_bed - r.share, 0.0);
  return r;
}

// The tone of a bar at `at`, seen at `e` m a pixel, `along` m along its channel, at the reach's point `r`: its mottle
// and its drift lines (each where it shows).
fn bar_tone(at: vec2f, e: f32, along: f32, r: Reach) -> f32 {
  var t = 1.0;
  let coarse = shows_at(BAR_PATCH_M.x, e);
  if (coarse > 0.0) { t += BAR_MOTTLE * coarse * noise_turned(at / BAR_PATCH_M.x, 76); }
  let fine = shows_at(BAR_PATCH_M.y, e);
  if (fine > 0.0) { t += 0.6 * BAR_MOTTLE * fine * noise_turned(at / BAR_PATCH_M.y, 77); }
  let lines = shows_at(STRAND_M * STRAND_W * 3.0, e);
  if (lines > 0.0) {
    let band = fract(r.edge / STRAND_M + 0.8 * noise_reach(along, STRAND_WAVE_M, 0.0, 80));
    t *= 1.0 - STRAND * lines * (1.0 - smoothstep(0.0, STRAND_W, abs(band - 0.5))) * (1.0 - r.up);
  }
  return t;
}

// What the light on the ground at the reach's point `r` of channel `c` is times, by its bank (`BANK_M`), seen at `e`
// m a pixel on ground `g`: over 1 where the bank's face is turned to the sun, under where from it.
fn bank_light(e: f32, g: Tiled, c: Channel, r: Reach) -> f32 {
  let wide = BANK_M + BANK_SHARE * c.bed;
  if (unseen(2.0 * (BANK_M + BANK_SHARE * BED_MOST_M))) { return 1.0; }
  let shown = shows_at(2.0 * wide, e);
  if (shown <= 0.0) { return 1.0; }
  // The bank's face looks at the channel's line, down the distance's slope.
  let facing = dot(-g.d_slope, LIGHT.xy) / length(LIGHT.xy);
  let inside = g.sd * g.lean < 0.0;
  let cut = mix(0.6, select(1.0, BANK_LOW, inside), c.bend);
  return 1.0 + BANK_SHADE * cut * shown * facing * (1.0 - ease(0.0, wide, abs(r.bank)));
}

// A channel shows as a line of at least `land.line.x` css pixels once its discharge passes the discharge that shows in the
// view (`land.time.w` times the metres a css pixel takes: tape/src/ground/channels.rs `SHOW_M`), as a map draws its
// rivers; wider as its discharge grows past that, as its discharge over that to the power `land.line.y` (more steeply
// than the channel law widens a channel: a map's choice, so the network's order reads), up to `land.line.z` times
// (tape/src/ground/channels.rs `LINE_PX`); it fades in from `LINE_FADE` times less, so no channel springs up as the view
// zooms. The line is the least width of the channel's own bed on screen, no more: a channel is drawn as wide as it
// is wherever that is wider (width in pixels = the larger of the two), so its width changes evenly with the zoom and
// with nothing else, and a bed narrower than its line is plain water, the same water as a bed's.
const LINE_FADE = 4.0;
// A channel wets the ground beside it (its fringe) as far as a pixel shows its water: none where its water is under
// `FRINGE_PX.x` pixels wide, all from `FRINGE_PX.y` (a rill's fringe drew a dark stroke straight down the slope).
const FRINGE_PX = vec2f(0.5, 2.0);

// The least half width a channel of discharge `q` is drawn at where a pixel takes `e` m, m.
fn line_of(q: f32, e: f32) -> f32 {
  if (q <= 0.0) { return 0.0; }
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  return 0.5 * land.line.x * css * min(pow(q / shows, land.line.y), land.line.z) * ease(shows / LINE_FADE, shows, q);
}
// The half width of the widest line drawn where a pixel takes `e` m, m.
fn line_most(e: f32) -> f32 { return 0.5 * land.line.x * land.line.z * e * land.samples.w; }
// A channel's bed, its ragged edge and its banks lie within `BED_OUT.x` of its bed's half widths and `BED_OUT.y` m of
// its line.
const BED_OUT = vec2f(2.0, 1.0);
// A channel's bed is as wide as its floods make it (lib/rock.wgsl `banks_of`). This month's water runs in it between
// bars of washed gravel (lib/river.wgsl `reach_at`). Where the bed takes under `BED_PX` pixels it is all water.
const BED_PX = 3.0;
// A channel's bars are its bed's make washed (`BAR_LIFT` times as light, `BAR_PALE` of the way to pale sand), and sand
// where its flow moves no stones bigger than `BAR_SAND_STONE` m: gravel on most reaches, sand and silt on the
// gentlest. A bar rises from the water's edge at `BAR_RISE` (rise over run), which is how far up it the water's fringe
// wets and darkens it; past that fringe its gravel drains dry.
const BAR_LIFT = 1.7; const BAR_PALE = 0.35; const BAR_RISE = 0.6;
// Snow bridges a channel's water narrower than the snow is deep, as it buries a plant lower than that (draw.wgsl
// `above_snow`): a rill under a snowfield does not show.
// The water within `EDGE_M` of its edge is lighter by up to `EDGE_LIGHT`: the wash at the bank, where the bed's last
// centimetres show.
const EDGE_M = 0.35; const EDGE_LIGHT = 0.25;

// A river runs into standing liquid through its band of shallows (`mouth` of the way through it, land.wgsl
// `through_band`), where its bed lies under that liquid: its bars go under within `MOUTH_BARS` of the band, so the
// band's pale bed runs on from them; its water keeps its look and widens (rock.wgsl `MOUTH_FLARE`), its signs of flow
// fading and its bed's shape across it flattening into the open water's depth as the band's own does, and from
// `MOUTH_MELTS` of the band on it melts into that water, all of it at the band's end. The sea's foam breaks no nearer
// its line than its bed is half wide, all of it from `MOUTH_CALM` times that. A channel cuts through the shallows only
// as far as it is deeper than they are: it melts into them from where the liquid stands `MOUTH_REACH.x` times as deep
// as the channel law makes its middle, gone at `MOUTH_REACH.y` times, so a rill ends at the water's edge.
const MOUTH_BARS = 0.5; const MOUTH_MELTS = 0.5; const MOUTH_CALM = 2.0; const MOUTH_REACH = vec2f(1.0, 2.0);

// The spots `s` of a point with its channel `ch` of discharge `q`, which moves stones of `stone` m, on ground `g` (the
// place along the channel growing by `grows` a pixel east and south), drawn no thinner than `line` m half wide
// (`line_of`): the bed (rock.wgsl `banks_of`) and this month's water in it (lib/river.wgsl `reach_at`), apart where the bed
// takes a few pixels and is wider than the line; the bars' look, the banks' light and the water's depth and signs of
// flow; `mouth` of the way through the shallows of the liquid it runs into (`MOUTH_BARS`).
fn river_spots(s: Spots, g: Tiled, ch: Channel, q: f32, stone: f32, grows: vec2f, line: f32, mouth: f32) -> Spots {
  var out = s;
  let at = s.dry.at;
  let e = s.dry.e;
  let wide = max(ch.bed, line);
  let middle = BED_MIDDLE * ch.mean * (1.0 + MOUTH_FLARE * mouth);
  let cuts = (1.0 - ease(MOUTH_REACH.x * middle, MOUTH_REACH.y * middle, standing(g))) * (1.0 - ease(MOUTH_MELTS, 1.0, mouth));
  out.dry.mouth = cuts * (1.0 - ease(wide, MOUTH_CALM * wide + 2.0 * e, g.d));
  // Outside its bed, its banks and its line from afar a channel draws nothing more.
  if (g.d > BED_OUT.x * ch.bed + BED_OUT.y + 2.0 * e && g.d > line + 2.0 * e) { return out; }
  var open = 0.0;
  // (No bed takes `BED_PX` pixels in a program made for draws from this far: rock.wgsl `BED_MOST_M`.)
  if (BED_PX * least_e() < BED_MOST_M) { open = ease(BED_PX * e, 2.0 * BED_PX * e, ch.law) * (1.0 - ease(ch.law, 2.0 * ch.law, line)); }
  let r = reach_at(at, e, g, ch, line, open, stone);
  let runs = 1.0 - mouth;
  out.share = r.share * above_snow(s.dry.snow * s.dry.snow_m, 2.0 * max(ch.water, line)) * cuts;
  out.bars = r.bars * (1.0 - ease(0.0, MOUTH_BARS, mouth));
  // The bars' colour: the bed's make washed, sand on the gentlest reaches; the bed seen through the water is the same.
  let washed = mix(min(bed_make_at(at) * BAR_LIFT, vec3f(1.0)), BEACH_SAND, BAR_PALE);
  let gravel = mix(BEACH_SAND, washed, ease(0.0, BAR_SAND_STONE, stone));
  out.bar = gravel * bar_tone(at, e, g.along, r);
  out.bar_over = BAR_RISE * r.edge;
  out.dry.small *= 1.0 - BAR_FINES * r.up;
  out.wet.ground = gravel;
  // The banks' light falls on the ground beside the bed, on its bars and on the water under the bank.
  var banks = 1.0;
  if (open > 0.0) { banks = mix(1.0, bank_light(e, g, ch, r), open * runs); }
  out.dry.shade *= banks;
  // Where the bed is drawn apart from the water and the water takes a few pixels, its depth shows: its pools and
  // riffles and its bed's shape across it; else its mean depth.
  let resolved = ease(e, 2.0 * e, ch.water) * open;
  let x = r.across;
  out.wet.depth = mix(ch.mean, depth_across(ch, x), resolved);
  if (resolved > 0.0) {
    // (Below a step's white water a steep reach's water lies in a pool: slower, and `STEP_POOL` deeper.)
    let signs = flow_signs(e, g.along, x, ch, r.steep);
    out.wet.depth *= 1.0 + STEP_POOL * resolved * (r.steep - signs.z);
    out.flow = resolved * runs * signs;
  }
  // Down the flow: square to the distance's slope, the way the place along the channel grows.
  let square = vec2f(-g.d_slope.y, g.d_slope.x);
  out.down = square * sign(dot(square, grows));
  // (A channel too narrow for its bed's shape to show is coloured as the river's deep water.)
  out.water = vec2f(mix(1.0, out.wet.depth / (BED_MIDDLE * ch.mean), resolved * runs), resolved * runs * (1.0 - ease(0.0, EDGE_M, (1.0 - abs(x)) * ch.water)));
  out.wet.stony *= runs;
  out.wet.blocks *= runs;
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
    let s = Stony(p.stony, p.stone, p.small, p.worn, BED_SUNK, 0.0, 0.0, 0.0);
    let least = p.depth / STONE_STANDS;
    if (flow.z > 0.0) { water = mix(water, FOAM_RGB, WAKE * flow.z * wake(p.at, p.e, s, least, down, 61 + 8)); }
    let st = stones(p.at, p.e, s, least, p.bed, 61 + 8);
    water = mix(water, st.rgb / max(st.a, 1e-4), st.a);
    // The blocks of a steep reach: the scree's own (colour.wgsl `ground_of`), where they stand out of the water.
    if (p.blocks > 0.0) {
      let fallen = Stony(p.blocks, SCREE_STONE_M, 1.0, 0.0, BED_SUNK, 0.0, 0.0, STONE_RARE);
      water = mix(water, FOAM_RGB, WAKE * flow.z * wake(p.at, p.e, fallen, least, down, 61));
      let block = stones(p.at, p.e, fallen, least, p.bed, 61);
      water = mix(water, block.rgb / max(block.a, 1e-4), block.a);
    }
  }
  let rgb = mix(water, SNOW, p.snow * 0.85);
  return Shaded(rgb, 1.0 + (lit - 1.0) * exp(-2.0 * RUNNING_ATTENUATION.g * p.depth), RIVER);
}
