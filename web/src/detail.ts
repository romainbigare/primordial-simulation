// The ground's detail up close (web/shaders/lib/detail.wgsl): photographs of ground seen from above, tiles of ground
// cover rendered from 3D models as the map sees them, and rendered plants (CC0, Poly Haven;
// web/textures/SOURCES.md), strip images beside the page (`ground.jpg`, `carpets.jpg`, then the rendered models' files
// that `STRIPS` names: `models.png`, `trees.png`, `trees2.png`, ...), each square of them a layer of one texture array with its mip levels down to one texel, which is its mean. The map draws each at its
// real size as a change around the colour the world gives the ground (the image over its mean), so from afar, where the
// mips average it, the colour is the world's alone.
import { Gpu, Texture } from './gpu';

/** The rendered models' strip files, each list in its layers' order (web/textures/strips.json, written by the bake tool
 * and put in the page by build.mjs). */
declare const STRIPS: { models: string[]; trees: string[] };

/** The side of each layer of the strips, texels: the photographs (tex.wgsl's `*_TEX`), the ground cover after them
 * (tex.wgsl `CARPETS_FROM`), the rendered models after those (tex.wgsl `MODELS_FROM`) and the trees last (tex.wgsl
 * `TREES_FROM`), as many layers as each strip holds. */
const PHOTO_SIDE = 512;
/** The mip levels of a layer: from `PHOTO_SIDE` texels down to one (detail-worker.ts makes them). */
const PHOTO_MIPS = Math.log2(PHOTO_SIDE) + 1;

let photoLevels: Uint8Array[] | null = null;
/** The layers of the strips, once they are fetched. */
let groundLayers = 0;
/** The blank texture's layers: an array of white texels (one layer would make a plain texture, not an array). */
const BLANK_LAYERS = 2;
const photoMade = new WeakMap<Gpu, Texture>();
const photoBlank = new WeakMap<Gpu, Texture>();

/** A cut-out texel shows where its cover is at least this (tex.wgsl `model_texel`, of the trees; detail-worker.ts
 * keeps it so in the mips). */
const CUT = 128;

/** The image `name` beside the page. */
function photoImage(name: string): Promise<ImageBitmap> {
  return fetch(name).then(r => (r.ok ? r.blob() : Promise.reject(new Error(`${name}: ${r.status}`)))).then(b => createImageBitmap(b));
}

/** Once the strips are fetched and their levels made (a failed fetch leaves the ground without its detail): the levels
 * are made off the page's thread (detail-worker.ts), so the page does not stop for them. */
export const groundReady: Promise<void> = Promise.all(['ground.jpg', 'carpets.jpg', ...STRIPS.models, ...STRIPS.trees].map(photoImage))
  .then(strips => new Promise<void>((done, failed) => {
    const trees = strips.slice(-STRIPS.trees.length).reduce((n, s) => n + s.height / PHOTO_SIDE, 0);
    const source = document.getElementById('detail-src')?.textContent ?? '';
    const w = new Worker(URL.createObjectURL(new Blob([source], { type: 'text/javascript' })));
    w.onerror = e => failed(new Error(e.message));
    w.onmessage = (e: MessageEvent<{ levels: Uint8Array[]; layers: number }>) => {
      photoLevels = e.data.levels; groundLayers = e.data.layers;
      w.terminate();
      done();
    };
    w.postMessage({ strips, trees, side: PHOTO_SIDE, cut: CUT }, strips);
  }))
  .catch(e => console.error(`detail: ${e}`));

/** The ground's detail on `gpu`: the strip's texture once it is fetched, else one white texel a layer (no detail). */
export function groundDetail(gpu: Gpu): Texture {
  if (photoLevels) {
    let t = photoMade.get(gpu);
    if (!t) {
      t = gpu.texture(PHOTO_SIDE, PHOTO_SIDE, 'rgba8unorm', groundLayers, PHOTO_MIPS);
      photoLevels.forEach((d, k) => gpu.write(t!, d, k));
      photoMade.set(gpu, t);
    }
    return t;
  }
  let t = photoBlank.get(gpu);
  if (!t) {
    t = gpu.texture(1, 1, 'rgba8unorm', BLANK_LAYERS, 1);
    gpu.write(t, new Uint8Array(4 * BLANK_LAYERS).fill(255));
    photoBlank.set(gpu, t);
  }
  return t;
}
