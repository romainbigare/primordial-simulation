// uses: draw.wgsl
// The marks of the plant cover (web/src/textures.ts) over the draw's rectangle, at every frame. Each kind of mark (a
// tuft of grass, of tundra or of steppe, a shrub, a tree's crown) lies on its own grid fixed in the world, one mark a
// grid cell at a place, size, tone and rank hashed from the cell (Worley 1996; the hashes are lowbias32, Wellons 2018,
// and pcg3d, Jarzynski and Olano 2020): the marks hold while the camera moves, never repeat and stand in no rows. A
// mark shows where its rank is below the density of its kind at its centre, read from the field of the cover (blended
// between the field's texels), so a thinner cover shows fewer marks and two kinds mix mark by mark where one gives way
// to the other. From above a crown is a soft round blob over its shadow, lit from the north-west; angled, it is lifted
// over its shadow. A tuft is a small turned blob, angled a few blades standing up. Points are in metres from the
// rectangle's top-left corner.

// The most kinds of mark.
const KINDS = 8;

struct Cover {
  // The field's and the genes' tint's windows: uv = xy + m × zw, m the metres from the picture's corner.
  field_window: vec4f, tint_window: vec4f,
  // Metres a pixel; 1 when angled, 0 from above; how strongly the marks cover the map; how strongly the genes tint them.
  look: vec4f,
  // The colour a woody cover darkens towards (0..1).
  dark: vec4f,
  // The kinds of mark; how many of the first are tufts; the first that is a tree's crown.
  kinds: vec4i,
  // The field's channels of the foliage's darkening, of the crowns' woodland share and of their jungle share.
  channels: vec4i,
  // The foliage of each kind of mark, then of the trees' woodland, forest and jungle (0..1).
  foliage: array<vec4f, KINDS>,
  // Of each kind: its grid's cell at the picture's corner (its fraction; its whole number is `base`), the side of its
  // grid's cells, m, and its marks' size, m.
  grid: array<vec4f, KINDS>,
  // Of each kind: how strongly it shows (0: not drawn), and the mip level of the field read about a pixel to see whether
  // any of its marks stands near.
  shown: array<vec4f, KINDS>,
  base: array<vec4i, KINDS>,
}
@group(0) @binding(1) var<uniform> cover: Cover;
@group(0) @binding(2) var field: texture_2d_array<f32>;
@group(0) @binding(3) var field_sampler: sampler;
@group(0) @binding(4) var tint: texture_2d<f32>;
@group(0) @binding(5) var tint_sampler: sampler;

const PI = 3.14159265;
// A mark is drawn from 1 - SPREAD / 2 to 1 + SPREAD / 2 times its kind's size; a jungle's emergent crowns (this share)
// are wider by this.
const SPREAD = 0.5; const EMERGENT = 0.15; const EMERGENT_WIDE = 1.2;
// An angled crown is lifted over its shadow by this share of its radius; a shadow darkens the ground this much, lies
// this many radii to the south-east of its crown from above and angled, and angled is this much flatter.
const ANGLED_LIFT = 0.7; const SHADOW = 0.35; const SHADOW_FLAT = 0.6;
const SHADOW_ABOVE = vec2f(0.25, 0.25); const SHADOW_ANGLED = vec2f(0.35, 0.15);
// No part of a crown's shadow lies farther from its foot than this many radii.
const SHADOW_REACH = 1.4;
// A crown's soft rim takes this share of its radius; its light falls from this many radii to the north-west of its
// middle and fades over this many; its lit side is this much lighter (and as much again by its tone), its far rim
// this much darker from above and angled.
const RIM = 0.25; const LIT_SPAN = 1.2; const LIGHT = 0.14; const DARK_ABOVE = 0.2; const DARK_ANGLED = 0.3;
const LIT_AT = vec2f(0.3, 0.35);
// A tuft covers this much, is this much narrower than long, and lighter or darker by up to this; angled, its three
// blades stand this many radii tall, lean this many radii either way and are this many radii wide at the foot.
const TUFT_COVER = 0.9; const TUFT_NARROW = 0.55; const TUFT_TONE = 0.25;
const BLADE_TALL = 1.4; const BLADE_LEAN = 0.5; const BLADE_WIDE = 0.25;

// pcg3d (Jarzynski and Olano 2020): three random words from three.
fn pcg3d(w: vec3u) -> vec3u {
  var v = w * 1664525u + 1013904223u;
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  v ^= v >> vec3u(16u);
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  return v;
}
fn rand3(c: vec2i, k: i32) -> vec3f { return vec3f(pcg3d(vec3u(vec2u(c), u32(k)))) * (1.0 / 4294967296.0); }
// A cheaper hash for the first look at a cell (lowbias32, Wellons 2018): one word, read as three of 11, 11 and 10 bits.
fn place(c: vec2i, k: i32) -> vec3f {
  var x = (u32(c.x) * 0x9E3779B1u) ^ (u32(c.y) * 0x85EBCA77u) ^ (u32(k) * 0xC2B2AE3Du);
  x ^= x >> 16u; x *= 0x7feb352du; x ^= x >> 15u; x *= 0x846ca68bu; x ^= x >> 16u;
  return vec3f(f32(x & 2047u) / 2048.0, f32((x >> 11u) & 2047u) / 2048.0, f32(x >> 22u) / 1024.0);
}
// Channel `ch` of the field at m metres from the picture's corner, at mip level lod (the mean of 2^lod texels square);
// nothing outside the field.
fn channel(m: vec2f, ch: i32, lod: f32) -> f32 {
  let uv = cover.field_window.xy + m * cover.field_window.zw;
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return 0.0; }
  return textureSampleLevel(field, field_sampler, uv, ch >> 2u, lod)[ch & 3];
}
// Paint colour rgb with coverage a over dst (premultiplied).
fn over(dst: ptr<function, vec4f>, rgb: vec3f, a: f32) { *dst = vec4f(rgb * a, a) + *dst * (1.0 - a); }
// 1 inside a disc of radius r at distance d, 0 outside, soft over w.
fn disc(d: f32, r: f32, w: f32) -> f32 { return 1.0 - smoothstep(r - w, r, d); }

fn crown(dst: ptr<function, vec4f>, p: vec2f, c: vec2f, r: f32, tone: f32, col: vec3f, e: f32, fade: f32) {
  let angled = cover.look.y;
  let lift = angled * r * ANGLED_LIFT;
  let s = c + r * mix(SHADOW_ABOVE, SHADOW_ANGLED, angled);
  let sr = r * vec2f(1.0, mix(1.0, SHADOW_FLAT, angled));
  let ds = length((p - s) / sr);
  over(dst, vec3f(0.0), SHADOW * fade * (1.0 - smoothstep(1.0 - RIM, 1.0, ds)));
  let b = c - vec2f(0.0, lift);
  let a = disc(length(p - b), r, max(RIM * r, e));
  if (a <= 0.0) { return; }
  let t = clamp(length(p - b + r * LIT_AT) / (LIT_SPAN * r), 0.0, 1.0);
  let dark = mix(col, vec3f(0.0), mix(DARK_ABOVE, DARK_ANGLED, angled) * (2.0 * t - 1.0));
  let lit = mix(col, vec3f(1.0), LIGHT * (1.0 + tone) * (1.0 - 2.0 * t));
  over(dst, select(dark, lit, t < 0.5), a * fade);
}

// A blade from its foot a to its tip b, as wide as w at its foot: the distance from p to its edge (negative inside).
fn blade(p: vec2f, a: vec2f, b: vec2f, w: f32) -> f32 {
  let ab = b - a;
  let h = clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0);
  return length(p - a - h * ab) - w * (1.0 - h);
}

fn tuft(dst: ptr<function, vec4f>, p: vec2f, c: vec2f, r: f32, tone: f32, turn: f32, col: vec3f, e: f32, fade: f32) {
  var a: f32;
  if (cover.look.y > 0.5) {
    var d = 1e9;
    for (var k = -1; k <= 1; k++) { d = min(d, blade(p, c, c + r * vec2f(BLADE_LEAN * f32(k), -BLADE_TALL), BLADE_WIDE * r)); }
    a = 1.0 - smoothstep(-0.5 * e, 0.5 * e, d);
  } else {
    let q = mat2x2f(cos(turn), -sin(turn), sin(turn), cos(turn)) * (p - c);
    let d = length(q / vec2f(r, TUFT_NARROW * r));
    let w = min(e / (TUFT_NARROW * r), 1.0);
    a = 1.0 - smoothstep(1.0 - w, 1.0, d);
  }
  if (a <= 0.0) { return; }
  let shaded = mix(col, vec3f(select(1.0, 0.0, tone < 0.5)), abs(tone - 0.5) * 2.0 * TUFT_TONE);
  over(dst, shaded, TUFT_COVER * a * fade);
}

// The marks of kind l over dst at p (m from the picture's corner), e m a pixel, of foliage darkened by dark and tinted
// by tint.
fn lattice(dst: ptr<function, vec4f>, p: vec2f, l: i32, e: f32, dark: f32, tint: vec4f) {
  let g = cover.grid[l];
  let fade = cover.shown[l].x;
  // Skip a kind that is drawn faint by none of the texels this pixel's marks could stand on.
  if (fade <= 0.0 || channel(p, l, cover.shown[l].y) <= 0.0) { return; }
  let angled = cover.look.y;
  let is_tuft = l < cover.kinds.y;
  let is_tree = l >= cover.kinds.z;
  // How far a mark reaches from its foot, in radii: a tuft from above its own, an angled one its blades, a crown its
  // shadow (and an angled one its lifted crown), and an emergent crown more.
  let reach = select(mix(SHADOW_REACH, 1.0 + ANGLED_LIFT, angled) * select(1.0, EMERGENT_WIDE, is_tree), mix(1.0, BLADE_TALL, angled), is_tuft);
  let side = g.z;
  let cell = floor(g.xy + p / side);
  // No mark reaches farther than this, m: a cell whose every point is farther is not looked at.
  let far = reach * 0.5 * g.w * (1.0 + 0.5 * SPREAD) + e;
  // An angled mark stands up over its foot: also the row south of the pixel's.
  let south = select(1, 2, angled > 0.5);
  let woodland_kind = cover.kinds.z;
  for (var j = -1; j <= south; j++) {
    for (var i = -1; i <= 1; i++) {
      let k = cell + vec2f(f32(i), f32(j));
      let lo = (k - g.xy) * side;
      if (length(max(max(lo - p, p - lo - side), vec2f(0.0))) > far) { continue; }
      let id = cover.base[l].xy + vec2i(k);
      let h = place(id, l);
      let c = lo + h.xy * side;
      var r = 0.5 * g.w * (1.0 + SPREAD * (h.z - 0.5));
      if (length(p - c) > reach * r + e) { continue; }
      let rank = rand3(id, l);
      let density = channel(c, l, 0.0);
      if (rank.x >= density) { continue; }
      var kind = l;
      var emergent = false;
      if (is_tree) {
        let woodland = channel(c, cover.channels.y, 0.0);
        let jungle = channel(c, cover.channels.z, 0.0);
        kind = select(select(woodland_kind + 1, woodland_kind + 2, rank.y < woodland + jungle), woodland_kind, rank.y < woodland);
        // The rank of a mark that shows is even below its density, so its lowest ranks are a fixed share of its crowns.
        emergent = kind == woodland_kind + 2 && rank.x < EMERGENT * density;
      }
      let col = mix(mix(cover.foliage[kind].rgb, cover.dark.rgb, dark), tint.rgb / max(tint.a, 1e-3), cover.look.w * tint.a);
      if (emergent) { r *= EMERGENT_WIDE; }
      if (is_tuft) { tuft(dst, p, c, r, rank.z, rank.y * PI, col, e, fade); } else { crown(dst, p, c, r, rank.z, col, e, fade); }
    }
  }
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let e = cover.look.x;
  let p = (v.px - draw.rect.xy) * e;
  let tinted = textureSampleLevel(tint, tint_sampler, cover.tint_window.xy + p * cover.tint_window.zw, 0.0);
  var dst = vec4f(0.0);
  let dark = channel(p, cover.channels.x, 0.0);
  for (var l = 0; l < cover.kinds.x; l++) { lattice(&dst, p, l, e, dark, tinted); }
  return dst * cover.look.z;
}
