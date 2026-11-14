// uses: draw.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl
// The takers over the live map (web/src/herds.ts), one by one, as many as each slot of each square holds, in herds.
// The field holds no place inside a cell, so each member stands at a place fixed by its square, slot and number: a
// square is cut into k x k parts, k the least whose square holds the slot's herds, and a fixed random order of the
// parts (a permutation keyed by the square and the slot, `permute`) gives the first of them one herd each, so exactly
// as many herds stand as the slot holds. A herd is the slot's members shared evenly, its adults and its young each
// shared evenly too, at most `herd.x` to a herd, at a random place in its part; its members stand about it on a
// sunflower's spiral (Vogel 1979), `herd.y` of their lengths apart (or closer, where the herds are crowded). Nothing
// depends on the view: a member stands still while its slot's counts stand still. A member stands on land only (not
// in liquid that stands over the ground, nor buried by snow deeper than it is tall: land.wgsl `state_at` at its herd's
// place). It is drawn at its body's length where that takes enough pixels, and no shorter than `look.zw` pixels;
// single members show only from where one takes `look.x` of a pixel. Farther out, each herd is a soft mark as wide as
// the ground it stands on, from where that takes `herd.z` pixels. Points are in metres from the map's north-west
// corner.

struct Herds {
  // The draw's corner in the world, m from the map's north-west corner; metres a device pixel; slots a square.
  corner: vec4f,
  // The pixels a body's length takes from which members start to show and show fully, and the least length an adult
  // and a young one are drawn, px (herds.ts `SHOW_PX`, `LEAST_PX`).
  look: vec4f,
  // The most members a herd holds, how many of their lengths apart they stand, and the pixels a herd's radius takes
  // from which its mark starts to show and shows fully (herds.ts `HERD_MOST`, `HERD_SPACE`, `MARK_PX`).
  herd: vec4f,
}
@group(0) @binding(1) var<uniform> herds: Herds;
// Of each slot of each square (herds.ts `set`), two planes of a texel a square, slot after slot: its adults and young,
// and an adult's and a young one's length, m; its colour (a reflectance) and its height, m.
@group(0) @binding(4) var herd_table: texture_2d<f32>;
const PARTS = 2;

// The sunflower's angle between one member and the next, radians: the golden angle.
const GOLDEN = 2.39996;
// A herd's mark is as strong as this at its middle, where single members do not show, in its members' colour this
// much darker (the ground they tread and shade).
const MARK = 0.45; const MARK_DARK = 0.6;
// A herd keeps from its part's edges by its spread, but by no more than this share of the part: crowded herds lie at
// random in their parts too, not on the parts' lattice, and reach into the parts beside them.
const PART_KEEP = 0.25;

// A dot is as wide as a body of this share of its length's width would cover: the dot's area is the body's (a body
// about a third as wide as long).
const DOT_WIDE = 0.58;
// The steps a permutation's walk takes at most to come back into its domain (`permute`): each step lands in it with a
// chance over a half.
const WALK_MOST = 64;

// A slot of the square at sq: its adults and young (whole: never more than it holds), and an adult's and a young
// one's length, m; and its colour and its height, m, read only where one of its groups lies near a pixel (`plane` 1).
struct Slot { adults: u32, young: u32, len: f32, young_len: f32 }
fn plane(x: i32, part: i32, sq: vec2i) -> vec4f { return textureLoad(herd_table, vec2i(sq.x, (PARTS * x + part) * i32(land.map.z) + sq.y), 0); }
fn slot_of(x: i32, sq: vec2i) -> Slot {
  let a = plane(x, 0, sq);
  return Slot(u32(floor(a.x)), u32(floor(a.y)), a.z, a.w);
}

// The place of q among 0..m-1 in an order fixed by `key`: a permutation of them. Each step is a bijection of the
// numbers below the least power of two that holds m (an odd multiplier, an addition and a shift's xor, each modulo it);
// a number that lands at m or above steps on until it comes back (cycle walking; Black and Rogaway 2002).
fn permute(q: u32, m: u32, key: u32) -> u32 {
  if (m <= 1u) { return q; }
  // (GLSL ES 3.00 has no count of leading bits.)
  var bits = 1u;
  while ((1u << bits) < m) { bits += 1u; }
  let mask = (1u << bits) - 1u;
  let shift = (bits + 1u) / 2u;
  var x = q;
  for (var i = 0; i < WALK_MOST; i++) {
    x = ((x ^ (key & mask)) * ((key >> 8u) | 1u)) & mask;
    x ^= x >> shift;
    x = (x * 0x2c1b3c6du + (key >> 16u)) & mask;
    x ^= x >> shift;
    if (x < m) { return x; }
  }
  return x;
}

// A random number 0..1 of word w's 16 bits from bit `at`.
fn part_of(w: u32, at: u32) -> f32 { return f32((w >> at) & 0xffffu) / 65536.0; }

// Member of colour rgb with its centre at d (m from the pixel), drawn `len` m long, over dst, at e m a pixel: a dot of its body's area, lit from the north-west as a low dome, its rim darker.
fn member(dst: ptr<function, vec4f>, rgb: vec3f, d: vec2f, len: f32, e: f32) {
  let r = 0.5 * DOT_WIDE * len;
  let rho = length(d) / r;
  let a = clamp((r - length(d)) / e + 0.5, 0.0, 1.0);
  if (a <= 0.0) { return; }
  let n = vec3f(-d / r, sqrt(max(1.0 - rho * rho, 0.0)));
  let lit = max(dot(n, LIGHT), 0.0) / LIGHT.z;
  over(dst, mark_colour(rgb, lit, rim(rho, 1.0, r, e)), a);
}

// The share of a member whose group stands at `at` that shows: none in standing liquid, less as snow buries it.
fn standing_at(at: vec2f, tall: f32) -> f32 {
  let st = state_at(at, ground_at(at, false), corners(at / land.map.x));
  return above_water(st.depth) * above_snow(st.snow_m, tall);
}

// The members of slot x of the square at sq that may reach the pixel at `u` (m from the square's corner), at e m a
// pixel, over dst; or, where they are too small to show, their herds' marks.
fn slot_members(dst: ptr<function, vec4f>, x: i32, sq: vec2i, u: vec2f, e: f32) {
  let s = slot_of(x, sq);
  let all = s.adults + s.young;
  if (all == 0u) { return; }
  let most = u32(herds.herd.x);
  let groups = (all + most - 1u) / most;
  var k = u32(ceil(sqrt(f32(groups))));
  if (k * k < groups) { k += 1u; }
  if (k > 1u && (k - 1u) * (k - 1u) >= groups) { k -= 1u; }
  let side = land.map.x / f32(k);
  // A herd as drawn (its members' lengths, how far apart they stand, how far it reaches) and in the world (how far it
  // spreads, by which its mark is drawn and it keeps from its part's edges, so its place does not move with the view);
  // each no farther than a part, so a pixel looks at three parts each way at most.
  let len = max(s.len, herds.look.z * e);
  let young_len = max(s.young_len, herds.look.w * e);
  let full = sqrt(f32(min(all, most)));
  let apart = min(herds.herd.y * len, max(0.5 * side - len, 0.0) / full);
  let reach = min(apart * full + len, side);
  let spread = min(herds.herd.y * s.len * full + s.len, side);
  let keep = min(spread, PART_KEEP * side);
  let fade = ease(herds.look.x, herds.look.y, s.len / e);
  let marked = (1.0 - fade) * ease(herds.herd.z, herds.herd.w, spread / e);
  if (fade <= 0.0 && marked <= 0.0) { return; }
  let key = lowbias(sq, x);
  let near = max(reach, spread);
  let lo = vec2i(clamp(floor((u - near) / side), vec2f(0.0), vec2f(f32(k - 1u))));
  let hi = vec2i(clamp(floor((u + near) / side), vec2f(0.0), vec2f(f32(k - 1u))));
  for (var cy = lo.y; cy <= hi.y; cy++) {
    for (var cx = lo.x; cx <= hi.x; cx++) {
      let q = u32(cy) * k + u32(cx);
      let rank = permute(q, k * k, key);
      if (rank >= groups) { continue; }
      let w = lowbias(vec2i(i32(q), i32(key)), x);
      let at = vec2f(f32(cx), f32(cy)) * side + keep + vec2f(part_of(w, 0u), part_of(w, 16u)) * (side - 2.0 * keep);
      let d = at - u;
      if (length(d) > near + e) { continue; }
      // Its members, adults and young, each the slot's shared evenly over its herds.
      let members = all / groups + select(0u, 1u, rank < all % groups);
      let adults = s.adults / groups + select(0u, 1u, rank < s.adults % groups);
      let looks = plane(x, 1, sq);
      var g = vec4f(0.0);
      if (marked > 0.0) {
        let r = spread * sqrt(f32(members) / f32(min(all, most)));
        over(&g, looks.rgb * MARK_DARK, MARK * marked * (1.0 - smoothstep(0.0, r + e, length(d))));
      }
      if (fade > 0.0 && length(d) <= reach + e) {
        let turn = 2.0 * PI * part_of(w, 8u);
        for (var j = 0u; j < members; j++) {
          let a = turn + GOLDEN * f32(j);
          let off = apart * sqrt(f32(j) + 0.5) * vec2f(cos(a), sin(a));
          let one = select(young_len, len, j < adults);
          var m = vec4f(0.0);
          member(&m, looks.rgb, d + off, one, e);
          g = m * fade + g * (1.0 - m.a * fade);
        }
      }
      if (g.a <= 0.0) { continue; }
      let shows = standing_at(vec2f(sq) * land.map.x + at, looks.w);
      *dst = g * shows + *dst * (1.0 - g.a * shows);
    }
  }
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let e = herds.corner.z;
  let p = herds.corner.xy + (v.px - draw.rect.xy) * e;
  let sq = vec2i(floor(p / land.map.x));
  if (any(sq < vec2i(0)) || any(sq >= vec2i(i32(land.map.y), i32(land.map.z)))) { return vec4f(0.0); }
  let u = p - vec2f(sq) * land.map.x;
  var dst = vec4f(0.0);
  for (var x = 0; x < i32(herds.corner.w); x++) { slot_members(&dst, x, sq, u, e); }
  if (dst.a <= 0.0) { return vec4f(0.0); }
  return vec4f(painted(srgb(dst.rgb / dst.a)), 1.0) * dst.a;
}
