// The ground's detail up close (web/shaders/lib/detail.wgsl): photographs of ground seen from above, tiles of ground
// cover rendered from 3D models as the angled view sees them, and rendered plants (CC0, Poly Haven;
// web/textures/SOURCES.md), four strip images beside the page (`ground.jpg`, `carpets.jpg`, `models.png`, `trees.png`),
// each a layer of one texture array with its mip levels down to one texel, which is its mean. The map draws each at its
// real size as a change around the colour the world gives the ground (the image over its mean), so from afar, where the
// mips average it, the colour is the world's alone.
import { Gpu, Texture } from './gpu';

/** The side of each layer of the strips, texels: the photographs (tex.wgsl's `*_TEX`), the ground cover after them
 * (tex.wgsl `CARPETS_FROM`), the rendered models after those (tex.wgsl `MODELS_FROM`) and the trees last (tex.wgsl
 * `TREES_FROM`), as many layers as each strip holds. */
const PHOTO_SIDE = 512;
/** The mip levels of a layer: from `PHOTO_SIDE` texels down to one. */
const PHOTO_MIPS = Math.log2(PHOTO_SIDE) + 1;

let photoLevels: Uint8Array[] | null = null;
/** The layers of the strips, once they are fetched. */
let groundLayers = 0;
/** The blank texture's layers: an array of white texels (one layer would make a plain texture, not an array). */
const BLANK_LAYERS = 2;
const photoMade = new WeakMap<Gpu, Texture>();
const photoBlank = new WeakMap<Gpu, Texture>();

/** The strips' levels, each of its `layers` after the last: level 0 as the image holds it, each next one the mean of
 * four texels (their colour weighted by their cover). */
function photoMips(base: Uint8Array, layers: number): Uint8Array[] {
  const out = [base];
  for (let k = 1; k < PHOTO_MIPS; k++) {
    const s = PHOTO_SIDE >> k, from = out[k - 1], next = new Uint8Array(4 * s * s * layers);
    for (let l = 0; l < layers; l++) {
      for (let y = 0; y < s; y++) for (let x = 0; x < s; x++) {
        const at = (dx: number, dy: number, c: number) => from[4 * ((l * 2 * s + 2 * y + dy) * 2 * s + 2 * x + dx) + c];
        const o = 4 * ((l * s + y) * s + x);
        // The colour is the mean of the texels by their cover, so a cut-out's mips do not take the black about it.
        const a = [at(0, 0, 3), at(1, 0, 3), at(0, 1, 3), at(1, 1, 3)], sum = a[0] + a[1] + a[2] + a[3];
        for (let c = 0; c < 3; c++) {
          next[o + c] = sum > 0 ? Math.round((a[0] * at(0, 0, c) + a[1] * at(1, 0, c) + a[2] * at(0, 1, c) + a[3] * at(1, 1, c)) / sum) : 0;
        }
        next[o + 3] = (sum + 2) >> 2;
      }
    }
    out.push(next);
  }
  return out;
}

/** The image `name` beside the page. */
function photoImage(name: string): Promise<ImageBitmap> {
  return fetch(name).then(r => (r.ok ? r.blob() : Promise.reject(new Error(`${name}: ${r.status}`)))).then(b => createImageBitmap(b));
}

/** Once the strips are fetched and their levels made (a failed fetch leaves the ground without its detail). */
export const groundReady: Promise<void> = Promise.all([photoImage('ground.jpg'), photoImage('carpets.jpg'), photoImage('models.png'), photoImage('trees.png')])
  .then(strips => {
    const layers = strips.reduce((n, s) => n + s.height / PHOTO_SIDE, 0);
    const c = new OffscreenCanvas(PHOTO_SIDE, PHOTO_SIDE * layers), g = c.getContext('2d')!;
    let at = 0;
    for (const s of strips) { g.drawImage(s, 0, at); at += s.height; }
    photoLevels = photoMips(new Uint8Array(g.getImageData(0, 0, PHOTO_SIDE, PHOTO_SIDE * layers).data.buffer), layers);
    groundLayers = layers;
  })
  .catch(() => undefined);

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
