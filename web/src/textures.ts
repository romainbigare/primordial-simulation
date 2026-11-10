// The marks of the plant cover (vegetation.ts), drawn by the map's renderer (gpu.ts) at every frame, a draw of
// web/shaders/lib/marks.wgsl over the view (cover_live.wgsl over the live map, which masks each mark at its foot by the
// laws inside a cell there; cover.wgsl over a tape's): each kind of mark (a tuft of grass, of tundra or of steppe, a shrub, a tree's
// crown of 4, 8 or 14 m) lies on its own grid fixed in the world, one mark a grid cell, so the marks hold while the
// camera moves, and a pan or a zoom shows them at once. The shader reads the field of the cover (`CoverField`, its
// channels `CH`) and the genes' tint; this sets out the grids and how strongly each kind shows.
import { Gpu, Pass, Texture, Uniforms } from './gpu';
import { Rgb, Style } from './sprites';
import { CH, COVER_LAYERS, CoverField, LATTICES, TREES, TUFTS } from './canopy';

/** The kinds of mark the shader holds at most (cover.wgsl `KINDS`). */
const KINDS = 8;
/** A kind fainter than this is not drawn: it would not show, and it costs as much as one that does. */
const FAINTEST = 0.02;
/** The largest mark, m: nothing shows while it takes less than the pixels a mark starts to show at. */
export const LARGEST_MARK = Math.max(...LATTICES.map(l => l.mark));
/** Of the uniforms of marks.wgsl (`Cover`), four-wide members. */
const COVER_VEC4S = 7 + 4 * KINDS;
/** The colours of the marks (display, 0..255): the foliage of grass, tundra, steppe, shrubs, woodland, forest and jungle,
 * the colour a woody cover darkens towards, and how strongly the genes of a place tint them. */
export interface Palette { foliage: Rgb[]; dark: Rgb; tint: number }

/** A draw of the marks: its rectangle of the canvas, device px; the metres a device pixel and a css pixel; the world's
 * point at its corner, m; the field's window and the window of the genes' tint (x, y, w, h in m); the css pixels a
 * mark takes from which its kind starts to show and from which it shows fully; how strongly it covers the map; the
 * style; and the live map's bindings its marks are masked by (land.ts `lawBinds`; null: a tape's map, none masked). */
export interface Marks { rect: number[]; mpp: number; cssMpp: number; at: [number, number]; field: number[]; tint: number[]; fade: number[]; alpha: number; style: Style; land: Record<number, ArrayBuffer | Texture> | null }

/** The marks drawn on the GPU: the field and the genes' tint they read, on the GPU once drawn. */
export class CoverMarks {
  private field: CoverField | null = null;
  private fieldTex: Texture | null = null;
  /** Whether any texel of the field holds marks of each lattice: the GPU skips those that none holds. */
  private held: boolean[] = [];
  private tint: { w: number; h: number; data: Uint8Array } | null = null;
  private tintTex: Texture | null = null;
  /** What waits to be sent to the GPU. */
  private fieldDue = false;
  private tintDue = false;

  constructor(private palette: Palette) {}

  /** Read the cover from field `f`. */
  setField(f: CoverField): void {
    this.field = f;
    const n = f.w * f.h;
    this.held = LATTICES.map((_, l) => { const from = (l >> 2) * n * 4 + (l & 3); for (let i = 0; i < n; i++) if (f.data[from + 4 * i]) return true; return false; });
    this.fieldDue = true;
  }

  /** Read the genes' tint, `w` x `h` texels over the map: per texel its colour times its weight, and its weight (RGBA
   * bytes). */
  setTint(w: number, h: number, data: Uint8Array): void {
    this.tint = { w, h, data };
    this.tintDue = true;
  }

  /** Draw marks `m` in pass `p` on `gpu`. */
  draw(gpu: Gpu, p: Pass, m: Marks): void {
    const f = this.field, t = this.tint;
    if (!f || !t) return;
    if (this.fieldDue) {
      if (this.fieldTex) gpu.drop(this.fieldTex);
      const mips = f.mips;
      this.fieldTex = gpu.texture(f.w, f.h, 'rgba8unorm', COVER_LAYERS, mips.length + 1);
      gpu.write(this.fieldTex, f.data);
      mips.forEach((d, k) => gpu.write(this.fieldTex!, d, k + 1));
      this.fieldDue = false;
    }
    if (this.tintDue) {
      if (this.tintTex?.w !== t.w || this.tintTex.h !== t.h) { if (this.tintTex) gpu.drop(this.tintTex); this.tintTex = gpu.texture(t.w, t.h, 'rgba8unorm'); }
      gpu.write(this.tintTex, t.data);
      this.tintDue = false;
    }
    // The windows as uv = uv0 + m × per, m the metres from the draw's corner.
    const window = ([x, y, w, h]: number[]) => [(m.at[0] - x) / w, (m.at[1] - y) / h, 1 / w, 1 / h];
    const u = new Uniforms(COVER_VEC4S).vec4(...window(m.field)).vec4(...window(m.tint)).vec4(...m.at)
      .vec4(m.mpp, m.style === 'angled' ? 1 : 0, m.alpha, this.palette.tint).vec4(...this.palette.dark.map(v => v / 255))
      .ivec4(LATTICES.length, TUFTS, TREES).ivec4(CH.dark, CH.woodland, CH.jungle);
    for (let k = 0; k < KINDS; k++) u.vec4(...(this.palette.foliage[k] ?? [0, 0, 0]).map(v => v / 255));
    // Each grid's cell at the draw's corner, split into its whole number and its fraction in double precision, so the
    // GPU's single precision only counts the cells across the view.
    const texel = Math.min(m.field[2] / f.w, m.field[3] / f.h), top = Math.floor(Math.log2(Math.max(f.w, f.h)));
    const grids = LATTICES.map((l, k) => {
      const step = l.mark * l.spacing, g = [m.at[0] / step, m.at[1] / step];
      // The mip level whose texels, read about a pixel, hold all the texels its marks' feet (at most two cells away)
      // read: half a texel of it reaches two cells and a texel.
      const near = Math.min(Math.max(Math.ceil(Math.log2(2 * ((2 * step) / texel + 1))), 0), top);
      const px = l.mark / m.cssMpp, s = Math.min(Math.max((px - m.fade[0]) / (m.fade[1] - m.fade[0]), 0), 1), shown = s * s * (3 - 2 * s);
      return { step, mark: l.mark, base: g.map(Math.floor), frac: g.map(v => v - Math.floor(v)), near, fade: this.held[k] && shown >= FAINTEST ? shown : 0 };
    });
    for (let k = 0; k < KINDS; k++) { const g = grids[k]; u.vec4(...(g ? [...g.frac, g.step, g.mark] : [])); }
    for (let k = 0; k < KINDS; k++) { const g = grids[k]; u.vec4(...(g ? [g.fade, g.near] : [])); }
    for (let k = 0; k < KINDS; k++) { const g = grids[k]; u.ivec4(...(g ? g.base : [])); }
    p.draw(m.land ? 'cover_live' : 'cover', { rect: m.rect, binds: { ...m.land, 1: u.buffer, 2: this.fieldTex!, 3: 'mip', 4: this.tintTex!, 5: 'smooth' } }, true);
  }

}
