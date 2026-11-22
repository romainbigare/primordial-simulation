// The ground's detail up close (web/shaders/lib/detail.wgsl): photographs of ground seen from above, tiles of ground
// cover rendered from 3D models as the map sees them, and rendered plants (CC0, Poly Haven;
// web/textures/SOURCES.md), strip images beside the page (`ground.jpg`, `carpets.jpg`, `fire.jpg`, `snags.png`, then the rendered models' files
// that `STRIPS` names: `models.png`, `heights.png`, `trees.png`, `trees2.png`, ...), each square of them a layer of one texture array with its mip levels down to one texel, which is its mean. The map draws each at its
// real size as a change around the colour the world gives the ground (the image over its mean), so from afar, where the
// mips average it, the colour is the world's alone.
import { Gpu, Texture } from './gpu';
import { timed } from './cost';

/** The rendered models' strip files, each list in its layers' order (web/textures/strips.json, written by the bake tool
 * and put in the page by build.mjs). */
declare const STRIPS: { models: string[]; heights: string[]; trees: string[] };

/** The side of each layer of the strips, texels: the photographs (tex.wgsl's `*_TEX`), the ground cover after them
 * (tex.wgsl `CARPETS_FROM`), the fire's sprites and its photograph (tex.wgsl `FIRE_FROM`), the snags (tex.wgsl
 * `SNAGS_FROM`), the rendered models after
 * those (tex.wgsl `MODELS_FROM`), the heights of those that stand
 * in water (tex.wgsl `HEIGHTS_FROM`) and the trees last (tex.wgsl `TREES_FROM`), as many layers as each strip holds. */
const PHOTO_SIDE = 512;
/** The mip levels of a layer: from `PHOTO_SIDE` texels down to one (detail-worker.ts makes them). */
const PHOTO_MIPS = Math.log2(PHOTO_SIDE) + 1;

/** The view needs the detail where a pixel of the screen takes less than `DETAIL_LOAD_E` m, and lets it go where one
 * takes more than `DETAIL_FREE_E` m: past that no layer is read finer than its mean, which the blank texture gives too
 * (the image over its mean is 1), so the picture is the same without it; the gap between them keeps a zoom about one
 * size from loading it and letting it go over and over. */
const DETAIL_LOAD_E = 25, DETAIL_FREE_E = 40;

/** The layers of the strips, once their files' heads are read; and the layers' levels made (detail-worker.ts) and not
 * yet sent to the GPU, each with its layer. */
let groundLayers = 0;
let layersMade: { layer: number; levels: Uint8Array[] }[] = [];
/** The blank texture's layers: an array of white texels (one layer would make a plain texture, not an array). */
const BLANK_LAYERS = 2;
/** Layers sent to the GPU a frame at most (each 1.4 MB with its levels): the strips go over some frames, so no frame
 * stops long for them, and each layer's levels are let go once sent. */
const LAYERS_A_FRAME = 4;
/** The GPU the detail is sent to (`sendDetail`) and what hears that all its layers are sent; the detail's texture there,
 * how many of its layers are filled, and the frame that sends the next. None is drawn from till all are
 * (`groundDetail`). */
let sending: { gpu: Gpu; sent: () => void } | null = null;
let detailHeld: { t: Texture | null; filled: number; frame: number } = { t: null, filled: 0, frame: 0 };
/** The load of the detail under way or done (null: none, or let go): its number (a later load or a letting go ends
 * an earlier one) and its worker. */
let detailLoad: { number: number; worker: Worker | null } | null = null;
let detailLoads = 0;
const photoBlank = new WeakMap<Gpu, Texture>();

/** A cut-out texel shows where its cover is at least this (tex.wgsl `model_texel`, of the trees; detail-worker.ts
 * keeps it so in the mips). */
const CUT = 128;

/** The strips' files beside the page, in their layers' order, and whether each is the trees' (cut-outs). */
function stripFiles(): [string, boolean][] {
  return [...['ground.jpg', 'carpets.jpg', 'fire.jpg', 'snags.png', ...STRIPS.models, ...STRIPS.heights].map(f => [f, false] as [string, boolean]),
    ...STRIPS.trees.map(f => [f, true] as [string, boolean])];
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

/** Fetch the strips, and decode one at a time and make its levels off the page's thread (detail-worker.ts), the next
 * once the worker is done with it, so the page holds one decoded strip at most and does not stop for them; each layer
 * is sent to the GPU as it comes. A failed fetch leaves the ground without its detail. */
async function fetchDetail(number: number): Promise<void> {
  const files = stripFiles(), current = () => detailLoad?.number === number;
  const bytes = await Promise.all(files.map(([f]) => fetch(f).then(r => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(`${f}: ${r.status}`))))));
  if (!current()) return;
  const layers = bytes.map(b => imageHeight(new Uint8Array(b)) / PHOTO_SIDE);
  groundLayers = layers.reduce((n, k) => n + k, 0);
  const source = document.getElementById('detail-src')?.textContent ?? '';
  const url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' })), w = new Worker(url);
  URL.revokeObjectURL(url);
  detailLoad!.worker = w;
  let stripDone = () => {};
  w.onmessage = (e: MessageEvent<{ layer: number; levels: Uint8Array[] }>) => {
    if (e.data.layer < 0) return stripDone();
    layersMade.push(e.data);
    sendSoon();
  };
  for (let k = 0, first = 0; k < files.length; first += layers[k++]) {
    const strip = await createImageBitmap(new Blob([bytes[k]]));
    if (!current()) { strip.close(); return; }
    const done = new Promise<void>(go => (stripDone = go));
    w.postMessage({ strip, first, cutOut: files[k][1], side: PHOTO_SIDE, cut: CUT }, [strip]);
    await done;
  }
  w.terminate();
  if (current()) detailLoad!.worker = null;
}

/** The view's pixels take `e` m of the screen's: load the detail where they are fine enough, let it go where they are
 * not (`DETAIL_LOAD_E`, `DETAIL_FREE_E`). */
export function detailSeen(e: number): void {
  if (e < DETAIL_LOAD_E && !detailLoad) loadDetail();
  else if (e > DETAIL_FREE_E && detailLoad) letDetailGo();
}

/** Load the detail anew. */
function loadDetail(): void {
  detailLoad = { number: ++detailLoads, worker: null };
  groundLayers = 0;
  fetchDetail(detailLoad.number).catch(e => console.error(`detail: ${e}`));
}

/** Let the detail go: its load stops, and its texture and the levels not yet sent are dropped. */
function letDetailGo(): void {
  detailLoad?.worker?.terminate();
  detailLoad = null;
  layersMade = [];
  if (detailHeld.t) sending?.gpu.drop(detailHeld.t);
  cancelAnimationFrame(detailHeld.frame);
  detailHeld = { t: null, filled: 0, frame: 0 };
}

/** The detail's layers sent to the GPU, and all it has (none known before the strips' heads are read). */
export function detailSent(): [number, number] { return [detailHeld.filled, groundLayers]; }

/** Whether detail the view asked for is still to come (look/page.mjs waits for it). */
export function detailComing(): boolean { return detailLoad !== null && (groundLayers === 0 || detailHeld.filled < groundLayers); }

/** Send the detail to `gpu` as its layers come, `LAYERS_A_FRAME` a frame; `sent` hears once all are. */
export function sendDetail(gpu: Gpu, sent: () => void): void {
  sending = { gpu, sent };
  sendSoon();
}

/** After the GPU lost what it detailHeld (its textures are empty again): the detail, if loaded, is loaded anew. */
export function detailLost(): void {
  if (!detailLoad) return;
  letDetailGo();
  loadDetail();
}

/** Send the next layers that came at the next frame. */
function sendSoon(): void {
  const s = sending, h = detailHeld;
  if (!s || h.frame || !layersMade.length) return;
  h.frame = requestAnimationFrame(() => timed('detail', () => {
    h.frame = 0;
    if (detailHeld !== h) return;
    h.t ??= s.gpu.texture('detail', PHOTO_SIDE, PHOTO_SIDE, 'rgba8unorm', groundLayers, PHOTO_MIPS);
    for (const { layer, levels } of layersMade.splice(0, LAYERS_A_FRAME)) {
      levels.forEach((d, k) => s.gpu.write(h.t!, d, k, layer));
      h.filled++;
    }
    if (h.filled === groundLayers) s.sent();
    else sendSoon();
  }));
}

/** The ground's detail on `gpu`: the strips' texture once all its layers are sent, else one white texel a layer (no
 * detail). */
export function groundDetail(gpu: Gpu): Texture {
  if (sending?.gpu === gpu && detailHeld.t && detailHeld.filled === groundLayers) return detailHeld.t;
  let t = photoBlank.get(gpu);
  if (!t) {
    t = gpu.texture('detail', 1, 1, 'rgba8unorm', BLANK_LAYERS, 1);
    gpu.write(t, new Uint8Array(4 * BLANK_LAYERS).fill(255));
    photoBlank.set(gpu, t);
  }
  return t;
}
