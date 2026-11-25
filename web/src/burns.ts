// The burns of god mode on the live map, kept while the page runs (fire.wgsl `burn_at`). The core burns a stroke's fire
// as a share of each cell it reached, with nothing of where in the cell (a small burn is spread over its patch, the core's
// `cover::hit`), so the page lays each square's share about the strike as the fire would have grown: an ellipse that
// grows downwind from the strike, its length over its breadth by the wind at mid-flame (Anderson 1983), its head as far
// ahead of the strike as its length and breadth give (Alexander 1985). Each square's burnt land is the part of it the
// ellipse reaches first, as much as its share (`limits`), so the burn's shape is the core's shares, before and after the
// world plays, while its front sweeps them in the order the fire reached them.

/** The most burns the map draws at once (fire.wgsl `BURNS`): the newest. */
export const BURNS = 4;
/** Numbers a burn takes in fire.wgsl's uniforms (`Fire.burns`): two four-wide members. */
export const BURN_FLOATS = 8;
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
 * layer's top): a square is a burn's while they are those since the burn within it. */
const AGE_SLACK = 0.5;

/** A burn: its strike, m; the way downwind (east, south; a unit); how far its head runs ahead of its mean growth and its
 * breadth over its length (`shape`); each square it burnt with how far its mean growth reaches when it has taken that
 * square's share (`limits`, m); the farthest of those; the world's month the core burnt it in; and its play: when it
 * began and was last drawn (performance.now, ms), how long it plays (ms), and whether it played out. */
interface Burn {
  strike: [number, number]; way: [number, number]; ahead: number; breadth: number;
  squares: Map<number, number>; end: number; month: number;
  begun: number; drawn: number; ms: number; over: boolean;
}

/** The shape of a fire in the low wind (`east`, `north`, m/s): the way downwind, how far its head runs ahead of its
 * mean growth (0: none, a circle about the strike), and its breadth over its length. */
function shape(east: number, north: number): Pick<Burn, 'way' | 'ahead' | 'breadth'> {
  const speed = Math.hypot(east, north), mph = (MIDFLAME * speed) / M_S_PER_MPH;
  const lb = Math.min(Math.max(LB_A * Math.exp(LB_B * mph) + LB_C * Math.exp(-LB_D * mph) - LB_E, 1), LB_MOST);
  // (Alexander 1985: the head's speed over the back's, (LB + √(LB² − 1)) / (LB − √(LB² − 1)).)
  const root = Math.sqrt(lb * lb - 1), hb = (lb + root) / (lb - root);
  return { way: speed > 0 ? [east / speed, -north / speed] : [1, 0], ahead: (hb - 1) / (hb + 1), breadth: 1 / lb };
}

/** How far burn `b`'s mean growth has reached when its edge reaches the point (x, y), m (fire.wgsl `burn_time`): the
 * ellipse of semi-axes t along the wind and `breadth` t across it, its centre `ahead` t downwind of the strike. */
function reachAt(b: Pick<Burn, 'strike' | 'way' | 'ahead' | 'breadth'>, x: number, y: number): number {
  const dx = x - b.strike[0], dy = y - b.strike[1], u = dx * b.way[0] + dy * b.way[1], v = b.way[0] * dy - b.way[1] * dx;
  const c = b.ahead, k = 1 - c * c;
  return (-u * c + Math.sqrt(u * u * c * c + k * (u * u + (v * v) / (b.breadth * b.breadth)))) / k;
}

/** Of burn `b` on squares of `cellM` m on a map `w` squares across: for each square of `shares` (its share burnt of
 * its dry land), how far the burn's mean growth reaches when it has taken that share of the square, m: the share's
 * quantile of the reach over the square's points. */
function limits(b: Pick<Burn, 'strike' | 'way' | 'ahead' | 'breadth'>, shares: Map<number, number>, w: number, cellM: number): Map<number, number> {
  const out = new Map<number, number>(), step = cellM / SIDE_POINTS, at = new Float64Array(SIDE_POINTS * SIDE_POINTS);
  for (const [i, share] of shares) {
    const x0 = (i % w) * cellM, y0 = Math.floor(i / w) * cellM;
    for (let k = 0; k < at.length; k++) at[k] = reachAt(b, x0 + ((k % SIDE_POINTS) + 0.5) * step, y0 + (Math.floor(k / SIDE_POINTS) + 0.5) * step);
    at.sort();
    out.set(i, at[Math.min(Math.round(Math.min(Math.max(share, 0), 1) * at.length), at.length - 1)]);
  }
  return out;
}

export class Burns {
  /** The burns kept, oldest first, no more than `BURNS`. */
  list: Burn[] = [];

  /** A burn struck at `strike` (m) in the low wind (`east`, `north`, m/s) while the world stands at its month `month`,
   * which burnt the share `shares` of each square's dry land, of a map `w` squares across of `cellM` m: it plays from
   * now. (The core burns an edit's fire in the coming month, the core's `flame::now`: its squares' months since they
   * burnt are none till that month has run.) */
  add(strike: [number, number], wind: [number, number], shares: Map<number, number>, month: number, w: number, cellM: number): void {
    const s = { strike, ...shape(...wind) }, squares = limits(s, shares, w, cellM), end = Math.max(0, ...squares.values()), now = performance.now();
    const ms = Math.min(Math.max((end / FRONT_M_S) * 1000, FRONT_LEAST_MS), FRONT_MOST_MS);
    this.list.push({ ...s, squares, end, month: month + 1, begun: now, drawn: 0, ms, over: false });
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

  /** The burns kept that still lie on some square of a map whose months since each square last burnt are `age`, in
   * the world's month `month` (`never`: the months that mean none): a square is a burn's while it has not burnt since. */
  keep(age: Float32Array, month: number, never: number): void {
    this.list = this.list.filter(b => [...b.squares.keys()].some(i => this.holds(b, i, age, month, never)));
  }

  /** Of square `i`, the newest burn kept that lies on it (its number among them, from 1; 0: none) and how far its mean
   * growth reaches when it has taken the square's share, m. */
  of(i: number, age: Float32Array, month: number, never: number): [number, number] {
    for (let k = this.list.length - 1; k >= 0; k--) {
      const b = this.list[k], limit = b.squares.get(i);
      if (limit !== undefined && this.holds(b, i, age, month, never)) return [k + 1, limit];
    }
    return [0, 0];
  }

  /** The burns as fire.wgsl reads them (`Fire.burns`) at time `now` (performance.now, ms): of each, its strike and its
   * way, then its head's lead, its breadth and how far its front has reached, m. */
  uniforms(now: number): number[] {
    const out = new Array<number>(BURNS * BURN_FLOATS).fill(0);
    this.list.forEach((b, k) => {
      const reach = b.over ? PASSED_M : b.end * Math.min(Math.max((now - b.begun) / b.ms, 0), 1);
      out.splice(k * BURN_FLOATS, BURN_FLOATS, ...b.strike, ...b.way, b.ahead, b.breadth, reach, 0);
    });
    return out;
  }

  /** Whether square `i` is still burn `b`'s: it has not burnt since (its months since it last burnt are those since
   * the burn). */
  private holds(b: Burn, i: number, age: Float32Array, month: number, never: number): boolean {
    return Math.abs(age[i] - Math.min(Math.max(month - b.month, 0), never)) <= AGE_SLACK;
  }
}
