// The form of a tree's crown in the Earth view (web/shaders/lib/models.wgsl draws each): a rule from its kind's genes
// alone (tape/src/stands.rs), never its plants' size, which grows with their age (the owner's rule; their size picks only
// the stage of a form's sprites, models.wgsl `model_set`), the one place the form is chosen, for the marks near (cover.wgsl) and the stands far
// away alike. Each form scores the numbers that favour it, the departure of each from a middle over a span; the best
// score wins, as surely as it leads the next (`FORM_SURE`): that share of the kind's trees stand in the form, each by
// its own draw, the rest round, so a kind that evolves across a border passes through the round crown plant by plant
// and never jumps from one form to another.
import { clamp, smooth } from './earth';

/** The genes of a kind its form reads: its capture area per kg, log10 m²/kg; its frost gene, K; its moisture gene; and
 * the share of what it builds that goes to seeds. */
export interface Shape { leaf: number; frost: number; moist: number; seed: number }
/** A form (0 round, 1 cone, 2 column, 3 umbrella, 4 tiered, 5 lobed: models.wgsl `model_set`) and how surely it is drawn
 * (0: as round .. 1: fully itself); how dry the ground its kind is made for, 0..1 (its moisture gene from the middle
 * of dry ground's to a span under it): the drawing picks a dry land's plant for a dry kind (models.wgsl); how wet,
 * 0..1 (its moisture gene from the middle of wet ground's to a span over it): a tree of wet ground stands on a flared
 * foot (models.wgsl `flared_foot`); and how tender to frost the kind is, 0..1 (`TENDER_SPANS`): only a tender kind's
 * soft plants are drawn as palms or ferns (models.wgsl). A kind is never both dry and wet (their middles part). */
export interface Form { form: number; sure: number; dry: number; wet: number; tender: number }

/** The middle and the span of each gene a form reads: thin leaves (capture area per kg, log10 m²/kg); tender (frost
 * gene, K); wet ground (moisture gene over the middle); dry ground (moisture gene under the middle); seedy (seeds'
 * share). Middles near those of the starting trees (worldgen life kinds), spans their spread after 50 years on the 22
 * islands of seeds 1 and 2. */
const BY_LEAF = [0.1, 0.1], BY_FROST = [269, 2], BY_WET = [-0.2, 0.15], BY_DRY = [-0.3, 0.2], BY_SEED = [0.05, 0.012];
/** A gene counts in a score as far as this many spans from its middle, no farther: one gene far out does not outvote all
 * the others. */
const SPANS_MOST = 1.5;
/** The weights of each form's score (in the order of `Form`): a constant, then thin, tender, wet, dry, seedy. The frost
 * gene weighs most: it parts the broad crowns of tender kinds from the needle trees of hardy ones. */
const FORM_SCORES: number[][] = [
  // Round: a broad crown is what a tender tree grows without a stronger cause.
  [0.7, 0, 1, 0, 0, 0],
  // Cone (a spruce, a fir): thick long-lived leaves on a kind hardy to frost, on ground not dry; a narrow cone sheds snow
  // and takes the low sun.
  [0.2, -0.5, -3, 0, -1, 0],
  // Column: a hardy kind that puts much into seeds races upward as a pioneer, narrow.
  [-0.6, 0, -1, 0, 0, 1],
  // Umbrella (a flat-topped tree of dry country, a quiver tree): a tender kind of dry ground spreads its crown flat over
  // its own roots.
  [-0.6, 0, 1, 0, 2, 0],
  // Tiered (a pine): thick long-lived leaves of a kind hardy to frost on dry ground, held in open layers over a tall
  // bare stem that pass light down (Horn 1971).
  [-0.5, -0.5, -3, -1, 2, 0],
  // Lobed: thin leaves of a tender kind on wet ground, fast shoots in loose clumps.
  [-0.8, 1, 1, 1, 0, 0],
];
/** A form leading the next by this much of a score or more is drawn fully as itself. */
const FORM_SURE = 0.5;
/** A kind is tender to frost as its frost gene lies over its middle (`BY_FROST`): none of it this many spans under, all
 * of it as many over. */
const TENDER_SPANS = 1;

const departure = (x: number, [mid, span]: number[]) => clamp((x - mid) / span, -SPANS_MOST, SPANS_MOST);

/** The form of a tree of kind `k`. */
export function formOf(k: Shape): Form {
  const wet = Math.max(departure(k.moist, BY_WET), 0), dry = Math.max(-departure(k.moist, BY_DRY), 0), tender = departure(k.frost, BY_FROST);
  const x = [1, departure(k.leaf, BY_LEAF), tender, wet, dry, departure(k.seed, BY_SEED)];
  const score = FORM_SCORES.map(w => w.reduce((s, v, i) => s + v * x[i], 0));
  const best = score.indexOf(Math.max(...score)), next = Math.max(...score.filter((_, i) => i !== best));
  return { form: best, sure: smooth(0, FORM_SURE, score[best] - next), dry: smooth(0, 1, dry), wet: smooth(0, 1, wet), tender: smooth(-TENDER_SPANS, TENDER_SPANS, tender) };
}
