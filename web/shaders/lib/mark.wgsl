// The marks of the plant cover (cover.wgsl): each lattice of marks lies fixed in the world, one mark a cell at a place
// and rank hashed from the cell, and what each mark is (whether a plant stands there, of which square and kind, how
// much of it shows over the snow) is a function of its cell alone (`decide`): decided once a frame for each cell in
// view (marks.wgsl), it is read by every pixel the mark may cover.

// The kinds' table's planes (canopy.ts `KIND_PLANES`), and the designs (style.ts `DESIGN`).
const CHANCE = 0; const WIDTH = 3; const NEED = 6; const ROOTS = 7; const FORM = 8; const FROM = 12; const TO = 16; const PETAL = 20; const SHAPE = 24;
const TREE = 0.0; const BUSH = 1.0;

struct Cover {
  // The draw's corner in the world, m from the map's north-west corner; metres a device pixel; 1 when angled.
  corner: vec4f,
  // The reflectance of the walls the cover builds: its stems (wasm `meta`).
  walls: vec4f,
  // Of each lattice: the row of the marks' texture its cells start at (`marks`).
  rows: vec4f,
  // Of each lattice: its cell at the draw's corner (its fraction; its whole number is `base`), its cell's side, m, and
  // the rows south of a pixel an angled plant on it may stand in.
  grid: array<vec4f, COVER_LATTICES>,
  // Of each lattice: the mip level of the field read about a pixel to see whether any of its marks stands near.
  near: array<vec4f, COVER_LATTICES>,
  base: array<vec4i, COVER_LATTICES>,
}
@group(0) @binding(1) var<uniform> cover: Cover;
@group(0) @binding(4) var kinds: texture_2d<f32>;
// The marks of the cells in view (marks.wgsl): a texel a cell, a lattice's from row `cover.rows` on and from
// `MARK_MARGIN` cells before its base on, each what `decide` decided of the mark in it.
@group(0) @binding(5) var marks: texture_2d<f32>;
// The cells a pixel looks at reach this many before the cell that holds the draw's corner.
const MARK_MARGIN = 1;
// A plant's size differs from its kind's by up to this share; a tree leans by up to `LEAN` of its height.
const SIZES = 0.3; const LEAN = 0.08;

// pcg3d (Jarzynski and Olano 2020): three random words from three.
fn pcg3d(w: vec3u) -> vec3u {
  var v = w * 1664525u + 1013904223u;
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  v ^= v >> vec3u(16u);
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  return v;
}
fn rand3(c: vec2i, k: i32) -> vec3f { return vec3f(pcg3d(vec3u(vec2u(c), u32(k)))) * (1.0 / 4294967296.0); }
// A cheaper hash for the first look at a cell (`lowbias`): one word, read as three of 11, 11 and 10 bits.
fn place(c: vec2i, k: i32) -> vec3f {
  let x = lowbias(c, k);
  return vec3f(f32(x & 2047u) / 2048.0, f32((x >> 11u) & 2047u) / 2048.0, f32(x >> 22u) / 1024.0);
}
// Plane `k` of the kinds' table at the square at column and row c, the nearest on the map.
fn kind(k: i32, c: vec2i) -> vec4f {
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let q = clamp(c, vec2i(0), n - 1);
  return textureLoad(kinds, vec2i(q.x, k * n.y + q.y), 0);
}

// What the ground at `at` leaves a plant: the share free of liquid over its roots, how wet it is and how wet its cells
// are on their mean (land.wgsl `state_at`), the share snow covers and the snow's depth, m.
struct Foot { stands: f32, wet: f32, mean: f32, snow: f32, snow_m: f32 }
fn foot(at: vec2f) -> Foot {
  let st = state_at(at, ground_at(at, false), corners(at / land.map.x));
  return Foot(above_water(st.depth), st.wet, st.mean, st.snow, st.snow_m);
}

// The share of each kind that stands where its roots need `need` m of loose ground (0: its square's ground holds it
// everywhere, canopy.ts) and `loose` m lies (land.wgsl `rooted`).
fn holds(loose: f32, need: vec4f) -> vec4f {
  return select(vec4f(rooted(loose, need.x, 0.0), rooted(loose, need.y, 0.0), rooted(loose, need.z, 0.0), rooted(loose, need.w, 0.0)), vec4f(1.0), need <= vec4f(0.0));
}

// The share of a kind that needs `need` that stands where the ground is `wet` against its cells' mean: its activity
// there over its activity at the mean, at most 1 (land.wgsl `activity`).
fn thrive(wet: f32, mean: f32, need: vec4f) -> vec4f {
  var out = vec4f(1.0);
  for (var s = 0; s < 4; s++) {
    if (need[s] <= 0.0) { continue; }
    let whole = activity(mean, need[s]);
    if (whole > 0.0) { out[s] = min(activity(wet, need[s]) / whole, 1.0); }
  }
  return out;
}

// The kind of the mark whose foot is at `at` (its squares `cs`, their kinds' chances there `chance`, `thick` over the
// patches, its ground `f`), of rank `x` among the plants that could stand there: its square (of the four) and kind, or
// -1 where none stands. Each kind of each square counts by its share at the foot; on the lattice of stands the squares'
// first kinds (their chances negated, canopy.ts `Paint`) are picked where `stand_pick` falls in their share, as the
// field far away has them, the rest elsewhere.
fn pick_kind(at: vec2f, cs: Corners, chance: array<vec4f, 4>, thick: f32, f: Foot, x: f32) -> vec2i {
  var w: array<vec4f, 4>;
  var first: array<vec4f, 4>;
  var needs: array<vec4f, 4>;
  var sum = 0.0;
  var firsts = 0.0;
  // The loose ground at the foot with its patches (rock.wgsl `loose_seen`, as the bare bed is drawn), read only where a
  // kind's roots may want more than lies there.
  var deepest = 0.0;
  for (var s = 0; s < 4; s++) {
    needs[s] = kind(ROOTS, cs.at[s]);
    deepest = max(deepest, max(max(needs[s].x, needs[s].y), max(needs[s].z, needs[s].w)));
  }
  let loose = select(0.0, loose_seen(at, 0.0), deepest > 0.0);
  for (var s = 0; s < 4; s++) {
    w[s] = abs(chance[s]) * thick * thrive(f.wet, f.mean, kind(NEED, cs.at[s])) * holds(loose, needs[s]) * f.stands;
    first[s] = select(vec4f(0.0), vec4f(1.0), chance[s] < vec4f(0.0));
    sum += dot(w[s], vec4f(1.0));
    firsts += dot(w[s], first[s]);
  }
  if (x >= sum) { return vec2i(-1); }
  // Of the first kinds or of the rest, and its place among them.
  var v = x;
  var group = false;
  if (firsts > 0.0) {
    v = stand_pick(stand_draw(at, STAND_M), x / sum) * sum;
    group = v < firsts;
    if (!group) { v -= firsts; }
  }
  var acc = 0.0;
  for (var s = 0; s < 4; s++) {
    let ws = w[s] * select(vec4f(1.0) - first[s], first[s], group);
    for (var t = 0; t < 4; t++) {
      acc += ws[t];
      if (v < acc) { return vec2i(s, t); }
    }
  }
  return vec2i(-1);
}

// The plant of kind `t` of the square at `sq` on lattice l with its foot at `c`, its size's draw `size`, its own random
// numbers `r` (its colour's draw, its lean's direction, its lean); angled no taller than `reach` lets it stand (the
// rows looked at, textures.ts `ROWS_MOST`).
fn plant_of(l: i32, sq: vec2i, t: i32, c: vec2f, size: f32, r: vec3f, reach: f32) -> Plant {
  let form = kind(FORM + t, sq);
  let first = kind(FROM + t, sq);
  let to = kind(TO + t, sq);
  // A tree's form, a bush's flowers.
  var shape = vec4f(0.0);
  var petal = vec4f(0.0);
  if (to.w == TREE) { shape = kind(SHAPE + t, sq); }
  if (to.w == BUSH) { petal = kind(PETAL + t, sq); }
  // Its own size: the plants of a stand differ, by up to `SIZES` either way of its kind's.
  let grow = 1.0 + SIZES * (2.0 * size - 1.0);
  let side = cover.grid[l].z;
  let width = kind(WIDTH + l, sq)[t] * grow;
  let fine = 1.0 - clamp((form.w - LEAF_THICK) / (LEAF_THIN - LEAF_THICK), 0.0, 1.0);
  let tall = select(form.x * grow, min(form.x * grow, reach / STAND), cover.corner.w > 0.5);
  let lean = LEAN * r.z * vec2f(cos(2.0 * PI * r.y), sin(2.0 * PI * r.y));
  let tint = vary(mix(first.rgb, to.rgb, r.x), fract(r.yz * 13.0));
  // A bush in flower or in fruit by a draw of its own (the next digits of its colour's), as its kind's share.
  let blooms = select(0.0, 1.0, fract(r.x * 16.0) < petal.w);
  return Plant(c, min(width, 2.0 * side), tall, form.y, form.z, fine, tint, first.w, fract(r * 7.0), to.w, shape.x, shape.y, petal.rgb, blooms, lean, side);
}

// What lattice l decides of the mark of its cell k (counted from its base, `cover.base`): its square and kind packed as
// (row x squares across + column) x 4 + kind (-1: none stands there, or the snow buries it), the share of it that shows
// over the snow, and the share of the ground snow covers at its foot. A function of the cell alone, so it is decided
// once a frame for each cell in view (marks.wgsl) and read by every pixel the mark may cover (cover.wgsl `lattice`).
fn decide(l: i32, k: vec2f) -> vec4f {
  let none = vec4f(-1.0, 0.0, 0.0, 0.0);
  let g = cover.grid[l];
  let side = g.z;
  let id = cover.base[l].xy + vec2i(k);
  let h = place(id, l);
  let c = (k - g.xy) * side + h.xy * side;
  let at = cover.corner.xy + c;
  let cs = corners(at / land.map.x);
  // The chances (on the lattice of stands, a square's first kind's negated: `pick_kind`).
  var chance: array<vec4f, 4>;
  var all = 0.0;
  for (var s = 0; s < 4; s++) {
    chance[s] = cs.w[s] * kind(CHANCE + l, cs.at[s]);
    all += dot(abs(chance[s]), vec4f(1.0));
  }
  // The patches thicken a stand by at most `PATCH`: a rank past that needs no look at them.
  let rank = rand3(id, l);
  if (rank.x >= all * (1.0 + PATCH)) { return none; }
  let thick = patchy(at);
  if (rank.x >= all * thick) { return none; }
  let f = foot(at);
  if (f.stands <= 0.0) { return none; }
  let pick = pick_kind(at, cs, chance, thick, f, rank.x);
  if (pick.x < 0) { return none; }
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let sq = clamp(cs.at[pick.x], vec2i(0), n - 1);
  let shows = above_snow(f.snow_m, mark_plant(l, k, sq, pick.y).h);
  if (shows <= 0.0) { return none; }
  return vec4f(f32((sq.y * n.x + sq.x) * 4 + pick.y), shows, f.snow, 0.0);
}

// The plant of kind `t` of the square at `sq` in cell k of lattice l (`decide`): at its place in the cell, of its size
// and its own random numbers, angled no taller than the rows looked at let it stand.
fn mark_plant(l: i32, k: vec2f, sq: vec2i, t: i32) -> Plant {
  let g = cover.grid[l];
  let side = g.z;
  let id = cover.base[l].xy + vec2i(k);
  let h = place(id, l);
  return plant_of(l, sq, t, (k - g.xy) * side + h.xy * side, h.z, rand3(id, l + COVER_LATTICES), g.w * side + side);
}

// The mark lattice l decided for its cell k (`decide`), from the marks' texture.
fn decided(l: i32, k: vec2f) -> vec4f {
  return textureLoad(marks, vec2i(k) + vec2i(MARK_MARGIN, MARK_MARGIN + i32(cover.rows[l])), 0);
}
