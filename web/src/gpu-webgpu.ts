// The map's renderer on WebGPU (gpu.ts): the programs' WGSL as it is, a pipeline for each program, target format and
// blend, and a bind group a draw.
import { Gpu, Texture, Format, TexelData, Sampler, Pass, Draw, Program, programs, bindingsOf, drawHeader } from './gpu';

// WebGPU's flags, as the browser defines them.
declare const GPUTextureUsage: { COPY_SRC: number; COPY_DST: number; TEXTURE_BINDING: number; RENDER_ATTACHMENT: number };
declare const GPUBufferUsage: { MAP_READ: number; COPY_DST: number; UNIFORM: number };
declare const GPUMapMode: { READ: number };

/** Bytes of a texel of each format. */
const TEXEL_BYTES: Record<Format, number> = { rgba8unorm: 4, rgba32float: 16, rgba16float: 8, rg16float: 4, r16float: 2 };
/** The size of each uniform buffer, bytes: no program's uniforms are larger. */
const UNIFORM_BYTES = 1024;
/** Rows copied out of a texture start at multiples of this many bytes (WebGPU's rule). */
const ROW_ALIGN = 256;

class WebGpuTexture implements Texture {
  constructor(readonly w: number, readonly h: number, readonly layers: number, readonly mips: number, readonly format: Format,
    readonly tex: GPUTexture, readonly view: GPUTextureView) {}
}

class WebGpu implements Gpu {
  readonly kind = 'WebGPU';
  private modules = new Map<string, GPUShaderModule>();
  private pipelines = new Map<string, GPURenderPipeline>();
  private samplers: Record<Sampler, GPUSampler>;
  private encoder: GPUCommandEncoder | null = null;
  /** The canvas's texture this frame (until the frame is sent). */
  private frame: GPUTexture | null = null;
  /** Uniform buffers: those from `used` on are free until the frame is sent. */
  private pool: GPUBuffer[] = [];
  private used = 0;

  constructor(readonly adapter: GPUAdapter, private device: GPUDevice, private ctx: GPUCanvasContext, private format: GPUTextureFormat) {
    const smooth: GPUSamplerDescriptor = { magFilter: 'linear', minFilter: 'linear', addressModeU: 'clamp-to-edge', addressModeV: 'clamp-to-edge' };
    this.samplers = { smooth: device.createSampler(smooth), mip: device.createSampler({ ...smooth, mipmapFilter: 'nearest' }) };
    device.lost.then(info => console.error(`gpu: the WebGPU device was lost: ${info.message}`));
    device.addEventListener('uncapturederror', e => console.error(`gpu: ${(e as GPUUncapturedErrorEvent).error.message}`));
  }

  texture(w: number, h: number, format: Format, layers = 1, mips = 1, target = false): Texture {
    const usage = GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | (target ? GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC : 0);
    const tex = this.device.createTexture({ size: [w, h, layers], format, mipLevelCount: mips, usage });
    return new WebGpuTexture(w, h, layers, mips, format, tex, tex.createView({ dimension: layers > 1 ? '2d-array' : '2d' }));
  }

  write(t: Texture, data: TexelData, level = 0, layer?: number): void {
    const w = Math.max(1, t.w >> level), h = Math.max(1, t.h >> level);
    const origin = [0, 0, layer ?? 0], layers = layer === undefined ? t.layers : 1;
    this.device.queue.writeTexture({ texture: (t as WebGpuTexture).tex, mipLevel: level, origin }, data as Uint8Array<ArrayBuffer>, { bytesPerRow: w * TEXEL_BYTES[t.format], rowsPerImage: h }, [w, h, layers]);
  }

  drop(t: Texture): void { (t as WebGpuTexture).tex.destroy(); }

  pass(target: Texture | null, clear?: number[], clip?: number[]): Pass {
    this.encoder ??= this.device.createCommandEncoder();
    const t = target as WebGpuTexture | null;
    if (!t) this.frame ??= this.ctx.getCurrentTexture();
    const tex = t ? t.tex : this.frame!, size: [number, number] = [tex.width, tex.height], format = t ? t.format : this.format;
    const enc = this.encoder.beginRenderPass({ colorAttachments: [{ view: t ? t.view : tex.createView(), loadOp: clear ? 'clear' : 'load', clearValue: clear, storeOp: 'store' }] });
    if (clip) enc.setScissorRect(clip[0], clip[1], clip[2], clip[3]);
    return {
      draw: (name: string, d: Draw, blend: boolean) => {
        const p = programs()[name], pipeline = this.pipeline(name, p, format, blend), b = bindingsOf(p);
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

  /** The pipeline of program `name` drawing into `format`, over what is there or not. */
  private pipeline(name: string, p: Program, format: GPUTextureFormat, blend: boolean): GPURenderPipeline {
    const key = `${name} ${format} ${blend}`;
    let made = this.pipelines.get(key);
    if (made) return made;
    let module = this.modules.get(name);
    if (!module) { module = this.device.createShaderModule({ code: p.wgsl, label: name }); this.modules.set(name, module); }
    const over: GPUBlendComponent = { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' };
    made = this.device.createRenderPipeline({
      layout: 'auto', label: name,
      vertex: { module, entryPoint: 'vs' },
      fragment: { module, entryPoint: 'fs', targets: [{ format, blend: blend ? { color: over, alpha: over } : undefined }] },
      primitive: { topology: 'triangle-strip' },
    });
    this.pipelines.set(key, made);
    return made;
  }
}

/** WebGPU on `canvas`, or null where the browser has none. */
export async function openWebGpu(canvas: HTMLCanvasElement): Promise<Gpu | null> {
  if (!navigator.gpu) return null;
  const adapter = await navigator.gpu.requestAdapter().catch(() => null);
  const device = adapter && await adapter.requestDevice().catch(() => null);
  const ctx = device && canvas.getContext('webgpu') as GPUCanvasContext | null;
  if (!device || !ctx) return null;
  const format = navigator.gpu.getPreferredCanvasFormat();
  ctx.configure({ device, format, alphaMode: 'opaque' });
  return new WebGpu(adapter!, device, ctx, format);
}
