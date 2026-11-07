// Vegetation: the plant cover drawn as textures at its real size (handover 10.3, 10.4), not plant by plant. Each place
// holds a mix of kinds of pattern (grass, steppe, tundra, shrubs, woodland, forest, jungle) from its cover's own numbers
// in the frame (its mass, walls and capture area per m², tape/src/layers.rs) and its ground temperature and rain; where
// the view is finer than the cells, from those of each pixel of the fine raster (canopy.ts), over its dry ground. A kind
// gives way to the next over a range of each of its thresholds, so kinds mix where one gives way to another, never at
// an edge. The marks (textures.ts) are as many as the cover is closed, at their real size (a tree's crown from the walls
// per m², a tuft of grass), coloured by the cover and, close up, by the genes of the heaviest slot of the cells
// (tape::things), all blended between the texels' centres as the map's colours are. A kind's marks fade in as they grow
// from under a pixel, and the picture is kept until the view or the cover changes.
import { World, decode } from './data';
import { CLASSES } from './earth';
import type { CellThings } from './avatars';
import type { FineCover } from './canopy';
import { Rgb, Style, plantColour, plantLookOf, woodiness } from './sprites';
import { CH, CROWN_SIZES, CoverField, CoverPainter, LARGEST_MARK, coverField, putChannel } from './textures';

/** The numbers a pattern reads, per texel of a grid of `w` x `h` (row-major), and whether cover is drawn on it (1: dry
 * open ground). */
interface Field { w: number; h: number; kg: Float32Array; wood: Float32Array; leaf: Float32Array; t: Float32Array; rain: Float32Array; open: Uint8Array }

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
/** A kind gives way to the next from a threshold over (1 + `SOFT`) times or this many K either side of it. */
const SOFT = 0.3, SOFT_K = 2;
/** A tree's crown across, m: `CROWN_LOW` plus `CROWN_RISE` × (1 - e^(-walls / `CROWN_WOOD`)), walls in kg/m² (3 m in a
 * young stand to about 13 m in a stand of 30 kg/m²). */
const CROWN_LOW = 3, CROWN_RISE = 12, CROWN_WOOD = 15;
/** The pixels a mark takes from which its kind starts to show, and from which it shows fully. */
const FADE_PX = [0.7, 2];
/** The share of the view's width and height the picture of the textures reaches beyond each edge. */
const COVER_MARGIN = 0.3;
/** The pixels by which the fine raster may miss the view's edge and still be taken to hold it. */
const SLACK_PX = 1;
/** How strongly the textures cover the map, so its hill shading still shows through. */
const COVER_ALPHA = 0.85;
/** The foliage darkens towards `DARK` by up to `DARK_MOST`, fully at 10^`DARK_DECADES` kg/m² of walls, as a tall
 * canopy's shade deepens (the map's forest is darker too, handover 10.3); the genes of a place tint it by `TINT`. */
const DARK: Rgb = [24, 50, 32], DARK_MOST = 0.4, DARK_DECADES = 2.5, TINT = 0.6;
/** The base foliage of grass, tundra, steppe, shrubs, woodland, forest and jungle (display colours, in the order of the
 * marks, textures.ts); a slot's genes tint it close up. */
const FOLIAGE: Rgb[] = [[124, 158, 84], [136, 142, 112], [168, 160, 106], [112, 118, 78], [92, 124, 70], [60, 104, 62], [44, 116, 56]];

/** The map classes on which cover is drawn (the land that is neither water, ice nor rock). */
const VEG_CLASSES = new Set(['Desert', 'Grassland', 'Forest'].map(n => CLASSES.findIndex(c => c[0] === n)));
const vegClamp = (x: number, a = 0, b = 1) => (x < a ? a : x > b ? b : x);
const vegSmooth = (a: number, b: number, x: number) => { const t = vegClamp((x - a) / (b - a)); return t * t * (3 - 2 * t); };
/** 0 below `at` less `soft`, 1 above `at` plus `soft`, smooth between. */
const past = (x: number, at: number, soft: number) => vegSmooth(at - soft, at + soft, x);
/** The same on a log scale: from `at` / (1 + `SOFT`) to `at` × (1 + `SOFT`). */
const pastLog = (x: number, at: number) => past(Math.log(Math.max(x, 1e-30)), Math.log(at), Math.log1p(SOFT));

/** The shares of a tree's crowns drawn at each of `CROWN_SIZES` for walls `wood` kg/m²: its size blended between the
 * two nearest (on a log scale). */
function crownShares(wood: number): number[] {
  const s = CROWN_SIZES, c = vegClamp(CROWN_LOW + CROWN_RISE * (1 - Math.exp(-wood / CROWN_WOOD)), s[0], s[s.length - 1]);
  const i = Math.min(s.findIndex((_, k) => k === s.length - 1 || c < s[k + 1]), s.length - 2), t = Math.log(c / s[i]) / Math.log(s[i + 1] / s[i]);
  return s.map((_, k) => (k === i ? 1 - t : k === i + 1 ? t : 0));
}

/** Write into texel `i` of `out` the marks of a place whose cover holds `kg` and `wood` kg/m² and `leaf` capture area per
 * m², whose ground is at `t` K and whose rain is `rain` mol/m²/s: as many marks as its cover is closed (1 - e^-area),
 * shared between the kinds by how far it is past each of their thresholds. */
function putStand(out: CoverField, i: number, kg: number, wood: number, leaf: number, t: number, rain: number): void {
  const closed = 1 - Math.exp(-leaf);
  if (kg <= 0 || closed <= 0) return;
  const cold = 1 - past(t, TUNDRA_T, SOFT_K), desert = 1 - pastLog(closed, DESERT_CLOSED), grown = 1 - desert;
  const woody = grown * pastLog(wood / kg, WOODY), herbs = grown - woody;
  const tundra = herbs * cold * (1 - pastLog(kg, TUNDRA_KG)), steppe = (herbs - tundra) * (1 - pastLog(rain, STEPPE_RAIN));
  const trees = woody * pastLog(wood, SHRUB_WOOD), forest = trees * pastLog(wood, FOREST_WOOD);
  const jungle = forest * past(t, JUNGLE_T, SOFT_K) * pastLog(rain, JUNGLE_RAIN) * pastLog(closed, JUNGLE_CLOSED);
  // A desert is rare tundra where it is cold and rare shrubs elsewhere.
  const kinds: [number, number][] = [[CH.grass, herbs - tundra - steppe], [CH.tundra, tundra + desert * cold], [CH.steppe, steppe], [CH.shrubs, woody - trees + desert * (1 - cold)]];
  for (const [ch, share] of kinds) putChannel(out, i, ch, share * closed);
  crownShares(wood).forEach((share, k) => putChannel(out, i, CH.crowns + k, share * trees * closed));
  putChannel(out, i, CH.dark, vegClamp(Math.log10(1 + wood) / DARK_DECADES) * DARK_MOST);
  if (trees > 0) { putChannel(out, i, CH.woodland, (trees - forest) / trees); putChannel(out, i, CH.jungle, jungle / trees); }
}

/** The texels of field `f` as the marks read them. */
function coverOf(f: Field): CoverField {
  const out = coverField(f.w, f.h);
  for (let i = 0; i < f.w * f.h; i++) if (f.open[i]) putStand(out, i, f.kg[i], f.wood[i], f.leaf[i], f.t[i], f.rain[i]);
  return out;
}

/** A grid of texels on screen: its field, its window in squares, and its texels as the marks read them (made when it is
 * first drawn). */
interface Grid { field: Field; at: [number, number, number, number]; cover: CoverField | null }

/** The cover pattern over the map: what it reads (`set`, `setFine`, `setThings`) and the picture it keeps until the view
 * or the cover change. */
export class Vegetation {
  private map: Grid | null = null;
  private fine: Grid | null = null;
  private cellM = 1;
  /** The foliage colour of the heaviest slot of the cells close up (avatars' list), by square. */
  private genes = new Map<number, Rgb>();
  private mapW = 0;
  private mapH = 0;
  private painter = new CoverPainter({ foliage: FOLIAGE, dark: DARK, tint: TINT });
  /** The grid whose texels and the genes whose tint the painter holds. */
  private uploaded: Grid | null = null;
  private tinted = false;
  private cache = document.createElement('canvas');
  /** Whether the cache holds a picture. */
  private shown = false;
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
    if (w) { this.cellM = w.meta.cell_m; this.mapW = w.meta.w; this.mapH = w.meta.h; this.tinted = false; }
    this.map = field && w ? { field, at: [0, 0, w.meta.w, w.meta.h], cover: null } : null;
    this.mapSum = sum;
    this.version++;
  }

  /** The cover of the fine raster's pixels and its window (x, y, w, h in squares), or none. */
  setFine(cover: FineCover | null, at: [number, number, number, number] | null): void {
    this.fine = cover && at ? { field: { w: cover.across, h: cover.down, kg: cover.kg, wood: cover.wood, leaf: cover.leaf, t: cover.t, rain: cover.rain, open: cover.dry }, at, cover: null } : null;
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
    this.tinted = false;
    this.version++;
  }

  /** Draw over the map, whose corner is at (ox, oy) and whose square takes `s` pixels. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    if (!this.map) return;
    const W = ctx.canvas.clientWidth, H = ctx.canvas.clientHeight, mpp = this.cellM / s;
    // Nothing to draw while the largest marks take less than a pixel.
    if (LARGEST_MARK / mpp < FADE_PX[0]) return;
    // The picture reaches `COVER_MARGIN` of the view beyond each edge, so a pan within it only moves the picture.
    const mx = Math.round(W * COVER_MARGIN), my = Math.round(H * COVER_MARGIN), cw = W + 2 * mx, ch = H + 2 * my;
    const key = [s, W, H, this.version, this.style].join();
    if (key !== this.key || Math.abs(ox - this.at[0]) > mx || Math.abs(oy - this.at[1]) > my) {
      this.key = key; this.at = [ox, oy];
      // The fine raster where it holds the view (it is made for it), else the map's squares.
      const f = this.fine && rectOf(this.fine.at, ox, oy, s), m = rectOf(this.map.at, ox, oy, s);
      const holds = f && f[0] <= Math.max(m[0], 0) + SLACK_PX && f[1] <= Math.max(m[1], 0) + SLACK_PX && f[0] + f[2] >= Math.min(m[0] + m[2], W) - SLACK_PX && f[1] + f[3] >= Math.min(m[1] + m[3], H) - SLACK_PX;
      this.shown = this.paint(holds && this.fine ? this.fine : this.map, [(-mx - ox) * mpp, (-my - oy) * mpp], mpp, cw, ch);
    }
    if (this.shown) ctx.drawImage(this.cache, ox - this.at[0] - mx, oy - this.at[1] - my, cw, ch);
  }

  /** Paint the marks of grid `g` into the cache, `w` x `h` px of `mpp` m each from the world's point `at`, m: a pixel a
   * css pixel, as the marks are soft. False when there is nothing to paint with. */
  private paint(g: Grid, at: [number, number], mpp: number, w: number, h: number): boolean {
    if (this.uploaded !== g) { g.cover ??= coverOf(g.field); this.painter.setField(g.cover); this.uploaded = g; }
    if (!this.tinted) { this.painter.setTint(this.mapW, this.mapH, this.tint()); this.tinted = true; }
    const m = this.cellM, [gx, gy, gw, gh] = g.at;
    if (!this.painter.paint({ w, h, mpp, at, field: [gx * m, gy * m, gw * m, gh * m], tint: [0, 0, this.mapW * m, this.mapH * m], fade: FADE_PX, alpha: COVER_ALPHA, style: this.style })) return false;
    if (this.cache.width !== w || this.cache.height !== h) { this.cache.width = w; this.cache.height = h; }
    const out = this.cache.getContext('2d')!;
    out.clearRect(0, 0, w, h);
    out.drawImage(this.painter.canvas, 0, 0);
    return true;
  }

  /** The genes' tint of the map's squares: per square its colour times its weight, and its weight (1 where a slot's
   * colour is known), as RGBA bytes. */
  private tint(): Uint8Array {
    const data = new Uint8Array(4 * this.mapW * this.mapH);
    for (const [i, c] of this.genes) { data.set(c.map(v => Math.round(vegClamp(v, 0, 255))), 4 * i); data[4 * i + 3] = 255; }
    return data;
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
