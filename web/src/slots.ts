// The slots of cover of the cells in view, from the list the core hands out when the map is close (tape/src/things.rs):
// hovering a cell shows each slot's numbers (handover 10.8). The cover itself is drawn as a texture (vegetation.ts).

interface Slot { root: number; kg: number; leaf: number; genes: number[]; full: boolean }
interface CellSlots { index: number; box: [number, number, number, number]; area: number; slots: Slot[] }

/** The cover's genes in their order (pcore::cover): their names and units for the inspect panel. */
const COVER_GENES: [string, string][] = [['light', 'log10 mol/kg/s'], ['leaf', 'log10 m²/kg'], ['walls', 'log10 kg/kg'], ['store', 'log10 kg/kg'], ['frost', 'K'], ['moist', 'log10'], ['seed', 'share'], ['repair', 'log10'], ['fix', 'log10 mol/kg/s'], ['gas', 'share']];
/** Pixels a map square takes from which the slots of the window in view are asked for. */
export const THINGS_PX = 64;

/** Reads the list of slots (tape::things). */
function readSlots(data: Uint8Array): CellSlots[] {
  const dv = new DataView(data.buffer, data.byteOffset, data.byteLength);
  let at = 0;
  const u8 = () => dv.getUint8(at++), u16 = () => { at += 2; return dv.getUint16(at - 2, true); };
  const u32 = () => { at += 4; return dv.getUint32(at - 4, true); }, f32 = () => { at += 4; return dv.getFloat32(at - 4, true); };
  const f32s = (n: number) => Array.from({ length: n }, f32);
  const cells: CellSlots[] = [];
  for (let n = u32(); n > 0; n--) {
    const index = u32(), box = f32s(4) as [number, number, number, number], area = f32();
    const slots = Array.from({ length: u16() }, (): Slot => { const root = u32(), kg = f32(), leaf = f32(), genes = f32s(COVER_GENES.length), full = u8() === 1; at += 3; return { root, kg, leaf, genes, full }; });
    cells.push({ index, box, area, slots });
  }
  return cells;
}

/** The slots of the window the core last described, and the cell with cover under the pointer. */
export class CoverSlots {
  private cells: CellSlots[] = [];
  /** The index of the cell under the pointer, if it has cover. */
  private over: number | null = null;
  /** The map's corner and scale (ox, oy, s) last drawn, or none. */
  private drawnAt: [number, number, number] | null = null;

  /** The list of slots the core sent (none: nothing to show). */
  set(data: Uint8Array | null): void { this.cells = data ? readSlots(data) : []; }

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
    return c ? coverText(c) : null;
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
function slotLine(sl: Slot): string {
  return `${num(sl.kg)} <u>kg/m²</u>, ${num(sl.leaf)} <u>m²/m²</u>${sl.full ? ', full' : ''}<br><small>${COVER_GENES.map(([n, u], k) => `${n} ${num(sl.genes[k])} ${u}`).join('; ')}</small>`;
}

/** The cover of cell `c`: each slot in a line, the heaviest first. */
function coverText(c: CellSlots): string {
  const slots = [...c.slots].sort((a, b) => b.kg - a.kg);
  return `<div class="cell"><b>Plant cover</b><span>cell ${c.index}</span></div><table>` + slots.map(sl => row(`Kind ${sl.root}`, slotLine(sl))).join('') + '</table>';
}
