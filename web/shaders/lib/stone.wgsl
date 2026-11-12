// The one maker of a stone of the map (colour.wgsl `ground_of`, land.wgsl): a block of rock lying on the ground, seen
// from above. Its outline is what five to eight planes leave of it (`STONE_CUTS`, each stone its own count), each at
// its own angle and distance from the centre (`STONE_CUT`), so no stone is a disc and no two are alike; it is longer
// one way than the other (`STONE_LONG`). Each
// plane is a face that falls from the stone's top (`STONE_TOP` of its radius high, tilted by up to `STONE_TILT`), and
// the faces meet over a width that says how worn the stone is: sharp edges on a block fresh from a cliff, round ones on a
// cobble a channel has rolled (`STONE_SOFT`, `STONE_FACE`: angular to rounded). The sun lights each face by how it is
// turned (draw.wgsl `LIGHT`), the stone throws a shadow on the ground away from the sun (`STONE_SHADOW`), and loose
// ground about it buries its foot (`sunk`). It is its bed's colour (`STONE_TONE`, each stone a little its own,
// `STONE_TONES`) with the bands of the layers it broke from (`STONE_BANDS`, every `STONE_BAND_M`, no closer than
// `STONE_BAND_LEAST` of its radius: a large block shows a few beds, not a hatching). Stones lie one to a cell of two lattices
// turned from one another, in `STONE_OCTAVES` sizes each `STONE_STEP` times smaller than the last, the smaller the
// more of them (debris has many small pieces and few large: the second largest cells hold `STONE_FEW` as often, the
// largest as often as the place says, `Stony.rare`, on a scree `STONE_RARE`): fixed in the world, and each size drawn
// as itself from `STONE_PX` pixels across, below as the tone its stones give the ground. A stone of a size is
// `STONE_SIZES` of it across, the small ones the more often (the draw to the power 1.5), and lies anywhere in its cell that
// holds it whole: a small one far from the cell's middle, so no rows show. A stone and its shadow lie within
// `STONE_REACH` of its radii of its centre.
const STONE_CUTS = vec2i(5, 8); const STONE_CUT = 0.38; const STONE_LONG = 0.7; const STONE_TOP = 0.75; const STONE_TILT = 0.3;
const STONE_SOFT = vec2f(0.04, 0.45); const STONE_FACE = vec2f(1.7, 1.2); const STONE_RISE = 1.1;
const STONE_SHADOW = 0.45; const STONE_SHADOW_REACH = 0.4; const STONE_REACH = 1.6;
const STONE_TONE = 1.15; const STONE_TONES = 0.18; const STONE_BANDS = 0.12; const STONE_BAND_M = 0.35; const STONE_BAND_LEAST = 0.6;
const STONE_OCTAVES = 4; const STONE_STEP = 2.2; const STONE_FEW = 0.5; const STONE_RARE = 0.2;
const STONE_GAP = 3.0; const STONE_SIZES = vec2f(0.55, 1.45); const STONE_PX = 2.0;
// Snow buries a stone as it buries a plant, by its depth against the stone's height (draw.wgsl `above_snow`); on one
// that stands out of it, it lies on the flat top (where its face is flatter than `SNOW_TOP`) once it is `SNOW_CAP_M`
// deep.
const SNOW_TOP = vec2f(0.8, 0.95); const SNOW_CAP_M = 0.1;
// The largest stone drawn, m: no draw shows one from this many m a pixel on.
const STONE_MOST_M = 4.5;

// A stone at a point: the share of the pixel it covers, the light on it over the light on flat ground, and how flat it
// is there, 0 on a face .. 1 on its top.
struct StoneAt { a: f32, lit: f32, flat: f32 }

// The stone of random word `x` at `v`, the point from its centre in its radii, a pixel `px` radii across: `worn` from
// 0 (angular) to 1 (rounded), its foot `sunk` of its height in the ground.
fn stone_at(v: vec2f, px: f32, x: u32, worn: f32, sunk: f32) -> StoneAt {
  let y = x * 747796405u + 2891336453u;
  let z = y * 747796405u + 2891336453u;
  let cuts = STONE_CUTS.x + i32((x >> 22u) % u32(STONE_CUTS.y - STONE_CUTS.x + 1));
  // The stone's own frame: turned, and squeezed across its long axis.
  let turn = f32(x & 255u) * (2.0 * PI / 256.0);
  let squeeze = 1.0 + STONE_LONG * f32((x >> 8u) & 15u) / 15.0;
  let rot = vec2f(cos(turn), sin(turn));
  let u = vec2f(rot.x * v.x + rot.y * v.y, (rot.x * v.y - rot.y * v.x) * squeeze);
  let soft = mix(STONE_SOFT.x, STONE_SOFT.y, worn);
  let steep = mix(STONE_FACE.x, STONE_FACE.y, worn);
  // Its height: the lowest of its tilted top and its faces, met softly; and that height's slope.
  var slope = STONE_TILT * (vec2f(f32((x >> 12u) & 31u), f32((x >> 17u) & 31u)) / 15.5 - 1.0);
  var h = STONE_TOP + dot(slope, u);
  for (var i = 0; i < STONE_CUTS.y; i++) {
    if (i >= cuts) { break; }
    let bits = (select(z, y, i < 5) >> u32(6 * (i % 5))) & 63u;
    let a = (f32(i) + 0.8 * (f32(bits & 7u) / 7.0 - 0.5)) * (2.0 * PI / f32(cuts));
    let n = vec2f(cos(a), sin(a));
    let face = steep * (1.0 - STONE_CUT * f32(bits >> 3u) / 7.0 - dot(n, u));
    let m = clamp(0.5 + 0.5 * (face - h) / soft, 0.0, 1.0);
    h = mix(face, h, m) - soft * m * (1.0 - m);
    slope = mix(-steep * n, slope, m);
  }
  // The slope back in the map's axes.
  let s = vec2f(slope.x, slope.y * squeeze);
  let grad = vec2f(rot.x * s.x - rot.y * s.y, rot.y * s.x + rot.x * s.y);
  let n = normalize(vec3f(-STONE_RISE * grad, 1.0));
  let over = h - sunk * STONE_TOP;
  return StoneAt(clamp(over / max(length(grad) * px, 1e-4) + 0.5, 0.0, 1.0), max(dot(n, LIGHT), 0.0) / LIGHT.z, n.z);
}

// What makes the stones of a place: the share of the cells that hold one; the largest's size, m, which sets where
// they lie (so it changes only in steps from place to place: a size that changed smoothly would move every stone);
// what each stone is of that size there, 0..1 (this may change smoothly: finer up a bar); how worn they are (0 angular
// .. 1 rounded), how deep their feet are sunk (0..1 of their height), the snow on their ground, 0..1, and its depth
// where it lies, m; and the share of the cells of the largest size that hold a block.
struct Stony { density: f32, size: f32, small: f32, worn: f32, sunk: f32, snow: f32, snow_m: f32, rare: f32 }

// The share of the cells of octave `o` (0: the largest) that hold a stone of `s`.
fn stones_held(s: Stony, o: i32) -> f32 {
  return s.density * select(select(1.0, STONE_FEW, o == 1), s.rare, o == 0);
}

// A stone of a lattice: whether its cell holds one, its random word, its radius, m, and the point from its centre, m.
struct Placed { held: bool, x: u32, r: f32, v: vec2f }

// The stone of size `size` m (those of its octave `o`: `density` of the cells hold one) of lattice `l` whose cell
// holds the point `at`, stream `k` of the world's random numbers. The second lattice lies turned and half a cell off
// the first, so no rows of stones show; a stone lies anywhere its cell holds it whole, with its shadow.
fn placed(at: vec2f, size: f32, density: f32, o: i32, l: i32, k: i32) -> Placed {
  let side = STONE_GAP * size;
  let q = select(at, vec2f(0.8 * at.x + 0.6 * at.y, 0.8 * at.y - 0.6 * at.x), l == 1) / side + f32(l) * 0.5;
  let c = floor(q);
  let x = lowbias(vec2i(c), k + 2 * o + l);
  let draw = f32((x >> 20u) & 1023u) / 1023.0;
  let r = 0.5 * size * mix(STONE_SIZES.x, STONE_SIZES.y, draw * sqrt(draw));
  let room = 0.5 - (1.0 + STONE_SHADOW_REACH) * r / side;
  let centre = c + 0.5 + room * (vec2f(f32((x >> 10u) & 31u), f32((x >> 15u) & 31u)) / 15.5 - 1.0);
  let v = (q - centre) * side;
  // The point from the centre back in the map's axes.
  return Placed(f32(x & 1023u) / 1024.0 < density, x, r, select(v, vec2f(0.8 * v.x - 0.6 * v.y, 0.6 * v.x + 0.8 * v.y), l == 1));
}

// The stones `s` at least `least` m large at `at`, seen at `e` m a pixel, of bed `bed` (linear, unlit), stream `k` of
// the world's random numbers: their colour times their coverage (with the shadow they throw as black), and that
// coverage.
fn stones(at: vec2f, e: f32, s: Stony, least: f32, bed: vec3f, k: i32) -> vec4f {
  var out = vec4f(0.0);
  if (s.density <= 0.0 || s.size <= 0.0) { return out; }
  let tone = bed * STONE_TONE;
  let snow_m = s.snow * s.snow_m;
  var size = s.size / pow(STONE_STEP, f32(STONE_OCTAVES - 1));
  // The smallest first, the larger over them.
  for (var o = STONE_OCTAVES - 1; o >= 0; o--) {
    let density = stones_held(s, o);
    // Out of the water, where it stands in a channel's.
    var stands = 1.0;
    if (least > 0.0) { stands = ease(least, 1.5 * least, s.small * size); }
    // Smaller than a few pixels: the share of the ground they cover, as a tone (as far as the snow leaves them).
    var near = 0.0;
    if (!unseen(STONE_MOST_M / STONE_PX)) { near = ease(STONE_PX * e, 2.0 * STONE_PX * e, s.small * size); }
    let high = STONE_TOP * (1.0 - s.sunk);
    let mean = 2.0 * density * stands * above_snow(snow_m, high * 0.5 * s.small * size) * PI * 0.25 * 0.7 * s.small * s.small / (STONE_GAP * STONE_GAP) * (1.0 - near);
    out = mix(out, vec4f(tone, 1.0), mean);
    if (near > 0.0 && stands > 0.0 && density > 0.0) {
      for (var l = 0; l < 2; l++) {
        let p = placed(at, size, density, o, l, k);
        let r = s.small * p.r;
        if (!p.held || dot(p.v, p.v) > STONE_REACH * STONE_REACH * r * r) { continue; }
        // Under snow deeper than it is tall, a stone does not show.
        let shows = near * stands * above_snow(snow_m, high * r);
        if (shows <= 0.0) { continue; }
        let v = p.v / r;
        let w = p.x * 2654435761u;
        let st = stone_at(v, e / r, w, s.worn, s.sunk);
        // Its shadow on the ground, away from the sun.
        let shade = STONE_SHADOW * (1.0 - smoothstep(0.6, 1.1, length(v + STONE_SHADOW_REACH * LIGHT.xy / length(LIGHT.xy))));
        out = mix(out, vec4f(0.0, 0.0, 0.0, 1.0), shade * shows * (1.0 - st.a));
        if (st.a <= 0.0) { continue; }
        // Its own tone, and the bands of its rock across it.
        let own = 1.0 + STONE_TONES * (f32((w >> 8u) & 255u) / 127.5 - 1.0);
        let dir = vec2f(cos(f32(w >> 24u)), sin(f32(w >> 24u)));
        let bands = 1.0 + STONE_BANDS * sin(dot(v, dir) * r / max(STONE_BAND_M, STONE_BAND_LEAST * r) * 2.0 * PI + f32((w >> 16u) & 255u));
        let cap = ease(0.0, SNOW_CAP_M, snow_m) * smoothstep(SNOW_TOP.x, SNOW_TOP.y, st.flat);
        let rgb = mix(lit_by(tone * own * bands, 0.35 + 0.65 * st.lit), SNOW, cap);
        out = mix(out, vec4f(rgb, 1.0), st.a * shows);
      }
    }
    size *= STONE_STEP;
  }
  return out;
}

// A stone that stands in a flow breaks it: white water piles against its face up the flow and trails down the flow
// behind it, `WAKE_LONG` of its size long, spreading by `WAKE_SPREAD` of the way, with calm water right behind it
// (its eddy, `WAKE_EDDY` of its radius).
const WAKE_LONG = 2.2; const WAKE_SPREAD = 0.35; const WAKE_EDDY = 0.8;

// The white water at `at`, seen at `e` m a pixel, about the stones `s` at least `least` m large (`stones`, stream `k`)
// in a flow that runs `down` (a unit vector), 0..1: each stone's the same stone `stones` draws, found from a point up
// the flow.
fn wake(at: vec2f, e: f32, s: Stony, least: f32, down: vec2f, k: i32) -> f32 {
  var white = 0.0;
  if (least <= 0.0) { return white; }
  var size = s.size;
  for (var o = 0; o < STONE_OCTAVES; o++) {
    let stands = ease(least, 1.5 * least, s.small * size) * ease(STONE_PX * e, 2.0 * STONE_PX * e, s.small * size);
    let density = stones_held(s, o);
    if (stands > 0.0 && density > 0.0) {
      for (var l = 0; l < 2; l++) {
        // The stone whose cell holds the point half a wake up the flow.
        let back = 0.5 * WAKE_LONG * size * down;
        let p = placed(at - back, size, density, o, l, k);
        if (!p.held) { continue; }
        let r = s.small * p.r;
        let v = p.v + back;
        let along = dot(v, down) / r;
        let across = abs(dot(v, vec2f(-down.y, down.x))) / r;
        // Behind it: two trails at its sides that spread and fade, calm between them at first; before it: a rim.
        let reach = 2.0 * WAKE_LONG;
        let wide = 1.0 + WAKE_SPREAD * max(along, 0.0);
        let trail = (1.0 - smoothstep(0.5 * wide, wide, across)) * smoothstep(WAKE_EDDY * (1.0 - along / reach), wide, across + 0.6 * along) * (1.0 - smoothstep(0.0, reach, along)) * step(0.0, along);
        let rim = (1.0 - smoothstep(1.0, 1.35, length(v) / r)) * smoothstep(-0.2, -0.9, along);
        white = max(white, stands * max(trail, rim));
      }
    }
    size /= STONE_STEP;
  }
  return white;
}
