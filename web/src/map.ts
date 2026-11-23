// The map, drawn on the GPU (gpu.ts) at the screen's own pixels: under it the page's background and the map's shadow,
// then the live map from the ground's tiles and the cells' table (land.ts), or a tape's map of squares (fields.ts, a
// texel a square), then what others draw on the GPU (the plant cover, vegetation.ts).
// The page's map canvas lies over the GPU's own canvas, takes the pointer, and holds in 2D what is drawn by hand: a globe's lines of latitude, the squares under the pointer and
// picked, and the scale bar.
import { Draw, Format, Gpu, HeldGpu, Pass, Texture, Uniforms, openGpu, seenFrom } from './gpu';
import { Fields } from './fields';
import { FieldWindow, MapLook, frameOf } from './field-window';
import { FLOW_FPS, Land } from './land';
import { Box, FIRE_FPS, Fire } from './fire';
import { detailLost, detailSeen, sendDetail } from './detail';
import { spend, timed } from './cost';

const LAT_STEP = 30; // degrees between a globe's lines of latitude
const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

/** The part of the map in view: its window in squares (clipped to the map), the pixels a square takes, and the zoom (1: the map fits). */
export interface View { x: number; y: number; w: number; h: number; s: number; zoom: number }
/** The most pixels a metre takes on screen. */
const MOST_PX_PER_M = 80;
/** The scale bar's margin from the free area's bottom left corner, css px. */
const SCALE_MARGIN_PX = 14;
/** The most pixels of the map's canvas a css pixel takes: a screen with more (a phone's, 3) is drawn at this many and
 * stretched onto it, 2.25 times fewer pixels for a picture a little softer (what is drawn over it, the lines and the
 * text, keeps the screen's own). */
export const MOST_DRAWN_PX = 2;
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
/** While the view moves (a pan, a pinch, a wheel: the last within `MOVE_REST_MS`), the last picture drawn is shown
 * moved and scaled as the camera did, at once (`showMoved`), and the map is drawn anew no sooner than `MOVE_REDRAW_MS`
 * after the card finished the last (to fill what the moved picture leaves bare); once the view rests it is drawn anew. */
const MOVE_REST_MS = 200, MOVE_REDRAW_MS = 400;
/** The pictures of the area about a view at rest (`MapView.rings`) are these many times its width, each at the screen's
 * pixels: so a view that moves out meets one at most three times softer than the last, and the whole map's picture (one
 * colour where the view is close) only once it is far. They are drawn once the view has rested `AROUND_REST_MS`, the
 * nearest first, a tile each time the card is free; the whole map's picture is `WHOLE_PX` px across, drawn again after
 * them once older than `WHOLE_EVERY_MS` (the world moves on). */
const RINGS = [2, 6, 18], AROUND_REST_MS = 250, WHOLE_PX = 512, WHOLE_EVERY_MS = 20000;
/** A picture drawn anew after the view moved, or sharp over the softer picture of its view, comes in over what the
 * screen showed in this long, ms (`present`): it does not jump in. */
const FADE_MS = 220;
/** A picture the map was drawn into: its texture, the square at its top-left corner (x, y, squares), the squares it
 * spans across and down, and the look and canvas it was drawn for (`MapView.lookKey`). */
interface Pic { t: Texture; x: number; y: number; w: number; h: number; look: string }
/** The picture texture `t` holds, drawn with camera `cam` (css px a square, the map's corner, css px) at `k` of its
 * pixels a css pixel. */
function picOf(t: Texture, cam: { s: number; ox: number; oy: number }, k: number, look: string): Pic {
  return { t, x: -cam.ox / cam.s, y: -cam.oy / cam.s, w: t.w / k / cam.s, h: t.h / k / cam.s, look };
}
/** How often a frame drawn without a program not made yet looks whether it is made, ms (`drawWhenMade`). */
const MADE_POLL_MS = 50;

/** Drawn on the GPU over the map by another module, in the map's pass, at every frame: what it works out before the
 * pass, where the map's corner is at (ox, oy) css px, a square takes `s` css px, a css px `d` px of the target and
 * `seen` px of the screen (its detail is chosen by the screen's), and the view is `w` x `h` css px, adding to `waiting`
 * the programs it goes without as they are not made yet (`prepare`); its draw in the pass; and what the live map's
 * ground draws of it, with a square taking `s` css px and a css px `seen` px of the screen (`under`, its bindings). */
export interface GpuLayer {
  prepare(gpu: Gpu, ox: number, oy: number, s: number, d: number, seen: number, w: number, h: number, waiting: string[]): void;
  draw(p: Pass): void;
  under(gpu: Gpu, s: number, seen: number): Draw['binds'];
}

/** The format of the textures the map is drawn into below the screen's pixels and for its sharp picture. */
const MAP_TARGET: Format = 'rgba8unorm';

/** The sharp picture being drawn once the view rests: its tiles to draw (x, y, w, h device px), the copies' places
 * (css px), the device pixels a css pixel, the view it is of (`viewKey`), the texture it is drawn into (out of
 * sight, shown once whole: `MapView.sharpenSoon`), the time its tiles drawn so far took the GPU, ms, and the camera it
 * is drawn with. */
interface Sharpening { tiles: number[][]; xs: number[]; d: number; key: string; into: Texture; ms: number; cam: { s: number; ox: number; oy: number } }
/** How long the world's next frame waits at most for the sharp picture of the last, ms (`shown`), and how often it
 * looks whether that picture is whole, ms. */
const SHOWN_MOST_MS = 1500, SHOWN_POLL_MS = 30;

export class MapView {
  hover = -1;
  pick = -1;
  /** The GPU once it is open (null: none yet, or the browser has none). */
  private gpu: HeldGpu | null = null;
  readonly ready: Promise<HeldGpu | null>;
  /** The canvas the GPU draws on, under the map's canvas (none before the GPU opens). */
  private gpuCv: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D;
  private cam = { s: 1, ox: 0, oy: 0 };
  private fitted = true;
  /** The edges of the map covered by the controls over it, css px (top, right, bottom, left): a fit fills the rest, and
   * the scale bar stands in its bottom left corner. */
  private covered: [number, number, number, number] = [0, 0, 0, 0];
  private w = 0; private h = 0; private cellM = 1;
  /** Edge latitudes (degrees) of a globe, whose columns wrap east to west; null for an island. */
  private globe: { north: number; south: number } | null = null;
  private raf = 0;
  /** The map's squares (a tape's map, and the classes of the live map's squares), and the live map. */
  private map: FieldWindow | null = null;
  private land: Land | null = null;
  private look: MapLook = { mode: 'earth', layer: '', ramp: [], ground: 0 };
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
  /** The textures the map drawn whole goes into, by turns: the one shown last stays whole while the next is drawn,
   * so it can be faded from (`present`). */
  private picture: Texture | null = null;
  private pictureNext: Texture | null = null;
  /** The last picture shown whole on the screen (`picture` or `sharp`) and the camera and look it was drawn with,
   * which a moving view shows moved (`showMoved`); when the view last moved, when the card finished the map drawn
   * anew last, ms (Infinity while it draws it), and the timer that draws it once the view rests. */
  private last: Pic | null = null;
  /** Pictures shown under the last as the view moves, so it moves over less detail, never over nothing: the area about
   * the view at rest, `RINGS` times its width, and the whole map at `WHOLE_PX` px across, each in its texture (and the
   * one the next ring is drawn into); the timer that starts drawing them once the view rests, and the count of those
   * starts (a move stops the one under way, `aroundSoon`). */
  private rings: (Pic | null)[] = [];
  private whole: Pic | null = null;
  private ringTex: (Texture | null)[] = [];
  private ringSpare: Texture | null = null;
  private wholeTex: Texture | null = null;
  private aroundTimer = 0;
  private aroundRun = 0;
  /** What a moving view showed last (`showMoved`), which the picture drawn anew comes in over; and the pictures the
   * last comes in over and when it began (`present`). */
  private movedShown: Pic[] | null = null;
  private fade: { from: Pic[]; at: number } | null = null;
  private fadeFrame = 0;
  private wholeAt = -Infinity;
  private movedAt = -Infinity;
  private lastDrawn = -Infinity;
  private moveTimer = 0;
  private sharpTimer = 0;
  /** Whether the GPU's picture is to be drawn anew at the next frame. */
  private gpuDue = false;
  /** The timer of the next frame of the flow's signs (`flowSoon`), and of the flames (`fireSoon`). */
  private flowTimer = 0;
  private fireTimer = 0;
  private sharpening: Sharpening | null = null;
  private sharp: Texture | null = null;
  /** Whether the view rests before its sharp picture is begun (`sharpTimer` runs) and the view that rests, and the
   * view the sharp picture's
   * texture holds sharp all over ('': none, while it holds a stretched picture in part), which a new frame of the
   * same view is drawn over, tile by tile, with no picture at fewer pixels between (`draw`). */
  private resting = false;
  private restOf = '';
  private sharpOf = '';
  /** The texture the next sharp picture of the same view is drawn into, out of sight, till it is whole. */
  private sharpNext: Texture | null = null;
  /** Whether the frame last drawn went without a program not made yet (`drawWhenMade`). */
  private wentWithout = false;
  /** Whether a frame waits for the card to finish the last (`draw`). */
  private waitingCard = false;
  /** The timer that looks whether the programs the last frame went without are made (`drawWhenMade`). */
  private madeTimer = 0;
  /** Drawn by another module on the GPU over the map, and on the 2D canvas over it at each redraw: its context, where
   * the map's corner is and the pixels a square takes. */
  layer: GpuLayer | null = null;
  /** Fire on the live map (null: none drawn), and whether its flames move: while the world plays. */
  fire: Fire | null = null;
  playing = false;
  overlay: ((ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number) => void) | null = null;
  /** A tool that paints with the pointer (god mode): told where a press of the main button starts (squares), true
   * takes the press and its drag in place of a pan; then told of each move and of the end. A second finger, or another
   * button, still moves the map. */
  paint: { start(x: number, y: number): boolean; move(x: number, y: number): void; end(): void } | null = null;

  constructor(private cv: HTMLCanvasElement, private onHover: (i: number) => void, private onPick: (i: number) => void, private onView: (v: View) => void = () => {}) {
    this.ctx = cv.getContext('2d')!;
    // The GPU's canvas, under the map's canvas.
    const c = document.createElement('canvas');
    c.className = 'gpu';
    cv.before(c);
    this.gpuCv = c;
    this.resize();
    const restored = () => { this.timing = false; this.sharpOf = ''; this.last = this.whole = this.movedShown = this.fade = null; this.rings = []; this.sharpening = null; this.resting = false; clearTimeout(this.sharpTimer); this.land?.tiles.lost(); this.fire?.stop(null); detailLost(); this.invalidate(); };
    this.ready = Promise.resolve(openGpu(c, restored)).then(g => {
      this.gpu = g;
      if (g) sendDetail(g, () => { this.sharpOf = ''; this.invalidate(); });
      this.invalidate();
      return g;
    });
    new ResizeObserver(() => { this.resize(); this.invalidate(); }).observe(cv);
    const ptrs = new Map<number, { x: number; y: number }>();
    let moved = 0;
    /** The pointer that paints (-1: none), and where it is. */
    let painter = -1, painterAt = { x: 0, y: 0 };
    const at = (e: PointerEvent): [number, number] => [(e.offsetX - this.cam.ox) / this.cam.s, (e.offsetY - this.cam.oy) / this.cam.s];
    const stopPainting = () => { if (painter >= 0) { painter = -1; this.paint?.end(); } };
    cv.addEventListener('pointerdown', e => {
      cv.setPointerCapture(e.pointerId);
      if (painter >= 0) {
        // A second finger: the stroke ends, and the two move the map.
        ptrs.set(painter, painterAt); stopPainting(); moved = 99;
      } else if (!ptrs.size && e.button === 0 && this.paint?.start(...at(e))) { painter = e.pointerId; painterAt = { x: e.offsetX, y: e.offsetY }; return; }
      ptrs.set(e.pointerId, { x: e.offsetX, y: e.offsetY });
      if (ptrs.size === 1) moved = 0;
    });
    cv.addEventListener('contextmenu', e => { if (this.paint) e.preventDefault(); });
    cv.addEventListener('pointermove', e => {
      if (e.pointerId === painter) { painterAt = { x: e.offsetX, y: e.offsetY }; this.setHover(this.cellAt(e.offsetX, e.offsetY)); return this.paint?.move(...at(e)); }
      const p = ptrs.get(e.pointerId);
      if (!p) return this.setHover(this.cellAt(e.offsetX, e.offsetY));
      const before = [...ptrs.values()].map(q => ({ ...q }));
      p.x = e.offsetX; p.y = e.offsetY;
      const after = [...ptrs.values()];
      const mid = (a: { x: number; y: number }[]) => ({ x: a.reduce((t, q) => t + q.x, 0) / a.length, y: a.reduce((t, q) => t + q.y, 0) / a.length });
      const m0 = mid(before), m1 = mid(after);
      if (ptrs.size > 1) { this.zoomAt(m0.x, m0.y, Math.hypot(after[0].x - after[1].x, after[0].y - after[1].y) / Math.hypot(before[0].x - before[1].x, before[0].y - before[1].y)); moved = 99; }
      moved += Math.abs(m1.x - m0.x) + Math.abs(m1.y - m0.y);
      this.cam.ox += m1.x - m0.x; this.cam.oy += m1.y - m0.y; this.fitted = false; this.moved(); this.invalidate();
    });
    const up = (e: PointerEvent) => {
      if (e.pointerId === painter) return stopPainting();
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
    const k = this.fitScale(), [top, right, bottom, left] = this.covered;
    this.cam = { s: k, ox: left + (this.cv.clientWidth - left - right - this.w * k) / 2, oy: top + (this.cv.clientHeight - top - bottom - this.h * k) / 2 };
    this.fitted = true; this.invalidate();
  }

  /** The controls over the map now cover its edges `covered` (css px: top, right, bottom, left); a fitted map fits
   * anew. */
  setCovered(covered: [number, number, number, number]): void {
    if (covered.every((v, k) => v === this.covered[k])) return;
    this.covered = covered;
    if (this.fitted && this.w) this.fit(); else this.redraw();
  }

  /** The share of the screen's pixels along each axis the map is drawn at now. */
  get drawnAt(): number { return this.scale; }

  /** Draw the map anew at the next frame, on the GPU and over it. */
  invalidate(): void { this.gpuDue = true; this.redraw(); }

  /** Draw what lies over the map anew at the next frame (the GPU's picture stays). */
  redraw(): void { if (!this.raf) this.raf = requestAnimationFrame(() => { this.raf = 0; this.draw(); }); }

  private fitScale = () => {
    const [top, right, bottom, left] = this.covered;
    return Math.min((this.cv.clientWidth - left - right) / this.w, (this.cv.clientHeight - top - bottom) / this.h) * 0.94 || 1;
  };
  private dpr = () => window.devicePixelRatio || 1;
  /** The pixels of the map's canvas a css pixel (`MOST_DRAWN_PX`). */
  private drawnDpr = () => Math.min(this.dpr(), MOST_DRAWN_PX);

  private resize(): void {
    for (const [c, d] of [[this.gpuCv, this.drawnDpr()], [this.cv, this.dpr()]] as const) if (c) { c.width = Math.round(this.cv.clientWidth * d); c.height = Math.round(this.cv.clientHeight * d); }
    if (this.fitted && this.w) this.fit();
  }

  private cellAt(px: number, py: number): number {
    let x = Math.floor((px - this.cam.ox) / this.cam.s);
    const y = Math.floor((py - this.cam.oy) / this.cam.s);
    if (this.globe && !this.fitted) x = ((x % this.w) + this.w) % this.w; // the map repeats east and west
    return x >= 0 && y >= 0 && x < this.w && y < this.h ? y * this.w + x : -1;
  }

  /** The view moved now: a move that starts after it rested shows the last picture moved first (the map is drawn
   * anew `MOVE_REDRAW_MS` on, as if the card had just finished it). */
  private moved(): void {
    const now = performance.now();
    if (now - this.movedAt >= MOVE_REST_MS) this.lastDrawn = now;
    this.movedAt = now;
  }

  private setHover(i: number): void { if (i !== this.hover) { this.hover = i; this.onHover(i); this.redraw(); } }

  private zoomAt(px: number, py: number, k: number): void {
    const c = this.cam, s = Math.min(Math.max(c.s * k, this.fitScale() * 0.5), MOST_PX_PER_M * this.cellM);
    c.ox = px - ((px - c.ox) / c.s) * s; c.oy = py - ((py - c.oy) / c.s) * s; c.s = s;
    this.fitted = false; this.moved(); this.invalidate();
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
    // While the sharp picture of this same view is being drawn or about to be (the world stepped or a tile came
    // meanwhile: the camera did not move), it is left to finish, and the frame stays due till it is whole (`sharpen`):
    // tiles that come one after another never keep the map from getting sharp.
    const viewed = this.viewKey(xs);
    const held = this.scale < 1 && !this.asked.length && (this.sharpening?.key === viewed || (this.resting && this.restOf === viewed));
    // A frame is sent only once the card has drawn the last, so frames never pile up in its queue; till then the
    // frame stays due and is drawn as soon as the card is free.
    const now = performance.now(), moving = now - this.movedAt < MOVE_REST_MS && now - this.lastDrawn < MOVE_REDRAW_MS;
    if (held) { /* (stays due) */ } else if (this.gpu && this.gpuDue && this.gpu.busy) {
      if (!this.waitingCard) { this.waitingCard = true; this.gpu.idle().then(() => { this.waitingCard = false; this.redraw(); }); }
    } else if (this.gpu && this.gpuDue && moving && this.last?.look === this.lookKey()) {
      // The view moves: the last picture moved with it, over those of the area about it (the widest lowest) and of the
      // whole map, and the map drawn anew once it rests (or `MOVE_REDRAW_MS` on).
      this.showMoved(this.gpu, [this.whole, ...[...this.rings].reverse(), this.last]);
      clearTimeout(this.moveTimer);
      this.moveTimer = window.setTimeout(() => this.redraw(), Math.min(this.movedAt + MOVE_REST_MS, this.lastDrawn + MOVE_REDRAW_MS) - now + 1);
    } else if (this.gpu && this.gpuDue) {
      this.gpuDue = false;
      // (Counted from when the card has drawn it: a slow card's frame is not drawn again at once while the view moves.)
      this.lastDrawn = Infinity;
      this.gpu.idle().then(() => { this.lastDrawn = performance.now(); });
      clearTimeout(this.sharpTimer);
      this.resting = false;
      this.sharpening = null;
      if (this.land) this.land.flowAt = null;
      // A view whose sharp picture is whole (the world stepped, a tile came: the camera did not move) is drawn sharp
      // again over it, so a playing world's map stays sharp on a slow card; else the frame is drawn whole first.
      // (A frame that draws with programs the last went without may cost far more than the last one timed: it is
      // drawn whole at fewer pixels first, and timed, as a frame after the ground's detail came is, `sendDetail`.)
      const again = this.scale < 1 && this.sharpOf === viewed && !this.asked.length && !this.wentWithout;
      timed('draw', () => (again ? this.sharpenSoon(xs, this.drawnDpr()) : this.drawGpu(this.gpu!, xs, this.drawnDpr())));
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
    const { cam: { s, ox, oy } } = this, W = this.gpuCv!.width, H = this.gpuCv!.height;
    const r = this.scale, k = d * r, tw = Math.max(1, Math.round(W * r)), th = Math.max(1, Math.round(H * r));
    // (Drawn into a texture at the screen's pixels too, so a moving view can show it moved, `showMoved`.)
    [this.picture, this.pictureNext] = [this.pictureNext, this.picture];
    if (this.picture?.w !== tw || this.picture.h !== th) {
      if (this.picture) this.dropPicture(gpu, this.picture);
      this.picture = gpu.texture('picture', tw, th, MAP_TARGET, 1, 1, true);
    }
    const waiting: string[] = [];
    this.layer?.prepare(gpu, ox, oy, s, k, d, this.cv.clientWidth, this.cv.clientHeight, waiting);
    this.fire?.prepare(gpu, waiting);
    const target = this.picture!, p = gpu.pass(target);
    // While a tile of the ground fades in, the map is drawn again at each frame; none fades where the card cannot
    // draw a frame at the screen's pixels in time (it would draw nothing but fades while tiles come).
    if (this.land) this.land.tiles.fades = r >= 1;
    const fading = this.land?.tiles.fading(performance.now()) ?? false;
    if (fading) this.invalidate();
    this.drawMap(gpu, p, xs, k, d, [tw, th], waiting);
    this.layer?.draw(p);
    this.drawFire(gpu, p, xs, k, d, [tw, th], waiting);
    p.end();
    this.last = picOf(target, this.cam, k, this.lookKey());
    this.present(gpu, this.movedShown);
    this.movedShown = null;
    if (this.asked.length && this.map) this.readClasses(gpu, this.map);
    gpu.submit();
    this.time(gpu, r);
    // The ground's detail is held only while the view is close enough to show it.
    detailSeen(this.cellM / (s * d));
    if (r < 1 && !fading && !waiting.length) {
      this.resting = true;
      this.restOf = this.viewKey(xs);
      this.sharpTimer = window.setTimeout(() => { this.resting = false; this.sharpenSoon(xs, d); }, REST_MS);
    } else if (r >= 1 && !fading && !waiting.length) this.aroundSoon();
    this.drawWhenMade(gpu, waiting);
    this.flowSoon();
    this.fireSoon();
  }

  /** Draw the map anew once the programs `waiting` that the frame just drawn went without are made (looked at every
   * `MADE_POLL_MS`); till then it stays due. So no frame waits for a program to be made: it draws without it. */
  private drawWhenMade(gpu: Gpu, waiting: string[]): void {
    clearTimeout(this.madeTimer);
    this.wentWithout = waiting.length > 0;
    if (!waiting.length) return;
    this.gpuDue = true;
    const look = () => {
      if (waiting.every(name => gpu.ready(name))) this.redraw();
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
    if (document.hidden || this.scale < 1 || !this.land?.flowing(v.x, v.y, v.w, v.h, this.cellM / (v.s * this.drawnDpr()))) return;
    const step = 1000 / FLOW_FPS;
    this.flowTimer = window.setTimeout(() => this.invalidate(), step - (performance.now() % step) + 1);
  }

  /** Draw the map again when the flames next move (`FIRE_FPS`), while the world plays, a fire burns in view, the page
   * is seen and the map is drawn at the screen's own pixels. */
  private fireSoon(): void {
    clearTimeout(this.fireTimer);
    const b = this.fire?.flames, v = this.view();
    if (!b || !this.playing || document.hidden || this.scale < 1 || b[2] <= v.x || b[0] >= v.x + v.w || b[3] <= v.y || b[1] >= v.y + v.h) return;
    const step = 1000 / FIRE_FPS;
    this.fireTimer = window.setTimeout(() => this.invalidate(), step - (performance.now() % step) + 1);
  }

  /** The view the map is drawn in, as text: the camera, the canvas, the copies and the look. */
  private viewKey(xs: number[]): string {
    const { cam, look } = this;
    return [cam.s, cam.ox, cam.oy, this.gpuCv?.width, this.gpuCv?.height, xs.length, look.mode, look.layer, look.ground].join();
  }

  /** Once the frame the map was last asked for is whole on the screen: drawn, and where the map is drawn at fewer
   * pixels than the screen's, its sharp picture drawn too; or after `SHOWN_MOST_MS` where the view moved (a view that
   * moves is never sharp). The world's worker posts its next frame then (live.ts `drawn`). */
  async shown(): Promise<void> {
    const t0 = performance.now();
    for (;;) {
      await this.gpu?.idle();
      // (A sharp picture being drawn is waited for whole, however long its card takes: it shows nothing till then.)
      if (!(this.gpuDue || this.resting || this.sharpening) || (!this.sharpening && performance.now() - t0 > SHOWN_MOST_MS)) return;
      await new Promise(go => setTimeout(go, SHOWN_POLL_MS));
    }
  }

  /** Start drawing the map at the screen's own pixels, in tiles each of about `SHARPEN_MS` on the GPU, into a texture
   * out of sight, shown once whole: the screen never shows a picture sharp in part (a tile drawn sharp, then the
   * stretched picture again as the view moves, flickers), and a picture of one date all over. */
  private sharpenSoon(xs: number[], d: number): void {
    const gpu = this.gpu, W = this.gpuCv!.width, H = this.gpuCv!.height, tiles = this.tilesOf(W, H);
    if (!gpu) return;
    const sized = (t: Texture | null) => {
      if (t?.w === W && t.h === H) return t;
      if (t) this.dropPicture(gpu, t);
      return gpu.texture('picture', W, H, MAP_TARGET, 1, 1, true);
    };
    if (this.sharp?.w !== W || this.sharp.h !== H) this.sharpOf = '';
    this.sharp = sized(this.sharp);
    this.sharpNext = sized(this.sharpNext);
    // All the sharp picture's tiles show the flow's signs where they stood as it began.
    if (this.land) this.land.flowAt = performance.now();
    // What the layer works out before its draws holds for the whole view, so it is worked out once for all the tiles
    // (a new frame stops them); a program it goes without is waited for as a whole frame's is (`drawWhenMade`).
    const { cam: { s, ox, oy } } = this, waiting: string[] = [];
    this.layer?.prepare(gpu, ox, oy, s, d, d, this.cv.clientWidth, this.cv.clientHeight, waiting);
    this.fire?.prepare(gpu, waiting);
    this.drawWhenMade(gpu, waiting);
    const job = { tiles, xs, d, key: this.viewKey(xs), into: this.sharpNext!, ms: 0, cam: { ...this.cam } };
    this.sharpening = job;
    this.sharpen(job);
  }

  /** The look the map is drawn in and the canvas's size, as text: a picture of another can not be shown moved. */
  private lookKey(): string {
    const { look } = this;
    return [look.mode, look.layer, look.ground, this.gpuCv?.width, this.gpuCv?.height].join();
  }

  /** Drop picture texture `t` on `gpu`, and forget it as a picture shown. */
  private dropPicture(gpu: Gpu, t: Texture): void {
    if (this.last?.t === t) this.last = null;
    if (this.fade?.from.some(p => p.t === t)) this.fade = null;
    if (this.movedShown?.some(p => p.t === t)) this.movedShown = null;
    gpu.drop(t);
  }

  /** Show on the screen pictures `pics` (those of this look, in order, each over the last) where the camera puts the
   * squares they hold now, over the map's background; the sharp picture being drawn of the view before is stopped. */
  private showMoved(gpu: Gpu, pics: (Pic | null)[]): void {
    clearTimeout(this.sharpTimer);
    clearTimeout(this.aroundTimer);
    this.aroundRun++;
    this.resting = false;
    this.sharpening = null;
    this.fade = null;
    const look = this.lookKey();
    this.movedShown = pics.filter((p): p is Pic => p?.look === look);
    this.compose(gpu, this.movedShown, 1);
    gpu.submit();
  }

  /** Draw on the screen pictures `pics`, each over the last, where the camera puts the squares they hold now, over
   * the map's background; the last of them covering what is under it by `alpha`. Each fades out towards its edges that
   * lie inside the screen (picture.wgsl), so a picture never ends on a line. */
  private compose(gpu: Gpu, pics: Pic[], alpha: number): void {
    const d = this.drawnDpr(), { s, ox, oy } = this.cam, q = gpu.pass(null, [...this.background(), 1]);
    pics.forEach((p, i) => q.draw('picture', { rect: [(ox + p.x * s) * d, (oy + p.y * s) * d, p.w * s * d, p.h * s * d], alpha: i === pics.length - 1 ? alpha : 1, binds: { 1: p.t, 2: 'smooth' } }, true));
    q.end();
  }

  /** Show the last picture drawn: where `from` is given (what the screen showed before it), it comes in over that in
   * `FADE_MS`, a frame at a time; a fade under way goes on from what it came in over. */
  private present(gpu: Gpu, from: Pic[] | null): void {
    if (from?.length && !this.fade) this.fade = { from, at: performance.now() };
    cancelAnimationFrame(this.fadeFrame);
    const paint = () => {
      const look = this.lookKey(), last = this.last;
      if (!last || last.look !== look) return;
      const f = this.fade, a = f ? Math.min(1, (performance.now() - f.at) / FADE_MS) : 1;
      if (a >= 1) this.fade = null;
      this.compose(gpu, f && a < 1 ? [...f.from.filter(p => p.look === look), last] : [last], a * a * (3 - 2 * a));
      if (this.fade) this.fadeFrame = requestAnimationFrame(() => { paint(); gpu.submit(); });
    };
    paint();
  }

  /** The tiles (x, y, w, h px) a picture of `w` x `h` px at the screen's pixels is drawn in, each of about
   * `SHARPEN_MS` on the GPU (by the time a whole frame at the screen's pixels took) and no less than `TILE_LEAST` px a
   * side: so no draw holds the card long. */
  private tilesOf(w: number, h: number): number[][] {
    const side = Math.max(TILE_LEAST, Math.floor(Math.sqrt((SHARPEN_MS / Math.max(this.fullMs, SHARPEN_MS)) * w * h))), tiles: number[][] = [];
    for (let y = 0; y < h; y += side) for (let x = 0; x < w; x += side) tiles.push([x, y, Math.min(side, w - x), Math.min(side, h - y)]);
    return tiles;
  }

  /** Once the view has rested `AROUND_REST_MS`, draw the pictures shown under the last as the view moves, a tile each
   * time the card is free (`drawAround`). A move or a new frame meanwhile puts them off. */
  private aroundSoon(): void {
    clearTimeout(this.aroundTimer);
    const run = ++this.aroundRun;
    this.aroundTimer = window.setTimeout(() => this.drawAround(run, 0, null, true), AROUND_REST_MS);
  }

  /** Draw the next tile (`tilesOf`) of picture `i` of those shown under the last as the view moves, then go on once the
   * card is free: the area about the view `RINGS[i]` times its width at the screen's pixels, the nearest first, then the
   * whole map where its picture is of another look or older than `WHOLE_EVERY_MS`. `tiles` are those of picture `i`
   * still to draw (null: none drawn yet), `ok` whether all drawn so far had their programs. Nothing where the view moved
   * or a sharp picture is being drawn since run `run` began. */
  private drawAround(run: number, i: number, tiles: number[][] | null, ok: boolean): void {
    const gpu = this.gpu, now = performance.now();
    if (run !== this.aroundRun || !gpu || this.sharpening || now - this.movedAt < AROUND_REST_MS) return;
    if (gpu.busy) { gpu.idle().then(() => this.drawAround(run, i, tiles, ok)); return; }
    const look = this.lookKey();
    if (i < RINGS.length) {
      const W = this.gpuCv!.width, H = this.gpuCv!.height, d = this.drawnDpr(), n = RINGS[i];
      // (Drawn into a spare texture: the picture it replaces is shown till it is whole.)
      if (!tiles) { this.ringSpare = this.sized(gpu, this.ringSpare, W, H); tiles = this.tilesOf(W, H); }
      const t = this.ringSpare!;
      // About the view's middle, `n` times as many squares across.
      const { s, ox, oy } = this.cam, cw = this.cv.clientWidth / 2, ch = this.cv.clientHeight / 2;
      const cam = { s: s / n, ox: cw - (cw - ox) / n, oy: ch - (ch - oy) / n };
      ok = this.drawInto(gpu, t, cam, d, tiles.shift()) && ok;
      if (!tiles.length && ok) { this.rings[i] = picOf(t, cam, d, look); [this.ringTex[i], this.ringSpare] = [t, this.ringTex[i] ?? null]; }
    } else if (this.whole?.look !== look || now - this.wholeAt > WHOLE_EVERY_MS) {
      const wide = Math.max(this.w, this.h), ww = Math.round((WHOLE_PX * this.w) / wide), wh = Math.round((WHOLE_PX * this.h) / wide);
      this.wholeTex = this.sized(gpu, this.wholeTex, ww, wh);
      const all = { s: WHOLE_PX / wide, ox: 0, oy: 0 };
      if (this.drawInto(gpu, this.wholeTex, all, 1)) { this.whole = picOf(this.wholeTex, all, 1, look); this.wholeAt = now; }
    }
    gpu.submit();
    if (i >= RINGS.length) return;
    const more = tiles!.length > 0;
    gpu.idle().then(() => nextFrame(() => this.drawAround(run, more ? i : i + 1, more ? tiles : null, more ? ok : true)));
  }

  /** Texture `t`, or a new one where it is not `w` x `h`, for a picture. */
  private sized(gpu: Gpu, t: Texture | null, w: number, h: number): Texture {
    if (t?.w === w && t.h === h) return t;
    if (t) this.dropPicture(gpu, t);
    return gpu.texture('picture', w, h, MAP_TARGET, 1, 1, true);
  }

  /** Draw the map and the layer over it (the plants, seen at its own pixels) into texture `t` with camera `cam`, `k` of
   * its pixels a css pixel, inside `clip` (x, y, w, h px) where given; false where a program it needs is not made yet
   * (then it is drawn without it). The layer is decided for this draw; the next frame decides it for the view again. */
  private drawInto(gpu: Gpu, t: Texture, cam: { s: number; ox: number; oy: number }, k: number, clip?: number[]): boolean {
    const was = this.cam, drawn = this.land?.drawn, waiting: string[] = [];
    this.cam = { ...cam };
    this.layer?.prepare(gpu, cam.ox, cam.oy, cam.s, k, k, t.w / k, t.h / k, waiting);
    const p = gpu.pass(t, [...this.background(), 1], clip);
    this.drawMap(gpu, p, [cam.ox], k, k, [t.w, t.h], waiting);
    this.layer?.draw(p);
    p.end();
    this.cam = was;
    if (this.land && drawn !== undefined) this.land.drawn = drawn;
    return !waiting.length;
  }

  /** The map's background colour (css `--map-bg`), 0..1. */
  private background(): number[] {
    this.ctx.fillStyle = css('--map-bg');
    return (this.ctx.fillStyle as string).match(/[0-9a-f]{2}/gi)?.map(c => parseInt(c, 16) / 255) ?? [1, 1, 1];
  }

  /** Draw the next tile of the sharp picture `job` once the GPU has finished the last, and show the picture once all
   * are; a new frame stops it. */
  private sharpen(job: Sharpening): void {
    const gpu = this.gpu, tile = job.tiles.shift(), W = this.gpuCv!.width, H = this.gpuCv!.height;
    if (this.sharpening !== job || !gpu || !tile || job.into.w !== W || job.into.h !== H) return;
    const p = gpu.pass(job.into, undefined, tile);
    this.drawMap(gpu, p, job.xs, job.d, job.d, [W, H], []);
    this.layer?.draw(p);
    this.drawFire(gpu, p, job.xs, job.d, job.d, [W, H], []);
    p.end();
    if (!job.tiles.length) {
      this.sharpening = null; this.sharpOf = job.key;
      [this.sharp, this.sharpNext] = [this.sharpNext, this.sharp];
      // (Sharp over the softer picture of the view it replaces, which it comes in over.)
      const before = this.last;
      this.last = picOf(this.sharp!, job.cam, job.d, this.lookKey());
      this.present(gpu, before && before.t !== this.sharpNext ? [before] : null);
      this.aroundSoon();
    }
    gpu.submit();
    // A frame that came due meanwhile is drawn now, over this picture.
    if (!job.tiles.length && this.gpuDue) this.redraw();
    // The time the whole picture took the GPU is that of a frame at the screen's pixels: the next one's tiles are
    // sized by it (`sharpenSoon`), as the frames drawn whole are timed (`time`).
    const t0 = performance.now(), last = !job.tiles.length;
    gpu.done().then(() => {
      job.ms += performance.now() - t0;
      if (last) this.fullMs = job.ms; else nextFrame(() => this.sharpen(job));
    });
  }

  /** Draw into pass `p` (of `size` px, `k` px a css px, on a screen of `d` px a css px) the
   * background and the map's shadow, and the live map or the map's squares (a copy at each of `xs`, css px); the live
   * map only once a program for it is made (`seenFrom`, else it adds the one it waits for to `waiting`). */
  private drawMap(gpu: Gpu, p: Pass, xs: number[], k: number, d: number, size: number[], waiting: string[]): void {
    const { cam: { s, ox, oy }, w, h, look } = this;
    const bg = this.background();
    const backdrop = new Uniforms(3).vec4(...bg).vec4(ox * k, oy * k, xs.length * w * s * k, h * s * k).vec4(SHADOW_DROP * k, SHADOW_BLUR * k, SHADOW_DARK);
    p.draw('backdrop', { rect: [0, 0, size[0], size[1]], binds: { 1: backdrop.buffer } }, false);
    // Copies meet on whole pixels, so no thin line shows between them.
    const edge = (c: number) => Math.round((xs[0] + c * w * s) * k);
    if (this.land) this.land.drawn = d;
    // The live map's program made for the screen's pixels (gpu.ts `seenFrom`): its detail is chosen by them.
    const mpp = this.cellM / (s * k), seen = this.cellM / (s * d), under = this.layer?.under(gpu, s, d);
    const program = this.land ? seenFrom(gpu, 'land', seen, waiting) : null;
    // Each copy is drawn over the part of the target it covers, so what a pixel reads of its place (`frameOf`) never
    // runs over more than the target's pixels.
    const frames = xs.map((_, c) => frameOf(edge(c), oy * k, mpp, seen));
    const lands = frames.map(f => this.land?.binds(gpu, look, f));
    if (lands[0] && under) {
      if (program) xs.forEach((_, c) => {
        const x0 = Math.max(edge(c), 0), x1 = Math.min(edge(c + 1), size[0]), y0 = Math.max(oy * k, 0), y1 = Math.min((oy + h * s) * k, size[1]);
        if (x1 <= x0 || y1 <= y0) return;
        // With the marks of fire on the ground (none in a layer's view, or where no fire is known).
        const fire = this.fire?.groundBinds(gpu, look, frames[c], this.fireTime());
        p.draw(program, { rect: [x0, y0, x1 - x0, y1 - y0], binds: { ...lands[c]!, ...under, ...fire } }, false);
      });
    } else if (this.map) xs.forEach((_, c) => p.draw('earth', { rect: [edge(c), oy * k, edge(c + 1) - edge(c), h * s * k], binds: this.map!.binds(gpu, look) }, false));
  }

  /** Draw into pass `p` (as `drawMap`) over the plants the flames and the smoke of the fires burning in view, over the
   * squares they reach, once their program is made; none in a layer's view. */
  private drawFire(gpu: Gpu, p: Pass, xs: number[], k: number, d: number, size: number[], waiting: string[]): void {
    if (!this.fire || this.look.mode === 'layer') return;
    this.drawOver(gpu, p, 'flames', this.fire.flames, xs, k, d, size, waiting);
  }

  /** Draw into pass `p` (as `drawMap`) program `name` over the map's squares in `box` (null: none), once it is made. */
  private drawOver(gpu: Gpu, p: Pass, name: string, box: Box | null, xs: number[], k: number, d: number, size: number[], waiting: string[]): void {
    const { cam: { s, oy }, w, h, look } = this;
    if (!box || !this.land) return;
    const mpp = this.cellM / (s * k), seen = this.cellM / (s * d);
    const program = seenFrom(gpu, name, seen, waiting);
    if (!program) return;
    const edge = (c: number) => Math.round((xs[0] + c * w * s) * k);
    xs.forEach((_, c) => {
      const map: Rect = [Math.max(edge(c), 0), Math.max(oy * k, 0), Math.min(edge(c + 1), size[0]), Math.min((oy + h * s) * k, size[1])];
      const r = clip(boxRect(box, edge(c), oy * k, s * k), map);
      const binds = r && this.fire!.binds(gpu, look, frameOf(edge(c), oy * k, mpp, seen), this.fireTime()), ground = this.land!.lawBinds(gpu);
      if (r && binds && ground) p.draw(program, { rect: [r[0], r[1], r[2] - r[0], r[3] - r[1]], binds: { ...ground, ...binds } }, true);
    });
  }

  /** The time the flames stand at, ms: while one picture is drawn in tiles, the one it began at (as the flow's signs). */
  private fireTime(): number { return this.land?.flowAt ?? performance.now(); }

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
      spend('gpu', ms); spend('gpu n', 1);
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
      this.classes = gpu.texture('classes', this.w, this.h, 'rgba8unorm', 1, 1, true);
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
    const px = (m / cellM) * s, x = this.covered[3] + SCALE_MARGIN_PX, y = this.cv.clientHeight - this.covered[2] - SCALE_MARGIN_PX;
    ctx.strokeStyle = ctx.fillStyle = css('--ink'); ctx.lineWidth = 1.5; ctx.globalAlpha = 0.75;
    ctx.beginPath(); ctx.moveTo(x, y - 5); ctx.lineTo(x, y); ctx.lineTo(x + px, y); ctx.lineTo(x + px, y - 5); ctx.stroke();
    ctx.font = '11px system-ui, sans-serif'; ctx.textBaseline = 'bottom';
    ctx.fillText(m >= 1000 ? `${m / 1000} km` : `${m} m`, x, y - 7);
    ctx.globalAlpha = 1;
  }
}

/** The longest wait for an animation frame, ms (`nextFrame`). */
const FRAME_MOST_MS = 500;
/** Run `f` at the next animation frame, or after `FRAME_MOST_MS` where none comes (a browser holds them back in a
 * frame it deems out of view or idle, as Safari does in an iframe). */
function nextFrame(f: () => void): void {
  const go = () => { clearTimeout(timer); cancelAnimationFrame(frame); f(); };
  const timer = setTimeout(go, FRAME_MOST_MS), frame = requestAnimationFrame(go);
}

/** A rectangle of the target, px: x0, y0, x1, y1. */
type Rect = [number, number, number, number];

/** The rectangle of the target box `b` of squares covers, where the map's corner is at (`left`, `top`) px and a square
 * takes `side` px, out to whole pixels. */
function boxRect(b: Box, left: number, top: number, side: number): Rect {
  return [Math.floor(left + b[0] * side), Math.floor(top + b[1] * side), Math.ceil(left + b[2] * side), Math.ceil(top + b[3] * side)];
}

/** Rectangle `r` inside `within`; null where none of it is. */
function clip(r: Rect, within: Rect): Rect | null {
  const out: Rect = [Math.max(r[0], within[0]), Math.max(r[1], within[1]), Math.min(r[2], within[2]), Math.min(r[3], within[3])];
  return out[2] > out[0] && out[3] > out[1] ? out : null;
}
