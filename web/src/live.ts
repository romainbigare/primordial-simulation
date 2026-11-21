import { World, makeWorld, Meta } from './data';

/** What the core reports with each frame: its cells and the months stepped per second of real time. */
export interface Stats { cells: number; pace: number }

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
  /** The relief the world last built its samples from, come since it was last taken (wasm/src/lib.rs `relief`). */
  private relief: Uint8Array | null = null;
  stats: Stats = { cells: 0, pace: 0 };
  private worker: Worker;
  private url: string;
  /** The frames taken that wait for their answer (`shown`), and the listener that answers them when the page is hidden. */
  private unanswered = 0;
  private onHide = () => { if (document.hidden) this.answer(); };

  constructor(workerSource: string, wasm: ArrayBuffer, on: { ready: () => void; world: (w: World) => void; frame: (stats: Stats, changed: boolean) => void; error: (text: string) => void; things: (data: Uint8Array | null) => void }) {
    this.url = URL.createObjectURL(new Blob([workerSource], { type: 'text/javascript' }));
    this.worker = new Worker(this.url);
    this.worker.onerror = e => on.error(e.message);
    this.worker.onmessage = e => {
      const m = e.data as { kind: string; meta: string; frame: Uint8Array; table: Float32Array; discharge: Uint16Array | null; plumes: Uint8Array; relief: Uint8Array | null; year: number; day: number; text: string; stats: Stats; things: Uint8Array | null };
      if (m.kind === 'ready') return on.ready();
      if (m.kind === 'error') return on.error(m.text);
      if (m.discharge) this.discharge = m.discharge;
      if (m.plumes) this.plumes = m.plumes;
      if (m.relief) this.relief = m.relief;
      if (m.kind === 'world') {
        const meta = JSON.parse(m.meta) as Meta;
        meta.frames = [{ day: m.day, year: m.year }];
        this.world = makeWorld(meta, m.frame, 0);
        this.table = m.table;
        this.stats = m.stats;
        on.world(this.world);
        return this.shown();
      }
      if (!this.world) return this.shown();
      // The world changes only as it steps: a frame of the date shown (a pause, or the things of another window)
      // brings its things and its stats, and the map is not drawn anew (`changed` false).
      const date = this.world.meta.frames[0], same = date.day === m.day && date.year === m.year;
      this.world.data = m.frame;
      this.table = m.table;
      this.world.meta.frames[0] = { day: m.day, year: m.year };
      this.stats = m.stats;
      on.things(m.things);
      on.frame(m.stats, !same);
      this.shown();
    };
    this.worker.postMessage({ cmd: 'init', wasm }, [wasm]);
    document.addEventListener('visibilitychange', this.onHide);
  }

  /** Tell the worker the frames taken are drawn, so it may post the next: at the next animation frame, after the map's
   * own draw (which asked for that frame first); at once in a hidden page, which draws nothing. */
  private shown(): void {
    this.unanswered++;
    if (document.hidden) this.answer(); else requestAnimationFrame(() => this.answer());
  }
  private answer(): void {
    if (this.unanswered > 0) { this.worker.postMessage({ cmd: 'shown', frames: this.unanswered }); this.unanswered = 0; }
  }
  /** The samples' discharge and load come since they were last taken, or null. */
  takeDischarge(): Uint16Array | null { const d = this.discharge; this.discharge = null; return d; }
  /** The relief the world built its samples from, come since it was last taken, or null. */
  takeRelief(): Uint8Array | null { const r = this.relief; this.relief = null; return r; }
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
  stop(): void { this.worker.terminate(); URL.revokeObjectURL(this.url); document.removeEventListener('visibilitychange', this.onHide); }
}
