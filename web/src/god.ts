// God mode on the live map (handover 11): the player picks a tool, then a place, and the world is edited there between
// two months (wasm/src/lib.rs `edit`, the core's `World::edit`): a share of the cell under the cursor cleared, the
// same cleared and planted with a starting kind, or a strike that lights the cell in the coming month. What the world
// cannot do there is greyed out with the reason, from the core's own test (`probe`: the flames' `lights`).

/** What the core says of the cell under a place (wasm/src/lib.rs `probe`): the cell, its edges in squares (west, north,
 * east, south), the centre of its sample under the place (squares), about which a clearing makes its patch, its area
 * and land (m²), its patches, and whether a strike lights it ("yes", or the core's reason). */
export interface Probe { cell: number; bounds: [number, number, number, number]; at: [number, number]; area: number; land: number; patches: number; fire: string }
/** A starting kind the world's plan places (wasm/src/lib.rs `kinds`): its row, the core's name, kg per m² placed. */
export interface StartKind { row: number; name: string; kg: number }
/** What an edit did (wasm/src/lib.rs `edit`): the cell, and whether each of its edits was made. */
export interface Edited { cell: number; done: boolean[] }
/** The worker's side of the tools: ask what lies under a place, and edit there. */
export interface Editor { probe(x: number, y: number): void; edit(act: number, x: number, y: number, share: number, row: number): void }

type Tool = 'clear' | 'plant' | 'fire';
/** The tools, their labels and the core's code of each (wasm/src/lib.rs `edit`: 0 clear, 1 clear and plant, 2 strike). */
const TOOLS: [Tool, string, number][] = [['clear', 'Clear', 0], ['plant', 'Plant', 1], ['fire', 'Set on fire', 2]];
/** The shares of a cell a clearing may take. */
const SHARES = [0.01, 0.05, 0.1, 0.25];
/** The starting kinds in plain words, by the core's name (worldgen/src/life/kinds.rs `KINDS`): a short name and a line. */
const PLAIN_KINDS: Record<string, [string, string]> = {
  'small fast light-eaters': ['Short grass', 'Small plants that grow and spread fast.'],
  'light-eaters': ['Herbs', 'Soft plants of middle size.'],
  'big slow light-eaters': ['Trees', 'Tall woody plants. They grow slowly and live long.'],
  'big light-eaters of wet ground': ['Swamp trees', 'Trees that stand in wet ground.'],
  'woody light-eaters of dry ground': ['Dry shrubs', 'Woody bushes of dry ground.'],
  'tall fast light-eaters': ['Tall grass', 'Tall soft plants that grow fast.'],
};
/** Why a strike does not light a cell, in plain words, by the core's reason (core/src/flame/mod.rs `Unlit`). */
const UNLIT: Record<string, string> = {
  NoLand: 'This is open water. Nothing here can burn.',
  Lifeless: 'Nothing in this world can burn.',
  Bare: 'Nothing lies on the ground here that can burn.',
  Sparse: 'Too little dead leaves and stems lie here. They do not cover the ground, so a fire cannot spread.',
  Thin: 'Too little dead leaves and stems lie here to keep a flame going.',
  Snow: 'Snow covers the ground.',
  Unread: 'Let the world run one month first.',
  Wet: 'The ground is too wet. Much of it is soaked.',
  Damp: 'The dead leaves and stems are too damp to burn.',
};
/** Least time between two questions about the place under the cursor, ms. */
const PROBE_MS = 80;

const pct = (s: number) => `${Math.round(s * 100)}%`;
const plainKind = (k: StartKind): [string, string] => PLAIN_KINDS[k.name] ?? [k.name, ''];
const hectares = (m2: number) => (m2 >= 1e4 ? `${(m2 / 1e4).toFixed(m2 >= 1e5 ? 0 : 1)} ha` : `${Math.round(m2)} m²`);

export class God {
  private tool: Tool | null = null;
  private share = SHARES[1];
  private row = -1;
  private kinds: StartKind[] = [];
  /** The place under the cursor (squares), and the probe of the place last answered. */
  private at: [number, number] | null = null;
  private probe: Probe | null = null;
  /** Whether a question is out, and when the last was asked (ms). */
  private asking = false;
  private asked = 0;
  /** The map's corner and the pixels a square takes, as last drawn. */
  private drawnAt: [number, number, number] | null = null;
  private note = '';
  /** The tool, share and kind of the edit last sent, for its answer (`edited`). */
  private pending: [Tool, number, number] = ['clear', 0, 0];

  constructor(private el: HTMLElement, private editor: Editor, private redraw: () => void) {}

  /** A new world: its starting kinds; the tools put away. */
  setKinds(kinds: StartKind[]): void {
    this.kinds = kinds; this.row = kinds.find(k => plainKind(k)[0] === 'Trees')?.row ?? kinds[0]?.row ?? -1;
    this.tool = null; this.probe = null; this.note = '';
    this.show();
  }

  /** Follow the pointer on `cv`: the place under it is asked of the core. */
  listen(cv: HTMLCanvasElement): void {
    cv.addEventListener('pointermove', e => {
      const v = this.drawnAt;
      if (!v) return;
      this.at = [(e.offsetX - v[0]) / v[2], (e.offsetY - v[1]) / v[2]];
      this.ask();
      if (this.tool) this.redraw();
    });
    cv.addEventListener('pointerleave', () => { this.at = null; if (this.tool) this.redraw(); });
    addEventListener('keydown', e => { if (e.key === 'Escape' && this.tool) { this.tool = null; this.show(); this.redraw(); } });
  }

  /** Ask the core about the place under the cursor, at most once every `PROBE_MS` and one question at a time. */
  private ask(): void {
    if (!this.at || this.asking) return;
    const wait = this.asked + PROBE_MS - performance.now();
    if (wait > 0) { setTimeout(() => this.ask(), wait); return; }
    this.asking = true; this.asked = performance.now();
    this.editor.probe(...this.at);
  }

  /** The core's answer about a place. */
  probed(p: Probe): void {
    this.asking = false;
    const was = this.probe;
    this.probe = p;
    if (this.tool) this.redraw();
    if (!was || was.cell !== p.cell || was.fire !== p.fire || was.land !== p.land || was.patches !== p.patches) { this.show(); this.redraw(); }
  }

  /** The core's answer to an edit. */
  edited(r: Edited): void {
    const [tool, share, row] = this.pending;
    const kind = this.kinds.find(k => k.row === row), name = kind ? plainKind(kind)[0].toLowerCase() : 'plants';
    const ok = r.done.every(d => d);
    this.note = tool === 'clear' ? (ok ? `Cleared ${pct(share)} of this cell. It will grow back.` : 'Nothing was cleared here.')
      : tool === 'plant' ? (ok ? `Cleared ${pct(share)} of this cell and planted ${name} in it.` : r.done[0] ? `Cleared ${pct(share)} of this cell, but ${name} found no room to grow.` : 'Nothing was planted here.')
      : ok ? 'Lightning will strike here this coming month. Play to watch it burn.' : 'This place cannot burn now.';
    this.probe = null;
    this.ask();
    this.show();
  }

  /** Why the tool `t` cannot be used at the place under the cursor, or ''. */
  private why(t: Tool): string {
    const p = this.probe;
    if (t === 'plant' && !this.kinds.length) return 'This world has no plants to plant.';
    if (!p) return '';
    if (t === 'fire') return p.fire === 'yes' ? '' : (UNLIT[p.fire] ?? p.fire);
    return p.land > 0 ? '' : 'This is open water. Nothing grows here.';
  }

  /** A tap at (x, y) squares: the tool's edit there, true; false if no tool is on (the map picks the square). */
  tap(x: number, y: number): boolean {
    const t = this.tool;
    if (!t) return false;
    if (this.probe && this.why(t)) { this.note = this.why(t); this.show(); return true; }
    const code = TOOLS.find(d => d[0] === t)![2];
    this.pending = [t, this.share, this.row];
    this.editor.edit(code, x, y, this.share, this.row);
    return true;
  }

  /** Over the map, with its corner at (ox, oy) and a square `s` pixels: the patch the tool would make under the cursor,
   * a circle of the share's area of the cell about the sample under the cursor (the patch's ground is the share of the
   * cell's samples nearest that sample, the core's `patch_ground`), cut to the cell; for fire, the cell that would be struck. Grey where it cannot be done. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    this.drawnAt = [ox, oy, s];
    const t = this.tool, at = this.at, p = this.probe;
    if (!t || !at) return;
    const no = this.why(t) !== '';
    const ink = no ? 'rgba(120,120,120,.95)' : t === 'fire' ? '#ff7a1a' : t === 'plant' ? '#7ed957' : '#fff';
    const [cx, cy] = [ox + at[0] * s, oy + at[1] * s];
    ctx.save();
    if (p) {
      const [w, n, e, so] = p.bounds, bx = ox + w * s, by = oy + n * s, bw = (e - w) * s, bh = (so - n) * s;
      ctx.setLineDash([4, 4]); ctx.lineWidth = 1; ctx.strokeStyle = 'rgba(255,255,255,.7)';
      ctx.strokeRect(bx, by, bw, bh);
      ctx.setLineDash([]);
      if (t === 'fire') {
        ctx.lineWidth = 2.5; ctx.strokeStyle = ink; ctx.strokeRect(bx + 1, by + 1, bw - 2, bh - 2);
      } else {
        const r = Math.sqrt((this.share * (e - w) * (so - n)) / Math.PI) * s;
        ctx.beginPath(); ctx.rect(bx, by, bw, bh); ctx.clip();
        ctx.beginPath(); ctx.arc(ox + p.at[0] * s, oy + p.at[1] * s, r, 0, 2 * Math.PI);
        ctx.fillStyle = no ? 'rgba(120,120,120,.25)' : t === 'plant' ? 'rgba(126,217,87,.22)' : 'rgba(255,255,255,.18)';
        ctx.fill();
        ctx.lineWidth = 2; ctx.strokeStyle = 'rgba(0,0,0,.45)'; ctx.stroke();
        ctx.lineWidth = 1.5; ctx.strokeStyle = ink; ctx.stroke();
      }
    }
    // The tool's mark at the cursor.
    ctx.restore();
    ctx.fillStyle = ink; ctx.strokeStyle = 'rgba(0,0,0,.5)'; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.arc(cx, cy, 3, 0, 2 * Math.PI); ctx.fill(); ctx.stroke();
  }

  /** The panel: the tools, the choices of the tool on, and a line of what it does here or why it cannot. */
  private show(): void {
    const el = this.el, t = this.tool;
    el.replaceChildren();
    const seg = document.createElement('div');
    seg.className = 'seg tools';
    for (const [tool, label] of TOOLS) {
      const b = document.createElement('button'), why = this.why(tool);
      b.textContent = label; b.classList.toggle('on', t === tool); b.classList.toggle('no', why !== '');
      b.title = why || '';
      b.onclick = () => { this.tool = t === tool ? null : tool; this.note = ''; this.show(); this.redraw(); };
      seg.append(b);
    }
    el.append(seg);
    const line = (cls: string, html: string) => { const d = document.createElement('p'); d.className = cls; d.innerHTML = html; el.append(d); return d; };
    if (t === 'clear' || t === 'plant') {
      line('lbl', 'Size: a share of the cell under the cursor');
      const chips = document.createElement('div');
      chips.className = 'chips';
      for (const s of SHARES) {
        const b = document.createElement('button');
        b.textContent = pct(s); b.classList.toggle('on', s === this.share);
        b.onclick = () => { this.share = s; this.show(); this.redraw(); };
        chips.append(b);
      }
      el.append(chips);
      if (this.probe) line('hint', `${pct(this.share)} of this cell is ${hectares(this.share * this.probe.area)}.`);
    }
    if (t === 'plant' && this.kinds.length) {
      line('lbl', 'What to plant');
      const list = document.createElement('div');
      list.className = 'kinds';
      for (const k of this.kinds) {
        const [name, text] = plainKind(k), b = document.createElement('button');
        b.innerHTML = `<b>${name}</b><small>${text}</small>`; b.classList.toggle('on', k.row === this.row);
        b.onclick = () => { this.row = k.row; this.show(); };
        list.append(b);
      }
      el.append(list);
    }
    const why = t ? this.why(t) : '';
    if (t && why) line('why', why);
    else if (t === 'fire') line('hint', 'Click a place. Lightning strikes it in the coming month, and the fire spreads as far as the dry litter carries it.');
    else if (t) line('hint', t === 'clear' ? 'Click a place. The plants there die; their remains stay on the ground.' : 'Click a place. It is cleared, then seeded.');
    else if (this.probe && this.why('fire')) line('hint', `Fire here: ${this.why('fire')}`);
    else line('hint', 'Pick a tool, then click the map.');
    if (this.note) line('note', this.note);
  }
}
