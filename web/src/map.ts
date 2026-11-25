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
import { detailComing, detailHeld, detailLost, detailSeen, sendDetail } from './detail';
import { spend, timed } from './cost';
import { Camera, Gesture } from './gesture';

const LAT_STEP = 30; // degrees between a globe's lines of latitude
const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

/** The part of the map in view: its window in squares (clipped to the map), the pixels a square takes, and the zoom (1: the map fits). */
export interface View { x: number; y: number; w: number; h: number; s: number; zoom: number }
/** The most pixels a metre takes on screen, and the least share of the fit's that a zoom out takes the map to. */
const MOST_PX_PER_M = 80, LEAST_ZOOM = 0.5;
/** A pointer that goes up having moved less than this, css px, taps the map (picks its square). */
const TAP_PX = 6;
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
 * moved and scaled as the camera did, at each frame (`showMoved`), and nothing heavy is begun on the card: no map drawn
 * anew, no ground's tile nor layer of its detail sent; once the view rests the map is drawn anew. */
const MOVE_REST_MS = 120;
/** Where no picture shown moved covers the map in view at least as sharp as this share of the screen's pixels along
 * each axis (a zoom out past the pictures of the area about the view), the map is drawn anew at it, at once, as the
 * view moves (`quick`): a sixteenth of the pixels, a sixteenth of the time. */
const QUICK_SCALE = 0.25;
/** The pictures of the area about a view at rest (`MapView.rings`) are these many times its width, each at the screen's
 * pixels (but those drawn with the view's detail, `MapView.ringScale`): so a view that moves out meets one at most
 * three times softer than the last, and the whole map's picture only once it is far (none is drawn that is no sharper
 * than it). They are drawn once the view has rested `AROUND_REST_MS`, in the order `AROUND_ORDER`, a tile each time
 * the card is free; the whole map's picture is `WHOLE_PX` px across (22 MB with its mip levels at most), drawn in
 * tiles too after them, again once older than `WHOLE_EVERY_MS` (the world moves on). */
const RINGS = [2, 6, 18, 54], AROUND_REST_MS = 100, WHOLE_PX = 2048, WHOLE_EVERY_MS = 20000;
/** The rings no wider than this many times the view are drawn with the detail of the view's own pixels (`around`):
 * the plants a pan shows in them are those the view shows, only softer, so none comes and goes as the screen passes
 * from one picture to the next. A wider ring takes its detail by its own pixels: at the view's, it would decide the
 * square of its width as many marks. */
const RING_AS_VIEW_MOST = 2;
/** Those rings are drawn at this share of the screen's pixels along each axis by default (`MapView.ringScale`): a
 * quarter of the pixels, so one costs about what a ring with its own detail does; shown under the last picture as a
 * pan moves it, softer only. */
export const RING_AS_VIEW_SCALE = 0.5;
/** The order the pictures about the view are drawn in (`drawAround`), as indices of `RINGS` (its length: the whole
 * map's picture): the nearest ring with its own detail first, which is cheap and covers a short pan soon after the
 * view rests; then the rings with the view's detail, which cost more; then the wider rings and the whole map. */
const AROUND_ORDER = (() => {
  const all = RINGS.map((_, i) => i), own = all.filter(i => RINGS[i] > RING_AS_VIEW_MOST);
  return [own[0], ...all.filter(i => RINGS[i] <= RING_AS_VIEW_MOST), ...own.slice(1), RINGS.length];
})();
/** A picture drawn anew after the view moved, or sharp over the softer picture of its view, comes in over what the
 * screen showed in this long, ms (`present`): it does not jump in. */
const FADE_MS = 220;
/** A picture the map was drawn into: its texture, the square at its top-left corner (x, y, squares), the squares it
 * spans across and down, and the look and canvas it was drawn for (`MapView.lookKey`). */
interface Pic { t: Texture; x: number; y: number; w: number; h: number; look: string }
/** The picture texture `t` holds, drawn with camera `cam` (css px a square, the map's corner, css px) at `k` of its
 * pixels a css pixel. */
function picOf(t: Texture, cam: Camera, k: number, look: string): Pic {
  return { t, x: -cam.ox / cam.s, y: -cam.oy / cam.s, w: t.w / k / cam.s, h: t.h / k / cam.s, look };
}
/** How often a frame drawn without a program not made yet looks whether it is made, ms (`drawWhenMade`). */
const MADE_POLL_MS = 50;

/** Drawn on the GPU over the map by another module, in the map's pass, at every frame: what it works out before the
 * pass, where the map's corner is at (ox, oy) css px, a square takes `s` css px, a css px `d` px of the target and
 * `seen` px of the screen (its detail is chosen by the screen's), and the view is `w` x `h` css px, of which only the
 * part `clip` of the target (x, y, w, h px) is drawn where given, adding to `waiting` the programs it goes without as
 * they are not made yet (`prepare`); its draw in the pass; and what the live map's
 * ground draws of it, with a square taking `s` css px and a css px `seen` px of the screen (`under`, its bindings). */
export interface GpuLayer {
  prepare(gpu: Gpu, ox: number, oy: number, s: number, d: number, seen: number, w: number, h: number, waiting: string[], clip?: number[]): void;
  draw(p: Pass): void;
  under(gpu: Gpu, s: number, seen: number): Draw['binds'];
}

/** The format of the textures the map is drawn into below the screen's pixels and for its sharp picture. */
const MAP_TARGET: Format = 'rgba8unorm';

/** The sharp picture being drawn once the view rests: its tiles to draw (x, y, w, h device px), the copies' places
 * (css px), the device pixels a css pixel, the view it is of (`viewKey`), the texture it is drawn into (out of
 * sight, shown once whole: `MapView.sharpenSoon`), the time its tiles drawn so far took the GPU, ms, and the camera it
 * is drawn with. */
interface Sharpening { tiles: number[][]; xs: number[]; d: number; key: string; into: Texture; ms: number; cam: Camera }
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
  private cam: Camera = { s: 1, ox: 0, oy: 0 };
  /** The pointers' gesture, which sets the camera once a frame (`draw`). */
  private gesture = new Gesture(() => this.cam, c => { this.cam = c; this.fitted = false; this.moved(); this.gpuDue = true; }, s => this.limit(s));
  private fitted = true;
  /** The edges of the map covered by the controls over it, css px (top, right, bottom, left): a fit fills the rest, and
   * the scale bar stands in its bottom left corner. */
  private covered: [number, number, number, number] = [0, 0, 0, 0];
  private w = 0; private h = 0; private cellM = 1;
  /** On a phone the map is drawn out to the screen's edges (past the map, the open sea: picture.wgsl) and a fit frames
   * `focus`, the land (squares: x0, y0, x1, y1; null: the whole map). */
  private fillScreen = false;
  private focus: [number, number, number, number] | null = null;
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
   * which a moving view shows moved (`showMoved`); when the view last moved, ms, and the timer that draws the map anew
   * once the view rests. */
  private last: Pic | null = null;
  /** Pictures shown under the last as the view moves, so it moves over less detail, never over nothing: the area about
   * the view at rest, `RINGS` times its width, and the whole map at `WHOLE_PX` px across; the textures they are in,
   * the whole map's last, and those the next ring and the next whole map's picture are drawn into; the timer that
   * starts drawing them once the view rests, and the count of those starts (a move stops the one under way,
   * `aroundSoon`). */
  private rings: (Pic | null)[] = [];
  private whole: Pic | null = null;
  private aroundTex: (Texture | null)[] = [];
  private spares: (Texture | null)[] = [null, null, null];
  /** The share of the screen's pixels along each axis the rings with the view's detail are drawn at
   * (`RING_AS_VIEW_SCALE`; a test setting of the page, `setRingScale`). */
  private ringScale = RING_AS_VIEW_SCALE;
  private aroundTimer = 0;
  private aroundRun = 0;
  /** What the screen showed that the next picture drawn whole comes in over (`present`): what a moving view showed
   * last (`showMoved`), or the last picture where the next draws what it went without (`fadeNext`); and the pictures
   * the last comes in over and when it began. */
  private fadeFrom: Pic[] | null = null;
  private fade: { from: Pic[]; at: number } | null = null;
  private fadeFrame = 0;
  private wholeAt = -Infinity;
  private movedAt = -Infinity;
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
   * button, still moves the map; while a tool is on (`on`), the right button's menu does not open. */
  paint: { on(): boolean; start(x: number, y: number): boolean; move(x: number, y: number): void; end(): void } | null = null;
  /** The pointer that paints is told where it is on the map anew (the camera moved under it). */
  private painterMoved = () => {};

  constructor(private cv: HTMLCanvasElement, private onHover: (i: number) => void, private onPick: (i: number) => void, private onView: (v: View) => void = () => {}) {
    this.ctx = cv.getContext('2d')!;
    // The GPU's canvas, under the map's canvas.
    const c = document.createElement('canvas');
    c.className = 'gpu';
    cv.before(c);
    this.gpuCv = c;
    this.resize();
    const restored = () => { this.stopPictures(); this.timing = false; this.sharpOf = ''; this.last = this.whole = this.fadeFrom = this.fade = null; this.rings = []; this.land?.tiles.lost(); this.fire?.stop(null); detailLost(); this.invalidate(); };
    this.ready = Promise.resolve(openGpu(c, restored)).then(g => {
      this.gpu = g;
      // A new set of the ground's detail: the pictures begun with the set before (whose bindings name it) stop, and
      // the map is drawn anew with the new one.
      if (g) sendDetail(g, () => { this.stopPictures(); this.sharpOf = ''; this.fadeNext(); this.invalidate(); });
      this.invalidate();
      return g;
    });
    new ResizeObserver(() => { this.resize(); this.invalidate(); }).observe(cv);
    /** The pointer that paints (-1: none), and where it is. */
    let painter = -1, painterAt = { x: 0, y: 0 };
    const at = (e: { offsetX: number; offsetY: number }): [number, number] => [(e.offsetX - this.cam.ox) / this.cam.s, (e.offsetY - this.cam.oy) / this.cam.s];
    this.painterMoved = () => { if (painter >= 0) this.paint?.move(...at({ offsetX: painterAt.x, offsetY: painterAt.y })); };
    const stopPainting = () => { if (painter >= 0) { painter = -1; this.paint?.end(); } };
    cv.addEventListener('pointerdown', e => {
      cv.setPointerCapture(e.pointerId);
      if (painter >= 0) {
        // A second finger: the stroke ends, and the two move the map.
        this.gesture.down(painter, painterAt.x, painterAt.y); stopPainting();
      } else if (!this.gesture.size && e.button === 0 && this.paint?.start(...at(e))) { painter = e.pointerId; painterAt = { x: e.offsetX, y: e.offsetY }; return; }
      this.gesture.down(e.pointerId, e.offsetX, e.offsetY);
    });
    cv.addEventListener('contextmenu', e => { if (this.paint?.on()) e.preventDefault(); });
    cv.addEventListener('pointermove', e => {
      if (e.pointerId === painter) { painterAt = { x: e.offsetX, y: e.offsetY }; return this.paint?.move(...at(e)); }
      if (this.gesture.move(e.pointerId, e.offsetX, e.offsetY)) this.redraw();
      // (While a tool paints the map, the brush shows where it acts: no square is outlined.)
      else this.setHover(this.paint?.on() ? -1 : this.cellAt(e.offsetX, e.offsetY));
    });
    const up = (e: PointerEvent) => {
      if (e.pointerId === painter) return stopPainting();
      if (this.gesture.size === 1 && this.gesture.moved < TAP_PX && e.type === 'pointerup' && !this.paint?.on()) { this.pick = this.cellAt(e.offsetX, e.offsetY); this.onPick(this.pick); this.redraw(); }
      this.gesture.up(e.pointerId);
      this.redraw();
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
    const [top, right, bottom, left] = this.covered, freeW = this.cv.clientWidth - left - right, freeH = this.cv.clientHeight - top - bottom;
    const [x0, y0, x1, y1] = this.fillScreen && this.focus ? this.focus : [0, 0, this.w, this.h];
    const k = this.fillScreen ? Math.min(freeW / (x1 - x0), freeH / (y1 - y0)) || 1 : this.fitScale();
    this.cam = { s: k, ox: left + freeW / 2 - ((x0 + x1) / 2) * k, oy: top + freeH / 2 - ((y0 + y1) / 2) * k };
    this.fitted = true; this.invalidate();
  }

  /** Draw the map out to the screen's edges and fit it to `focus` (squares: x0, y0, x1, y1; null: the whole map), or
   * not (`fill` false: the map on the page, fitted whole). */
  setFill(fill: boolean, focus: [number, number, number, number] | null): void {
    this.fillScreen = fill; this.focus = focus;
    if (this.fitted && this.w) this.fit(); else this.invalidate();
  }

  /** The controls over the map now cover its edges `covered` (css px: top, right, bottom, left); a fitted map fits
   * anew. */
  setCovered(covered: [number, number, number, number]): void {
    if (covered.every((v, k) => v === this.covered[k])) return;
    this.covered = covered;
    if (this.fitted && this.w) this.fit(); else this.redraw();
  }

  /** The place in the middle of the part of the map the controls leave free, squares. */
  sightAt(): [number, number] {
    const [top, right, bottom, left] = this.covered, x = (left + this.cv.clientWidth - right) / 2, y = (top + this.cv.clientHeight - bottom) / 2;
    return [(x - this.cam.ox) / this.cam.s, (y - this.cam.oy) / this.cam.s];
  }

  /** The share of the screen's pixels along each axis the map is drawn at now. */
  get drawnAt(): number { return this.scale; }

  /** Draw the map anew at the next frame, on the GPU and over it. */
  invalidate(): void { this.gpuDue = true; this.redraw(); }
  /** Forget the pictures of the area about the view and of the whole map (they show a fire as it was): they are drawn
   * again once the view rests. */
  forgetAround(): void { this.rings = []; this.whole = null; this.invalidate(); }

  /** Draw the map anew at the next frame though a sharp picture of this same view is being drawn (`sharpen`, which a
   * playing world's frames leave to finish): the player changed the world, and sees it at once, drawn whole (at fewer
   * pixels on a slow card) and sharp once it rests; not sharp again over the last, which a change every tenth of a
   * second, as a held brush makes, would stop before it is done each time. */
  showNow(): void {
    this.sharpening = null; this.resting = false; this.sharpOf = '';
    clearTimeout(this.sharpTimer);
    this.invalidate();
  }

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

  /** The view moved now. */
  private moved(): void { this.movedAt = performance.now(); this.painterMoved(); }

  private setHover(i: number): void { if (i !== this.hover) { this.hover = i; this.onHover(i); this.redraw(); } }

  /** The scale a camera may take nearest `s`, css px a square: from `LEAST_ZOOM` of the fit to `MOST_PX_PER_M`. */
  private limit(s: number): number { return Math.min(Math.max(s, this.fitScale() * LEAST_ZOOM), MOST_PX_PER_M * this.cellM); }

  private zoomAt(px: number, py: number, k: number): void {
    const c = this.cam, s = this.limit(c.s * k);
    c.ox = px - ((px - c.ox) / c.s) * s; c.oy = py - ((py - c.oy) / c.s) * s; c.s = s;
    this.fitted = false; this.moved(); this.invalidate();
  }

  /** Whether one of pictures `pics` of this look covers the map in view (its copies at `xs`, css px) at least as sharp
   * as a frame drawn at `QUICK_SCALE`. */
  private coversView(pics: (Pic | null)[], xs: number[]): boolean {
    const { s, ox, oy } = this.cam, look = this.lookKey(), least = this.drawnDpr() * QUICK_SCALE;
    const x0 = Math.max(0, xs[0]), x1 = Math.min(this.cv.clientWidth, xs[0] + xs.length * this.w * s);
    const y0 = Math.max(0, oy), y1 = Math.min(this.cv.clientHeight, oy + this.h * s);
    return pics.some(p => p !== null && p.look === look && p.t.w / (p.w * s) >= least
      && ox + p.x * s <= x0 + 1 && ox + (p.x + p.w) * s >= x1 - 1 && oy + p.y * s <= y0 + 1 && oy + (p.y + p.h) * s >= y1 - 1);
  }

  /** Hold back what is sent to the card (the ground's tiles, the layers of its detail) while the view moves (`held`),
   * so the moved picture's frames find the card free; once it rests they are sent. */
  private hold(held: boolean): void {
    if (this.land) this.land.tiles.held = held;
    detailHeld(held);
  }

  /** Draw the map: on the GPU if it is due, and what lies over it. */
  private draw(): void {
    if (!this.w) return;
    this.gesture.apply();
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
    const now = performance.now(), moving = now - this.movedAt < MOVE_REST_MS && this.last?.look === this.lookKey();
    this.hold(moving);
    if (held) { /* (stays due) */ } else if (this.gpu && this.gpuDue && moving) {
      // The view moves: the last picture moved with it, over those of the area about it (the widest lowest) and of the
      // whole map, at every frame (a few textured rectangles: it does not wait for the card); where none of them
      // covers the view sharp enough, the map drawn anew at `QUICK_SCALE` once the card is free (a zoom out past them);
      // and the map drawn anew once the view rests.
      const pics = [this.whole, ...[...this.rings].reverse(), this.last];
      if (!this.gpu.busy && !this.coversView(pics, xs)) { this.stopPictures(); this.drawGpu(this.gpu, xs, this.drawnDpr(), true); }
      else this.showMoved(this.gpu, pics);
      clearTimeout(this.moveTimer);
      this.moveTimer = window.setTimeout(() => this.redraw(), this.movedAt + MOVE_REST_MS - now + 1);
    } else if (this.gpu && this.gpuDue && this.gpu.busy) {
      // A frame is sent only once the card has drawn the last, so frames never pile up in its queue; till then the
      // frame stays due and is drawn as soon as the card is free.
      if (!this.waitingCard) { this.waitingCard = true; this.gpu.idle().then(() => { this.waitingCard = false; this.redraw(); }); }
    } else if (this.gpu && this.gpuDue) {
      this.gpuDue = false;
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
      if (i < 0 || this.paint?.on()) continue;
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
   * screen's pixels (`sharpen`). A `quick` frame, drawn as the view moves, is drawn at `QUICK_SCALE`, shown at once and
   * neither timed nor sharpened. */
  private drawGpu(gpu: HeldGpu, xs: number[], d: number, quick = false): void {
    const { cam: { s, ox, oy } } = this, W = this.gpuCv!.width, H = this.gpuCv!.height;
    const r = quick ? QUICK_SCALE : this.scale, k = d * r, tw = Math.max(1, Math.round(W * r)), th = Math.max(1, Math.round(H * r));
    // (Drawn into a texture at the screen's pixels too, so a moving view can show it moved, `showMoved`.)
    [this.picture, this.pictureNext] = [this.pictureNext, this.picture];
    this.picture = this.sized(gpu, this.picture, tw, th);
    const waiting: string[] = [];
    gpu.begin();
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
    gpu.mipmap(target);
    this.last = picOf(target, this.cam, k, this.lookKey());
    this.present(gpu, quick ? null : this.fadeFrom);
    if (!quick) this.fadeFrom = null;
    if (this.asked.length && this.map) this.readClasses(gpu, this.map);
    gpu.submit();
    if (quick) return;
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
      if (waiting.every(name => gpu.ready(name))) { this.fadeNext(); this.redraw(); }
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

  /** Whether the picture is whole: no ground tile in view and no detail still to come, no tile fading in, no frame due,
   * drawn, timed or being made sharp (the loading screen lifts, and the picture tool takes its picture, only then). */
  settled(): boolean {
    const tiles = this.land?.tiles;
    if (tiles && (tiles.toCome() > 0 || tiles.fading(performance.now()))) return false;
    return !detailComing() && !this.gpuDue && !this.raf && !this.timing && !this.resting && !this.sharpening;
  }

  /** Whether a picture was drawn whole with every program it draws with (none still being made). */
  programsMade(): boolean { return this.last !== null && !this.wentWithout; }

  /** Start drawing the map at the screen's own pixels, in tiles each of about `SHARPEN_MS` on the GPU, into a texture
   * out of sight, shown once whole: the screen never shows a picture sharp in part (a tile drawn sharp, then the
   * stretched picture again as the view moves, flickers), and a picture of one date all over. */
  private sharpenSoon(xs: number[], d: number): void {
    const gpu = this.gpu, W = this.gpuCv!.width, H = this.gpuCv!.height, tiles = this.tilesOf(W, H);
    if (!gpu) return;
    if (this.sharp?.w !== W || this.sharp.h !== H) this.sharpOf = '';
    this.sharp = this.sized(gpu, this.sharp, W, H);
    this.sharpNext = this.sized(gpu, this.sharpNext, W, H);
    // All the sharp picture's tiles show the flow's signs where they stood as it began.
    if (this.land) this.land.flowAt = performance.now();
    // What the layer works out before its draws holds for the whole view, so it is worked out once for all the tiles
    // (a new frame stops them); a program it goes without is waited for as a whole frame's is (`drawWhenMade`).
    const { cam: { s, ox, oy } } = this, waiting: string[] = [];
    gpu.begin();
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
    if (this.fadeFrom?.some(p => p.t === t)) this.fadeFrom = null;
    gpu.drop(t);
  }

  /** The next picture drawn whole comes in over the last (it draws what the last went without: the ground's detail, a
   * program), so it does not change at once. */
  private fadeNext(): void { this.fadeFrom ??= this.last ? [this.last] : null; }

  /** Stop the pictures drawn over frames: the sharp picture (and the rest before it) and those about the view. */
  private stopPictures(): void {
    clearTimeout(this.sharpTimer);
    clearTimeout(this.aroundTimer);
    this.aroundRun++;
    this.resting = false;
    this.sharpening = null;
  }

  /** Show on the screen pictures `pics` (those of this look, in order, each over the last) where the camera puts the
   * squares they hold now, over the map's background; the sharp picture being drawn of the view before is stopped. */
  private showMoved(gpu: Gpu, pics: (Pic | null)[]): void {
    this.stopPictures();
    this.fade = null;
    cancelAnimationFrame(this.fadeFrame);
    const look = this.lookKey();
    this.fadeFrom = pics.filter((p): p is Pic => p?.look === look);
    this.compose(gpu, this.fadeFrom, 1);
    gpu.submit();
  }

  /** Draw on the screen pictures `pics`, each over the last, where the camera puts the squares they hold now, over
   * the map's background; the last of them covering what is under it by `alpha`. Each fades out towards its edges that
   * lie inside the screen (picture.wgsl), so a picture never ends on a line. */
  private compose(gpu: Gpu, pics: Pic[], alpha: number): void {
    const d = this.drawnDpr(), { s, ox, oy } = this.cam, q = gpu.pass(null, [...this.background(), 1]);
    pics.forEach((p, i) => {
      const r = [(ox + p.x * s) * d, (oy + p.y * s) * d, p.w * s * d, p.h * s * d], { rect, src, keep } = this.reach(p, r, d);
      q.draw('picture', { rect, src, alpha: i === pics.length - 1 ? alpha : 1, binds: { 1: p.t, 2: 'smooth', 3: keep.buffer } }, true);
    });
    q.end();
  }

  /** Where picture `p`, drawn at `r` (px: x, y, w, h), is drawn on a screen of `d` px a css px: past each of its sides
   * that is the map's edge out to the screen's edge while the map is drawn out to them (a globe's east and west edges
   * wrap), with the part of the source that rectangle reads and the part of the picture that holds the map (uv). */
  private reach(p: Pic, r: number[], d: number): { rect: number[]; src: number[]; keep: Uniforms } {
    if (!this.fillScreen) return { rect: r, src: [0, 0, 1, 1], keep: new Uniforms(1).vec4(0, 0, 1, 1) };
    const W = this.cv.clientWidth * d, H = this.cv.clientHeight * d, ew = this.globe === null;
    const keep = new Uniforms(1).vec4(ew ? Math.max(0, -p.x / p.w) : 0, Math.max(0, -p.y / p.h), ew ? Math.min(1, (this.w - p.x) / p.w) : 1, Math.min(1, (this.h - p.y) / p.h));
    const x0 = ew && p.x <= 0 ? Math.min(r[0], 0) : r[0], y0 = p.y <= 0 ? Math.min(r[1], 0) : r[1];
    const x1 = ew && p.x + p.w >= this.w ? Math.max(r[0] + r[2], W) : r[0] + r[2], y1 = p.y + p.h >= this.h ? Math.max(r[1] + r[3], H) : r[1] + r[3];
    return { rect: [x0, y0, x1 - x0, y1 - y0], src: [(x0 - r[0]) / r[2], (y0 - r[1]) / r[3], (x1 - r[0]) / r[2], (y1 - r[1]) / r[3]], keep };
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

  /** Draw the rings with the view's detail at `r` of the screen's pixels along each axis from now on: those drawn
   * are let go and drawn anew. */
  setRingScale(r: number): void {
    this.ringScale = r;
    RINGS.forEach((n, i) => { if (n <= RING_AS_VIEW_MOST) this.rings[i] = null; });
    this.aroundSoon();
  }

  /** Once the view has rested `AROUND_REST_MS`, draw the pictures shown under the last as the view moves, a tile each
   * time the card is free (`drawAround`). A move or a new frame meanwhile puts them off. */
  private aroundSoon(): void {
    clearTimeout(this.aroundTimer);
    const run = ++this.aroundRun;
    this.aroundTimer = window.setTimeout(() => this.drawAround(run, 0, null, true), AROUND_REST_MS);
  }

  /** Draw the next tile (`tilesOf`) of the picture of step `step` of `AROUND_ORDER`, of those shown under the last as
   * the view moves (`around`), then go on once the card is free, each into a spare texture of its size (the picture it
   * replaces is shown till it is whole). `tiles` are those of the picture still to draw (null: none drawn yet), `ok`
   * whether all drawn so far had their programs. Nothing where the view moved or a sharp picture is being drawn since
   * run `run` began. */
  private drawAround(run: number, step: number, tiles: number[][] | null, ok: boolean): void {
    const gpu = this.gpu, now = performance.now(), i = AROUND_ORDER[step];
    if (run !== this.aroundRun || !gpu || this.sharpening || now - this.movedAt < AROUND_REST_MS || i === undefined) return;
    if (gpu.busy) { gpu.idle().then(() => this.drawAround(run, step, tiles, ok)); return; }
    const a = this.around(i, now);
    if (!a) { this.drawAround(run, step + 1, null, true); return; }
    // (One spare of each size: the rings with the view's detail, the other rings, the whole map.)
    const spare = i === RINGS.length ? 2 : RINGS[i] <= RING_AS_VIEW_MOST ? 0 : 1;
    if (!tiles) { this.spares[spare] = this.sized(gpu, this.spares[spare], a.w, a.h); tiles = this.tilesOf(a.w, a.h); }
    const t = this.spares[spare]!;
    ok = this.drawInto(gpu, t, a.cam, a.k, a.seen, tiles.shift()) && ok;
    if (!tiles.length && ok) {
      gpu.mipmap(t);
      const pic = picOf(t, a.cam, a.k, this.lookKey());
      if (i < RINGS.length) this.rings[i] = pic; else { this.whole = pic; this.wholeAt = now; }
      [this.aroundTex[i], this.spares[spare]] = [t, this.aroundTex[i] ?? null];
    }
    gpu.submit();
    const more = tiles.length > 0;
    gpu.idle().then(() => nextFrame(() => this.drawAround(run, more ? step : step + 1, more ? tiles : null, more ? ok : true)));
  }

  /** Picture `i` of those shown under the last as the view moves, at `now`: its camera, its pixels a css pixel, the
   * pixels of the screen a css pixel its detail is chosen by, and its size, px; null where none is drawn. The area about
   * the view `RINGS[i]` times its width at the screen's pixels (none where the whole map's picture is as sharp: its ring
   * is let go), with the view's own detail at `ringScale` of the screen's pixels up to `RING_AS_VIEW_MOST`,
   * then the whole map where its picture is of another look or older than `WHOLE_EVERY_MS`. */
  private around(i: number, now: number): { cam: Camera; k: number; seen: number; w: number; h: number } | null {
    const wide = Math.max(this.w, this.h), d = this.drawnDpr();
    if (i < RINGS.length) {
      // About the view's middle, `n` times as many squares across.
      const n = RINGS[i], { s, ox, oy } = this.cam, cw = this.cv.clientWidth / 2, ch = this.cv.clientHeight / 2;
      const asView = n <= RING_AS_VIEW_MOST, r = asView ? this.ringScale : 1;
      if ((s / n) * d * r <= WHOLE_PX / wide) { this.rings[i] = null; return null; }
      // (A css pixel of the ring takes n of the view's: at n times the screen's pixels, the ring's metres a pixel of
      // detail are the view's.)
      const seen = asView ? n * d : d, w = Math.round(this.gpuCv!.width * r), h = Math.round(this.gpuCv!.height * r);
      return { cam: { s: s / n, ox: cw - (cw - ox) / n, oy: ch - (ch - oy) / n }, k: (d * w) / this.gpuCv!.width, seen, w, h };
    }
    if (this.whole?.look === this.lookKey() && now - this.wholeAt <= WHOLE_EVERY_MS) return null;
    return { cam: { s: WHOLE_PX / wide, ox: 0, oy: 0 }, k: 1, seen: 1, w: Math.round((WHOLE_PX * this.w) / wide), h: Math.round((WHOLE_PX * this.h) / wide) };
  }

  /** Texture `t`, or a new one where it is not `w` x `h`, for a picture, with its mip levels (a picture shown smaller
   * than it was drawn reads them, `compose`). */
  private sized(gpu: Gpu, t: Texture | null, w: number, h: number): Texture {
    if (t?.w === w && t.h === h) return t;
    if (t) this.dropPicture(gpu, t);
    return gpu.texture('picture', w, h, MAP_TARGET, 1, Math.floor(Math.log2(Math.max(w, h))) + 1, true);
  }

  /** Draw the map and the layer over it into texture `t` with camera `cam`, `k` of its pixels a css pixel, its detail
   * (the ground's and the plants') chosen as on a screen of `seen` px a css pixel, inside `clip` (x, y, w, h px) where
   * given; false where a program it needs is not made yet (then it is drawn without it). The layer is decided for this
   * draw; the next frame decides it for the view again. */
  private drawInto(gpu: HeldGpu, t: Texture, cam: Camera, k: number, seen: number, clip?: number[]): boolean {
    const was = this.cam, drawn = this.land?.drawn, waiting: string[] = [];
    this.cam = { ...cam };
    gpu.begin();
    this.layer?.prepare(gpu, cam.ox, cam.oy, cam.s, k, seen, t.w / k, t.h / k, waiting, clip);
    const p = gpu.pass(t, [...this.background(), 1], clip);
    this.drawMap(gpu, p, [cam.ox], k, seen, [t.w, t.h], waiting);
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
      gpu.mipmap(job.into);
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
    const { cam: { s, oy }, w, h } = this;
    if (!box || !this.land) return;
    const mpp = this.cellM / (s * k), seen = this.cellM / (s * d);
    const program = seenFrom(gpu, name, seen, waiting);
    if (!program) return;
    const edge = (c: number) => Math.round((xs[0] + c * w * s) * k);
    xs.forEach((_, c) => {
      const map: Rect = [Math.max(edge(c), 0), Math.max(oy * k, 0), Math.min(edge(c + 1), size[0]), Math.min((oy + h * s) * k, size[1])];
      const r = clip(boxRect(box, edge(c), oy * k, s * k), map);
      const binds = r && this.fire!.binds(gpu, frameOf(edge(c), oy * k, mpp, seen), this.fireTime()), ground = this.land!.lawBinds(gpu);
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
    // (On the page in the ink's colour; over the map drawn out to the screen's edges, white with a dark halo.)
    ctx.strokeStyle = ctx.fillStyle = this.fillScreen ? '#fff' : css('--ink'); ctx.lineWidth = 1.5; ctx.globalAlpha = this.fillScreen ? 0.95 : 0.75;
    if (this.fillScreen) { ctx.shadowColor = 'rgba(0,0,0,.6)'; ctx.shadowBlur = 3; }
    ctx.beginPath(); ctx.moveTo(x, y - 5); ctx.lineTo(x, y); ctx.lineTo(x + px, y); ctx.lineTo(x + px, y - 5); ctx.stroke();
    ctx.font = '11px system-ui, sans-serif'; ctx.textBaseline = 'bottom';
    ctx.fillText(m >= 1000 ? `${m / 1000} km` : `${m} m`, x, y - 7);
    ctx.globalAlpha = 1; ctx.shadowBlur = 0;
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
