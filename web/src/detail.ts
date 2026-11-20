// The ground's detail up close (web/shaders/lib/detail.wgsl): photographs of ground seen from above, tiles of ground
// cover rendered from 3D models as the map sees them, and rendered plants (CC0, Poly Haven;
// web/textures/SOURCES.md), strip images beside the page (`ground.jpg`, `carpets.jpg`, `fire.jpg`, `snags.png`, then the rendered models' files
// that `STRIPS` names: `models.png`, `heights.png`, `trees.png`, `trees2.png`, ...), each square of them a layer of one texture array with its mip levels down to one texel, which is its mean. The map draws each at its
// real size as a change around the colour the world gives the ground (the image over its mean), so from afar, where the
// mips average it, the colour is the world's alone.
import { Gpu, Texture } from './gpu';

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

/** The layers of the strips, once they are fetched; and the layers' levels made (detail-worker.ts) and not yet sent to
 * the GPU, each with its layer. */
let groundLayers = 0;
let layersMade: { layer: number; levels: Uint8Array[] }[] = [];
/** The blank texture's layers: an array of white texels (one layer would make a plain texture, not an array). */
const BLANK_LAYERS = 2;
/** Layers sent to the GPU a frame at most (each 1.4 MB with its levels): the strips go over some frames, so no frame
 * stops long for them, and each layer's levels are let go once sent. */
const LAYERS_A_FRAME = 4;
/** The detail's texture on the GPU it is sent to (`sendDetail`), how many of its layers are filled, the frame that sends
 * the next and what hears that all are sent. None is drawn from till all are (`groundDetail`). */
let sending: { gpu: Gpu; t: Texture | null; filled: number; frame: number; sent: () => void } | null = null;
const photoBlank = new WeakMap<Gpu, Texture>();

/** A cut-out texel shows where its cover is at least this (tex.wgsl `model_texel`, of the trees; detail-worker.ts
 * keeps it so in the mips). */
const CUT = 128;

/** The image `name` beside the page. */
function photoImage(name: string): Promise<ImageBitmap> {
  return fetch(name).then(r => (r.ok ? r.blob() : Promise.reject(new Error(`${name}: ${r.status}`)))).then(b => createImageBitmap(b));
}

/** Fetch the strips and make their levels off the page's thread, layer by layer (detail-worker.ts), so the page does
 * not stop for them; each is sent to the GPU as it comes (a failed fetch leaves the ground without its detail). */
function fetchDetail(): void {
  Promise.all(['ground.jpg', 'carpets.jpg', 'fire.jpg', 'snags.png', ...STRIPS.models, ...STRIPS.heights, ...STRIPS.trees].map(photoImage))
    .then(strips => new Promise<void>((done, failed) => {
      const trees = strips.slice(-STRIPS.trees.length).reduce((n, s) => n + s.height / PHOTO_SIDE, 0);
      groundLayers = strips.reduce((n, s) => n + s.height / PHOTO_SIDE, 0);
      const source = document.getElementById('detail-src')?.textContent ?? '';
      const w = new Worker(URL.createObjectURL(new Blob([source], { type: 'text/javascript' })));
      w.onerror = e => failed(new Error(e.message));
      w.onmessage = (e: MessageEvent<{ layer: number; levels: Uint8Array[] }>) => {
        layersMade.push(e.data);
        sendSoon();
        if (e.data.layer === groundLayers - 1) { w.terminate(); done(); }
      };
      w.postMessage({ strips, trees, side: PHOTO_SIDE, cut: CUT }, strips);
    }))
    .catch(e => console.error(`detail: ${e}`));
}
fetchDetail();

/** Send the detail to `gpu` as its layers come, `LAYERS_A_FRAME` a frame; `sent` hears once all are. */
export function sendDetail(gpu: Gpu, sent: () => void): void {
  sending = { gpu, t: null, filled: 0, frame: 0, sent };
  sendSoon();
}

/** After the GPU lost what it held (its textures are empty again): fetch the strips again and send them anew. */
export function detailLost(): void {
  if (!sending) return;
  if (sending.t) sending.gpu.drop(sending.t);
  cancelAnimationFrame(sending.frame);
  sendDetail(sending.gpu, sending.sent);
  layersMade = [];
  fetchDetail();
}

/** Send the next layers that came at the next frame. */
function sendSoon(): void {
  const s = sending;
  if (!s || s.frame || !layersMade.length) return;
  s.frame = requestAnimationFrame(() => {
    s.frame = 0;
    s.t ??= s.gpu.texture(PHOTO_SIDE, PHOTO_SIDE, 'rgba8unorm', groundLayers, PHOTO_MIPS);
    for (const { layer, levels } of layersMade.splice(0, LAYERS_A_FRAME)) {
      levels.forEach((d, k) => s.gpu.write(s.t!, d, k, layer));
      s.filled++;
    }
    if (s.filled === groundLayers) s.sent();
    else sendSoon();
  });
}

/** The ground's detail on `gpu`: the strips' texture once all its layers are sent, else one white texel a layer (no
 * detail). */
export function groundDetail(gpu: Gpu): Texture {
  if (sending?.gpu === gpu && sending.t && sending.filled === groundLayers) return sending.t;
  let t = photoBlank.get(gpu);
  if (!t) {
    t = gpu.texture(1, 1, 'rgba8unorm', BLANK_LAYERS, 1);
    gpu.write(t, new Uint8Array(4 * BLANK_LAYERS).fill(255));
    photoBlank.set(gpu, t);
  }
  return t;
}
