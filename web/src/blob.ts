// A herd just too small to see one by one (handover 10.5: populations far away are drawn as density and colour): where
// a member takes less than a pixel, but no less than `1 / NEAR` of one, its node is drawn as one light, soft blob over
// its box (its cell, or the room it is inside), a little stronger the more members a m² holds, in its kind's colour
// made lighter on dark ground and darker on light ground (`Ground`), so it reads on any cover. It fades in as the zoom
// comes near the members' own size; farther out no herd is drawn. No count is drawn: the inspect panel gives it.
import { Rgb } from './sprites';
import type { Under } from './map';

/** How many times smaller than a pixel a member may be for its node's blob to show: two doublings of the zoom. */
const NEAR = 4;
/** Members a m² from which a blob starts to grow stronger, and at which it is at its strongest (log scale): about the
 * lower and upper tenth of the herds of a live island. */
const SPARSE = 1e-5, CROWDED = 1e-2;
/** A blob's opacity at its centre, from a sparse herd to a crowded one, once the zoom is within a doubling of the
 * members' size. */
const ALPHA = [0.05, 0.15];
/** The lightness (0..1) a blob departs from the ground's under it, and the mean lightness of the ground from which it
 * is darker than the ground (below, lighter). */
const CONTRAST = 0.2, SPLIT = 0.5;
/** Points along each side of a box the mean of the ground under it is taken at. */
const MEAN_POINTS = 5;
/** The share of the way from the blob's centre to its edge at which it starts to fade. */
const SOLID_TO = 0.35;
/** A box's least side in pixels for its blob to be drawn: below, its cell is too small to show a group. */
const LEAST_SIDE_PX = 3;
/** The ground is read every this many css pixels, as the mean over them: a blob's tone follows the ground's shading and
 * cover, not the marks of its texture. */
const GROUND_PX = 16;
/** A read of the ground whose lightness departs from the last by more than this, on the mean, draws the things anew. */
const GROUND_CHANGE = 0.02;

/** The lightness of what is drawn under the things: the map's colours under the view as the GPU draws them (map.ts
 * `under`), read a frame or so after the picture of the things asks for them, so the page never waits for the GPU. */
export class Ground {
  private u: Under = { w: 0, h: 0, rgba: new Uint8Array(0) };
  private reading = false;
  /** `changed` is told when a read differs from the last by more than `GROUND_CHANGE`. */
  constructor(private changed: () => void) {}
  /** Read the map under the view anew from `under` (one read at a time). */
  read(under: (cssPx: number) => Promise<Under>): void {
    if (this.reading) return;
    this.reading = true;
    under(GROUND_PX).then(next => {
      this.reading = false;
      const before = this.u.rgba;
      let diff = next.rgba.length === before.length ? 0 : Infinity;
      for (let i = 0; i < next.rgba.length && diff !== Infinity; i += 4) diff += Math.abs(lightness(next.rgba[i], next.rgba[i + 1], next.rgba[i + 2]) - lightness(before[i], before[i + 1], before[i + 2]));
      this.u = next;
      if (diff / (next.w * next.h) > GROUND_CHANGE) this.changed();
    });
  }
  /** The lightness (0..1) at (x, y) css px of the view, the nearest read on it where it lies off it (none yet: the
   * middle). */
  at(x: number, y: number): number {
    const { w, h, rgba } = this.u;
    if (!w || !h) return SPLIT;
    const i = 4 * (Math.min(Math.max(Math.floor(y / GROUND_PX), 0), h - 1) * w + Math.min(Math.max(Math.floor(x / GROUND_PX), 0), w - 1));
    return lightness(rgba[i], rgba[i + 1], rgba[i + 2]);
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

/** The blob of a node of `perM2` members a m², each `px` pixels long (below a pixel), colour `colour` (display), over
 * `box` (west, north, east, south, px on `ctx`) if it meets `view` (px); `ground(x, y)` is the lightness under a point
 * of `ctx`. Returns where a pointer finds it (centre and radius), or nothing where it is not drawn. */
export function blob(ctx: CanvasRenderingContext2D, perM2: number, px: number, colour: Rgb, box: [number, number, number, number],
  view: [number, number, number, number], ground: (x: number, y: number) => number): [number, number, number] | undefined {
  const [x0, y0, x1, y1] = box, w = x1 - x0, h = y1 - y0;
  if (perM2 <= 0 || px * NEAR < 1 || Math.min(w, h) < LEAST_SIDE_PX || x1 < view[0] || x0 > view[2] || y1 < view[1] || y0 > view[3]) return;
  const crowd = Math.min(Math.max(Math.log(perM2 / SPARSE) / Math.log(CROWDED / SPARSE), 0), 1);
  // It fades in over the first doubling of the zoom past `1 / NEAR` of a pixel.
  const near = Math.min(Math.log2(px * NEAR), 1);
  // The mean of the ground under the part of the box in view.
  const [vx0, vy0, vx1, vy1] = [Math.max(x0, view[0]), Math.max(y0, view[1]), Math.min(x1, view[2]), Math.min(y1, view[3])];
  let under = 0;
  for (let j = 0; j < MEAN_POINTS; j++) for (let i = 0; i < MEAN_POINTS; i++) under += ground(vx0 + ((i + 0.5) / MEAN_POINTS) * (vx1 - vx0), vy0 + ((j + 0.5) / MEAN_POINTS) * (vy1 - vy0));
  under /= MEAN_POINTS ** 2;
  const c = lightTo(colour, Math.min(Math.max(under + (under < SPLIT ? CONTRAST : -CONTRAST), 0), 1)).map(Math.round).join(',');
  const a = (ALPHA[0] + (ALPHA[1] - ALPHA[0]) * crowd) * near;
  const mx = x0 + w / 2, my = y0 + h / 2, r = Math.min(w, h) / 2;
  const g = ctx.createRadialGradient(0, 0, 0, 0, 0, 1);
  g.addColorStop(0, `rgba(${c},${a})`); g.addColorStop(SOLID_TO, `rgba(${c},${a})`); g.addColorStop(1, `rgba(${c},0)`);
  ctx.save();
  ctx.translate(mx, my); ctx.scale(w / 2, h / 2);
  ctx.fillStyle = g;
  ctx.beginPath(); ctx.arc(0, 0, 1, 0, 2 * Math.PI); ctx.fill();
  ctx.restore();
  return [mx, my, r];
}
