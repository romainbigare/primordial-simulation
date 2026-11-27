// God mode on the live map (handover 11): the player picks a tool in the menu and edits the world with a round brush
// (wasm/src/edit.rs `brush`, the core's `World::edit`): a tool paints as the brush moves, paints at each of the page's
// frames while it is held (the ground, along the brush's path; heat), or makes one thing where the stroke begins (a
// lake); when the stroke ends it settles (`settle`: the ground built again, what was heated burns), a stroke of the
// ground once the brush rested after it. With a mouse the brush is where the pointer is; on a phone it stands at a
// sight in the middle of the map (`GodPage.aim`): a finger moves the map under it, and a stroke
// lasts while the tool's big button is held, so a finger never hides the brush nor paints by mistake. A burn's stroke
// ends as soon as the ground lights, and its fire spreads on the map (front.ts). The tools themselves stand in classes
// (god-tools.ts `godClasses`); this is what they share: the menu (god-menu.ts: the panel of the home of the tools' classes,
// of a class's cards to pick a tool, and of the tool on with its sizes and its setting, done and the undo of the last stroke), the strokes and
// the core's answers to them, the question to the core about the place under the cursor (`probe`), and the brush drawn
// over the map at its true size (god-draw.ts).

import { UNCHANGED, type Brushed, type Editor, type GodClass, type GodPage, type GodTool, type Probe, type Settled } from './god-types';
import { drawBrush, drawFill, drawFlames, drawReason, drawSight, drawSizeTag, drawTrail, litSites } from './god-draw';
import { holdButton, mostWhy, radiusText } from './god-controls';
import { GodMenu } from './god-menu';

/** The brush's radii, m (its sizes), and the one it starts at. */
const RADII_M = [150, 300, 600, 1000, 2000, 3000], FIRST_RADIUS = 2;
/** Of a tool that paints where the brush moves, the least time between two paints of a stroke, ms, and the least move
 * of the brush's centre between them, a share of its radius: cells are edited once a stroke, so a paint where the
 * brush has not moved adds nothing. A paint that follows the brush's path (`GodTool.follows`) lays its places no
 * farther apart than that share of the radius. */
const PAINT_MS = 90, PAINT_MOVE = 0.25;
/** A tool with a rate and no tap (`GodTool.tapS`) paints only once held this long, ms: a tap does nothing. */
const HOLD_MS = 220;
/** A stroke of a tool that rests (`GodTool.rests`) settles this long after the brush was let go, ms, unless a stroke of
 * it begins before then and goes on with it: a short lift of the brush does not settle the world (a pause of the core
 * of a few hundred ms). */
const SHAPE_REST_MS = 400;
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
 * brush is now (squares) and where it was since its last paint, in order (of a tool that follows the path), the paints
 * sent, the timer that waits `HOLD_MS` and the animation frame that ticks while it is held (`tick`), and whether a
 * paint of it waits for the core's answer (a tool with a rate paints again only once answered, with the time since and
 * the path since: the core may take longer over a paint than a frame). */
interface Stroke {
  tool: GodTool; last: [number, number, number]; began: number; now: [number, number]; path: [number, number][];
  sent: number; held: number; frame: number; waiting: boolean;
  /** Of a tool with a rate: the time its paints covered, ms (the last covers what a tap lacks, `GodTool.tapS`). */
  covered: number;
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
   * asked, so the answers to their paints and their settling come before those of the stroke under way. The stroke
   * let go that waits `SHAPE_REST_MS` to settle (`GodTool.rests`), and its timer; and what waits for the world to be
   * settled (`whenSettled`). */
  private ended: Stroke[] = [];
  private resting: Stroke | null = null;
  private restTimer = 0;
  private waiters: (() => void)[] = [];
  /** The map's corner and the pixels a square takes, as last drawn. */
  private drawnAt: [number, number, number] | null = null;
  /** When the size of the brush picked last stops showing on the map (ms). */
  private sizeShownTill = 0;
  /** The fire of the last burn, drawn where it struck (squares) till the map's own has begun (`till`, ms). */
  private lingers: { at: [number, number]; progress: number; till: number } = { at: [0, 0], progress: 0, till: 0 };
  /** Whether a stroke settled that can be undone (the core keeps one). */
  private undoable = false;
  /** The button held for a stroke where the brush stands at the sight (`GodPage.aim`), and the menu it stands in. */
  private holdButton: HTMLButtonElement;
  private menu: GodMenu;
  /** The trail of the stroke of a tool that trails (`GodTool.trails`): each paint's place (squares) and radius (m), and
   * its ink; drawn till the core settled it. */
  private trail: { at: [number, number][]; radius: number[]; ink: string } | null = null;
  /** The lake being dug: where (squares), its radius (m), its tool's ink, when its stroke ended (ms), the version of
   * the world's samples then, and when the core answered its settling (ms; 0: not yet). Its water is drawn in its
   * ring till the ground in view shows the samples it built (`GodPage.ground`). */
  private filling: { at: [number, number]; radius: number; ink: string; began: number; from: number; settled: number } | null = null;

  /** `editor`: the live world's, where there is one; `classes`: the tools, in their classes. */
  constructor(private page: GodPage, private editor: () => Editor | null, classes: GodClass[]) {
    // (The button starts a stroke at the sight, on a phone alone.)
    this.holdButton = holdButton(() => { const aim = this.page.aim(); return aim !== null && this.start(...aim); }, () => this.end());
    this.menu = new GodMenu(page.holdBox, page.panel, classes, this.holdButton, { pick: t => this.pick(t), undo: () => this.undo(), close: () => page.close(), size: m => this.sized(m) });
    this.show();
  }

  /** Whether the brush stands at the sight (a phone), not under the pointer. */
  aiming(): boolean { return this.page.aim() !== null; }

  /** Put the tool away and forget the place: a new world, or god mode closed. A stroke held ends as a stroke does
   * (`end`) and settles at once, so the core's answers to it come in their turn. */
  reset(): void {
    this.end(); this.settleNow();
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

  /** Whether a stroke is under way, rests (`SHAPE_REST_MS`) or its settling is still to come: the world does not play
   * meanwhile. */
  private busy(): boolean { return this.stroke !== null || this.resting !== null || this.ended.length > 0; }

  /** Run `f` once the world is settled (play, save, undo): a stroke that rests settles at once, and `f` runs once the
   * core answered every settling asked; never while a stroke is under way. */
  whenSettled(f: () => void): void {
    if (this.stroke) return;
    this.settleNow();
    if (this.busy()) this.waiters.push(f); else f();
  }

  /** The stroke that rests settles now (`Editor.settle`), where there is one. */
  private settleNow(): void {
    const st = this.resting;
    if (!st) return;
    clearTimeout(this.restTimer);
    this.resting = null;
    this.editor()?.settle();
    this.ended.push(st);
  }

  /** Undo the last stroke once the world is settled (`whenSettled`), where there is one. */
  undo(): void {
    this.whenSettled(() => {
      if (!this.undoable) return;
      this.undoable = false;
      this.editor()?.undo();
      this.show();
    });
  }

  /** Put the tool on away: true if one was on. */
  putAway(): boolean {
    if (!this.tool) return false;
    this.pick(null);
    return true;
  }

  /** Take tool `t` (null: none), and say how to use it; the world pauses while one is on. A stroke under way ends, and
   * where the tool changes, settles at once. */
  private pick(t: GodTool | null): void {
    if (t !== this.tool) { this.end(); this.settleNow(); }
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
   * way ends first; a stroke that rests (`SHAPE_REST_MS`) goes on as this one where its tool rests (with its paint
   * still awaited), else it settles first. A tool with a rate begins to paint at once where it has a tap
   * (`GodTool.tapS`), else once held `HOLD_MS`; another tool paints at once, and while held paints where the brush
   * moves to (`tick`). */
  start(x: number, y: number): boolean {
    const t = this.tool;
    if (!t) return false;
    this.end();
    const on = this.resting?.tool === t && t.rests ? this.resting : null;
    if (on) { clearTimeout(this.restTimer); this.resting = null; } else this.settleNow();
    this.page.picked();
    this.at = [x, y]; this.lastWhy = '';
    const now = performance.now();
    const st: Stroke = this.stroke = { tool: t, last: [x, y, now], began: now, now: [x, y], path: [], sent: on?.sent ?? 0, held: 0, frame: 0, waiting: on?.waiting ?? false, covered: 0, progress: -1 };
    if (t.rate && t.tapS === undefined) st.held = window.setTimeout(() => { if (this.stroke !== st) return; this.frames(st); this.paint(st); }, HOLD_MS);
    else if (t.rate) this.frames(st);
    else { this.paint(st); if (!t.amount) this.frames(st); }
    // A lake's water is drawn in its ring from the tap.
    if (t.amount) this.filling = { at: [x, y], radius: this.radius, ink: t.ink, began: now, from: this.page.ground()[0], settled: 0 };
    return true;
  }

  /** Tick stroke `st` at each of the page's frames while it is held (`tick`). */
  private frames(st: Stroke): void {
    const next = () => { if (this.stroke !== st) return; this.tick(st); st.frame = requestAnimationFrame(next); };
    st.frame = requestAnimationFrame(next);
  }

  /** A tick of stroke `st` while held: the brush follows the sight where it stands there; a tool with a rate paints
   * again once the core answered its last paint, another where the brush moved. */
  private tick(st: Stroke): void {
    const aim = this.page.aim();
    if (aim) { this.follow(st, aim); this.at = aim; }
    if (st.tool.rate) { if (!st.waiting) this.paint(st); } else this.moved(st);
  }

  /** The brush of stroke `st` is at `at` (squares) now: of a tool that follows the path, where it was is kept. */
  private follow(st: Stroke, at: [number, number]): void {
    if (at[0] === st.now[0] && at[1] === st.now[1]) return;
    if (st.tool.follows) st.path.push(st.now);
    st.now = at;
  }

  /** The pointer of the stroke moved to (x, y) squares: a paint there, once it moved far and long enough. */
  move(x: number, y: number): void {
    const st = this.stroke;
    if (!st) return;
    this.follow(st, [x, y]);
    if (!st.tool.rate && !st.tool.amount) this.moved(st);
  }

  /** A paint of stroke `st` where its brush is now, if it moved far and long enough since its last and the core
   * answered that one (a fast drag never queues paints faster than the core does them; the next tick paints). */
  private moved(st: Stroke): void {
    const [x, y] = st.now, [lx, ly, lt] = st.last, far = Math.hypot(x - lx, y - ly) >= (PAINT_MOVE * this.radius) / this.page.squareM();
    if (far && !st.waiting && performance.now() - lt >= PAINT_MS) this.paint(st);
  }

  /** The stroke ends: a last paint (of a tool with a rate, what the time since its last adds, and what a tap lacks;
   * of another, where the brush moved since), and it settles, a stroke of a tool that rests once it rested
   * `SHAPE_REST_MS`; a tool with a rate and no tap let go before it painted does nothing. */
  end(): void {
    const st = this.stroke;
    if (!st) return;
    clearTimeout(st.held); cancelAnimationFrame(st.frame);
    this.stroke = null;
    if (st.tool.rate && st.tool.tapS === undefined && !st.sent) { this.page.redraw(); return; }
    if (st.tool.rate) this.paint(st, true);
    else if (!st.tool.amount && (st.now[0] !== st.last[0] || st.now[1] !== st.last[1])) this.paint(st);
    this.resting = st;
    if (st.tool.rests) this.restTimer = window.setTimeout(() => this.settleNow(), SHAPE_REST_MS);
    else this.settleNow();
  }

  /** Paint with stroke `st`'s tool where its brush is now (of a tool that follows the path, along it: `along`): by its
   * amount, or for a tool with a rate by what the time since its last paint adds (the stroke's `last` paint: no less
   * than what its tap lacks), its rate doubling every `doubling` s it has been held, at most `doublings` times. */
  private paint(st: Stroke, last = false): void {
    const t = st.tool, [x, y] = st.now, now = performance.now();
    const ms = Math.max(now - st.last[2], last && t.tapS ? 1000 * t.tapS - st.covered : 0);
    st.covered += ms;
    const rate = t.rate ? t.rate(this.radius) * (t.doubling ? 2 ** Math.min((now - st.began) / 1000 / t.doubling, t.doublings ?? Infinity) : 1) : 0;
    const amount = t.amount ? t.amount() : (rate * ms) / 1000;
    const stamps = t.follows ? this.along(st, amount) : [[x, y, amount] as [number, number, number]];
    st.last = [x, y, now]; st.path = [];
    if (t.trails) {
      if (this.stroke === st && st.sent === 0) this.trail = { at: [], radius: [], ink: t.ink };
      this.trail?.at.push([x, y]); this.trail?.radius.push(this.radius);
    }
    st.waiting = true; st.sent++;
    this.editor()?.brush(t.code, stamps, this.radius, t.row());
  }

  /** The places of a paint of `amount` of stroke `st` along the brush's path since its last paint (`Stroke.path`):
   * evenly along it, as few as keep them no more than `PAINT_MOVE` of the radius apart (one, where it is now, where it
   * did not move), each with its share of the amount. */
  private along(st: Stroke, amount: number): [number, number, number][] {
    const points: [number, number][] = [[st.last[0], st.last[1]], ...st.path, st.now];
    const legs = points.slice(1).map((p, i) => Math.hypot(p[0] - points[i][0], p[1] - points[i][1]));
    const length = legs.reduce((a, b) => a + b, 0), n = Math.max(1, Math.ceil(length / ((PAINT_MOVE * this.radius) / this.page.squareM())));
    const out: [number, number, number][] = [];
    for (let s = 1, leg = 0, walked = 0; s <= n; s++) {
      const want = (length * s) / n;
      while (leg < legs.length - 1 && walked + legs[leg] < want) walked += legs[leg++];
      const f = legs[leg] > 0 ? Math.min((want - walked) / legs[leg], 1) : 1, [a, b] = [points[leg], points[leg + 1]];
      out.push([a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1]), amount / n]);
    }
    return out;
  }

  /** The core's answer to a paint: the heat it holds, or the reason, where it edited none of its cells (a lake that was
   * not dug draws no water). */
  brushed(r: Brushed): void {
    const st = this.ended[0] ?? this.resting ?? this.stroke;
    if (!st) return;
    st.waiting = false;
    const heat = r.cells.filter(c => c.progress !== undefined);
    if (heat.length) this.heated(st, heat);
    // (The ground's tools and a lake answer no cells: only whether they changed the world.)
    else if (r.cells.length) {
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
    if (!this.busy()) this.waiters.splice(0).forEach(f => f());
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

  /** The menu as god mode stands: the tool on, the place under the brush, the brush's sizes, and whether a stroke can
   * be undone. */
  private show(): void {
    // (A stroke that rests is undone once it settled, `undo`.)
    this.menu.show({ tool: this.tool, probe: this.probe, radius: this.radius, radii: this.radii(), all: RADII_M, undoable: this.undoable || this.resting !== null });
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

