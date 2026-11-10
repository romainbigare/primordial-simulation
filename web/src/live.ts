import { World, makeWorld, Meta } from './data';

/** What the core reports with each frame: its cells and the months stepped per second of real time. */
export interface Stats { cells: number; pace: number }

/** The live world: the core in a Web Worker (worker.ts) that makes a world, steps it and posts a frame as bytes with the
 * cells' table (tape/src/table.rs). */
export class Live {
  world: World | null = null;
  table: Float32Array<ArrayBufferLike> = new Float32Array(0);
  stats: Stats = { cells: 0, pace: 0 };
  private worker: Worker;
  private url: string;

  constructor(workerSource: string, wasm: ArrayBuffer, on: { ready: () => void; world: (w: World) => void; frame: (stats: Stats) => void; error: (text: string) => void; things: (data: Uint8Array | null) => void }) {
    this.url = URL.createObjectURL(new Blob([workerSource], { type: 'text/javascript' }));
    this.worker = new Worker(this.url);
    this.worker.onerror = e => on.error(e.message);
    this.worker.onmessage = e => {
      const m = e.data as { kind: string; meta: string; frame: Uint8Array; table: Float32Array; year: number; day: number; text: string; stats: Stats; things: Uint8Array | null };
      if (m.kind === 'ready') return on.ready();
      if (m.kind === 'error') return on.error(m.text);
      if (m.kind === 'world') {
        const meta = JSON.parse(m.meta) as Meta;
        meta.frames = [{ day: m.day, year: m.year }];
        this.world = makeWorld(meta, m.frame, 0);
        this.table = m.table;
        this.stats = m.stats;
        return on.world(this.world);
      }
      if (!this.world) return;
      this.world.data = m.frame;
      this.table = m.table;
      this.world.meta.frames[0] = { day: m.day, year: m.year };
      this.stats = m.stats;
      on.things(m.things);
      on.frame(m.stats);
    };
    this.worker.postMessage({ cmd: 'init', wasm }, [wasm]);
  }

  newWorld(seed: number, size: number): void { this.worker.postMessage({ cmd: 'new', seed, size }); }
  play(on: boolean): void { this.worker.postMessage({ cmd: 'play', on }); }
  /** Months of the world per second of real time. */
  rate(monthsPerSecond: number): void { this.worker.postMessage({ cmd: 'rate', rate: monthsPerSecond }); }
  /** Ask with every frame for the things of a window (x, y, w, h in squares) to draw one by one, or for none. */
  things(at: [number, number, number, number] | null): void { this.worker.postMessage({ cmd: 'things', at }); }
  stop(): void { this.worker.terminate(); URL.revokeObjectURL(this.url); }
}
