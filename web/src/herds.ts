// The takers over the live map, one by one where one takes a few pixels (web/shaders/herds.wgsl). Of each square the
// core sends its cell's slots of takers with each frame (tape/src/takers.rs: of each slot its adults and its young, a
// real count, their mean living mass and the genes they are shown by); here each slot becomes its members' size and
// colour, and the GPU sets every member at a place fixed by its square, slot and number, as many as the slot holds.
// No kind is a class: a member's size and colour are functions of its numbers.
import type { Rgb } from './style';
import { smooth } from './earth';
import { mix } from './canopy';
import { Draw, Gpu, Pass, Texture, Uniforms } from './gpu';

/** The numbers of a slot in the core's row (tape/src/takers.rs `row`). */
const T = { root: 0, adults: 1, young: 2, kg: 3, youngKg: 4, coat: 5, keep: 6, reach: 7 } as const;
/** Texels of four numbers a slot holds in the takers' table (herds.wgsl `slot_of`): its adults and young, an adult's
 * and a young one's length, m; its colour and its height, m. */
const PARTS = 2;
/** A body's length grows as the cube root of its mass (geometric similarity): head and body of a mammal of 1 kg about
 * this long, m (about 0.1 m at 30 g, 0.5 m at 3 kg, 1.3 m at 60 kg, 2.4 m at 400 kg; Calder 1984, from memory). */
const LENGTH_1KG = 0.33;
/** A body stands this share of its length tall at the shoulder (snow deeper than this hides it). */
const TALL = 0.5;
/** Single takers show from where a body's length takes this many device pixels, fully from the second (`herds.wgsl`):
 * below, one is less than a pixel and is not drawn. */
const SHOW_PX = [0.25, 0.5];
/** A body is drawn at least this many device pixels long, a young one at least the second: where its true length is
 * shorter (from `SHOW_PX`), it is drawn this large, so it reads as one taker. */
const LEAST_PX = [4, 2.6];
/** The body temperature, K, below which a kind keeps no heat of its own in any ground it lives on: a cold body's skin. */
const WARM_K = 280;

// Colour: a coat's reflectance in linear light, earth tones that read on green, on straw and on snow.
/** A short coat of a kind that searches high (browsers, `reach` from `REACH_LOW` to `REACH_HIGH`, log10 m) is a
 * red-brown, one that searches low a grey-brown. */
const RED_BROWN: Rgb = [0.2, 0.09, 0.04], GREY_BROWN: Rgb = [0.13, 0.1, 0.07];
const REACH_LOW = -1, REACH_HIGH = 0;
/** A thick coat (the `COAT` gene up to its top, 2) is paler and greyer, towards `PALE_COAT`, by at most this share. */
const PALE_COAT: Rgb = [0.3, 0.27, 0.22];
const COAT_MOST = 2, COAT_PALE_MOST = 0.6;
/** A cold body's skin: an olive grey. */
const COLD_SKIN: Rgb = [0.09, 0.1, 0.06];

/** The colour of a slot of row `r`: a cold body's skin, else its coat by where it searches and how thick it is. */
function coatOf(r: Float32Array): Rgb {
  if (r[T.keep] < WARM_K) return COLD_SKIN;
  const short = mix(GREY_BROWN, RED_BROWN, smooth(REACH_LOW, REACH_HIGH, r[T.reach]));
  return mix(short, PALE_COAT, (COAT_PALE_MOST * r[T.coat]) / COAT_MOST);
}

/** The length of a body of `kg`, m. */
const lengthOf = (kg: number) => LENGTH_1KG * Math.cbrt(Math.max(kg, 0));

/** The members of the uniforms of herds.wgsl (`Herds`), four-wide. */
const HERDS_VEC4S = 2;

/** The takers on the GPU: their table, sent once a frame of the world, and the draw over the map. */
export class Herds {
  private table: Float32Array | null = null;
  private w = 1;
  private h = 1;
  private slots = 0;
  /** The longest body any square holds, m: no taker shows where it takes less than `SHOW_PX` pixels. */
  private longest = 0;
  private tex: Texture | null = null;
  private due = false;
  private drawn: Draw | null = null;

  /** Read the takers from the core's table `rows` of a map of `w` x `h` squares: `slots` slots a square of `slotRow`
   * numbers, from `at` on (none: no takers drawn). */
  set(rows: Float32Array | null, w: number, h: number, at: number, slots: number, slotRow: number): void {
    this.drawn = null;
    if (!rows) { this.table = null; return; }
    const n = w * h, table = new Float32Array(4 * PARTS * slots * n);
    let longest = 0;
    for (let i = 0; i < n; i++) for (let x = 0; x < slots; x++) {
      const r = rows.subarray(at + (i * slots + x) * slotRow, at + (i * slots + x + 1) * slotRow);
      if (r[T.root] < 0 || r[T.adults] + r[T.young] <= 0) continue;
      const len = lengthOf(r[T.kg]);
      longest = Math.max(longest, len);
      const put = (part: number, v: number[]) => table.set(v, 4 * ((PARTS * x + part) * n + i));
      put(0, [r[T.adults], r[T.young], len, lengthOf(Math.min(r[T.youngKg], r[T.kg]))]);
      put(1, [...coatOf(r), TALL * len]);
    }
    Object.assign(this, { table, w, h, slots, longest, due: true });
  }

  /** Make the draw of the takers over a rectangle `rect` of the canvas, device px, whose corner is at `at` in the world,
   * m, a device pixel taking `e` m, with the live map's bindings `land`: none where no taker would take a pixel. */
  prepare(gpu: Gpu, rect: number[], at: [number, number], e: number, land: Draw['binds']): void {
    this.drawn = null;
    if (!this.table || this.longest / e < SHOW_PX[0]) return;
    if (this.tex?.w !== this.w || this.tex.h !== PARTS * this.slots * this.h) {
      if (this.tex) gpu.drop(this.tex);
      this.tex = gpu.texture(this.w, PARTS * this.slots * this.h, 'rgba32float');
      this.due = true;
    }
    if (this.due) { gpu.write(this.tex, this.table); this.due = false; }
    const u = new Uniforms(HERDS_VEC4S).vec4(...at, e, this.slots).vec4(...SHOW_PX, ...LEAST_PX);
    this.drawn = { rect, binds: { ...land, 1: u.buffer, 4: this.tex } };
  }

  /** Draw the takers in pass `p`, as `prepare` made the draw. */
  draw(p: Pass): void {
    if (this.drawn) p.draw('herds', this.drawn, true);
  }
}
