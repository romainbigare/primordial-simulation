// The map's renderer on WebGPU (gpu.ts): the programs' WGSL as it is, a pipeline for each program, target format and
// blend, and a bind group a draw.
import { Back, TEXEL_BYTES, Texture, Format, TexelData, Sampler, Pass, Draw, Warming, programs, bindingsOf, drawHeader } from './gpu';

// WebGPU's flags, as the browser defines them.
declare const GPUTextureUsage: { COPY_SRC: number; COPY_DST: number; TEXTURE_BINDING: number; RENDER_ATTACHMENT: number };
declare const GPUBufferUsage: { MAP_READ: number; COPY_DST: number; UNIFORM: number };
declare const GPUMapMode: { READ: number };

/** The size of each uniform buffer, bytes: no program's uniforms are larger. */
const UNIFORM_BYTES = 1024;
/** A texture laid on the ground is read along the view's slant this many texels at a time at most (anisotropic). */
const TILE_ANISOTROPY = 4;
/** Rows copied out of a texture start at multiples of this many bytes (WebGPU's rule). */
const ROW_ALIGN = 256;
/** The canvas's format: that of the textures the map is drawn into below the screen's pixels (map.ts `MAP_TARGET`),
 * which every WebGPU canvas takes, so one pipeline of a program draws into both and none is made twice. */
const CANVAS_FORMAT: GPUTextureFormat = 'rgba8unorm';
/** The longest wait for the browser's adapter and device, ms: past it the map is drawn with WebGL2. */
const OPEN_MS = 5000;

class WebGpuTexture implements Texture {
  constructor(readonly w: number, readonly h: number, readonly layers: number, readonly mips: number, readonly format: Format,
    readonly tex: GPUTexture, readonly view: GPUTextureView) {}
}

class WebGpu implements Back {
  readonly kind = 'WebGPU';
  private modules = new Map<string, GPUShaderModule>();
  private pipelines = new Map<string, GPURenderPipeline>();
  private warming = new Warming(key => this.makeAhead(key), key => this.pipelines.has(key));
  private samplers: Record<Sampler, GPUSampler>;
  /** Whether the device is gone: lost, or ended by the page (`close`, whose loss is no news). What it was still
   * making or reading then fails with it, and is no news either. */
  private gone = false;
  /** Whether a render pass failed to begin (said once). */
  private passFailed = false;
  private encoder: GPUCommandEncoder | null = null;
  /** The canvas's texture this frame (until the frame is sent). */
  private frame: GPUTexture | null = null;
  /** Uniform buffers: those from `used` on are free until the frame is sent. */
  private pool: GPUBuffer[] = [];
  private used = 0;

  constructor(private adapter: GPUAdapter, private device: GPUDevice, private ctx: GPUCanvasContext, private format: GPUTextureFormat, lost: (why: string) => void) {
    ctx.configure({ device, format, alphaMode: 'opaque' });
    const smooth: GPUSamplerDescriptor = { magFilter: 'linear', minFilter: 'linear', addressModeU: 'clamp-to-edge', addressModeV: 'clamp-to-edge' };
    const tile: GPUSamplerDescriptor = { magFilter: 'linear', minFilter: 'linear', mipmapFilter: 'linear', addressModeU: 'repeat', addressModeV: 'repeat', maxAnisotropy: TILE_ANISOTROPY };
    this.samplers = { smooth: device.createSampler(smooth), mip: device.createSampler({ ...smooth, mipmapFilter: 'nearest' }), tile: device.createSampler(tile) };
    device.lost.then(info => { const ended = this.gone; this.gone = true; if (!ended) lost(info.message); });
    // An error the card gives without words (an out-of-memory error has none) is named by its kind.
    device.addEventListener('uncapturederror', e => { const err = (e as GPUUncapturedErrorEvent).error; reportError(new Error(`gpu: ${err.constructor.name}${err.message ? `: ${err.message}` : ''}`)); });
  }

  /** End the device: what it holds is let go, and nothing it was still making or drawing is heard of. */
  close(): void { this.gone = true; this.device.destroy(); }

  texture(label: string, w: number, h: number, format: Format, layers = 1, mips = 1, target = false): Texture {
    const usage = GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | (target ? GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC : 0);
    const tex = this.device.createTexture({ label, size: [w, h, layers], format, mipLevelCount: mips, usage });
    return new WebGpuTexture(w, h, layers, mips, format, tex, tex.createView({ dimension: layers > 1 ? '2d-array' : '2d' }));
  }

  write(t: Texture, data: TexelData, level = 0, layer?: number): void {
    const w = Math.max(1, t.w >> level), h = Math.max(1, t.h >> level);
    const origin = [0, 0, layer ?? 0], layers = layer === undefined ? t.layers : 1;
    this.device.queue.writeTexture({ texture: (t as WebGpuTexture).tex, mipLevel: level, origin }, data as Uint8Array<ArrayBuffer>, { bytesPerRow: w * TEXEL_BYTES[t.format], rowsPerImage: h }, [w, h, layers]);
  }

  drop(t: Texture): void { (t as WebGpuTexture).tex.destroy(); }

  making(): string { return this.warming.pending(); }

  about(): string {
    const i = this.adapter.info, l = this.adapter.limits;
    return `${i.vendor} ${i.architecture}${i.description ? ` ${i.description}` : ''}; adapter limits: 2D ${l.maxTextureDimension2D}, layers ${l.maxTextureArrayLayers}, buffer ${Math.round(l.maxBufferSize / 2 ** 20)} MB`;
  }

  pass(target: Texture | Texture[] | null, clear?: number[], clip?: number[]): Pass {
    this.encoder ??= this.device.createCommandEncoder();
    const all = (Array.isArray(target) ? target : [target]) as (WebGpuTexture | null)[], t = all[0];
    if (!t) this.frame ??= this.ctx.getCurrentTexture();
    const tex = t ? t.tex : this.frame!, size: [number, number] = [tex.width, tex.height], format = t ? t.format : this.format;
    const views = t ? all.map(a => a!.view) : [tex.createView()];
    let enc: GPURenderPassEncoder;
    try {
      enc = this.encoder.beginRenderPass({ colorAttachments: views.map(view => ({ view, loadOp: clear ? 'clear' : 'load', clearValue: clear, storeOp: 'store' })) });
    } catch (e) {
      // A device being lost refuses its passes before it says it is lost (`lost`): its frame is not drawn, and the
      // failure is said once a back end.
      if (!this.gone && !this.passFailed) reportError(new Error(`gpu: a render pass could not begin (${e instanceof Error ? e.message : e})`));
      this.passFailed = true;
      return { draw: () => {}, end: () => {} };
    }
    if (clip) enc.setScissorRect(clip[0], clip[1], clip[2], clip[3]);
    return {
      draw: (name: string, d: Draw, blend: boolean) => {
        const p = programs()[name], pipeline = this.pipeline(name, format, blend), b = bindingsOf(p);
        const entries: GPUBindGroupEntry[] = [];
        for (const n of b.uniforms) entries.push({ binding: n, resource: { buffer: this.uniform(n === 0 ? drawHeader(size, 1, d).buffer as ArrayBuffer : d.binds[n] as ArrayBuffer) } });
        for (const n of b.textures) entries.push({ binding: n, resource: (d.binds[n] as WebGpuTexture).view });
        for (const n of b.samplers) entries.push({ binding: n, resource: this.samplers[d.binds[n] as Sampler] });
        enc.setPipeline(pipeline);
        enc.setBindGroup(0, this.device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries }));
        enc.draw(4);
      },
      end: () => enc.end(),
    };
  }

  warm(name: string, target: Format | null, blend: boolean, first = false): void {
    this.warming.ask(pipelineKey(name, target ?? this.format, blend), first);
  }

  ready(name: string, target: Format | null, blend: boolean): boolean { return this.pipelines.has(pipelineKey(name, target ?? this.format, blend)); }

  submit(): void {
    if (this.encoder) this.device.queue.submit([this.encoder.finish()]);
    this.encoder = this.frame = null;
    this.used = 0;
  }

  /** (It fails only with the device, whose loss `lost` hears: then it never comes.) */
  done(): Promise<void> { return this.device.queue.onSubmittedWorkDone().catch(() => new Promise<void>(() => {})); }

  async read(t: Texture): Promise<Uint8Array> {
    const tex = (t as WebGpuTexture).tex, row = Math.ceil((4 * t.w) / ROW_ALIGN) * ROW_ALIGN;
    const buf = this.device.createBuffer({ size: row * t.h, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
    this.encoder ??= this.device.createCommandEncoder();
    this.encoder.copyTextureToBuffer({ texture: tex }, { buffer: buf, bytesPerRow: row }, [t.w, t.h]);
    this.submit();
    // A read the device took with it is never answered: the map asks again at its next draw.
    if (!await buf.mapAsync(GPUMapMode.READ).then(() => true, e => { if (this.gone) return false; throw e; })) return new Promise(() => {});
    const all = new Uint8Array(buf.getMappedRange()), out = new Uint8Array(4 * t.w * t.h);
    for (let y = 0; y < t.h; y++) out.set(all.subarray(y * row, y * row + 4 * t.w), 4 * t.w * y);
    buf.destroy();
    return out;
  }

  /** A uniform buffer of the pool holding `data`. */
  private uniform(data: ArrayBuffer): GPUBuffer {
    if (data.byteLength > UNIFORM_BYTES) throw new Error(`gpu: uniforms of ${data.byteLength} bytes`);
    if (this.used === this.pool.length) this.pool.push(this.device.createBuffer({ size: UNIFORM_BYTES, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST }));
    const buf = this.pool[this.used++];
    this.device.queue.writeBuffer(buf, 0, data);
    return buf;
  }

  /** The pipeline of program `name` drawing into `format`, over what is there or not: made at once where it is not
   * (also where it is being made ahead, `warm`, which then keeps this one). */
  private pipeline(name: string, format: GPUTextureFormat, blend: boolean): GPURenderPipeline {
    const key = pipelineKey(name, format, blend);
    let made = this.pipelines.get(key);
    if (!made) { made = this.device.createRenderPipeline(this.describe(name, format, blend)); this.pipelines.set(key, made); }
    return made;
  }

  /** Make the pipeline of `key` (`pipelineKey`) ahead, without the page waiting for it. */
  private async makeAhead(key: string): Promise<void> {
    const [name, format, blend] = key.split(' ');
    try {
      const made = await this.device.createRenderPipelineAsync(this.describe(name, format as GPUTextureFormat, blend === 'true'));
      if (!this.pipelines.has(key)) this.pipelines.set(key, made);
    } catch (e) {
      if (!this.gone) throw e;
    }
  }

  /** The pipeline of program `name` drawing into `format`, over what is there or not, as WebGPU is asked for it. */
  private describe(name: string, format: GPUTextureFormat, blend: boolean): GPURenderPipelineDescriptor {
    let module = this.modules.get(name);
    if (!module) { module = this.device.createShaderModule({ code: programs()[name].wgsl, label: name }); this.modules.set(name, module); }
    const over: GPUBlendComponent = { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' };
    const target: GPUColorTargetState = { format, blend: blend ? { color: over, alpha: over } : undefined };
    return {
      layout: 'auto', label: name,
      vertex: { module, entryPoint: 'vs' },
      fragment: { module, entryPoint: 'fs', targets: Array(programs()[name].targets).fill(target) },
      primitive: { topology: 'triangle-strip' },
    };
  }
}

/** The key of program `name`'s pipeline drawing into `format`, over what is there or not. */
const pipelineKey = (name: string, format: GPUTextureFormat, blend: boolean) => `${name} ${format} ${blend}`;

/** The browser's WebGPU adapter and a device of it, or null where it has none or gives none within `OPEN_MS`. */
async function deviceOf(): Promise<[GPUAdapter, GPUDevice] | null> {
  if (!navigator.gpu) return null;
  const late = new Promise<null>(none => setTimeout(() => none(null), OPEN_MS));
  const asked = navigator.gpu.requestAdapter().then(async a => (a ? [a, await a.requestDevice()] as [GPUAdapter, GPUDevice] : null)).catch(() => null);
  return Promise.race([asked, late]);
}

/** WebGPU on `canvas` and what ends it, or null where the browser has none (`deviceOf`); `lost` hears that its device
 * was lost. */
export async function openWebGpu(canvas: HTMLCanvasElement, lost: (why: string) => void): Promise<{ gpu: Back; close: () => void } | null> {
  const opened = await deviceOf();
  const ctx = opened && canvas.getContext('webgpu') as GPUCanvasContext | null;
  if (!opened || !ctx) return null;
  const gpu = new WebGpu(opened[0], opened[1], ctx, CANVAS_FORMAT, lost);
  return { gpu, close: () => gpu.close() };
}
