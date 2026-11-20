// The map's renderer: one description of what is drawn, two back ends. The shaders are written once in WGSL
// (web/shaders/); at build time naga writes them as GLSL ES 3.00 too, with where each binding lies (web/shade, put on
// the page as `SHADERS`). A pass draws rectangles of a target (the canvas, or a texture the renderer draws into to read
// back) with a program, its uniforms, textures and samplers given by binding. WebGPU draws where the browser has it
// (gpu-webgpu.ts), else WebGL2 (gpu-webgl2.ts).
import { openWebGpu } from './gpu-webgpu';
import { openWebGl2 } from './gpu-webgl2';

/** Where WebGL2 finds a binding of a program: a uniform block's binding, or a sampler's texture and sampler bindings
 * (null: the texture is read whole texels at a time, with no sampler). */
type Bind = { uniform: [number, number] } | { texture: [number, number]; sampler: [number, number] | null };
/** A program as web/shade writes it: its WGSL, its GLSL stages, the GLSL names of its bindings, the least metres a
 * pixel takes for which it is made once more (as its declaration of `LEAST_E` writes them), and the targets it draws
 * into at once (a pass of as many, `Gpu.pass`). */
export interface Program { wgsl: string; vertex: string; fragment: string; binds: Record<string, Bind>; least: string[]; targets: number }
declare const SHADERS: Record<string, Program>;
/** The declaration of `LEAST_E` in a program made for no least size of a pixel (WGSL and GLSL; web/shaders/lib/draw.wgsl),
 * and its value. */
const LEAST_DECLARED = /(LEAST_E(?:: f32)? = )0\.0;/;
let all: Record<string, Program> | null = null;
/** The page's programs (web/shaders/<name>.wgsl), and each one made for draws whose pixels take at least `m` metres,
 * named `<name>:<m>`: the same text with `LEAST_E` set to `m` (web/shade checks that this is what it makes). */
export function programs(): Record<string, Program> {
  if (all) return all;
  const out: Record<string, Program> = { ...SHADERS };
  for (const [name, p] of Object.entries(SHADERS)) {
    for (const m of p.least) {
      const set = (text: string) => text.replace(LEAST_DECLARED, `$1${m};`);
      out[`${name}:${m}`] = { ...p, wgsl: set(p.wgsl), vertex: set(p.vertex), fragment: set(p.fragment), least: [] };
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
export function madeFor(name: string, e: number): string[] {
  const fit = Object.keys(programs()).map(key => [key, key.split(':')] as const)
    .filter(([, [base, m]]) => base === name && m !== undefined && +m <= e * LEAST_SPARE)
    .sort((x, y) => +y[1][1] - +x[1][1]).map(([key]) => key);
  return [...fit, name];
}

/** A program for draws into a target (null: the canvas) over what is there or not: its name, the target's format and
 * whether it blends. */
export type Made = [name: string, target: Format | null, blend: boolean];

/** Of program `name`, the one a draw into `target` (null: the canvas), over what is there or not (`blend`), where a pixel
 * takes `e` metres, draws with: the best of those that may (`madeFor`) that is made, so a draw never waits for a
 * program while one that draws the same picture is made; the best is made in its turn (`Gpu.warm`). With none made,
 * the best is made before all others and the draw is drawn again once it is (`waiting`), meanwhile by the one made for
 * the nearest size of a pixel (none when none is made), and the page does not stop while it is made. */
export function seenFrom(gpu: Gpu, name: string, e: number, target: Format | null, blend: boolean, waiting: Made[]): string | null {
  const may = madeFor(name, e), made = may.find(key => gpu.ready(key, target, blend));
  gpu.warm(may[0], target, blend, made === undefined);
  if (made !== undefined) return made;
  waiting.push([may[0], target, blend]);
  // Till one is made, the one made for the nearest size of a pixel draws (a little more or less detail for a moment),
  // so a zoom past a size never takes away what was drawn.
  const size = (key: string) => Math.abs(Math.log(Math.max(+(key.split(':')[1] ?? 0), EVERY_SIZE_M) / e));
  return madeFor(name, Infinity).filter(key => gpu.ready(key, target, blend)).sort((x, y) => size(x) - size(y))[0] ?? null;
}
/** Where programs are sorted by the sizes of a pixel they are made for (`seenFrom`, map.ts `warmAhead`), the one made
 * for every size counts as made for this many metres. */
export const EVERY_SIZE_M = 0.25;

/** Of `programs`, those not made (`Gpu.ready`), each asked to be made before all others (`Gpu.warm`). */
export function notMade(gpu: Gpu, programs: Made[]): Made[] {
  const missing = programs.filter(([name, target, blend]) => !gpu.ready(name, target, blend));
  missing.forEach(([name, target, blend]) => gpu.warm(name, target, blend, true));
  return missing;
}

/** The programs a back end makes in the background (`Gpu.warm`): one at a time, in the order asked, each by `make`
 * (done when its promise is), none that is made already (`made`). */
export class Warming {
  private queue: string[] = [];
  private making: string | null = null;
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
  private next(): void {
    if (this.making !== null) return;
    let key = this.queue.shift();
    while (key !== undefined && this.made(key)) key = this.queue.shift();
    if (key === undefined) return;
    this.making = key;
    this.make(key).catch(e => console.error(`gpu: ${key}: ${e}`)).finally(() => { this.making = null; this.next(); });
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

/** What a texel holds: four bytes (read as 0..1), or four 32-bit floats, or four or one 16-bit floats (the floats read
 * whole texels at a time). */
export type Format = 'rgba8unorm' | 'rgba32float' | 'rgba16float' | 'rg16float' | 'r16float';
/** The data a texture of each format is filled with: bytes, 32-bit floats, or 16-bit floats as their bits. */
export type TexelData = Uint8Array | Float32Array | Uint16Array;
/** How a sampler reads a texture: blended between its texels' centres, and also between its mip levels' nearest; or
 * repeated across the plane and blended between its mip levels too (`tile`: a texture laid on the ground). */
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
  readonly kind: 'WebGPU' | 'WebGL2';
  /** A texture of `w` x `h` texels of `format`, `layers` deep with `mips` levels; `target`: it can be drawn into. */
  texture(w: number, h: number, format: Format, layers?: number, mips?: number, target?: boolean): Texture;
  /** Fill mip level `level` of texture `t` (all its layers, layer after layer, rows top first; or layer `layer` alone,
   * which the renderer does not keep: after a lost context it is empty). */
  write(t: Texture, data: TexelData, level?: number, layer?: number): void;
  drop(t: Texture): void;
  /** Draw into `target` (null: the canvas, sized to its pixels; several textures of one size and format: a program's
   * targets, `Program.targets`, in the order of its outputs), first filling it with `clear` (r, g, b, a, 0..1) if given,
   * and only inside `clip` (x, y, w, h px from its top-left corner) if given. */
  pass(target: Texture | Texture[] | null, clear?: number[], clip?: number[]): Pass;
  /** Make program `name` for draws into `target` (null: the canvas) over what is there or not (`blend`) in the
   * background, one program at a time in the order asked (`first`: before all others), so its first draw need not
   * wait for it. */
  warm(name: string, target: Format | null, blend: boolean, first?: boolean): void;
  /** Whether program `name` is made for such draws (a draw with one that is not waits until it is). */
  ready(name: string, target: Format | null, blend: boolean): boolean;
  /** Send what was drawn to the GPU. */
  submit(): void;
  /** Once the GPU has finished all that was sent. */
  done(): Promise<void>;
  /** The bytes of texture `t` (four a texel, rows top first), once the GPU has drawn it. */
  read(t: Texture): Promise<Uint8Array>;
}

/** The draw header every program reads at binding 0 (web/shaders/lib/draw.wgsl `Draw`): the target's size, px, and the sign of its rows, with the draw's alpha, rectangle and source. */
export function drawHeader(size: [number, number], flip: number, d: Draw): Float32Array {
  return new Float32Array([size[0], size[1], flip, d.alpha ?? 1, ...d.rect, ...(d.src ?? [0, 0, 1, 1])]);
}

/** A uniform buffer laid out as a WGSL struct whose members are all four-wide (vec4f, vec4i and arrays of them), so
 * that member k starts at byte 16 k on both back ends. */
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

/** The GPU of `canvas`: WebGPU where the browser offers it, else WebGL2; null with neither. */
export async function openGpu(canvas: HTMLCanvasElement, restored: () => void): Promise<Gpu | null> {
  return (await openWebGpu(canvas)) ?? openWebGl2(canvas, restored);
}
