// The live map as the GPU draws it (web/shaders/land.wgsl): the ground from its tiles (ground.ts), the state from the
// cells' table the world's worker sends with each frame (tape/src/table.rs), what runs through each of the terrain's
// samples (sent when it changed enough to show), and the numbers of the laws the core applies inside a cell. The page
// adds to the table what it decides itself: the ground's own colour and the sea, the true colour, and the layer drawn.
import { Draw, Gpu, Texture, Uniforms } from './gpu';
import { GroundTiles, Preview, TILE_FADE_MS } from './ground';
import { World, plane, decode } from './data';
import { MapLook, earthUniforms } from './field-window';
import { Wet, seaSquares } from './earth';
import { half, unhalf } from './canopy';
import { groundDetail } from './detail';

/** The numbers of the laws inside a cell as the core sends them (wasm/src/lib.rs `meta`, `Physics`), the discharge
 * per metre a css pixel takes past which a channel shows as a line, m³/s per m, and that line's law (its css pixels,
 * the power it widens by, the most times: tape/src/ground/channels.rs `LINE_PX`). */
export interface Laws { soil_m: number; wet_depth: number; rooted_from: number; channel_width: number; channel_width_exp: number; channel_depth: number; channel_depth_exp: number; line: number; line_law: [number, number, number]; flood: number; station: number }
/** The terrain's samples: across, down, and their depth. */
export interface SamplesOf { across: number; down: number; depth: number }

/** Planes of four numbers a square of the table (land.wgsl `GROUND` to `GRAIN_SHIFT`): the core's row (`TileMeta.row`
 * numbers) with the wetness index of saturation (`SATURATED_AT`), then the page's four. */
const PLANES = 7;
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
/** Seconds in a day. */
export const DAY_S = 86400;
/** A plume (drawing only: the core moves no currents in a body of liquid): what a river brings into standing liquid,
 * kg/s, is in the river's own water at its mouth, its discharge (no less than `MUD_Q_LEAST` m³/s): its load there, g/m³,
 * held to `MUD_PLUME_MOST_G`. That water spreads over the body as a layer `MUD_PLUME_DEEP_M` thick for
 * `MUD_PLUME_DAYS` before its load is mixed down and settled (a river's plume is a fresh layer a metre or so thick over
 * the body's water, its mud settling out within hours to days; from memory): the load falls by e over the radius of the
 * area the river fills in that time, no more than `MUD_PLUME_MOST_M`, out to where it falls to `MUD_CLEAR_G` g/m³; a
 * plume narrower than a sample (125 m) is drawn as its mean over the sample. So a river's plume is as wide as its
 * discharge makes it, and it shows the farther the more load it brings. */
const MUD_Q_LEAST = 0.01, MUD_PLUME_MOST_G = 2e4, MUD_PLUME_DEEP_M = 1, MUD_PLUME_DAYS = 1, MUD_PLUME_MOST_M = 8000, MUD_CLEAR_G = 1;

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
  /** What runs through each sample, m³/s, and what its liquid carries, g/m³, as 16-bit floats, a texel a sample (none
   * before the world's worker sent them); the plumes spread into the bodies of liquid (`plumesIn`); how far edits
   * moved each sample's ground since the samples the core was told the page shows (`preview`), m; the three as the GPU
   * reads them (the plumes' load where it is more than the sample's own), and whether that is still to be sent. */
  private flows: Texture | null = null;
  private discharge: Uint16Array;
  private plumes: Uint8Array = new Uint8Array(0);
  private moved: Float32Array;
  private flowsDue = true;
  /** The version of the world's samples the core was last told the page's ground shows (`Live.groundShown`): the
   * preview of edits is of what moved since. */
  told: number;
  /** The rock of each sample on the GPU (ground.ts `rock`), and the version of the ground's rock it holds (ground.ts
   * `rockVersion`; before it came: loose ground of `NO_ROCK_YET_M` everywhere). */
  private rock: Texture | null = null;
  private rockAt = -1;
  /** Of each square, the mean of its samples' shares of coarse and fine grain (the rock's third plane), and the version
   * of the rock they are of (`grainShift`). */
  private grainOf: { version: number; coarse: Float32Array; fine: Float32Array } | null = null;
  /** The time the flow's signs stand at while one picture is drawn in tiles (performance.now, ms; null: each draw's
   * own), so all its tiles show the signs at one place along the channels. */
  flowAt: number | null = null;

  /** `version`: the version of the world's samples `tiles` are made of. */
  constructor(readonly tiles: GroundTiles, private laws: Laws, private samples: SamplesOf, version: number) {
    this.discharge = new Uint16Array(2 * samples.across * samples.down);
    this.moved = new Float32Array(samples.across * samples.down);
    this.told = version;
  }

  /** The ground shows the samples of a version the core was not told of yet: that version (now told), else null. */
  toTell(): number | null {
    const v = this.tiles.shownVersion();
    if (v === null || v === this.told) return null;
    this.told = v;
    return v;
  }

  /** What edits moved the ground since the samples the core was told the page shows (wasm/src/lib.rs `preview`): drawn
   * over the tiles made of those samples (land.wgsl `moved_at`), till the tiles of the samples built again come. */
  preview(p: Preview): void {
    const across = this.samples.across, [x0, y0, w, h] = p.window;
    this.moved.fill(0);
    for (let y = 0; y < h; y++) this.moved.set(p.dh.subarray(y * w, (y + 1) * w), (y0 + y) * across + x0);
    // (Over no tile where nothing moved: the map reads none of it.)
    this.tiles.previewOver(w * h > 0 ? this.told : NaN);
    this.flowsDue = true;
  }

  /** Where load came into a body of liquid in the last month, `bytes` (a sample, u32, and kg/s, f32, each: wasm/src/lib.rs
   * `plumes`): spread into the flows the GPU reads when they are new (`flowsOf`). */
  plumesIn(bytes: Uint8Array): void {
    // (Each frame brings its own bytes: only new plumes make the flows again.)
    if (bytes.length === this.plumes.length && bytes.every((b, i) => b === this.plumes[i])) return;
    this.plumes = bytes;
    this.flowsDue = true;
  }

  /** The flows the GPU reads: the samples' discharge and load, with each plume's load (`MUD_PLUME_DAYS`) where it is
   * more, and how far edits moved each sample's ground (`preview`). */
  private flowsOf(): Uint16Array {
    const s = this.samples, out = new Uint16Array(4 * s.across * s.down), view = new DataView(this.plumes.buffer, this.plumes.byteOffset, this.plumes.byteLength);
    const sampleM = (this.tiles.map.cell_m * this.tiles.map.w) / s.across;
    for (let k = 0; k < s.across * s.down; k++) out.set([this.discharge[2 * k], this.discharge[2 * k + 1], half(this.moved[k])], 4 * k);
    for (let p = 0; p + 8 <= this.plumes.byteLength; p += 8) {
      const k = view.getUint32(p, true), kg = view.getFloat32(p + 4, true), x = k % s.across, y = Math.floor(k / s.across);
      if (!(kg > 0) || k >= s.across * s.down) continue;
      // The river's discharge at its mouth: the most of the sample and those about it (under a body none runs).
      let q = 0;
      for (let j = Math.max(0, y - 1); j <= Math.min(s.down - 1, y + 1); j++) for (let i = Math.max(0, x - 1); i <= Math.min(s.across - 1, x + 1); i++) q = Math.max(q, unhalf(this.discharge[2 * (j * s.across + i)]));
      const flow = Math.max(q, MUD_Q_LEAST), at = Math.min((1000 * kg) / flow, MUD_PLUME_MOST_G);
      const fall = Math.min(Math.sqrt((flow * MUD_PLUME_DAYS * DAY_S) / (Math.PI * MUD_PLUME_DEEP_M)), MUD_PLUME_MOST_M) / sampleM;
      // A plume narrower than a sample is its mean over the sample: its e-fold's area, 2π fall², over the sample's.
      // and is drawn falling by e over a sample at least, so no sample's square shows.
      const mean = at * Math.min(1, 2 * Math.PI * fall * fall), spread = Math.max(fall, 1);
      const r = Math.ceil(spread * Math.log(Math.max(mean / MUD_CLEAR_G, 1)));
      for (let j = Math.max(0, y - r); j <= Math.min(s.down - 1, y + r); j++) {
        for (let i = Math.max(0, x - r); i <= Math.min(s.across - 1, x + r); i++) {
          const d = Math.hypot(i - x, j - y), c = mean * Math.exp(-d / spread), t = 4 * (j * s.across + i) + 1;
          if (d <= r && c > unhalf(out[t])) out[t] = half(c);
        }
      }
    }
    return out;
  }

  /** The table of frame 0 of world `w`: the core's rows `rows` (the melting point, then a row a square, then each
   * square's kinds of cover, the places of its patches and their dead walls, which canopy.ts reads, then the level of
   * each pond, then its
   * fetch, last the kinds a square holds), the samples'
   * discharge and load if they came since (`discharge`, else null), its wet squares `wet`, and the layer `look` draws. The ponds
   * lie in rows under the planes, two a texel, each its level and its fetch (land.wgsl `pond_texel`; a level no body
   * holds far below any ground). */
  set(w: World, rows: Float32Array, discharge: Uint16Array | null, wet: Wet, look: MapLook): void {
    if (discharge) { this.discharge = discharge; this.flowsDue = true; }
    const { w: W, h: H } = w.meta, n = W * H, { row, kind_row, place_row, dead_row } = this.tiles.meta, kinds = rows[rows.length - 1];
    const ponds = rows.subarray(1 + (row + kinds * kind_row + place_row + dead_row) * n, rows.length - 1), count = ponds.length / 2;
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
    // The soil's grain the laws read, each cell's (`World::soil`), over its samples' as the rock draws them: what the
    // page adds to the samples' grain (land.wgsl `GRAIN_SHIFT`), so the ground drawn has the world's soil on its mean
    // and the samples' pattern inside a square.
    const coarse = w.index.has('coarse') ? decode(w, 0, 'coarse') : new Float32Array(n), fine = w.index.has('fine') ? decode(w, 0, 'fine') : new Float32Array(n);
    const samplesGrain = this.grainShift(W, H, coarse, fine);
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
      put(6, i, [coarse[i] - samplesGrain.coarse[i], fine[i] - samplesGrain.fine[i], 0, 0]);
      data[4 * n * Math.floor(SATURATED_AT / 4) + 4 * i + SATURATED_AT % 4] = saturated[i];
    }
    blurSea(data, Array.from(sea, (s, i) => (s > 0 && data[4 * i] < this.sea - OPEN_SEA_M ? 1 : 0)), W, H);
    // The sea's squares are marked by their air's fall with height as -(it + 1) (land.wgsl `state_on`): only a point
    // with one of them about it reads the land's own numbers.
    for (let i = 0; i < n; i++) if (wet.sea[i]) data[4 * i + 2] = -(data[4 * i + 2] + 1);
    this.data = data;
    this.due = true;
  }

  /** Of each square of a map `W` x `H`, the mean of its samples' shares of coarse and fine grain as the rock draws
   * them; before the rock came, the cells' own (`coarse`, `fine`: no shift). */
  private grainShift(W: number, H: number, coarse: Float32Array, fine: Float32Array): { coarse: Float32Array; fine: Float32Array } {
    const r = this.tiles.rock, s = this.samples;
    if (!r) return { coarse, fine };
    if (this.grainOf?.version !== this.tiles.rockVersion) {
      const per = s.across / W, plane = 2 * 4 * s.across * s.down, out = { version: this.tiles.rockVersion, coarse: new Float32Array(W * H), fine: new Float32Array(W * H) };
      for (let y = 0; y < s.down; y++) for (let x = 0; x < s.across; x++) {
        const i = Math.floor(y / per) * W + Math.floor(x / per), t = plane + 4 * (y * s.across + x);
        out.coarse[i] += unhalf(r[t]) / (per * per);
        out.fine[i] += unhalf(r[t + 1]) / (per * per);
      }
      this.grainOf = out;
    }
    return this.grainOf;
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
    for (let r = y0; r <= y1; r++) for (let c = x0; c <= x1; c++) if (unhalf(this.discharge[2 * (r * s.across + c)]) >= least) return true;
    return false;
  }

  /** The bindings of lib/land.wgsl on `gpu` (its uniforms, the tiles and the table), or null before the first table. */
  lawBinds(gpu: Gpu): Draw['binds'] | null {
    if (!this.data) return null;
    const m = this.tiles.map;
    if (this.tex && this.tex.h !== this.rows) { gpu.drop(this.tex); this.tex = null; }
    this.tex ??= gpu.texture('land', m.w, this.rows, 'rgba32float');
    if (this.due) { gpu.write(this.tex, this.data); this.due = false; }
    const s = this.samples;
    this.flows ??= gpu.texture('land', s.across, s.down, 'rgba16float');
    if (this.flowsDue) { gpu.write(this.flows, this.flowsOf()); this.flowsDue = false; }
    this.rock ??= gpu.texture('land', s.across, ROCK_PLANES * s.down, 'rgba16float');
    if (this.rockAt !== this.tiles.rockVersion) {
      const r = this.tiles.rock;
      gpu.write(this.rock, r ?? Uint16Array.from({ length: 4 * ROCK_PLANES * s.across * s.down }, (_, i) => (i % 4 || i >= 4 * s.across * s.down ? 0 : half(NO_ROCK_YET_M))));
      this.rockAt = this.tiles.rockVersion;
    }
    const t = this.tiles.textures(gpu), l = this.laws, tm = this.tiles.meta;
    const u = new Uniforms(LAND_VEC4S).vec4(m.cell_m, m.w, m.h, m.square_depth).vec4(tm.size, tm.gutter, this.tiles.depth, tm.step_most)
      .vec4(t.now, TILE_FADE_MS, this.melt, l.line).vec4(l.soil_m, l.wet_depth, l.rooted_from, 0).vec4(l.channel_width, l.channel_width_exp, l.channel_depth, l.channel_depth_exp)
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
