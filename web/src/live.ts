import { World, makeWorld, Meta } from './data';
import { timed } from './cost';
import type { Probe, StartKind, Brushed, Settled } from './god-types';
import type { Built, Preview } from './ground';

/** What the core reports with each frame: its cells, the months stepped per second of real time, the time a month
 * took to step, ms, the share of the time the worker stepped, and the bytes of its memory. */
export interface Stats { cells: number; pace: number; monthMs: number; busy: number; memory: number }

/** The longest time a frame waits for its answer when no animation frame comes, ms: the worker posts its next frame
 * only once the last is answered, so where the page's animation frames stop the date shown still moves each second. */
const ANSWER_MS = 1000;

/** The live world: the core in a Web Worker (worker.ts) that makes a world, steps it and posts a frame as bytes with the
 * cells' table (tape/src/table.rs), the samples' discharge and load as 16-bit floats when they changed enough to show
 * (held until the map takes them, `takeDischarge`), and where load came into a body of liquid in the last month
 * (`plumes`), and the relief once the world built its samples anew (`takeRelief`). Each frame is answered once the page has drawn it (`shown`): while the
 * world plays, the worker posts the next only then. */
export class Live {
  world: World | null = null;
  table: Float32Array<ArrayBufferLike> = new Float32Array(0);
  private discharge: Uint16Array | null = null;
  /** Where load came into a body of liquid in the last month: a sample (u32) and kg/s (f32) each (wasm/src/lib.rs
   * `plumes`). */
  plumes: Uint8Array = new Uint8Array(0);
  /** The relief the world last built its samples from and its version, come since it was last taken (wasm/src/lib.rs
   * `relief`, `ground_version`). */
  private relief: Built | null = null;
  stats: Stats = { cells: 0, pace: 0, monthMs: 0, busy: 0, memory: 0 };
  /** The version of the world's samples, as the last frame said (wasm/src/lib.rs `ground_version`). */
  version = 0;
  /** The starting kinds the world's plan places (wasm/src/edit.rs `kinds`). */
  kinds: StartKind[] = [];
  private worker: Worker;
  /** The answers awaited about places, in the order asked (the worker answers in turn). */
  private answers: ((p: Probe) => void)[] = [];
  private url: string;
  /** The frames taken that wait for their answer (`shown`), and the listener that answers them when the page is hidden. */
  private unanswered = 0;
  private onHide = () => { if (document.hidden) this.answer(); };

  constructor(workerSource: string, wasm: ArrayBuffer, on: { ready: () => void; world: (w: World) => void; frame: (stats: Stats, changed: boolean, edited: boolean) => void; error: (text: string) => void; things: (data: Uint8Array | null) => void; brushed: (r: Brushed) => void; settled: (r: Settled) => void; undone: (done: boolean) => void; preview: (p: Preview) => void; relief: () => void; saved: (gz: Uint8Array<ArrayBuffer>, year: number) => void }) {
    this.url = URL.createObjectURL(new Blob([workerSource], { type: 'text/javascript' }));
    this.worker = new Worker(this.url);
    this.worker.onerror = e => on.error(e.message || 'The world\'s worker failed.');
    this.worker.onmessage = e => {
      const m = e.data as { kind: string; meta: string; kinds: string; edited: boolean; frame: Uint8Array; table: Float32Array; discharge: Uint16Array | null; plumes: Uint8Array; relief: Uint8Array | null; reliefVersion: number; year: number; day: number; text: string; stats: Stats; things: Uint8Array | null; preview?: Preview; saved?: Uint8Array<ArrayBuffer> };
      if (m.kind === 'ready') return on.ready();
      if (m.kind === 'error') return on.error(m.text);
      if (m.kind === 'probe') return this.answers.shift()?.(JSON.parse(m.text) as Probe);
      if (m.kind === 'saved') return on.saved(m.saved!, m.year);
      // The things of the window asked for, of a paused world (worker.ts `things`).
      if (m.kind === 'things') return timed('world frames', () => on.things(m.things));
      // (What the edits moved since the ground the page shows comes with a brush of the ground, an undo, and the answer
      // to `groundShown`.)
      if (m.preview) on.preview(m.preview);
      if (m.kind === 'preview') return;
      // A stroke's relief, before the world settles (worker.ts `settle`): the ground is made again meanwhile.
      if (m.kind === 'relief') { this.relief = { relief: m.relief!, version: m.reliefVersion }; this.version = m.reliefVersion; return on.relief(); }
      if (m.kind === 'brushed') return on.brushed(JSON.parse(m.text) as Brushed);
      if (m.kind === 'settled') return on.settled(JSON.parse(m.text) as Settled);
      if (m.kind === 'undone') return on.undone((JSON.parse(m.text) as { done: boolean }).done);
      if (m.discharge) this.discharge = m.discharge;
      if (m.plumes) this.plumes = m.plumes;
      if (m.relief) this.relief = { relief: m.relief, version: m.reliefVersion };
      this.version = m.reliefVersion;
      if (m.kind === 'world') {
        const meta = JSON.parse(m.meta) as Meta;
        meta.frames = [{ day: m.day, year: m.year }];
        this.world = makeWorld(meta, m.frame, 0);
        this.table = m.table;
        this.stats = m.stats;
        this.kinds = JSON.parse(m.kinds) as StartKind[];
        // The frame is answered even where the page fails to show it (the error goes to the page's error handler), so
        // the world never waits on it.
        try { timed('world frames', () => on.world(this.world!)); } finally { this.shown(); }
        return;
      }
      if (!this.world) return this.shown();
      // The world changes only as it steps: a frame of the date shown (a pause) brings its things and its stats, and
      // the map is not drawn anew (`changed` false), unless an edit changed it.
      const date = this.world.meta.frames[0], same = date.day === m.day && date.year === m.year;
      this.world.data = m.frame;
      this.table = m.table;
      this.world.meta.frames[0] = { day: m.day, year: m.year };
      this.stats = m.stats;
      try { timed('world frames', () => { on.things(m.things); on.frame(m.stats, !same || m.edited, m.edited); }); } finally { this.shown(); }
    };
    this.worker.postMessage({ cmd: 'init', wasm }, [wasm]);
    document.addEventListener('visibilitychange', this.onHide);
  }

  /** Once the map shows the frame whole (main.ts sets it to map.ts `MapView.shown`). */
  drawn: () => Promise<void> = () => Promise.resolve();

  /** Tell the worker the frames taken are drawn, so it may post the next: at the next animation frame, after the map's
   * own draw (which asked for that frame first), or after `ANSWER_MS` where no animation frame comes (a browser that
   * holds them back in a frame it deems out of view or idle, as Safari does in an iframe), and once the map shows it
   * whole (so a slow card slows the world's frames, and never gets more than it can draw); at once in a hidden page,
   * which draws nothing. */
  private shown(): void {
    this.unanswered++;
    if (document.hidden) return this.answer();
    const then = () => { clearTimeout(timer); this.drawn().then(() => this.answer()); };
    const timer = setTimeout(then, ANSWER_MS);
    requestAnimationFrame(then);
  }
  private answer(): void {
    if (this.unanswered > 0) { this.worker.postMessage({ cmd: 'shown', frames: this.unanswered }); this.unanswered = 0; }
  }
  /** The samples' discharge and load come since they were last taken, or null. */
  takeDischarge(): Uint16Array | null { const d = this.discharge; this.discharge = null; return d; }
  /** The relief the world built its samples from and its version, come since it was last taken, or null. */
  takeRelief(): Built | null { const r = this.relief; this.relief = null; return r; }
  /** A new island of `seed` and `size` between the latitudes `band` (south, north; NaN: the core's default band), its
   * large shape drawn from the seed if `drawn`, else one rise in the middle, its cells holding at most `variety` kinds
   * of cover, its weather soft if `soft` (the core's `soft_weather`), else strong. */
  newWorld(seed: number, size: number, band: [number, number], drawn: boolean, variety: number, soft: boolean): void { this.worker.postMessage({ cmd: 'new', seed, size, band, drawn, variety, soft }); }
  /** The saved world `saved` (the gzip of a snapshot of `ew run`), which then steps on as a new world does. */
  open(saved: ArrayBuffer): void { this.worker.postMessage({ cmd: 'open', saved }); }
  play(on: boolean): void { this.worker.postMessage({ cmd: 'play', on }); }
  /** Months of the world per second of real time. */
  rate(monthsPerSecond: number): void { this.worker.postMessage({ cmd: 'rate', rate: monthsPerSecond }); }
  /** Ask with every frame for the things of a window (x, y, w, h in squares) to draw one by one, or for none. */
  things(at: [number, number, number, number] | null): void { this.worker.postMessage({ cmd: 'things', at }); }
  /** Ask what lies under the place (x, y) in squares (`probe`), the core's answer to `answer`. */
  probe(x: number, y: number, answer: (p: Probe) => void): void { this.answers.push(answer); this.worker.postMessage({ cmd: 'probe', at: [x, y] }); }
  /** Paint with brush tool `tool` (god-tools.ts) circles of `radius` m about the places of `stamps` (x and y in squares,
   * and the amount there: m the ground moves at its centre, J/m² of heat, m of a lake's depth), planting the starting
   * kind of `row`: one message, one answer (`on.brushed`). */
  brush(tool: number, stamps: [number, number, number][], radius: number, row: number): void {
    const flat = Float64Array.from(stamps.flat());
    this.worker.postMessage({ cmd: 'brush', tool, stamps: flat, radius, row }, [flat.buffer]);
  }
  /** End the stroke: what its edits start that takes no time runs at once (answered by `on.settled`). */
  settle(): void { this.worker.postMessage({ cmd: 'settle' }); }
  /** Settle what the stroke under way did so far and go on with it (answered by a frame). */
  soften(): void { this.worker.postMessage({ cmd: 'soften' }); }
  /** The world as a snapshot file of `ew run`, its gzip (answered by `on.saved`, with its year). */
  save(): void { this.worker.postMessage({ cmd: 'save' }); }
  /** Put the world back as before the last stroke (answered by `on.undone`). */
  undo(): void { this.worker.postMessage({ cmd: 'undo' }); }
  /** The page's ground shows the samples of `version` now (answered by `on.preview`: what edits moved since). */
  groundShown(version: number): void { this.worker.postMessage({ cmd: 'groundShown', version }); }
  stop(): void { this.worker.terminate(); URL.revokeObjectURL(this.url); document.removeEventListener('visibilitychange', this.onHide); }
}
