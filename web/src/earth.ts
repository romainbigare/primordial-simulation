// What the map makes of a frame's layers before the GPU colours it (web/shaders/lib/earth.wgsl): the fields of the map's
// squares, the sea, the rivers, and the classes and ramps the page names and draws. The viewer, not the core, decides
// what counts as sea, river, desert...
import { World, Meta, plane, decode, latOf } from './data';
import { Fields, put4 } from './fields';

export type RGB = [number, number, number];
const rgb = (h: string): RGB => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16)) as RGB;
const ramps = (...h: string[]) => h.map(rgb);

export const CLASSES = [
  ['Deep sea', '#1f5594'], ['Shallow sea', '#62b6d6'], ['Lake', '#4c92cc'], ['River', '#2f86e6'], ['Snow and ice', '#f3f7fa'],
  ['Mountain', '#8e8c88'], ['Bare ground', '#a89a80'], ['Plant cover', '#5f8f4a'],
] as const;
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

const WET = 2; // metres of liquid that make a square water
const SEA_SHARE = 0.01; // a wet region of a globe with this share of the cells is sea, a smaller one a lake
export const POLE_COS = 0.25; // a globe's east-west cell width, as a share of the equator's, is not taken below this when shading
/** A shore of a square by the sea is sand up to this height over the liquid, m (a square's height is its mean). */
const SQUARE_BEACH_M = 40;

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

/** The squares of the map that hold standing liquid, and those of them that are the sea's (a byte each). */
export interface Wet { wet: Uint8Array; sea: Uint8Array }
/** The wet squares and the sea of frame f. */
export function wetOf(w: World, f: number): Wet {
  const wet = Uint8Array.from(decode(w, f, 'depth'), d => (d >= WET ? 1 : 0));
  return { wet, sea: oceanFill(wet, w.meta.w, w.meta.h, w.meta.globe) };
}

/** The squares in the sea, a byte each: the squares of the sea, and the dry squares beside one. */
export function seaSquares(wet: Wet, meta: Meta): Uint8Array {
  const W = meta.w, H = meta.h;
  return Uint8Array.from({ length: W * H }, (_, i) => {
    if (wet.sea[i]) return 1;
    if (wet.wet[i]) return 0;
    const sx = i % W, sy = (i / W) | 0;
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
      const nx = meta.globe ? (sx + dx + W) % W : sx + dx, ny = sy + dy;
      if (nx >= 0 && ny >= 0 && nx < W && ny < H && wet.sea[ny * W + nx]) return 1;
    }
    return 0;
  });
}

/** The fields of the map's squares at frame f, a texel a square (fields.ts), and its wet squares and sea (a frame does
 * not say how wet the ground is: 0). The liquid's
 * surface over the ground is a wet square's depth, and in a dry square lies below its ground by the depth it lacks to
 * be wet and by its height over the surface of the wet squares beside it, so the shore drawn between a wet square and
 * a dry one lies nearer the lower. A square's slope is the Sobel gradient of the heights about it. */
export function mapFields(w: World, f: number): { fields: Fields; wet: Wet } {
  const m = w.meta, { w: W, h: H, cell_m, globe } = m, n = W * H;
  const z = decode(w, f, 'z'), depth = decode(w, f, 'depth'), ice = decode(w, f, 'ice'), tg = decode(w, f, 't_ground');
  const leaf = decode(w, f, 'leaf'), soaked = decode(w, f, 'wet'), channelW = decode(w, f, 'channel_w'), channelD = decode(w, f, 'channel_d');
  const [tr, tgr, tb, gr, gg, gb] = ['true_r', 'true_g', 'true_b', 'ground_r', 'ground_g', 'ground_b'].map(name => plane(w, f, name));
  const wet = wetOf(w, f), sea = seaSquares(wet, m);
  const wrapX = (x: number) => (globe ? (x + W) % W : x);
  const height = new Float32Array(4 * n), matter = new Float32Array(4 * n), bytes = new Uint8Array(4 * n), ground = new Uint8Array(4 * n), inside = new Float32Array(4 * n);
  for (let i = 0; i < n; i++) {
    const x = i % W, y = (i / W) | 0;
    const zAt = (dx: number, dy: number) => z[clamp(y + dy, 0, H - 1) * W + clamp(wrapX(x + dx), 0, W - 1)];
    // A globe's cells are narrower east to west towards the poles.
    const width = globe ? cell_m * Math.max(Math.cos((latOf(m, y) * Math.PI) / 180), POLE_COS) : cell_m;
    const gx = (zAt(1, -1) + 2 * zAt(1, 0) + zAt(1, 1) - zAt(-1, -1) - 2 * zAt(-1, 0) - zAt(-1, 1)) / (8 * width);
    const gy = (zAt(-1, 1) + 2 * zAt(0, 1) + zAt(1, 1) - zAt(-1, -1) - 2 * zAt(0, -1) - zAt(1, -1)) / (8 * cell_m);
    let water = depth[i];
    if (!wet.wet[i]) {
      let sum = 0, k = 0;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
        const nx = wrapX(x + dx), ny = y + dy, j = ny * W + nx;
        if (nx >= 0 && ny >= 0 && nx < W && ny < H && wet.wet[j]) { sum += z[j] + depth[j]; k++; }
      }
      water = depth[i] - WET - (k ? Math.max(z[i] - sum / k, 0) : 0);
    }
    put4(height, i, z[i], water, gx, gy);
    put4(matter, i, ice[i], tg[i], sea[i], wet.wet[i] ? 0 : Math.min(channelW[i] / width, 1));
    put4(bytes, i, tr[i], tgr[i], tb[i], 0);
    put4(ground, i, gr[i], gg[i], gb[i], 0);
    put4(inside, i, soaked[i], channelD[i], Math.exp(-leaf[i]), 0);
  }
  const o = w.base + f * w.frameBytes;
  const fields: Fields = { w: W, h: H, height, matter, bytes, ground, inside, layers: w.data.subarray(o, o + w.frameBytes), names: w.meta.layers.map(l => l.name), look: [1, 0, SQUARE_BEACH_M, 1], place: [0, 0, W * cell_m, H * cell_m] };
  return { fields, wet };
}
