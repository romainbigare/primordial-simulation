// The ground's tiles: a second Web Worker with its own instance of the core's WebAssembly (wasm/src/lib.rs `ground`,
// `tile`, `rock`), which needs no world, so the world steps in its own worker while tiles are made. The ground is that of a
// new island (`ground`, from its seed), or of a saved world handed in (`ground_of_world`). It runs as a separate
// script, built into the page as text (build.mjs), started from a Blob by ground.ts, and makes one tile at a time, as
// the page asks.
import type { GroundOf } from './ground';

interface Core {
  memory: WebAssembly.Memory;
  ground(seed: number, size: number, south: number, north: number, drawn: number): void;
  input(len: number): number;
  ground_of_world(): number;
  tile(depth: number, x: number, y: number): number;
  rock(): number;
  out_len(): number;
}
type Command =
  | { cmd: 'init'; wasm: ArrayBuffer; ground: GroundOf }
  | { cmd: 'tile'; depth: number; x: number; y: number };

const host = self as unknown as { postMessage(m: unknown, transfer: Transferable[]): void; onmessage: ((e: MessageEvent<Command>) => void) | null };
let core: Core | null = null;

host.onmessage = async e => {
  const m = e.data;
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
      // The rock of each of the terrain's samples, which never changes, with the first word that the worker is ready.
      const at = core.rock();
      const rock = new Uint16Array(core.memory.buffer.slice(at, at + core.out_len()));
      host.postMessage({ kind: 'ready', rock }, [rock.buffer]);
    } else if (m.cmd === 'tile' && core) {
      const at = core.tile(m.depth, m.x, m.y);
      const data = new Uint16Array(core.memory.buffer.slice(at, at + core.out_len()));
      host.postMessage({ kind: 'tile', depth: m.depth, x: m.x, y: m.y, data }, [data.buffer]);
    }
  } catch (err) {
    host.postMessage({ kind: 'error', text: String(err) }, []);
  }
};

export {};
