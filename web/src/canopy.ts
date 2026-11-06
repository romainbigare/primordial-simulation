// The cover of each pixel of a fine raster (fine.ts), for the ground patterns drawn over it (vegetation.ts): read in
// the worker from the raster's layers and handed to the page with its picture.
import { decode } from './data';
import { Fine } from './fine';

/** Per pixel of a fine raster `across` x `down`: the cover's mass and walls, kg/m², its capture area, m²/m², the
 * ground's temperature, K, and rain, mol/m²/s; and whether it is dry open ground (1), on which cover is drawn. */
export interface FineCover { across: number; down: number; kg: Float32Array; wood: Float32Array; leaf: Float32Array; t: Float32Array; rain: Float32Array; dry: Uint8Array }

/** Ice thicker than this, m, hides the cover (where fine.ts starts to paint snow). */
const SNOW_M = 0.02;

/** The cover of the pixels of `f`; null for a tape without the cover's layers. A pixel is dry where no liquid's surface
 * lies over its ground and no ice over it. */
export function fineCover(f: Fine): FineCover | null {
  const w = f.world;
  if (!['plants', 'wood', 'leaf', 't_ground', 'rain', 'ice'].every(n => w.index.has(n))) return null;
  const ice = decode(w, 0, 'ice');
  const dry = Uint8Array.from(f.ground, (g, i) => ((Number.isNaN(f.water[i]) || f.water[i] <= g) && ice[i] < SNOW_M ? 1 : 0));
  return { across: f.across, down: f.down, kg: decode(w, 0, 'plants'), wood: decode(w, 0, 'wood'), leaf: decode(w, 0, 'leaf'), t: decode(w, 0, 't_ground'), rain: decode(w, 0, 'rain'), dry };
}
