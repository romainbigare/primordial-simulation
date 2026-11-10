// The live world: the core as WebAssembly (wasm/src/lib.rs) in a Web Worker, so that the page never freezes. It runs as
// a separate script, built into the page as text (build.mjs), and started from a Blob. With each frame it posts the
// cells' table (tape/src/table.rs), from which the page draws the state of the ground, and the samples' discharge when
// it changed enough to show (wasm/src/lib.rs `discharge`); it makes no pictures.

interface Core {
  memory: WebAssembly.Memory;
  new_world(seed: number, size: number): number;
  step_month(): void;
  year(): number;
  day(): number;
  meta(): number;
  frame(): number;
  table(): number;
  discharge(): number;
  things(x: number, y: number, w: number, h: number): number;
  cells(): number;
  out_len(): number;
}
type Command =
  | { cmd: 'init'; wasm: ArrayBuffer }
  | { cmd: 'new'; seed: number; size: number }
  | { cmd: 'play'; on: boolean }
  | { cmd: 'rate'; rate: number }
  | { cmd: 'things'; at: [number, number, number, number] | null };

/** Longest time a tick steps before it lets messages in, ms. */
const TICK_MS = 50;
/** Pause between ticks, ms. */
const GAP_MS = 4;
/** Real time between two world frames posted to the page while the world plays, at least, ms: each costs the page's
 * main thread a repaint of the map (its fields, its table and textures, a read back of the classes, the cover's field). */
const FRAME_MS = 250;
/** Real time the pace of the world is measured over, ms. */
const PACE_MS = 1000;

const host = self as unknown as { postMessage(m: unknown, transfer: Transferable[]): void; onmessage: ((e: MessageEvent<Command>) => void) | null };
let core: Core | null = null;
let playing = false;
/** Months of the world per second of real time (Infinity: as many as a tick holds). */
let rate = 12;
/** Months owed to the rate, fractions included. */
let due = 0;
let lastTick = 0;
let timer = 0;
/** The window (x, y, w, h in squares) whose things the page draws one by one (null: none). */
let thingsAt: [number, number, number, number] | null = null;
/** The months stepped since `since` (ms), and the pace measured over the last whole second, months per second. */
let stepped = 0, since = 0, pace = 0;
/** When the last frame was posted (ms), and whether months were stepped since. */
let posted = 0, owed = false;

/** A copy of `n` bytes of the core's memory from `at`, which the page can take over. */
function bytes(at: number, n: number): Uint8Array {
  return new Uint8Array(core!.memory.buffer, at, n).slice();
}

function postFrame(kind: 'world' | 'frame'): void {
  const c = core!;
  const head = kind === 'world' ? new TextDecoder().decode(bytes(c.meta(), c.out_len())) : '';
  const frame = bytes(c.frame(), c.out_len());
  const table = new Float32Array(bytes(c.table(), c.out_len()).buffer);
  const flows = c.discharge(), discharge = c.out_len() > 0 ? new Uint16Array(bytes(flows, c.out_len()).buffer) : null;
  const things = thingsAt ? bytes(c.things(...thingsAt), c.out_len()) : null;
  const now = performance.now();
  if (now - since >= PACE_MS) { pace = (stepped * 1000) / (now - since); stepped = 0; since = now; }
  const stats = { cells: c.cells(), pace };
  const moved = [frame.buffer, table.buffer, ...(things ? [things.buffer] : []), ...(discharge ? [discharge.buffer] : [])];
  host.postMessage({ kind, meta: head, frame, table, discharge, year: c.year(), day: c.day(), stats, things }, moved);
  posted = now; owed = false;
}

function tick(): void {
  const c = core!, t0 = performance.now();
  due = Math.min(due + ((t0 - lastTick) / 1000) * rate, rate * 2);
  lastTick = t0;
  let made = 0;
  try {
    while (performance.now() - t0 < TICK_MS && (due >= 1 || rate === Infinity)) {
      c.step_month(); due -= 1; made++;
    }
    stepped += made;
    owed ||= made > 0;
    if (owed && performance.now() - posted >= FRAME_MS) postFrame('frame');
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
  if (playing) { lastTick = since = performance.now(); stepped = pace = 0; timer = setTimeout(tick, 0); }
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
      due = 0; thingsAt = null; stepped = pace = 0;
      postFrame('world');
    } else if (m.cmd === 'play') {
      // A pause shows the state it stopped at.
      const was = playing;
      start(m.on);
      if (was && !playing) postFrame('frame');
    }
    else if (m.cmd === 'rate') rate = m.rate;
    else if (m.cmd === 'things' && core) {
      thingsAt = m.at;
      if (!playing) postFrame('frame');
    }
  } catch (err) {
    host.postMessage({ kind: 'error', text: String(err) }, []);
  }
};


export {};
