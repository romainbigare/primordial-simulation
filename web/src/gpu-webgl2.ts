// The map's renderer on WebGL2 (gpu.ts): the programs' GLSL ES 3.00 as naga wrote it from their WGSL (web/shade),
// each uniform block bound to its WGSL binding's number, each GLSL sampler on a texture unit of its own. A texture the
// renderer draws into is drawn upside down (the draw header's `flip`), so its first row is its top, as the canvas's.
import { Back, Texture, Format, TexelData, Sampler, Pass, Draw, Warming, programs, drawHeader } from './gpu';

/** A linked program: where each binding of its WGSL lies. */
interface Linked { prog: WebGLProgram; units: { unit: number; texture: number; sampler: number | null }[] }
/** The extension that compiles and links programs without stopping the page, and asks whether they are done. */
interface Parallel { COMPLETION_STATUS_KHR: number }
/** The extension that times what the card does (EXT_disjoint_timer_query_webgl2): a query of the time elapsed, and
 * whether a timing was disturbed (the card's clock jumped: its results mean nothing). */
interface Timer { TIME_ELAPSED_EXT: number; GPU_DISJOINT_EXT: number }

class WebGl2Texture implements Texture {
  tex: WebGLTexture | null = null;
  fb: WebGLFramebuffer | null = null;
  /** The framebuffer that draws into this texture and `with` after it, at once (`pass` of several targets). */
  fbWith: WebGLFramebuffer | null = null;
  with: WebGl2Texture[] = [];
  constructor(readonly w: number, readonly h: number, readonly layers: number, readonly mips: number, readonly format: Format, readonly target: boolean) {}
  get array(): boolean { return this.layers > 1; }
}

class WebGl2 implements Back {
  private linked = new Map<string, Linked>();
  /** Programs sent to compile and link and not yet looked at (`build`, `finish`). */
  private built = new Map<string, WebGLProgram>();
  private parallel: Parallel | null;
  /** The card's timer (none: the browser has none), and whether a timing is under way (one at a time). */
  private timerExt: Timer | null;
  private timing = false;
  private warming = new Warming(name => this.makeAhead(name), name => this.linked.has(name));
  private samplers!: Record<Sampler | 'texel', WebGLSampler>;
  /** A uniform buffer a binding number. */
  private buffers: WebGLBuffer[] = [];

  /** The noise's gradients every program that draws noise reads (`NOISE_BINDING`), made with the context. */
  private noise!: WebGl2Texture;
  /** Of each format: its internal format, the format of its data and the type of its data's numbers. */
  private formats: Record<Format, [number, number, number]>;

  constructor(private gl: WebGL2RenderingContext, private canvas: HTMLCanvasElement, restored: () => void) {
    this.parallel = gl.getExtension('KHR_parallel_shader_compile') as Parallel | null;
    this.timerExt = gl.getExtension('EXT_disjoint_timer_query_webgl2') as Timer | null;
    this.formats = { rgba8unorm: [gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE], 'rgba8unorm-srgb': [gl.SRGB8_ALPHA8, gl.RGBA, gl.UNSIGNED_BYTE], rgba32float: [gl.RGBA32F, gl.RGBA, gl.FLOAT], rgba16float: [gl.RGBA16F, gl.RGBA, gl.HALF_FLOAT], rg16float: [gl.RG16F, gl.RG, gl.HALF_FLOAT], r16float: [gl.R16F, gl.RED, gl.HALF_FLOAT] };
    this.setUp();
    // A lost context draws nothing and throws nothing: say so, so a page that stops drawing (a program too large for
    // the card, which the browser drops the context for) is seen.
    canvas.addEventListener('webglcontextlost', e => { e.preventDefault(); reportError(new Error('gpu: the WebGL2 context was lost; the map is drawn again once the browser gives it back')); });
    canvas.addEventListener('webglcontextrestored', () => {
      this.linked.clear(); this.built.clear(); this.warming.clear(); this.buffers = [];
      this.setUp();
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
    this.samplers = { smooth: sampler(gl.LINEAR_MIPMAP_LINEAR, gl.LINEAR), mip: sampler(gl.LINEAR_MIPMAP_NEAREST, gl.LINEAR), texel: sampler(gl.NEAREST, gl.NEAREST), tile: sampler(gl.LINEAR_MIPMAP_LINEAR, gl.LINEAR, gl.REPEAT) };
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    this.noise = this.texture('noise', NOISE_SIDE, NOISE_SIDE, 'rgba32float') as WebGl2Texture;
    this.write(this.noise, noiseTable());
  }

  texture(_label: string, w: number, h: number, format: Format, layers = 1, mips = 1, target = false): Texture {
    const t = new WebGl2Texture(w, h, layers, mips, format, target);
    this.make(t);
    return t;
  }

  /** Make the GL objects of texture `t`. */
  private make(t: WebGl2Texture): void {
    const gl = this.gl, kind = t.array ? gl.TEXTURE_2D_ARRAY : gl.TEXTURE_2D, inner = this.formats[t.format][0];
    t.tex = gl.createTexture();
    t.fbWith = null;
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

  /** The framebuffer that draws into `all` at once, in their order: kept with the first, made anew when the others
   * differ. */
  private framebufferOf(all: WebGl2Texture[]): WebGLFramebuffer | null {
    const gl = this.gl, [t, ...rest] = all;
    if (t.fbWith && t.with.length === rest.length && t.with.every((w, k) => w === rest[k])) return t.fbWith;
    gl.deleteFramebuffer(t.fbWith);
    t.fbWith = gl.createFramebuffer();
    t.with = rest;
    gl.bindFramebuffer(gl.FRAMEBUFFER, t.fbWith);
    all.forEach((a, k) => gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0 + k, gl.TEXTURE_2D, a.tex, 0));
    gl.drawBuffers(all.map((_, k) => gl.COLOR_ATTACHMENT0 + k));
    return t.fbWith;
  }

  write(t: Texture, data: TexelData, level = 0, layer?: number): void {
    const gl = this.gl, g = t as WebGl2Texture, w = Math.max(1, t.w >> level), h = Math.max(1, t.h >> level);
    const [, format, type] = this.formats[t.format];
    if (gl.isContextLost()) return;
    if (g.array) { gl.bindTexture(gl.TEXTURE_2D_ARRAY, g.tex); gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, level, 0, 0, layer ?? 0, w, h, layer === undefined ? t.layers : 1, format, type, data); }
    else { gl.bindTexture(gl.TEXTURE_2D, g.tex); gl.texSubImage2D(gl.TEXTURE_2D, level, 0, 0, w, h, format, type, data); }
  }

  writeRows(t: Texture, data: TexelData, y: number, rows: number): void {
    const gl = this.gl, g = t as WebGl2Texture, [, format, type] = this.formats[t.format];
    if (gl.isContextLost()) return;
    const row = (data.length / t.h) | 0;
    gl.bindTexture(gl.TEXTURE_2D, g.tex);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, y, t.w, rows, format, type, data, y * row);
  }

  mipmap(t: Texture): void {
    const gl = this.gl, g = t as WebGl2Texture;
    if (gl.isContextLost() || g.mips < 2) return;
    gl.bindTexture(gl.TEXTURE_2D, g.tex);
    gl.generateMipmap(gl.TEXTURE_2D);
  }

  drop(t: Texture): void {
    const g = t as WebGl2Texture;
    this.gl.deleteTexture(g.tex); this.gl.deleteFramebuffer(g.fb); this.gl.deleteFramebuffer(g.fbWith);
  }

  pass(target: Texture | Texture[] | null, clear?: number[], clip?: number[]): Pass {
    const gl = this.gl, all = (Array.isArray(target) ? target : [target]) as (WebGl2Texture | null)[], t = all[0];
    const size: [number, number] = t ? [t.w, t.h] : [this.canvas.width, this.canvas.height];
    gl.bindFramebuffer(gl.FRAMEBUFFER, all.length > 1 ? this.framebufferOf(all as WebGl2Texture[]) : t ? t.fb : null);
    gl.viewport(0, 0, size[0], size[1]);
    // A texture's rows are kept top first (`flip`), the canvas's bottom first. (The clip holds for the clear too.)
    if (clip) { gl.enable(gl.SCISSOR_TEST); gl.scissor(clip[0], t ? clip[1] : size[1] - clip[1] - clip[3], clip[2], clip[3]); }
    else gl.disable(gl.SCISSOR_TEST);
    if (clear) { gl.clearColor(clear[0], clear[1], clear[2], clear[3]); gl.clear(gl.COLOR_BUFFER_BIT); }
    return {
      draw: (name: string, d: Draw, blend: boolean) => {
        if (gl.isContextLost()) return;
        const l = this.link(name);
        gl.useProgram(l.prog);
        this.uniform(0, drawHeader(size, t ? -1 : 1, d));
        for (const [n, v] of Object.entries(d.binds)) if (v instanceof ArrayBuffer) this.uniform(+n, v);
        for (const u of l.units) {
          gl.activeTexture(gl.TEXTURE0 + u.unit);
          // (The noise's gradients are the renderer's own: no draw gives them.)
          const tex = (u.texture === NOISE_BINDING ? this.noise : d.binds[u.texture]) as WebGl2Texture;
          gl.bindTexture(tex.array ? gl.TEXTURE_2D_ARRAY : gl.TEXTURE_2D, tex.tex);
          gl.bindSampler(u.unit, this.samplers[u.sampler === null ? 'texel' : d.binds[u.sampler] as Sampler]);
        }
        if (blend) { gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA); } else gl.disable(gl.BLEND);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      },
      end: () => {},
    };
  }

  warm(name: string, first = false): void { this.warming.ask(name, first); }

  ready(name: string): boolean {
    if (this.linked.has(name)) return true;
    const prog = this.built.get(name);
    if (!prog || !this.parallel || !this.gl.getProgramParameter(prog, this.parallel.COMPLETION_STATUS_KHR)) return false;
    this.finish(name, prog);
    return true;
  }

  submit(): void { this.gl.flush(); }

  making(): string { return this.warming.pending(); }

  about(): string {
    const gl = this.gl, info = gl.getExtension('WEBGL_debug_renderer_info');
    return info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : String(gl.getParameter(gl.RENDERER));
  }

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

  timer(): { end(): Promise<number | null> } {
    const gl = this.gl, ext = this.timerExt;
    if (!ext || this.timing || gl.isContextLost()) {
      return { end: () => { const t0 = performance.now(); return this.done().then(() => performance.now() - t0); } };
    }
    const q = gl.createQuery();
    gl.beginQuery(ext.TIME_ELAPSED_EXT, q);
    this.timing = true;
    return {
      end: () => {
        gl.endQuery(ext.TIME_ELAPSED_EXT);
        gl.flush();
        this.timing = false;
        return new Promise(resolve => {
          const poll = () => {
            if (gl.isContextLost()) return resolve(null);
            if (!gl.getQueryParameter(q, gl.QUERY_RESULT_AVAILABLE)) { setTimeout(poll, FENCE_POLL_MS); return; }
            const ns = gl.getQueryParameter(q, gl.QUERY_RESULT) as number, disturbed = gl.getParameter(ext.GPU_DISJOINT_EXT) as boolean;
            gl.deleteQuery(q);
            resolve(disturbed ? null : ns / NS_A_MS);
          };
          poll();
        });
      },
    };
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

  /** Program `name`, compiled and linked once (waiting for it where it is being made), its uniform blocks bound to their
   * bindings and its samplers to units. */
  private link(name: string): Linked {
    return this.linked.get(name) ?? this.finish(name, this.built.get(name) ?? this.build(name));
  }

  /** Send program `name` to compile and link, without asking how it went (which would wait for it). */
  private build(name: string): WebGLProgram {
    const gl = this.gl, p = programs()[name], prog = gl.createProgram();
    for (const [type, src] of [[gl.VERTEX_SHADER, p.vertex], [gl.FRAGMENT_SHADER, p.fragment]] as const) {
      const s = gl.createShader(type)!;
      gl.shaderSource(s, src); gl.compileShader(s); gl.attachShader(prog, s);
    }
    gl.linkProgram(prog);
    this.built.set(name, prog);
    return prog;
  }

  /** Program `name` sent as `prog` (`build`), checked (waiting for it where it is not done) and set up: its uniform
   * blocks bound to their bindings and its samplers to units. */
  private finish(name: string, prog: WebGLProgram): Linked {
    const gl = this.gl, p = programs()[name];
    this.built.delete(name);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      const logs = gl.getAttachedShaders(prog)?.map(s => gl.getShaderInfoLog(s)).join('\n');
      throw new Error(`gpu: ${name}: ${gl.getProgramInfoLog(prog)}\n${logs}`);
    }
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
    const l = { prog, units };
    this.linked.set(name, l);
    return l;
  }

  /** Make program `name` ahead (`warm`): sent to compile and link, then looked at once the browser says it is done
   * (`KHR_parallel_shader_compile`); where it cannot say, looked at when the page is idle (which waits for it then). */
  private makeAhead(name: string): Promise<void> {
    const prog = this.build(name);
    return new Promise(done => {
      const look = () => {
        if (this.gl.isContextLost() || this.built.get(name) !== prog) return done();
        if (this.parallel && !this.ready(name)) { setTimeout(look, WARM_POLL_MS); return; }
        if (!this.parallel) this.finish(name, prog);
        done();
      };
      if (this.parallel) look(); else idle(look);
    });
  }
}

/** The binding of the noise's gradients (draw.wgsl `noise_gradients`) and the lattice points a side of its table
 * (draw.wgsl `NOISE_SIDE`). */
const NOISE_BINDING = 29, NOISE_SIDE = 256;
/** The noise's gradients (draw.wgsl `gradient`): of each lattice point of the table, row by row, the two 16-bit halves
 * of its random word of stream 0 (draw.wgsl `lowbias`, lowbias32, Wellons 2018), each -1..1, four floats a texel. */
function noiseTable(): Float32Array {
  const out = new Float32Array(4 * NOISE_SIDE * NOISE_SIDE);
  for (let y = 0; y < NOISE_SIDE; y++) for (let x = 0; x < NOISE_SIDE; x++) {
    let h = (Math.imul(x, 0x9e3779b1) ^ Math.imul(y, 0x85ebca77)) >>> 0;
    h ^= h >>> 16; h = Math.imul(h, 0x7feb352d) >>> 0; h ^= h >>> 15; h = Math.imul(h, 0x846ca68b) >>> 0; h ^= h >>> 16;
    out.set([((h & 0xffff) * 2) / 65535 - 1, ((h >>> 16) * 2) / 65535 - 1], 4 * (y * NOISE_SIDE + x));
  }
  return out;
}

/** How often a program made ahead is looked at for whether it is done, ms. */
const WARM_POLL_MS = 16;
/** The longest wait for the page to be idle before a program is made anyway, ms: a page that draws while the world
 * plays may never be idle, and the views that need the program would wait for it for ever. */
const IDLE_MOST_MS = 1000;
/** Run `f` when the page is idle, or after `IDLE_MOST_MS` (at once where the browser cannot say). */
const idle = (f: () => void) => ('requestIdleCallback' in window ? requestIdleCallback(() => f(), { timeout: IDLE_MOST_MS }) : setTimeout(f, 0));

/** How often a wait for the GPU looks whether it has finished, ms. */
const FENCE_POLL_MS = 4;
/** Nanoseconds in a millisecond (the card's timer counts nanoseconds). */
const NS_A_MS = 1e6;

/** WebGL2 on `canvas`, or null where the browser has none; `restored` redraws after a lost context comes back. */
export function openWebGl2(canvas: HTMLCanvasElement, restored: () => void): Back | null {
  const gl = canvas.getContext('webgl2', { alpha: false, antialias: false, depth: false, premultipliedAlpha: true });
  return gl ? new WebGl2(gl, canvas, restored) : null;
}
