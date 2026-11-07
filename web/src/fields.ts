// The fields of a window of the map that the GPU draws from (web/shaders/lib/earth.wgsl): the page makes those of the
// map's squares from a frame (earth.ts `mapFields`), the worker those of a window finer than its cells (fine.ts
// `fineFields`). A field is a texture of four channels; a new one is a line of `FIELD_TEXTURES`, its channels filled by
// both makers, and its binding read in the shader.

/** The fields of a window, `w` x `h` texels row by row from its top-left corner, and how they are read. */
export interface Fields {
  w: number; h: number;
  /** Four floats a texel: the ground's height, m; the liquid's surface over the ground, m (negative where the ground
   * stands above it); the ground's slope east and south (rise over run). */
  height: Float32Array;
  /** Four floats a texel: the ice, m; the ground's temperature, K; the share of the texel that is the sea's; the share
   * that is a river. */
  matter: Float32Array;
  /** Four bytes a texel: the byte of the plant cover's mass (the `plants` layer) and the true colour (r, g, b). */
  bytes: Uint8Array;
  /** Four floats a texel: how wet the ground is, 0..1 (tape/src/fine/inside.rs; 0 where the frame does not say); the
   * depth of the river through it, m; two unused. */
  inside: Float32Array;
  /** Every layer of the table (tape/src/layers.rs), a byte a texel, layer after layer: a layer view and the worn paths
   * read them. */
  layers: Uint8Array;
  /** How the shader reads them (earth.wgsl `Earth.look`): the slope's factor, the melting point by which a texel's own
   * temperature lays snow (0: the ice alone), the height up to which a sea's shore is sand, m, and the bytes of a layer
   * over which an empty texel fades into a full one. */
  look: [number, number, number, number];
  /** Where the window lies in the world: its top-left corner and its size, m (the water's ripples are fixed there). */
  place: [number, number, number, number];
}

/** The textures of the fields, by their binding in earth.wgsl. */
export const FIELD_TEXTURES = [
  { binding: 2, field: 'height', format: 'rgba32float' },
  { binding: 3, field: 'matter', format: 'rgba32float' },
  { binding: 4, field: 'bytes', format: 'rgba8unorm' },
  { binding: 7, field: 'inside', format: 'rgba32float' },
] as const;

/** Set texel `i` of a field of four channels to (a, b, c, d). */
export function put4(f: Float32Array | Uint8Array, i: number, a: number, b: number, c: number, d: number): void {
  f[4 * i] = a; f[4 * i + 1] = b; f[4 * i + 2] = c; f[4 * i + 3] = d;
}

/** The buffers of `f`, to hand them from the worker to the page. */
export const fieldBuffers = (f: Fields): ArrayBuffer[] => [f.height.buffer, f.matter.buffer, f.bytes.buffer, f.inside.buffer, f.layers.buffer] as ArrayBuffer[];
