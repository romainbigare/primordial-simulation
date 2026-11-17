// A kind's row of the core's table as the page reads it (tape/src/stands.rs `row`): its numbers, its groups of plants by
// size, its crowns as they stand through the year, and the colour of its leaves.
import type { KindLook, Rgb } from './style';
import { clamp, smooth } from './earth';
import { grownWalls, kindLook } from './style';

/** The numbers of a kind in the core's row (tape/src/stands.rs `row`). */
export const K = { root: 0, kg: 1, stem: 2, perM2: 3, height: 4, width: 5, cover: 6, crown: 7, wood: 8, leaf: 9, pigment: 10, moist: 11, frost: 12, fuel: 13, act: 14, frozen: 15, seed: 16, rooted: 17, need: 18, walls: 19, slot: 20 } as const;
/** A kind's groups of plants in its row (tape/src/stands.rs `row`): from `GROUP_AT` on, `GROUP_ROW` numbers each, the
 * large plants first, empty ones last: plants a m², height, stem's width and crown's width, m. The kinds' table holds
 * `GROUPS` of them (one number of a plane each, lib/mark.wgsl `GROUPS`; the core's `cover::GROUPS`). */
const GROUP_AT = 21, GROUP_ROW = 4;
export const GROUPS = 4;

/** The groups of plants of the kind of row `r` (`GROUP_AT`), the held ones, the large first: each a row of its own of
 * its plants' count, height, stem and crown, and the share of the ground its crowns cover (tape/src/stands.rs `plants`:
 * the crowns of its plants a m² at their width), the rest its kind's. */
export function groupsOf(r: Float32Array): Float32Array[] {
  const out: Float32Array[] = [];
  for (let g = 0; g < GROUPS && GROUP_AT + (g + 1) * GROUP_ROW <= r.length; g++) {
    const at = GROUP_AT + g * GROUP_ROW;
    if (!(r[at] > 0)) break;
    const q = r.slice(0, GROUP_AT);
    [q[K.perM2], q[K.height], q[K.stem], q[K.width]] = r.subarray(at, at + GROUP_ROW);
    q[K.cover] = (Math.PI / 4) * q[K.perM2] * q[K.width] * q[K.width];
    out.push(q);
  }
  return out;
}

/** A crown's width over its stem's: about 20 for broad-leaved trees grown in the open (Hemery, Savill and Pryor 2005:
 * 20 to 25), the reach its branches hold whatever leaves they bear. */
const CROWN_PER_STEM = 20;

/** The crowns of a kind that stands as plants (row `r`) on the share `rooted` of its square: as wide as its wood holds
 * them through the year, not as its leaves cover the ground this month, so its plants keep their size and place as it
 * sheds and leafs out: `CROWN_PER_STEM` of its stem's width, no wider than its plants' share of the ground lets them stand, never
 * narrower than its leaves cover; the share of the ground they cover then, and how closed a crown is with the leaves
 * it bears (the cover's light law over that ground, as `tape/src/stands.rs` `plants`). */
export function standingCrowns(r: Float32Array, rooted: number): { width: number; cover: number; closed: number } {
  if (r[K.perM2] <= 0 || r[K.cover] <= 0) return { width: r[K.width], cover: r[K.cover], closed: r[K.crown] };
  const room = Math.sqrt(rooted / ((Math.PI / 4) * r[K.perM2]));
  const width = Math.max(r[K.width], Math.min(CROWN_PER_STEM * r[K.stem], room));
  const cover = Math.min((Math.PI / 4) * r[K.perM2] * width * width, rooted);
  // Its capture area a m² of its square, from the cover its leaves close (the light law's inverse).
  const leaf = -rooted * Math.log(Math.max(1 - r[K.cover] / rooted, 1e-6));
  return { width, cover, closed: 1 - Math.exp(-leaf / Math.max(cover, 1e-9)) };
}

// Colour: reflectances in linear light, from a leaf's measured reflectance moved by the numbers the model holds; the
// GPU turns light into the screen's bytes once (draw.wgsl `srgb`). Chemistry 0 gives every part of the cover one
// reflectance (its walls'), so a leaf's own colour is the measured one and only its departures come from the model.
/** A green leaf's reflectance in the visible: about 0.05 red, 0.12 green, 0.04 blue (Gates and others 1965; Knipling
 * 1970). */
const LEAF_REFL: Rgb = [0.05, 0.12, 0.04];
/** More light capacity per m² of capture area is more pigment per area, which absorbs more: per decade above
 * `PIGMENT_MID` (log10 mol/m²/s) the reflectance falls by the factor 10^`PIGMENT_SLOPE` (within `PIGMENT_MOST` either
 * way). */
const PIGMENT_MID = -5.6, PIGMENT_SLOPE = 0.3, PIGMENT_MOST = 2;
/** Thick leaves (capture area per kg below `LEAF_MID`, log10 m²/kg) are darker and duller: per decade thicker, this
 * share less reflectance, and this share of the way to a grey of their brightness (more mesophyll and cuticle absorb
 * and scatter the green peak away; Wright and others 2004 for the thickness). */
const LEAF_MID = 0.3, THICK_DARK = 0.25, THICK_DULL = 0.4;
/** Leaves of dry ground carry hairs and wax that reflect more and greyer: per decade of moisture gene below `MOIST_MID`
 * (the starting cover's, fully active at half the ground's saturation), this far towards `DRY_REFL` (Ehleringer and
 * Björkman 1978, from memory). */
const MOIST_MID = -0.3;
const DRY_REFL: Rgb = [0.11, 0.13, 0.09];
/** Foliage hardy to frost (the frost gene from `TENDER_K` down to `HARDY_K`, K) is darker and bluer: towards
 * `HARDY_REFL` (the needles and waxes of hardy foliage). */
const TENDER_K = 272, HARDY_K = 258;
const HARDY_REFL: Rgb = [0.03, 0.08, 0.06];
/** Short of fuel (fuel under its reserve) a leaf yellows towards `PALE_REFL` (Knipling 1970), at most this share; how
 * it turns dormant, from the cold or from drought, is the style's (style.ts). */
const PALE_REFL: Rgb = [0.10, 0.14, 0.04];
const PALE_MOST = 0.4;
const mix = (a: Rgb, b: Rgb, t: number): Rgb => a.map((v, i) => v + (b[i] - v) * clamp(t)) as Rgb;

/** The reflectance of the crowns of the kind of row `r` whose walls reflect `walls`: a leaf's, less the more pigment
 * per area and the thicker its leaves (and duller), greyer the drier the ground it is made for, bluer the hardier to
 * frost; its walls showing through an open crown as its wall share is high; yellowed short of fuel. */
function foliage(r: Float32Array, walls: Rgb): Rgb {
  const pigment = clamp(10 ** (-PIGMENT_SLOPE * (r[K.pigment] - PIGMENT_MID)), 1 / PIGMENT_MOST, PIGMENT_MOST);
  const thick = clamp(LEAF_MID - r[K.leaf], 0, 1 / THICK_DARK);
  let c = LEAF_REFL.map(v => v * pigment * (1 - THICK_DARK * thick)) as Rgb;
  const grey = (c[0] + c[1] + c[2]) / 3;
  c = mix(c, [grey, grey, grey], THICK_DULL * thick);
  c = mix(c, DRY_REFL, MOIST_MID - r[K.moist]);
  c = mix(c, HARDY_REFL, smooth(TENDER_K, HARDY_K, r[K.frost]));
  c = mix(c, walls, r[K.wood] * (1 - r[K.crown]));
  return mix(c, PALE_REFL, PALE_MOST * (1 - clamp(r[K.fuel])));
}

/** The look of the kind of row `r` (style.ts `kindLook`), drawn with design `design`, on ground at `t` K where the
 * frozen cover melts at `melt` K, while the ground cools by `falling`, its walls reflecting `walls`. */
export function lookOfRow(r: Float32Array, design: number, walls: Rgb, t: number, melt: number, falling: number): KindLook {
  const k = { leaf: r[K.leaf], wood: r[K.wood], walls: grownWalls(r[K.walls]), moist: r[K.moist], frost: r[K.frost], pigment: r[K.pigment], act: r[K.act], frozen: r[K.frozen], seed: r[K.seed], design, t, melt };
  return kindLook(k, foliage(r, walls), falling);
}
