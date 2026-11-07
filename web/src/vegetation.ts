// Vegetation: the plant cover drawn as textures at its real size (handover 10.3, 10.4), not plant by plant. Each place
// reads as one kind of pattern (grass, steppe, tundra, shrubs, woodland, forest, jungle) from its cover's own numbers in
// the frame (its mass, walls and capture area per m², tape/src/layers.rs) and its ground temperature and rain; where
// the view is finer than the cells, from those of each pixel of the fine raster (canopy.ts), over its dry ground. The
// pattern's tile (textures.ts) repeats at the real size of its marks (a tree's crown from the walls per m², a tuft of
// grass), its marks as many as the cover is closed, coloured by the cover (close up, the genes of the heaviest slot of
// the cell, tape::things) and blended between the tiles' centres as the map's colours are. It is drawn once its marks
// take about a pixel, and kept as a picture until the view or the cover changes.
import { World, decode } from './data';
import { CLASSES } from './earth';
import type { CellThings } from './avatars';
import type { FineCover } from './canopy';
import { Rgb, Style, plantColour, plantLookOf, woodiness } from './sprites';
import { Kind, OPEN_SHARE, tileOf, sizeFor } from './textures';

/** The look of a place: its kind of pattern, how closed its cover is (0..1), the size of its marks, m, and its colour. */
interface Stand { kind: Kind; closed: number; mark: number; colour: Rgb }
/** The numbers a pattern reads, per tile of a grid of `w` x `h` (row-major), and whether cover is drawn on it (1: dry
 * open ground). */
interface Field { w: number; h: number; kg: Float32Array; wood: Float32Array; leaf: Float32Array; t: Float32Array; rain: Float32Array; open: Uint8Array }

/** Below this cover share nothing is drawn. */
const BARE = 0.02;
/** The woody share of built cover (walls over walls and matter) from which a cover is drawn as crowns. */
const WOODY = 0.6;
/** Walls per m², kg, from which a woody cover is a woodland of small trees (below, shrubs), and from which a forest; a
 * forest holds about 10 to 50 kg/m² of dry matter (Whittaker and Likens 1975), mostly wood. */
const SHRUB_WOOD = 1, FOREST_WOOD = 3;
/** A forest is a jungle where its ground is warmer than this, K, its rain at least this, mol/m²/s (about 1,100 mm a
 * year), and its cover share at least this. */
const JUNGLE_T = 293, JUNGLE_RAIN = 2e-3, JUNGLE_CLOSED = 0.85;
/** Herbs are steppe where rain is below this, mol/m²/s (about 400 mm a year), and tundra where the ground is colder
 * than this, K, and the cover thinner than `TUNDRA_KG` kg/m². */
const STEPPE_RAIN = 7e-4, TUNDRA_T = 271, TUNDRA_KG = 1;
/** Below this cover share a place is a desert of rare shrubs. */
const DESERT_CLOSED = 0.12;
/** A tree's crown across, m: `CROWN_LOW` plus `CROWN_RISE` × (1 - e^(-walls / `CROWN_WOOD`)), walls in kg/m² (3 m in a
 * young stand to about 13 m in a stand of 30 kg/m²); drawn at the nearest of `CROWN_SIZES`, blended between two. */
const CROWN_LOW = 3, CROWN_RISE = 12, CROWN_WOOD = 15, CROWN_SIZES = [4, 8, 14];
/** The size of the marks of the other kinds, m: a shrub, and a tuft of grass, of steppe and of tundra. */
const MARK_M: Partial<Record<Kind, number>> = { shrubs: 1.5, grass: 0.25, steppe: 0.5, tundra: 0.35 };
/** The pixels a mark takes from which its pattern starts to show, and from which it shows fully. */
const FADE_PX = [0.7, 2];
/** The share of the view's width and height the picture of the textures reaches beyond each edge. */
const COVER_MARGIN = 0.3;
/** The pixels by which the fine raster may miss the view's edge and still be taken to hold it. */
const SLACK_PX = 1;
/** How strongly the textures cover the map, so its hill shading still shows through. */
const COVER_ALPHA = 0.85;
/** The kinds in the order they are drawn: the low under the tall. */
const KIND_ORDER: Kind[] = ['grass', 'steppe', 'tundra', 'shrubs', 'woodland', 'forest', 'jungle'];

/** The map classes on which cover is drawn (the land that is neither water, ice nor rock). */
const VEG_CLASSES = new Set(['Desert', 'Grassland', 'Forest'].map(n => CLASSES.findIndex(c => c[0] === n)));
const vegClamp = (x: number, a = 0, b = 1) => (x < a ? a : x > b ? b : x);
const vegMix = (a: Rgb, b: Rgb, t: number): Rgb => [0, 1, 2].map(k => a[k] + (b[k] - a[k]) * t) as Rgb;
const vegSmooth = (a: number, b: number, x: number) => { const t = vegClamp((x - a) / (b - a)); return t * t * (3 - 2 * t); };

/** The base foliage of each kind (display colours); a slot's genes tint it close up. */
const FOLIAGE: Record<Kind, Rgb> = {
  forest: [60, 104, 62], jungle: [44, 116, 56], woodland: [92, 124, 70], shrubs: [112, 118, 78], grass: [124, 158, 84],
  steppe: [168, 160, 106], tundra: [136, 142, 112],
};

/** The stand of a place: its pattern from its cover (`kg` and `wood` kg/m², `leaf` capture area per m²) and its ground
 * temperature `t` K and rain mol/m²/s; null where there is too little to draw. */
function stand(kg: number, wood: number, leaf: number, t: number, rain: number): Stand | null {
  const closed = 1 - Math.exp(-leaf);
  if (closed < BARE || kg <= 0) return null;
  let kind: Kind;
  if (closed < DESERT_CLOSED) kind = t < TUNDRA_T ? 'tundra' : 'shrubs';
  else if (wood / kg >= WOODY) kind = wood < SHRUB_WOOD ? 'shrubs' : wood < FOREST_WOOD ? 'woodland' : t > JUNGLE_T && rain >= JUNGLE_RAIN && closed >= JUNGLE_CLOSED ? 'jungle' : 'forest';
  else if (t < TUNDRA_T && kg < TUNDRA_KG) kind = 'tundra';
  else kind = rain < STEPPE_RAIN ? 'steppe' : 'grass';
  // Darker the more wood it holds, as a tall canopy's shade deepens (the map's forest is darker too, handover 10.3).
  const colour = vegMix(FOLIAGE[kind], [24, 50, 32], vegClamp(Math.log10(1 + wood) / 2.5) * 0.4);
  return { kind, closed, mark: MARK_M[kind] ?? CROWN_LOW + CROWN_RISE * (1 - Math.exp(-wood / CROWN_WOOD)), colour };
}

/** The marks' sizes a stand is drawn at, m, with their weights: a crown blended between the two nearest of
 * `CROWN_SIZES` (on a log scale), another mark at its own size. */
function sizesOf(st: Stand): [number, number][] {
  if (MARK_M[st.kind] !== undefined) return [[st.mark, 1]];
  const s = CROWN_SIZES, c = vegClamp(st.mark, s[0], s[s.length - 1]);
  const i = Math.min(s.findIndex((_, k) => k === s.length - 1 || c < s[k + 1]), s.length - 2), t = Math.log(c / s[i]) / Math.log(s[i + 1] / s[i]);
  return [[s[i], 1 - t], [s[i + 1], t]];
}

/** A layer of a grid: one kind at one size of mark, m, and per level (open, closing) its colour and weight per tile,
 * as a picture of the grid's tiles with the box of the tiles that hold any (first and last column and row), or null
 * where no tile has any. */
interface Layer { kind: Kind; mark: number; levels: ({ pic: HTMLCanvasElement; box: number[] } | null)[] }
/** A grid of tiles on screen: its field, its window in squares, the colour of a tile's slots close up, and its layers
 * (made when its cover changes). */
interface Grid { field: Field; at: [number, number, number, number]; tint: (i: number) => Rgb | undefined; layers: Layer[] | null }

/** The cover pattern over the map: what it reads (`set`, `setFine`, `setThings`) and the picture it keeps until the view
 * or the cover change. */
export class Vegetation {
  private map: Grid | null = null;
  private fine: Grid | null = null;
  private cellM = 1;
  /** The foliage colour of the heaviest slot of the cells close up (avatars' list), by square. */
  private genes = new Map<number, Rgb>();
  private mapW = 0;
  private cache = document.createElement('canvas');
  private scratch = document.createElement('canvas');
  private patterns = new Map<HTMLCanvasElement, CanvasPattern>();
  private key = '';
  /** The map's corner the picture was made for. */
  private at: [number, number] = [0, 0];
  private version = 0;
  /** A number that changes when the map's cover changes. */
  private mapSum = NaN;
  /** The style the marks are drawn in. */
  style: Style = 'above';

  /** The frame `f` of world `w` and its map classes `cls`; null for none (another view than the earth's, or a tape
   * without the cover's layers). */
  set(w: World | null, f: number, cls: Uint8Array): void {
    const field = w ? fieldOf(w, f, cls) : null, sum = field ? signature(field) : NaN;
    if (field && this.map && sum === this.mapSum) return;
    if (w) { this.cellM = w.meta.cell_m; this.mapW = w.meta.w; }
    this.map = field && w ? { field, at: [0, 0, w.meta.w, w.meta.h], tint: i => this.genes.get(i), layers: null } : null;
    this.mapSum = sum;
    this.version++;
  }

  /** The cover of the fine raster's pixels and its window (x, y, w, h in squares), or none. */
  setFine(cover: FineCover | null, at: [number, number, number, number] | null): void {
    if (!cover || !at) { this.fine = null; this.version++; return; }
    const field = { w: cover.across, h: cover.down, kg: cover.kg, wood: cover.wood, leaf: cover.leaf, t: cover.t, rain: cover.rain, open: cover.dry };
    const [fx, fy, fw, fh] = at, g = field;
    const tint = (i: number) => this.genes.get(Math.floor(fy + ((Math.floor(i / g.w) + 0.5) * fh) / g.h) * this.mapW + Math.floor(fx + (((i % g.w) + 0.5) * fw) / g.w));
    this.fine = { field, at, tint, layers: null };
    this.version++;
  }

  /** The cover colours of the slots in `cells` (tape::things), each square by its cells' heaviest slot. */
  setThings(cells: CellThings[]): void {
    this.genes.clear();
    const best = new Map<number, number>();
    for (const c of cells) for (const sl of c.slots) {
      const [x0, y0, x1, y1] = c.box;
      for (let y = Math.floor(y0); y < Math.ceil(y1); y++) for (let x = Math.floor(x0); x < Math.ceil(x1); x++) {
        const i = y * this.mapW + x;
        if ((best.get(i) ?? -1) >= sl.kg) continue;
        best.set(i, sl.kg);
        this.genes.set(i, plantColour(plantLookOf(sl.genes, sl.kg), woodiness(sl.genes[2])));
      }
    }
    for (const g of [this.map, this.fine]) if (g) g.layers = null;
    this.version++;
  }

  /** Draw over the map, whose corner is at (ox, oy) and whose square takes `s` pixels. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    if (!this.map) return;
    const W = ctx.canvas.clientWidth, H = ctx.canvas.clientHeight, ppm = s / this.cellM;
    // Nothing to draw while the largest marks take less than a pixel.
    if (CROWN_SIZES[CROWN_SIZES.length - 1] * ppm < FADE_PX[0]) return;
    // The picture reaches `COVER_MARGIN` of the view beyond each edge, so a pan within it only moves the picture.
    const mx = Math.round(W * COVER_MARGIN), my = Math.round(H * COVER_MARGIN), cw = W + 2 * mx, ch = H + 2 * my;
    const key = [s, W, H, this.version, this.style].join();
    if (key !== this.key || Math.abs(ox - this.at[0]) > mx || Math.abs(oy - this.at[1]) > my) {
      this.key = key; this.at = [ox, oy];
      for (const c of [this.cache, this.scratch]) { c.width = cw; c.height = ch; }
      const out = this.cache.getContext('2d')!, [px, py] = [ox + mx, oy + my];
      // The fine raster where it holds the view (it is made for it), else the map's squares.
      const f = this.fine && rectOf(this.fine.at, ox, oy, s), m = rectOf(this.map.at, ox, oy, s);
      const holds = f && f[0] <= Math.max(m[0], 0) + SLACK_PX && f[1] <= Math.max(m[1], 0) + SLACK_PX && f[0] + f[2] >= Math.min(m[0] + m[2], W) - SLACK_PX && f[1] + f[3] >= Math.min(m[1] + m[3], H) - SLACK_PX;
      this.paint(out, holds && this.fine ? this.fine : this.map, px, py, s, ppm, cw, ch);
    }
    ctx.drawImage(this.cache, ox - this.at[0] - mx, oy - this.at[1] - my, cw, ch);
  }

  /** Paint grid `g` onto `out` (`W` x `H` px): each of its layers whose marks show, per level its colours and weights
   * blended across the screen, kept where the tile's coverage is, shaded by the tile's shading; each only over the
   * tiles that hold it. The picture is made a pixel a css pixel: the marks are soft, and it costs a quarter of a
   * screen's pixels. */
  private paint(out: CanvasRenderingContext2D, g: Grid, ox: number, oy: number, s: number, ppm: number, W: number, H: number): void {
    g.layers ??= layersOf(g.field, g.tint);
    const [gx, gy, gw, gh] = rectOf(g.at, ox, oy, s), tx = gw / g.field.w, ty = gh / g.field.h;
    const sc = this.scratch.getContext('2d')!;
    sc.imageSmoothingEnabled = true; sc.imageSmoothingQuality = 'high';
    for (const layer of g.layers) {
      const px = layer.mark * ppm, fade = vegSmooth(FADE_PX[0], FADE_PX[1], px);
      if (fade <= 0) continue;
      const tile = tileOf(layer.kind, this.style), size = sizeFor(tile, px), k = px / (tile.mark / 2 ** size);
      // Each size of mark has its own place of the pattern, so two sizes blended do not line up.
      const shift = ((layer.mark * 0.618034) % 1) * tile.side * k;
      layer.levels.forEach((mask, level) => {
        if (!mask) return;
        // The tiles that hold it, one more each side for the blend, on screen.
        const [b0, b1, b2, b3] = mask.box, x0 = Math.max(Math.floor(gx + (b0 - 1) * tx), 0), y0 = Math.max(Math.floor(gy + (b1 - 1) * ty), 0);
        const x1 = Math.min(Math.ceil(gx + (b2 + 2) * tx), W), y1 = Math.min(Math.ceil(gy + (b3 + 2) * ty), H);
        if (x1 <= x0 || y1 <= y0) return;
        sc.globalCompositeOperation = 'source-over';
        sc.clearRect(x0, y0, x1 - x0, y1 - y0);
        sc.drawImage(mask.pic, gx, gy, gw, gh);
        for (const [op, pic] of [['destination-in', tile.cov[level][size]], ['source-atop', tile.shade[level][size]]] as const) {
          const p = this.pattern(sc, pic);
          p.setTransform(new DOMMatrix([k, 0, 0, k, ox + shift, oy + shift]));
          sc.globalCompositeOperation = op; sc.fillStyle = p; sc.fillRect(x0, y0, x1 - x0, y1 - y0);
        }
        out.globalAlpha = fade * COVER_ALPHA;
        out.drawImage(this.scratch, x0, y0, x1 - x0, y1 - y0, x0, y0, x1 - x0, y1 - y0);
      });
    }
    out.globalAlpha = 1;
  }

  /** The repeating pattern of picture `pic`, made once. */
  private pattern(ctx: CanvasRenderingContext2D, pic: HTMLCanvasElement): CanvasPattern {
    let p = this.patterns.get(pic);
    if (!p) { p = ctx.createPattern(pic, 'repeat')!; this.patterns.set(pic, p); }
    return p;
  }
}

/** The screen rectangle (x, y, w, h) of a window (x, y, w, h in squares) of a map whose corner is at (ox, oy) and whose square takes `s` pixels. */
const rectOf = (at: [number, number, number, number], ox: number, oy: number, s: number) => [ox + at[0] * s, oy + at[1] * s, at[2] * s, at[3] * s];

/** A number that changes when the cover of field `f` changes as drawn. */
function signature(f: Field): number {
  let h = f.w;
  for (let i = 0; i < f.w * f.h; i++) h = (Math.imul(h, 31) + Math.round(f.kg[i] * 50) * 7 + Math.round(f.wood[i] * 50) * 3 + Math.round(f.leaf[i] * 20) + Math.round(f.t[i]) * 5 + Math.round(f.rain[i] * 1e4) + f.open[i]) | 0;
  return h;
}

/** The frame's cover numbers of world `w` at frame `f`, drawn on the squares whose map class `cls` is land that is
 * neither water, ice nor rock; null for a tape that lacks them. */
function fieldOf(w: World, f: number, cls: Uint8Array): Field | null {
  if (!['plants', 'wood', 'leaf', 't_ground', 'rain'].every(n => w.index.has(n))) return null;
  const open = Uint8Array.from(cls, c => (VEG_CLASSES.has(c) ? 1 : 0));
  return { w: w.meta.w, h: w.meta.h, kg: decode(w, f, 'plants'), wood: decode(w, f, 'wood'), leaf: decode(w, f, 'leaf'), t: decode(w, f, 't_ground'), rain: decode(w, f, 'rain'), open };
}

/** The layers of field `f`: each tile's stand split between its kind's sizes of mark and between the two levels (the
 * marks of an open cover come first, the closing ones as its cover closes past `OPEN_SHARE`), coloured by its stand,
 * tinted by its slots' genes (`tint`) close up. */
function layersOf(f: Field, tint: (i: number) => Rgb | undefined): Layer[] {
  const n = f.w * f.h, found = new Map<string, { kind: Kind; mark: number; data: (Uint8ClampedArray<ArrayBuffer> | null)[]; box: number[][] }>();
  for (let i = 0; i < n; i++) {
    if (!f.open[i]) continue;
    const st = stand(f.kg[i], f.wood[i], f.leaf[i], f.t[i], f.rain[i]);
    if (!st) continue;
    const tone = tint(i), colour = tone ? vegMix(st.colour, tone, 0.6) : st.colour;
    const weights = [vegClamp(st.closed / OPEN_SHARE), vegClamp((st.closed - OPEN_SHARE) / (1 - OPEN_SHARE))];
    for (const [mark, w] of sizesOf(st)) {
      if (w <= 0) continue;
      const key = `${st.kind}:${mark}`;
      let l = found.get(key);
      if (!l) { l = { kind: st.kind, mark, data: [null, null], box: [[f.w, f.h, -1, -1], [f.w, f.h, -1, -1]] }; found.set(key, l); }
      weights.forEach((lw, level) => {
        if (lw * w <= 0) return;
        const data = (l!.data[level] ??= new Uint8ClampedArray(4 * n)), b = l!.box[level], x = i % f.w, y = (i / f.w) | 0;
        b[0] = Math.min(b[0], x); b[1] = Math.min(b[1], y); b[2] = Math.max(b[2], x); b[3] = Math.max(b[3], y);
        data[4 * i] = colour[0]; data[4 * i + 1] = colour[1]; data[4 * i + 2] = colour[2]; data[4 * i + 3] = 255 * lw * w;
      });
    }
  }
  const picture = (data: Uint8ClampedArray<ArrayBuffer> | null, box: number[]) => {
    if (!data) return null;
    const pic = document.createElement('canvas');
    pic.width = f.w; pic.height = f.h;
    pic.getContext('2d')!.putImageData(new ImageData(data, f.w, f.h), 0, 0);
    return { pic, box };
  };
  return [...found.values()].sort((a, b) => KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind) || a.mark - b.mark)
    .map(l => ({ kind: l.kind, mark: l.mark, levels: l.data.map((d, k) => picture(d, l.box[k])) }));
}
