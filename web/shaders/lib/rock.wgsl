// The rock and the channels of the live map (web/src/land.ts, tape/src/ground/rock.rs): of each of the terrain's
// samples the depth of its loose ground, the largest stone its channel moves and its channel's links; how much of the
// bed shows where the loose ground thins; and a channel at a point: what runs through it and the shape of its bed and
// of its water (`channel_at`), from the place along its line and the line's curvature that the tiles hold (land.wgsl
// `Tiled`).

// Of each sample (a texel each): the depth of its loose ground, m, the largest stone its channel moves, m, and its
// channel's links (`links_of`); and in a second plane under the first, the reflectance of its bed's make
// (tape/src/ground/rock.rs).
@group(0) @binding(18) var rock_field: texture_2d<f32>;
@group(0) @binding(19) var rock_sampler: sampler;

// Loose ground hides the bed from this depth on, m, and shows all of it at none: soils thinner than a few decimetres
// show their rock in outcrops (lithic soils are under 0.1 m deep; from memory).
const HIDES_BED_M = 0.1;
// The loose ground's depth departs from the samples' by up to this share over patches `PATCHY_M` across and in
// `PATCHY_OCTAVES` octaves each `PATCHY_STEP` times finer, each weighing `PATCHY_FALL` of the last and fading out as it
// shrinks to a few pixels (draw.wgsl `shows_at`): a mantle is thicker in hollows and thinner on knobs at every
// scale, so the bed shows in outcrops with ragged edges, not along a smooth line.
const PATCHY = 0.6; const PATCHY_M = 40.0; const PATCHY_OCTAVES = 4; const PATCHY_STEP = 2.9; const PATCHY_FALL = 0.6;
// Loose ground deeper than this, m, creeps into smooth swells; thinner, the bed's roughness shows through it
// (relief.wgsl).
const SMOOTH_LOOSE_M = 0.5;

// The depth of the loose ground at `at`, m from the map's north-west corner: through the samples' centres as a cubic
// B-spline (draw.wgsl `bspline`), so no sample's square shows in the bed it bares.
fn loose_at(at: vec2f) -> f32 {
  let size = land.samples.xy;
  let b = bspline(at / (land.map.x * land.map.yz) * size - 0.5, size);
  let tap = vec4f(
    textureSampleLevel(rock_field, rock_sampler, rock_plane(b.h0, 0.0), 0.0).x, textureSampleLevel(rock_field, rock_sampler, rock_plane(vec2f(b.h1.x, b.h0.y), 0.0), 0.0).x,
    textureSampleLevel(rock_field, rock_sampler, rock_plane(vec2f(b.h0.x, b.h1.y), 0.0), 0.0).x, textureSampleLevel(rock_field, rock_sampler, rock_plane(b.h1, 0.0), 0.0).x);
  return b.g0.y * (b.g0.x * tap.x + b.g1.x * tap.y) + b.g1.y * (b.g0.x * tap.z + b.g1.x * tap.w);
}

// The place in the rock's texture of the point `uv` (0..1 over the samples) of its plane `plane` (0 the loose ground's,
// 1 the bed's make), held within the plane's own rows so no blend reads the other plane.
fn rock_plane(uv: vec2f, plane: f32) -> vec2f {
  let down = land.samples.y;
  return vec2f(uv.x, (clamp(uv.y * down, 0.5, down - 0.5) + plane * down) / (2.0 * down));
}

// The reflectance of the bed's make at `at`, m from the map's north-west corner, blended between the samples' centres:
// so bare rock shows the bands the folded layers make (core/src/space/strata.rs).
fn bed_make_at(at: vec2f) -> vec3f {
  return textureSampleLevel(rock_field, rock_sampler, rock_plane(at / (land.map.x * land.map.yz), 1.0), 0.0).xyz;
}

// The largest stone the channel of sample `s` moves, m (0 for none, `s.x < 0`).
fn stone_of(s: vec2i) -> f32 {
  if (s.x < 0) { return 0.0; }
  return textureLoad(rock_field, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).y;
}

// A sample's channel's links are one number (tape/src/ground/rock.rs `links`): the step to the sample it runs to (1 to
// 8, its neighbours row by row without itself; 8 more where it joins that one as a tributary; 0: none), and `LINK_UP`
// times the step to its main neighbour upstream.
const LINK_UP = 17;
// The links of the channel of sample `s`.
fn links_of(s: vec2i) -> i32 {
  return i32(textureLoad(rock_field, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).w + 0.5);
}

// The depth of the loose ground at `at`, m, seen at `e` m a pixel, with its patches (`PATCHY`): the field every edge of
// bare bed is drawn from, the ground's and the plants' alike.
fn loose_seen(at: vec2f, e: f32) -> f32 {
  let mean = loose_at(at);
  // Where the thinnest the patches make it is deeper than shows the bed or its roughness, they change nothing drawn
  // (and no noise is read).
  if (mean * (1.0 - PATCHY) >= max(HIDES_BED_M, SMOOTH_LOOSE_M)) { return mean; }
  var n = 0.0;
  var size = PATCHY_M;
  var weight = 1.0;
  var sum = 0.0;
  for (var o = 0; o < PATCHY_OCTAVES; o++) {
    let shown = shows_at(size, e);
    // An octave too small to show reads no noise.
    if (shown > 0.0) { n += weight * shown * noise_turned(at / size + f32(o) * 7.3, 55 + o); }
    sum += weight;
    size /= PATCHY_STEP;
    weight *= PATCHY_FALL;
  }
  return mean * (1.0 + PATCHY * clamp(2.0 * n / sum, -1.0, 1.0));
}

// The share of the bed that shows through `loose` m of loose ground (`loose_seen`).
fn bed_shows(loose: f32) -> f32 { return 1.0 - ease(0.0, HIDES_BED_M, loose); }

// What runs through sample `s` (-1: none), m³/s.
fn discharge_of(s: vec2i) -> f32 {
  if (s.x < 0) { return 0.0; }
  return textureLoad(flows, clamp(s, vec2i(0), vec2i(land.samples.xy) - 1), 0).x;
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

// At one place a channel's water grows wider with its discharge as its power `STATION_EXP` (at-a-station hydraulic
// geometry, Leopold and Maddock 1953: 0.26), far less than from channel to channel: at its mean flow the water fills
// most of the bed its floods cut.
const STATION_EXP = 0.26;
// A bed's width varies along it by up to `BED_VARY` either way, over lengths of `BED_VARY_M` (the second half as
// strong), each a whole share of the length the place along a channel is kept modulo.
const BED_VARY = 0.22; const BED_VARY_M = vec2f(32.0, 8.0);
// On the inside of a bend a bed reaches no farther from its line than this share of the radius the line turns on (its
// banks would meet past the bend's centre).
const INNER_MOST = 0.8;
// A bend is full where the line's curvature times the bed's width passes `BEND_FULL` (a bed turning on a radius of 2.5
// of its widths). There the water is a pool against the outer bank, `POOL_NARROW` narrower, over a bar on the inside
// (Leopold and Wolman 1960); between bends it is a riffle, `RIFFLE_WIDE` wider and shallower, across the whole bed.
const BEND_FULL = 0.4; const POOL_NARROW = 0.2; const RIFFLE_WIDE = 0.3;
// The water lies against the outer bank from this much of a full bend on.
const POOL_AT_BANK = 0.6;
// In a full bend the pool's deepest lies toward the outer bank: across the water the bed's parabola leans by this much
// (its depth times 1 + `POOL_SKEW` × the share of the way to the outer edge).
const POOL_SKEW = 0.8;

// Value noise over a channel's reach, of stream `k`, -1..1: at `along` m along its line, in cells `cell` m long (a whole
// number of them in the length the place along is kept modulo, after which it comes again), and `across` cells
// across it.
fn noise_reach(along: f32, cell: f32, across: f32, k: i32) -> f32 {
  let n = i32(round(land.reach.x / cell));
  let p = vec2f(along / cell, across);
  let i = vec2i(floor(p));
  let f = p - floor(p);
  let u = f * f * (3.0 - 2.0 * f);
  let x = vec2i(((i.x % n) + n) % n, (((i.x + 1) % n) + n) % n);
  let v = vec4f(f32(lowbias(vec2i(x.x, i.y), k)), f32(lowbias(vec2i(x.y, i.y), k)), f32(lowbias(vec2i(x.x, i.y + 1), k)), f32(lowbias(vec2i(x.y, i.y + 1), k))) * (2.0 / 4294967296.0) - 1.0;
  return mix(mix(v.x, v.y, u.x), mix(v.z, v.w, u.x), u.y);
}

// A channel at a point of ground `g`, of this month's discharge `q`, m³/s: the half width of its bed on the point's
// side of its line, m (`banks_of`, varied along it, held on the inside of a bend); the half width of this month's
// water, m, and where its middle lies across the line (as `Tiled.sd`), m: against the outer bank in a bend; its mean
// depth by the channel law, m, and what the depth there is of the law's (a pool is deeper, a riffle shallower: the same
// flow through a narrower or a wider water); how much of a bend the place is, 0..1; and the side its outer bank lies
// on (the sign of `Tiled.sd` there; 1 where the line is straight).
struct Channel { bed: f32, water: f32, middle: f32, mean: f32, deep: f32, bend: f32, outer: f32 }
fn channel_at(g: Tiled, q: f32) -> Channel {
  if (q <= 0.0) { return Channel(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0); }
  let law = banks_of(q);
  let vary = 1.0 + BED_VARY * (noise_reach(g.along, BED_VARY_M.x, 0.0, 68) + 0.5 * noise_reach(g.along, BED_VARY_M.y, 0.0, 69)) / 1.5;
  let turn = abs(g.curve);
  let bend = min(turn * 2.0 * law / BEND_FULL, 1.0);
  let outer = select(1.0, -1.0, g.curve > 0.0);
  var bed = law * vary;
  if (g.sd * g.curve > 0.0) { bed = min(bed, INNER_MOST / max(turn, 1e-6)); }
  let wide = mix(1.0 + RIFFLE_WIDE, 1.0 - POOL_NARROW, bend);
  let water = min(law * vary * pow(land.reach.y, -STATION_EXP) * wide, law * vary);
  let middle = outer * ease(0.0, POOL_AT_BANK, bend) * (law * vary - water);
  return Channel(bed, water, middle, land.channel.z * pow(q, land.channel.w), 1.0 / wide, bend, outer);
}

// The depth of the bed of channel `c` under its water's surface, m, the share `x` of the way across the water from its
// middle to its outer edge (-1 at its inner edge): a parabola across the water, as deep at its middle as its mean
// depth makes it (`BED_MIDDLE`), leaning to the outer bank in a bend (`POOL_SKEW`), so the water is deepest in its
// thread and its banks catch the light.
fn depth_across(c: Channel, x: f32) -> f32 {
  return BED_MIDDLE * c.mean * c.deep * (1.0 - x * x) * (1.0 + POOL_SKEW * c.bend * x);
}

// The depth of the bed of channel `c` under its water's surface at the point of ground `g`, m (`depth_across`).
fn depth_in(g: Tiled, c: Channel) -> f32 {
  return depth_across(c, clamp((g.sd - c.middle) * c.outer / max(c.water, 1e-6), -1.0, 1.0));
}
