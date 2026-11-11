// The plant cover on the GPU (vegetation.ts): the kinds' table and the field (canopy.ts `Kinds`), and how strongly each
// lattice of marks shows at a scale. The live map's ground draws the field where the marks take too few pixels
// (web/shaders/lib/cover_field.wgsl, `fieldBinds`); a draw of web/shaders/cover.wgsl draws the marks over it (`draw`).
// Each lattice lies fixed in the world, one mark a cell, so the marks hold while the camera moves, and a pan or a zoom
// shows them at once.
import { Draw, Gpu, Pass, Texture, Uniforms } from './gpu';
import { Style } from './sprites';
import { FIELD, Kinds, LATTICES, KIND_PLANES, half } from './canopy';
import { clamp } from './earth';

/** Of the uniforms of cover.wgsl (`Cover`) and of cover_field.wgsl (`CoverLook`), four-wide members. */
const COVER_VEC4S = 2 + 3 * LATTICES.length, LOOK_VEC4S = 3;
/** A lattice fainter than this is not drawn as marks: they would not show, and they cost as much as marks that do. */
const FAINTEST = 0.02;
/** Angled, a plant stands up this share of its height on the map (lib/crown.wgsl `STAND`): the rows south of a pixel
 * looked at reach its tallest plant, but no more than `ROWS_MOST` (each row costs a pixel three looks at marks), and a
 * taller plant is drawn as tall as they reach. */
const STAND = 0.5, ROWS_MOST = 2;
/** The css pixels a lattice's cell takes from which its marks start to show, and from which they show fully. */
const FADE_PX = [2, 5];

/** A draw of the marks: its rectangle of the canvas, device px; the metres a device pixel and a css pixel; the world's
 * point at its corner, m; the style; and the live map's bindings (land.ts `lawBinds`). */
export interface Marks { rect: number[]; mpp: number; cssMpp: number; at: [number, number]; style: Style; land: Draw['binds'] }

/** The cover on the GPU: the kinds' table and the field, sent once a frame of the world. */
export class CoverMarks {
  private kinds: Kinds | null = null;
  private tableTex: Texture | null = null;
  private fieldTex: Texture | null = null;
  private due = false;
  /** Metres a square of the map. */
  private cellM = 1;

  /** `alpha`: how strongly the cover covers the map. */
  constructor(private alpha: number) {}

  /** Read the cover from `k`, of a map whose square is `cellM` metres; none: no cover is drawn. */
  set(k: Kinds | null, cellM: number): void {
    this.kinds = k;
    this.cellM = cellM;
    this.due = true;
  }

  /** How strongly the marks of each lattice show where a css pixel takes `cssMpp` m: 0 for a lattice no square holds
   * or whose marks would take too few pixels to show. */
  private shown(cssMpp: number): number[] {
    return LATTICES.map((a, l) => {
      const s = clamp((a / cssMpp - FADE_PX[0]) / (FADE_PX[1] - FADE_PX[0])), shown = s * s * (3 - 2 * s);
      return this.kinds?.held[l] && shown >= FAINTEST ? shown : 0;
    });
  }

  /** Send what waits to the GPU. */
  private upload(gpu: Gpu): void {
    if (!this.due && this.fieldTex) return;
    const k = this.kinds, w = k?.w ?? 1, h = k?.h ?? 1;
    if (this.fieldTex?.w !== w || this.fieldTex.h !== h) {
      if (this.tableTex) gpu.drop(this.tableTex);
      if (this.fieldTex) gpu.drop(this.fieldTex);
      this.tableTex = gpu.texture(w, KIND_PLANES.count * h, 'rgba32float');
      this.fieldTex = gpu.texture(w, h, 'rgba16float', FIELD.count, (k?.field.mips.length ?? 0) + 1);
    }
    if (k) {
      gpu.write(this.tableTex!, k.table);
      gpu.write(this.fieldTex!, Uint16Array.from(k.field.data, half));
      k.field.mips.forEach((d, l) => gpu.write(this.fieldTex!, Uint16Array.from(d, half), l + 1));
    }
    this.due = false;
  }

  /** The bindings of cover_field.wgsl on `gpu` where a css pixel takes `cssMpp` m: the field and how it shows. */
  fieldBinds(gpu: Gpu, cssMpp: number): Draw['binds'] {
    this.upload(gpu);
    const u = new Uniforms(LOOK_VEC4S).vec4(...LATTICES).vec4(...this.shown(cssMpp)).vec4(this.kinds ? 1 : 0, this.alpha);
    return { 14: u.buffer, 15: this.fieldTex!, 16: 'mip' };
  }

  /** Draw marks `m` in pass `p` on `gpu`. */
  draw(gpu: Gpu, p: Pass, m: Marks): void {
    const k = this.kinds;
    if (!k || this.shown(m.cssMpp).every(s => s <= 0)) return;
    const angled = m.style === 'angled';
    const u = new Uniforms(COVER_VEC4S).vec4(...m.at, m.mpp, angled ? 1 : 0).vec4(...k.walls, 0);
    // Each lattice's cell at the draw's corner, split into its whole number and its fraction in double precision, so
    // the GPU's single precision only counts the cells across the view.
    const top = Math.floor(Math.log2(Math.max(k.w, k.h)));
    const grids = LATTICES.map((a, l) => {
      const g = [m.at[0] / a, m.at[1] / a];
      // The mip level whose texels, read about a pixel, hold all the squares its marks' feet (at most two cells away)
      // stand on: half a texel of it reaches two cells and a square.
      const near = Math.min(Math.max(Math.ceil(Math.log2(2 * ((2 * a) / this.cellM + 1))), 0), top);
      const rows = angled ? Math.min(Math.ceil((STAND * k.tallest[l]) / a), ROWS_MOST) : 0;
      return { a, base: g.map(Math.floor), frac: g.map(v => v - Math.floor(v)), near, rows };
    });
    grids.forEach(g => u.vec4(...g.frac, g.a, g.rows));
    grids.forEach(g => u.vec4(g.near));
    grids.forEach(g => u.ivec4(...g.base));
    p.draw('cover', { rect: m.rect, binds: { ...m.land, ...this.fieldBinds(gpu, m.cssMpp), 1: u.buffer, 4: this.tableTex! } }, true);
  }
}
