// Vegetation: where the cover of a place is dense it is drawn as a ground pattern, not plant by plant (handover 10.3,
// 10.4): a canopy of overlapping crowns where it is woody, layered and greener where it is also warm and wet, fine
// strokes where it is herbs, low tufts in the cold, bare ground with rare shrubs where it is thin. The pattern comes from
// the cover's own numbers in the frame (its mass, walls and capture area per m², tape/src/layers.rs) and the place's
// ground temperature and rain; where the view is finer than the cells, from those of each pixel of the fine raster
// (canopy.ts), drawn over that raster's dry ground; close up, its colour from the genes of the slots of its cell
// (tape::things). Each tile (a square, or a pixel of the fine raster) is seeded by its place, so the pattern holds
// from frame to frame.
import { World, decode } from './data';
import { CLASSES } from './earth';
import type { CellThings } from './avatars';
import type { FineCover } from './canopy';
import { Rgb, seeded, plantColour, plantLookOf, woodiness } from './sprites';

/** What the pattern of a tile is drawn as. */
export type Biome = 'forest' | 'jungle' | 'woodland' | 'grass' | 'steppe' | 'tundra' | 'desert';
/** The look of a tile: its kind of pattern, how closed its cover is (0..1), its height, m, and its foliage colour. */
interface Stand { biome: Biome; closed: number; height: number; colour: Rgb }
/** The numbers a pattern reads, per tile of a grid of `w` x `h` (row-major), and whether cover is drawn on it (1: dry
 * open ground). */
interface Field { w: number; h: number; kg: Float32Array; wood: Float32Array; leaf: Float32Array; t: Float32Array; rain: Float32Array; open: Uint8Array }

/** Pixels a map square takes from which the pattern is drawn; below, the map's colour shows the cover. */
export const VEG_PX = 10;
/** Pixels a map square takes from which plants are drawn one by one everywhere (avatars.ts), not as a pattern. */
export const PLANTS_CLOSE_PX = 1200;
/** The cover share (1 - e^-capture area per m², the light a canopy of that area absorbs, laws.md 4.5) below which a
 * place is open: close up its plants are drawn one by one (avatars.ts) and the pattern leaves it to them. */
export const OPEN = 0.5;
/** Below this cover share nothing is drawn. */
const BARE = 0.02;
/** The woody share of built cover (walls over walls and matter) from which a cover is drawn as crowns. */
const WOODY = 0.6;
/** Walls per m², kg, from which a woody cover is a forest (below, an open woodland of shrubs and small trees); a forest
 * holds about 10 to 50 kg/m² of dry matter (Whittaker and Likens 1975), mostly wood. */
const FOREST_WOOD = 3;
/** A forest is a jungle where its ground is warmer than this, K, its rain at least this, mol/m²/s (about 1,100 mm a
 * year), and its cover share at least this. */
const JUNGLE_T = 293, JUNGLE_RAIN = 2e-3, JUNGLE_CLOSED = 0.85;
/** Herbs are steppe where rain is below this, mol/m²/s (about 400 mm a year), and tundra where the ground is colder
 * than this, K, and the cover thinner than `TUNDRA_KG` kg/m². */
const STEPPE_RAIN = 7e-4, TUNDRA_T = 271, TUNDRA_KG = 1;
/** Below this cover share a place is a desert of rare shrubs. */
const DESERT_CLOSED = 0.12;
/** The height of a woody stand, m: `TREE_LOW` plus `TREE_RISE` × (1 - e^(-walls / `TREE_WOOD`)), walls in kg/m² (a
 * stand of 30 kg/m² stands about 25 m). */
const TREE_LOW = 3, TREE_RISE = 25, TREE_WOOD = 15;
/** The smallest crown, stroke and tuft drawn, px, and the most marks drawn in one pass. */
const MIN_MARK_PX = 3, MOST_MARKS = 40000;

/** The map classes on which cover is drawn (the land that is neither water, ice nor rock). */
const VEG_CLASSES = new Set(['Desert', 'Grassland', 'Forest'].map(n => CLASSES.findIndex(c => c[0] === n)));
const vegClamp = (x: number, a = 0, b = 1) => (x < a ? a : x > b ? b : x);
const vegRgb = (c: Rgb, f = 1, a = 1) => `rgba(${c.map(v => Math.round(vegClamp(v * f, 0, 255))).join(',')},${a})`;
const vegMix = (a: Rgb, b: Rgb, t: number): Rgb => [0, 1, 2].map(k => a[k] + (b[k] - a[k]) * t) as Rgb;

/** The base foliage of each pattern (display colours); a slot's genes tint it close up. */
const FOLIAGE: Record<Biome, Rgb> = {
  forest: [52, 104, 58], jungle: [34, 122, 50], woodland: [92, 128, 66], grass: [128, 170, 82],
  steppe: [178, 170, 104], tundra: [138, 146, 112], desert: [128, 122, 84],
};

/** The stand of a tile: its pattern from its cover (`kg` and `wood` kg/m², `leaf` capture area per m²) and its ground
 * temperature `t` K and rain mol/m²/s; null where there is too little to draw. */
export function stand(kg: number, wood: number, leaf: number, t: number, rain: number): Stand | null {
  const closed = 1 - Math.exp(-leaf);
  if (closed < BARE || kg <= 0) return null;
  const woody = wood / kg, height = TREE_LOW + TREE_RISE * (1 - Math.exp(-wood / TREE_WOOD));
  let biome: Biome;
  if (closed < DESERT_CLOSED) biome = t < TUNDRA_T ? 'tundra' : 'desert';
  else if (woody >= WOODY && wood >= FOREST_WOOD) biome = t > JUNGLE_T && rain >= JUNGLE_RAIN && closed >= JUNGLE_CLOSED ? 'jungle' : 'forest';
  else if (woody >= WOODY) biome = 'woodland';
  else if (t < TUNDRA_T && kg < TUNDRA_KG) biome = 'tundra';
  else biome = rain < STEPPE_RAIN ? 'steppe' : 'grass';
  // Darker the more wood it holds, as a tall canopy's shade deepens (the map's forest is darker too, handover 10.3).
  const colour = vegMix(FOLIAGE[biome], [20, 48, 30], vegClamp(Math.log10(1 + wood) / 2.5) * (biome === 'forest' ? 0.6 : 0.3));
  return { biome, closed, height: woody >= WOODY ? height : 0.3 + 0.7 * Math.min(kg, 2), colour };
}

/** The cover pattern over the map: what it reads (`set`, `setFine`, `setThings`) and the picture it keeps until the view
 * or the cover change. */
export class Vegetation {
  private field: Field | null = null;
  private fine: { field: Field; at: [number, number, number, number] } | null = null;
  private cellM = 1;
  /** The foliage colour of the heaviest slot of the cells close up (avatars' list), by square. */
  private genes = new Map<number, Rgb>();
  private mapW = 0;
  private cache = document.createElement('canvas');
  private key = '';

  /** The frame `f` of world `w` and its map classes `cls`; null for none (another view than the earth's, or a tape
   * without the cover's layers). */
  set(w: World | null, f: number, cls: Uint8Array): void {
    this.field = w ? fieldOf(w, f, cls) : null;
    if (w) { this.cellM = w.meta.cell_m; this.mapW = w.meta.w; }
  }

  /** The cover of the fine raster's pixels and its window (x, y, w, h in squares), or none. */
  setFine(cover: FineCover | null, at: [number, number, number, number] | null): void {
    this.fine = cover && at ? { field: { w: cover.across, h: cover.down, kg: cover.kg, wood: cover.wood, leaf: cover.leaf, t: cover.t, rain: cover.rain, open: cover.dry }, at } : null;
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
  }

  /** Draw over the map, whose corner is at (ox, oy) and whose square takes `s` pixels. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    const f = this.field;
    if (!f || s < VEG_PX || s >= PLANTS_CLOSE_PX) return;
    const W = ctx.canvas.clientWidth, H = ctx.canvas.clientHeight, d = window.devicePixelRatio || 1;
    const grids: Grid[] = [{ field: f, x: ox, y: oy, px: s, py: s, seed: 0, tint: i => this.genes.get(i) }];
    if (this.fine) {
      const g = this.fine.field, [fx, fy, fw, fh] = this.fine.at, mapW = this.mapW;
      const tint = (i: number) => this.genes.get(Math.floor(fy + ((Math.floor(i / g.w) + 0.5) * fh) / g.h) * mapW + Math.floor(fx + (((i % g.w) + 0.5) * fw) / g.w));
      grids.push({ field: g, x: ox + fx * s, y: oy + fy * s, px: (fw * s) / g.w, py: (fh * s) / g.h, seed: 0x40000000 + Math.round(fx * 4096) * 7919 + Math.round(fy * 4096), tint });
    }
    const key = [ox, oy, s, W, H, d, ...grids.map(g => signature(g, W, H))].join();
    if (key !== this.key) {
      this.key = key;
      this.cache.width = Math.round(W * d); this.cache.height = Math.round(H * d);
      const c = this.cache.getContext('2d')!;
      c.setTransform(d, 0, 0, d, 0, 0);
      paintGrids(c, grids, this.cellM / s, W, H);
    }
    ctx.drawImage(this.cache, 0, 0, W, H);
  }
}

/** A grid of tiles on screen: its field, the screen corner of its first tile and a tile's size, px; the seed of its
 * tiles' streams; and the colour of a tile's slots close up. A later grid (the fine raster) covers the earlier where it
 * lies. */
interface Grid { field: Field; x: number; y: number; px: number; py: number; seed: number; tint: (i: number) => Rgb | undefined }

/** The frame's cover numbers of world `w` at frame `f`, drawn on the squares whose map class `cls` is land that is
 * neither water, ice nor rock; null for a tape that lacks them. */
function fieldOf(w: World, f: number, cls: Uint8Array): Field | null {
  if (!['plants', 'wood', 'leaf', 't_ground', 'rain'].every(n => w.index.has(n))) return null;
  const open = Uint8Array.from(cls, c => (VEG_CLASSES.has(c) ? 1 : 0));
  return { w: w.meta.w, h: w.meta.h, kg: decode(w, f, 'plants'), wood: decode(w, f, 'wood'), leaf: decode(w, f, 'leaf'), t: decode(w, f, 't_ground'), rain: decode(w, f, 'rain'), open };
}

/** The cover at (u, v) in tiles of field `f` (a tile's centre at its index + 0.5), blended between the tiles' centres
 * as the map's colours are, and how much of it is open ground (0..1); the colour of the nearest tile's slots by `tint`. */
function sample(f: Field, u: number, v: number, tint: (i: number) => Rgb | undefined): { stand: Stand | null; open: number } {
  const x = Math.min(Math.max(u - 0.5, 0), f.w - 1), y = Math.min(Math.max(v - 0.5, 0), f.h - 1);
  const x0 = Math.floor(x), y0 = Math.floor(y), x1 = Math.min(x0 + 1, f.w - 1), y1 = Math.min(y0 + 1, f.h - 1), a = x - x0, b = y - y0;
  const at = (q: Float32Array | Uint8Array) => (q[y0 * f.w + x0] * (1 - a) + q[y0 * f.w + x1] * a) * (1 - b) + (q[y1 * f.w + x0] * (1 - a) + q[y1 * f.w + x1] * a) * b;
  // Cover is read where there is ground: the open tiles' numbers weigh by their openness, so a forest's edge on a lake
  // keeps the forest's numbers and fades by the openness alone.
  const open = at(f.open), w = (q: Float32Array) => {
    let s = 0, n = 0;
    for (const [i, k] of [[y0 * f.w + x0, (1 - a) * (1 - b)], [y0 * f.w + x1, a * (1 - b)], [y1 * f.w + x0, (1 - a) * b], [y1 * f.w + x1, a * b]]) { s += q[i] * k * f.open[i]; n += k * f.open[i]; }
    return n > 0 ? s / n : 0;
  };
  if (open <= 0) return { stand: null, open };
  const st = stand(w(f.kg), w(f.wood), w(f.leaf), w(f.t), w(f.rain));
  const near = Math.min(Math.round(v - 0.5), f.h - 1) * f.w + Math.min(Math.round(u - 0.5), f.w - 1), tone = tint(Math.max(near, 0));
  if (st && tone) st.colour = vegMix(st.colour, tone, 0.6);
  return { stand: st, open };
}

/** A number that changes when the cover of the tiles of `g` in view changes. */
function signature(g: Grid, W: number, H: number): number {
  const f = g.field;
  const x0 = Math.max(0, Math.floor(-g.x / g.px) - 1), y0 = Math.max(0, Math.floor(-g.y / g.py) - 1);
  const x1 = Math.min(f.w, Math.ceil((W - g.x) / g.px) + 1), y1 = Math.min(f.h, Math.ceil((H - g.y) / g.py) + 1);
  let h = g.seed | 0;
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
    const i = y * f.w + x;
    h = (Math.imul(h, 31) + Math.round(f.kg[i] * 50) * 7 + Math.round(f.wood[i] * 50) * 3 + Math.round(f.leaf[i] * 20) + f.open[i] + (g.tint(i)?.[0] ?? 0)) | 0;
  }
  return h;
}

/** How many marks a stand holds per px² of its open ground at a cover share of one, and their size and lift on screen,
 * px, at `mpp` metres a pixel: a crown of a woody stand is about a fifth of its height across and is lifted by its
 * height, an herb's stroke is its height; at least `MIN_MARK_PX` however far the view. */
function marksOf(st: Stand, mpp: number): { per: number; size: number; lift: number } {
  const woody = st.biome === 'forest' || st.biome === 'jungle' || st.biome === 'woodland';
  const size = Math.max(MIN_MARK_PX, (woody ? 1.5 + 0.2 * st.height : st.height) / mpp) * (woody ? 1 : 1.4);
  const lift = woody ? Math.min(size * 0.9, Math.max(1, (st.height / mpp) * 0.25)) : 0;
  const area = woody ? Math.PI * size * size * 0.55 : st.biome === 'desert' ? size * size * 60 : size * size * 0.7;
  return { per: 1 / area, size, lift };
}

/** Paint the marks of `grids` in a view of `W` x `H` px at `mpp` metres a screen pixel: in each tile,
 * candidates at the densest a stand of its neighbourhood holds, each kept by the blended cover where it falls (its
 * cover share, its openness, and its own stand's density over the candidates'), so patterns fade into each other and
 * into the shore as the map's colours do; at most `MOST_MARKS`. Crowns are drawn back to front over all tiles. */
function paintGrids(c: CanvasRenderingContext2D, grids: Grid[], mpp: number, W: number, H: number): void {
  type Mark = { x: number; y: number; st: Stand; m: { size: number; lift: number }; r: number };
  const marks: Mark[] = [];
  grids.forEach((g, k) => {
    const f = g.field, later = grids.slice(k + 1);
    const covered = (sx: number, sy: number) => later.some(o => sx >= o.x && sy >= o.y && sx < o.x + o.px * o.field.w && sy < o.y + o.py * o.field.h);
    const x0 = Math.max(0, Math.floor(-g.x / g.px)), y0 = Math.max(0, Math.floor(-g.y / g.py));
    const x1 = Math.min(f.w, Math.ceil((W - g.x) / g.px)), y1 = Math.min(f.h, Math.ceil((H - g.y) / g.py));
    // The density of each tile's own stand, then the densest of each tile's neighbourhood.
    const bw = x1 - x0 + 2, own = new Float32Array(bw * (y1 - y0 + 2));
    for (let y = y0 - 1; y <= y1; y++) for (let x = x0 - 1; x <= x1; x++) {
      const i = y * f.w + x;
      if (x < 0 || y < 0 || x >= f.w || y >= f.h || !f.open[i]) continue;
      const st = stand(f.kg[i], f.wood[i], f.leaf[i], f.t[i], f.rain[i]);
      if (st) own[(y - y0 + 1) * bw + x - x0 + 1] = marksOf(st, mpp).per;
    }
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
      let per = 0;
      for (let dy = 0; dy < 3; dy++) for (let dx = 0; dx < 3; dx++) per = Math.max(per, own[(y - y0 + dy) * bw + x - x0 + dx]);
      if (per <= 0) continue;
      // As many candidates as the densest stand holds on the tile, the fraction kept by chance.
      const r = seeded((g.seed + y * 65521 + x * 31) >>> 0), want = per * g.px * g.py, n = Math.floor(want) + (r() < want % 1 ? 1 : 0);
      for (let q = 0; q < n; q++) {
        const u = x + r(), v = y + r(), keep = r(), sx = g.x + u * g.px, sy = g.y + v * g.py;
        if (covered(sx, sy)) continue;
        const { stand: st, open } = sample(f, u, v, g.tint);
        if (!st) continue;
        const m = marksOf(st, mpp);
        if (keep < st.closed * open * (m.per / per)) marks.push({ x: sx, y: sy, st, m, r: r() });
      }
    }
  });
  const keep = Math.min(1, MOST_MARKS / Math.max(marks.length, 1));
  const shown = marks.filter((_, k) => keep >= 1 || (k * 2654435761 >>> 0) / 4294967296 < keep).sort((a, b) => a.y - b.y);
  for (const k of shown) {
    const { st, m } = k, col = st.colour;
    switch (st.biome) {
      case 'forest': case 'jungle': case 'woodland': {
        // A crown: its shade on the ground under it, its shadow cast to the south-east by its height, its round top, a
        // lit side to the north-west; a jungle's crowns in three layers.
        const layer = st.biome === 'jungle' ? Math.floor(k.r * 3) : 0, size = m.size * [1, 0.75, 1.6][layer] * (0.75 + 0.5 * ((k.r * 7) % 1));
        const lift = m.lift * [1, 0.5, 1.4][layer], tone = [1, 1.15, 0.9][layer] * (0.85 + 0.3 * ((k.r * 13) % 1));
        if (st.biome !== 'woodland') { c.fillStyle = vegRgb(col, 0.5, 0.5); c.beginPath(); c.arc(k.x, k.y, size * 1.3, 0, 2 * Math.PI); c.fill(); }
        c.fillStyle = 'rgba(8,24,12,.32)';
        c.beginPath(); c.ellipse(k.x + lift * 0.6, k.y + lift * 0.35, size, size * 0.7, 0, 0, 2 * Math.PI); c.fill();
        c.fillStyle = vegRgb(col, tone);
        c.beginPath(); c.arc(k.x, k.y - lift * 0.4, size, 0, 2 * Math.PI); c.fill();
        c.fillStyle = vegRgb(col, tone * 1.3, 0.9);
        c.beginPath(); c.arc(k.x - size * 0.32, k.y - lift * 0.4 - size * 0.32, size * 0.45, 0, 2 * Math.PI); c.fill();
        break;
      }
      case 'grass': case 'steppe': {
        const len = m.size * (0.7 + 0.6 * k.r), lean = ((k.r * 17) % 1 - 0.5) * 0.9;
        c.lineWidth = Math.max(1, m.size * 0.35); c.lineCap = 'round';
        c.strokeStyle = vegRgb(col, 0.55 + 0.45 * ((k.r * 5) % 1), 0.9);
        c.beginPath(); c.moveTo(k.x, k.y); c.lineTo(k.x + lean * len, k.y - len); c.stroke();
        break;
      }
      case 'tundra': {
        c.lineWidth = Math.max(0.7, m.size * 0.22); c.lineCap = 'round';
        c.strokeStyle = vegRgb(col, 0.7 + 0.4 * k.r, 0.8);
        for (const lean of [-0.6, 0, 0.6]) { c.beginPath(); c.moveTo(k.x, k.y); c.lineTo(k.x + lean * m.size * 0.5, k.y - m.size * 0.6); c.stroke(); }
        break;
      }
      case 'desert': {
        const rr = m.size * (0.6 + 0.5 * k.r);
        c.fillStyle = 'rgba(0,0,0,.18)'; c.beginPath(); c.ellipse(k.x + rr * 0.4, k.y + rr * 0.3, rr, rr * 0.55, 0, 0, 2 * Math.PI); c.fill();
        c.fillStyle = vegRgb(col, 0.9 + 0.3 * k.r); c.beginPath(); c.arc(k.x, k.y - rr * 0.3, rr * 0.8, 0, 2 * Math.PI); c.fill();
        break;
      }
    }
  }
}
