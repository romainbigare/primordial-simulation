// uses: draw.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl cover_field.wgsl crown.wgsl
// The trees and bushes over the live map (web/src/textures.ts), at every frame, plant by plant where a plant takes a
// few pixels; where it takes less, the ground draws it as a field of the same colours (lib/cover_field.wgsl), and the
// herbs always so, as its sward. Each lattice of marks (web/src/canopy.ts `LATTICES`) lies fixed in the world, one mark
// a cell at a place and rank hashed from the cell (Worley 1996; pcg3d, Jarzynski and Olano 2020; lowbias32, draw.wgsl),
// so the marks hold while the camera moves and stand in no rows. A mark picks its kind at its own foot: of the four
// squares about it, blended as the map's other fields (land.wgsl `corners`), each kind's chance that a cell holds one of
// its plants (thicker and thinner over the field's patches, cover_field.wgsl `patchy`), less where the ground at the foot
// is drier than its cell's mean as the kind's activity law says and where its loose ground is thinner than its roots
// need (land.wgsl `rooted`, its chance raised where its square's ground is thin, so the square keeps its cover); a plant
// buried by the snow's depth against its height does not show, one standing in snow carries it on its crown; on the
// trees' lattice in stands of their kinds as the
// field far away has them (cover_field.wgsl `stand_pick`), elsewhere by the mark's own rank. So kinds mix mark by mark
// across a square's edge and a kind that needs wet ground gathers in the hollows. None stands where liquid stands over
// its roots or under snow. Its size, shape and colour are its kind's numbers (canopy.ts, web/src/style.ts,
// web/src/form.ts; lib/crown.wgsl draws its body): a tree is a crown of its kind's form over its stem and its shadow,
// leaning a little its own way; a bush a low knot of lobes with its flowers or fruit. Angled, each stands up on its
// foot. Points are in metres from the draw's top-left corner.

// The kinds' table's planes (canopy.ts `KIND_PLANES`), and the designs (style.ts `DESIGN`).
const CHANCE = 0; const WIDTH = 3; const NEED = 6; const ROOTS = 7; const FORM = 8; const FROM = 12; const TO = 16; const PETAL = 20; const SHAPE = 24;
const TREE = 0.0; const BUSH = 1.0;

struct Cover {
  // The draw's corner in the world, m from the map's north-west corner; metres a device pixel; 1 when angled.
  corner: vec4f,
  // The reflectance of the walls the cover builds: its stems (wasm `meta`).
  walls: vec4f,
  // Of each lattice: its cell at the draw's corner (its fraction; its whole number is `base`), its cell's side, m, and
  // the rows south of a pixel an angled plant on it may stand in.
  grid: array<vec4f, COVER_LATTICES>,
  // Of each lattice: the mip level of the field read about a pixel to see whether any of its marks stands near.
  near: array<vec4f, COVER_LATTICES>,
  base: array<vec4i, COVER_LATTICES>,
}
@group(0) @binding(1) var<uniform> cover: Cover;
@group(0) @binding(4) var kinds: texture_2d<f32>;
// The sun from the north-west lays a shadow this share of the height to the south-east, but no farther than `SHADOW_OFF`
// of the crown's radius (so a tree stands on its shadow), soft over `SHADOW_SOFT` of its radius.
const SHADOW_RUN = 0.5; const SHADOW_OFF = 0.45; const SHADOW_SOFT = 0.35;
// A shadow lets through the sky's light and what crowns scatter, about half of the sun's (canopy.ts `CROWN_SHADOW`); a
// plant's size differs from its kind's by up to this share; a tree leans by up to `LEAN` of its height.
const SHADOW = 0.42; const SIZES = 0.3; const LEAN = 0.08;
// Snow lies on a crown's upper, lit side (its light from `SNOW_LIT.x` to `SNOW_LIT.y` times flat ground's) up to this
// share of its colour, as closed as the crown is (a bare one holds little), in the colour of fresh snow (colour.wgsl
// `SNOW`).
const CROWN_SNOW = 0.8; const SNOW_LIT = vec2f(0.9, 1.15); const CROWN_SNOW_RGB = vec3f(0.90, 0.92, 0.95);

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
// Paint colour rgb with coverage a over dst (premultiplied).
fn over(dst: ptr<function, vec4f>, rgb: vec3f, a: f32) { *dst = vec4f(rgb * a, a) + *dst * (1.0 - a); }

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
  return select(vec4f(rooted(loose, need.x), rooted(loose, need.y), rooted(loose, need.z), rooted(loose, need.w)), vec4f(1.0), need <= vec4f(0.0));
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

// Plant q over dst at p, e m a pixel, as strongly as `fade`, where snow covers `snow` of the ground: its shadow, its stem
// (a tree), its body by its design (a closed crown lit from above carries the snow, `CROWN_SNOW`), and a bush's flowers
// or fruit.
fn plant(dst: ptr<function, vec4f>, q: Plant, p: vec2f, e: f32, fade: f32, snow: f32) {
  let angled = cover.corner.w;
  let half = min(0.5 * q.d, q.reach / 1.1);
  // The shadow: from above to the south-east of its foot by its height (`SHADOW_OFF`); angled, flat on the ground at its
  // foot.
  let off = mix(min(min(SHADOW_RUN * q.h, SHADOW_OFF * half), max(q.reach - half, 0.0) / 0.7071), 0.3 * half, angled);
  let sv = (p - q.at - off * vec2f(0.7071)) / vec2f(half, half * mix(1.0, 0.5, angled));
  over(dst, vec3f(0.0), SHADOW * fade * (1.0 - smoothstep(1.0 - SHADOW_SOFT, 1.0, length(sv))));
  var body: Body;
  if (q.design == TREE) {
    // The stem: from above its top through an open crown, angled from its foot to its crown, leaning with it, as wide
    // as it is.
    let bark = cover.walls.rgb;
    let w = max(0.5 * q.stem, 0.35 * e);
    if (angled > 0.5) {
      let s = p - q.at;
      let lean = q.lean.x * -s.y;
      let inside = step(abs(s.x - lean), w) * step(s.y, 0.0) * step(-STAND * q.h * (1.0 - 0.5 * CROWN_SHARE), s.y);
      over(dst, bark, q.wood * inside * fade);
    } else {
      over(dst, bark, q.wood * (1.0 - q.closed) * (1.0 - smoothstep(w - e, w, length(p - q.at))) * fade);
    }
    body = crown(q, p, e);
  } else {
    body = bush(q, p, e);
  }
  if (body.a > 0.0) {
    let snowy = CROWN_SNOW * snow * q.closed * smoothstep(SNOW_LIT.x, SNOW_LIT.y, body.lit);
    over(dst, mark_colour(mix(q.rgb, CROWN_SNOW_RGB, snowy), body.lit, body.rim), body.a * fade);
  }
  if (q.blooms > 0.0) { bush_dots(dst, q, p, e, fade); }
}

// The marks of lattice l over dst at p, m from the draw's corner, e m a pixel.
fn lattice(dst: ptr<function, vec4f>, p: vec2f, l: i32, e: f32) {
  let g = cover.grid[l];
  let fade = cover_look.shown[l];
  let world = cover.corner.xy;
  // Skip a lattice none of whose marks stands on any square this pixel's marks could stand on.
  if (fade <= 0.0 || field_at(world + p, l, cover.near[l].x).a <= 0.0) { return; }
  let side = g.z;
  let cell = floor(g.xy + p / side);
  // A mark reaches no farther than a cell's side from its foot, and angled as far north as its tallest stands.
  let rise = g.w * side;
  for (var j = -1; j <= 1 + i32(g.w); j++) {
    for (var i = -1; i <= 1; i++) {
      let k = cell + vec2f(f32(i), f32(j));
      let id = cover.base[l].xy + vec2i(k);
      let h = place(id, l);
      let c = (k - g.xy) * side + h.xy * side;
      let d = p - c;
      if (abs(d.x) > side + e || d.y > side + e || d.y < -side - rise - e) { continue; }
      let at = world + c;
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
      if (rank.x >= all * (1.0 + PATCH)) { continue; }
      let thick = patchy(at);
      if (rank.x >= all * thick) { continue; }
      let f = foot(at);
      if (f.stands <= 0.0) { continue; }
      let pick = pick_kind(at, cs, chance, thick, f, rank.x);
      if (pick.x < 0) { continue; }
      let q = plant_of(l, cs.at[pick.x], pick.y, c, h.z, rand3(id, l + COVER_LATTICES), rise + side);
      let shows = above_snow(f.snow_m, q.h);
      if (shows > 0.0) { plant(dst, q, p, e, fade * shows, f.snow); }
    }
  }
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
  // The loose ground at the foot (rock.wgsl `loose_at`), read only where a kind's roots may want more than lies there.
  var deepest = 0.0;
  for (var s = 0; s < 4; s++) {
    needs[s] = kind(ROOTS, cs.at[s]);
    deepest = max(deepest, max(max(needs[s].x, needs[s].y), max(needs[s].z, needs[s].w)));
  }
  let loose = select(0.0, loose_at(at), deepest > 0.0);
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

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let e = cover.corner.z;
  let p = (v.px - draw.rect.xy) * e;
  var dst = vec4f(0.0);
  for (var l = 0; l < COVER_LATTICES; l++) { lattice(&dst, p, l, e); }
  // Linear light turned into the screen's share once (draw.wgsl `srgb`), in the style (style.wgsl `painted`),
  // premultiplied.
  if (dst.a <= 0.0) { return vec4f(0.0); }
  return vec4f(painted(srgb(dst.rgb / dst.a)), 1.0) * dst.a * cover_look.look.y;
}
