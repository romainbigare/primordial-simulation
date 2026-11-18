// The marks' three lattices (textures.ts, web/shaders/lib/mark.wgsl): their cells, how far a mark reaches, which plants
// each stands, and how a kind's groups of plants are set out on them as marks.
import { clamp } from './earth';
import { K } from './kind_row';

/** The side of a cell of each lattice of marks, m: one mark a cell at most. A mark reaches at most `REACH` cells from its
 * cell, so it is drawn at most twice that many sides across; a kind is drawn on the lattices whose side is nearest its
 * crown's width (on a log scale), so the three hold every bush and tree from 5 cm to 6 m across, wider ones as more
 * marks. Herbs have no marks: they are the ground's sward (cover_field.wgsl `sward`). */
export const LATTICES = [0.1, 0.5, 2.5];
/** Of each lattice, the cells a mark reaches from its own (cover.wgsl looks that many cells each way about a pixel): the
 * trees' two, so a crown up to four sides across stands among the small ones of its lattice. */
export const REACH = [1, 1, 2];
/** A plant stands up this share of its height on the map (lib/plant.wgsl `STAND`): the rows south of a pixel
 * looked at, past those its marks reach, reach its tallest plant, but no more than `ROWS_MOST` (each row costs a pixel
 * a look at marks for each cell across, textures.ts), so lattice l stands a plant up to (`ROWS_MOST` + `REACH`[l]) of
 * its sides over `STAND` tall (`shareOn`). */
export const STAND = 0.5, ROWS_MOST = 3;
/** The lattice whose kinds grow in stands of their own, near and far alike (cover_field.wgsl `stand_pick`): the trees'. */
export const STANDS = 2;
/** A mark's sprite is drawn up to this much wider than its crown, so its leaves, which have gaps, cover its
 * crown's ground (models.wgsl `MODEL_WIDER`): a crown is at most its reach over this. */
const SPRITE_WIDER = 1.6;
/** A bush's mark is a clump of its plants this many times as wide as one, so its lobes read at a 10 m scale bar. */
const BUSH_GROW = 2.5;
/** A plant's size differs from its place on its kind's sizes by up to this share either way (lib/mark.wgsl `SIZES`). */
const SIZE_SPREAD = 0.3;
/** A stand's crowns share its canopy and touch; the marks' places are random, so crowns each as wide as the ground it
 * holds overlap and leave gaps. Marks whose crowns sum to x of the ground cover 1 - e^-x of it: a stand that closes the
 * share c of its ground draws each crown's area -ln(1 - c) / c times as large, so the ground its crowns cover together,
 * overlaps counted once, is the stand's (c at most this; a mark no wider than its lattice lets it reach, and more of
 * them where it would be wider). */
const CLOSED_MOST = 0.95;

/** How many plants across a mark of a group of plants `height` m tall is, of a kind whose plants are trees from `from`
 * m (style.ts `treeFrom`): `BUSH_GROW` as far as its marks are bushes, each mark by its own size (`SIZE_SPREAD`), so the
 * group's marks change their width smoothly as they pass from bushes to trees. */
function clumpOf(height: number, from: number): number {
  const trees = clamp((1 + SIZE_SPREAD - from / Math.max(height, 1e-9)) / (2 * SIZE_SPREAD));
  return BUSH_GROW + (1 - BUSH_GROW) * trees;
}

/** The share of a kind of crown width `w`, m, drawn on lattice `l` by its width: between the two lattices whose sides
 * lie either side of it, by where it lies between them on a log scale; all on the first below it and on the last above
 * it. */
function shareByWidth(w: number, l: number): number {
  const x = Math.log(Math.max(w, 1e-6)), at = LATTICES.map(Math.log);
  const below = l > 0 ? clamp((x - at[l - 1]) / (at[l] - at[l - 1])) : 1, above = l < at.length - 1 ? clamp((at[l + 1] - x) / (at[l + 1] - at[l])) : 1;
  return Math.min(below, above);
}

/** A lattice too small for the tallest plants hands them up over a band of heights: from this share of the height it
 * stands on (`standsOn`). */
const HANDS_UP_FROM = 0.6;

/** The share of plants `h` m tall that lattice `l` may stand: all up to `HANDS_UP_FROM` of the height it stands
 * (`STAND`, `ROWS_MOST`), none from that height on, between on a log scale, so the plants of a growing stand move up a
 * share a year, the tallest first; the last stands all. */
function standsOn(h: number, l: number): number {
  if (l === LATTICES.length - 1) return 1;
  const most = ((ROWS_MOST + REACH[l]) * LATTICES[l]) / STAND;
  return clamp(Math.log(most / Math.max(h, 1e-9)) / -Math.log(HANDS_UP_FROM));
}

/** The share of a kind of crown width `w`, m, and height `h`, m, drawn on lattice `l`: by its width (`shareByWidth`),
 * of what the smaller lattices do not stand at its height (`standsOn`), so a tall plant of a narrow crown stands as tall
 * as it is and a growing plant moves up over a band of heights. */
function shareOn(w: number, h: number, l: number): number {
  let passed = 0;
  for (let m = 0; m < l; m++) passed = (passed + shareByWidth(w, m)) * (1 - standsOn(h, m));
  return (passed + shareByWidth(w, l)) * standsOn(h, l);
}

/** The marks of a group of a kind on a lattice: the chance a cell holds one (as a share of the whole square), the width
 * a mark is drawn, m, the plants a mark stands for, and the share of the square its marks on this lattice cover. */
export interface MarksOn { chance: number; width: number; plants: number; covers: number }

/** The marks of the groups `groups` of a kind (`kind_row.ts` `groupsOf`, their crowns as they stand) on lattice `l`, its
 * plants trees from `from` m, on the share `rooted` of the square that holds it. Its plants stand in patches that
 * cover the share of the square its crowns do, each crown the ground it holds there; a cell of a patch holds the plants
 * of a group that fall in it as one mark (a clump where they are many, `clumpOf`), never more marks than plants. Each
 * mark is drawn as much wider as closes the kind's canopy, all its groups together, at random places (`CLOSED_MOST`),
 * at most `REACH` cells from its own, and where that is too narrow its marks stand more often (at most one a cell), so
 * the ground they cover on the rooted share is the kind's, each group by its crowns' share of it. A group's width is
 * given whatever its share on the lattice, so a mark's size passes smoothly between its kind's groups (mark.wgsl
 * `along`). */
export function marksOf(groups: Float32Array[], l: number, from: number, rooted: number): MarksOn[] {
  const a = LATTICES[l], most = (2 * REACH[l] * a) / SPRITE_WIDER;
  const all = groups.reduce((s, q) => s + Math.max(q[K.cover], 0), 0);
  // The crowns of all its groups sum to -ln(1 - closed) of the ground (taken as ln(1 + x), exact for a stand that
  // closes almost none).
  const closed = Math.min(all / rooted, CLOSED_MOST), sum = -Math.log1p(-closed), wider = closed > 0 ? Math.sqrt(sum / closed) : 1;
  return groups.map(q => {
    const grow = clumpOf(q[K.height], from), share = shareOn(q[K.width] * grow, q[K.height], l);
    const plants = q[K.cover] > 0 ? Math.max((q[K.perM2] / q[K.cover]) * a * a / (grow * grow), 1) : 1;
    const width = Math.min(Math.min(q[K.width] * grow * Math.sqrt(plants), most) * wider, most);
    if (share <= 0 || q[K.cover] <= 0 || q[K.perM2] <= 0 || all <= 0) return { chance: 0, width, plants, covers: 0 };
    // The marks a cell of the rooted share holds: their crowns sum to the group's part of the kind's, or as near as
    // one a cell comes; the ground they cover is what is drawn, near and far.
    const area = (Math.PI / 4) * width * width, chance = Math.min(1, ((q[K.cover] / all) * sum * a * a) / area);
    return { chance: share * chance * rooted, width, plants, covers: share * rooted * -Math.expm1(-(chance * area) / (a * a)) };
  });
}
