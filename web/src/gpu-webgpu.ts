// The map's renderer on WebGPU (gpu.ts): the programs' WGSL as it is, a pipeline for each program, target format and
// blend, and a bind group a draw.
import { Gpu, Texture, Format, TexelData, Sampler, Pass, Draw, Warming, programs, bindingsOf, drawHeader } from './gpu';

// WebGPU's flags, as the browser defines them.
declare const GPUTextureUsage: { COPY_SRC: number; COPY_DST: number; TEXTURE_BINDING: number; RENDER_ATTACHMENT: number };
declare const GPUBufferUsage: { MAP_READ: number; COPY_DST: number; UNIFORM: number };
declare const GPUMapMode: { READ: number };

/** Bytes of a texel of each format. */
const TEXEL_BYTES: Record<Format, number> = { rgba8unorm: 4, rgba32float: 16, rgba16float: 8, rg16float: 4, r16float: 2 };
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
/** Times a lost device is replaced by a new one at most: a device lost again and again is lost for a cause the new one
 * meets too (the card's memory, a program it cannot run), and the page says so instead. */
const LOSSES_MOST = 2;

class WebGpuTexture implements Texture {
  tex!: GPUTexture;
  view!: GPUTextureView;
  /** What each mip level was last filled with whole, to fill it again on a new device (a layer filled alone is not
   * kept). */
  levels: TexelData[] = [];
  constructor(readonly w: number, readonly h: number, readonly layers: number, readonly mips: number, readonly format: Format, readonly target: boolean) {}
}

class WebGpu implements Gpu {
  readonly kind = 'WebGPU';
  private modules = new Map<string, GPUShaderModule>();
  private pipelines = new Map<string, GPURenderPipeline>();
  private warming = new Warming(key => this.makeAhead(key), key => this.pipelines.has(key));
  private samplers!: Record<Sampler, GPUSampler>;
  private textures = new Set<WebGpuTexture>();
  /** Devices lost so far. */
  private losses = 0;
  private encoder: GPUCommandEncoder | null = null;
  /** The canvas's texture this frame (until the frame is sent). */
  private frame: GPUTexture | null = null;
  /** Uniform buffers: those from `used` on are free until the frame is sent. */
  private pool: GPUBuffer[] = [];
  private used = 0;

  constructor(private device: GPUDevice, private ctx: GPUCanvasContext, private format: GPUTextureFormat, private restored: () => void) {
    this.setUp();
  }

  /** Configure the canvas for the device, make its samplers, and hear its errors and its loss. */
  private setUp(): void {
    const device = this.device;
    this.ctx.configure({ device, format: this.format, alphaMode: 'opaque' });
    const smooth: GPUSamplerDescriptor = { magFilter: 'linear', minFilter: 'linear', addressModeU: 'clamp-to-edge', addressModeV: 'clamp-to-edge' };
    const tile: GPUSamplerDescriptor = { magFilter: 'linear', minFilter: 'linear', mipmapFilter: 'linear', addressModeU: 'repeat', addressModeV: 'repeat', maxAnisotropy: TILE_ANISOTROPY };
    this.samplers = { smooth: device.createSampler(smooth), mip: device.createSampler({ ...smooth, mipmapFilter: 'nearest' }), tile: device.createSampler(tile) };
    device.lost.then(info => this.lost(info.message));
    device.addEventListener('uncapturederror', e => reportError(new Error(`gpu: ${(e as GPUUncapturedErrorEvent).error.message}`)));
  }

  /** The device was lost (a phone's browser drops it to free memory, or when the card stops): draw on a new one, its
   * textures made anew and those filled whole filled again (`restored` fills the rest), as WebGL2 does after a lost
   * context; after `LOSSES_MOST` losses, or where no new one is given, say so. */
  private async lost(why: string): Promise<void> {
    const device = ++this.losses <= LOSSES_MOST ? await deviceOf() : null;
    if (!device) return reportError(new Error(`gpu: the WebGPU device was lost (${why}); reload the page to draw the map again`));
    this.device = device;
    this.modules.clear(); this.pipelines.clear(); this.warming.clear();
    this.pool = []; this.used = 0; this.encoder = this.frame = null;
    this.setUp();
    for (const t of this.textures) { this.make(t); t.levels.forEach((d, k) => d && this.write(t, d, k)); }
    this.restored();
  }

  texture(w: number, h: number, format: Format, layers = 1, mips = 1, target = false): Texture {
    const t = new WebGpuTexture(w, h, layers, mips, format, target);
    this.make(t);
    this.textures.add(t);
    return t;
  }

  /** Make the GPU texture of `t` on the device. */
  private make(t: WebGpuTexture): void {
    const usage = GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | (t.target ? GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC : 0);
    t.tex = this.device.createTexture({ size: [t.w, t.h, t.layers], format: t.format, mipLevelCount: t.mips, usage });
    t.view = t.tex.createView({ dimension: t.layers > 1 ? '2d-array' : '2d' });
  }

  write(t: Texture, data: TexelData, level = 0, layer?: number): void {
    const w = Math.max(1, t.w >> level), h = Math.max(1, t.h >> level);
    const origin = [0, 0, layer ?? 0], layers = layer === undefined ? t.layers : 1;
    if (layer === undefined) (t as WebGpuTexture).levels[level] = data;
    this.device.queue.writeTexture({ texture: (t as WebGpuTexture).tex, mipLevel: level, origin }, data as Uint8Array<ArrayBuffer>, { bytesPerRow: w * TEXEL_BYTES[t.format], rowsPerImage: h }, [w, h, layers]);
  }

  drop(t: Texture): void { (t as WebGpuTexture).tex.destroy(); this.textures.delete(t as WebGpuTexture); }

  pass(target: Texture | Texture[] | null, clear?: number[], clip?: number[]): Pass {
    this.encoder ??= this.device.createCommandEncoder();
    const all = (Array.isArray(target) ? target : [target]) as (WebGpuTexture | null)[], t = all[0];
    if (!t) this.frame ??= this.ctx.getCurrentTexture();
    const tex = t ? t.tex : this.frame!, size: [number, number] = [tex.width, tex.height], format = t ? t.format : this.format;
    const views = t ? all.map(a => a!.view) : [tex.createView()];
    const enc = this.encoder.beginRenderPass({ colorAttachments: views.map(view => ({ view, loadOp: clear ? 'clear' : 'load', clearValue: clear, storeOp: 'store' })) });
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

  done(): Promise<void> { return this.device.queue.onSubmittedWorkDone(); }

  async read(t: Texture): Promise<Uint8Array> {
    const tex = (t as WebGpuTexture).tex, row = Math.ceil((4 * t.w) / ROW_ALIGN) * ROW_ALIGN;
    const buf = this.device.createBuffer({ size: row * t.h, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
    this.encoder ??= this.device.createCommandEncoder();
    this.encoder.copyTextureToBuffer({ texture: tex }, { buffer: buf, bytesPerRow: row }, [t.w, t.h]);
    this.submit();
    await buf.mapAsync(GPUMapMode.READ);
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
    const [name, format, blend] = key.split(' '), device = this.device;
    try {
      const made = await device.createRenderPipelineAsync(this.describe(name, format as GPUTextureFormat, blend === 'true'));
      if (this.device === device && !this.pipelines.has(key)) this.pipelines.set(key, made);
    } catch (e) {
      // One begun on a device lost since fails with it, and is made again on the new one when it is next asked for.
      if (this.device === device) throw e;
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

/** A WebGPU device of the browser's adapter, or null where it has none or gives none within `OPEN_MS`. */
async function deviceOf(): Promise<GPUDevice | null> {
  if (!navigator.gpu) return null;
  const late = new Promise<null>(none => setTimeout(() => none(null), OPEN_MS));
  const asked = navigator.gpu.requestAdapter().then(a => a && a.requestDevice()).catch(() => null);
  return Promise.race([asked, late]);
}

/** WebGPU on `canvas`, or null where the browser has none (`deviceOf`); `restored` redraws on a new device after a
 * lost one. */
export async function openWebGpu(canvas: HTMLCanvasElement, restored: () => void): Promise<Gpu | null> {
  const device = await deviceOf();
  const ctx = device && canvas.getContext('webgpu') as GPUCanvasContext | null;
  return device && ctx ? new WebGpu(device, ctx, CANVAS_FORMAT, restored) : null;
}
