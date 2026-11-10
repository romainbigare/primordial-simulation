// The ground's tiles on the GPU (tape/src/ground): made once in a second worker (tiles-worker.ts) and kept, so the
// world never waits for a picture. The view asks for the tiles about it, coarse before fine and nearest its middle
// first; a request that left the view is dropped; the tiles drawn least lately go first once `MOST_TILES` are kept. A
// page of the tiles of the depth the view is drawn at (`pages`) names, for each, the finest kept tile that holds it and
// the one before it, so a tile not made yet is drawn from its nearest ancestor (stretched, softer), and a new one fades
// in over that one. The coarsest tiles are kept always.
import { Gpu, Texture } from './gpu';
import type { View } from './map';

/** The tiles of the ground as the core makes them (wasm/src/lib.rs `meta`): texels a side inside the gutter, texels of
 * the gutter along each edge, numbers a texel, the depths from the coarsest to the finest, and numbers of a row of the
 * cells' table. */
export interface TileMeta { size: number; gutter: number; texel: number; depths: [number, number]; row: number }
/** The map the tiles are of: squares across and down, metres a square and the depth of a square. */
export interface MapOf { w: number; h: number; cell_m: number; square_depth: number }

/** Tiles kept on the GPU at most: those of a view at its depth and its ancestors, with room for a pan. */
const MOST_TILES = 96;
/** A new tile fades in over the one before it over this long, ms. */
export const TILE_FADE_MS = 250;
/** The view's tiles are asked for this share of its width and height past each of its edges, so a pan finds them. */
const AHEAD = 0.25;
/** A tile's key: its depth, row and column, as one number. */
const KEY_ROW = 1 << 12, KEY_DEPTH = KEY_ROW * KEY_ROW;
const keyOf = (depth: number, x: number, y: number) => depth * KEY_DEPTH + y * KEY_ROW + x;

/** A tile kept: its depth, column and row, its layer of the atlas, when it came (ms since the tiles started) and the
 * number of the last view that asked for it. */
interface Kept { depth: number; x: number; y: number; layer: number; came: number; asked: number }

export class GroundTiles {
  private worker: Worker;
  private url: string;
  private ready = false;
  private kept = new Map<number, Kept>();
  /** The tiles the view asks for and has not, in order; the one being made. */
  private queue: { depth: number; x: number; y: number }[] = [];
  private making: number | null = null;
  /** The views asked so far, and the keys of the last one's tiles in view at its own depth. */
  private views = 0;
  private inView = new Set<number>();
  /** The depth the view is drawn at. */
  depth: number;
  private free: number[] = Array.from({ length: MOST_TILES }, (_, k) => MOST_TILES - 1 - k);
  /** Tiles come, not yet on the GPU; the page and the tiles' numbers to send to it. */
  private come: { layer: number; data: Float32Array }[] = [];
  private page = { w: 1, h: 1, data: new Float32Array(4) };
  private numbers = new Float32Array(4 * MOST_TILES);
  private pageDue = true;
  private numbersDue = true;
  private atlas: Texture | null = null;
  private pages: Texture | null = null;
  private tilesTex: Texture | null = null;
  private start = performance.now();
  private lastCame = -Infinity;

  constructor(source: string, wasm: ArrayBuffer, seed: number, size: number, readonly map: MapOf, readonly meta: TileMeta, private changed: () => void, error: (text: string) => void) {
    this.depth = meta.depths[0];
    this.url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
    this.worker = new Worker(this.url);
    this.worker.onerror = e => error(e.message);
    this.worker.onmessage = e => {
      const m = e.data as { kind: string; depth: number; x: number; y: number; data: Float32Array; text: string };
      if (m.kind === 'error') return error(m.text);
      if (m.kind === 'ready') { this.ready = true; return this.next(); }
      if (m.kind === 'tile') this.took(m.depth, m.x, m.y, m.data);
    };
    this.worker.postMessage({ cmd: 'init', wasm, seed, size }, [wasm]);
  }

  /** Squares a tile of `depth` spans. */
  private squaresOf(depth: number): number { return this.meta.size / 2 ** (depth - this.map.square_depth); }
  /** Tiles of `depth` across and down the map. */
  private across(depth: number): [number, number] { const s = this.squaresOf(depth); return [Math.ceil(this.map.w / s), Math.ceil(this.map.h / s)]; }

  /** The tiles of view `v`: drawn at the depth whose texels take no more than a css pixel, all its ancestors' first. */
  view(v: View): void {
    const [d0, d1] = this.meta.depths, sq = this.map.square_depth;
    const depth = Math.min(Math.max(sq + Math.ceil(Math.log2(Math.max(v.s, 1))), d0), d1);
    const cx = v.x + v.w / 2, cy = v.y + v.h / 2, asked = ++this.views, wanted: { depth: number; x: number; y: number; far: number }[] = [];
    this.inView.clear();
    for (let d = d0; d <= depth; d++) {
      const s = this.squaresOf(d), [nx, ny] = this.across(d), ahead = d === depth ? AHEAD : 0;
      const range = (from: number, span: number, n: number) => [Math.max(0, Math.floor((from - ahead * span) / s)), Math.min(n - 1, Math.floor((from + span * (1 + ahead)) / s))];
      const [x0, x1] = range(v.x, v.w, nx), [y0, y1] = range(v.y, v.h, ny);
      for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
        const inside = (x + 1) * s > v.x && x * s < v.x + v.w && (y + 1) * s > v.y && y * s < v.y + v.h;
        if (d === depth && inside) this.inView.add(keyOf(d, x, y));
        const k = this.kept.get(keyOf(d, x, y));
        if (k) k.asked = asked;
        else wanted.push({ depth: d, x, y, far: (inside ? 0 : 1e9) + Math.hypot((x + 0.5) * s - cx, (y + 0.5) * s - cy) });
      }
    }
    this.queue = wanted.sort((a, b) => a.depth - b.depth || a.far - b.far);
    if (depth !== this.depth) { this.depth = depth; this.pageDue = true; }
    this.next();
  }

  /** The tiles in view at its own depth still to come. */
  toCome(): number { let n = 0; for (const k of this.inView) if (!this.kept.has(k)) n++; return n; }
  /** The tiles kept. */
  count(): number { return this.kept.size; }
  /** Whether a tile still fades in at `now` (performance.now, ms). */
  fading(now: number): boolean { return now - this.start - this.lastCame < TILE_FADE_MS; }

  /** Ask the worker for the next tile of the queue, if none is being made and a layer is free or can be freed. */
  private next(): void {
    if (!this.ready || this.making !== null) return;
    let t = this.queue.shift();
    while (t && this.kept.has(keyOf(t.depth, t.x, t.y))) t = this.queue.shift();
    if (!t || (!this.free.length && this.oldest() === null)) return;
    this.making = keyOf(t.depth, t.x, t.y);
    this.worker.postMessage({ cmd: 'tile', depth: t.depth, x: t.x, y: t.y });
  }

  /** The tile the last views asked for least lately that the last did not ask for (none of the coarsest). */
  private oldest(): Kept | null {
    let best: Kept | null = null;
    for (const k of this.kept.values()) if (k.asked < this.views && k.depth > this.meta.depths[0] && (!best || k.asked < best.asked)) best = k;
    return best;
  }

  /** Keep the tile that came, on a free layer or on the oldest's. */
  private took(depth: number, x: number, y: number, data: Float32Array): void {
    this.making = null;
    let layer = this.free.pop();
    if (layer === undefined) {
      const old = this.oldest();
      if (old) { this.kept.delete(keyOf(old.depth, old.x, old.y)); layer = old.layer; }
    }
    if (layer !== undefined) {
      const came = performance.now() - this.start;
      this.kept.set(keyOf(depth, x, y), { depth, x, y, layer, came, asked: this.views });
      this.come.push({ layer, data });
      this.numbers.set([depth, x, y, came], 4 * layer);
      this.lastCame = came;
      this.numbersDue = this.pageDue = true;
      this.changed();
    }
    this.next();
  }

  /** The page at the view's depth: for each of its tiles, the layer of the finest kept tile that holds it and of the
   * one kept before that (its nearest kept ancestor), -1 for none; found depth by depth from the coarsest. */
  private makePage(): void {
    const [d0] = this.meta.depths;
    let shown = new Float32Array(1).fill(-1), under = new Float32Array(1).fill(-1), w = 1;
    for (let d = d0; d <= this.depth; d++) {
      const [nx, ny] = this.across(d), a = new Float32Array(nx * ny), b = new Float32Array(nx * ny);
      for (let y = 0; y < ny; y++) for (let x = 0; x < nx; x++) {
        const up = d === d0 ? -1 : (y >> 1) * w + (x >> 1), k = this.kept.get(keyOf(d, x, y)), i = y * nx + x;
        const above = up < 0 ? -1 : shown[up];
        a[i] = k ? k.layer : above;
        b[i] = k ? above : up < 0 ? -1 : under[up];
      }
      shown = a; under = b; w = nx;
    }
    const [nx, ny] = this.across(this.depth), data = new Float32Array(4 * nx * ny);
    for (let i = 0; i < nx * ny; i++) { data[4 * i] = shown[i]; data[4 * i + 1] = under[i]; }
    this.page = { w: nx, h: ny, data };
  }

  /** The textures the land program reads (land.wgsl): the atlas of the tiles, the page and each layer's tile
   * (depth, column, row, when it came), sent to `gpu` as they change; and the time since the tiles started, ms. */
  textures(gpu: Gpu): { atlas: Texture; pages: Texture; tiles: Texture; now: number } {
    const side = this.meta.size + 2 * this.meta.gutter;
    this.atlas ??= gpu.texture(side, side, 'rgba32float', MOST_TILES);
    for (const c of this.come) gpu.write(this.atlas, c.data, 0, c.layer);
    this.come = [];
    if (this.pageDue) {
      this.makePage();
      if (this.pages?.w !== this.page.w || this.pages.h !== this.page.h) { if (this.pages) gpu.drop(this.pages); this.pages = gpu.texture(this.page.w, this.page.h, 'rgba32float'); }
      gpu.write(this.pages, this.page.data);
      this.pageDue = false;
    }
    this.tilesTex ??= gpu.texture(MOST_TILES, 1, 'rgba32float');
    if (this.numbersDue) { gpu.write(this.tilesTex, this.numbers); this.numbersDue = false; }
    return { atlas: this.atlas, pages: this.pages!, tiles: this.tilesTex, now: performance.now() - this.start };
  }

  stop(gpu: Gpu | null): void {
    this.worker.terminate();
    URL.revokeObjectURL(this.url);
    if (gpu) for (const t of [this.atlas, this.pages, this.tilesTex]) if (t) gpu.drop(t);
  }
}
