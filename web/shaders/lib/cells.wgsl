// The cells of the plant cover's lattices of marks in view (cover.wgsl): each lattice lies fixed in the world, one mark
// a cell at a place hashed from the cell (`place`); what each mark is, decided once a frame for each cell in view
// (mark.wgsl `decide`, marks.wgsl), is kept in textures a texel a cell, with the cells whose marks reach into each
// cell (`reach_of`, reach.wgsl), and read by every pixel the marks may cover.

// The lattices of marks (canopy.ts `LATTICES`).
const COVER_LATTICES = 3;

struct Cover {
  // The draw's corner in the world, m from the map's north-west corner; metres a device pixel; w is not read.
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
  // the rows south of a pixel a plant on it may stand in.
  grid: array<vec4f, COVER_LATTICES>,
  // Of each lattice: the cells a mark reaches from its own (x; canopy.ts `REACH`).
  reach: array<vec4f, COVER_LATTICES>,
  base: array<vec4i, COVER_LATTICES>,
}
@group(0) @binding(1) var<uniform> cover: Cover;
// The marks of the cells in view (marks.wgsl): a texel a cell, a lattice's from row `cover.rows` on and from
// `MARK_MARGIN` cells before its base on, each what `decide` decided of the mark in it as four words (`Mark`), a
// texture of four bytes a texel for each (`MARK_WORDS`; bytes, which every GPU draws into): one more than its square
// and kind (0: none), as a whole number, the bits of the share that shows, so it comes back to the bit, the shares of
// the ground at its foot a byte each (its snow and fresh char, mark.wgsl `ground_shares`), and the box it covers about
// its foot (`mark_box`), as a whole number.
@group(0) @binding(5) var marks_kind: texture_2d<f32>;
@group(0) @binding(6) var marks_shows: texture_2d<f32>;
@group(0) @binding(7) var marks_snow: texture_2d<f32>;
@group(0) @binding(22) var marks_box: texture_2d<f32>;
const MARK_WORDS = 4;
// Of each cell in view, the cells about it whose marks reach into it (`reach_of`, drawn after the marks by reach.wgsl),
// a word of four bytes a texel.
@group(0) @binding(23) var marks_reach: texture_2d<f32>;
// The cells a pixel looks at reach this many before the cell that holds the draw's corner (the farthest a mark reaches,
// canopy.ts `REACH`).
const MARK_MARGIN = 2;

// A cheaper hash for the first look at a cell (`lowbias`): one word, read as three of 11, 11 and 10 bits.
fn place(c: vec2i, k: i32) -> vec3f {
  let x = lowbias(c, k);
  return vec3f(f32(x & 2047u) / 2048.0, f32((x >> 11u) & 2047u) / 2048.0, f32(x >> 22u) / 1024.0);
}

// A byte is drawn this far past its own level, in levels, so it is stored as itself whether the GPU rounds a colour to
// the nearest level or down.
const BYTE_LIFT = 0.25;

// Word x as its four bytes, each a level of 255.
fn word_bytes(x: u32) -> vec4f {
  return (vec4f(f32(x & 255u), f32((x >> 8u) & 255u), f32((x >> 16u) & 255u), f32(x >> 24u)) + BYTE_LIFT) / 255.0;
}

// A mark as decided (mark.wgsl `decide`): one more than its code, its square, kind and place (0: none), as a whole
// number; the share of it that shows with the depth of the liquid over its foot (`wade_word`); the shares of the
// ground at its foot (`ground_shares`); and the box it covers about its foot (`mark_box`), a whole number.
struct Mark { code: u32, shows: f32, ground: vec4f, box: f32 }
const NO_MARK = Mark(0u, 0.0, vec4f(0.0), 0.0);

// Word `w` of mark `m` (`decide`) as its four bytes (marks.wgsl): the ground's shares a byte each as they are, the
// others as the bytes of a 32-bit word.
fn mark_word(m: Mark, w: i32) -> vec4f {
  if (w == 2) { return (round(m.ground * 255.0) + BYTE_LIFT) / 255.0; }
  var x = m.code;
  if (w == 1) { x = bitcast<u32>(m.shows); } else if (w == 3) { x = u32(m.box); }
  return word_bytes(x);
}

// The word of four bytes, each a level of 255, read from a texel (`mark_word`).
fn word_of(b: vec4f) -> u32 {
  let v = vec4u(round(b * 255.0));
  return v.x | (v.y << 8u) | (v.z << 16u) | (v.w << 24u);
}

// The texel of the marks' textures that holds cell k of lattice l (counted from its base).
fn mark_texel(l: i32, k: vec2f) -> vec2i { return vec2i(k) + vec2i(MARK_MARGIN, MARK_MARGIN + i32(cover.rows[l])); }

// The foot of the mark of cell k of lattice l, m from the draw's corner: its place in its cell (`place`).
fn mark_foot(l: i32, k: vec2f) -> vec2f {
  let g = cover.grid[l];
  return (k - g.xy) * g.z + place(cover.base[l].xy + vec2i(k), l).xy * g.z;
}

// The cells about cell k of lattice l whose marks may reach a pixel in it: of the cells a pixel would look at (its
// lattice's reach each way, `cover.reach`, and the rows south its tallest plants stand in), those whose mark
// stands and whose box (`mark_box`) meets the cell, within a pixel. Their least and most offsets across and down, each
// `REACH_BIAS` more, in four bits each over a first bit of 1; 0 where none reaches. A function of the decided marks
// alone, so it is worked out once a frame for each cell in view (reach.wgsl) and read by each pixel (cover.wgsl
// `lattice`).
const REACH_BIAS = 8;
fn reach_of(l: i32, k: vec2f) -> u32 {
  let g = cover.grid[l];
  let side = g.z;
  let e = cover.seen.x;
  let r = i32(cover.reach[l].x);
  // The lattice's own rows of the marks' textures: a cell past them is not decided.
  let size = vec2i(textureDimensions(marks_kind));
  let first = i32(cover.rows[l]);
  var last = size.y;
  if (l + 1 < COVER_LATTICES) { last = i32(cover.rows[l + 1]); }
  // The cell's corners, m from the draw's corner.
  let lo = (k - g.xy) * side;
  let hi = lo + side;
  var least = vec2i(REACH_BIAS);
  var most = vec2i(-REACH_BIAS);
  for (var j = -r; j <= r + i32(g.w); j++) {
    for (var i = -r; i <= r; i++) {
      let c = k + vec2f(f32(i), f32(j));
      let t = mark_texel(l, c);
      if (t.x < 0 || t.x >= size.x || t.y < first || t.y >= last) { continue; }
      if (word_of(textureLoad(marks_kind, t, 0)) == 0u) { continue; }
      let foot = mark_foot(l, c);
      if (!box_meets(word_of(textureLoad(marks_box, t, 0)), lo - foot, hi - foot, side, e)) { continue; }
      least = min(least, vec2i(i, j));
      most = max(most, vec2i(i, j));
    }
  }
  if (most.x < least.x) { return 0u; }
  let b = vec4u(vec4i(least, most) + REACH_BIAS);
  return 1u | (b.x << 1u) | (b.y << 5u) | (b.z << 9u) | (b.w << 13u);
}

// The cells about a pixel's to look at for marks that reach it (`reach_of`): the least and most offsets across and
// down; none (the least past the most) where no mark reaches its cell k of lattice l.
struct Reach { least: vec2i, most: vec2i }
fn reach_at(l: i32, k: vec2f) -> Reach {
  let w = word_of(textureLoad(marks_reach, mark_texel(l, k), 0));
  if (w == 0u) { return Reach(vec2i(0), vec2i(-1)); }
  let b = vec4i(vec4u(w >> 1u, w >> 5u, w >> 9u, w >> 13u) & vec4u(15u)) - REACH_BIAS;
  return Reach(b.xy, b.zw);
}

// The mark lattice l decided for its cell k (`decide`), from the marks' textures, where it covers the point `d` m from
// its foot within a pixel `e` m (its box, `mark_box`), else none (`code` 0): one more than its code, and its share
// (`decided_ground`: the rest of what was decided of it).
struct Seen { code: u32, shows: f32 }
fn decided(l: i32, k: vec2f, d: vec2f, e: f32) -> Seen {
  let t = mark_texel(l, k);
  let code = word_of(textureLoad(marks_kind, t, 0));
  if (code == 0u || !box_holds(word_of(textureLoad(marks_box, t, 0)), d, cover.grid[l].z, e)) { return Seen(0u, 0.0); }
  return Seen(code, bitcast<f32>(word_of(textureLoad(marks_shows, t, 0))));
}

// The months since a plant a burn killed died, from its word's byte (mark.wgsl `dead_word`).
fn dead_months(b: f32) -> f32 { return round(b * 255.0) - 1.0; }

// Of the mark lattice l decided for its cell k: the share of the ground at its foot under snow and the fire on its plant
// (mark.wgsl `ground_shares`, its bytes as they are), read only where a plant stands there.
fn decided_ground(l: i32, k: vec2f) -> vec4f { return textureLoad(marks_snow, mark_texel(l, k), 0); }

// A box (mark.wgsl `mark_box`) counts its extents in steps of its cell's side over this.
const BOX_STEPS = 12.0;

// Whether the box `word` of a mark (`mark_box`) on a lattice of cells `side` m holds the point `d` m from its foot,
// within a pixel `e` m.
fn box_holds(word: u32, d: vec2f, side: f32, e: f32) -> bool { return box_meets(word, d, d, side, e); }

// Whether the box `word` of a mark (`mark_box`) on a lattice of cells `side` m meets the rectangle from `lo` to `hi` m
// from its foot, within a pixel `e` m.
fn box_meets(word: u32, lo: vec2f, hi: vec2f, side: f32, e: f32) -> bool {
  let x = vec4f(vec4u(word, word >> 6u, word >> 12u, word >> 18u) & vec4u(63u)) * (side / BOX_STEPS) + e;
  return hi.x >= -x.x && lo.x <= x.y && hi.y >= -x.z && lo.y <= x.w;
}

