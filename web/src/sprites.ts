// Sprites: the shape of one individual or object at its real size, made only from its numbers (handover 10.4), in one
// of two styles: seen from above, or angled (blocks seen from one axonometric angle, lit from the north-west, standing
// on their anchor). Sizes are in metres; a sprite is made for a scale in pixels a metre. The colours are muted, so a
// sprite sits on the map.

export type Rgb = [number, number, number];
/** How life and objects are drawn: from above, or angled. */
export type Style = 'above' | 'angled';
/** A sprite: its filled paths in drawing order, in pixels about its anchor (x along its heading from above, x right
 * and y down when angled), its shadow (drawn first, unturned), and its radius about the anchor, px. */
export interface Sprite { paths: { path: Path2D; fill: string }[]; shadow: Path2D | null; r: number }

/** What an animal's look is made of: a member's mass kg, strength 0..1, colour (reflectance 0..1), the temperatures it
 * keeps warm below and cools above (K), what it takes (tape::DIET_* bits), whether adult, and its lineage. */
export interface AnimalLook { mass: number; strength: number; colour: Rgb; keepT: number; coolT: number; diet: number; adult: boolean; lineage: number }
/** What an object's look is made of: 3 outer sizes m (longest first), hollowness, walls' colour (reflectance) and
 * hardness, depth buried m, and whether its structure closes a space. */
export interface PartLook { size: [number, number, number]; hollow: number; colour: Rgb; hardness: number; buried: number; room: boolean }

/** Screen x and y of a metre along u (the thing's length) and v (its width), the 30° axonometric angle. */
const ISO_X = 0.87, ISO_Y = 0.5;
/** Shade of the top, the face on the left and the face on the right (light from the north-west), soft. */
const LIT = [1.06, 0.9, 0.76];
/** A body's density, kg/m³ (that of water, which bodies are mostly made of), and its length over the cube root of its
 * volume (a body about 1.5 times as long as it is wide and high). */
const BODY_DENSITY = 1000, BODY_ASPECT = 1.5;
/** A young member's length over an adult's. */
const YOUNG = 0.62;
/** Diet bits (tape::DIET_*): it takes from the cover, bites bodies, takes remains. */
export const DIET_COVER = 1, DIET_BODIES = 2, DIET_REMAINS = 4;
/** The temperatures over which an animal goes from one that keeps warm only in deep cold to one that keeps warm early
 * (rounder, Bergmann's rule), and from one that cools only when hot to one that cools early (bigger ears and longer legs
 * to shed heat, Allen's rule), K. */
const KEEP_LOW = 250, KEEP_HIGH = 290, COOL_HIGH = 325, COOL_LOW = 300;
/** An animal with less strength than this has no legs (it cannot move: laws.md 3.5). */
const MOVES = 0.05;
/** The hardness from which an object's walls read as stone. */
const STONE = 0.3;
/** How much of its own colour's departure from grey a sprite keeps, and the lightness it is held between (0..255). */
const MUTE = 0.7, LIGHT_LOW = 60, LIGHT_HIGH = 190;
/** The darkness of a shadow. */
const SHADOW = 'rgba(0,0,0,.2)';
/** The width of a sprite's rim, px. */
const RIM_PX = 1;

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

/** A seed from an id and a number (a member, a plant, a dot). */
export const seedOf = (id: bigint, k: number) => (Number(id & 0xffffffffn) * 2654435761 ^ Number(id >> 32n) * 40503 ^ (k + 1) * 2246822519) >>> 0;

/** A reflectance (linear, 0..1) as a display colour (sRGB, 0..255). */
export const display = (c: Rgb): Rgb => c.map(v => 255 * Math.pow(Math.min(Math.max(v, 0), 1), 1 / 2.2)) as Rgb;
export const tone = (c: Rgb, f: number, a = 1) => `rgba(${c.map(v => Math.round(Math.min(255, Math.max(0, v * f)))).join(',')},${a})`;
/** A colour from hue (degrees), saturation and lightness (0..1). */
export function hsl(h: number, s: number, l: number): Rgb {
  const k = (n: number) => (n + h / 30) % 12, a = s * Math.min(l, 1 - l);
  return [0, 8, 4].map(n => 255 * (l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1)))) as Rgb;
}
/** 0 at `a`, 1 at `b` (either order), smooth between. */
const ramp01 = (a: number, b: number, x: number) => { const t = Math.min(Math.max((x - a) / (b - a), 0), 1); return t * t * (3 - 2 * t); };

/** A display colour muted: `MUTE` of its departure from grey, its lightness held between `LIGHT_LOW` and `LIGHT_HIGH`. */
export function muted(c: Rgb): Rgb {
  const grey = (c[0] + c[1] + c[2]) / 3, held = Math.min(Math.max(grey, LIGHT_LOW), LIGHT_HIGH);
  return c.map(v => held + (v - grey) * MUTE) as Rgb;
}

/** The length of a member, m: a body of `BODY_DENSITY` as long as `BODY_ASPECT` times the cube root of its volume, a
 * young one `YOUNG` of that. */
export const bodyLength = (kg: number, adult: boolean) => BODY_ASPECT * Math.cbrt(Math.max(kg, 0) / BODY_DENSITY) * (adult ? 1 : YOUNG);

/**
 * The colour of an animal: its own (the reflectance of what it is made of) muted, turned a little in hue by its lineage,
 * so the members of one line of descent share a tint and two lines that look alike can be told apart.
 */
export function animalColour(a: AnimalLook): Rgb {
  const [r, g, b] = muted(display(a.colour));
  const turn = ((Math.imul(a.lineage + 1, 2654435761) >>> 0) / 4294967296 - 0.5) * 0.5; // radians, at most ±0.25
  // A turn about the grey axis (the hue), which keeps the lightness.
  const cos = Math.cos(turn), k = (1 - cos) / 3, q = Math.sqrt(1 / 3) * Math.sin(turn);
  const out = [r * (cos + k) + g * (k - q) + b * (k + q), r * (k + q) + g * (cos + k) + b * (k - q), r * (k - q) + g * (k + q) + b * (cos + k)];
  return out.map(v => Math.min(Math.max(v, 0), 255)) as Rgb;
}

/** The proportions of an animal from its numbers, m: its length from a member's mass (`bodyLength`); wider and higher
 * the stronger; rounder the earlier it keeps warm (Bergmann's rule); longer legs and bigger ears the earlier it cools
 * (Allen's rule); no legs without the strength to move; its head bigger when young; a snout low and long if it takes
 * remains, broad if it grazes. */
function animalForm(a: AnimalLook) {
  const L = bodyLength(a.mass, a.adult), b = a.strength;
  const warm = ramp01(KEEP_LOW, KEEP_HIGH, a.keepT), cool = ramp01(COOL_HIGH, COOL_LOW, a.coolT);
  const bites = (a.diet & DIET_BODIES) !== 0, grazes = !bites && (a.diet & DIET_COVER) !== 0, scavenges = !bites && !grazes && (a.diet & DIET_REMAINS) !== 0;
  const len = L * (1 - 0.12 * warm), wide = len * (0.28 + 0.24 * b + 0.1 * warm), high = len * (0.26 + 0.26 * b + 0.14 * warm);
  const head = len * (a.adult ? 0.3 : 0.4), legLen = b >= MOVES ? len * (0.12 + 0.36 * b) * (1 + 0.4 * cool - 0.3 * warm) : 0;
  return { L, len, wide, high, head, legLen, legW: wide * (0.18 + 0.16 * b), ear: head * (0.2 + 0.4 * cool), snout: scavenges ? head * 0.7 : grazes ? head * 0.35 : 0, grazes, scavenges };
}

/** An ellipse about (x, y) of radii rx, ry, turned by `a`, onto `p`. */
const oval = (p: Path2D, x: number, y: number, rx: number, ry: number, a = 0) => { p.moveTo(x + rx * Math.cos(a), y + rx * Math.sin(a)); p.ellipse(x, y, rx, ry, a, 0, 2 * Math.PI); };

/** An animal of look `a` at `ppm` pixels a metre in `style`: from above, facing along x (its body, head, snout and
 * ears, and a faint shadow); angled, a block body on legs with its head, facing right or, if `flip`, left. */
export function animal(a: AnimalLook, ppm: number, style: Style, flip = false): Sprite {
  const f = animalForm(a), c = animalColour(a);
  if (style === 'angled') return angledAnimal(f, c, ppm, flip);
  const k = ppm, body = new Path2D(), head = new Path2D(), back = new Path2D(), shadow = new Path2D();
  oval(shadow, 0, 0, (f.len / 2) * k, (f.wide / 2) * k * 1.1);
  oval(body, -f.len * 0.08 * k, 0, f.len * 0.4 * k, (f.wide / 2) * k);
  oval(body, -f.len * 0.5 * k, 0, f.len * 0.1 * k, f.wide * 0.08 * k); // tail
  const hx = (f.len * 0.32 + f.head * 0.3) * k, hr = (f.head / 2) * k;
  oval(head, hx, 0, hr, hr * 0.8);
  if (f.snout > 0) oval(head, hx + hr + f.snout * 0.4 * k, 0, f.snout * 0.5 * k, hr * (f.grazes ? 0.6 : 0.35));
  if (f.ear > f.head * 0.3) for (const s of [-1, 1]) oval(head, hx - hr * 0.3, s * hr * 0.8, f.ear * 0.5 * k, f.ear * 0.3 * k);
  oval(back, -f.len * 0.12 * k, 0, f.len * 0.26 * k, f.wide * 0.16 * k); // the lit line of its back
  return { paths: [{ path: body, fill: tone(c, 1) }, { path: back, fill: tone(c, 1.12) }, { path: head, fill: tone(c, 0.92) }], shadow, r: (f.len / 2 + f.snout) * k };
}

/** Draws blocks into a sprite at `k` pixels a metre, mirrored if `flip`. */
class Pen {
  paths: { path: Path2D; fill: string }[] = [];
  constructor(private k: number, private flip = false) {}
  private at(u: number, v: number, z: number): [number, number] {
    const x = (u - v) * ISO_X * this.k;
    return [this.flip ? -x : x, ((u + v) * ISO_Y - z) * this.k];
  }
  poly(corners: [number, number, number][], fill: string): void {
    const p = new Path2D();
    corners.forEach((c, i) => (i ? p.lineTo(...this.at(...c)) : p.moveTo(...this.at(...c))));
    p.closePath();
    this.paths.push({ path: p, fill });
  }
  /** A block from (u, v, z) of size du x dv x dz, its three seen faces shaded. */
  box(u: number, v: number, z: number, du: number, dv: number, dz: number, c: Rgb): void {
    const [u1, v1, z1] = [u + du, v + dv, z + dz];
    const [left, right] = this.flip ? [LIT[2], LIT[1]] : [LIT[1], LIT[2]];
    this.poly([[u, v1, z], [u1, v1, z], [u1, v1, z1], [u, v1, z1]], tone(c, left));
    this.poly([[u1, v, z], [u1, v1, z], [u1, v1, z1], [u1, v, z1]], tone(c, right));
    this.poly([[u, v, z1], [u1, v, z1], [u1, v1, z1], [u, v1, z1]], tone(c, LIT[0]));
  }
  /** A shadow on the ground about the anchor, `ru` by `rv` m. */
  shadow(ru: number, rv: number): Path2D {
    const p = new Path2D();
    for (let a = 0; a < 2 * Math.PI; a += Math.PI / 8) { const [x, y] = this.at(Math.cos(a) * ru, Math.sin(a) * rv, 0); a ? p.lineTo(x, y) : p.moveTo(x, y); }
    p.closePath();
    return p;
  }
  done(shadow: Path2D | null, r: number): Sprite { return { paths: this.paths, shadow, r: r * this.k }; }
}

/** An angled animal of form `f` and colour `c`: its legs, its body, its head with its ears and snout. */
function angledAnimal(f: ReturnType<typeof animalForm>, c: Rgb, ppm: number, flip: boolean): Sprite {
  const p = new Pen(ppm, flip), { len, wide, high, head, legLen, legW } = f, z0 = legLen, legC = c.map(v => v * 0.85) as Rgb;
  const shadow = p.shadow(len * 0.55, wide * 0.7);
  const legs = (side: number) => { if (legLen > 0) for (const u of [-len / 2, len / 2 - legW]) p.box(u, side < 0 ? -wide / 2 : wide / 2 - legW, 0, legW, legW, legLen, legC); };
  legs(-1);
  p.box(-len / 2, -wide / 2, z0, len, wide, high, c);
  legs(1);
  // The head: level with the back, low for a grazer, lower still with a long snout for one that takes remains.
  const hu = len / 2 - head * 0.35, hz = z0 + high * (f.grazes ? 0.15 : f.scavenges ? 0.05 : 0.55);
  p.box(hu, -head / 2, hz, head, head, head, c);
  if (f.snout > 0) p.box(hu + head, -head * (f.grazes ? 0.36 : 0.2), hz, f.snout, head * (f.grazes ? 0.72 : 0.4), head * 0.4, c.map(v => v * 0.94) as Rgb);
  for (const v of [-head * 0.4, head * 0.15]) p.poly([[hu + head * 0.2, v, hz + head], [hu + head * 0.2, v + head * 0.25, hz + head], [hu + head * 0.1, v + head * 0.12, hz + head + f.ear]], tone(c, 0.9));
  return p.done(shadow, len / 2 + f.snout + legLen + high);
}

/**
 * An object (a part) of look `o` at `ppm` pixels a metre, its longest side along x: from above, a
 * shape of its length and width (a rounded stone if its walls are hard), open (darker inside) if hollow, with a light
 * edge; angled, a block of its three sizes sunk as deep as it is buried, open on top if hollow, with a doorway if its
 * structure closes a space. Its colour is its walls' muted.
 */
export function object(o: PartLook, ppm: number, style: Style): Sprite {
  const c = muted(display(o.colour)), [a, b, t] = o.size, k = ppm, hollow = o.hollow > 0.3, stone = o.hardness >= STONE;
  if (style === 'angled') {
    const p = new Pen(k), dz = t * Math.max(0.2, 1 - o.buried / Math.max(t, 1e-6)), [u, v] = [-a / 2, -b / 2];
    const shadow = p.shadow(a * 0.6, b * 0.7);
    if (stone && !hollow && !o.room) {
      // A boulder: a lower block and a smaller one set on it, so its outline is not a box's.
      p.box(u, v, 0, a, b, dz * 0.6, c);
      p.box(u + a * 0.15, v + b * 0.15, dz * 0.6, a * 0.65, b * 0.65, dz * 0.4, c.map(x => x * 0.96) as Rgb);
    } else p.box(u, v, 0, a, b, dz, c);
    const e = Math.min(a, b) * (1 - Math.sqrt(o.hollow)) * 0.5;
    if (hollow) p.poly([[u + e, v + e, dz], [u + a - e, v + e, dz], [u + a - e, v + b - e, dz], [u + e, v + b - e, dz]], tone(c, 0.6));
    if (o.room) p.poly([[u + a * 0.35, v + b, 0], [u + a * 0.65, v + b, 0], [u + a * 0.65, v + b, dz * 0.6], [u + a * 0.35, v + b, dz * 0.6]], tone(c, 0.5));
    return p.done(shadow, Math.hypot(a, b) / 2 + dz);
  }
  const body = new Path2D(), inner = new Path2D(), edge = new Path2D(), shadow = new Path2D(), [ha, hb] = [(a / 2) * k, (b / 2) * k];
  oval(shadow, 0, 0, ha * 1.08, hb * 1.15);
  if (stone) oval(body, 0, 0, ha, hb);
  else body.rect(-ha, -hb, 2 * ha, 2 * hb);
  if (hollow) oval(inner, 0, 0, ha * Math.sqrt(o.hollow), hb * Math.sqrt(o.hollow));
  oval(edge, -Math.min(ha, hb) * 0.2, -Math.min(ha, hb) * 0.2, ha * 0.7, hb * 0.6); // the lit side of its top
  return { paths: [{ path: body, fill: tone(c, 0.95) }, { path: edge, fill: tone(c, 1.15, 0.6) }, { path: inner, fill: tone(c, 0.6) }], shadow, r: Math.max(ha, hb) };
}

/** Draw sprite `s` with its anchor at (x, y), turned by `turn` (from above), its shadow cast `lift` px to the south-east,
 * and if `rim` is given, edged by a line of that colour `RIM_PX` wide outside its outline (each path stroked twice as
 * wide before all are filled, so only the outer half of the outline's stroke shows). */
export function paintSprite(ctx: CanvasRenderingContext2D, s: Sprite, x: number, y: number, turn: number, lift: number, rim?: string): void {
  const m = ctx.getTransform();
  if (s.shadow) {
    ctx.translate(x + lift, y + lift); ctx.rotate(turn);
    ctx.fillStyle = SHADOW; ctx.fill(s.shadow);
    ctx.setTransform(m);
  }
  ctx.translate(x, y); ctx.rotate(turn);
  if (rim) {
    ctx.strokeStyle = rim; ctx.lineWidth = 2 * RIM_PX; ctx.lineJoin = 'round';
    for (const p of s.paths) ctx.stroke(p.path);
  }
  for (const p of s.paths) { ctx.fillStyle = p.fill; ctx.fill(p.path); }
  ctx.setTransform(m);
}
