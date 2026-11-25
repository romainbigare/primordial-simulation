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
const FIRE_VEC4S = 5;

/** A burn of god mode plays on the map outward from its strike, over the squares its front reached (fire.wgsl
 * `burn_rank`): its reach grows at `FRONT_M_S` m a second of the page's time, taking no less than `FRONT_LEAST_MS` and no
 * more than `FRONT_MOST_MS`, to the radius of a disc as large as the land it burnt. Its edge is ragged over
 * `FRONT_EDGE` of that radius (and no less than `FRONT_EDGE_M`). */
const FRONT_M_S = 30, FRONT_LEAST_MS = 5000, FRONT_MOST_MS = 15000, FRONT_EDGE = 0.12, FRONT_EDGE_M = 12;
/** Of a square a burn plays on: the share burnt (fire.wgsl: its level is 0, where the front's distance past the point
 * is none), the share burning while it plays (a band behind its edge), and its flames' length where the core keeps
 * none, m. */
const FRONT_SHARE = 0.5, FRONT_BURNING = 0.35, FRONT_FLAME_M = 1.5;
/** Frames a second while a burn plays. */
const FRONT_FPS = 20;

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
  /** A burn of god mode played on the map while the world stands still: the squares its front reached (1), its strike
   * and the radius its reach grows to, m, the metres a unit of its patches' number takes, when it began to play and was
   * last drawn (performance.now, ms), how long it plays (ms), and whether it has played out (its scar stays shown till
   * the world steps, `still`); null: none. */
  private front: { reached: Uint8Array; strike: [number, number]; radius: number; edge: number; begun: number; drawn: number; ms: number; over: boolean } | null = null;
  /** Whether a burn is about to play (`hold`): the frame after its stroke came, but where it burns is still being found,
   * so the planes shown stay those before it (else its squares would show burnt whole at once). */
  private held = false;

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
   * ash, the gas sent to the air and the wind east and south; its third the cured leaves standing, the share of the land saturated and whether a burn of god mode plays on it
   * (`play`). */
  put(w: number, h: number, cellM: number, p: FirePlanes | null, never: number): void {
    this.last = [w, h, cellM, p, never];
    if (this.held) return;
    const reached = this.front && p && this.front.reached.length === w * h ? this.front.reached : null;
    if (reached) p = frontAt(p!, reached, !this.front!.over);
    this.w = w; this.h = h; this.cellM = cellM; this.never = never;
    // (Saturated ground does not burn: a scar lies on the dry land alone.)
    const n = w * h, scar = (i: number) => (!p ? 0 : reached?.[i] ? FRONT_SHARE : Math.min(p.age[i] < never ? 1 : p.burnt[i], 1 - p.wet[i]));
    this.flames = p ? smokeBox(p, w, h, cellM) : null;
    if (!p || !(this.flames || p.ash.some((a, i) => a > 0 || p.cured[i] > 0 || scar(i) > 0))) { this.data = null; return; }
    const data = new Uint16Array(4 * FIRE_PLANES * n);
    for (let i = 0; i < n; i++) {
      const s = scar(i), f = p.flame[i];
      data.set([s, s * Math.min(p.age[i], never), f, f * p.length[i]].map(half), 4 * i);
      data.set([p.ash[i], p.air[i], p.east[i], -p.north[i]].map(half), 4 * (n + i));
      data.set([p.cured[i], p.wet[i], reached?.[i] ?? 0, 0].map(half), 4 * (2 * n + i));
    }
    this.data = data;
    this.due = true;
    this.smokeDue = this.flames !== null;
  }

  /** Play the burn whose front reached the squares `reached` (1) from its strike `strike` (squares), which burnt `m2`
   * of land: it burns outward from the strike to a disc of that area, over the squares it reached. */
  play(reached: Uint8Array, strike: [number, number], m2: number): void {
    const radius = Math.sqrt(m2 / Math.PI), cellM = this.last[2];
    const ms = Math.min(Math.max((radius / FRONT_M_S) * 1000, FRONT_LEAST_MS), FRONT_MOST_MS);
    this.front = { reached, strike: [strike[0] * cellM, strike[1] * cellM], radius, edge: Math.max(FRONT_EDGE * radius, FRONT_EDGE_M), begun: performance.now(), drawn: 0, ms, over: false };
    this.held = false;
    this.put(...this.last);
  }

  /** A burn is about to play: the planes shown stay as they are till it does (`play`), or the world steps (`still`). */
  hold(): void { this.held = true; }

  /** The edge of the burn playing at time `now` (ms): its strike and its reach, m, and the metres its edge is ragged
   * over; null where none is burning. */
  edgeAt(now: number): { strike: [number, number]; reach: number; edge: number } | null {
    const f = this.front;
    if (!f || f.over) return null;
    return { strike: f.strike, reach: this.frontAt(now)[2], edge: f.edge };
  }

  /** The world stepped: the fire shown is the core's own again. */
  still(): void { this.front = null; this.held = false; }

  /** While a burn plays, whether the map is to be drawn anew at time `now` (performance.now, ms): at most `FRONT_FPS`
   * times a second, its front further on; once it played out, its planes are put without its flames. */
  step(now: number): boolean {
    const f = this.front;
    if (!f || f.over || now - f.drawn < 1000 / FRONT_FPS) return false;
    f.drawn = now;
    if (now - f.begun > f.ms) { f.over = true; this.put(...this.last); }
    return true;
  }

  /** The burn playing at time `now` (ms), as fire.wgsl reads it (`Fire.front`): its strike, its reach, and the metres a
   * unit of its patches' number takes; none (0) where none plays. Its reach grows evenly, then stays. */
  private frontAt(now: number): number[] {
    const f = this.front;
    if (!f) return [0, 0, 0, 0];
    return [...f.strike, f.radius * Math.min(Math.max((now - f.begun) / f.ms, 0), 1), f.edge];
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
      .vec4((now / 1000) % TIME_WRAP_S, this.never).vec4(...this.frontAt(now));
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

/** The planes `p` as a burn of god mode plays on them over the squares `reached` (1): those burn fresh, flaming while it
 * plays (`burning`), their flames at least `FRONT_FLAME_M` long; no other square flames (the core keeps the flames of
 * the month last stepped while the world stands still, which burn no more), and only those send their gas to the air. */
function frontAt(p: FirePlanes, reached: Uint8Array, burning: boolean): FirePlanes {
  const q = { ...p, age: p.age.slice(), flame: new Float32Array(p.flame.length), length: p.length.slice(), air: new Float32Array(p.air.length) };
  reached.forEach((r, i) => {
    if (!r) return;
    q.age[i] = 0; q.flame[i] = burning ? FRONT_BURNING : 0; q.length[i] = Math.max(p.length[i], FRONT_FLAME_M); q.air[i] = p.air[i];
  });
  return q;
}
