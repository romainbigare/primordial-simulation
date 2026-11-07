// A window of the map as the GPU draws it (web/shaders/earth.wgsl): its fields (fields.ts) as textures, and the
// uniforms of the view it is drawn in.
import { Gpu, Texture, Sampler, Uniforms } from './gpu';
import { Fields, FIELD_TEXTURES } from './fields';
import { RGB } from './earth';

/** The stops a layer's ramp holds at most (earth.wgsl `Earth.ramp`). */
const RAMP_STOPS = 10;
/** Of the uniforms of earth.wgsl (`Earth`), four-wide members. */
const EARTH_VEC4S = 3 + RAMP_STOPS;

/** What the map draws: the Earth's colours, the true colour, or a layer (its index in the table, and its ramp); and
 * the index of the worn paths' layer where they are drawn (-1: not drawn). */
export interface MapLook { mode: 'earth' | 'true' | number; ramp: RGB[]; paths: number }

/** A window of the map drawn from its fields: where it lies (x, y, w, h in squares), the share of it along its west,
 * north, east and south edges it fades out over, and when it came (performance.now, ms); its textures, made on the GPU
 * at its first draw, and the texture of the layer the view draws. */
export class FieldWindow {
  private textures: Record<number, Texture> | null = null;
  private view: Texture | null = null;
  private viewKey = '';
  constructor(readonly at: number[], readonly edge: number[], readonly fields: Fields, readonly since: number) {}

  /** The bindings earth.wgsl reads for this window on `gpu` in view `look`. */
  binds(gpu: Gpu, look: MapLook): Record<number, ArrayBuffer | Texture | Sampler> {
    const f = this.fields, n = f.w * f.h;
    if (!this.textures) {
      this.textures = {};
      for (const t of FIELD_TEXTURES) { const tex = gpu.texture(f.w, f.h, t.format); gpu.write(tex, f[t.field]); this.textures[t.binding] = tex; }
    }
    // The layer drawn and the worn paths, a byte each of the texel's four.
    const key = `${look.mode} ${look.paths}`;
    if (key !== this.viewKey) {
      const data = new Uint8Array(4 * n), plane = (k: number, ch: number) => { for (let i = 0; i < n; i++) data[4 * i + ch] = f.layers[k * n + i]; };
      if (typeof look.mode === 'number') plane(look.mode, 0);
      if (look.paths >= 0) plane(look.paths, 1);
      this.view ??= gpu.texture(f.w, f.h, 'rgba8unorm');
      gpu.write(this.view, data);
      this.viewKey = key;
    }
    const mode = look.mode === 'earth' ? 0 : look.mode === 'true' ? 1 : 2, u = new Uniforms(EARTH_VEC4S);
    u.vec4(...f.look).vec4(mode, look.ramp.length, look.paths >= 0 ? 1 : 0).vec4(...this.edge);
    for (let k = 0; k < RAMP_STOPS; k++) u.vec4(...(look.ramp[k] ?? [0, 0, 0]));
    return { ...this.textures, 1: u.buffer, 5: this.view!, 6: 'smooth' };
  }

  drop(gpu: Gpu | null): void {
    if (!gpu) return;
    for (const t of Object.values(this.textures ?? {})) gpu.drop(t);
    if (this.view) gpu.drop(this.view);
  }
}

