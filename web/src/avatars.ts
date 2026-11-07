// Avatars: when the map is close, the bodies and parts of each cell are drawn one by one at their real size (handover
// 10.4, 10.5), from the list the core hands out (tape/src/things.rs). Each member of a node has its own place across
// its cell's real area, seeded by the node's id and the member's index, so it holds while the camera moves. A member
// whose length takes `SHAPE_PX` pixels or more is drawn as its shape (sprites.ts, from above or angled), a smaller one
// as a soft dot, and below a pixel its node as a stipple of its members per m² (stipple.ts), so a herd shows from afar.
// Hovering or clicking one shows its numbers (10.8); the plant cover is a texture (vegetation.ts). A lot of alike parts
// (laws.md 3.1) is drawn part by part, each at a place seeded by the lot's id and the part's index.
import { Rgb, Sprite, Style, AnimalLook, PartLook, animal, object, paintSprite, bodyLength, animalColour, woodiness, display, muted, seeded, seedOf, tone, DIET_COVER, DIET_BODIES, DIET_REMAINS } from './sprites';
import { Ground, rimOver, stipple } from './stipple';

interface Animal { id: bigint; members: number; mass: number; living: number; strength: number; colour: Rgb; copyMass: number; coat: number; keepT: number; coolT: number; root: number; lineage: number; diet: number; adult: boolean; hidden: number; hold: bigint; inside: bigint }
interface Slot { root: number; kg: number; leaf: number; genes: number[]; full: boolean }
interface Thing { id: bigint; count: number; size: [number, number, number]; hollow: number; mass: number; colour: Rgb; hardness: number; buried: number; held: bigint; root: bigint; space: number; closed: number }
interface Bond { a: bigint; b: bigint; strength: number }
export interface CellThings { index: number; box: [number, number, number, number]; area: number; bodies: Animal[]; slots: Slot[]; parts: Thing[]; bonds: Bond[] }

/** The cover's genes in their order (pcore::cover): their names and units for the inspect panel. */
const COVER_GENES: [string, string][] = [['light', 'log10 mol/kg/s'], ['leaf', 'log10 m²/kg'], ['walls', 'log10 kg/kg'], ['store', 'log10 kg/kg'], ['frost', 'K'], ['moist', 'log10'], ['seed', 'share'], ['repair', 'log10'], ['fix', 'log10 mol/kg/s']];
/** Pixels a map square takes from which the things of the window in view are asked for and drawn: a square of a few
 * pixels, so the herds show at every zoom that shows the map's squares as more than points. */
export const THINGS_PX = 8;
/** Pixels a member's length takes from which it is drawn as its shape, and from which as a soft dot (below, its node
 * is a stipple). */
const SHAPE_PX = 3, DOT_PX = 1;
/** A member this many pixels long or more is edged by a rim that stands out from the ground under it, so one animal
 * reads on any ground (fewer than `MOST_SHOWN` take that many pixels in a view). */
const RIM_FROM_PX = 8;
/** Members drawn as shapes or dots at most in a frame; past it, the same share of each node is drawn. */
const MOST_SHOWN = 20000;
/** How strong a soft dot is. */
const DOT_ALPHA = 0.85;
/** A part smaller than this, px, is not drawn; up to `SHAPE_PX` it is a dot. */
const PART_HIDE_PX = 1;
/** The parts of a lot drawn at most. */
const MOST_OF_LOT = 2000;
/** The pointer finds a thing within this many pixels of its edge. */
const REACH_PX = 3;
/** The share of the view's width and height the picture of the things reaches beyond each edge. */
const THINGS_MARGIN = 0.3;
/** A shadow is cast this share of a thing's radius to the south-east from above. */
const SHADOW_LIFT = 0.18;

/** Reads the list of things (tape::things). */
function readThings(data: Uint8Array): CellThings[] {
  const dv = new DataView(data.buffer, data.byteOffset, data.byteLength);
  let at = 0;
  const u8 = () => dv.getUint8(at++), u16 = () => { at += 2; return dv.getUint16(at - 2, true); };
  const u32 = () => { at += 4; return dv.getUint32(at - 4, true); }, f32 = () => { at += 4; return dv.getFloat32(at - 4, true); };
  const id = () => { const lo = u32(), hi = u32(); return (BigInt(hi) << 32n) | BigInt(lo); };
  const f32s = (n: number) => Array.from({ length: n }, f32);
  const cells: CellThings[] = [];
  for (let n = u32(); n > 0; n--) {
    const index = u32(), box = f32s(4) as [number, number, number, number], area = f32();
    const [nb, ns, np, nl] = [u16(), u16(), u16(), u16()];
    const bodies = Array.from({ length: nb }, (): Animal => {
      const i = id(), [members, mass, living, strength, r, g, b, copyMass, coat, keepT, coolT] = f32s(11);
      const root = u32(), lineage = u32(), diet = u8(), adult = u8() === 1, hidden = u8(); u8();
      return { id: i, members, mass, living, strength, colour: [r, g, b], copyMass, coat, keepT, coolT, root, lineage, diet, adult, hidden, hold: id(), inside: id() };
    });
    const slots = Array.from({ length: ns }, (): Slot => { const root = u32(), kg = f32(), leaf = f32(), genes = f32s(9), full = u8() === 1; at += 3; return { root, kg, leaf, genes, full }; });
    const parts = Array.from({ length: np }, (): Thing => {
      const i = id(), [count, l, w, t, hollow, mass, r, g, b, hardness, buried] = f32s(11);
      return { id: i, count, size: [l, w, t], hollow, mass, colour: [r, g, b], hardness, buried, held: id(), root: id(), space: f32(), closed: f32() };
    });
    const bonds = Array.from({ length: nl }, (): Bond => ({ a: id(), b: id(), strength: f32() }));
    cells.push({ index, box, area, bodies, slots, parts, bonds });
  }
  return cells;
}

/** The members of node `b` drawn (its count's whole part, and one more by chance of its fraction, seeded by its id). */
const shownOf = (b: Animal) => Math.floor(b.members) + (seeded(seedOf(b.id, -1))() < b.members % 1 ? 1 : 0);
/** The look of a node's members. */
const lookOf = (b: Animal): AnimalLook => ({ mass: b.mass, strength: b.strength, colour: b.colour, keepT: b.keepT, coolT: b.coolT, diet: b.diet, adult: b.adult, lineage: b.lineage });
/** The look of a part from its record: the one place that reads a part's fields for drawing. */
const partLook = (p: Thing): PartLook => ({ size: p.size, hollow: p.hollow, colour: p.colour, hardness: p.hardness, buried: p.buried, room: p.space > 0 && p.closed > 0.5 });

/** Members a block of a node's layout holds, about. */
const PER_BLOCK = 16;
/** A box in squares: west, north, east, south. */
type Box = [number, number, number, number];
/**
 * The layout of node `b`'s members over `box` (its cell, or the room of a structure it is inside): a grid of `g` x `g`
 * blocks, member m dealt to block m mod g², at a seeded place in it, heading near its node's (`herd`). So any first
 * members cover the box evenly, and the members in a window are found by their blocks.
 */
interface Layout { b: Animal; box: Box; g: number; shown: number; herd: number }
function layoutOf(b: Animal, box: Box): Layout {
  const shown = shownOf(b);
  return { b, box, g: Math.max(1, Math.ceil(Math.sqrt(shown / PER_BLOCK))), shown, herd: seeded(seedOf(b.id, -2))() * 2 * Math.PI };
}
/** The blocks of layout `l` that window `v` overlaps: their first and last columns and rows. */
function blocksIn(l: Layout, v: Box): Box {
  const [x0, y0, x1, y1] = l.box, g = l.g, col = (x: number) => Math.min(Math.max(Math.floor(((x - x0) / Math.max(x1 - x0, 1e-12)) * g), 0), g - 1);
  const row = (y: number) => Math.min(Math.max(Math.floor(((y - y0) / Math.max(y1 - y0, 1e-12)) * g), 0), g - 1);
  return v[2] < x0 || v[0] > x1 || v[3] < y0 || v[1] > y1 ? [0, 0, -1, -1] : [col(v[0]), row(v[1]), col(v[2]), row(v[3])];
}
/** The members of layout `l` in window `v` (squares), at most `per` a block: each with its place, squares, and heading. */
function eachIn(l: Layout, v: Box, per: number, fn: (m: number, x: number, y: number, turn: number) => void): void {
  const [c0, r0, c1, r1] = blocksIn(l, v), [x0, y0, x1, y1] = l.box, g = l.g, bw = (x1 - x0) / g, bh = (y1 - y0) / g;
  for (let row = r0; row <= r1; row++) for (let col = c0; col <= c1; col++) for (let j = 0; j < per; j++) {
    const m = row * g + col + j * g * g;
    if (m >= l.shown) break;
    const r = seeded(seedOf(l.b.id, m));
    fn(m, x0 + (col + r()) * bw, y0 + (row + r()) * bh, l.herd + (r() - 0.5) * 1.6);
  }
}
/** The share of layout `l`'s blocks in window `v`. */
const shareIn = (l: Layout, v: Box) => { const [c0, r0, c1, r1] = blocksIn(l, v); return (Math.max(c1 - c0 + 1, 0) * Math.max(r1 - r0 + 1, 0)) / (l.g * l.g); };
/** The parts of lot `p` drawn (its count's whole part, and one more by chance of its fraction, seeded by its id), at
 * most `MOST_OF_LOT`. */
const partsOf = (p: Thing) => Math.min(MOST_OF_LOT, Math.floor(p.count) + (seeded(seedOf(p.id, -1))() < p.count % 1 ? 1 : 0));
/** The place of part `m` of lot `p` lying alone in cell `c`, squares: the first at its structure's place, each other
 * one seeded by the lot's id and its index, so a window shows the same parts at the same places. */
function partAt(c: CellThings, p: Thing, m: number): [number, number] {
  if (m === 0) return structureAt(c, p.root);
  const r = seeded(seedOf(p.id, m)), [x0, y0, x1, y1] = c.box;
  return [x0 + (0.05 + 0.9 * r()) * (x1 - x0), y0 + (0.05 + 0.9 * r()) * (y1 - y0)];
}
/** A structure's place in its cell, squares: seeded by its root. */
function structureAt(c: CellThings, root: bigint): [number, number] {
  const r = seeded(seedOf(root, -3)), [x0, y0, x1, y1] = c.box;
  return [x0 + (0.1 + 0.8 * r()) * (x1 - x0), y0 + (0.1 + 0.8 * r()) * (y1 - y0)];
}

/** Something drawn that the pointer can find: its place and radius on screen, its key and its numbers. */
interface Hit { x: number; y: number; r: number; key: string; what: () => string }
/** A thing to draw this frame: its sprite, place, turn, and the order it is drawn in (angled: back to front). */
interface Shape { sprite: Sprite; x: number; y: number; turn: number; lift: number; hit: Hit; rim?: string }

/** The bodies and parts of the window the core last described, drawn over the map, and the one under the pointer. */
export class Avatars {
  private cells: CellThings[] = [];
  private cellM = 1;
  private hits: Hit[] = [];
  /** The keys of the thing under the pointer and of the one picked. */
  private over: string | null = null;
  private pinned: string | null = null;
  /** The style the shapes are drawn in. */
  style: Style = 'above';
  /** Sprites made at a scale, by node or part and style, kept until the scale or the things change. */
  private sprites = new Map<string, Sprite>();
  private spriteScale = '';
  /** The map's corner and scale (ox, oy, s) last drawn with things, or none. */
  private drawnAt: [number, number, number] | null = null;
  /** The picture of the things, reaching `THINGS_MARGIN` of the view beyond each edge, kept until the things, the scale or
   * the style change or the view leaves it: its key, the map's corner it was made for, and where it lies on screen. */
  private cache = document.createElement('canvas');
  private key = '';
  private madeAt: [number, number] = [0, 0];
  private shift: [number, number] = [0, 0];
  private version = 0;
  /** What is drawn under the things, read when their picture is made, for the stipples' colours. */
  private ground = new Ground();

  /** The list of things the core sent (none: nothing is drawn), for a map whose square is `cellM` metres. */
  set(data: Uint8Array | null, cellM: number): void { this.cells = data ? readThings(data) : []; this.cellM = cellM; this.sprites.clear(); this.version++; }
  /** The cells of that list. */
  get list(): CellThings[] { return this.cells; }

  /** Show the numbers of what is under the pointer on `cv` (else the cover of the cell under it), and keep what a click
   * picks; `changed` redraws the panel. */
  listen(cv: HTMLCanvasElement, changed: () => void): void {
    cv.addEventListener('pointermove', e => { const d = this.at(e.offsetX, e.offsetY) ?? this.coverAt(e.offsetX, e.offsetY); if (d !== this.over) { this.over = d; changed(); } });
    cv.addEventListener('pointerleave', () => { if (this.over) { this.over = null; changed(); } });
    cv.addEventListener('click', e => { this.pinned = this.at(e.offsetX, e.offsetY); changed(); });
  }

  /** The panel's text for the thing under the pointer, else the one picked (if the pointer is off the map's cells). */
  inspect(overCell: boolean): string | null {
    const key = this.over ?? (overCell ? null : this.pinned);
    if (key?.startsWith('c')) { const c = this.cells.find(x => x.index === Number(key.slice(1))); return c ? coverText(c) : null; }
    const d = key && this.hits.find(x => x.key === key);
    return d ? d.what() : null;
  }

  /** The key of the cover of the smallest cell with cover under (x, y), in the view last drawn. */
  private coverAt(x: number, y: number): string | null {
    const v = this.drawnAt;
    if (!v) return null;
    const X = (x - v[0]) / v[2], Y = (y - v[1]) / v[2];
    const c = this.cells.filter(c => c.slots.length && X >= c.box[0] && X < c.box[2] && Y >= c.box[1] && Y < c.box[3]).sort((a, b) => a.area - b.area)[0];
    return c ? `c${c.index}` : null;
  }

  /** The key of the front-most thing within `REACH_PX` of (x, y). */
  private at(x: number, y: number): string | null {
    x -= this.shift[0]; y -= this.shift[1];
    for (let k = this.hits.length - 1; k >= 0; k--) {
      const h = this.hits[k];
      if (Math.hypot(x - h.x, y - h.y) <= h.r + REACH_PX) return h.key;
    }
    return null;
  }

  /** The sprite `key` at scale `ppm`, made by `make` once. */
  private sprite(key: string, ppm: number, make: () => Sprite): Sprite {
    const scale = `${ppm},${this.style}`;
    if (scale !== this.spriteScale) { this.sprites.clear(); this.spriteScale = scale; }
    let s = this.sprites.get(key);
    if (!s) { s = make(); this.sprites.set(key, s); }
    return s;
  }

  /** Draw over the map, whose corner is at (ox, oy) and whose square takes `s` pixels: nothing below `THINGS_PX`. A
   * pan within the picture's margin only moves it. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    this.drawnAt = null;
    if (s < THINGS_PX || !this.cells.length) { this.hits = []; return; }
    this.drawnAt = [ox, oy, s];
    const W = ctx.canvas.clientWidth, H = ctx.canvas.clientHeight, d = window.devicePixelRatio || 1;
    const mx = Math.round(W * THINGS_MARGIN), my = Math.round(H * THINGS_MARGIN), cw = W + 2 * mx, ch = H + 2 * my;
    const key = [s, W, H, d, this.version, this.style].join();
    if (key !== this.key || Math.abs(ox - this.madeAt[0]) > mx || Math.abs(oy - this.madeAt[1]) > my) {
      this.key = key; this.madeAt = [ox, oy];
      this.cache.width = Math.round(cw * d); this.cache.height = Math.round(ch * d);
      const c = this.cache.getContext('2d')!;
      c.setTransform(d, 0, 0, d, 0, 0);
      this.ground.read(ctx, W, H);
      this.render(c, ox + mx, oy + my, s, cw, ch, (x, y) => this.ground.at(x - mx, y - my));
    }
    this.shift = [ox - this.madeAt[0] - mx, oy - this.madeAt[1] - my];
    ctx.drawImage(this.cache, this.shift[0], this.shift[1], cw, ch);
    // The thing under the pointer and the one picked: a thin ring.
    for (const key of [this.pinned, this.over]) for (const h of this.hits.filter(x => key && x.key === key)) {
      ctx.strokeStyle = key === this.over ? 'rgba(0,0,0,.7)' : 'rgba(255,255,255,.9)'; ctx.lineWidth = 1.2;
      ctx.beginPath(); ctx.arc(h.x + this.shift[0], h.y + this.shift[1], h.r + 2, 0, 2 * Math.PI); ctx.stroke();
    }
  }

  /** Draw the things onto `ctx` (`W` x `H` px), the map's corner at (ox, oy), its square `s` pixels, and keep where each
   * is; `ground(x, y)` is the lightness drawn under a point of `ctx`. */
  private render(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number, W: number, H: number, ground: (x: number, y: number) => number): void {
    this.hits = [];
    const ppm = s / this.cellM, sq = (x: number, y: number) => [ox + x * s, oy + y * s];
    const seen = this.cells.filter(c => c.box[0] * s + ox < W && c.box[2] * s + ox > 0 && c.box[1] * s + oy < H && c.box[3] * s + oy > 0);
    const onScreen = (x: number, y: number, r: number) => x + r > 0 && y + r > 0 && x - r < W && y - r < H;
    // The members' layouts, and the window in view widened by a body's length.
    const laid = seen.flatMap(c => {
      const byId = new Map(c.parts.map(p => [p.id, p] as const)), sizes = new Map<bigint, number>();
      for (const p of c.parts) sizes.set(p.root, Math.max(sizes.get(p.root) ?? 0, p.size[0]));
      return c.bodies.map(b => {
        const home = byId.get(b.inside), L = bodyLength(b.mass, b.adult), half = home ? (sizes.get(home.root) ?? 0) / this.cellM / 2 : 0;
        const at = home && structureAt(c, home.root), box: Box = at ? [at[0] - half, at[1] - half, at[0] + half, at[1] + half] : c.box;
        const pad = L / this.cellM;
        return { c, l: layoutOf(b, box), L, byId, view: [-ox / s - pad, -oy / s - pad, (W - ox) / s + pad, (H - oy) / s + pad] as Box };
      });
    });
    // The share of each node drawn, so the frame holds `MOST_SHOWN` shapes and dots.
    const wanted = laid.reduce((t, d) => t + (d.L * ppm >= DOT_PX ? d.l.shown * shareIn(d.l, d.view) : 0), 0);
    const keep = Math.min(1, MOST_SHOWN / Math.max(wanted, 1));
    const shapes: Shape[] = [];
    for (const c of seen) this.layParts(ctx, c, ppm, sq, onScreen, shapes);
    for (const { c, l, L, byId, view } of laid) {
      const b = l.b, px = L * ppm, what = () => bodyText(b, c), colour = animalColour(lookOf(b)), g2 = l.g * l.g;
      if (px < DOT_PX) {
        const [bx0, by0] = sq(l.box[0], l.box[1]), [bx1, by1] = sq(l.box[2], l.box[3]);
        // Its cell's real area, or that of the room it is inside.
        const m2 = l.box === c.box ? c.area : (l.box[2] - l.box[0]) * (l.box[3] - l.box[1]) * this.cellM * this.cellM;
        stipple(ctx, b.id, b.members / Math.max(m2, 1e-9), colour, [bx0, by0, bx1, by1], [0, 0, W, H], ground, (x, y, r, key) => this.hits.push({ x, y, r, key: `b${b.id}:s${key}`, what }));
        continue;
      }
      ctx.fillStyle = tone(colour, 1, DOT_ALPHA);
      ctx.beginPath();
      eachIn(l, view, Math.ceil(Math.ceil(l.shown / g2) * keep), (m, X, Y, turn) => {
        const [x, y] = sq(X, Y);
        if (!onScreen(x, y, px)) return;
        const hit: Hit = { x, y, r: px / 2, key: `b${b.id}:${m}`, what };
        if (px < SHAPE_PX) { ctx.moveTo(x + px / 2, y); ctx.arc(x, y, px / 2, 0, 2 * Math.PI); this.hits.push(hit); return; }
        const flip = this.style === 'angled' && Math.cos(turn) < 0;
        const sprite = this.sprite(`b${b.id}${flip ? 'f' : ''}`, ppm, () => animal(lookOf(b), ppm, this.style, flip));
        const rim = px >= RIM_FROM_PX ? rimOver(ground(x, y)) : undefined;
        shapes.push({ sprite, x, y, turn: this.style === 'above' ? turn : 0, lift: this.style === 'above' ? sprite.r * SHADOW_LIFT : 0, hit, rim });
        if (m === 0) this.heldBy(b, byId, x, y, sprite.r, ppm, shapes);
      });
      ctx.fill();
    }
    if (this.style === 'angled') shapes.sort((a, b) => a.y - b.y);
    for (const d of shapes) { paintSprite(ctx, d.sprite, d.x, d.y, d.turn, d.lift, d.rim); this.hits.push(d.hit); }
  }

  /** The parts of cell `c` lying free: each structure at its place, its parts side by side in a row by their real
   * lengths, and the parts of a lot one by one (`partAt`); a part smaller than `PART_HIDE_PX` is not drawn, one smaller
   * than `SHAPE_PX` is a dot. */
  private layParts(ctx: CanvasRenderingContext2D, c: CellThings, ppm: number, sq: (x: number, y: number) => number[], onScreen: (x: number, y: number, r: number) => boolean, shapes: Shape[]): void {
    const along = new Map<bigint, number>();
    for (const p of c.parts) {
      if (p.held !== 0n) continue;
      const look = partLook(p), px = look.size[0] * ppm;
      const off = along.get(p.root) ?? 0;
      along.set(p.root, off + look.size[0] * 1.1);
      if (px < PART_HIDE_PX) continue;
      for (let m = 0, n = partsOf(p); m < n; m++) {
        const [x, y] = sq(...partAt(c, p, m)), cx = x + (m === 0 ? off + look.size[0] / 2 : 0) * ppm;
        if (!onScreen(cx, y, px)) continue;
        const hit: Hit = { x: cx, y, r: px / 2, key: `p${p.id}`, what: () => partText(p, c) };
        if (px < SHAPE_PX) {
          ctx.fillStyle = tone(muted(display(look.colour)), 1, DOT_ALPHA);
          ctx.beginPath(); ctx.arc(cx, y, px / 2, 0, 2 * Math.PI); ctx.fill();
          this.hits.push(hit);
          continue;
        }
        const sprite = this.sprite(`p${p.id}`, ppm, () => object(look, ppm, this.style)), turn = this.style === 'above' ? (seeded(seedOf(p.id, m))() - 0.5) * 0.6 : 0;
        shapes.push({ sprite, x: cx, y, turn, lift: this.style === 'above' ? Math.min(sprite.r, look.size[2] * ppm) * SHADOW_LIFT : 0, hit });
      }
    }
  }

  /** The part node `b` holds, beside its first member drawn at (x, y) of radius `r`. */
  private heldBy(b: Animal, byId: Map<bigint, Thing>, x: number, y: number, r: number, ppm: number, shapes: Shape[]): void {
    const held = byId.get(b.hold);
    if (!held) return;
    const look = partLook(held), px = look.size[0] * ppm;
    if (px < SHAPE_PX) return;
    const sprite = this.sprite(`p${held.id}`, ppm, () => object(look, ppm, this.style));
    shapes.push({ sprite, x: x + r * 0.8, y, turn: 0, lift: 0, hit: { x: x + r * 0.8, y, r: px / 2, key: `p${held.id}`, what: () => `Held: ${num(held.count)} part${held.count > 1 ? 's' : ''} of ${num(held.mass)} kg` } });
  }
}

const fmtCount = (n: number) => n >= 1000 ? `${(n / 1000).toFixed(n >= 1e4 ? 0 : 1)}k` : n >= 10 ? n.toFixed(0) : n.toFixed(1);
const num = (v: number, d = 3) => (Math.abs(v) >= 1e4 || (Math.abs(v) < 1e-2 && v !== 0) ? v.toExponential(2) : Number(v.toPrecision(d)).toString());
const row = (k: string, v: string) => `<tr><td>${k}</td><td>${v}</td></tr>`;
const swatch = (c: Rgb) => `<s style="background:rgb(${display(c).map(Math.round)})"></s>`;
const head = (title: string, c: CellThings) => `<div class="cell"><b>${title}</b><span>cell ${c.index}</span></div>`;

function bodyText(b: Animal, c: CellThings): string {
  const eats = [[DIET_COVER, 'cover'], [DIET_BODIES, 'bodies (bites)'], [DIET_REMAINS, 'remains']].filter(([bit]) => b.diet & (bit as number)).map(x => x[1]).join(', ') || 'nothing';
  const density = b.members / Math.max(c.area, 1e-9);
  return head(b.members > 1 ? `Herd of ${fmtCount(b.members)}` : 'Animal', c) + '<table>' + [
    row('Id', String(b.id)), row('Kind (root)', b.root === 0xffffffff ? 'none' : String(b.root)), row('Lineage', String(b.lineage)),
    row('Stage', b.adult ? 'adult' : 'young'), row('Members', num(b.members)), row('Members per km²', num(density * 1e6)),
    row('Mass of a member', `${num(b.mass)} <u>kg</u>`), row('Length of a member', `${num(bodyLength(b.mass, b.adult))} <u>m</u>`),
    row('Living body of a member', `${num(b.living)} <u>kg</u>`), row('Copy mass', `${num(b.copyMass)} <u>kg</u>`), row('Strength', num(b.strength, 2)),
    row('Coat', num(b.coat, 2)), row('Keeps warm below', `${num(b.keepT)} <u>K</u>`), row('Cools above', `${num(b.coolT)} <u>K</u>`),
    row('Eats', eats), row('Brain', b.hidden ? `${b.hidden} hidden units` : 'none'), row('Colour', swatch(b.colour)),
    ...(b.hold ? [row('Holds part', String(b.hold))] : []), ...(b.inside ? [row('Inside structure of', String(b.inside))] : []),
  ].join('') + '</table>';
}

/** A slot of cover in one line: its form by its walls gene, its built cover and capture area, its genes. */
function slotLine(sl: Slot): string {
  const wood = woodiness(sl.genes[2]), form = wood < 0.2 ? 'grass' : wood < 0.5 ? 'shrubs' : 'trees';
  return `${form}, ${num(sl.kg)} <u>kg/m²</u>, ${num(sl.leaf)} <u>m²/m²</u>${sl.full ? ', full' : ''}<br><small>${COVER_GENES.map(([n, u], k) => `${n} ${num(sl.genes[k])} ${u}`).join('; ')}</small>`;
}

/** The cover of cell `c`: each slot in a line, the heaviest first. */
function coverText(c: CellThings): string {
  const slots = [...c.slots].sort((a, b) => b.kg - a.kg);
  return head('Plant cover', c) + '<table>' + slots.map(sl => row(`Kind ${sl.root}`, slotLine(sl))).join('') + '</table>';
}

function partText(p: Thing, c: CellThings): string {
  const bonds = c.bonds.filter(l => l.a === p.id || l.b === p.id).length;
  const title = p.hardness >= 0.3 ? 'Object (stone)' : 'Object';
  return head(p.count > 1 ? `${title}, ${fmtCount(p.count)} alike` : title, c) + '<table>' + [
    row('Id', String(p.id)), row('Alike parts', num(p.count)), row('Sizes', `${p.size.map(v => num(v)).join(' × ')} <u>m</u>`), row('Hollow', num(p.hollow, 2)),
    row('Mass of each', `${num(p.mass)} <u>kg</u>`), row('Hardness', num(p.hardness, 2)), row('Buried', `${num(p.buried)} <u>m</u>`),
    row('Held by', p.held ? String(p.held) : 'nobody'), row('Structure root', String(p.root)), row('Bonds', String(bonds)),
    row('Space closed', `${num(p.space)} <u>m³</u>, ${num(p.closed, 2)} closed`), row('Colour', swatch(p.colour)),
  ].join('') + '</table>';
}
