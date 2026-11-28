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
fn strips_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + q; }
fn rest_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + groups() + q; }
fn roots_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + 2 * groups() + q; }
fn label_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + 3 * groups() + q; }
fn reach_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + 4 * groups() + q; }
fn patch_plane(q: i32) -> i32 { return COVER_LATTICES * kinds_n() + 5 * groups() + q; }
fn kind_plane(at: i32, t: i32) -> i32 { return COVER_LATTICES * kinds_n() + 6 * groups() + at * kinds_n() + t; }
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

// What the ground at `at` leaves a plant: the depth of the liquid over its roots, m (land.wgsl `state_at`), how
// saturated it is (land.wgsl `saturated_at`: the law's strips), the snow there (lib/land.wgsl `snow_at`, 1 where it
// lies), its cells' snow's depth, m, and how lush it is (lush.wgsl `lush_of`, on the tiles' ground and in its gullies
// as the ground under it draws them at the marks' size of a pixel, relief.wgsl; no relief finer than them).
struct Foot { depth: f32, saturated: f32, snow: f32, snow_m: f32, lush: f32 }
fn foot(at: vec2f) -> Foot {
  let g = ground_at(at, false);
  let st = state_at(at, g, corners(at / land.map.x));
  let place = sample_place(at, 0.0);
  let loose = loose_at(place);
  // (The gullies are read on the lattices of the foot's place, draw.wgsl `lat`, and the pixel's place is set back;
  // the ground's height over the sea is not known here, the ponds' alone.)
  let pixel = vec4f(pix_origin, pix_local);
  pix_origin = 1024.0 * floor(at / 1024.0);
  pix_local = at - pix_origin;
  let cut = gullies(cover.seen.x, GullyGround(g.slope, bed_shows(loose), loose, laid_at(place).deposit.x, g.twi, over_ponds(g, 0.0), g.d, discharge_of(sample_at(g))));
  pix_origin = pixel.xy;
  pix_local = pixel.zw;
  let lush = lush_of(g.twi, g.slope, place.n, 0.0, vec2f(cut.spur, cut.fold));
  return Foot(st.depth, saturated_of(g, at), select(snow_at(at, 0.0, st.snow, snow_past(st.snow, st.bares), 0.0, 0.0), st.snow, st.depth > 0.0), st.snow_m, lush);
}

// Of each of four kinds that stand in liquid as deep as `reach` m (canopy.ts: down to the core's `Cover::line`), 1 where
// it stands in liquid `depth` m deep (draw.wgsl `stands_in`), else 0.
fn stand_in(depth: f32, reach: vec4f) -> vec4f {
  return vec4f(stands_in(depth, reach.x), stands_in(depth, reach.y), stands_in(depth, reach.z), stands_in(depth, reach.w));
}

// The share of each kind that stands where its roots need `need` m of loose ground (0: its square's ground holds it
// everywhere, canopy.ts) and `loose` m lies (land.wgsl `rooted`, over the field's span, cover_field.wgsl `ROOTS_SOFT`).
fn holds(loose: f32, need: vec4f) -> vec4f {
  let s = ROOTS_SOFT * need;
  return select(vec4f(rooted(loose, need.x, s.x), rooted(loose, need.y, s.y), rooted(loose, need.z, s.z), rooted(loose, need.w, s.w)), vec4f(1.0), need <= vec4f(0.0));
}

// How dense the four kinds from 4q stand at a point of the square at c that is saturated by `saturated` (land.wgsl
// `saturated_at`), over their mean on its land: the core's split of each kind's plants between its land's saturated
// strips and its drier rest (tape/src/stands.rs `dense`, canopy.ts), the strips' where the point is saturated, the
// rest's where it is dry, and between the two over the edge of the wet ground. The law knows no other place in a land.
fn dense_at(saturated: f32, c: vec2i, q: i32) -> vec4f {
  return mix(kind(rest_plane(q), c), kind(strips_plane(q), c), saturated);
}

// What the four kinds from 4q of square s count for at a foot of ground `f` where `loose` m of loose ground lies,
// `thick` times their mean (`marks_thick`): its share there (`patch_corners`), none for a kind of another patch of its cell's cover
// than the foot's (`cell_patch`), how dense each stands on that ground (`dense_at`), how much of it its roots
// hold and whether they reach through the liquid there (`stand_in`); or, not `exact`, the most they may count for
// there, before the ground is read (`pick_kind`; `thick` then the most it may be): the denser of the two grounds.
fn counts(cs: Corners, s: i32, q: i32, thick: f32, f: Foot, loose: f32, exact: bool) -> vec4f {
  let here = select(vec4f(0.0), vec4f(cs.w[s]), kind(patch_plane(q), cs.at[s]) == vec4f(f32(cell_patch)));
  if (!exact) { return here * thick * max(kind(strips_plane(q), cs.at[s]), kind(rest_plane(q), cs.at[s])); }
  return here * thick * dense_at(f.saturated, cs.at[s], q) * holds(loose, kind(roots_plane(q), cs.at[s])) * stand_in(f.depth, kind(reach_plane(q), cs.at[s]));
}

// The squares `cs` about the point `at` as their kinds count there: in a patch past the first of its cell's cover
// (`cell_patch`), that patch's square alone and whole (`patch_cell`), as a patch is its cell's alone; else as they are.
fn patch_corners(cs: Corners) -> Corners {
  if (cell_patch == 0) { return cs; }
  var out = cs;
  out.w = vec4f(0.0);
  for (var s = 0; s < rolled(4); s++) {
    if (all(cs.at[s] == patch_cell)) { out.w[s] = 1.0; break; }
  }
  return out;
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

// The loose ground at the foot `at` (its squares `cs`) its plants' roots hold by (cover_field.wgsl `rooted_loose`, of
// its depth with its patches, rock.wgsl `loose_seen`, as the bare bed is drawn), read only where a kind's roots may
// want more than lies there.
fn foot_loose(at: vec2f, cs: Corners) -> f32 {
  var deepest = 0.0;
  for (var s = 0; s < rolled(4); s++) {
    for (var q = 0; q < groups(); q++) {
      let needs = kind(roots_plane(q), cs.at[s]);
      deepest = max(deepest, max(max(needs.x, needs.y), max(needs.z, needs.w)));
    }
  }
  if (deepest <= 0.0) { return 0.0; }
  let mean = loose_at(sample_place(at, 0.0));
  return rooted_loose(loose_seen(at, mean, 0.0), mean);
}

// The kind of the mark of lattice l whose foot is at `at` (its squares `cs`, `thick` times their mean, its ground `f`),
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
  var loose = 0.0;
  if (exact) { loose = foot_loose(at, cs); }
  // The weights by height, and of each bin the weight from it up.
  var bins: array<f32, TALL_BINS>;
  for (var s = 0; s < rolled(4); s++) {
    if (cs.w[s] <= 0.0) { continue; }
    for (var q = 0; q < groups(); q++) {
      let m = counts(cs, s, q, thick, f, loose, exact);
      for (var c = 0; c < rolled(4); c++) {
        let t = 4 * q + c;
        if (t >= n || m[c] <= 0.0) { continue; }
        let w = abs(kind(chance_plane(l, t), cs.at[s])) * m[c];
        let h = kind(height_plane(t), cs.at[s]);
        for (var g = 0; g < rolled(GROUPS); g++) {
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
  for (var b = rolled(TALL_BINS) - 1; b >= 0; b--) { sum += bins[b]; up[b] = sum; }
  // Each kind's claim, and of those that claim the cell, the tallest plant by the race.
  var best = vec3i(-1);
  var first_at = 1e30;
  for (var s = 0; s < rolled(4); s++) {
    if (cs.w[s] <= 0.0) { continue; }
    for (var q = 0; q < groups(); q++) {
      let m = counts(cs, s, q, thick, f, loose, exact);
      let slots = kind(label_plane(q), cs.at[s]);
      for (var c = 0; c < rolled(4); c++) {
        let t = 4 * q + c;
        if (t >= n || m[c] <= 0.0) { continue; }
        let chance = kind(chance_plane(l, t), cs.at[s]);
        let w = abs(chance) * m[c];
        let h = kind(height_plane(t), cs.at[s]);
        var claim = vec4f(0.0);
        for (var g = 0; g < rolled(GROUPS); g++) {
          if (w[g] <= 0.0) { continue; }
          // The plants of the bins about its height that stand over it, all of those past them, less its own groups'
          // (they share its draw, not its cells).
          let b = i32(round(tall_bin(h[g])));
          var over = select(0.0, up[min(b + OVER_BINS, TALL_BINS - 1)], b + OVER_BINS < TALL_BINS);
          for (var d = 1 - OVER_BINS; d < rolled(OVER_BINS); d++) {
            let o = b + d;
            if (o >= 0 && o < TALL_BINS && bins[o] > 0.0) { over += bins[o] * over_bins(d); }
          }
          var mine = 0.0;
          for (var o = 0; o < rolled(GROUPS); o++) { mine += w[o] * over_one(h[o], h[g]); }
          claim[g] = w[g] / max(1.0 - max(over - mine, 0.0), FREE_LEAST);
        }
        let slot = i32(slots[c] + 0.5);
        let draws = vec3f(pcg3d(vec3u(vec2u(id), u32(RACE_STREAM + 4 * slot + s)))) * (1.0 / 4294967296.0);
        var key = draws.x;
        if (any(chance < vec4f(0.0))) { key = stand_pick(stand_draw(at, STAND_M), own); } else { key = kind_key(at, slot, draws.x); }
        if (key >= dot(claim, vec4f(1.0))) { continue; }
        if (!exact) { return vec3i(0); }
        // Its place among its groups, the large first.
        var z = 0.0;
        var below = 0.0;
        for (var g = 0; g < rolled(GROUPS); g++) {
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

// The plants of a kind gather in patches about `KIND_PATCH_M` m across, each kind's its own (noise streams from
// `KIND_PATCH_STREAM` by its slot): its draw for a cell (`pick_kind`) is the patches' mixed with `KIND_PATCH_MIX` of its
// own draw `own` (`even_mix`), even over 0..1, so the kind keeps its share and its plants stand in drifts of their own, not as an
// even scatter of single crowns among the others' (from memory: shrubs spread in clonal patches, and the seedlings of a
// tree come up about their parent). The squares' first kinds on the lattice of stands stand in the stands the field
// far away draws (`stand_pick`); the field draws the rest, and the low plants, as their mean.
const KIND_PATCH_M = 30.0; const KIND_PATCH_MIX = 0.1; const KIND_PATCH_STREAM = 400;
fn kind_key(at: vec2f, slot: i32, own: f32) -> f32 {
  return even_mix(smoothstep(-EVEN_ONE, EVEN_ONE, noise_turned(at / KIND_PATCH_M, KIND_PATCH_STREAM + slot)), own, KIND_PATCH_MIX);
}

// A kind's form and how surely it is drawn share one number: form + this x sure (canopy.ts `SURE_PACK`).
const SURE_PACK = 0.5;
// How closed a kind's crown is and the share of its leaves left share one number: the leaves left in steps of
// 1 / `LEFT_STEPS` + how closed (canopy.ts `LEFT_STEPS`).
const LEFT_STEPS = 100.0;

// The plant at place z among the groups (`along`) of kind `t` of the square at `sq` on lattice l with its foot at `c`, its size's draw `size`,
// its own random numbers `r` (its hue's and tone's draws, its lean's direction, its lean) and its colour's draw
// `tint` (`tint_draw`); no taller than `up` m north of its foot lets it stand (the rows looked at, textures.ts
// `ROWS_MOST`); no wider than its lattice's reach each way.
fn plant_of(l: i32, sq: vec2i, t: i32, z: f32, c: vec2f, size: f32, r: vec3f, tint: f32, up: f32, wades: f32) -> Plant {
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
  let rgb = vary(mix(first.rgb, to.rgb, tint), fract(r.yz * 13.0));
  return Plant(c, min(width, 2.0 * reach), tall, closed, rgb, first.w, own, design, shape.x, shape.y, lean, reach, dry, tender, left, wet, wades);
}

// What lattice l decides of the mark of its cell k (counted from its base, `cover.base`): its square, kind and place
// among the kind's groups packed as ((row x squares across + column) x the kinds a square holds (`kinds_n`) + kind) x
// `GROUPS` x `PLACE_STEPS` + its place in steps, one more (`Mark`; 0:
// none stands there, or the snow buries it, or its lattice's marks there take too few pixels to show), the share of it
// that shows (over the snow, and as a crown by the width of its lattice's marks at its foot, draw.wgsl `as_crown`: the
// field far away, cover_field.wgsl `lattice_field`, is drawn by the same width where they do not) with the depth of the
// liquid over its foot (`wade_word`), the shares of the ground at its foot (`ground_shares`) and its box.
// A function of the cell alone, so it is decided once a frame for each cell in view (marks.wgsl) and read by every
// pixel the mark may cover (cover.wgsl `lattice`).
fn decide(l: i32, k: vec2f) -> Mark {
  let id = cover.base[l].xy + vec2i(k);
  let at = cover.corner.xy + mark_foot(l, k);
  let fp = foot_patch_at(at);
  let cs = patch_corners(corners(at / land.map.x));
  // Whether the lattice's marks take pixels enough to show there (`as_crown` by their width), and whether any kind of
  // the four squares has marks on it (read only where they do).
  let thing = as_crown(field_at(at, WIDE_LAYER)[l], cover.seen.x);
  var all = 0.0;
  if (thing > 0.0) {
    for (var s = 0; s < rolled(4); s++) {
      for (var t = 0; t < kinds_n(); t++) { all += cs.w[s] * dot(abs(kind(chance_plane(l, t), cs.at[s])), vec4f(1.0)); }
    }
  }
  let living = living_decided(l, k, id, at, cs, all, thing);
  // (One call where no living plant claims the cell, so the program holds `dead_decided` once.)
  if (!living.claimed) { return dead_decided(l, k, id, at, fp); }
  return living.mark;
}

// How many times their mean the plants of lattice l stand at a mark's foot at `at` among clumps `c` where the ground
// is as lush as `lush` (lush.wgsl `lushed`; the most they may, before the ground is read, where it is the lushest, 1):
// as the field draws them (cover_field.wgsl `lattice_field`), the share they cover of the ground the trees leave in
// clumps (clumps.wgsl `clumped`, the trees' edges eased over their crowns, `crowned`, the low plants in the trees'
// gaps, `low_rank`) over their mean; so on the mean they keep their number, and a clump's edge thins mark by mark.
fn marks_thick(l: i32, at: vec2f, c: Clumps, lush: f32) -> f32 {
  let trees = min(field_at(at, STANDS).a, 1.0);
  let own = min(select(field_at(at, l).a, trees, l == STANDS), 1.0);
  let left = select(1.0 - trees, 1.0, l == STANDS);
  var cl = c;
  if (l == STANDS) { cl = crowned(c, field_at(at, WIDE_LAYER)[STANDS]); } else { cl = low_rank(c); }
  let near = lushed(own, lush) * left;
  return max(cl.fill, near) * clump_ramp(cl.rank, clump_line(cl, near)) / max(own * left, 1e-6);
}

// The share of the ground at a mark's foot at `at` that the trees' crowns close, among clumps `c` where the ground is
// as lush as `lush`, as the field draws them (cover_field.wgsl `cover_field`): a low plant there stands in their
// shade, as the sward does (`SWARD_SHADE`).
fn crowns_over(at: vec2f, c: Clumps, lush: f32) -> f32 {
  let crowns = lushed(min(field_at(at, STANDS).a, 1.0), lush);
  return clumped(crowned(c, field_at(at, WIDE_LAYER)[STANDS]), crowns, crowns);
}

// The mark of a living plant of cell k of lattice l (`decide`), or that none claims the cell (its dead tree is decided
// then, `dead_decided`).
struct Living { mark: Mark, claimed: bool }
fn living_decided(l: i32, k: vec2f, id: vec2i, at: vec2f, cs: Corners, all: f32, thing: f32) -> Living {
  let none = Living(NO_MARK, true);
  let unclaimed = Living(NO_MARK, false);
  if (thing <= 0.0 || all <= 0.0) { return unclaimed; }
  // First whether any kind may claim the cell, before its ground is read; then, on open ground (`ground_open`), the
  // kind its ground lets claim it. (Both by one call, so the program holds `pick_kind` once.)
  let own = rand3(id, l).y;
  var f = Foot(0.0, 0.0, 0.0, 0.0, 0.0);
  let clumps = clumps_at(at, 0.0);
  var thick = marks_thick(l, at, clumps, 1.0);
  var pick = vec3i(-1);
  for (var look = 0; look < rolled(2); look++) {
    let exact = look == 1;
    if (exact) {
      if (!ground_open(at)) { return unclaimed; }
      f = foot(at);
      thick = marks_thick(l, at, clumps, f.lush);
    }
    pick = pick_kind(l, at, cs, thick, f, own, id, exact);
    if (pick.x < 0) { return unclaimed; }
  }
  let n = vec2i(i32(land.map.y), i32(land.map.z));
  let sq = clamp(cs.at[pick.x], vec2i(0), n - 1);
  let wades = select(0.0, f.depth, f.depth > FILM_M);
  let q = mark_plant(l, k, sq, pick.y, place_of(pick.z), 0.0, wades);
  let shows = above_snow(f.snow * f.snow_m, q.h) * thing;
  if (shows <= 0.0) { return none; }
  // On dry ground under a scar or a passing front (fire.wgsl `plant_fire`, as the field reads it): sooted and burning;
  // killed by its own draw with the chance the burn killed the soft tissue standing there, once it has stopped burning
  // (below `DEAD_AFTER` of its glow), so a burn's plants stand dead and living among each other as the field's mean
  // says; a dead one stands bare and charred (cover.wgsl `burnt_tree`), the months since it died kept in its word.
  var burnt = vec3f(0.0);
  if (f.depth <= 0.0 && fire.map.x > 0.0) {
    let fa = fire_at(at);
    let b = burn_at(at, cover.seen.x);
    let g = plant_fire(fa, scar_of(fa, b, at, cover.seen.x), b);
    let dead = unit(id, KILLED_DRAW) < g.y && g.w < DEAD_AFTER;
    burnt = vec3f(g.x, select(0.0, dead_word(g.z), dead), g.w);
  }
  let code = u32(((sq.y * n.x + sq.x) * kinds_n() + pick.y) * GROUPS * PLACE_STEPS + pick.z) + 1u;
  let under = select(crowns_over(at, clumps, f.lush), 0.0, l == STANDS);
  return Living(Mark(code, wade_word(shows, wades), ground_shares(f.snow, burnt), mark_box(l, q), tint_draw(at, rand3(id, l + COVER_LATTICES).x), under), true);
}

// Whether a plant may stand on the ground at `at`: not in a channel's water (rock.wgsl `channel_at`, its water's
// half width about its middle), whose bed floods take; and where the ground there holds no plant at all, none.
fn ground_open(at: vec2f) -> bool {
  let g = ground_at(at, false);
  let ch = channel_at(g, discharge_of(sample_at(g)));
  return ch.water <= 0.0 || abs(g.sd - ch.middle) >= ch.water;
}

// The noise stream of a living mark's own draw of whether a burn killed it (`living_decided`), and the share of its
// burning's glow below which a killed one stands dead.
const KILLED_DRAW = 45; const DEAD_AFTER = 0.3;
// The months since a plant a burn killed died, as its ground's byte (`ground_shares`): 1 more than them, at most the
// byte's top but one (0: living). `dead_months` reads it back.
fn dead_word(months: f32) -> f32 { return min(1.0 + round(max(months, 0.0)), 254.0) / 255.0; }

// The code of a dead tree's mark (`dead_decided`): past every living mark's.
fn dead_code() -> u32 { return u32(i32(land.map.y) * i32(land.map.z) * kinds_n() * GROUPS * PLACE_STEPS); }

// The dead tree cell k of lattice l may hold (snag.wgsl; its kind -1: none), the dead trees its cells hold on the
// mean (0: none) and the height of the stems that died, m: on the trees' lattice, on the ground of the patch whose plants flames killed whole and whose walls
// still stand there (the patch the fire made: cover_field.wgsl `foot_patch`, `patch_dead`), as many as those walls make
// (`snag_kg`, each as heavy as a stem of its height), in clumps (`DEAD_CLUMP_M` across) inside it; as tall as the plants
// that died, broken down by the years since they died (`snag_of`). `fp` is the patch at its foot (`foot_patch`).
const DEAD_CLUMP_M = 40.0;
struct Dead { q: Snag, mean: f32, tall: f32 }
fn dead_at(l: i32, k: vec2f, fp: vec3i) -> Dead {
  var out = Dead(Snag(-1, 0, 0.0, 0.0, 1.0, vec3f(1.0), 0.0), 0.0, 0.0);
  if (l != STANDS) { return out; }
  let at = cover.corner.xy + mark_foot(l, k);
  let dead = patch_dead(fp);
  if (dead.x <= 0.0 || dead.y <= 0.0) { return out; }
  let side = cover.grid[l].z;
  let clump = 2.0 * smoothstep(-EVEN_ONE, EVEN_ONE, noise_turned(at / DEAD_CLUMP_M, 97));
  out.q = snag_of(cover.base[l].xy + vec2i(k), dead.z, dead.y, cover.reach[l].x * side, (cover.grid[l].w + cover.reach[l].x) * side);
  out.mean = clump * dead.x * side * side / snag_kg(dead.y);
  out.tall = dead.y;
  return out;
}

// The dead tree of cell k (`id` in the world, its foot at `at`) of lattice l where no living plant claims it
// (`decide`): with the chance its mean makes (`dead_at`; its own draw, fixed in the world, so a cell's dead tree stays
// as long as the walls that make it stand), those of a burn spread over the rest of its cell standing on the land it
// burnt (fire.wgsl `dead_on_scar`), on open ground (`ground_open`) with no liquid over `FILM_M` and loose ground
// over the bed, as deep in snow as shows it: a mark of `dead_code` and its box (`dead_box`). `fp` is the patch at its
// foot (`foot_patch`).
fn dead_decided(l: i32, k: vec2f, id: vec2i, at: vec2f, fp: vec3i) -> Mark {
  let none = NO_MARK;
  let dead = dead_at(l, k, fp);
  if (dead.q.kind < 0) { return none; }
  var on_scar = 1.0;
  if (fp.x == 0 && fire.map.x > 0.0) {
    let f = fire_at(at);
    on_scar = dead_on_scar(f, patch_dead(fp).z, scar_of(f, burn_at(at, cover.seen.x), at, cover.seen.x).x);
  }
  if (unit(id, 41) >= dead.mean * on_scar) { return none; }
  let f = foot(at);
  if (f.depth > FILM_M || !ground_open(at) || bed_shows(loose_at(sample_place(at, 0.0))) > 0.5) { return none; }
  // (They show as things by the ground they cover, as their specks far away give way to them, burn.wgsl `dead_far`.)
  let shows = above_snow(f.snow * f.snow_m, dead.q.h) * as_crown(dead_seen_w(dead.tall), cover.seen.x);
  if (shows <= 0.0) { return none; }
  return Mark(dead_code() + 1u, shows, ground_shares(f.snow, vec3f(0.0)), dead_box(l, dead.q), 0.0, 0.0);
}

// The box dead tree q of lattice l covers about its foot, as `mark_box`: its sprite with its lean, and its shadow.
fn dead_box(l: i32, q: Snag) -> f32 {
  let g = cover.grid[l];
  let r = cover.reach[l].x;
  let w = snag_w(q);
  let body = 0.5 * w + abs(sin(q.lean)) * q.h;
  let c = snag_cast(q, r * g.z);
  let reach = cast_reach(c);
  let m = vec4f(body, max(body, reach.x), q.h, reach.y);
  let steps = min(ceil(m / g.z * BOX_STEPS), vec4f(r, r, r + g.w, r) * BOX_STEPS);
  return steps.x + 64.0 * (steps.y + 64.0 * (steps.z + 64.0 * steps.w));
}

// The share of the ground at a mark's foot that snow covers, and the fire on its plant (`living_decided`: its soot,
// whether it is dead and since when, `dead_word`, and its burning), each 0..1 (`decide`): a byte each of the marks'
// texture `marks_snow` as they are (cells.wgsl `mark_word`).
fn ground_shares(snow: f32, g: vec3f) -> vec4f { return vec4f(snow, g.x, g.y, clamp(g.z, 0.0, 1.0)); }

// The share of a mark that shows (0..1, more than 0) and the depth of the liquid over its foot, m (0: on land), as one
// number (`decide`): on land the share; in liquid, negated, the depth in steps of `WADE_STEP_M` m and half the share.
const WADE_STEP_M = 0.01;
fn wade_word(shows: f32, depth: f32) -> f32 { return select(shows, -(round(depth / WADE_STEP_M) + 0.5 * shows), depth > 0.0); }
// The share and the depth of `wade_word`.
fn wade_of(word: f32) -> vec2f {
  if (word >= 0.0) { return vec2f(word, 0.0); }
  let steps = floor(-word);
  return vec2f(2.0 * (-word - steps), steps * WADE_STEP_M);
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
  let c = model_cast(q, fit);
  let reach = cast_reach(c);
  let m = vec4f(body, max(body, min(reach.x + fit.spread, q.reach)), fit.h + fit.spread, max(reach.y, foot_rise(fit.s) * fit.h) + fit.spread);
  let steps = min(ceil(m / g.z * BOX_STEPS), vec4f(r, r, r + g.w, r) * BOX_STEPS);
  return steps.x + 64.0 * (steps.y + 64.0 * (steps.z + 64.0 * steps.w));
}

// The plant at place z among the groups of kind `t` of the square at `sq` in cell k of lattice l (`decide`): at its
// place in the cell, of its size, its own random numbers and its colour's draw `tint` (`tint_draw`), no taller than the
// rows looked at let it stand; liquid `wades` m deep over its foot (0: on land).
fn mark_plant(l: i32, k: vec2f, sq: vec2i, t: i32, z: f32, tint: f32, wades: f32) -> Plant {
  let g = cover.grid[l];
  let id = cover.base[l].xy + vec2i(k);
  return plant_of(l, sq, t, z, mark_foot(l, k), place(id, l).z, rand3(id, l + COVER_LATTICES), tint, (g.w + cover.reach[l].x) * g.z, wades);
}

// A plant's colour lies between its kind's two (`plant_of`: a leaf turning in its season, `KindLook`) by its own draw
// `own`, `TINT_MIX` of it, mixed with a draw over patches `TINT_M` m across (`even_mix`): the crowns of a stand turn
// together in patches and groups, as the ground and the shelter they share time their season, not one by one as
// confetti; at the foot `at`, m from the map's corner. Even over 0..1, as the field's mean counts it (style.ts `seenFar`).
const TINT_M = 40.0; const TINT_MIX = 0.15; const TINT_STREAM = 260;
fn tint_draw(at: vec2f, own: f32) -> f32 { return even_mix(smoothstep(-EVEN_ONE, EVEN_ONE, noise_turned(at / TINT_M, TINT_STREAM)), own, TINT_MIX); }

