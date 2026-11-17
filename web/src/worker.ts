// The live world: the core as WebAssembly (wasm/src/lib.rs) in a Web Worker, so that the page never freezes. It runs as
// a separate script, built into the page as text (build.mjs), and started from a Blob. With each frame it posts the
// cells' table (tape/src/table.rs), from which the page draws the state of the ground, and the samples' discharge when
// it changed enough to show (wasm/src/lib.rs `discharge`); it makes no pictures. The page answers each frame once it has
// drawn it ('shown'). While the world plays, the next frame waits for that answer, and once it is due the world waits
// too: the world is never more than a frame's time ahead of the picture, so the date and the picture shown are the
// world's within about `FRAME_MS`, and a pause stops it there, even where the page draws slowly.

interface Core {
  memory: WebAssembly.Memory;
  new_world(seed: number, size: number, south: number, north: number, drawn: number, variety: number): number;
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
  | { cmd: 'new'; seed: number; size: number; band: [number, number]; drawn: boolean; variety: number }
  | { cmd: 'play'; on: boolean }
  | { cmd: 'rate'; rate: number }
  | { cmd: 'things'; at: [number, number, number, number] | null }
  | { cmd: 'shown'; frames: number };

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
/** Months owed to a finite rate, fractions included (at an infinite rate a tick steps as many as it holds and owes none). */
let due = 0;
let lastTick = 0;
let timer = 0;
/** The window (x, y, w, h in squares) whose things the page draws one by one (null: none). */
let thingsAt: [number, number, number, number] | null = null;
/** The months stepped since `since` (ms), and the pace measured over the last whole second, months per second. */
let stepped = 0, since = 0, pace = 0;
/** When the last frame was posted (ms), whether months were stepped since, and how many frames posted the page has not
 * yet drawn. */
let posted = 0, owed = false, unshown = 0;

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
  posted = now; owed = false; unshown++;
}

/** Is a frame due that waits for the page to draw the last one? Then the world waits with it. */
function waiting(): boolean {
  return unshown > 0 && performance.now() - posted >= FRAME_MS;
}

function tick(): void {
  const c = core!, t0 = performance.now(), bounded = Number.isFinite(rate);
  due = bounded ? Math.min(due + ((t0 - lastTick) / 1000) * rate, rate * 2) : 0;
  lastTick = t0;
  let made = 0;
  try {
    while (performance.now() - t0 < TICK_MS && (!bounded || due >= 1) && !waiting()) {
      c.step_month(); made++;
      if (bounded) due -= 1;
    }
    stepped += made;
    owed ||= made > 0;
    if (owed && unshown === 0 && performance.now() - posted >= FRAME_MS) postFrame('frame');
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
  if (playing) { lastTick = since = performance.now(); stepped = pace = unshown = 0; timer = setTimeout(tick, 0); }
}

host.onmessage = async e => {
  const m = e.data;
  try {
    if (m.cmd === 'init') {
      core = (await WebAssembly.instantiate(m.wasm, {})).instance.exports as unknown as Core;
      host.postMessage({ kind: 'ready' }, []);
    } else if (m.cmd === 'new' && core) {
      start(false);
      if (core.new_world(m.seed, m.size, m.band[0], m.band[1], m.drawn ? 1 : 0, m.variety) !== 0) throw new Error('The world was refused.');
      due = 0; thingsAt = null; stepped = pace = 0;
      postFrame('world');
    } else if (m.cmd === 'play') {
      // A pause shows the state it stopped at.
      const was = playing;
      start(m.on);
      if (was && !playing) postFrame('frame');
    }
    else if (m.cmd === 'rate') rate = m.rate;
    else if (m.cmd === 'shown') unshown = Math.max(0, unshown - m.frames);
    else if (m.cmd === 'things' && core) {
      thingsAt = m.at;
      if (!playing) postFrame('frame');
    }
  } catch (err) {
    host.postMessage({ kind: 'error', text: String(err) }, []);
  }
};


export {};
