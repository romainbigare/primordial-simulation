// The map's renderer on WebGL2 (gpu.ts): the programs' GLSL ES 3.00 as naga wrote it from their WGSL (web/shade),
// each uniform block bound to its WGSL binding's number, each GLSL sampler on a texture unit of its own. A texture the
// renderer draws into is drawn upside down (the draw header's `flip`), so its first row is its top, as on WebGPU.
import { Gpu, Texture, Format, TexelData, Sampler, Pass, Draw, programs, drawHeader } from './gpu';

/** A linked program: where each binding of its WGSL lies. */
interface Linked { prog: WebGLProgram; units: { unit: number; texture: number; sampler: number | null }[] }

class WebGl2Texture implements Texture {
  tex: WebGLTexture | null = null;
  fb: WebGLFramebuffer | null = null;
  /** What each mip level was last filled with, to fill it again after the context is lost (a layer filled alone is
   * not kept). */
  levels: TexelData[] = [];
  constructor(readonly w: number, readonly h: number, readonly layers: number, readonly mips: number, readonly format: Format, readonly target: boolean) {}
  get array(): boolean { return this.layers > 1; }
}

class WebGl2 implements Gpu {
  readonly kind = 'WebGL2';
  private linked = new Map<string, Linked>();
  private samplers!: Record<Sampler | 'texel', WebGLSampler>;
  /** A uniform buffer a binding number. */
  private buffers: WebGLBuffer[] = [];
  private textures = new Set<WebGl2Texture>();

  constructor(private gl: WebGL2RenderingContext, private canvas: HTMLCanvasElement, restored: () => void) {
    this.setUp();
    canvas.addEventListener('webglcontextlost', e => e.preventDefault());
    canvas.addEventListener('webglcontextrestored', () => {
      this.linked.clear(); this.buffers = [];
      this.setUp();
      for (const t of this.textures) { this.make(t); t.levels.forEach((d, k) => d && this.write(t, d, k)); }
      restored();
    });
  }

  private setUp(): void {
    const gl = this.gl, sampler = (min: number, mag: number, wrap: number = gl.CLAMP_TO_EDGE) => {
      const s = gl.createSampler();
      gl.samplerParameteri(s, gl.TEXTURE_MIN_FILTER, min); gl.samplerParameteri(s, gl.TEXTURE_MAG_FILTER, mag);
      gl.samplerParameteri(s, gl.TEXTURE_WRAP_S, wrap); gl.samplerParameteri(s, gl.TEXTURE_WRAP_T, wrap);
      return s;
    };
    this.samplers = { smooth: sampler(gl.LINEAR, gl.LINEAR), mip: sampler(gl.LINEAR_MIPMAP_NEAREST, gl.LINEAR), texel: sampler(gl.NEAREST, gl.NEAREST), tile: sampler(gl.LINEAR_MIPMAP_LINEAR, gl.LINEAR, gl.REPEAT) };
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  }

  texture(w: number, h: number, format: Format, layers = 1, mips = 1, target = false): Texture {
    const t = new WebGl2Texture(w, h, layers, mips, format, target);
    this.make(t);
    this.textures.add(t);
    return t;
  }

  /** Make the GL objects of texture `t`. */
  private make(t: WebGl2Texture): void {
    const gl = this.gl, kind = t.array ? gl.TEXTURE_2D_ARRAY : gl.TEXTURE_2D, inner = this.formats()[t.format][0];
    t.tex = gl.createTexture();
    gl.bindTexture(kind, t.tex);
    if (t.array) gl.texStorage3D(kind, t.mips, inner, t.w, t.h, t.layers);
    else gl.texStorage2D(kind, t.mips, inner, t.w, t.h);
    if (t.target) {
      t.fb = gl.createFramebuffer();
      gl.bindFramebuffer(gl.FRAMEBUFFER, t.fb);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, t.tex, 0);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    }
  }

  /** Of each format: its internal format, the format of its data and the type of its data's numbers. */
  private formats(): Record<Format, [number, number, number]> {
    const gl = this.gl;
    return { rgba8unorm: [gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE], rgba32float: [gl.RGBA32F, gl.RGBA, gl.FLOAT], rgba16float: [gl.RGBA16F, gl.RGBA, gl.HALF_FLOAT], rg16float: [gl.RG16F, gl.RG, gl.HALF_FLOAT], r16float: [gl.R16F, gl.RED, gl.HALF_FLOAT] };
  }

  write(t: Texture, data: TexelData, level = 0, layer?: number): void {
    const gl = this.gl, g = t as WebGl2Texture, w = Math.max(1, t.w >> level), h = Math.max(1, t.h >> level);
    const [, format, type] = this.formats()[t.format];
    if (layer === undefined) g.levels[level] = data;
    if (gl.isContextLost()) return;
    if (g.array) { gl.bindTexture(gl.TEXTURE_2D_ARRAY, g.tex); gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, level, 0, 0, layer ?? 0, w, h, layer === undefined ? t.layers : 1, format, type, data); }
    else { gl.bindTexture(gl.TEXTURE_2D, g.tex); gl.texSubImage2D(gl.TEXTURE_2D, level, 0, 0, w, h, format, type, data); }
  }

  drop(t: Texture): void {
    const g = t as WebGl2Texture;
    this.gl.deleteTexture(g.tex); this.gl.deleteFramebuffer(g.fb);
    this.textures.delete(g);
  }

  pass(target: Texture | null, clear?: number[], clip?: number[]): Pass {
    const gl = this.gl, t = target as WebGl2Texture | null;
    const size: [number, number] = t ? [t.w, t.h] : [this.canvas.width, this.canvas.height];
    gl.bindFramebuffer(gl.FRAMEBUFFER, t ? t.fb : null);
    gl.viewport(0, 0, size[0], size[1]);
    if (clear) { gl.clearColor(clear[0], clear[1], clear[2], clear[3]); gl.clear(gl.COLOR_BUFFER_BIT); }
    // A texture's rows are kept top first (`flip`), the canvas's bottom first.
    if (clip) { gl.enable(gl.SCISSOR_TEST); gl.scissor(clip[0], t ? clip[1] : size[1] - clip[1] - clip[3], clip[2], clip[3]); }
    else gl.disable(gl.SCISSOR_TEST);
    return {
      draw: (name: string, d: Draw, blend: boolean) => {
        if (gl.isContextLost()) return;
        const l = this.link(name);
        gl.useProgram(l.prog);
        this.uniform(0, drawHeader(size, t ? -1 : 1, d));
        for (const [n, v] of Object.entries(d.binds)) if (v instanceof ArrayBuffer) this.uniform(+n, v);
        for (const u of l.units) {
          gl.activeTexture(gl.TEXTURE0 + u.unit);
          const tex = d.binds[u.texture] as WebGl2Texture;
          gl.bindTexture(tex.array ? gl.TEXTURE_2D_ARRAY : gl.TEXTURE_2D, tex.tex);
          gl.bindSampler(u.unit, this.samplers[u.sampler === null ? 'texel' : d.binds[u.sampler] as Sampler]);
        }
        if (blend) { gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA); } else gl.disable(gl.BLEND);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      },
      end: () => {},
    };
  }

  submit(): void { this.gl.flush(); }

  done(): Promise<void> {
    const gl = this.gl, sync = gl.fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE, 0);
    gl.flush();
    return new Promise(resolve => {
      const poll = () => {
        if (gl.isContextLost() || !sync || gl.getSyncParameter(sync, gl.SYNC_STATUS) === gl.SIGNALED) { if (sync && !gl.isContextLost()) gl.deleteSync(sync); resolve(); }
        else setTimeout(poll, FENCE_POLL_MS);
      };
      poll();
    });
  }

  /** Read into a buffer on the GPU, and copy it out once the GPU has drawn it, so the page never waits for the GPU. */
  async read(t: Texture): Promise<Uint8Array> {
    const gl = this.gl, out = new Uint8Array(4 * t.w * t.h), buf = gl.createBuffer();
    gl.bindBuffer(gl.PIXEL_PACK_BUFFER, buf);
    gl.bufferData(gl.PIXEL_PACK_BUFFER, out.byteLength, gl.STREAM_READ);
    gl.bindFramebuffer(gl.FRAMEBUFFER, (t as WebGl2Texture).fb);
    gl.readPixels(0, 0, t.w, t.h, gl.RGBA, gl.UNSIGNED_BYTE, 0);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.bindBuffer(gl.PIXEL_PACK_BUFFER, null);
    await this.done();
    if (!gl.isContextLost()) {
      gl.bindBuffer(gl.PIXEL_PACK_BUFFER, buf);
      gl.getBufferSubData(gl.PIXEL_PACK_BUFFER, 0, out);
      gl.bindBuffer(gl.PIXEL_PACK_BUFFER, null);
    }
    gl.deleteBuffer(buf);
    return out;
  }

  /** Bind `data` as the uniform block of binding `n`. */
  private uniform(n: number, data: ArrayBufferView | ArrayBuffer): void {
    const gl = this.gl;
    this.buffers[n] ??= gl.createBuffer();
    gl.bindBufferBase(gl.UNIFORM_BUFFER, n, this.buffers[n]);
    gl.bufferData(gl.UNIFORM_BUFFER, data, gl.DYNAMIC_DRAW);
  }

  /** Program `name`, compiled and linked once, its uniform blocks bound to their bindings and its samplers to units. */
  private link(name: string): Linked {
    let l = this.linked.get(name);
    if (l) return l;
    const gl = this.gl, p = programs()[name];
    const shader = (type: number, src: string) => {
      const s = gl.createShader(type)!;
      gl.shaderSource(s, src); gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(`gpu: ${name}: ${gl.getShaderInfoLog(s)}`);
      return s;
    };
    const prog = gl.createProgram();
    gl.attachShader(prog, shader(gl.VERTEX_SHADER, p.vertex)); gl.attachShader(prog, shader(gl.FRAGMENT_SHADER, p.fragment));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(`gpu: ${name}: ${gl.getProgramInfoLog(prog)}`);
    gl.useProgram(prog);
    const units: Linked['units'] = [];
    for (const [glName, b] of Object.entries(p.binds)) {
      if ('uniform' in b) {
        const k = gl.getUniformBlockIndex(prog, glName);
        if (k !== gl.INVALID_INDEX) gl.uniformBlockBinding(prog, k, b.uniform[1]);
      } else {
        gl.uniform1i(gl.getUniformLocation(prog, glName), units.length);
        units.push({ unit: units.length, texture: b.texture[1], sampler: b.sampler ? b.sampler[1] : null });
      }
    }
    l = { prog, units };
    this.linked.set(name, l);
    return l;
  }
}

/** How often a wait for the GPU looks whether it has finished, ms. */
const FENCE_POLL_MS = 4;

/** WebGL2 on `canvas`, or null where the browser has none; `restored` redraws after a lost context comes back. */
export function openWebGl2(canvas: HTMLCanvasElement, restored: () => void): Gpu | null {
  const gl = canvas.getContext('webgl2', { alpha: false, antialias: false, depth: false, premultipliedAlpha: true });
  return gl ? new WebGl2(gl, canvas, restored) : null;
}
