// The window in view finer than its cells (tape/src/fine.rs), painted as the base map: hill shading from the fine
// ground, the liquid where its surface lies over the fine ground (so shores follow the ground, not the cells' edges),
// snow by the fine ground's temperature, and rivers as lines along the way their liquid runs.
import { World, Meta, makeWorld, latOf } from './data';
import { Spot, Wet, colour, hillShade, overOf, put, riverOf, RIVER_RGB, EXAGGERATE, clamp, smooth } from './earth';

/** The window of a fine raster: from column x and row y, w x h squares, a pixel a leaf of `depth`; its picture fades
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
}

/** Read the bytes the core made for window `at` of the world of `meta` (tape/src/fine.rs `fine`). */
export function readFine(meta: Meta, at: FineAt, data: Uint8Array): Fine {
  const per = 2 ** (at.depth - (meta.square_depth ?? 0));
  const across = Math.max(1, Math.round(at.w * per)), down = Math.max(1, Math.round(at.h * per)), n = across * down;
  const head = new DataView(data.buffer, data.byteOffset, 8), count = head.getUint32(4, true);
  const f32 = (from: number, k: number) => new Float32Array(data.buffer, data.byteOffset + from, k);
  const start = 8 + 24 * count;
  const world = makeWorld({ ...meta, w: across, h: down, cell_m: meta.cell_m / per, frames: [meta.frames[0]] }, data, start + 8 * n);
  return { at, across, down, per, pxM: meta.cell_m / per, melt: head.getFloat32(0, true), ways: f32(8, 6 * count), ground: f32(start, n), water: f32(start + 4 * n, n), world };
}

/** The ground's slope over a pixel is read as a square's slope (for its shading and its rock) times its size over a
 * square's to this power, and no less than SLOPE_LEAST times it: the slope over a short run is steeper than over a long
 * one, and the map shades and colours slopes as they look from afar. */
const SLOPE_POWER = 0.5;
const SLOPE_LEAST = 0.15;
/** The least rise across a pixel a shore is smoothed over, m. */
const SHORE_RISE_M = 0.05;
/** The bytes of a layer view over which an empty pixel fades into a full one (earth.ts `Over`). */
const OVER_FADE = 64;
/** A sea shore is sand up to this height over the liquid's surface, m. */
const BEACH_M = 6;
/** Snow lies by the pixel's own temperature where that is within these of the cover's melting point, K (fading
 * between), and the cell's cover elsewhere. */
const SNOW_NEAR_K = [2, 6];
/** A river's width grows as the square root of its discharge (hydraulic geometry, Leopold and Maddock 1953): this
 * many m at 1 m³/s; a mol of the common liquid takes LIQUID_M3 m³. */
const RIVER_WIDTH_M = 3;
const LIQUID_M3 = 1.8e-5;
/** A river is drawn at least this wide, pixels, and this much wider for each tenfold of flux over RIVER_FLUX_REF mol/s. */
const RIVER_MIN_PX = 0.7;
const RIVER_PER_DECADE_PX = 0.5;
const RIVER_FLUX_REF = 1e4;
/** A river's middle bends to the lowest ground within this share of its length either side of its straight line. */
const BEND_SHARE = 0.3;
const BEND_STEPS = 6;

/** Whether the pixel at (x, y) squares is in the sea: in a square of the sea, or in a dry square beside one. */
function seaAt(wet: Wet, meta: Meta, x: number, y: number): boolean {
  const W = meta.w, H = meta.h, sx = clamp(Math.floor(x), 0, W - 1), sy = clamp(Math.floor(y), 0, H - 1), i = sy * W + sx;
  if (wet.sea[i]) return true;
  if (wet.wet[i]) return false;
  for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
    const nx = meta.globe ? (sx + dx + W) % W : sx + dx, ny = sy + dy;
    if (nx >= 0 && ny >= 0 && nx < W && ny < H && wet.sea[ny * W + nx]) return true;
  }
  return false;
}

/** The fine raster `f` of the world of `meta` painted in view `mode` (with the worn paths if `paths`), the map's squares
 * being wet and sea as `wet` says. In the Earth view the land comes first, then the rivers, then the liquid over them;
 * other views paint the liquid with the land. */
export function paintFine(f: Fine, meta: Meta, mode: string, paths: boolean, wet: Wet): OffscreenCanvas {
  const { across, down, ground, water, world, per, pxM } = f, n = across * down;
  const earth = mode === 'earth';
  const o = overOf(world, 0, mode, paths, OVER_FADE);
  const lut = (name: string) => world.lut[world.index.get(name)!];
  const bytes = (name: string) => world.data.subarray(world.base + world.index.get(name)! * n, world.base + (world.index.get(name)! + 1) * n);
  const ice = bytes('ice'), tg = bytes('t_ground'), plants = bytes('plants');
  const iceLut = lut('ice'), tLut = lut('t_ground');
  const land = new ImageData(across, down), top = earth ? new ImageData(across, down) : land;
  const flatten = Math.max((pxM / meta.cell_m) ** SLOPE_POWER, SLOPE_LEAST);
  const c = [0, 0, 0], m = [1];
  const p: Spot = { depth: 0, sea: false, cover: 0, snow: 0, t: 0, plants: 0, z: 0, slope: 0, shade: 1, sand: 0, river: 0 };
  for (let y = 0; y < down; y++) {
    const ys = f.at.y + (y + 0.5) / per;
    // A globe's pixels are narrower east to west towards the poles.
    const wide = meta.globe ? pxM * Math.max(Math.cos((latOf(meta, ys - 0.5) * Math.PI) / 180), 0.25) : pxM;
    for (let x = 0; x < across; x++) {
      const i = y * across + x, g = ground[i];
      const gx = (ground[y * across + Math.min(x + 1, across - 1)] - ground[y * across + Math.max(x - 1, 0)]) / (wide * (Math.min(x + 1, across - 1) - Math.max(x - 1, 0) || 1));
      const gy = (ground[Math.min(y + 1, down - 1) * across + x] - ground[Math.max(y - 1, 0) * across + x]) / (pxM * (Math.min(y + 1, down - 1) - Math.max(y - 1, 0) || 1));
      const slope = Math.sqrt(gx * gx + gy * gy), level = water[i], depth = level - g;
      p.slope = slope * flatten; p.shade = hillShade(gx, gy, EXAGGERATE * flatten); p.z = g;
      p.t = tLut[tg[i]]; p.plants = plants[i];
      const iceI = smooth(0.02, 0.5, iceLut[ice[i]]);
      const near = f.melt > 0 ? 1 - smooth(SNOW_NEAR_K[0], SNOW_NEAR_K[1], Math.abs(p.t - f.melt)) : 0;
      p.snow = iceI + (smooth(f.melt + 1, f.melt - 1, p.t) - iceI) * near;
      const cover = Number.isNaN(level) ? 0 : clamp(0.5 + depth / Math.max(slope * pxM, SHORE_RISE_M));
      p.sea = !Number.isNaN(level) && seaAt(wet, meta, f.at.x + (x + 0.5) / per, ys);
      p.depth = Math.max(depth, 0);
      p.sand = p.sea && depth < 0 ? 0.55 * (1 - smooth(0, BEACH_M, -depth)) : 0;
      p.cover = earth ? 0 : cover;
      colour(p, c, m);
      put(o, i, c, m[0], land.data, i * 4);
      if (earth && cover > 0) {
        p.cover = 1;
        colour(p, c, m);
        put(o, i, c, m[0], top.data, i * 4);
        top.data[i * 4 + 3] = 255 * cover;
      }
    }
  }
  const cv = new OffscreenCanvas(across, down), ctx = cv.getContext('2d')!;
  ctx.putImageData(land, 0, 0);
  if (earth) {
    drawRivers(ctx, f);
    const over = new OffscreenCanvas(across, down);
    over.getContext('2d')!.putImageData(top, 0, 0);
    ctx.drawImage(over, 0, 0);
  }
  soften(ctx, f, meta);
  return cv;
}

/** Fade the picture on `ctx` of raster `f` out along each edge of its window that is not the map's, over `soft`
 * squares (fine.ts `FineAt`). */
function soften(ctx: OffscreenCanvasRenderingContext2D, f: Fine, meta: Meta): void {
  const { at, across, down, per } = f, band = at.soft * per;
  if (band <= 0) return;
  // The share kept at pixel `k` of `n` along an axis whose window starts at `from` squares and takes `w` of a map `wide`.
  const ramp = (n: number, from: number, w: number, wide: number) => Float32Array.from({ length: n }, (_, k) => {
    const west = from > 0 ? smooth(0, band, k + 0.5) : 1, east = from + w < wide ? smooth(0, band, n - k - 0.5) : 1;
    return west * east;
  });
  const ax = ramp(across, at.x, at.w, meta.w), ay = ramp(down, at.y, at.h, meta.h);
  const img = ctx.getImageData(0, 0, across, down), d = img.data;
  for (let y = 0; y < down; y++) for (let x = 0; x < across; x++) d[(y * across + x) * 4 + 3] *= ax[x] * ay[y];
  ctx.putImageData(img, 0, 0);
}

/** The rivers of `f` onto `ctx` (its pixels): each way a curve from its cell's centre to the next cell's, its middle
 * bent to the lowest ground beside its straight line, smooth through the centres where one way leads into the next, as
 * wide as its discharge makes it or at least RIVER_MIN_PX and more for more flux, as strong as its cell is a river. */
function drawRivers(ctx: OffscreenCanvasRenderingContext2D, f: Fine): void {
  const { ways, at, per, across, down, ground, pxM } = f, count = ways.length / 6;
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
  ctx.save();
  ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  const mid = (a: [number, number], b: [number, number]): [number, number] => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  for (let k = 0; k < count; k++) {
    const flux = ways[6 * k + 4], strength = riverOf(flux, ways[6 * k + 5]);
    if (strength < 0.05) continue;
    const a = px(k, 0), b = px(k, 2), m = bend[k], next = from.get(key(ways[6 * k + 2], ways[6 * k + 3]));
    const wide = (RIVER_WIDTH_M * Math.sqrt(flux * LIQUID_M3)) / pxM;
    ctx.lineWidth = Math.max(wide, RIVER_MIN_PX + RIVER_PER_DECADE_PX * Math.max(0, Math.log10(flux / RIVER_FLUX_REF)));
    ctx.strokeStyle = `rgba(${RIVER_RGB.join(',')},${0.9 * strength})`;
    ctx.beginPath();
    const start = mid(a, m), end = mid(m, b);
    if (!fed.has(key(ways[6 * k], ways[6 * k + 1]))) { ctx.moveTo(a[0], a[1]); ctx.lineTo(start[0], start[1]); }
    else ctx.moveTo(start[0], start[1]);
    ctx.quadraticCurveTo(m[0], m[1], end[0], end[1]);
    if (next !== undefined) { const on = mid(b, bend[next]); ctx.quadraticCurveTo(b[0], b[1], on[0], on[1]); }
    else ctx.lineTo(b[0], b[1]);
    ctx.stroke();
  }
  ctx.restore();
}
