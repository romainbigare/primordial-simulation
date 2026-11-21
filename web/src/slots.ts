// The slots of cover of the cells in view, from the list the core hands out when the map is close (tape/src/things.rs):
// hovering a cell shows each slot's numbers (handover 10.8). The cover itself is drawn as a texture (vegetation.ts).

interface Slot { root: number; kg: number; leaf: number; genes: number[]; full: boolean; patch: number }
/** A patch of a cell's cover: its share of the cell and its age, years since it was made (the core's `cover::Patch`). */
interface Patch { share: number; age: number }
interface CellSlots { index: number; box: [number, number, number, number]; area: number; patches: Patch[]; slots: Slot[] }

/** Pixels a map square takes from which the slots of the window in view are asked for. */
export const THINGS_PX = 64;

/** The cover's genes as the core names them (pcore::cover::GENES, sent with the list): name and unit of each. */
type Genes = [string, string][];

/** Reads the list of slots (tape::things): the genes it names, then the cells. */
function readSlots(data: Uint8Array): { genes: Genes; cells: CellSlots[] } {
  const dv = new DataView(data.buffer, data.byteOffset, data.byteLength);
  let at = 0;
  const u8 = () => dv.getUint8(at++), u16 = () => { at += 2; return dv.getUint16(at - 2, true); };
  const u32 = () => { at += 4; return dv.getUint32(at - 4, true); }, f32 = () => { at += 4; return dv.getFloat32(at - 4, true); };
  const f32s = (n: number) => Array.from({ length: n }, f32);
  const text = () => { const n = u8(); at += n; return new TextDecoder().decode(data.subarray(at - n, at)); };
  const named: Genes = Array.from({ length: u16() }, (): [string, string] => [text(), text()]);
  const cells: CellSlots[] = [];
  for (let n = u32(); n > 0; n--) {
    const index = u32(), box = f32s(4) as [number, number, number, number], area = f32();
    const patches = Array.from({ length: u8() }, (): Patch => ({ share: f32(), age: f32() }));
    const slots = Array.from({ length: u16() }, (): Slot => { const root = u32(), kg = f32(), leaf = f32(), genes = f32s(named.length), full = u8() === 1, patch = u8(); at += 2; return { root, kg, leaf, genes, full, patch }; });
    cells.push({ index, box, area, patches, slots });
  }
  return { genes: named, cells };
}

/** The slots of the window the core last described, and the cell with cover under the pointer. */
export class CoverSlots {
  private cells: CellSlots[] = [];
  /** The genes the core last named. */
  private genes: Genes = [];
  /** The index of the cell under the pointer, if it has cover. */
  private over: number | null = null;
  /** The map's corner and scale (ox, oy, s) last drawn, or none. */
  private drawnAt: [number, number, number] | null = null;

  /** The list of slots the core sent (none: nothing to show). */
  set(data: Uint8Array | null): void { ({ genes: this.genes, cells: this.cells } = data ? readSlots(data) : { genes: [], cells: [] }); }

  /** Keep where the map was last drawn: its corner at (ox, oy), a square `s` pixels; below `THINGS_PX`, nowhere. */
  seen(ox: number, oy: number, s: number): void { this.drawnAt = s >= THINGS_PX && this.cells.length ? [ox, oy, s] : null; }

  /** Follow the pointer on `cv`; `changed` redraws the panel. */
  listen(cv: HTMLCanvasElement, changed: () => void): void {
    cv.addEventListener('pointermove', e => { const c = this.at(e.offsetX, e.offsetY); if (c !== this.over) { this.over = c; changed(); } });
    cv.addEventListener('pointerleave', () => { if (this.over !== null) { this.over = null; changed(); } });
  }

  /** The panel's text for the cover under the pointer, if any. */
  inspect(): string | null {
    const c = this.over === null ? undefined : this.cells.find(x => x.index === this.over);
    return c ? coverText(c, this.genes) : null;
  }

  /** The index of the smallest cell with cover under (x, y), in the view last drawn. */
  private at(x: number, y: number): number | null {
    const v = this.drawnAt;
    if (!v) return null;
    const X = (x - v[0]) / v[2], Y = (y - v[1]) / v[2];
    const c = this.cells.filter(c => c.slots.length && X >= c.box[0] && X < c.box[2] && Y >= c.box[1] && Y < c.box[3]).sort((a, b) => a.area - b.area)[0];
    return c ? c.index : null;
  }
}

const num = (v: number, d = 3) => (Math.abs(v) >= 1e4 || (Math.abs(v) < 1e-2 && v !== 0) ? v.toExponential(2) : Number(v.toPrecision(d)).toString());
const row = (k: string, v: string) => `<tr><td>${k}</td><td>${v}</td></tr>`;

/** A slot of cover in one line: its built cover and capture area, its genes. */
function slotLine(sl: Slot, genes: Genes): string {
  return `${num(sl.kg)} <u>kg/m²</u>, ${num(sl.leaf)} <u>m²/m²</u>${sl.full ? ', full' : ''}<br><small>${genes.map(([n, u], k) => `${n} ${num(sl.genes[k])} ${u}`).join('; ')}</small>`;
}

/** Patch `k` of a cell in a line: its share of the cell and how long ago it was made (the first: the rest of the cell). */
function patchLine(p: Patch, k: number): string {
  const years = Math.round(p.age);
  const made = k === 0 ? 'the rest of the cell' : years < 1 ? 'made this year' : `made ${years} year${years === 1 ? '' : 's'} ago`;
  return `<tr><td colspan="2"><b>Patch ${k}</b>: ${num(100 * p.share, 2)}% of the cell, ${made}</td></tr>`;
}

/** The cover of cell `c`: each slot in a line, the heaviest first, kg/m² of the cell's area; in a cell of patches,
 * under the patch it stands in. */
function coverText(c: CellSlots, genes: Genes): string {
  const slots = [...c.slots].sort((a, b) => b.kg - a.kg);
  const lines = (k: number) => slots.filter(sl => sl.patch === k).map(sl => row(`Kind ${sl.root}`, slotLine(sl, genes))).join('');
  const body = c.patches.length > 1 ? c.patches.map((p, k) => patchLine(p, k) + lines(k)).join('') : lines(0);
  return `<div class="cell"><b>Plant cover</b><span>cell ${c.index}</span></div><table>` + body + '</table>';
}
