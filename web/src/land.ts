// The live map as the GPU draws it (web/shaders/land.wgsl): the ground from its tiles (ground.ts), the state from the
// cells' table the world's worker sends with each frame (tape/src/table.rs), what runs through each of the terrain's
// samples (sent when it changed enough to show), and the numbers of the laws the core applies inside a cell. The page
// adds to the table what it decides itself: the ground's own colour and the sea, the true colour, and the layer drawn.
import { Draw, Gpu, Texture, Uniforms } from './gpu';
import { GroundTiles, TILE_FADE_MS } from './ground';
import { World, plane, decode } from './data';
import { MapLook, earthUniforms } from './field-window';
import { Wet, seaSquares } from './earth';
import { half, unhalf } from './canopy';
import { groundDetail } from './detail';

/** The numbers of the laws inside a cell as the core sends them (wasm/src/lib.rs `meta`, `Physics`), the discharge
 * per metre a css pixel takes past which a channel shows as a line, m³/s per m, and that line's law (its css pixels,
 * the power it widens by, the most times: tape/src/ground/channels.rs `LINE_PX`). */
export interface Laws { soil_m: number; wet_depth: number; dormancy: number; rooted_from: number; channel_width: number; channel_width_exp: number; channel_depth: number; channel_depth_exp: number; line: number; line_law: [number, number, number]; flood: number; station: number }
/** The terrain's samples: across, down, and their depth. */
export interface SamplesOf { across: number; down: number; depth: number }

/** Planes of four numbers a square of the table (land.wgsl `GROUND` to `VIEW`): the core's (`TileMeta.row` numbers), then
 * the page's three. */
const PLANES = 6;
/** The number of the table a square's wetness index of saturation goes in (land.wgsl `saturated_at`): the last of its
 * soil's plane, past the core's numbers. */
const SATURATED_AT = 4 * 2 + 3;
/** The ground's slope at the view's depth is read as a square's slope (for its shading and its rock) times its texel's
 * size over a square's to this power, and no less than `SLOPE_LEAST` times it: the slope over a short run is steeper
 * than over a long one, and the map shades and colours slopes as they look from afar. */
const SLOPE_POWER = 0.5, SLOPE_LEAST = 0.15;
/** A sea shore is sand up to this height over the liquid's surface, m (colour.wgsl `beach`). */
const BEACH_M = 5;
/** The bytes of a layer view over which an empty point fades into a full one (colour.wgsl `in_view_of`): of a log layer,
 * whose least bytes are next to nothing; a layer of another scale fades over its first byte alone, as its least value
 * is a value like any other (a share of none). */
const OVER_FADE = 64;
/** The level a pond no body holds is drawn at, m below 0 (land.wgsl `NO_LIQUID_M`): under any ground. */
const NO_POND_M = 1000;
/** Before the rock of the samples came, the ground is drawn as if its loose ground were this deep, m: no bed shows. */
const NO_ROCK_YET_M = 1;
/** The planes of the rock of the samples, one under the other (rock.wgsl `ROCK_PLANES`). */
const ROCK_PLANES = 4;
/** Of the uniforms of land.wgsl (`Land`), four-wide members. */
const LAND_VEC4S = 8;
/** The open low ground freezes cell by cell, and its cells are blocks of squares (8 km: the core's coarse low ground), so
 * an open sea square's ice and temperature are drawn blurred over the open sea squares about it, by a Gaussian of
 * `SEA_BLUR` squares out to `SEA_REACH`: the ice's edge follows a smooth field through the blocks' values, and the same
 * ice is drawn on the mean. A square is open sea where it is the sea's and its ground lies `OPEN_SEA_M` under its
 * surface on the mean: a coast's squares hold land, whose ice is its snow. */
const SEA_BLUR = 1.5, SEA_REACH = 4, OPEN_SEA_M = 20;
/** The signs of the flow on a channel's water slide along it (lib/flow.wgsl): they step this many times a second, the
 * slowest water's by this many metres a second; they show where a pixel takes no more than `FLOW_SHOWS_E` m (the
 * shader's streaks fade out there) on water at least `FLOW_PX` pixels wide. */
export const FLOW_FPS = 6;
const FLOW_M_S = 0.35, FLOW_SHOWS_E = 0.6, FLOW_PX = 4;

export class Land {
  /** The screen's pixels a css pixel, which the detail drawn is chosen by (map.ts `drawMap`): a picture drawn at fewer
   * pixels and stretched is the same picture, and what is sized in css pixels does not change. */
  drawn = devicePixelRatio;
  private data: Float32Array | null = null;
  private tex: Texture | null = null;
  private due = false;
  /** The rows of the table's texture: its planes, then the ponds' levels and fetches. */
  private rows = 0;
  /** The melting point of the frozen cover, K (0: none), and the sea's level, m (the lowest body's), as the table last
   * said. */
  private melt = 0;
  private sea = 0;
  /** The bytes of the layer shown over which an empty point fades into a full one (`OVER_FADE`). */
  private fade = OVER_FADE;
  /** What runs through each sample, m³/s, as 16-bit floats, a texel a sample (none before the world's worker sent it),
   * and whether it is still to be sent to the GPU. */
  private flows: Texture | null = null;
  private discharge: Uint16Array;
  private flowsDue = true;
  /** The rock of each sample on the GPU (ground.ts `rock`), and whether it is the ground's (before it came: loose ground
   * of `NO_ROCK_YET_M` everywhere). */
  private rock: Texture | null = null;
  private rockSent = false;
  /** The time the flow's signs stand at while one picture is drawn in tiles (performance.now, ms; null: each draw's
   * own), so all its tiles show the signs at one place along the channels. */
  flowAt: number | null = null;

  constructor(readonly tiles: GroundTiles, private laws: Laws, private samples: SamplesOf) {
    this.discharge = new Uint16Array(samples.across * samples.down);
  }

  /** The table of frame 0 of world `w`: the core's rows `rows` (the melting point, then a row a square, then each
   * square's kinds of cover, which canopy.ts reads, then the level of each pond, then its fetch), the samples'
   * discharge if it came since (`discharge`, else null), its wet squares `wet`, and the layer `look` draws. The ponds
   * lie in rows under the planes, two a texel, each its level and its fetch (land.wgsl `pond_texel`; a level no body
   * holds far below any ground). */
  set(w: World, rows: Float32Array, discharge: Uint16Array | null, wet: Wet, look: MapLook): void {
    if (discharge) { this.discharge = discharge; this.flowsDue = true; }
    const { w: W, h: H } = w.meta, n = W * H, { row, kinds, kind_row } = this.tiles.meta;
    const ponds = rows.subarray(1 + (row + kinds * kind_row) * n), count = ponds.length / 2;
    const levels = ponds.subarray(0, count), fetches = ponds.subarray(count);
    this.rows = PLANES * H + Math.ceil(count / 2 / W);
    const data = new Float32Array(4 * W * this.rows);
    data.set(Array.from(levels).flatMap((l, p) => [Number.isNaN(l) ? -NO_POND_M : l, fetches[p]]), 4 * PLANES * n);
    this.sea = levels.reduce((low, l) => (l < low ? l : low), Infinity);
    if (!Number.isFinite(this.sea)) this.sea = -NO_POND_M;
    const sea = seaSquares(wet, w.meta), ground = fromLand(['ground_r', 'ground_g', 'ground_b'].map(name => plane(w, 0, name)), wet.sea, W, H);
    // The land's mean height, ground temperature and ice of each square (the sea's squares' from the land nearest): the land a
    // coastal square of the open low ground holds is the land's, not its 8 km cell's.
    const [landZ, landT, landIce] = fromLand([0, 1, 3].map(k => Float32Array.from({ length: n }, (_, i) => rows[1 + row * i + k])), wet.sea, W, H);
    // The true colour, read over standing liquid alone (land.wgsl `fs`): the dry squares' from the wet ones nearest.
    const truth = fromLand(['true_r', 'true_g', 'true_b'].map(name => plane(w, 0, name)), Uint8Array.from(wet.wet, v => 1 - v), W, H);
    // The share of organic matter of each square's loose ground, which the Earth view darkens it by (land.wgsl).
    const organic = w.index.has('organic') ? decode(w, 0, 'organic') : new Float32Array(n);
    // The wetness index from which each square's land is saturated (the core's `inside::wet_ground`).
    const saturated = decode(w, 0, 'lambda_sat');
    const shown = look.mode === 'layer' && w.index.has(look.layer) ? plane(w, 0, look.layer) : null;
    this.fade = shown && w.meta.layers[w.index.get(look.layer)!].scale !== 'log' ? 1 : OVER_FADE;
    const put = (k: number, i: number, v: number[]) => data.set(v, 4 * (k * n + i));
    this.melt = rows[0];
    for (let i = 0; i < n; i++) {
      for (let k = 0; k < Math.ceil(row / 4); k++) put(k, i, Array.from(rows.subarray(1 + row * i + 4 * k, 1 + row * i + Math.min(4 * k + 4, row))));
      put(3, i, [ground[0][i] / 255, ground[1][i] / 255, ground[2][i] / 255, sea[i]]);
      put(4, i, [truth[0][i] / 255, truth[1][i] / 255, truth[2][i] / 255, landIce[i]]);
      put(5, i, [shown ? shown[i] / 255 : 0, organic[i], landT[i], landZ[i]]);
      data[4 * n * Math.floor(SATURATED_AT / 4) + 4 * i + SATURATED_AT % 4] = saturated[i];
    }
    blurSea(data, Array.from(sea, (s, i) => (s > 0 && data[4 * i] < this.sea - OPEN_SEA_M ? 1 : 0)), W, H);
    // The sea's squares are marked by their air's fall with height as -(it + 1) (land.wgsl `state_on`): only a point
    // with one of them about it reads the land's own numbers.
    for (let i = 0; i < n; i++) if (wet.sea[i]) data[4 * i + 2] = -(data[4 * i + 2] + 1);
    this.data = data;
    this.due = true;
  }

  /** The bindings land.wgsl reads on `gpu` in view `look` (`lawBinds` and colour.wgsl's), for a draw whose pixels lie as
   * `frame` says (field-window.ts `frameOf`), or null before the first table. */
  binds(gpu: Gpu, look: MapLook, frame: number[]): Draw['binds'] | null {
    const laws = this.lawBinds(gpu);
    if (!laws) return null;
    const m = this.tiles.map, texel = 2 ** (m.square_depth - this.tiles.depth);
    const read = [Math.max(texel ** SLOPE_POWER, SLOPE_LEAST), this.melt, BEACH_M, this.fade];
    return { ...laws, 1: earthUniforms(look, read, [0, 0, m.w * m.cell_m, m.h * m.cell_m], this.sea, frame), 20: groundDetail(gpu), 21: 'tile' };
  }

  /** How far the flow's signs have slid along the slowest water at `now` (performance.now, ms), m: in steps of
   * 1 / `FLOW_FPS` s, modulo the length a place along a channel is kept modulo (the signs come again after it, so the
   * slide has no jump). */
  private slid(now: number): number {
    return ((Math.floor((now * FLOW_FPS) / 1000) / FLOW_FPS) * FLOW_M_S) % this.tiles.meta.along_wrap;
  }

  /** Whether a channel in the window of squares from (`x`, `y`), `w` x `h` of them, shows the signs of its flow where a
   * pixel takes `e` m: one of the samples there carries water at least `FLOW_PX` pixels wide (the bed's width by the
   * channel law at the flood's discharge; the water fills most of it). */
  flowing(x: number, y: number, w: number, h: number, e: number): boolean {
    if (e > FLOW_SHOWS_E) return false;
    const s = this.samples, per = s.across / this.tiles.map.w, l = this.laws;
    // The discharge whose bed is `FLOW_PX` pixels wide.
    const least = (FLOW_PX * e / l.channel_width) ** (1 / l.channel_width_exp) / l.flood;
    const x0 = Math.max(0, Math.floor(x * per)), x1 = Math.min(s.across - 1, Math.ceil((x + w) * per));
    const y0 = Math.max(0, Math.floor(y * per)), y1 = Math.min(s.down - 1, Math.ceil((y + h) * per));
    for (let r = y0; r <= y1; r++) for (let c = x0; c <= x1; c++) if (unhalf(this.discharge[r * s.across + c]) >= least) return true;
    return false;
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
    this.rock ??= gpu.texture(s.across, ROCK_PLANES * s.down, 'rgba16float');
    if (!this.rockSent) {
      const r = this.tiles.rock;
      gpu.write(this.rock, r ?? Uint16Array.from({ length: 4 * ROCK_PLANES * s.across * s.down }, (_, i) => (i % 4 || i >= 4 * s.across * s.down ? 0 : half(NO_ROCK_YET_M))));
      this.rockSent = r !== null;
    }
    const t = this.tiles.textures(gpu), l = this.laws, tm = this.tiles.meta;
    const u = new Uniforms(LAND_VEC4S).vec4(m.cell_m, m.w, m.h, m.square_depth).vec4(tm.size, tm.gutter, this.tiles.depth, tm.step_most)
      .vec4(t.now, TILE_FADE_MS, this.melt, l.line).vec4(l.soil_m, l.wet_depth, l.dormancy, l.rooted_from).vec4(l.channel_width, l.channel_width_exp, l.channel_depth, l.channel_depth_exp)
      .vec4(s.across, s.down, s.depth, this.drawn).vec4(tm.along_wrap, l.flood, this.slid(this.flowAt ?? performance.now()), l.flood ** -l.station).vec4(...l.line_law);
    return { 8: u.buffer, 9: t.atlas, 10: t.pages, 11: t.tiles, 12: this.tex, 13: t.areas, 17: this.flows, 18: this.rock, 19: 'smooth' };
  }

  stop(gpu: Gpu | null): void {
    this.tiles.stop(gpu);
    if (gpu) for (const t of [this.tex, this.flows, this.rock]) if (t) gpu.drop(t);
  }
}

/** The open sea squares' ground temperature and ice (the first plane's second and fourth numbers) of the table `data`, `w`
 * x `h` squares, `open` 1 where open sea, blurred over the open sea squares about each (`SEA_BLUR`). */
function blurSea(data: Float32Array, open: ArrayLike<number>, w: number, h: number): void {
  const k = Array.from({ length: 2 * SEA_REACH + 1 }, (_, d) => Math.exp(-0.5 * ((d - SEA_REACH) / SEA_BLUR) ** 2));
  const was = data.slice(0, 4 * w * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const i = y * w + x;
    if (open[i] <= 0) continue;
    let t = 0, ice = 0, sum = 0;
    for (let dy = -SEA_REACH; dy <= SEA_REACH; dy++) for (let dx = -SEA_REACH; dx <= SEA_REACH; dx++) {
      const yy = y + dy, xx = x + dx;
      if (yy < 0 || yy >= h || xx < 0 || xx >= w) continue;
      const j = yy * w + xx, wt = k[dx + SEA_REACH] * k[dy + SEA_REACH] * open[j];
      if (wt <= 0) continue;
      t += wt * was[4 * j + 1]; ice += wt * was[4 * j + 3]; sum += wt;
    }
    data[4 * i + 1] = t / sum; data[4 * i + 3] = ice / sum;
  }
}

/** Planes `planes` (a number a square) of the map's `w` x `h` squares with the squares marked in `other` (1 where
 * marked) taken from the nearest squares not marked, ring by ring (each the mean of its neighbours already found). The
 * open low ground is cells of 8 km, whose numbers are their liquid surface's: the land a coastal square of one holds is
 * the land's, so no 8 km square of the sea's numbers shows on a coast; and the liquid's true colour, read over the
 * liquid alone, is the wet squares', so no land's colour blurs into the water along a shore. */
function fromLand(planes: ArrayLike<number>[], other: ArrayLike<number>, w: number, h: number): Float32Array[] {
  const out = planes.map(p => Float32Array.from(p)), known = Uint8Array.from(other, v => (v ? 0 : 1));
  for (let left = true; left;) {
    left = false;
    const now = known.slice();
    for (let i = 0; i < w * h; i++) {
      if (known[i]) continue;
      const x = i % w, y = (i / w) | 0;
      let n = 0;
      const sum = [0, 0, 0];
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
        const xx = x + dx, yy = y + dy, j = yy * w + xx;
        if (xx < 0 || yy < 0 || xx >= w || yy >= h || !known[j]) continue;
        out.forEach((p, c) => (sum[c] += p[j]));
        n++;
      }
      if (n) { out.forEach((p, c) => (p[i] = sum[c] / n)); now[i] = 1; } else left = true;
    }
    if (now.every((v, i) => v === known[i])) break;
    known.set(now);
  }
  return out;
}
