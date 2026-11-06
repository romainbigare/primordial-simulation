// Turns layers into pixels. The viewer, not the core, decides what counts as sea, river, desert...
import { World, plane, decode, latOf } from './data';

export type RGB = [number, number, number];
const rgb = (h: string): RGB => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16)) as RGB;
const ramps = (...h: string[]) => h.map(rgb);

export const CLASSES = [
  ['Deep sea', '#1f5594'], ['Shallow sea', '#62b6d6'], ['Lake', '#4c92cc'], ['River', '#2f86e6'], ['Snow and ice', '#f3f7fa'],
  ['Mountain', '#8e8c88'], ['Desert', '#dccb98'], ['Grassland', '#9cc16c'], ['Forest', '#487f4a'],
] as const;
const [DEEP, SHALLOW, LAKE, RIVER, ICE, ROCK, DESERT, GRASS, FOREST] = CLASSES.map((_, i) => i);

const SEA = ramps('#8fd6dc', '#4fa8d2', '#2b6cab', '#143d73');
const LAKEC = ramps('#a3d8e8', '#5a9fd2', '#3a73b5');
const VEG = ramps('#e6d29c', '#d9ca8a', '#bccb7c', '#92bd6a', '#5c9b57', '#32714a');
const SAND = rgb('#e2d3a4'), TUNDRA = rgb('#b9bfa8'), UPLAND = rgb('#a89c7c'), SNOW = rgb('#f7fafc'), RIVERC = rgb('#2a82ea');
const ROCK_LO = rgb('#8d8a84'), ROCK_HI = rgb('#cdcac4');
const TURBO = ramps('#30123b', '#4662d7', '#36aaf9', '#1ae4b6', '#72fe5e', '#c7ef34', '#fabb2f', '#f66b19', '#cb2a04', '#7a0403');
const WATER = ramps('#eaf4fb', '#b6dcf0', '#6fb4e0', '#3a82c8', '#1f4f9b', '#12295e');
const GREEN = ramps('#f1f3d0', '#cde27a', '#8fc65a', '#4a9a50', '#1e6a3d', '#0c3d27');
const EARTHY = ramps('#f6ecd3', '#e1c68b', '#c49a5a', '#8e6636', '#4e3418');
const ICY = ramps('#f4f8fb', '#cfe3f2', '#9cc3e6', '#6a96d4', '#3d5fb0');
const VIRIDIS = ramps('#440154', '#414487', '#2a788e', '#22a884', '#7ad151', '#fde725');
const FIRE = ramps('#fcf4dd', '#f6c26b', '#e4793c', '#b0382f', '#5f1a45', '#1e0f2e');

export function rampFor(name: string): RGB[] {
  if (/^t_|temp/.test(name)) return TURBO;
  if (/rain|vapor|depth|flux|water/.test(name)) return WATER;
  if (/plant|forest|grass|crop/.test(name)) return GREEN;
  if (/loose|sediment|soil/.test(name)) return EARTHY;
  if (/ice|snow/.test(name)) return ICY;
  if (/absorb|fire|heat|energy/.test(name)) return FIRE;
  return VIRIDIS;
}
export const rampCss = (r: RGB[]) => `linear-gradient(90deg,${r.map(c => `rgb(${c})`).join(',')})`;

export const clamp = (x: number, a = 0, b = 1) => (x < a ? a : x > b ? b : x);
export const smooth = (a: number, b: number, x: number) => { const t = clamp((x - a) / (b - a)); return t * t * (3 - 2 * t); };
function ramp(r: RGB[], t: number, o: number[]): void {
  const x = clamp(t) * (r.length - 1), i = Math.min(Math.floor(x), r.length - 2), f = x - i;
  for (let k = 0; k < 3; k++) o[k] = r[i][k] + (r[i + 1][k] - r[i][k]) * f;
}
const mix = (o: number[], c: RGB, t: number) => { for (let k = 0; k < 3; k++) o[k] += (c[k] - o[k]) * t; };

const WET = 2; // metres of liquid that make a cell water
// A river carries at least RIVER_CATCH (10) times what its own area sheds (the `catch` layer) and RIVER_FLOOR (9e3) mol/s:
// tape/src/layers.rs RIVER_CATCH and RIVER_FLOOR (keep the three in step). It fades in over these log10 spans, half at the value.
const RIVER_CATCH_LOG = [0.7, 1.3];
const RIVER_FLOOR_LOG = [3.6, 4.3];
const L = [-0.588, -0.588, 0.647]; // light from the north-west
/** How many times steeper than it is the ground of the map, a square a pixel, is shaded. */
export const EXAGGERATE = 6;
const SEA_SHARE = 0.01; // a wet region of a globe with this share of the cells is sea, a smaller one a lake
const POLE_COS = 0.25; // a globe's east-west cell width, as a share of the equator's, is not taken below this when shading

/**
 * The sea: on an island, the wet cells connected to the map border; the others are lakes. A globe has no border, so
 * its sea is each wet region of at least SEA_SHARE of the cells. A globe's rows wrap east to west.
 */
function oceanFill(wet: Uint8Array, W: number, H: number, globe: boolean): Uint8Array {
  const sea = new Uint8Array(W * H), seen = new Uint8Array(W * H);
  for (let start = 0; start < W * H; start++) {
    if (!wet[start] || seen[start]) continue;
    const region = [start];
    seen[start] = 1;
    let border = false;
    for (let k = 0; k < region.length; k++) {
      const x = region[k] % W, y = (region[k] / W) | 0;
      border ||= x === 0 || y === 0 || x === W - 1 || y === H - 1;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
        const nx = globe ? (x + dx + W) % W : x + dx, ny = y + dy, j = ny * W + nx;
        if (nx >= 0 && ny >= 0 && nx < W && ny < H && wet[j] && !seen[j]) { seen[j] = 1; region.push(j); }
      }
    }
    if (globe ? region.length >= SEA_SHARE * W * H : border) for (const i of region) sea[i] = 1;
  }
  return sea;
}

/** Where movers wore paths (the `worn` layer: the most cleared link of the cell), drawn over the map. */
export const PATH_LAYER = 'worn';
const PATH = rgb('#e8590c');

/** What the base map's colour of a spot reads: a cell of the map, or a pixel of a raster finer than the cells (fine.ts). */
export interface Spot {
  /** Depth of the liquid standing there, m; whether it is the sea's; the share of the spot it covers, 0..1 */
  depth: number; sea: boolean; cover: number;
  /** Snow and ice on it, 0..1 */
  snow: number;
  /** Ground temperature, K; the byte of the `plants` layer; the ground's height, m; its slope (rise over run) */
  t: number; plants: number; z: number; slope: number;
  /** The light on the ground from the north-west over the light on flat ground (`hillShade`) */
  shade: number;
  /** Beach sand on it and a river through it, 0..1 */
  sand: number; river: number;
}

/** The light from the north-west on ground of slope (gx, gy) (east, south), made `exaggerate` times steeper, over the
 * light on flat ground. */
export function hillShade(gx: number, gy: number, exaggerate: number): number {
  const nx = -exaggerate * gx, ny = -exaggerate * gy;
  return ((nx * L[0] + ny * L[1] + L[2]) / Math.sqrt(nx * nx + ny * ny + 1)) / L[2];
}

const landRgb = [0, 0, 0], waterRgb = [0, 0, 0], rockRgb: RGB = [0, 0, 0], riverRgb: RGB = [0, 0, 0];
/** The shaded colour of spot `p` into `c`, its shade's multiplier into `m[0]`; returns its map class (the water's where
 * the liquid covers half of it or more). */
export function colour(p: Spot, c: number[], m: number[]): number {
  const k = clamp(p.cover);
  let wetCls = LAKE, dryCls = DESERT, wetM = 1, dryM = 1;
  if (k > 0) {
    if (p.sea) { ramp(SEA, 1 - Math.exp(-p.depth / 350), waterRgb); wetCls = p.depth > 200 ? DEEP : SHALLOW; }
    else ramp(LAKEC, 1 - Math.exp(-p.depth / 60), waterRgb);
    mix(waterRgb, SNOW, p.snow * 0.85);
    if (p.snow > 0.5) wetCls = ICE;
    wetM = clamp(1 + (p.shade - 1) * (0.25 + 0.5 * p.snow), 0.5, 1.45);
    for (let j = 0; j < 3; j++) waterRgb[j] *= wetM;
  }
  if (k < 1) {
    const v = Math.sqrt(p.plants / 255);
    ramp(VEG, v, landRgb);
    dryCls = v < 0.3 ? DESERT : v < 0.62 ? GRASS : FOREST;
    mix(landRgb, TUNDRA, smooth(278, 266, p.t) * 0.7);
    mix(landRgb, UPLAND, smooth(1200, 2600, p.z) * 0.5);
    if (p.sand > 0) mix(landRgb, SAND, p.sand);
    const rock = Math.max(smooth(0.14, 0.42, p.slope) * smooth(150, 700, p.z), smooth(2000, 3200, p.z) * 0.85) * (1 - 0.5 * v);
    if (rock > 0) {
      const hi = smooth(1500, 4000, p.z);
      for (let j = 0; j < 3; j++) rockRgb[j] = ROCK_LO[j] + (ROCK_HI[j] - ROCK_LO[j]) * hi;
      mix(landRgb, rockRgb, rock);
    }
    if (rock > 0.5) dryCls = ROCK;
    mix(landRgb, SNOW, p.snow); if (p.snow > 0.5) dryCls = ICE;
    dryM = clamp(1 + (p.shade - 1) * (0.8 + 0.3 * p.snow), 0.5, 1.45);
    for (let j = 0; j < 3; j++) landRgb[j] *= dryM;
    const river = p.river * (1 - 0.6 * p.snow);
    if (river > 0) {
      for (let j = 0; j < 3; j++) riverRgb[j] = RIVERC[j] * (0.7 + 0.3 * dryM);
      mix(landRgb, riverRgb, river * 0.9);
    }
    if (river > 0.5) dryCls = RIVER;
  }
  for (let j = 0; j < 3; j++) c[j] = landRgb[j] + (waterRgb[j] - landRgb[j]) * k;
  m[0] = k >= 0.5 ? wetM : dryM;
  return k >= 0.5 ? wetCls : dryCls;
}

/** What a view reads besides the base map: the true colour, the layer of a layer view (none in the Earth and true
 * views), the bytes of it over which an empty spot (byte 0) fades into a full one (1: none; a blend of an empty cell and
 * a full one fades along its own lines, not along the cells'), and the worn paths (none when not drawn). */
export interface Over { mode: string; tr: Uint8Array; tg: Uint8Array; tb: Uint8Array; over: Uint8Array | null; lut: Float32Array | null; ramp: RGB[]; fade: number; worn: Uint8Array | null }
export function overOf(w: World, f: number, mode: string, paths: boolean, fade = 1): Over {
  const over = ['earth', 'true'].includes(mode) ? null : plane(w, f, mode);
  return {
    mode, tr: plane(w, f, 'true_r'), tg: plane(w, f, 'true_g'), tb: plane(w, f, 'true_b'),
    over, lut: over ? w.lut[w.index.get(mode)!] : null, ramp: rampFor(mode), fade,
    worn: paths && w.index.has(PATH_LAYER) ? plane(w, f, PATH_LAYER) : null,
  };
}

const overRgb = [0, 0, 0];
/** Spot `i` in view `o` over its base map colour `c` (of shade multiplier `m`), into `out` from byte `at`. */
export function put(o: Over, i: number, c: number[], m: number, out: Uint8ClampedArray, at: number): void {
  let [r, g, b] = c;
  if (o.mode === 'true') {
    const mt = clamp(1 + (m - 1) * 0.7, 0.55, 1.4);
    r = o.tr[i] * mt; g = o.tg[i] * mt; b = o.tb[i] * mt;
  } else if (o.over && o.lut) {
    // Empty (byte 0): the base map faded; full: the layer's colour over a little of the base map.
    const full = o.lut[o.over[i]] === 0 ? 0 : clamp(o.over[i] / o.fade);
    const grey = 0.3 * r + 0.59 * g + 0.11 * b, mo = 0.72 * (0.75 + 0.25 * m);
    ramp(o.ramp, o.over[i] / 255, overRgb);
    r += ((r * 0.5 + grey * 0.5) * 0.6 + 100 - r) * (1 - full) + (overRgb[0] * mo - r * 0.72) * full;
    g += ((g * 0.5 + grey * 0.5) * 0.6 + 100 - g) * (1 - full) + (overRgb[1] * mo - g * 0.72) * full;
    b += ((b * 0.5 + grey * 0.5) * 0.6 + 100 - b) * (1 - full) + (overRgb[2] * mo - b * 0.72) * full;
  }
  if (o.worn && o.worn[i] > 1) { // a path (byte 1 is the uncleared ground, 1x): stronger the more cleared
    const t = 0.4 + 0.5 * ((o.worn[i] - 1) / 254);
    r += (PATH[0] - r) * t; g += (PATH[1] - g) * t; b += (PATH[2] - b) * t;
  }
  out[at] = r; out[at + 1] = g; out[at + 2] = b; out[at + 3] = 255;
}

/** How much a spot of liquid flux `flux`, mol/s, and catchment `catchment` (the `catch` layer) is a river, 0..1. */
export const riverOf = (flux: number, catchment: number) =>
  Math.min(smooth(RIVER_CATCH_LOG[0], RIVER_CATCH_LOG[1], Math.log10(catchment + 1)), smooth(RIVER_FLOOR_LOG[0], RIVER_FLOOR_LOG[1], Math.log10(flux + 1)));
/** The colour of a river. */
export const RIVER_RGB = RIVERC;

/** The squares of the map that hold standing liquid, and those of them that are the sea's (a byte each). */
export interface Wet { wet: Uint8Array; sea: Uint8Array }
/** The wet squares and the sea of frame f. */
export function wetOf(w: World, f: number): Wet {
  const wet = Uint8Array.from(decode(w, f, 'depth'), d => (d >= WET ? 1 : 0));
  return { wet, sea: oceanFill(wet, w.meta.w, w.meta.h, w.meta.globe) };
}

/** Fill `img` for frame f, a pixel a square. mode: 'earth', 'true' or a layer name. `cls` receives the map class of every
 * square. `paths` draws the worn links over it. */
export function paint(w: World, f: number, mode: string, img: ImageData, cls: Uint8Array, paths: boolean): void {
  const { w: W, h: H, cell_m, globe } = w.meta;
  const z = decode(w, f, 'z'), depth = decode(w, f, 'depth'), ice = decode(w, f, 'ice'), tg = decode(w, f, 't_ground');
  const flux = decode(w, f, 'flux'), catchment = decode(w, f, 'catch'), plants = plane(w, f, 'plants');
  const o = overOf(w, f, mode, paths), { wet, sea } = wetOf(w, f);
  const wrapX = (x: number) => (globe ? (x + W) % W : x);
  const zAt = (x: number, y: number) => z[clamp(y, 0, H - 1) * W + clamp(wrapX(x), 0, W - 1)];
  const nearSea = (x: number, y: number) => {
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
      const nx = wrapX(x + dx), ny = y + dy;
      if (nx >= 0 && nx < W && ny >= 0 && ny < H && sea[ny * W + nx]) return true;
    }
    return false;
  };
  const c = [0, 0, 0], m = [1], out = img.data;
  const p: Spot = { depth: 0, sea: false, cover: 0, snow: 0, t: 0, plants: 0, z: 0, slope: 0, shade: 1, sand: 0, river: 0 };
  for (let i = 0; i < W * H; i++) {
    const x = i % W, y = (i / W) | 0;
    // A globe's cells are narrower east to west towards the poles.
    const width = globe ? cell_m * Math.max(Math.cos((latOf(w.meta, y) * Math.PI) / 180), POLE_COS) : cell_m;
    const gx = (zAt(x + 1, y - 1) + 2 * zAt(x + 1, y) + zAt(x + 1, y + 1) - zAt(x - 1, y - 1) - 2 * zAt(x - 1, y) - zAt(x - 1, y + 1)) / (8 * width);
    const gy = (zAt(x - 1, y + 1) + 2 * zAt(x, y + 1) + zAt(x + 1, y + 1) - zAt(x - 1, y - 1) - 2 * zAt(x, y - 1) - zAt(x + 1, y - 1)) / (8 * cell_m);
    p.slope = Math.hypot(gx, gy); p.shade = hillShade(gx, gy, EXAGGERATE);
    p.depth = depth[i]; p.sea = sea[i] === 1; p.cover = wet[i]; p.snow = smooth(0.02, 0.5, ice[i]);
    p.t = tg[i]; p.plants = plants[i]; p.z = z[i];
    p.sand = !wet[i] && nearSea(x, y) ? 0.55 * (1 - smooth(0, 40, z[i])) : 0;
    p.river = wet[i] ? 0 : riverOf(flux[i], catchment[i]);
    cls[i] = colour(p, c, m);
    put(o, i, c, m[0], out, i * 4);
  }
}
