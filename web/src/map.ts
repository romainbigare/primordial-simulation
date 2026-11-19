// The map, drawn on the GPU (gpu.ts) at the screen's own pixels: under it the page's background and the map's shadow,
// then the live map from the ground's tiles and the cells' table (land.ts), or a tape's map of squares (fields.ts, a
// texel a square), then what others draw on the GPU (the plant cover, vegetation.ts).
// Over it a 2D canvas holds what is drawn by hand: a globe's lines of latitude, the squares under the pointer and
// picked, and the scale bar.
import { Draw, Format, Gpu, Made, Pass, Texture, Uniforms, madeFor, openGpu, seenFrom } from './gpu';
import { Fields } from './fields';
import { FieldWindow, MapLook, frameOf } from './field-window';
import { FLOW_FPS, Land } from './land';
import { detailLost, sendDetail } from './detail';

const LAT_STEP = 30; // degrees between a globe's lines of latitude
const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

/** The part of the map in view: its window in squares (clipped to the map), the pixels a square takes, and the zoom (1: the map fits). */
export interface View { x: number; y: number; w: number; h: number; s: number; zoom: number }
/** The most pixels a metre takes on screen. */
const MOST_PX_PER_M = 80;
/** The map's shadow on the page: it falls this far south and is blurred this much (a Gaussian's deviation), css px, and
 * darkens the background this much at most. */
const SHADOW_DROP = 5, SHADOW_BLUR = 11, SHADOW_DARK = 0.28;
/** The time a frame may take on the GPU, ms, before the map is drawn at fewer pixels; and the shares of the screen's
 * pixels along each axis it may be drawn at, largest first: few, so the picture does not pump, and none under a half. */
const FRAME_BUDGET_MS = 16, SCALES = [1, 0.75, 0.5];
/** The time the view rests before the map drawn at fewer pixels is drawn anew at the screen's own, ms, a tile a frame,
 * each of about `SHARPEN_MS` on the GPU (the view rests, so a frame may take longer) and no less than `TILE_LEAST` px a
 * side. */
const REST_MS = 150, SHARPEN_MS = 50, TILE_LEAST = 64;
/** How often a frame drawn without a program not made yet looks whether it is made, ms (`drawWhenMade`). */
const MADE_POLL_MS = 50;

/** Drawn on the GPU over the map by another module, in the map's pass, at every frame: what it works out before the
 * pass, where the map's corner is at (ox, oy) css px, a square takes `s` css px, a css px `d` px of the target and
 * `seen` px of the screen (its detail is chosen by the screen's), and the view is `w` x `h` css px, adding to `waiting`
 * the programs it goes without as they are not made yet (`prepare`); its draw in the pass; and what the live map's
 * ground draws of it, with a square taking `s` css px and a css px `seen` px of the screen (`under`, its bindings). */
export interface GpuLayer {
  prepare(gpu: Gpu, ox: number, oy: number, s: number, d: number, seen: number, w: number, h: number, waiting: Made[]): void;
  draw(p: Pass): void;
  /** Make its programs ahead (`Gpu.warm`) for draws into the map's targets (null: the canvas; `MAP_TARGET`). */
  warm(gpu: Gpu, targets: (Format | null)[]): void;
  under(gpu: Gpu, s: number, seen: number): Draw['binds'];
}

/** The format of the textures the map is drawn into below the screen's pixels and for its sharp picture. */
export const MAP_TARGET: Format = 'rgba8unorm';
/** Where the live map's programs are made ahead in order of their sizes of a pixel (`warmAhead`), the one made for
 * every size counts as made for this many metres. */
const LAND_FINEST_M = 0.25;

/** The sharp picture being drawn once the view rests: its tiles to draw (x, y, w, h device px), the copies' places
 * (css px) and the device pixels a css pixel. */
interface Sharpening { tiles: number[][]; xs: number[]; d: number }

export class MapView {
  hover = -1;
  pick = -1;
  /** The GPU once it is open (null: none yet, or the browser has none). */
  private gpu: Gpu | null = null;
  readonly ready: Promise<Gpu | null>;
  private over = document.createElement('canvas');
  private ctx: CanvasRenderingContext2D;
  private cam = { s: 1, ox: 0, oy: 0 };
  private fitted = true;
  private w = 0; private h = 0; private cellM = 1;
  /** Edge latitudes (degrees) of a globe, whose columns wrap east to west; null for an island. */
  private globe: { north: number; south: number } | null = null;
  private raf = 0;
  /** The map's squares (a tape's map, and the classes of the live map's squares), and the live map. */
  private map: FieldWindow | null = null;
  private land: Land | null = null;
  private look: MapLook = { mode: 'earth', layer: '', ramp: [] };
  /** The map's classes asked for: what waits for them, and the texture they are drawn into. */
  private asked: ((cls: Uint8Array) => void)[] = [];
  private classes: Texture | null = null;
  private seen = '';
  /** The share of the screen's pixels along each axis the map is drawn at, the time the last timed frame would have
   * taken the GPU at the screen's pixels, ms, whether a frame is being timed, the texture the map is drawn into below the screen's
   * pixels; and the timer of the sharp picture drawn once the view rests, the tiles of it left to draw (none: it is not
   * being drawn), and the texture it is drawn into. */
  private scale = 1;
  /** Told the share the map is drawn at whenever it changes. */
  onScale: () => void = () => {};
  private fullMs = 0;
  private timing = false;
  private picture: Texture | null = null;
  private sharpTimer = 0;
  /** Whether the GPU's picture is to be drawn anew at the next frame. */
  private gpuDue = false;
  /** The timer of the next frame of the flow's signs (`flowSoon`). */
  private flowTimer = 0;
  private sharpening: Sharpening | null = null;
  private sharp: Texture | null = null;
  /** Whether the programs the map may draw next are being made ahead (`warmAhead`). */
  private warmed = false;
  /** The timer that looks whether the programs the last frame went without are made (`drawWhenMade`). */
  private madeTimer = 0;
  /** Drawn by another module on the GPU over the map, and on the 2D canvas over it at each redraw: its context, where
   * the map's corner is and the pixels a square takes. */
  layer: GpuLayer | null = null;
  overlay: ((ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number) => void) | null = null;

  constructor(private cv: HTMLCanvasElement, private onHover: (i: number) => void, private onPick: (i: number) => void, private onView: (v: View) => void = () => {}) {
    this.over.className = 'over';
    cv.after(this.over);
    this.ctx = this.over.getContext('2d')!;
    this.ready = openGpu(cv, () => { this.land?.tiles.lost(); detailLost(); this.invalidate(); }).then(g => {
      this.gpu = g;
      if (g) sendDetail(g, () => this.invalidate());
      this.invalidate();
      return g;
    });
    new ResizeObserver(() => { this.resize(); this.invalidate(); }).observe(cv);
    const ptrs = new Map<number, { x: number; y: number }>();
    let moved = 0;
    cv.addEventListener('pointerdown', e => { cv.setPointerCapture(e.pointerId); ptrs.set(e.pointerId, { x: e.offsetX, y: e.offsetY }); if (ptrs.size === 1) moved = 0; });
    cv.addEventListener('pointermove', e => {
      const p = ptrs.get(e.pointerId);
      if (!p) return this.setHover(this.cellAt(e.offsetX, e.offsetY));
      const before = [...ptrs.values()].map(q => ({ ...q }));
      p.x = e.offsetX; p.y = e.offsetY;
      const after = [...ptrs.values()];
      const mid = (a: { x: number; y: number }[]) => ({ x: a.reduce((t, q) => t + q.x, 0) / a.length, y: a.reduce((t, q) => t + q.y, 0) / a.length });
      const m0 = mid(before), m1 = mid(after);
      if (ptrs.size > 1) { this.zoomAt(m0.x, m0.y, Math.hypot(after[0].x - after[1].x, after[0].y - after[1].y) / Math.hypot(before[0].x - before[1].x, before[0].y - before[1].y)); moved = 99; }
      moved += Math.abs(m1.x - m0.x) + Math.abs(m1.y - m0.y);
      this.cam.ox += m1.x - m0.x; this.cam.oy += m1.y - m0.y; this.fitted = false; this.invalidate();
    });
    const up = (e: PointerEvent) => {
      if (ptrs.size === 1 && moved < 6 && e.type === 'pointerup') { this.pick = this.cellAt(e.offsetX, e.offsetY); this.onPick(this.pick); this.redraw(); }
      ptrs.delete(e.pointerId);
    };
    cv.addEventListener('pointerup', up); cv.addEventListener('pointercancel', up);
    cv.addEventListener('pointerleave', () => this.setHover(-1));
    cv.addEventListener('wheel', e => { e.preventDefault(); this.zoomAt(e.offsetX, e.offsetY, Math.exp(-e.deltaY * 0.0015)); }, { passive: false });
    cv.addEventListener('dblclick', () => this.fit());
  }

  setWorld(w: number, h: number, cellM: number, globe: { north: number; south: number } | null): void {
    this.w = w; this.h = h; this.cellM = cellM; this.globe = globe;
    this.pick = this.hover = -1;
    this.resize(); this.fit();
  }

  /** Draw the map's squares from `fields`. */
  setMap(fields: Fields): void {
    this.map?.drop(this.gpu);
    this.map = new FieldWindow(fields);
    this.invalidate();
  }

  /** Draw in view `look`. */
  setLook(look: MapLook): void { this.look = look; this.invalidate(); }

  /** The map class of each square (earth.ts `CLASSES`), as the GPU colours the map's squares now. */
  mapClasses(): Promise<Uint8Array> {
    const done = new Promise<Uint8Array>(resolve => this.asked.push(resolve));
    this.invalidate();
    return done;
  }

  /** Draw the live map `land` in place of the map's squares (null: the squares). */
  setLand(land: Land | null): void {
    if (this.land !== land) this.land?.stop(this.gpu);
    this.land = land;
    this.invalidate();
  }

  /** The part of the map in view. */
  view(): View {
    const { cam, w, h } = this, x0 = Math.max(0, -cam.ox / cam.s), y0 = Math.max(0, -cam.oy / cam.s);
    const x1 = Math.min(w, (this.cv.clientWidth - cam.ox) / cam.s), y1 = Math.min(h, (this.cv.clientHeight - cam.oy) / cam.s);
    return { x: x0, y: y0, w: Math.max(0, x1 - x0), h: Math.max(0, y1 - y0), s: cam.s, zoom: cam.s / this.fitScale() };
  }

  fit(): void {
    const k = this.fitScale();
    this.cam = { s: k, ox: (this.cv.clientWidth - this.w * k) / 2, oy: (this.cv.clientHeight - this.h * k) / 2 };
    this.fitted = true; this.invalidate();
  }

  /** The share of the screen's pixels along each axis the map is drawn at now. */
  get drawnAt(): number { return this.scale; }

  /** Draw the map anew at the next frame, on the GPU and over it. */
  invalidate(): void { this.gpuDue = true; this.redraw(); }

  /** Draw what lies over the map anew at the next frame (the GPU's picture stays). */
  redraw(): void { if (!this.raf) this.raf = requestAnimationFrame(() => { this.raf = 0; this.draw(); }); }

  private fitScale = () => Math.min(this.cv.clientWidth / this.w, this.cv.clientHeight / this.h) * 0.94 || 1;
  private dpr = () => window.devicePixelRatio || 1;

  private resize(): void {
    const d = this.dpr();
    for (const c of [this.cv, this.over]) { c.width = Math.round(this.cv.clientWidth * d); c.height = Math.round(this.cv.clientHeight * d); }
    if (this.fitted && this.w) this.fit();
  }

  private cellAt(px: number, py: number): number {
    let x = Math.floor((px - this.cam.ox) / this.cam.s);
    const y = Math.floor((py - this.cam.oy) / this.cam.s);
    if (this.globe && !this.fitted) x = ((x % this.w) + this.w) % this.w; // the map repeats east and west
    return x >= 0 && y >= 0 && x < this.w && y < this.h ? y * this.w + x : -1;
  }

  private setHover(i: number): void { if (i !== this.hover) { this.hover = i; this.onHover(i); this.redraw(); } }

  private zoomAt(px: number, py: number, k: number): void {
    const c = this.cam, s = Math.min(Math.max(c.s * k, this.fitScale() * 0.5), MOST_PX_PER_M * this.cellM);
    c.ox = px - ((px - c.ox) / c.s) * s; c.oy = py - ((py - c.oy) / c.s) * s; c.s = s;
    this.fitted = false; this.invalidate();
  }

  /** Draw the map: on the GPU if it is due, and what lies over it. */
  private draw(): void {
    if (!this.w) return;
    const { ctx, cam, w } = this, d = this.dpr(), span = w * cam.s, W = this.cv.clientWidth, H = this.cv.clientHeight;
    // Once moved, a globe is drawn again east and west of itself, so panning round it shows no seam.
    const wrap = this.globe !== null && !this.fitted;
    if (wrap) cam.ox = (((cam.ox % span) - span) % span);
    const { s, ox, oy } = cam, xs: number[] = [ox];
    if (wrap) for (let x = ox + span; x < W; x += span) xs.push(x);
    ctx.setTransform(d, 0, 0, d, 0, 0);
    ctx.clearRect(0, 0, W, H);
    if (this.gpu && this.gpuDue) {
      this.gpuDue = false;
      clearTimeout(this.sharpTimer);
      this.sharpening = null;
      if (this.land) this.land.flowAt = null;
      this.drawGpu(this.gpu, xs, d);
    }
    this.overlay?.(ctx, ox, oy, s);
    if (this.globe) this.latitudeLines(ox, oy, xs.length * span);
    for (const [i, col] of [[this.pick, '#fff'], [this.hover, '#000']] as const) {
      if (i < 0) continue;
      ctx.lineWidth = 1.5; ctx.strokeStyle = col;
      for (const x of xs) ctx.strokeRect(x + (i % w) * s - 0.5, oy + Math.floor(i / w) * s - 0.5, s + 1, s + 1);
    }
    this.scaleBar();
    const v = this.view(), key = [v.x, v.y, v.w, v.h, v.s].join();
    if (key !== this.seen) { this.seen = key; this.onView(v); }
  }

  /** The map on the GPU: the background and the map's shadow, the live map or the map's squares (a copy at each of
   * `xs`, css px), the layer over them; and the map's classes if they were asked for. While frames take the GPU longer
   * than `FRAME_BUDGET_MS` the map is drawn at fewer pixels than the screen's and stretched onto it: the same picture,
   * softer, as every program chooses its detail by the screen's pixel; once the view rests it is drawn anew at the
   * screen's pixels (`sharpen`). */
  private drawGpu(gpu: Gpu, xs: number[], d: number): void {
    const { cam: { s, ox, oy } } = this, W = this.cv.width, H = this.cv.height;
    const r = this.scale, k = d * r, tw = Math.max(1, Math.round(W * r)), th = Math.max(1, Math.round(H * r));
    if (r < 1 && (this.picture?.w !== tw || this.picture.h !== th)) {
      if (this.picture) gpu.drop(this.picture);
      this.picture = gpu.texture(tw, th, MAP_TARGET, 1, 1, true);
    }
    const waiting: Made[] = [];
    this.layer?.prepare(gpu, ox, oy, s, k, d, this.cv.clientWidth, this.cv.clientHeight, waiting);
    const target = r < 1 ? this.picture : null, p = gpu.pass(target);
    // While a tile of the ground fades in, the map is drawn again at each frame.
    const fading = this.land?.tiles.fading(performance.now()) ?? false;
    if (fading) this.invalidate();
    this.drawMap(gpu, p, target, xs, k, d, [tw, th], waiting);
    this.layer?.draw(p);
    p.end();
    if (target) this.show(target);
    if (this.asked.length && this.map) this.readClasses(gpu, this.map);
    gpu.submit();
    this.time(gpu, r);
    if (!this.warmed) { this.warmed = true; this.warmAhead(gpu, this.cellM / (s * d)); }
    if (r < 1 && !fading && !waiting.length) this.sharpTimer = window.setTimeout(() => this.sharpenSoon(xs, d), REST_MS);
    this.drawWhenMade(gpu, waiting);
    this.flowSoon();
  }

  /** Draw the map anew once the programs `waiting` that the frame just drawn went without are made (looked at every
   * `MADE_POLL_MS`); till then it stays due. So no frame waits for a program to be made: it draws without it. */
  private drawWhenMade(gpu: Gpu, waiting: Made[]): void {
    clearTimeout(this.madeTimer);
    if (!waiting.length) return;
    this.gpuDue = true;
    const look = () => {
      if (waiting.every(([name, target, blend]) => gpu.ready(name, target, blend))) this.redraw();
      else this.madeTimer = window.setTimeout(look, MADE_POLL_MS);
    };
    this.madeTimer = window.setTimeout(look, MADE_POLL_MS);
  }

  /** Draw the map again when the signs of the flow next step along the channels (land.ts `FLOW_FPS`), while a channel
   * in view shows them, the page is seen, and the map is drawn at the screen's own pixels (a card that cannot keep up
   * shows them still: the sharp picture is not drawn over and over). Else no frame is drawn until something changes. */
  private flowSoon(): void {
    clearTimeout(this.flowTimer);
    const v = this.view();
    if (document.hidden || this.scale < 1 || !this.land?.flowing(v.x, v.y, v.w, v.h, this.cellM / (v.s * this.dpr()))) return;
    const step = 1000 / FLOW_FPS;
    this.flowTimer = window.setTimeout(() => this.invalidate(), step - (performance.now() % step) + 1);
  }

  /** Start drawing the map at the screen's own pixels, in tiles each of about `SHARPEN_MS` on the GPU, into the sharp
   * picture's texture, which first holds the map drawn at fewer pixels, stretched: so each frame shows that one texture,
   * the tiles drawn so far over the stretched picture. */
  private sharpenSoon(xs: number[], d: number): void {
    const gpu = this.gpu, W = this.cv.width, H = this.cv.height, tiles: number[][] = [];
    if (!gpu) return;
    const side = Math.max(TILE_LEAST, Math.floor(Math.sqrt((SHARPEN_MS / Math.max(this.fullMs, SHARPEN_MS)) * W * H)));
    for (let y = 0; y < H; y += side) for (let x = 0; x < W; x += side) tiles.push([x, y, Math.min(side, W - x), Math.min(side, H - y)]);
    if (this.sharp?.w !== W || this.sharp.h !== H) {
      if (this.sharp) gpu.drop(this.sharp);
      this.sharp = gpu.texture(W, H, MAP_TARGET, 1, 1, true);
    }
    const q = gpu.pass(this.sharp);
    q.draw('picture', { rect: [0, 0, W, H], binds: { 1: this.picture!, 2: 'smooth' } }, false);
    q.end();
    // All the sharp picture's tiles show the flow's signs where they stood as it began.
    if (this.land) this.land.flowAt = performance.now();
    // What the layer works out before its draws holds for the whole view, so it is worked out once for all the tiles
    // (a new frame stops them); the frame it sharpens went without no program, so none is waited for here.
    const { cam: { s, ox, oy } } = this;
    this.layer?.prepare(gpu, ox, oy, s, d, d, this.cv.clientWidth, this.cv.clientHeight, []);
    const job = { tiles, xs, d };
    this.sharpening = job;
    this.sharpen(job);
  }

  /** Show on the screen the picture `t` drawn before, stretched over it. */
  private show(t: Texture): void {
    const gpu = this.gpu!, q = gpu.pass(null);
    q.draw('picture', { rect: [0, 0, this.cv.width, this.cv.height], binds: { 1: t, 2: 'smooth' } }, false);
    q.end();
  }

  /** Draw the next tile of the sharp picture `job` once the GPU has finished the last, and show each tile as soon as it
   * is drawn; a new frame stops it. */
  private sharpen(job: Sharpening): void {
    const gpu = this.gpu, tile = job.tiles.shift(), W = this.cv.width, H = this.cv.height;
    if (this.sharpening !== job || !gpu || !tile || this.sharp?.w !== W || this.sharp.h !== H) return;
    const p = gpu.pass(this.sharp, undefined, tile);
    this.drawMap(gpu, p, this.sharp, job.xs, job.d, job.d, [W, H], []);
    this.layer?.draw(p);
    p.end();
    this.show(this.sharp!);
    if (!job.tiles.length) this.sharpening = null;
    gpu.submit();
    if (job.tiles.length) gpu.done().then(() => requestAnimationFrame(() => this.sharpen(job)));
  }

  /** Make ahead, after the first frame, the programs the map may draw next (`Gpu.warm`): the live map's made for the
   * sizes of a pixel nearest the first frame's `seen` m first, the layer's after the nearest two, then the rest; each
   * for the canvas and for the textures the map is drawn into (`MAP_TARGET`). So zooming in or out first does not wait
   * for a program to be made. */
  private warmAhead(gpu: Gpu, seen: number): void {
    const lands = madeFor('land', Infinity), m = (key: string) => +(key.split(':')[1] ?? 0) || LAND_FINEST_M;
    lands.sort((a, b) => Math.abs(Math.log(m(a) / seen)) - Math.abs(Math.log(m(b) / seen)));
    const targets = [null, MAP_TARGET];
    lands.forEach((key, i) => {
      targets.forEach(t => gpu.warm(key, t, false));
      if (i === 1) this.layer?.warm(gpu, targets);
    });
    gpu.warm('picture', null, false);
  }

  /** Draw into pass `p` (`target`, null: the canvas; of `size` px, `k` px a css px, on a screen of `d` px a css px) the
   * background and the map's shadow, and the live map or the map's squares (a copy at each of `xs`, css px); the live
   * map only once a program for it is made (`seenFrom`, else it adds the one it waits for to `waiting`). */
  private drawMap(gpu: Gpu, p: Pass, target: Texture | null, xs: number[], k: number, d: number, size: number[], waiting: Made[]): void {
    const { cam: { s, ox, oy }, w, h, look } = this;
    this.ctx.fillStyle = css('--map-bg');
    const bg = (this.ctx.fillStyle as string).match(/[0-9a-f]{2}/gi)?.map(c => parseInt(c, 16) / 255) ?? [1, 1, 1];
    const backdrop = new Uniforms(3).vec4(...bg).vec4(ox * k, oy * k, xs.length * w * s * k, h * s * k).vec4(SHADOW_DROP * k, SHADOW_BLUR * k, SHADOW_DARK);
    p.draw('backdrop', { rect: [0, 0, size[0], size[1]], binds: { 1: backdrop.buffer } }, false);
    // Copies meet on whole pixels, so no thin line shows between them.
    const edge = (c: number) => Math.round((xs[0] + c * w * s) * k);
    if (this.land) this.land.drawn = d;
    // The live map's program made for the screen's pixels (gpu.ts `seenFrom`): its detail is chosen by them.
    const mpp = this.cellM / (s * k), seen = this.cellM / (s * d), under = this.layer?.under(gpu, s, d);
    const program = this.land ? seenFrom(gpu, 'land', seen, target?.format ?? null, false, waiting) : null;
    // Each copy is drawn over the part of the target it covers, so what a pixel reads of its place (`frameOf`) never
    // runs over more than the target's pixels.
    const lands = xs.map((_, c) => this.land?.binds(gpu, look, frameOf(edge(c), oy * k, mpp, seen)));
    if (lands[0] && under) {
      if (program) xs.forEach((_, c) => {
        const x0 = Math.max(edge(c), 0), x1 = Math.min(edge(c + 1), size[0]), y0 = Math.max(oy * k, 0), y1 = Math.min((oy + h * s) * k, size[1]);
        if (x1 > x0 && y1 > y0) p.draw(program, { rect: [x0, y0, x1 - x0, y1 - y0], binds: { ...lands[c]!, ...under } }, false);
      });
    } else if (this.map) xs.forEach((_, c) => p.draw('earth', { rect: [edge(c), oy * k, edge(c + 1) - edge(c), h * s * k], binds: this.map!.binds(gpu, look) }, false));
  }

  /** Time the frame just sent at `r` of the screen's pixels along each axis on the GPU (one at a time), and take the
   * share the next frames are drawn at from it: the largest of `SCALES` at which a frame would meet `FRAME_BUDGET_MS`,
   * else the least. */
  private time(gpu: Gpu, r: number): void {
    if (this.timing) return;
    this.timing = true;
    const t0 = performance.now();
    gpu.done().then(() => {
      this.timing = false;
      const ms = performance.now() - t0;
      this.fullMs = ms / (r * r);
      const was = this.scale;
      this.scale = SCALES.find(k => this.fullMs * k * k <= FRAME_BUDGET_MS) ?? SCALES[SCALES.length - 1];
      if (this.scale !== was) this.onScale();
    });
  }

  /** Draw the classes of the map's squares, a pixel a square, and hand them to those that asked. */
  private readClasses(gpu: Gpu, map: FieldWindow): void {
    const asked = this.asked;
    this.asked = [];
    if (this.classes?.w !== this.w || this.classes.h !== this.h) {
      if (this.classes) gpu.drop(this.classes);
      this.classes = gpu.texture(this.w, this.h, 'rgba8unorm', 1, 1, true);
    }
    const p = gpu.pass(this.classes);
    p.draw('class', { rect: [0, 0, this.w, this.h], binds: map.binds(gpu, this.look) }, false);
    p.end();
    gpu.read(this.classes).then(rgba => {
      const cls = Uint8Array.from({ length: this.w * this.h }, (_, i) => rgba[4 * i]);
      asked.forEach(a => a(cls));
    });
  }

  /** A globe's lines of latitude every LAT_STEP degrees (the equator heavier), named at the map's west edge. */
  private latitudeLines(ox: number, oy: number, width: number): void {
    const { ctx, cam: { s }, h, globe } = this;
    if (!globe) return;
    const { north, south } = globe, left = Math.max(ox, 0);
    ctx.save();
    ctx.font = '11px system-ui, sans-serif'; ctx.textBaseline = 'bottom'; ctx.lineJoin = 'round';
    for (let lat = Math.ceil(south / LAT_STEP) * LAT_STEP; lat <= north; lat += LAT_STEP) {
      const y = oy + ((north - lat) / (north - south)) * h * s;
      if (y <= oy + 1 || y >= oy + h * s - 1) continue; // the poles are the map's edges
      ctx.strokeStyle = '#fff'; ctx.globalAlpha = lat === 0 ? 0.7 : 0.45; ctx.lineWidth = lat === 0 ? 1.5 : 1;
      ctx.setLineDash(lat === 0 ? [] : [5, 4]);
      ctx.beginPath(); ctx.moveTo(ox, y); ctx.lineTo(ox + width, y); ctx.stroke();
      const name = lat === 0 ? '0°' : `${Math.abs(lat)}°${lat > 0 ? 'N' : 'S'}`;
      ctx.setLineDash([]); ctx.globalAlpha = 0.9; ctx.lineWidth = 3; ctx.strokeStyle = 'rgba(0,0,0,.55)'; ctx.fillStyle = '#fff';
      ctx.strokeText(name, left + 5, y - 2); ctx.fillText(name, left + 5, y - 2);
    }
    ctx.restore();
  }

  private scaleBar(): void {
    const { ctx, cam: { s }, cellM } = this, target = (96 / s) * cellM; // metres for ~96 px
    const p = 10 ** Math.floor(Math.log10(target)), m = [1, 2, 5, 10].map(k => k * p).reduce((a, b) => (Math.abs(b - target) < Math.abs(a - target) ? b : a));
    const px = (m / cellM) * s, x = 14, y = this.cv.clientHeight - 14;
    ctx.strokeStyle = ctx.fillStyle = css('--ink'); ctx.lineWidth = 1.5; ctx.globalAlpha = 0.75;
    ctx.beginPath(); ctx.moveTo(x, y - 5); ctx.lineTo(x, y); ctx.lineTo(x + px, y); ctx.lineTo(x + px, y - 5); ctx.stroke();
    ctx.font = '11px system-ui, sans-serif'; ctx.textBaseline = 'bottom';
    ctx.fillText(m >= 1000 ? `${m / 1000} km` : `${m} m`, x, y - 7);
    ctx.globalAlpha = 1;
  }
}
