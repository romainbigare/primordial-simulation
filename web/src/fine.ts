// The window in view finer than its cells (tape/src/fine/mod.rs), as the fields the GPU draws it from (fields.ts): the
// fine ground and its slope, the liquid's surface over it (so shores follow the ground, not the cells' edges), the
// fine ground's temperature for its snow, how wet the ground is and the plant cover inside each cell
// (tape/src/fine/inside.rs), and the channels the liquid runs in, at their true width and depth
// (tape/src/fine/channels.rs).
import { World, Meta, makeWorld, latOf } from './data';
import { Wet, seaSquares, clamp, POLE_COS } from './earth';
import { Fields, put4 } from './fields';

/** The slots of cover a cell holds at most (core/src/cover `SLOTS`; keep the two in step). */
export const SLOTS = 4;

/** The window of a fine raster: from column x and row y, w x h squares, a pixel a leaf of `depth`; it fades
 * out over `soft` squares along each of its edges that is not the map's, so the map under it shows no hard edge; the
 * layers the core makes for it (names, in table order): each costs the core a field. */
export interface FineAt { x: number; y: number; w: number; h: number; depth: number; soft: number; layers: string[] }

/** The layers a fine raster always holds: those `fineFields` and canopy.ts `fineCover` read. */
export const FINE_LAYERS = ['ice', 't_ground', 'rain', 'plants', 'leaf', 'true_r', 'true_g', 'true_b'];

/** A fine raster as the core hands it out. */
export interface Fine {
  at: FineAt; across: number; down: number;
  /** Pixels a square, and metres a pixel north to south */
  per: number; pxM: number;
  /** The melting point of the frozen cover, K (0: the world has none) */
  melt: number;
  /** Of each pixel: the ground's height and the level of the liquid its own cell holds (NaN: none), m; the share of it a
   * channel covers, 0..1, and that channel's depth, m */
  ground: Float32Array; water: Float32Array; channel: Float32Array; channelDepth: Float32Array;
  /** The layers of the pixels the window holds (`FineAt.layers`), as a world of `across` x `down` cells with one frame */
  world: World;
  /** Of each pixel, inside its own cell: how wet its ground is, 0..1; the built cover of each slot of its cell, kg/m²
   * (`SLOTS` planes, plane k of slot k); the number of its cell among the window's cells */
  wet: Float32Array; cover: Float32Array[]; cell: Uint32Array;
  /** Of each of the window's cells, for each of its slots: its walls and its capture area per kg of its built cover
   * (kg/kg, m²/kg), `2 * SLOTS` numbers a cell */
  make: Float32Array;
}

/** Read the bytes the core made for window `at` of the world of `meta` (tape/src/fine/mod.rs `fine`). */
export function readFine(meta: Meta, at: FineAt, data: Uint8Array): Fine {
  const per = 2 ** (at.depth - (meta.square_depth ?? 0));
  const across = Math.max(1, Math.round(at.w * per)), down = Math.max(1, Math.round(at.h * per)), n = across * down;
  const head = new DataView(data.buffer, data.byteOffset, 4);
  const f32 = (from: number, k: number) => new Float32Array(data.buffer, data.byteOffset + from, k);
  const start = 4, layers = start + 16 * n;
  const held = meta.layers.filter(l => at.layers.includes(l.name));
  const world = makeWorld({ ...meta, w: across, h: down, cell_m: meta.cell_m / per, layers: held, frames: [meta.frames[0]] }, data, layers);
  // The fields inside the cells, from the next multiple of 4 bytes after the layers.
  const inside = Math.ceil((layers + held.length * n) / 4) * 4, numbers = inside + 4 * (1 + SLOTS) * n;
  const cells = new DataView(data.buffer, data.byteOffset + numbers + 4 * n, 4).getUint32(0, true);
  return {
    at, across, down, per, pxM: meta.cell_m / per, melt: head.getFloat32(0, true), ground: f32(start, n), water: f32(start + 4 * n, n),
    channel: f32(start + 8 * n, n), channelDepth: f32(start + 12 * n, n), world,
    wet: f32(inside, n), cover: Array.from({ length: SLOTS }, (_, k) => f32(inside + 4 * (1 + k) * n, n)),
    cell: new Uint32Array(data.buffer, data.byteOffset + numbers, n), make: f32(numbers + 4 * (n + 1), 2 * SLOTS * cells),
  };
}

/** The ground's slope over a pixel is read as a square's slope (for its shading and its rock) times its size over a
 * square's to this power, and no less than SLOPE_LEAST times it: the slope over a short run is steeper than over a long
 * one, and the map shades and colours slopes as they look from afar. */
const SLOPE_POWER = 0.5;
const SLOPE_LEAST = 0.15;
/** Where no liquid stands in a pixel's cell, its surface is taken this far below the ground, m. */
const NO_LIQUID_M = 1000;
/** The bytes of a layer view over which an empty pixel fades into a full one (earth.wgsl `in_view`). */
const OVER_FADE = 64;
/** A sea shore is sand up to this height over the liquid's surface, m. */
const BEACH_M = 6;

/** The fields of fine raster `f` of the world of `meta` (fields.ts), the map's squares being wet and sea as `wet` says:
 * a pixel's share of the sea is that of the squares about it (`seaSquares`: a square of the sea or a dry one beside it),
 * blended between their centres as the squares' own map blends it, so a beach shows no square's edge. Its channels are
 * cut into its ground: a pixel's ground lies lower by the volume a channel takes of it over its area (its share times
 * its depth), so the channel's banks show in the shading, and the water drawing shows the share it covers. */
export function fineFields(f: Fine, meta: Meta, wet: Wet): Fields {
  const { across, down, water, world, per, pxM, channel, channelDepth } = f, n = across * down;
  const ground = f.ground.map((g, i) => g - channel[i] * channelDepth[i]);
  const lut = (name: string) => world.lut[world.index.get(name)!];
  const bytesOf = (name: string) => world.data.subarray(world.base + world.index.get(name)! * n, world.base + (world.index.get(name)! + 1) * n);
  const ice = bytesOf('ice'), tg = bytesOf('t_ground'), plants = bytesOf('plants'), tr = bytesOf('true_r'), tgr = bytesOf('true_g'), tb = bytesOf('true_b');
  const iceLut = lut('ice'), tLut = lut('t_ground'), leafLut = lut('leaf'), leaf = bytesOf('leaf');
  const sea = seaSquares(wet, meta);
  // The sea's share at column x and row y (squares): blended between the centres of the four squares about it.
  const seaAt = (x: number, y: number) => {
    const fx = x - 0.5, fy = y - 0.5, x0 = Math.floor(fx), y0 = Math.floor(fy), tx = fx - x0, ty = fy - y0;
    const at = (c: number, r: number) => sea[clamp(r, 0, meta.h - 1) * meta.w + (meta.globe ? (c + meta.w) % meta.w : clamp(c, 0, meta.w - 1))];
    return (at(x0, y0) * (1 - tx) + at(x0 + 1, y0) * tx) * (1 - ty) + (at(x0, y0 + 1) * (1 - tx) + at(x0 + 1, y0 + 1) * tx) * ty;
  };
  const height = new Float32Array(4 * n), matter = new Float32Array(4 * n), bytes = new Uint8Array(4 * n), inside = new Float32Array(4 * n);
  for (let y = 0; y < down; y++) {
    const ys = f.at.y + (y + 0.5) / per;
    // A globe's pixels are narrower east to west towards the poles.
    const wide = meta.globe ? pxM * Math.max(Math.cos((latOf(meta, ys - 0.5) * Math.PI) / 180), POLE_COS) : pxM;
    for (let x = 0; x < across; x++) {
      const i = y * across + x, level = water[i], wetHere = !Number.isNaN(level);
      const x0 = Math.max(x - 1, 0), x1 = Math.min(x + 1, across - 1), y0 = Math.max(y - 1, 0), y1 = Math.min(y + 1, down - 1);
      const gx = (ground[y * across + x1] - ground[y * across + x0]) / (wide * (x1 - x0 || 1));
      const gy = (ground[y1 * across + x] - ground[y0 * across + x]) / (pxM * (y1 - y0 || 1));
      put4(height, i, ground[i], wetHere ? level - ground[i] : -NO_LIQUID_M, gx, gy);
      put4(matter, i, iceLut[ice[i]], tLut[tg[i]], seaAt(f.at.x + (x + 0.5) / per, ys), channel[i]);
      put4(bytes, i, plants[i], tr[i], tgr[i], tb[i]);
      put4(inside, i, f.wet[i], channelDepth[i], Math.exp(-leafLut[leaf[i]]), 0);
    }
  }
  const look: Fields['look'] = [Math.max((pxM / meta.cell_m) ** SLOPE_POWER, SLOPE_LEAST), f.melt, BEACH_M, OVER_FADE];
  const m = meta.cell_m, place: Fields['place'] = [f.at.x * m, f.at.y * m, f.at.w * m, f.at.h * m];
  const layers = world.data.subarray(world.base, world.base + world.frameBytes), names = world.meta.layers.map(l => l.name);
  return { w: across, h: down, height, matter, bytes, inside, layers, names, look, place };
}
