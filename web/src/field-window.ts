// A window of the map as the GPU draws it (web/shaders/earth.wgsl): its fields (fields.ts) as textures, and the
// uniforms of the view it is drawn in.
import { Gpu, Texture, Sampler, Uniforms } from './gpu';
import { Fields, FIELD_TEXTURES } from './fields';
import { RGB, PATH_LAYER } from './earth';

/** The stops a layer's ramp holds at most (colour.wgsl `Earth.ramp`). */
const RAMP_STOPS = 10;
/** Of the uniforms of colour.wgsl (`Earth`), four-wide members. */
const EARTH_VEC4S = 3 + RAMP_STOPS;

/** What the map draws: the Earth's colours, the true colour, or a layer (its name, and its ramp); and whether the worn
 * paths are drawn. */
export interface MapLook { mode: 'earth' | 'true' | 'layer'; layer: string; ramp: RGB[]; paths: boolean }

/** The uniforms of colour.wgsl (`Earth`) for view `look`: how the fields are read (`Fields.look`), where they lie in
 * the world (`Fields.place`), and the sea's level, m. */
export function earthUniforms(look: MapLook, read: number[], place: number[], sea = 0): ArrayBuffer {
  const mode = look.mode === 'earth' ? 0 : look.mode === 'true' ? 1 : 2, u = new Uniforms(EARTH_VEC4S);
  u.vec4(...read).vec4(mode, look.ramp.length, look.paths ? 1 : 0, sea).vec4(...place);
  for (let k = 0; k < RAMP_STOPS; k++) u.vec4(...(look.ramp[k] ?? [0, 0, 0]));
  return u.buffer;
}

/** The map of squares drawn from its fields: its textures, made on the GPU at its first draw, and the texture of the
 * layer the view draws. */
export class FieldWindow {
  private textures: Record<number, Texture> | null = null;
  private view: Texture | null = null;
  private viewKey = '';
  constructor(readonly fields: Fields) {}

  /** The bindings earth.wgsl reads for this window on `gpu` in view `look`. */
  binds(gpu: Gpu, look: MapLook): Record<number, ArrayBuffer | Texture | Sampler> {
    const f = this.fields, n = f.w * f.h;
    if (!this.textures) {
      this.textures = {};
      for (const t of FIELD_TEXTURES) { const tex = gpu.texture(f.w, f.h, t.format); gpu.write(tex, f[t.field]); this.textures[t.binding] = tex; }
    }
    // The layer drawn and the worn paths, a byte each of the texel's four (none where the window does not hold it).
    const key = `${look.mode} ${look.layer} ${look.paths}`;
    if (key !== this.viewKey) {
      const data = new Uint8Array(4 * n), plane = (name: string, ch: number) => {
        const k = f.names.indexOf(name);
        if (k >= 0) for (let i = 0; i < n; i++) data[4 * i + ch] = f.layers[k * n + i];
      };
      if (look.mode === 'layer') plane(look.layer, 0);
      if (look.paths) plane(PATH_LAYER, 1);
      this.view ??= gpu.texture(f.w, f.h, 'rgba8unorm');
      gpu.write(this.view, data);
      this.viewKey = key;
    }
    return { ...this.textures, 1: earthUniforms(look, f.look, f.place), 5: this.view!, 6: 'smooth' };
  }

  drop(gpu: Gpu | null): void {
    if (!gpu) return;
    for (const t of Object.values(this.textures ?? {})) gpu.drop(t);
    if (this.view) gpu.drop(this.view);
  }
}

