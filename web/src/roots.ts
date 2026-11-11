// Where a kind's roots hold (web/shaders/lib/land.wgsl `rooted`): the drawing spreads a cell's cover over its ground by
// the depth of the loose ground at each point against the depth the kind's roots need. Bare bed has no pores to hold
// liquid and nothing to anchor a tall plant; so a kind thins where the ground is thinner than its roots need, and what
// a cell's bare part does not hold stands on its deep part: a kind's chance at a point is its rooted share there over
// its mean rooted share in its square (`rootedMean`), so the square keeps its cover.
import { smooth } from './earth';

/** A kind roots about this share of its height deep, m of loose ground per m of height (rooting depth grows with a
 * plant's size: grasses tens of cm, shrubs about a metre, trees a few metres; Schenk and Jackson 2002), and no kind
 * needs less than `ROOT_LEAST` m (a low mat's centimetres). */
export const ROOT_PER_HEIGHT = 0.1, ROOT_LEAST = 0.02;
/** A kind holds none of its ground from this share of its need down (land.wgsl `ROOTED_FROM`, the same rule). */
const ROOTED_FROM = 0.25;
/** A square's mean rooted share is counted no lower than this, so a kind the core keeps on a square of bare bed is
 * drawn there sparse rather than heaped on its few deep points. */
const MEAN_LEAST = 0.1;
/** A square's samples are counted by their loose depth in `BINS` bins spaced evenly in its log from `LEAST_M` to
 * `MOST_M` m (all under the first in the first, all over the last in the last). */
const BINS = 24, LEAST_M = 0.005, MOST_M = 20;

/** The loose ground of each square: of its samples the share in each bin (`BINS` a square). */
export interface Loose { w: number; shares: Float32Array }

/** The loose depth each bin stands for, m: its middle on the log scale. */
const BIN_M = Array.from({ length: BINS }, (_, b) => LEAST_M * (MOST_M / LEAST_M) ** ((b + 0.5) / BINS));

/** The share of a kind whose roots need `need` m that holds where `loose` m of loose ground lies (land.wgsl `rooted`). */
export function rooted(loose: number, need: number): number { return smooth(ROOTED_FROM * need, need, loose); }

/** The loose ground needed by a kind whose plants stand `height` m tall, m. */
export function needOf(height: number): number { return Math.max(ROOT_PER_HEIGHT * height, ROOT_LEAST); }

/** The loose ground of a map of `w` x `h` squares from its terrain's samples (`depths`, m, a sample each, `across` x
 * `down` of them, row by row). */
export function looseOf(depths: (s: number) => number, across: number, down: number, w: number, h: number): Loose {
  const shares = new Float32Array(w * h * BINS), per = across / w, perDown = down / h;
  for (let y = 0; y < down; y++) for (let x = 0; x < across; x++) {
    const sq = Math.min(Math.floor(y / perDown), h - 1) * w + Math.min(Math.floor(x / per), w - 1);
    const t = Math.log(Math.max(depths(y * across + x), LEAST_M) / LEAST_M) / Math.log(MOST_M / LEAST_M);
    shares[sq * BINS + Math.min(Math.floor(t * BINS), BINS - 1)] += 1 / (per * perDown);
  }
  return { w, shares };
}

/** The mean rooted share in square `i` of `loose` of a kind needing `need` m (1 before the samples' loose ground is
 * known), no lower than `MEAN_LEAST`. */
export function rootedMean(loose: Loose | null, i: number, need: number): number {
  if (!loose) return 1;
  let sum = 0;
  for (let b = 0; b < BINS; b++) sum += loose.shares[i * BINS + b] * rooted(BIN_M[b], need);
  return Math.max(sum, MEAN_LEAST);
}
