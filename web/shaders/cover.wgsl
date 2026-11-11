// uses: draw.wgsl water.wgsl style.wgsl land.wgsl cover_field.wgsl
// The plant cover over the live map (web/src/textures.ts), at every frame, plant by plant where a plant takes a few
// pixels; where it takes less, the ground draws it as a field of the same colours (lib/cover_field.wgsl). Each lattice
// of marks (web/src/canopy.ts `LATTICES`) lies fixed in the world, one mark a cell at a place and rank hashed from the
// cell (Worley 1996; pcg3d, Jarzynski and Olano 2020; lowbias32, draw.wgsl), so the marks hold while the camera moves and
// stand in no rows. A mark picks its kind at its own foot: of the four squares about it, blended as the map's other
// fields (land.wgsl `corners`), each kind's chance that a cell holds one of its plants (thicker and thinner over the
// field's patches, cover_field.wgsl `patchy`), less where the ground at the foot is drier than its cell's mean as the
// kind's activity law says, picked by the mark's own rank; so kinds mix mark by mark across a square's edge and a kind
// that needs wet ground gathers in the hollows. None stands where liquid stands over its roots or under snow. Its size,
// shape and colour are its kind's numbers (canopy.ts, web/src/style.ts): a tree is a crown of lobes over its stem and
// its shadow, round or pointed by its height against its width, open where its crown is; a bush a dense knot of a few
// lobes; a herb a tuft of blades, with flowers in its season and as reeds on soaked ground. Angled, each stands up on its
// foot. Points are in metres from the draw's top-left corner.

// The kinds' table's planes (canopy.ts `KIND_PLANES`), and the designs (style.ts `DESIGN`).
const CHANCE = 0; const WIDTH = 3; const NEED = 6; const FORM = 7; const FROM = 11; const TO = 15; const PETAL = 19;
const TREE = 0.0; const BUSH = 1.0;

struct Cover {
  // The draw's corner in the world, m from the map's north-west corner; metres a device pixel; 1 when angled.
  corner: vec4f,
  // The reflectance of the walls the cover builds: its stems (wasm `meta`); and the share of a plant in flower its
  // flowers cover (style.ts `FLOWERS_COVER`).
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
// The share of a plant's height its crown takes (the live crown ratio of trees in stands, 0.4 to 0.7; from memory).
const CROWN_SHARE = 0.6;
// Angled, a plant stands up this share of its height on the map (textures.ts `STAND`); the sun from the north-west
// lays a shadow this share of the height to the south-east.
const STAND = 0.5; const SHADOW_RUN = 0.5;
// Seen from above a crown's edge draws in lobes, as many as `LOBES_FINE` for the smallest leaves and `LOBES_BROAD` for
// the broadest (capture area per kg, log10 m²/kg, from `LEAF_THICK` to `LEAF_THIN`), each this share of its radius deep;
// its leaves' clumps are as many across its width as its lobes, the ground showing between them as its crown is open.
const LOBES_FINE = 13.0; const LOBES_BROAD = 6.0; const LEAF_THICK = -0.5; const LEAF_THIN = 0.7; const LOBE_DEEP = 0.18;
// A bush is a knot of this many lobes, each this share of its radius deep.
const BUSH_LOBES = 5.0; const BUSH_DEEP = 0.3;
// A crown is a dome from above (2) where it is as deep as wide or less, a cone (1) where it is `POINTED` times as deep
// as wide; it is lit from the north-west, its slope read no steeper than this, and its light departs from the flat
// ground's by this share (the sky lights its far side, and light passes through its leaves).
const POINTED = 2.5; const STEEPEST = 1.5; const LIGHT_SHARE = 0.55;
// A shadow lets through the sky's light and what crowns scatter, about half of the sun's (canopy.ts `CROWN_SHADOW`); a
// plant's size differs from its kind's by up to this share.
const SHADOW = 0.5; const SIZES = 0.3;
// A herb's tuft: between `BLADES_FEW` and `BLADES_MANY` blades, solid to this share of its radius and narrowing to
// their tips by this power; reeds (a herb that needs ground at least `REED_NEED` wet, on ground at least `REED_WET`
// wet) have narrower blades.
const BLADES_FEW = 5.0; const BLADES_MANY = 9.0; const BLADE_SOLID = 0.3; const BLADE_SHARP = 2.0;
const REED_SHARP = 7.0; const REED_NEED = 0.4; const REED_WET = 0.85;
// A herb in flower holds up to this many flowers, this share of its radius from its middle, each with a heart this
// share of it across (the colour of pollen).
const FLORETS_MOST = 3.0; const FLORET_AT = 0.5; const FLORET_HEART = 0.35; const POLLEN = vec3f(0.85, 0.62, 0.08);

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
// crown is; its leaves' fineness (0 broad .. 1 fine); its colour and its walls' share; its own random numbers; its
// design (style.ts `DESIGN`; reeds: 1 for a herb on soaked ground); its flowers' colour, and 1 if it is in flower.
struct Plant { at: vec2f, d: f32, h: f32, stem: f32, closed: f32, fine: f32, rgb: vec3f, wood: f32, r: vec3f, design: f32, reed: f32, petal: vec3f, blooms: f32 }

// What a plant's body shows at a point: its coverage, its light (1 on flat ground), and how near its rim the point is
// (0 inside .. 1 at its edge).
struct Body { a: f32, lit: f32, rim: f32 }

// The crown of plant q at p seen from above or, angled, lifted on its stem, with `n` lobes `deep` of its radius deep:
// its coverage, its light and its rim, at e m a pixel. A bush (`round`) is a dome, closed.
fn crown(q: Plant, p: vec2f, e: f32, n: f32, deep: f32, round: bool) -> Body {
  let angled = cover.corner.w;
  let depth = CROWN_SHARE * q.h;
  let half = 0.5 * q.d;
  // Pointed (1) or round (0) by its depth against its width.
  let pointed = select(smoothstep(1.0, POINTED, depth / max(q.d, 1e-3)), 0.0, round);
  // Its centre, lifted when angled to the middle of its crown; its shape squeezed upright by its depth.
  let up = angled * STAND * (q.h - 0.5 * depth);
  let tall = mix(1.0, max(STAND * depth / max(q.d, 1e-3), 0.3), angled);
  var v = (p - q.at + vec2f(0.0, up)) / vec2f(half, half * tall);
  // Angled, a pointed crown narrows to its top (v.y = -1).
  let taper = mix(1.0, mix(1.0, clamp(0.5 * (1.0 + v.y) + 0.15, 0.15, 1.0), pointed), angled);
  v.x /= taper;
  let rho = length(v);
  let ang = atan2(v.y, v.x);
  let edge = 1.0 - deep * (0.5 + 0.5 * sin(n * ang + 2.0 * PI * q.r.y)) * (0.55 + 0.45 * sin((0.5 * n + 1.0) * ang + 2.0 * PI * q.r.z));
  let soft = e / max(half, 1e-4);
  var a = 1.0 - smoothstep(edge - soft, edge + soft * 0.5, rho);
  if (a <= 0.0) { return Body(0.0, 1.0, 0.0); }
  // Its leaves' clumps: the ground shows between them as far as the crown is open.
  let k = n * 0.5;
  let clump = 0.5 + 0.5 * sin(k * v.x * PI + 2.0 * PI * q.r.x) * sin(k * v.y * PI + 3.0 * q.r.y);
  let closed = select(q.closed, 1.0, round);
  a *= mix(1.0, smoothstep(0.0, 0.25, clump - (1.0 - closed)), smoothstep(0.0, 1.0, (1.0 - closed) * 4.0));
  // Its light: from above a dome (beta 2) or a cone (beta 1) of its depth; angled, lit on its north-west side.
  let beta = mix(2.0, 1.0, pointed);
  let r = clamp(rho, 1e-3, 0.999);
  let slope = min(STEEPEST, depth / half * pow(r, beta - 1.0) * pow(1.0 - pow(r, beta), 1.0 / beta - 1.0));
  let dir = v / max(length(v), 1e-4);
  let normal = normalize(vec3f(dir * slope, 1.0));
  let lit = 1.0 + LIGHT_SHARE * (dot(normal, LIGHT) / LIGHT.z - 1.0);
  return Body(a, mix(lit, 1.0 - LIGHT_SHARE * (0.35 * v.x + 0.15 * v.y), angled), rim(rho, edge, half, e));
}

// The tuft of herb q at p, at e m a pixel: blades from its foot, seen from above as a star of them, angled standing up
// from it; lighter towards their tips.
fn tuft(q: Plant, p: vec2f, e: f32) -> Body {
  let angled = cover.corner.w;
  let half = 0.5 * q.d;
  // Angled, its blades rise as high as it stands, fanned upward from its foot.
  var v = (p - q.at) / vec2f(half, mix(half, max(STAND * q.h, half), angled));
  if (angled > 0.5 && v.y > 0.0) { return Body(0.0, 1.0, 0.0); }
  let ang = select(atan2(v.y, v.x), 2.0 * atan2(v.x, -v.y), angled > 0.5);
  let n = floor(mix(BLADES_FEW, BLADES_MANY + 1.0, q.r.y));
  let sharp = mix(BLADE_SHARP, REED_SHARP, q.reed);
  let edge = BLADE_SOLID + (1.0 - BLADE_SOLID) * pow(abs(cos(0.5 * n * ang + 2.0 * PI * q.r.z)), sharp);
  let rho = length(v);
  let soft = e / max(half, 1e-4);
  let a = 1.0 - smoothstep(edge - soft, edge + soft * 0.5, rho);
  return Body(a, 0.8 + 0.4 * rho, rim(rho, edge, half, e));
}

// The flowers of herb q at p, at e m a pixel: up to `FLORETS_MOST` round flowers over its tuft (angled, at its top),
// as big as cover `cover.walls.w` of it; each over dst, with its heart.
fn florets(dst: ptr<function, vec4f>, q: Plant, p: vec2f, e: f32, fade: f32) {
  let half = 0.5 * q.d;
  let count = 1.0 + floor(q.r.x * FLORETS_MOST);
  let radius = half * sqrt(cover.walls.w / count);
  let lift = vec2f(0.0, cover.corner.w * STAND * q.h * 0.8);
  for (var i = 0.0; i < count; i += 1.0) {
    let turn = 2.0 * PI * (q.r.z + i / count);
    let d = length(p - q.at + lift - FLORET_AT * half * vec2f(cos(turn), sin(turn)));
    let a = 1.0 - smoothstep(radius - e, radius, d);
    if (a <= 0.0) { continue; }
    over(dst, mark_colour(select(q.petal, POLLEN, d < FLORET_HEART * radius), 1.0, rim(d / radius, 1.0, radius, e)), a * fade);
  }
}

// Plant q over dst at p, e m a pixel, as strongly as `fade`: its shadow, its stem (a tree), its body by its design,
// and its flowers.
fn plant(dst: ptr<function, vec4f>, q: Plant, p: vec2f, e: f32, fade: f32) {
  let angled = cover.corner.w;
  let half = 0.5 * q.d;
  // The shadow: from above to the south-east of its foot by its height, no farther than its cell's reach; angled,
  // flat on the ground at its foot.
  let off = mix(min(SHADOW_RUN * q.h, 0.5 * half), 0.3 * half, angled);
  let sv = (p - q.at - off * vec2f(0.7071)) / vec2f(half, half * mix(1.0, 0.5, angled));
  over(dst, vec3f(0.0), SHADOW * fade * (1.0 - smoothstep(0.7, 1.0, length(sv))));
  var body: Body;
  if (q.design == TREE) {
    // The stem: from above its top through an open crown, angled from its foot to its crown, as wide as it is.
    let bark = cover.walls.rgb;
    let w = max(0.5 * q.stem, 0.35 * e);
    if (angled > 0.5) {
      let s = p - q.at;
      let inside = step(abs(s.x), w) * step(s.y, 0.0) * step(-STAND * q.h * (1.0 - 0.5 * CROWN_SHARE), s.y);
      over(dst, bark, q.wood * inside * fade);
    } else {
      over(dst, bark, q.wood * (1.0 - q.closed) * (1.0 - smoothstep(w - e, w, length(p - q.at))) * fade);
    }
    body = crown(q, p, e, mix(LOBES_BROAD, LOBES_FINE, q.fine), LOBE_DEEP, false);
  } else if (q.design == BUSH) {
    body = crown(q, p, e, BUSH_LOBES, BUSH_DEEP, true);
  } else {
    body = tuft(q, p, e);
  }
  if (body.a > 0.0) { over(dst, mark_colour(q.rgb, body.lit, body.rim), body.a * fade); }
  if (q.blooms > 0.0) { florets(dst, q, p, e, fade); }
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
      var chance: array<vec4f, 4>;
      var all = 0.0;
      let thick = patchy(at, l);
      for (var s = 0; s < 4; s++) {
        chance[s] = cs.w[s] * kind(CHANCE + l, cs.at[s]) * thick;
        all += dot(chance[s], vec4f(1.0));
      }
      let rank = rand3(id, l);
      if (rank.x >= all) { continue; }
      let f = foot(at);
      if (f.stands <= 0.0) { continue; }
      // Each kind of each square by its share at the foot; the rank picks one, and where it falls in that kind's
      // share is a draw of its own (`u`).
      var sum = 0.0;
      var pick = vec2i(-1);
      var u = 0.0;
      var need = 0.0;
      for (var s = 0; s < 4; s++) {
        let needs = kind(NEED, cs.at[s]);
        let w = chance[s] * thrive(f.wet, f.mean, needs) * f.stands;
        for (var t = 0; t < 4; t++) {
          sum += w[t];
          if (pick.x < 0 && rank.x < sum) { pick = vec2i(s, t); u = 1.0 - (sum - rank.x) / w[t]; need = needs[t]; }
        }
      }
      if (pick.x < 0) { continue; }
      plant(dst, plant_of(l, cs.at[pick.x], pick.y, c, h.z, u, rank, need, f.wet, rise + side), p, e, fade);
    }
  }
}

// The plant of kind `t` of the square at `sq` on lattice l with its foot at `c`, its size's draw `size`, its draw in
// its kind's share `u`, its own random numbers `r`, its kind's need of wet ground and the wetness at its foot; angled
// no taller than `reach` lets it stand (the rows looked at, textures.ts `ROWS_MOST`).
fn plant_of(l: i32, sq: vec2i, t: i32, c: vec2f, size: f32, u: f32, r: vec3f, need: f32, wet: f32, reach: f32) -> Plant {
  let form = kind(FORM + t, sq);
  let first = kind(FROM + t, sq);
  let to = kind(TO + t, sq);
  // Its own size: the plants of a stand differ, by up to `SIZES` either way of its kind's.
  let grow = 1.0 + SIZES * (2.0 * size - 1.0);
  let width = kind(WIDTH + l, sq)[t] * grow;
  let fine = 1.0 - clamp((form.w - LEAF_THICK) / (LEAF_THIN - LEAF_THICK), 0.0, 1.0);
  let tall = select(form.x * grow, min(form.x * grow, reach / STAND), cover.corner.w > 0.5);
  var q = Plant(c, min(width, 2.0 * cover.grid[l].z), tall, form.y, form.z, fine, vary(mix(first.rgb, to.rgb, u), r.yz), first.w, r, to.w, 0.0, vec3f(0.0), 0.0);
  if (to.w > BUSH) {
    q.reed = select(0.0, 1.0, need >= REED_NEED && wet >= REED_WET);
    let petal = kind(PETAL + t, sq);
    // In flower by a draw of its own (the next digits of `u`), as its kind's share in flower.
    q.blooms = select(0.0, 1.0, fract(u * 16.0) < petal.w) * (1.0 - q.reed);
    q.petal = petal.rgb;
  }
  return q;
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
