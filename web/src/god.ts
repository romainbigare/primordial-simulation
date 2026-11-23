// God mode on the live map (handover 11): the player picks a tool on the strip and paints the world with a round
// brush: while the pointer is down, each cell under the brush is edited once (wasm/src/lib.rs `brush`, the core's
// `World::edit`), and when it lifts the stroke settles (`settle`: strikes burn at once). The tools themselves are a list
// (god-tools.ts `godTools`); this is what they share: the strip of their buttons, the card of the tool on (the brush's
// radius, its choices, what it does or why it cannot), the stroke, a toast of what a stroke did, the question to the
// core about the place under the cursor (`probe`), and the brush drawn over the map at its true size with the reason
// beside it where the tool cannot act.
import { IconName, iconSvg } from './icons';

/** What the core says of the cell under a place (wasm/src/lib.rs `probe`): the cell, its edges in squares (west, north,
 * east, south), the centre of its sample under the place (squares), its area and land (m²), its patches, and whether a
 * strike lights it ("yes", or the core's reason). */
export interface Probe { cell: number; bounds: [number, number, number, number]; at: [number, number]; area: number; land: number; patches: number; fire: string }
/** A starting kind the world's plan places (wasm/src/lib.rs `kinds`): its row, the core's name, kg per m² placed. */
export interface StartKind { row: number; name: string; kg: number }
/** What a brush did (wasm/src/lib.rs `brush`): each cell under it not yet edited in the stroke, the share of it under
 * the brush, whether each of its edits was made and the core's reason where not; whether any cell changed. */
export interface Brushed { cells: { cell: number; share: number; done: boolean[]; why: string }[]; changed: boolean }
/** What a stroke's settling did (wasm/src/lib.rs `settle`): each cell the flames of its strikes reached, and when, s
 * after the strike. */
export interface Settled { burnt: [number, number][] }
/** The worker's side of the tools: ask what lies under a place, paint, and end a stroke. */
export interface Editor { probe(x: number, y: number): void; brush(tool: number, x: number, y: number, radius: number, row: number): void; settle(): void }

/** A tool of god mode. The strip shows the tools in the order of their list; a new tool is one more entry. */
export interface GodTool {
  /** Its key (its button's `data-tool`), its icon, its short name, and its colour on the map. */
  key: string; icon: IconName; label: string; ink: string;
  /** How to use it, in a line. */
  hint: string;
  /** The core's brush tool it paints with (wasm/src/lib.rs `brush`), and the starting kind it places (-1: none). */
  code: number; row(): number;
  /** Why it cannot act at the place the core answered about (null: none yet), or ''. */
  why(p: Probe | null): string;
  /** The core's reason a cell was not edited (`Brushed`), in plain words. */
  reason(why: string): string;
  /** Its own choices, shown under the radius (`show` draws the card anew after one is picked). */
  choices?(show: () => void): HTMLElement;
  /** What a stroke did, in a line: the cells it edited, and the plain reason most of the others were not. */
  told(done: number, why: string): string;
}
/** What the page gives god mode: its strip, card and toast; the map drawn anew; that a tool was picked (the world
 * pauses); the side of a square, m; and the cells a stroke burnt with when (`Settled`). */
export interface GodPage {
  strip: HTMLElement; card: HTMLElement; toast: HTMLElement;
  redraw(): void; picked(): void; squareM(): number; burnt(burnt: [number, number][]): void;
}

/** The brush's radii, m (the slider's stops), and the one it starts at. */
const RADII_M = [150, 300, 600, 1000, 2000, 3000], FIRST_RADIUS = 2;
/** The least time between two paints of a stroke, ms, and the least move of the brush's centre between them, a share
 * of its radius: cells are edited once a stroke, so a paint where the brush has not moved adds nothing. */
const PAINT_MS = 90, PAINT_MOVE = 0.25;
/** The ink of the brush where the tool cannot act. */
const NO_INK = 'rgba(120,120,120,.95)';
/** The widest line of the box that says beside the cursor why a tool cannot act there, css px. */
const LABEL_PX = 220;
/** How long the toast of a stroke shows, ms. */
const TOAST_MS = 5000;
/** Least time between two questions about the place under the cursor, ms. */
const PROBE_MS = 80;

const radiusText = (m: number) => (m >= 1000 ? `${m / 1000} km` : `${m} m`);

/** A stroke under way: the cells it edited, its reasons for those it did not, the place and time of its last paint
 * (squares, ms), and where the brush is now. */
interface Stroke { tool: GodTool; done: number; whys: Map<string, number>; last: [number, number, number]; now: [number, number] }

export class God {
  private tool: GodTool | null = null;
  private radius = RADII_M[FIRST_RADIUS];
  private stroke: Stroke | null = null;
  /** The plain reason the last paint edited none of the cells under it ('': it did, or none was asked). */
  private lastWhy = '';
  /** The place under the cursor (squares), and the probe of the place last answered. */
  private at: [number, number] | null = null;
  private probe: Probe | null = null;
  /** Whether a question is out, and when the last was asked (ms). */
  private asking = false;
  private asked = 0;
  /** The map's corner and the pixels a square takes, as last drawn. */
  private drawnAt: [number, number, number] | null = null;
  /** When the toast of the last stroke goes. */
  private toastTimer = 0;

  constructor(private page: GodPage, private editor: Editor, private tools: GodTool[]) {
    page.strip.replaceChildren(...tools.map(t => {
      const b = document.createElement('button');
      b.className = 'ib'; b.dataset.tool = t.key; b.innerHTML = iconSvg(t.icon);
      b.setAttribute('aria-label', t.label);
      b.onclick = () => { this.pick(this.tool === t ? null : t); };
      return b;
    }));
    this.show();
  }

  /** Put the tool away and forget the place: a new world, or god mode closed. */
  reset(): void { this.stroke = null; this.probe = null; this.asking = false; this.page.toast.classList.remove('on'); this.pick(null); }

  /** Put the tool on away: true if one was on. */
  putAway(): boolean {
    if (!this.tool) return false;
    this.pick(null);
    return true;
  }

  /** Take tool `t` (null: none); the world pauses while one is on. */
  private pick(t: GodTool | null): void {
    this.tool = t; this.lastWhy = '';
    if (t) this.page.picked();
    this.show(); this.page.redraw();
  }

  /** Follow the pointer on `cv`: the place under it is asked of the core. */
  listen(cv: HTMLCanvasElement): void {
    cv.addEventListener('pointermove', e => {
      const v = this.drawnAt;
      if (!v) return;
      this.at = [(e.offsetX - v[0]) / v[2], (e.offsetY - v[1]) / v[2]];
      this.ask();
      if (this.tool) this.page.redraw();
    });
    cv.addEventListener('pointerleave', () => { this.at = null; if (this.tool) this.page.redraw(); });
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
    if (this.tool) this.page.redraw();
    if (!was || was.cell !== p.cell || was.fire !== p.fire || was.land !== p.land || was.patches !== p.patches) { this.show(); this.page.redraw(); }
  }

  /** A press at (x, y) squares: the stroke of the tool on starts there, true; false if no tool is on (the map moves). */
  start(x: number, y: number): boolean {
    const t = this.tool;
    if (!t) return false;
    this.at = [x, y];
    this.stroke = { tool: t, done: 0, whys: new Map(), last: [x, y, 0], now: [x, y] };
    this.paint();
    return true;
  }

  /** The pointer of the stroke moved to (x, y) squares: a paint there, once it moved far and long enough. */
  move(x: number, y: number): void {
    const st = this.stroke;
    if (!st) return;
    st.now = [x, y];
    const [lx, ly, lt] = st.last, far = Math.hypot(x - lx, y - ly) >= (PAINT_MOVE * this.radius) / this.page.squareM();
    if (far && performance.now() - lt >= PAINT_MS) this.paint();
  }

  /** The stroke ends: a last paint where the brush moved since, and it settles. */
  end(): void {
    const st = this.stroke;
    if (!st) return;
    if (st.now[0] !== st.last[0] || st.now[1] !== st.last[1]) this.paint();
    this.editor.settle();
  }

  /** Paint with the stroke's tool where its brush is now. */
  private paint(): void {
    const st = this.stroke!, [x, y] = st.now;
    st.last = [x, y, performance.now()];
    this.editor.brush(st.tool.code, x, y, this.radius, st.tool.row());
  }

  /** The core's answer to a paint: its cells counted to the stroke; the reason, where it edited none of them. */
  brushed(r: Brushed): void {
    const st = this.stroke;
    if (!st) return;
    const whys = new Map<string, number>();
    for (const c of r.cells) {
      if (c.done.every(d => d)) st.done++;
      else if (c.why) whys.set(c.why, (whys.get(c.why) ?? 0) + 1);
    }
    for (const [why, n] of whys) st.whys.set(why, (st.whys.get(why) ?? 0) + n);
    if (r.cells.length) this.lastWhy = r.cells.some(c => c.done.every(d => d)) ? '' : st.tool.reason(most(whys));
    this.page.redraw();
  }

  /** The core's answer to the stroke's end: what it did, in the toast; the cells it burnt, to the page. */
  settled(r: Settled): void {
    const st = this.stroke;
    this.stroke = null;
    if (!st) return;
    this.say(st.tool.told(st.done, st.tool.reason(most(st.whys))));
    if (r.burnt.length) this.page.burnt(r.burnt);
  }

  /** Over the map, with its corner at (ox, oy) and a square `s` pixels: the brush at the cursor at its true size, and
   * the reason beside it where the tool cannot act there. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    this.drawnAt = [ox, oy, s];
    const t = this.tool, at = this.at;
    if (!t || !at) return;
    const why = this.lastWhy || t.why(this.probe), ink = why ? NO_INK : t.ink;
    const [cx, cy] = [ox + at[0] * s, oy + at[1] * s], r = (this.radius / this.page.squareM()) * s;
    ctx.save();
    ctx.beginPath(); ctx.arc(cx, cy, r, 0, 2 * Math.PI);
    ctx.globalAlpha = this.stroke ? 0.3 : 0.15; ctx.fillStyle = ink; ctx.fill();
    ctx.globalAlpha = 1; ctx.lineWidth = 3; ctx.strokeStyle = 'rgba(0,0,0,.45)'; ctx.stroke();
    ctx.lineWidth = 1.5; ctx.strokeStyle = ink; ctx.stroke();
    ctx.fillStyle = ink; ctx.strokeStyle = 'rgba(0,0,0,.5)'; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.arc(cx, cy, 3, 0, 2 * Math.PI); ctx.fill(); ctx.stroke();
    ctx.restore();
    if (why) this.label(ctx, cx, cy, why);
  }

  /** The reason a tool cannot act, in a box beside the cursor at (x, y), its words wrapped at `LABEL_PX`. */
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

  /** The strip's buttons (the tool on pressed, a tool that cannot act here greyed with the reason), and the card: the
   * tool's name, the brush's radius, its choices, and a line of how to use it or why it cannot act here. */
  private show(): void {
    const t = this.tool;
    for (const b of this.page.strip.querySelectorAll<HTMLElement>('button')) {
      const tool = this.tools.find(d => d.key === b.dataset.tool)!, why = tool.why(this.probe);
      b.setAttribute('aria-pressed', String(tool === t));
      b.classList.toggle('no', why !== '');
      b.title = why ? `${tool.label}: ${why}` : tool.label;
    }
    const el = this.page.card;
    el.replaceChildren();
    const line = (cls: string, text: string) => { const d = document.createElement('p'); d.className = cls; d.textContent = text; el.append(d); };
    if (!t) { line('hint', 'Pick a tool, then drag on the map.'); return; }
    const head = document.createElement('div'), size = document.createElement('b');
    head.className = 'top';
    head.innerHTML = '<h2></h2>';
    head.querySelector('h2')!.textContent = t.label;
    head.append(size);
    el.append(head);
    this.slider(size);
    if (t.choices) el.append(t.choices(() => this.show()));
    const why = t.why(this.probe);
    line(why ? 'why' : 'hint', why || t.hint);
  }

  /** The brush's radius slider on the card, the radius in `words`. */
  private slider(words: HTMLElement): void {
    const input = document.createElement('input');
    words.textContent = `radius ${radiusText(this.radius)}`;
    Object.assign(input, { type: 'range', min: '0', max: String(RADII_M.length - 1), step: '1', value: String(RADII_M.indexOf(this.radius)) });
    input.setAttribute('aria-label', 'Brush radius'); input.title = 'Brush radius';
    input.oninput = () => {
      this.radius = RADII_M[+input.value];
      words.textContent = `radius ${radiusText(this.radius)}`;
      this.page.redraw();
    };
    this.page.card.append(input);
  }

  /** What a stroke did in a toast that fades after `TOAST_MS`. */
  private say(text: string): void {
    const toast = this.page.toast;
    toast.textContent = text;
    toast.classList.add('on');
    clearTimeout(this.toastTimer);
    this.toastTimer = window.setTimeout(() => toast.classList.remove('on'), TOAST_MS);
  }
}

/** The key counted most in `counts` ('' for none). */
function most(counts: Map<string, number>): string {
  let best = '', n = 0;
  for (const [k, c] of counts) if (c > n) { best = k; n = c; }
  return best;
}
