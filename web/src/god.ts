// God mode on the live map (handover 11): the player picks a tool on the strip and edits the world with a round brush
// (wasm/src/lib.rs `brush`, the core's `World::edit`): a tool paints as the pointer moves, paints again and again while
// it is held (the ground, heat), or makes one thing where the press begins (a lake); when the pointer lifts the stroke
// settles (`settle`: the ground built again, what was heated burns). The tools themselves are a list (god-tools.ts
// `godTools`); this is what they share: the strip (done, the tools named, the undo of the last stroke), the card of the
// brush's sizes and the tool's choices, the strokes and the core's answers to them, a short message of what a stroke
// did or how to use a tool, the question to the core about the place under the cursor (`probe`), and the brush drawn
// over the map at its true size (god-draw.ts).
import { IconName, iconSvg } from './icons';
import type { Brushed, Editor, GodPage, GodTool, Probe, Settled } from './god-types';
import { drawBrush, drawFlames, drawReason, drawSizeTag } from './god-draw';

/** The brush's radii, m (its size chips), and the one it starts at. */
const RADII_M = [150, 300, 600, 1000, 2000, 3000], FIRST_RADIUS = 2;
/** The least time between two paints of a stroke, ms, and the least move of the brush's centre between them, a share
 * of its radius: cells are edited once a stroke, so a paint where the brush has not moved adds nothing. A tool with a
 * rate paints every `PAINT_MS` while held. */
const PAINT_MS = 90, PAINT_MOVE = 0.25;
/** A tool with a rate paints only once the pointer has been held this long, ms: a tap does nothing. */
const HOLD_MS = 220;
/** The ink of the brush where the tool cannot act: a dull red. */
const NO_INK = 'rgba(196,76,60,.95)';
/** How long a message shows, ms, and one with a button to press. */
const TOAST_MS = 3000, TOAST_ACT_MS = 7000;
/** How long the brush shows in the middle of the map after its size is picked, ms. */
const SIZE_SHOWN_MS = 1500;
/** The brush shown after its size is picked stands this far down the map, a share of its height (above the bars). */
const SIZE_SHOWN_AT = 0.42;
/** Least time between two questions about the place under the cursor, ms. */
const PROBE_MS = 80;

const radiusText = (m: number) => (m >= 1000 ? `${m / 1000} km` : `${m} m`);
/** An area of m² in km², to two figures. */
const km2Text = (m2: number) => { const k = m2 / 1e6; return `${k >= 10 ? k.toFixed(0) : k >= 1 ? k.toFixed(1) : k.toFixed(2)} km²`; };

/** A stroke under way: the cells it edited (for a tool with a rate, the paints that moved the ground), its reasons for
 * those it did not, the place and time of its last paint (squares, ms), when it began (ms), where the brush is now, the
 * paints sent, the timer that paints while a tool with a rate is held, and whether a paint of it waits for the core's
 * answer (a tool with a rate paints again only once answered, with the time since: the core may take longer over a
 * paint than the timer's step). */
interface Stroke {
  tool: GodTool; done: number; whys: Map<string, number>; last: [number, number, number]; began: number; now: [number, number]; sent: number; held: number; waiting: boolean;
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
  /** The timer of the question waiting for `PROBE_MS` to pass (0: none). */
  private askTimer = 0;
  /** The strokes ended whose settling the core has not answered yet, oldest first: the core answers in the order it was
   * asked, so the answers to their paints and their settling come before those of the stroke under way. */
  private ended: Stroke[] = [];
  /** The map's corner and the pixels a square takes, as last drawn. */
  private drawnAt: [number, number, number] | null = null;
  /** When the message goes, and when the brush picked last stops showing in the middle of the map (ms). */
  private toastTimer = 0;
  private sizeShownTill = 0;
  /** Whether a stroke settled that can be undone (the core keeps one), and the button that undoes it. */
  private undoable = false;
  private undoButton: HTMLButtonElement;

  /** `editor`: the live world's, where there is one. */
  constructor(private page: GodPage, private editor: () => Editor | null, private tools: GodTool[]) {
    const button = (icon: IconName, label: string, title: string, act: () => void) => {
      const b = document.createElement('button');
      b.className = 'ib tool'; b.innerHTML = `${iconSvg(icon)}<span class="lb">${label}</span>`;
      b.title = title; b.setAttribute('aria-label', label); b.onclick = act;
      return b;
    };
    const done = button('close', 'Done', 'Done: close god mode', () => page.close());
    done.classList.add('done');
    this.undoButton = button('undo', 'Undo', 'Undo the last stroke (Ctrl+Z)', () => this.undo());
    page.strip.replaceChildren(done, ...tools.map(t => {
      const b = button(t.icon, t.label, t.label, () => this.pick(this.tool === t ? null : t));
      b.dataset.tool = t.key;
      return b;
    }), this.undoButton);
    this.show();
  }

  /** Put the tool away and forget the place and the stroke to undo: a new world, or god mode closed. */
  reset(): void {
    // (A stroke held is ended: its timer stops, and the core is told, so its next stroke is a new one.)
    if (this.stroke) { clearInterval(this.stroke.held); if (this.stroke.sent) this.editor()?.settle(); }
    this.stroke = null; this.ended = []; this.probe = null; this.asking = false; this.undoable = false;
    this.page.toast.classList.remove('on'); this.pick(null);
  }

  /** God mode opened: say what to do first. */
  opened(): void { this.say('Pick a tool to change the world.'); }

  /** Whether a tool is on. */
  toolOn(): boolean { return this.tool !== null; }

  /** Whether a stroke is under way or its settling still to come: the world does not play meanwhile. */
  busy(): boolean { return this.stroke !== null || this.ended.length > 0; }

  /** Undo the last stroke, where there is one and no stroke is under way. */
  undo(): void {
    if (!this.undoable || this.busy()) return;
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

  /** Take tool `t` (null: none), and say how to use it; the world pauses while one is on. */
  private pick(t: GodTool | null): void {
    this.tool = t; this.lastWhy = '';
    if (t) { this.page.picked(); this.say(t.hint); }
    this.show(); this.page.redraw();
  }

  /** Follow the pointer on `cv`: the place under it is asked of the core. */
  listen(cv: HTMLCanvasElement): void {
    const follow = (e: PointerEvent) => {
      const v = this.drawnAt;
      if (!v) return;
      this.at = [(e.offsetX - v[0]) / v[2], (e.offsetY - v[1]) / v[2]];
      this.ask();
      if (this.tool) this.page.redraw();
    };
    cv.addEventListener('pointermove', follow);
    cv.addEventListener('pointerdown', follow);
    cv.addEventListener('pointerleave', () => { this.at = null; if (this.tool) this.page.redraw(); });
  }

  /** Ask the core about the place under the cursor, at most once every `PROBE_MS` and one question at a time. */
  private ask(): void {
    if (!this.at || this.asking) return;
    const wait = this.asked + PROBE_MS - performance.now();
    if (wait > 0) { this.askTimer ||= window.setTimeout(() => { this.askTimer = 0; this.ask(); }, wait); return; }
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

  /** A press at (x, y) squares: the stroke of the tool on starts there, true; false if no tool is on (the map moves).
   * A tool with a rate begins to paint once held `HOLD_MS`; a tool that moves begins at once. */
  start(x: number, y: number): boolean {
    const t = this.tool;
    if (!t) return false;
    this.page.picked();
    this.at = [x, y];
    const now = performance.now();
    const st: Stroke = this.stroke = { tool: t, done: 0, whys: new Map(), last: [x, y, now], began: now, now: [x, y], sent: 0, held: 0, waiting: false, lit: new Set(), progress: -1, grown: true };
    if (t.rate) st.held = window.setTimeout(() => { st.held = window.setInterval(() => { if (!st.waiting) this.paint(); }, PAINT_MS); this.paint(); }, HOLD_MS);
    else this.paint();
    return true;
  }

  /** The pointer of the stroke moved to (x, y) squares: a paint there, once it moved far and long enough. */
  move(x: number, y: number): void {
    const st = this.stroke;
    if (!st) return;
    st.now = [x, y];
    if (st.tool.rate || st.tool.amount) return;
    const [lx, ly, lt] = st.last, far = Math.hypot(x - lx, y - ly) >= (PAINT_MOVE * this.radius) / this.page.squareM();
    if (far && performance.now() - lt >= PAINT_MS) this.paint();
  }

  /** The stroke ends: a last paint where the brush moved since, and it settles; a tool with a rate let go before it
   * painted (a tap) does nothing. */
  end(): void {
    const st = this.stroke;
    if (!st) return;
    clearInterval(st.held);
    if (st.tool.rate && !st.sent) { this.stroke = null; this.page.redraw(); return; }
    if (st.tool.rate || (!st.tool.amount && (st.now[0] !== st.last[0] || st.now[1] !== st.last[1]))) this.paint();
    this.editor()?.settle();
    this.ended.push(st);
    this.stroke = null;
  }

  /** Paint with the stroke's tool where its brush is now: by its amount, or for a tool with a rate by what the time
   * since its last paint adds, its rate doubling every `doubling` s it has been held. */
  private paint(): void {
    const st = this.stroke!, t = st.tool, [x, y] = st.now, now = performance.now();
    const rate = (t.rate ?? 0) * (t.doubling ? 2 ** ((now - st.began) / 1000 / t.doubling) : 1);
    const amount = t.amount ? t.amount() : (rate * (now - st.last[2])) / 1000;
    st.last = [x, y, now];
    st.waiting = true; st.sent++;
    this.editor()?.brush(t.code, x, y, this.radius, amount, t.row());
  }

  /** The core's answer to a paint: its cells counted to its stroke; the reason, where it edited none of them. */
  brushed(r: Brushed): void {
    const st = this.ended[0] ?? this.stroke;
    if (!st) return;
    st.waiting = false;
    if (r.grown === false) st.grown = false;
    const heat = r.cells.filter(c => c.progress !== undefined);
    if (heat.length) this.heated(st, heat);
    // (The ground's tools and a lake answer their cells as numbers.)
    else if (typeof r.cells[0] !== 'object') st.done += +r.changed;
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

  /** The core's answer to the stroke's end: what it did, said; the cells it burnt, to the page, which tells the area
   * burnt once the frame after it came (`spread`). */
  settled(r: Settled): void {
    const st = this.ended.shift();
    if (!st) return;
    this.undoable = true;
    this.show();
    if (r.burnt.length) this.page.burnt(r.burnt);
    else this.say(st.tool.told(st.done, st.tool.reason(most(st.whys)), st.grown));
  }

  /** The land a burn's flames took, m², known once the frame after it came: said, with the world to play on. */
  spread(m2: number): void {
    this.say(`${m2 > 0 ? `Fire burnt ${km2Text(m2)}.` : 'Fire took.'} Play to let the world go on.`, ['Play', () => this.page.play()]);
  }

  /** Over the map, with its corner at (ox, oy) and a square `s` pixels: the brush at its true size where the cursor is
   * (or in the middle of the map just after its size was picked), with the flames of the heat it holds, and the reason
   * beside it where the tool cannot act there. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    this.drawnAt = [ox, oy, s];
    const t = this.tool, now = performance.now(), sizeShown = !this.stroke && now < this.sizeShownTill;
    if (!t || (!this.at && !sizeShown)) return;
    const why = this.lastWhy || (sizeShown ? '' : t.why(this.probe)), ink = why ? NO_INK : t.ink;
    const [cx, cy] = sizeShown ? [ctx.canvas.clientWidth / 2, ctx.canvas.clientHeight * SIZE_SHOWN_AT] : [ox + this.at![0] * s, oy + this.at![1] * s];
    const r = (this.radius / this.page.squareM()) * s, progress = this.stroke?.progress ?? -1;
    drawBrush(ctx, cx, cy, r, ink, this.stroke !== null, progress);
    if (this.stroke && progress > 0 && !why) drawFlames(ctx, cx, cy, r, progress, now);
    if (sizeShown) drawSizeTag(ctx, cx, cy, r, radiusText(this.radius));
    if (why) drawReason(ctx, cx, cy, r, why);
  }

  /** The strip's buttons (the tool on pressed, a tool that cannot act here greyed with the reason, the undo greyed
   * where no stroke can be undone), and the card: its title, the brush's sizes, the tool's choices, and a line of how
   * to use it or why it cannot act here. */
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
    const title = document.createElement('h2');
    title.textContent = t?.label ?? 'God mode';
    el.append(title);
    if (!t) { line('hint', 'Pick a tool, then use it on the map.'); return; }
    el.append(this.sizes());
    if (t.choices) el.append(t.choices(() => this.show()));
    const why = t.why(this.probe);
    line(why ? 'why' : 'hint', why || t.hint);
  }

  /** The brush's sizes as chips; a size picked shows the brush in the middle of the map for `SIZE_SHOWN_MS`. */
  private sizes(): HTMLElement {
    const list = document.createElement('div');
    list.className = 'chips sizes'; list.setAttribute('aria-label', 'Brush size');
    for (const m of RADII_M) {
      const b = document.createElement('button');
      b.textContent = radiusText(m); b.classList.toggle('on', m === this.radius);
      b.onclick = () => {
        this.radius = m; this.sizeShownTill = performance.now() + SIZE_SHOWN_MS;
        this.show(); this.page.redraw();
        window.setTimeout(() => this.page.redraw(), SIZE_SHOWN_MS);
      };
      list.append(b);
    }
    return list;
  }

  /** Say `text` for a moment, with a button (its label and what it does) where given. */
  private say(text: string, act?: [string, () => void]): void {
    const toast = this.page.toast;
    toast.replaceChildren(document.createTextNode(text));
    if (act) {
      const b = document.createElement('button');
      b.textContent = act[0];
      b.onclick = () => { toast.classList.remove('on'); act[1](); };
      toast.append(b);
    }
    toast.classList.add('on');
    toast.classList.toggle('act', act !== undefined);
    clearTimeout(this.toastTimer);
    this.toastTimer = window.setTimeout(() => toast.classList.remove('on'), act ? TOAST_ACT_MS : TOAST_MS);
  }
}

/** The key counted most in `counts` ('' for none). */
function most(counts: Map<string, number>): string {
  let best = '', n = 0;
  for (const [k, c] of counts) if (c > n) { best = k; n = c; }
  return best;
}
