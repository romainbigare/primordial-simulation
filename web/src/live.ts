import { World, makeWorld, Meta } from './data';
import { FineAt } from './fine';
import { FineCover } from './canopy';

/** Where the camera looks: the window in map squares and the depth of cells it asks for. */
export interface Look { x: number; y: number; w: number; h: number; depth: number }
/** The window in view finer than its cells as the worker painted it (fine.ts): its window, its picture, the times the
 * core took to make it and the worker to paint it, ms, and the cover of its pixels (canopy.ts). */
export interface FinePicture { at: FineAt; picture: ImageBitmap; made: number; painted: number; cover: FineCover | null }
/** What the core reports with each frame: its cells, the finest depth among them, and the months stepped per second of real time. */
export interface Stats { cells: number; finest: number; pace: number }

/** The live world: the core in a Web Worker (worker.ts) that makes a world, steps it and posts a frame as bytes. */
export class Live {
  world: World | null = null;
  stats: Stats = { cells: 0, finest: 0, pace: 0 };
  private worker: Worker;
  private url: string;

  constructor(workerSource: string, wasm: ArrayBuffer, on: { ready: () => void; world: (w: World) => void; frame: (stats: Stats, fine: FinePicture | null) => void; fine: (fine: FinePicture) => void; error: (text: string) => void; things: (data: Uint8Array | null) => void }) {
    this.url = URL.createObjectURL(new Blob([workerSource], { type: 'text/javascript' }));
    this.worker = new Worker(this.url);
    this.worker.onerror = e => on.error(e.message);
    this.worker.onmessage = e => {
      const m = e.data as { kind: string; meta: string; frame: Uint8Array; year: number; day: number; text: string; stats: Stats; fine: FinePicture | null; things: Uint8Array | null };
      if (m.kind === 'ready') return on.ready();
      if (m.kind === 'error') return on.error(m.text);
      if (m.kind === 'fine') return on.fine(m.fine!);
      if (m.kind === 'world') {
        const meta = JSON.parse(m.meta) as Meta;
        meta.frames = [{ day: m.day, year: m.year }];
        this.world = makeWorld(meta, m.frame, 0);
        this.stats = m.stats;
        return on.world(this.world);
      }
      if (!this.world) return;
      this.world.data = m.frame;
      this.world.meta.frames[0] = { day: m.day, year: m.year };
      this.stats = m.stats;
      on.things(m.things);
      on.frame(m.stats, m.fine);
    };
    this.worker.postMessage({ cmd: 'init', wasm }, [wasm]);
  }

  newWorld(seed: number, size: number): void { this.worker.postMessage({ cmd: 'new', seed, size }); }
  play(on: boolean): void { this.worker.postMessage({ cmd: 'play', on }); }
  /** Months of the world per second of real time. */
  rate(monthsPerSecond: number): void { this.worker.postMessage({ cmd: 'rate', rate: monthsPerSecond }); }
  /** Look at a window (the core refines the cells there), or at the whole map only; `rest`: the camera is away and the cells go back to those as built. */
  look(look: Look | null, rest = false): void { this.worker.postMessage({ cmd: 'look', look, rest }); }
  /** Ask with every frame for the things of a window (x, y, w, h in squares) to draw one by one, or for none. */
  things(at: [number, number, number, number] | null): void { this.worker.postMessage({ cmd: 'things', at }); }
  /** Ask for a picture of a window finer than its cells, at once and then about once a second while the world plays, or for none. */
  fine(at: FineAt | null): void { this.worker.postMessage({ cmd: 'fine', at }); }
  /** The view the page shows (a layer's name, 'earth' or 'true') and whether it draws the worn paths: the worker paints its pictures so. */
  view(mode: string, paths: boolean): void { this.worker.postMessage({ cmd: 'view', mode, paths }); }
  stop(): void { this.worker.terminate(); URL.revokeObjectURL(this.url); }
}
