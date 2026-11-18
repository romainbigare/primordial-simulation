// The Earth view's style for the plant cover (web/shaders/lib/style.wgsl holds the style of each pixel): the colours a
// kind of cover is drawn in, pushed far apart from one another, lighter than real leaves, and turning with the season;
// and the design its plants are drawn with (the form of a tree's crown is form.ts's). Each is a function of the kind's
// numbers alone (tape/src/stands.rs), so the same numbers always give the same look and a kind that evolves drifts
// smoothly from its parent. The marks near (cover.wgsl) and the field far away (cover_field.wgsl) draw the same colours
// (`seenFar`). Colours are worked in OKLab (Ottosson 2020), whose distances follow the eye's, and handed on as
// reflectances in linear light.
import { clamp, smooth } from './earth';

export type Rgb = [number, number, number];
/** The numbers of a kind its look reads (`Kind::row`): its capture area per kg, log10 m²/kg; its walls' share; its
 * grown plants' walls' share (`grownWalls`); its moisture gene; its frost gene, K; its light capacity per m² of capture
 * area, log10 mol/m²/s; its activity; the share of it frozen this month; the share of what it builds that goes to
 * seeds; its design (`DESIGN`); and its ground's temperature and the melting point of the frozen cover, K. */
export interface Leafy { leaf: number; wood: number; walls: number; moist: number; frost: number; pigment: number; act: number; frozen: number; seed: number; design: number; t: number; melt: number }

/** A kind's look this month: the colours its plants span (each plant takes a point between them by its own random
 * number), a reflectance each; the share of its leaves left this season (less as a kind that sheds stands bare while
 * dormant); and its flowers: their colour and the share of its plants in flower (the sward draws a herb's). */
export interface KindLook { from: Rgb; to: Rgb; left: number; petal: Rgb; bloom: number }

/** The designs a kind's plants are drawn with: a tree's crown on its stem and a bush (cover.wgsl
 * `plant`), and a herb, drawn only as the sward of the ground (cover_field.wgsl `sward`). */
export const DESIGN = { tree: 0, bush: 1, herb: 2 } as const;
/** A woody plant (its grown plants hold `BUSH_WOOD` of their dry mass as walls or more) this tall or taller, m, is a
 * tree, a lower one a bush (`designOf`). A soft kind stands as plants (a palm, a tree fern, a giant herb) from
 * `SOFT_STANDS_M`[0] m tall, all of it from [1] m, and is the sward below (`standsOf`); it is soft as far as its walls'
 * share lies under `BUSH_WOOD`, over `SOFT_MIX` about it (lib/models.wgsl `SOFT_WOOD`, `SOFT_MIX`, which pick its
 * sprites). */
const TREE_M = 3, BUSH_WOOD = 0.6, SOFT_MIX = 0.1;
const SOFT_STANDS_M = [2, 4];

// The palette of the greens: a kind's hue (OKLCH, degrees) from its leaves' thickness (thin leaves yellow-green, thick
// ones blue-green), its frost gene (tender kinds warmer) and its moisture gene (kinds of wet ground bluer); its
// lightness from its leaves (thin ones lighter) and its walls (woody kinds darker); its chroma less the drier the
// ground it is made for (the grey-green of dry country). Each is the middle value plus a slope times the number's
// departure from the starting cover's.
/** Hue at the starting cover's numbers, degrees, and its change per decade of capture area, per K of frost gene, per
 * decade of moisture gene. */
const HUE_MID = 140, HUE_PER_LEAF = -55, HUE_PER_FROST = -4, HUE_PER_MOIST = 25;
/** The greens' hues stay living greens, from yellow-green to blue-green, degrees: a kind of thick hardy leaves is the
 * bluest green of the palette, not a teal. */
const HUE_GREENS = [105, 150];
/** Lightness at the starting cover's numbers, and its change per decade of capture area and per share of walls. */
const L_MID = 0.75, L_PER_LEAF = 0.14, L_PER_WOOD = -0.15;
/** Chroma at the starting cover's numbers, and its share lost per decade of moisture gene below theirs. */
const C_MID = 0.13, C_DRY = 0.3;
/** The starting cover's numbers (worldgen/src/life/kinds.rs `COVER_GENES`): capture area, log10 m²/kg; walls' share;
 * moisture gene; frost gene, K; seeds' share. */
const LEAF_0 = 0.4, WOOD_0 = 0.5, MOIST_0 = -0.3, FROST_0 = 268, SEED_0 = 0.1;
/** A herb is the ground's sward under the trees and bushes: its lightness is drawn toward `SWARD_L` (keeping this share
 * of its departure from it) and its chroma is this share of a plant's, so the ground does not outshine what stands on it
 * and the plants stand out on it by shape and shadow. */
const SWARD_L = 0.64, SWARD_L_KEEP = 0.5, SWARD_CHROMA = 0.7;
/** The physical leaf's lightness (canopy.ts `foliage`) counts this much over that of a green leaf (`LEAF_PHYS`), so a
 * kind short of fuel still shows paler. */
const PHYSICAL_L = 0.3;
const LEAF_PHYS: Rgb = [0.05, 0.12, 0.04];

// The season: a woody kind with thin leaves sheds them (thin leaves are short-lived, and a short-lived leaf is shed in
// the season that cannot pay for it: Kikuzawa 1991; Wright and others 2004). It turns while the ground cools (autumn)
// towards its frost gene, through yellow and orange as far towards red as its leaves' pigment reaches, then stands
// brown and thin while dormant; while the ground warms (spring) it greens again from bare. Any kind turns where the
// cold shuts it down (frozen ground, its frozen share), an evergreen kind only then. A kind shut down by drought goes
// straw, then brown.
/** A woody kind sheds as its capture area per kg rises from the first to the second, log10 m²/kg. */
const SHED_LEAF = [0.05, 0.2];
/** A shedding kind's turn starts this many K over its frost gene; ground within this many K over the melting point of
 * the frozen cover holds its liquid frozen, so a kind dormant there is dormant from the cold, not from drought. */
const COLD_NEAR_K = 25, FROZEN_GROUND_K = 1.5;
/** The coldest time of the year in the north, a share of the year after its midwinter (the ground lags the sun): the
 * ground cools for half a year before it and warms for half a year after it; mirrored in the south. The turn shows
 * fully once the ground's warming, as the sine of the season, falls under `-COOLING_SINE`. */
const COLDEST = 0.15, COOLING_SINE = 0.3;
/** The turning colours (OKLCH: lightness, chroma, hue): yellow, orange, red, dormant brown, and straw. */
const YELLOW: Rgb = [0.86, 0.17, 95], ORANGE: Rgb = [0.74, 0.18, 58], RED: Rgb = [0.6, 0.19, 28], BARE: Rgb = [0.5, 0.06, 55], STRAW: Rgb = [0.8, 0.09, 88];
/** How far a kind turns from yellow through orange to red: from 0 to 1 as its light capacity per m² of capture area
 * (more pigment per area) rises from `RED_FROM` to `RED_TO`, log10 mol/m²/s; its plants spread `HUE_SPREAD` of it
 * either way. */
const RED_FROM = -5.32, RED_TO = -5.12, HUE_SPREAD = 0.3;
/** The stages of the turn: green to a pale form of its deepest colour by `TO_PALE` (this share of the way from
 * yellow to it), to its deepest by `TO_DEEP`, to bare by `TO_BARE`. */
const TO_PALE = 0.25, PALE = 0.4, TO_DEEP = 0.45, TO_BARE = 0.8;
/** A kind dry this far into its dormancy has gone straw; past it, brown. */
const TO_STRAW = 0.5;
/** Weathered straw is this far from straw towards dormant brown. */
const WEATHERED = 0.35;
/** One plant turns at its own time: the plants of a kind span this much of the turn either way of its stage, once the
 * stage is twice this far in. */
const TURN_SPREAD = 0.2;
/** A dormant crown of a kind that sheds keeps this share of its leaves (and a crown whose plants stand bare, canopy.ts
 * `kindsOf`). */
export const BARE_LEFT = 0.35;
/** A kind that keeps its leaves keeps them green through the cold and the drought: darker by up to this share of its
 * lightness in the cold, and duller by up to this share of its chroma in the drought. */
const KEPT_DARK = 0.25, KEPT_DULL = 0.4;

// Flowers: a herb in its active season flowers as much as it puts into seeds; the flowers' colour turns round the hue
// circle with its seed share and its moisture gene.
/** The share of plants in flower per share of what is built that goes to seeds, at most 1. */
const BLOOM_PER_SEED = 2.5;
/** The flowers' place on their palette (0..1, round) at the starting cover, and its change per share of seeds and per
 * decade of moisture gene. */
const PETAL_0 = 0.15, PETAL_PER_SEED = 2.2, PETAL_PER_MOIST = 0.8;
/** The flowers' palette (OKLCH), round: white, yellow, orange, red, pink, purple, blue. */
const PETALS: Rgb[] = [[0.96, 0.02, 100], [0.9, 0.17, 95], [0.78, 0.17, 60], [0.64, 0.2, 25], [0.78, 0.14, 350], [0.62, 0.16, 310], [0.7, 0.13, 260]];

const lerp3 = (a: Rgb, b: Rgb, t: number): Rgb => [0, 1, 2].map(i => a[i] + (b[i] - a[i]) * t) as Rgb;

/** Linear sRGB `c` in OKLab (Ottosson 2020). */
function oklab(c: Rgb): Rgb {
  const l = Math.cbrt(0.4122214708 * c[0] + 0.5363325363 * c[1] + 0.0514459929 * c[2]);
  const m = Math.cbrt(0.2119034982 * c[0] + 0.6806995451 * c[1] + 0.1073969566 * c[2]);
  const s = Math.cbrt(0.0883024619 * c[0] + 0.2817188376 * c[1] + 0.6299787005 * c[2]);
  return [0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s, 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s, 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s];
}
/** OKLab `p` in linear sRGB, held within 0..1. */
function linear(p: Rgb): Rgb {
  const l = (p[0] + 0.3963377774 * p[1] + 0.2158037573 * p[2]) ** 3;
  const m = (p[0] - 0.1055613458 * p[1] - 0.0638541728 * p[2]) ** 3;
  const s = (p[0] - 0.0894841775 * p[1] - 1.291485548 * p[2]) ** 3;
  return [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s, -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s].map(v => clamp(v)) as Rgb;
}
/** OKLCH (lightness, chroma, hue in degrees) in OKLab. */
const fromLch = (c: Rgb): Rgb => [c[0], c[1] * Math.cos((c[2] * Math.PI) / 180), c[1] * Math.sin((c[2] * Math.PI) / 180)];

/** How much the ground is cooling at `year` (years: its fraction is the season from the north's midwinter) at latitude
 * `lat`, degrees: 0 (warming) .. 1 (cooling). */
export function cooling(year: number, lat: number): number {
  const phase = year - Math.floor(year) - COLDEST + (lat < 0 ? 0.5 : 0);
  return smooth(COOLING_SINE, -COOLING_SINE, Math.sin(2 * Math.PI * phase));
}

/** The design of the standing plants of a kind whose plants stand `height` m tall and whose walls gene is `walls`
 * (log10 kg of walls per kg of matter, the core's `WALLS`): a woody one (its grown plants hold `BUSH_WOOD` of their dry
 * mass as walls or more, `grownWalls`; laws.md 3: a plant's share of walls nears the gene as it grows) `TREE_M` tall
 * or taller is a tree, a lower one a bush (so a young stand of a woody kind is drawn as young woody plants, never as
 * meadow); a soft one stands as a tree (a palm, a tree fern or a giant herb: models.wgsl `model_set`) as far as it
 * stands (`standsOf`). */
export function designOf(height: number, walls: number): number {
  return height >= treeFrom(walls) ? DESIGN.tree : DESIGN.bush;
}

/** The height from which a plant of a kind whose walls gene is `walls` is drawn as a tree, m (`designOf`): a woody
 * kind's `TREE_M`, a soft kind's 0. Each plant takes its design by its own height (lib/mark.wgsl `plant_of`), so a kind
 * passes from bushes to trees plant by plant as it grows. */
export function treeFrom(walls: number): number {
  return grownWalls(walls) < BUSH_WOOD ? 0 : TREE_M;
}

/** The share of a kind whose plants stand `height` m tall and whose walls gene is `walls` that stands as plants (marks,
 * of design `designOf`); the rest is the sward. A woody kind stands whole; a soft one comes in over `SOFT_STANDS_M` as
 * it grows. */
export function standsOf(height: number, walls: number): number {
  return grownWalls(walls) < BUSH_WOOD ? smooth(SOFT_STANDS_M[0], SOFT_STANDS_M[1], height) : 1;
}

/** The share of kind `k` that is a soft standing plant (a palm, a tree fern, a giant herb: of the tree design, whose
 * grown plants' walls lie under `BUSH_WOOD`, over `SOFT_MIX`, as its sprites are picked): such plants keep their
 * leaves and their colour all year (the owner's rule). */
function softOf(k: Leafy): number {
  return k.design === DESIGN.tree ? smooth(BUSH_WOOD + SOFT_MIX / 2, BUSH_WOOD - SOFT_MIX / 2, k.walls) : 0;
}

/** The share of a grown plant's dry mass that is walls, of a kind whose walls gene is `walls` (log10 kg of walls per kg
 * of matter): 10^walls kg of walls a kg of matter. */
export function grownWalls(walls: number): number {
  return 10 ** walls / (1 + 10 ** walls);
}

/** The green of kind `k` (OKLab), its lightness drawn in part from its physical leaf `phys` (linear); a herb's as the
 * sward's (`SWARD_L`). */
function green(k: Leafy, phys: Rgb): Rgb {
  const hue = Math.min(Math.max(HUE_MID + HUE_PER_LEAF * (k.leaf - LEAF_0) + HUE_PER_FROST * (k.frost - FROST_0) + HUE_PER_MOIST * (k.moist - MOIST_0), HUE_GREENS[0]), HUE_GREENS[1]);
  const lightness = L_MID + L_PER_LEAF * (k.leaf - LEAF_0) + L_PER_WOOD * (k.wood - WOOD_0) + PHYSICAL_L * (oklab(phys)[0] - oklab(LEAF_PHYS)[0]);
  const chroma = C_MID * clamp(1 - C_DRY * Math.max(MOIST_0 - k.moist, 0));
  const sward = k.design === DESIGN.herb;
  return fromLch([sward ? SWARD_L + SWARD_L_KEEP * (lightness - SWARD_L) : lightness, sward ? SWARD_CHROMA * chroma : chroma, hue]);
}

/** The deepest colour of the turn (OKLCH) of a kind reaching `reach` of the way from yellow through orange to red. */
function deepest(reach: number): Rgb {
  const r = clamp(reach);
  return r < 0.5 ? lerp3(YELLOW, ORANGE, 2 * r) : lerp3(ORANGE, RED, 2 * r - 1);
}

/** Colour (OKLab) at stage `s` of the turn (0 green .. 1 dormant) from green `g`, reaching `reach` (`deepest`), while
 * the ground cools by `falling`: through a pale form of its deepest colour to it; while it warms, from bare to green. */
function turned(g: Rgb, s: number, reach: number, falling: number): Rgb {
  const deep = fromLch(deepest(reach)), pale = lerp3(fromLch(YELLOW), deep, PALE), bare = fromLch(BARE);
  const autumn = s < TO_PALE ? lerp3(g, pale, s / TO_PALE) : s < TO_DEEP ? lerp3(pale, deep, (s - TO_PALE) / (TO_DEEP - TO_PALE))
    : lerp3(deep, bare, Math.min((s - TO_DEEP) / (TO_BARE - TO_DEEP), 1));
  const spring = lerp3(g, bare, smooth(TO_DEEP, TO_BARE, s));
  return lerp3(spring, autumn, falling);
}

/** How much kind `k` sheds its leaves in the cold or the drought, 0..1: as thin as its leaves are; herbs not (they
 * die back, `kindLook`), nor soft standing plants (`softOf`). */
function shedOf(k: Leafy): number {
  return k.design === DESIGN.herb ? 0 : smooth(SHED_LEAF[0], SHED_LEAF[1], k.leaf) * (1 - softOf(k));
}

/** The look of kind `k` whose physical leaf reflects `phys` (linear), while the ground cools by `falling` (`cooling`):
 * its stage in the cold's turn (a shedding kind as its ground nears its frost gene; any kind on frozen ground or by its
 * frozen share) and in the drought's (dormant on unfrozen ground). Only a kind of thin leaves turns and sheds (`shedOf`;
 * a herb dies back to straw); one of thick leaves keeps them, darker in the cold and duller in the drought; a soft
 * standing plant keeps its leaves and its colour all year (`softOf`). */
export function kindLook(k: Leafy, phys: Rgb, falling: number): KindLook {
  const g = green(k, phys), frozenGround = smooth(k.melt + FROZEN_GROUND_K, k.melt, k.t);
  const sheds = k.design === DESIGN.herb ? 1 : shedOf(k), seasons = 1 - softOf(k);
  const cold = seasons * Math.max(shedOf(k) * smooth(k.frost + COLD_NEAR_K, k.frost, k.t), (1 - k.act) * frozenGround, k.frozen);
  const drought = seasons * (1 - k.act) * (1 - frozenGround);
  const dry = (c: Rgb) => lerp3(lerp3(c, fromLch(STRAW), smooth(0, TO_STRAW, drought)), fromLch(BARE), smooth(TO_STRAW, 1, drought));
  const reach = smooth(RED_FROM, RED_TO, k.pigment), spread = TURN_SPREAD * smooth(0, 2 * TURN_SPREAD, cold);
  const kept: Rgb = [g[0] * (1 - KEPT_DARK * clamp(cold)), g[1] * (1 - KEPT_DULL * drought), g[2] * (1 - KEPT_DULL * drought)];
  const at = (s: number, r: number) => linear(lerp3(kept, dry(turned(g, clamp(s), r, falling)), sheds));
  const dormant = Math.max(smooth(TO_DEEP, TO_BARE, cold), smooth(TO_STRAW, 1, drought)) * sheds;
  const place = PETAL_0 + PETAL_PER_SEED * (k.seed - SEED_0) + PETAL_PER_MOIST * (k.moist - MOIST_0);
  const x = (((place % 1) + 1) % 1) * PETALS.length, i = Math.floor(x);
  return {
    from: at(cold - spread, reach - HUE_SPREAD), to: at(cold + spread, reach + HUE_SPREAD), left: 1 - (1 - BARE_LEFT) * dormant,
    petal: linear(lerp3(fromLch(PETALS[i]), fromLch(PETALS[(i + 1) % PETALS.length]), x - i)),
    bloom: clamp(BLOOM_PER_SEED * k.seed) * (1 - Math.max(cold, drought)),
  };
}

/** Each plant of a kind takes its own colour between the kind's two (`KindLook`) and its own tone, up to this share
 * lighter or darker (web/shaders/lib/style.wgsl `TONE_JITTER`). */
const TONE_SPREAD = 0.2;
/** The colours between the kind's two and the tones that the field's mean is taken over. */
const SEEN_COLOURS = 5, SEEN_TONES = 3;

/** Linear light `x` as the screen's share of full scale (IEC 61966-2-1, as web/shaders/lib/draw.wgsl `srgb`), and back. */
export const toScreen = (x: number) => (x <= 0.0031308 ? 12.92 * x : 1.055 * x ** (1 / 2.4) - 0.055);
export const fromScreen = (x: number) => (x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4);

/** The mean colour of a kind of look `l` as the field far away draws it: the middle of its plants' colours as the screen
 * shows them. A mix of colours on a screen averages what the screen shows, not the light (the screen's curve bends), so
 * plants of many colours look deeper from near than the mean of their light; the field is the mean of what the plants
 * show, as the near picture comes to when its plants take a pixel each. */
/** The colour of cured straw standing (the mean of a straw and a weathered straw), as the field far away draws it. */
export function strawSeen(): Rgb {
  const s = linear(fromLch(STRAW));
  return seenFar({ from: s, to: linear(lerp3(fromLch(STRAW), fromLch(BARE), WEATHERED)), left: 1, petal: s, bloom: 0 });
}

export function seenFar(l: KindLook): Rgb {
  const sum: Rgb = [0, 0, 0];
  for (let i = 0; i < SEEN_COLOURS; i++) {
    const c = lerp3(l.from, l.to, (i + 0.5) / SEEN_COLOURS);
    for (let j = 0; j < SEEN_TONES; j++) {
      const tone = 1 + TONE_SPREAD * ((2 * (j + 0.5)) / SEEN_TONES - 1);
      for (let q = 0; q < 3; q++) sum[q] += toScreen(Math.min(c[q] * tone, 1));
    }
  }
  return sum.map(v => fromScreen(v / (SEEN_COLOURS * SEEN_TONES))) as Rgb;
}
