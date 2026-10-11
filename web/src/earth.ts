// Turns layers into pixels. The viewer, not the core, decides what counts as sea, river, desert...
import { World, plane, decode } from './data';

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

const clamp = (x: number, a = 0, b = 1) => (x < a ? a : x > b ? b : x);
const smooth = (a: number, b: number, x: number) => { const t = clamp((x - a) / (b - a)); return t * t * (3 - 2 * t); };
function ramp(r: RGB[], t: number, o: number[]): void {
  const x = clamp(t) * (r.length - 1), i = Math.min(Math.floor(x), r.length - 2), f = x - i;
  for (let k = 0; k < 3; k++) o[k] = r[i][k] + (r[i + 1][k] - r[i][k]) * f;
}
const mix = (o: number[], c: RGB, t: number) => { for (let k = 0; k < 3; k++) o[k] += (c[k] - o[k]) * t; };

const WET = 2; // metres of liquid that make a cell water
const L = [-0.588, -0.588, 0.647]; // light from the north-west
const EXAGGERATE = 6;

/** Wet cells connected to the map border are sea; the others are lakes. */
function oceanFill(wet: Uint8Array, W: number, H: number): Uint8Array {
  const sea = new Uint8Array(W * H), stack: number[] = [];
  const seed = (i: number) => { if (wet[i] && !sea[i]) { sea[i] = 1; stack.push(i); } };
  for (let x = 0; x < W; x++) { seed(x); seed((H - 1) * W + x); }
  for (let y = 0; y < H; y++) { seed(y * W); seed(y * W + W - 1); }
  while (stack.length) {
    const i = stack.pop()!, x = i % W, y = (i / W) | 0;
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
      const nx = x + dx, ny = y + dy;
      if (nx >= 0 && ny >= 0 && nx < W && ny < H) seed(ny * W + nx);
    }
  }
  return sea;
}

/** Fill `img` for frame f. mode: 'earth', 'true' or a layer name. `cls` receives the map class of every cell. */
export function paint(w: World, f: number, mode: string, img: ImageData, cls: Uint8Array): void {
  const { w: W, h: H, cell_m } = w.meta;
  const z = decode(w, f, 'z'), depthL = decode(w, f, 'depth'), ice = decode(w, f, 'ice'), tg = decode(w, f, 't_ground');
  const rain = plane(w, f, 'rain'), flux = decode(w, f, 'flux'), plants = plane(w, f, 'plants');
  const tr = plane(w, f, 'true_r'), tgc = plane(w, f, 'true_g'), tb = plane(w, f, 'true_b');
  const over = ['earth', 'true'].includes(mode) ? null : plane(w, f, mode);
  const overLut = over ? w.lut[w.index.get(mode)!] : null;
  const overRamp = rampFor(mode);

  const depth = depthL ?? Float32Array.from(z ?? [], v => Math.max(0, -v));
  const wet = Uint8Array.from(depth, d => (d >= WET ? 1 : 0));
  const sea = oceanFill(wet, W, H);
  const zAt = (x: number, y: number) => z![clamp(y, 0, H - 1) * W + clamp(x, 0, W - 1)];
  const nearSea = (x: number, y: number) => {
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++)
      if (x + dx >= 0 && x + dx < W && y + dy >= 0 && y + dy < H && sea[(y + dy) * W + x + dx]) return true;
    return false;
  };
  const c = [0, 0, 0], out = img.data;

  for (let i = 0; i < W * H; i++) {
    const x = i % W, y = (i / W) | 0;
    let shade = 1, slope = 0;
    if (z) {
      const gx = (zAt(x + 1, y - 1) + 2 * zAt(x + 1, y) + zAt(x + 1, y + 1) - zAt(x - 1, y - 1) - 2 * zAt(x - 1, y) - zAt(x - 1, y + 1)) / (8 * cell_m);
      const gy = (zAt(x - 1, y + 1) + 2 * zAt(x, y + 1) + zAt(x + 1, y + 1) - zAt(x - 1, y - 1) - 2 * zAt(x, y - 1) - zAt(x + 1, y - 1)) / (8 * cell_m);
      slope = Math.hypot(gx, gy);
      const nx = -EXAGGERATE * gx, ny = -EXAGGERATE * gy;
      shade = ((nx * L[0] + ny * L[1] + L[2]) / Math.hypot(nx, ny, 1)) / L[2];
    }
    const zi = z ? z[i] : 0, iceI = ice ? smooth(0.02, 0.5, ice[i]) : 0;
    let k = 0.8, river = 0;
    if (wet[i]) {
      if (sea[i]) { ramp(SEA, 1 - Math.exp(-depth[i] / 350), c); cls[i] = depth[i] > 200 ? DEEP : SHALLOW; }
      else { ramp(LAKEC, 1 - Math.exp(-depth[i] / 60), c); cls[i] = LAKE; }
      mix(c, SNOW, iceI * 0.85); if (iceI > 0.5) cls[i] = ICE;
      k = 0.25 + 0.5 * iceI;
    } else {
      const t = tg ? tg[i] : 288, rn = rain ? rain[i] / 255 : 0.5;
      const v = plants ? Math.sqrt(plants[i] / 255)
        : smooth(0.22, 0.7, rn) * (0.25 + 0.75 * smooth(258, 278, t)) * (1 - 0.4 * smooth(318, 335, t));
      ramp(VEG, v, c); cls[i] = v < 0.3 ? DESERT : v < 0.62 ? GRASS : FOREST;
      mix(c, TUNDRA, smooth(278, 266, t) * 0.7);
      mix(c, UPLAND, smooth(1200, 2600, zi) * 0.5);
      const coast = nearSea(x, y);
      if (coast) mix(c, SAND, 0.55 * (1 - smooth(0, 40, zi)));
      const rock = Math.max(smooth(0.14, 0.42, slope) * smooth(150, 700, zi), smooth(2000, 3200, zi) * 0.85) * (1 - 0.5 * v);
      if (rock > 0) { const hi = smooth(1500, 4000, zi); mix(c, [0, 1, 2].map(j => ROCK_LO[j] + (ROCK_HI[j] - ROCK_LO[j]) * hi) as RGB, rock); }
      if (rock > 0.5) cls[i] = ROCK;
      const snow = ice ? iceI : smooth(266, 258, t);
      mix(c, SNOW, snow); if (snow > 0.5) cls[i] = ICE;
      k = 0.8 + 0.3 * snow;
      river = flux ? smooth(3.6, 4.3, Math.log10(flux[i] + 1)) * (1 - 0.6 * snow) : 0;
    }
    const m = clamp(1 + (shade - 1) * k, 0.5, 1.45);
    for (let j = 0; j < 3; j++) c[j] *= m;
    if (river > 0) mix(c, RIVERC.map(v => v * (0.7 + 0.3 * m)) as RGB, river * 0.9);
    let [r, g, b] = c;
    if (river > 0.5) cls[i] = RIVER;

    if (mode === 'true' && tr && tgc && tb) {
      const mt = clamp(1 + (shade - 1) * 0.7, 0.55, 1.4);
      r = tr[i] * mt; g = tgc[i] * mt; b = tb[i] * mt;
    } else if (over && overLut) {
      if (overLut[over[i]] === 0) { // empty: show the base map faded
        const grey = 0.3 * r + 0.59 * g + 0.11 * b;
        r = (r * 0.5 + grey * 0.5) * 0.6 + 100; g = (g * 0.5 + grey * 0.5) * 0.6 + 100; b = (b * 0.5 + grey * 0.5) * 0.6 + 100;
      } else {
        ramp(overRamp, over[i] / 255, c);
        const mo = 0.72 * (0.75 + 0.25 * m);
        r = c[0] * mo + r * 0.28; g = c[1] * mo + g * 0.28; b = c[2] * mo + b * 0.28;
      }
    }
    out[i * 4] = r; out[i * 4 + 1] = g; out[i * 4 + 2] = b; out[i * 4 + 3] = 255;
  }
}
