// The ground's detail up close (web/shaders/lib/detail.wgsl): photographs of ground seen from above, tiles of ground
// cover rendered from 3D models as the map sees them, and rendered plants (CC0, Poly Haven;
// web/textures/SOURCES.md), strip images beside the page that `STRIPS` names by group (`ground.jpg`, `ground2.jpg`, ...,
// a few layers a file; each also at smaller sides, `ground@128.jpg`, ...), each square of them a layer of one texture
// array with its mip levels down to one texel, which is its mean. Its layers are as large as the view needs
// (`sideFor`): a set of the next side is loaded as the view comes close, drawn from once whole. The map draws each at its
// real size as a change around the colour the world gives the ground (the image over its mean), so from afar, where the
// mips average it, the colour is the world's alone.
import { Gpu, Texture } from './gpu';
import { timed } from './cost';

/** The strips' files of each group, in its layers' order (web/textures/strips.json, written by the bake tools,
 * bake/split.py and bake/sprites.cjs, and put in the page by build.mjs). */
declare const STRIPS: Record<StripGroup, string[]>;
/** The groups of the strips in their layers' order in the texture array (tex.wgsl's `*_FROM`): the photographs, the
 * ground cover, the fire's, the snags, the rendered models, the heights of those that stand in water, and the trees
 * last (cut-outs). */
const STRIP_GROUPS = ['ground', 'carpets', 'fire', 'snags', 'models', 'heights', 'trees'] as const;
type StripGroup = typeof STRIP_GROUPS[number];

/** The side of each layer of the strips' files, texels (tex.wgsl's `*_TEX` and the rest: the photographs, the ground
 * cover, the fire's sprites and its photograph, the snags, the rendered models, the heights of those that stand in
 * water and the trees, as many layers as each strip holds), and the smaller sides their files are also kept at
 * (web/textures/bake/sizes.py: the mip levels the page makes from them). */
const PHOTO_SIDE = 512;
const SIDES = [64, 128, 256, PHOTO_SIDE];
/** The largest side loaded: on a screen touched with a finger (a phone's or a tablet's, whose browser gives a page
 * little memory) half the files' (50 MB at most, not 200). The shaders read the side from the texture. */
const SIDE_MOST = matchMedia('(pointer: coarse)').matches ? PHOTO_SIDE / 2 : PHOTO_SIDE;
/** The widest a layer lies on the ground, m: the rock's photograph repeats every 50 m (detail.wgsl `detail_m`); every
 * other layer (a photograph, a tile of cover, a plant) is narrower. A view whose pixel takes `e` m reads no layer
 * finer than this over `e` texels across (its mip level), so a side of that many shows the same picture. */
const SPAN_M = 50;
/** A set of layers is let go for one this many times smaller once the view needs no more (a zoom about one size does
 * not load the two over and over). */
const SHRINK = 4;

/** The view needs the detail where a pixel of the screen takes less than `DETAIL_LOAD_E` m (farther no layer is read
 * finer than its mean, which the blank texture gives too). */
const DETAIL_LOAD_E = 25;
/** The detail is loaded as for pixels this many times finer than the view's, so a zoom in finds it loaded (as gpu.ts
 * `ZOOM_AHEAD` makes the programs ahead). */
const DETAIL_AHEAD = 3;

/** The side of the layers a view whose pixel takes `e` m needs (`SPAN_M`), of `SIDES`, at most `SIDE_MOST`. */
function sideFor(e: number): number {
  return SIDES.find(s => s >= SPAN_M / e && s <= SIDE_MOST) ?? SIDE_MOST;
}

/** The blank texture's layers: an array of white texels (one layer would make a plain texture, not an array). */
const BLANK_LAYERS = 2;
/** Layers sent to the GPU a frame at most (each 1.4 MB with its levels at 512 texels a side): the strips go over some
 * frames, so no frame stops long for them, and each layer's levels are let go once sent. */
const LAYERS_A_FRAME = 4;
/** The GPU the detail is sent to (`sendDetail`) and what hears that a set of its layers is whole. */
let sending: { gpu: Gpu; sent: () => void } | null = null;
/** The detail drawn from: its texture, whole, and the side of its layers (none: the blank texture). */
let shown: { t: Texture; side: number } | null = null;
/** A set of layers being loaded (`load`): its number (a later load ends it), side, worker, texture, its layers (0 till
 * the strips' heads are read), those filled, their levels made and not yet sent, and the frame that sends the next. */
interface Loading { number: number; side: number; worker: Worker | null; t: Texture | null; layers: number; filled: number; made: { layer: number; levels: Uint8Array[] }[]; frame: number }
let loading: Loading | null = null;
let loads = 0;
const photoBlank = new WeakMap<Gpu, Texture>();

/** A cut-out texel shows where its cover is at least this (tex.wgsl `model_texel`, of the trees; detail-worker.ts
 * keeps it so in the mips). */
const CUT = 128;

/** The strips' files beside the page at layers `side` texels a side (web/textures/bake/sizes.py), in their layers'
 * order, and whether each is the trees' (cut-outs). */
function stripFiles(side: number): [string, boolean][] {
  const at = (f: string) => (side === PHOTO_SIDE ? f : f.replace(/(\.[a-z]+)$/, `@${side}$1`));
  return STRIP_GROUPS.flatMap(g => STRIPS[g].map(f => [at(f), g === 'trees'] as [string, boolean]));
}

/** The height of the image in `bytes`, px, read from its head (a PNG's IHDR, a JPEG's start of frame), so the layers
 * are known before any is decoded. */
function imageHeight(bytes: Uint8Array): number {
  const v = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (v.getUint32(0) === 0x89504e47) return v.getUint32(20);
  for (let at = 2; at + 9 < bytes.length; at += 2 + v.getUint16(at + 2)) {
    const marker = v.getUint16(at);
    if (marker >= 0xffc0 && marker <= 0xffcf && marker !== 0xffc4 && marker !== 0xffc8 && marker !== 0xffcc) return v.getUint16(at + 5);
  }
  throw new Error('an image with no height in its head');
}

/** Strips decoded and handed to the worker that it has not finished, at most: a few, so the worker always has the
 * next while the page holds a few small decoded files (4 MB each) and never all of them. */
const STRIPS_AHEAD = 3;

/** Fetch the strips, decode them on the page (`STRIPS_AHEAD` ahead of the worker at most) and make their levels off
 * the page's thread (detail-worker.ts), so the page does not stop for them; each layer is sent to the GPU as it comes.
 * A failed fetch, decode or worker leaves the ground without its detail and says why (`detailFailure`). */
async function fetchDetail(l: Loading): Promise<void> {
  const files = stripFiles(l.side), current = () => loading === l;
  const bytes = await Promise.all(files.map(([f]) => fetch(f).then(r => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(`${f}: ${r.status}`))))));
  if (!current()) return;
  const layers = bytes.map((b, k) => { try { return imageHeight(new Uint8Array(b)) / l.side; } catch (e) { throw new Error(`${files[k][0]}: ${e instanceof Error ? e.message : e}`); } });
  l.layers = layers.reduce((n, k) => n + k, 0);
  const source = document.getElementById('detail-src')?.textContent ?? '';
  const url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' })), w = new Worker(url);
  URL.revokeObjectURL(url);
  l.worker = w;
  // The strips the worker has not finished, and what waits for one to finish (or for the worker to fail).
  let ahead = 0, finished = () => {}, failed = (_: Error) => {};
  const one = () => {
    const p = new Promise<void>((go, fail) => { finished = go; failed = fail; });
    p.catch(() => {}); // (a failure is thrown where it is awaited, not reported before)
    return p;
  };
  w.onmessage = (e: MessageEvent<{ layer: number; levels: Uint8Array[] }>) => {
    if (e.data.layer < 0) { ahead--; return finished(); }
    l.made.push(e.data);
    sendSoon(l);
  };
  w.onerror = e => failed(new Error(e.message || 'its worker failed'));
  let waiting = one();
  for (let k = 0, first = 0; k < files.length; first += layers[k++]) {
    while (ahead >= STRIPS_AHEAD) { await waiting; waiting = one(); }
    const strip = await createImageBitmap(new Blob([bytes[k]])).catch(e => Promise.reject(new Error(`${files[k][0]}: ${e}`)));
    if (!current()) { strip.close(); return; }
    w.postMessage({ strip, first, cutOut: files[k][1], side: l.side, cut: CUT }, [strip]);
    ahead++;
  }
  while (ahead > 0) { await waiting; waiting = one(); }
  w.terminate();
  l.worker = null;
}

/** The view's pixels take `e` m of the screen's: load the layers of the side a view `DETAIL_AHEAD` times closer needs
 * (`sideFor`) where they are fine enough to show (`DETAIL_LOAD_E`): a larger set as it comes close, a smaller one once
 * it needs `SHRINK` times less. */
export function detailSeen(e: number): void {
  const ahead = e / DETAIL_AHEAD, want = sideFor(ahead), have = loading?.side ?? shown?.side;
  if (want === failedSide) return;
  if (have === undefined ? ahead < DETAIL_LOAD_E : want > have || want * SHRINK <= have) loadSide(want);
}

/** The side whose load failed last (none: null): it is not tried again, so a file that is missing or broken is not
 * fetched at every frame. */
let failedSide: number | null = null;

/** Load the layers at `side` texels a side; the set drawn from stays till they are whole (`sendSoon`). */
function loadSide(side: number): void {
  stopLoading();
  const l: Loading = { number: ++loads, side, worker: null, t: null, layers: 0, filled: 0, made: [], frame: 0 };
  loading = l;
  fetchDetail(l).catch(e => {
    if (loading !== l) return;
    // The load stops (nothing more of it is waited for: the picture settles), said on the page once and kept for the
    // cost line: the ground is drawn without its detail, or with the set before.
    stopLoading();
    failedSide = side;
    if (!detailFailed) reportError(new Error(`detail: ${e instanceof Error ? e.message : e}`));
    detailFailed = String(e instanceof Error ? e.message : e);
  });
}

/** Stop the load under way: its worker, its texture and its levels not yet sent let go. */
function stopLoading(): void {
  const l = loading;
  if (!l) return;
  l.worker?.terminate();
  cancelAnimationFrame(l.frame);
  if (l.t) sending?.gpu.drop(l.t);
  loading = null;
}

/** The detail's layers sent to the GPU, and all of the set (none known before the strips' heads are read), and the
 * side of the layers drawn from (0: none). */
export function detailSent(): [number, number, number] {
  const whole = shown?.t.layers ?? 0;
  return loading ? [loading.filled, loading.layers, shown?.side ?? 0] : [whole, whole, shown?.side ?? 0];
}

/** Why the detail last failed to load (null: it did not). */
let detailFailed: string | null = null;
export function detailFailure(): string | null { return detailFailed; }

/** Whether detail the view asked for is still to come (map.ts `settled` waits for it). */
export function detailComing(): boolean { return loading !== null; }

/** Send the detail to `gpu` as its layers come, `LAYERS_A_FRAME` a frame; `sent` hears once a set is whole. */
export function sendDetail(gpu: Gpu, sent: () => void): void {
  sending = { gpu, sent };
  if (loading) sendSoon(loading);
}

/** After the GPU lost what it held (its textures are empty again): the set drawn, or being loaded, is loaded anew. */
export function detailLost(): void {
  const side = loading?.side ?? shown?.side;
  stopLoading();
  if (shown) sending?.gpu.drop(shown.t);
  shown = null;
  if (side !== undefined) loadSide(side);
}

/** Whether the layers that came wait to be sent to the GPU (the map holds them while the view moves, map.ts `hold`). */
let held = false;

/** Hold the layers that come (`held`), or send them again. */
export function detailHeld(hold: boolean): void {
  if (hold === held) return;
  held = hold;
  if (!held && loading) sendSoon(loading);
}

/** Send the next layers of load `l` that came, at the next frame, while they are not held; once all are, draw from
 * them. */
function sendSoon(l: Loading): void {
  const s = sending;
  if (!s || l.frame || !l.made.length || held) return;
  l.frame = requestAnimationFrame(() => timed('detail', () => {
    l.frame = 0;
    if (loading !== l || held) return;
    l.t ??= s.gpu.texture('detail', l.side, l.side, 'rgba8unorm', l.layers, Math.log2(l.side) + 1);
    for (const { layer, levels } of l.made.splice(0, LAYERS_A_FRAME)) {
      levels.forEach((d, k) => s.gpu.write(l.t!, d, k, layer));
      l.filled++;
    }
    if (l.filled < l.layers) return sendSoon(l);
    if (shown) s.gpu.drop(shown.t);
    shown = { t: l.t, side: l.side };
    loading = null;
    s.sent();
  }));
}

/** The ground's detail on `gpu`: the whole set drawn from, else one white texel a layer (no detail). */
export function groundDetail(gpu: Gpu): Texture {
  if (sending?.gpu === gpu && shown) return shown.t;
  let t = photoBlank.get(gpu);
  if (!t) {
    t = gpu.texture('detail', 1, 1, 'rgba8unorm', BLANK_LAYERS, 1);
    gpu.write(t, new Uint8Array(4 * BLANK_LAYERS).fill(255));
    photoBlank.set(gpu, t);
  }
  return t;
}
