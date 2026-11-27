// The rock and the channels of the live map (web/src/land.ts, tape/src/ground/rock.rs): of each of the terrain's
// samples the depth of its loose ground, the largest stone its channel moves, its reach's slope and its channel's links; how much of the
// bed shows where the loose ground thins; and a channel at a point: what runs through it and the shape of its bed and
// of its water (`channel_at`), from the place along its line and its swing from its line without bends that the tiles
// hold (land.wgsl `Tiled`).

// Of each sample (a texel each): the depth of its loose ground, m, the largest stone its channel moves, m, the slope of
// its channel's reach (rise over run), and its channel's links (`links_of`); in a second plane under the first, the
// reflectance of its bed's make; and in a third, its loose ground's shares of coarse and fine grain, how slowly it
// passes liquid (`soil_at`) and the liquid it holds for roots as the layer `avail` shows it; and in a fourth, the
// slope up from it to the highest ground toward the light (`horizon_at`), a zero, how deep the load the liquid laid
// lies on it, m, and the
// decimal logarithm of the years its loose ground has lain (tape/src/ground/rock.rs).
@group(0) @binding(18) var rock_field: texture_2d<f32>;
@group(0) @binding(19) var rock_sampler: sampler;

// Loose ground hides the bed from this depth on, m, and shows all of it at none: soils thinner than a few decimetres
// show their rock in outcrops (lithic soils are under 0.1 m deep; from memory).
const HIDES_BED_M = 0.1;
// The loose ground's depth departs from the samples' by up to this share over patches from `PATCHY_M` across down, in
// `PATCHY_OCTAVES` octaves (ragged.wgsl): so the bed shows in outcrops with ragged edges, not along a smooth line.
const PATCHY = 0.6; const PATCHY_M = 40.0; const PATCHY_OCTAVES = 4;
// Loose ground deeper than this, m, creeps into smooth swells; thinner, the bed's roughness shows through it
// (relief.wgsl).
const SMOOTH_LOOSE_M = 0.5;
// The patches fade out as the thinnest they make deepens from `PATCHES_FADE_M` to where it no longer shows the bed or
// its roughness (`HIDES_BED_M`, `SMOOTH_LOOSE_M`), past which they are none: so no line shows where they end on what
// reads deeper loose ground (a soil's colour, scree, roots).
const PATCHES_FADE_M = 0.25;

// The rock's planes (`rock_field`): the loose ground's and its channel's, the bed's make, the loose ground's grain, its
// deposit.
const LOOSE_PLANE = 0; const MAKE_PLANE = 1; const GRAIN_PLANE = 2; const LAID_PLANE = 3; const ROCK_PLANES = 4.0;

// Plane `plane` of the samples at `at`, m from the map's north-west corner: through the samples' centres as a
// B-spline read texel by texel (draw.wgsl `b_spline`), so no sample's square shows.
fn samples_at(at: vec2f, plane: i32) -> vec4f {
  let c = b_spline(at / (land.map.x * land.map.yz) * land.samples.xy - 0.5);
  let last = vec2i(land.samples.xy) - 1;
  let rows = vec2i(0, plane * i32(land.samples.y));
  var sum = vec4f(0.0);
  for (var n = 0; n < B_TAPS; n++) { sum += b_weight(c, n) * textureLoad(rock_field, clamp(b_texel(c, n), vec2i(0), last) + rows, 0); }
  return sum;
}

// The slope up from `at` to the highest ground toward the light, rise over run (the rock's deposit plane holds each
// sample's, tape/src/ground/horizon.rs; `samples_at`: no sample's square shows in the shade the heights cast).
fn horizon_at(at: vec2f) -> f32 { return samples_at(at, LAID_PLANE).x; }

// The depth of the loose ground at `at`, m (`samples_at`): no sample's square shows in the bed it bares and its change
// across a pixel, which eases the plants' edge (`rooted`), is smooth.
fn loose_at(at: vec2f) -> f32 { return samples_at(at, LOOSE_PLANE).x; }

// The place in the rock's texture of the point `uv` (0..1 over the samples) of its plane `plane` (`MAKE_PLANE`...),
// held within the plane's own rows so no blend reads another plane.
fn rock_plane(uv: vec2f, plane: i32) -> vec2f {
  let down = land.samples.y;
  return vec2f(uv.x, (clamp(uv.y * down, 0.5, down - 0.5) + f32(plane) * down) / (ROCK_PLANES * down));
}

// The loose ground's grain shares swap coarse for fine by up to `GRAIN_PATCHY` of themselves either way over patches from
// `GRAIN_PATCH_M` across down, in `GRAIN_OCTAVES` octaves (ragged.wgsl): what the wear lays down is sorted in lenses
// and patches finer than a sample. Even in the patches' noise, so the patches' mean is the samples' grain. The samples'
// grain is read `GRAIN_WARP_M` away at most, along noise `GRAIN_WARP_SIZE_M` across: a sample's grain is a mean over
// its square, and its edges are no lines on the ground, so no square of the samples shows where the grain changes
// fast.
const GRAIN_PATCHY = 0.6; const GRAIN_PATCH_M = 60.0; const GRAIN_OCTAVES = 3;
const GRAIN_WARP_M = 110.0; const GRAIN_WARP_SIZE_M = 260.0;
// The place the samples' soil is read for `at`: up to `GRAIN_WARP_M` away.
fn soil_place(at: vec2f) -> vec2f {
  return at + GRAIN_WARP_M * vec2f(noise_turned(at / GRAIN_WARP_SIZE_M, 214), noise_turned(at / GRAIN_WARP_SIZE_M + 3.7, 215));
}
// Plane `plane` of the samples' soil at `at` (`samples_at`, at `soil_place`).
fn soil_samples_at(at: vec2f, plane: i32) -> vec4f { return samples_at(soil_place(at), plane); }

// The loose ground at a point: its grain as `Spot.grain` holds it, and its deposit as `Spot.deposit` does.
struct SoilAt { grain: vec4f, deposit: vec2f }
// The loose ground at `at`, seen at `e` m a pixel, with its organic matter `organic` (the cells', which changes as the
// dead matter on the ground rots): the samples' grain (`samples_at`: the shares of coarse and fine grain and how
// slowly it passes liquid) moved by `shift`, the cells' grain (`World::soil`, what the laws read) over the samples' in
// their square (web/src/land.ts), so the world's soil is drawn on the mean and the samples' pattern inside a square,
// with its patches; and how deep the load the liquid laid lies there, m, and the decimal
// logarithm of the years its loose ground has lain (`LAID_PLANE`, `rock_blended`: a deposit's edge is eased where
// it is drawn, colour.wgsl `laid_share`). Both read at one place (`soil_place`).
fn soil_at(at: vec2f, e: f32, organic: f32, shift: vec2f) -> SoilAt {
  let place = soil_place(at);
  let raw = samples_at(place, GRAIN_PLANE);
  let g = vec4f(clamp(raw.xy + shift, vec2f(0.0), vec2f(1.0)), raw.zw);
  let n = GRAIN_PATCHY * ragged(at, e, GRAIN_PATCH_M, GRAIN_OCTAVES, 210);
  let laid = rock_blended(place, LAID_PLANE).zw;
  return SoilAt(vec4f(g.x * (1.0 + n), g.y * (1.0 - n), organic, g.z), laid);
}

// Plane `plane` of the samples at `at`, m from the map's north-west corner, blended between the samples' centres (one
// filtered read).
fn rock_blended(at: vec2f, plane: i32) -> vec4f {
  return textureSampleLevel(rock_field, rock_sampler, rock_plane(at / (land.map.x * land.map.yz), plane), 0.0);
}

// The reflectance of the bed's make at `at`, m from the map's north-west corner (`rock_blended`): so bare rock shows
// the bands the folded layers make (core/src/space/strata.rs).
fn bed_make_at(at: vec2f) -> vec3f { return rock_blended(at, MAKE_PLANE).xyz; }

// Of the channel of sample `s`: the largest stone it moves, m, and the slope of its reach, rise over run (both 0 for
// none, `s.x < 0`).
fn stone_and_fall(s: vec2i) -> vec2f {
  if (s.x < 0) { return vec2f(0.0); }
  return textureLoad(rock_field, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).yz;
}

// The wavelength of the bends of the channel of sample `s`, m (tape/src/ground/rock.rs: its make's plane; 0 for none,
// `s.x < 0`).
fn bend_length_of(s: vec2i) -> f32 {
  if (s.x < 0) { return 0.0; }
  return textureLoad(rock_field, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1) + vec2i(0, MAKE_PLANE * i32(land.samples.y)), 0).w;
}

// A sample's channel's links are one number (tape/src/ground/rock.rs `links`): the step to the sample it runs to (1 to
// 8, its neighbours row by row without itself; 8 more where it joins that one as a tributary; 0: none), and `LINK_UP`
// times the step to its main neighbour upstream.
const LINK_UP = 17;
// The links of the channel of sample `s`.
fn links_of(s: vec2i) -> i32 {
  return i32(textureLoad(rock_field, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).w + 0.5);
}

// The depth of the loose ground at `at`, m, seen at `e` m a pixel, with its patches (`PATCHY`, ragged.wgsl `ragged`)
// about its depth there without them, `mean` (`loose_at`): the field every edge of bare bed is drawn from, the
// ground's and the plants' alike.
fn loose_seen(at: vec2f, mean: f32, e: f32) -> f32 {
  // (Where they are none no noise is read.)
  let share = patches_of(mean);
  if (share <= 0.0) { return mean; }
  return with_patches(mean, ragged(at, e, PATCHY_M, PATCHY_OCTAVES, 55), share);
}

// How far the patches move loose ground `mean` m deep on the mean, as a share of it (`PATCHY`, `PATCHES_FADE_M`).
fn patches_of(mean: f32) -> f32 { return PATCHY * (1.0 - ease(PATCHES_FADE_M, max(HIDES_BED_M, SMOOTH_LOOSE_M), mean * (1.0 - PATCHY))); }

// How far the loose ground's depth swings, m, about its depth `mean` at a point (`loose_at`), over the patches
// `loose_seen` does not show at `e` m a pixel: a rule on the depth read there is spread over this much, so it holds
// on the mean where the near picture's patches would make it (the cover's roots, cover_field.wgsl `cover_field`).
fn loose_unseen(mean: f32, e: f32) -> f32 { return mean * patches_of(mean) * ragged_hidden(e, PATCHY_M, PATCHY_OCTAVES); }

// The share of the bed that shows through `loose` m of loose ground (`loose_seen`).
fn bed_shows(loose: f32) -> f32 { return 1.0 - ease(0.0, HIDES_BED_M, loose); }

// What runs through sample `s` (-1: none), m³/s.
fn discharge_of(s: vec2i) -> f32 {
  if (s.x < 0) { return 0.0; }
  return textureLoad(flows, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).x;
}
// What the channel's liquid of sample `s` (-1: none) carries, g/m³.
fn load_of(s: vec2i) -> f32 {
  if (s.x < 0) { return 0.0; }
  return textureLoad(flows, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).y;
}
// What the standing liquid at `at`, m from the map's north-west corner, carries, g/m³: the plumes' load of the samples
// (web/src/land.ts) through their centres as a B-spline (draw.wgsl `b_spline`), so no sample's square shows in a
// plume. A channel's load is not read here: it would spread from the samples of the shore into the body.
fn plume_at(at: vec2f) -> f32 {
  let c = b_spline(at / (land.map.x * land.map.yz) * land.samples.xy - 0.5);
  let last = vec2i(land.samples.xy) - 1;
  var sum = 0.0;
  for (var n = 0; n < B_TAPS; n++) { sum += b_weight(c, n) * textureLoad(flows, clamp(b_texel(c, n), vec2i(0), last), 0).w; }
  return sum;
}

// The sample next along the channel of sample `s` from the point `at` (m from the map's north-west corner), and the
// share of the way from `s`'s centre to that one's the point lies (0: none): a channel's numbers grow evenly from one
// sample's centre to the next one's along it (`links_of`), so its width has no step where two samples meet; a
// tributary keeps its own to its join.
struct Next { s: vec2i, share: f32 }
fn next_along(s: vec2i, at: vec2f) -> Next {
  var next = Next(s, 0.0);
  if (s.x < 0) { return next; }
  let links = links_of(s);
  let p = at / (land.map.x * exp2(land.map.w - land.samples.z)) - (vec2f(s) + 0.5);
  for (var k = 0; k < 2; k++) {
    // Down the channel (not into the channel a tributary joins), then up its main way.
    let code = select(links / LINK_UP, links % LINK_UP, k == 0);
    if (code <= 0 || code > 8) { continue; }
    let n = select(code, code - 1, code <= 4);
    let step = vec2i(n % 3, n / 3) - 1;
    let a = min(dot(p, vec2f(step)) / f32(dot(step, step)), 1.0);
    if (a > next.share) { next = Next(s + step, a); }
  }
  return next;
}

// The sample of the channel at a point of ground `g`: its nearest texel's channel's, or the channel's of the texel
// across from that where it carries more and that texel lies in its water, so a pixel in the water of a channel a
// tributary joins is that channel's.
fn sample_at(g: Tiled) -> vec2i {
  let across = discharge_of(g.channels.zw);
  if (across > discharge_of(g.channels.xy) && g.ends.y < 0.5 * land.channel.x * pow(across, land.channel.y)) { return g.channels.zw; }
  return g.channels.xy;
}

// What runs through the channel at a point `at` of ground `g`, m³/s (`sample_at`, `next_along`).
fn discharge_at(g: Tiled, at: vec2f) -> f32 {
  let s = sample_at(g);
  let n = next_along(s, at);
  return mix(discharge_of(s), discharge_of(n.s), n.share);
}

// A channel's bed is as wide as the flow that shapes it, its floods: the bankfull discharge, the flood of every year or
// two, `land.reach.y` times the mean flow (tape/src/ground/bends.rs `FLOOD`). The half width of the bed of a channel
// that carries `q` m³/s this month, m: the channel law's (flow/channel.rs) at that many times it.
fn banks_of(q: f32) -> f32 { return 0.5 * land.channel.x * pow(land.reach.y * q, land.channel.y); }

// At one place a channel's water grows wider with its discharge far less than from channel to channel (tape/src/ground/
// bends.rs `STATION_EXP`): at its mean flow the water is `land.reach.w` of the bed its floods cut wide.
// A bed's width varies along it by up to `BED_VARY` either way, over lengths of `BED_VARY_M` (the second half as
// strong), each a whole share of the length the place along a channel is kept modulo. (Farther from the line than
// `BED_NEAR` of the bed's half widths the width is the law's: nothing drawn there tells the two apart, and no noise is
// read.)
const BED_VARY = 0.22; const BED_VARY_M = vec2f(32.0, 8.0); const BED_NEAR = 4.0;
// Where a program is made for draws seen from afar (draw.wgsl `LEAST_E`), a bed's half width is taken as no more than
// this, m (the bed of a river of some 300 m³/s): what of a bed that wide cannot show at the program's pixels is
// compiled out of it.
const BED_MOST_M = 60.0;
// A bend shows from where the line has swung `BEND_OFF.x` of the bed's half width from its line without bends
// (`Tiled.lean`), and is full from `BEND_OFF.y`: at its apex. There the water is a pool against the outer bank,
// `POOL_NARROW` narrower, over a bar on the inside (Leopold and Wolman 1960); where the line crosses over between two
// bends it is a riffle, `RIFFLE_WIDE` wider and shallower, across the whole bed.
const BEND_OFF = vec2f(0.5, 3.0); const POOL_NARROW = 0.2; const RIFFLE_WIDE = 0.3;
// The water lies against the outer bank from this much of a full bend on.
const POOL_AT_BANK = 0.6;
// In a full bend the pool's deepest lies toward the outer bank: across the water the bed's parabola leans by this much
// (its depth times 1 + `POOL_SKEW` × the share of the way to the outer edge).
const POOL_SKEW = 0.8;

// Value noise over a channel's reach, of stream `k`, -1..1: at `along` m along its line, in cells about `cell` m long
// (as many whole ones as fit nearest in the length the place along is kept modulo, each stretched to fill it, so it
// comes again with no break where the place wraps), and `across` cells across it.
fn noise_reach(along: f32, cell: f32, across: f32, k: i32) -> f32 {
  let n = i32(max(round(land.reach.x / cell), 1.0));
  let p = vec2f(along * f32(n) / land.reach.x, across);
  let i = vec2i(floor(p));
  let f = p - floor(p);
  let u = f * f * (3.0 - 2.0 * f);
  let x = vec2i(((i.x % n) + n) % n, (((i.x + 1) % n) + n) % n);
  let v = vec4f(f32(lowbias(vec2i(x.x, i.y), k)), f32(lowbias(vec2i(x.y, i.y), k)), f32(lowbias(vec2i(x.x, i.y + 1), k)), f32(lowbias(vec2i(x.y, i.y + 1), k))) * (2.0 / 4294967296.0) - 1.0;
  return mix(mix(v.x, v.y, u.x), mix(v.z, v.w, u.x), u.y);
}

// A channel widens into the standing liquid it runs into: its bed and its water grow evenly to `MOUTH_FLARE` wider (and
// its water that much shallower) across that liquid's band of shallows (land.wgsl `through_band`), none of it on the
// land. A channel that moves no stones of `BAR_SAND_STONE` m carries sand and silt alone.
const MOUTH_FLARE = 1.5;
const BAR_SAND_STONE = 0.03;

// A channel at a point of ground `g`, of this month's discharge `q`, m³/s: the half width of its bed on the point's
// side of its line, m (`banks_of`, varied along it); the half width of this month's
// water, m, and where its middle lies across the line (as `Tiled.sd`), m: against the outer bank in a bend; its mean
// depth by the channel law, m, and what the depth there is of the law's (a pool is deeper, a riffle shallower: the same
// flow through a narrower or a wider water); how much of a bend the place is, 0..1; and the side its outer bank lies
// on (the sign of `Tiled.sd` there); and the half width the channel law gives its bed, m (`banks_of`).
struct Channel { bed: f32, water: f32, middle: f32, mean: f32, deep: f32, bend: f32, outer: f32, law: f32 }
fn channel_at(g: Tiled, q: f32) -> Channel { return channel_of(g, q, banks_of(q), through_band(g, 0.0)); }
// The height of ground `g` over the nearest water, standing or running, m: over the standing liquid's level `over`
// (land.wgsl `over_ponds`), or over channel `ch`'s water as its banks rise from its edge, as far as the channel shows
// (`shown`, 0..1).
fn over_water(g: Tiled, ch: Channel, over: f32, shown: f32) -> f32 {
  if (ch.water <= 0.0) { return over; }
  return mix(over, min(over, max(abs(g.sd - ch.middle) - ch.water, 0.0) * max(length(g.slope), ch.mean / max(ch.water, 1e-6))), shown);
}
// (The same, of a channel whose bed the law makes `banks` m half wide, `mouth` of the way through the band of shallows
// of the liquid it runs into.)
fn channel_of(g: Tiled, q: f32, banks: f32, mouth: f32) -> Channel {
  if (q <= 0.0) { return Channel(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0); }
  let flare = 1.0 + MOUTH_FLARE * mouth;
  let law = banks * flare;
  var vary = 1.0;
  if (!unseen(BED_VARY * BED_MOST_M) && g.d < BED_NEAR * law) { vary += BED_VARY * (noise_reach(g.along, BED_VARY_M.x, 0.0, 68) + 0.5 * noise_reach(g.along, BED_VARY_M.y, 0.0, 69)) / 1.5; }
  let bend = ease(BEND_OFF.x * law, BEND_OFF.y * law, abs(g.lean));
  let outer = select(1.0, -1.0, g.lean < 0.0);
  let bed = law * vary;
  let wide = mix(1.0 + RIFFLE_WIDE, 1.0 - POOL_NARROW, bend);
  let water = min(law * vary * land.reach.w * wide, law * vary);
  let middle = outer * ease(0.0, POOL_AT_BANK, bend) * (law * vary - water);
  return Channel(bed, water, middle, land.channel.z * pow(q, land.channel.w) / flare, 1.0 / wide, bend, outer, banks);
}

// The depth of the bed of channel `c` under its water's surface, m, the share `x` of the way across the water from its
// middle to its outer edge (-1 at its inner edge): a parabola across the water, as deep at its middle as its mean
// depth makes it (`BED_MIDDLE`), leaning to the outer bank in a bend (`POOL_SKEW`), so the water is deepest in its
// thread and its banks catch the light.
fn depth_across(c: Channel, x: f32) -> f32 {
  return BED_MIDDLE * c.mean * c.deep * (1.0 - x * x) * (1.0 + POOL_SKEW * c.bend * x);
}

// The depth of the bed of channel `c` under its water's surface at the point of ground `g`, m (`depth_across`): none
// where the point is standing liquid's (the share of it that is: a body's floor is its own, and no channel is cut
// into it).
fn depth_in(g: Tiled, c: Channel) -> f32 {
  return depth_across(c, clamp((g.sd - c.middle) * c.outer / max(c.water, 1e-6), -1.0, 1.0)) * (1.0 - g.water.y);
}
