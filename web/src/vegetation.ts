// Vegetation: the plant cover of the live map at its real size (handover 10.3, 10.4), plant by plant where a plant takes
// a few pixels and as a field of the same colours in the ground where it takes less. Of each square the core sends its
// kinds of cover with each frame (tape/src/stands.rs); canopy.ts sets them out on three lattices of marks, and the GPU
// draws them at every frame (textures.ts, web/shaders/cover.wgsl, web/shaders/lib/cover_field.wgsl), so a pan, a zoom
// or a new frame shows them at once. A tape's map holds only totals per square and draws the cover in its ground's
// colours (colour.wgsl).
import { World } from './data';
import { kindsOf } from './canopy';
import { cooling } from './style';
import { CoverMarks, Marks } from './textures';
import { Draw, Gpu, Pass } from './gpu';
import { GpuLayer } from './map';
import type { Land } from './land';
import type { Fire } from './fire';
import { frameOf } from './field-window';

/** How strongly the marks cover the map, so its hill shading still shows through them (the field in the ground is lit
 * by it). */
const COVER_ALPHA = 0.85;

/** The cover over the live map: what it reads (`set`), drawn by the GPU at every frame. */
export class Vegetation implements GpuLayer {
  private marks = new CoverMarks(COVER_ALPHA);
  private cellM = 1;
  private mapW = 0;
  private mapH = 0;
  /** The live map, whose laws mask each mark at its foot (none: a tape's map, no cover drawn over it). */
  land: Land | null = null;
  /** The draw of the marks `prepare` decided last (none: no cover drawn). */
  private drawn: Marks | null = null;

  /** `fire`: what fire left on the live map, which scorches the plants on a fresh scar (fire.wgsl `scorched`). */
  constructor(private fire: Fire) {}

  /** The kinds of world `w` from its table `rows` (tape/src/table.rs), or none (another view than the earth's, or a
   * tape). */
  set(w: World | null, rows: Float32Array | null): void {
    const t = w?.meta.tiles;
    if (!w || !rows || !t) { this.marks.set(null); return; }
    const m = w.meta, n = m.w * m.h;
    this.cellM = m.cell_m; this.mapW = m.w; this.mapH = m.h;
    const falling = cooling(w.meta.frames[0].year, (m.lat_north + m.lat_south) / 2);
    // The kinds a square holds this frame: the table's last number (tape/src/table.rs).
    this.marks.set(kindsOf(m.w, m.h, rows, t.row, 1 + n * t.row, rows[rows.length - 1], t.kind_row, t.walls, falling));
  }

  /** The bindings the live map's ground draws the cover's field with (cover_field.wgsl), a square taking `s` css px and a
   * css px `seen` px of the screen. */
  under(gpu: Gpu, s: number, seen: number): Draw['binds'] {
    return this.marks.fieldBinds(gpu, this.cellM / (s * seen));
  }

  /** Decide the marks of the cover on a map whose corner is at (ox, oy) and whose square takes `s` css px, `d` px of the
   * target and `seen` px of the screen a css px, in a view `W` x `H` css px of which only the part `clip` of the target
   * (x, y, w, h px) is drawn where given, with fire as it stands at time `now` (performance.now, ms: the ground's), before
   * the map's pass, adding to `waiting` the programs it goes without (textures.ts `prepare`): decided again only where
   * what they read changed. */
  prepare(gpu: Gpu, ox: number, oy: number, s: number, d: number, seen: number, W: number, H: number, now: number, waiting: string[], clip?: number[]): void {
    this.drawn = null;
    const laws = this.land?.lawBinds(gpu);
    if (!laws) return;
    const mpp = this.cellM / s;
    // The map's part of the view, on whole device pixels, inside the clip: only the marks of the cells drawn are
    // decided, with those of the cells about them whose marks reach into them (textures.ts `MARK_MARGIN`).
    const c = clip ?? [0, 0, Infinity, Infinity];
    const x0 = Math.max(Math.floor(Math.max(ox, 0) * d), c[0]), y0 = Math.max(Math.floor(Math.max(oy, 0) * d), c[1]);
    const x1 = Math.min(Math.ceil(Math.min(ox + this.mapW * s, W) * d), c[0] + c[2]), y1 = Math.min(Math.ceil(Math.min(oy + this.mapH * s, H) * d), c[1] + c[3]);
    if (x1 <= x0 || y1 <= y0) return;
    const fire = this.fire.planeBinds(gpu, frameOf(ox * d, oy * d, mpp / d, mpp / seen), now);
    this.drawn = { rect: [x0, y0, x1 - x0, y1 - y0], mpp: mpp / d, seenMpp: mpp / seen, at: [(x0 / d - ox) * mpp, (y0 / d - oy) * mpp], land: { ...laws, ...fire } };
    // (While a tile fades in, the ground the marks stand on changes at each frame: they are decided anew.)
    const land = this.land!, fading = land.tiles.fading(performance.now()) ? performance.now() : 0;
    this.marks.prepare(gpu, this.drawn, waiting, [land.contents(), fading, this.fire.contents(now)].join('|'));
  }

  /** Draw the cover in pass `p`, as `prepare` decided it. */
  draw(p: Pass): void {
    if (this.drawn) this.marks.draw(p, this.drawn);
  }
}
