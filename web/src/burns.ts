// The burns of god mode on the live map, kept while the page runs (fire.wgsl `burn_at`). The core burns a stroke's fire
// as a share of each cell it reached, with nothing of where in the cell (a small burn is spread over its patch, the core's
// `cover::hit`), so the page lays each square's share about where the fire took as it would have grown: the brush's heat
// lights the remains at some places under it first (god-draw.ts `litSites`), and from each the fire grows as an ellipse
// downwind, its length over its breadth by the wind at mid-flame (Anderson 1983), its head as far ahead of where it
// took as its length and breadth give (Alexander 1985). Each square's burnt land is the part of it those ellipses reach
// first, as much as its share (`limits`), so the burn's shape is the core's shares, before and after the world plays,
// while its front sweeps them in the order the fire reached them. Up to `BURNS` burns lie on the map, each with its own
// share of a square, age and damage.

/** The most burns the map draws at once (fire.wgsl `BURNS`): the newest. */
export const BURNS = 4;
/** The most places a burn grows from (fire.wgsl `SEEDS`). */
export const SEEDS = 8;
/** Of fire.wgsl's uniforms (`Fire.burns`), the four-wide members a burn takes: its shape, its play and damage, the
 * count of its places, and its places two a member. */
export const BURN_VEC4S = 3 + SEEDS / 2;
/** A front that has passed: its reach, m, past any square's (fire.wgsl `burn_at`). */
const PASSED_M = 1e9;
/** The wind at mid-flame is this share of the low wind (an unsheltered fuel bed's wind adjustment factor, about 0.4:
 * Andrews 2012). */
const MIDFLAME = 0.4;
/** Metres a second in a mile an hour (Anderson's fit is in miles an hour). */
const M_S_PER_MPH = 0.44704;
/** Anderson's (1983) length over breadth of a wind-driven fire: 0.936 e^(0.2566 U) + 0.461 e^(-0.1548 U) - 0.397, U
 * in mph at mid-flame, held to `LB_MOST` (his fits reach about 8). */
const LB_A = 0.936, LB_B = 0.2566, LB_C = 0.461, LB_D = 0.1548, LB_E = 0.397, LB_MOST = 8;
/** A square's burnt land is found from this many points along each of its sides. */
const SIDE_POINTS = 32;
/** A burn plays at `FRONT_M_S` m of its mean growth a second of the page's time, taking no less than `FRONT_LEAST_MS`
 * and no more than `FRONT_MOST_MS`. */
const FRONT_M_S = 30, FRONT_LEAST_MS = 4000, FRONT_MOST_MS = 15000;
/** Frames a second while a burn plays. */
const FRONT_FPS = 20;
/** A square's months since it last burnt are told to within this many months (the tape's levels are a 255th of the
 * layer's top). */
export const AGE_SLACK = 0.5;

/** How a burn grows: the way downwind (east, south; a unit), how far its head runs ahead of its mean growth, its breadth
 * over its length (`shape`), and the places it grows from, m. */
interface Growth { way: [number, number]; ahead: number; breadth: number; seeds: [number, number][] }

/** A burn: how it grows; each square it burnt with how far its mean growth reaches when it has taken that square's
 * share (`limits`, m); the farthest of those; the share of the soft tissue on its land it killed and its flames' length,
 * m (the core's, of its squares); the world's month the core burnt it in; and its play: when it began and was last drawn
 * (performance.now, ms), how long it plays (ms), and whether it played out. */
interface Burn extends Growth {
  squares: Map<number, number>; end: number; crown: number; length: number; month: number;
  begun: number; drawn: number; ms: number; over: boolean;
}

/** The shape of a fire in the low wind (`east`, `north`, m/s): the way downwind, how far its head runs ahead of its
 * mean growth (0: none, a circle about where it took), and its breadth over its length. */
function shape(east: number, north: number): Omit<Growth, 'seeds'> {
  const speed = Math.hypot(east, north), mph = (MIDFLAME * speed) / M_S_PER_MPH;
  const lb = Math.min(Math.max(LB_A * Math.exp(LB_B * mph) + LB_C * Math.exp(-LB_D * mph) - LB_E, 1), LB_MOST);
  // (Alexander 1985: the head's speed over the back's, (LB + √(LB² − 1)) / (LB − √(LB² − 1)).)
  const root = Math.sqrt(lb * lb - 1), hb = (lb + root) / (lb - root);
  return { way: speed > 0 ? [east / speed, -north / speed] : [1, 0], ahead: (hb - 1) / (hb + 1), breadth: 1 / lb };
}

/** How far burn `g`'s mean growth has reached when its edge reaches the point (x, y), m (fire.wgsl `burn_time`): the
 * least over its places of the ellipse of semi-axes t along the wind and `breadth` t across it, its centre `ahead` t
 * downwind of the place. */
function reachAt(g: Growth, x: number, y: number): number {
  const c = g.ahead, k = 1 - c * c;
  let least = Infinity;
  for (const [sx, sy] of g.seeds) {
    const dx = x - sx, dy = y - sy, u = dx * g.way[0] + dy * g.way[1], v = g.way[0] * dy - g.way[1] * dx;
    least = Math.min(least, (-u * c + Math.sqrt(u * u * c * c + k * (u * u + (v * v) / (g.breadth * g.breadth)))) / k);
  }
  return least;
}

/** Of burn `g` on squares of `cellM` m on a map `w` squares across: for each square of `shares` (its share burnt of
 * its dry land), how far the burn's mean growth reaches when it has taken that share of the square, m: the share's
 * quantile of the reach over the square's points. */
function limits(g: Growth, shares: Map<number, number>, w: number, cellM: number): Map<number, number> {
  const out = new Map<number, number>(), step = cellM / SIDE_POINTS, at = new Float64Array(SIDE_POINTS * SIDE_POINTS);
  for (const [i, share] of shares) {
    const x0 = (i % w) * cellM, y0 = Math.floor(i / w) * cellM;
    for (let k = 0; k < at.length; k++) at[k] = reachAt(g, x0 + ((k % SIDE_POINTS) + 0.5) * step, y0 + (Math.floor(k / SIDE_POINTS) + 0.5) * step);
    at.sort();
    out.set(i, at[Math.min(Math.round(Math.min(Math.max(share, 0), 1) * at.length), at.length - 1)]);
  }
  return out;
}

export class Burns {
  /** The burns kept, oldest first, no more than `BURNS`. */
  list: Burn[] = [];

  /** A burn that took at `seeds` (m, at least one) in the low wind (`east`, `north`, m/s) while the world stands at its
   * month `month`, which burnt the share `shares` of each square's dry land, of a map `w` squares across of `cellM` m,
   * killing the share `crown` of the soft tissue standing there with flames `length` m long: it plays from now. (The
   * core burns an edit's fire in the coming month, the core's `flame::now`: its squares' months since they burnt are none
   * till that month has run.) */
  add(seeds: [number, number][], wind: [number, number], shares: Map<number, number>, damage: { crown: number; length: number }, month: number, w: number, cellM: number): void {
    // (More places than it holds: as many taken evenly among them, so they spread as the lit places did.)
    const taken = seeds.length <= SEEDS ? seeds : Array.from({ length: SEEDS }, (_, k) => seeds[Math.floor((k * seeds.length) / SEEDS)]);
    const g = { ...shape(...wind), seeds: taken }, squares = limits(g, shares, w, cellM);
    const end = Math.max(0, ...squares.values()), now = performance.now();
    const ms = Math.min(Math.max((end / FRONT_M_S) * 1000, FRONT_LEAST_MS), FRONT_MOST_MS);
    this.list.push({ ...g, ...damage, squares, end, month: month + 1, begun: now, drawn: 0, ms, over: false });
    this.list.splice(0, this.list.length - BURNS);
  }

  /** Whether a burn plays now. */
  playing(): boolean { return this.list.some(b => !b.over); }

  /** The world stepped: every burn has burnt out. */
  still(): void { this.list.forEach(b => (b.over = true)); }

  /** Whether the map is to be drawn anew at time `now` (performance.now, ms) as a burn plays: at most `FRONT_FPS` times
   * a second; and the burns that played out by now, which then stop. */
  step(now: number): { redraw: boolean; ended: boolean } {
    let redraw = false, ended = false;
    for (const b of this.list.filter(b => !b.over && now - b.drawn >= 1000 / FRONT_FPS)) {
      redraw = true; b.drawn = now;
      if (now - b.begun > b.ms) { b.over = true; ended = true; }
    }
    return { redraw, ended };
  }

  /** The burns kept that still lie on some square of a map whose months since each square last burnt are `age`, in the
   * world's month `month` (`never`: the months that mean none): a square holds a burn while it has burnt no earlier
   * since (an undo takes the burn back), and no burn older than `never` months is kept. */
  keep(age: Float32Array, month: number, never: number): void {
    this.list = this.list.filter(b => month - b.month < never && [...b.squares.keys()].some(i => this.holds(b, i, age, month)));
  }

  /** Of square `i`: into `limits` how far each burn kept that lies on it reaches when it has taken the square's share
   * (m, by the burn's place among them; 0: none lies on it); whether its last burn is one of them. */
  of(i: number, age: Float32Array, month: number, limits: number[]): boolean {
    limits.fill(0);
    let last = false;
    this.list.forEach((b, k) => {
      const limit = b.squares.get(i);
      if (limit === undefined || !this.holds(b, i, age, month)) return;
      // (A limit of none would read as no burn: the least is kept above it.)
      limits[k] = Math.max(limit, 1e-3);
      last ||= Math.abs(age[i] - this.since(b, month)) <= AGE_SLACK;
    });
    return last;
  }

  /** The burns as fire.wgsl reads them (`Fire.burns`) at time `now` (performance.now, ms) in the world's month `month`:
   * of each, the way downwind, its head's lead and its breadth; how far its front has reached (m), the months since it
   * burnt, the share of the soft tissue it killed and its flames' length (m); the count of its places; and its places
   * (m). */
  uniforms(now: number, month: number): number[] {
    const out = new Array<number>(BURNS * BURN_VEC4S * 4).fill(0);
    this.list.forEach((b, k) => {
      const reach = b.over ? PASSED_M : b.end * Math.min(Math.max((now - b.begun) / b.ms, 0), 1);
      const seeds = Array.from({ length: SEEDS }, (_, s) => b.seeds[s] ?? [0, 0]).flat();
      out.splice(k * BURN_VEC4S * 4, BURN_VEC4S * 4, ...b.way, b.ahead, b.breadth, reach, this.since(b, month), b.crown, b.length, b.seeds.length, 0, 0, 0, ...seeds);
    });
    return out;
  }

  /** The months since burn `b` burnt, in the world's month `month`. */
  private since(b: Burn, month: number): number { return Math.max(month - b.month, 0); }

  /** Whether square `i` still holds burn `b`: it has burnt no earlier since (its months since it last burnt are no more
   * than those since the burn). */
  private holds(b: Burn, i: number, age: Float32Array, month: number): boolean {
    return age[i] <= this.since(b, month) + AGE_SLACK;
  }
}
