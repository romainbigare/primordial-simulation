// The map's renderer: what is drawn, on WebGL2 (gpu-webgl2.ts). The shaders are written in WGSL (web/shaders/); at
// build time naga writes them as GLSL ES 3.00, with where each binding lies (web/shade, put on the page as `SHADERS`).
// A pass draws rectangles of a target (the canvas, or a texture the renderer draws into to read back) with a program,
// its uniforms, textures and samplers given by binding. The page holds its textures through `HeldGpu`, which fills
// them again after a lost context.
import { openWebGl2 } from './gpu-webgl2';

/** Where WebGL2 finds a binding of a program: a uniform block's binding, or a sampler's texture and sampler bindings
 * (null: the texture is read whole texels at a time, with no sampler). */
type Bind = { uniform: [number, number] } | { texture: [number, number]; sampler: [number, number] | null };
/** A program as web/shade writes it: its GLSL stages, the GLSL names of its bindings, the least metres a
 * pixel takes for which it is made once more (as its declaration of `LEAST_E` writes them), and the targets it draws
 * into at once (a pass of as many, `Gpu.pass`). */
export interface Program { vertex: string; fragment: string; binds: Record<string, Bind>; least: string[]; targets: number }
declare const SHADERS: Record<string, Program>;
/** The declaration of `LEAST_E` in a program made for no least size of a pixel (its GLSL; web/shaders/lib/draw.wgsl), and
 * its value. */
const LEAST_DECLARED = /(LEAST_E = )0\.0;/;
let all: Record<string, Program> | null = null;
/** The page's programs (web/shaders/<name>.wgsl), and each one made for draws whose pixels take at least `m` metres,
 * named `<name>:<m>`: the same text with `LEAST_E` set to `m` (web/shade checks that this is what it makes). */
export function programs(): Record<string, Program> {
  if (all) return all;
  const out: Record<string, Program> = { ...SHADERS };
  for (const [name, p] of Object.entries(SHADERS)) {
    for (const m of p.least) {
      const set = (text: string) => text.replace(LEAST_DECLARED, `$1${m};`);
      out[`${name}:${m}`] = { ...p, vertex: set(p.vertex), fragment: set(p.fragment), least: [] };
    }
  }
  return (all = out);
}

/** The share of the metres a pixel takes that a program made for draws seen from at least so many metres (web/shade)
 * is chosen within, so that every pixel of the draw takes at least that many (they differ by a pixel's rounding). */
const LEAST_SPARE = 0.99;
/** Of program `name`, those that may draw where a pixel of the draw takes `e` metres, the best first: those made for
 * draws whose pixels take at least no more than `e` metres (`programs`), the most first, and `name` itself last. Each
 * draws the same picture there (draw.wgsl `LEAST_E`: what one leaves out cannot show at `e`); the first does least. */
function madeFor(name: string, e: number): string[] {
  const fit = sizedOf(name).filter(([, m]) => m <= e * LEAST_SPARE).map(([key]) => key);
  return [...fit, name];
}
/** Of each program, those made for a least size of a pixel (`programs`) with that size, m, the most first: found once
 * (the programs never change). */
const sized = new Map<string, [string, number][]>();
function sizedOf(name: string): [string, number][] {
  let out = sized.get(name);
  if (!out) {
    out = Object.keys(programs()).map(key => [key, ...key.split(':')]).filter(([, base, m]) => base === name && m !== undefined)
      .map(([key, , m]): [string, number] => [key, +m]).sort((x, y) => y[1] - x[1]);
    sized.set(name, out);
  }
  return out;
}

/** Of program `name`, the one a draw where a pixel takes `e` metres draws with: the best of those that may (`madeFor`)
 * that is made, so a draw never waits for a program while one that draws the same picture is made; the best is made
 * in its turn (`Gpu.warm`). With none made, the best is made before all others and the draw is drawn again once it is
 * (`waiting`), meanwhile by the one made for the nearest size of a pixel (none when none is made), and the page does
 * not stop while it is made. */
export function seenFrom(gpu: Gpu, name: string, e: number, waiting: string[]): string | null {
  const may = madeFor(name, e), made = may.find(key => gpu.ready(key));
  gpu.warm(may[0], made === undefined);
  // The ones a zoom in and a zoom out need next are made after all others asked, so a zoom finds them made.
  gpu.warm(madeFor(name, e / ZOOM_AHEAD)[0]);
  gpu.warm(madeFor(name, e * ZOOM_AHEAD)[0]);
  if (made !== undefined) return made;
  waiting.push(may[0]);
  // Till one is made, the one made for the nearest size of a pixel draws (a little more or less detail for a moment),
  // so a zoom past a size never takes away what was drawn.
  const size = (key: string) => Math.abs(Math.log(Math.max(+(key.split(':')[1] ?? 0), EVERY_SIZE_M) / e));
  return madeFor(name, Infinity).filter(key => gpu.ready(key)).sort((x, y) => size(x) - size(y))[0] ?? null;
}
/** The programs made ahead are those for pixels this many times finer and coarser than the view's (`seenFrom`). */
const ZOOM_AHEAD = 3;
/** Where programs are sorted by the sizes of a pixel they are made for (`seenFrom`), the one made
 * for every size counts as made for this many metres. */
const EVERY_SIZE_M = 0.25;

/** Of `programs`, those not made (`Gpu.ready`), each asked to be made before all others (`Gpu.warm`). */
export function notMade(gpu: Gpu, programs: string[]): string[] {
  const missing = programs.filter(name => !gpu.ready(name));
  missing.forEach(name => gpu.warm(name, true));
  return missing;
}

/** The programs made in the background (`Gpu.warm`): one at a time, in the order asked, each by `make` (done when its
 * promise is), none that is made already (`made`), each begun only once the task that asked for it is over (the frame
 * that went without it is sent first). */
export class Warming {
  private queue: string[] = [];
  private making: string | null = null;
  /** When the program being made was begun, ms, and the last made with the time it took. */
  private begun = 0;
  private last: [string, number] | null = null;
  private soon = 0;
  constructor(private make: (key: string) => Promise<void>, private made: (key: string) => boolean) {}
  /** Make `key` after those asked before it, or before all (`first`). */
  ask(key: string, first = false): void {
    if (this.made(key) || this.making === key) return;
    const at = this.queue.indexOf(key);
    if (at >= 0 && !first) return;
    if (at >= 0) this.queue.splice(at, 1);
    if (first) this.queue.unshift(key); else this.queue.push(key);
    this.next();
  }
  /** Forget all asked (a lost context). */
  clear(): void { this.queue = []; }
  /** The program being made (for how long) and how many wait, and the last made (in how long), as text. */
  pending(): string {
    const s = (ms: number) => `${(ms / 1000).toFixed(1)} s`;
    const now = this.making === null ? 'none being made' : `${this.making} being made for ${s(performance.now() - this.begun)}`;
    return `${now}, ${this.queue.length} waiting${this.last ? `; the last, ${this.last[0]}, in ${s(this.last[1])}` : ''}`;
  }
  private next(): void {
    if (this.making !== null || this.soon) return;
    this.soon = window.setTimeout(() => {
      this.soon = 0;
      let key = this.queue.shift();
      while (key !== undefined && this.made(key)) key = this.queue.shift();
      if (key === undefined) return;
      this.making = key; this.begun = performance.now();
      this.make(key).catch(e => reportError(new Error(`gpu: ${key}: ${e}`))).finally(() => {
        this.last = [key, performance.now() - this.begun];
        this.making = null; this.next();
      });
    }, 0);
  }
}

/** The bindings a program reads, by name of its GLSL global: the numbers of those that are uniform blocks, textures
 * and samplers. */
export function bindingsOf(p: Program): { uniforms: number[]; textures: number[]; samplers: number[] } {
  const u = new Set<number>(), t = new Set<number>(), s = new Set<number>();
  for (const b of Object.values(p.binds)) {
    if ('uniform' in b) u.add(b.uniform[1]);
    else { t.add(b.texture[1]); if (b.sampler) s.add(b.sampler[1]); }
  }
  return { uniforms: [...u], textures: [...t], samplers: [...s] };
}

/** Bytes of a texel of each format. */
export const TEXEL_BYTES: Record<Format, number> = { rgba8unorm: 4, 'rgba8unorm-srgb': 4, rgba32float: 16, rgba16float: 8, rg16float: 4, r16float: 2 };

/** What a texel holds: four bytes (read as 0..1), or four 32-bit floats, or four or one 16-bit floats (the floats read
 * whole texels at a time). */
export type Format = 'rgba8unorm' | 'rgba8unorm-srgb' | 'rgba32float' | 'rgba16float' | 'rg16float' | 'r16float';
/** The data a texture of each format is filled with: bytes, 32-bit floats, or 16-bit floats as their bits. */
export type TexelData = Uint8Array | Float32Array | Uint16Array;
/** How a sampler reads a texture: blended between its texels' centres and between its mip levels (`smooth`); blended
 * between its texels' centres on its nearest mip level (`mip`); or repeated across the plane and blended between its
 * mip levels too (`tile`: a texture laid on the ground). */
export type Sampler = 'smooth' | 'mip' | 'tile';
/** A texture on the GPU: `w` x `h` texels, `layers` of them (more than one: an array), `mips` levels. */
export interface Texture { readonly w: number; readonly h: number; readonly layers: number; readonly mips: number; readonly format: Format }

/** One draw: its rectangle of the target (x, y, w, h px), what of the source it shows (u0, v0, u1, v1), how strongly
 * it covers what is under it, and its other bindings by number: uniforms (a buffer laid out as the WGSL struct),
 * textures and samplers. */
export interface Draw { rect: number[]; src?: number[]; alpha?: number; binds: Record<number, ArrayBuffer | Texture | Sampler> }

/** Draws into one target, in order. */
export interface Pass {
  /** Draw program `name`; `blend` draws over what is there (colours premultiplied by their cover), else replaces it. */
  draw(name: string, d: Draw, blend: boolean): void;
  end(): void;
}

export interface Gpu {
  /** A texture of `w` x `h` texels of `format`, `layers` deep with `mips` levels; `target`: it can be drawn into. Its
   * `label` says what it holds (the cost line sums the GPU's memory by it). */
  texture(label: string, w: number, h: number, format: Format, layers?: number, mips?: number, target?: boolean): Texture;
  /** Fill mip level `level` of texture `t` (all its layers, layer after layer, rows top first; or layer `layer` alone,
   * which the renderer does not keep: after a lost context it is empty). */
  write(t: Texture, data: TexelData, level?: number, layer?: number): void;
  /** Fill rows `y` to `y + rows` of the first level of texture `t` (one layer) from `data`, all of its texels: the
   * renderer keeps `data` itself as that level, so the rows not sent must already hold what it holds. */
  writeRows(t: Texture, data: TexelData, y: number, rows: number): void;
  /** Make the mip levels of texture `t` from its first (a picture drawn into it, shown smaller later). */
  mipmap(t: Texture): void;
  /** Let texture `t` go: at once on the back end; on the page's GPU once the next picture begins (`HeldGpu.begin`). */
  drop(t: Texture): void;
  /** Draw into `target` (null: the canvas, sized to its pixels; several textures of one size and format: a program's
   * targets, `Program.targets`, in the order of its outputs), first filling it with `clear` (r, g, b, a, 0..1) if given,
   * and only inside `clip` (x, y, w, h px from its top-left corner) if given. */
  pass(target: Texture | Texture[] | null, clear?: number[], clip?: number[]): Pass;
  /** Make program `name` in the background, one program at a time in the order asked (`first`: before all others), so
   * its first draw need not wait for it. */
  warm(name: string, first?: boolean): void;
  /** Whether program `name` is made (a draw with one that is not waits until it is). */
  ready(name: string): boolean;
  /** Send what was drawn to the GPU. */
  submit(): void;
  /** Once the GPU has finished all that was sent. */
  done(): Promise<void>;
  /** Time on the GPU what is sent from now till `end`: its ms once the GPU has drawn it, by a timer on the card where it
   * has one (null where that timer was disturbed: no time), else the time from `end` till the GPU has finished all
   * sent, the page's own work in it. */
  timer(): { end(): Promise<number | null> };
  /** The bytes of texture `t` (four a texel, rows top first), once the GPU has drawn it. */
  read(t: Texture): Promise<Uint8Array>;
}

/** The back end (gpu-webgl2.ts), which also says, for a card that does not finish its frames, the programs it is
 * making ahead (`Warming.pending`) and what card it is. */
export interface Back extends Gpu {
  making(): string;
  about(): string;
}

/** The draw header every program reads at binding 0 (web/shaders/lib/draw.wgsl `Draw`): the target's size, px, and the sign of its rows, with the draw's alpha, rectangle and source. */
export function drawHeader(size: [number, number], flip: number, d: Draw): Float32Array {
  return new Float32Array([size[0], size[1], flip, d.alpha ?? 1, ...d.rect, ...(d.src ?? [0, 0, 1, 1])]);
}

/** A uniform buffer laid out as a WGSL struct whose members are all four-wide (vec4f, vec4i and arrays of them), so
 * that member k starts at byte 16 k (std140 in the GLSL naga writes). */
export class Uniforms {
  readonly buffer: ArrayBuffer;
  private f: Float32Array;
  private i: Int32Array;
  private at = 0;
  constructor(vec4s: number) { this.buffer = new ArrayBuffer(16 * vec4s); this.f = new Float32Array(this.buffer); this.i = new Int32Array(this.buffer); }
  /** The next member, four floats (missing ones 0). */
  vec4(...v: number[]): this { this.f.set(v, 4 * this.at++); return this; }
  /** The next member, four integers (missing ones 0). */
  ivec4(...v: number[]): this { this.i.set(v, 4 * this.at++); return this; }
}

/** The time the card may take to finish the frames sent, ms, before the page says it does not draw: long, as a
 * software GL (the test browser's) takes up to tens of seconds for a frame at the finest views. */
const STALL_MS = 30000;

/** A texture as the page holds it: the back end's texture, and what each of its mip levels was last filled with whole,
 * to fill it again after a lost context (a layer filled alone is not kept: its owner fills it again, `restored`). */
class HeldTexture implements Texture {
  levels: TexelData[] = [];
  constructor(readonly label: string, readonly w: number, readonly h: number, readonly layers: number, readonly mips: number, readonly format: Format, readonly target: boolean, public inner: Texture) {}
  /** Bytes it takes on the card: each of its levels, a texel `TEXEL_BYTES`. */
  get bytes(): number {
    let n = 0;
    for (let k = 0; k < this.mips; k++) n += Math.max(1, this.w >> k) * Math.max(1, this.h >> k);
    return n * this.layers * TEXEL_BYTES[this.format];
  }
}

/** The page's GPU: the back end and the textures the page holds on it. Where it loses what it held (a lost context
 * given back), the textures are made anew, filled again, and the page is told (`restored`) to fill the rest. */
export class HeldGpu implements Gpu {
  private back!: Back;
  private held = new Set<HeldTexture>();
  /** Textures dropped since the last picture began (`begin`): bindings worked out for it before its draws (a layer's
   * marks, decided once for a picture drawn in tiles over many frames) may name them till then. */
  private dropped: HeldTexture[] = [];
  /** Whether frames sent wait for the card to finish them, how many were sent since they began to wait and the
   * programs they drew; the frames the back end finished, the time the last took, ms, and the programs drawn in a
   * frame it finished. What a stalled card leaves undone is said from these (`stalled`). */
  private watching = false;
  private sent = 0;
  private drawn = new Set<string>();
  private finished = 0;
  private lastMs = 0;
  private drewDone = new Set<string>();
  /** The frames sent to the back end and those it finished (`busy`), and what waits for it to finish all (`idle`). */
  private sentAll = 0;
  private doneAll = 0;
  private waiters: (() => void)[] = [];

  /** `restored` fills again what the textures held after they were made anew. */
  constructor(private restored: () => void) {}

  /** Open WebGL2 on `canvas`; false where the browser has none. */
  open(canvas: HTMLCanvasElement): boolean {
    const g = openWebGl2(canvas, () => this.use(g!, true));
    if (g) this.use(g, false);
    return g !== null;
  }

  /** Draw with `back` from now on: the textures made on it and filled again, and the page told so (`rebuilt`). */
  private use(back: Back, rebuilt: boolean): void {
    this.back = back; this.watching = false; this.finished = 0; this.drewDone.clear();
    this.sentAll = this.doneAll = 0;
    this.dropped = [];
    this.wake();
    for (const t of this.held) { t.inner = back.texture(t.label, t.w, t.h, t.format, t.layers, t.mips, t.target); t.levels.forEach((d, k) => d && back.write(t.inner, d, k)); }
    if (rebuilt) this.restored();
  }

  texture(label: string, w: number, h: number, format: Format, layers = 1, mips = 1, target = false): Texture {
    const t = new HeldTexture(label, w, h, layers, mips, format, target, this.back.texture(label, w, h, format, layers, mips, target));
    this.held.add(t);
    return t;
  }
  /** The bytes the textures held take on the card, by label, the most first. */
  memory(): [string, number][] {
    const by = new Map<string, number>();
    for (const t of this.held) by.set(t.label, (by.get(t.label) ?? 0) + t.bytes);
    return [...by].sort((a, b) => b[1] - a[1]);
  }
  write(t: Texture, data: TexelData, level = 0, layer?: number): void {
    const h = t as HeldTexture;
    if (layer === undefined) h.levels[level] = data;
    this.back.write(h.inner, data, level, layer);
  }
  writeRows(t: Texture, data: TexelData, y: number, rows: number): void {
    const h = t as HeldTexture;
    h.levels[0] = data;
    this.back.writeRows(h.inner, data, y, rows);
  }
  mipmap(t: Texture): void { this.back.mipmap((t as HeldTexture).inner); }
  drop(t: Texture): void { this.held.delete(t as HeldTexture); this.dropped.push(t as HeldTexture); }
  /** A picture begins (its bindings are worked out anew, none from before is drawn with again): the textures dropped
   * since the last are let go now, so no draw ever binds one let go (which WebGL reads as black). */
  begin(): void {
    this.dropped.forEach(t => this.back.drop(t.inner));
    this.dropped = [];
  }
  pass(target: Texture | Texture[] | null, clear?: number[], clip?: number[]): Pass {
    const inner = (t: Texture) => (t as HeldTexture).inner;
    const p = this.back.pass(target === null ? null : Array.isArray(target) ? target.map(inner) : inner(target), clear, clip);
    return {
      draw: (name, d, blend) => {
        this.drawn.add(name);
        const binds: Draw['binds'] = {};
        for (const [n, b] of Object.entries(d.binds)) binds[+n] = b instanceof HeldTexture ? b.inner : b;
        p.draw(name, { ...d, binds }, blend);
      },
      end: () => p.end(),
    };
  }
  warm(name: string, first?: boolean): void { this.back.warm(name, first); }
  ready(name: string): boolean { return this.back.ready(name); }
  done(): Promise<void> { return this.back.done(); }
  timer(): { end(): Promise<number | null> } { return this.back.timer(); }
  read(t: Texture): Promise<Uint8Array> { return this.back.read((t as HeldTexture).inner); }
  /** The programs being made (`Warming.pending`), as text. */
  making(): string { return this.back.making(); }

  /** Send what was drawn, and watch that the card finishes it: one that has not within `STALL_MS` is said on the page. */
  submit(): void {
    const back = this.back, n = ++this.sentAll;
    back.submit();
    back.done().then(() => {
      if (this.back !== back || n <= this.doneAll) return;
      this.doneAll = n;
      if (!this.busy) this.wake();
    });
    this.sent++;
    if (this.watching) return;
    this.watching = true; this.sent = 1;
    const t0 = performance.now();
    const timer = setTimeout(() => { if (this.back === back) reportError(new Error(`gpu: ${this.stalled(STALL_MS)}`)); }, STALL_MS);
    back.done().then(() => {
      clearTimeout(timer);
      if (this.back !== back) return;
      this.watching = false; this.finished++; this.lastMs = performance.now() - t0;
      for (const name of this.drawn) this.drewDone.add(name);
      this.drawn.clear();
    });
  }

  /** Whether frames sent are still being drawn by the card: the map sends no new one meanwhile (map.ts `draw`), so
   * frames never pile up in the card's queue, where they hold memory and hold back the programs it makes. */
  get busy(): boolean { return this.doneAll < this.sentAll; }

  /** Once the card has drawn all the frames sent (or its context was given back). */
  idle(): Promise<void> {
    return this.busy ? new Promise(go => this.waiters.push(go)) : Promise.resolve();
  }

  /** Tell what waits for the card that it may go on. */
  private wake(): void {
    const w = this.waiters;
    this.waiters = [];
    w.forEach(go => go());
  }

  /** What a card that has not finished the frames sent in `ms` leaves undone, in a line or two: the card, the frames
   * sent and finished before, the programs the frames drew that no finished frame drew, and those being made. */
  private stalled(ms: number): string {
    const back = this.back, unfinished = [...this.drawn].filter(name => !this.drewDone.has(name));
    return `the card (${back.about()}) has not finished a frame of the map in ${ms / 1000} s: ${this.sent} sent, `
      + `${this.finished} finished before${this.finished ? ` (the last in ${Math.round(this.lastMs)} ms)` : ''}; drawn in none finished: ${unfinished.join(', ') || 'none'}; ${back.making()}`;
  }
}

/** The page's GPU (`HeldGpu`) on `canvas`, or null where the browser has no WebGL2; `restored` fills again what the
 * textures held after a lost context was given back. */
export function openGpu(canvas: HTMLCanvasElement, restored: () => void): HeldGpu | null {
  const g = new HeldGpu(restored);
  return g.open(canvas) ? g : null;
}
