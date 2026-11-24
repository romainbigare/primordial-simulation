// Fire on the live map: what the core's layers say fire did on each square (tape/src/layers.rs), sent to the GPU as a
// texture of three planes (web/shaders/lib/fire.wgsl): the burn scars and the cured grass on the ground (land.wgsl,
// lib/burn.wgsl), the flames and the smoke over the plants (flames.wgsl, over the squares that burn and the smoke's reach).
// Where no fire is known the ground reads none of it (`groundBinds`).
import { World, decode } from './data';
import { Draw, Gpu, Texture, Uniforms, notMade } from './gpu';
import { half } from './canopy';
import { groundDetail } from './detail';
import type { MapLook } from './field-window';

/** What fire did on each square, a number a square (the core's layers, `LAYERS`). */
export interface FirePlanes {
  burnt: Float32Array; age: Float32Array; flame: Float32Array; length: Float32Array; ash: Float32Array; air: Float32Array;
  east: Float32Array; north: Float32Array; cured: Float32Array; wet: Float32Array;
}
/** The core's layers of fire, by the planes' names: the share of the land burnt in the last 12 months, the months since
 * the square last burnt (at most the layer's top: never), the share burning this month, its flames' mean length (m),
 * the ash on the ground and the gas the flames sent to the air this month (kg/m²), the low wind east and north (m/s),
 * the cured leaves standing (kg/m²), the share of the land saturated. */
const LAYERS: Record<keyof FirePlanes, string> = {
  burnt: 'burnt', age: 'burnt_age', flame: 'flame', length: 'flame_len', ash: 'ash', air: 'flame_air',
  east: 'wind_u', north: 'wind_v', cured: 'cured', wet: 'wet_share',
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
const FIRE_PLANES = 3;
/** The format of the smoke's texture: four bytes (lib/smoke.wgsl `smoke_packed`). */
const SMOKE_FORMAT = 'rgba8unorm';
/** Frames a second while a fire in view burns and the world plays. */
export const FIRE_FPS = 8;
/** The time the flames are drawn at, s, wraps round after this many seconds (a float keeps its steps fine). */
const TIME_WRAP_S = 600;
/** Of the uniforms of fire.wgsl (`Fire`), four-wide members. */
const FIRE_VEC4S = 4;

/** A fire front of god mode is played on the map over this time for the longest the core lets a front run (its
 * `front_life`); each square burns with flames over `FRONT_GLOW_MS` from when the front reaches it. */
const FRONT_MS = 2500, FRONT_GLOW_MS = 1200;
/** Frames a second while a front plays. */
const FRONT_FPS = 20;
/** The flames' length shown on a square the front reaches where the core keeps none, m. */
const FRONT_FLAME_M = 1.5;

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
  /** The planes last put, as `put` took them. */
  private last: [number, number, number, FirePlanes | null, number] = [0, 0, 1, null, 0];
  /** A fire front played on the map while the world stands still: when it reaches each square, s after its strike
   * (NaN: it does not), the longest a front runs, s, when it began to play and was last put (performance.now, ms), and
   * whether it has played out (its scars stay shown till the world steps, `still`); null: none. */
  private front: { at: Float32Array; life: number; begun: number; put: number; over: boolean } | null = null;

  /** What fire did on world `w`'s squares, from its layers (none on a world whose tape has no fire). */
  set(w: World | null): void {
    const has = w !== null && Object.values(LAYERS).every(name => w.index.has(name));
    if (!w || !has) { this.put(0, 0, 1, null, 0); return; }
    const p = Object.fromEntries(Object.entries(LAYERS).map(([k, name]) => [k, decode(w, 0, name)])) as unknown as FirePlanes;
    this.put(w.meta.w, w.meta.h, w.meta.cell_m, p, w.meta.layers[w.index.get(LAYERS.age)!].max);
  }

  /** What fire did on a map of `w` x `h` squares of `cellM` m, `p` (null: nothing), where `never` months since a burn
   * means none. A square burns whole, so its land is under a scar while it is younger than that, else its share burnt
   * this year. The texture's first plane holds that share, the months since the burn times it, the burning share and
   * the flames' length times that share (so the blend between squares gives each its share's mean); its second the
   * ash, the gas sent to the air and the wind east and south; its third the cured leaves standing and the share of the land saturated. */
  put(w: number, h: number, cellM: number, p: FirePlanes | null, never: number): void {
    this.last = [w, h, cellM, p, never];
    const f = this.front;
    if (f && p && f.at.length === w * h) { f.put = performance.now(); p = frontAt(p, f.at, never, ((f.put - f.begun) / FRONT_MS) * f.life, (FRONT_GLOW_MS / FRONT_MS) * f.life); }
    this.w = w; this.h = h; this.cellM = cellM; this.never = never;
    // (Saturated ground does not burn: a scar lies on the dry land alone.)
    const n = w * h, scar = (i: number) => (!p ? 0 : Math.min(p.age[i] < never ? 1 : p.burnt[i], 1 - p.wet[i]));
    this.flames = p ? smokeBox(p, w, h, cellM) : null;
    if (!p || !(this.flames || p.ash.some((a, i) => a > 0 || p.cured[i] > 0 || scar(i) > 0))) { this.data = null; return; }
    const data = new Uint16Array(4 * FIRE_PLANES * n);
    for (let i = 0; i < n; i++) {
      const s = scar(i), f = p.flame[i];
      data.set([s, s * Math.min(p.age[i], never), f, f * p.length[i]].map(half), 4 * i);
      data.set([p.ash[i], p.air[i], p.east[i], -p.north[i]].map(half), 4 * (n + i));
      data.set([p.cured[i], p.wet[i], 0, 0].map(half), 4 * (2 * n + i));
    }
    this.data = data;
    this.due = true;
    this.smokeDue = this.flames !== null;
  }

  /** Play the fire front that reaches each square `at` s after its strike (NaN: none): each square burns as it comes,
   * the longest a front runs (`life`, s: the core's `front_life`) over `FRONT_MS`. */
  play(at: Float32Array, life: number): void {
    this.front = { at, life, begun: performance.now(), put: 0, over: false };
    this.put(...this.last);
  }

  /** The world stepped: the fire shown is the core's own again. */
  still(): void { this.front = null; }

  /** While a front plays, its planes at time `now` (performance.now, ms), at most `FRONT_FPS` times a second: true when
   * they were put anew (the map is to be drawn anew). */
  step(now: number): boolean {
    const f = this.front;
    if (!f || f.over || now - f.put < 1000 / FRONT_FPS) return false;
    f.over = now - f.begun > FRONT_MS + FRONT_GLOW_MS;
    this.put(...this.last);
    return true;
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

/** The planes `p` as a front that reaches each square `at` s after its strike stands at `t` s, each square burning
 * over `glow` s from when the front reaches it. The flames shown are
 * the front's alone, and their smoke: the core keeps the flames of the month last stepped while the world stands still,
 * which burn no more. A square the front has not reached shows no fire; one it reached less than `glow` ago burns whole,
 * its flames along the edge of the burnt squares (flames.wgsl); one it passed shows the scar the core says. */
function frontAt(p: FirePlanes, at: Float32Array, never: number, t: number, glow: number): FirePlanes {
  const q = { ...p, burnt: p.burnt.slice(), age: p.age.slice(), flame: new Float32Array(p.flame.length), length: p.length.slice(), ash: p.ash.slice(), air: new Float32Array(p.air.length) };
  at.forEach((a, i) => {
    if (Number.isNaN(a) || t >= a + glow) return;
    if (t < a) { q.burnt[i] = 0; q.age[i] = never; q.ash[i] = 0; return; }
    q.flame[i] = 1; q.length[i] = Math.max(p.length[i], FRONT_FLAME_M); q.air[i] = p.air[i];
  });
  return q;
}
