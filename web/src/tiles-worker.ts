// The ground's tiles: a second Web Worker with its own instance of the core's WebAssembly (wasm/src/ground.rs `ground`,
// `tile`, `rock`), which needs no world, so the world steps in its own worker while tiles are made. The ground is that of a
// new island (`ground`, from its seed), or of a saved world handed in (`ground_of_world`), made again on the samples the
// world builds from the relief the page hands in (`ground_built`). It runs as a separate
// script, built into the page as text (build.mjs), started from a Blob by ground.ts, and makes one tile at a time, as
// the page asks, in the order it asks.
import type { GroundOf } from './ground';

interface Core {
  memory: WebAssembly.Memory;
  ground(seed: number, size: number, south: number, north: number, drawn: number): void;
  input(len: number): number;
  ground_of_world(): number;
  tile(depth: number, x: number, y: number): number;
  rock(): number;
  ground_built(): number;
  out_len(): number;
}
type Command =
  | { cmd: 'init'; wasm: ArrayBuffer; ground: GroundOf }
  | { cmd: 'tile'; depth: number; x: number; y: number; gen: number }
  | { cmd: 'built'; relief: ArrayBuffer; gen: number; answer: boolean };

const host = self as unknown as { postMessage(m: unknown, transfer: Transferable[]): void; onmessage: ((e: MessageEvent<Command>) => void) | null };
let core: Core | null = null;

/** The messages, each handled once the one before is done: a tile or a build asked while the core is still starting
 * waits for it. (The page hands a build only once every worker made the last, ground.ts `rebuild`.) */
let handled: Promise<void> = Promise.resolve();
host.onmessage = e => {
  handled = handled.then(() => handle(e.data));
};

async function handle(m: Command): Promise<void> {
  try {
    if (m.cmd === 'init') {
      core = (await WebAssembly.instantiate(m.wasm, {})).instance.exports as unknown as Core;
      const g = m.ground;
      if ('saved' in g) {
        // The saved world handed in (`input`); the core answers nothing, or why it refused it.
        const n = g.saved.byteLength;
        const into = core.input(n); // (it may grow the memory: its buffer is read after)
        new Uint8Array(core.memory.buffer, into, n).set(new Uint8Array(g.saved));
        const at = core.ground_of_world(), refused = new TextDecoder().decode(new Uint8Array(core.memory.buffer, at, core.out_len()));
        if (refused) throw new Error(refused);
      } else core.ground(g.seed, g.size, g.band[0], g.band[1], g.drawn ? 1 : 0);
      // The rock of each of the terrain's samples (again after each build, `built`), with the first word that the worker
      // is ready.
      const at = core.rock();
      const rock = new Uint16Array(core.memory.buffer.slice(at, at + core.out_len()));
      host.postMessage({ kind: 'ready', rock }, [rock.buffer]);
    } else if (m.cmd === 'tile' && core) {
      const t0 = performance.now(), at = core.tile(m.depth, m.x, m.y);
      const data = new Uint16Array(core.memory.buffer.slice(at, at + core.out_len()));
      // With the time it took, ms, and the bytes of the core's memory, for the page's cost line.
      host.postMessage({ kind: 'tile', depth: m.depth, x: m.x, y: m.y, data, gen: m.gen, ms: performance.now() - t0, memory: core.memory.buffer.byteLength }, [data.buffer]);
    } else if (m.cmd === 'built' && core) {
      // The ground made again on the samples the world built (`ground_built`): a byte a square, 1 where its tiles
      // changed, or why the relief was refused; then the rock again (of the worker that answers; the others say they
      // are done).
      const n = m.relief.byteLength, into = core.input(n);
      new Uint8Array(core.memory.buffer, into, n).set(new Uint8Array(m.relief));
      const at = core.ground_built(), out = new Uint8Array(core.memory.buffer.slice(at, at + core.out_len()));
      if (out.some(v => v > 1)) throw new Error(new TextDecoder().decode(out));
      if (!m.answer) { host.postMessage({ kind: 'rebuilt', gen: m.gen }, []); return; }
      const r = core.rock(), rock = new Uint16Array(core.memory.buffer.slice(r, r + core.out_len()));
      host.postMessage({ kind: 'rebuilt', squares: out, rock, gen: m.gen }, [out.buffer, rock.buffer]);
    }
  } catch (err) {
    host.postMessage({ kind: 'error', text: err instanceof Error ? err.message : String(err) }, []);
  }
}

export {};
