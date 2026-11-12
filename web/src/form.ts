// The form of a tree's crown in the Earth view (web/shaders/lib/crown.wgsl draws each): a rule from its kind's own
// numbers (tape/src/stands.rs), the one place the form is chosen, for the marks near (cover.wgsl) and the stands far
// away alike. Each form scores the numbers that favour it, the departure of each from a middle over a span; the best
// score wins, as surely as it leads the next (`FORM_SURE`), so a kind that evolves across a border passes through the
// round crown and never jumps from one form to another.
import { smooth } from './earth';

/** The numbers of a kind its form reads: its capture area per kg, log10 m²/kg; its frost gene, K; its moisture gene;
 * its plants' height and crown's width, m; and the share of what it builds that goes to seeds. */
export interface Shape { leaf: number; frost: number; moist: number; height: number; width: number; seed: number }
/** A form (0 round, 1 cone, 2 column, 3 umbrella, 4 tiered, 5 lobed: crown.wgsl `crown`) and how surely it is drawn
 * (0: as round .. 1: fully itself). */
export interface Form { form: number; sure: number }

/** The middle and the span of each number a form reads: thin leaves (capture area per kg, log10 m²/kg); tender (frost
 * gene, K); wet ground (moisture gene over the middle); dry ground (moisture gene under the middle); slender (height
 * over crown's width); seedy (seeds' share). Middles near those of the starting trees (worldgen life kinds), spans their
 * spread after 50 years on the 22 islands of seeds 1 and 2; a stand's slenderness is its plants' count and mass at
 * work (the cover's law, laws.md 4.5: a young dense stand is poles, an old thinned one broad crowns), its middle and
 * span the median and half of the 16% to 84% range, by built cover, of the stands 3 m and taller after 50 years on the
 * page's island of seed 1. */
const BY_LEAF = [0.1, 0.1], BY_FROST = [269, 2], BY_WET = [-0.2, 0.15], BY_DRY = [-0.3, 0.2], BY_SLENDER = [4.4, 1], BY_SEED = [0.05, 0.012];
/** The weights of each form's score (in the order of `Form`): a constant, then thin, tender, wet, dry, slender, seedy. */
const FORM_SCORES: number[][] = [
  // Round: a broad crown is what a tree grows without a stronger cause.
  [0.7, 0, 0, 0, 0, 0, 0],
  // Cone: thick long-lived leaves on a kind hardy to frost; a narrow cone sheds snow and takes the low sun.
  [0, -1, -1, -1, 0, 0, 0],
  // Column: a slender kind that puts much into seeds races upward as a pioneer.
  [-0.6, 0, 0, 0, 0, 1, 1],
  // Umbrella: a kind of dry ground with a crown wide for its height spreads flat over its own roots.
  [-0.6, 0, 0, 0, 2, -1, 0],
  // Tiered: thick long-lived leaves of a tender kind on ground neither wet nor dry, held in flat layers that pass light
  // down (Horn 1971).
  [0, -1, 1, -1, -1, 0, 0],
  // Lobed: thin leaves on wet ground, fast shoots in loose clumps.
  [-0.8, 1, 0, 1, 0, 0, 0],
];
/** A form leading the next by this much of a score or more is drawn fully as itself. */
const FORM_SURE = 0.5;

const departure = (x: number, [mid, span]: number[]) => (x - mid) / span;

/** The form of a tree of kind `k`. */
export function formOf(k: Shape): Form {
  const wet = Math.max(departure(k.moist, BY_WET), 0), dry = Math.max(-departure(k.moist, BY_DRY), 0);
  const x = [1, departure(k.leaf, BY_LEAF), departure(k.frost, BY_FROST), wet, dry, departure(k.height / Math.max(k.width, 1e-6), BY_SLENDER), departure(k.seed, BY_SEED)];
  const score = FORM_SCORES.map(w => w.reduce((s, v, i) => s + v * x[i], 0));
  const best = score.indexOf(Math.max(...score)), next = Math.max(...score.filter((_, i) => i !== best));
  return { form: best, sure: smooth(0, FORM_SURE, score[best] - next) };
}
