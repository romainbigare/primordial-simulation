// Avatars: when the map is close, what lies in each cell is drawn thing by thing (handover 10.5) from the list the core
// hands out (tape/src/things.rs): its bodies as animals, a node as some of its members with their count, the slots of
// its cover as plants by their density, its parts as objects. Drawn back to front by map position (10.4); hovering
// or clicking one shows its numbers (10.8).
import { Rgb, Sprite, animal, plant, object, paintSprite, animalLength, woodiness, display, plantLookOf, DIET_COVER, DIET_BODIES, DIET_REMAINS } from './sprites';
import { OPEN, PLANTS_CLOSE_PX } from './vegetation';

export interface Animal { id: bigint; members: number; mass: number; living: number; strength: number; colour: Rgb; copyMass: number; coat: number; keepT: number; coolT: number; root: number; lineage: number; diet: number; adult: boolean; hidden: number; hold: bigint; inside: bigint }
export interface Slot { root: number; kg: number; leaf: number; genes: number[]; full: boolean }
export interface Thing { id: bigint; size: [number, number, number]; hollow: number; mass: number; colour: Rgb; hardness: number; buried: number; held: bigint; root: bigint; space: number; closed: number }
export interface Bond { a: bigint; b: bigint; strength: number }
export interface CellThings { index: number; box: [number, number, number, number]; area: number; bodies: Animal[]; slots: Slot[]; parts: Thing[]; bonds: Bond[] }

/** The cover's genes in their order (pcore::cover): their names and units for the inspect panel. */
const COVER_GENES: [string, string][] = [['light', 'log10 mol/kg/s'], ['leaf', 'log10 m²/kg'], ['walls', 'log10 kg/kg'], ['store', 'log10 kg/kg'], ['frost', 'K'], ['moist', 'log10'], ['seed', 'share'], ['repair', 'log10'], ['fix', 'log10 mol/kg/s']];
/** Pixels a map square takes from which avatars are drawn. */
export const AVATAR_PX = 64;
/** Avatars drawn at most in a frame; past it, fewer plants and members are drawn and the map's colour shows the rest. */
const MOST_AVATARS = 3000;
/** Members of a node drawn at most, and at least before the budget cuts it. */
const MOST_MEMBERS = 40, FEW_MEMBERS = 3;
/** The share of a cell's screen that its animals' footprints fill at most, and the bodies of a cell whose count is written by them (the largest). */
const HERD_FILL = 0.35, LABELS_PER_CELL = 2;
/** The width in pixels from which a cell writes the counts of its herds. */
const LABEL_CELL_PX = 100;
/** The share of `MOST_AVATARS` parts take at most. */
const PART_SHARE = 0.25;
/** Plants a cell draws at most per its base size squared of screen, at a closed cover. */
const PLANTS_PER_UNIT2 = 2;
/** Built cover, kg/m², at which grass and a forest close (cover share 1 - 1/e). */
const GRASS_CLOSED = 0.3, FOREST_CLOSED = 4;

/** Reads the list of things (tape::things). */
export function readThings(data: Uint8Array): CellThings[] {
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
      const i = id(), [l, w, t, hollow, mass, r, g, b, hardness, buried] = f32s(10);
      return { id: i, size: [l, w, t], hollow, mass, colour: [r, g, b], hardness, buried, held: id(), root: id(), space: f32(), closed: f32() };
    });
    const bonds = Array.from({ length: nl }, (): Bond => ({ a: id(), b: id(), strength: f32() }));
    cells.push({ index, box, area, bodies, slots, parts, bonds });
  }
  return cells;
}

/** A seed from an id and a number (a member, a plant). */
const seedOf = (id: bigint, k: number) => (Number(id & 0xffffffffn) * 2654435761 ^ Number(id >> 32n) * 40503 ^ (k + 1) * 2246822519) >>> 0;
/** What the budget lets a frame draw: members of a node at most, and the share of each cell's parts and plants. */
interface Share { members: number; parts: number; plants: number }
/** Something drawn: its sprite at (x, y), and what it is for the inspect panel. */
interface Drawn { y: number; x: number; sprite: Sprite; key: string; what: () => string }

/** The avatars of the window the core last described, drawn over the map, and the one under the pointer. */
export class Avatars {
  private cells: CellThings[] = [];
  private drawn: Drawn[] = [];
  private labels: { x: number; y: number; text: string }[] = [];
  /** The keys of the avatar under the pointer and of the one picked. */
  private over: string | null = null;
  private pinned: string | null = null;
  /** The things and the view (corner, scale, canvas size) the layout was made for. */
  private laid: CellThings[] | null = null;
  private laidView = '';

  /** The list of things the core sent (none: nothing is drawn). */
  set(data: Uint8Array | null): void { this.cells = data ? readThings(data) : []; }
  /** The cells of that list. */
  get list(): CellThings[] { return this.cells; }

  /** Show the numbers of what is under the pointer on `cv`, and keep what a click picks; `changed` redraws the panel. */
  listen(cv: HTMLCanvasElement, changed: () => void): void {
    cv.addEventListener('pointermove', e => { const d = this.at(e.offsetX, e.offsetY); if (d !== this.over) { this.over = d; changed(); } });
    cv.addEventListener('pointerleave', () => { if (this.over) { this.over = null; changed(); } });
    cv.addEventListener('click', e => { this.pinned = this.at(e.offsetX, e.offsetY); changed(); });
  }

  /** The panel's text for the avatar under the pointer, else the one picked (if the pointer is off the map's cells). */
  inspect(overCell: boolean): string | null {
    const key = this.over ?? (overCell ? null : this.pinned), d = key && this.drawn.find(x => x.key === key);
    return d ? d.what() : null;
  }

  /** The key of the front-most avatar at (x, y). */
  private at(x: number, y: number): string | null {
    for (let k = this.drawn.length - 1; k >= 0; k--) {
      const d = this.drawn[k], [x0, y0, x1, y1] = d.sprite.box;
      if (x >= d.x + x0 && x <= d.x + x1 && y >= d.y + y0 && y <= d.y + y1) return d.key;
    }
    return null;
  }

  /** Draw over the map, whose corner is at (ox, oy) and whose square takes `s` pixels: nothing below `AVATAR_PX`. The layout is kept until the things or the view change. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    const W = ctx.canvas.clientWidth, H = ctx.canvas.clientHeight, view = [ox, oy, s, W, H].join();
    if (s < AVATAR_PX || !this.cells.length) { this.drawn = []; this.labels = []; this.laid = null; return; }
    if (this.laid !== this.cells || this.laidView !== view) {
      this.drawn = []; this.labels = []; this.laid = this.cells; this.laidView = view;
      const unit = Math.min(Math.max(s * 0.05, 4), 18);
      const seen = this.cells.filter(c => c.box[0] * s + ox < W && c.box[2] * s + ox > 0 && c.box[1] * s + oy < H && c.box[3] * s + oy > 0);
      const share = this.budget(seen, s, unit);
      for (const c of seen) this.layCell(c, [c.box[0] * s + ox, c.box[1] * s + oy, c.box[2] * s + ox, c.box[3] * s + oy], unit, share, s >= PLANTS_CLOSE_PX);
      this.drawn.sort((a, b) => a.y - b.y);
    }
    ctx.save();
    for (const d of this.drawn) paintSprite(ctx, d.sprite, d.x, d.y);
    ctx.font = '600 10px system-ui, sans-serif'; ctx.textAlign = 'center'; ctx.lineJoin = 'round'; ctx.lineWidth = 3;
    for (const l of this.labels) { ctx.strokeStyle = 'rgba(0,0,0,.6)'; ctx.strokeText(l.text, l.x, l.y); ctx.fillStyle = '#fff'; ctx.fillText(l.text, l.x, l.y); }
    for (const key of [this.pinned, this.over]) for (const d of this.drawn.filter(x => key && x.key === key)) {
      const [x0, y0, x1, y1] = d.sprite.box;
      ctx.strokeStyle = key === this.over ? '#000' : '#fff'; ctx.lineWidth = 1.5; ctx.strokeRect(d.x + x0 - 2, d.y + y0 - 2, x1 - x0 + 4, y1 - y0 + 4);
    }
    ctx.restore();
  }

  /** Members a node shows at most, and the share of each cell's parts and plants shown, so the frame holds `MOST_AVATARS`: bodies first, then parts (at most `PART_SHARE` of it), then plants. */
  private budget(cells: CellThings[], s: number, unit: number): Share {
    const cellPx = (c: CellThings) => (c.box[2] - c.box[0]) * (c.box[3] - c.box[1]) * s * s;
    let members = MOST_MEMBERS;
    const want = (k: number) => cells.reduce((t, c) => t + herds(c, cellPx(c), unit, k).reduce((u, n) => u + n, 0), 0);
    while (members > FEW_MEMBERS && want(members) > MOST_AVATARS / 2) members--;
    const partsWanted = cells.reduce((t, c) => t + c.parts.length, 0), partRoom = MOST_AVATARS * PART_SHARE;
    const parts = partsWanted > partRoom ? partRoom / partsWanted : 1;
    const room = MOST_AVATARS - want(members) - Math.min(partsWanted, partRoom);
    const plantsWanted = cells.reduce((t, c) => t + (drawsPlants(c, s >= PLANTS_CLOSE_PX) ? c.slots.reduce((u, sl) => u + plantsIn(sl, cellPx(c), unit), 0) : 0), 0);
    return { members, parts, plants: plantsWanted > room ? Math.max(room, 0) / plantsWanted : 1 };
  }

  /** Lay out cell `c` (at x0, y0 to x1, y1 on the screen): plants scattered by their density where its cover is open or the view `close` (else the ground pattern draws them, vegetation.ts), each structure at a place of its own (its parts side by side, the largest structures first), the bodies inside one by it, each node as a herd around a place of its own, a held part by its holder. */
  private layCell(c: CellThings, [x0, y0, x1, y1]: number[], unit: number, share: Share, close: boolean): void {
    const spot = (r: () => number, margin: number) => [x0 + margin + r() * Math.max(x1 - x0 - 2 * margin, 1), y0 + margin + r() * Math.max(y1 - y0 - 2 * margin, 1)];
    if (drawsPlants(c, close)) c.slots.forEach((sl, k) => {
      const n = Math.floor(plantsIn(sl, (x1 - x0) * (y1 - y0), unit) * share.plants), look = plantLook(sl);
      const r = seededAt(c, 7919 * (k + 1));
      for (let j = 0; j < n; j++) {
        const [x, y] = spot(r, unit * 0.2), seed = (r() * 4294967296) >>> 0;
        this.drawn.push({ x, y, sprite: plant(look, seed, unit), key: `s${c.box[0]},${c.box[1]},${k}`, what: () => slotText(sl, c) });
      }
    });
    const structures = new Map<bigint, Thing[]>(), byId = new Map<bigint, Thing>(), place = new Map<bigint, [number, number]>();
    for (const p of c.parts) {
      byId.set(p.id, p);
      if (p.held === 0n) structures.has(p.root) ? structures.get(p.root)!.push(p) : structures.set(p.root, [p]);
    }
    let left = Math.ceil(c.parts.length * share.parts);
    for (const [root, parts] of [...structures].sort((a, b) => b[1].length - a[1].length)) {
      if (left <= 0) break;
      const r = seededAt(c, Number(root & 0xffffn)), at = spot(r, unit) as [number, number], side = Math.ceil(Math.sqrt(parts.length));
      place.set(root, at);
      parts.slice(0, left).forEach((p, k) => {
        const [i, j] = [k % side, Math.floor(k / side)];
        this.drawn.push({ x: at[0] + (i - j) * unit * 0.45, y: at[1] + (i + j) * unit * 0.22, sprite: object(partLook(p), seedOf(p.id, 0), unit), key: `p${p.id}`, what: () => partText(p, c) });
      });
      left -= parts.length;
    }
    const counts = herds(c, (x1 - x0) * (y1 - y0), unit, share.members), labelled = new Set([...c.bodies].sort((a, b) => b.members - a.members).slice(0, LABELS_PER_CELL));
    c.bodies.forEach((b, q) => {
      const shown = counts[q], r = seededAt(c, seedOf(b.id, 0));
      const room = byId.get(b.inside), home = room && place.get(room.root);
      const [cx, cy] = home ?? spot(r, unit * 1.2), heading = r() < 0.5;
      const spread = Math.min(unit * animalLength(b.mass) * 1.1 * Math.sqrt(shown), (x1 - x0) * 0.45);
      const look = { mass: b.mass, strength: b.strength, colour: b.colour, coat: b.coat, keepT: b.keepT, coolT: b.coolT, diet: b.diet, hidden: b.hidden, adult: b.adult, lineage: b.lineage };
      let first: [number, number] | null = null;
      for (let m = 0; m < shown; m++) {
        const a = r() * 2 * Math.PI, d = spread * Math.sqrt(r()) * (m ? 1 : 0);
        const x = Math.min(Math.max(cx + Math.cos(a) * d, x0), x1), y = Math.min(Math.max(cy + Math.sin(a) * d * 0.6, y0), y1);
        first ??= [x, y];
        this.drawn.push({ x, y, sprite: animal(look, seedOf(b.id, m), unit, r() < 0.85 ? heading : !heading), key: `b${b.id}`, what: () => bodyText(b, c) });
      }
      if (b.members > shown && first && labelled.has(b) && x1 - x0 >= LABEL_CELL_PX) this.labels.push({ x: cx, y: cy - unit * animalLength(b.mass) * 1.4 - spread * 0.6, text: `×${fmtCount(b.members)}` });
      const held = byId.get(b.hold);
      if (held && first) this.drawn.push({ x: first[0] + unit * 0.7, y: first[1] + 1, sprite: object(partLook(held), seedOf(held.id, 0), unit * 0.6), key: `p${held.id}`, what: () => partText(held, c) });
    });
  }
}

/** Members of each of cell `c`'s bodies drawn, in a cell of `px` square pixels: each body at least one and at most `most`, the room of the cell (`HERD_FILL` of it in animals' footprints) shared by the root of their members, so a crowded cell reads as crowded, not as a heap. */
function herds(c: CellThings, px: number, unit: number, most: number): number[] {
  const foot = c.bodies.reduce((t, b) => t + (unit * animalLength(b.mass)) ** 2, 0) / Math.max(c.bodies.length, 1);
  const room = Math.max(c.bodies.length, Math.floor((HERD_FILL * px) / Math.max(foot, 1))), weight = c.bodies.reduce((t, b) => t + Math.sqrt(b.members), 0);
  return c.bodies.map(b => Math.max(1, Math.min(Math.ceil(b.members), most, Math.floor((room * Math.sqrt(b.members)) / weight))));
}

/** Plants a slot draws in a cell of `px` square pixels: by its cover share, closing at `GRASS_CLOSED` to `FOREST_CLOSED` kg/m² as it is woodier. */
function plantsIn(sl: Slot, px: number, unit: number): number {
  const wood = woodiness(sl.genes[2]), closed = GRASS_CLOSED + (FOREST_CLOSED - GRASS_CLOSED) * wood;
  const share = 1 - Math.exp(-sl.kg / closed);
  return share * PLANTS_PER_UNIT2 * px / (unit * unit * (1 + 3 * wood));
}
/** Whether cell `c`'s plants are drawn one by one: where its cover is open (its slots' capture area per m² absorbs less than `OPEN` of the light), or the view is close. */
const drawsPlants = (c: CellThings, close: boolean) => close || 1 - Math.exp(-c.slots.reduce((t, sl) => t + sl.leaf, 0)) < OPEN;
const plantLook = (sl: Slot) => plantLookOf(sl.genes, sl.kg);
const partLook = (p: Thing) => ({ size: p.size, hollow: p.hollow, colour: p.colour, hardness: p.hardness, buried: p.buried, room: p.space > 0 && p.closed > 0.5 });
/** A seeded stream for cell `c` by its place, so its layout holds from frame to frame. */
const seededAt = (c: CellThings, k: number) => {
  let a = (Math.round(c.box[0] * 4096) * 73856093 ^ Math.round(c.box[1] * 4096) * 19349663 ^ k) >>> 0;
  return () => { a = (Math.imul(a ^ (a >>> 16), 0x45d9f3b) + 0x9e3779b9) >>> 0; return a / 4294967296; };
};

const fmtCount = (n: number) => n >= 1000 ? `${(n / 1000).toFixed(n >= 1e4 ? 0 : 1)}k` : n >= 10 ? n.toFixed(0) : n.toFixed(1);
const num = (v: number, d = 3) => (Math.abs(v) >= 1e4 || (Math.abs(v) < 1e-2 && v !== 0) ? v.toExponential(2) : Number(v.toPrecision(d)).toString());
const row = (k: string, v: string) => `<tr><td>${k}</td><td>${v}</td></tr>`;
const swatch = (c: Rgb) => `<s style="background:rgb(${display(c).map(Math.round)})"></s>`;
const head = (title: string, c: CellThings) => `<div class="cell"><b>${title}</b><span>cell ${c.index}</span></div>`;

function bodyText(b: Animal, c: CellThings): string {
  const eats = [[DIET_COVER, 'cover'], [DIET_BODIES, 'bodies (bites)'], [DIET_REMAINS, 'remains']].filter(([bit]) => b.diet & (bit as number)).map(x => x[1]).join(', ') || 'nothing';
  return head(b.members > 1 ? `Herd of ${fmtCount(b.members)}` : 'Animal', c) + '<table>' + [
    row('Id', String(b.id)), row('Kind (root)', b.root === 0xffffffff ? 'none' : String(b.root)), row('Lineage', String(b.lineage)),
    row('Stage', b.adult ? 'adult' : 'young'), row('Members', num(b.members)), row('Mass of a member', `${num(b.mass)} <u>kg</u>`),
    row('Living body of a member', `${num(b.living)} <u>kg</u>`), row('Copy mass', `${num(b.copyMass)} <u>kg</u>`), row('Strength', num(b.strength, 2)),
    row('Coat', num(b.coat, 2)), row('Keeps warm below', `${num(b.keepT)} <u>K</u>`), row('Cools above', `${num(b.coolT)} <u>K</u>`),
    row('Eats', eats), row('Brain', b.hidden ? `${b.hidden} hidden units` : 'none'), row('Colour', swatch(b.colour)),
    ...(b.hold ? [row('Holds part', String(b.hold))] : []), ...(b.inside ? [row('Inside structure of', String(b.inside))] : []),
  ].join('') + '</table>';
}

function slotText(sl: Slot, c: CellThings): string {
  const wood = woodiness(sl.genes[2]), form = wood < 0.2 ? 'grass' : wood < 0.5 ? 'shrubs' : 'trees';
  return head(`Plant cover (${form})`, c) + '<table>' + [
    row('Kind (root)', String(sl.root)), row('Built cover', `${num(sl.kg)} <u>kg/m²</u>`), row('Capture area', `${num(sl.leaf)} <u>m²/m²</u>`), row('Full', sl.full ? 'yes' : 'no'),
    ...COVER_GENES.map(([n, u], k) => row(`Gene ${n}`, `${num(sl.genes[k])} <u>${u}</u>`)),
  ].join('') + '</table>';
}

function partText(p: Thing, c: CellThings): string {
  const bonds = c.bonds.filter(l => l.a === p.id || l.b === p.id).length;
  return head(p.hardness >= 0.3 ? 'Object (stone)' : 'Object', c) + '<table>' + [
    row('Id', String(p.id)), row('Sizes', `${p.size.map(v => num(v)).join(' × ')} <u>m</u>`), row('Hollow', num(p.hollow, 2)),
    row('Mass', `${num(p.mass)} <u>kg</u>`), row('Hardness', num(p.hardness, 2)), row('Buried', `${num(p.buried)} <u>m</u>`),
    row('Held by', p.held ? String(p.held) : 'nobody'), row('Structure root', String(p.root)), row('Bonds', String(bonds)),
    row('Space closed', `${num(p.space)} <u>m³</u>, ${num(p.closed, 2)} closed`), row('Colour', swatch(p.colour)),
  ].join('') + '</table>';
}
