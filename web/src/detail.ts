// The ground's detail up close (web/shaders/lib/detail.wgsl): photographs of ground seen from above, tiles of ground
// cover rendered from 3D models as the angled view sees them, and rendered plants (CC0, Poly Haven;
// web/textures/SOURCES.md), three strip images beside the page (`ground.jpg`, `carpets.jpg`, `models.png`), each a layer
// of one texture array with its mip levels down to one texel, which is its mean. The map draws each at its real size as
// a change around the colour the world gives the ground (the image over its mean), so from afar, where the mips average
// it, the colour is the world's alone.
import { Gpu, Texture } from './gpu';

/** The layers of the photographs (tex.wgsl's `*_TEX`), of the ground cover after them (tex.wgsl `CARPETS_FROM`) and of
 * the rendered plants after those (tex.wgsl `MODELS_FROM`), and the side of each, texels. */
const PHOTO_LAYERS = 12, CARPET_LAYERS = 10, MODEL_LAYERS = 12, GROUND_LAYERS = PHOTO_LAYERS + CARPET_LAYERS + MODEL_LAYERS;
const PHOTO_SIDE = 512;
/** The mip levels of a layer: from `PHOTO_SIDE` texels down to one. */
const PHOTO_MIPS = Math.log2(PHOTO_SIDE) + 1;

let photoLevels: Uint8Array[] | null = null;
const photoMade = new WeakMap<Gpu, Texture>();
const photoBlank = new WeakMap<Gpu, Texture>();

/** The strips' levels, each layer after layer: level 0 as the image holds it, each next one the mean of four texels
 * (their colour weighted by their cover). */
function photoMips(base: Uint8Array): Uint8Array[] {
  const out = [base];
  for (let k = 1; k < PHOTO_MIPS; k++) {
    const s = PHOTO_SIDE >> k, from = out[k - 1], next = new Uint8Array(4 * s * s * GROUND_LAYERS);
    for (let l = 0; l < GROUND_LAYERS; l++) {
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
export const groundReady: Promise<void> = Promise.all([photoImage('ground.jpg'), photoImage('carpets.jpg'), photoImage('models.png')])
  .then(([photos, carpets, models]) => {
    const c = new OffscreenCanvas(PHOTO_SIDE, PHOTO_SIDE * GROUND_LAYERS), g = c.getContext('2d')!;
    g.drawImage(photos, 0, 0);
    g.drawImage(carpets, 0, PHOTO_SIDE * PHOTO_LAYERS);
    g.drawImage(models, 0, PHOTO_SIDE * (PHOTO_LAYERS + CARPET_LAYERS));
    photoLevels = photoMips(new Uint8Array(g.getImageData(0, 0, PHOTO_SIDE, PHOTO_SIDE * GROUND_LAYERS).data.buffer));
  })
  .catch(() => undefined);

/** The ground's detail on `gpu`: the strip's texture once it is fetched, else one white texel a layer (no detail). */
export function groundDetail(gpu: Gpu): Texture {
  if (photoLevels) {
    let t = photoMade.get(gpu);
    if (!t) {
      t = gpu.texture(PHOTO_SIDE, PHOTO_SIDE, 'rgba8unorm', GROUND_LAYERS, PHOTO_MIPS);
      photoLevels.forEach((d, k) => gpu.write(t!, d, k));
      photoMade.set(gpu, t);
    }
    return t;
  }
  let t = photoBlank.get(gpu);
  if (!t) {
    t = gpu.texture(1, 1, 'rgba8unorm', GROUND_LAYERS, 1);
    gpu.write(t, new Uint8Array(4 * GROUND_LAYERS).fill(255));
    photoBlank.set(gpu, t);
  }
  return t;
}
