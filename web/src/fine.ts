// The window in view finer than its cells (tape/src/fine/mod.rs), as the fields the GPU draws it from (fields.ts): the
// fine ground and its slope, the liquid's surface over it (so shores follow the ground, not the cells' edges), the
// fine ground's temperature for its snow, how wet the ground is and the plant cover inside each cell
// (tape/src/fine/inside.rs), and rivers as lines along the way their liquid runs.
import { World, Meta, makeWorld, latOf, encodeByte } from './data';
import { Wet, riverOf, riverWidthOf, riverDepthOf, seaSquares, clamp, POLE_COS } from './earth';
import { Fields, put4 } from './fields';

/** The slots of cover a cell holds at most (core/src/cover `SLOTS`; keep the two in step). */
export const SLOTS = 4;

/** The window of a fine raster: from column x and row y, w x h squares, a pixel a leaf of `depth`; it fades
 * out over `soft` squares along each of its edges that is not the map's, so the map under it shows no hard edge. */
export interface FineAt { x: number; y: number; w: number; h: number; depth: number; soft: number }

/** A fine raster as the core hands it out. */
export interface Fine {
  at: FineAt; across: number; down: number;
  /** Pixels a square, and metres a pixel north to south */
  per: number; pxM: number;
  /** The melting point of the frozen cover, K (0: the world has none) */
  melt: number;
  /** The ways rivers run: x0, y0, x1, y1 (squares), flux (mol/s) and catchment, six numbers a way */
  ways: Float32Array;
  /** Of each pixel: the ground's height and the surface of the liquid of the cells about it (NaN: none), m */
  ground: Float32Array; water: Float32Array;
  /** The layers of the pixels, as a world of `across` x `down` cells with one frame */
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
  const head = new DataView(data.buffer, data.byteOffset, 8), count = head.getUint32(4, true);
  const f32 = (from: number, k: number) => new Float32Array(data.buffer, data.byteOffset + from, k);
  const start = 8 + 24 * count, layers = start + 8 * n;
  const world = makeWorld({ ...meta, w: across, h: down, cell_m: meta.cell_m / per, frames: [meta.frames[0]] }, data, layers);
  // The fields inside the cells, from the next multiple of 4 bytes after the layers.
  const inside = Math.ceil((layers + meta.layers.length * n) / 4) * 4, numbers = inside + 4 * (1 + SLOTS) * n;
  const cells = new DataView(data.buffer, data.byteOffset + numbers + 4 * n, 4).getUint32(0, true);
  return {
    at, across, down, per, pxM: meta.cell_m / per, melt: head.getFloat32(0, true), ways: f32(8, 6 * count), ground: f32(start, n), water: f32(start + 4 * n, n), world,
    wet: f32(inside, n), cover: Array.from({ length: SLOTS }, (_, k) => f32(inside + 4 * (1 + k) * n, n)),
    cell: new Uint32Array(data.buffer, data.byteOffset + numbers, n), make: f32(numbers + 4 * (n + 1), 2 * SLOTS * cells),
  };
}

/** The built cover of all the slots of pixel `i` of `f`, kg/m². */
export const coverAt = (f: Fine, i: number) => f.cover.reduce((t, c) => t + c[i], 0);

/** The ground's slope over a pixel is read as a square's slope (for its shading and its rock) times its size over a
 * square's to this power, and no less than SLOPE_LEAST times it: the slope over a short run is steeper than over a long
 * one, and the map shades and colours slopes as they look from afar. */
const SLOPE_POWER = 0.5;
const SLOPE_LEAST = 0.15;
/** Where no liquid stands near a pixel, its surface is taken this far below the ground, m (the core's surface there
 * lies hundreds of metres below the ground, tape/src/fine/mod.rs `WET_LEAST`). */
const NO_LIQUID_M = 1000;
/** The bytes of a layer view over which an empty pixel fades into a full one (earth.wgsl `in_view`). */
const OVER_FADE = 64;
/** A sea shore is sand up to this height over the liquid's surface, m. */
const BEACH_M = 6;
/** A river's depth is drawn in steps of this, m, up to 255 of them. */
const RIVER_DEPTH_STEP_M = 0.1;
/** A river is drawn at least this wide, pixels, and this much wider for each tenfold of flux over RIVER_FLUX_REF mol/s. */
const RIVER_MIN_PX = 0.7;
const RIVER_PER_DECADE_PX = 0.5;
const RIVER_FLUX_REF = 1e4;
/** A river's middle bends to the lowest ground within this share of its length either side of its straight line. */
const BEND_SHARE = 0.3;
const BEND_STEPS = 6;
/** A way less a river than this is not drawn. */
const RIVER_LEAST = 0.05;

/** The fields of fine raster `f` of the world of `meta` (fields.ts), the map's squares being wet and sea as `wet` says:
 * a pixel is in the sea where liquid stands near it and its square is in the sea (`seaSquares`). */
export function fineFields(f: Fine, meta: Meta, wet: Wet): Fields {
  const { across, down, ground, water, world, per, pxM } = f, n = across * down;
  const lut = (name: string) => world.lut[world.index.get(name)!];
  const bytesOf = (name: string) => world.data.subarray(world.base + world.index.get(name)! * n, world.base + (world.index.get(name)! + 1) * n);
  const ice = bytesOf('ice'), tg = bytesOf('t_ground'), tr = bytesOf('true_r'), tgr = bytesOf('true_g'), tb = bytesOf('true_b');
  const iceLut = lut('ice'), tLut = lut('t_ground'), plants = meta.layers[world.index.get('plants')!];
  const rivers = riverShares(f), sea = seaSquares(wet, meta);
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
      const sq = clamp(Math.floor(ys), 0, meta.h - 1) * meta.w + clamp(Math.floor(f.at.x + (x + 0.5) / per), 0, meta.w - 1);
      put4(matter, i, iceLut[ice[i]], tLut[tg[i]], wetHere ? sea[sq] : 0, rivers[4 * i + 3] / 255);
      // The plant cover drawn is the cover inside the pixel's cell where it stands (its slots' built cover).
      put4(bytes, i, encodeByte(plants, coverAt(f, i)), tr[i], tgr[i], tb[i]);
      put4(inside, i, f.wet[i], rivers[4 * i] * RIVER_DEPTH_STEP_M, 0, 0);
    }
  }
  const look: Fields['look'] = [Math.max((pxM / meta.cell_m) ** SLOPE_POWER, SLOPE_LEAST), f.melt, BEACH_M, OVER_FADE];
  const m = meta.cell_m, place: Fields['place'] = [f.at.x * m, f.at.y * m, f.at.w * m, f.at.h * m];
  return { w: across, h: down, height, matter, bytes, inside, layers: world.data.subarray(world.base), look, place };
}

/** Of each pixel of `f`, four bytes: its river's depth in steps of RIVER_DEPTH_STEP_M (as deep as its discharge makes
 * it, `riverDepthOf`), two unused, and the share of it that is a river: each way a curve from its cell's centre to the
 * next cell's, its middle bent to the lowest ground beside its straight line, smooth through the centres where one way
 * leads into the next, as wide as its discharge makes it or at least RIVER_MIN_PX and more for more flux, as strong as
 * its cell is a river. */
function riverShares(f: Fine): Uint8ClampedArray {
  const { ways, at, per, across, down, ground, pxM } = f, count = ways.length / 6;
  const cv = new OffscreenCanvas(across, down), ctx = cv.getContext('2d', { willReadFrequently: true })!;
  const key = (x: number, y: number) => `${Math.round(x * 4096)},${Math.round(y * 4096)}`;
  const from = new Map<string, number>(), fed = new Set<string>();
  for (let k = 0; k < count; k++) { from.set(key(ways[6 * k], ways[6 * k + 1]), k); fed.add(key(ways[6 * k + 2], ways[6 * k + 3])); }
  const px = (k: number, e: number): [number, number] => [(ways[6 * k + e] - at.x) * per, (ways[6 * k + e + 1] - at.y) * per];
  const groundAt = (x: number, y: number) => ground[clamp(Math.round(y - 0.5), 0, down - 1) * across + clamp(Math.round(x - 0.5), 0, across - 1)];
  // The bent middle of each way.
  const bend: [number, number][] = [];
  for (let k = 0; k < count; k++) {
    const [x0, y0] = px(k, 0), [x1, y1] = px(k, 2), len = Math.hypot(x1 - x0, y1 - y0) || 1;
    const mx = (x0 + x1) / 2, my = (y0 + y1) / 2, nx = -(y1 - y0) / len, ny = (x1 - x0) / len;
    let best: [number, number] = [mx, my], low = groundAt(mx, my);
    for (let s = -BEND_STEPS; s <= BEND_STEPS; s++) {
      const t = (s / BEND_STEPS) * BEND_SHARE * len, bx = mx + nx * t, by = my + ny * t, z = groundAt(bx, by);
      if (z < low) { low = z; best = [bx, by]; }
    }
    bend.push(best);
  }
  ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  const mid = (a: [number, number], b: [number, number]): [number, number] => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  for (let k = 0; k < count; k++) {
    const flux = ways[6 * k + 4], strength = riverOf(flux, ways[6 * k + 5]);
    if (strength < RIVER_LEAST) continue;
    const a = px(k, 0), b = px(k, 2), m = bend[k], next = from.get(key(ways[6 * k + 2], ways[6 * k + 3]));
    ctx.lineWidth = Math.max(riverWidthOf(flux) / pxM, RIVER_MIN_PX + RIVER_PER_DECADE_PX * Math.max(0, Math.log10(flux / RIVER_FLUX_REF)));
    ctx.strokeStyle = `rgba(${Math.min(Math.round(riverDepthOf(flux) / RIVER_DEPTH_STEP_M), 255)},0,0,${strength})`;
    ctx.beginPath();
    const start = mid(a, m), end = mid(m, b);
    if (!fed.has(key(ways[6 * k], ways[6 * k + 1]))) { ctx.moveTo(a[0], a[1]); ctx.lineTo(start[0], start[1]); }
    else ctx.moveTo(start[0], start[1]);
    ctx.quadraticCurveTo(m[0], m[1], end[0], end[1]);
    if (next !== undefined) { const on = mid(b, bend[next]); ctx.quadraticCurveTo(b[0], b[1], on[0], on[1]); }
    else ctx.lineTo(b[0], b[1]);
    ctx.stroke();
  }
  return ctx.getImageData(0, 0, across, down).data;
}

