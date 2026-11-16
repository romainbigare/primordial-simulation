// The ground's detail up close (web/shaders/lib/detail.wgsl): photographs of ground seen from above and cut-out plants
// (CC0, Poly Haven; web/textures/SOURCES.md), two strip images beside the page (`ground.jpg`, and `sprites.png` with
// the plants four to a layer), each a layer of one texture array with its mip photoLevels down to one texel, which is its
// mean. The map draws each at its real size as a change around the colour the
// world gives the ground (the photograph over its mean), so from afar, where the mips average it, the colour is the
// world's alone.
import { Gpu, Texture } from './gpu';

/** The layers of the photographs (detail.wgsl's `*_TEX`, each its photograph's side in the world in `detail_m`) and of
 * the plants after them (detail.wgsl `SPRITES_FROM`), and the side of each, texels. */
const PHOTO_LAYERS = 12, SPRITE_LAYERS = 5, MODEL_LAYERS = 7, GROUND_LAYERS = PHOTO_LAYERS + SPRITE_LAYERS + MODEL_LAYERS;
const PHOTO_SIDE = 512;
/** The mip photoLevels of a layer: from `PHOTO_SIDE` texels down to one. */
const PHOTO_MIPS = Math.log2(PHOTO_SIDE) + 1;

let photoLevels: Uint8Array[] | null = null;
const photoMade = new WeakMap<Gpu, Texture>();
const photoBlank = new WeakMap<Gpu, Texture>();

/** The strip's photoLevels, each layer after layer: level 0 as the image holds it, each next one the mean of four texels. */
function photoMips(base: Uint8Array): Uint8Array[] {
  const out = [base];
  for (let k = 1; k < PHOTO_MIPS; k++) {
    const s = PHOTO_SIDE >> k, from = out[k - 1], next = new Uint8Array(4 * s * s * GROUND_LAYERS);
    for (let l = 0; l < GROUND_LAYERS; l++) {
      for (let y = 0; y < s; y++) for (let x = 0; x < s; x++) for (let c = 0; c < 4; c++) {
        const at = (dx: number, dy: number) => from[4 * ((l * 2 * s + 2 * y + dy) * 2 * s + 2 * x + dx) + c];
        next[4 * ((l * s + y) * s + x) + c] = (at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) >> 2;
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

/** Once the strips are fetched and their photoLevels photoMade (a failed fetch leaves the ground without its detail). */
export const groundReady: Promise<void> = Promise.all([photoImage('ground.jpg'), photoImage('sprites.png'), photoImage('models.png')])
  .then(([photos, plants, models]) => {
    const c = new OffscreenCanvas(PHOTO_SIDE, PHOTO_SIDE * GROUND_LAYERS), g = c.getContext('2d')!;
    g.drawImage(photos, 0, 0);
    g.drawImage(plants, 0, PHOTO_SIDE * PHOTO_LAYERS);
    g.drawImage(models, 0, PHOTO_SIDE * (PHOTO_LAYERS + SPRITE_LAYERS));
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
