// The marks of the plant cover (cover.wgsl): each lattice of marks lies fixed in the world, one mark a cell at a place
// and rank hashed from the cell, and what each mark is (whether a plant stands there, of which square and kind, how
// much of it shows over the snow) is a function of its cell alone (`decide`): decided once a frame for each cell in
// view (marks.wgsl), it is read by every pixel the mark may cover.

// The designs (style.ts `DESIGN`).
const TREE = 0.0; const BUSH = 1.0;
// The kinds' table lays its planes this many under each other, then in the next column (canopy.ts `PLANES_DOWN`).
const PLANES_DOWN = 32;

struct Cover {
  // The draw's corner in the world, m from the map's north-west corner; metres a device pixel; 1 when angled.
  corner: vec4f,
  // The metres a pixel of the screen takes, which the detail drawn is chosen by (x; the target's pixel may be larger
  // while the map is drawn at fewer pixels, web/src/map.ts `SCALES`).
  seen: vec4f,
  // The reflectance of the walls the cover builds: its stems (wasm `meta`); and the kinds a square holds (the world's
  // variety, wasm `meta`'s `tiles.kinds`).
  walls: vec4f,
  // Of each lattice: the row of the marks' texture its cells start at (`marks`).
  rows: vec4f,
  // Of each lattice: its cell at the draw's corner (its fraction; its whole number is `base`), its cell's side, m, and
  // the rows south of a pixel an angled plant on it may stand in.
  grid: array<vec4f, COVER_LATTICES>,
  // Of each lattice: the mip level of the field read about a pixel to see whether any of its marks stands near, and the
  // cells a mark reaches from its own (canopy.ts `REACH`).
  near: array<vec4f, COVER_LATTICES>,
  base: array<vec4i, COVER_LATTICES>,
}
@group(0) @binding(1) var<uniform> cover: Cover;
@group(0) @binding(4) var kinds: texture_2d<f32>;
// The marks of the cells in view (marks.wgsl): a texel a cell, a lattice's from row `cover.rows` on and from
// `MARK_MARGIN` cells before its base on, each what `decide` decided of the mark in it as three 32-bit words, a texture
// of four bytes a texel for each (`MARK_WORDS`; bytes, which every GPU draws into): one more than its square and kind
// (0: none), and the bits of the share that shows and of the snow, so each comes back to the bit.
@group(0) @binding(5) var marks_kind: texture_2d<f32>;
@group(0) @binding(6) var marks_shows: texture_2d<f32>;
@group(0) @binding(7) var marks_snow: texture_2d<f32>;
const MARK_WORDS = 3;
// The cells a pixel looks at reach this many before the cell that holds the draw's corner (the farthest a mark reaches,
// canopy.ts `REACH`).
const MARK_MARGIN = 2;
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
// Plane `k` of the kinds' table at the square at column and row c, the nearest on the map (`PLANES_DOWN` planes a
// column).
fn kind(k: i32, c: vec2i) -> vec4f {
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let q = clamp(c, vec2i(0), n - 1);
  return textureLoad(kinds, vec2i((k / PLANES_DOWN) * n.x + q.x, (k % PLANES_DOWN) * n.y + q.y), 0);
}

// The kinds' table's planes (canopy.ts `planesOf`) for squares of `kinds_n` kinds: of the planes of one number a kind,
// that of group g of four (and of lattice l); of the planes of a kind's four numbers, that of kind t.
fn kinds_n() -> i32 { return i32(cover.walls.w); }
fn groups() -> i32 { return (kinds_n() + 3) / 4; }
fn chance_plane(l: i32, g: i32) -> i32 { return l * groups() + g; }
fn width_plane(l: i32, g: i32) -> i32 { return (3 + l) * groups() + g; }
fn need_plane(g: i32) -> i32 { return 6 * groups() + g; }
fn roots_plane(g: i32) -> i32 { return 7 * groups() + g; }
fn form_plane(t: i32) -> i32 { return 8 * groups() + t; }
fn from_plane(t: i32) -> i32 { return 8 * groups() + kinds_n() + t; }
fn to_plane(t: i32) -> i32 { return 8 * groups() + 2 * kinds_n() + t; }
fn petal_plane(t: i32) -> i32 { return 8 * groups() + 3 * kinds_n() + t; }
fn shape_plane(t: i32) -> i32 { return 8 * groups() + 4 * kinds_n() + t; }

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

// The chances on lattice l of the kinds of group g of square s of the four about a foot (`cs`), times its share there
// (on the lattice of stands, a square's first kind's negated: `pick_kind`).
fn chance_at(l: i32, cs: Corners, s: i32, g: i32) -> vec4f { return cs.w[s] * kind(chance_plane(l, g), cs.at[s]); }

// What the kinds of group g of square s count for at a foot of ground `f` where `loose` m of loose ground lies, their
// chances `chance`, `thick` over the patches.
fn weight(cs: Corners, s: i32, g: i32, chance: vec4f, thick: f32, f: Foot, loose: f32) -> vec4f {
  return abs(chance) * thick * thrive(f.wet, f.mean, kind(need_plane(g), cs.at[s])) * holds(loose, kind(roots_plane(g), cs.at[s])) * f.stands;
}

// The kind of the mark of lattice l whose foot is at `at` (its squares `cs`, `thick` over the patches, its ground `f`),
// of rank `x` among the plants that could stand there: its square (of the four) and kind, or -1 where none stands. Each
// kind of each square counts by its share at the foot; on the lattice of stands the squares' first kinds (their chances
// negated, canopy.ts `Paint`) are picked where `stand_pick` falls in their share, as the field far away has them, the
// rest elsewhere. Whether a mark stands is its rank `x` against the kinds' shares together; which kind, a race of their
// own draws (`RACE_STREAM`) for the cell `id`.
const RACE_STREAM = 175;
// A kind's form and how surely it is drawn share one number: form + this x sure (canopy.ts `SURE_PACK`).
const SURE_PACK = 0.5;
// How closed a kind's crown is and the share of its leaves left share one number: the leaves left in steps of
// 1 / `LEFT_STEPS` + how closed (canopy.ts `LEFT_STEPS`).
const LEFT_STEPS = 100.0;
fn pick_kind(l: i32, at: vec2f, cs: Corners, thick: f32, f: Foot, x: f32, id: vec2i) -> vec2i {
  let groups = groups();
  var sum = 0.0;
  var firsts = 0.0;
  // The loose ground at the foot with its patches (rock.wgsl `loose_seen`, as the bare bed is drawn), read only where a
  // kind's roots may want more than lies there.
  var deepest = 0.0;
  for (var s = 0; s < 4; s++) {
    for (var g = 0; g < groups; g++) {
      let needs = kind(roots_plane(g), cs.at[s]);
      deepest = max(deepest, max(max(needs.x, needs.y), max(needs.z, needs.w)));
    }
  }
  let loose = select(0.0, loose_seen(at, 0.0), deepest > 0.0);
  for (var s = 0; s < 4; s++) {
    for (var g = 0; g < groups; g++) {
      let chance = chance_at(l, cs, s, g);
      let w = weight(cs, s, g, chance, thick, f, loose);
      sum += dot(w, vec4f(1.0));
      firsts += dot(w, select(vec4f(0.0), vec4f(1.0), chance < vec4f(0.0)));
    }
  }
  if (x >= sum) { return vec2i(-1); }
  // Of the first kinds or of the rest.
  var group = false;
  if (firsts > 0.0) { group = stand_pick(stand_draw(at, STAND_M), x / sum) * sum < firsts; }
  // Of those, the kind whose own draw for the cell, by its weight, comes first (a weighted race: -ln u / weight), so each
  // kind keeps its marks' places as the others' shares and order change (its draw keyed by its starting kind, the
  // label it keeps, and its square).
  var best = vec2i(-1);
  var first_at = 1e30;
  for (var s = 0; s < 4; s++) {
    for (var g = 0; g < groups; g++) {
      let chance = chance_at(l, cs, s, g);
      let first = select(vec4f(0.0), vec4f(1.0), chance < vec4f(0.0));
      let ws = weight(cs, s, g, chance, thick, f, loose) * select(vec4f(1.0) - first, first, group);
      for (var t = 0; t < 4; t++) {
        if (ws[t] <= 0.0) { continue; }
        let root = u32(max(kind(shape_plane(4 * g + t), cs.at[s]).y, 0.0) + 0.5);
        let u = (f32(lowbias(id, RACE_STREAM + 4 * i32(root) + s) >> 8u) + 0.5) / 16777216.0;
        let at_t = -log(u) / ws[t];
        if (at_t < first_at) { first_at = at_t; best = vec2i(s, 4 * g + t); }
      }
    }
  }
  return best;
}

// The plant of kind `t` of the square at `sq` on lattice l with its foot at `c`, its size's draw `size`, its own random
// numbers `r` (its colour's draw, its lean's direction, its lean); angled no taller than `up` m north of its foot lets it
// stand (the rows looked at, textures.ts `ROWS_MOST`); no wider than its lattice's reach each way.
fn plant_of(l: i32, sq: vec2i, t: i32, c: vec2f, size: f32, r: vec3f, up: f32) -> Plant {
  let form = kind(form_plane(t), sq);
  // Its leaves left this season, and how closed its crown is with them (how dense times the leaves left).
  let left = floor(form.z) / LEFT_STEPS;
  let closed = fract(form.z) * left;
  let first = kind(from_plane(t), sq);
  let to = kind(to_plane(t), sq);
  // A tree's form, a bush's flowers.
  var shape = vec4f(0.0);
  var petal = vec4f(0.0);
  if (to.w == TREE) {
    // Its form and how surely, packed as one number (canopy.ts `SURE_PACK`).
    let packed = kind(shape_plane(t), sq);
    shape = vec4f(floor(packed.x), fract(packed.x) / SURE_PACK, packed.z, packed.w);
    // A mark of the trees' lattice that stands for several plants is their cluster, a crown of knots, each a plant's
    // crown (crown.wgsl `knots`): its kind's form fades into the round crown as the plants pass one, and the cluster
    // is drawn surely from two.
    let cluster = select(0.0, clamp(shape.z - 1.0, 0.0, 1.0), l == STANDS);
    if (cluster > 0.5) { shape = vec4f(f32(LOBED), 2.0 * cluster - 1.0, 0.0, 0.0); } else { shape.y *= 1.0 - 2.0 * cluster; }
  }
  if (to.w == BUSH) { petal = kind(petal_plane(t), sq); }
  // How dry the kind's ground (form.ts `formOf`), for the drawing's choice of plant.
  let dry = kind(shape_plane(t), sq).w;
  // Its own size: the plants of a stand differ, by up to `SIZES` either way of its kind's.
  let grow = 1.0 + SIZES * (2.0 * size - 1.0);
  let side = cover.grid[l].z;
  let reach = cover.near[l].y * side;
  let width = kind(width_plane(l, t / 4), sq)[t % 4] * grow;
  let fine = 1.0 - clamp((form.w - LEAF_THICK) / (LEAF_THIN - LEAF_THICK), 0.0, 1.0);
  let tall = select(form.x * grow, min(form.x * grow, up / STAND), cover.corner.w > 0.5);
  let lean = LEAN * r.z * vec2f(cos(2.0 * PI * r.y), sin(2.0 * PI * r.y));
  let tint = vary(mix(first.rgb, to.rgb, r.x), fract(r.yz * 13.0));
  // A bush in flower or in fruit by a draw of its own (the next digits of its colour's), as its kind's share.
  let blooms = select(0.0, 1.0, fract(r.x * 16.0) < petal.w);
  return Plant(c, min(width, 2.0 * reach), tall, form.y, closed, fine, tint, first.w, fract(r * 7.0), to.w, shape.x, shape.y, petal.rgb, blooms, lean, reach, dry, left);
}

// What lattice l decides of the mark of its cell k (counted from its base, `cover.base`): its square and kind packed as
// (row x squares across + column) x the kinds a square holds (`kinds_n`) + kind (-1: none stands there, or the snow
// buries it, or its lattice's marks there take too few pixels to show), the share of it that shows (over the snow, and
// as a thing by the width of its lattice's marks at its foot, draw.wgsl `as_thing`: the field far away, cover_field.wgsl
// `lattice_field`, is drawn by the same width where they do not), and the share of the ground snow covers at its foot.
// A function of the cell alone, so it is decided once a frame for each cell in view (marks.wgsl) and read by every
// pixel the mark may cover (cover.wgsl `lattice`).
fn decide(l: i32, k: vec2f) -> vec4f {
  let none = vec4f(-1.0, 0.0, 0.0, 0.0);
  let g = cover.grid[l];
  let side = g.z;
  let id = cover.base[l].xy + vec2i(k);
  let h = place(id, l);
  let c = (k - g.xy) * side + h.xy * side;
  let at = cover.corner.xy + c;
  let thing = as_thing(field_at(at, WIDE_LAYER, 0.0)[l], cover.seen.x);
  if (thing <= 0.0) { return none; }
  let cs = corners(at / land.map.x);
  // The chances (`chance_at`).
  var all = 0.0;
  for (var s = 0; s < 4; s++) {
    for (var g = 0; g < groups(); g++) { all += dot(abs(chance_at(l, cs, s, g)), vec4f(1.0)); }
  }
  // The patches thicken a stand by at most `PATCH`: a rank past that needs no look at them.
  let rank = rand3(id, l);
  if (rank.x >= all * (1.0 + PATCH)) { return none; }
  let thick = patchy(at);
  if (rank.x >= all * thick) { return none; }
  let f = foot(at);
  if (f.stands <= 0.0) { return none; }
  let pick = pick_kind(l, at, cs, thick, f, rank.x, id);
  if (pick.x < 0) { return none; }
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let sq = clamp(cs.at[pick.x], vec2i(0), n - 1);
  let shows = above_snow(f.snow_m, mark_plant(l, k, sq, pick.y).h) * thing;
  if (shows <= 0.0) { return none; }
  return vec4f(f32((sq.y * n.x + sq.x) * kinds_n() + pick.y), shows, f.snow, 0.0);
}

// The plant of kind `t` of the square at `sq` in cell k of lattice l (`decide`): at its place in the cell, of its size
// and its own random numbers, angled no taller than the rows looked at let it stand.
fn mark_plant(l: i32, k: vec2f, sq: vec2i, t: i32) -> Plant {
  let g = cover.grid[l];
  let side = g.z;
  let id = cover.base[l].xy + vec2i(k);
  let h = place(id, l);
  return plant_of(l, sq, t, (k - g.xy) * side + h.xy * side, h.z, rand3(id, l + COVER_LATTICES), (g.w + cover.near[l].y) * side);
}

// A byte is drawn this far past its own level, in levels, so it is stored as itself whether the GPU rounds a colour to
// the nearest level or down.
const BYTE_LIFT = 0.25;

// Word `w` of mark `m` (`decide`) as its four bytes, each a level of 255 (marks.wgsl).
fn mark_word(m: vec4f, w: i32) -> vec4f {
  var x = u32(m.x + 1.0);
  if (w == 1) { x = bitcast<u32>(m.y); } else if (w == 2) { x = bitcast<u32>(m.z); }
  return (vec4f(f32(x & 255u), f32((x >> 8u) & 255u), f32((x >> 16u) & 255u), f32(x >> 24u)) + BYTE_LIFT) / 255.0;
}

// The word of four bytes, each a level of 255, read from a texel (`mark_word`).
fn word_of(b: vec4f) -> u32 {
  let v = vec4u(round(b * 255.0));
  return v.x | (v.y << 8u) | (v.z << 16u) | (v.w << 24u);
}

// The mark lattice l decided for its cell k (`decide`), from the marks' textures: its share and snow read only where a
// plant stands.
fn decided(l: i32, k: vec2f) -> vec4f {
  let t = vec2i(k) + vec2i(MARK_MARGIN, MARK_MARGIN + i32(cover.rows[l]));
  let code = word_of(textureLoad(marks_kind, t, 0));
  if (code == 0u) { return vec4f(-1.0, 0.0, 0.0, 0.0); }
  return vec4f(f32(code - 1u), bitcast<f32>(word_of(textureLoad(marks_shows, t, 0))), bitcast<f32>(word_of(textureLoad(marks_snow, t, 0))), 0.0);
}
