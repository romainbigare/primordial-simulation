// The live world: the core as WebAssembly (wasm/src/lib.rs) in a Web Worker, so that the page never freezes.
// It runs as a separate script, built into the page as text (build.mjs) and started from a Blob.

interface Core {
  memory: WebAssembly.Memory;
  new_world(seed: number, size: number): number;
  step_month(): void;
  year(): number;
  day(): number;
  meta(): number;
  frame(): number;
  out_len(): number;
}
type Command =
  | { cmd: 'init'; wasm: ArrayBuffer }
  | { cmd: 'new'; seed: number; size: number }
  | { cmd: 'play'; on: boolean }
  | { cmd: 'rate'; rate: number };

/** Longest time a tick steps before it posts a frame and lets messages in, ms. */
const TICK_MS = 50;
/** Pause between ticks, ms. */
const GAP_MS = 4;

const host = self as unknown as { postMessage(m: unknown, transfer: Transferable[]): void; onmessage: ((e: MessageEvent<Command>) => void) | null };
let core: Core | null = null;
let playing = false;
/** Months of the world per second of real time (Infinity: as many as a tick holds). */
let rate = 12;
/** Months owed to the rate, fractions included. */
let due = 0;
let lastTick = 0;
let timer = 0;

/** A copy of `n` bytes of the core's memory from `at`, which the page can take over. */
function bytes(at: number, n: number): Uint8Array {
  return new Uint8Array(core!.memory.buffer, at, n).slice();
}

function postFrame(kind: 'world' | 'frame'): void {
  const c = core!;
  const meta = kind === 'world' ? new TextDecoder().decode(bytes(c.meta(), c.out_len())) : '';
  const frame = bytes(c.frame(), c.out_len());
  host.postMessage({ kind, meta, frame, year: c.year(), day: c.day() }, [frame.buffer]);
}

function tick(): void {
  const c = core!, t0 = performance.now();
  due = Math.min(due + ((t0 - lastTick) / 1000) * rate, rate * 2);
  lastTick = t0;
  let stepped = 0;
  try {
    while (performance.now() - t0 < TICK_MS && (due >= 1 || rate === Infinity)) {
      c.step_month(); due -= 1; stepped++;
    }
    if (stepped) postFrame('frame');
  } catch (e) {
    playing = false;
    host.postMessage({ kind: 'error', text: `The world stopped: ${String(e)}` }, []);
    return;
  }
  if (playing) timer = setTimeout(tick, GAP_MS);
}

function start(on: boolean): void {
  clearTimeout(timer);
  playing = on && core !== null;
  if (playing) { lastTick = performance.now(); timer = setTimeout(tick, 0); }
}

host.onmessage = async e => {
  const m = e.data;
  try {
    if (m.cmd === 'init') {
      core = (await WebAssembly.instantiate(m.wasm, {})).instance.exports as unknown as Core;
      host.postMessage({ kind: 'ready' }, []);
    } else if (m.cmd === 'new' && core) {
      start(false);
      if (core.new_world(m.seed, m.size) !== 0) throw new Error('The world was refused.');
      due = 0;
      postFrame('world');
    } else if (m.cmd === 'play') start(m.on);
    else if (m.cmd === 'rate') rate = m.rate;
  } catch (err) {
    host.postMessage({ kind: 'error', text: String(err) }, []);
  }
};

export {};
