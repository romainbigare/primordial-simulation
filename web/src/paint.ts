// The field of the cover (canopy.ts, cover_field.wgsl): what the kinds of a square paint the ground with, layer by
// layer, near and far.
import type { Rgb } from './style';
import { KindLook, fromScreen, seenFar, strawSeen, toScreen } from './style';
import { K } from './kind_row';
import { LATTICES, STANDS } from './lattices';

/** The layers of the field (`CoverField`): one a lattice (its marks' plants), then the herbs' sward, the herbs' flowers,
 * of the sward and of the lattice of stands the part its first kind makes (`Paint`), of each lattice and the sward
 * (red, green, blue, alpha) its plants' mean height, m, the mean share of the square that holds them and the mean depth
 * of loose ground their roots need, m (the core's, tape/src/stands.rs), of each lattice its marks' mean width, m, and the
 * share of the ground its plants leave that their shadows darken (`Paint.put`), with in the shadows' fourth number the
 * share of the ground under crowns whose leaves fall this month (`leavesFalling`), the sward and the lattices under the
 * trees' as one layer, as the field draws them far away (`lowOf`), 1 where the square's cell holds land or a kind
 * stands in it (a square with neither holds 0 in every layer: cover_field.wgsl blends over the squares with cover
 * alone, `on_land`), and of the sward and of the lattice of stands the part the kinds but the first make; and of the
 * plants that stand in liquid (`wet`): the share of the square's cover that is of kinds the core stands on its ground
 * under liquid there (their wet share), the depth they stand in on their mean by cover, m, and its herbs' (the
 * sward's) depth and height on their mean by cover, m. A square's field is that of the first patch of its cell's
 * cover; after it two layers of where its cell's other patches lie (canopy.ts `PLACES`), and, in a frame where any cell has
 * patches, the field of each other patch, made of its own kinds alone (canopy.ts `patchLayer`). */
export const FIELD = { sward: LATTICES.length, flowers: LATTICES.length + 1, swardFirst: LATTICES.length + 2, standsFirst: LATTICES.length + 3, tall: LATTICES.length + 4, rooted: LATTICES.length + 5, need: LATTICES.length + 6, wide: LATTICES.length + 7, shade: LATTICES.length + 8, low: LATTICES.length + 9, land: LATTICES.length + 10, swardRest: LATTICES.length + 11, standsRest: LATTICES.length + 12, wet: LATTICES.length + 13, count: LATTICES.length + 14 } as const;
/** A shadow lets through the sky's light and what crowns scatter, about half of the sun's (draw.wgsl
 * `CROWN_SHADOW`). */
const CROWN_SHADOW = 0.42;
/** The ground a kg of a herb's walls (its stems and last year's thatch) shades, standing, m² (a cured grassland of 0.1
 * kg/m² of stems closes about two thirds of its ground). */
const STRAW_AREA = 10;

/** What a layer of the field (`FIELD`) gathers in a square: its kinds' colours as the screen shows them (a mix of colours
 * on a screen averages what it shows, not the light: style.ts `seenFar`) summed by the ground each covers, that
 * ground, the ground they leave, the shadow they cast (the plants a m² times their shadow's area), their heights,
 * rooted shares, roots' needs and marks' widths summed by the ground each covers, and its first kind:
 * the one first in the order of starting kinds, then slots (labels a kind keeps, so neighbouring squares agree and two
 * slots of one starting kind do not trade the place), with its colour and the ground it covers. */
export class Paint {
  rgb: Rgb = [0, 0, 0];
  covered = 0;
  bare = 1;
  shadow = 0;
  tall = 0;
  held = 0;
  deep = 0;
  wide = 0;
  first = { order: Infinity, rgb: [0, 0, 0] as Rgb, covers: 0, k: -1 };

  /** Add (a group of) kind `k`, `order` in the order of first kinds, of colour `c`, covering `covers` of the ground and casting `shadow`, its
   * plants `height` m tall, held by the share `rooted` of the square, their roots needing `need` m of loose ground,
   * their marks `width` m wide (0: none, the sward). */
  add(k: number, order: number, c: Rgb, covers: number, shadow: number, [height, rooted, need]: number[], width: number): void {
    // A kind of marks that covers none of the ground here (its plants bare) is still one of the layer's kinds, in its
    // order.
    if (covers <= 0) {
      if (order < this.first.order) this.first = { order, rgb: c, covers: 0, k };
      return;
    }
    const rgb = this.rgb;
    rgb[0] += covers * toScreen(c[0]); rgb[1] += covers * toScreen(c[1]); rgb[2] += covers * toScreen(c[2]);
    this.covered += covers;
    this.tall += covers * height;
    this.held += covers * rooted;
    this.deep += covers * need;
    this.wide += covers * width;
    this.bare *= 1 - covers;
    this.shadow += shadow;
    if (order < this.first.order) this.first = { order, rgb: c, covers, k };
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

/** Into `sward` and `flowers`: the share `part` of the kind `k` of row `r` (`order` in the order of first kinds,
 * `Paint`) that is the sward, of look `look`, its plants `stand` (height, rooted share, roots' need): they cover their
 * share of the ground, each as closed as its crown, and their stems stand where their leaves are gone (cured straw), closing the ground as their walls'
 * area does. */
export function swardOf(r: Float32Array, k: number, order: number, look: KindLook, part: number, stand: number[], sward: Paint, flowers: Paint): void {
  const c = seenFar(look), crown = r[K.crown] * look.left;
  const straw = (1 - Math.exp(-STRAW_AREA * r[K.kg] * r[K.wood])) * (1 - crown), closes = crown + straw;
  const covers = part * r[K.cover] * closes, mixed = closes > 0 ? straw / closes : 0, s = strawSeen();
  if (covers > 0) sward.add(k, order, [0, 1, 2].map(q => c[q] + (s[q] - c[q]) * mixed) as Rgb, covers, 0, stand, 0);
  flowers.add(k, 0, look.petal, covers * look.bloom, 0, stand, 0);
}

/** The share of the ground whose leaves fall this month under crowns that close `closed` of it in leaf, of which
 * `left` is left this season, while the ground cools by `falling` (style.ts `cooling`): the leaves a kind has lost
 * fall as the season turns, and none while it warms. */
export function leavesFalling(closed: number, left: number, falling: number): number {
  return closed * (1 - left) * falling;
}

/** Into `field` at square `i` (`layer` the place of a layer's texel of a square): what each lattice's marks (`paint`), the sward and
 * its flowers paint, their stands' means, the shadows with the share `shed` whose leaves fall (`leavesFalling`), and the far
 * layer (`lowOf`). */
export function putField(field: Float32Array, i: number, layer: (l: number, i: number) => number, paint: Paint[], sward: Paint, flowers: Paint, shed: number): void {
  paint.forEach((p, l) => p.put(field, layer(l, i), l === STANDS ? layer(FIELD.standsFirst, i) : undefined, l === STANDS ? layer(FIELD.standsRest, i) : undefined));
  sward.put(field, layer(FIELD.sward, i), layer(FIELD.swardFirst, i), layer(FIELD.swardRest, i));
  [...paint, sward].forEach((p, q) => p.putStand(field, [FIELD.tall, FIELD.rooted, FIELD.need, FIELD.wide].map(f => layer(f, i) + q)));
  paint.forEach((p, l) => { field[layer(FIELD.shade, i) + l] = p.shade(); });
  field[layer(FIELD.shade, i) + 3] = Math.min(shed, 1);
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

