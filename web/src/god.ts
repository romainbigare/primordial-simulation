// God mode on the live map (handover 11): the player picks a tool on the strip and edits the world with a round brush
// (wasm/src/edit.rs `brush`, the core's `World::edit`): a tool paints as the brush moves, paints again and again while
// it is held (the ground, heat), or makes one thing where the stroke begins (a lake); when the stroke ends it settles
// (`settle`: the ground built again, what was heated burns). With a mouse the brush is where the pointer is; on a
// phone it stands at a sight in the middle of the map (`GodPage.aim`): a finger moves the map under it, and a stroke
// lasts while the tool's big button is held, so a finger never hides the brush nor paints by mistake. A burn's stroke
// ends as soon as the ground lights, and its fire spreads on the map (front.ts). The tools themselves are a list
// (god-tools.ts `godTools`); this is what they share: the strip (done, the tools named, the undo of the last stroke), the
// card of the brush's sizes and the tool's choices, the strokes and the core's answers to them, the question to the core
// about the place under the cursor (`probe`), and the brush drawn over the map at its true size (god-draw.ts).

import { UNCHANGED, type Brushed, type Editor, type GodPage, type GodTool, type Probe, type Settled } from './god-types';
import { drawBrush, drawFill, drawFlames, drawReason, drawSight, drawSizeTag, drawTrail, litSites } from './god-draw';
import { holdButton, makeStrip, mostWhy, radiusText, showCard, showStrip } from './god-controls';

/** The brush's radii, m (its size chips), and the one it starts at. */
const RADII_M = [150, 300, 600, 1000, 2000, 3000], FIRST_RADIUS = 2;
/** The least time between two paints of a stroke, ms, and the least move of the brush's centre between them, a share
 * of its radius: cells are edited once a stroke, so a paint where the brush has not moved adds nothing. A tool with a
 * rate paints every `PAINT_MS` while held. */
const PAINT_MS = 90, PAINT_MOVE = 0.25;
/** A tool with a rate paints only once held this long, ms: a tap does nothing. */
const HOLD_MS = 220;
/** A tool that paints in steps (`GodTool.step`) looks this many times a step whether one is due, so each lands within a
 * quarter of its time of its start. */
const STEP_LOOKS = 4;
/** A held stroke of a tool that softens (`GodTool.softens`) settles what it did so far at most this often, ms: the
 * world's samples are built again and its liquid settles over them while the ground moves (a pause of the core of a
 * few hundred ms, so not at every step; the steps it holds up are painted together after it). */
const SOFTEN_MS = 1000;
/** The ink of the brush where the tool cannot act: a dull red. */
const NO_INK = 'rgba(196,76,60,.95)';
/** How long the brush's size shows on the map after one is picked, ms. */
const SIZE_SHOWN_MS = 1500;
/** The fire of a burn that ended is drawn this long after the core answered its end, ms: the map's own fire spreads
 * from it once the frame after it came (front.ts). */
const FIRE_LINGERS_MS = 1500;
/** Least time between two questions about the place under the cursor, ms. */
const PROBE_MS = 80;
/** A lake's water is drawn in its ring at most this long after the stroke settled, ms, where the world's samples do not
 * change (a lake the core did not dig). */
const FILL_MOST_MS = 4000;


/** A stroke under way: its tool, the place and time of its last paint (squares, ms), when it began (ms), where the
 * brush is now (squares), the paints sent, the timer that paints while it is held (its tick, `tick`), and whether a
 * paint of it waits for the core's answer (a tool with a rate paints again only once answered, with the time since: the
 * core may take longer over a paint than the timer's step). */
interface Stroke {
  tool: GodTool; last: [number, number, number]; began: number; now: [number, number]; sent: number; held: number; waiting: boolean;
  /** Of a tool that paints in steps: the steps painted. Of a tool that softens: when it last settled what it did so far
   * (ms), and the paints sent then. */
  steps: number; softened: number; softenedAt: number;
  /** Of heat: how much of the heat that lights them the cells under the brush hold, their mean by share over those that
   * can light (-1: no heat asked). */
  progress: number;
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
  /** When the size of the brush picked last stops showing on the map (ms). */
  private sizeShownTill = 0;
  /** The fire of the last burn, drawn where it struck (squares) till the map's own has begun (`till`, ms). */
  private lingers: { at: [number, number]; progress: number; till: number } = { at: [0, 0], progress: 0, till: 0 };
  /** Whether a stroke settled that can be undone (the core keeps one), and the button that undoes it. */
  private undoable = false;
  private undoButton: HTMLButtonElement;
  /** The button held for a stroke where the brush stands at the sight (`GodPage.aim`). */
  private holdButton: HTMLButtonElement;
  /** The trail of the stroke of a tool that trails (`GodTool.trails`): each paint's place (squares) and radius (m), and
   * its ink; drawn till the core settled it. */
  private trail: { at: [number, number][]; radius: number[]; ink: string } | null = null;
  /** The lake being dug: where (squares), its radius (m), its tool's ink, when its stroke ended (ms), the version of
   * the world's samples then, and when the core answered its settling (ms; 0: not yet). Its water is drawn in its
   * ring till the ground in view shows the samples it built (`GodPage.ground`). */
  private filling: { at: [number, number]; radius: number; ink: string; began: number; from: number; settled: number } | null = null;

  /** `editor`: the live world's, where there is one. */
  constructor(private page: GodPage, private editor: () => Editor | null, private tools: GodTool[]) {
    this.undoButton = makeStrip(page.strip, tools, () => page.close(), t => this.pick(this.tool === t ? null : t), () => this.undo());
    // (The button starts a stroke at the sight, on a phone alone.)
    this.holdButton = holdButton(() => { const aim = this.page.aim(); return aim !== null && this.start(...aim); }, () => this.end());
    this.show();
  }

  /** Whether the brush stands at the sight (a phone), not under the pointer. */
  aiming(): boolean { return this.page.aim() !== null; }

  /** Put the tool away and forget the place: a new world, or god mode closed. A stroke held ends as a stroke does
   * (`end`), so the core's answers to it come in their turn. */
  reset(): void {
    this.end();
    this.holdButton.classList.remove('held');
    this.probe = null; this.asking = false; this.filling = null;
    this.pick(null);
  }

  /** The last stroke can no longer be undone: the world played on (the core forgets it), or a new world came. */
  forgetUndo(): void {
    if (!this.undoable) return;
    this.undoable = false;
    this.show();
  }

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

  /** Put the tool on away: true if one was on. */
  putAway(): boolean {
    if (!this.tool) return false;
    this.pick(null);
    return true;
  }

  /** Take tool `t` (null: none), and say how to use it; the world pauses while one is on. A stroke under way ends. */
  private pick(t: GodTool | null): void {
    if (t !== this.tool) this.end();
    this.tool = t; this.lastWhy = '';
    if (t?.least && this.radius < t.least) this.radius = this.radii()[0];
    if (t) this.page.picked();
    this.show(); this.page.redraw();
  }

  /** Follow the pointer on `cv`: the place under it is asked of the core. */
  listen(cv: HTMLCanvasElement): void {
    const follow = (e: PointerEvent) => {
      const v = this.drawnAt;
      if (!v || this.aiming()) return;
      this.at = [(e.offsetX - v[0]) / v[2], (e.offsetY - v[1]) / v[2]];
      this.ask();
      if (this.tool) this.page.redraw();
    };
    cv.addEventListener('pointermove', follow);
    cv.addEventListener('pointerdown', follow);
    cv.addEventListener('pointerleave', () => { if (this.aiming()) return; this.at = null; if (this.tool) this.page.redraw(); });
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
    // (The reason a paint gave is of its own place.)
    if (!this.stroke && was && was.cell !== p.cell) this.lastWhy = '';
    if (this.tool) this.page.redraw();
    if (!was || was.cell !== p.cell || was.fire !== p.fire || was.land !== p.land || was.patches !== p.patches) { this.show(); this.page.redraw(); }
  }

  /** A stroke of the tool on starting at (x, y) squares, true; false if no tool is on (the map moves). A stroke under
   * way ends first. A tool with a rate begins to paint once held `HOLD_MS`; another tool paints at once, and while held
   * paints where the brush moves to (`tick`). */
  start(x: number, y: number): boolean {
    const t = this.tool;
    if (!t) return false;
    this.end();
    this.page.picked();
    this.at = [x, y]; this.lastWhy = '';
    const now = performance.now();
    const st: Stroke = this.stroke = { tool: t, last: [x, y, now], began: now, now: [x, y], sent: 0, held: 0, waiting: false, steps: 0, softened: now, softenedAt: 0, progress: -1 };
    const ticks = () => window.setInterval(() => this.tick(st), t.step ? t.step.every / STEP_LOOKS : PAINT_MS);
    if (t.rate) st.held = window.setTimeout(() => { if (this.stroke !== st) return; st.held = ticks(); this.paint(st); }, HOLD_MS);
    else { this.paint(st); if (!t.amount) st.held = ticks(); }
    // A lake's water is drawn in its ring from the tap.
    if (t.amount) this.filling = { at: [x, y], radius: this.radius, ink: t.ink, began: now, from: this.page.ground()[0], settled: 0 };
    return true;
  }

  /** A tick of stroke `st` while held: the brush follows the sight where it stands there; a tool with a rate paints
   * again once the core answered its last paint, one with steps once a step is due too, another where the brush moved.
   * A tick of a stroke that ended stops. */
  private tick(st: Stroke): void {
    if (this.stroke !== st) { clearInterval(st.held); return; }
    const aim = this.page.aim();
    if (aim) { st.now = aim; this.at = aim; }
    if (st.tool.softens && !st.waiting && st.sent > st.softenedAt && performance.now() - st.softened >= SOFTEN_MS) this.soften(st);
    if (st.tool.rate || st.tool.step) { if (!st.waiting && this.due(st) !== 0) this.paint(st); } else this.moved(st);
  }

  /** Stroke `st`, held, settles what it did so far and goes on (`Editor.soften`): the core answers its paints after. */
  private soften(st: Stroke): void {
    st.softened = performance.now(); st.softenedAt = st.sent;
    this.editor()?.soften();
  }

  /** Of stroke `st` of a tool with steps, the steps due and not yet painted: one at its start and one at the start of
   * each `every` ms held since, so a hold of t ms makes ⌈t / every⌉ (at least one); of another tool, 1. */
  private due(st: Stroke): number {
    const s = st.tool.step;
    return s ? Math.max(1, Math.ceil((performance.now() - st.began) / s.every)) - st.steps : 1;
  }

  /** The pointer of the stroke moved to (x, y) squares: a paint there, once it moved far and long enough. */
  move(x: number, y: number): void {
    const st = this.stroke;
    if (!st) return;
    st.now = [x, y];
    if (!st.tool.rate && !st.tool.step && !st.tool.amount) this.moved(st);
  }

  /** A paint of stroke `st` where its brush is now, if it moved far and long enough since its last and the core
   * answered that one (a fast drag never queues paints faster than the core does them; the next tick paints). */
  private moved(st: Stroke): void {
    const [x, y] = st.now, [lx, ly, lt] = st.last, far = Math.hypot(x - lx, y - ly) >= (PAINT_MOVE * this.radius) / this.page.squareM();
    if (far && !st.waiting && performance.now() - lt >= PAINT_MS) this.paint(st);
  }

  /** The stroke ends: a last paint where the brush moved since (of a tool with steps, the steps still due), and it
   * settles; a tool with a rate let go before it painted (a tap) does nothing. */
  end(): void {
    const st = this.stroke;
    if (!st) return;
    clearInterval(st.held);
    this.stroke = null;
    if (st.tool.rate && !st.sent) { this.page.redraw(); return; }
    if (st.tool.step) { if (this.due(st) > 0) this.paint(st); }
    else if (st.tool.rate || (!st.tool.amount && (st.now[0] !== st.last[0] || st.now[1] !== st.last[1]))) this.paint(st);
    this.editor()?.settle();
    this.ended.push(st);
  }

  /** Paint with stroke `st`'s tool where its brush is now: by its amount, for a tool with steps by the steps due (all
   * at once where the core was slower than a step), or for a tool with a rate by what the time since its last paint
   * adds, its rate doubling every `doubling` s it has been held, at most `doublings` times. */
  private paint(st: Stroke): void {
    const t = st.tool, [x, y] = st.now, now = performance.now();
    const rate = (t.rate ?? 0) * (t.doubling ? 2 ** Math.min((now - st.began) / 1000 / t.doubling, t.doublings ?? Infinity) : 1);
    const steps = t.step ? this.due(st) : 0;
    st.steps += steps;
    const amount = t.amount ? t.amount() : t.step ? steps * t.step.amount : (rate * (now - st.last[2])) / 1000;
    st.last = [x, y, now];
    if (t.trails) {
      if (this.stroke === st && st.sent === 0) this.trail = { at: [], radius: [], ink: t.ink };
      this.trail?.at.push([x, y]); this.trail?.radius.push(this.radius);
    }
    st.waiting = true; st.sent++;
    this.editor()?.brush(t.code, x, y, this.radius, amount, t.row());
  }

  /** The core's answer to a paint: the heat it holds, or the reason, where it edited none of its cells (a lake that was
   * not dug draws no water). */
  brushed(r: Brushed): void {
    const st = this.ended[0] ?? this.stroke;
    if (!st) return;
    st.waiting = false;
    const heat = r.cells.filter(c => c.progress !== undefined);
    if (heat.length) this.heated(st, heat);
    // (The ground's tools and a lake answer their cells as numbers.)
    else if (typeof r.cells[0] === 'object') {
      const done = (c: Brushed['cells'][number]) => c.done!.every(d => d);
      this.lastWhy = r.cells.some(done) ? '' : st.tool.reason(mostWhy(r.cells.filter(c => !done(c))));
    } else this.lastWhy = r.changed ? '' : st.tool.reason(UNCHANGED);
    if (st.tool.amount && !r.changed) this.filling = null;
    this.page.redraw();
  }

  /** Heat's answer: the progress under the brush, and why none would light there; a tool whose fire runs on its own
   * (`settlesLit`) ends its stroke once a cell under it lights, so the fire spreads at once. */
  private heated(st: Stroke, cells: Brushed['cells']): void {
    const can = cells.filter(c => !c.why), share = can.reduce((n, c) => n + c.share, 0);
    st.progress = share > 0 ? can.reduce((n, c) => n + c.share * c.progress!, 0) / share : 0;
    this.lastWhy = can.length ? '' : st.tool.reason(mostWhy(cells));
    if (st === this.stroke && st.tool.settlesLit && can.some(c => c.progress! >= 1)) this.end();
  }

  /** The core's answer to the stroke's end: the cells it burnt, to the page, from where the stroke ended. */
  settled(r: Settled): void {
    const st = this.ended.shift();
    if (!st) return;
    if (this.filling && st.tool.amount) this.filling.settled = performance.now();
    if (st.tool.trails) { this.trail = null; this.page.redraw(); }
    if (r.unplanted) this.lastWhy = st.tool.reason(r.unplanted);
    if (r.undo) this.undoable = true;
    this.show();
    if (!r.burnt.length) return;
    const m = this.page.squareM();
    this.page.burnt(r.squares, st.now, litSites([st.now[0] * m, st.now[1] * m], this.radius, st.progress).map(s => [s.x, s.y]));
    this.lingers = { at: st.now, progress: st.progress, till: performance.now() + FIRE_LINGERS_MS };
  }

  /** Over the map, with its corner at (ox, oy) and a square `s` pixels: the brush at its true size at the sight (a
   * phone: the place under it asked of the core as the map moves) or under the cursor, with the flames of the heat it
   * holds, its size just after one was picked, and the reason over it where the tool cannot act there. */
  draw(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number): void {
    this.drawnAt = [ox, oy, s];
    const t = this.tool, aim = this.page.aim(), now = performance.now();
    this.drawFilling(ctx, ox, oy, s, now);
    if (this.trail) drawTrail(ctx, this.trail.at.map(([x, y]) => [ox + x * s, oy + y * s]), this.trail.radius.map(r => (r / this.page.squareM()) * s), this.trail.ink);
    if (t && aim && !this.stroke && (!this.at || this.at[0] !== aim[0] || this.at[1] !== aim[1])) { this.at = aim; this.ask(); }
    if (!t || !this.at) return;
    const why = this.lastWhy || t.why(this.probe), ink = why ? NO_INK : t.ink;
    const [cx, cy] = [ox + this.at[0] * s, oy + this.at[1] * s];
    const r = (this.radius / this.page.squareM()) * s, progress = this.stroke?.progress ?? -1;
    drawBrush(ctx, cx, cy, r, ink, this.stroke !== null, t.settlesLit === true, progress);
    // The fire of the heat held, and of a burn that ended till the map's own fire has begun to spread from it. (It
    // flickers: the map's overlay is drawn again at the next frame while it burns.)
    const burnt = this.stroke ?? this.ended[0], linger = now < this.lingers.till ? this.lingers : null;
    const fire = burnt && !why ? { at: burnt.now, progress: burnt.progress } : linger;
    if (fire && fire.progress > 0) {
      const m = this.page.squareM();
      drawFlames(ctx, { ox, oy, k: s / m }, [fire.at[0] * m, fire.at[1] * m], this.radius, fire.progress, now);
      this.page.redraw();
    }
    if (aim) drawSight(ctx, cx, cy);
    if (now < this.sizeShownTill) drawSizeTag(ctx, cx, cy, r, radiusText(this.radius));
    if (why) drawReason(ctx, cx, cy, r, why);
  }

  /** The water of the lake being dug (`filling`), in its ring, till the core settled it and the ground in view shows
   * the samples it built, or `FILL_MOST_MS` after it settled where they did not change. */
  private drawFilling(ctx: CanvasRenderingContext2D, ox: number, oy: number, s: number, now: number): void {
    const f = this.filling;
    if (!f) return;
    const [version, shown] = this.page.ground();
    if (f.settled && ((version !== f.from && shown === version) || now - f.settled > FILL_MOST_MS)) { this.filling = null; this.page.redraw(); return; }
    drawFill(ctx, ox + f.at[0] * s, oy + f.at[1] * s, (f.radius / this.page.squareM()) * s, f.ink, now - f.began);
    this.page.redraw();
  }

  /** The strip's buttons (the tool on pressed, a tool that cannot act here greyed with the reason, the undo greyed
   * where no stroke can be undone), and the card: its title, the brush's sizes (chips; on a phone a smaller and a
   * larger button), the tool's choices, a line of how to use it or why it cannot act here, and on a phone the button
   * held for a stroke. */
  private show(): void {
    showStrip(this.page.strip, this.tools, this.tool, this.probe, this.undoable, this.undoButton);
    showCard(this.page.card, this.holdButton, this.tool, this.probe, { radii: this.radii(), radius: this.radius, pick: m => this.sized(m) }, () => this.show());
  }

  /** The brush's radii the tool on may take (`GodTool.least`). */
  private radii(): number[] { return RADII_M.filter(m => m >= (this.tool?.least ?? 0)); }

  /** Take the brush's radius `m`; it shows its size on the map for `SIZE_SHOWN_MS`. */
  private sized(m: number): void {
    this.radius = m; this.sizeShownTill = performance.now() + SIZE_SHOWN_MS;
    this.show(); this.page.redraw();
    window.setTimeout(() => this.page.redraw(), SIZE_SHOWN_MS);
  }

}

