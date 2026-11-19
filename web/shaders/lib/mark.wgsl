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
// that of the four kinds from 4q; of the planes of a kind's four numbers (one a group of its plants), that of kind t
// (and of lattice l).
fn kinds_n() -> i32 { return i32(cover.walls.w); }
fn groups() -> i32 { return (kinds_n() + 3) / 4; }
fn chance_plane(l: i32, t: i32) -> i32 { return l * kinds_n() + t; }
fn need_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + q; }
fn roots_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + groups() + q; }
fn label_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + 2 * groups() + q; }
fn reach_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + 3 * groups() + q; }
fn kind_plane(at: i32, t: i32) -> i32 { return COVER_LATTICES * kinds_n() + 4 * groups() + at * kinds_n() + t; }
fn form_plane(t: i32) -> i32 { return kind_plane(0, t); }
fn from_plane(t: i32) -> i32 { return kind_plane(1, t); }
fn to_plane(t: i32) -> i32 { return kind_plane(2, t); }
fn height_plane(t: i32) -> i32 { return kind_plane(3, t); }
fn width_plane(l: i32, t: i32) -> i32 { return kind_plane(4 + l, t); }
// A kind's plants come in up to this many groups by size, the large first (kind_row.ts `GROUPS`); a mark's place among
// them is kept in this many steps a group (`decide`).
const GROUPS = 4; const PLACE_STEPS = 16;

// The place among a kind's groups (`along`) at the middle of step `step` of `PLACE_STEPS` a group.
fn place_of(step: i32) -> f32 { return (f32(step) + 0.5) / f32(PLACE_STEPS); }

// The groups a kind holds, its groups' heights `heights` (empty ones last, of height 0).
fn held_of(heights: vec4f) -> i32 { return i32(dot(select(vec4f(0.0), vec4f(1.0), heights > vec4f(0.0)), vec4f(1.0))); }

// A size of the plants at place z (0 to `held`: a group's plants from g to g + 1, the large first) among a kind's
// groups whose sizes are v: each group's own at its middle, toward the next group's on a log scale over the half
// toward it, to the two's mean at their edge, as far as the two are near in size (`NEAR_SIZES`), so a mark keeps
// about its size when two groups of near sizes join or one splits, and the large plants keep theirs when a group of
// seedlings comes in.
fn along(v: vec4f, z: f32, held: i32) -> f32 {
  let g = clamp(i32(z), 0, max(held - 1, 0));
  let f = z - f32(g);
  let o = select(g + 1, g - 1, f < 0.5);
  let own = max(v[g], 1e-6);
  if (o < 0 || o >= held) { return own; }
  let ratio = log(max(v[o], 1e-6) / own);
  let near = clamp(1.0 - abs(ratio) / log(NEAR_SIZES), 0.0, 1.0);
  return own * exp(ratio * near * abs(f - 0.5));
}
// Two groups of a kind whose sizes differ by this factor or more are apart: a mark of one takes nothing of the
// other's size (`along`).
const NEAR_SIZES = 3.0;

// What the ground at `at` leaves a plant: the depth of the liquid over its roots, m, how wet it is and how wet its cells
// are on their mean (land.wgsl `state_at`), the snow there (lib/land.wgsl `snow_at`, 1 where it lies) and its cells'
// snow's depth, m.
struct Foot { depth: f32, wet: f32, mean: f32, snow: f32, snow_m: f32 }
fn foot(at: vec2f) -> Foot {
  let st = state_at(at, ground_at(at, false), corners(at / land.map.x));
  return Foot(st.depth, st.wet, st.mean, select(snow_at(st.snow, st.bares, 0.0), st.snow, st.depth > 0.0), st.snow_m);
}

// Of each of four kinds that stand in liquid as deep as `reach` m (canopy.ts: down to the core's `Cover::line`), 1 where
// it stands in liquid `depth` m deep (draw.wgsl `stands_in`), else 0.
fn stand_in(depth: f32, reach: vec4f) -> vec4f {
  return vec4f(stands_in(depth, reach.x), stands_in(depth, reach.y), stands_in(depth, reach.z), stands_in(depth, reach.w));
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

// What the four kinds from 4q of square s count for at a foot of ground `f` where `loose` m of loose ground lies,
// `thick` over the patches: its share there, how well each thrives, how much of it its roots hold and whether they
// reach through the liquid there (`stand_in`); or, not `exact`,
// the most they may count for there, before the ground is read (`pick_kind`).
fn counts(cs: Corners, s: i32, q: i32, thick: f32, f: Foot, loose: f32, exact: bool) -> vec4f {
  if (!exact) { return vec4f(cs.w[s] * (1.0 + PATCH)); }
  return cs.w[s] * thick * thrive(f.wet, f.mean, kind(need_plane(q), cs.at[s])) * holds(loose, kind(roots_plane(q), cs.at[s])) * stand_in(f.depth, kind(reach_plane(q), cs.at[s]));
}

// Where two plants would stand in one cell, one `h` m tall stands over one `under` m tall with this chance: a race whose
// rates go as their heights to the power `TALLER`, so the tallest plants take a lattice's cells and a much smaller
// kind fills only what they leave (a plant 1.5 times as tall wins 5 times in 6, one 3 times as tall 80 in 81), and
// two kinds of one height share them evenly.
const TALLER = 4.0;
fn over_one(h: f32, under: f32) -> f32 { return 1.0 / (1.0 + pow(max(under, 1e-6) / max(h, 1e-6), TALLER)); }
// The weights of a cell's plants are summed by height in bins `TALL_STEP` apart on a log scale from `TALL_LEAST` m;
// a bin d bins over another stands over it with chance `over_bins`(d) (`over_one`), all of it from `OVER_BINS` bins on.
const TALL_BINS = 24; const TALL_LEAST = 0.02; const TALL_STEP = 1.5; const OVER_BINS = 7;
fn tall_bin(h: f32) -> f32 { return clamp(log(max(h, 1e-6) / TALL_LEAST) / log(TALL_STEP), 0.0, f32(TALL_BINS - 1)); }
fn over_bins(d: i32) -> f32 { return 1.0 / (1.0 + pow(TALL_STEP, -TALLER * f32(d))); }
// Of a cell, the share a kind's taller neighbours leave it is counted at least this (all of it taken: its weight there
// is lost).
const FREE_LEAST = 0.05;
// The stream of a kind's draws for a cell, by its slot and square (`pick_kind`).
const RACE_STREAM = 175;

// The kind of the mark of lattice l whose foot is at `at` (its squares `cs`, `thick` over the patches, its ground `f`),
// its own draw `own`: its square (of the four), kind and place among the kind's groups in steps (`PLACE_STEPS`), or -1
// where none stands. Each group of each kind of each square weighs its chance on the lattice by its share at the foot.
// A kind claims the cell by a draw of its own (keyed by its slot in its cell's cover, a label it keeps, and its square;
// on the lattice of stands the squares' first kinds, their chances negated, canopy.ts `Paint`, by `stand_pick` of the
// stands' draw, so they stand in stands as the field far away has them): where the draw falls under its claim, its
// plants stand there, the large ones on its lowest draws, so a kind's marks keep their places as the other kinds'
// shares change, its smaller plants come and go above its large ones, and its place among its groups is its draw over
// its claim. Where several kinds claim one cell the taller plant stands (`over_one`), and each group claims as much more
// than its weight as its taller neighbours take of it, so it stands as often as it weighs. Not `exact`: whether any
// kind may claim the cell at the most its kinds count for there (`counts`), so a cell none may claim needs no look at
// its ground (vec3i(0) or -1).
fn pick_kind(l: i32, at: vec2f, cs: Corners, thick: f32, f: Foot, own: f32, id: vec2i, exact: bool) -> vec3i {
  let n = kinds_n();
  // The loose ground at the foot with its patches (rock.wgsl `loose_seen`, as the bare bed is drawn), read only where a
  // kind's roots may want more than lies there.
  var loose = 0.0;
  if (exact) {
    var deepest = 0.0;
    for (var s = 0; s < 4; s++) {
      for (var q = 0; q < groups(); q++) {
        let needs = kind(roots_plane(q), cs.at[s]);
        deepest = max(deepest, max(max(needs.x, needs.y), max(needs.z, needs.w)));
      }
    }
    loose = select(0.0, loose_seen(at, 0.0), deepest > 0.0);
  }
  // The weights by height, and of each bin the weight from it up.
  var bins: array<f32, TALL_BINS>;
  for (var s = 0; s < 4; s++) {
    if (cs.w[s] <= 0.0) { continue; }
    for (var q = 0; q < groups(); q++) {
      let m = counts(cs, s, q, thick, f, loose, exact);
      for (var c = 0; c < 4; c++) {
        let t = 4 * q + c;
        if (t >= n || m[c] <= 0.0) { continue; }
        let w = abs(kind(chance_plane(l, t), cs.at[s])) * m[c];
        let h = kind(height_plane(t), cs.at[s]);
        for (var g = 0; g < GROUPS; g++) {
          if (w[g] <= 0.0) { continue; }
          let x = tall_bin(h[g]);
          let b = min(i32(x), TALL_BINS - 2);
          bins[b] += w[g] * (1.0 - (x - f32(b)));
          bins[b + 1] += w[g] * (x - f32(b));
        }
      }
    }
  }
  var up: array<f32, TALL_BINS>;
  var sum = 0.0;
  for (var b = TALL_BINS - 1; b >= 0; b--) { sum += bins[b]; up[b] = sum; }
  // Each kind's claim, and of those that claim the cell, the tallest plant by the race.
  var best = vec3i(-1);
  var first_at = 1e30;
  for (var s = 0; s < 4; s++) {
    if (cs.w[s] <= 0.0) { continue; }
    for (var q = 0; q < groups(); q++) {
      let m = counts(cs, s, q, thick, f, loose, exact);
      let slots = kind(label_plane(q), cs.at[s]);
      for (var c = 0; c < 4; c++) {
        let t = 4 * q + c;
        if (t >= n || m[c] <= 0.0) { continue; }
        let chance = kind(chance_plane(l, t), cs.at[s]);
        let w = abs(chance) * m[c];
        let h = kind(height_plane(t), cs.at[s]);
        var claim = vec4f(0.0);
        for (var g = 0; g < GROUPS; g++) {
          if (w[g] <= 0.0) { continue; }
          // The plants of the bins about its height that stand over it, all of those past them, less its own groups'
          // (they share its draw, not its cells).
          let b = i32(round(tall_bin(h[g])));
          var over = select(0.0, up[min(b + OVER_BINS, TALL_BINS - 1)], b + OVER_BINS < TALL_BINS);
          for (var d = 1 - OVER_BINS; d < OVER_BINS; d++) {
            let o = b + d;
            if (o >= 0 && o < TALL_BINS && bins[o] > 0.0) { over += bins[o] * over_bins(d); }
          }
          var mine = 0.0;
          for (var o = 0; o < GROUPS; o++) { mine += w[o] * over_one(h[o], h[g]); }
          claim[g] = w[g] / max(1.0 - max(over - mine, 0.0), FREE_LEAST);
        }
        let draws = vec3f(pcg3d(vec3u(vec2u(id), u32(RACE_STREAM + 4 * i32(slots[c] + 0.5) + s)))) * (1.0 / 4294967296.0);
        let key = select(draws.x, stand_pick(stand_draw(at, STAND_M), own), any(chance < vec4f(0.0)));
        if (key >= dot(claim, vec4f(1.0))) { continue; }
        if (!exact) { return vec3i(0); }
        // Its place among its groups, the large first.
        var z = 0.0;
        var below = 0.0;
        for (var g = 0; g < GROUPS; g++) {
          if (claim[g] > 0.0 && key < below + claim[g]) { z = f32(g) + (key - below) / claim[g]; break; }
          below += claim[g];
        }
        let tall = along(h, z, held_of(h));
        let at_t = log(-log(max(draws.y, 1e-9))) - TALLER * log(max(tall, 1e-6));
        if (at_t < first_at) { first_at = at_t; best = vec3i(s, t, min(i32(z * f32(PLACE_STEPS)), GROUPS * PLACE_STEPS - 1)); }
      }
    }
  }
  return best;
}

// A kind's form and how surely it is drawn share one number: form + this x sure (canopy.ts `SURE_PACK`).
const SURE_PACK = 0.5;
// How closed a kind's crown is and the share of its leaves left share one number: the leaves left in steps of
// 1 / `LEFT_STEPS` + how closed (canopy.ts `LEFT_STEPS`).
const LEFT_STEPS = 100.0;

// The plant at place z among the groups (`along`) of kind `t` of the square at `sq` on lattice l with its foot at `c`, its size's draw `size`,
// its own random numbers `r` (its colour's draw, its lean's direction, its lean); no taller than `up` m north of
// its foot lets it stand (the rows looked at, textures.ts `ROWS_MOST`); no wider than its lattice's reach each way.
fn plant_of(l: i32, sq: vec2i, t: i32, z: f32, c: vec2f, size: f32, r: vec3f, up: f32, wades: f32) -> Plant {
  let form = kind(form_plane(t), sq);
  // Its leaves left this season, and how closed its crown is in leaf.
  let left = floor(form.x) / LEFT_STEPS;
  let closed = fract(form.x);
  let first = kind(from_plane(t), sq);
  let to = kind(to_plane(t), sq);
  // Its own size: the plants of a stand differ, by up to `SIZES` either way of their place's.
  let grow = 1.0 + SIZES * (2.0 * size - 1.0);
  let heights = kind(height_plane(t), sq);
  let held = held_of(heights);
  let height = along(heights, z, held) * grow;
  // A tree from the height a woody plant is one (`TREE_M`), a soft plant from its first (its own draw, as models.wgsl
  // `model_set` picks its sprites).
  let own = fract(r * 7.0);
  let design = select(BUSH, TREE, height >= select(TREE_M, 0.0, under_edge(first.w, SOFT_WOOD, SOFT_MIX, set_draw(own))));
  // A tree's form.
  var shape = vec4f(0.0);
  if (design == TREE) {
    // Its form and how surely, packed as one number (canopy.ts `SURE_PACK`): its kind's, from its genes alone.
    shape = vec4f(floor(form.y), fract(form.y) / SURE_PACK, 0.0, 0.0);
  }
  // How dry and how wet the kind's ground (one number, form.ts `formOf` dry less wet: a kind is never both) and how
  // tender to frost it is, for the drawing's choice of plant and its foot.
  let dry = max(form.z, 0.0);
  let wet = max(-form.z, 0.0);
  let tender = form.w;
  let side = cover.grid[l].z;
  let reach = cover.reach[l].x * side;
  let width = along(kind(width_plane(l, t), sq), z, held) * grow;
  let tall = min(height, up / STAND);
  let lean = LEAN * r.z * vec2f(cos(2.0 * PI * r.y), sin(2.0 * PI * r.y));
  let tint = vary(mix(first.rgb, to.rgb, r.x), fract(r.yz * 13.0));
  return Plant(c, min(width, 2.0 * reach), tall, closed, tint, first.w, own, design, shape.x, shape.y, lean, reach, dry, tender, left, wet, wades);
}

// What lattice l decides of the mark of its cell k (counted from its base, `cover.base`): its square, kind and place
// among the kind's groups packed as ((row x squares across + column) x the kinds a square holds (`kinds_n`) + kind) x
// `GROUPS` x `PLACE_STEPS` + its place in steps (-1:
// none stands there, or the snow buries it, or its lattice's marks there take too few pixels to show), the share of it
// that shows (over the snow, and as a thing by the width of its lattice's marks at its foot, draw.wgsl `as_thing`: the
// field far away, cover_field.wgsl `lattice_field`, is drawn by the same width where they do not), negated where its
// foot stands in liquid, and the share of the ground snow covers at its foot.
// A function of the cell alone, so it is decided once a frame for each cell in view (marks.wgsl) and read by every
// pixel the mark may cover (cover.wgsl `lattice`).
fn decide(l: i32, k: vec2f) -> vec4f {
  let none = vec4f(-1.0, 0.0, 0.0, 0.0);
  let id = cover.base[l].xy + vec2i(k);
  let at = cover.corner.xy + mark_foot(l, k);
  let thing = as_thing(field_at(at, WIDE_LAYER)[l], cover.seen.x);
  if (thing <= 0.0) { return none; }
  let cs = corners(at / land.map.x);
  // Whether any kind of the four squares has marks on the lattice.
  var all = 0.0;
  for (var s = 0; s < 4; s++) {
    for (var t = 0; t < kinds_n(); t++) { all += cs.w[s] * dot(abs(kind(chance_plane(l, t), cs.at[s])), vec4f(1.0)); }
  }
  if (all <= 0.0) { return none; }
  // Whether any kind may claim the cell, before its ground is read.
  let own = rand3(id, l).y;
  let most = Foot(0.0, 0.0, 0.0, 0.0, 0.0);
  if (pick_kind(l, at, cs, 1.0 + PATCH, most, own, id, false).x < 0) { return none; }
  let f = foot(at);
  let pick = pick_kind(l, at, cs, patchy(at), f, own, id, true);
  if (pick.x < 0) { return none; }
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let sq = clamp(cs.at[pick.x], vec2i(0), n - 1);
  let wades = select(0.0, 1.0, f.depth > FILM_M);
  let q = mark_plant(l, k, sq, pick.y, place_of(pick.z), wades);
  let shows = above_snow(f.snow * f.snow_m, q.h) * thing;
  if (shows <= 0.0) { return none; }
  return vec4f(f32(((sq.y * n.x + sq.x) * kinds_n() + pick.y) * GROUPS * PLACE_STEPS + pick.z), select(shows, -shows, wades > 0.0), f.snow, mark_box(l, q));
}

// The box plant q of lattice l covers about its foot, so a pixel outside it does not draw it (cover.wgsl `lattice`):
// its sprite with its lean and its shadow (models.wgsl `model_fit`, `model_plant`, `model_shadow`), within what a mark
// may reach (its lattice's reach each way, `cover.reach`, north also the rows its tallest plants stand in). Its extents
// west, east, north and south, each in steps of its cell's side over `BOX_STEPS`, rounded up, in six bits each (a reach
// of 2 cells and 3 rows is 60 steps), packed as one whole number under 2^24 (a 32-bit float holds it exactly).
fn mark_box(l: i32, q: Plant) -> f32 {
  let g = cover.grid[l];
  let r = cover.reach[l].x;
  let fit = model_fit(q);
  // Its clump's plants stand up to `spread` from its foot (models.wgsl `clump_one`).
  let body = fit.spread + 0.5 * fit.w + abs(q.lean.x) * fit.h;
  let long = model_shadow_long(q, fit);
  let m = vec4f(body, max(body, min(long + 0.5 * fit.w + fit.spread, q.reach)), fit.h + fit.spread, long + fit.spread);
  let steps = min(ceil(m / g.z * BOX_STEPS), vec4f(r, r, r + g.w, r) * BOX_STEPS);
  return steps.x + 64.0 * (steps.y + 64.0 * (steps.z + 64.0 * steps.w));
}

// The plant at place z among the groups of kind `t` of the square at `sq` in cell k of lattice l (`decide`): at its
// place in the cell, of its size and its own random numbers, no taller than the rows looked at let it stand; its foot
// in liquid where `wades` is 1.
fn mark_plant(l: i32, k: vec2f, sq: vec2i, t: i32, z: f32, wades: f32) -> Plant {
  let g = cover.grid[l];
  let id = cover.base[l].xy + vec2i(k);
  return plant_of(l, sq, t, z, mark_foot(l, k), place(id, l).z, rand3(id, l + COVER_LATTICES), (g.w + cover.reach[l].x) * g.z, wades);
}

