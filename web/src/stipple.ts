// A herd seen from afar (handover 10.5: populations far away are drawn as density and colour): where a member takes
// less than a pixel, its node is drawn as dots over its box (its cell, or the room it is inside), the denser and the
// stronger the more members a m² holds, alike at every zoom: from `LEAST` dots a px², so a sparse herd still shows as a
// soft patch, to `MOST`, so a crowded one does not paint its box solid; in its kind's colour made lighter on dark ground
// and darker on light ground (`Ground`), so it reads on any cover; thinning towards the box's edges. No count is drawn:
// the inspect panel gives it.
import { Rgb, seeded, seedOf } from './sprites';

/** Members a m² from which a stipple starts to grow denser and stronger, and at which it is at its densest (log scale):
 * about the lower and upper tenth of the herds of a live island. */
const SPARSE = 1e-5, CROWDED = 1e-2;
/** Dots a px² of the box from a sparse herd to a crowded one (a geometric scale between). */
const LEAST = 0.008, MOST = 0.06;
/** A dot's side, px. */
const DOT = 1.6;
/** A dot's opacity, from a sparse herd to a crowded one. */
const ALPHA = [0.35, 0.9];
/** The lightness (0..1) a dot departs from the ground's under it, and the mean lightness of the ground under a stipple
 * from which its dots are darker than the ground (below, lighter): one way for a whole herd, so it reads as one patch. */
const CONTRAST = 0.32, SPLIT = 0.5;
/** Points along each side of a box the mean of the ground under it is taken at. */
const MEAN_POINTS = 5;
/** The bins of the ground's lightness a node's dots are coloured by (one fill each). */
const BINS = 8;
/** The dots thin from this share of the way from the box's centre to its edge, measured as a rounded square of this
 * power, so the patch has round corners and a soft edge. */
const THIN_FROM = 0.55, ROUND = 4;
/** A box's least side in pixels for its stipple to be drawn: below, its cell is too small to show a group. */
const LEAST_SIDE_PX = 3;
/** The ground is read every this many css pixels, as the mean over them: a dot's tone follows the ground's shading and
 * cover, not the marks of its texture. */
const GROUND_PX = 16;

/** The lightness of what is drawn under the things: read from the map's canvas when the picture of the things is made. */
export class Ground {
  private cv = document.createElement('canvas');
  private data: Uint8ClampedArray = new Uint8ClampedArray(4);
  /** Read the lightness of what `ctx` holds now, a view `W` x `H` css px. */
  read(ctx: CanvasRenderingContext2D, W: number, H: number): void {
    const w = Math.max(1, Math.ceil(W / GROUND_PX)), h = Math.max(1, Math.ceil(H / GROUND_PX));
    this.cv.width = w; this.cv.height = h;
    const c = this.cv.getContext('2d', { willReadFrequently: true })!;
    c.imageSmoothingEnabled = true; c.imageSmoothingQuality = 'high';
    // A read covers `GROUND_PX` css px a side: the canvas holds `d` device pixels a css pixel.
    const d = ctx.canvas.width / Math.max(W, 1);
    c.drawImage(ctx.canvas, 0, 0, w * GROUND_PX * d, h * GROUND_PX * d, 0, 0, w, h);
    this.data = c.getImageData(0, 0, w, h).data;
  }
  /** The lightness (0..1) at (x, y) css px of the view, the nearest read on it where it lies off it. */
  at(x: number, y: number): number {
    const w = this.cv.width, h = this.cv.height;
    const i = 4 * (Math.min(Math.max(Math.floor(y / GROUND_PX), 0), h - 1) * w + Math.min(Math.max(Math.floor(x / GROUND_PX), 0), w - 1));
    return lightness(this.data[i], this.data[i + 1], this.data[i + 2]);
  }
}

/** The colour of a rim about a shape drawn over ground of lightness `g` (0..1): light on dark ground, dark on light. */
export const rimOver = (g: number) => (g < SPLIT ? 'rgba(255,255,255,.6)' : 'rgba(0,0,0,.5)');

/** The lightness (0..1) of a display colour (its luma, Rec. 709). */
const lightness = (r: number, g: number, b: number) => (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;

/** Colour `c` (display) made as light as `t` (0..1): mixed towards white, or darkened. */
function lightTo(c: Rgb, t: number): Rgb {
  const l = Math.max(lightness(...c), 1e-3);
  return (t > l ? c.map(v => v + (255 - v) * Math.min((t - l) / Math.max(1 - l, 1e-3), 1)) : c.map(v => (v * t) / l)) as Rgb;
}

/** The stipple of a node of `perM2` members a m² and id `id`, colour `colour` (display), over `box` (west, north, east,
 * south, px on `ctx`), each dot inside `view` (px) drawn and handed to `hit` with its key; `ground(x, y)` is the
 * lightness under a point of `ctx`. */
export function stipple(ctx: CanvasRenderingContext2D, id: bigint, perM2: number, colour: Rgb, box: [number, number, number, number],
  view: [number, number, number, number], ground: (x: number, y: number) => number, hit: (x: number, y: number, r: number, key: string) => void): void {
  const [x0, y0, x1, y1] = box, w = x1 - x0, h = y1 - y0;
  if (perM2 <= 0 || Math.min(w, h) < LEAST_SIDE_PX || x1 < view[0] || x0 > view[2] || y1 < view[1] || y0 > view[3]) return;
  const crowd = Math.min(Math.max(Math.log(perM2 / SPARSE) / Math.log(CROWDED / SPARSE), 0), 1);
  const step = 1 / Math.sqrt(LEAST * (MOST / LEAST) ** crowd);
  const cols = Math.max(1, Math.round(w / step)), rows = Math.max(1, Math.round(h / step)), cw = w / cols, rh = h / rows;
  const bins: number[][] = Array.from({ length: BINS }, () => []);
  const c0 = Math.max(0, Math.floor((view[0] - x0) / cw)), c1 = Math.min(cols - 1, Math.floor((view[2] - x0) / cw));
  const r0 = Math.max(0, Math.floor((view[1] - y0) / rh)), r1 = Math.min(rows - 1, Math.floor((view[3] - y0) / rh));
  const base = seedOf(id, -4), mx = x0 + w / 2, my = y0 + h / 2;
  // The mean of the ground under the part of the box in view.
  const [vx0, vy0, vx1, vy1] = [Math.max(x0, view[0]), Math.max(y0, view[1]), Math.min(x1, view[2]), Math.min(y1, view[3])];
  let under = 0;
  for (let j = 0; j < MEAN_POINTS; j++) for (let i = 0; i < MEAN_POINTS; i++) under += ground(vx0 + ((i + 0.5) / MEAN_POINTS) * (vx1 - vx0), vy0 + ((j + 0.5) / MEAN_POINTS) * (vy1 - vy0));
  const way = under / MEAN_POINTS ** 2 < SPLIT ? CONTRAST : -CONTRAST;
  for (let row = r0; row <= r1; row++) for (let col = c0; col <= c1; col++) {
    const r = seeded((base ^ Math.imul(col + 1, 73856093) ^ Math.imul(row + 1, 19349663)) >>> 0);
    const x = x0 + (col + 0.1 + 0.8 * r()) * cw, y = y0 + (row + 0.1 + 0.8 * r()) * rh;
    const d = Math.pow(Math.abs((x - mx) / (w / 2)) ** ROUND + Math.abs((y - my) / (h / 2)) ** ROUND, 1 / ROUND);
    const t = Math.min(Math.max((d - THIN_FROM) / (1 - THIN_FROM), 0), 1);
    if (r() < t * t * (3 - 2 * t)) continue;
    bins[Math.min(Math.floor(ground(x, y) * BINS), BINS - 1)].push(x, y);
    hit(x, y, DOT / 2, `${col}.${row}`);
  }
  ctx.globalAlpha = ALPHA[0] + (ALPHA[1] - ALPHA[0]) * crowd;
  bins.forEach((at, k) => {
    if (!at.length) return;
    const c = lightTo(colour, Math.min(Math.max((k + 0.5) / BINS + way, 0), 1));
    ctx.fillStyle = `rgb(${c.map(Math.round).join(',')})`;
    ctx.beginPath();
    for (let i = 0; i < at.length; i += 2) ctx.rect(at[i] - DOT / 2, at[i + 1] - DOT / 2, DOT, DOT);
    ctx.fill();
  });
  ctx.globalAlpha = 1;
}
