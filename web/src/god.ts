// God mode on the live map (handover 11): the player picks a tool on the strip, then a place, and the world is edited
// there between two months (wasm/src/lib.rs `edit`, the core's `World::edit`). The tools themselves are a list
// (god-tools.ts `godTools`); this is what they share: the strip of their buttons, the card of the tool on (its brush
// size, its choices, what it does or why it cannot), a toast of what an edit did, the question to the core about the
// place under the cursor (`probe`), the tap that edits, and the brush drawn over the map with the reason beside it
// where the tool cannot be used there.
import { IconName, iconSvg } from './icons';

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

/** A tool's brush sizes: the stops of the size slider, the stop it starts at, and the words for a size at the place
 * probed. */
export interface Sizes { stops: number[]; first: number; text: (size: number, p: Probe | null) => string }
/** Where a tool's brush is drawn: the map's corner (css px) and the pixels a square takes, the place under the cursor
 * (squares), the core's answer about it, the size chosen, whether the tool cannot be used there, and the ink to draw
 * in (grey where it cannot). */
export interface Brush { ox: number; oy: number; s: number; at: [number, number]; probe: Probe | null; size: number; no: boolean; ink: string }
/** A tool of god mode. The strip shows the tools in the order of their list; a new tool is one more entry. */
export interface GodTool {
  /** Its key (its button's `data-tool`), its icon, its short name, and its colour on the map. */
  key: string; icon: IconName; label: string; ink: string;
  /** What a click on the map does, in a line. */
  hint: string;
  /** Its brush sizes, where the size slider applies to it. */
  sizes?: Sizes;
  /** Why it cannot be used at the place the core answered about (null: none yet), or ''. */
  why(p: Probe | null): string;
  /** Its own choices, shown under the size (`show` draws the card anew after one is picked). */
  choices?(show: () => void): HTMLElement;
  /** Edit the world at (x, y), squares, with the brush size `size`. */
  edit(x: number, y: number, size: number): void;
  /** What its last edit did, in a line, from the core's answer. */
  edited(r: Edited): string;
  /** Draw its brush over the map. */
  draw(ctx: CanvasRenderingContext2D, b: Brush): void;
}

/** The ink of a tool's brush where it cannot be used. */
const NO_INK = 'rgba(120,120,120,.95)';
/** The widest line of the box that says beside the cursor why a tool cannot be used there, css px. */
const LABEL_PX = 220;
/** How long the toast of an edit shows, ms. */
const TOAST_MS = 4000;
/** Least time between two questions about the place under the cursor, ms. */
const PROBE_MS = 80;

export class God {
  private tool: GodTool | null = null;
  /** The size slider's stop of each tool that has sizes, by key. */
  private stop = new Map<string, number>();
  /** The place under the cursor (squares), and the probe of the place last answered. */
  private at: [number, number] | null = null;
  private probe: Probe | null = null;
  /** Whether a question is out, and when the last was asked (ms). */
  private asking = false;
  private asked = 0;
  /** The map's corner and the pixels a square takes, as last drawn. */
  private drawnAt: [number, number, number] | null = null;
  /** When the toast of the last edit goes. */
  private toastTimer = 0;
  /** The tool of the edit last sent, for its answer (`edited`). */
  private pending: GodTool | null = null;

  constructor(private strip: HTMLElement, private card: HTMLElement, private toast: HTMLElement, private editor: Editor, private tools: GodTool[], private redraw: () => void) {
    strip.replaceChildren(...tools.map(t => {
      const b = document.createElement('button');
      b.className = 'ib'; b.dataset.tool = t.key; b.innerHTML = iconSvg(t.icon);
      b.setAttribute('aria-label', t.label);
      b.onclick = () => { this.tool = this.tool === t ? null : t; this.show(); this.redraw(); };
      return b;
    }));
    this.show();
  }

  /** Put the tool away and forget the place: a new world, or god mode closed. */
  reset(): void { this.tool = null; this.probe = null; this.toast.classList.remove('on'); this.show(); this.redraw(); }

  /** Put the tool on away: true if one was on. */
  putAway(): boolean {
    if (!this.tool) return false;
    this.tool = null; this.show(); this.redraw();
    return true;
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
    if (this.pending) this.say(this.pending.edited(r));
    this.probe = null;
    this.ask();
    this.show();
  }

  /** The brush size of tool `t` (0 for a tool without sizes). */
  private size(t: GodTool): number {
    return t.sizes ? t.sizes.stops[this.stop.get(t.key) ?? t.sizes.first] : 0;
  }

  /** A tap at (x, y) squares: the tool's edit there, true; false if no tool is on (the map picks the square). */
  tap(x: number, y: number): boolean {
    const t = this.tool;
    if (!t) return false;
    const why = this.probe ? t.why(this.probe) : '';
    if (why) { this.say(why); return true; }
    this.pending = t;
    t.edit(x, y, this.size(t));
    return true;
  }

  /** Over the map, with its corner at (ox, oy) and a square `s` pixels: the tool's brush under the cursor, its mark at
   * the cursor, and the reason beside it where the tool cannot be used there. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    this.drawnAt = [ox, oy, s];
    const t = this.tool, at = this.at;
    if (!t || !at) return;
    const why = t.why(this.probe), ink = why ? NO_INK : t.ink;
    ctx.save();
    t.draw(ctx, { ox, oy, s, at, probe: this.probe, size: this.size(t), no: why !== '', ink });
    ctx.restore();
    const [cx, cy] = [ox + at[0] * s, oy + at[1] * s];
    ctx.fillStyle = ink; ctx.strokeStyle = 'rgba(0,0,0,.5)'; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.arc(cx, cy, 3, 0, 2 * Math.PI); ctx.fill(); ctx.stroke();
    if (why) this.label(ctx, cx, cy, why);
  }

  /** The reason a tool cannot be used, in a box beside the cursor at (x, y), its words wrapped at `LABEL_PX`. */
  private label(ctx: CanvasRenderingContext2D, x: number, y: number, text: string): void {
    ctx.font = '12px system-ui, sans-serif';
    const lines: string[] = [];
    for (const word of text.split(' ')) {
      const last = lines.length - 1;
      if (last >= 0 && ctx.measureText(`${lines[last]} ${word}`).width <= LABEL_PX) lines[last] += ` ${word}`;
      else lines.push(word);
    }
    const w = Math.max(...lines.map(l => ctx.measureText(l).width)) + 16, h = lines.length * 16 + 10;
    const W = ctx.canvas.clientWidth || ctx.canvas.width, bx = x + 14 + w > W ? x - 14 - w : x + 14, by = y + 12;
    ctx.fillStyle = 'rgba(35,39,43,.88)';
    ctx.beginPath(); ctx.roundRect(bx, by, w, h, 6); ctx.fill();
    ctx.fillStyle = '#fff';
    lines.forEach((l, k) => ctx.fillText(l, bx + 8, by + 18 + 16 * k));
  }

  /** The strip's buttons (the tool on pressed, a tool that cannot be used here greyed with the reason), and the card:
   * the tool's name, its size, its choices, and a line of what it does here or why it cannot. */
  private show(): void {
    const t = this.tool;
    for (const b of this.strip.querySelectorAll<HTMLElement>('button')) {
      const tool = this.tools.find(d => d.key === b.dataset.tool)!, why = tool.why(this.probe);
      b.setAttribute('aria-pressed', String(tool === t));
      b.classList.toggle('no', why !== '');
      b.title = why ? `${tool.label}: ${why}` : tool.label;
    }
    const el = this.card;
    el.replaceChildren();
    const line = (cls: string, text: string) => { const d = document.createElement('p'); d.className = cls; d.textContent = text; el.append(d); };
    if (!t) { line('hint', 'Pick a tool, then click the map.'); return; }
    const head = document.createElement('div'), size = document.createElement('b');
    head.className = 'top';
    head.innerHTML = '<h2></h2>';
    head.querySelector('h2')!.textContent = t.label;
    head.append(size);
    el.append(head);
    if (t.sizes) this.slider(t, t.sizes, size);
    if (t.choices) el.append(t.choices(() => this.show()));
    const why = t.why(this.probe);
    line(why ? 'why' : 'hint', why || t.hint);
  }

  /** The size slider of tool `t` on the card, with the words for the size chosen at the place probed in `words`. */
  private slider(t: GodTool, sizes: Sizes, words: HTMLElement): void {
    const input = document.createElement('input');
    words.textContent = sizes.text(this.size(t), this.probe);
    Object.assign(input, { type: 'range', min: '0', max: String(sizes.stops.length - 1), step: '1', value: String(this.stop.get(t.key) ?? sizes.first) });
    input.setAttribute('aria-label', 'Brush size'); input.title = 'Brush size';
    input.oninput = () => {
      this.stop.set(t.key, +input.value);
      words.textContent = sizes.text(this.size(t), this.probe);
      this.redraw();
    };
    this.card.append(input);
  }

  /** What an edit did, or why it was not made, in a toast that fades after `TOAST_MS`. */
  private say(text: string): void {
    this.toast.textContent = text;
    this.toast.classList.add('on');
    clearTimeout(this.toastTimer);
    this.toastTimer = window.setTimeout(() => this.toast.classList.remove('on'), TOAST_MS);
  }
}
