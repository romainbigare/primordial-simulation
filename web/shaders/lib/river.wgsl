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
const BAR_GROWN = 0.55; const BAR_TUFT_M = 1.1; const BANK_GROWN = 0.5;
// A bank is a low scarp at the bed's edge, `BANK_M` wide plus `BANK_SHARE` of the bed's half width: its face is lit
// or shaded as it is turned to the sun, by up to `BANK_SHADE` where the bend cuts it (the outer bank) and `BANK_LOW`
// of that on the low slope inside a bend.
const BANK_M = 0.4; const BANK_SHARE = 0.12; const BANK_SHADE = 0.45; const BANK_LOW = 0.25;

// What a reach builds follows its numbers by one law. Its bars bare or grown: the power its floods spend on a m² of its
// bed (specific stream power, ω = ρg Q S / w, of the bankfull flood, `land.reach.y` times its mean flow; `WATER_WEIGHT`
// N/m³) keeps them washed bare from `POWER_BARE.x` W/m² on, all of them from `POWER_BARE.y`; under it plants take the
// bed down to the water's damp edge (flood plains of low power are fine and grown, of high power coarse and stripped:
// Nanson and Croke 1992). Its threads: a gravel bed braids where its slope passes `BRAID_S` times its bankfull flood
// (m³/s) to the power `BRAID_EXP` (Leopold and Wolman 1957, in metric units), fully from `BRAID_OVER` times that slope,
// on a bed at least `BRAID_WIDE_M.x` m half wide (all of it from `.y`), and not on a reach steeper than
// `STEP_POOL_S`, where the flow drops in steps and pools between walls (Montgomery and Buffington 1997).
const WATER_WEIGHT = 9810.0; const POWER_BARE = vec2f(15.0, 90.0);
const BRAID_S = 0.0125; const BRAID_EXP = -0.44; const BRAID_OVER = 2.0; const BRAID_WIDE_M = vec2f(1.5, 3.5);
const STEP_POOL_S = vec2f(0.025, 0.045);
// A braided reach's bars rise out of its water where a noise along the flow, cells `BRAID_M` m long (the second half as
// strong) and `BRAID_ACROSS` across the water, held low near the banks, passes `BRAID_FROM` (from `.x` to `.y` as the
// reach braids more); their flanks run under the water over `BRAID_FLANK` of the noise, its depth there shallowing to
// the bar's edge, and their tops rise `BRAID_UP` of the way up to the bank.
const BRAID_M = vec2f(26.0, 9.0); const BRAID_ACROSS = 1.7; const BRAID_FROM = vec2f(0.55, 0.15); const BRAID_FLANK = 0.3;
const BRAID_UP = 0.4;
// Across the edges the eye crosses no line: where its bed is drawn apart from its water, the water thins over
// `WATER_SOFT` of its half width at its edge (the last centimetres over the bar), and the bed meets the ground beside
// it over `BED_SOFT` of its half width, the plants thinning into it.
const WATER_SOFT = 0.12; const BED_SOFT = 0.15;

// Where two channels meet, the water is the union of their waters: of the other channel near the point (live_map.wgsl
// `other_of`), its water's half width, m, the point's distance to its line, m, and the depth of its middle, m (0: none).
// Within its water, and within `JOIN_SOFT` of its half width past its edge, no bank, bar or cut of this one is drawn.
const JOIN_SOFT = 0.5;
// How far the point at `d` m from the other channel's line, its water `half` m half wide, lies in or by that water,
// 0..1 (`JOIN_SOFT`).
fn joined(half: f32, d: f32) -> f32 { return 1.0 - ease(half, (1.0 + JOIN_SOFT) * half, d); }

// The kind of a reach that carries `q` m³/s this month down slope `fall`, moving stones of `stone` m, its bed `law` m
// half wide by the channel law: how bare its bars are, 0..1 (`POWER_BARE`), and how far it braids, 0..1 (`BRAID_S`).
fn reach_kind(q: f32, fall: f32, stone: f32, law: f32) -> vec2f {
  let flood = land.reach.y * q;
  let power = WATER_WEIGHT * flood * fall / max(2.0 * law, 1e-3);
  let over = fall / (BRAID_S * pow(max(flood, 1e-6), BRAID_EXP));
  let braids = ease(1.0, BRAID_OVER, over) * (1.0 - ease(STEP_POOL_S.x, STEP_POOL_S.y, fall)) * ease(0.0, BAR_SAND_STONE, stone) * ease(BRAID_WIDE_M.x, BRAID_WIDE_M.y, law);
  return vec2f(ease(POWER_BARE.x, POWER_BARE.y, power), braids);
}

// How far a reach that braids `braids` (0..1) rises toward a bar at `along` m along it, the share `x` of the way across
// its water (`Reach.across`): over 0 on a bar, from -`BRAID_FLANK` on its flank under the water.
fn braid_bar(along: f32, x: f32, braids: f32) -> f32 {
  let n = (noise_reach(along, BRAID_M.x, BRAID_ACROSS * x, 84) + 0.5 * noise_reach(along, BRAID_M.y, 2.0 * BRAID_ACROSS * x + 5.0, 85)) / 1.5;
  return n * (1.0 - x * x) - mix(BRAID_FROM.x, BRAID_FROM.y, braids);
}

// A reach at a point: the share of the pixel its water covers, and its bars (the bed out of the water); how far across
// the water the point lies, -1 at its inner edge, 0 at its middle and 1 at its outer edge (`across`); how steep the
// reach reads, 0..1 (`steep`); how far out of the water the point lies on the bed, m (`edge`), and that as a share of
// the way to the bed's edge (`up`); how far outside the bed's edge it lies, m, negative inside (`bank`); and the share
// of its water's depth left over a braid bar's flank (`flank`, 1 off them).
struct Reach { share: f32, bars: f32, across: f32, steep: f32, edge: f32, up: f32, bank: f32, flank: f32 }

// The reach at the pixel, seen at `e` m a pixel, on ground `g` of the channel `c` (rock.wgsl `channel_at`), drawn no
// thinner than `line` m half wide (`line_of`), `open` of its bed drawn apart from its water, whose flow moves stones
// of `stone` m, of kind `kind` (`reach_kind`), beside the other channel `other` (`joined`).
fn reach_at(e: f32, g: Tiled, c: Channel, line: f32, open: f32, stone: f32, kind: vec2f, other: vec3f) -> Reach {
  var r: Reach;
  r.steep = ease(STEEP_STONE.x, STEEP_STONE.y, stone);
  r.flank = 1.0;
  var half = c.water;
  var banks = c.bed;
  // The edges' raggedness shows where the water takes a few pixels.
  if (c.water > e) {
    var edge = 0.0;
    var bank_edge = 0.0;
    // Octaves each weaker than the last, each read only where it shows.
    let coarse = shows_at(RAG_M.x, e);
    if (coarse > 0.0) { edge += coarse * noise_l(lat_turned(RAG_M.x, vec2f(0.0)), 66).x; bank_edge += coarse * noise_l(lat_turned(RAG_M.x, vec2f(11.0)), 70).x; }
    let fine = 0.5 * shows_at(RAG_M.y, e);
    if (fine > 0.0) { edge += fine * noise_l(lat_turned(RAG_M.y, vec2f(0.0)), 67).x; bank_edge += fine * noise_l(lat_turned(RAG_M.y, vec2f(11.0)), 71).x; }
    let finest = 0.3 * shows_at(RAG_M.z, e);
    if (finest > 0.0) { bank_edge += finest * noise_l(lat_turned(RAG_M.z, vec2f(0.0)), 72).x; }
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
  let real = 1.0 - ease(reach + e, reach + 2.0 * e, g.ends);
  // The bed's edge and the water's are spread over a pixel, and over `BED_SOFT` and `WATER_SOFT` of them where the bed
  // is drawn apart: the bed's inward from its edge, so it reaches no farther than a pixel past it.
  let bed_px = max(e, open * BED_SOFT * banks);
  let water_px = max(e, open * WATER_SOFT * half);
  let bed_edge = reach - (bed_px - e);
  var in_bed = (tent((bed_edge - g.d) / bed_px) - tent((-bed_edge - g.d) / bed_px)) * real;
  // The water lies in its bed: where the two edges' raggedness would put it past the bank, the bank holds it; and
  // round the bars of a braided reach (`braid_bar`), shallowing over their flanks.
  var water = min(tent((half - thread) / water_px) - tent((-half - thread) / water_px), in_bed);
  if (kind.y > 0.0 && open > 0.0 && thread < half) {
    let rise = braid_bar(g.along, r.across, kind.y);
    r.flank = 1.0 - ease(-BRAID_FLANK, 0.0, rise);
    water *= 1.0 - open * tent(rise * half / water_px);
    if (rise > 0.0) {
      r.edge = rise * half;
      r.up = BRAID_UP * ease(0.0, 1.0 - BRAID_FROM.y, rise);
    }
  }
  // The top of a bar is the plants' again, in tufts with an edge of their own: more of them the nearer the bank, and
  // down to the water's damp edge where the floods spend little power on the bed (`reach_kind`). The flow lays its
  // bars inside its bends; off them, along a straight reach and outside a bend, the plants take the bed's margin
  // `BANK_GROWN` more.
  let grown_from = BAR_GROWN * kind.x;
  let point = c.bend * select(0.0, 1.0, r.across < 0.0);
  let tufts = shows_at(BAR_TUFT_M, e);
  if (tufts > 0.0 && r.up > grown_from) {
    let grown = noise_l(lat_turned(BAR_TUFT_M, vec2f(0.0)), 79).x + 1.2 * ease(grown_from, 1.0, r.up) - 0.6 + BANK_GROWN * (1.0 - point);
    in_bed = max(in_bed * (1.0 - open * tufts * tent(grown * BAR_TUFT_M / e)), water);
  }
  // The other channel's water joins this one's: water and bed both.
  if (other.x > 0.0) {
    let theirs = tent((other.x - other.y) / water_px);
    water = max(water, theirs);
    in_bed = max(in_bed, theirs);
  }
  r.share = mix(in_bed, water, open);
  r.bars = max(in_bed - r.share, 0.0);
  return r;
}

// The tone of a bar at the pixel, seen at `e` m a pixel, `along` m along its channel, at the reach's point `r`: its mottle
// and its drift lines (each where it shows).
fn bar_tone(e: f32, along: f32, r: Reach) -> f32 {
  var t = 1.0;
  let coarse = shows_at(BAR_PATCH_M.x, e);
  if (coarse > 0.0) { t += BAR_MOTTLE * coarse * noise_l(lat_turned(BAR_PATCH_M.x, vec2f(0.0)), 76).x; }
  let fine = shows_at(BAR_PATCH_M.y, e);
  if (fine > 0.0) { t += 0.6 * BAR_MOTTLE * fine * noise_l(lat_turned(BAR_PATCH_M.y, vec2f(0.0)), 77).x; }
  let lines = shows_at(STRAND_M * STRAND_W * 3.0, e);
  if (lines > 0.0) {
    let band = fract(r.edge / STRAND_M + 0.8 * noise_reach(along, STRAND_WAVE_M, 0.0, 80));
    t *= 1.0 - STRAND * lines * (1.0 - smoothstep(0.0, STRAND_W, abs(band - 0.5))) * (1.0 - r.up);
  }
  return t;
}

// What the light on the ground at the reach's point `r` of channel `c` is times, by its bank (`BANK_M`), seen at `e`
// m a pixel on ground `g`: over 1 where the bank's face is turned to the sun, under where from it; fully on the cut
// bank outside a bend, `BANK_LOW` of that on the low banks of a straight reach and inside a bend.
fn bank_light(e: f32, g: Tiled, c: Channel, r: Reach) -> f32 {
  let wide = BANK_M + BANK_SHARE * c.bed;
  if (unseen(2.0 * (BANK_M + BANK_SHARE * BED_MOST_M))) { return 1.0; }
  let shown = shows_at(2.0 * wide, e);
  if (shown <= 0.0) { return 1.0; }
  // The bank's face looks at the channel's line, down the distance's slope.
  let facing = dot(-g.d_slope, LIGHT.xy) / length(LIGHT.xy);
  let inside = g.sd * g.lean < 0.0;
  let cut = mix(BANK_LOW, select(1.0, BANK_LOW, inside), c.bend);
  return 1.0 + BANK_SHADE * cut * shown * facing * (1.0 - ease(0.0, wide, abs(r.bank)));
}

// A bank as the flow shapes it (Leopold and Wolman 1960): on the outside of a bend the flow cuts it into a steep face
// of bare soil, as far as the place is a bend, up to `CUT_M` wide plus `CUT_SHARE` of the bed's half width out from
// the bed's edge, in stretches where the turf has slumped, as a noise along the bank over `CUT_PATCH_M` m rises from
// `CUT_FROM.x` to `.y` (none on a straight reach or inside a bend: there the turf comes down to the water); the soil
// `CUT_DARK` darker and more so under the turf's lip at its top (`LIP_DARK`), roots hanging down it in strands
// `ROOT_M` apart (`ROOT_DARK` darker); on the inside of a bend the bed rises gently into its bank, a low point bar
// (the bars, `reach_at`). Along the water's edge the ground is damp over the part of the
// bed the water's level swings across: `DAMP_M` plus `DAMP_SWING` of the bed the water leaves at its mean flow.
const CUT_M = 0.3; const CUT_SHARE = 0.15; const CUT_PATCH_M = 4.0; const CUT_FROM = vec2f(-0.4, 0.4); const CUT_DARK = 0.12;
const LIP_DARK = 0.25;
const ROOT_M = vec2f(0.35, 1.2); const ROOT_DARK = 0.3; const DAMP_M = 0.3; const DAMP_SWING = 0.6;

// The cut bank at the reach's point `r` of channel `c`, seen at `e` m a pixel on ground `g`: the share of the pixel
// its face covers, and the colour of the ground there over the soil's own (its darkening, its lip and its roots).
fn cut_bank(e: f32, g: Tiled, c: Channel, r: Reach) -> vec2f {
  let outer = select(0.0, 1.0, (g.sd - c.middle) * c.outer > 0.0);
  if (outer * c.bend <= 0.0) { return vec2f(0.0, 1.0); }
  let wide = (CUT_M + CUT_SHARE * c.bed) * c.bend * ease(CUT_FROM.x, CUT_FROM.y, noise_reach(g.along, CUT_PATCH_M, 0.0, 87));
  let shown = shows_at(wide, e);
  if (shown <= 0.0 || r.bank < -e || r.bank > wide + e) { return vec2f(0.0, 1.0); }
  let face = (tent((wide - r.bank) / e) - tent(-r.bank / e)) * shown;
  let up = clamp(r.bank / wide, 0.0, 1.0);
  var tone = (1.0 - CUT_DARK) * (1.0 - LIP_DARK * smoothstep(0.5, 1.0, up));
  let roots = shows_at(ROOT_M.x, e);
  if (roots > 0.0) { tone *= 1.0 - ROOT_DARK * roots * ease(0.35, 0.75, noise_reach(g.along, ROOT_M.x, r.bank / ROOT_M.y, 83)); }
  return vec2f(face, tone);
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
// `FRINGE_PX.x` pixels wide, all from `FRINGE_PX.y` (a rill's fringe drew a dark stroke straight down the slope). And
// a channel is drawn itself as far as its bed, or its line from afar, takes those pixels: a rill under a pixel wide
// that no line widens carries next to nothing, and drew a faint thin line through the forest.
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
// A channel's bars are the ground's own soil, and where its flow moves stones of `BAR_SAND_STONE` m `BAR_OF_BED` of the
// way to its bed's make (gravel worn from it, stained by the silt and the film the floods leave: no bar is bare
// rock's colour). A bar rises from the water's edge at `BAR_RISE` (rise over run), which is how far up it the water's fringe
// wets and darkens it; past that fringe its gravel drains dry.
const BAR_OF_BED = 0.5; const BAR_RISE = 0.6;
// Driftwood lies on a bar from `DRIFTWOOD_UP.x` of the way up it to the bank, most from `DRIFTWOOD_UP.y` (the floods' wrack
// line, at the top of the bars).
const DRIFTWOOD_UP = vec2f(0.1, 0.35);
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
// its line than its bed is half wide, all of it from `MOUTH_CALM` times that; the shore's beach and swash are not drawn
// in its bed.
const MOUTH_BARS = 0.5; const MOUTH_MELTS = 0.3; const MOUTH_CALM = 2.0;

// The spots `s` of a point with its channel `ch` of discharge `q`, which moves stones of `stone` m down a reach of
// slope `fall` (rise over run), on ground `g` (the
// place along the channel growing by `grows` a pixel east and south), drawn no thinner than `line` m half wide
// (`line_of`): the bed (rock.wgsl `banks_of`) and this month's water in it (lib/river.wgsl `reach_at`), apart where the bed
// takes a few pixels and is wider than the line; the bars' look, the banks' light and the water's depth and signs of
// flow; `mouth` of the way through the shallows of the liquid it runs into (`MOUTH_BARS`); the other channel near it
// `other` (`joined`), whose water joins its own.
fn river_spots(s: Spots, g: Tiled, ch: Channel, q: f32, stone: f32, fall: f32, grows: vec2f, line: f32, mouth: f32, other: vec3f) -> Spots {
  var out = s;
  let at = s.dry.at;
  let e = s.dry.e;
  let wide = max(ch.bed, line);
  let cuts = 1.0 - ease(MOUTH_MELTS, 1.0, mouth);
  out.dry.mouth = cuts * (1.0 - ease(wide, MOUTH_CALM * wide + 2.0 * e, g.d));
  // Outside its bed, its banks and its line from afar a channel draws nothing more.
  if (g.d > BED_OUT.x * ch.bed + BED_OUT.y + 2.0 * e && g.d > line + 2.0 * e) { return out; }
  var open = 0.0;
  // (No bed takes `BED_PX` pixels in a program made for draws from this far: rock.wgsl `BED_MOST_M`.)
  if (BED_PX * least_e() < BED_MOST_M) { open = ease(BED_PX * e, 2.0 * BED_PX * e, ch.law) * (1.0 - ease(ch.law, 2.0 * ch.law, line)); }
  let r = reach_at(e, g, ch, line, open, stone, reach_kind(q, fall, stone, ch.law), other);
  let runs = 1.0 - mouth;
  let apart = 1.0 - select(0.0, joined(other.x, other.y), other.x > 0.0);
  let seen = ease(FRINGE_PX.x * e, FRINGE_PX.y * e, 2.0 * wide);
  out.share = r.share * above_snow(s.dry.snow * s.dry.snow_m, 2.0 * max(ch.water, line)) * cuts * seen;
  // The bars' colour (`BAR_OF_BED`); the bed seen through the water is the same, soaked (colour.wgsl `soil`).
  let gravel = mix(s.dry.ground, bed_make_at(at), BAR_OF_BED * ease(0.0, BAR_SAND_STONE, stone));
  // On its bars the ground is that gravel, bare, rising from the water's edge, as far as they cover the pixel.
  let bars = r.bars * (1.0 - ease(0.0, MOUTH_BARS, mouth)) * seen;
  out.dry.ground = mix(out.dry.ground, gravel * bar_tone(e, g.along, r), bars);
  // The floods strand dead wood high on the bars, as much as woody crowns stand about the channel (the trees' share of
  // the ground of its cells, lib/cover_field.wgsl `STANDS`).
  if (bars > 0.0) { out.dry.driftwood = min(field_at(at, STANDS).a, 1.0) * bars * ease(DRIFTWOOD_UP.x, DRIFTWOOD_UP.y, r.up); }
  let bed = (r.share + r.bars) * seen;
  out.dry.sand *= 1.0 - bed;
  out.dry.swash *= 1.0 - bed;
  out.dry.rock *= 1.0 - bars;
  out.dry.cover *= 1.0 - bars;
  out.dry.canopy *= 1.0 - bars;
  out.dry.wet *= 1.0 - bars;
  // The water's edge is damp as far as its level swings (`DAMP_SWING`); past that its fringe wets the bar as it rises.
  let damp = DAMP_M + DAMP_SWING * max(ch.bed - ch.water, 0.0);
  out.dry.over = mix(out.dry.over, BAR_RISE * max(r.edge - damp, 0.0), bars);
  out.dry.small *= 1.0 - BAR_FINES * r.up;
  out.wet.ground = gravel;
  // The banks' light falls on the ground beside the bed, on its bars and on the water under the bank.
  // A cut bank's face is bare soil (`cut_bank`).
  var banks = 1.0;
  if (open > 0.0) {
    banks = mix(1.0, bank_light(e, g, ch, r), open * runs * apart);
    let cut = cut_bank(e, g, ch, r);
    let face = cut.x * open * runs * apart;
    out.dry.cover *= 1.0 - face;
    out.dry.ground = mix(out.dry.ground, out.dry.ground * cut.y, face);
  }
  out.dry.shade *= banks;
  // Where the bed is drawn apart from the water and the water takes a few pixels, its depth shows: its pools and
  // riffles and its bed's shape across it; else its mean depth.
  let resolved = ease(e, 2.0 * e, ch.water) * open;
  let x = r.across;
  out.wet.depth = mix(ch.mean, depth_across(ch, x) * r.flank, resolved);
  // In the other channel's water the depth is the deeper of the two.
  if (other.x > 0.0) { out.wet.depth = max(out.wet.depth, other.z * (1.0 - min(other.y * other.y / (other.x * other.x), 1.0))); }
  if (resolved > 0.0) {
    // (Below a step's white water a steep reach's water lies in a pool: slower, and `STEP_POOL` deeper.)
    let signs = flow_signs(e, g.along, x, ch, r.steep, fall, stone);
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
  // Water too narrow for its pools and riffles to show is as broken as a reach on the mean (river_water.wgsl).
  out.flow.x += (1.0 - resolved) * runs * ROUGH_FAR;
  out.shaped = resolved * runs;
  return out;
}
