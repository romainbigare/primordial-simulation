// Vegetation: where the cover of a place is dense it is drawn as a ground pattern, not plant by plant (handover 10.3,
// 10.4): a canopy of overlapping crowns where it is woody, layered and greener where it is also warm and wet, fine
// strokes where it is herbs, low tufts in the cold, bare ground with rare shrubs where it is thin. The pattern comes from
// the cover's own numbers in the frame (its mass, walls and capture area per m², tape/src/layers.rs) and the place's
// ground temperature and rain; close up, its colour from the genes of the slots of its cell (tape::things). Each tile
// (a square, or a pixel of the fine raster) is seeded by its place, so the pattern holds from frame to frame.
import { World, decode } from './data';
import { CLASSES } from './earth';
import type { CellThings } from './avatars';
import { Rgb, seeded, plantColour, plantLookOf, woodiness } from './sprites';

/** What the pattern of a tile is drawn as. */
export type Biome = 'forest' | 'jungle' | 'woodland' | 'grass' | 'steppe' | 'tundra' | 'desert';
/** The look of a tile: its kind of pattern, how closed its cover is (0..1), its height, m, and its foliage colour. */
interface Stand { biome: Biome; closed: number; height: number; colour: Rgb }
/** The frame's numbers a pattern reads, per tile of a grid of `w` x `h` (row-major), and the map class of each. */
interface Field { w: number; h: number; kg: Float32Array; wood: Float32Array; leaf: Float32Array; t: Float32Array; rain: Float32Array; cls: Uint8Array }

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

/** The cover pattern over the map: what it reads (`set`) and the picture it keeps until the view or the cover change. */
export class Vegetation {
  private field: Field | null = null;
  private fine: { field: Field; at: [number, number, number, number] } | null = null;
  private cellM = 1;
  /** The slots of the cells close up (avatars' list), by square, for their genes' colour. */
  private genes = new Map<number, Rgb>();
  private mapW = 0;
  private cache = document.createElement('canvas');
  private key = '';

  /** The frame `f` of world `w` and its map classes `cls`, and the fine raster's world, classes and window if any;
   * null for none (another view than the earth's, or a tape without the cover's layers). */
  set(w: World | null, f: number, cls: Uint8Array, fine: { world: World; cls: Uint8Array; at: [number, number, number, number] } | null): void {
    this.field = w ? fieldOf(w, f, cls) : null;
    this.fine = fine && this.field ? (() => { const fl = fieldOf(fine.world, 0, fine.cls); return fl ? { field: fl, at: fine.at } : null; })() : null;
    if (w) { this.cellM = w.meta.cell_m; this.mapW = w.meta.w; }
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
    const tiles = this.tiles(ox, oy, s, W, H);
    const key = [ox, oy, s, W, H, d, signature(tiles)].join();
    if (key !== this.key) {
      this.key = key;
      this.cache.width = Math.round(W * d); this.cache.height = Math.round(H * d);
      const c = this.cache.getContext('2d')!;
      c.setTransform(d, 0, 0, d, 0, 0);
      paintTiles(c, tiles, this.cellM / s);
    }
    ctx.drawImage(this.cache, 0, 0, W, H);
  }

  /** The tiles in view: each square of the map, but where the fine raster covers it, its pixels. */
  private tiles(ox: number, oy: number, s: number, W: number, H: number): Tile[] {
    const out: Tile[] = [], f = this.field!, fine = this.fine;
    const x0 = Math.max(0, Math.floor(-ox / s)), y0 = Math.max(0, Math.floor(-oy / s));
    const x1 = Math.min(f.w, Math.ceil((W - ox) / s)), y1 = Math.min(f.h, Math.ceil((H - oy) / s));
    const inFine = (x: number, y: number) => fine && x >= fine.at[0] && x + 1 <= fine.at[0] + fine.at[2] && y >= fine.at[1] && y + 1 <= fine.at[1] + fine.at[3];
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
      if (inFine(x, y)) continue;
      const i = y * f.w + x, t = tileOf(f, i, [ox + x * s, oy + y * s, s, s], i, this.genes.get(i));
      if (t) out.push(t);
    }
    if (fine) {
      const g = fine.field, [fx, fy, fw, fh] = fine.at, pw = (fw * s) / g.w, ph = (fh * s) / g.h;
      for (let y = 0; y < g.h; y++) for (let x = 0; x < g.w; x++) {
        const px = ox + fx * s + x * pw, py = oy + fy * s + y * ph;
        if (px + pw < 0 || py + ph < 0 || px > W || py > H) continue;
        const sq = Math.floor(fy + ((y + 0.5) * fh) / g.h) * this.mapW + Math.floor(fx + ((x + 0.5) * fw) / g.w);
        const t = tileOf(g, y * g.w + x, [px, py, pw, ph], 0x40000000 + Math.round((fx * g.w) / fw + x) * 65521 + Math.round((fy * g.h) / fh + y), this.genes.get(sq));
        if (t) out.push(t);
      }
    }
    return out;
  }
}

/** A tile to draw: its stand, its screen box (x, y, width, height px), its seed, and the byte signature of its cover. */
interface Tile { stand: Stand; box: [number, number, number, number]; seed: number; sig: number }

/** The frame's cover numbers of world `w` at frame `f`; null for a tape that lacks them. */
function fieldOf(w: World, f: number, cls: Uint8Array): Field | null {
  if (!['plants', 'wood', 'leaf', 't_ground', 'rain'].every(n => w.index.has(n))) return null;
  return { w: w.meta.w, h: w.meta.h, kg: decode(w, f, 'plants'), wood: decode(w, f, 'wood'), leaf: decode(w, f, 'leaf'), t: decode(w, f, 't_ground'), rain: decode(w, f, 'rain'), cls };
}

/** Tile `i` of field `f` at screen box `box`: its stand, tinted by its slots' genes `tint` close up; null where there is
 * nothing to draw, or where the place is open and its plants are drawn one by one (`OPEN`). */
function tileOf(f: Field, i: number, box: [number, number, number, number], seed: number, tint: Rgb | undefined): Tile | null {
  if (!VEG_CLASSES.has(f.cls[i])) return null;
  const st = stand(f.kg[i], f.wood[i], f.leaf[i], f.t[i], f.rain[i]);
  if (!st || (tint && st.closed < OPEN)) return null;
  if (tint) st.colour = vegMix(st.colour, tint, 0.6);
  return { stand: st, box, seed: seed >>> 0, sig: Math.round(f.kg[i] * 50) * 31 + Math.round(f.wood[i] * 50) * 7 + Math.round(f.leaf[i] * 20) + f.cls[i] };
}

/** A number that changes when the tiles or their cover change. */
function signature(tiles: Tile[]): number {
  let h = tiles.length;
  for (const t of tiles) h = (Math.imul(h, 31) + t.sig + t.seed + (t.stand.colour[0] | 0)) | 0;
  return h;
}

/** Paint `tiles` at `mpp` metres a pixel: the marks of each pattern, at most `MOST_MARKS`. */
function paintTiles(c: CanvasRenderingContext2D, tiles: Tile[], mpp: number): void {
  const marks = tiles.reduce((n, t) => n + marksOf(t, mpp).count, 0), keep = Math.min(1, MOST_MARKS / Math.max(marks, 1));
  // Trees are drawn back to front over every tile, so crowns of neighbouring tiles overlap as one canopy.
  const crowns: { x: number; y: number; r: number; lift: number; colour: Rgb; light: number }[] = [];
  for (const t of tiles) {
    const r = seeded(t.seed), { stand: st, box: [x, y, w, h] } = t, m = marksOf(t, mpp), n = Math.round(m.count * keep);
    // Marks spill half their size past the tile's edges, so neighbouring tiles of one pattern read as one.
    const ex = Math.min(m.size / 2, w / 4), ey = Math.min(m.size / 2, h / 4);
    const at = () => [x - ex + r() * (w + 2 * ex), y - ey + r() * (h + 2 * ey)];
    switch (st.biome) {
      case 'forest': case 'jungle': case 'woodland': {
        // The ground under a canopy is its shade, as dark and as hidden as the canopy is closed.
        if (st.biome !== 'woodland') { c.fillStyle = vegRgb(st.colour, 0.6, 0.85 * st.closed); c.fillRect(x, y, w + 0.5, h + 0.5); }
        const layers = st.biome === 'jungle' ? 3 : 1;
        for (let L = 0; L < layers; L++) for (let k = 0; k < n / layers; k++) {
          const [cx, cy] = at(), size = m.size * (L === 2 ? 1.6 : L === 1 ? 0.75 : 1) * (0.75 + 0.5 * r());
          const tone = L === 1 ? 1.15 : L === 2 ? 0.9 : 1;
          crowns.push({ x: cx, y: cy, r: size, lift: m.lift * (L === 2 ? 1.4 : L === 1 ? 0.5 : 1), colour: st.colour.map(v => v * tone * (0.85 + 0.3 * r())) as Rgb, light: r() });
        }
        break;
      }
      case 'grass': case 'steppe': {
        c.lineWidth = Math.max(0.8, m.size * 0.28); c.lineCap = 'round';
        for (let k = 0; k < n; k++) {
          const [gx, gy] = at(), len = m.size * (0.7 + 0.6 * r()), lean = (r() - 0.5) * 0.9;
          c.strokeStyle = vegRgb(st.colour, 0.75 + 0.5 * r(), 0.75);
          c.beginPath(); c.moveTo(gx, gy); c.lineTo(gx + lean * len, gy - len); c.stroke();
        }
        break;
      }
      case 'tundra': {
        c.lineWidth = Math.max(0.7, m.size * 0.22); c.lineCap = 'round';
        for (let k = 0; k < n; k++) {
          const [gx, gy] = at();
          c.strokeStyle = vegRgb(st.colour, 0.7 + 0.4 * r(), 0.8);
          for (const lean of [-0.6, 0, 0.6]) { c.beginPath(); c.moveTo(gx, gy); c.lineTo(gx + lean * m.size * 0.5, gy - m.size * 0.6); c.stroke(); }
        }
        break;
      }
      case 'desert': {
        for (let k = 0; k < n; k++) {
          const [gx, gy] = at(), rr = m.size * (0.6 + 0.5 * r());
          c.fillStyle = 'rgba(0,0,0,.18)'; c.beginPath(); c.ellipse(gx + rr * 0.4, gy + rr * 0.3, rr, rr * 0.55, 0, 0, 2 * Math.PI); c.fill();
          c.fillStyle = vegRgb(st.colour, 0.9 + 0.3 * r()); c.beginPath(); c.arc(gx, gy - rr * 0.3, rr * 0.8, 0, 2 * Math.PI); c.fill();
        }
        break;
      }
    }
  }
  crowns.sort((a, b) => a.y - b.y);
  // A crown: its shadow cast to the south-east by its height, its round top, a lit side to the north-west.
  for (const k of crowns) {
    c.fillStyle = 'rgba(8,24,12,.32)';
    c.beginPath(); c.ellipse(k.x + k.lift * 0.6, k.y + k.lift * 0.35, k.r, k.r * 0.7, 0, 0, 2 * Math.PI); c.fill();
    c.fillStyle = vegRgb(k.colour);
    c.beginPath(); c.arc(k.x, k.y - k.lift * 0.4, k.r, 0, 2 * Math.PI); c.fill();
    c.fillStyle = vegRgb(k.colour, 1.25 + 0.15 * k.light, 0.9);
    c.beginPath(); c.arc(k.x - k.r * 0.32, k.y - k.lift * 0.4 - k.r * 0.32, k.r * 0.45, 0, 2 * Math.PI); c.fill();
  }
}

/** How many marks a tile holds and their size and lift on screen, px: a crown of a woody stand is about a fifth of its
 * height across and is lifted by its height, an herb's stroke is its height; at least `MIN_MARK_PX` however far the
 * view. As many as fill its cover share of the tile, overlapping a little in a closed canopy. */
function marksOf(t: Tile, mpp: number): { count: number; size: number; lift: number } {
  const st = t.stand, area = t.box[2] * t.box[3], woody = ['forest', 'jungle', 'woodland'].includes(st.biome);
  const size = Math.max(MIN_MARK_PX, (woody ? 1.5 + 0.2 * st.height : st.height) / mpp) * (woody ? 1 : 1.4);
  const lift = woody ? Math.min(size * 0.9, Math.max(1, st.height / mpp * 0.25)) : 0;
  const per = woody ? Math.PI * size * size * 0.8 : st.biome === 'desert' ? size * size * 60 : size * size * 1.2;
  return { count: Math.min(area * st.closed / per, area / 4), size, lift };
}
