// uses: draw.wgsl land.wgsl cover_field.wgsl
// The plant cover over the live map (web/src/textures.ts), at every frame, plant by plant where a plant takes a few
// pixels; where it takes less, the ground draws it as a field of the same colours (lib/cover_field.wgsl). Each lattice
// of marks (web/src/canopy.ts `LATTICES`) lies fixed in the world, one mark a cell at a place and rank hashed from the cell (Worley 1996; pcg3d, Jarzynski and Olano 2020;
// lowbias32, draw.wgsl), so the marks hold while the camera moves and stand in no rows. A mark picks its kind at its own
// foot: of the four squares about it, blended as the map's other fields (land.wgsl `corners`), each kind's chance that
// a cell holds one of its plants, less where the ground at the foot is drier than its cell's mean as the kind's
// activity law says, picked by the mark's own rank; so kinds mix mark by mark across a square's edge and a kind that
// needs wet ground gathers in the hollows. None stands where liquid stands over its roots or under snow. Its size,
// shape and colour are its kind's numbers (canopy.ts): from above a crown of lobes, round or pointed by its height
// against its width, open where its crown is, over its stem and its shadow; angled, the plant stands up on its stem.
// Points are in metres from the draw's top-left corner.

// The kinds' table's planes (canopy.ts `KIND_PLANES`).
const CHANCE = 0; const WIDTH = 3; const NEED = 6; const FORM = 7; const COLOUR = 11;

struct Cover {
  // The draw's corner in the world, m from the map's north-west corner; metres a device pixel; 1 when angled.
  corner: vec4f,
  // Of each lattice: its cell at the draw's corner (its fraction; its whole number is `base`), its cell's side, m, and
  // the rows south of a pixel an angled plant on it may stand in.
  grid: array<vec4f, COVER_LATTICES>,
  // Of each lattice: the mip level of the field read about a pixel to see whether any of its marks stands near.
  near: array<vec4f, COVER_LATTICES>,
  base: array<vec4i, COVER_LATTICES>,
}
@group(0) @binding(1) var<uniform> cover: Cover;
@group(0) @binding(4) var kinds: texture_2d<f32>;
// The share of a plant's height its crown takes (the live crown ratio of trees in stands, 0.4 to 0.7; from memory).
const CROWN_SHARE = 0.6;
// Angled, a plant stands up this share of its height on the map (textures.ts `STAND`); the sun from the north-west
// lays a shadow this share of the height to the south-east.
const STAND = 0.5; const SUN = 0.5;
// Seen from above a crown's edge draws in lobes, as many as `LOBES_FINE` for the smallest leaves and `LOBES_BROAD` for
// the broadest (capture area per kg, log10 m²/kg, from `LEAF_THICK` to `LEAF_THIN`), each this share of its radius deep;
// its leaves' clumps are as many across its width as its lobes, the ground showing between them as its crown is open.
const LOBES_FINE = 13.0; const LOBES_BROAD = 6.0; const LEAF_THICK = -0.5; const LEAF_THIN = 0.7; const LOBE_DEEP = 0.18;
// A crown is a dome from above (2) where it is as deep as wide or less, a cone (1) where it is `POINTED` times as deep
// as wide; it is lit from the north-west, its slope read no steeper than this, and its light departs from the flat
// ground's by this share (the sky lights its far side, and light passes through its leaves).
const POINTED = 2.5; const STEEPEST = 1.5; const LIGHT_SHARE = 0.55;
const LIGHT_DIR = vec3f(-0.5, -0.5, 0.7071);
// Shadows darken the ground this much; a mark's tone differs from its kind's by up to this, and its size by this share.
const SHADOW = 0.22; const TONE = 0.12; const SIZES = 0.3;
// Its stem is drawn this much darker than its foliage, as the bark its walls make (substance 13 is browner).
const STEM_DARK = 0.45;

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

// What the ground at `at` leaves a plant: the share free of liquid over its roots and of snow, how wet it is and how
// wet its cells are on their mean (land.wgsl `state_at`).
struct Foot { stands: f32, wet: f32, mean: f32 }
fn foot(at: vec2f) -> Foot {
  let st = state_at(at, ground_at(at, false), corners(at / land.map.x));
  return Foot(stands_in(st.depth, st.snow), st.wet, st.mean);
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

// A plant: its foot, m from the draw's corner; its crown's width, its height and its stem's width, m; how closed its
// crown is; its leaves' fineness (0 broad .. 1 fine); its foliage and its walls' share; and its own random numbers.
struct Plant { at: vec2f, d: f32, h: f32, stem: f32, closed: f32, fine: f32, rgb: vec3f, wood: f32, r: vec3f }

// The crown of plant q at p seen from above or, angled, lifted on its stem: its coverage and its light (1 on flat
// ground), at e m a pixel.
fn crown(q: Plant, p: vec2f, e: f32) -> vec2f {
  let angled = cover.corner.w;
  let depth = CROWN_SHARE * q.h;
  let half = 0.5 * q.d;
  // Pointed (1) or round (0) by its depth against its width.
  let pointed = smoothstep(1.0, POINTED, depth / max(q.d, 1e-3));
  // Its centre, lifted when angled to the middle of its crown; its shape squeezed upright by its depth.
  let up = angled * STAND * (q.h - 0.5 * depth);
  let tall = mix(1.0, max(STAND * depth / max(q.d, 1e-3), 0.3), angled);
  var v = (p - q.at + vec2f(0.0, up)) / vec2f(half, half * tall);
  // Angled, a pointed crown narrows to its top (v.y = -1).
  let taper = mix(1.0, mix(1.0, clamp(0.5 * (1.0 + v.y) + 0.15, 0.15, 1.0), pointed), angled);
  v.x /= taper;
  let rho = length(v);
  let n = mix(LOBES_BROAD, LOBES_FINE, q.fine);
  let ang = atan2(v.y, v.x);
  let edge = 1.0 - LOBE_DEEP * (0.5 + 0.5 * sin(n * ang + 6.2832 * q.r.y)) * (0.55 + 0.45 * sin((0.5 * n + 1.0) * ang + 6.2832 * q.r.z));
  let soft = e / max(half, 1e-4);
  var a = 1.0 - smoothstep(edge - soft, edge + soft * 0.5, rho);
  if (a <= 0.0) { return vec2f(0.0); }
  // Its leaves' clumps: the ground shows between them as far as the crown is open.
  let k = n * 0.5;
  let clump = 0.5 + 0.5 * sin(k * v.x * 3.1416 + 6.2832 * q.r.x) * sin(k * v.y * 3.1416 + 3.0 * q.r.y);
  a *= mix(1.0, smoothstep(0.0, 0.25, clump - (1.0 - q.closed)), smoothstep(0.0, 1.0, (1.0 - q.closed) * 4.0));
  // Its light: from above a dome (beta 2) or a cone (beta 1) of its depth; angled, lit on its north-west side.
  let beta = mix(2.0, 1.0, pointed);
  let r = clamp(rho, 1e-3, 0.999);
  let slope = min(STEEPEST, depth / half * pow(r, beta - 1.0) * pow(1.0 - pow(r, beta), 1.0 / beta - 1.0));
  let dir = v / max(length(v), 1e-4);
  let normal = normalize(vec3f(dir * slope, 1.0));
  let lit = 1.0 + LIGHT_SHARE * (dot(normal, LIGHT_DIR) / LIGHT_DIR.z - 1.0);
  return vec2f(a, mix(lit, 1.0 - LIGHT_SHARE * (0.35 * v.x + 0.15 * v.y), angled));
}

// Plant q over dst at p, e m a pixel, as strongly as `fade`: its shadow, its stem, its crown.
fn plant(dst: ptr<function, vec4f>, q: Plant, p: vec2f, e: f32, fade: f32) {
  let angled = cover.corner.w;
  let half = 0.5 * q.d;
  // The shadow: from above to the south-east of its foot by its height, no farther than its cell's reach; angled,
  // flat on the ground at its foot.
  let off = mix(min(SUN * q.h, 0.5 * half), 0.3 * half, angled);
  let sv = (p - q.at - off * vec2f(0.7071)) / vec2f(half, half * mix(1.0, 0.5, angled));
  over(dst, vec3f(0.0), SHADOW * fade * (1.0 - smoothstep(0.7, 1.0, length(sv))));
  // The stem: from above its top through an open crown, angled from its foot to its crown, as wide as it is.
  let bark = q.rgb * (1.0 - STEM_DARK);
  if (angled > 0.5) {
    let top = STAND * q.h * (1.0 - 0.5 * CROWN_SHARE);
    let w = max(0.5 * q.stem, 0.35 * e);
    let s = p - q.at;
    let inside = step(abs(s.x), w) * step(s.y, 0.0) * step(-top, s.y);
    over(dst, bark, q.wood * inside * fade);
  } else {
    let w = max(0.5 * q.stem, 0.35 * e);
    over(dst, bark, q.wood * (1.0 - q.closed) * (1.0 - smoothstep(w - e, w, length(p - q.at))) * fade);
  }
  let c = crown(q, p, e);
  if (c.x <= 0.0) { return; }
  let tone = 1.0 + TONE * (2.0 * q.r.z - 1.0);
  over(dst, clamp(q.rgb * c.y * tone, vec3f(0.0), vec3f(1.0)), c.x * fade);
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
      let lo = (k - g.xy) * side;
      let id = cover.base[l].xy + vec2i(k);
      let h = place(id, l);
      let c = lo + h.xy * side;
      let d = p - c;
      if (abs(d.x) > side + e || d.y > side + e || d.y < -side - rise - e) { continue; }
      let at = world + c;
      let cs = corners(at / land.map.x);
      var chance: array<vec4f, 4>;
      var all = 0.0;
      for (var s = 0; s < 4; s++) {
        chance[s] = cs.w[s] * kind(CHANCE + l, cs.at[s]);
        all += dot(chance[s], vec4f(1.0));
      }
      let rank = rand3(id, l);
      if (rank.x >= all) { continue; }
      let f = foot(at);
      if (f.stands <= 0.0) { continue; }
      // Each kind of each square by its share at the foot; the rank picks one.
      var sum = 0.0;
      var pick = vec2i(-1);
      for (var s = 0; s < 4; s++) {
        let w = chance[s] * thrive(f.wet, f.mean, kind(NEED, cs.at[s])) * f.stands;
        for (var t = 0; t < 4; t++) {
          sum += w[t];
          if (pick.x < 0 && rank.x < sum) { pick = vec2i(s, t); }
        }
      }
      if (pick.x < 0) { continue; }
      let sq = cs.at[pick.x];
      let form = kind(FORM + pick.y, sq);
      let col = kind(COLOUR + pick.y, sq);
      // Its own size: the plants of a stand differ, by up to `SIZES` either way of its kind's.
      let size = 1.0 + SIZES * (2.0 * h.z - 1.0);
      let width = kind(WIDTH + l, sq)[pick.y] * size;
      let fine = 1.0 - clamp((form.w - LEAF_THICK) / (LEAF_THIN - LEAF_THICK), 0.0, 1.0);
      // Angled, a plant stands no taller than the rows looked at reach (textures.ts `ROWS_MOST`).
      let tall = select(form.x * size, min(form.x * size, (rise + side) / STAND), cover.corner.w > 0.5);
      plant(dst, Plant(c, min(width, 2.0 * side), tall, form.y, form.z, fine, col.rgb, col.w, rank), p, e, fade);
    }
  }
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let e = cover.corner.z;
  let p = (v.px - draw.rect.xy) * e;
  var dst = vec4f(0.0);
  for (var l = 0; l < COVER_LATTICES; l++) { lattice(&dst, p, l, e); }
  return dst * cover_look.look.y;
}
