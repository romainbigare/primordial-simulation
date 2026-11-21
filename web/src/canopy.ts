// The cover's kinds as the marks draw them (textures.ts, web/shaders/cover.wgsl): of each square the kinds of cover
// the core sends with each frame (tape/src/stands.rs: a kind a slot of the cell's cover, with its plants as the cover's
// law holds them, how many, how tall, how deep their roots and the share of the square that holds them, its genes and
// its state this month), set out on the marks' three lattices, with their colours and the field the marks too small to
// draw make. No kind is a class: a plant's size, shape and colour are functions of its numbers.
import type { Rgb } from './style';
import { BARE_LEFT, DESIGN, designOf, grownWalls, seenFar, standsOf, treeFrom } from './style';
import { formOf } from './form';
import { K, groupsOf, lookOfRow, standingCrowns } from './kind_row';
import { LATTICES, STANDS, marksOf } from './lattices';
import { FIELD, Paint, leavesFalling, putField, swardOf } from './paint';

/** A kind whose mean rooted share in its square is this near 1 is held at every point of it: its root need is sent as 0
 * (mark.wgsl `holds`), so the marks read no loose ground there. */
const ROOTED_ALL = 0.999;
/** A kind's form and how surely it is drawn share one number: form + this x sure (mark.wgsl `SURE_PACK`). */
const SURE_PACK = 0.5;
/** How closed a kind's crown is and the share of its leaves left this season share one number: the leaves left in
 * steps of 1 / `LEFT_STEPS` (its whole part) + how closed (its fraction, under 1) (mark.wgsl `LEFT_STEPS`). */
const LEFT_STEPS = 100;
/** The planes of the kinds' table (lib/mark.wgsl `kind`) for squares of `kinds` kinds (the world's variety, the core's
 * `Config::variety`): of each lattice and each kind, the chance a cell holds a mark of each of its groups (kind_row.ts `GROUPS`, the
 * large plants first) where its roots hold (over the share of its square that holds it, the core's `Cover::rooted`; on
 * the lattice of stands, `STANDS`, negated for its square's first kind, `Paint`); how dense each kind stands on its
 * land's saturated strips and on its drier rest, over its mean (kind_row.ts `K.strips`, `K.rest`, the core's split of
 * its plants), and how deep its loose ground its roots need, m (the core's `Cover::need`; 0: its square holds it
 * everywhere, `ROOTED_ALL`); its slot in
 * its cell's cover (a label it keeps, so its marks keep their places as the kinds' order and shares change, mark.wgsl
 * `pick_kind`), the depth of liquid it stands in, m (how deep its line lies under the level, the core's
 * `Cover::line`), and the patch of its cell's cover it stands in (the core's `Cover::patch`); then of each kind its
 * form (how closed its crown is and the leaves it has left, `LEFT_STEPS`, its crown's form and how surely, form.ts
 * `formOf`, packed as one number, `SURE_PACK`, how dry its ground less how wet (one number: a kind is never both) and how
 * tender to frost it is), the
 * colours its plants span (style.ts `KindLook`: the first with its grown plants' walls' share, `grownWalls`); of each
 * of its groups its plants' height, m; and of each lattice the width its groups' marks are drawn, m
 * (lattices.ts `marksOf`). A plane holds four numbers: one for each kind of a group of four of a square
 * (`groups` such planes for each such number), or one of each group of a kind. */
export function planesOf(kinds: number) {
  const g = Math.ceil(kinds / 4), lat = LATTICES.length, strips = lat * kinds, form = strips + 6 * g;
  return {
    kinds, groups: g, chance: 0, strips, rest: strips + g, roots: strips + 2 * g, label: strips + 3 * g, reach: strips + 4 * g, patch: strips + 5 * g, form, from: form + kinds, to: form + 2 * kinds,
    height: form + 3 * kinds, width: form + 4 * kinds, count: form + (4 + lat) * kinds,
  };
}
export type Planes = ReturnType<typeof planesOf>;
/** The kinds' table lays its planes this many under each other, then in the next column (lib/mark.wgsl
 * `PLANES_DOWN`), so its texture stays within what every GPU holds at the largest map and variety. */
export const PLANES_DOWN = 32;
/** The number of a square's row of the core's table that is the share of its cell that is land (tape/src/table.rs `row`). */
const LAND_AT = 10;
/** The patches of a cell's cover (the core's `cover::PATCHES`). */
const PATCHES = 3;
/** The numbers of the places of a square's patches past the first in the core's table (tape/src/stands.rs `places`):
 * of each, its point, x and y in squares from the map's corner, and its reach in squares. */
const PLACE = 3;
/** The layers after a square's first patch's field that hold where its other patches lie (cover_field.wgsl
 * `patch_of`): of patch 1 its point from the square's north-west corner, x and y, in squares, and its reach in squares,
 * then patch 2's point's x; then patch 2's y and reach (16-bit floats hold a place within a square to a 2,000th). */
const PLACES = FIELD.count;
/** The first layer of the field of patch `p` (from 1) of a square's cell's cover (cover_field.wgsl `PATCH_FIELDS`). */
export const patchLayer = (p: number) => FIELD.count + 2 + (p - 1) * FIELD.count;

/** A field of the cover: `w` x `h` texels over the map, a texel a square, of each of `layers` layers (`FIELD`, then the
 * patches') a reflectance times the share of the ground it covers, and that share, RGBA layer by layer. */
export interface CoverField { w: number; h: number; layers: number; data: Float32Array }
/** The cover of a frame: the kinds' table (`planesOf` its `planes`, a texel a square, row by row, plane under plane,
 * `PLANES_DOWN` planes a column: `cols` columns), its field, of each lattice the tallest plant and the widest mark drawn
 * on it, m (0: no square holds its marks), and the reflectance of the walls the cover builds (its stems). */
export interface Kinds { w: number; h: number; planes: Planes; cols: number; table: Float32Array; field: CoverField; tallest: number[]; widest: number[]; walls: Rgb }

/** A plant's shadow darkens this many times the square of its crown's width of ground: the marks cast their
 * sprite, which covers as much as its crown, `MODEL_SHADOW_RUN` (0.9) as long (models.wgsl `model_shadow`), so the field
 * far away is as dark as the marks near. */
const SHADOW_WIDTHS = 0.9 * (Math.PI / 4);

/** Where a frame's kinds come from in the core's table (tape/src/table.rs) and what they are set out into
 * (`kindsOf`): the table `rows`, `row` numbers a square; `kinds` kinds a square of `kindRow` numbers from `at` on, and
 * the places of its patches from `places` on; the reflectance of the walls; the ground cooling by `falling`; the
 * kinds' table and the field being made, of `n` squares (`texel` the place of a plane of a square); the tallest plant
 * and the widest mark of each lattice. */
interface Sheet {
  rows: Float32Array; row: number; at: number; kinds: number; kindRow: number; places: number; walls: Rgb; falling: number;
  P: Planes; n: number; table: Float32Array; texel: (plane: number, i: number) => number; field: Float32Array; tallest: number[]; widest: number[];
}

/** The kinds of the core's table `rows` for a map of `w` x `h` squares (`row` numbers a square, tape/src/table.rs):
 * `kinds` kinds a square of `kindRow` numbers, from `at` on, then the places of each square's patches; while the
 * ground cools by `falling` (style.ts `cooling`). */
export function kindsOf(w: number, h: number, rows: Float32Array, row: number, at: number, kinds: number, kindRow: number, walls: Rgb, falling: number): Kinds {
  const P = planesOf(kinds), cols = Math.ceil(P.count / PLANES_DOWN), down = Math.min(P.count, PLANES_DOWN), n = w * h;
  const places = at + n * kinds * kindRow;
  const patched = Array.from({ length: n }, (_, i) => placeOf(rows, places, i, 1)[2] > 0).some(Boolean);
  const layers = patchLayer(patched ? PATCHES : 1);
  // Plane `plane` of square `i`: in column plane / `PLANES_DOWN`, row plane % `PLANES_DOWN`.
  const texel = (plane: number, i: number) => 4 * ((((plane % PLANES_DOWN) * h + Math.floor(i / w)) * cols + Math.floor(plane / PLANES_DOWN)) * w + (i % w));
  const s: Sheet = { rows, row, at, kinds, kindRow, places, walls, falling, P, n, table: new Float32Array(4 * cols * down * n), texel, field: new Float32Array(4 * layers * n), tallest: LATTICES.map(() => 0), widest: LATTICES.map(() => 0) };
  for (let i = 0; i < n; i++) {
    // A square whose cell holds no land and no kind (tape/src/stands.rs: a kind may stand on its ponds' ground) holds
    // 0 in every layer of the field.
    if (!(rows[1 + row * i + LAND_AT] > 0) && !Array.from({ length: kinds }, (_, k) => holdsKind(s, i, k)).some(Boolean)) continue;
    placesOf(s, i, w);
    for (let p = 0; p < PATCHES; p++) if (p === 0 || placeOf(rows, places, i, p)[2] > 0) patchOf(s, i, p, p === 0 ? 0 : patchLayer(p));
  }
  return { w, h, planes: P, cols, table: s.table, field: { w, h, layers, data: s.field }, tallest: s.tallest, widest: s.widest, walls };
}

/** The place of patch `p` (from 1) of square `i`'s cell in the core's table `rows`, whose places start at `places`:
 * its point, x and y in squares from the map's corner, and its reach in squares (0: none). */
function placeOf(rows: Float32Array, places: number, i: number, p: number): Float32Array {
  const at = places + (i * (PATCHES - 1) + p - 1) * PLACE;
  return rows.subarray(at, at + PLACE);
}

/** Does kind `k` of square `i` of `s` hold cover? */
function holdsKind(s: Sheet, i: number, k: number): boolean {
  const o = s.at + (i * s.kinds + k) * s.kindRow;
  return s.rows[o + K.root] >= 0 && s.rows[o + K.kg] > 0;
}

/** Into the layers `PLACES` of square `i` (of a map `w` squares wide) of `s`: where its cell's patches past the first
 * lie, their points from its north-west corner (0 for a patch not in use). */
function placesOf(s: Sheet, i: number, w: number): void {
  const x = i % w, y = Math.floor(i / w);
  const [a, b] = [1, 2].map(p => { const q = placeOf(s.rows, s.places, i, p); return q[2] > 0 ? [q[0] - x, q[1] - y, q[2]] : [0, 0, 0]; });
  [a[0], a[1], a[2], b[0]].forEach((v, c) => { s.field[4 * (PLACES * s.n + i) + c] = v; });
  [b[1], b[2], 0, 0].forEach((v, c) => { s.field[4 * ((PLACES + 1) * s.n + i) + c] = v; });
}

/** What a patch's kinds gather in a square (`patchOf`): each lattice's marks, the sward and its flowers (`Paint`), the
 * share of the ground whose leaves fall, and of the plants in liquid their cover and its depth by cover, and the herbs'
 * cover, depth and height by cover. */
interface Gather { paint: Paint[]; sward: Paint; flowers: Paint; shed: number; covered: number; wading: number; reach: number; herbReach: number; herbTall: number; herbs: number }

/** Into `s`, square `i`'s kinds that stand in patch `p` of its cell's cover: their planes of the kinds' table, and the
 * field they make from layer `base` on; on the lattice of stands its first kind's chances negated (mark.wgsl
 * `pick_kind`). */
function patchOf(s: Sheet, i: number, p: number, base: number): void {
  const layer = (l: number, at: number) => 4 * ((base + l) * s.n + at);
  s.field[layer(FIELD.land, i)] = 1;
  const g: Gather = { paint: LATTICES.map(() => new Paint()), sward: new Paint(), flowers: new Paint(), shed: 0, covered: 0, wading: 0, reach: 0, herbReach: 0, herbTall: 0, herbs: 0 };
  for (let k = 0; k < s.kinds; k++) {
    const o = s.at + (i * s.kinds + k) * s.kindRow;
    // A copy: its crowns' numbers are set (`standingCrowns`).
    if (holdsKind(s, i, k) && s.rows[o + K.patch] === p) kindOf(s, i, k, s.rows.slice(o, o + s.kindRow), g);
  }
  const first = g.paint[STANDS].first.k;
  for (let q = 0; first >= 0 && q < 4; q++) s.table[s.texel(s.P.chance + STANDS * s.kinds + first, i) + q] *= -1;
  putField(s.field, i, layer, g.paint, g.sward, g.flowers, g.shed);
  [g.covered > 0 ? g.wading / g.covered : 0, g.wading > 0 ? g.reach / g.wading : 0, g.herbs > 0 ? g.herbReach / g.herbs : 0, g.herbs > 0 ? g.herbTall / g.herbs : 0].forEach((v, q) => { s.field[layer(FIELD.wet, i) + q] = v; });
}

/** Kind `k` of row `r` of square `i`: into `s` its planes of the kinds' table, into `g` what it adds to its patch's
 * field. */
function kindOf(s: Sheet, i: number, k: number, r: Float32Array, g: Gather): void {
  const { P, rows, row } = s;
  const put = (plane: number, c: number, v: number) => { s.table[s.texel(plane, i) + c] = v; };
  // Kind k in planes of one number a kind, the first of which is `plane`: its group's.
  const group = (plane: number) => plane + (k >> 2);
  // The depth of liquid it stands in here: down to its line, under its square's level, where the core stands it on
  // its square's ground under liquid (its wet share); plants do not move, so the ground above its line holds them
  // whatever the level. None where it stands on the land alone.
  const wades = r[K.wet] > 0 ? Math.max(r[K.deep], 0) : 0;
  g.covered += r[K.cover];
  if (wades > 0) { g.wading += r[K.cover]; g.reach += r[K.cover] * wades; }
  // The share of the square that holds its plants and the loose ground their roots need, as the core found them.
  const mean = Math.max(r[K.rooted], 1e-3), stand = [r[K.height], mean, r[K.need]];
  // The square's ground temperature (the table's second number of a row) and the frozen cover's melting point.
  const lookAs = (design: number) => lookOfRow(r, design, s.walls, rows[1 + row * i + 1], rows[0], s.falling);
  // What of it does not stand as plants is the sward (`standsOf`).
  const stands = standsOf(r[K.height], r[K.walls]);
  // Its place in the order the first kind is chosen by: its starting kind (a label fixed in the world, so neighbouring
  // squares agree), then its slot (`Paint`).
  const order = r[K.root] * s.kinds + r[K.slot];
  if (stands < 1) {
    swardOf(r, k, order, lookAs(DESIGN.herb), 1 - stands, stand, g.sward, g.flowers);
    const share = r[K.cover] * (1 - stands);
    g.herbReach += share * wades; g.herbTall += share * r[K.height]; g.herbs += share;
  }
  if (stands <= 0) return;
  const design = designOf(r[K.height], r[K.walls]), look = lookAs(design);
  // Each group's crowns as wide as their wood holds them through the year (`standingCrowns`), as closed as its leaves.
  const groups = groupsOf(r).map(q => {
    const crowns = standingCrowns(q, mean);
    q[K.width] = crowns.width; q[K.cover] = crowns.cover; q[K.crown] = crowns.closed;
    return q;
  });
  if (!groups.length) return;
  const large = groups[0], c = seenFar(look);
  // Its leaves left this season and how closed its crown is in leaf: its large plants'. Where they stand bare (no
  // capture area) it is drawn as a dormant crown (`BARE_LEFT`), as closed as that of its largest plants in leaf (its
  // whole frame where none are), so its marks keep their size and show their bare wood.
  const leafy = groups.find(q => q[K.crown] > 0), left = large[K.crown] > 0 ? look.left : Math.min(look.left, BARE_LEFT);
  const packed = Math.round(left * LEFT_STEPS) + Math.min(leafy ? leafy[K.crown] : 1, 1 - 1e-3);
  const shape = formOf({ leaf: r[K.leaf], frost: r[K.frost], moist: r[K.moist], seed: r[K.seed] });
  const ofGroups = (plane: number, v: (q: number) => number) => groups.forEach((_, q) => put(plane + k, q, v(q)));
  put(group(P.strips), k & 3, r[K.strips]);
  put(group(P.rest), k & 3, r[K.rest]);
  put(group(P.roots), k & 3, mean >= ROOTED_ALL ? 0 : r[K.need]);
  put(group(P.label), k & 3, r[K.slot]);
  put(group(P.reach), k & 3, wades);
  put(group(P.patch), k & 3, r[K.patch]);
  // Its form and how surely as one number (form + `SURE_PACK` x sure).
  [packed, shape.form + SURE_PACK * shape.sure, shape.dry - shape.wet, shape.tender].forEach((v, q) => put(P.form + k, q, v));
  [...look.from, grownWalls(r[K.walls])].forEach((v, q) => put(P.from + k, q, v));
  const from = treeFrom(r[K.walls]);
  look.to.forEach((v, q) => put(P.to + k, q, v));
  ofGroups(P.height, q => groups[q][K.height]);
  LATTICES.forEach((a, l) => {
    const on = marksOf(groups, l, from, mean), chances = on.map(o => o.chance * stands);
    if (!chances.some(x => x > 0)) return;
    ofGroups(P.chance + l * s.kinds, q => chances[q] / mean);
    ofGroups(P.width + l * s.kinds, q => on[q].width);
    on.forEach((o, q) => {
      if (chances[q] <= 0) return;
      const pl = groups[q];
      s.tallest[l] = Math.max(s.tallest[l], pl[K.height]);
      s.widest[l] = Math.max(s.widest[l], o.width);
      // The share of the ground its plants cover, each as closed as its crown.
      g.paint[l].add(k, order, c, o.covers * stands * pl[K.crown] * look.left, (chances[q] / (a * a)) * o.width * o.width * SHADOW_WIDTHS, [pl[K.height], mean, r[K.need]], o.width);
      g.shed += leavesFalling(o.covers * stands * pl[K.crown], look.left, s.falling);
    });
  });
}

/** Scratch of `half`: one float and its bits. */
const ONE = new Float32Array(1), BITS = new Uint32Array(ONE.buffer);
/** The bits of `v` as an IEEE binary16 float, rounded to the nearest (the field goes to the GPU as 16-bit floats: a
 * leaf's reflectance of 0.04 in 8 bits would hold ten steps). */
export function half(v: number): number {
  ONE[0] = v;
  const x = BITS[0], sign = (x >>> 16) & 0x8000, e = ((x >>> 23) & 0xff) - 112, m = x & 0x7fffff;
  if (e <= 0) return e < -10 ? sign : sign | (((m | 0x800000) >> (1 - e)) + 0x1000) >> 13;
  if (e >= 31) return sign | 0x7c00;
  return sign | ((e << 10) + ((m + 0x1000) >> 13));
}

/** The number whose IEEE binary16 bits are `bits` (`half`'s inverse). */
export function unhalf(bits: number): number {
  const e = (bits >> 10) & 0x1f, m = bits & 0x3ff, sign = bits & 0x8000 ? -1 : 1;
  if (e === 0) return sign * m * 2 ** -24;
  if (e === 31) return m ? NaN : sign * Infinity;
  return sign * (1 + m / 1024) * 2 ** (e - 15);
}
