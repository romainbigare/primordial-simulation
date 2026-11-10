// The live map as the GPU draws it (web/shaders/land.wgsl): the ground from its tiles (ground.ts), the state from the
// cells' table the world's worker sends with each frame (tape/src/table.rs), and the numbers of the laws the core
// applies inside a cell. The page adds to the table what it decides itself: the sea, the cover's byte and bare share,
// the true colour, and the layer drawn.
import { Gpu, Texture, Uniforms } from './gpu';
import { GroundTiles, TILE_FADE_MS } from './ground';
import { World, plane, decode } from './data';
import { MapLook, earthUniforms } from './field-window';
import { Wet, seaSquares, PATH_LAYER } from './earth';

/** The numbers of the laws inside a cell as the core sends them (wasm/src/lib.rs `meta`, `Physics`). */
export interface Laws { soil_m: number; wet_depth: number; dormancy: number; channel_width: number; channel_width_exp: number; channel_depth: number; channel_depth_exp: number }

/** Planes of four numbers a square of the table (land.wgsl `GROUND` to `VIEW`): the core's (`TileMeta.row` numbers), then
 * the page's three. */
const PLANES = 6;
/** The ground's slope at the view's depth is read as a square's slope (for its shading and its rock) times its texel's
 * size over a square's to this power, and no less than `SLOPE_LEAST` times it: the slope over a short run is steeper
 * than over a long one, and the map shades and colours slopes as they look from afar. */
const SLOPE_POWER = 0.5, SLOPE_LEAST = 0.15;
/** A sea shore is sand up to this height over the liquid's surface, m. */
const BEACH_M = 6;
/** The bytes of a layer view over which an empty point fades into a full one (colour.wgsl `in_view_of`). */
const OVER_FADE = 64;
/** The level a pond no body holds is drawn at, m below 0 (land.wgsl `NO_LIQUID_M`): under any ground. */
const NO_POND_M = 1000;
/** Of the uniforms of land.wgsl (`Land`), four-wide members. */
const LAND_VEC4S = 5;

export class Land {
  private data: Float32Array | null = null;
  private tex: Texture | null = null;
  private due = false;
  /** The rows of the table's texture: its planes, then the ponds' levels. */
  private rows = 0;
  /** The melting point of the frozen cover, K (0: none), as the table last said. */
  private melt = 0;

  constructor(readonly tiles: GroundTiles, private laws: Laws) {}

  /** The table of frame 0 of world `w`: the core's rows `rows` (the melting point, then a row a square, then each
   * square's kinds of cover, which canopy.ts reads, then the level of each pond), its wet squares `wet`, and the layer `look` draws. The ponds' levels lie in rows under the planes,
   * four a texel (land.wgsl `pond_level`; one no body holds far below any ground). */
  set(w: World, rows: Float32Array, wet: Wet, look: MapLook): void {
    const { w: W, h: H } = w.meta, n = W * H, { row, kinds, kind_row } = this.tiles.meta;
    const levels = rows.subarray(1 + (row + kinds * kind_row) * n);
    this.rows = PLANES * H + Math.ceil(levels.length / 4 / W);
    const data = new Float32Array(4 * W * this.rows);
    data.set(Array.from(levels, l => (Number.isNaN(l) ? -NO_POND_M : l)), 4 * PLANES * n);
    const sea = seaSquares(wet, w.meta), plants = plane(w, 0, 'plants'), leaf = decode(w, 0, 'leaf');
    const truth = ['true_r', 'true_g', 'true_b'].map(name => plane(w, 0, name));
    const shown = look.mode === 'layer' && w.index.has(look.layer) ? plane(w, 0, look.layer) : null;
    const worn = look.paths && w.index.has(PATH_LAYER) ? plane(w, 0, PATH_LAYER) : null;
    const put = (k: number, i: number, v: number[]) => data.set(v, 4 * (k * n + i));
    this.melt = rows[0];
    for (let i = 0; i < n; i++) {
      for (let k = 0; k < Math.ceil(row / 4); k++) put(k, i, Array.from(rows.subarray(1 + row * i + 4 * k, 1 + row * i + Math.min(4 * k + 4, row))));
      put(3, i, [plants[i] / 255, Math.exp(-leaf[i]), sea[i], 0]);
      put(4, i, [truth[0][i] / 255, truth[1][i] / 255, truth[2][i] / 255, 0]);
      put(5, i, [shown ? shown[i] / 255 : 0, worn ? worn[i] / 255 : 0, 0, 0]);
    }
    this.data = data;
    this.due = true;
  }

  /** The bindings land.wgsl reads on `gpu` in view `look` (`lawBinds` and colour.wgsl's), or null before the first
   * table. */
  binds(gpu: Gpu, look: MapLook): Record<number, ArrayBuffer | Texture> | null {
    const laws = this.lawBinds(gpu);
    if (!laws) return null;
    const m = this.tiles.map, texel = 2 ** (m.square_depth - this.tiles.depth);
    const read = [Math.max(texel ** SLOPE_POWER, SLOPE_LEAST), this.melt, BEACH_M, OVER_FADE];
    return { ...laws, 1: earthUniforms(look, read, [0, 0, m.w * m.cell_m, m.h * m.cell_m]) };
  }

  /** The bindings of lib/land.wgsl on `gpu` (its uniforms, the tiles and the table), or null before the first table. */
  lawBinds(gpu: Gpu): Record<number, ArrayBuffer | Texture> | null {
    if (!this.data) return null;
    const m = this.tiles.map;
    if (this.tex && this.tex.h !== this.rows) { gpu.drop(this.tex); this.tex = null; }
    this.tex ??= gpu.texture(m.w, this.rows, 'rgba32float');
    if (this.due) { gpu.write(this.tex, this.data); this.due = false; }
    const t = this.tiles.textures(gpu), l = this.laws;
    const u = new Uniforms(LAND_VEC4S).vec4(m.cell_m, m.w, m.h, m.square_depth).vec4(this.tiles.meta.size, this.tiles.meta.gutter, this.tiles.depth)
      .vec4(t.now, TILE_FADE_MS, this.melt, this.tiles.meta.runoff).vec4(l.soil_m, l.wet_depth, l.dormancy).vec4(l.channel_width, l.channel_width_exp, l.channel_depth, l.channel_depth_exp);
    return { 8: u.buffer, 9: t.atlas, 10: t.pages, 11: t.tiles, 12: this.tex, 13: t.areas };
  }

  stop(gpu: Gpu | null): void {
    this.tiles.stop(gpu);
    if (gpu && this.tex) gpu.drop(this.tex);
  }
}
