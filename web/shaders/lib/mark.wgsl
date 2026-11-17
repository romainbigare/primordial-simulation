// The marks of the plant cover (cover.wgsl): each lattice of marks lies fixed in the world, one mark a cell at a place
// and rank hashed from the cell (cells.wgsl), and what each mark is (whether a plant stands there, of which square and
// kind, how much of it shows over the snow, the box it covers) is a function of its cell alone (`decide`): decided once
// a frame for each cell in view (marks.wgsl), it is read by every pixel the mark may cover.

// The designs (style.ts `DESIGN`).
const TREE = 0.0; const BUSH = 1.0;
// The kinds' table lays its planes this many under each other, then in the next column (canopy.ts `PLANES_DOWN`).
const PLANES_DOWN = 32;

@group(0) @binding(4) var kinds: texture_2d<f32>;
// A plant's size differs from its group's by up to this share (canopy.ts `SIZE_SPREAD`); a tree leans by up to `LEAN`
// of its height.
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
// Plane `k` of the kinds' table at the square at column and row c, the nearest on the map (`PLANES_DOWN` planes a
// column).
fn kind(k: i32, c: vec2i) -> vec4f {
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let q = clamp(c, vec2i(0), n - 1);
  return textureLoad(kinds, vec2i((k / PLANES_DOWN) * n.x + q.x, (k % PLANES_DOWN) * n.y + q.y), 0);
}

// The kinds' table's planes (canopy.ts `planesOf`) for squares of `kinds_n` kinds: of the planes of one number a kind,
// that of group g of four (and of lattice l); of the planes of a kind's four numbers, that of kind t (and of lattice l).
fn kinds_n() -> i32 { return i32(cover.walls.w); }
fn groups() -> i32 { return (kinds_n() + 3) / 4; }
fn chance_plane(l: i32, g: i32) -> i32 { return l * groups() + g; }
fn need_plane(g: i32) -> i32 { return COVER_LATTICES * groups() + g; }
fn roots_plane(g: i32) -> i32 { return (COVER_LATTICES + 1) * groups() + g; }
fn label_plane(g: i32) -> i32 { return (COVER_LATTICES + 2) * groups() + g; }
fn kind_plane(at: i32, t: i32) -> i32 { return (COVER_LATTICES + 3) * groups() + at * kinds_n() + t; }
fn form_plane(t: i32) -> i32 { return kind_plane(0, t); }
fn from_plane(t: i32) -> i32 { return kind_plane(1, t); }
fn to_plane(t: i32) -> i32 { return kind_plane(2, t); }
fn petal_plane(t: i32) -> i32 { return kind_plane(3, t); }
fn height_plane(t: i32) -> i32 { return kind_plane(4, t); }
fn stem_plane(t: i32) -> i32 { return kind_plane(5, t); }
fn clump_plane(t: i32) -> i32 { return kind_plane(6, t); }
fn cut_plane(l: i32, t: i32) -> i32 { return kind_plane(7 + l, t); }
fn width_plane(l: i32, t: i32) -> i32 { return kind_plane(7 + COVER_LATTICES + l, t); }
// A kind's plants come in up to this many groups by size, the large first (canopy.ts `GROUPS`): a mark is of one.
const GROUPS = 4;

// The group of kind `t` of the square at `sq` a mark on lattice l is of, by its rank `u` among the marks that stand
// there (0..1): the lowest ranks are the large plants', then the next group's, each by its share of the kind's marks
// there (canopy.ts `cut`), so a group's marks keep their places while the groups after it come and go.
fn group_of(l: i32, t: i32, sq: vec2i, u: f32) -> i32 {
  let cut = kind(cut_plane(l, t), sq);
  return i32(dot(select(vec4f(0.0), vec4f(1.0), vec4f(u) >= cut.xyz.xyzz), vec4f(1.0, 1.0, 1.0, 0.0)));
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

// The chances on lattice l of the kinds of group g of square s of the four about a foot (`cs`), times its share there
// (on the lattice of stands, a square's first kind's negated: `pick_kind`).
fn chance_at(l: i32, cs: Corners, s: i32, g: i32) -> vec4f { return cs.w[s] * kind(chance_plane(l, g), cs.at[s]); }

// What the kinds of group g of square s count for at a foot of ground `f` where `loose` m of loose ground lies, their
// chances `chance`, `thick` over the patches.
fn weight(cs: Corners, s: i32, g: i32, chance: vec4f, thick: f32, f: Foot, loose: f32) -> vec4f {
  return abs(chance) * thick * thrive(f.wet, f.mean, kind(need_plane(g), cs.at[s])) * holds(loose, kind(roots_plane(g), cs.at[s])) * f.stands;
}

// The kind of the mark of lattice l whose foot is at `at` (its squares `cs`, `thick` over the patches, its ground `f`),
// of rank `x.x` among the plants that could stand there and its own draw `x.y`: its square (of the four), kind and group
// (`group_of`), or -1 where none stands. Each kind of each square counts by its share at the foot; on the lattice of
// stands the squares' first kinds (their chances negated, canopy.ts `Paint`) are picked where `stand_pick` of its own
// draw falls in their share, as the field far away has them, the rest elsewhere. Whether a mark stands is its rank
// against the kinds' shares together; which kind, a race of their own draws (`RACE_STREAM`) for the cell `id`; which
// group, its rank among those that stand.
const RACE_STREAM = 175;
// A kind's form and how surely it is drawn share one number: form + this x sure (canopy.ts `SURE_PACK`).
const SURE_PACK = 0.5;
// How closed a kind's crown is and the share of its leaves left share one number: the leaves left in steps of
// 1 / `LEFT_STEPS` + how closed (canopy.ts `LEFT_STEPS`).
const LEFT_STEPS = 100.0;
fn pick_kind(l: i32, at: vec2f, cs: Corners, thick: f32, f: Foot, x: vec2f, id: vec2i) -> vec3i {
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
  if (x.x >= sum) { return vec3i(-1); }
  // Of the first kinds or of the rest.
  var group = false;
  if (firsts > 0.0) { group = stand_pick(stand_draw(at, STAND_M), x.y) * sum < firsts; }
  // Of those, the kind whose own draw for the cell, by its weight, comes first (a weighted race: -ln u / weight), so each
  // kind keeps its marks' places as the others' shares and order change (its draw keyed by its slot in its cell's
  // cover, the label it keeps, and its square).
  var best = vec3i(-1);
  var first_at = 1e30;
  for (var s = 0; s < 4; s++) {
    for (var g = 0; g < groups; g++) {
      let chance = chance_at(l, cs, s, g);
      let first = select(vec4f(0.0), vec4f(1.0), chance < vec4f(0.0));
      let ws = weight(cs, s, g, chance, thick, f, loose) * select(vec4f(1.0) - first, first, group);
      for (var t = 0; t < 4; t++) {
        if (ws[t] <= 0.0) { continue; }
        let slot = u32(kind(label_plane(g), cs.at[s])[t] + 0.5);
        let u = (f32(lowbias(id, RACE_STREAM + 4 * i32(slot) + s) >> 8u) + 0.5) / 16777216.0;
        let at_t = -log(u) / ws[t];
        if (at_t < first_at) { first_at = at_t; best = vec3i(s, 4 * g + t, 0); }
      }
    }
  }
  if (best.x >= 0) { best.z = group_of(l, best.y, cs.at[best.x], x.x / sum); }
  return best;
}

// The plant of group `gr` of kind `t` of the square at `sq` on lattice l with its foot at `c`, its size's draw `size`,
// its own random numbers `r` (its colour's draw, its lean's direction, its lean); angled no taller than `up` m north of
// its foot lets it stand (the rows looked at, textures.ts `ROWS_MOST`); no wider than its lattice's reach each way.
fn plant_of(l: i32, sq: vec2i, t: i32, gr: i32, c: vec2f, size: f32, r: vec3f, up: f32) -> Plant {
  let form = kind(form_plane(t), sq);
  // Its leaves left this season, and how closed its crown is with them (how dense times the leaves left).
  let left = floor(form.x) / LEFT_STEPS;
  let closed = fract(form.x) * left;
  let first = kind(from_plane(t), sq);
  let to = kind(to_plane(t), sq);
  // Its own size: the plants of a stand differ, by up to `SIZES` either way of its group's.
  let grow = 1.0 + SIZES * (2.0 * size - 1.0);
  let height = kind(height_plane(t), sq)[gr] * grow;
  // A tree from the height its kind's plants are trees from (style.ts `treeFrom`), a bush below.
  let design = select(BUSH, TREE, height >= to.w);
  // A tree's form, a bush's flowers.
  var shape = vec4f(0.0);
  var petal = vec4f(0.0);
  if (design == TREE) {
    // Its form and how surely, packed as one number (canopy.ts `SURE_PACK`), and the plants a mark stands for.
    shape = vec4f(floor(form.z), fract(form.z) / SURE_PACK, kind(clump_plane(t), sq)[gr], 0.0);
    // A mark of the trees' lattice that stands for several plants is their cluster, a crown of knots, each a plant's
    // crown (crown.wgsl `knots`): its kind's form fades into the round crown as the plants pass one, and the cluster
    // is drawn surely from two.
    let cluster = select(0.0, clamp(shape.z - 1.0, 0.0, 1.0), l == STANDS);
    if (cluster > 0.5) { shape = vec4f(f32(LOBED), 2.0 * cluster - 1.0, 0.0, 0.0); } else { shape.y *= 1.0 - 2.0 * cluster; }
  }
  if (design == BUSH) { petal = kind(petal_plane(t), sq); }
  // How dry the kind's ground (form.ts `formOf`), for the drawing's choice of plant.
  let dry = form.w;
  let side = cover.grid[l].z;
  let reach = cover.reach[l].x * side;
  let width = kind(width_plane(l, t), sq)[gr] * grow;
  let fine = 1.0 - clamp((form.y - LEAF_THICK) / (LEAF_THIN - LEAF_THICK), 0.0, 1.0);
  let tall = select(height, min(height, up / STAND), cover.corner.w > 0.5);
  let lean = LEAN * r.z * vec2f(cos(2.0 * PI * r.y), sin(2.0 * PI * r.y));
  let tint = vary(mix(first.rgb, to.rgb, r.x), fract(r.yz * 13.0));
  // A bush in flower or in fruit by a draw of its own (the next digits of its colour's), as its kind's share.
  let blooms = select(0.0, 1.0, fract(r.x * 16.0) < petal.w);
  return Plant(c, min(width, 2.0 * reach), tall, kind(stem_plane(t), sq)[gr], closed, fine, tint, first.w, fract(r * 7.0), design, shape.x, shape.y, petal.rgb, blooms, lean, reach, dry, left);
}

// What lattice l decides of the mark of its cell k (counted from its base, `cover.base`): its square, kind and group
// packed as ((row x squares across + column) x the kinds a square holds (`kinds_n`) + kind) x `GROUPS` + group (-1:
// none stands there, or the snow buries it, or its lattice's marks there take too few pixels to show), the share of it
// that shows (over the snow, and as a thing by the width of its lattice's marks at its foot, draw.wgsl `as_thing`: the
// field far away, cover_field.wgsl `lattice_field`, is drawn by the same width where they do not), and the share of the
// ground snow covers at its foot.
// A function of the cell alone, so it is decided once a frame for each cell in view (marks.wgsl) and read by every
// pixel the mark may cover (cover_marks.wgsl `lattice`).
fn decide(l: i32, k: vec2f) -> vec4f {
  let none = vec4f(-1.0, 0.0, 0.0, 0.0);
  let id = cover.base[l].xy + vec2i(k);
  let at = cover.corner.xy + mark_foot(l, k);
  let thing = as_thing(field_at(at, WIDE_LAYER)[l], cover.seen.x);
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
  let pick = pick_kind(l, at, cs, thick, f, rank.xy, id);
  if (pick.x < 0) { return none; }
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let sq = clamp(cs.at[pick.x], vec2i(0), n - 1);
  let q = mark_plant(l, k, sq, pick.y, pick.z);
  let shows = above_snow(f.snow_m, q.h) * thing;
  if (shows <= 0.0) { return none; }
  return vec4f(f32(((sq.y * n.x + sq.x) * kinds_n() + pick.y) * GROUPS + pick.z), shows, f.snow, mark_box(l, q));
}

// The box plant q of lattice l covers about its foot, so a pixel outside it does not draw it (cover_marks.wgsl `lattice`):
// angled, its sprite with its lean and its shadow (models.wgsl `model_fit`, `model_plant`, `model_shadow`), from above
// what a mark may reach (its lattice's reach each way, `cover.reach`; angled, north also the rows its tallest plants stand
// in), which bounds both. Its extents west, east, north and south, each in steps of its cell's side over `BOX_STEPS`,
// rounded up, in six bits each (a reach of 2 cells and 3 rows is 60 steps), packed as one whole number under 2^24 (a
// 32-bit float holds it exactly).
fn mark_box(l: i32, q: Plant) -> f32 {
  let g = cover.grid[l];
  let r = cover.reach[l].x;
  var steps = vec4f(r, r, r + g.w, r) * BOX_STEPS;
  if (cover.corner.w > 0.5 && models_ready()) {
    let fit = model_fit(q);
    let body = 0.5 * fit.w + abs(q.lean.x) * fit.h;
    let long = model_shadow_long(q, fit);
    let m = vec4f(body, max(body, min(long + 0.5 * fit.w, q.reach)), fit.h, long);
    steps = min(ceil(m / g.z * BOX_STEPS), steps);
  }
  return steps.x + 64.0 * (steps.y + 64.0 * (steps.z + 64.0 * steps.w));
}

// The plant of group `gr` of kind `t` of the square at `sq` in cell k of lattice l (`decide`): at its place in the
// cell, of its size and its own random numbers, angled no taller than the rows looked at let it stand.
fn mark_plant(l: i32, k: vec2f, sq: vec2i, t: i32, gr: i32) -> Plant {
  let g = cover.grid[l];
  let id = cover.base[l].xy + vec2i(k);
  return plant_of(l, sq, t, gr, mark_foot(l, k), place(id, l).z, rand3(id, l + COVER_LATTICES), (g.w + cover.reach[l].x) * g.z);
}

