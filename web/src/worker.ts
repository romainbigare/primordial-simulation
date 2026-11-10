// The live world: the core as WebAssembly (wasm/src/lib.rs) in a Web Worker, so that the page never freezes.
// It runs as a separate script, built into the page as text with data.ts, fields.ts, earth.ts, fine.ts and canopy.ts
// (build.mjs), and started from a Blob. It also makes the fields of the window in view finer than its cells (fine.ts),
// which the page hands to the GPU.
import { Meta, makeWorld } from './data';
import { wetOf } from './earth';
import { FineAt, fineFields, readFine } from './fine';
import { fineCover } from './canopy';
import { fieldBuffers } from './fields';
import type { FineWindow } from './live';

interface Core {
  memory: WebAssembly.Memory;
  new_world(seed: number, size: number): number;
  step_month(): void;
  year(): number;
  day(): number;
  meta(): number;
  frame(): number;
  fine(x: number, y: number, w: number, h: number, depth: number, layers: number): number;
  things(x: number, y: number, w: number, h: number): number;
  cells(): number;
  out_len(): number;
}
type Command =
  | { cmd: 'init'; wasm: ArrayBuffer }
  | { cmd: 'new'; seed: number; size: number }
  | { cmd: 'play'; on: boolean }
  | { cmd: 'rate'; rate: number }
  | { cmd: 'things'; at: [number, number, number, number] | null }
  | { cmd: 'fine'; at: FineAt | null };

/** Longest time a tick steps before it posts a frame and lets messages in, ms. */
const TICK_MS = 50;
/** Pause between ticks, ms. */
const GAP_MS = 4;
/** Real time the pace of the world is measured over, ms. */
const PACE_MS = 1000;
/** Real time between two fine rasters of the same window while the world plays, at least, ms (a new window gets one at
 * once). */
const FINE_MS = 1000;
/** Of an unchanged window the next raster comes no sooner than this many times the time the last one took after it, so
 * the rasters take at most about a tenth of the worker's time and the world steps the rest. */
const FINE_REST = 9;

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
/** The window the page draws finer than its cells (null: none), whether it waits for its raster, when the last was
 * made and how long it took, ms. */
let fineAt: FineAt | null = null, fineDue = false, fineLast = 0, fineTook = 0;
/** The world's header. */
let meta: Meta | null = null;
/** The months stepped since `since` (ms), and the pace measured over the last whole second, months per second. */
let stepped = 0, since = 0, pace = 0;

/** A copy of `n` bytes of the core's memory from `at`, which the page can take over. */
function bytes(at: number, n: number): Uint8Array {
  return new Uint8Array(core!.memory.buffer, at, n).slice();
}

/** The buffers of a fine raster's message, handed over to the page. */
const fineBuffers = (p: FineWindow): Transferable[] => [...fieldBuffers(p.fields), ...(p.cover ? [p.cover.data, ...p.cover.mips].map(a => a.buffer) : [])];

function postFrame(kind: 'world' | 'frame'): void {
  const c = core!;
  const head = kind === 'world' ? new TextDecoder().decode(bytes(c.meta(), c.out_len())) : '';
  if (kind === 'world') meta = JSON.parse(head) as Meta;
  meta!.frames = [{ day: c.day(), year: c.year() }];
  const frame = bytes(c.frame(), c.out_len());
  const things = thingsAt ? bytes(c.things(...thingsAt), c.out_len()) : null;
  let now = performance.now(), win: FineWindow | null = null;
  if (fineAt && (fineDue || now - fineLast >= Math.max(FINE_MS, FINE_REST * fineTook))) {
    const layers = meta!.layers.reduce((bits, l, k) => (fineAt!.layers.includes(l.name) ? bits | (1 << k) : bits), 0) >>> 0;
    const data = bytes(c.fine(fineAt.x, fineAt.y, fineAt.w, fineAt.h, fineAt.depth, layers), c.out_len());
    const made = performance.now() - now, f = readFine(meta!, fineAt, data);
    const fields = fineFields(f, meta!, wetOf(makeWorld(meta!, frame, 0), 0));
    win = { at: fineAt, fields, made, fieldsMs: performance.now() - now - made, cover: fineCover(f) };
    fineTook = performance.now() - now;
    now = fineLast = performance.now(); fineDue = false;
  }
  if (now - since >= PACE_MS) { pace = (stepped * 1000) / (now - since); stepped = 0; since = now; }
  const stats = { cells: c.cells(), pace };
  const moved = [frame.buffer, ...(win ? fineBuffers(win) : []), ...(things ? [things.buffer] : [])];
  host.postMessage({ kind, meta: head, frame, year: c.year(), day: c.day(), stats, fine: win, things }, moved);
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
      due = 0; thingsAt = null; fineAt = null; stepped = pace = 0;
      postFrame('world');
    } else if (m.cmd === 'play') start(m.on);
    else if (m.cmd === 'rate') rate = m.rate;
    else if (m.cmd === 'things' && core) {
      thingsAt = m.at;
      if (!playing) postFrame('frame');
    } else if (m.cmd === 'fine' && core) {
      fineAt = m.at; fineDue = fineAt !== null;
      if (!playing && fineDue) postFrame('frame');
    }
  } catch (err) {
    host.postMessage({ kind: 'error', text: String(err) }, []);
  }
};

