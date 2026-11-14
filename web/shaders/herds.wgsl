// uses: draw.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl
// The takers over the live map (web/src/herds.ts), one by one, as many as each slot of each square holds. The field
// holds no place inside a cell, so each member stands at a place fixed by its square, slot and number: a square is cut
// into k x k parts, k the least whose square holds the slot's groups, and a fixed random order of the parts (a
// permutation keyed by the square and the slot, `permute`) gives the first of them one group each, so exactly as many
// groups stand as the slot holds. A group is an adult with its share of the slot's young beside it (young without an
// adult where the slot holds too few adults to lead them, `YOUNG_MOST`), at a random place in its part. Nothing depends
// on the view: a member stands still while its slot's counts stand still. A member stands on land only (not in liquid
// that stands over the ground, nor buried by snow deeper than it is tall: land.wgsl `state_at` at its group's place).
// It is drawn at its body's length where that takes enough pixels, and no shorter than `look.zw` pixels; single
// members show only from where one takes `look.x` of a pixel. Points are in metres from the map's north-west corner.

struct Herds {
  // The draw's corner in the world, m from the map's north-west corner; metres a device pixel; slots a square.
  corner: vec4f,
  // The pixels a body's length takes from which members start to show and show fully, and the least length an adult
  // and a young one are drawn, px (herds.ts `SHOW_PX`, `LEAST_PX`).
  look: vec4f,
}
@group(0) @binding(1) var<uniform> herds: Herds;
// Of each slot of each square (herds.ts `set`), two planes of a texel a square, slot after slot: its adults and young,
// and an adult's and a young one's length, m; its colour (a reflectance) and its height, m.
@group(0) @binding(4) var herd_table: texture_2d<f32>;
const PARTS = 2;

// A group holds at most this many young: a slot with more young than its adults could lead in groups this large has
// groups of young alone.
const YOUNG_MOST = 6u;
// A young one stands this many of its adult's drawn lengths from it, nearer or farther by up to `YOUNG_SPREAD` of that.
const YOUNG_AWAY = 1.1; const YOUNG_SPREAD = 0.2;
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
// pixel, over dst.
fn slot_members(dst: ptr<function, vec4f>, x: i32, sq: vec2i, u: vec2f, e: f32) {
  let s = slot_of(x, sq);
  let fade = ease(herds.look.x, herds.look.y, s.len / e);
  if (s.adults + s.young == 0u || fade <= 0.0) { return; }
  let groups = max(s.adults, (s.young + YOUNG_MOST - 1u) / YOUNG_MOST);
  var k = u32(ceil(sqrt(f32(groups))));
  if (k * k < groups) { k += 1u; }
  if (k > 1u && (k - 1u) * (k - 1u) >= groups) { k -= 1u; }
  let side = land.map.x / f32(k);
  // The drawn lengths, the group's reach about its adult as drawn, and in the world (by which it keeps from its part's
  // edges, so its place does not move with the view).
  let len = max(s.len, herds.look.z * e);
  let young_len = max(s.young_len, herds.look.w * e);
  let away = YOUNG_AWAY * len;
  let reach = min(0.5 * len + select(0.0, away + 0.5 * young_len, s.young > 0u), 0.5 * side);
  let keep = min((0.5 + YOUNG_AWAY) * s.len + s.young_len, 0.5 * side);
  let key = lowbias(sq, x);
  let lo = vec2i(clamp(floor((u - reach) / side), vec2f(0.0), vec2f(f32(k - 1u))));
  let hi = vec2i(clamp(floor((u + reach) / side), vec2f(0.0), vec2f(f32(k - 1u))));
  for (var cy = lo.y; cy <= hi.y; cy++) {
    for (var cx = lo.x; cx <= hi.x; cx++) {
      let q = u32(cy) * k + u32(cx);
      let rank = permute(q, k * k, key);
      if (rank >= groups) { continue; }
      let w = lowbias(vec2i(i32(q), i32(key)), x);
      let at = vec2f(f32(cx), f32(cy)) * side + keep + vec2f(part_of(w, 0u), part_of(w, 16u)) * (side - 2.0 * keep);
      let d = at - u;
      if (length(d) > reach + e) { continue; }
      var g = vec4f(0.0);
      let looks = plane(x, 1, sq);
      let young = s.young / groups + select(0u, 1u, rank < s.young % groups);
      for (var j = 0u; j < young; j++) {
        // (Its random word from a stream past the slots'.)
        let r = lowbias(vec2i(i32(q), i32(j)), x + i32(herds.corner.w));
        let turn = 2.0 * PI * part_of(r, 0u);
        let off = away * (1.0 + YOUNG_SPREAD * (2.0 * part_of(r, 16u) - 1.0)) * vec2f(cos(turn), sin(turn));
        member(&g, looks.rgb, d + off, young_len, e);
      }
      if (rank < s.adults) { member(&g, looks.rgb, d, len, e); }
      if (g.a <= 0.0) { continue; }
      let shows = fade * standing_at(vec2f(sq) * land.map.x + at, looks.w);
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
