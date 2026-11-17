// The cover's kinds as the marks draw them (textures.ts, web/shaders/cover.wgsl): of each square the kinds of cover
// the core sends with each frame (tape/src/stands.rs: a kind a slot of the cell's cover, with its plants as the cover's
// law holds them, how many, how tall, how deep their roots and the share of the square that holds them, its genes and
// its state this month), set out on the marks' three lattices, with their colours and the field the marks too small to
// draw make. No kind is a class: a plant's size, shape and colour are functions of its numbers.
import type { Rgb } from './style';
import { clamp } from './earth';
import { DESIGN, KindLook, designOf, fromScreen, grownWalls, seenFar, standsOf, strawSeen, toScreen, treeFrom } from './style';
import { formOf } from './form';
import { GROUPS, K, groupsOf, lookOfRow, standingCrowns } from './kind_row';

/** The side of a cell of each lattice of marks, m: one mark a cell at most. A mark reaches at most `REACH` cells from its
 * cell, so it is drawn at most twice that many sides across; a kind is drawn on the lattices whose side is nearest its
 * crown's width (on a log scale), so the three hold every bush and tree from 5 cm to 6 m across, wider ones as more
 * marks. Herbs have no marks:
 * they are the ground's sward (cover_field.wgsl `sward`). */
export const LATTICES = [0.1, 0.5, 2.5];
/** Of each lattice, the cells a mark reaches from its own (cover.wgsl looks that many cells each way about a pixel): the
 * trees' two, so a crown up to four sides across stands among the small ones of its lattice. */
export const REACH = [1, 1, 2];
/** Angled, a plant stands up this share of its height on the map (lib/crown.wgsl `STAND`): the rows south of a pixel
 * looked at, past those its marks reach, reach its tallest plant, but no more than `ROWS_MOST` (each row costs a pixel
 * a look at marks for each cell across, textures.ts), so lattice l stands a plant up to (`ROWS_MOST` + `REACH`[l]) of
 * its sides over `STAND` tall (`shareOn`). */
export const STAND = 0.5, ROWS_MOST = 3;
/** Angled, a mark's sprite is drawn up to this much wider than its crown, so its leaves, which have gaps, cover its
 * crown's ground (models.wgsl `MODEL_WIDER`): a crown is at most its reach over this. */
const SPRITE_WIDER = 1.6;
/** A kind whose mean rooted share in its square is this near 1 is held at every point of it: its root need is sent as 0
 * (mark.wgsl `holds`), so the marks read no loose ground there. */
const ROOTED_ALL = 0.999;
/** A bush's mark is a clump of its plants this many times as wide as one, so its lobes read at a 10 m scale bar. */
const BUSH_GROW = 2.5;
/** A plant's size differs from its group's by up to this share either way (lib/mark.wgsl `SIZES`). */
const SIZE_SPREAD = 0.3;

/** How many plants across a mark of a group of plants `height` m tall is, of a kind whose plants are trees from `from`
 * m (style.ts `treeFrom`): `BUSH_GROW` as far as its marks are bushes, each mark by its own size (`SIZE_SPREAD`), so the
 * group's marks change their width smoothly as they pass from bushes to trees. */
function clumpOf(height: number, from: number): number {
  const trees = clamp((1 + SIZE_SPREAD - from / Math.max(height, 1e-9)) / (2 * SIZE_SPREAD));
  return BUSH_GROW + (1 - BUSH_GROW) * trees;
}
/** A kind's form and how surely it is drawn share one number: form + this x sure (mark.wgsl `SURE_PACK`). */
const SURE_PACK = 0.5;
/** How closed a kind's crown is and the share of its leaves left this season share one number: the leaves left in
 * steps of 1 / `LEFT_STEPS` (its whole part) + how closed (its fraction, under 1) (mark.wgsl `LEFT_STEPS`). */
const LEFT_STEPS = 100;
/** The planes of the kinds' table (lib/mark.wgsl `kind`) for squares of `kinds` kinds (the world's variety, the core's
 * `Config::variety`): of each lattice the chance a cell holds a mark of each kind where its roots hold (over the share
 * of its square that holds it, the core's `Cover::rooted`; on the lattice of stands, `STANDS`, negated for its square's
 * first kind, `Paint`); how wet each kind needs its ground and how deep its loose ground, m (the core's `Cover::need`; 0:
 * its square holds it everywhere, `ROOTED_ALL`); its slot in its cell's cover (a label it keeps, so its marks keep their
 * places as the kinds' order and shares change, mark.wgsl `pick_kind`); then of each kind its form (how closed its crown is and the leaves it
 * has left, `LEFT_STEPS`, its capture area per kg, log10 m²/kg, its crown's form and how surely, form.ts `formOf`, packed
 * as one number, `SURE_PACK`, and how dry its ground), the colours its plants span (style.ts `KindLook`: the first with
 * its grown plants' walls' share, `grownWalls`, the second with the height its plants are trees from, `treeFrom`), its flowers (their colour and
 * the share of its plants in flower); of each of its groups (`GROUPS`) its plants' height and stem's width, m, and the
 * plants one of its marks on the lattice of stands stands for; and of each lattice, of each of its groups, the share of
 * the kind's marks there that are of it and the groups before it (the large plants first: mark.wgsl `group_of`) and the
 * width its marks are drawn, m. A plane holds four numbers: one for each kind of a group of four of a square (`groups`
 * such planes for each of the first numbers, and of each lattice), or one of each group of a kind. */
export function planesOf(kinds: number) {
  const g = Math.ceil(kinds / 4), form = 6 * g, lat = LATTICES.length;
  return {
    kinds, groups: g, chance: 0, need: lat * g, roots: (lat + 1) * g, label: (lat + 2) * g, form, from: form + kinds, to: form + 2 * kinds, petal: form + 3 * kinds,
    height: form + 4 * kinds, stem: form + 5 * kinds, clump: form + 6 * kinds, cut: form + 7 * kinds, width: form + (7 + lat) * kinds, count: form + (7 + 2 * lat) * kinds,
  };
}
export type Planes = ReturnType<typeof planesOf>;
/** The kinds' table lays its planes this many under each other, then in the next column (lib/mark.wgsl
 * `PLANES_DOWN`), so its texture stays within what every GPU holds at the largest map and variety. */
export const PLANES_DOWN = 32;
/** The layers of the field (`CoverField`): one a lattice (its marks' plants), then the herbs' sward, the herbs' flowers,
 * of the sward and of the lattice of stands the part its first kind makes (`Paint`), of each lattice and the sward
 * (red, green, blue, alpha) its plants' mean height, m, the mean share of the square that holds them and the mean depth
 * of loose ground their roots need, m (the core's, tape/src/stands.rs), of each lattice its marks' mean width, m, and the
 * share of the ground its plants leave that their shadows darken (`Paint.put`), the sward and the lattices under the
 * trees' as one layer, as the field draws them far away (`lowOf`), and 1 where the
 * square's cell holds land (a square with none holds 0 in every layer: cover_field.wgsl blends over the squares with land
 * alone, `on_land`), and of the sward and of the lattice of stands the part the kinds but the first make. */
export const FIELD = { sward: LATTICES.length, flowers: LATTICES.length + 1, swardFirst: LATTICES.length + 2, standsFirst: LATTICES.length + 3, tall: LATTICES.length + 4, rooted: LATTICES.length + 5, need: LATTICES.length + 6, wide: LATTICES.length + 7, shade: LATTICES.length + 8, low: LATTICES.length + 9, land: LATTICES.length + 10, swardRest: LATTICES.length + 11, standsRest: LATTICES.length + 12, count: LATTICES.length + 13 } as const;
/** The number of a square's row of the core's table that is the share of its cell that is land (tape/src/table.rs `row`). */
const LAND_AT = 10;
/** The lattice whose kinds grow in stands of their own, near and far alike (cover_field.wgsl `stand_pick`): the trees'. */
export const STANDS = 2;

/** A field of the cover: `w` x `h` texels over the map, a texel a square, of each layer (`FIELD`) a reflectance times the
 * share of the ground it covers, and that share, RGBA layer by layer. */
export interface CoverField { w: number; h: number; data: Float32Array }
/** The cover of a frame: the kinds' table (`planesOf` its `planes`, a texel a square, row by row, plane under plane,
 * `PLANES_DOWN` planes a column: `cols` columns), its field, of each lattice the tallest plant and the widest mark drawn
 * on it, m (0: no square holds its marks), and the reflectance of the walls the cover builds (its stems). */
export interface Kinds { w: number; h: number; planes: Planes; cols: number; table: Float32Array; field: CoverField; tallest: number[]; widest: number[]; walls: Rgb }

/** A shadow lets through the sky's light and what crowns scatter, about half of the sun's (crown.wgsl `SHADOW`,
 * cover_field.wgsl `FIELD_SHADOW`). */
const CROWN_SHADOW = 0.42;
/** A plant's shadow darkens this many times the square of its crown's width of ground: angled, the marks cast their
 * sprite, which covers as much as its crown, `MODEL_SHADOW_RUN` (0.9) as long (models.wgsl `model_shadow`), so the field
 * far away is as dark as the marks near. */
const SHADOW_WIDTHS = 0.9 * (Math.PI / 4);

/** The share of a kind of crown width `w`, m, drawn on lattice `l` by its width: between the two lattices whose sides
 * lie either side of it, by where it lies between them on a log scale; all on the first below it and on the last above
 * it. */
function shareByWidth(w: number, l: number): number {
  const x = Math.log(Math.max(w, 1e-6)), at = LATTICES.map(Math.log);
  const below = l > 0 ? clamp((x - at[l - 1]) / (at[l] - at[l - 1])) : 1, above = l < at.length - 1 ? clamp((at[l + 1] - x) / (at[l + 1] - at[l])) : 1;
  return Math.min(below, above);
}

/** The share of a kind of crown width `w`, m, and height `h`, m, drawn on lattice `l`: by its width (`shareByWidth`),
 * but none on a lattice too small to stand it at its height (`STAND`, `ROWS_MOST`): that share is drawn on the least
 * that does (or the last), so a tall plant of a narrow crown stands as tall as it is. */
function shareOn(w: number, h: number, l: number): number {
  const stands = LATTICES.findIndex((a, m) => ((ROWS_MOST + REACH[m]) * a) / STAND >= h), least = stands < 0 ? LATTICES.length - 1 : stands;
  if (l < least) return 0;
  if (l > least) return shareByWidth(w, l);
  return LATTICES.slice(0, l + 1).reduce((sum, _, m) => sum + shareByWidth(w, m), 0);
}

/** A stand's crowns share its canopy and touch; the marks' places are random, so crowns each as wide as the ground it
 * holds overlap and leave gaps. Marks whose crowns sum to x of the ground cover 1 - e^-x of it: a stand that closes the
 * share c of its ground draws each crown's area -ln(1 - c) / c times as large, so the ground its crowns cover together,
 * overlaps counted once, is the stand's (c at most this; a mark no wider than its lattice lets it reach, and more of
 * them where it would be wider). */
const CLOSED_MOST = 0.95;

/** The ground a kg of a herb's walls (its stems and last year's thatch) shades, standing, m² (a cured grassland of 0.1
 * kg/m² of stems closes about two thirds of its ground). */
const STRAW_AREA = 10;

/** The marks of a kind on a lattice: the chance a cell holds one (as a share of the whole square), the width a mark is
 * drawn, m, the plants a mark stands for, and the share of the square its marks on this lattice cover. */
interface MarksOn { chance: number; width: number; plants: number; covers: number }

/** The marks of a kind (row `r`) on lattice `l`, each a clump `grow` plants across, on the share `rooted` of the square
 * that holds it. Its plants stand in patches that cover the share `cover` of the square, `perM2` a m², each crown the
 * ground it holds there (the stand's crown cover over its plants); a cell of a patch holds the plants that fall in it
 * as one mark (a clump where they are many), never more marks than plants; a mark is drawn as much wider as closes the
 * stand's canopy at random places (`CLOSED_MOST`), at most `REACH` cells from its own, and where that is too narrow its
 * marks stand more often (at most one a cell), so the ground they cover on the rooted share is the stand's. */
function marksOn(r: Float32Array, l: number, grow: number, rooted: number): MarksOn {
  const share = shareOn(r[K.width] * grow, r[K.height], l), a = LATTICES[l], most = (2 * REACH[l] * a) / SPRITE_WIDER;
  if (share <= 0 || r[K.cover] <= 0 || r[K.perM2] <= 0) return { chance: 0, width: 0, plants: 0, covers: 0 };
  const inside = r[K.perM2] / r[K.cover], plants = Math.max((inside * a * a) / (grow * grow), 1);
  const held = Math.min(r[K.width] * grow * Math.sqrt(plants), most);
  // Their crowns sum to -ln(1 - closed) of the ground (taken as ln(1 + x), exact for a stand that closes almost none).
  const closed = Math.min(r[K.cover] / rooted, CLOSED_MOST), sum = -Math.log1p(-closed), width = Math.min(held * Math.sqrt(sum / closed), most);
  // The marks a cell of the rooted share holds: their crowns sum to that of its ground, or as near as one a cell comes;
  // the ground they cover is what is drawn, near and far.
  const area = (Math.PI / 4) * width * width, chance = Math.min(1, (sum * a * a) / area);
  return { chance: share * chance * rooted, width, plants, covers: share * rooted * -Math.expm1(-(chance * area) / (a * a)) };
}

/** What a layer of the field (`FIELD`) gathers in a square: its kinds' colours as the screen shows them (a mix of colours
 * on a screen averages what it shows, not the light: style.ts `seenFar`) summed by the ground each covers, that
 * ground, the ground they leave, the shadow they cast (the plants a m² times their shadow's area), their heights,
 * rooted shares, roots' needs and marks' widths summed by the ground each covers, and its first kind:
 * the one of the lowest starting kind (a label fixed in the world, so neighbouring squares agree), with its colour and
 * the ground it covers. */
class Paint {
  rgb: Rgb = [0, 0, 0];
  covered = 0;
  bare = 1;
  shadow = 0;
  tall = 0;
  held = 0;
  deep = 0;
  wide = 0;
  first = { root: Infinity, rgb: [0, 0, 0] as Rgb, covers: 0, k: -1 };

  /** Add (a group of) kind `k` of starting kind `root` and colour `c`, covering `covers` of the ground and casting `shadow`, its
   * plants `height` m tall, held by the share `rooted` of the square, their roots needing `need` m of loose ground,
   * their marks `width` m wide (0: none, the sward). */
  add(k: number, root: number, c: Rgb, covers: number, shadow: number, [height, rooted, need]: number[], width: number): void {
    if (covers <= 0) return;
    this.rgb = this.rgb.map((v, q) => v + covers * toScreen(c[q])) as Rgb;
    this.covered += covers;
    this.tall += covers * height;
    this.held += covers * rooted;
    this.deep += covers * need;
    this.wide += covers * width;
    this.bare *= 1 - covers;
    this.shadow += shadow;
    if (root < this.first.root) this.first = { root, rgb: c, covers, k };
    else if (k === this.first.k) this.first.covers += covers;
  }

  /** Into `out` at `at`: premultiplied, the layer's mean reflectance times the share of the ground it covers, and that
   * share; of its first kind (`firstAt`, if any), its part of that. */
  put(out: Float32Array, at: number, firstAt?: number, restAt?: number): void {
    const cover = 1 - this.bare, n = Math.max(this.covered, 1e-9);
    this.rgb.forEach((v, q) => { out[at + q] = fromScreen(v / n) * cover; });
    out[at + 3] = cover;
    if (firstAt === undefined || restAt === undefined) return;
    const share = this.first.covers / n, rest = n - this.first.covers;
    this.first.rgb.forEach((v, q) => { out[firstAt + q] = v * share * cover; });
    out[firstAt + 3] = share * cover;
    // The rest's colour, as the screen shows its kinds (the sum of all less the first's).
    this.rgb.forEach((v, q) => { out[restAt + q] = rest > 1e-9 ? fromScreen(Math.max(v - this.first.covers * toScreen(this.first.rgb[q]), 0) / rest) * (1 - share) * cover : 0; });
    out[restAt + 3] = (1 - share) * cover;
  }

  /** The share of the ground the layer's shadows fall on: 1 - e^-(the shadows' area per m²) (Poisson-placed plants),
   * so a tall rough canopy is darker than a low one of the same leaves. It is no cover: the ground between the crowns
   * is darker (by `CROWN_SHADOW`), not covered. */
  shade(): number { return -Math.expm1(-this.shadow); }

  /** Into `out` at each of `at` (the layers `FIELD.tall`, `rooted`, `need` and `wide`): its plants' mean height, the
   * mean share of the square that holds them (1 where it holds none), their roots' mean need and their marks' mean
   * width. */
  putStand(out: Float32Array, at: number[]): void {
    const n = Math.max(this.covered, 1e-9);
    [this.tall / n, this.covered > 0 ? this.held / this.covered : 1, this.deep / n, this.wide / n].forEach((v, q) => { out[at[q]] = v; });
  }
}

/** Into `sward` and `flowers`: the share `part` of the kind of row `r` (its starting kind `k`) that is the sward, of look
 * `look`, its plants `stand` (height, rooted share, roots' need): they cover their share of the ground, each as closed
 * as its crown, and their stems stand where their leaves are gone (cured straw), closing the ground as their walls'
 * area does. */
function swardOf(r: Float32Array, k: number, look: KindLook, part: number, stand: number[], sward: Paint, flowers: Paint): void {
  const c = seenFar(look), crown = r[K.crown] * look.left;
  const straw = (1 - Math.exp(-STRAW_AREA * r[K.kg] * r[K.wood])) * (1 - crown), closes = crown + straw;
  const covers = part * r[K.cover] * closes, mixed = closes > 0 ? straw / closes : 0, s = strawSeen();
  sward.add(k, r[K.root], [0, 1, 2].map(q => c[q] + (s[q] - c[q]) * mixed) as Rgb, covers, 0, stand, 0);
  flowers.add(k, 0, look.petal, covers * look.bloom, 0, stand, 0);
}

/** The kinds of the core's table `rows` for a map of `w` x `h` squares (`row` numbers a square, tape/src/table.rs):
 * `kinds` kinds a square of `kindRow` numbers, from `at` on; while the ground cools by `falling` (style.ts
 * `cooling`). */
export function kindsOf(w: number, h: number, rows: Float32Array, row: number, at: number, kinds: number, kindRow: number, walls: Rgb, falling: number): Kinds {
  const P = planesOf(kinds), cols = Math.ceil(P.count / PLANES_DOWN), down = Math.min(P.count, PLANES_DOWN);
  const n = w * h, table = new Float32Array(4 * cols * down * n), tallest = LATTICES.map(() => 0), widest = LATTICES.map(() => 0);
  const field = new Float32Array(4 * FIELD.count * n);
  // Plane `plane` of square `i`: in column plane / `PLANES_DOWN`, row plane % `PLANES_DOWN`.
  const texel = (plane: number, i: number) => 4 * ((((plane % PLANES_DOWN) * h + Math.floor(i / w)) * cols + Math.floor(plane / PLANES_DOWN)) * w + (i % w));
  const put = (plane: number, i: number, c: number, v: number) => { table[texel(plane, i) + c] = v; };
  // Kind k of a square in planes of one number a kind, the first of which is `plane` (and of lattice `l`): its group's.
  const group = (plane: number, k: number, l = 0) => plane + l * P.groups + (k >> 2);
  const layer = (l: number, i: number) => 4 * (l * n + i);
  for (let i = 0; i < n; i++) {
    // A square whose cell holds no land holds no kinds (tape/src/stands.rs) and 0 in every layer of the field.
    if (!(rows[1 + row * i + LAND_AT] > 0)) continue;
    field.fill(1, layer(FIELD.land, i), layer(FIELD.land, i) + 4);
    const paint = LATTICES.map(() => new Paint()), sward = new Paint(), flowers = new Paint();
    for (let k = 0; k < kinds; k++) {
      if (rows[at + (i * kinds + k) * kindRow + K.root] < 0 || rows[at + (i * kinds + k) * kindRow + K.kg] <= 0) continue;
      // A copy: its crowns' numbers are set below (`standingCrowns`).
      const r = rows.slice(at + (i * kinds + k) * kindRow, at + (i * kinds + k + 1) * kindRow);
      // The share of the square that holds its plants and the loose ground their roots need, as the core found them.
      const mean = Math.max(r[K.rooted], 1e-3), stand = [r[K.height], mean, r[K.need]];
      // The square's ground temperature (the table's second number of a row) and the frozen cover's melting point.
      const lookAs = (design: number) => lookOfRow(r, design, walls, rows[1 + row * i + 1], rows[0], falling);
      // What of it does not stand as plants is the sward (`standsOf`).
      const stands = standsOf(r[K.height], r[K.walls]);
      if (stands < 1) swardOf(r, k, lookAs(DESIGN.herb), 1 - stands, stand, sward, flowers);
      if (stands <= 0) continue;
      const design = designOf(r[K.height], r[K.walls]), look = lookAs(design);
      // Each group's crowns as wide as their wood holds them through the year (`standingCrowns`), as closed as its leaves.
      const groups = groupsOf(r).map(q => {
        const crowns = standingCrowns(q, mean);
        q[K.width] = crowns.width; q[K.cover] = crowns.cover; q[K.crown] = crowns.closed;
        return q;
      });
      if (!groups.length) continue;
      const large = groups[0], c = seenFar(look);
      // How closed its crown is with the leaves it bears this season: how dense times the leaves left.
      const packed = Math.round(look.left * LEFT_STEPS) + Math.min(large[K.crown], 1 - 1e-3);
      const shape = formOf({ leaf: r[K.leaf], frost: r[K.frost], moist: r[K.moist], height: large[K.height], width: large[K.width], seed: r[K.seed] });
      const ofGroups = (plane: number, v: (g: number) => number) => groups.forEach((_, g) => put(plane + k, i, g, v(g)));
      put(group(P.need, k), i, k & 3, 10 ** r[K.moist]);
      put(group(P.roots, k), i, k & 3, mean >= ROOTED_ALL ? 0 : r[K.need]);
      put(group(P.label, k), i, k & 3, r[K.slot]);
      // Its form and how surely as one number (form + `SURE_PACK` x sure).
      [packed, r[K.leaf], shape.form + SURE_PACK * shape.sure, shape.dry].forEach((v, q) => put(P.form + k, i, q, v));
      [...look.from, grownWalls(r[K.walls])].forEach((v, q) => put(P.from + k, i, q, v));
      const from = treeFrom(r[K.walls]);
      [...look.to, from].forEach((v, q) => put(P.to + k, i, q, v));
      [...look.petal, look.bloom].forEach((v, q) => put(P.petal + k, i, q, v));
      ofGroups(P.height, g => groups[g][K.height]);
      ofGroups(P.stem, g => groups[g][K.stem]);
      LATTICES.forEach((a, l) => {
        const on = groups.map(q => marksOn(q, l, clumpOf(q[K.height], from), mean)), chances = on.map(o => o.chance * stands);
        const all = chances.reduce((x, y) => x + y, 0);
        if (all <= 0) return;
        put(group(P.chance, k, l), i, k & 3, all / mean);
        // Of each group the share of the kind's marks here that are of it and the larger groups (the rest: 1).
        let sum = 0;
        for (let g = 0; g < GROUPS; g++) { sum += chances[g] ?? 0; put(P.cut + l * kinds + k, i, g, g < groups.length - 1 ? sum / all : 1); }
        ofGroups(P.width + l * kinds, g => on[g].width);
        if (l === STANDS) ofGroups(P.clump, g => on[g].plants);
        on.forEach((o, g) => {
          if (chances[g] <= 0) return;
          const q = groups[g];
          tallest[l] = Math.max(tallest[l], q[K.height]);
          widest[l] = Math.max(widest[l], o.width);
          // The share of the ground its plants cover, each as closed as its crown.
          paint[l].add(k, r[K.root], c, o.covers * stands * q[K.crown] * look.left, (chances[g] / (a * a)) * o.width * o.width * SHADOW_WIDTHS, [q[K.height], mean, r[K.need]], o.width);
        });
      });
    }
    const first = paint[STANDS].first.k, chanceAt = texel(group(P.chance, first, STANDS), i) + (first & 3);
    if (first >= 0) table[chanceAt] = -table[chanceAt];
    putField(field, i, layer, paint, sward, flowers);
  }
  return { w, h, planes: P, cols, table, field: { w, h, data: field }, tallest, widest, walls };
}

/** Into `field` at square `i` (`layer` the place of a layer's texel): what each lattice's marks (`paint`), the sward and
 * its flowers paint, their stands' means, the shadows and the far layer (`lowOf`). */
function putField(field: Float32Array, i: number, layer: (l: number, i: number) => number, paint: Paint[], sward: Paint, flowers: Paint): void {
  paint.forEach((p, l) => p.put(field, layer(l, i), l === STANDS ? layer(FIELD.standsFirst, i) : undefined, l === STANDS ? layer(FIELD.standsRest, i) : undefined));
  sward.put(field, layer(FIELD.sward, i), layer(FIELD.swardFirst, i), layer(FIELD.swardRest, i));
  [...paint, sward].forEach((p, q) => p.putStand(field, [FIELD.tall, FIELD.rooted, FIELD.need, FIELD.wide].map(f => layer(f, i) + q)));
  paint.forEach((p, l) => { field[layer(FIELD.shade, i) + l] = p.shade(); });
  lowOf(field, i, layer);
  // The flowers: their mean colour times the share of the herbs in flower times the sward's cover, and that share.
  const bloom = flowers.covered / Math.max(sward.covered, 1e-9), cover = 1 - sward.bare;
  flowers.rgb.forEach((v, q) => { field[layer(FIELD.flowers, i) + q] = (v / Math.max(flowers.covered, 1e-9)) * bloom * cover; });
  field[layer(FIELD.flowers, i) + 3] = bloom * cover;
}

/** Into the far layer of `field` at square `i` (`FIELD.low`, `layer` its place): the sward, covering the share of the
 * ground its leaves close by the cover's light law, the lattices under the trees' over it, each darkening the ground
 * below by its shadows and over it as cover_field.wgsl lays them, at their mean (no patches: far away they do not
 * show). */
function lowOf(field: Float32Array, i: number, layer: (l: number, i: number) => number): void {
  const s = layer(FIELD.sward, i), sward = field[s + 3], seen = Math.min(sward, 1);
  const dst = [0, 1, 2].map(q => (sward > 0 ? (field[s + q] / sward) * seen : 0));
  let a = seen;
  for (let l = 0; l < STANDS; l++) {
    const at = layer(l, i), share = Math.min(field[at + 3], 1), shade = CROWN_SHADOW * field[layer(FIELD.shade, i) + l];
    if (share <= 0) continue;
    // Its shadows (black) over what lies below, then its crowns.
    for (let q = 0; q < 3; q++) dst[q] = (field[at + q] / field[at + 3]) * share + dst[q] * (1 - shade) * (1 - share);
    a = share + (shade + a * (1 - shade)) * (1 - share);
  }
  const out = layer(FIELD.low, i);
  dst.forEach((v, q) => { field[out + q] = v; });
  field[out + 3] = a;
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
