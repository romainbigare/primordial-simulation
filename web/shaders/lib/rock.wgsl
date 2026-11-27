// The rock and the channels of the live map (web/src/land.ts, tape/src/ground/rock.rs): of each of the terrain's
// samples the depth of its loose ground, the largest stone its channel moves, its reach's slope and its channel's links; how much of the
// bed shows where the loose ground thins; and a channel at a point: what runs through it and the shape of its bed and
// of its water (`channel_at`), from the place along its line and its swing from its line without bends that the tiles
// hold (land.wgsl `Tiled`).

// Of each sample (a texel each): the depth of its loose ground, m, the largest stone its channel moves, m, the slope of
// its channel's reach (rise over run), and its channel's links (`links_of`); in a second plane under the first, the
// reflectance of its bed's make; and in a third, its loose ground's shares of coarse and fine grain, how slowly it
// passes liquid (`grain_at`) and the liquid it holds for roots as the layer `avail` shows it; and in a fourth, the
// slope up from it to the highest ground toward the light (`Laid.horizon`), a zero, how deep the load the liquid laid
// lies on it, m, and the
// decimal logarithm of the years its loose ground has lain (tape/src/ground/rock.rs).
@group(0) @binding(18) var rock_field: texture_2d<f32>;

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
const LOOSE_PLANE = 0; const MAKE_PLANE = 1; const GRAIN_PLANE = 2; const LAID_PLANE = 3;

// Plane `plane` of the samples at `at`, m from the map's north-west corner: through the samples' centres as a
// B-spline read texel by texel (draw.wgsl `b_spline`), so no sample's square shows; and how far the samples it blends
// spread about that, the root of their weighted variance (`Blend.spread`).
struct Blend { mean: vec4f, spread: vec4f }
fn samples_blend(at: vec2f, plane: i32) -> Blend {
  let c = b_spline(at / (land.map.x * land.map.yz) * land.samples.xy - 0.5);
  let last = vec2i(land.samples.xy) - 1;
  let rows = vec2i(0, plane * i32(land.samples.y));
  var sum = vec4f(0.0);
  var squares = vec4f(0.0);
  for (var n = 0; n < B_TAPS; n++) {
    let v = textureLoad(rock_field, clamp(b_texel(c, n), vec2i(0), last) + rows, 0);
    sum += b_weight(c, n) * v;
    squares += b_weight(c, n) * v * v;
  }
  return Blend(sum, sqrt(max(squares - sum * sum, vec4f(0.0))));
}
// (The same, its mean alone.)
fn samples_at(at: vec2f, plane: i32) -> vec4f { return samples_blend(at, plane).mean; }

// A field held per sample is a mean over the sample's square, and its edges are no lines on the ground: each is read
// at a place moved from the point (`sample_place`), along the slope of octaves of gradient noise `PLACE_WARP_M` across
// (noise streams from `PLACE_STREAM`), by `PLACE_WARP` m per unit of that slope (it reaches about 2), so no edge of a
// field shows the samples' grid, and every edge wanders at every scale down to the relief's (relief.wgsl). An octave
// fades out as it shrinks to a few pixels (draw.wgsl `shows_at`). The same noise, its values summed over the octaves
// shown, each weighing less the finer (`Place.n`, about -1..1), moves the thresholds read on those fields (the
// deposit's depth, `laid_at`) and the tone of the ground (style.wgsl `tone_of`).
const PLACE_WARP_M = vec3f(1000.0, 250.0, 62.0); const PLACE_WARP = vec3f(40.0, 55.0, 25.0); const PLACE_STREAM = 214;
// The place the samples' fields are read for a point, m from the map's north-west corner, and the noise there.
struct Place { at: vec2f, n: f32 }
// The place of the point `at`, seen at `e` m a pixel.
fn sample_place(at: vec2f, e: f32) -> Place {
  var out = Place(at, 0.0);
  for (var o = 0; o < 3; o++) {
    let shown = shows_at(PLACE_WARP_M[o], e);
    if (shown <= 0.0) { break; }
    // (From `at` itself, not the pixel's place: the marks read it at a plant's foot.)
    let q = mat2x2f(cos(TURN_A), sin(TURN_A), -sin(TURN_A), cos(TURN_A)) * at / PLACE_WARP_M[o];
    let i = floor(q);
    let n = gradient_noise(Lat(vec2i(i), q - i), PLACE_STREAM + o);
    out = Place(out.at + shown * PLACE_WARP[o] * n.yz, out.n + shown * NOISE_GAIN * n.x / f32(o + 1));
  }
  return out;
}

// The depth of the loose ground at the place `place` (`sample_place`), m (`samples_blend`): a sample's depth is a mean,
// and inside it the ground is as deep as its neighbours' about as often (a mantle thickens and thins at every scale),
// so the blend departs from the samples' mean by the place's noise (`Place.n`) times `LOOSE_SPREAD` of how far the
// samples it blends spread: a rule read on it (the bed it bares, the plants' roots, `rooted`) follows no sample's
// square where two samples' depths differ, and keeps the mean where they do not. Its change across a pixel is smooth.
// Inside a sample the mantle lies deeper where the ground is lush (lush.wgsl `lush_of`: creep and wash fill the
// hollows and drainage lines and strip the ridges and knobs), up to `LOOSE_LUSH` of itself either way, about none on
// the mean (drawing only): so the bed shows and the plants' roots fail first on the crests, and the edges of both
// follow the relief. Of a point as lush as `lush`.
const LOOSE_SPREAD = 1.0; const LOOSE_LUSH = 0.6;
fn loose_at(place: Place, lush: f32) -> f32 {
  let b = samples_blend(place.at, LOOSE_PLANE);
  return max(b.mean.x + LOOSE_SPREAD * b.spread.x * place.n, 0.0) * (1.0 + LOOSE_LUSH * lush);
}

// The loose ground's grain shares swap coarse for fine, at most all of themselves either way: what the wear lays down
// is sorted finer than a sample. Rain wash carries the fines off the crests and the dry sunny sides into the hollows
// and drainage lines, where they settle, and leaves the coarse behind (Kirkby 1969; from memory): `GRAIN_LUSH` of
// themselves by how lush the ground is (lush.wgsl `lush_of`). Over that, lenses and patches: the noise of the
// samples' places (`Place.n`, from a kilometre down) times `GRAIN_PLACE`, and patches from `GRAIN_PATCH_M` across
// down in `GRAIN_OCTAVES` octaves (ragged.wgsl) times `GRAIN_PATCHY`. All about none on the mean, so the mean is the
// samples' grain.
const GRAIN_LUSH = 0.45; const GRAIN_PLACE = 0.3; const GRAIN_PATCHY = 0.3; const GRAIN_PATCH_M = 25.0; const GRAIN_OCTAVES = 3;

// A deposit's depth departs from the samples' by a factor of up to e^`LAID_PATCHY` either way over the noise of the
// samples' places (`Place.n`), and its age by up to `LAID_AGE_PATCHY` of its decimal logarithm: a load is laid thicker
// in the swales and thinner on the bars, and reworked unevenly, so a deposit's edge and its fresh part wander.
const LAID_PATCHY = 0.7; const LAID_AGE_PATCHY = 0.4;
// The loose ground's grain at `at`, as lush as `lush`, as `Spot.grain` holds it, read at the place `place`
// (`sample_place`) and moved by its noise, seen at `e` m a pixel, with its organic matter `organic` (the cells', which
// changes as the dead matter on the ground rots): the samples' grain (`samples_at`: the shares of coarse and fine grain
// and how slowly it passes liquid) moved by `shift`, the cells' grain (`World::soil`, what the laws read) over the
// samples' in their square (web/src/land.ts), so the world's soil is drawn on the mean and the samples' pattern inside
// a square, with its patches. Through the samples' centres as a B-spline (`samples_at`): a field eased over a narrow
// edge shows no sample's square.
fn grain_at(at: vec2f, place: Place, e: f32, organic: f32, shift: vec2f, lush: f32) -> vec4f {
  let raw = samples_at(place.at, GRAIN_PLANE);
  let g = vec4f(clamp(raw.xy + shift, vec2f(0.0), vec2f(1.0)), raw.zw);
  let n = clamp(GRAIN_PATCHY * ragged(at, e, GRAIN_PATCH_M, GRAIN_OCTAVES, 210) + GRAIN_PLACE * place.n - GRAIN_LUSH * lush, -1.0, 1.0);
  return vec4f(g.x * (1.0 + n), g.y * (1.0 - n), organic, g.z);
}

// What the samples' fourth plane holds at a point: its deposit as `Spot.deposit` holds it, and the slope up from it to
// the highest ground toward the light, rise over run (tape/src/ground/horizon.rs).
struct Laid { deposit: vec2f, horizon: f32 }
// The deposit at the place `place` (`sample_place`), moved by its noise: how deep the load the liquid laid lies there,
// m, and the decimal logarithm of the years its loose ground has lain (`LAID_PLANE`: a deposit's edge is eased where it
// is drawn, colour.wgsl `laid_share`); and the horizon the same plane holds. Through the samples' centres as a B-spline
// (`samples_at`).
fn laid_at(place: Place) -> Laid {
  let laid = samples_at(place.at, LAID_PLANE);
  return Laid(vec2f(laid.z * exp(LAID_PATCHY * place.n), laid.w + LAID_AGE_PATCHY * place.n), laid.x);
}

// A point's bed is of one make, not a blend of two: bare rock and the soil broken from it show the bands the folded
// layers make (core/src/space/strata.rs) with an edge that wanders like an outcrop's. Of the samples about the place a
// point reads, each make weighs its samples' weights through their centres as a B-spline (`b_spline`); the leading
// make and the next meet where the difference of their weights passes the patches' noise (`ragged`, from
// `MAKE_PATCH` samples' spacing down, `MAKE_OCTAVES`, streams from `MAKE_STREAM`) times `MAKE_RAGGED`, short of one, so
// no make shows past where its weight starts: the contour of a smooth weight follows no sample's square, and the noise
// moves it by up to most of a sample, at every scale. An octave shows from `MAKE_PX` times the pixels `shows_at` asks
// (a wiggle of a few pixels is a jagged line), and the two meet over `MAKE_EDGE_PX` pixels and no less than
// `MAKE_EDGE_M` (a soil broken from two beds mixes over metres). As a pixel grows to the samples' spacing the pick
// fades into the samples' plain blend (`shows_at`).
const MAKE_RAGGED = 0.95; const MAKE_PATCH = 2.0; const MAKE_OCTAVES = 6; const MAKE_STREAM = 240;
const MAKE_PX = 2.0; const MAKE_EDGE_PX = 1.5; const MAKE_EDGE_M = 25.0;
// The reflectance of the bed's make at `at`, m from the map's north-west corner, read at the place `place`
// (`sample_place`), seen at `e` m a pixel: as the bare bed shows it, of one make (`sharp`), and as a soil over it
// shows it (`soft`): the samples' plain blend, as creep and wash smear a mantle's colour over its beds' contacts across
// tens to hundreds of metres (a residual soil's colour is its bed's, mixed downslope: from memory). Not `crisp`, the
// soft alone (no noise is read).
struct Make { sharp: vec3f, soft: vec3f }
fn bed_make_at(at: vec2f, place: vec2f, e: f32, crisp: bool) -> Make {
  let spacing = land.map.x * land.map.y / land.samples.x;
  let p = place / spacing - 0.5;
  let bs = b_spline(p);
  let last = vec2i(land.samples.xy) - 1;
  let rows = vec2i(0, MAKE_PLANE * i32(land.samples.y));
  var c: array<vec3f, B_TAPS>;
  var blend = vec3f(0.0);
  for (var n = 0; n < B_TAPS; n++) {
    c[n] = textureLoad(rock_field, clamp(b_texel(bs, n), vec2i(0), last) + rows, 0).xyz;
    blend += b_weight(bs, n) * c[n];
  }
  let sharp = shows_at(spacing, e);
  if (sharp <= 0.0 || !crisp) { return Make(blend, blend); }
  // Each sample's make's weight; the leading make `a`, and the next `b` (none where all are of one make).
  var s: array<f32, B_TAPS>;
  for (var n = 0; n < B_TAPS; n++) {
    s[n] = 0.0;
    for (var m = 0; m < B_TAPS; m++) { s[n] += select(0.0, b_weight(bs, m), all(c[m] == c[n])); }
  }
  var a = 0;
  for (var n = 1; n < B_TAPS; n++) { if (s[n] > s[a]) { a = n; } }
  var b = -1;
  for (var n = 0; n < B_TAPS; n++) { if (any(c[n] != c[a]) && (b < 0 || s[n] > s[b])) { b = n; } }
  if (b < 0) { return Make(c[a], blend); }
  // The lighter of the two is `hi`, so the noise moves their edge the same way from either side of it.
  let swap = dot(c[b], LUMA) > dot(c[a], LUMA);
  let hi = c[select(a, b, swap)];
  let lo = c[select(b, a, swap)];
  // Their weights' difference and its change along x and y, a sample (the B-spline's weights' slopes).
  let t = p - round(p);
  let dwx = vec3f(t.x - 0.5, -2.0 * t.x, 0.5 + t.x);
  let dwy = vec3f(t.y - 0.5, -2.0 * t.y, 0.5 + t.y);
  var d = 0.0;
  var slope = vec2f(0.0);
  for (var m = 0; m < B_TAPS; m++) {
    let sign = select(0.0, 1.0, all(c[m] == hi)) - select(0.0, 1.0, all(c[m] == lo));
    d += sign * b_weight(bs, m);
    slope += sign * vec2f(dwx[m % 3] * bs.wy[m / 3], bs.wx[m % 3] * dwy[m / 3]);
  }
  let x = d - MAKE_RAGGED * ragged(at, MAKE_PX * e, MAKE_PATCH * spacing, MAKE_OCTAVES, MAKE_STREAM);
  let across = length(slope) / spacing + MAKE_RAGGED * ragged_slope(MAKE_PX * e, MAKE_PATCH * spacing, MAKE_OCTAVES);
  let half = max(across * max(MAKE_EDGE_PX * e, MAKE_EDGE_M), 1e-6);
  return Make(mix(blend, mix(lo, hi, smoothstep(-half, half, x)), sharp), blend);
}

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
