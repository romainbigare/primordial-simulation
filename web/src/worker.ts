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
  frame_window(x: number, y: number, w: number, h: number, across: number, down: number): number;
  set_camera(x: number, y: number, w: number, h: number, depth: number, rest: number): void;
  things(x: number, y: number, w: number, h: number): number;
  cells(): number;
  finest(): number;
  out_len(): number;
}
type Command =
  | { cmd: 'init'; wasm: ArrayBuffer }
  | { cmd: 'new'; seed: number; size: number }
  | { cmd: 'play'; on: boolean }
  | { cmd: 'rate'; rate: number }
  | { cmd: 'look'; look: Look | null; rest: boolean }
  | { cmd: 'things'; at: [number, number, number, number] | null };

/** What the page looks at: the window in map squares (from column x and row y), the depth of cells it asks for, and the raster of it that comes back (across x down pixels). */
interface Look { x: number; y: number; w: number; h: number; depth: number; across: number; down: number }

/** Longest time a tick steps before it posts a frame and lets messages in, ms. */
const TICK_MS = 50;
/** Pause between ticks, ms. */
const GAP_MS = 4;
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
/** The window the page looks at (null: the whole map only), and whether the camera is on it. */
let look: Look | null = null;
let cameraOn = false;
/** Whether the camera was sent away to rest (the core's budget of cells is then the cells as built). */
let resting = false;
/** The window (x, y, w, h in squares) whose things the page draws one by one (null: none). */
let thingsAt: [number, number, number, number] | null = null;
/** The months stepped since `since` (ms), and the pace measured over the last whole second, months per second. */
let stepped = 0, since = 0, pace = 0;

/** A copy of `n` bytes of the core's memory from `at`, which the page can take over. */
function bytes(at: number, n: number): Uint8Array {
  return new Uint8Array(core!.memory.buffer, at, n).slice();
}

function postFrame(kind: 'world' | 'frame'): void {
  const c = core!;
  const meta = kind === 'world' ? new TextDecoder().decode(bytes(c.meta(), c.out_len())) : '';
  const frame = bytes(c.frame(), c.out_len());
  const fine = look ? bytes(c.frame_window(look.x, look.y, look.w, look.h, look.across, look.down), c.out_len()) : null;
  const things = thingsAt ? bytes(c.things(...thingsAt), c.out_len()) : null;
  const now = performance.now();
  if (now - since >= PACE_MS) { pace = (stepped * 1000) / (now - since); stepped = 0; since = now; }
  const stats = { cells: c.cells(), finest: c.finest(), pace };
  const moved = [frame.buffer, ...(fine ? [fine.buffer] : []), ...(things ? [things.buffer] : [])];
  host.postMessage({ kind, meta, frame, year: c.year(), day: c.day(), stats, look, fine, things }, moved);
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
    if (made) postFrame('frame');
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
      due = 0; look = null; cameraOn = false; resting = false; thingsAt = null; stepped = pace = 0;
      postFrame('world');
    } else if (m.cmd === 'play') start(m.on);
    else if (m.cmd === 'rate') rate = m.rate;
    else if (m.cmd === 'look' && core) {
      look = m.look;
      if (look) core.set_camera(look.x, look.y, look.w, look.h, look.depth, 0);
      else if (cameraOn || resting || m.rest) core.set_camera(0, 0, 0, 0, 0, m.rest ? 1 : 0);
      cameraOn = look !== null;
      resting = m.rest;
      if (!playing) postFrame('frame');
    } else if (m.cmd === 'things' && core) {
      thingsAt = m.at;
      if (!playing) postFrame('frame');
    }
  } catch (err) {
    host.postMessage({ kind: 'error', text: String(err) }, []);
  }
};

export {};
