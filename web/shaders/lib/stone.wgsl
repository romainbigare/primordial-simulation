// The stones of the map (colour.wgsl `ground_of`, river.wgsl `river_on`): blocks of rock lying on the ground, each a
// rendered model (things.wgsl `stone_sprite`): a cobble where its ground's stones are worn round, an angular rock where
// they are not, each stone by its own draw against how worn they are; it is its bed's colour (`STONE_TONE`), throws a
// shadow on the ground away from the sun (`STONE_SHADOW`), and loose ground about it buries its foot (`sunk`). Stones
// lie one to a cell of two lattices turned from one another, in `STONE_OCTAVES` sizes each `STONE_STEP` times smaller
// than the last, the smaller the more of them (debris has many small pieces and few large: the second largest cells
// hold `STONE_FEW` as often, the largest as often as the place says, `Stony.rare`, on a scree `STONE_RARE`): fixed in
// the world, and each size drawn as itself by the one rule for things (draw.wgsl `as_thing`), below as the tone its
// stones give the ground. A stone of a size is `STONE_SIZES` of it across, the small ones the more often (the draw to
// the power 1.5), and lies anywhere in its cell that holds it whole, with its shadow: a small one far from the cell's
// middle, so no rows show. A stone stands `STONE_TOP` of its radius high (the snow buries it by that); it and its
// shadow lie within `STONE_REACH` of its radii of its centre.
const STONE_TOP = 0.75; const STONE_SHADOW = 0.45; const STONE_SHADOW_REACH = 0.4; const STONE_TONE = 1.15;
const STONE_OCTAVES = 4; const STONE_STEP = 2.2; const STONE_FEW = 0.5; const STONE_RARE = 0.2;
const STONE_GAP = 3.0; const STONE_SIZES = vec2f(0.55, 1.45); const STONE_REACH = 1.6;
// The largest stone drawn, m: no draw shows one from this many m a pixel on.
const STONE_MOST_M = 4.5;
// Under the size its sprite shows from (draw.wgsl `THING_PX`) a stone is still drawn one by one from the size a
// plant's speck shows from (cover_field.wgsl `SPECK_PX`), as a speck: a dome lit by the sun, `STONE_SPECK_LIGHT.x`
// times its colour on its shaded side to `.y` on its lit one, over its shadow (as its sprite's), its edge eased over a
// pixel. Smaller, it is the tone the stones give the ground. So stony ground keeps its grain of lit and shaded points
// as it shrinks.
const STONE_SPECK_LIGHT = vec2f(0.55, 1.3);

// A stone of the stones `s` (`placed`) as a speck, seen at `e` m a pixel, over `out` (its colour times its coverage,
// and that coverage), as strongly as `shows`, of the bed's tone `tone`.
fn stone_speck(out: vec4f, p: Placed, s: Stony, e: f32, shows: f32, tone: vec3f) -> vec4f {
  let r = s.small * p.r;
  let v = p.v / r;
  let d = length(v);
  let shade = STONE_SHADOW * (1.0 - smoothstep(0.6, 1.1, length(v + STONE_SHADOW_REACH * LIGHT.xy / length(LIGHT.xy))));
  let a = 1.0 - smoothstep(1.0 - 0.5 * e / r, 1.0 + 0.5 * e / r, d);
  let dst = mix(out, vec4f(0.0, 0.0, 0.0, 1.0), shade * shows * (1.0 - a));
  if (a <= 0.0) { return dst; }
  let lit = clamp(dot(vec3f(v, sqrt(max(1.0 - d * d, 0.0))), LIGHT) / LIGHT.z, STONE_SPECK_LIGHT.x, STONE_SPECK_LIGHT.y);
  return mix(dst, vec4f(wet_stone(tone * lit, lit, s.wet, s.worn), 1.0), a * shows);
}

// A wet stone is darker: water fills the pores of its face, and light that would scatter back from the dry grains goes
// into the rock (wet rock and soil send back about half to two thirds of the light dry ones do: Lekner and Dorf 1988),
// by up to `WET_STONE_DARK`; its film of water shines where its face turns the sun's light up to the eye, by up to
// `WET_GLOSS` of the sky's white, on faces lit from `GLOSS_LIT.x` to `.y` times flat ground. A stone dry in the sun
// is pale (`DRY_PALE` of the way to its own grey made light, the worn ones most): the film of fine silt the water left
// on it dries light.
const WET_STONE_DARK = 0.55; const WET_GLOSS = 0.35; const GLOSS_LIT = vec2f(1.05, 1.35); const DRY_PALE = 0.45;

// The colour of a stone's face of colour `rgb` (linear, lit), `lit` times the light on flat ground,
// `wet` wet (0..1) and `worn` worn (0..1): the one wet look of every stone and rock drawn.
fn wet_stone(rgb: vec3f, lit: f32, wet: f32, worn: f32) -> vec3f {
  let dry = mix(rgb, vec3f(dot(rgb, LUMA) * 1.6), DRY_PALE * (0.5 + 0.5 * worn) * (1.0 - wet));
  return dry * (1.0 - WET_STONE_DARK * wet) + WET_GLOSS * wet * smoothstep(GLOSS_LIT.x, GLOSS_LIT.y, lit) * SKY;
}

// What makes the stones of a place: the share of the cells that hold one; the largest's size, m, which sets where
// they lie (so it changes only in steps from place to place: a size that changed smoothly would move every stone);
// what each stone is of that size there, 0..1 (this may change smoothly: finer up a bar); how worn they are (0 angular
// .. 1 rounded), how deep their feet are sunk (0..1 of their height), the snow on their ground, 0..1, and its depth
// where it lies, m; the share of the cells of the largest size that hold a block; and how wet their faces are, 0..1
// (`wet_stone`).
struct Stony { density: f32, size: f32, small: f32, worn: f32, sunk: f32, snow: f32, snow_m: f32, rare: f32, wet: f32 }

// The share of the cells of octave `o` (0: the largest) that hold a stone of `s`: none where fewer than
// `STONES_LEAST` of the places hold one (a stone in a hundred places is no scree, and the ground under closed plants
// reads none of it), all of them from twice that.
const STONES_LEAST = 0.04;
fn stones_held(s: Stony, o: i32) -> f32 {
  return s.density * ease(STONES_LEAST, 2.0 * STONES_LEAST, s.density) * select(select(1.0, STONE_FEW, o == 1), s.rare, o == 0);
}

// A stone of a lattice: whether its cell holds one, its random word, its radius, m, the point from its centre, m, and
// how far inside its cell the point lies, 0 at its edge to 1 from `CELL_INSIDE` of the cell in.
struct Placed { held: bool, x: u32, r: f32, v: vec2f, inside: f32 }
const CELL_INSIDE = 0.15;

// The stone of size `size` m (those of its octave `o`: `density` of the cells hold one) of lattice `l` whose cell
// holds the pixel's place moved by `shift` m, stream `k` of the world's random numbers. The second lattice lies turned
// and half a cell off the first, so no rows of stones show; a stone lies anywhere its cell holds it whole, with its
// shadow.
fn placed(size: f32, density: f32, o: i32, l: i32, k: i32, shift: vec2f) -> Placed {
  let side = STONE_GAP * size;
  var turn = mat2x2f(1.0 / side, 0.0, 0.0, 1.0 / side);
  if (l == 1) { turn = mat2x2f(0.8, -0.6, 0.6, 0.8) * (1.0 / side); }
  let q = lat(turn, turn * shift + f32(l) * 0.5);
  let x = lowbias(q.i, k + 2 * o + l);
  let draw = f32((x >> 20u) & 1023u) / 1023.0;
  let r = 0.5 * size * mix(STONE_SIZES.x, STONE_SIZES.y, draw * sqrt(draw));
  let room = 0.5 - (1.0 + STONE_SHADOW_REACH) * r / side;
  let centre = 0.5 + room * (vec2f(f32((x >> 10u) & 31u), f32((x >> 15u) & 31u)) / 15.5 - 1.0);
  let v = (q.f - centre) * side;
  // The point from the centre back in the map's axes.
  let edge = min(min(q.f.x, 1.0 - q.f.x), min(q.f.y, 1.0 - q.f.y));
  return Placed(f32(x & 1023u) / 1024.0 < density, x, r, select(v, vec2f(0.8 * v.x - 0.6 * v.y, 0.6 * v.x + 0.8 * v.y), l == 1), smoothstep(0.0, CELL_INSIDE, edge));
}

// The stones `s` at least `least` m large at the pixel, seen at `e` m a pixel, of bed `bed` (linear, unlit), stream `k` of
// the world's random numbers: their colour times their coverage (with the shadow they throw as black), and that
// coverage.
fn stones(e: f32, s: Stony, least: f32, bed: vec3f, k: i32) -> vec4f {
  var out = vec4f(0.0);
  if (s.density <= STONES_LEAST || s.size <= 0.0) { return out; }
  let tone = bed * STONE_TONE;
  let snow_m = s.snow * s.snow_m;
  var size = s.size / pow(STONE_STEP, f32(STONE_OCTAVES - 1));
  // The smallest first, the larger over them.
  for (var o = rolled(STONE_OCTAVES) - 1; o >= 0; o--) {
    let density = stones_held(s, o);
    // Out of the water, where it stands in a channel's.
    var stands = 1.0;
    if (least > 0.0) { stands = ease(least, 1.5 * least, s.small * size); }
    // Drawn one by one from a speck's size (`SPECK_PX`), as its sprite from a thing's (draw.wgsl `as_thing`);
    // smaller, the share of the ground they cover, as a tone (as far as the snow leaves them).
    var one = 0.0;
    var near = 0.0;
    if (!unseen(STONE_MOST_M / SPECK_PX.x)) { one = ease(SPECK_PX.x * e, SPECK_PX.y * e, s.small * size); }
    if (stone_sprites() && !unseen(STONE_MOST_M / THING_PX.x)) { near = as_thing(s.small * size, e); }
    let high = STONE_TOP * (1.0 - s.sunk);
    let mean = 2.0 * density * stands * above_snow(snow_m, high * 0.5 * s.small * size) * PI * 0.25 * 0.7 * s.small * s.small / (STONE_GAP * STONE_GAP) * (1.0 - one);
    out = mix(out, vec4f(wet_stone(tone, 1.0, s.wet, s.worn), 1.0), mean);
    if (one > 0.0 && stands > 0.0 && density > 0.0) {
      for (var l = 0; l < 2; l++) {
        let p = placed(size, density, o, l, k, vec2f(0.0));
        let r = s.small * p.r;
        if (!p.held || dot(p.v, p.v) > STONE_REACH * STONE_REACH * r * r) { continue; }
        // Under snow deeper than it is tall, a stone does not show.
        let shows = stands * above_snow(snow_m, high * r);
        if (shows <= 0.0) { continue; }
        if (near > 0.0) { out = stone_sprite(out, p, s, e, near * shows, tone); }
        if (one > near) { out = stone_speck(out, p, s, e, (one - near) * shows, tone); }
      }
    }
    size *= STONE_STEP;
  }
  return out;
}

// A stone that stands in a flow breaks it: a thin cushion of white water piles against its face up the flow
// (`CUSHION` of its radius out), and a short V of ripples trails down the flow behind it, `WAKE_LONG` of its size
// long, spreading by `WAKE_SPREAD` of the way, its white in ripples `WAKE_RIPPLE` of its radius apart, with calm water
// right behind it (its eddy, `WAKE_EDDY` of its radius). The wake is found from its stone's cell, and fades out at
// that cell's edge (`Placed.inside`), so no wake ends on a cell's straight side.
const WAKE_LONG = 1.1; const WAKE_SPREAD = 0.45; const WAKE_EDDY = 0.8; const CUSHION = 1.15; const WAKE_RIPPLE = 0.7;

// The white water at the pixel, seen at `e` m a pixel, about the stones `s` at least `least` m large (`stones`, stream `k`)
// in a flow that runs `down` (a unit vector), 0..1: each stone's the same stone `stones` draws, found from a point up
// the flow.
fn wake(e: f32, s: Stony, least: f32, down: vec2f, k: i32) -> f32 {
  var white = 0.0;
  if (least <= 0.0) { return white; }
  var size = s.size;
  for (var o = 0; o < rolled(STONE_OCTAVES); o++) {
    let stands = ease(least, 1.5 * least, s.small * size) * as_thing(s.small * size, e);
    let density = stones_held(s, o);
    if (stands > 0.0 && density > 0.0) {
      for (var l = 0; l < 2; l++) {
        // The stone whose cell holds the point half a wake up the flow.
        let back = 0.5 * WAKE_LONG * size * down;
        let p = placed(size, density, o, l, k, -back);
        if (!p.held) { continue; }
        let r = s.small * p.r;
        let v = p.v + back;
        let along = dot(v, down) / r;
        let across = abs(dot(v, vec2f(-down.y, down.x))) / r;
        // Behind it: two trails at its sides that spread and fade, calm between them at first; before it: a rim.
        let reach = 2.0 * WAKE_LONG;
        let wide = 1.0 + WAKE_SPREAD * max(along, 0.0);
        let ripples = 0.5 + 0.5 * cos(2.0 * PI * (along + 0.5 * across) / WAKE_RIPPLE);
        let trail = (1.0 - smoothstep(0.5 * wide, wide, across)) * smoothstep(WAKE_EDDY * (1.0 - along / reach), wide, across + 0.6 * along) * (1.0 - smoothstep(0.0, reach, along)) * step(0.0, along) * ripples;
        let rim = (1.0 - smoothstep(1.0, CUSHION, length(v) / r)) * smoothstep(-0.2, -0.9, along);
        white = max(white, stands * p.inside * max(trail, rim));
      }
    }
    size /= STONE_STEP;
  }
  return white;
}
