// The cover's kinds as the marks draw them (textures.ts, web/shaders/cover.wgsl): of each square the kinds of cover
// the core sends with each frame (tape/src/stands.rs: per kind its plants, made from its mass by the self-thinning and
// buckling laws, its genes and its state this month), set out on the marks' three lattices, with their colours and the
// field the marks too small to draw make. No kind is a class: a plant's size, shape and colour are functions of its
// numbers.
import { clamp, smooth } from './earth';
import type { Rgb } from './sprites';
import { KindLook, designOf, kindLook, seenFar } from './style';

/** The side of a cell of each lattice of marks, m: one mark a cell at most. A mark reaches at most a cell from its
 * cell, so it is drawn at most `REACH` sides across; a kind is drawn on the lattices whose side is nearest its crown's
 * width (on a log scale), so the three hold every plant from a herb of 5 cm to a tree of 5 m across. */
export const LATTICES = [0.1, 0.5, 2.5];
/** A mark is drawn at most this many cell sides across (cover.wgsl looks one cell about a pixel). */
const REACH = 2;
/** The numbers of a kind in the core's row (tape/src/stands.rs `Kind::row`). */
const K = { root: 0, kg: 1, stem: 2, perM2: 3, height: 4, width: 5, cover: 6, crown: 7, wood: 8, leaf: 9, pigment: 10, moist: 11, frost: 12, fuel: 13, act: 14, frozen: 15, seed: 16 } as const;
/** The planes of the kinds' table (cover.wgsl): of each lattice the chance a cell holds a mark of each kind and the
 * mark's width, m; how wet each kind needs its ground; then of each kind its form (height and stem's width, m, how
 * closed its crown is, its capture area per kg, log10 m²/kg), the colours its plants span (style.ts `KindLook`: the
 * first with its walls' share, the second with its design, `designOf`) and its flowers (their colour and the share of
 * its plants in flower). A plane holds four numbers: one for each kind of a square, or one kind's four. */
export const KIND_PLANES = { chance: 0, width: 3, need: 6, form: 7, from: 11, to: 15, petal: 19, count: 23 } as const;
/** Layers of the field (`CoverField`): one a lattice. */
export const FIELD_LAYERS = LATTICES.length;

/** A field of the cover: `w` x `h` texels over the map, a texel a square, of each lattice the reflectance of its marks
 * times the share of the ground they cover, and that share with the shadow they cast, RGBA layer by layer; and its
 * levels below (`mipsOf`). */
export interface CoverField { w: number; h: number; data: Float32Array; mips: Float32Array[] }
/** The cover of a frame: the kinds' table (`KIND_PLANES`, a texel a square, row by row, plane under plane), its field, of
 * each lattice whether any square holds its marks and the tallest plant drawn on it, m, and the reflectance of the walls
 * the cover builds (its stems). */
export interface Kinds { w: number; h: number; table: Float32Array; field: CoverField; held: boolean[]; tallest: number[]; walls: Rgb }

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
/** The sun at 45° lays a plant's shadow as long as it is tall: m of shadow per m of height. */
const SHADOW_RUN = 1;

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

/** The look of the kind of row `r` (style.ts `kindLook`) on ground at `t` K where the frozen cover melts at `melt` K,
 * while the ground cools by `falling`, its walls reflecting `walls`. */
function lookOfRow(r: Float32Array, walls: Rgb, t: number, melt: number, falling: number): KindLook {
  const k = { leaf: r[K.leaf], wood: r[K.wood], moist: r[K.moist], frost: r[K.frost], pigment: r[K.pigment], act: r[K.act], frozen: r[K.frozen], seed: r[K.seed], t, melt };
  return kindLook(k, foliage(r, walls), falling);
}

/** The share of a kind of crown width `w`, m, drawn on lattice `l`: between the two lattices whose sides lie either side
 * of it, by where it lies between them on a log scale; all on the first below it and on the last above it. */
function shareOn(w: number, l: number): number {
  const x = Math.log(Math.max(w, 1e-6)), at = LATTICES.map(Math.log);
  const below = l > 0 ? clamp((x - at[l - 1]) / (at[l] - at[l - 1])) : 1, above = l < at.length - 1 ? clamp((at[l + 1] - x) / (at[l + 1] - at[l])) : 1;
  return Math.min(below, above);
}

/** The marks of a kind (row `r`) on lattice `l`: the chance a cell holds one and its width, m. Its plants stand in
 * patches that cover the share `cover` of the ground, `perM2` a m², each crown as wide as the ground it holds there; a
 * cell of a patch holds the plants that fall in it as one mark of their crowns' area (a clump where they are many), at
 * most `REACH` sides across, and the chance a cell holds one keeps the share of the ground the marks cover the stand's. */
function marksOn(r: Float32Array, l: number): [number, number] {
  const share = shareOn(r[K.width], l), a = LATTICES[l];
  if (share <= 0 || r[K.cover] <= 0 || r[K.perM2] <= 0) return [0, 0];
  const inside = r[K.perM2] / r[K.cover], plants = inside * a * a;
  const width = Math.min(r[K.width] * Math.sqrt(Math.max(plants, 1)), REACH * a);
  const chance = Math.min(1, (r[K.cover] * a * a) / ((Math.PI / 4) * width * width));
  return [share * chance, width];
}

/** The kinds of the core's table `rows` for a map of `w` x `h` squares (`row` numbers a square, tape/src/table.rs):
 * `kinds` kinds a square of `kindRow` numbers, from `at` on; while the ground cools by `falling` (style.ts
 * `cooling`). */
export function kindsOf(w: number, h: number, rows: Float32Array, row: number, at: number, kinds: number, kindRow: number, walls: Rgb, falling: number): Kinds {
  const n = w * h, table = new Float32Array(4 * KIND_PLANES.count * n), held = LATTICES.map(() => false), tallest = LATTICES.map(() => 0);
  const field = new Float32Array(4 * FIELD_LAYERS * n);
  const put = (plane: number, i: number, c: number, v: number) => { table[4 * (plane * n + i) + c] = v; };
  for (let i = 0; i < n; i++) {
    // Of each lattice: its marks' colours summed by the ground each covers, that ground, the ground they leave, and the
    // shadow they cast (the plants a m² times their shadow's area).
    const paint = LATTICES.map(() => ({ rgb: [0, 0, 0], covered: 0, bare: 1, shadow: 0 }));
    for (let k = 0; k < Math.min(kinds, 4); k++) {
      const r = rows.subarray(at + (i * kinds + k) * kindRow, at + (i * kinds + k + 1) * kindRow);
      if (r[K.root] < 0 || r[K.kg] <= 0) continue;
      // The square's ground temperature (the table's second number of a row) and the frozen cover's melting point.
      const look = lookOfRow(r, walls, rows[1 + row * i + 1], rows[0], falling), design = designOf(r[K.height], r[K.wood]);
      const c = seenFar(look, design), crown = r[K.crown] * look.left;
      put(KIND_PLANES.need, i, k, 10 ** r[K.moist]);
      [r[K.height], r[K.stem], crown, r[K.leaf]].forEach((v, q) => put(KIND_PLANES.form + k, i, q, v));
      [...look.from, r[K.wood]].forEach((v, q) => put(KIND_PLANES.from + k, i, q, v));
      [...look.to, design].forEach((v, q) => put(KIND_PLANES.to + k, i, q, v));
      [...look.petal, look.bloom].forEach((v, q) => put(KIND_PLANES.petal + k, i, q, v));
      LATTICES.forEach((a, l) => {
        const [chance, width] = marksOn(r, l);
        if (chance <= 0) return;
        put(KIND_PLANES.chance + l, i, k, chance);
        put(KIND_PLANES.width + l, i, k, width);
        held[l] = true;
        tallest[l] = Math.max(tallest[l], r[K.height]);
        // The share of the ground its marks cover, each as closed as its crown.
        const covers = Math.min(1, (chance * (Math.PI / 4) * width * width) / (a * a)) * crown;
        paint[l].rgb = paint[l].rgb.map((v, q) => v + covers * c[q]);
        paint[l].covered += covers;
        paint[l].bare *= 1 - covers;
        paint[l].shadow += (chance / (a * a)) * width * SHADOW_RUN * r[K.height];
      });
    }
    // Premultiplied: the marks' mean reflectance times the share of the ground they cover; and that share with the
    // shadow they lay on the ground between them (a shadow over the share 1 - e^-(their shadows' area per m²) of it,
    // Poisson-placed plants), so a tall rough canopy is darker than a low one of the same leaves.
    paint.forEach((p, l) => {
      const cover = 1 - p.bare, shaded = (1 - cover) * -Math.expm1(-p.shadow) * CROWN_SHADOW;
      p.rgb.forEach((v, q) => { field[4 * (l * n + i) + q] = (v / Math.max(p.covered, 1e-9)) * cover; });
      field[4 * (l * n + i) + 3] = cover + shaded;
    });
  }
  return { w, h, table, field: { w, h, data: field, mips: mipsOf(field, w, h) }, held, tallest, walls };
}

/** The levels of field `data` (`w` x `h`) below its own, each texel the most of the four (or, at an odd edge, six or
 * nine) above it: cover.wgsl reads a level about a pixel to skip a lattice none of whose marks stands near, so a level
 * holds a lattice wherever any texel under it does. */
function mipsOf(data: Float32Array, w: number, h: number): Float32Array[] {
  const out: Float32Array[] = [];
  let src = data;
  while (w > 1 || h > 1) {
    const nw = Math.max(1, w >> 1), nh = Math.max(1, h >> 1), dst = new Float32Array(FIELD_LAYERS * 4 * nw * nh);
    for (let l = 0; l < FIELD_LAYERS; l++) for (let y = 0; y < nh; y++) for (let x = 0; x < nw; x++) {
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
