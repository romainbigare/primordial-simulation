// The ground's tiles on the GPU (tape/src/ground): made once in workers of their own (tiles-worker.ts) and kept, so the
// world never waits for a picture. The view asks for the tiles about it at its depth, those of the depth above and the
// coarsest, coarse before fine and nearest its middle first; a request that left the view is dropped; the tiles drawn
// least lately go first once `MOST_TILES` are kept. A page of the tiles of the depth the view is drawn at (`pages`)
// names, for each, the finest kept tile that holds it and the one before it, so a tile not made yet is drawn from its
// nearest kept ancestor (stretched, softer), and a new one fades in over that one. The view is drawn at a new depth
// only once its tiles in view at it came (`settle`): till then at the last, so a zoom never shows a whole view of
// tiles stretched from a coarser depth, nor the ground of one depth and then of another. The coarsest tiles are kept always.
// The page keeps no copy of a tile: after a lost context the card's are empty, and it asks for them again (`lost`).
import { Gpu, Texture } from './gpu';
import { MOST_DRAWN_PX, type View } from './map';
import { spend, timed } from './cost';

/** The tiles of the ground as the core makes them (wasm/src/lib.rs `meta`): texels a side inside the gutter, texels of
 * the gutter along each edge, the depths from the coarsest to the finest, the most steps along each axis by which a
 * texel names its channel's sample, the length its place along its channel is kept modulo, m, numbers of a row of the
 * cells' table (of a square, of a kind, of its patches' places and of their dead walls), and the reflectance of the walls the cover builds. */
export interface TileMeta { size: number; gutter: number; depths: [number, number]; step_most: number; along_wrap: number; row: number; kind_row: number; place_row: number; dead_row: number; walls: [number, number, number] }
/** Whose ground the tiles are of: a new island's, made from its seed, size, band of latitudes (south, north) and
 * whether its large shape is drawn from the seed (wasm/src/ground.rs `ground`); or a saved world's, the gzip of its
 * snapshot (`ground_of_world`). */
export type GroundOf = { seed: number; size: number; band: [number, number]; drawn: boolean } | { saved: ArrayBuffer };
/** The map the tiles are of: squares across and down, metres a square and the depth of a square. */
export interface MapOf { w: number; h: number; cell_m: number; square_depth: number }

/** Tiles kept on the GPU at most: a view touches at most `VIEW_TILES` at its depth and a quarter as many at the one
 * above, and the coarsest; the rest is room for a pan. Each takes 1.1 MB on the card (16 bytes a texel), so 56 take
 * 61 MB. */
const MOST_TILES = 56;
/** The most tiles the view touches at its depth: past them it is drawn a depth coarser. */
const VIEW_TILES = 30;
/** Of a tile, 16-bit floats a texel: the first plane four (the height as two, the wetness index, the signed distance to
 * its channel's line), the second four (that channel's sample, as its step from the texel's own, and one more than the
 * pond the texel lies in; 0: none; how far along its channel's line the texel lies, and its distance to that channel's line without its bends). */
const PLANE_A = 4, PLANE_B = 4;
/** A new tile fades in over the one before it over this long, ms. */
export const TILE_FADE_MS = 250;
/** The view's tiles are asked for this share of its width and height past each of its edges, so a pan finds them. */
const AHEAD = 0.25;
/** What a tile out of the view adds to its distance from the view's middle (squares): it comes after those in it. */
const OUT_OF_VIEW = 1e9;
/** The view is drawn at the depth whose texels take no more than this many of the pixels the map is drawn at, so its ground and
 * its shores are read between texels no larger than a pixel; where that asks for more than `VIEW_TILES` tiles, a depth
 * coarser (each halving of a texel costs four times the tiles). */
const TEXEL_PX = 1;
/** Tiles asked of each worker at once: it makes the next while the page takes the last. */
const MAKING = 2;
/** Cores the page, the world's worker, a tiles' worker and the drawing keep busy. */
const SPARE_CORES = 4;
/** The tiles' workers: two where the browser has cores to spare beside the world's worker and the page (it says how
 * many it has), else one. Each holds its own copy of the core and of the terrain's samples. */
const WORKERS = (navigator.hardwareConcurrency ?? 1) > SPARE_CORES ? 2 : 1;
/** While the world plays, the ground's workers make the ground again at most this often, ms: a world at full speed
 * builds its samples every few seconds, each making of the ground takes a worker seconds and every kept tile is made
 * again after it, so a worker would do nothing else, on a core the world's own worker needs; ten years' load moves
 * the ground too little to see between two. */
const REBUILD_EVERY_MS = 30000;
/** A tile's key: its depth, row and column, as one number. */
const KEY_ROW = 1 << 12, KEY_DEPTH = KEY_ROW * KEY_ROW;
const keyOf = (depth: number, x: number, y: number) => depth * KEY_DEPTH + y * KEY_ROW + x;

/** A tile kept: its depth, column and row, its layer of the atlas, when it came (ms since the tiles started), the
 * number of the last view that asked for it, and the ground's version (`gen`) and the version of the world's samples
 * it is made of. */
interface Kept { depth: number; x: number; y: number; layer: number; came: number; asked: number; gen: number; version: number }
/** The relief the world built its samples from, and the version of those samples (wasm/src/lib.rs `relief`,
 * `ground_version`). */
export interface Built { relief: Uint8Array; version: number }
/** What edits moved the ground since the samples of a version (wasm/src/edit.rs `preview`): the window of the terrain's
 * samples that moved (west column, north row, width, height), that version (`over`), and of its samples row by row how
 * far each moved, m, and the depth of its loose ground once the samples are built again, m. */
export interface Preview { window: [number, number, number, number]; over: number; dh: Float32Array; loose: Float32Array }

export class GroundTiles {
  private workers: { worker: Worker; ready: boolean; making: number; memory: number }[] = [];
  private url: string;
  private kept = new Map<number, Kept>();
  /** The tiles the view asks for and has not, in order; those being made. */
  private queue: { depth: number; x: number; y: number }[] = [];
  private making = new Set<number>();
  /** The views asked so far, and the keys of the last one's tiles in view at its own depth (`wanted`). */
  private views = 0;
  private inView = new Set<number>();
  /** The depth the view is drawn at, and the depth the last view asked for, which it is drawn at once its tiles in
   * view came (`settle`). */
  depth: number;
  private wanted: number;
  private free: number[] = Array.from({ length: MOST_TILES }, (_, k) => MOST_TILES - 1 - k);
  /** Tiles come, not yet on the GPU; the page and the tiles' numbers to send to it. */
  private come: { layer: number; data: Uint16Array }[] = [];
  private page = { w: 1, h: 1, data: new Float32Array(0) };
  /** Of each layer: its tile's depth, column, row and when it came; then whether it is made of the samples the preview
   * of edits is drawn over (`previewOver`). */
  private numbers = new Float32Array(2 * 4 * MOST_TILES);
  private pageDue = true;
  private numbersDue = true;
  private atlas: Texture | null = null;
  private areas: Texture | null = null;
  private pages: Texture | null = null;
  /** The last view asked for, to ask again after a lost context. */
  private last: View | null = null;
  private tilesTex: Texture | null = null;
  /** The rock of each of the terrain's samples (wasm/src/ground.rs `rock`), four 16-bit floats a sample in four planes:
   * the depth of its loose ground and the largest stone its channel moves, m, the slope of its reach and its channel's
   * links; the reflectance of its bed's make; its loose ground's grain; its pores and the deposit on it
   * (tape/src/ground/rock.rs); none until a worker made the ground. */
  rock: Uint16Array | null = null;
  /** How many times the rock came (the land sends it to the GPU when this moves). */
  rockVersion = 0;
  /** The ground's version: how many times the world's samples were built anew and handed to the workers (`rebuild`); a
   * tile made of an older one is stale. The last version whose changed tiles the first worker told (`rebuilt`): till it
   * is the latest, a tile kept may be stale and not yet known to be. The tiles kept that a build changed, drawn until
   * each comes again. */
  private gen = 0;
  private told = 0;
  private stale = new Set<number>();
  /** The workers still making the ground of the last build handed to them: the next waits till none is
   * (`madeAgain`), so a worker never makes a build a newer one would make whole again; and whether that one is to be
   * handed at once. */
  private building = 0;
  private urgent = false;
  /** The relief of the latest build not yet handed to the workers (`rebuild`), when the last was handed, ms, and the
   * timer of the next. */
  private waiting: Built | null = null;
  /** The version of the samples of each ground version handed to the workers (`gen`), and the version the preview of
   * edits is drawn over (`previewOver`). */
  private versionOf = new Map<number, number>();
  private previewed: number;
  private rebuiltAt = -Infinity;
  private rebuildTimer = 0;
  private start = performance.now();
  private lastCame = -Infinity;
  /** Whether a new tile fades in (else it shows at once): the map says so, and lets none fade while its card is too
   * slow to draw a frame at each step of a fade (map.ts `drawGpu`). */
  fades = true;
  /** Whether the tiles that came wait to be sent to the GPU, with the page and the tiles' numbers that name them (the
   * map holds them while the view moves, map.ts `hold`): what the GPU holds stays as it was, whole. */
  held = false;
  /** How many times something was sent to the GPU (`textures`): what is drawn from the tiles changed. */
  written = 0;

  /** `version`: the version of the world's samples `ground` is made of. */
  constructor(source: string, wasm: ArrayBuffer, ground: GroundOf, readonly map: MapOf, readonly meta: TileMeta, version: number, private changed: (shown: boolean) => void, error: (text: string) => void) {
    this.depth = this.wanted = meta.depths[0];
    this.versionOf.set(0, version);
    this.previewed = version;
    this.makePage();
    this.url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
    for (let k = 0; k < WORKERS; k++) {
      const w = { worker: new Worker(this.url), ready: false, making: 0, memory: 0 };
      w.worker.onerror = e => error(e.message || 'The worker of the ground\'s tiles failed.');
      w.worker.onmessage = e => {
        const m = e.data as { kind: string; depth: number; x: number; y: number; data: Uint16Array; rock: Uint16Array; text: string; gen: number; squares: Uint8Array; ms: number; memory: number };
        if (m.kind === 'error') return error(m.text);
        if (m.kind === 'ready') { w.ready = true; if (!this.rock) { this.rock = m.rock; this.rockVersion++; } this.changed(true); return this.next(); }
        if (m.kind === 'tile') { w.making--; w.memory = m.memory; spend('tile ms', m.ms); timed('tiles', () => this.took(m.depth, m.x, m.y, m.data, m.gen)); }
        if (m.kind === 'rebuilt') this.madeAgain(k === 0 ? m : null, m.gen);
      };
      const copy = k === WORKERS - 1 ? wasm : wasm.slice(0);
      w.worker.postMessage({ cmd: 'init', wasm: copy, ground }, [copy]);
      this.workers.push(w);
    }
  }

  /** The world built its samples anew from `relief` (wasm/src/lib.rs `relief`; null: no new build): each worker makes
   * its ground again (`ground_built`), and the tiles asked after are of it. While the world plays (`now` false) at
   * most once every `REBUILD_EVERY_MS`, of the latest build come; a paused world's ground at once. */
  rebuild(relief: Built | null, now: boolean): void {
    this.waiting = relief ?? this.waiting;
    this.urgent = now;
    clearTimeout(this.rebuildTimer);
    if (!this.waiting || this.building > 0) return;
    const wait = now ? 0 : this.rebuiltAt + REBUILD_EVERY_MS - performance.now();
    if (wait > 0) { this.rebuildTimer = window.setTimeout(() => this.rebuild(null, true), wait); return; }
    const built = this.waiting;
    this.waiting = null;
    this.rebuiltAt = performance.now();
    this.gen++;
    this.building = this.workers.length;
    this.versionOf.set(this.gen, built.version);
    const own = built.relief.byteOffset === 0 && built.relief.byteLength === built.relief.buffer.byteLength;
    this.workers.forEach((w, k) => {
      // (The last worker takes the relief itself where it is a buffer of its own: no copy.)
      const copy = own && k === this.workers.length - 1 ? built.relief : built.relief.slice();
      // (Only the first sends back the rock and the squares changed.)
      w.worker.postMessage({ cmd: 'built', relief: copy.buffer, gen: this.gen, answer: k === 0 }, [copy.buffer]);
    });
  }

  /** A worker made the ground of the ground's version `gen` again: the build waiting is handed once all did. The
   * first's answer (`first`) brings the rock of it, and each tile kept of an older version over a square whose tiles
   * changed (`squares`, a byte a square row by row) is stale, asked for again and drawn until it comes. */
  private madeAgain(first: { squares: Uint8Array; rock: Uint16Array } | null, gen: number): void {
    this.building--;
    if (first) this.rebuilt(first.squares, first.rock, gen);
    if (this.building === 0 && this.waiting) this.rebuild(null, this.urgent);
  }

  private rebuilt(squares: Uint8Array, rock: Uint16Array, gen: number): void {
    this.told = gen;
    this.rock = rock;
    this.rockVersion++;
    const version = this.versionOf.get(gen) ?? this.previewed;
    for (const [key, k] of this.kept) {
      if (k.gen >= gen || this.stale.has(key)) continue;
      const s = this.squaresOf(k.depth), x0 = Math.floor(k.x * s), y0 = Math.floor(k.y * s);
      const x1 = Math.min(Math.ceil((k.x + 1) * s), this.map.w), y1 = Math.min(Math.ceil((k.y + 1) * s), this.map.h);
      let changed = false;
      for (let y = y0; y < y1 && !changed; y++) for (let x = x0; x < x1 && !changed; x++) changed = squares[y * this.map.w + x] === 1;
      if (changed) { this.stale.add(key); continue; }
      // A tile over no square the build changed holds the ground of this build too: it is of its version, so the
      // preview of the next edits (of what moved since the version shown) is drawn over it.
      k.gen = gen;
      k.version = version;
      this.numbers[4 * (MOST_TILES + k.layer)] = +(version === this.previewed);
      this.numbersDue = true;
    }
    this.changed(true);
    if (this.last) this.view(this.last);
  }

  /** Squares a tile of `depth` spans. */
  private squaresOf(depth: number): number { return this.meta.size / 2 ** (depth - this.map.square_depth); }
  /** Tiles of `depth` across and down the map. */
  private across(depth: number): [number, number] { const s = this.squaresOf(depth); return [Math.ceil(this.map.w / s), Math.ceil(this.map.h / s)]; }

  /** The tiles of view `v`: drawn at the depth whose texels take no more than `TEXEL_PX` of the screen's pixels (no
   * more than `VIEW_TILES` of them), its tiles there, at the depth above and at the coarsest, coarse first; then those
   * in view of the depth below, nearest its middle first, as many as the tiles kept have room for, so a zoom in finds
   * them made. */
  view(v: View): void {
    this.last = v;
    const [d0, d1] = this.meta.depths, sq = this.map.square_depth;
    const tiles = (d: number) => { const s = this.squaresOf(d); return (Math.ceil(v.w / s) + 1) * (Math.ceil(v.h / s) + 1); };
    // (By the pixels the map is drawn at, map.ts `MOST_DRAWN_PX`, not the screen's: a finer tile shows nothing more.)
    let depth = Math.min(Math.max(sq + Math.ceil(Math.log2(Math.max((v.s * Math.min(devicePixelRatio, MOST_DRAWN_PX)) / TEXEL_PX, 1))), d0), d1);
    while (depth > d0 && tiles(depth) > VIEW_TILES) depth--;
    const cx = v.x + v.w / 2, cy = v.y + v.h / 2, asked = ++this.views, wanted: { depth: number; x: number; y: number; stale: boolean; far: number }[] = [];
    this.inView.clear();
    // (The tiles this view asks for, kept or not: the depth below takes the room the others leave.)
    let asks = 0;
    for (const d of new Set([d0, Math.max(depth - 1, d0), depth])) {
      const s = this.squaresOf(d), [nx, ny] = this.across(d), ahead = d === depth ? AHEAD : 0;
      const range = (from: number, span: number, n: number) => [Math.max(0, Math.floor((from - ahead * span) / s)), Math.min(n - 1, Math.floor((from + span * (1 + ahead)) / s))];
      const [x0, x1] = range(v.x, v.w, nx), [y0, y1] = range(v.y, v.h, ny);
      for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
        const inside = (x + 1) * s > v.x && x * s < v.x + v.w && (y + 1) * s > v.y && y * s < v.y + v.h;
        if (d === depth && inside) this.inView.add(keyOf(d, x, y));
        const key = keyOf(d, x, y), k = this.kept.get(key);
        asks++;
        if (k) k.asked = asked;
        if (!k || this.stale.has(key)) wanted.push({ depth: d, x, y, stale: k !== undefined, far: (inside ? 0 : OUT_OF_VIEW) + Math.hypot((x + 0.5) * s - cx, (y + 0.5) * s - cy) });
        // A tile in view not kept is drawn from its nearest kept ancestor till it comes: that one is asked too, so it
        // is not let go first (the view would fall back to a coarser tile, then come back).
        if (!k && inside) this.keepAncestor(d, x, y, asked);
      }
    }
    if (depth < d1) {
      const d = depth + 1, s = this.squaresOf(d), [nx, ny] = this.across(d), below: { x: number; y: number; far: number }[] = [];
      const cover = (from: number, span: number, n: number) => [Math.max(0, Math.floor(from / s)), Math.min(n - 1, Math.floor((from + span) / s))];
      const [x0, x1] = cover(v.x, v.w, nx), [y0, y1] = cover(v.y, v.h, ny);
      for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) below.push({ x, y, far: Math.hypot((x + 0.5) * s - cx, (y + 0.5) * s - cy) });
      for (const { x, y, far } of below.sort((a, b) => a.far - b.far).slice(0, Math.max(0, MOST_TILES - asks))) {
        const key = keyOf(d, x, y), k = this.kept.get(key);
        if (k) k.asked = asked;
        if (!k || this.stale.has(key)) wanted.push({ depth: d, x, y, stale: k !== undefined, far });
      }
    }
    // The tiles in view at its depth come first (where an edit just changed the ground, its tiles there come before
    // those about it); then tiles not kept before those kept stale (which are drawn meanwhile), so a zoom's tiles never
    // wait behind the ground a build changed; then coarse before fine, nearest the view's middle first.
    const shown = (q: { depth: number; far: number }) => +!(q.depth === depth && q.far < OUT_OF_VIEW);
    this.queue = wanted.sort((a, b) => shown(a) - shown(b) || +a.stale - +b.stale || a.depth - b.depth || a.far - b.far);
    this.wanted = depth;
    this.settle();
    this.next();
  }

  /** Draw the view at the depth it asked for (`wanted`) once its tiles in view there came: a page made anew. */
  private settle(): void {
    if (this.wanted === this.depth || [...this.inView].some(k => !this.kept.has(k))) return;
    this.depth = this.wanted;
    this.makePage();
  }

  /** Mark the nearest kept ancestor of the tile of `depth` at column `x` and row `y` as asked by view `asked`. */
  private keepAncestor(depth: number, x: number, y: number, asked: number): void {
    for (let d = depth - 1; d >= this.meta.depths[0]; d--) {
      x >>= 1; y >>= 1;
      const k = this.kept.get(keyOf(d, x, y));
      if (k) { k.asked = asked; return; }
    }
  }

  /** The version of the world's samples every tile in view at its own depth is made of, once all came; null while
   * some are still to come, a build is still to be handed to the workers, or the tiles the last changed are not yet
   * told. */
  shownVersion(): number | null {
    if (this.waiting || this.told !== this.gen) return null;
    for (const k of this.inView) if (!this.kept.has(k) || this.stale.has(k)) return null;
    return this.versionOf.get(this.gen) ?? null;
  }

  /** The preview of edits (land.ts `preview`) is of what moved since the samples of `version` (NaN: nothing moved): it
   * is drawn over the tiles made of them alone (a tile made since holds what moved). */
  previewOver(version: number): void {
    this.previewed = version;
    for (const k of this.kept.values()) this.numbers[4 * (MOST_TILES + k.layer)] = +(k.version === version);
    this.numbersDue = true;
  }

  /** The tiles in view at its own depth still to come. */
  toCome(): number { let n = 0; for (const k of this.inView) if (!this.kept.has(k)) n++; return n; }
  /** The tiles in view at its own depth, come or not. */
  inViewCount(): number { return this.inView.size; }
  /** The tiles kept. */
  count(): number { return this.kept.size; }
  /** The bytes of each worker's core's memory, as it last said. */
  memory(): number[] { return this.workers.map(w => w.memory); }
  /** Whether a tile still fades in at `now` (performance.now, ms). */
  fading(now: number): boolean { return this.fades && now - this.start - this.lastCame < TILE_FADE_MS; }

  /** Ask the workers for the next tiles of the queue, up to `MAKING` each at once (the least busy first): a tile kept
   * stale comes again on its own layer; another, while a layer is free or can be freed. */
  private next(): void {
    const known = (q: { depth: number; x: number; y: number }) => { const key = keyOf(q.depth, q.x, q.y); return (this.kept.has(key) && !this.stale.has(key)) || this.making.has(key); };
    for (;;) {
      const w = this.workers.filter(v => v.ready && v.making < MAKING).sort((a, b) => a.making - b.making)[0];
      if (!w) return;
      this.queue = this.queue.filter(q => !known(q));
      const newMaking = [...this.making].filter(key => !this.kept.has(key)).length;
      const room = this.free.length > newMaking || this.oldest() !== null;
      const k = this.queue.findIndex(q => room || this.kept.has(keyOf(q.depth, q.x, q.y)));
      if (k < 0) return;
      const [t] = this.queue.splice(k, 1);
      this.making.add(keyOf(t.depth, t.x, t.y));
      w.making++;
      w.worker.postMessage({ cmd: 'tile', depth: t.depth, x: t.x, y: t.y, gen: this.gen });
    }
  }

  /** The tile the last views asked for least lately that the last did not ask for (none of the coarsest). */
  private oldest(): Kept | null {
    let best: Kept | null = null;
    for (const k of this.kept.values()) if (k.asked < this.views && k.depth > this.meta.depths[0] && (!best || k.asked < best.asked)) best = k;
    return best;
  }

  /** After a lost context the card's tiles are empty: forget them and ask for the view's again. */
  lost(): void {
    this.kept.clear();
    this.stale.clear();
    this.come = [];
    this.free = Array.from({ length: MOST_TILES }, (_, k) => MOST_TILES - 1 - k);
    this.numbers.fill(0);
    this.numbersDue = true;
    this.makePage();
    if (this.last) this.view(this.last);
  }

  /** Keep the tile that came (made of the ground's version `gen`), on its own layer where it is kept stale, else on a
   * free layer or on the oldest's. Only a tile that changes what the view shows draws the map anew and fades in: one
   * ahead of the view, or under finer tiles kept, is kept for later. */
  private took(depth: number, x: number, y: number, data: Uint16Array, gen: number): void {
    const key = keyOf(depth, x, y), had = this.kept.get(key);
    this.making.delete(key);
    if (gen === this.gen) this.stale.delete(key);
    else this.stale.add(key);
    let layer = had ? had.layer : this.free.pop();
    if (layer === undefined) {
      const old = this.oldest();
      if (old) { this.kept.delete(keyOf(old.depth, old.x, old.y)); this.stale.delete(keyOf(old.depth, old.x, old.y)); layer = old.layer; }
    }
    if (layer !== undefined) {
      const came = performance.now() - this.start;
      const version = this.versionOf.get(gen) ?? this.previewed;
      this.kept.set(keyOf(depth, x, y), { depth, x, y, layer, came: had ? had.came : came, asked: this.views, gen, version });
      this.come.push({ layer, data });
      this.numbers.set([depth, x, y, had ? had.came : came], 4 * layer);
      this.numbers[4 * (MOST_TILES + layer)] = +(version === this.previewed);
      this.numbersDue = true;
      const before = this.page;
      this.makePage();
      this.settle();
      // A tile that takes a stale one's place shows at once, with no fade (the ground under it barely moved), so a
      // playing world's builds do not keep the map drawing frame after frame.
      const shown = this.showsOtherThan(before) || (had !== undefined && this.seen(depth, x, y));
      if (shown && !had) this.lastCame = came;
      this.changed(shown);
    }
    // A tile of an older build (made by another worker while the first told what the new build changed) is stale: the
    // view asks for it again, else nothing would till the view moves.
    if (gen !== this.gen && this.last) this.view(this.last);
    else this.next();
  }

  /** Whether the tile of `depth` at column `x` and row `y` lies in the last view. */
  private seen(depth: number, x: number, y: number): boolean {
    const v = this.last, s = this.squaresOf(depth);
    return v !== null && (x + 1) * s > v.x && x * s < v.x + v.w && (y + 1) * s > v.y && y * s < v.y + v.h;
  }

  /** Whether the page shows another layer than page `before` anywhere in the last view. */
  private showsOtherThan(before: { w: number; h: number; data: Float32Array }): boolean {
    const v = this.last, { w, h, data } = this.page;
    if (!v || before.w !== w || before.h !== h) return true;
    const s = this.squaresOf(this.depth), clip = (c: number, n: number) => Math.min(Math.max(Math.floor(c / s), 0), n - 1);
    for (let y = clip(v.y, h); y <= clip(v.y + v.h, h); y++) {
      for (let x = clip(v.x, w); x <= clip(v.x + v.w, w); x++) if (data[4 * (y * w + x)] !== before.data[4 * (y * w + x)]) return true;
    }
    return false;
  }

  /** The page at the view's depth, made anew as the tiles kept or that depth change (sent to the GPU at the next
   * draw): for each of its tiles, the layer of the finest kept tile that holds it and of the one kept before that (its
   * nearest kept ancestor), -1 for none; found depth by depth from the coarsest. */
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
    this.pageDue = true;
  }

  /** The textures the land program reads (land.wgsl): the tiles' two planes, the page and each layer's tile (depth,
   * column, row, when it came), sent to `gpu` as they change; and the time since the tiles started, ms. */
  textures(gpu: Gpu): { atlas: Texture; areas: Texture; pages: Texture; tiles: Texture; now: number } {
    const side = this.meta.size + 2 * this.meta.gutter, n = side * side;
    this.atlas ??= gpu.texture('tiles', side, side, 'rgba16float', MOST_TILES);
    this.areas ??= gpu.texture('tiles', side, side, 'rgba16float', MOST_TILES);
    this.tilesTex ??= gpu.texture('tiles', MOST_TILES, 2, 'rgba32float');
    const now = performance.now() - this.start;
    if (this.held && this.pages) return { atlas: this.atlas, areas: this.areas, pages: this.pages, tiles: this.tilesTex, now };
    if (this.come.length) this.written++;
    for (const c of this.come) {
      gpu.write(this.atlas, c.data.subarray(0, PLANE_A * n), 0, c.layer);
      gpu.write(this.areas, c.data.subarray(PLANE_A * n, (PLANE_A + PLANE_B) * n), 0, c.layer);
    }
    this.come = [];
    if (this.pageDue) {
      if (this.pages?.w !== this.page.w || this.pages.h !== this.page.h) { if (this.pages) gpu.drop(this.pages); this.pages = gpu.texture('tiles', this.page.w, this.page.h, 'rgba32float'); }
      gpu.write(this.pages, this.page.data);
      this.pageDue = false;
      this.written++;
    }
    if (!this.fades) this.faded(now);
    if (this.numbersDue) { gpu.write(this.tilesTex, this.numbers); this.numbersDue = false; this.written++; }
    return { atlas: this.atlas, areas: this.areas, pages: this.pages!, tiles: this.tilesTex, now };
  }

  /** End at `now` (ms since the tiles started) the fade of every tile still fading in: a frame drawn with no fade shows
   * each whole, and a later frame that fades (the map drawn at the screen's pixels again) must not show the tile it
   * replaced once more. */
  private faded(now: number): void {
    for (const k of this.kept.values()) {
      if (now - k.came >= TILE_FADE_MS) continue;
      k.came = now - TILE_FADE_MS;
      this.numbers[4 * k.layer + 3] = k.came;
      this.numbersDue = true;
    }
    this.lastCame = Math.min(this.lastCame, now - TILE_FADE_MS);
  }

  stop(gpu: Gpu | null): void {
    clearTimeout(this.rebuildTimer);
    this.workers.forEach(w => w.worker.terminate());
    URL.revokeObjectURL(this.url);
    if (gpu) for (const t of [this.atlas, this.areas, this.pages, this.tilesTex]) if (t) gpu.drop(t);
  }
}
