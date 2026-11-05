import { World, makeWorld, Meta } from './data';

/** The live world: the core in a Web Worker (worker.ts) that makes a world, steps it and posts a frame as bytes. */
export class Live {
  world: World | null = null;
  private worker: Worker;
  private url: string;

  constructor(workerSource: string, wasm: ArrayBuffer, on: { ready: () => void; world: (w: World) => void; frame: () => void; error: (text: string) => void }) {
    this.url = URL.createObjectURL(new Blob([workerSource], { type: 'text/javascript' }));
    this.worker = new Worker(this.url);
    this.worker.onerror = e => on.error(e.message);
    this.worker.onmessage = e => {
      const m = e.data as { kind: string; meta: string; frame: Uint8Array; year: number; day: number; text: string };
      if (m.kind === 'ready') return on.ready();
      if (m.kind === 'error') return on.error(m.text);
      if (m.kind === 'world') {
        const meta = JSON.parse(m.meta) as Meta;
        meta.frames = [{ day: m.day, year: m.year }];
        this.world = makeWorld(meta, m.frame, 0);
        return on.world(this.world);
      }
      if (!this.world) return;
      this.world.data = m.frame;
      this.world.meta.frames[0] = { day: m.day, year: m.year };
      on.frame();
    };
    this.worker.postMessage({ cmd: 'init', wasm }, [wasm]);
  }

  newWorld(seed: number, size: number): void { this.worker.postMessage({ cmd: 'new', seed, size }); }
  play(on: boolean): void { this.worker.postMessage({ cmd: 'play', on }); }
  /** Months of the world per second of real time. */
  rate(monthsPerSecond: number): void { this.worker.postMessage({ cmd: 'rate', rate: monthsPerSecond }); }
  stop(): void { this.worker.terminate(); URL.revokeObjectURL(this.url); }
}
