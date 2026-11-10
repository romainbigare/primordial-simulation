// Vegetation: the plant cover drawn as textures at its real size (handover 10.3, 10.4), not plant by plant. Each place
// holds a mix of kinds of pattern (grass, steppe, tundra, shrubs, woodland, forest, jungle), the cover's field
// (canopy.ts) of the map's squares. The marks (textures.ts) are as many as the cover is closed, at their real size (a tree's crown from the walls
// per m², a tuft of grass), coloured by the cover and, close up, by the genes of the heaviest slot of the cells
// (tape::things), all blended between the texels' centres as the map's colours are. A kind's marks fade in as they grow
// from under a pixel. The GPU draws them at every frame, so a pan, a zoom or a new frame shows them at once.
import { World, decode } from './data';
import { CLASSES, clamp } from './earth';
import type { CellThings } from './avatars';
import { CoverField, CoverNumbers, coverOf } from './canopy';
import { Rgb, Style, plantColour, plantLookOf, woodiness } from './sprites';
import { CoverMarks, LARGEST_MARK } from './textures';
import { Gpu, Pass } from './gpu';
import { GpuLayer } from './map';

/** The pixels a mark takes from which its kind starts to show, and from which it shows fully. */
const FADE_PX = [0.7, 2];
/** How strongly the textures cover the map, so its hill shading still shows through. */
const COVER_ALPHA = 0.85;
/** The colour a woody cover darkens towards (canopy.ts `CH.dark`), as a tall canopy's shade deepens (the map's forest
 * is darker too, handover 10.3); the genes of a place tint the marks by `TINT`. */
const DARK: Rgb = [24, 50, 32], TINT = 0.6;
/** The base foliage of grass, tundra, steppe, shrubs, woodland, forest and jungle (display colours, in the order of the
 * marks, textures.ts); a slot's genes tint it close up. */
const FOLIAGE: Rgb[] = [[124, 158, 84], [136, 142, 112], [168, 160, 106], [112, 118, 78], [92, 124, 70], [60, 104, 62], [44, 116, 56]];

/** The map classes on which cover is drawn (the land that is neither water, ice nor rock). */
const VEG_CLASSES = new Set(['Desert', 'Grassland', 'Forest'].map(n => CLASSES.findIndex(c => c[0] === n)));

/** The cover pattern over the map: what it reads (`set`, `setThings`), drawn by the GPU at every frame. */
export class Vegetation implements GpuLayer {
  /** The cover's field of the map's squares (none: not drawn), and whether the marks hold it. */
  private field: CoverField | null = null;
  private held = false;
  private cellM = 1;
  /** The foliage colour of the heaviest slot of the cells close up (avatars' list), by square. */
  private genes = new Map<number, Rgb>();
  private mapW = 0;
  private mapH = 0;
  private marks = new CoverMarks({ foliage: FOLIAGE, dark: DARK, tint: TINT });
  private tinted = false;
  /** A number that changes when the map's cover changes. */
  private mapSum = NaN;
  /** The style the marks are drawn in. */
  style: Style = 'above';

  /** The frame `f` of world `w` and its map classes `cls`; null for none (another view than the earth's, or a tape
   * without the cover's layers). */
  set(w: World | null, f: number, cls: Uint8Array): void {
    const field = w ? fieldOf(w, f, cls) : null, sum = field ? signature(field) : NaN;
    if (field && this.field && sum === this.mapSum) return;
    if (w) { this.cellM = w.meta.cell_m; this.mapW = w.meta.w; this.mapH = w.meta.h; this.tinted = false; }
    this.field = field ? coverOf(field) : null;
    this.held = false;
    this.mapSum = sum;
  }

  /** The cover colours of the slots in `cells` (tape::things), each square by its cells' heaviest slot. */
  setThings(cells: CellThings[]): void {
    this.genes.clear();
    const best = new Map<number, number>();
    for (const c of cells) for (const sl of c.slots) {
      const [x0, y0, x1, y1] = c.box;
      for (let y = Math.floor(y0); y < Math.ceil(y1); y++) for (let x = Math.floor(x0); x < Math.ceil(x1); x++) {
        const i = y * this.mapW + x;
        if ((best.get(i) ?? -1) >= sl.kg) continue;
        best.set(i, sl.kg);
        this.genes.set(i, plantColour(plantLookOf(sl.genes, sl.kg), woodiness(sl.genes[2])));
      }
    }
    this.tinted = false;
  }

  /** Draw the marks on a map whose corner is at (ox, oy) and whose square takes `s` css px, `d` device px a css px, in
   * a view `W` x `H` css px, from the cover's field of the map's squares. */
  draw(gpu: Gpu, p: Pass, ox: number, oy: number, s: number, d: number, W: number, H: number): void {
    if (!this.field) return;
    const mpp = this.cellM / s;
    // Nothing to draw while the largest marks take less than a pixel.
    if (LARGEST_MARK / mpp < FADE_PX[0]) return;
    if (!this.tinted) { this.marks.setTint(this.mapW, this.mapH, this.tint()); this.tinted = true; }
    if (!this.held) { this.marks.setField(this.field); this.held = true; }
    // The map's part of the view, on whole device pixels.
    const x0 = Math.floor(Math.max(ox, 0) * d), y0 = Math.floor(Math.max(oy, 0) * d);
    const x1 = Math.ceil(Math.min(ox + this.mapW * s, W) * d), y1 = Math.ceil(Math.min(oy + this.mapH * s, H) * d);
    if (x1 <= x0 || y1 <= y0) return;
    const m = this.cellM, whole = [0, 0, this.mapW * m, this.mapH * m];
    this.marks.draw(gpu, p, {
      rect: [x0, y0, x1 - x0, y1 - y0], mpp: mpp / d, cssMpp: mpp, at: [(x0 / d - ox) * mpp, (y0 / d - oy) * mpp],
      field: whole, tint: whole, fade: FADE_PX, alpha: COVER_ALPHA, style: this.style,
    });
  }

  /** The genes' tint of the map's squares: per square its colour times its weight, and its weight (1 where a slot's
   * colour is known), as RGBA bytes. */
  private tint(): Uint8Array {
    const data = new Uint8Array(4 * this.mapW * this.mapH);
    for (const [i, c] of this.genes) { data.set(c.map(v => Math.round(clamp(v, 0, 255))), 4 * i); data[4 * i + 3] = 255; }
    return data;
  }
}

/** A number that changes when the cover of field `f` changes as drawn. */
function signature(f: CoverNumbers): number {
  let h = f.w;
  for (let i = 0; i < f.w * f.h; i++) h = (Math.imul(h, 31) + Math.round(f.kg[i] * 50) * 7 + Math.round(f.wood[i] * 50) * 3 + Math.round(f.leaf[i] * 20) + Math.round(f.t[i]) * 5 + Math.round(f.rain[i] * 1e4) + f.open[i]) | 0;
  return h;
}

/** The frame's cover numbers of world `w` at frame `f`, drawn on the squares whose map class `cls` is land that is
 * neither water, ice nor rock; null for a tape that lacks them. */
function fieldOf(w: World, f: number, cls: Uint8Array): CoverNumbers | null {
  if (!['plants', 'wood', 'leaf', 't_ground', 'rain'].every(n => w.index.has(n))) return null;
  const open = Uint8Array.from(cls, c => (VEG_CLASSES.has(c) ? 1 : 0));
  return { w: w.meta.w, h: w.meta.h, kg: decode(w, f, 'plants'), wood: decode(w, f, 'wood'), leaf: decode(w, f, 'leaf'), t: decode(w, f, 't_ground'), rain: decode(w, f, 'rain'), open };
}
