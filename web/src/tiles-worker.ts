// The ground's tiles: a second Web Worker with its own instance of the core's WebAssembly (wasm/src/lib.rs `ground_from`,
// `tile`, `rock`), which holds no world, so the world steps in its own worker while tiles are made. The ground is made
// from what the world's ground is made of, its terrain's samples too, which the world's worker found and the page hands
// in (`ground_bytes`), and made again from them each time the world builds its samples anew (`ground_built`). It runs
// as a separate script, built into the page as text (build.mjs), started from a Blob by ground.ts, and makes one tile
// at a time, as the page asks, in the order it asks.

interface Core {
  memory: WebAssembly.Memory;
  input(len: number): number;
  ground_from(): number;
  tile(depth: number, x: number, y: number): number;
  rock(): number;
  ground_built(): number;
  out_len(): number;
}
type Command =
  | { cmd: 'init'; wasm: ArrayBuffer; ground: ArrayBuffer }
  | { cmd: 'tile'; depth: number; x: number; y: number; gen: number }
  | { cmd: 'built'; ground: ArrayBuffer; gen: number };

const host = self as unknown as { postMessage(m: unknown, transfer: Transferable[]): void; onmessage: ((e: MessageEvent<Command>) => void) | null };
let core: Core | null = null;

/** The messages, each handled once the one before is done: a tile or a build asked while the core is still starting
 * waits for it. */
let handled: Promise<void> = Promise.resolve();
host.onmessage = e => { handled = handled.then(() => handle(e.data)); };

/** Hand `data` to the core (`input`) and run `use`, which reads it: what it returns, at `at` and `out_len` long. */
function handIn(c: Core, data: ArrayBuffer, use: () => number): Uint8Array {
  const n = data.byteLength, into = c.input(n); // (it may grow the memory: its buffer is read after)
  new Uint8Array(c.memory.buffer, into, n).set(new Uint8Array(data));
  const at = use();
  return new Uint8Array(c.memory.buffer.slice(at, at + c.out_len()));
}

async function handle(m: Command): Promise<void> {
  try {
    if (m.cmd === 'init') {
      const c = core = (await WebAssembly.instantiate(m.wasm, {})).instance.exports as unknown as Core;
      // The world's ground's makings; the core answers nothing, or why it refused them.
      const refused = new TextDecoder().decode(handIn(c, m.ground, () => c.ground_from()));
      if (refused) throw new Error(refused);
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
      // changed, or why the makings were refused; then the rock again.
      const c = core, t0 = performance.now(), out = handIn(c, m.ground, () => c.ground_built());
      if (out.some(v => v > 1)) throw new Error(new TextDecoder().decode(out));
      const ms = performance.now() - t0, r = c.rock(), rock = new Uint16Array(c.memory.buffer.slice(r, r + c.out_len()));
      host.postMessage({ kind: 'rebuilt', squares: out, rock, gen: m.gen, ms }, [out.buffer, rock.buffer]);
    }
  } catch (err) {
    host.postMessage({ kind: 'error', text: err instanceof Error ? err.message : String(err) }, []);
  }
}

export {};
