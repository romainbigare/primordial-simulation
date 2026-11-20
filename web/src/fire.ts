// Fire on the live map: what the core's layers say fire did on each square (tape/src/layers.rs), sent to the GPU as a
// texture of two planes (web/shaders/lib/fire.wgsl), and the boxes of squares the map draws it in: the burn scars in
// the ground's program made with them (burnt.wgsl, over the squares that hold one; land.wgsl elsewhere), the flames and
// the smoke over the plants (flames.wgsl, over the squares that burn and the smoke's reach). Where nothing burns or has
// burnt, nothing of it is drawn and no pixel pays for it.
import { World, decode } from './data';
import { Draw, Gpu, Texture, Uniforms } from './gpu';
import { half } from './canopy';
import { groundDetail } from './detail';
import type { MapLook } from './field-window';

/** What fire did on each square, a number a square (the core's layers, `LAYERS`). */
export interface FirePlanes { burnt: Float32Array; age: Float32Array; flame: Float32Array; length: Float32Array; ash: Float32Array; air: Float32Array }
/** The core's layers of fire, by the planes' names: the share of the land burnt in the last 12 months, the mean of
 * its land's months since it last burnt (land never burnt counts as `AGE_MOST`), the share burning this month, its flames' mean length (m), the ash on the ground and the gas the
 * flames sent to the air this month (kg/m²). */
const LAYERS: Record<keyof FirePlanes, string> = { burnt: 'burnt', age: 'burnt_age', flame: 'flame', length: 'flame_len', ash: 'ash', air: 'flame_air' };

/** The most months since a burn the core tells apart (`flame::AGE_MOST`), and the months its burnt share spans. */
const AGE_MOST = 120, YEAR_MONTHS = 12;
/** The months since the burnt share of a square burnt, from the mean `age` of its land's months since a burn, that
 * share `burnt` and the share burning this month `flame` (none old): the rest of the land taken as never burnt,
 * within the year the share spans. The layers do not tell it exactly: where the rest burnt too, earlier, the share
 * reads younger than it is. */
function scarAge(age: number, burnt: number, flame: number): number {
  if (burnt <= 0) return AGE_MOST;
  const b = Math.min(burnt, 1), older = Math.min(Math.max((age - (1 - b) * AGE_MOST) / b, 0), YEAR_MONTHS);
  return older * Math.max(1 - flame / b, 0);
}

/** The tape holds no wind: the smoke drifts with the westerlies of the middle latitudes the page's islands lie in
 * (a unit vector east and south: from the west, a little from the south), at a breeze's speed, m/s. */
const WIND = [0.97, -0.24], WIND_M_S = 5;
/** The smoke reaches this far downwind, m, and spreads this share of the way gone to each side (flames.wgsl
 * `PLUME_M`, `SPREAD`). */
const PLUME_M = 8000, SPREAD = 0.25;
/** A square's burn scar and glow are blended into the squares about it (fire.wgsl `fire_plane`): the boxes reach one
 * square farther. */
const BLEND_SQUARES = 1;
/** Frames a second while a fire in view burns and the world plays. */
export const FIRE_FPS = 8;
/** The time the flames are drawn at, s, wraps round after this many seconds (a float keeps its steps fine). */
const TIME_WRAP_S = 600;
/** Of the uniforms of fire.wgsl (`Fire`), four-wide members. */
const FIRE_VEC4S = 4;

/** A box of squares: x0, y0, x1, y1 (x1, y1 past its last square). */
export type Box = [number, number, number, number];

export class Fire {
  private data: Uint16Array | null = null;
  private tex: Texture | null = null;
  private due = false;
  private w = 0;
  private h = 0;
  private cellM = 1;
  /** The squares the burn scars lie in, and those the flames and their smoke reach (null: none). */
  scars: Box | null = null;
  flames: Box | null = null;

  /** What fire did on world `w`'s squares, from its layers (none on a world whose tape has no fire). */
  set(w: World | null): void {
    const has = w !== null && Object.values(LAYERS).every(name => w.index.has(name));
    if (!w || !has) { this.put(0, 0, 1, null); return; }
    const p = Object.fromEntries(Object.entries(LAYERS).map(([k, name]) => [k, decode(w, 0, name)])) as unknown as FirePlanes;
    this.put(w.meta.w, w.meta.h, w.meta.cell_m, p);
  }

  /** What fire did on a map of `w` x `h` squares of `cellM` m, `p` (null: nothing). The texture's first plane holds the
   * burnt share, the months since a burn times that share, the burning share and the flames' length times that share
   * (so the blend between squares gives each its share's mean), its second the ash and the gas sent to the air. */
  put(w: number, h: number, cellM: number, p: FirePlanes | null): void {
    this.w = w; this.h = h; this.cellM = cellM;
    const n = w * h, burnt = (i: number) => (p ? p.burnt[i] : 0), burning = (i: number) => (p ? p.flame[i] : 0);
    this.scars = p ? boxOf(w, h, i => burnt(i) > 0 || p.ash[i] > 0, BLEND_SQUARES) : null;
    const lit = p ? boxOf(w, h, i => burning(i) > 0 || p.air[i] > 0, BLEND_SQUARES) : null;
    this.flames = lit && smokeBox(lit, w, h, cellM);
    if (!p || (!this.scars && !this.flames)) { this.data = null; return; }
    const data = new Uint16Array(8 * n);
    for (let i = 0; i < n; i++) {
      const b = burnt(i), f = burning(i);
      data.set([b, b * scarAge(p.age[i], b, f), f, f * p.length[i]].map(half), 4 * i);
      data.set([p.ash[i], p.air[i], 0, 0].map(half), 4 * (n + i));
    }
    this.data = data;
    this.due = true;
  }

  /** The bindings fire.wgsl (and the sprites and photograph of tex.wgsl) read on `gpu` for a draw in view `look` whose pixels lie as `frame` says
   * (field-window.ts `frameOf`), at time `now` (performance.now, ms); null when nothing of fire is known. */
  binds(gpu: Gpu, look: MapLook, frame: number[], now: number): Draw['binds'] | null {
    if (!this.data) return null;
    if (this.tex && (this.tex.w !== this.w || this.tex.h !== 2 * this.h)) { gpu.drop(this.tex); this.tex = null; }
    this.tex ??= gpu.texture(this.w, 2 * this.h, 'rgba16float');
    if (this.due) { gpu.write(this.tex, this.data); this.due = false; }
    const mode = look.mode === 'earth' ? 0 : 1;
    const u = new Uniforms(FIRE_VEC4S).vec4(this.cellM, this.w, this.h, mode).vec4(...frame.slice(0, 4)).vec4(...frame.slice(4))
      .vec4((now / 1000) % TIME_WRAP_S, WIND[0], WIND[1], WIND_M_S);
    return { 20: groundDetail(gpu), 21: 'tile', 25: this.tex, 26: 'smooth', 27: u.buffer };
  }

  stop(gpu: Gpu | null): void {
    if (gpu && this.tex) gpu.drop(this.tex);
    this.tex = null;
    this.due = this.data !== null;
  }
}

/** The box of the squares of a `w` x `h` map where `has` holds, `grow` squares wider each way within the map; null:
 * none. */
function boxOf(w: number, h: number, has: (i: number) => boolean, grow: number): Box | null {
  let x0 = w, y0 = h, x1 = -1, y1 = -1;
  for (let i = 0; i < w * h; i++) {
    if (!has(i)) continue;
    const x = i % w, y = (i / w) | 0;
    x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y);
  }
  if (x1 < 0) return null;
  return [Math.max(x0 - grow, 0), Math.max(y0 - grow, 0), Math.min(x1 + 1 + grow, w), Math.min(y1 + 1 + grow, h)];
}

/** Box `b` grown by the reach of the smoke downwind (`PLUME_M`) and its spread across, within the map. */
function smokeBox(b: Box, w: number, h: number, cellM: number): Box {
  const reach = PLUME_M / cellM, side = SPREAD * reach;
  const dx = WIND[0] * reach, dy = WIND[1] * reach, sx = Math.abs(WIND[1]) * side, sy = Math.abs(WIND[0]) * side;
  return [Math.max(Math.floor(b[0] + Math.min(dx, 0) - sx), 0), Math.max(Math.floor(b[1] + Math.min(dy, 0) - sy), 0),
    Math.min(Math.ceil(b[2] + Math.max(dx, 0) + sx), w), Math.min(Math.ceil(b[3] + Math.max(dy, 0) + sy), h)];
}
