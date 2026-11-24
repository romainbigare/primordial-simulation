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
/** What a brush did (wasm/src/lib.rs `brush`): of a planting, each cell newly under it, the share of it under the
 * brush, whether each of its edits was made and the core's reason where not, and whether the kind planted stands
 * grown somewhere (`grown`; else it is planted young); of heat, each cell under it, its share, how much of the heat
 * that lights its remains it holds (`progress`, 0 to 1) and why none would light; whether any cell changed. (A brush
 * of the ground answers its cells as numbers alone.) */
export interface Brushed { cells: { cell: number; share: number; done?: boolean[]; progress?: number; why: string }[]; changed: boolean; grown?: boolean }
/** What a stroke's settling did (wasm/src/lib.rs `settle`): whether the ground was built again, and each cell the
 * flames of its strikes reached, and when, s after the strike. */
export interface Settled { ground: boolean; burnt: [number, number][] }
/** The worker's side of the tools (live.ts `Live`): ask what lies under a place, paint, end a stroke, and undo the last. */
export interface Editor {
  probe(x: number, y: number, answer: (p: Probe) => void): void; brush(tool: number, x: number, y: number, radius: number, amount: number, row: number): void;
  settle(): void; undo(): void;
}

/** A tool of god mode. The strip shows the tools in the order of their list; a new tool is one more entry. */
export interface GodTool {
  /** Its key (its button's `data-tool`), its icon, its short name, and its colour on the map. */
  key: string; icon: IconName; label: string; ink: string;
  /** How to use it, in a line. */
  hint: string;
  /** The core's brush tool it paints with (wasm/src/lib.rs `brush`), and the starting kind it places (-1: none). */
  code: number; row(): number;
  /** What it adds at the brush's centre each second it is held, in its core tool's unit (m of ground, up or down where
   * less than 0; J/m² of heat): it paints again and again while the pointer is down, held still or not. */
  rate?: number;
  /** Why it cannot act at the place the core answered about (null: none yet), or ''. */
  why(p: Probe | null): string;
  /** The core's reason a cell was not edited (`Brushed`), in plain words. */
  reason(why: string): string;
  /** Its own choices, shown under the radius (`show` draws the card anew after one is picked). */
  choices?(show: () => void): HTMLElement;
  /** What a stroke did, in a line: the cells it edited (lit, for heat), the plain reason most of the others were not,
   * and whether what it planted stood grown. */
  told(done: number, why: string, grown: boolean): string;
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
 * of its radius: cells are edited once a stroke, so a paint where the brush has not moved adds nothing. A tool with a
 * rate paints every `PAINT_MS` while held. */
const PAINT_MS = 90, PAINT_MOVE = 0.25;
/** The ink of the brush where the tool cannot act: a dull red. */
const NO_INK = 'rgba(196,76,60,.95)';
/** The width of the ring that shows the heat held under the brush, css px. */
const PROGRESS_PX = 5;
/** The widest line of the box that says beside the cursor why a tool cannot act there, css px. */
const LABEL_PX = 220;
/** How long the toast of a stroke shows, ms. */
const TOAST_MS = 5000;
/** Least time between two questions about the place under the cursor, ms. */
const PROBE_MS = 80;

const radiusText = (m: number) => (m >= 1000 ? `${m / 1000} km` : `${m} m`);

/** A stroke under way: the cells it edited (for a tool with a rate, the paints that moved the ground), its reasons for
 * those it did not, the place and time of its last paint (squares, ms), where the brush is now, the timer that paints
 * while a tool with a rate is held, and whether a paint of it waits for the core's answer (a tool with a rate paints
 * again only once answered, with the time since: the core may take longer over a paint than the timer's step). */
interface Stroke {
  tool: GodTool; done: number; whys: Map<string, number>; last: [number, number, number]; now: [number, number]; held: number; waiting: boolean;
  /** Of heat: the cells lit so far, and how much of the heat that lights them the cells under the brush hold, their
   * mean by share over those that can light (-1: no heat asked). Of a planting: whether its kind stood grown. */
  lit: Set<number>; progress: number; grown: boolean;
}

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
  /** Whether a stroke settled that can be undone (the core keeps one), and the button that undoes it. */
  private undoable = false;
  private undoButton: HTMLButtonElement;

  /** `editor`: the live world's, where there is one. */
  constructor(private page: GodPage, private editor: () => Editor | null, private tools: GodTool[]) {
    page.strip.replaceChildren(...tools.map(t => {
      const b = document.createElement('button');
      b.className = 'ib'; b.dataset.tool = t.key; b.innerHTML = iconSvg(t.icon);
      b.setAttribute('aria-label', t.label);
      b.onclick = () => { this.pick(this.tool === t ? null : t); };
      return b;
    }));
    const u = this.undoButton = document.createElement('button');
    u.className = 'ib undo'; u.innerHTML = iconSvg('undo'); u.title = 'Undo the last stroke (Ctrl+Z)';
    u.setAttribute('aria-label', 'Undo');
    u.onclick = () => this.undo();
    page.strip.append(u);
    this.show();
  }

  /** Put the tool away and forget the place and the stroke to undo: a new world, or god mode closed. */
  reset(): void { this.stroke = null; this.probe = null; this.asking = false; this.undoable = false; this.page.toast.classList.remove('on'); this.pick(null); }

  /** Undo the last stroke, where there is one and no stroke is under way. */
  undo(): void {
    if (!this.undoable || this.stroke) return;
    this.undoable = false;
    this.editor()?.undo();
    this.show();
  }

  /** The core's answer to an undo. */
  undone(done: boolean): void { if (done) this.say('Undone: the world is as before the last stroke.'); }

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
    this.editor()?.probe(...this.at, p => this.probed(p));
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
    const st: Stroke = this.stroke = { tool: t, done: 0, whys: new Map(), last: [x, y, performance.now()], now: [x, y], held: 0, waiting: false, lit: new Set(), progress: -1, grown: true };
    if (t.rate) st.held = window.setInterval(() => { if (!st.waiting) this.paint(); }, PAINT_MS);
    else this.paint();
    return true;
  }

  /** The pointer of the stroke moved to (x, y) squares: a paint there, once it moved far and long enough. */
  move(x: number, y: number): void {
    const st = this.stroke;
    if (!st) return;
    st.now = [x, y];
    if (st.tool.rate) return;
    const [lx, ly, lt] = st.last, far = Math.hypot(x - lx, y - ly) >= (PAINT_MOVE * this.radius) / this.page.squareM();
    if (far && performance.now() - lt >= PAINT_MS) this.paint();
  }

  /** The stroke ends: a last paint where the brush moved since, and it settles. */
  end(): void {
    const st = this.stroke;
    if (!st) return;
    clearInterval(st.held);
    if (st.tool.rate || st.now[0] !== st.last[0] || st.now[1] !== st.last[1]) this.paint();
    this.editor()?.settle();
  }

  /** Paint with the stroke's tool where its brush is now; a tool with a rate by what the time since its last paint
   * adds. */
  private paint(): void {
    const st = this.stroke!, [x, y] = st.now, now = performance.now(), amount = ((st.tool.rate ?? 0) * (now - st.last[2])) / 1000;
    st.last = [x, y, now];
    st.waiting = true;
    this.editor()?.brush(st.tool.code, x, y, this.radius, amount, st.tool.row());
  }

  /** The core's answer to a paint: its cells counted to the stroke; the reason, where it edited none of them. */
  brushed(r: Brushed): void {
    const st = this.stroke;
    if (!st) return;
    st.waiting = false;
    if (r.grown === false) st.grown = false;
    const heat = r.cells.filter(c => c.progress !== undefined);
    if (heat.length) this.heated(st, heat);
    else if (st.tool.rate) st.done += +r.changed;
    else {
      const whys = new Map<string, number>();
      for (const c of r.cells) {
        if (c.done!.every(d => d)) st.done++;
        else if (c.why) whys.set(c.why, (whys.get(c.why) ?? 0) + 1);
      }
      for (const [why, n] of whys) st.whys.set(why, (st.whys.get(why) ?? 0) + n);
      if (r.cells.length) this.lastWhy = r.cells.some(c => c.done!.every(d => d)) ? '' : st.tool.reason(most(whys));
    }
    this.page.redraw();
  }

  /** Heat's answer: the cells lit so far, the progress under the brush, and why none would light there. */
  private heated(st: Stroke, cells: Brushed['cells']): void {
    const can = cells.filter(c => !c.why), share = can.reduce((n, c) => n + c.share, 0);
    for (const c of can) if (c.progress! >= 1) st.lit.add(c.cell);
    st.done = st.lit.size;
    st.progress = share > 0 ? can.reduce((n, c) => n + c.share * c.progress!, 0) / share : 0;
    st.whys = new Map();
    for (const c of cells) if (c.why) st.whys.set(c.why, (st.whys.get(c.why) ?? 0) + 1);
    this.lastWhy = can.length ? '' : st.tool.reason(most(st.whys));
  }

  /** The core's answer to the stroke's end: what it did, in the toast; the cells it burnt, to the page. */
  settled(r: Settled): void {
    const st = this.stroke;
    this.stroke = null;
    if (!st) return;
    this.undoable = true;
    this.show();
    this.say(st.tool.told(st.done, st.tool.reason(most(st.whys)), st.grown));
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
    // Heat held: a ring that fills as the cells under the brush near lighting, and a glow that grows with it.
    const p = this.stroke?.progress ?? -1;
    if (p > 0) {
      ctx.globalAlpha = 0.35 * p; ctx.fillStyle = ink; ctx.fill();
      ctx.globalAlpha = 1; ctx.lineWidth = PROGRESS_PX; ctx.strokeStyle = ink;
      ctx.beginPath(); ctx.arc(cx, cy, r, -Math.PI / 2, -Math.PI / 2 + 2 * Math.PI * Math.min(p, 1)); ctx.stroke();
    }
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
   * tool's name, the brush's radius, its choices, and a line of how to use it or why it cannot act here; the undo
   * button, where a stroke can be undone. */
  private show(): void {
    const t = this.tool;
    this.undoButton.disabled = !this.undoable;
    for (const b of this.page.strip.querySelectorAll<HTMLElement>('button[data-tool]')) {
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
