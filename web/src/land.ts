// The live map as the GPU draws it (web/shaders/land.wgsl): the ground from its tiles (ground.ts), the state from the
// cells' table the world's worker sends with each frame (tape/src/table.rs), what runs through each of the terrain's
// samples (sent when it changed enough to show), and the numbers of the laws the core applies inside a cell. The page
// adds to the table what it decides itself: the ground's own colour and the sea, the true colour, and the layer drawn.
import { Draw, Gpu, Texture, Uniforms } from './gpu';
import { GroundTiles, TILE_FADE_MS } from './ground';
import { World, plane } from './data';
import { MapLook, earthUniforms } from './field-window';
import { Wet, seaSquares, PATH_LAYER } from './earth';
import { half } from './canopy';

/** The numbers of the laws inside a cell as the core sends them (wasm/src/lib.rs `meta`, `Physics`), and the discharge
 * per metre a css pixel takes past which a channel shows as a line, m³/s per m. */
export interface Laws { soil_m: number; wet_depth: number; dormancy: number; channel_width: number; channel_width_exp: number; channel_depth: number; channel_depth_exp: number; line: number }
/** The terrain's samples: across, down, and their depth. */
export interface SamplesOf { across: number; down: number; depth: number }

/** Planes of four numbers a square of the table (land.wgsl `GROUND` to `VIEW`): the core's (`TileMeta.row` numbers), then
 * the page's three. */
const PLANES = 7;
/** The ground's slope at the view's depth is read as a square's slope (for its shading and its rock) times its texel's
 * size over a square's to this power, and no less than `SLOPE_LEAST` times it: the slope over a short run is steeper
 * than over a long one, and the map shades and colours slopes as they look from afar. */
const SLOPE_POWER = 0.5, SLOPE_LEAST = 0.15;
/** A sea shore is sand up to this height over the liquid's surface, m (colour.wgsl `beach`). */
const BEACH_M = 5;
/** The bytes of a layer view over which an empty point fades into a full one (colour.wgsl `in_view_of`). */
const OVER_FADE = 64;
/** The level a pond no body holds is drawn at, m below 0 (land.wgsl `NO_LIQUID_M`): under any ground. */
const NO_POND_M = 1000;
/** Before the rock of the samples came, the ground is drawn as if its loose ground were this deep, m: no bed shows. */
const NO_ROCK_YET_M = 1;
/** Of the uniforms of land.wgsl (`Land`), four-wide members. */
const LAND_VEC4S = 6;

export class Land {
  private data: Float32Array | null = null;
  private tex: Texture | null = null;
  private due = false;
  /** The rows of the table's texture: its planes, then the ponds' levels. */
  private rows = 0;
  /** The melting point of the frozen cover, K (0: none), and the sea's level, m (the lowest body's), as the table last
   * said. */
  private melt = 0;
  private sea = 0;
  /** What runs through each sample, m³/s, as 16-bit floats, a texel a sample (none before the world's worker sent it),
   * and whether it is still to be sent to the GPU. */
  private flows: Texture | null = null;
  private discharge: Uint16Array;
  private flowsDue = true;
  /** The rock of each sample on the GPU (ground.ts `rock`), and whether it is the ground's (before it came: loose ground
   * of `NO_ROCK_YET_M` everywhere). */
  private rock: Texture | null = null;
  private rockSent = false;

  constructor(readonly tiles: GroundTiles, private laws: Laws, readonly samples: SamplesOf) {
    this.discharge = new Uint16Array(samples.across * samples.down);
  }

  /** The table of frame 0 of world `w`: the core's rows `rows` (the melting point, then a row a square, then each
   * square's kinds of cover, which canopy.ts reads, then the level of each pond), the samples' discharge if it came
   * since (`discharge`, else null), its wet squares `wet`, and the layer `look` draws. The ponds' levels lie in rows
   * under the planes, four a texel (land.wgsl `pond_level`; one no body holds far below any ground). */
  set(w: World, rows: Float32Array, discharge: Uint16Array | null, wet: Wet, look: MapLook): void {
    if (discharge) { this.discharge = discharge; this.flowsDue = true; }
    const { w: W, h: H } = w.meta, n = W * H, { row, kinds, kind_row } = this.tiles.meta;
    const levels = rows.subarray(1 + (row + kinds * kind_row) * n);
    this.rows = PLANES * H + Math.ceil(levels.length / 4 / W);
    const data = new Float32Array(4 * W * this.rows);
    data.set(Array.from(levels, l => (Number.isNaN(l) ? -NO_POND_M : l)), 4 * PLANES * n);
    this.sea = levels.reduce((low, l) => (l < low ? l : low), Infinity);
    if (!Number.isFinite(this.sea)) this.sea = -NO_POND_M;
    const sea = seaSquares(wet, w.meta), ground = ['ground_r', 'ground_g', 'ground_b'].map(name => plane(w, 0, name));
    const truth = ['true_r', 'true_g', 'true_b'].map(name => plane(w, 0, name));
    const shown = look.mode === 'layer' && w.index.has(look.layer) ? plane(w, 0, look.layer) : null;
    const worn = look.paths && w.index.has(PATH_LAYER) ? plane(w, 0, PATH_LAYER) : null;
    const put = (k: number, i: number, v: number[]) => data.set(v, 4 * (k * n + i));
    this.melt = rows[0];
    for (let i = 0; i < n; i++) {
      for (let k = 0; k < Math.ceil(row / 4); k++) put(k, i, Array.from(rows.subarray(1 + row * i + 4 * k, 1 + row * i + Math.min(4 * k + 4, row))));
      put(4, i, [ground[0][i] / 255, ground[1][i] / 255, ground[2][i] / 255, sea[i]]);
      put(5, i, [truth[0][i] / 255, truth[1][i] / 255, truth[2][i] / 255, 0]);
      put(6, i, [shown ? shown[i] / 255 : 0, worn ? worn[i] / 255 : 0, 0, 0]);
    }
    this.data = data;
    this.due = true;
  }

  /** The bindings land.wgsl reads on `gpu` in view `look` (`lawBinds` and colour.wgsl's), or null before the first
   * table. */
  binds(gpu: Gpu, look: MapLook): Draw['binds'] | null {
    const laws = this.lawBinds(gpu);
    if (!laws) return null;
    const m = this.tiles.map, texel = 2 ** (m.square_depth - this.tiles.depth);
    const read = [Math.max(texel ** SLOPE_POWER, SLOPE_LEAST), this.melt, BEACH_M, OVER_FADE];
    return { ...laws, 1: earthUniforms(look, read, [0, 0, m.w * m.cell_m, m.h * m.cell_m], this.sea) };
  }

  /** The bindings of lib/land.wgsl on `gpu` (its uniforms, the tiles and the table), or null before the first table. */
  lawBinds(gpu: Gpu): Draw['binds'] | null {
    if (!this.data) return null;
    const m = this.tiles.map;
    if (this.tex && this.tex.h !== this.rows) { gpu.drop(this.tex); this.tex = null; }
    this.tex ??= gpu.texture(m.w, this.rows, 'rgba32float');
    if (this.due) { gpu.write(this.tex, this.data); this.due = false; }
    const s = this.samples;
    this.flows ??= gpu.texture(s.across, s.down, 'r16float');
    if (this.flowsDue) { gpu.write(this.flows, this.discharge); this.flowsDue = false; }
    this.rock ??= gpu.texture(s.across, 2 * s.down, 'rgba16float');
    if (!this.rockSent) {
      const r = this.tiles.rock;
      gpu.write(this.rock, r ?? Uint16Array.from({ length: 8 * s.across * s.down }, (_, i) => (i % 4 || i >= 4 * s.across * s.down ? 0 : half(NO_ROCK_YET_M))));
      this.rockSent = r !== null;
    }
    const t = this.tiles.textures(gpu), l = this.laws, tm = this.tiles.meta;
    const u = new Uniforms(LAND_VEC4S).vec4(m.cell_m, m.w, m.h, m.square_depth).vec4(tm.size, tm.gutter, this.tiles.depth, tm.step_most)
      .vec4(t.now, TILE_FADE_MS, this.melt, l.line).vec4(l.soil_m, l.wet_depth, l.dormancy).vec4(l.channel_width, l.channel_width_exp, l.channel_depth, l.channel_depth_exp)
      .vec4(s.across, s.down, s.depth, devicePixelRatio);
    return { 8: u.buffer, 9: t.atlas, 10: t.pages, 11: t.tiles, 12: this.tex, 13: t.areas, 17: this.flows, 18: this.rock, 19: 'smooth' };
  }

  stop(gpu: Gpu | null): void {
    this.tiles.stop(gpu);
    if (gpu) for (const t of [this.tex, this.flows, this.rock]) if (t) gpu.drop(t);
  }
}
