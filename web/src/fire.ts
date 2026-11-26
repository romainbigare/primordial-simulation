// Fire on the live map: what the core's layers say fire did on each square (tape/src/layers.rs), sent to the GPU as a
// texture of four planes (web/shaders/lib/fire.wgsl): the burn scars and the cured grass on the ground (land.wgsl,
// lib/burn.wgsl), the flames and the smoke over the plants (flames.wgsl, over the squares that burn and the smoke's reach),
// and the burns of god mode laid about their strikes (burns.ts). Where no fire is known the ground reads none of it
// (`groundBinds`).
import { World, decode } from './data';
import { AGE_SLACK, BURNS, BURN_VEC4S, Burns } from './burns';
import { Draw, Gpu, Texture, Uniforms, notMade } from './gpu';
import { half } from './canopy';
import { groundDetail } from './detail';
import type { MapLook } from './field-window';

/** What fire did on each square, a number a square (the core's layers, `LAYERS`). */
export interface FirePlanes {
  last: Float32Array; crown: Float32Array; age: Float32Array; flame: Float32Array; length: Float32Array; ash: Float32Array;
  air: Float32Array; east: Float32Array; north: Float32Array; cured: Float32Array; wet: Float32Array;
}
/** The core's layers of fire, by the planes' names: of the square's last burn the share of its land burnt and the share
 * of the soft tissue standing there killed, the months since it last burnt (at most the layer's top: never), the share
 * burning this month, its flames' mean length (m), the ash on the ground and the gas the flames sent to the air this
 * month (kg/m²), the low wind east and north (m/s), the cured leaves standing (kg/m²), the share of the land saturated. */
const LAYERS: Record<keyof FirePlanes, string> = {
  last: 'burnt_last', crown: 'crown_kill', age: 'burnt_age', flame: 'flame', length: 'flame_len', ash: 'ash',
  air: 'flame_air', east: 'wind_u', north: 'wind_v', cured: 'cured', wet: 'wet_share',
};

/** The smoke reaches this far downwind, m, and spreads this share of the way gone to each side (lib/smoke.wgsl
 * `PLUME_M`, smoke.wgsl `SPREAD`). */
const PLUME_M = 8000, SPREAD = 0.5;
/** The box of a plume reaches this many times its spread to each side, so its soft edge is not cut. */
const SPREAD_BOX = 2.5;
/** A square's numbers are blended into the squares about it (fire.wgsl `fire_plane`): the boxes reach one square
 * farther. */
const BLEND_SQUARES = 1;
/** The smoke's texels a square's side (lib/smoke.wgsl `SMOKE_PER_SQUARE`), worked out once a world frame. */
const SMOKE_PER_SQUARE = 8;
/** The texture's planes (fire.wgsl `FIRE_PLANES`). */
const FIRE_PLANES = 4;
/** The format of the smoke's texture: four bytes (lib/smoke.wgsl `smoke_packed`). */
const SMOKE_FORMAT = 'rgba8unorm';
/** Frames a second while a fire in view burns and the world plays. */
export const FIRE_FPS = 8;
/** The time the flames are drawn at, s, wraps round after this many seconds (a float keeps its steps fine). */
const TIME_WRAP_S = 600;
/** Of the uniforms of fire.wgsl (`Fire`), four-wide members: four, then the burns'. */
const FIRE_VEC4S = 4 + BURNS * BURN_VEC4S;
/** A burn's flames are at least this long where the core keeps none, m. */
const BURN_FLAME_M = 1.5;
/** Months a year (the core's `Config::months_per_year`). */
const YEAR_MONTHS = 12;

/** A box of squares: x0, y0, x1, y1 (x1, y1 past its last square). */
export type Box = [number, number, number, number];

export class Fire {
  private data: Uint16Array | null = null;
  private tex: Texture | null = null;
  private due = false;
  private w = 0;
  private h = 0;
  private cellM = 1;
  /** The months since a burn that mean none (the layer's top). */
  private never = 0;
  /** The box the flames and their smoke reach (null: none); and whether dead walls stand on any square. */
  flames: Box | null = null;
  /** The texture of the plant cover's draws where nothing of fire is known: one texel of nothing. */
  private blank: Texture | null = null;
  /** The smoke worked out for the frame shown (smoke.wgsl), till then one texel of none, and whether it is still to
   * work out. */
  private smoke: Texture | null = null;
  private noSmoke: Texture | null = null;
  private smokeDue = false;
  /** How many times planes were put (`put`): what is drawn of fire changed. */
  private written = 0;
  /** The planes last put, as `put` took them. */
  private last: [number, number, number, FirePlanes | null, number, number] = [0, 0, 1, null, 0, 0];
  /** The burns of god mode laid about their strikes (burns.ts). */
  private burns = new Burns();
  /** Whether a burn is about to play (`hold`): the frame after its stroke came, but where it burns is still being found,
   * so the planes shown stay those before it (else its squares would show their shares laid in patches at once); and
   * the planes before it, whose shares burnt this month are not its own. */
  private held = false;
  private before: FirePlanes | null = null;
  /** Told when a burn begins to play (true) and when one has played out or none could (false) (main.ts: the plants it
   * held are let go, and the pictures about the view drawn again). */
  played: (begun: boolean) => void = () => {};

  /** What fire did on world `w`'s squares, from its layers (none on a world whose tape has no fire). */
  set(w: World | null): void {
    const has = w !== null && Object.values(LAYERS).every(name => w.index.has(name));
    if (!w || !has) { this.put(0, 0, 1, null, 0, 0); return; }
    const p = Object.fromEntries(Object.entries(LAYERS).map(([k, name]) => [k, decode(w, 0, name)])) as unknown as FirePlanes;
    // (A frame's day counts from the world's start.)
    const month = Math.floor((w.meta.frames[0].day * YEAR_MONTHS) / w.meta.days_per_year);
    this.put(w.meta.w, w.meta.h, w.meta.cell_m, p, w.meta.layers[w.index.get(LAYERS.age)!].max, month);
  }

  /** What fire did on a map of `w` x `h` squares of `cellM` m in the world's month `month`, `p` (null: nothing), where
   * `never` months since a burn means none. A square's land is under a scar as large as the share its last burn took
   * while that is younger than that: laid about the strike of a burn of god mode that lies on it (`Burns.of`), else in
   * patches over the square. The texture's first plane holds that share, the months since the burn times it, the share
   * burning this month in patches and its flames' length times that share (so the blend between squares gives each its
   * share's mean); its second the ash, the gas sent to the air and the wind east and south; its third the cured leaves
   * standing, the share of the land saturated, the share of the soft tissue standing on the scar its burn killed times
   * the scar's share, and the scar's share in patches (none where its last burn is one of god mode); its fourth, for
   * each burn of god mode by its place among them, how far its mean growth reaches when it has taken the square's share
   * (m; none: it does not lie on the square). */
  put(w: number, h: number, cellM: number, p: FirePlanes | null, never: number, month: number): void {
    this.last = [w, h, cellM, p, never, month];
    if (this.held) return;
    this.written++;
    this.w = w; this.h = h; this.cellM = cellM; this.never = never;
    if (p) this.burns.keep(p.age, month, never);
    // (Saturated ground does not burn: a scar lies on the dry land alone.)
    const n = w * h, scar = (i: number) => (!p || p.age[i] >= never ? 0 : Math.min(p.last[i], 1 - p.wet[i]));
    this.flames = p ? smokeBox(p, w, h, cellM) : null;
    if (!p || !(this.flames || p.ash.some((a, i) => a > 0 || p.cured[i] > 0 || scar(i) > 0))) { this.data = null; return; }
    const data = new Uint16Array(4 * FIRE_PLANES * n), limits = new Array<number>(BURNS).fill(0);
    const put = (at: number, a: number, b: number, c: number, d: number) => { data[at] = half(a); data[at + 1] = half(b); data[at + 2] = half(c); data[at + 3] = half(d); };
    for (let i = 0; i < n; i++) {
      const s = scar(i), last = this.burns.of(i, p.age, month, limits);
      // (A burn of god mode draws its own flames: the core's of its square, the same burn, are not drawn again.)
      const f = last ? Math.max(p.flame[i] - p.last[i], 0) : p.flame[i];
      put(4 * i, s, s * Math.min(p.age[i], never), f, f * p.length[i]);
      put(4 * (n + i), p.ash[i], p.air[i], p.east[i], -p.north[i]);
      put(4 * (2 * n + i), p.cured[i], p.wet[i], s * p.crown[i], last ? 0 : s);
      for (let b = 0; b < BURNS; b++) data[4 * (3 * n + i) + b] = half(limits[b]);
    }
    this.data = data;
    this.due = true;
    this.smokeDue = this.flames !== null;
  }

  /** Play the burn that took at `seeds` (m; none: at its strike `strike`, squares) and burnt the squares `reached` (1):
   * it grows from where it took in the low wind there over the share of each square's dry land the core says it burnt
   * (less what burnt there this month before it), killing what the core says. */
  play(reached: Uint8Array, strike: [number, number], seeds: [number, number][]): void {
    const [w, h, cellM, p, , month] = this.last, was = this.before;
    this.held = false; this.before = null;
    if (p && reached.length === p.last.length) {
      const shares = new Map<number, number>();
      let [all, crown, length] = [0, 0, 0];
      reached.forEach((r, i) => {
        if (!r) return;
        const own = Math.max(p.last[i] - (was && was.age[i] <= AGE_SLACK ? was.last[i] : 0), 0);
        shares.set(i, own / Math.max(1 - p.wet[i], 1e-6));
        [all, crown, length] = [all + own, crown + own * p.crown[i], length + own * p.length[i]];
      });
      const sq = Math.min(Math.max(Math.floor(strike[1]), 0), h - 1) * w + Math.min(Math.max(Math.floor(strike[0]), 0), w - 1);
      const damage = { crown: all > 0 ? crown / all : p.crown[sq], length: Math.max(all > 0 ? length / all : p.length[sq], BURN_FLAME_M) };
      const at: [number, number][] = seeds.length ? seeds : [[strike[0] * cellM, strike[1] * cellM]];
      this.burns.add(at, [p.east[sq], p.north[sq]], shares, damage, month, w, cellM);
    }
    this.put(...this.last);
    this.played(this.burns.playing());
  }

  /** A burn is about to play: the planes shown stay as they are till it does (`play`), or the world steps (`still`). */
  hold(): void { this.held = true; this.before ??= this.last[3]; }

  /** Whether a burn is about to play or plays now. */
  playing(): boolean { return this.held || this.burns.playing(); }

  /** What the plants' marks read of fire (`planeBinds`) at time `now` (performance.now, ms), as text, but for the time
   * itself (which only the flames read): the planes put and the burns of god mode as they stand. */
  contents(now: number): string { return [this.written, ...this.burns.uniforms(now, this.last[5])].join(); }

  /** The world stepped: the burns have burnt out, and the fire shown is the core's own (each burn kept still lies
   * about its strike). */
  still(): void { this.burns.still(); this.held = false; this.before = null; }

  /** While a burn plays, whether the map is to be drawn anew at time `now` (performance.now, ms): its front further on;
   * once one played out, `played` is told. */
  step(now: number): boolean {
    const { redraw, ended } = this.burns.step(now);
    if (ended) this.played(false);
    return redraw;
  }

  /** Work out the smoke of this frame on `gpu` once, before the map's pass (smoke.wgsl), once its program is made
   * (till then, added to `waiting`, no smoke is drawn). */
  prepare(gpu: Gpu, waiting: string[]): void {
    if (!this.smokeDue || !this.data) return;
    const missing = notMade(gpu, ['smoke']);
    if (missing.length) { waiting.push(...missing); return; }
    const w = this.w * SMOKE_PER_SQUARE, h = this.h * SMOKE_PER_SQUARE;
    if (this.smoke?.w !== w || this.smoke.h !== h) {
      if (this.smoke) gpu.drop(this.smoke);
      this.smoke = gpu.texture('fire', w, h, SMOKE_FORMAT, 1, 1, true);
    }
    const p = gpu.pass(this.smoke);
    p.draw('smoke', { rect: [0, 0, w, h], binds: this.binds(gpu, [0, 0, 0, 0, 0, 0, 0, 0], 0)! }, false);
    p.end();
    this.smokeDue = false;
  }

  /** The bindings fire.wgsl (and the sprites and photograph of tex.wgsl) read on `gpu` for a draw of the Earth view
   * whose pixels lie as `frame` says (field-window.ts `frameOf`), at time `now` (performance.now, ms); null when nothing of
   * fire is known. */
  binds(gpu: Gpu, frame: number[], now: number): Draw['binds'] | null {
    if (!this.data) return null;
    if (this.tex && (this.tex.w !== this.w || this.tex.h !== FIRE_PLANES * this.h)) { gpu.drop(this.tex); this.tex = null; }
    this.tex ??= gpu.texture('fire', this.w, FIRE_PLANES * this.h, 'rgba16float');
    if (this.due) { gpu.write(this.tex, this.data); this.due = false; }
    const u = new Uniforms(FIRE_VEC4S).vec4(this.cellM, this.w, this.h).vec4(...frame.slice(0, 4)).vec4(...frame.slice(4))
      .vec4((now / 1000) % TIME_WRAP_S, this.never);
    const burns = this.burns.uniforms(now, this.last[5]);
    for (let k = 0; k < burns.length; k += 4) u.vec4(...burns.slice(k, k + 4));
    if (!this.noSmoke) { this.noSmoke = gpu.texture('fire', 1, 1, SMOKE_FORMAT); gpu.write(this.noSmoke, new Uint8Array(4)); }
    return { 20: groundDetail(gpu), 21: 'tile', 25: this.tex, 26: 'smooth', 27: u.buffer, 28: this.smoke && !this.smokeDue ? this.smoke : this.noSmoke };
  }

  /** The bindings fire.wgsl reads on `gpu` in the ground's draws (land.wgsl, lib/burn.wgsl), as `binds`; where no fire
   * is known, or in a layer's view, a texture of one texel and squares of side 0: the ground reads none of it. */
  groundBinds(gpu: Gpu, look: MapLook, frame: number[], now: number): Draw['binds'] {
    return this.planeBinds(gpu, look.mode === 'layer' ? null : frame, now);
  }

  /** The bindings fire.wgsl reads on `gpu` in a draw whose pixels lie as `frame` says (field-window.ts `frameOf`; a
   * draw that reads only `fire_at`, the plants' marks, gives none it reads), as `groundBinds`; null: none of fire. */
  planeBinds(gpu: Gpu, frame: number[] | null, now: number): Draw['binds'] {
    const b = frame ? this.binds(gpu, frame, now) : null;
    if (b) return { 25: b[25], 26: b[26], 27: b[27] };
    return { 25: this.none(gpu), 26: 'smooth', 27: new Uniforms(FIRE_VEC4S).vec4(0, 1, 1).buffer };
  }

  /** A texture of one texel of nothing, where no fire is known. */
  private none(gpu: Gpu): Texture {
    if (!this.blank) { this.blank = gpu.texture('fire', 1, 1, 'rgba16float'); gpu.write(this.blank, new Uint16Array(4)); }
    return this.blank;
  }

  stop(gpu: Gpu | null): void {
    if (gpu) for (const t of [this.tex, this.blank, this.smoke, this.noSmoke]) if (t) gpu.drop(t);
    this.tex = this.blank = this.smoke = this.noSmoke = null;
    this.due = this.data !== null;
    this.smokeDue = this.flames !== null && this.data !== null;
  }
}

/** The box of the squares that burn or send gas to the air on a `w` x `h` map of `cellM` m squares, each grown by the
 * reach of its smoke down its own wind (`PLUME_M`) and its spread across, within the map; null: none. */
function smokeBox(p: FirePlanes, w: number, h: number, cellM: number): Box | null {
  const reach = PLUME_M / cellM, side = SPREAD * reach;
  let b: Box | null = null;
  for (let i = 0; i < w * h; i++) {
    if (!(p.flame[i] > 0 || p.air[i] > 0)) continue;
    const x = i % w, y = (i / w) | 0, speed = Math.hypot(p.east[i], p.north[i]) || 1;
    const ex = (p.east[i] / speed) * reach, ey = (-p.north[i] / speed) * reach;
    const sx = Math.abs(ey / reach) * SPREAD_BOX * side + BLEND_SQUARES, sy = Math.abs(ex / reach) * SPREAD_BOX * side + BLEND_SQUARES;
    const own: Box = [x + Math.min(ex, 0) - sx, y + Math.min(ey, 0) - sy, x + 1 + Math.max(ex, 0) + sx, y + 1 + Math.max(ey, 0) + sy];
    b = b ? [Math.min(b[0], own[0]), Math.min(b[1], own[1]), Math.max(b[2], own[2]), Math.max(b[3], own[3])] : own;
  }
  return b && [Math.max(Math.floor(b[0]), 0), Math.max(Math.floor(b[1]), 0), Math.min(Math.ceil(b[2]), w), Math.min(Math.ceil(b[3]), h)];
}
