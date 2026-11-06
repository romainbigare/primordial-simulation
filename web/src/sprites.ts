// Sprites: a small drawing of one thing, made only from its numbers and a seed (handover 10.4). One style for all: blocks
// seen from one axonometric angle, lit from the north-west (top light, the face on the left mid, the face on the right
// dark), each sprite standing on its anchor, the point of the ground it is drawn at.

export type Rgb = [number, number, number];
/** A filled polygon or a stroked line, its points in pixels from the anchor (x right, y down). */
export interface Shape { pts: number[]; fill?: string; line?: string; width?: number }
/** A drawing: its shapes in drawing order, and its box around the anchor [x0, y0, x1, y1] in pixels. */
export interface Sprite { shapes: Shape[]; box: [number, number, number, number] }

/** What an animal's look is made of: a member's mass kg, strength 0..1, colour (reflectance 0..1), coat 0..1, the
 * temperatures it keeps warm below and cools above (K), what it takes (tape::DIET_* bits), its brain's hidden units,
 * whether adult, and its lineage (its line of descent). */
export interface AnimalLook { mass: number; strength: number; colour: Rgb; coat: number; keepT: number; coolT: number; diet: number; hidden: number; adult: boolean; lineage: number }
/** What a plant's look is made of: the cover's genes (LIGHT, LEAF, WALLS, STORE, FROST, MOIST, SEED, REPAIR, FIX) and its built cover, kg/m². */
export interface PlantLook { light: number; leaf: number; walls: number; store: number; frost: number; moist: number; seed: number; repair: number; fix: number; kg: number }
/** What an object's look is made of: 3 outer sizes m (longest first), hollowness, walls' colour (reflectance) and hardness, depth buried m, and whether its structure closes a space. */
export interface PartLook { size: [number, number, number]; hollow: number; colour: Rgb; hardness: number; buried: number; room: boolean }

/** Screen x and y of a unit along u (the thing's length) and v (its width), the 30° axonometric angle. */
const ISO_X = 0.87, ISO_Y = 0.5;
/** Shade of the top, the face on the left and the face on the right (light from the north-west). */
const LIT = [1.12, 0.86, 0.64];
/** The cover's frost gene (K) over which a plant goes from tender to frost-hardy (bluer, and a tree pointed), and the
 * fixing gene (log10) above which it fixes (a mark at its foot). */
const TENDER_K = 272, HARDY_K = 258, FIX_ON = -10;
/** The cover's moisture gene (log10 of the wetness it is fully active at) over which a plant goes from drought-hardy
 * (greyer, small leaves) to one that needs wet ground (lush). */
const DRY_MOIST = -1, WET_MOIST = -0.1;
/** Diet bits (tape::DIET_*): it takes from the cover, bites bodies, takes remains. */
export const DIET_COVER = 1, DIET_BODIES = 2, DIET_REMAINS = 4;
/** The temperatures over which an animal goes from one that keeps warm only in deep cold to one that keeps warm early
 * (rounder, Bergmann's rule), and from one that cools only when hot to one that cools early (bigger ears and longer legs
 * to shed heat, Allen's rule), K. */
const KEEP_LOW = 250, KEEP_HIGH = 290, COOL_HIGH = 325, COOL_LOW = 300;
/** Hidden units of a brain at which an animal's head is drawn at its usual size. */
const BRAIN_UNITS = 12;
/** An animal lighter than this (kg) walks on six legs; one with less strength than this has none (it cannot move: laws.md 3.5). */
const SIX_LEGS_KG = 0.03, MOVES = 0.05;
/** The hardness from which an object's walls read as stone. */
const STONE = 0.3;

/** A seeded stream of numbers in [0, 1) (mulberry32). */
export function seeded(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** A reflectance (linear, 0..1) as a display colour (sRGB, 0..255). */
export const display = (c: Rgb): Rgb => c.map(v => 255 * Math.pow(Math.min(Math.max(v, 0), 1), 1 / 2.2)) as Rgb;
const tone = (c: Rgb, f: number) => `rgb(${c.map(v => Math.round(Math.min(255, Math.max(0, v * f)))).join(',')})`;
/** A colour from hue (degrees), saturation and lightness (0..1). */
function hsl(h: number, s: number, l: number): Rgb {
  const k = (n: number) => (n + h / 30) % 12, a = s * Math.min(l, 1 - l);
  return [0, 8, 4].map(n => 255 * (l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1)))) as Rgb;
}

/** Draws into a sprite in units of `k` pixels, mirrored if `flip`. */
class Pen {
  shapes: Shape[] = [];
  constructor(private k: number, private flip = false) {}
  private at(u: number, v: number, z: number): [number, number] {
    const x = (u - v) * ISO_X * this.k;
    return [this.flip ? -x : x, ((u + v) * ISO_Y - z) * this.k];
  }
  poly(corners: [number, number, number][], fill: string): void {
    this.shapes.push({ pts: corners.flatMap(c => this.at(...c)), fill });
  }
  line(a: [number, number, number], b: [number, number, number], line: string, width: number): void {
    this.shapes.push({ pts: [...this.at(...a), ...this.at(...b)], line, width: width * this.k });
  }
  /** A block from (u, v, z) of size du x dv x dz, its three seen faces shaded. */
  box(u: number, v: number, z: number, du: number, dv: number, dz: number, c: Rgb): void {
    const [u1, v1, z1] = [u + du, v + dv, z + dz];
    const [left, right] = this.flip ? [LIT[2], LIT[1]] : [LIT[1], LIT[2]];
    this.poly([[u, v1, z], [u1, v1, z], [u1, v1, z1], [u, v1, z1]], tone(c, left));
    this.poly([[u1, v, z], [u1, v1, z], [u1, v1, z1], [u1, v, z1]], tone(c, right));
    this.poly([[u, v, z1], [u1, v, z1], [u1, v1, z1], [u, v1, z1]], tone(c, LIT[0]));
  }
  /** A shadow on the ground around (0, 0), `r` units across. */
  shadow(r: number): void {
    const pts: [number, number, number][] = [];
    for (let a = 0; a < 2 * Math.PI; a += Math.PI / 6) pts.push([Math.cos(a) * r, Math.sin(a) * r, 0]);
    this.poly(pts, 'rgba(0,0,0,.2)');
  }
  done(): Sprite {
    let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
    for (const s of this.shapes) for (let i = 0; i < s.pts.length; i += 2) {
      x0 = Math.min(x0, s.pts[i]); x1 = Math.max(x1, s.pts[i]); y0 = Math.min(y0, s.pts[i + 1]); y1 = Math.max(y1, s.pts[i + 1]);
    }
    return { shapes: this.shapes, box: [x0, y0, x1, y1] };
  }
}

/** The length of an animal on screen, in units of the base size: by the log of a member's mass, so a mouse and a whale both show. */
export const animalLength = (kg: number) => Math.min(Math.max(1 + 0.3 * Math.log10(Math.max(kg, 1e-6)), 0.35), 2.6);

/** 0 at `a`, 1 at `b` (either order), smooth between. */
const ramp01 = (a: number, b: number, x: number) => { const t = Math.min(Math.max((x - a) / (b - a), 0), 1); return t * t * (3 - 2 * t); };

/**
 * The display colour of an animal: its own (the reflectance of what it is made of), its departure from grey drawn
 * three times as strong so that small differences of make-up show, turned a little in hue by its lineage, so the members
 * of one line of descent share a tint and two lines that look alike can be told apart.
 */
export function animalColour(a: AnimalLook): Rgb {
  const c = display(a.colour), grey = (c[0] + c[1] + c[2]) / 3;
  const [r, g, b] = c.map(v => grey + (v - grey) * 3);
  const turn = ((Math.imul(a.lineage + 1, 2654435761) >>> 0) / 4294967296 - 0.5) * 0.9; // radians, at most ±0.45
  // A turn about the grey axis (the hue), which keeps the lightness.
  const cos = Math.cos(turn), k = (1 - cos) / 3, q = Math.sqrt(1 / 3) * Math.sin(turn);
  const out = [r * (cos + k) + g * (k - q) + b * (k + q), r * (k + q) + g * (cos + k) + b * (k - q), r * (k - q) + g * (k + q) + b * (cos + k)];
  return out.map(v => Math.min(Math.max(v, 0), 255)) as Rgb;
}

/**
 * An animal, from its own numbers: its length from a member's mass (`animalLength`); its bulk and legs from its
 * strength (none without the strength to move); a rounder body and shorter legs the earlier it keeps warm (Bergmann's
 * rule), bigger ears and longer legs the earlier it cools (Allen's rule); fur as thick as its coat; its head by its brain;
 * its mouth by what it takes: fangs if it bites bodies (bigger the stronger), a broad low muzzle if it grazes, a long
 * snout low to the ground if it takes remains; smaller with a bigger head when young; its colour `animalColour`. The
 * seed (from its id and member) varies its size, tone and stance a little, so the members of a herd differ but look of
 * one kind. `unit` is the pixels of a base length.
 */
export function animal(a: AnimalLook, seed: number, unit: number, flip: boolean): Sprite {
  const r = seeded(seed), jit = (w: number) => 1 + (r() * 2 - 1) * w;
  const L = animalLength(a.mass) * (a.adult ? 1 : 0.62) * jit(0.08), b = a.strength;
  const warm = ramp01(KEEP_LOW, KEEP_HIGH, a.keepT), cool = ramp01(COOL_HIGH, COOL_LOW, a.coolT);
  const bites = (a.diet & DIET_BODIES) !== 0, grazes = !bites && (a.diet & DIET_COVER) !== 0, scavenges = !bites && !grazes && (a.diet & DIET_REMAINS) !== 0;
  const p = new Pen(unit / 10, flip);
  const c = animalColour(a).map(v => v * jit(0.07)) as Rgb;
  const len = 10 * L * (1 - 0.12 * warm), wide = len * (0.3 + 0.4 * b + 0.12 * warm), high = len * (0.26 + 0.26 * b + 0.14 * warm);
  const walks = b >= MOVES, legs = !walks ? 0 : a.mass < SIX_LEGS_KG ? 6 : 4;
  const legLen = walks ? len * (0.12 + 0.36 * b) * (1 + 0.4 * cool - 0.3 * warm) * jit(0.1) : 0, legW = wide * (0.18 + 0.16 * b);
  const head = len * (a.adult ? 0.34 : 0.46) * (0.8 + 0.4 * Math.min(a.hidden / BRAIN_UNITS, 1.5) / 1.5), z0 = legLen;
  p.shadow(len * 0.55);
  const leg = (k: number, side: number) => {
    const u = -len / 2 + legW / 2 + (k / Math.max(legs / 2 - 1, 1)) * (len - 2 * legW) + (r() - 0.5) * legW * 0.6;
    p.box(u - legW / 2, side < 0 ? -wide / 2 : wide / 2 - legW, 0, legW, legW, legLen + 0.5, c.map(v => v * 0.8) as Rgb);
  };
  for (let k = 0; k < legs / 2; k++) leg(k, -1);
  p.box(-len / 2 - len * 0.18, -wide * 0.08, z0 + high * 0.55, len * 0.2, wide * 0.16, high * 0.18, c); // tail
  p.box(-len / 2, -wide / 2, z0, len, wide, high, c);
  if (a.coat >= 0.2) fur(p, r, a.coat, len, wide, z0 + high, c);
  else p.line([-len * 0.3, -wide * 0.2, z0 + high], [len * 0.25, -wide * 0.2, z0 + high], 'rgba(255,255,255,.55)', wide * 0.08);
  for (let k = 0; k < legs / 2; k++) leg(k, 1);
  // The head: level with the back, low for a grazer, lower still with a long snout for one that takes remains.
  const hu = len / 2 - head * 0.35, hz = z0 + high * (grazes ? 0.15 : scavenges ? 0.05 : 0.55);
  const snout = scavenges ? head * 0.7 : grazes ? head * 0.35 : 0;
  p.box(hu, -head / 2, hz, head, head, head, c);
  if (snout > 0) p.box(hu + head, -head * (grazes ? 0.36 : 0.2), hz, snout, head * (grazes ? 0.72 : 0.4), head * (grazes ? 0.5 : 0.35), c.map(v => v * 0.92) as Rgb);
  const ear = head * (0.22 + 0.5 * cool) * (bites ? 1.1 : 1);
  for (const v of [-head * 0.4, head * 0.15]) p.poly([[hu + head * 0.2, v, hz + head], [hu + head * 0.2, v + head * 0.25, hz + head], [hu + head * (bites ? 0.35 : 0.1), v + head * 0.12, hz + head + ear]], tone(c, 0.85));
  p.box(hu + head * 0.65, -head * 0.38, hz + head * 0.68, head * 0.12, head * 0.12, head * 0.12, [20, 16, 12]); // eye
  const front = hu + head + snout;
  const mouth: [number, number, number] = [front, -head * 0.35, hz + head * 0.22], end: [number, number, number] = [front, head * 0.35, hz + head * 0.22];
  p.line(mouth, end, bites ? '#7a1a14' : 'rgba(30,20,15,.8)', head * 0.07);
  if (bites) {
    const f = head * (0.18 + 0.3 * b);
    for (const v of [-head * 0.28, head * 0.12]) p.poly([[front, v, hz + head * 0.22], [front, v + f * 0.6, hz + head * 0.22], [front, v + f * 0.3, hz + head * 0.22 - f]], '#f4efe4');
  }
  return p.done();
}

/** Fur: short strokes over the top of a body of `len` x `wide` at height `z`, as many and as long as the coat is thick. */
function fur(p: Pen, r: () => number, coat: number, len: number, wide: number, z: number, c: Rgb): void {
  const n = Math.round(6 + 24 * coat), hair = len * 0.05 * (0.4 + 1.6 * coat);
  for (let k = 0; k < n; k++) {
    const u = (r() - 0.5) * len * 0.9, v = (r() - 0.5) * wide * 0.9, light = r() < 0.5;
    p.line([u, v, z], [u - hair * 0.6, v + hair * 0.4, z + hair], tone(c, light ? 1.35 : 0.7), len * 0.018);
  }
  for (let k = 0; k < n / 2; k++) { // a fringe along the near side
    const u = (r() - 0.5) * len * 0.9;
    p.line([u, wide / 2, z - len * 0.05], [u - hair * 0.4, wide / 2 + hair * 0.3, z - len * 0.05 - hair * 1.4], tone(c, 0.75), len * 0.02);
  }
}

/** The look of a slot of cover from its 9 genes in their order (pcore::cover) and its built cover, kg/m². */
export const plantLookOf = (g: number[], kg: number): PlantLook => ({ light: g[0], leaf: g[1], walls: g[2], store: g[3], frost: g[4], moist: g[5], seed: g[6], repair: g[7], fix: g[8], kg });
/** Woodiness 0..1 from the WALLS gene (log10 kg of walls per kg of matter): grass below 0.2, a shrub below 0.5, a tree above. */
export const woodiness = (walls: number) => Math.min(Math.max((walls + 0.5) / 2, 0), 1);
/**
 * The foliage colour of a plant from its genes: its hue by how much light it can use per capture area (LIGHT less
 * LEAF: a richer, bluer green the more), bluer the more frost-hardy (FROST), greyer and paler the less wet ground it
 * needs (MOIST, as drought-hardy leaves are small and grey), darker the woodier.
 */
export function plantColour(g: PlantLook, wood: number): Rgb {
  const hardy = ramp01(TENDER_K, HARDY_K, g.frost), dry = ramp01(WET_MOIST, DRY_MOIST, g.moist);
  const use = g.light - g.leaf; // log10 mol per m² of capture area per s
  const hue = 92 + 14 * Math.tanh((use + 5.4) * 2) + 50 * hardy - 22 * dry;
  return hsl(hue, 0.5 - 0.22 * dry - 0.12 * hardy, 0.42 - 0.12 * wood + 0.08 * dry);
}

/**
 * A plant of the cover: grass tufts, a shrub or a tree by its woodiness (`woodiness`); a fuller, bigger crown the more
 * leaf it has (LEAF); pointed and tiered if frost-hardy (FROST); a thicker trunk the longer it lives (REPAIR); a swollen
 * base the more fuel it keeps (STORE); seed heads as many as its share to seeds (SEED); its colour `plantColour`; a mark
 * at its foot if it fixes. Bigger where the cover is thicker. `unit` is the pixels of a base length.
 */
export function plant(g: PlantLook, seed: number, unit: number): Sprite {
  const r = seeded(seed), wood = woodiness(g.walls), leaf = 0.5 + 0.5 * Math.tanh(g.leaf);
  const grow = 0.7 + 0.3 * Math.tanh(g.kg / 2), p = new Pen((unit / 10) * grow * (0.85 + 0.3 * r()));
  const c = plantColour(g, wood).map(v => v * (0.9 + 0.2 * r())) as Rgb;
  const hardy = ramp01(TENDER_K, HARDY_K, g.frost), store = ramp01(-2, 0, g.store), heads = Math.round(8 * Math.min(g.seed, 0.5));
  const seedHead = (u: number, v: number, z: number) => p.box(u - 0.3, v - 0.3, z, 0.6, 0.6, 0.6, [236, 214, 120]);
  if (wood < 0.2) {
    if (store > 0.1) p.box(-1.2 * store, -1.2 * store, 0, 2.4 * store, 2.4 * store, 1.5 * store, c.map(v => v * 0.8) as Rgb);
    const blades = 5 + Math.round(4 * leaf);
    for (let k = 0; k < blades; k++) {
      const u = (r() - 0.5) * 4, v = (r() - 0.5) * 4, h = 3 + 4 * r() + 4 * leaf;
      p.poly([[u - 0.5, v, 0], [u + 0.5, v, 0], [u + (r() - 0.5) * 2, v, h]], tone(c, 0.8 + 0.5 * r()));
      if (k < heads) seedHead(u, v, h);
    }
  } else if (wood < 0.5) {
    p.shadow(4);
    const n = 2 + Math.round(3 * leaf);
    for (let k = 0; k < n; k++) {
      const s = 2.5 + 2 * r();
      p.box((r() - 0.5) * 4 - s / 2, (r() - 0.5) * 4 - s / 2, k * 0.6, s, s, s * 0.8, c);
    }
    for (let k = 0; k < heads; k++) seedHead((r() - 0.5) * 4, (r() - 0.5) * 4, 2 + 2 * r());
  } else {
    const h = 8 + 22 * (wood - 0.5), trunk = (1 + 1.6 * wood) * (0.8 + 0.5 * ramp01(-1, 1, g.repair)), crown = (5 + 6 * leaf) * (0.6 + wood);
    p.shadow(crown * 0.5);
    p.box(-trunk / 2, -trunk / 2, 0, trunk, trunk, h * 0.55, [110, 78, 48]);
    if (hardy > 0.5) for (let k = 0; k < 4; k++) {
      const s = crown * (1 - k * 0.24) * (0.7 + 0.3 * hardy);
      p.box(-s / 2, -s / 2, h * 0.3 + k * crown * 0.36, s, s, crown * 0.36, c);
    }
    else {
      p.box(-crown / 2, -crown / 2, h * 0.45, crown, crown, crown * 0.8, c);
      for (let k = 0; k < 1 + Math.round(3 * leaf); k++) {
        const s = crown * (0.35 + 0.2 * r());
        p.box((r() - 0.5) * crown - s / 2, (r() - 0.5) * crown - s / 2, h * 0.45 + crown * (0.5 + 0.4 * r()), s, s, s * 0.8, c.map(v => v * 1.08) as Rgb);
      }
    }
  }
  if (g.fix > FIX_ON) p.box(0.6, 0.6, 0, 0.7, 0.7, 0.7, [226, 120, 40]);
  return p.done();
}

/** The on-screen length of an object's longest side, in units of the base size: by the log of its length in metres. */
const partLength = (m: number) => Math.min(Math.max(0.6 + 0.3 * Math.log10(Math.max(m, 1e-4) / 0.05), 0.3), 2.5);

/**
 * An object (a part): a block of its three sizes (their ratios softened so a rod still reads long and a plate flat),
 * open on top if hollow, with a doorway if its structure closes a space, a speckled boulder if its walls
 * are hard, sunk into the ground as deep as it is buried, in its walls' colour. `unit` is the pixels of a base length.
 */
export function object(o: PartLook, seed: number, unit: number): Sprite {
  const r = seeded(seed), p = new Pen(unit / 10), c = display(o.colour);
  const [a, b, t] = o.size, L = 10 * partLength(a);
  const du = L, dv = Math.max(L * Math.pow(b / a, 0.6), 0.6), dzFull = Math.max(L * Math.pow(t / a, 0.6), 0.5);
  const dz = dzFull * Math.max(0.2, 1 - o.buried / Math.max(t, 1e-6));
  const [u, v] = [-du / 2, -dv / 2];
  p.shadow(Math.max(du, dv) * 0.6);
  if (o.buried > 0) p.poly([[u - 1, v - 1, 0], [u + du + 1, v - 1, 0], [u + du + 1, v + dv + 1, 0], [u - 1, v + dv + 1, 0]], 'rgba(92,70,46,.55)');
  const hollow = o.hollow > 0.3, stone = o.hardness >= STONE;
  if (stone && !hollow && !o.room) {
    // A boulder: a lower block and a smaller one set on it off centre, so its outline is not a box's.
    const [iu, iv] = [du * (0.12 + 0.2 * r()), dv * (0.1 + 0.2 * r())], [su, sv] = [du * (0.55 + 0.2 * r()), dv * (0.55 + 0.2 * r())];
    p.box(u, v, 0, du, dv, dz * 0.6, c);
    p.box(u + Math.min(iu, du - su), v + Math.min(iv, dv - sv), dz * 0.6, su, sv, dz * 0.4, c.map(x => x * 0.94) as Rgb);
    for (let k = 0; k < 4 + Math.round(du); k++) { // speckles of its grains on its faces
      const x = u + r() * du, s = 0.25 + 0.3 * r();
      p.poly([[x, v + dv, dz * 0.1], [x + s, v + dv, dz * 0.1], [x + s, v + dv, dz * 0.1 + s], [x, v + dv, dz * 0.1 + s]], tone(c, r() < 0.5 ? 0.55 : 1.15));
    }
  } else p.box(u, v, 0, du, dv, dz, c);
  if (hollow) {
    const e = Math.min(du, dv) * (1 - Math.sqrt(o.hollow)) * 0.5 + 0.3;
    p.poly([[u + e, v + e, dz], [u + du - e, v + e, dz], [u + du - e, v + dv - e, dz], [u + e, v + dv - e, dz]], tone(c, 0.45));
  }
  if (o.room) p.poly([[u + du * 0.35, v + dv, 0], [u + du * 0.65, v + dv, 0], [u + du * 0.65, v + dv, dz * 0.6], [u + du * 0.35, v + dv, dz * 0.6]], tone(c, 0.3));
  return p.done();
}

/** Draw sprite `s` with its anchor at (x, y). */
export function paintSprite(ctx: CanvasRenderingContext2D, s: Sprite, x: number, y: number): void {
  for (const sh of s.shapes) {
    ctx.beginPath();
    ctx.moveTo(x + sh.pts[0], y + sh.pts[1]);
    for (let i = 2; i < sh.pts.length; i += 2) ctx.lineTo(x + sh.pts[i], y + sh.pts[i + 1]);
    if (sh.fill) { ctx.closePath(); ctx.fillStyle = sh.fill; ctx.fill(); }
    if (sh.line) { ctx.strokeStyle = sh.line; ctx.lineWidth = sh.width ?? 1; ctx.stroke(); }
  }
}
