// The live world: the core as WebAssembly (wasm/src/lib.rs) in a Web Worker, so that the page never freezes. The world
// is new, or a saved one the page hands in (a snapshot of `ew run`), and steps on the same way. It runs as
// a separate script, built into the page as text (build.mjs), and started from a Blob. With each frame it posts the
// cells' table (tape/src/table.rs), from which the page draws the state of the ground, the samples' discharge and load when
// they changed enough to show (wasm/src/lib.rs `discharge`), where load came into a body of liquid (`plumes`), and the
// relief once the world built its samples anew (`relief`, with its version `reliefVersion`); it makes no pictures. The page answers each frame once it has
// drawn it ('shown'). While the world plays, the next frame waits for that answer and the world steps on at the pace
// asked: a page that draws slowly shows fewer frames (each the world's latest state), and never holds the world. A
// pause stops the world at once, and its frame shows the date it stopped at.

interface Core {
  memory: WebAssembly.Memory;
  new_world(seed: number, size: number, south: number, north: number, drawn: number, variety: number, soft: number): number;
  input(len: number): number;
  open_world(): number;
  step_month(): void;
  year(): number;
  day(): number;
  meta(): number;
  frame(): number;
  table(): number;
  discharge(): number;
  plumes(): number;
  ground_version(): number;
  settled_relief(): number;
  relief(): number;
  things(x: number, y: number, w: number, h: number): number;
  cells(): number;
  probe(x: number, y: number): number;
  kinds(): number;
  brush(tool: number, x: number, y: number, radius: number, amount: number, row: number): number;
  settle(): number;
  soften(): void;
  save_world(): number;
  undo(): number;
  hold(): void;
  ground_shown(version: number): void;
  preview(): number;
  out_len(): number;
}
type Command =
  | { cmd: 'init'; wasm: ArrayBuffer }
  | { cmd: 'new'; seed: number; size: number; band: [number, number]; drawn: boolean; variety: number; soft: boolean }
  | { cmd: 'open'; saved: ArrayBuffer }
  | { cmd: 'play'; on: boolean }
  | { cmd: 'rate'; rate: number }
  | { cmd: 'things'; at: [number, number, number, number] | null }
  | { cmd: 'probe'; at: [number, number] }
  | { cmd: 'brush'; tool: number; at: [number, number]; radius: number; amount?: number; row: number }
  | { cmd: 'settle' }
  | { cmd: 'soften' }
  | { cmd: 'undo' }
  | { cmd: 'groundShown'; version: number }
  | { cmd: 'save' }
  | { cmd: 'shown'; frames: number };

/** The numbers before the heights of a preview (wasm/src/edit.rs `preview`): its window and its version, u32. */
const PREVIEW_HEAD = 5;
/** Longest time a tick steps before it lets messages in, ms. */
const TICK_MS = 50;
/** Pause between ticks, ms. */
const GAP_MS = 4;
/** Real time between two world frames posted to the page while the world plays, at least, ms: each costs the page's
 * main thread a repaint of the map (its fields, its table and textures, a read back of the classes, the cover's field),
 * and one is posted only once the page drew the last. */
const FRAME_MS = 250;
/** Real time the pace of the world is measured over, ms. */
const PACE_MS = 1000;

const host = self as unknown as { postMessage(m: unknown, transfer: Transferable[]): void; onmessage: ((e: MessageEvent<Command>) => void) | null };
let core: Core | null = null;
/** The core compiled (`init`): a new or saved world after the core stopped stands on a new instance of it. */
let compiled: WebAssembly.Module | null = null;
/** Why the core stopped, in plain words, once a call trapped inside it (a panic aborts and leaves its state locked):
 * every command after is answered with it, till a new or saved world starts a new instance. */
let stopped = '';
let playing = false;
/** Months of the world per second of real time (Infinity: as many as a tick holds). */
let rate = 12;
/** Months owed to a finite rate, fractions included (at an infinite rate a tick steps as many as it holds and owes none). */
let due = 0;
let lastTick = 0;
let timer = 0;
/** The window (x, y, w, h in squares) whose things the page draws one by one (null: none). */
let thingsAt: [number, number, number, number] | null = null;
/** The months stepped since `since` (ms) and the time stepping them took, ms, and of the last whole second: the pace,
 * months per second, the time a month took, ms, and the share of the time spent stepping. */
let stepped = 0, since = 0, pace = 0, steppingMs = 0, monthMs = 0, busy = 0;
/** The version of the samples (`ground_version`) whose relief the page has: a new one's is sent with the next frame. */
let sentBuilds = 0;
/** When the last frame was posted (ms), whether months were stepped since, and how many frames posted the page has not
 * yet drawn. */
let posted = 0, owed = false, unshown = 0;

/** A copy of `n` bytes of the core's memory from `at`, which the page can take over. */
function bytes(at: number, n: number): Uint8Array {
  return new Uint8Array(core!.memory.buffer, at, n).slice();
}

/** Hand `data` to the core (`input`) and run `use`, which reads it: nothing back, or why it refused it, as text. */
function handIn(data: ArrayBuffer, use: () => number): string {
  const c = core!, n = data.byteLength;
  const into = c.input(n); // (it may grow the memory: its buffer is read after)
  new Uint8Array(c.memory.buffer, into, n).set(new Uint8Array(data));
  const at = use();
  return new TextDecoder().decode(bytes(at, c.out_len()));
}

/** What edits moved the ground since the ground the page shows (wasm/src/edit.rs `preview`): the window of the
 * terrain's samples (west column, north row, width, height), the version of the samples it moved from (`over`: the
 * page draws it over the tiles made of that version alone), and of its samples row by row how far each moved, m, and
 * the depth of its loose ground once built again, m. */
function preview(): { window: [number, number, number, number]; over: number; dh: Float32Array; loose: Float32Array } {
  const c = core!, raw = bytes(c.preview(), c.out_len());
  const head = new Uint32Array(raw.buffer, 0, PREVIEW_HEAD), n = head[2] * head[3], at = 4 * PREVIEW_HEAD;
  return { window: [head[0], head[1], head[2], head[3]], over: head[4], dh: new Float32Array(raw.buffer.slice(at, at + 4 * n)), loose: new Float32Array(raw.buffer.slice(at + 4 * n)) };
}

/** The text the core last returned at `at`. */
function text(at: number): string {
  return new TextDecoder().decode(bytes(at, core!.out_len()));
}

/** Post the world's frame: a new world's with its header and starting kinds; `edited`, a frame of the same date that
 * an edit changed. */
function postFrame(kind: 'world' | 'frame', edited = false): void {
  const c = core!;
  const head = kind === 'world' ? text(c.meta()) : '';
  const kinds = kind === 'world' ? text(c.kinds()) : '';
  const frame = bytes(c.frame(), c.out_len());
  const table = new Float32Array(bytes(c.table(), c.out_len()).buffer);
  const flows = c.discharge(), discharge = c.out_len() > 0 ? new Uint16Array(bytes(flows, c.out_len()).buffer) : null;
  const plumes = bytes(c.plumes(), c.out_len());
  // A new world's ground is the seed's, a saved one's as last built: the relief goes to the ground once the world builds
  // its samples anew.
  if (kind === 'world') sentBuilds = c.ground_version();
  const relief = c.ground_version() !== sentBuilds ? bytes(c.relief(), c.out_len()) : null;
  sentBuilds = c.ground_version();
  const things = thingsAt ? bytes(c.things(...thingsAt), c.out_len()) : null;
  const now = performance.now();
  if (now - since >= PACE_MS) {
    pace = (stepped * 1000) / (now - since); monthMs = stepped ? steppingMs / stepped : 0; busy = steppingMs / (now - since);
    stepped = 0; steppingMs = 0; since = now;
  }
  const stats = { cells: c.cells(), pace, monthMs, busy, memory: c.memory.buffer.byteLength };
  const moved = [frame.buffer, table.buffer, plumes.buffer, ...(things ? [things.buffer] : []), ...(discharge ? [discharge.buffer] : []), ...(relief ? [relief.buffer] : [])];
  host.postMessage({ kind, meta: head, kinds, edited, frame, table, discharge, plumes, relief, reliefVersion: sentBuilds, year: c.year(), day: c.day(), stats, things }, moved);
  posted = now; owed = false; unshown++;
}

function tick(): void {
  const c = core!, t0 = performance.now(), bounded = Number.isFinite(rate);
  due = bounded ? Math.min(due + ((t0 - lastTick) / 1000) * rate, rate * 2) : 0;
  lastTick = t0;
  let made = 0;
  try {
    while (performance.now() - t0 < TICK_MS && (!bounded || due >= 1)) {
      c.step_month(); made++;
      if (bounded) due -= 1;
    }
    stepped += made;
    steppingMs += performance.now() - t0;
    owed ||= made > 0;
    if (owed && unshown === 0 && performance.now() - posted >= FRAME_MS) postFrame('frame');
  } catch (e) {
    playing = false;
    failed(e);
    return;
  }
  if (playing) timer = setTimeout(tick, GAP_MS);
}

/** Tell the page of the error `e`; a trap inside the core stops it (`stopped`). */
function failed(e: unknown): void {
  if (e instanceof WebAssembly.RuntimeError) {
    core = null;
    stopped = `The world stopped: the core failed inside (${e.message}). Make a new world, or open a saved one.`;
  }
  host.postMessage({ kind: 'error', text: stopped || (e instanceof Error ? e.message : String(e)) }, []);
}

function start(on: boolean): void {
  clearTimeout(timer);
  playing = on && core !== null;
  if (playing) { lastTick = since = performance.now(); stepped = pace = steppingMs = unshown = 0; timer = setTimeout(tick, 0); }
}

host.onmessage = async e => {
  const m = e.data;
  try {
    if (m.cmd === 'init') {
      compiled = await WebAssembly.compile(m.wasm);
      core = (await WebAssembly.instantiate(compiled, {})).exports as unknown as Core;
      host.postMessage({ kind: 'ready' }, []);
    } else if ((m.cmd === 'new' || m.cmd === 'open') && compiled) {
      // A new world, or the saved one handed in (on a new instance of the core where it stopped).
      start(false);
      if (!core) { core = (await WebAssembly.instantiate(compiled, {})).exports as unknown as Core; stopped = ''; }
      const c = core, refused = m.cmd === 'open' ? handIn(m.saved, () => c.open_world())
        : c.new_world(m.seed, m.size, m.band[0], m.band[1], m.drawn ? 1 : 0, m.variety, m.soft ? 1 : 0) !== 0 ? 'The world was refused.' : '';
      if (refused) throw new Error(refused);
      due = 0; thingsAt = null; stepped = pace = 0;
      postFrame('world');
      c.hold();
    } else if (m.cmd === 'play') {
      // A pause shows the state it stopped at.
      const was = playing;
      start(m.on);
      if (was && !playing) postFrame('frame');
      // A paused world waits for edits: the core keeps a copy for the first stroke's undo now, not on its first call.
      if (!playing && core) core.hold();
    }
    else if (m.cmd === 'rate') rate = m.rate;
    else if (stopped && m.cmd !== 'shown') host.postMessage({ kind: 'error', text: stopped }, []);
    else if (m.cmd === 'shown') unshown = Math.max(0, unshown - m.frames);
    else if (m.cmd === 'probe' && core) host.postMessage({ kind: 'probe', text: text(core.probe(...m.at)) }, []);
    else if (m.cmd === 'brush' && core) {
      // The brush between two months (wasm/src/edit.rs `brush`): its answer says whether the page is sent the preview
      // of the ground it moved (drawn till the page's ground shows it) and a frame (with the relief of the ground's new
      // version, where it built the samples again).
      const answer = text(core.brush(m.tool, m.at[0], m.at[1], m.radius, m.amount ?? 0, m.row));
      const sends = JSON.parse(answer) as { preview: boolean; frame: boolean; refused?: string };
      if (sends.refused) throw new Error(sends.refused);
      host.postMessage({ kind: 'brushed', text: answer, ...(sends.preview ? { preview: preview() } : {}) }, []);
      if (sends.frame) postFrame('frame', true);
    }
    else if (m.cmd === 'save' && core) {
      // The world as a snapshot file (wasm/src/lib.rs `save_world`).
      const saved = bytes(core.save_world(), core.out_len());
      host.postMessage({ kind: 'saved', saved, year: core.year() }, [saved.buffer]);
    }
    else if ((m.cmd === 'settle' || m.cmd === 'soften') && core) {
      // The stroke ends, or a held stroke settles what it did so far and goes on (`soften`); what its edits start that
      // takes no time runs at once (`settle`), and its frame shows it. The relief of the ground's new version is sent
      // first, before the world settles, so the page's ground is made again meanwhile.
      const at = core.settled_relief(), n = core.out_len();
      if (n > 0) {
        const relief = bytes(at, n);
        sentBuilds = core.ground_version() + 1;
        host.postMessage({ kind: 'relief', relief, reliefVersion: sentBuilds }, [relief.buffer]);
      }
      // A stroke's end sends a frame where the world changed since the last (a lake's paint sent its own).
      let framed = true;
      if (m.cmd === 'settle') {
        const answer = text(core.settle());
        framed = (JSON.parse(answer) as { frame: boolean }).frame;
        host.postMessage({ kind: 'settled', text: answer }, []);
      } else core.soften();
      if (framed) postFrame('frame', true);
    }
    else if (m.cmd === 'undo' && core) {
      // The world as before the last stroke (`undo`): its frame, the relief of its ground's new version, and the preview
      // of what moved since the ground the page shows; then a copy kept for the next stroke (`hold`), as at a pause.
      host.postMessage({ kind: 'undone', text: text(core.undo()), preview: preview() }, []);
      postFrame('frame', true);
      core.hold();
    }
    else if (m.cmd === 'groundShown' && core) {
      // The page's ground shows the samples of this version now: what edits moved since (`preview`).
      core.ground_shown(m.version);
      host.postMessage({ kind: 'preview', preview: preview() }, []);
    }
    else if (m.cmd === 'things' && core) {
      // A playing world's next frame brings them; a paused one's come alone (the frame would be the same).
      thingsAt = m.at;
      if (!playing) {
        const things = thingsAt ? bytes(core.things(...thingsAt), core.out_len()) : null;
        host.postMessage({ kind: 'things', things }, things ? [things.buffer] : []);
      }
    }
  } catch (err) {
    failed(err);
  }
};


export {};
