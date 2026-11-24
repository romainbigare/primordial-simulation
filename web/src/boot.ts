// The loading screen: over the whole page (the map, the bar and every panel) till the map first stands whole, with the
// steps of the loading, how far the one under way is, and a picture: a slab of ground seen at a slant, its tiles
// dropping into place one by one as the loading goes on. They are bare earth till the world is made, then the world's
// own ground and water, and its plants once the map draws them. How far each step is comes from the page (main.ts):
// bytes come, tiles made, the picture drawn; a step that says nothing of its own progress (the core making the world)
// moves by the time it is expected to take, and never ends before it does.
import { World, decode } from './data';
import { mapFields } from './earth';
import { toScreen } from './style';

/** A step of the loading: its words and its weight, its share of the whole bar. */
export interface BootStep { label: string; weight: number }

/** Tiles of the slab along its longer side. */
const SLAB_TILES = 8;
/** Texels of the slab's picture along its longer side. */
const SLAB_TEXELS = 256;
/** A tile drops into place over this long, ms, from this high over the slab, as a share of the slab's side. */
const SLAB_DROP_MS = 520, SLAB_DROP_FROM = 0.35;
/** The world's ground and its plants fade in over the bare earth over this long, ms. */
const SLAB_PAINT_MS = 900;
/** The screen fades out over this long, ms. */
const BOOT_LIFT_MS = 600;
/** The bar shown moves towards the progress at this rate, a share of the distance a second (so it never jumps). */
const BOOT_EASE_RATE = 4;
/** A step that says nothing of its progress (`wait`) shows no more than this share of it before it ends. */
const BOOT_WAIT_MOST = 0.92;
/** The slab bobs this high, as a share of its side, once every `SLAB_BOB_MS`. */
const SLAB_BOB = 0.012, SLAB_BOB_MS = 4200;
/** The plants drawn on the slab: dots of this many texels across at most, where a square's leaves let through least. */
const SLAB_PLANT_DOT = 2.2;
/** The slab's ground: liquid this deep, m, is drawn darker by a factor e; ice from this thick, m, is drawn white; a
 * slope's light is 1 and its rise towards the south-east (so it faces the north-west) times this, kept between these;
 * its grain changes the light by this share at most. */
const SLAB_DEEP_M = 40, SLAB_ICE_M = 0.01, SLAB_RELIEF = 2.5, SLAB_LIT = [0.7, 1.25], SLAB_GRAIN = 0.16;
/** The light the slab's ground is shown in, a share of the sun's in red, green and blue: less than the map's, and a little
 * warm, as the map's ground looks with the photographs of its ground over it. */
const SLAB_EXPOSURE = [0.62, 0.56, 0.46];
/** The colours of deep liquid and of shallow, as the map draws its sea (bytes). */
const SLAB_DEEP = [24, 60, 112], SLAB_SHALLOW = [60, 130, 182];
/** The thickness of a tile, as a share of a tile's side. */
const SLAB_TILE_DEPTH = 0.22;
/** The slab seen at a slant: a step along its rows or columns goes this share of a step across the screen down it. */
const SLAB_SLANT = 0.54;
/** The slab's side: at most these shares of the screen's width and height, and this many css px; its middle this
 * share of the screen's height from its top (the words lie below it). */
const SLAB_FIT = [0.9, 0.9], SLAB_MOST_PX = 680, SLAB_MIDDLE = 0.42;
/** Its shadow lies this share of its side below it, blurred over this many css px. */
const SLAB_SHADOW = 0.06, SLAB_SHADOW_PX = 18;
/** The tiles are laid from the middle out, each moved by up to this many tiles at random. */
const SLAB_SCATTER = 1.2;

/** Whether the person asked for less motion: tiles fade in where they stand, and the slab stays still. */
const BOOT_STILL = matchMedia('(prefers-reduced-motion: reduce)');

/** A tile of the slab: its column and row, and when it was laid (ms; NaN: not yet). */
interface Tile { x: number; y: number; at: number }

export class Boot {
  /** Once the screen has lifted. */
  readonly lifted: Promise<void>;
  /** Whether the screen is up. */
  up = true;
  private lift!: () => void;
  private ctx: CanvasRenderingContext2D;
  private steps: [string, BootStep][] = [];
  /** The step under way, its share done, and when it began to wait (`wait`; NaN: it says its own progress). */
  private now = 0;
  private share = 0;
  private waitFrom = NaN;
  private waitMs = 1;
  private waitMost = BOOT_WAIT_MOST;
  private shown = 0;
  private tiles: Tile[] = [];
  private across = SLAB_TILES;
  private down = SLAB_TILES;
  /** The slab's picture: bare earth, the world's ground and its plants (each a canvas `SLAB_TEXELS` along the longer
   * side), what is drawn of them now, and when the world and the plants came (ms; NaN: not yet). */
  private earth: HTMLCanvasElement;
  private ground: HTMLCanvasElement | null = null;
  private plants: HTMLCanvasElement | null = null;
  private top: HTMLCanvasElement;
  private groundAt = NaN;
  private plantsAt = NaN;
  private last = performance.now();
  private endAt = NaN;

  /** `root`: the screen's element, holding a canvas and the text (index.html `#boot`); `watch`: read at each frame,
   * to tell the steps whose progress the page reads. */
  constructor(private root: HTMLElement, private watch: () => void) {
    this.ctx = root.querySelector('canvas')!.getContext('2d')!;
    this.lifted = new Promise(go => (this.lift = go));
    this.earth = this.slabCanvas();
    this.top = this.slabCanvas();
    this.paintEarth();
    this.layTiles();
    requestAnimationFrame(t => this.draw(t));
  }

  /** The steps of this loading, by key, in their order. */
  plan(steps: [string, BootStep][]): void {
    this.steps = steps;
    this.root.querySelector('.dots')!.innerHTML = steps.map(() => '<b><i></i></b>').join('');
    this.now = 0; this.share = 0; this.waitFrom = NaN;
  }

  /** Whether this loading has step `key`. */
  has(key: string): boolean { return this.steps.some(([k]) => k === key); }

  /** Step `key` is under way, `share` of it done (0..1), and `note` says how far (the steps before it are done). */
  at(key: string, share: number, note = ''): void {
    const i = this.steps.findIndex(([k]) => k === key);
    if (i < this.now) return;
    if (i > this.now) this.share = 0;
    else this.share = this.stepShare(performance.now());
    this.now = i;
    this.share = Math.max(this.share, Math.min(share, 1));
    this.waitFrom = NaN;
    this.say(note);
  }

  /** Step `key` is under way and says nothing of its progress: expected to take `ms`, and to reach no more than `most`
   * of the step before it says more (`at`) or the next begins. */
  wait(key: string, ms: number, note = '', most = BOOT_WAIT_MOST): void {
    const i = this.steps.findIndex(([k]) => k === key);
    if (i < this.now) return;
    if (i === this.now && !Number.isNaN(this.waitFrom)) return this.say(note);
    if (i > this.now) this.share = 0;
    this.now = i;
    this.waitFrom = performance.now();
    this.waitMs = ms;
    this.waitMost = most;
    this.say(note);
  }

  /** The world is made: the slab shows its ground, shaped as the map (`across` by `down` squares). */
  world(w: World): void {
    const { w: W, h: H } = w.meta;
    this.across = W >= H ? SLAB_TILES : Math.max(1, Math.round((SLAB_TILES * W) / H));
    this.down = H >= W ? SLAB_TILES : Math.max(1, Math.round((SLAB_TILES * H) / W));
    const laid = this.tiles.filter(t => !Number.isNaN(t.at)).length;
    this.layTiles();
    // (The tiles laid stay laid on the new slab, as many as before.)
    this.tiles.slice(0, laid).forEach(t => (t.at = -Infinity));
    [this.earth, this.top].forEach(c => this.size(c, W, H));
    this.paintEarth();
    this.ground = this.paintGround(w);
    this.plants = this.paintPlants(w);
    this.groundAt = performance.now();
  }

  /** The map draws the world's plants: they show on the slab. */
  plantsDrawn(): void { if (Number.isNaN(this.plantsAt)) this.plantsAt = performance.now(); }

  /** All the steps are done: the bar fills, and the screen fades out and lets the page be seen. `now`: at once. */
  end(now = false): void {
    if (!this.up || !Number.isNaN(this.endAt)) return;
    this.now = this.steps.length; this.share = 0;
    this.endAt = performance.now() + (now ? 0 : SLAB_DROP_MS);
  }

  private say(note: string): void {
    const step = this.steps[this.now]?.[1];
    if (!step) return;
    this.root.querySelector('.now')!.textContent = step.label;
    this.root.querySelector('.note')!.textContent = note ? ` · ${note}` : '';
  }

  /** The share of the loading done: the steps before the one under way, and its share of it. */
  private progress(now: number): number {
    const total = this.steps.reduce((s, [, st]) => s + st.weight, 0) || 1;
    let done = 0;
    this.steps.forEach(([, st], i) => { if (i < this.now) done += st.weight; });
    return Math.min(1, (done + (this.steps[this.now]?.[1].weight ?? 0) * this.stepShare(now)) / total);
  }

  /** The share of the step under way done at `now`: as it said, or as far as its wait has gone. */
  private stepShare(now: number): number {
    if (Number.isNaN(this.waitFrom)) return this.share;
    return Math.max(this.share, this.waitMost * (1 - Math.exp(-(now - this.waitFrom) / this.waitMs)));
  }

  private draw(now: number): void {
    const dt = Math.min((now - this.last) / 1000, 0.1);
    this.last = now;
    if (Number.isNaN(this.endAt)) this.watch();
    this.shown += (this.progress(now) - this.shown) * Math.min(1, dt * BOOT_EASE_RATE);
    if (!Number.isNaN(this.endAt)) this.shown = Math.max(this.shown, Math.min(1, this.shown + dt * 2));
    // As many tiles laid as the share of the loading done.
    const want = Math.floor(this.shown * this.tiles.length + 1e-6);
    this.tiles.forEach((t, k) => { if (k < want && Number.isNaN(t.at)) t.at = now; });
    this.text();
    this.paint(now);
    if (!Number.isNaN(this.endAt) && now > this.endAt) {
      const a = (now - this.endAt) / BOOT_LIFT_MS;
      this.root.style.opacity = String(Math.max(0, 1 - a));
      if (a >= 1) { this.root.hidden = true; this.up = false; this.lift(); return; }
    }
    requestAnimationFrame(t => this.draw(t));
  }

  /** The percentage, the step's words and a dash for each step, filled as far as it is done. */
  private text(): void {
    const total = this.steps.reduce((s, [, st]) => s + st.weight, 0) || 1;
    this.root.querySelector('.pct')!.textContent = `${Math.floor(this.shown * 100)}%`;
    let from = 0;
    this.root.querySelectorAll<HTMLElement>('.dots i').forEach((dash, i) => {
      const w = this.steps[i][1].weight / total;
      dash.style.width = `${Math.min(1, Math.max(0, (this.shown - from) / w)) * 100}%`;
      from += w;
    });
  }

  /** The slab at time `now`: its shadow, the grid where tiles will lie, and the tiles laid, back to front. */
  private paint(now: number): void {
    const cv = this.ctx.canvas, d = Math.min(devicePixelRatio || 1, 2), W = cv.clientWidth, H = cv.clientHeight;
    if (cv.width !== Math.round(W * d) || cv.height !== Math.round(H * d)) { cv.width = Math.round(W * d); cv.height = Math.round(H * d); }
    const x = this.ctx;
    x.setTransform(d, 0, 0, d, 0, 0);
    x.clearRect(0, 0, W, H);
    this.compose(now);
    const n = Math.max(this.across, this.down);
    // The slab's side: as wide as the screen lets it, above the text.
    const side = Math.min(W * SLAB_FIT[0], H * SLAB_FIT[1], SLAB_MOST_PX), cx = W / 2, rx = side / 2 / n, ry = rx * SLAB_SLANT;
    const bob = BOOT_STILL.matches ? 0 : Math.sin((now / SLAB_BOB_MS) * 2 * Math.PI) * SLAB_BOB * side;
    const cy = H * SLAB_MIDDLE - (ry * (this.across + this.down)) / 2 + bob;
    const iso = (u: number, v: number, z: number): [number, number] => [cx + (u - v) * rx, cy + (u + v) * ry - z];
    const quad = (pts: [number, number][]) => { x.beginPath(); pts.forEach(([a, b], k) => (k ? x.lineTo(a, b) : x.moveTo(a, b))); x.closePath(); };
    // The shadow, and the grid of the tiles to come.
    x.save();
    x.filter = `blur(${SLAB_SHADOW_PX}px)`;
    x.fillStyle = 'rgba(0,0,0,.4)';
    const below = -side * SLAB_SHADOW - bob;
    quad([iso(0, 0, below), iso(this.across, 0, below), iso(this.across, this.down, below), iso(0, this.down, below)]);
    x.fill();
    x.restore();
    x.strokeStyle = 'rgba(111,178,227,.16)';
    x.lineWidth = 1;
    for (const t of this.tiles) if (Number.isNaN(t.at)) { quad([iso(t.x, t.y, 0), iso(t.x + 1, t.y, 0), iso(t.x + 1, t.y + 1, 0), iso(t.x, t.y + 1, 0)]); x.stroke(); }
    // The tiles laid, back to front, each dropping from above as it comes.
    const depth = (side / n) * SLAB_TILE_DEPTH, tw = this.top.width / this.across, th = this.top.height / this.down;
    const laid = this.tiles.filter(t => !Number.isNaN(t.at)).sort((a, b) => a.x + a.y - (b.x + b.y));
    for (const t of laid) {
      const a = Math.min(1, (now - t.at) / SLAB_DROP_MS), ease = 1 - (1 - a) ** 3;
      const lift = BOOT_STILL.matches ? 0 : (1 - ease) * SLAB_DROP_FROM * side;
      x.globalAlpha = BOOT_STILL.matches ? a : Math.min(1, a * 2);
      const p = (u: number, v: number, z: number) => iso(t.x + u, t.y + v, z + lift);
      x.fillStyle = '#3b2f25';
      quad([p(0, 1, depth), p(1, 1, depth), p(1, 1, 0), p(0, 1, 0)]); x.fill();
      x.fillStyle = '#2b221b';
      quad([p(1, 0, depth), p(1, 1, depth), p(1, 1, 0), p(1, 0, 0)]); x.fill();
      // The top: the tile's part of the slab's picture, set on the slant.
      const [ax, ay] = p(0, 0, depth);
      x.save();
      x.transform(rx / tw, ry / tw, -rx / th, ry / th, ax, ay);
      x.drawImage(this.top, t.x * tw, t.y * th, tw, th, 0, 0, tw, th);
      x.restore();
      // A tile just laid flashes.
      if (a < 1) { x.fillStyle = `rgba(255,255,255,${0.35 * (1 - a)})`; quad([p(0, 0, depth), p(1, 0, depth), p(1, 1, depth), p(0, 1, depth)]); x.fill(); }
    }
    x.globalAlpha = 1;
  }

  /** The slab's picture now: the bare earth, the world's ground fading in over it, and its plants over that. */
  private compose(now: number): void {
    const t = this.top.getContext('2d')!;
    t.globalAlpha = 1;
    t.drawImage(this.earth, 0, 0);
    if (this.ground) { t.globalAlpha = Math.min(1, (now - this.groundAt) / SLAB_PAINT_MS); t.drawImage(this.ground, 0, 0); }
    if (this.plants && !Number.isNaN(this.plantsAt)) { t.globalAlpha = Math.min(1, (now - this.plantsAt) / SLAB_PAINT_MS); t.drawImage(this.plants, 0, 0); }
    t.globalAlpha = 1;
  }

  /** The tiles of the slab, in the order they are laid: from the middle out, a little at random. */
  private layTiles(): void {
    const mx = (this.across - 1) / 2, my = (this.down - 1) / 2, order: (Tile & { far: number })[] = [];
    for (let y = 0; y < this.down; y++) for (let x = 0; x < this.across; x++) order.push({ x, y, at: NaN, far: Math.hypot(x - mx, y - my) + slabHash(x, y) * SLAB_SCATTER });
    this.tiles = order.sort((a, b) => a.far - b.far).map(({ x, y, at }) => ({ x, y, at }));
  }

  private slabCanvas(): HTMLCanvasElement { const c = document.createElement('canvas'); this.size(c, 1, 1); return c; }
  /** Size canvas `c` to a map of `w` by `h` squares, `SLAB_TEXELS` along its longer side. */
  private size(c: HTMLCanvasElement, w: number, h: number): void {
    c.width = Math.max(1, Math.round((SLAB_TEXELS * w) / Math.max(w, h)));
    c.height = Math.max(1, Math.round((SLAB_TEXELS * h) / Math.max(w, h)));
  }

  /** Bare earth: brown, speckled. */
  private paintEarth(): void {
    const c = this.earth, img = new ImageData(c.width, c.height);
    for (let i = 0; i < c.width * c.height; i++) {
      const k = 0.8 + 0.3 * slabHash(i % c.width, (i / c.width) | 0);
      img.data.set([92 * k, 74 * k, 58 * k, 255], 4 * i);
    }
    c.getContext('2d')!.putImageData(img, 0, 0);
  }

  /** The world's ground in its first frame, the map's numbers of its squares (earth.ts `mapFields`) read between
   * their middles at each texel: its reflectance lit from the north-west by its slope, grained, and shown as the map shows
   * light (style.ts `toScreen`); its liquid blue where the
   * liquid's surface stands over the ground (deeper, darker); its ice white. */
  private paintGround(w: World): HTMLCanvasElement {
    const m = w.meta, c = this.slabCanvas(), { height, matter, ground } = mapFields(w, 0).fields;
    this.size(c, m.w, m.h);
    // Number `k` of the four a square holds in `f`, read between the squares' middles at (x, y), in squares.
    const read = (f: ArrayLike<number>, k: number, x: number, y: number) => {
      const x0 = Math.min(m.w - 1, Math.max(0, Math.floor(x))), y0 = Math.min(m.h - 1, Math.max(0, Math.floor(y)));
      const x1 = Math.min(m.w - 1, x0 + 1), y1 = Math.min(m.h - 1, y0 + 1), u = Math.min(1, Math.max(0, x - x0)), v = Math.min(1, Math.max(0, y - y0));
      const at = (sx: number, sy: number) => f[4 * (sy * m.w + sx) + k];
      return (at(x0, y0) * (1 - u) + at(x1, y0) * u) * (1 - v) + (at(x0, y1) * (1 - u) + at(x1, y1) * u) * v;
    };
    const img = new ImageData(c.width, c.height), step = m.w / c.width;
    for (let py = 0; py < c.height; py++) for (let px = 0; px < c.width; px++) {
      const x = (px + 0.5) * step - 0.5, y = (py + 0.5) * step - 0.5, water = read(height, 1, x, y);
      let rgb: number[];
      if (water > 0) { const k = Math.exp(-water / SLAB_DEEP_M); rgb = SLAB_DEEP.map((d, j) => d + (SLAB_SHALLOW[j] - d) * k); }
      else if (read(matter, 0, x, y) >= SLAB_ICE_M) rgb = [226, 232, 238];
      else {
        const fall = read(height, 2, x, y) + read(height, 3, x, y);
        const lit = Math.min(SLAB_LIT[1], Math.max(SLAB_LIT[0], 1 + fall * SLAB_RELIEF)) * (1 - SLAB_GRAIN / 2 + SLAB_GRAIN * slabHash(px, py));
        rgb = [0, 1, 2].map(k => 255 * toScreen(Math.min(1, (read(ground, k, x, y) / 255) * lit * SLAB_EXPOSURE[k])));
      }
      img.data.set([rgb[0], rgb[1], rgb[2], 255], 4 * (py * c.width + px));
    }
    c.getContext('2d')!.putImageData(img, 0, 0);
    return c;
  }

  /** The world's plants in its first frame: green dots, as many on a square as its leaves shade it, darker where they
   * shade it more. */
  private paintPlants(w: World): HTMLCanvasElement {
    const m = w.meta, c = this.slabCanvas();
    this.size(c, m.w, m.h);
    const leaf = decode(w, 0, 'leaf'), x = c.getContext('2d')!, per = (c.width * c.height) / (m.w * m.h);
    for (let i = 0; i < m.w * m.h; i++) {
      const cover = 1 - Math.exp(-leaf[i]);
      if (!(cover > 0.05)) continue;
      const sx = i % m.w, sy = (i / m.w) | 0, dots = Math.ceil(cover * per * 0.6);
      for (let k = 0; k < dots; k++) {
        const u = slabHash(i, k), v = slabHash(k, i + 7);
        x.fillStyle = `rgba(${48 - 20 * cover},${110 - 30 * cover},${46 - 10 * cover},${0.55 + 0.4 * cover})`;
        x.beginPath();
        x.arc(((sx + u) * c.width) / m.w, ((sy + v) * c.height) / m.h, SLAB_PLANT_DOT * (0.5 + 0.5 * slabHash(i + k, 3)), 0, 2 * Math.PI);
        x.fill();
      }
    }
    return c;
  }
}

/** A number in 0..1 that looks random, the same for the same `a` and `b`. */
function slabHash(a: number, b: number): number {
  let h = Math.imul(a | 0, 0x27d4eb2d) ^ Math.imul(b | 0, 0x165667b1);
  h = Math.imul(h ^ (h >>> 15), 0x85ebca6b);
  h ^= h >>> 13;
  return (h >>> 0) / 4294967296;
}
