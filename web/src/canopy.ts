// The cover's kinds as the marks draw them (textures.ts, web/shaders/cover.wgsl): of each square the kinds of cover
// the core sends with each frame (tape/src/stands.rs: per kind its plants, made from its mass by the self-thinning and
// buckling laws, its genes and its state this month), set out on the marks' three lattices, with their colours and the
// field the marks too small to draw make. No kind is a class: a plant's size, shape and colour are functions of its
// numbers.
import { clamp, smooth } from './earth';
import { Rgb, hsl } from './sprites';

/** The side of a cell of each lattice of marks, m: one mark a cell at most. A mark reaches at most a cell from its
 * cell, so it is drawn at most `REACH` sides across; a kind is drawn on the lattices whose side is nearest its crown's
 * width (on a log scale), so the three hold every plant from a herb of 5 cm to a tree of 5 m across. */
export const LATTICES = [0.1, 0.5, 2.5];
/** A mark is drawn at most this many cell sides across (cover.wgsl looks one cell about a pixel). */
const REACH = 2;
/** The numbers of a kind in the core's row (tape/src/stands.rs `Kind::row`). */
const K = { root: 0, kg: 1, stem: 2, perM2: 3, height: 4, width: 5, cover: 6, crown: 7, wood: 8, leaf: 9, pigment: 10, moist: 11, frost: 12, fuel: 13, act: 14, frozen: 15 } as const;
/** The planes of the kinds' table (cover.wgsl): of each lattice the chance a cell holds a mark of each kind and the
 * mark's width, m; how wet each kind needs its ground; then of each kind its form (height and stem's width, m, how
 * closed its crown is, its capture area per kg, log10 m²/kg) and its colour (display, 0..1, and its walls' share). A
 * plane holds four numbers: one for each kind of a square, or one kind's four. */
export const KIND_PLANES = { chance: 0, width: 3, need: 6, form: 7, colour: 11, count: 15 } as const;
/** Layers of the field (`CoverField`): one a lattice. */
export const FIELD_LAYERS = LATTICES.length;

/** A field of the cover: `w` x `h` texels over the map, a texel a square, of each lattice the colour of its marks
 * (display, 0..1) times the share of the ground they cover, and that share, as RGBA bytes layer by layer; and its levels
 * below (`mipsOf`). */
export interface CoverField { w: number; h: number; data: Uint8Array; mips: Uint8Array[] }
/** The cover of a frame: the kinds' table (`KIND_PLANES`, a texel a square, row by row, plane under plane), its field, and
 * of each lattice whether any square holds its marks and the tallest plant drawn on it, m. */
export interface Kinds { w: number; h: number; table: Float32Array; field: CoverField; held: boolean[]; tallest: number[] }

// Colour (display, from the numbers; chemistry 0 gives every part of the cover one reflectance, so the hue is the
// viewer's and only its departures come from the model).
/** The foliage of a plant of middle numbers: hue (degrees), saturation and lightness. */
const HUE = 105, SAT = 0.42, LIGHTNESS = 0.34;
/** Light capacity per m² of capture area, log10 mol/m²/s: at `PIGMENT_MID` the middle lightness; per decade more, this
 * much darker (more pigment per area absorbs more). */
const PIGMENT_MID = -5.6, PIGMENT_DARK = 0.12;
/** Capture area per kg, log10 m²/kg: per decade less (thick small leaves), this much darker. */
const LEAF_MID = 0.3, LEAF_DARK = 0.06;
/** The moisture gene (log10 of the wetness a kind is fully active at): per decade less (a plant of dry ground: small,
 * hairy, waxy leaves), this much less saturated, lighter and turned to yellow, degrees (grey-green). */
const DRY_SAT = 0.35, DRY_LIGHT = 0.12, DRY_HUE = 30;
/** The frost gene, K: from `TENDER_K` to `HARDY_K` a plant turns this much to blue, degrees, and darker (the needles and
 * waxes of hardy foliage). */
const TENDER_K = 272, HARDY_K = 258, HARDY_HUE = 25, HARDY_DARK = 0.05;
/** The state this month: a plant short of fuel (its fuel under its reserve) pales towards `PALE`, a dormant one (dry or
 * cold: activity under 1) fades towards `DORMANT`, a frozen share browns towards `FROZEN` (display colours). */
const PALE: Rgb = [196, 190, 112], DORMANT: Rgb = [150, 132, 96], FROZEN: Rgb = [120, 86, 52];
const PALE_MOST = 0.4, DORMANT_MOST = 0.6;

const mix = (a: Rgb, b: Rgb, t: number): Rgb => a.map((v, i) => v + (b[i] - v) * clamp(t)) as Rgb;

/** The foliage of the kind of row `r` (display, 0..255): darker the more light capacity per capture area and the
 * thicker its leaves; greyer the drier the ground it is made for; bluer the hardier to frost; then as its state this
 * month says: paler short of fuel, faded dormant, brown where frozen. */
function foliage(r: Float32Array): Rgb {
  const dry = clamp(-r[K.moist]), hardy = smooth(TENDER_K, HARDY_K, r[K.frost]);
  const dark = PIGMENT_DARK * (r[K.pigment] - PIGMENT_MID) + LEAF_DARK * (LEAF_MID - r[K.leaf]) + HARDY_DARK * hardy;
  let c = hsl(HUE + HARDY_HUE * hardy - DRY_HUE * dry, clamp(SAT - DRY_SAT * dry), clamp(LIGHTNESS - dark + DRY_LIGHT * dry, 0.08, 0.8));
  c = mix(c, PALE, PALE_MOST * (1 - clamp(r[K.fuel])));
  c = mix(c, DORMANT, DORMANT_MOST * (1 - r[K.act]));
  return mix(c, FROZEN, r[K.frozen]);
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

/** The kinds of the core's table `rows` for a map of `w` x `h` squares: `kinds` kinds a square of `kindRow` numbers,
 * from `at` on (tape/src/table.rs). */
export function kindsOf(w: number, h: number, rows: Float32Array, at: number, kinds: number, kindRow: number): Kinds {
  const n = w * h, table = new Float32Array(4 * KIND_PLANES.count * n), held = LATTICES.map(() => false), tallest = LATTICES.map(() => 0);
  const field: CoverField = { w, h, data: new Uint8Array(4 * FIELD_LAYERS * n), mips: [] };
  const put = (plane: number, i: number, c: number, v: number) => { table[4 * (plane * n + i) + c] = v; };
  for (let i = 0; i < n; i++) {
    // Of each lattice: its marks' colours summed by the ground each covers, that ground, and the ground they leave.
    const paint = LATTICES.map(() => ({ rgb: [0, 0, 0], covered: 0, bare: 1 }));
    for (let k = 0; k < Math.min(kinds, 4); k++) {
      const r = rows.subarray(at + (i * kinds + k) * kindRow, at + (i * kinds + k + 1) * kindRow);
      if (r[K.root] < 0 || r[K.kg] <= 0) continue;
      const c = foliage(r);
      put(KIND_PLANES.need, i, k, 10 ** r[K.moist]);
      [r[K.height], r[K.stem], r[K.crown], r[K.leaf]].forEach((v, q) => put(KIND_PLANES.form + k, i, q, v));
      [...c.map(v => v / 255), r[K.wood]].forEach((v, q) => put(KIND_PLANES.colour + k, i, q, v));
      LATTICES.forEach((a, l) => {
        const [chance, width] = marksOn(r, l);
        if (chance <= 0) return;
        put(KIND_PLANES.chance + l, i, k, chance);
        put(KIND_PLANES.width + l, i, k, width);
        held[l] = true;
        tallest[l] = Math.max(tallest[l], r[K.height]);
        // The share of the ground its marks cover, each as closed as its crown.
        const covers = Math.min(1, (chance * (Math.PI / 4) * width * width) / (a * a)) * r[K.crown];
        paint[l].rgb = paint[l].rgb.map((v, q) => v + covers * c[q]);
        paint[l].covered += covers;
        paint[l].bare *= 1 - covers;
      });
    }
    // Premultiplied: the marks' mean colour (0..1) times the share of the ground they cover, and that share.
    paint.forEach((p, l) => {
      const cover = 1 - p.bare, mean = p.rgb.map(v => v / 255 / Math.max(p.covered, 1e-9));
      field.data.set([...mean, 1].map(v => Math.round(clamp(v * cover) * 255)), 4 * (l * n + i));
    });
  }
  field.mips = mipsOf(field);
  return { w, h, table, field, held, tallest };
}

/** The levels of field `f` below its own, each texel the most of the four (or, at an odd edge, six or nine) above it:
 * cover.wgsl reads a level about a pixel to skip a lattice none of whose marks stands near, so a level holds a lattice
 * wherever any texel under it does. */
function mipsOf(f: CoverField): Uint8Array[] {
  const out: Uint8Array[] = [];
  let w = f.w, h = f.h, src = f.data;
  while (w > 1 || h > 1) {
    const nw = Math.max(1, w >> 1), nh = Math.max(1, h >> 1), dst = new Uint8Array(FIELD_LAYERS * 4 * nw * nh);
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
