// The plant cover's marks drawn once into tiles fixed in the world and kept (web/shaders/forest.wgsl), so a frame whose
// view moves within a band of scales, or does not move, reads each pixel's cover from a tile instead of looking at the
// marks about it (cover.wgsl, a few hundred texture reads a pixel). A band's tiles take `BAND` times the metres a texel
// of the next finer one; the view reads the band whose texels are as large as its pixels or up to `BAND` times smaller,
// so a tile's texel is never larger than a pixel. Each tile is the cover program's own picture of its square of ground
// (textures.ts `prepare` and `draw` over its rectangle, the marks about it decided with it, so a crown or a shadow that
// reaches over its edge is in both tiles), with a gutter of a texel. A tile holds while what it was drawn from holds
// (the key of what it shows, vegetation.ts `inputs`); where the view's tile is not drawn, or is of another key, the live
// cover draws there. Tiles are drawn between the map's frames (`ahead`), only once the key has held `STEADY_MS` (not
// while the world plays or the ground's tiles come), `DRAWN_A_FRAME` of a screen at a time, the view's middle first; the
// map is drawn anew from them once the view's are all drawn. The least lately seen go first once the texture's layers
// are full.
import { Draw, Gpu, Made, Pass, Texture, Uniforms } from './gpu';
import { MAP_TARGET } from './map';
import type { CoverMarks, Marks } from './textures';

/** A tile's texels inside its gutter along each side, and of the gutter (forest.wgsl `TILE`, `GUTTER`). */
const TILE = 256, GUTTER = 1, SIDE = TILE + 2 * GUTTER;
/** The metres a texel of one band over the next finer one. */
const BAND = Math.SQRT2;
/** The format the tiles are kept in: the cover's picture as the map's targets hold it. */
const TILES_FORMAT = MAP_TARGET;
/** The tiles kept: as many as the screen's pixels in tiles, at the finest of a band (`BAND` squared texels a pixel),
 * times this, for a pan; and no fewer than `LEAST_LAYERS`. */
const KEPT_SPARE = 1.5, LEAST_LAYERS = 16;
/** The texels of tiles drawn at a time between frames at most, as a share of the screen's pixels (each costs what a
 * pixel of the live cover does), and at least one tile. */
const DRAWN_A_FRAME = 0.5;
/** What a tile shows must have held this long before tiles are drawn, ms: a world that plays changes it each of its
 * frames, and its live cover is drawn as before. */
const STEADY_MS = 400;
/** Of the uniforms of forest.wgsl, four-wide members. */
const FOREST_VEC4S = 2;

/** A tile kept: its layer, the key of what it shows, and the number of the last frame that showed it. */
interface Kept { layer: number; key: string; seen: number }
/** A tile of a band: its column and row, and its name. */
interface Slot { x: number; y: number; id: string }

/** What a frame draws of the cover from the tiles: the bindings of forest.wgsl, and the rectangles of the target (x, y,
 * w, h px) the live cover draws where no tile of the key is drawn. */
export interface FromTiles { binds: Draw['binds']; live: number[][] }

export class ForestTiles {
  private store: Texture | null = null;
  private page: Texture | null = null;
  private kept = new Map<string, Kept>();
  private free: number[] = [];
  private frame = 0;
  /** The key of what the tiles show, and since when it holds (performance.now, ms). */
  private key = '';
  private since = 0;
  /** The last frame's tiles not drawn: its draw, its band's metres a texel, the tiles, and how many to draw at a time;
   * and whether a tile of this key could not be drawn (the marks' programs or models are not ready, the frame that
   * waits for them draws again; or no layer is free of the view's tiles). */
  private wanted: { m: Marks; e: number; missing: Slot[]; most: number } | null = null;
  private stuck = false;

  /** The programs the tiles are drawn and read with. */
  static programs(): Made[] { return [['forest', null, true], ['forest', MAP_TARGET, true]]; }

  /** The cover of draw `m` (the target's pixels `m.rect`, a screen of `w` x `h` px) from the tiles of `key` (what the
   * cover is drawn from), and when the tiles of the view not drawn yet are to be drawn (`ahead`, ms from now; -1:
   * none). Where they are not, the live cover draws: `marks` is left decided for the view. */
  draw(gpu: Gpu, marks: CoverMarks, m: Marks, key: string, w: number, h: number, waiting: Made[]): { tiles: FromTiles; due: number } {
    const now = performance.now();
    if (key !== this.key) { this.key = key; this.since = now; this.stuck = false; }
    this.frame++;
    const band = Math.floor(Math.log(m.seenMpp) / Math.log(BAND) + 1e-9), e = BAND ** band, side = TILE * e;
    this.ensure(gpu, w, h);
    // The view's tiles, the middle first.
    const x0 = Math.floor(m.at[0] / side), y0 = Math.floor(m.at[1] / side);
    const x1 = Math.floor((m.at[0] + m.rect[2] * m.mpp) / side), y1 = Math.floor((m.at[1] + m.rect[3] * m.mpp) / side);
    const cols = x1 - x0 + 1, rows = y1 - y0 + 1, mx = (x0 + x1) / 2, my = (y0 + y1) / 2;
    const slots: Slot[] = [];
    for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) slots.push({ x, y, id: `${band} ${x} ${y}` });
    slots.sort((a, b) => Math.hypot(a.x - mx, a.y - my) - Math.hypot(b.x - mx, b.y - my));
    // The page: each tile's layer, or -1; and the rectangle of the target of each tile not drawn, on whole pixels (a
    // pixel is its tile's where its middle lies in it, as forest.wgsl reads them).
    const data = new Float32Array(4 * cols * rows).fill(-1), live: number[][] = [], missing: Slot[] = [];
    const px = (v: number, at: number, n: number) => Math.min(Math.max(Math.round((v * side - at) / m.mpp), 0), n);
    for (const s of slots) {
      const k = this.kept.get(s.id), i = (s.y - y0) * cols + (s.x - x0);
      if (k?.key === key) { k.seen = this.frame; data[4 * i] = k.layer; continue; }
      missing.push(s);
      const l = px(s.x, m.at[0], m.rect[2]), r = px(s.x + 1, m.at[0], m.rect[2]), t = px(s.y, m.at[1], m.rect[3]), b = px(s.y + 1, m.at[1], m.rect[3]);
      if (r > l && b > t) live.push([m.rect[0] + l, m.rect[1] + t, r - l, b - t]);
    }
    if (this.page?.w !== cols || this.page.h !== rows) {
      if (this.page) gpu.drop(this.page);
      this.page = gpu.texture(cols, rows, 'rgba32float');
    }
    gpu.write(this.page, data);
    if (live.length) marks.prepare(gpu, m, waiting);
    this.wanted = { m, e, missing, most: Math.max(Math.floor((DRAWN_A_FRAME * w * h) / (SIDE * SIDE)), 1) };
    const u = new Uniforms(FOREST_VEC4S).vec4(m.mpp, side, cols, rows).vec4(x0 * side - m.at[0], y0 * side - m.at[1], m.rect[0], m.rect[1]);
    const binds: Draw['binds'] = { 1: u.buffer, 2: this.page, 3: this.store!, 4: 'smooth' };
    return { tiles: { binds, live }, due: missing.length && !this.stuck ? Math.max(STEADY_MS - (now - this.since), 0) : -1 };
  }

  /** Draw between frames the next of the last frame's tiles not drawn (`draw`), `DRAWN_A_FRAME` of a screen at a time,
   * once their key has held `STEADY_MS`: when to draw more (ms from now), or -1 once none is left to draw (the map is
   * then drawn anew from them). */
  ahead(gpu: Gpu, marks: CoverMarks): number {
    const w = this.wanted;
    if (!w || !w.missing.length || this.stuck) return -1;
    const wait = STEADY_MS - (performance.now() - this.since);
    if (wait > 0) return wait;
    let drawn = 0;
    while (w.missing.length && drawn < w.most) {
      if (!this.drawTile(gpu, marks, w.m, w.e, w.missing.shift()!, this.key)) { this.stuck = true; return -1; }
      drawn++;
    }
    return w.missing.length ? 0 : -1;
  }

  /** Draw in pass `p` the cover of the frame `f` (`draw`). */
  show(p: Pass, marks: CoverMarks, rect: number[], f: FromTiles): void {
    p.draw('forest', { rect, binds: f.binds }, true);
    marks.draw(p, f.live);
  }

  /** Forget every tile (a lost context empties them). */
  lost(): void {
    for (const k of this.kept.values()) this.free.push(k.layer);
    this.kept.clear();
  }

  /** The texture of the tiles, as many layers as a screen of `w` x `h` px asks (`KEPT_SPARE`), made anew when it asks
   * more. */
  private ensure(gpu: Gpu, w: number, h: number): void {
    const across = (n: number) => Math.ceil((n * BAND) / TILE) + 1;
    const layers = Math.max(Math.ceil(across(w) * across(h) * KEPT_SPARE), LEAST_LAYERS);
    if (this.store && this.store.layers >= layers) return;
    if (this.store) gpu.drop(this.store);
    this.store = gpu.texture(SIDE, SIDE, TILES_FORMAT, layers, 1, true);
    this.kept.clear();
    this.free = Array.from({ length: layers }, (_, k) => layers - 1 - k);
  }

  /** Draw tile `s` of the band of `e` m a texel into a free layer (or the one seen least lately, not this frame): the
   * cover program over its rectangle, the marks about it decided for it. False where the marks cannot be decided yet
   * (their programs or models) or no layer is free. */
  private drawTile(gpu: Gpu, marks: CoverMarks, m: Marks, e: number, s: Slot, key: string): boolean {
    const was = this.kept.get(s.id);
    let layer = was?.layer ?? this.free.pop();
    if (layer === undefined) {
      let oldest: [string, Kept] | null = null;
      for (const entry of this.kept) if (entry[1].seen < this.frame && (!oldest || entry[1].seen < oldest[1].seen)) oldest = entry;
      if (!oldest) return false;
      this.kept.delete(oldest[0]);
      layer = oldest[1].layer;
    }
    const side = TILE * e, rect = [0, 0, SIDE, SIDE];
    marks.prepare(gpu, { rect, mpp: e, seenMpp: e, at: [s.x * side - GUTTER * e, s.y * side - GUTTER * e], land: m.land }, []);
    if (!marks.ready) { if (!was) this.free.push(layer); return false; }
    const p = gpu.pass(this.store, [0, 0, 0, 0], undefined, layer);
    marks.draw(p, [rect]);
    p.end();
    this.kept.set(s.id, { layer, key, seen: this.frame });
    return true;
  }
}
