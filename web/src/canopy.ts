// The cover's kinds as the marks draw them (textures.ts, web/shaders/cover.wgsl): of each square the kinds of cover
// the core sends with each frame (tape/src/stands.rs: a kind a slot of the cell's cover, with its plants as the cover's
// law holds them, how many, how tall, how deep their roots and the share of the square that holds them, its genes and
// its state this month), set out on the marks' three lattices, with their colours and the field the marks too small to
// draw make. No kind is a class: a plant's size, shape and colour are functions of its numbers.
import type { Rgb } from './style';
import { clamp, smooth } from './earth';
import { DESIGN, KindLook, designOf, fromScreen, kindLook, seenFar, strawSeen, toScreen } from './style';
import { formOf } from './form';

/** The side of a cell of each lattice of marks, m: one mark a cell at most. A mark reaches at most a cell from its
 * cell, so it is drawn at most `REACH` sides across; a kind is drawn on the lattices whose side is nearest its crown's
 * width (on a log scale), so the three hold every bush and tree from 5 cm to 5 m across. Herbs have no marks: they are
 * the ground's sward (cover_field.wgsl `sward`). */
export const LATTICES = [0.1, 0.5, 2.5];
/** A mark is drawn at most this many cell sides across (cover.wgsl looks one cell about a pixel). */
const REACH = 2;
/** A kind whose mean rooted share in its square is this near 1 is held at every point of it: its root need is sent as 0
 * (cover.wgsl `holds`), so the marks read no loose ground there. */
const ROOTED_ALL = 0.999;
/** A bush's mark is a clump of its plants this many times as wide as one, so its lobes read at a 10 m scale bar. */
const BUSH_GROW = 2.5;
/** The numbers of a kind in the core's row (tape/src/stands.rs `row`). */
const K = { root: 0, kg: 1, stem: 2, perM2: 3, height: 4, width: 5, cover: 6, crown: 7, wood: 8, leaf: 9, pigment: 10, moist: 11, frost: 12, fuel: 13, act: 14, frozen: 15, seed: 16, rooted: 17, need: 18, walls: 19 } as const;
/** The planes of the kinds' table (lib/mark.wgsl `kind`) for squares of `kinds` kinds (the world's variety, the core's
 * `Config::variety`): of each lattice the chance a cell holds a mark of each kind where its roots hold (over the share
 * of its square that holds it, the core's `Cover::rooted`; on the lattice of stands, `STANDS`, negated for its square's
 * first kind, `Paint`) and the mark's width, m; how wet each kind needs its ground and how deep its loose ground, m (the
 * core's `Cover::need`; 0: its square holds it everywhere, `ROOTED_ALL`); then of each kind its form (height and stem's
 * width, m, how closed its crown is, its capture area per kg, log10 m²/kg), the colours its plants span (style.ts
 * `KindLook`: the first with its walls' share, the second with its design, `designOf`), its flowers (their colour and
 * the share of its plants in flower) and its shape (its crown's form and how surely, form.ts `formOf`, the plants one
 * of its marks on the lattice of stands stands for, and how dry its ground). A plane holds four numbers: one for each kind of a group of four
 * of a square (`groups` such planes for each of the first numbers, and of each lattice), or one kind's four. */
export function planesOf(kinds: number) {
  const g = Math.ceil(kinds / 4), form = 8 * g;
  return { kinds, groups: g, chance: 0, width: 3 * g, need: 6 * g, roots: 7 * g, form, from: form + kinds, to: form + 2 * kinds, petal: form + 3 * kinds, shape: form + 4 * kinds, count: form + 5 * kinds };
}
export type Planes = ReturnType<typeof planesOf>;
/** The kinds' table lays its planes this many under each other, then in the next column (lib/mark.wgsl
 * `PLANES_DOWN`), so its texture stays within what every GPU holds at the largest map and variety. */
export const PLANES_DOWN = 32;
/** The layers of the field (`CoverField`): one a lattice (its marks' plants), then the herbs' sward, the herbs' flowers,
 * of the sward and of the lattice of stands the part its first kind makes (`Paint`), of each lattice and the sward
 * (red, green, blue, alpha) its plants' mean height, m, the mean share of the square that holds them and the mean depth
 * of loose ground their roots need, m (the core's, tape/src/stands.rs), of each lattice its marks' mean width, m, the
 * sward and the lattices under the trees' as one layer, as the field draws them far away (`lowOf`), and 1 where the
 * square's cell holds land (a square with none holds 0 in every layer: cover_field.wgsl blends over the squares with land
 * alone, `on_land`), and of the sward and of the lattice of stands the part the kinds but the first make. */
export const FIELD = { sward: LATTICES.length, flowers: LATTICES.length + 1, swardFirst: LATTICES.length + 2, standsFirst: LATTICES.length + 3, tall: LATTICES.length + 4, rooted: LATTICES.length + 5, need: LATTICES.length + 6, wide: LATTICES.length + 7, low: LATTICES.length + 8, land: LATTICES.length + 9, swardRest: LATTICES.length + 10, standsRest: LATTICES.length + 11, count: LATTICES.length + 12 } as const;
/** The number of a square's row of the core's table that is the share of its cell that is land (tape/src/table.rs `row`). */
const LAND_AT = 10;
/** The lattice whose kinds grow in stands of their own, near and far alike (cover_field.wgsl `stand_pick`): the trees'. */
export const STANDS = 2;

/** A field of the cover: `w` x `h` texels over the map, a texel a square, of each layer (`FIELD`) a reflectance times the
 * share of the ground it covers, and that share (with the shadow its plants cast), RGBA layer by layer; and its levels
 * below (`mipsOf`). */
export interface CoverField { w: number; h: number; data: Float32Array; mips: Float32Array[] }
/** The cover of a frame: the kinds' table (`planesOf` its `planes`, a texel a square, row by row, plane under plane,
 * `PLANES_DOWN` planes a column: `cols` columns), its field, of each lattice whether any square holds its marks and the
 * tallest plant drawn on it, m, and the reflectance of the walls the cover builds (its stems). */
export interface Kinds { w: number; h: number; planes: Planes; cols: number; table: Float32Array; field: CoverField; held: boolean[]; tallest: number[]; walls: Rgb }

// Colour: reflectances in linear light, from a leaf's measured reflectance moved by the numbers the model holds; the
// GPU turns light into the screen's bytes once (draw.wgsl `srgb`). Chemistry 0 gives every part of the cover one
// reflectance (its walls'), so a leaf's own colour is the measured one and only its departures come from the model.
/** A green leaf's reflectance in the visible: about 0.05 red, 0.12 green, 0.04 blue (Gates and others 1965; Knipling
 * 1970). */
const LEAF_REFL: Rgb = [0.05, 0.12, 0.04];
/** More light capacity per m² of capture area is more pigment per area, which absorbs more: per decade above
 * `PIGMENT_MID` (log10 mol/m²/s) the reflectance falls by the factor 10^`PIGMENT_SLOPE` (within `PIGMENT_MOST` either
 * way). */
const PIGMENT_MID = -5.6, PIGMENT_SLOPE = 0.3, PIGMENT_MOST = 2;
/** Thick leaves (capture area per kg below `LEAF_MID`, log10 m²/kg) are darker and duller: per decade thicker, this
 * share less reflectance, and this share of the way to a grey of their brightness (more mesophyll and cuticle absorb
 * and scatter the green peak away; Wright and others 2004 for the thickness). */
const LEAF_MID = 0.3, THICK_DARK = 0.25, THICK_DULL = 0.4;
/** Leaves of dry ground carry hairs and wax that reflect more and greyer: per decade of moisture gene below `MOIST_MID`
 * (the starting cover's, fully active at half the ground's saturation), this far towards `DRY_REFL` (Ehleringer and
 * Björkman 1978, from memory). */
const MOIST_MID = -0.3;
const DRY_REFL: Rgb = [0.11, 0.13, 0.09];
/** Foliage hardy to frost (the frost gene from `TENDER_K` down to `HARDY_K`, K) is darker and bluer: towards
 * `HARDY_REFL` (the needles and waxes of hardy foliage). */
const TENDER_K = 272, HARDY_K = 258;
const HARDY_REFL: Rgb = [0.03, 0.08, 0.06];
/** Short of fuel (fuel under its reserve) a leaf yellows towards `PALE_REFL` (Knipling 1970), at most this share; how
 * it turns dormant, from the cold or from drought, is the style's (style.ts). */
const PALE_REFL: Rgb = [0.10, 0.14, 0.04];
const PALE_MOST = 0.4;
/** A shadow lets through the sky's light and what crowns scatter, about half of the sun's (cover.wgsl `SHADOW`). */
const CROWN_SHADOW = 0.5;
/** A plant's shadow darkens this many times the square of its crown's width of ground: the marks draw it as a soft
 * ellipse about their crown's size beside it (cover.wgsl `plant`), so the field far away is as dark as the marks near. */
const SHADOW_WIDTHS = 0.9;

const mix = (a: Rgb, b: Rgb, t: number): Rgb => a.map((v, i) => v + (b[i] - v) * clamp(t)) as Rgb;

/** The reflectance of the crowns of the kind of row `r` whose walls reflect `walls`: a leaf's, less the more pigment
 * per area and the thicker its leaves (and duller), greyer the drier the ground it is made for, bluer the hardier to
 * frost; its walls showing through an open crown as its wall share is high; yellowed short of fuel. */
function foliage(r: Float32Array, walls: Rgb): Rgb {
  const pigment = clamp(10 ** (-PIGMENT_SLOPE * (r[K.pigment] - PIGMENT_MID)), 1 / PIGMENT_MOST, PIGMENT_MOST);
  const thick = clamp(LEAF_MID - r[K.leaf], 0, 1 / THICK_DARK);
  let c = LEAF_REFL.map(v => v * pigment * (1 - THICK_DARK * thick)) as Rgb;
  const grey = (c[0] + c[1] + c[2]) / 3;
  c = mix(c, [grey, grey, grey], THICK_DULL * thick);
  c = mix(c, DRY_REFL, MOIST_MID - r[K.moist]);
  c = mix(c, HARDY_REFL, smooth(TENDER_K, HARDY_K, r[K.frost]));
  c = mix(c, walls, r[K.wood] * (1 - r[K.crown]));
  return mix(c, PALE_REFL, PALE_MOST * (1 - clamp(r[K.fuel])));
}

/** The look of the kind of row `r` (style.ts `kindLook`), drawn with design `design`, on ground at `t` K where the
 * frozen cover melts at `melt` K, while the ground cools by `falling`, its walls reflecting `walls`. */
function lookOfRow(r: Float32Array, design: number, walls: Rgb, t: number, melt: number, falling: number): KindLook {
  const k = { leaf: r[K.leaf], wood: r[K.wood], moist: r[K.moist], frost: r[K.frost], pigment: r[K.pigment], act: r[K.act], frozen: r[K.frozen], seed: r[K.seed], design, t, melt };
  return kindLook(k, foliage(r, walls), falling);
}

/** The share of a kind of crown width `w`, m, drawn on lattice `l`: between the two lattices whose sides lie either side
 * of it, by where it lies between them on a log scale; all on the first below it and on the last above it. */
function shareOn(w: number, l: number): number {
  const x = Math.log(Math.max(w, 1e-6)), at = LATTICES.map(Math.log);
  const below = l > 0 ? clamp((x - at[l - 1]) / (at[l] - at[l - 1])) : 1, above = l < at.length - 1 ? clamp((at[l + 1] - x) / (at[l + 1] - at[l])) : 1;
  return Math.min(below, above);
}

/** A stand's crowns share its canopy and touch; the marks' places are random, so crowns each as wide as the ground it
 * holds overlap and leave gaps. Marks whose crowns sum to x of the ground cover 1 - e^-x of it: a stand that closes the
 * share c of its ground draws each crown's area -ln(1 - c) / c times as large, so the ground its crowns cover together,
 * overlaps counted once, is the stand's (c at most this, and a mark no wider than its lattice lets it reach). */
const CLOSED_MOST = 0.95;

/** The ground a kg of a herb's walls (its stems and last year's thatch) shades, standing, m² (a cured grassland of 0.1
 * kg/m² of stems closes about two thirds of its ground). */
const STRAW_AREA = 10;

/** The marks of a kind on a lattice: the chance a cell holds one (as a share of the whole square), the width a mark is
 * drawn, m, the plants a mark stands for, and the share of the square the kind's plants on this lattice cover. */
interface MarksOn { chance: number; width: number; plants: number; covers: number }

/** The marks of a kind (row `r`) on lattice `l`, each a clump `grow` plants across, on the share `rooted` of the square
 * that holds it. Its plants stand in patches that cover the share `cover` of the square, `perM2` a m², each crown the
 * ground it holds there (the stand's crown cover over its plants); a cell of a patch holds the plants that fall in it
 * as one mark (a clump where they are many), at most `REACH` sides across, never more marks than plants; a mark is
 * drawn as much wider as closes the stand's canopy at random places (`CLOSED_MOST`). */
function marksOn(r: Float32Array, l: number, grow: number, rooted: number): MarksOn {
  const share = shareOn(r[K.width] * grow, l), a = LATTICES[l];
  if (share <= 0 || r[K.cover] <= 0 || r[K.perM2] <= 0) return { chance: 0, width: 0, plants: 0, covers: 0 };
  const inside = r[K.perM2] / r[K.cover], plants = Math.max((inside * a * a) / (grow * grow), 1);
  const held = Math.min(r[K.width] * grow * Math.sqrt(plants), REACH * a);
  const closed = Math.min(r[K.cover] / rooted, CLOSED_MOST), wider = Math.sqrt(-Math.log(1 - closed) / closed);
  const chance = Math.min(1, (r[K.cover] * a * a) / ((Math.PI / 4) * held * held));
  return { chance: share * chance, width: Math.min(held * wider, REACH * a), plants, covers: share * r[K.cover] };
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

  /** Add kind `k` of starting kind `root` and colour `c`, covering `covers` of the ground and casting `shadow`, its
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
  }

  /** Into `out` at `at`: premultiplied, the layer's mean reflectance times the share of the ground it covers, and that
   * share with the shadow it lays on the ground between (a shadow over the share 1 - e^-(the shadows' area per m²) of
   * it, Poisson-placed plants), so a tall rough canopy is darker than a low one of the same leaves; of its first kind
   * (`firstAt`, if any), its part of that. */
  put(out: Float32Array, at: number, firstAt?: number, restAt?: number): void {
    const cover = 1 - this.bare, shaded = (1 - cover) * -Math.expm1(-this.shadow) * CROWN_SHADOW, n = Math.max(this.covered, 1e-9);
    this.rgb.forEach((v, q) => { out[at + q] = fromScreen(v / n) * cover; });
    out[at + 3] = cover + shaded;
    if (firstAt === undefined || restAt === undefined) return;
    const share = this.first.covers / n, rest = n - this.first.covers;
    this.first.rgb.forEach((v, q) => { out[firstAt + q] = v * share * cover; });
    out[firstAt + 3] = share * (cover + shaded);
    // The rest's colour, as the screen shows its kinds (the sum of all less the first's).
    this.rgb.forEach((v, q) => { out[restAt + q] = rest > 1e-9 ? fromScreen(Math.max(v - this.first.covers * toScreen(this.first.rgb[q]), 0) / rest) * (1 - share) * cover : 0; });
    out[restAt + 3] = (1 - share) * (cover + shaded);
  }

  /** Into `out` at each of `at` (the layers `FIELD.tall`, `rooted`, `need` and `wide`): its plants' mean height, the
   * mean share of the square that holds them (1 where it holds none), their roots' mean need and their marks' mean
   * width. */
  putStand(out: Float32Array, at: number[]): void {
    const n = Math.max(this.covered, 1e-9);
    [this.tall / n, this.covered > 0 ? this.held / this.covered : 1, this.deep / n, this.wide / n].forEach((v, q) => { out[at[q]] = v; });
  }
}

/** The kinds of the core's table `rows` for a map of `w` x `h` squares (`row` numbers a square, tape/src/table.rs):
 * `kinds` kinds a square of `kindRow` numbers, from `at` on; while the ground cools by `falling` (style.ts
 * `cooling`). */
export function kindsOf(w: number, h: number, rows: Float32Array, row: number, at: number, kinds: number, kindRow: number, walls: Rgb, falling: number): Kinds {
  const P = planesOf(kinds), cols = Math.ceil(P.count / PLANES_DOWN), down = Math.min(P.count, PLANES_DOWN);
  const n = w * h, table = new Float32Array(4 * cols * down * n), held = LATTICES.map(() => false), tallest = LATTICES.map(() => 0);
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
      const r = rows.subarray(at + (i * kinds + k) * kindRow, at + (i * kinds + k + 1) * kindRow);
      if (r[K.root] < 0 || r[K.kg] <= 0) continue;
      // The square's ground temperature (the table's second number of a row) and the frozen cover's melting point.
      const design = designOf(r[K.height], r[K.walls]), look = lookOfRow(r, design, walls, rows[1 + row * i + 1], rows[0], falling);
      const c = seenFar(look), crown = r[K.crown] * look.left;
      const shape = formOf({ leaf: r[K.leaf], frost: r[K.frost], moist: r[K.moist], height: r[K.height], width: r[K.width], seed: r[K.seed] });
      // The share of the square that holds its plants and the loose ground their roots need, as the core found them.
      const mean = Math.max(r[K.rooted], 1e-3), stand = [r[K.height], mean, r[K.need]];
      put(group(P.need, k), i, k & 3, 10 ** r[K.moist]);
      put(group(P.roots, k), i, k & 3, mean >= ROOTED_ALL ? 0 : r[K.need]);
      [r[K.height], r[K.stem], crown, r[K.leaf]].forEach((v, q) => put(P.form + k, i, q, v));
      [...look.from, r[K.wood]].forEach((v, q) => put(P.from + k, i, q, v));
      [...look.to, design].forEach((v, q) => put(P.to + k, i, q, v));
      [...look.petal, look.bloom].forEach((v, q) => put(P.petal + k, i, q, v));
      [shape.form, shape.sure].forEach((v, q) => put(P.shape + k, i, q, v));
      put(P.shape + k, i, 3, shape.dry);
      if (design === DESIGN.herb) {
        // A herb is the sward: its plants cover their share of the ground, each as closed as its crown, and its stems
        // stand where its leaves are gone (cured straw), closing the ground as their walls' area does.
        const straw = (1 - Math.exp(-STRAW_AREA * r[K.kg] * r[K.wood])) * (1 - crown), closes = crown + straw;
        const covers = r[K.cover] * closes, mixed = closes > 0 ? straw / closes : 0, s = strawSeen();
        sward.add(k, r[K.root], [0, 1, 2].map(q => c[q] + (s[q] - c[q]) * mixed) as Rgb, covers, 0, stand, 0);
        flowers.add(k, 0, look.petal, covers * look.bloom, 0, stand, 0);
        continue;
      }
      LATTICES.forEach((a, l) => {
        const { chance, width, plants, covers: covered } = marksOn(r, l, design === DESIGN.bush ? BUSH_GROW : 1, mean);
        if (chance <= 0) return;
        if (l === STANDS) put(P.shape + k, i, 2, plants);
        put(group(P.chance, k, l), i, k & 3, chance / mean);
        put(group(P.width, k, l), i, k & 3, width);
        held[l] = true;
        tallest[l] = Math.max(tallest[l], r[K.height]);
        // The share of the ground its plants cover, each as closed as its crown.
        paint[l].add(k, r[K.root], c, covered * crown, (chance / (a * a)) * width * width * SHADOW_WIDTHS, stand, width);
      });
    }
    paint.forEach((p, l) => p.put(field, layer(l, i), l === STANDS ? layer(FIELD.standsFirst, i) : undefined, l === STANDS ? layer(FIELD.standsRest, i) : undefined));
    const first = paint[STANDS].first.k, chanceAt = texel(group(P.chance, first, STANDS), i) + (first & 3);
    if (first >= 0) table[chanceAt] = -table[chanceAt];
    sward.put(field, layer(FIELD.sward, i), layer(FIELD.swardFirst, i), layer(FIELD.swardRest, i));
    [...paint, sward].forEach((p, q) => p.putStand(field, [FIELD.tall, FIELD.rooted, FIELD.need, FIELD.wide].map(f => layer(f, i) + q)));
    lowOf(field, i, layer);
    // The flowers: their mean colour times the share of the herbs in flower times the sward's cover, and that share.
    const bloom = flowers.covered / Math.max(sward.covered, 1e-9), cover = 1 - sward.bare;
    flowers.rgb.forEach((v, q) => { field[layer(FIELD.flowers, i) + q] = (v / Math.max(flowers.covered, 1e-9)) * bloom * cover; });
    field[layer(FIELD.flowers, i) + 3] = bloom * cover;
  }
  return { w, h, planes: P, cols, table, field: { w, h, data: field, mips: mipsOf(field, w, h) }, held, tallest, walls };
}

/** Into the far layer of `field` at square `i` (`FIELD.low`, `layer` its place): the sward, covering the share of the
 * ground its leaves close by the cover's light law, the lattices under the trees' over it, each over the one below as cover_field.wgsl lays them, at their
 * mean (no patches, no grain: far away neither shows). */
function lowOf(field: Float32Array, i: number, layer: (l: number, i: number) => number): void {
  const s = layer(FIELD.sward, i), sward = field[s + 3], seen = Math.min(sward, 1);
  const dst = [0, 1, 2].map(q => (sward > 0 ? (field[s + q] / sward) * seen : 0));
  let a = seen;
  for (let l = 0; l < STANDS; l++) {
    const at = layer(l, i), share = Math.min(field[at + 3], 1);
    if (share <= 0) continue;
    for (let q = 0; q < 3; q++) dst[q] = (field[at + q] / field[at + 3]) * share + dst[q] * (1 - share);
    a = share + a * (1 - share);
  }
  const out = layer(FIELD.low, i);
  dst.forEach((v, q) => { field[out + q] = v; });
  field[out + 3] = a;
}

/** The levels of field `data` (`w` x `h`) below its own, each texel the most of the four (or, at an odd edge, six or
 * nine) above it: cover.wgsl reads a level about a pixel to skip a lattice none of whose marks stands near, so a level
 * holds a lattice wherever any texel under it does. Only the lattices' layers are made (no other layer is read below
 * its own level); the others' levels stay 0. */
function mipsOf(data: Float32Array, w: number, h: number): Float32Array[] {
  const out: Float32Array[] = [];
  let src = data;
  while (w > 1 || h > 1) {
    const nw = Math.max(1, w >> 1), nh = Math.max(1, h >> 1), dst = new Float32Array(FIELD.count * 4 * nw * nh);
    for (let l = 0; l < LATTICES.length; l++) for (let y = 0; y < nh; y++) for (let x = 0; x < nw; x++) {
      const ys = 2 * y, ye = y === nh - 1 ? h : Math.min(2 * y + 2, h), xs = 2 * x, xe = x === nw - 1 ? w : Math.min(2 * x + 2, w);
      for (let c = 0; c < 4; c++) {
        let most = 0;
        for (let sy = ys; sy < ye; sy++) for (let sx = xs; sx < xe; sx++) most = Math.max(most, src[((l * h + sy) * w + sx) * 4 + c]);
        dst[((l * nh + y) * nw + x) * 4 + c] = most;
      }
    }
    out.push(dst);
    w = nw; h = nh; src = dst;
  }
  return out;
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
