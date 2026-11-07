// The cover's field (textures.ts, web/shaders/cover.wgsl): of each texel the density of each kind of mark, from the
// cover's own numbers (its mass, walls and capture area per m², tape/src/layers.rs, or inside a cell each slot's own,
// tape/src/fine/inside.rs) and its ground temperature and rain. A kind gives way to the next over a range of each of its
// thresholds, so kinds mix where one gives way to another, never at an edge. The page makes it for the map's squares
// (vegetation.ts), the worker for each pixel of a fine raster (fine.ts), whose cover it hands to the page with its
// fields.
import { decode } from './data';
import { Fine, SLOTS } from './fine';
import { clamp, smooth } from './earth';

/** The numbers a pattern reads, per texel of a grid of `w` x `h` (row-major): the cover of each of `stands` stands
 * (`kg`, `wood` and `leaf` hold `stands` planes of a texel each), its ground's temperature and rain, the depth of the
 * liquid over it (none: not known, as on the map's squares) and whether cover is drawn on it (1: open ground). */
export interface CoverNumbers {
  w: number; h: number; stands: number; kg: Float32Array; wood: Float32Array; leaf: Float32Array; t: Float32Array; rain: Float32Array;
  depth: Float32Array | null; open: Uint8Array;
}

/** A tree's crown is drawn at these sizes across, m, blended between the two nearest. */
const CROWN_SIZES = [4, 8, 14];
/** A kind of mark: its size across, m, and the side of its grid cell over its size. */
interface Lattice { mark: number; spacing: number }
/** The kinds of mark in the order they are drawn (the low under the tall): the tufts of grass, tundra and steppe (the
 * first `TUFTS`), a shrub's crown, and the trees' crowns (from `TREES`). The spacings keep a mark within its own cell
 * and the eight about it. */
export const LATTICES: Lattice[] = [
  { mark: 0.25, spacing: 0.9 }, { mark: 0.35, spacing: 1.1 }, { mark: 0.5, spacing: 1.6 }, { mark: 1.5, spacing: 1.1 },
  ...CROWN_SIZES.map(mark => ({ mark, spacing: 0.8 })),
];
export const TUFTS = 3, TREES = 4;
/** The channels of a field's texel, each 0..1: the density of the marks of each lattice (in the order of `LATTICES`:
 * the share of its cells that hold a mark), how much darker the foliage is for the wood it holds, and the shares of the
 * crowns that are woodland and jungle (the rest are forest). */
export const CH = { grass: 0, tundra: 1, steppe: 2, shrubs: 3, crowns: 4, dark: 7, woodland: 8, jungle: 9 } as const;
/** Channels a texel holds: three layers of four. */
export const COVER_LAYERS = 3;
/** A field of the cover: `w` x `h` texels over a window of the map, its channels (`CH`) as bytes, layer by layer, and
 * its levels below (`mipsOf`). */
export interface CoverField { w: number; h: number; data: Uint8Array; mips: Uint8Array[] }
/** Set channel `ch` of texel `i` of field `f` to `v` (0..1). */
function putChannel(f: CoverField, i: number, ch: number, v: number): void {
  f.data[((ch >> 2) * f.w * f.h + i) * 4 + (ch & 3)] = Math.round(255 * Math.min(Math.max(v, 0), 1));
}

/** The levels of field `f` below its own, each texel the most of the four (or, at an odd edge, six or nine) above it:
 * the shader reads a level about a pixel to skip a kind none of whose marks stands near, so a level holds a kind
 * wherever any texel under it does. */
function mipsOf(f: CoverField): Uint8Array[] {
  const out: Uint8Array[] = [];
  let w = f.w, h = f.h, src = f.data;
  while (w > 1 || h > 1) {
    const nw = Math.max(1, w >> 1), nh = Math.max(1, h >> 1), dst = new Uint8Array(COVER_LAYERS * 4 * nw * nh);
    for (let l = 0; l < COVER_LAYERS; l++) for (let y = 0; y < nh; y++) for (let x = 0; x < nw; x++) {
      const ys = 2 * y, ye = y === nh - 1 ? h : Math.min(2 * y + 2, h), xs = 2 * x, xe = x === nw - 1 ? w : Math.min(2 * x + 2, w);
      for (let c = 0; c < 4; c++) {
        let most = 0;
        for (let sy = ys; sy < ye; sy++) for (let sx = xs; sx < xe; sx++) most = Math.max(most, src[((l * h + sy) * w + sx) * 4 + c]);
        dst[((l * nh + y) * nw + x) * 4 + c] = most;
      }
    }
    out.push(dst);
    w = nw; h = nh; src = dst;
  }
  return out;
}

/** The woody share of built cover (walls over walls and matter) from which a cover is drawn as crowns. */
const WOODY = 0.6;
/** Walls per m², kg, from which a woody cover is a woodland of small trees (below, shrubs), and from which a forest; a
 * forest holds about 10 to 50 kg/m² of dry matter (Whittaker and Likens 1975), mostly wood. */
const SHRUB_WOOD = 1, FOREST_WOOD = 3;
/** A forest is a jungle where its ground is warmer than this, K, its rain at least this, mol/m²/s (about 1,100 mm a
 * year), and its cover share at least this. */
const JUNGLE_T = 293, JUNGLE_RAIN = 2e-3, JUNGLE_CLOSED = 0.85;
/** Herbs are steppe where rain is below this, mol/m²/s (about 400 mm a year), and tundra where the ground is colder
 * than this, K, and the cover thinner than `TUNDRA_KG` kg/m². */
const STEPPE_RAIN = 7e-4, TUNDRA_T = 271, TUNDRA_KG = 1;
/** Below this cover share a place is a desert of rare shrubs. */
const DESERT_CLOSED = 0.12;
/** A kind gives way to the next from a threshold over (1 + `SOFT`) times or this many K either side of it. */
const SOFT = 0.3, SOFT_K = 2;
/** A tree's crown across, m: `CROWN_LOW` plus `CROWN_RISE` × (1 - e^(-walls / `CROWN_WOOD`)), walls in kg/m² (3 m in a
 * young stand to about 13 m in a stand of 30 kg/m²). */
const CROWN_LOW = 3, CROWN_RISE = 12, CROWN_WOOD = 15;
/** The foliage darkens by up to `DARK_MOST`, fully at 10^`DARK_DECADES` kg/m² of walls, as a tall canopy's shade
 * deepens (the map's forest is darker too, handover 10.3). */
const DARK_MOST = 0.4, DARK_DECADES = 2.5;
/** 0 below `at` less `soft`, 1 above `at` plus `soft`, smooth between. */
const past = (x: number, at: number, soft: number) => smooth(at - soft, at + soft, x);
/** The same on a log scale: from `at` / (1 + `SOFT`) to `at` × (1 + `SOFT`). */
const pastLog = (x: number, at: number) => past(Math.log(Math.max(x, 1e-30)), Math.log(at), Math.log1p(SOFT));

/** The shares of a tree's crowns drawn at each of `CROWN_SIZES` for walls `wood` kg/m²: its size blended between the
 * two nearest (on a log scale). */
function crownShares(wood: number): number[] {
  const s = CROWN_SIZES, c = clamp(CROWN_LOW + CROWN_RISE * (1 - Math.exp(-wood / CROWN_WOOD)), s[0], s[s.length - 1]);
  const i = Math.min(s.findIndex((_, k) => k === s.length - 1 || c < s[k + 1]), s.length - 2), t = Math.log(c / s[i]) / Math.log(s[i + 1] / s[i]);
  return s.map((_, k) => (k === i ? 1 - t : k === i + 1 ? t : 0));
}

/** What a stand puts in a texel: the density of each lattice's marks, and its trees' density, the part of it that is
 * forest and the part that is jungle. */
interface Stand { marks: number[]; trees: number; forest: number; jungle: number }

/** The marks of a stand whose cover holds `kg` and `wood` kg/m² and `leaf` capture area per m², whose ground is at `t` K
 * and whose rain is `rain` mol/m²/s: as many marks as its cover is closed (1 - e^-area), shared between the kinds by how
 * far it is past each of their thresholds. */
function standOf(kg: number, wood: number, leaf: number, t: number, rain: number): Stand {
  const closed = 1 - Math.exp(-leaf), cold = 1 - past(t, TUNDRA_T, SOFT_K), desert = 1 - pastLog(closed, DESERT_CLOSED), grown = 1 - desert;
  const woody = grown * pastLog(wood / kg, WOODY), herbs = grown - woody;
  const tundra = herbs * cold * (1 - pastLog(kg, TUNDRA_KG)), steppe = (herbs - tundra) * (1 - pastLog(rain, STEPPE_RAIN));
  const trees = woody * pastLog(wood, SHRUB_WOOD), forest = trees * pastLog(wood, FOREST_WOOD);
  const jungle = forest * past(t, JUNGLE_T, SOFT_K) * pastLog(rain, JUNGLE_RAIN) * pastLog(closed, JUNGLE_CLOSED);
  // A desert is rare tundra where it is cold and rare shrubs elsewhere.
  const herbal = [herbs - tundra - steppe, tundra + desert * cold, steppe, woody - trees + desert * (1 - cold)];
  return { marks: [...herbal, ...crownShares(wood).map(share => share * trees)].map(share => share * closed), trees: trees * closed, forest: forest * closed, jungle: jungle * closed };
}

/** The share of a mark of `size` m that stands out of liquid `depth` m deep over its foot: a plant about as tall as its
 * mark is wide stands out of shallow liquid and is under the surface in deeper. */
const emerged = (depth: number, size: number) => 1 - smooth(0.5 * size, size, depth);

/** Write into texel `i` of `out` the marks of the stands of `f` there: each stand's own (`standOf`), as many as the
 * stands hold together, those that stand under the liquid there left out. */
function putStands(out: CoverField, i: number, f: CoverNumbers): void {
  const n = f.w * f.h, marks = LATTICES.map(() => 0);
  let wood = 0, trees = 0, forest = 0, jungle = 0;
  for (let s = 0; s < f.stands; s++) {
    const kg = f.kg[s * n + i], leaf = f.leaf[s * n + i];
    if (kg <= 0 || leaf <= 0) continue;
    const st = standOf(kg, f.wood[s * n + i], leaf, f.t[i], f.rain[i]);
    st.marks.forEach((m, l) => { marks[l] += m; });
    wood += f.wood[s * n + i]; trees += st.trees; forest += st.forest; jungle += st.jungle;
  }
  const depth = f.depth ? f.depth[i] : 0;
  marks.forEach((m, l) => putChannel(out, i, l, m * emerged(depth, LATTICES[l].mark)));
  putChannel(out, i, CH.dark, clamp(Math.log10(1 + wood) / DARK_DECADES) * DARK_MOST);
  if (trees > 0) { putChannel(out, i, CH.woodland, (trees - forest) / trees); putChannel(out, i, CH.jungle, jungle / trees); }
}

/** The texels of the cover of numbers `f` as the marks read them, with the levels below them. */
export function coverOf(f: CoverNumbers): CoverField {
  const out: CoverField = { w: f.w, h: f.h, data: new Uint8Array(COVER_LAYERS * 4 * f.w * f.h), mips: [] };
  for (let i = 0; i < f.w * f.h; i++) if (f.open[i]) putStands(out, i, f);
  out.mips = mipsOf(out);
  return out;
}

/** Ice thicker than this, m, hides the cover (where the map starts to draw snow, web/shaders/lib/earth.wgsl). */
const SNOW_M = 0.02;

/** The cover's field of the pixels of `f`; null for a tape without the cover's layers. Each slot of a pixel's cell is a
 * stand of its own: its built cover there (`Fine.cover`), its walls and capture area by what it builds per kg
 * (`Fine.make`); it is drawn wherever it stands and no ice lies over it, in shallow liquid too. */
export function fineCover(f: Fine): CoverField | null {
  const w = f.world, n = f.across * f.down;
  if (!['t_ground', 'rain', 'ice'].every(name => w.index.has(name))) return null;
  const ice = decode(w, 0, 'ice');
  const kg = new Float32Array(SLOTS * n), wood = new Float32Array(SLOTS * n), leaf = new Float32Array(SLOTS * n);
  for (let k = 0; k < SLOTS; k++) for (let i = 0; i < n; i++) {
    const c = f.cover[k][i], make = 2 * (SLOTS * f.cell[i] + k);
    kg[k * n + i] = c; wood[k * n + i] = c * f.make[make]; leaf[k * n + i] = c * f.make[make + 1];
  }
  const depth = Float32Array.from(f.ground, (g, i) => (Number.isNaN(f.water[i]) ? 0 : Math.max(f.water[i] - g, 0)));
  const open = Uint8Array.from(ice, v => (v < SNOW_M ? 1 : 0));
  return coverOf({ w: f.across, h: f.down, stands: SLOTS, kg, wood, leaf, t: decode(w, 0, 't_ground'), rain: decode(w, 0, 'rain'), depth, open });
}
