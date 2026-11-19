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
import { Draw, Format, Gpu, Made, Pass, notMade } from './gpu';
import { ForestTiles, FromTiles } from './forest';
import { GpuLayer } from './map';
import type { Land } from './land';

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
  /** The draw of the marks `prepare` decided last (none: no cover drawn), and what of it is read from the tiles the
   * marks are kept in (none: all drawn live). */
  private drawn: Marks | null = null;
  private fromTiles: FromTiles | null = null;
  private tiles = new ForestTiles();

  /** The kinds of world `w` from its table `rows` (tape/src/table.rs), or none (another view than the earth's, or a
   * tape). */
  set(w: World | null, rows: Float32Array | null): void {
    const t = w?.meta.tiles;
    if (!w || !rows || !t) { this.marks.set(null); return; }
    const m = w.meta, n = m.w * m.h;
    this.cellM = m.cell_m; this.mapW = m.w; this.mapH = m.h;
    const falling = cooling(w.meta.frames[0].year, (m.lat_north + m.lat_south) / 2);
    this.marks.set(kindsOf(m.w, m.h, rows, t.row, 1 + n * t.row, t.kinds, t.kind_row, t.walls, falling));
  }

  /** The bindings the live map's ground draws the cover's field with (cover_field.wgsl), a square taking `s` css px and a
   * css px `seen` px of the screen. */
  under(gpu: Gpu, s: number, seen: number): Draw['binds'] {
    return this.marks.fieldBinds(gpu, this.cellM / (s * seen));
  }

  /** Decide the marks of the cover on a map whose corner is at (ox, oy) and whose square takes `s` css px, `d` px of the
   * target and `seen` px of the screen a css px, in a view `W` x `H` css px, before the map's pass, adding to
   * `waiting` the programs it goes without (textures.ts `prepare`): from the tiles they are kept in where they are drawn
   * (forest.ts), else live. Returns when to draw ahead the tiles not drawn (`ahead`), ms from now (-1: none). */
  prepare(gpu: Gpu, ox: number, oy: number, s: number, d: number, seen: number, W: number, H: number, waiting: Made[]): number {
    this.drawn = null;
    this.fromTiles = null;
    const land = this.land?.lawBinds(gpu);
    if (!land) return -1;
    const mpp = this.cellM / s;
    // The map's part of the view, on whole device pixels.
    const x0 = Math.floor(Math.max(ox, 0) * d), y0 = Math.floor(Math.max(oy, 0) * d);
    const x1 = Math.ceil(Math.min(ox + this.mapW * s, W) * d), y1 = Math.ceil(Math.min(oy + this.mapH * s, H) * d);
    if (x1 <= x0 || y1 <= y0) return -1;
    this.drawn = { rect: [x0, y0, x1 - x0, y1 - y0], mpp: mpp / d, seenMpp: mpp / seen, at: [(x0 / d - ox) * mpp, (y0 / d - oy) * mpp], land };
    if (!this.marks.shows(gpu, this.drawn.seenMpp) || notMade(gpu, ForestTiles.programs()).length) {
      this.marks.prepare(gpu, this.drawn, waiting);
      return -1;
    }
    const f = this.tiles.draw(gpu, this.marks, this.drawn, this.inputs(seen), W * seen, H * seen, waiting);
    this.fromTiles = f.tiles;
    return f.due;
  }

  /** The key of what the cover's tiles show (forest.ts): the cover read (each frame of the world), the laws' table and
   * the rock, the ground's tiles shown and the depth they are drawn at, while one fades in the time (it changes the
   * ground under the marks), and the screen's pixels a css pixel (which the width of a channel's line is drawn by). */
  private inputs(seen: number): string {
    const t = this.land!.tiles, now = performance.now();
    return [this.marks.version, this.land!.version, t.version, t.depth, t.fading(now) ? now : 0, seen].join(' ');
  }

  /** Draw ahead, between frames, the tiles of the cover the last frame drew live (forest.ts `ahead`). */
  ahead(gpu: Gpu): number { return this.tiles.ahead(gpu, this.marks); }

  /** After a lost context: the tiles the cover is kept in are empty. */
  lost(): void { this.tiles.lost(); }

  warm(gpu: Gpu, targets: (Format | null)[]): void {
    this.marks.warm(gpu, targets);
    for (const t of targets) gpu.warm('forest', t, true);
  }

  /** Draw the cover in pass `p`, as `prepare` decided it. */
  draw(p: Pass): void {
    if (!this.drawn) return;
    if (this.fromTiles) this.tiles.show(p, this.marks, this.drawn.rect, this.fromTiles);
    else this.marks.draw(p, [this.drawn.rect]);
  }
}
