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
/** A program as web/shade writes it: its WGSL, its GLSL stages and the GLSL names of its bindings. */
export interface Program { wgsl: string; vertex: string; fragment: string; binds: Record<string, Bind> }
declare const SHADERS: Record<string, Program>;
/** The page's programs (web/shaders/<name>.wgsl). */
export const programs = (): Record<string, Program> => SHADERS;

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

/** What a texel holds: four bytes (read as 0..1) or four 32-bit floats (read whole texels at a time). */
export type Format = 'rgba8unorm' | 'rgba32float';
/** How a sampler reads a texture: blended between its texels' centres, and also between its mip levels' nearest. */
export type Sampler = 'smooth' | 'mip';
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
  /** Fill mip level `level` of texture `t` (all its layers, layer after layer, rows top first; or layer `layer` alone). */
  write(t: Texture, data: Uint8Array | Float32Array, level?: number, layer?: number): void;
  drop(t: Texture): void;
  /** Draw into `target` (null: the canvas, sized to its pixels), first filling it with `clear` (r, g, b, a, 0..1) if given,
   * and only inside `clip` (x, y, w, h px from its top-left corner) if given. */
  pass(target: Texture | null, clear?: number[], clip?: number[]): Pass;
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
