// The plant cover on the GPU (vegetation.ts): the kinds' table and the field (canopy.ts `Kinds`), and how strongly each
// lattice of marks shows at a scale. The live map's ground draws the field where the marks take too few pixels
// (web/shaders/lib/cover_field.wgsl, `fieldBinds`); before the map, each mark in view is decided once, a texel a cell
// in textures of four bytes a texel (web/shaders/marks.wgsl, `prepare`), and a draw of web/shaders/cover.wgsl (angled,
// from the rendered models) or web/shaders/crowns.wgsl draws the marks over the map (`draw`).
// Each lattice lies fixed in the world, one mark a cell, so the marks hold while the camera moves, and a pan or a zoom
// shows them at once.
import { Draw, Gpu, Pass, Texture, Uniforms } from './gpu';
import { Style } from './style';
import { FIELD, Kinds, LATTICES, PLANES_DOWN, REACH, half } from './canopy';
import { groundDetail } from './detail';

/** Of the uniforms of cover.wgsl (`Cover`) and of cover_field.wgsl (`CoverLook`), four-wide members. */
const COVER_VEC4S = 4 + 3 * LATTICES.length, LOOK_VEC4S = 3;
/** The cells a pixel looks at about its own along each axis (lib/cells.wgsl `MARK_MARGIN`: the farthest a mark reaches,
 * canopy.ts `REACH`), and one more for a pixel at the draw's far edge. */
const MARK_MARGIN = Math.max(...REACH), MARK_SPARE = 1;
/** The words a mark is kept in, a texture each (lib/cells.wgsl `MARK_WORDS`), and their bindings; and the binding of the
 * cells whose marks reach each cell (lib/cells.wgsl `marks_reach`). */
const MARK_BINDINGS = [5, 6, 7, 22], REACH_BINDING = 23;
/** Angled, a plant stands up this share of its height on the map (lib/crown.wgsl `STAND`): the rows south of a pixel
 * looked at, past those its marks reach, reach its tallest plant, but no more than `ROWS_MOST` (each row costs a pixel
 * a look at marks for each cell across), and a taller plant is drawn as tall as they reach. */
const STAND = 0.5, ROWS_MOST = 3;
/** A mark narrower than this many pixels of the screen is not drawn as a thing (lib/draw.wgsl `THING_PX`): a lattice
 * none of whose marks is as wide is not drawn as marks. */
const THING_LEAST_PX = 4;

/** A draw of the marks: its rectangle of the target, px; the metres a pixel of the target and of the screen (which the
 * detail drawn is chosen by); the world's point at its corner, m; the style; and the live map's bindings (land.ts
 * `lawBinds`). */
export interface Marks { rect: number[]; mpp: number; seenMpp: number; at: [number, number]; style: Style; land: Draw['binds'] }

/** The cover on the GPU: the kinds' table and the field, sent once a frame of the world. */
export class CoverMarks {
  private kinds: Kinds | null = null;
  private tableTex: Texture | null = null;
  private fieldTex: Texture | null = null;
  private due = false;
  /** The marks decided this frame (`prepare`): the bindings the cover draws with; the textures they are in, one for
   * each of their words; and the texture of the cells whose marks reach each cell. */
  private decided: Draw['binds'] | null = null;
  /** The program the marks are drawn with: angled from the rendered models once they are loaded (web/shaders/cover.wgsl),
   * else as drawn crowns (web/shaders/crowns.wgsl). */
  private program = 'crowns';
  private marksTex: Texture[] = [];
  private reachTex: Texture | null = null;

  /** `alpha`: how strongly the cover covers the map. */
  constructor(private alpha: number) {}

  /** Read the cover from `k`; none: no cover is drawn. */
  set(k: Kinds | null): void {
    this.kinds = k;
    this.due = true;
  }

  /** Of each lattice, 1 where a pixel of the screen takes `mpp` m and its widest marks take enough pixels to show as
   * things (each shows by its own width, lib/mark.wgsl `decide`), else 0: none of its marks is drawn. */
  private shown(mpp: number): number[] {
    return LATTICES.map((_, l) => ((this.kinds?.widest[l] ?? 0) >= THING_LEAST_PX * mpp ? 1 : 0));
  }

  /** Send what waits to the GPU. */
  private upload(gpu: Gpu): void {
    if (!this.due && this.fieldTex) return;
    const k = this.kinds, w = k?.w ?? 1, h = k?.h ?? 1, tw = (k?.cols ?? 1) * w, th = Math.min(k?.planes.count ?? 1, PLANES_DOWN) * h;
    if (this.fieldTex?.w !== w || this.fieldTex.h !== h || this.tableTex?.w !== tw || this.tableTex.h !== th) {
      if (this.tableTex) gpu.drop(this.tableTex);
      if (this.fieldTex) gpu.drop(this.fieldTex);
      this.tableTex = gpu.texture(tw, th, 'rgba32float');
      this.fieldTex = gpu.texture(w, h, 'rgba16float', FIELD.count);
    }
    if (k) {
      gpu.write(this.tableTex!, k.table);
      gpu.write(this.fieldTex!, Uint16Array.from(k.field.data, half));
    }
    this.due = false;
  }

  /** The bindings of cover_field.wgsl on `gpu` where a pixel of the screen takes `mpp` m, the plants standing up if
   * `angled`: the field and how it shows. */
  fieldBinds(gpu: Gpu, mpp: number, angled: boolean): Draw['binds'] {
    this.upload(gpu);
    // A lattice no square holds is sent with side 0: the field does not read it.
    const u = new Uniforms(LOOK_VEC4S).vec4(...LATTICES.map((a, l) => ((this.kinds?.widest[l] ?? 0) > 0 ? a : 0)), 0).vec4(...this.shown(mpp)).vec4(this.kinds ? 1 : 0, this.alpha, angled ? 1 : 0);
    return { 14: u.buffer, 15: this.fieldTex!, 16: 'mip' };
  }

  /** Decide each mark of draw `m` on `gpu` once, before the map's pass: a texel a cell in view of each lattice that
   * shows, its rows below the last's (web/shaders/marks.wgsl). */
  prepare(gpu: Gpu, m: Marks): void {
    this.decided = null;
    const k = this.kinds, shown = this.shown(m.seenMpp);
    if (!k || shown.every(s => s <= 0)) return;
    const angled = m.style === 'angled', size = [m.rect[2] * m.mpp, m.rect[3] * m.mpp];
    // Each lattice's cell at the draw's corner, split into its whole number and its fraction in double precision, so
    // the GPU's single precision only counts the cells across the view.
    let row = 0;
    const grids = LATTICES.map((a, l) => {
      const g = [m.at[0] / a, m.at[1] / a], frac = g.map(v => v - Math.floor(v));
      const rows = angled ? Math.min(Math.max(Math.ceil((STAND * k.tallest[l]) / a) - REACH[l], 0), ROWS_MOST) : 0;
      // The cells about the view's: from `MARK_MARGIN` before its first to the rows an angled plant stands in past its last.
      const span = (c: number, n: number) => (shown[l] > 0 ? Math.floor(frac[c] + size[c] / a) + 1 + 2 * MARK_MARGIN + MARK_SPARE + n : 0);
      const cells = { cols: span(0, 0), rows: span(1, rows), row };
      row += cells.rows;
      return { a, base: g.map(Math.floor), frac, rows, cells };
    });
    const cols = Math.max(...grids.map(g => g.cells.cols)), was = this.marksTex[0];
    if (!was || was.w < cols || was.h < row) {
      [...this.marksTex, ...(this.reachTex ? [this.reachTex] : [])].forEach(t => gpu.drop(t));
      const made = () => gpu.texture(Math.max(cols, was?.w ?? 0), Math.max(row, was?.h ?? 0), 'rgba8unorm', 1, 1, true);
      this.marksTex = MARK_BINDINGS.map(made);
      this.reachTex = made();
    }
    const u = new Uniforms(COVER_VEC4S).vec4(...m.at, m.mpp, angled ? 1 : 0).vec4(m.seenMpp).vec4(...k.walls, k.planes.kinds).vec4(...grids.map(g => g.cells.row));
    grids.forEach(g => u.vec4(...g.frac, g.a, g.rows));
    grids.forEach((_, l) => u.vec4(REACH[l]));
    grids.forEach(g => u.ivec4(...g.base));
    const detail = groundDetail(gpu);
    const binds: Draw['binds'] = { ...m.land, ...this.fieldBinds(gpu, m.seenMpp, angled), 1: u.buffer, 4: this.tableTex!, 20: detail, 21: 'tile' };
    this.program = angled && detail.w > 1 ? 'cover' : 'crowns';
    // Each word of the marks, then from them the cells whose marks reach each cell (web/shaders/reach.wgsl).
    const cellsOf = (p: Pass, name: string, drawn: Draw['binds']) => {
      for (const g of grids) if (g.cells.rows > 0) p.draw(name, { rect: [0, g.cells.row, g.cells.cols, g.cells.rows], binds: drawn }, false);
      p.end();
    };
    this.marksTex.forEach((t, word) => cellsOf(gpu.pass(t), 'marks', { ...binds, 2: new Uniforms(1).ivec4(word).buffer }));
    const decided = { ...binds, ...Object.fromEntries(MARK_BINDINGS.map((b, word) => [b, this.marksTex[word]])) };
    cellsOf(gpu.pass(this.reachTex!), 'reach', decided);
    this.decided = { ...decided, [REACH_BINDING]: this.reachTex! };
  }

  /** Draw marks `m` in pass `p` on `gpu`, as decided before the pass (`prepare`). */
  draw(p: Pass, m: Marks): void {
    if (this.decided) p.draw(this.program, { rect: m.rect, binds: this.decided }, true);
  }
}
